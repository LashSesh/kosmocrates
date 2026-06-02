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

use kosmo_core::{Digest, GateResult, PolicyProfile, Q16};
use kosmo_materialize::{MaterializeOptions, MaterializeReport, Materializer, PatchValidator};
use kosmo_pipeline::{ActionItem, IntegrationRunOptions, WorkspacePipelineSession};
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
}

impl Default for AgentOptions {
    fn default() -> Self {
        Self {
            max_steps: 5,
            min_confidence: Q16::HALF,
            dry_run: true,
            pipeline_options: IntegrationRunOptions::report_only(),
        }
    }
}

impl AgentOptions {
    pub fn with_max_steps(mut self, n: u32) -> Self { self.max_steps = n; self }
    pub fn with_min_confidence(mut self, c: Q16) -> Self { self.min_confidence = c; self }
    pub fn with_pipeline_options(mut self, o: IntegrationRunOptions) -> Self {
        self.pipeline_options = o;
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
struct AttemptContent { patch_id: Digest, applied: bool }

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
}

impl MaterializationAttempt {
    fn new_dry_run(patch_id: Digest, action_id: Digest, lines_added: u32) -> Self {
        let attempt_id = Digest::of(&AttemptContent { patch_id, applied: false });
        Self {
            attempt_id,
            patch_id,
            action_id,
            applied: false,
            validation: ValidationResult::dry_run(),
            blocking_reason: Some("dry-run mode".into()),
            lines_added,
        }
    }

    /// Build from a real [`MaterializeReport`] produced by `kosmo-materialize`.
    fn from_materialize_report(action_id: Digest, lines_added: u32, report: &MaterializeReport) -> Self {
        let applied = report.applied_to_host;
        let attempt_id = Digest::of(&AttemptContent { patch_id: report.patch_id, applied });
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
        Self { feedback_id, action_id, synthesis_confidence: confidence, materialization, is_positive }
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

#[derive(Serialize)]
struct ReportContent { workspace_hash: Digest, step_ids: Vec<Digest> }

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
}

impl AgentRunReport {
    fn build(
        workspace_path: &str,
        pipeline_run_number: u32,
        total_actions_available: usize,
        steps: Vec<AgentStep>,
        pipeline_gate: GateResult,
        steps_skipped_low_confidence: u32,
    ) -> Self {
        let workspace_hash = Digest::of(&workspace_path.to_string());
        let step_ids: Vec<Digest> = steps.iter().map(|s| s.feedback.feedback_id).collect();
        let run_id = Digest::of(&ReportContent { workspace_hash, step_ids });

        let steps_synthesized = steps.len() as u32;
        let steps_materialized = steps.iter().filter(|s| {
            s.materialization.as_ref().map(|m| m.applied).unwrap_or(false)
        }).count() as u32;
        let total_lines_proposed = steps.iter()
            .map(|s| s.synthesis.patch.total_lines())
            .sum();

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
        }
    }
}

// ─── AgentError ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AgentError {
    Pipeline(String),
    Synthesis(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::Pipeline(s)   => write!(f, "pipeline error: {s}"),
            AgentError::Synthesis(s)  => write!(f, "synthesis error: {s}"),
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
    policy: PolicyProfile,
    /// When set and `options.dry_run == false`, patches are really applied and
    /// validated via `kosmo-materialize` (policy-gated). `None` ⇒ no real
    /// materialization happens even outside dry-run.
    validator: Option<Arc<dyn PatchValidator>>,
}

impl AgentSession {
    pub fn new(
        options: AgentOptions,
        policy: PolicyProfile,
        synthesizer: Arc<dyn ActionSynthesizer>,
    ) -> Self {
        let pipeline_session = WorkspacePipelineSession::new(
            options.pipeline_options.clone(),
            policy.clone(),
        );
        Self {
            pipeline_session,
            synthesizer,
            options,
            feedback_history: vec![],
            policy,
            validator: None,
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

    pub fn feedback_history(&self) -> &[ExecutionFeedback] { &self.feedback_history }
    pub fn run_count(&self) -> u32 { self.pipeline_session.run_count() }
    pub fn synthesizer_name(&self) -> &str { self.synthesizer.name() }

    /// Execute one full agent iteration on `workspace`.
    ///
    /// Runs the pipeline, selects up to `max_steps` action items, calls the
    /// synthesizer on each, and records the outcomes. In dry-run mode (the
    /// default) no files are modified.
    pub fn run(&mut self, workspace: &str) -> Result<AgentRunReport, AgentError> {
        // ── 1. Plan ──────────────────────────────────────────────────────────
        let report = self.pipeline_session.run(workspace)
            .map_err(|e| AgentError::Pipeline(e.to_string()))?;

        let pipeline_gate = report.final_result.clone();
        let all_items = report.action_items();
        let total_available = all_items.len();

        // ── 2. Synthesize top-N ──────────────────────────────────────────────
        let mut steps: Vec<AgentStep> = Vec::new();
        let mut skipped_low_confidence = 0u32;

        for (idx, action) in all_items.into_iter().enumerate() {
            if steps.len() >= self.options.max_steps as usize { break; }

            let request = SynthesisRequest::new(action.clone(), workspace);

            let synthesis = match self.synthesizer.synthesize(&request) {
                Ok(r) => r,
                Err(e) => {
                    // Permanent synthesis failure: record as negative feedback, continue.
                    let fb = ExecutionFeedback::skipped(action.action_id, Q16::ZERO);
                    self.feedback_history.push(fb);
                    if !e.recoverable { continue; }
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
                        let opts = MaterializeOptions::default();
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

            let is_positive = attempt.as_ref()
                .map(|a| a.validation.is_acceptable())
                .unwrap_or(false);

            let feedback = ExecutionFeedback::new(
                action.action_id,
                synthesis.confidence,
                attempt.clone(),
                is_positive,
            );
            self.feedback_history.push(feedback.clone());

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
        ))
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_synthesizer::MockSynthesizer;

    fn tmp() -> String { std::env::temp_dir().to_string_lossy().to_string() }

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
        };
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s = AgentSession::new(opts, PolicyProfile::operator_approved(), synth)
            .with_validator(Arc::new(AlwaysPass));
        let report = s.run(dir.to_str().unwrap()).unwrap();

        // Every synthesized step was materialized to the host (validator passed).
        assert_eq!(report.steps_materialized, report.steps_synthesized);
        for step in &report.steps {
            let attempt = step.materialization.as_ref().unwrap();
            assert!(attempt.applied, "patch should be applied with a passing validator");
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
        let opts = AgentOptions::default().with_max_steps(2);
        let synth = Arc::new(MockSynthesizer::confident());
        let mut s1 = AgentSession::new(
            opts.clone(), PolicyProfile::default_report_only(), synth.clone(),
        );
        let mut s2 = AgentSession::new(
            opts, PolicyProfile::default_report_only(), synth,
        );
        let r1 = s1.run(&tmp()).unwrap();
        let r2 = s2.run(&tmp()).unwrap();
        assert_eq!(r1.run_id, r2.run_id);
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
        let synth = Arc::new(
            MockSynthesizer::confident()
                .with_change(kosmo_synthesizer::FileChange::create("a.rs", "fn a(){}\nfn b(){}"))
        );
        let mut s = AgentSession::new(opts, PolicyProfile::default_report_only(), synth);
        let report = s.run(&tmp()).unwrap();
        let expected: u32 = report.steps.iter().map(|s| s.synthesis.patch.total_lines()).sum();
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
}
