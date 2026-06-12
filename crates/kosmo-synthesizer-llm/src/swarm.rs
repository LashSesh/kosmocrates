//! Swarm synthesis — assimilated from Wonderlamp's Merkaba orbit core
//! (`SwarmOracle`), delivered fail-closed on the epistemic substrate.
//!
//! One request fans out into `n` oracle calls, each viewing the SAME user
//! prompt through a different quality lens (Chameleon). The candidates are
//! scored by the integer consensus core
//! ([`kosmo_synthesizer::consensus`]); when
//! the ensemble diverges, up to [`ConsensusConfig::max_repair_rounds`]
//! Coagula rounds ask the best candidate to complete itself with the
//! functions its peers agree on.
//!
//! **The transformation that matters:** Wonderlamp emitted the best
//! candidate even when consensus failed ("graceful degradation"). Here the
//! swarm answers *honestly* instead: the final confidence is
//! `min(best.confidence, d_total)` — the weaker of self-belief and swarm
//! agreement. A divergent ensemble therefore lands below the agent's
//! `min_confidence` filter and is skipped by the **existing** policy gate;
//! no new gate, no float, CROSS-007/010 untouched. An ensemble in which no
//! candidate even parses is a hard [`SynthesisError::permanent`].
//!
//! The lenses are quality dimensions only — never stack or architecture
//! prescriptions — and their wording is pinned verbatim by tests so drift
//! cannot creep in.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use kosmo_core::Q16;
use kosmo_synthesizer::consensus::{assess_candidates, ConsensusConfig, ConsensusReport};
use kosmo_synthesizer::{ActionSynthesizer, SynthesisError, SynthesisRequest, SynthesisResult};

use crate::{build_user_prompt, parse_synthesis_response, system_prompt, LlmSynthesizer};

/// One chat turn against a backing model: `(system, user) → (text, tokens)`.
///
/// [`LlmSynthesizer`] implements this by delegating to its existing
/// transport (backoff included); [`ScriptedOracle`] replays canned
/// responses for hermetic offline tests.
pub trait ChatOracle: Send + Sync {
    fn complete(&self, system: &str, user: &str) -> Result<(String, u32), SynthesisError>;
    fn label(&self) -> &str;
}

impl ChatOracle for LlmSynthesizer {
    fn complete(&self, system: &str, user: &str) -> Result<(String, u32), SynthesisError> {
        self.call(system, user)
    }

    fn label(&self) -> &str {
        self.name()
    }
}

/// Deterministic test oracle: replays a script of responses in order.
/// `Err` entries simulate transport failures. An exhausted script is a
/// permanent error — a test that under-provisions its script should fail
/// loudly, not hang on improvised answers.
pub struct ScriptedOracle {
    script: Mutex<VecDeque<Result<String, String>>>,
    /// Number of `complete` calls served so far (telemetry for tests).
    calls: Mutex<u32>,
}

impl ScriptedOracle {
    pub fn new(responses: impl IntoIterator<Item = Result<String, String>>) -> Self {
        Self {
            script: Mutex::new(responses.into_iter().collect()),
            calls: Mutex::new(0),
        }
    }

    /// Convenience: a script of plain successful responses.
    pub fn replying(responses: impl IntoIterator<Item = String>) -> Self {
        Self::new(responses.into_iter().map(Ok))
    }

    pub fn calls_served(&self) -> u32 {
        *self.calls.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl ChatOracle for ScriptedOracle {
    fn complete(&self, _system: &str, _user: &str) -> Result<(String, u32), SynthesisError> {
        let mut calls = self.calls.lock().unwrap_or_else(|e| e.into_inner());
        *calls += 1;
        let mut script = self.script.lock().unwrap_or_else(|e| e.into_inner());
        match script.pop_front() {
            Some(Ok(text)) => {
                let tokens = (text.len() / 4) as u32;
                Ok((text, tokens))
            }
            Some(Err(e)) => Err(SynthesisError::transient(e)),
            None => Err(SynthesisError::permanent("scripted oracle exhausted")),
        }
    }

    fn label(&self) -> &str {
        "scripted"
    }
}

/// The four Chameleon lenses: quality dimensions appended to the system
/// prompt, cycling when `n > 4`. Wording is pinned verbatim by tests —
/// these must stay free of stack, framework, and architecture
/// prescriptions.
pub const LENSES: [&str; 4] = [
    "Focus on correctness: every name you reference must resolve against \
     the provided context; do not invent fields, functions, or modules.",
    "Focus on completeness: resolve the action fully; no placeholders or \
     deferred work where the action asks for substance.",
    "Focus on robustness: handle failure paths explicitly; do not silently \
     swallow errors the surrounding code would need to see.",
    "Focus on consistency: stay consistent with the naming, style, and \
     structure visible in the provided context.",
];

/// The system prompt for perspective `k` of `n`: the shared contract plus
/// one lens.
pub fn lens_prompt(k: usize, n: usize) -> String {
    format!(
        "{}\n\n## Focus directive (perspective {}/{})\n{}",
        system_prompt(),
        k + 1,
        n,
        LENSES[k % LENSES.len()]
    )
}

/// A swarm of `n` perspectives over one oracle, emitting fail-closed
/// consensus-weighted synthesis results.
pub struct SwarmSynthesizer {
    oracle: Arc<dyn ChatOracle>,
    n: usize,
    config: ConsensusConfig,
    label: String,
}

impl SwarmSynthesizer {
    /// `n` is clamped to `2..=6` — one perspective is no swarm, and beyond
    /// six the variance gain no longer pays for the token cost.
    pub fn new(oracle: Arc<dyn ChatOracle>, n: usize) -> Self {
        let n = n.clamp(2, 6);
        let label = format!("swarm{}:{}", n, oracle.label());
        Self {
            oracle,
            n,
            config: ConsensusConfig::default(),
            label,
        }
    }

    pub fn with_config(mut self, config: ConsensusConfig) -> Self {
        self.config = config;
        self
    }

    /// The Coagula repair prompt: the best candidate completes itself with
    /// the functions its peers agree on. Framed as self-completion — the
    /// repair never imports a peer's body wholesale.
    fn repair_prompt(user: &str, best_raw: &str, targets: &[String]) -> String {
        format!(
            "{user}\n\n## Repair directive\nYour previous answer (JSON below) is \
             missing functions that independent solutions of the same action \
             agree on: {}.\nReturn the COMPLETE corrected JSON patch object — \
             your previous answer plus the missing functions, nothing else.\n\n\
             Previous answer:\n{best_raw}",
            targets.join(", ")
        )
    }
}

/// One parsed candidate with the raw text it came from (kept for repair).
struct Candidate {
    raw: String,
    result: SynthesisResult,
}

impl ActionSynthesizer for SwarmSynthesizer {
    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisResult, SynthesisError> {
        let user = build_user_prompt(request);
        let mut tokens_total: u32 = 0;
        let mut candidates: Vec<Candidate> = Vec::new();
        let mut last_error: Option<SynthesisError> = None;

        for k in 0..self.n {
            let system = lens_prompt(k, self.n);
            match self.oracle.complete(&system, &user) {
                Ok((raw, tokens)) => {
                    tokens_total += tokens;
                    match parse_synthesis_response(&raw, request, &self.label) {
                        Ok(result) => candidates.push(Candidate { raw, result }),
                        Err(e) => last_error = Some(e),
                    }
                }
                Err(e) => last_error = Some(e),
            }
        }

        if candidates.is_empty() {
            return Err(SynthesisError::permanent(format!(
                "swarm: no candidate parsed across {} perspectives ({})",
                self.n,
                last_error.map(|e| e.message).unwrap_or_default()
            )));
        }

        let results: Vec<SynthesisResult> = candidates.iter().map(|c| c.result.clone()).collect();
        let mut report = assess_candidates(&results, &self.config);

        // Coagula: bounded self-completion rounds while divergent and a
        // concrete repair menu exists.
        let mut rounds = 0u32;
        while !report.convergent(&self.config)
            && rounds < self.config.max_repair_rounds
            && !report.repair_targets.is_empty()
        {
            rounds += 1;
            let best = &candidates[report.best_index];
            let prompt = Self::repair_prompt(&user, &best.raw, &report.repair_targets);
            let system = lens_prompt(0, self.n); // correctness lens guards repairs
            match self.oracle.complete(&system, &prompt) {
                Ok((raw, tokens)) => {
                    tokens_total += tokens;
                    match parse_synthesis_response(&raw, request, &self.label) {
                        Ok(result) => {
                            candidates.push(Candidate { raw, result });
                            let results: Vec<SynthesisResult> =
                                candidates.iter().map(|c| c.result.clone()).collect();
                            report = assess_candidates(&results, &self.config);
                        }
                        Err(_) => break, // a non-answer ends repair, honestly
                    }
                }
                Err(_) => break,
            }
        }

        // Honest delivery: the weaker of self-belief and swarm agreement.
        let best = &candidates[report.best_index];
        let confidence = best.result.confidence.min(report.d_total);
        let mut result = best.result.clone();
        result.confidence = confidence;
        result.tokens_used = tokens_total;
        result.rationale = format!(
            "{} | {}",
            consensus_telemetry(&report, candidates.len(), rounds),
            best.result.rationale
        );
        Ok(result.citing(request))
    }

    fn name(&self) -> &str {
        &self.label
    }
}

/// One-line consensus telemetry for rationales and reports.
fn consensus_telemetry(report: &ConsensusReport, candidates: usize, rounds: u32) -> String {
    let best = report.scores.get(report.best_index).copied().unwrap_or(
        kosmo_synthesizer::consensus::CandidateScore {
            psi: Q16::ZERO,
            rho: Q16::ZERO,
            omega: Q16::ZERO,
            d: Q16::ZERO,
        },
    );
    format!(
        "swarm: {} candidate(s), D_total={:.4}, best D={:.4} (psi={:.2} rho={:.2} omega={:.2}), coagula_rounds={}",
        candidates,
        report.d_total.to_f64(),
        best.d.to_f64(),
        best.psi.to_f64(),
        best.rho.to_f64(),
        best.omega.to_f64(),
        rounds
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::PolicyProfile;
    use kosmo_pipeline::{ActionItem, ActionItemKind};
    use kosmo_synthesizer::consensus::ConsensusConfig;

    fn make_request() -> SynthesisRequest {
        let policy = PolicyProfile::default_report_only();
        let item = ActionItem {
            action_id: kosmo_core::Digest::of(&"action-1"),
            priority_score: Q16::ONE,
            kind: ActionItemKind::FillVoid {
                void_id: kosmo_core::Digest::of(&"void-1"),
            },
            description: "missing module foo".into(),
            policy_id: policy.id,
        };
        SynthesisRequest::new(item, "/workspace/demo")
    }

    /// A canned JSON patch response in the wire schema.
    fn patch_json(files: &[(&str, &str)], confidence_pct: u32) -> String {
        serde_json::json!({
            "rationale": "scripted",
            "confidence_pct": confidence_pct,
            "test_hint": null,
            "files": files.iter().map(|(path, content)| {
                serde_json::json!({"path": path, "op": "create", "content": content})
            }).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Rich, convergence-friendly Rust content: 3 fns + 2 types ⇒ the
    /// Ophanim richness rule saturates (ρ = ONE).
    const RICH: &str =
        "pub fn a() {}\npub fn b() {}\npub fn c() {}\npub struct S;\npub struct T;\n";

    #[test]
    fn lenses_are_pinned_verbatim_and_stack_free() {
        assert_eq!(
            LENSES[0],
            "Focus on correctness: every name you reference must resolve against \
             the provided context; do not invent fields, functions, or modules."
        );
        assert_eq!(
            LENSES[1],
            "Focus on completeness: resolve the action fully; no placeholders or \
             deferred work where the action asks for substance."
        );
        assert_eq!(
            LENSES[2],
            "Focus on robustness: handle failure paths explicitly; do not silently \
             swallow errors the surrounding code would need to see."
        );
        assert_eq!(
            LENSES[3],
            "Focus on consistency: stay consistent with the naming, style, and \
             structure visible in the provided context."
        );
        for lens in LENSES {
            let lower = lens.to_lowercase();
            for forbidden in ["actix", "sqlx", "axum", "jwt", "postgres", "rest", "crud"] {
                assert!(!lower.contains(forbidden), "lens prescribes {forbidden}");
            }
        }
    }

    #[test]
    fn lens_prompts_cycle_and_carry_the_contract() {
        let p = lens_prompt(4, 6);
        assert!(p.contains(LENSES[0]), "lens 4 of 6 cycles back to lens 0");
        assert!(p.contains("perspective 5/6"));
        assert!(p.contains("JSON"), "the shared output contract travels");
    }

    #[test]
    fn convergent_swarm_preserves_self_confidence() {
        let oracle =
            ScriptedOracle::replying((0..3).map(|_| patch_json(&[("src/lib.rs", RICH)], 80)));
        let swarm = SwarmSynthesizer::new(Arc::new(oracle), 3);
        let result = swarm.synthesize(&make_request()).unwrap();
        assert_eq!(
            result.confidence,
            Q16::ratio(80, 100).unwrap(),
            "full agreement (D_total = ONE) leaves self-belief untouched: {}",
            result.rationale
        );
        assert!(result.confidence >= Q16::HALF);
        assert!(result.rationale.contains("swarm: 3 candidate(s)"));
    }

    #[test]
    fn divergent_swarm_lands_below_the_confidence_gate() {
        // Three disjoint answers, each loudly self-confident. Wonderlamp
        // would emit one anyway; here the swarm answers honestly and the
        // existing min_confidence filter (HALF) will skip it.
        let oracle = ScriptedOracle::replying([
            patch_json(&[("src/a.rs", "pub fn alpha() {}\npub struct A;\n")], 90),
            patch_json(
                &[(
                    "src/b.rs",
                    "pub fn beta(x: u32) -> u32 { x }\nuse std::fmt;\n",
                )],
                90,
            ),
            patch_json(&[("src/c.py", "def gamma(a, b):\n    return a\n")], 90),
        ]);
        let swarm = SwarmSynthesizer::new(Arc::new(oracle), 3);
        let result = swarm.synthesize(&make_request()).unwrap();
        assert!(
            result.confidence < Q16::HALF,
            "no graceful degradation: divergence must cap confidence, got {:?} ({})",
            result.confidence,
            result.rationale
        );
    }

    #[test]
    fn coagula_repairs_the_best_candidate_within_bounds() {
        // The richest candidate (A: five functions, ρ = ONE) wins D, yet
        // lacks `extra` — which two *loosely similar* peers share. Identical
        // peers would out-ψ the rich one; loose peers keep A on top while
        // still forming a ≥2 quorum for the repair menu.
        let best_missing =
            "pub fn a() {}\npub fn b() {}\npub fn c() {}\npub fn d() {}\npub fn e() {}\n";
        let peer_one = "pub fn a() {}\npub fn b() {}\npub fn c() {}\npub fn extra() {}\n";
        let peer_two = "pub fn a() {}\npub fn b() {}\npub fn d() {}\npub fn extra() {}\n";
        let repaired = "pub fn a() {}\npub fn b() {}\npub fn c() {}\npub fn d() {}\npub fn e() {}\npub fn extra() {}\n";
        let oracle = ScriptedOracle::replying([
            patch_json(&[("src/lib.rs", best_missing)], 70),
            patch_json(&[("src/lib.rs", peer_one)], 70),
            patch_json(&[("src/lib.rs", peer_two)], 70),
            patch_json(&[("src/lib.rs", repaired)], 70), // the repair answer
        ]);
        // Force divergence so Coagula engages: a near-unreachable theta.
        let cfg = ConsensusConfig {
            theta: Q16::ratio(99, 100).unwrap(),
            ..ConsensusConfig::default()
        };
        let swarm = SwarmSynthesizer::new(Arc::new(oracle), 3).with_config(cfg);
        let result = swarm.synthesize(&make_request()).unwrap();
        assert!(
            result.rationale.contains("coagula_rounds=1")
                || result.rationale.contains("coagula_rounds=2"),
            "repair engaged: {}",
            result.rationale
        );
        assert!(result.rationale.contains("candidate(s)"));
    }

    #[test]
    fn all_unparseable_is_a_permanent_error() {
        let oracle =
            ScriptedOracle::replying((0..3).map(|_| "I cannot answer in JSON.".to_string()));
        let swarm = SwarmSynthesizer::new(Arc::new(oracle), 3);
        let err = swarm.synthesize(&make_request()).unwrap_err();
        assert!(!err.recoverable, "no candidate at all is permanent");
        assert!(err.message.contains("no candidate parsed"));
    }

    #[test]
    fn transport_failures_are_tolerated_within_the_ensemble() {
        let oracle = ScriptedOracle::new([
            Err("connection reset".to_string()),
            Ok(patch_json(&[("src/lib.rs", RICH)], 75)),
            Ok(patch_json(&[("src/lib.rs", RICH)], 75)),
        ]);
        let swarm = SwarmSynthesizer::new(Arc::new(oracle), 3);
        let result = swarm.synthesize(&make_request()).unwrap();
        assert!(result.rationale.contains("swarm: 2 candidate(s)"));
        assert!(result.confidence >= Q16::HALF, "{}", result.rationale);
    }

    #[test]
    fn tokens_sum_across_all_served_calls() {
        let r1 = patch_json(&[("src/lib.rs", RICH)], 80);
        let expected = ((r1.len() / 4) * 3) as u32;
        let oracle = ScriptedOracle::replying((0..3).map(|_| r1.clone()));
        let swarm = SwarmSynthesizer::new(Arc::new(oracle), 3);
        let result = swarm.synthesize(&make_request()).unwrap();
        assert_eq!(result.tokens_used, expected);
    }

    #[test]
    fn provenance_citations_survive_the_swarm() {
        let req = make_request().with_grounding(vec![kosmo_pse_bridge::MemoryGroundingEntry {
            crystal_id: "ab12cd34ef56ab12".into(),
            stability: 0.76,
            qtic_class: Some(5),
            tripolar_score: 0.4668,
            commit_index: 3,
            scale_tag: "il-refined".into(),
            question: "kosmo-promote:/ws/alpha".into(),
            claims: vec!["anchored claim".into()],
        }]);
        let oracle =
            ScriptedOracle::replying((0..2).map(|_| patch_json(&[("src/lib.rs", RICH)], 80)));
        let swarm = SwarmSynthesizer::new(Arc::new(oracle), 2);
        let result = swarm.synthesize(&req).unwrap();
        assert_eq!(result.grounding_crystal_ids, vec!["ab12cd34ef56ab12"]);
    }

    #[test]
    fn swarm_size_is_clamped_to_a_sane_band() {
        let mk = |n| {
            SwarmSynthesizer::new(Arc::new(ScriptedOracle::replying([])), n)
                .name()
                .to_string()
        };
        assert!(mk(0).starts_with("swarm2:"), "one perspective is no swarm");
        assert!(mk(99).starts_with("swarm6:"));
        assert!(mk(4).starts_with("swarm4:"));
    }
}
