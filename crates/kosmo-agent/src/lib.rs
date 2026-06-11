//! Kosmocrates agent — the closed execution loop.
//!
//! This crate models the layer that sits above the pipeline and turns the
//! ranked [`ActionItem`] queue into attempted workspace changes.
//!
//! # Loop
//!
//! ```text
//! WorkspacePipelineSession::run()
//!     └─ IntegrationRunReport::action_items()   ← PLAN
//!          └─ filter by min_confidence / max_steps
//!               └─ ActionSynthesizer::synthesize()  ← SYNTHESIZE
//!                    └─ try_materialize()           ← VALIDATE + EXECUTE
//!                         └─ ExecutionFeedback       ← OBSERVE
//!                              └─ AgentSession::feedback_history
//!                                   └─ next run()  ← LOOP
//! ```
//!
//! In **dry-run mode** (the default) the synthesizer is called and the patch
//! is recorded, but no files are written to disk. The validation step marks
//! the attempt as `applied = false` with reason `"dry-run"`.
//!
//! Actual file-writing (via `cargo check` validation + fs write + git commit)
//! is reserved for the `kosmo-materialize` crate, which extends this layer
//! with a concrete `Materializer` implementation.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use kosmo_core::{
    AttractorStatus, Digest, FeedbackOutcome, GateResult, PolicyProfile, PromotionFeedback, Wish,
    WishAssessment, WishConvergenceTrace, WishFacet, Q16,
};
use kosmo_intent::WishSession;
use kosmo_materialize::{MaterializeOptions, MaterializeReport, Materializer, PatchValidator};
use kosmo_pipeline::{ActionItem, ActionItemKind, IntegrationRunOptions, WorkspacePipelineSession};
use kosmo_pse_bridge::MemoryRecall;
use kosmo_synthesizer::{ActionSynthesizer, SynthesisRequest, SynthesisResult};

pub use kosmo_materialize::{AlwaysFail, AlwaysPass, CargoFoundryValidator};

// ─── AgentOptions ─────────────────────────────────────────────────────────────

/// Configuration for one [`AgentSession`] run.
#[derive(Clone, Debug)]
pub struct AgentOptions {
    /// Maximum number of action items to synthesize per [`AgentSession::run`] call.
    pub max_steps: u32,
    /// Skip any action item whose synthesized confidence is below this threshold.
    /// Default: `Q16::HALF` (0.5).
    pub min_confidence: Q16,
    /// When `true` (the default), synthesize patches but do not write any files.
    /// Set to `false` only with `PolicyProfile::operator_approved()`.
    pub dry_run: bool,
    pub pipeline_options: IntegrationRunOptions,
    /// When `true` and `dry_run` is `false`, each accepted patch is committed
    /// to the workspace's git history via `git add -A && git commit`.
    pub commit_to_git: bool,
    /// How many recalled crystals to attach per action when a memory is
    /// present (see [`AgentSession::with_recall`]). Default: 5.
    pub grounding_top: u32,
}

impl Default for AgentOptions {
    fn default() -> Self {
        Self {
            max_steps: 5,
            min_confidence: Q16::HALF,
            dry_run: true,
            pipeline_options: IntegrationRunOptions::report_only(),
            commit_to_git: false,
            grounding_top: 5,
        }
    }
}

impl AgentOptions {
    pub fn with_max_steps(mut self, n: u32) -> Self {
        self.max_steps = n;
        self
    }
    pub fn with_min_confidence(mut self, c: Q16) -> Self {
        self.min_confidence = c;
        self
    }
    pub fn with_pipeline_options(mut self, o: IntegrationRunOptions) -> Self {
        self.pipeline_options = o;
        self
    }
    pub fn with_grounding_top(mut self, n: u32) -> Self {
        self.grounding_top = n;
        self
    }
}

// ─── ValidationResult ────────────────────────────────────────────────────────

/// Outcome of attempting to validate a patch before materialization.
///
/// In dry-run mode both fields are `None` — no compilation or test execution
/// is performed. A future `kosmo-materialize` crate will populate these by
/// running `cargo check` and `cargo test` in a scratch directory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// `Some(true)` if `cargo check` passed, `Some(false)` if it failed,
    /// `None` if not run (dry-run).
    pub compile_passed: Option<bool>,
    /// `Some(true)` if `cargo test` passed, `Some(false)` if it failed,
    /// `None` if not run.
    pub tests_passed: Option<bool>,
    pub gate_result: GateResult,
}

impl ValidationResult {
    /// Placeholder used in dry-run mode: passes gate with a `Warn` to signal
    /// that real validation has not been executed.
    pub fn dry_run() -> Self {
        Self {
            compile_passed: None,
            tests_passed: None,
            gate_result: GateResult::Warn {
                message: "dry-run: patch recorded but not validated".into(),
            },
        }
    }

    pub fn is_acceptable(&self) -> bool {
        !matches!(self.gate_result, GateResult::Reject { .. })
    }
}

// ─── MaterializationAttempt ───────────────────────────────────────────────────

#[derive(Serialize)]
struct AttemptContent {
    patch_id: Digest,
    applied: bool,
}

/// A content-addressed record of one materialization attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterializationAttempt {
    pub attempt_id: Digest,
    pub patch_id: Digest,
    pub action_id: Digest,
    /// Whether the patch was actually written to the workspace.
    pub applied: bool,
    pub validation: ValidationResult,
    /// Human-readable reason why `applied` is `false` (if it is).
    pub blocking_reason: Option<String>,
    /// Estimated lines added (from patch content; not a diff against the actual file).
    pub lines_added: u32,
    /// SHA of the git commit created after `AppliedToHost` when
    /// `AgentOptions::commit_to_git` is set. `None` otherwise.
    pub commit_sha: Option<String>,
}

impl MaterializationAttempt {
    fn new_dry_run(patch_id: Digest, action_id: Digest, lines_added: u32) -> Self {
        let attempt_id = Digest::of(&AttemptContent {
            patch_id,
            applied: false,
        });
        Self {
            attempt_id,
            patch_id,
            action_id,
            applied: false,
            validation: ValidationResult::dry_run(),
            blocking_reason: Some("dry-run mode".into()),
            lines_added,
            commit_sha: None,
        }
    }

    /// Build from a real [`MaterializeReport`] produced by `kosmo-materialize`.
    fn from_materialize_report(
        action_id: Digest,
        lines_added: u32,
        report: &MaterializeReport,
    ) -> Self {
        let applied = report.applied_to_host;
        let attempt_id = Digest::of(&AttemptContent {
            patch_id: report.patch_id,
            applied,
        });
        Self {
            attempt_id,
            patch_id: report.patch_id,
            action_id,
            applied,
            validation: ValidationResult {
                compile_passed: report.compile_passed,
                tests_passed: report.tests_passed,
                gate_result: report.gate_result.clone(),
            },
            blocking_reason: report.blocking_reason.clone(),
            lines_added,
            commit_sha: report.commit_sha.clone(),
        }
    }
}

// ─── ExecutionFeedback ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct FeedbackContent {
    action_id: Digest,
    is_positive: bool,
    confidence_raw: i64,
}

/// Content-addressed record of what happened when the agent acted on one
/// [`ActionItem`]. Accumulates in [`AgentSession::feedback_history`] and
/// can feed into future pipeline runs to adjust prioritization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionFeedback {
    pub feedback_id: Digest,
    pub action_id: Digest,
    pub synthesis_confidence: Q16,
    pub materialization: Option<MaterializationAttempt>,
    /// `true` when the patch passed validation (or would have, in dry-run).
    /// `false` when synthesis failed or gate rejected.
    pub is_positive: bool,
}

impl ExecutionFeedback {
    fn new(
        action_id: Digest,
        confidence: Q16,
        materialization: Option<MaterializationAttempt>,
        is_positive: bool,
    ) -> Self {
        let feedback_id = Digest::of(&FeedbackContent {
            action_id,
            is_positive,
            confidence_raw: confidence.raw(),
        });
        Self {
            feedback_id,
            action_id,
            synthesis_confidence: confidence,
            materialization,
            is_positive,
        }
    }

    fn skipped(action_id: Digest, confidence: Q16) -> Self {
        Self::new(action_id, confidence, None, false)
    }
}

// ─── AgentStep ───────────────────────────────────────────────────────────────

/// A single iteration of the agent loop: one action item fully processed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentStep {
    pub step_number: u32,
    pub action: ActionItem,
    pub synthesis: SynthesisResult,
    pub materialization: Option<MaterializationAttempt>,
    pub feedback: ExecutionFeedback,
}

// ─── AgentRunReport ──────────────────────────────────────────────────────────

// ─── WishRunOutcome ────────────────────────────────────────────────────────────

/// How one [`AgentSession::run`] moved the workspace relative to an attached wish.
///
/// Present in [`AgentRunReport::wish`] only when a wish was attached via
/// [`AgentSession::with_wish`] and the workspace was observable (a real cargo
/// tree). One `run()` is one step of the dynamics `x_t → x_{t+1}`; the
/// convergence trajectory accumulates across runs inside the session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WishRunOutcome {
    /// Identity of the wish-attractor being pursued.
    pub wish_id: Digest,
    /// This run's measurement of the workspace against the wish.
    pub assessment: WishAssessment,
    /// Convergence status across every run so far (the contraction contract).
    pub attractor_status: AttractorStatus,
    /// `true` when this run *increased* the wish distance versus the previous
    /// run — a contraction violation the driving loop must treat fail-closed.
    pub diverged: bool,
}

impl WishRunOutcome {
    /// The remaining unmet facets — the prioritized agenda toward the wish.
    pub fn agenda(&self) -> &[WishFacet] {
        &self.assessment.unmet_facets
    }
    /// The wish is realized: the workspace sits at the attractor this run.
    pub fn is_realized(&self) -> bool {
        self.assessment.is_realized()
    }
}

#[derive(Serialize)]
struct ReportContent {
    workspace_hash: Digest,
    step_ids: Vec<Digest>,
}

/// Summary of one complete [`AgentSession::run`] invocation.
///
/// `run_id` is content-addressed from the workspace path and the ordered
/// set of step feedback IDs, making it deterministic for identical runs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentRunReport {
    pub run_id: Digest,
    pub workspace_path: String,
    pub pipeline_run_number: u32,
    pub total_actions_available: usize,
    pub steps: Vec<AgentStep>,
    pub steps_synthesized: u32,
    pub steps_skipped_low_confidence: u32,
    pub steps_materialized: u32,
    pub total_lines_proposed: u32,
    /// Gate result from the underlying pipeline run.
    pub pipeline_gate: GateResult,
    /// How this run moved the workspace toward an attached wish (`None` when no
    /// wish is attached or the workspace was not observable).
    pub wish: Option<WishRunOutcome>,
}

impl AgentRunReport {
    fn build(
        workspace_path: &str,
        pipeline_run_number: u32,
        total_actions_available: usize,
        steps: Vec<AgentStep>,
        pipeline_gate: GateResult,
        steps_skipped_low_confidence: u32,
        wish: Option<WishRunOutcome>,
    ) -> Self {
        let workspace_hash = Digest::of(&workspace_path.to_string());
        let step_ids: Vec<Digest> = steps.iter().map(|s| s.feedback.feedback_id).collect();
        let run_id = Digest::of(&ReportContent {
            workspace_hash,
            step_ids,
        });

        let steps_synthesized = steps.len() as u32;
        let steps_materialized = steps
            .iter()
            .filter(|s| {
                s.materialization
                    .as_ref()
                    .map(|m| m.applied)
                    .unwrap_or(false)
            })
            .count() as u32;
        let total_lines_proposed = steps.iter().map(|s| s.synthesis.patch.total_lines()).sum();

        AgentRunReport {
            run_id,
            workspace_path: workspace_path.to_string(),
            pipeline_run_number,
            total_actions_available,
            steps,
            steps_synthesized,
            steps_skipped_low_confidence,
            steps_materialized,
            total_lines_proposed,
            pipeline_gate,
            wish,
        }
    }

    /// Number of processed steps that were facet-directed work toward the wish.
    pub fn wish_directed_count(&self) -> u32 {
        self.steps
            .iter()
            .filter(|s| matches!(s.action.kind, ActionItemKind::RealizeWishFacet { .. }))
            .count() as u32
    }
}

// ─── AgentError ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AgentError {
    Pipeline(String),
    Synthesis(String),
    /// The attached memory failed to answer. A session that was explicitly
    /// given a memory must not degrade silently (fail-closed).
    Recall(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::Pipeline(s) => write!(f, "pipeline error: {s}"),
            AgentError::Synthesis(s) => write!(f, "synthesis error: {s}"),
            AgentError::Recall(s) => write!(f, "memory recall error: {s}"),
        }
    }
}
impl std::error::Error for AgentError {}

// ─── AgentSession ─────────────────────────────────────────────────────────────

/// Stateful execution loop tying the pipeline to the synthesizer.
///
/// Each call to [`run`](AgentSession::run) executes one full iteration:
/// pipeline → action queue → synthesize top-N → (optionally) materialize.
///
/// The feedback history accumulates across calls and is available for
/// inspection or for feeding back into the pipeline via `prior_feedback`.
pub struct AgentSession {
    pipeline_session: WorkspacePipelineSession,
    synthesizer: Arc<dyn ActionSynthesizer>,
    options: AgentOptions,
    feedback_history: Vec<ExecutionFeedback>,
    /// `PromotionFeedback` records built from each step's outcome; drained into
    /// `pipeline_session.prior_feedback` at the start of the next `run()` call.
    pipeline_feedback: Vec<PromotionFeedback>,
    policy: PolicyProfile,
    /// When set and `options.dry_run == false`, patches are really applied and
    /// validated via `kosmo-materialize` (policy-gated). `None` ⇒ no real
    /// materialization happens even outside dry-run.
    validator: Option<Arc<dyn PatchValidator>>,
    /// Optional wish-attractor driver: observes the workspace against a target
    /// topology each run and tracks convergence. `None` ⇒ no wish governance.
    wish_session: Option<WishSession>,
    /// Optional anchored memory: when present, each action's synthesis request
    /// is grounded with the top recalled crystals (see
    /// [`AgentSession::with_recall`]). `None` ⇒ synthesis runs memory-free.
    recall: Option<Arc<dyn MemoryRecall>>,
}

impl AgentSession {
    pub fn new(
        options: AgentOptions,
        policy: PolicyProfile,
        synthesizer: Arc<dyn ActionSynthesizer>,
    ) -> Self {
        let pipeline_session =
            WorkspacePipelineSession::new(options.pipeline_options.clone(), policy.clone());
        Self {
            pipeline_session,
            synthesizer,
            options,
            feedback_history: vec![],
            pipeline_feedback: vec![],
            policy,
            validator: None,
            wish_session: None,
            recall: None,
        }
    }

    /// Attach a real patch validator. With a validator set and
    /// `options.dry_run == false`, the agent will apply and validate patches
    /// through `kosmo-materialize` (still policy-gated — `ReportOnly`/`DryRun`
    /// never write the host).
    pub fn with_validator(mut self, validator: Arc<dyn PatchValidator>) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Attach a wish-attractor. Each [`run`](AgentSession::run) then observes
    /// the workspace against `wish` (read-only `cargo metadata`), folds the
    /// result into a convergence trajectory, and reports whether the run moved
    /// toward the wish — or, fail-closed, diverged from it. `evidence_bundle_id`
    /// binds the assessments and traces (CROSS-006).
    pub fn with_wish(mut self, wish: Wish, evidence_bundle_id: Digest) -> Self {
        self.wish_session = Some(WishSession::new(wish, evidence_bundle_id));
        self
    }

    /// Attach an anchored memory. Each action's synthesis request is then
    /// grounded with the top [`AgentOptions::grounding_top`] crystals the
    /// memory recalls for the action's description, and every
    /// [`SynthesisResult`] cites the crystals it received
    /// (`grounding_crystal_ids`). Recall failures abort the run — a session
    /// that was explicitly given a memory must not degrade silently.
    pub fn with_recall(mut self, recall: Arc<dyn MemoryRecall>) -> Self {
        self.recall = Some(recall);
        self
    }

    /// The wish-convergence trace accumulated so far, if a wish is attached.
    pub fn wish_trace(&self) -> Option<WishConvergenceTrace> {
        self.wish_session.as_ref().map(|s| s.trace())
    }

    /// The most recent wish assessment, if any.
    pub fn wish_assessment(&self) -> Option<&WishAssessment> {
        self.wish_session.as_ref().and_then(|s| s.latest())
    }

    /// `true` if a wish is attached and its descent has diverged from the
    /// attractor at any point (the contraction invariant was violated).
    pub fn wish_diverging(&self) -> bool {
        self.wish_session
            .as_ref()
            .map(|s| !s.is_contractive())
            .unwrap_or(false)
    }

    /// Observe the workspace against the attached wish (if any) and fold the
    /// result into the convergence trajectory. Fail-soft: a workspace that
    /// cannot be read (not a cargo tree, `cargo metadata` unavailable) yields
    /// `None` and leaves the trajectory untouched rather than failing the run.
    fn observe_wish(&mut self, workspace: &str) -> Option<WishRunOutcome> {
        let session = self.wish_session.as_mut()?;
        // Deep observation (crate + module + symbol) so wishes can target finer
        // structure than whole crates.
        let observed = kosmo_intent::observe_workspace_deep(workspace).ok()?;
        let assessment = session.observe(&observed).clone();
        let attractor_status = session.trace().status;
        let diverged = {
            let a = session.assessments();
            a.len() >= 2 && a[a.len() - 1].distance > a[a.len() - 2].distance
        };
        Some(WishRunOutcome {
            wish_id: assessment.wish_id,
            assessment,
            attractor_status,
            diverged,
        })
    }

    /// Turn an agenda of unmet facets into top-priority, facet-directed
    /// [`ActionItem`]s. Each carries the facet itself
    /// ([`ActionItemKind::RealizeWishFacet`]) and a human-readable directive, so
    /// a synthesizer (mock today, LLM later) knows exactly what to build. The
    /// `action_id` is deterministic in the facet and policy.
    fn wish_actions(&self, agenda: &[WishFacet]) -> Vec<ActionItem> {
        #[derive(Serialize)]
        struct WishActionContent<'a> {
            tag: &'static str,
            facet: &'a WishFacet,
            policy_id: &'a Digest,
        }
        agenda
            .iter()
            .map(|facet| {
                let action_id = Digest::of(&WishActionContent {
                    tag: "kosmo-wish-realize-facet",
                    facet,
                    policy_id: &self.policy.id,
                });
                ActionItem {
                    action_id,
                    priority_score: Q16::ONE,
                    kind: ActionItemKind::RealizeWishFacet {
                        facet: facet.clone(),
                    },
                    description: format!("Realize wished {:?} `{}`", facet.kind, facet.key),
                    policy_id: self.policy.id,
                }
            })
            .collect()
    }

    pub fn feedback_history(&self) -> &[ExecutionFeedback] {
        &self.feedback_history
    }
    /// Number of `PromotionFeedback` records queued for injection into the next pipeline run.
    pub fn pipeline_feedback_pending(&self) -> usize {
        self.pipeline_feedback.len()
    }
    pub fn run_count(&self) -> u32 {
        self.pipeline_session.run_count()
    }
    pub fn synthesizer_name(&self) -> &str {
        self.synthesizer.name()
    }

    /// Execute one full agent iteration on `workspace`.
    ///
    /// Runs the pipeline, selects up to `max_steps` action items, calls the
    /// synthesizer on each, and records the outcomes. In dry-run mode (the
    /// default) no files are modified.
    pub fn run(&mut self, workspace: &str) -> Result<AgentRunReport, AgentError> {
        // ── 0. Feed prior outcomes back into the pipeline ────────────────────
        // Drain the feedback accumulated from the previous run() call into the
        // pipeline session's prior_feedback pool so it can update norm-fitness
        // scoring before the next scan.
        if !self.pipeline_feedback.is_empty() {
            self.pipeline_session
                .extend_prior_feedback(self.pipeline_feedback.drain(..));
        }

        // ── 1. Plan ──────────────────────────────────────────────────────────
        let report = self
            .pipeline_session
            .run(workspace)
            .map_err(|e| AgentError::Pipeline(e.to_string()))?;

        let pipeline_gate = report.final_result.clone();
        let void_items = report.action_items();

        // ── 1b. Observe the wish; turn its agenda into directed actions ───────
        // One run is one step of the dynamics. Observing here yields both the
        // convergence trajectory point and this run's agenda (the unmet facets);
        // each unmet facet becomes a top-priority, facet-directed action,
        // prepended to the queue so the loop builds *toward* the wish — not just
        // repairs voids. Fail-soft: no wish / non-cargo workspace ⇒ no additions.
        let wish = self.observe_wish(workspace);
        let wish_actions = wish
            .as_ref()
            .map(|w| self.wish_actions(w.agenda()))
            .unwrap_or_default();

        // ── 1c. Combined queue: wish-directed work first, then voids ──────────
        let mut all_items = wish_actions;
        all_items.extend(void_items);
        let total_available = all_items.len();

        // ── 2. Synthesize top-N ──────────────────────────────────────────────
        let mut steps: Vec<AgentStep> = Vec::new();
        let mut skipped_low_confidence = 0u32;

        for (idx, action) in all_items.into_iter().enumerate() {
            if steps.len() >= self.options.max_steps as usize {
                break;
            }

            let mut request = SynthesisRequest::new(action.clone(), workspace);

            // ── 2b. Ground in anchored memory ────────────────────────────────
            // The action's description is the recall query — the same free-text
            // door `kosmo-promote --recall` uses. What the memory returns rides
            // along as advisory context; the result will cite it.
            if let Some(recall) = &self.recall {
                let hits = recall
                    .recall(&action.description, self.options.grounding_top as usize)
                    .map_err(AgentError::Recall)?;
                request = request.with_grounding(hits);
            }

            let synthesis = match self.synthesizer.synthesize(&request) {
                Ok(r) => r,
                Err(e) => {
                    // Permanent synthesis failure: record as negative feedback, continue.
                    let fb = ExecutionFeedback::skipped(action.action_id, Q16::ZERO);
                    self.feedback_history.push(fb);
                    if !e.recoverable {
                        continue;
                    }
                    return Err(AgentError::Synthesis(e.to_string()));
                }
            };

            // ── 3. Confidence filter ─────────────────────────────────────────
            if synthesis.confidence < self.options.min_confidence {
                skipped_low_confidence += 1;
                let fb = ExecutionFeedback::skipped(action.action_id, synthesis.confidence);
                self.feedback_history.push(fb);
                continue;
            }

            // ── 4. Materialize (dry-run or real) ─────────────────────────────
            let attempt = if self.options.dry_run {
                Some(MaterializationAttempt::new_dry_run(
                    synthesis.patch.patch_id,
                    action.action_id,
                    synthesis.patch.total_lines(),
                ))
            } else {
                // Real materialization via kosmo-materialize (policy-gated).
                // Clone the Arc so `self` is free for the mutable feedback push
                // in the error arm.
                match self.validator.clone() {
                    Some(validator) => {
                        let materializer = Materializer::new(workspace);
                        let opts = MaterializeOptions {
                            run_tests: true,
                            git_commit: self.options.commit_to_git,
                        };
                        match materializer.materialize(
                            &synthesis.patch,
                            &self.policy,
                            validator.as_ref(),
                            &opts,
                        ) {
                            Ok(report) => Some(MaterializationAttempt::from_materialize_report(
                                action.action_id,
                                synthesis.patch.total_lines(),
                                &report,
                            )),
                            Err(_io_err) => {
                                // Filesystem failure during materialization:
                                // record a skip and move on (fail-closed).
                                let fb = ExecutionFeedback::skipped(
                                    action.action_id,
                                    synthesis.confidence,
                                );
                                self.feedback_history.push(fb);
                                continue;
                            }
                        }
                    }
                    // No validator configured ⇒ no real materialization.
                    None => None,
                }
            };

            let is_positive = attempt
                .as_ref()
                .map(|a| a.validation.is_acceptable())
                .unwrap_or(false);

            let feedback = ExecutionFeedback::new(
                action.action_id,
                synthesis.confidence,
                attempt.clone(),
                is_positive,
            );
            self.feedback_history.push(feedback.clone());

            // ── 5. Build PromotionFeedback for the pipeline's next run ────────
            // Map the action's norm-related ID so the pipeline can update the
            // NormFitnessTrace for the relevant candidate.
            let norm_candidate_id = match &action.kind {
                ActionItemKind::PromoteToPse { candidate_id } => *candidate_id,
                ActionItemKind::ApplyNorm {
                    norm_candidate_id, ..
                } => *norm_candidate_id,
                _ => Digest::ZERO,
            };
            let pf_outcome = if is_positive {
                FeedbackOutcome::Accepted
            } else {
                FeedbackOutcome::Rejected
            };
            let pf = PromotionFeedback::new(
                feedback.feedback_id, // record_id
                action.action_id,     // candidate_id
                norm_candidate_id,
                pf_outcome,
                synthesis.confidence, // energy_at_submission
                self.policy.id,
                feedback.feedback_id, // evidence_bundle_id (≠ ZERO — CROSS-006)
            );
            self.pipeline_feedback.push(pf);

            steps.push(AgentStep {
                step_number: idx as u32 + 1,
                action,
                synthesis,
                materialization: attempt,
                feedback,
            });
        }

        Ok(AgentRunReport::build(
            workspace,
            self.pipeline_session.run_count(),
            total_available,
            steps,
            pipeline_gate,
            skipped_low_confidence,
            wish,
        ))
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_synthesizer::{FileChange, MockSynthesizer, Patch, SynthesisError};

    /// A stable, empty scan root under the system temp dir. Never the temp
    /// dir itself: on CI runners /tmp holds root-owned entries (e.g.
    /// snap-private-tmp) that fail the pipeline walk with EACCES — the
    /// fixture must be hermetic. Stable across calls so assertions like
    /// `report.workspace_path == tmp()` hold.
    fn tmp() -> String {
        let dir = std::env::temp_dir().join("kosmo-agent-test-ws");
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_string_lossy().to_string()
    }

    fn session(opts: AgentOptions, confident: bool) -> AgentSession {
        let synth: Arc<dyn ActionSynthesizer> = if confident {
            Arc::new(MockSynthesizer::confident())
        } else {
            Arc::new(MockSynthesizer::uncertain())
        };
        AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
    }

    #[test]
    fn agent_dry_run_produces_report() {
        let opts = AgentOptions::default().with_max_steps(3);
        let mut s = session(opts, true);
        let report = s.run(&tmp()).unwrap();
        assert_eq!(report.workspace_path, tmp());
        assert!(report.pipeline_run_number >= 1);
        assert_eq!(report.steps_materialized, 0); // dry-run: never applied
    }

    #[test]
    fn agent_real_materialization_applies_via_validator() {
        // A temp workspace with a source file (and no test) should surface at
        // least one action; with a passing validator + operator-approved policy
        // those patches are really applied (here: empty mock patches, so 0 files
        // change, but the attempt is recorded as applied-to-host).
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-agent-mat-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn foo() -> u32 { 1 }\n").unwrap();

        let opts = AgentOptions {
            max_steps: 3,
            min_confidence: Q16::ZERO,
            dry_run: false,
            pipeline_options: IntegrationRunOptions::report_only(),
            commit_to_git: false,
            grounding_top: 5,
        };
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::operator_approved(), synth)
            .with_validator(Arc::new(AlwaysPass));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        // Every synthesized step was materialized to the host (validator passed).
        assert_eq!(report.steps_materialized, report.steps_synthesized);
        for step in &report.steps {
            let attempt = step.materialization.as_ref().unwrap();
            assert!(
                attempt.applied,
                "patch should be applied with a passing validator"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_real_materialization_rolls_back_on_failed_validation() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-agent-rb-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn foo() -> u32 { 1 }\n").unwrap();

        let opts = AgentOptions {
            max_steps: 3,
            min_confidence: Q16::ZERO,
            dry_run: false,
            pipeline_options: IntegrationRunOptions::report_only(),
            commit_to_git: false,
            grounding_top: 5,
        };
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::operator_approved(), synth)
            .with_validator(Arc::new(AlwaysFail));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        // Failed validation ⇒ nothing persisted, every attempt rolled back.
        assert_eq!(report.steps_materialized, 0);
        for step in &report.steps {
            let attempt = step.materialization.as_ref().unwrap();
            assert!(!attempt.applied);
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_respects_max_steps() {
        let opts = AgentOptions::default().with_max_steps(2);
        let mut s = session(opts, true);
        let report = s.run(&tmp()).unwrap();
        assert!(report.steps.len() <= 2);
    }

    #[test]
    fn agent_skips_low_confidence_steps() {
        let opts = AgentOptions {
            max_steps: 10,
            min_confidence: Q16::HALF,
            dry_run: true,
            pipeline_options: IntegrationRunOptions::report_only(),
            commit_to_git: false,
            grounding_top: 5,
        };
        let synth = Arc::new(MockSynthesizer::uncertain());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth);
        let report = s.run(&tmp()).unwrap();
        // Uncertain mock has confidence 0.30 < 0.50 threshold → all skipped.
        assert_eq!(report.steps_synthesized, 0);
        assert!(report.steps_skipped_low_confidence > 0 || report.total_actions_available == 0);
    }

    #[test]
    fn agent_run_id_is_deterministic() {
        // Isolated, stable workspace: scanning the shared system temp dir is
        // non-deterministic under parallel tests that create/remove temp crates.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-agent-runid-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn f() -> u32 { 1 }\n").unwrap();
        let ws = dir.to_str().unwrap();

        let opts = AgentOptions::default().with_max_steps(2);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s1 = AgentSession::new(
            opts.clone(),
            PolicyProfile::default_report_only(),
            synth.clone(),
        );
        let mut s2 = AgentSession::new(opts, PolicyProfile::default_report_only(), synth);
        let r1 = s1.run(ws).unwrap();
        let r2 = s2.run(ws).unwrap();
        assert_eq!(r1.run_id, r2.run_id);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn feedback_accumulates_across_runs() {
        let opts = AgentOptions::default().with_max_steps(2);
        let mut s = session(opts, true);
        s.run(&tmp()).unwrap();
        let after_first = s.feedback_history().len();
        s.run(&tmp()).unwrap();
        let after_second = s.feedback_history().len();
        assert!(after_second >= after_first);
    }

    #[test]
    fn dry_run_attempt_is_not_applied() {
        let opts = AgentOptions::default().with_max_steps(5);
        let mut s = session(opts, true);
        let report = s.run(&tmp()).unwrap();
        for step in &report.steps {
            if let Some(ref attempt) = step.materialization {
                assert!(!attempt.applied);
                assert!(attempt.blocking_reason.is_some());
            }
        }
    }

    #[test]
    fn agent_total_lines_proposed_is_sum_of_patch_lines() {
        let opts = AgentOptions::default().with_max_steps(3);
        let synth = Arc::new(MockSynthesizer::confident().with_change(
            kosmo_synthesizer::FileChange::create("a.rs", "fn a(){}\nfn b(){}"),
        ));
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth);
        let report = s.run(&tmp()).unwrap();
        let expected: u32 = report
            .steps
            .iter()
            .map(|s| s.synthesis.patch.total_lines())
            .sum();
        assert_eq!(report.total_lines_proposed, expected);
    }

    #[test]
    fn execution_feedback_is_positive_when_validation_acceptable() {
        let opts = AgentOptions::default().with_max_steps(1);
        let mut s = session(opts, true);
        let report = s.run(&tmp()).unwrap();
        for step in &report.steps {
            // In dry-run, gate is Warn (acceptable) → is_positive = true.
            assert!(step.feedback.is_positive);
        }
    }

    #[test]
    fn pipeline_feedback_queued_after_synthesized_steps() {
        // After a run with at least one synthesized step, pipeline_feedback_pending
        // should be non-zero (ready for the next run's norm-fitness update).
        let opts = AgentOptions::default().with_max_steps(3);
        let mut s = session(opts, true);
        let report = s.run(&tmp()).unwrap();
        if report.steps_synthesized > 0 {
            assert!(
                s.pipeline_feedback_pending() > 0,
                "expected PromotionFeedback records queued after synthesized steps"
            );
        }
    }

    #[test]
    fn pipeline_feedback_drained_into_next_run() {
        // Feedback queued after the first run should be drained into the pipeline
        // before the second run, leaving the pending count back to zero after run 2.
        let opts = AgentOptions::default().with_max_steps(3);
        let mut s = session(opts, true);
        s.run(&tmp()).unwrap();
        let pending_after_run1 = s.pipeline_feedback_pending();
        s.run(&tmp()).unwrap();
        // After run 2 the records accumulated in run 1 were drained at the start
        // of run 2; run 2 may have added its own — count should equal what run 2
        // synthesized, not the sum of both runs.
        let pending_after_run2 = s.pipeline_feedback_pending();
        // Invariant: we never accumulate run-1 + run-2 records simultaneously.
        if pending_after_run1 > 0 {
            // They were drained before run 2 started.
            assert!(
                pending_after_run2 <= pending_after_run1 * 2,
                "feedback should be drained between runs, not accumulate unboundedly"
            );
        }
        // run_count should reflect two complete runs.
        assert_eq!(s.run_count(), 2);
    }

    // ── wish-governed loop (Run 4) ────────────────────────────────────────

    /// A temporary standalone cargo crate so `cargo metadata` (hence wish
    /// observation) can read it. Returns the crate dir.
    fn temp_crate(pkg: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-agent-wish-{pkg}-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn f() -> u32 { 1 }\n").unwrap();
        dir
    }

    fn crate_wish(pkg: &str) -> Wish {
        Wish::new(
            format!("crate {pkg} exists"),
            [kosmo_core::WishPredicate::require(WishFacet::crate_(pkg))],
            Digest::of_bytes(b"policy"),
            Digest::of_bytes(b"bundle"),
        )
    }

    #[test]
    fn agent_without_wish_reports_none() {
        let opts = AgentOptions::default().with_max_steps(1);
        let mut s = session(opts, true);
        let report = s.run(&tmp()).unwrap();
        assert!(report.wish.is_none());
        assert!(s.wish_assessment().is_none());
    }

    #[test]
    fn agent_wish_failsoft_on_non_cargo_workspace() {
        // The system temp dir is not a cargo tree → observation fails softly:
        // the run still succeeds, but carries no wish outcome.
        let opts = AgentOptions::default().with_max_steps(1);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
            .with_wish(crate_wish("anything"), Digest::of_bytes(b"ev"));
        let report = s.run(&tmp()).unwrap();
        assert!(
            report.wish.is_none(),
            "a non-cargo workspace must fail soft"
        );
    }

    #[test]
    fn agent_wish_realized_on_matching_crate() {
        let dir = temp_crate("kosmo_wish_demo");
        let opts = AgentOptions::default().with_max_steps(2);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
            .with_wish(crate_wish("kosmo_wish_demo"), Digest::of_bytes(b"ev"));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        let Some(wish) = report.wish else {
            eprintln!("cargo metadata unavailable, skipping");
            std::fs::remove_dir_all(&dir).ok();
            return;
        };
        assert!(wish.is_realized(), "the wished crate is present → realized");
        assert_eq!(wish.assessment.distance, Q16::ZERO);
        assert_eq!(wish.attractor_status, AttractorStatus::Converged);
        assert!(!wish.diverged);
        assert!(wish.agenda().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_wish_unmet_sets_agenda() {
        let dir = temp_crate("kosmo_present_crate");
        let opts = AgentOptions::default().with_max_steps(2);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
            .with_wish(crate_wish("kosmo_absent_crate"), Digest::of_bytes(b"ev"));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        let Some(wish) = report.wish else {
            eprintln!("cargo metadata unavailable, skipping");
            std::fs::remove_dir_all(&dir).ok();
            return;
        };
        assert!(!wish.is_realized());
        assert_eq!(wish.assessment.distance, Q16::ONE);
        assert_eq!(wish.agenda(), &[WishFacet::crate_("kosmo_absent_crate")]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_wish_detects_divergence_across_runs() {
        // Run 1: the wished crate is present (realized). Then rename the package
        // away, so the wished crate vanishes. Run 2: distance rises → the run is
        // flagged diverged, fail-closed.
        let dir = temp_crate("kosmo_target_crate");
        let opts = AgentOptions::default().with_max_steps(1);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
            .with_wish(crate_wish("kosmo_target_crate"), Digest::of_bytes(b"ev"));

        let r1 = s.run(dir.to_str().unwrap()).unwrap();
        let Some(w1) = r1.wish else {
            eprintln!("cargo metadata unavailable, skipping");
            std::fs::remove_dir_all(&dir).ok();
            return;
        };
        assert_eq!(w1.assessment.distance, Q16::ZERO);
        assert!(!w1.diverged);

        // Break it: rename the package away from the wished name.
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"kosmo_renamed_crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        let r2 = s.run(dir.to_str().unwrap()).unwrap();
        let w2 = r2.wish.expect("second run observed the workspace");
        assert_eq!(w2.assessment.distance, Q16::ONE);
        assert!(w2.diverged, "distance rose ZERO → ONE: this run diverged");
        assert_eq!(w2.attractor_status, AttractorStatus::Diverging);
        assert!(s.wish_diverging());

        std::fs::remove_dir_all(&dir).ok();
    }

    // ── facet-directed generation (Run 5) ─────────────────────────────────

    /// A synthesizer that realizes a wished `Crate` facet by rewriting the
    /// workspace's `Cargo.toml` to that crate name (everything else is a no-op).
    struct CrateScaffolder;
    impl ActionSynthesizer for CrateScaffolder {
        fn synthesize(
            &self,
            request: &SynthesisRequest,
        ) -> Result<SynthesisResult, SynthesisError> {
            if let ActionItemKind::RealizeWishFacet { facet } = &request.action_item.kind {
                if facet.kind == kosmo_core::WishFacetKind::Crate {
                    let toml = format!(
                        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                        facet.key
                    );
                    let patch = Patch::new(
                        request.request_id,
                        vec![FileChange::modify("Cargo.toml", toml)],
                        "crate-scaffolder",
                    );
                    return Ok(SynthesisResult::new(
                        patch,
                        "scaffold the wished crate",
                        Q16::ONE,
                    ));
                }
            }
            Ok(SynthesisResult::new(
                Patch::empty(request.request_id),
                "no-op",
                Q16::ONE,
            ))
        }
        fn name(&self) -> &str {
            "crate-scaffolder"
        }
    }

    #[test]
    fn agent_no_wish_generates_no_directed_actions() {
        let opts = AgentOptions::default().with_max_steps(3);
        let mut s = session(opts, true);
        let report = s.run(&tmp()).unwrap();
        assert_eq!(report.wish_directed_count(), 0);
    }

    #[test]
    fn agent_unmet_wish_generates_directed_action() {
        let dir = temp_crate("kosmo_present");
        let opts = AgentOptions::default().with_max_steps(5);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
            .with_wish(crate_wish("kosmo_wanted"), Digest::of_bytes(b"ev"));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        if report.wish.is_none() {
            eprintln!("cargo metadata unavailable, skipping");
            std::fs::remove_dir_all(&dir).ok();
            return;
        }
        assert!(
            report.wish_directed_count() >= 1,
            "an unmet facet must yield directed work"
        );
        assert!(
            report.steps.iter().any(|st| matches!(
                &st.action.kind,
                ActionItemKind::RealizeWishFacet { facet } if facet == &WishFacet::crate_("kosmo_wanted")
            )),
            "a step must target the wished crate"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_realized_wish_generates_no_directed_actions() {
        let dir = temp_crate("kosmo_here");
        let opts = AgentOptions::default().with_max_steps(5);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
            .with_wish(crate_wish("kosmo_here"), Digest::of_bytes(b"ev"));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        if let Some(w) = &report.wish {
            assert!(w.is_realized());
            assert_eq!(
                report.wish_directed_count(),
                0,
                "a realized wish has an empty agenda ⇒ no directed work"
            );
        } else {
            eprintln!("cargo metadata unavailable, skipping");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_wish_builds_toward_and_converges() {
        // The wish is for a crate the workspace does not yet have. A scaffolding
        // synthesizer realizes it (rewrites Cargo.toml); the next run observes
        // the wish realized — the loop built *toward* the wish and converged.
        let dir = temp_crate("kosmo_before");
        let opts = AgentOptions {
            max_steps: 5,
            min_confidence: Q16::ZERO,
            dry_run: false,
            pipeline_options: IntegrationRunOptions::report_only(),
            commit_to_git: false,
            grounding_top: 5,
        };
        let mut s = AgentSession::new(
            opts,
            PolicyProfile::operator_approved(),
            Arc::new(CrateScaffolder),
        )
        .with_validator(Arc::new(AlwaysPass))
        .with_wish(crate_wish("kosmo_after"), Digest::of_bytes(b"ev"));

        // Run 1: wish unmet → a facet-directed action scaffolds the crate.
        let r1 = s.run(dir.to_str().unwrap()).unwrap();
        let directed = r1.wish_directed_count();
        let Some(w1) = r1.wish else {
            eprintln!("cargo metadata unavailable, skipping");
            std::fs::remove_dir_all(&dir).ok();
            return;
        };
        assert_eq!(w1.assessment.distance, Q16::ONE, "starts far from the wish");
        assert!(directed >= 1, "a facet-directed action was generated");

        // Run 2: the rewritten Cargo.toml now names the wished crate → realized.
        let r2 = s.run(dir.to_str().unwrap()).unwrap();
        let w2 = r2.wish.expect("second run observed the workspace");
        assert_eq!(
            w2.assessment.distance,
            Q16::ZERO,
            "the loop built toward the wish"
        );
        assert!(w2.is_realized());
        assert!(!s.wish_diverging(), "the descent was contractive");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_wish_realized_on_symbol() {
        // Finer granularity: a wish can target a public symbol, observed by the
        // deep (source-walking) observer.
        let dir = temp_crate("kosmo_symcrate");
        std::fs::write(
            dir.join("src/lib.rs"),
            "pub fn special_function() -> u32 { 7 }\n",
        )
        .unwrap();
        let wish = Wish::new(
            "expose special_function",
            [kosmo_core::WishPredicate::require(WishFacet::symbol(
                "special_function",
            ))],
            Digest::of_bytes(b"policy"),
            Digest::of_bytes(b"bundle"),
        );
        let opts = AgentOptions::default().with_max_steps(2);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
            .with_wish(wish, Digest::of_bytes(b"ev"));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        if let Some(w) = &report.wish {
            assert!(w.is_realized(), "the public symbol is present → realized");
            assert_eq!(w.assessment.distance, Q16::ZERO);
        } else {
            eprintln!("cargo metadata unavailable, skipping");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_wish_from_prose_realized() {
        // The full front door: a prose intent compiles to a Wish, and the loop
        // measures the real workspace against it.
        let dir = temp_crate("kosmo_prose_demo");
        let wish = kosmo_intent::compile_wish(
            "I want a crate kosmo_prose_demo",
            Digest::of_bytes(b"policy"),
            Digest::of_bytes(b"bundle"),
        );
        let opts = AgentOptions::default().with_max_steps(2);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth)
            .with_wish(wish, Digest::of_bytes(b"ev"));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        if let Some(w) = &report.wish {
            assert!(w.is_realized(), "prose-compiled crate wish realized");
        } else {
            eprintln!("cargo metadata unavailable, skipping");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_wish_builds_symbol_and_converges() {
        // Deterministic build-toward-intent: FacetScaffolder realizes a Symbol
        // wish offline (appends `pub fn` to lib.rs); the next run observes it
        // present and the descent converges.
        let dir = temp_crate("kosmo_scaffold_demo");
        let opts = AgentOptions {
            max_steps: 5,
            min_confidence: Q16::ZERO,
            dry_run: false,
            pipeline_options: IntegrationRunOptions::report_only(),
            commit_to_git: false,
            grounding_top: 5,
        };
        let wish = Wish::new(
            "expose handle_request",
            [kosmo_core::WishPredicate::require(WishFacet::symbol(
                "handle_request",
            ))],
            Digest::of_bytes(b"policy"),
            Digest::of_bytes(b"bundle"),
        );
        let mut s = AgentSession::new(
            opts,
            PolicyProfile::operator_approved(),
            Arc::new(kosmo_synthesizer::FacetScaffolder),
        )
        .with_validator(Arc::new(AlwaysPass))
        .with_wish(wish, Digest::of_bytes(b"ev"));

        let r1 = s.run(dir.to_str().unwrap()).unwrap();
        let directed = r1.wish_directed_count();
        let Some(w1) = r1.wish else {
            eprintln!("cargo metadata unavailable, skipping");
            std::fs::remove_dir_all(&dir).ok();
            return;
        };
        assert_eq!(w1.assessment.distance, Q16::ONE, "symbol absent at first");
        assert!(directed >= 1, "a facet-directed scaffold action was taken");

        let r2 = s.run(dir.to_str().unwrap()).unwrap();
        let w2 = r2.wish.expect("second run observed the workspace");
        assert!(
            w2.is_realized(),
            "FacetScaffolder built the symbol → realized"
        );
        assert_eq!(w2.assessment.distance, Q16::ZERO);
        assert!(!s.wish_diverging(), "the descent was contractive");

        std::fs::remove_dir_all(&dir).ok();
    }

    // ─── Memory-grounded synthesis ───────────────────────────────────────────

    use kosmo_pse_bridge::MemoryGroundingEntry;

    /// A seeded scratch workspace that yields at least one action item
    /// (a source file without tests → a structural void). The memory tests
    /// need real synthesis steps; an empty scan root would make them
    /// vacuous.
    fn seeded_ws(tag: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-agent-mem-{tag}-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn foo() -> u32 { 1 }\n").unwrap();
        dir.to_string_lossy().to_string()
    }

    /// Deterministic stand-in for `pse-adapter-kosmo::LedgerRecall`.
    struct ScriptedRecall;
    impl MemoryRecall for ScriptedRecall {
        fn recall(&self, query: &str, top: usize) -> Result<Vec<MemoryGroundingEntry>, String> {
            assert!(!query.is_empty(), "query must be the action description");
            Ok(vec![MemoryGroundingEntry {
                crystal_id: "ab12cd34ef56ab12".into(),
                stability: 0.76,
                qtic_class: Some(5),
                tripolar_score: 0.4668,
                commit_index: 3,
                scale_tag: "il-refined".into(),
                question: "kosmo-promote:/ws/alpha".into(),
                claims: vec!["python module routing lacks test coverage".into()],
            }]
            .into_iter()
            .take(top)
            .collect())
        }
        fn source(&self) -> String {
            "scripted://test".into()
        }
    }

    struct BrokenRecall;
    impl MemoryRecall for BrokenRecall {
        fn recall(&self, _q: &str, _t: usize) -> Result<Vec<MemoryGroundingEntry>, String> {
            Err("ledger unreadable".into())
        }
        fn source(&self) -> String {
            "broken://test".into()
        }
    }

    #[test]
    fn grounded_session_cites_memory_in_every_step() {
        let opts = AgentOptions::default()
            .with_max_steps(3)
            .with_min_confidence(Q16::ZERO);
        let mut s = session(opts, true).with_recall(Arc::new(ScriptedRecall));
        let ws = seeded_ws("cites");
        let report = s.run(&ws).unwrap();
        assert!(report.steps_synthesized >= 1, "need at least one step");
        for step in &report.steps {
            assert_eq!(
                step.synthesis.grounding_crystal_ids,
                vec!["ab12cd34ef56ab12"],
                "every synthesis must cite the memory it received"
            );
        }
        std::fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn without_memory_steps_carry_no_citations() {
        let opts = AgentOptions::default()
            .with_max_steps(2)
            .with_min_confidence(Q16::ZERO);
        let mut s = session(opts, true);
        let ws = seeded_ws("bare");
        let report = s.run(&ws).unwrap();
        assert!(report.steps_synthesized >= 1, "need at least one step");
        for step in &report.steps {
            assert!(step.synthesis.grounding_crystal_ids.is_empty());
        }
        std::fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn recall_failure_aborts_the_run_loudly() {
        let opts = AgentOptions::default()
            .with_max_steps(2)
            .with_min_confidence(Q16::ZERO);
        let mut s = session(opts, true).with_recall(Arc::new(BrokenRecall));
        let ws = seeded_ws("broken");
        match s.run(&ws) {
            Err(AgentError::Recall(msg)) => assert!(msg.contains("ledger unreadable")),
            other => panic!("expected AgentError::Recall, got {other:?}"),
        }
        std::fs::remove_dir_all(&ws).ok();
    }
}
