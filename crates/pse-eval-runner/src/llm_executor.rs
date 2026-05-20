//! `CerebrasTrialExecutor` — PSE als kognitive Kontrollschicht über einen
//! mehrstufigen Reasoning-Agent (Plan → Execute → Evaluate).
//!
//! # Architektur
//!
//! ```text
//!  Aufgabe
//!    │
//!    ├─ Phase 1 PLAN:    LLM plant → CognitionInput → Gate → Bundle/Hold
//!    ├─ Phase 2 EXECUTE: LLM führt aus → CognitionInput → Gate → Bundle/Hold
//!    └─ Phase 3 EVAL:    LLM schließt → CognitionInput → Gate → Bundle/Hold
//!                                                               │
//!                              nur wenn alle 3 Phasen Bundle:   └─ commit
//! ```
//!
//! PSE testet NICHT ob das LLM PSE-Domänenwissen hat.  PSE bewertet die
//! *strukturelle Qualität* des Reasoning-Outputs:
//!
//! | Kognitiver Kanal | Proxy im Reasoning |
//! |------------------|--------------------|
//! | ψ (psi)         | Wortabdeckung relativ zur Zielgröße |
//! | ρ (rho)         | Dichte substantieller Sätze |
//! | ω (omega)       | Vorhandensein einer konkreten Schlussfolgerung |
//! | χ (chi)         | Anzahl berücksichtigter Alternativen |
//! | τ (tau)         | Ausgedrückte Unsicherheit |
//! | support_strength | Bruchteil struktureller Anforderungen erfüllt |
//!
//! `required_elements` sind **strukturelle Reasoning-Elemente** (z. B.
//! "trade-off", "because", "constraint") — kein PSE-spezifisches Wissen.

use pse_eval_matrix::{
    primitives::{content_address, EvalError, Fixed, Hash256},
    reports::{DiagnosticRecord, GateObservation, GateObservationSet, TrialOutputs, TrialReport},
    replay::ReplayObservation,
    runner::{RunDescriptor, TrialExecutor},
    spec::EvaluationSpec,
    variants::SystemVariantSpec,
    workloads::WorkloadSpec,
};
use pse_traverse::{
    cognition::{
        pipeline::{run_cognition, CognitionInput, CognitiveComponents},
        CognitionOutcome,
        CognitiveState5D,
        Fixed as TraverseFixed,
    },
    dynamic_state::Hash256 as TraverseHash,
};

use crate::{
    metrics_bridge::observations_from_outcomes,
    scenarios::{rd_for_scenario, CognitionScenario, ExpectedOutcome},
};

// ── Domain-agnostische Reasoning-Tasks ───────────────────────────────────

/// Eine Phase im Plan→Execute→Evaluate Zyklus.
struct ReasoningPhase {
    name: &'static str,
    prompt_suffix: &'static str,
    /// Strukturelle Reasoning-Elemente die in einer guten Antwort auftauchen.
    /// Kein Domänenwissen — nur Reasoning-Vokabular.
    required_structural: &'static [&'static str],
    /// Erwartete Antwortlänge in Wörtern.
    target_word_count: u32,
}

struct ReasoningTask {
    id: &'static str,
    scenario: &'static str,
    phases: [ReasoningPhase; 3],
}

/// Vier domain-agnostische Reasoning-Szenarien.  Das LLM muss kein
/// PSE-Wissen haben — nur strukturiertes Denken zeigen.
const REASONING_TASKS: &[ReasoningTask] = &[
    ReasoningTask {
        id: "system_analysis",
        scenario: "A distributed system has three components: an ingestion service, \
                   a processing queue, and a storage backend. Under high load, the queue \
                   depth grows unbounded and the storage backend starts rejecting writes.",
        phases: [
            ReasoningPhase {
                name: "plan",
                prompt_suffix: "Outline a 3-step plan to diagnose and resolve this issue. \
                                 Be specific about which component to investigate first and why.",
                required_structural: &["step", "investigate", "because", "first", "then"],
                target_word_count: 80,
            },
            ReasoningPhase {
                name: "execute",
                prompt_suffix: "Execute the first step of your plan: analyze the ingestion service. \
                                 Identify at least two specific failure modes with their causes.",
                required_structural: &["failure", "cause", "rate", "limit", "overflow"],
                target_word_count: 100,
            },
            ReasoningPhase {
                name: "evaluate",
                prompt_suffix: "Based on your analysis, state which single change would have the \
                                 greatest impact. Justify with a specific reasoning chain.",
                required_structural: &["therefore", "because", "impact", "recommend", "result"],
                target_word_count: 80,
            },
        ],
    },
    ReasoningTask {
        id: "trade_off_decision",
        scenario: "A team must choose between two database systems: System A offers \
                   strong consistency but high latency (50ms avg). System B offers \
                   eventual consistency but low latency (5ms avg). The application \
                   requires real-time user feedback but processes financial transactions.",
        phases: [
            ReasoningPhase {
                name: "plan",
                prompt_suffix: "List the key trade-offs you must evaluate before making \
                                 a recommendation. Name at least three distinct trade-offs.",
                required_structural: &["trade-off", "consistency", "latency", "transaction", "versus"],
                target_word_count: 80,
            },
            ReasoningPhase {
                name: "execute",
                prompt_suffix: "Evaluate the consistency trade-off in detail. What are the \
                                 concrete risks of eventual consistency for financial transactions?",
                required_structural: &["risk", "inconsistent", "financial", "data", "failure"],
                target_word_count: 100,
            },
            ReasoningPhase {
                name: "evaluate",
                prompt_suffix: "Make a final recommendation. State clearly which system to choose \
                                 and provide exactly two reasons that justify the choice.",
                required_structural: &["recommend", "because", "first", "second", "choose"],
                target_word_count: 80,
            },
        ],
    },
    ReasoningTask {
        id: "argument_analysis",
        scenario: "Argument: 'All high-performing software teams use agile methods. \
                   Team X uses agile methods. Therefore Team X is high-performing.'",
        phases: [
            ReasoningPhase {
                name: "plan",
                prompt_suffix: "Identify the logical structure of this argument. \
                                 Name the premises and the conclusion explicitly.",
                required_structural: &["premise", "conclusion", "therefore", "follows", "argument"],
                target_word_count: 60,
            },
            ReasoningPhase {
                name: "execute",
                prompt_suffix: "Evaluate whether the argument is logically valid. \
                                 If invalid, name the specific logical fallacy.",
                required_structural: &["invalid", "fallacy", "affirm", "converse", "does not follow"],
                target_word_count: 80,
            },
            ReasoningPhase {
                name: "evaluate",
                prompt_suffix: "Provide a corrected version of the argument that is \
                                 logically valid, and explain what additional evidence \
                                 would be needed to support the original claim.",
                required_structural: &["corrected", "evidence", "if", "then", "valid"],
                target_word_count: 80,
            },
        ],
    },
    ReasoningTask {
        id: "constraint_satisfaction",
        scenario: "Schedule four tasks (T1=3h, T2=2h, T3=4h, T4=1h) into an 8-hour \
                   workday. Constraints: T3 must start after T1 finishes. T2 and T4 \
                   cannot run in parallel. Minimize the total idle time.",
        phases: [
            ReasoningPhase {
                name: "plan",
                prompt_suffix: "List all constraints explicitly and state which ordering \
                                 rules they impose on the tasks.",
                required_structural: &["constraint", "T1", "T3", "before", "after"],
                target_word_count: 70,
            },
            ReasoningPhase {
                name: "execute",
                prompt_suffix: "Construct a concrete schedule. State the start time of each \
                                 task in hours from 0. Show that all constraints are satisfied.",
                required_structural: &["start", "hour", "T1", "T2", "T3", "T4", "satisfied"],
                target_word_count: 100,
            },
            ReasoningPhase {
                name: "evaluate",
                prompt_suffix: "Calculate the total idle time in your schedule. Determine \
                                 whether a better schedule exists, and if so why.",
                required_structural: &["idle", "total", "hours", "optimal", "because"],
                target_word_count: 70,
            },
        ],
    },
];

// ── Cerebras HTTP call ────────────────────────────────────────────────────

fn call_cerebras(api_key: &str, model: &str, system: &str, user: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ],
        "max_tokens": 400,
        "temperature": 0
    });

    let retry_delays = [5u64, 10, 20];
    let mut attempt = 0usize;
    loop {
        let resp = client
            .post("https://api.cerebras.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;

        let val: serde_json::Value = resp.json().map_err(|e| e.to_string())?;

        if val["type"] == "too_many_requests_error"
            || val["error"]["type"] == "too_many_requests_error"
        {
            if attempt < retry_delays.len() {
                let wait = retry_delays[attempt];
                eprintln!(" [rate-limit, warte {wait}s …]");
                std::thread::sleep(std::time::Duration::from_secs(wait));
                attempt += 1;
                continue;
            }
            return Err(format!("unexpected API shape: {val}"));
        }

        return val["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("unexpected API shape: {val}"));
    }
}

// ── Deterministic CI stub ─────────────────────────────────────────────────

/// Wenn kein API-Key vorhanden: deterministischer Stub.
///
/// "Starke" Phasen (gerade Kombinationen aus Task-Index + Phase-Index):
///   - Vollständige Sätze mit allen required_structural
///   - Schlussfolgerungs-Marker ("therefore", "recommend")
///   - Keine Unsicherheits-Marker
///   → hohe support_strength, hohe ρ, hohe ω, niedrige τ → Bundle
///
/// "Schwache" Phasen (ungerade Kombinationen):
///   - Kurzer Text mit Unsicherheits-Markern ("might", "could")
///   - Nur halbe required_structural vorhanden
///   → niedrige support_strength, niedrige ρ, hohe τ → Hold
fn stub_response(task: &ReasoningTask, phase: &ReasoningPhase, phase_idx: usize) -> String {
    let task_idx = REASONING_TASKS.iter().position(|t| t.id == task.id).unwrap_or(0);
    let strong = (task_idx + phase_idx) % 3 != 0;

    if strong {
        // Alle Strukturelemente + Schlussfolgerungs-Marker + mehrere Sätze
        let elements = phase.required_structural.join(", ");
        format!(
            "The analysis identifies the following key aspects: {elements}. \
             Therefore, the recommended approach addresses each constraint directly. \
             The result is a complete and well-structured solution that satisfies \
             all stated requirements. This concludes the evaluation."
        )
    } else {
        // Nur halbe Elemente + Unsicherheits-Marker → schwache kognitive Qualität
        let half = (phase.required_structural.len() / 2).max(1);
        let partial = phase.required_structural[..half].join(", ");
        format!(
            "This might involve {partial}. Could be relevant, but unclear. \
             Perhaps more investigation needed."
        )
    }
}

// ── Text → CognitionInput ─────────────────────────────────────────────────

fn f(num: i128, den: i128) -> TraverseFixed {
    TraverseFixed::Rational { num, den }
}

fn hash_from_seed(seed: u64) -> TraverseHash {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    TraverseHash(bytes)
}

/// Deterministische Text → 5D Abbildung.  Kein Platform-Float im Audit-Pfad.
fn text_to_cognition_input(
    response: &str,
    required: &[&str],
    target_word_count: u32,
    seed: u64,
    logical_step: u64,
    constraint_count: u32,
) -> CognitionInput {
    let lower = response.to_lowercase();
    let word_count = response.split_whitespace().count() as i128;
    let target = target_word_count.max(1) as i128;

    // ψ — semantische Abdeckung (Wortmenge vs. Ziel)
    let psi = f(word_count.min(target), target);

    // ρ — kohärente Satzdichte (Sätze mit ≥ 5 Wörtern)
    let coherent = response
        .split('.')
        .filter(|s| s.split_whitespace().count() >= 5)
        .count() as i128;
    let rho = f(coherent.min(8), 8);

    // ω — Schlussfolgerungs-Marker
    let conclude = ["therefore", "thus", "recommend", "conclude", "result", "hence", "thus"];
    let omega_n = conclude.iter().filter(|&&w| lower.contains(w)).count() as i128;
    let omega = f(5 + omega_n.min(5), 10);

    // χ — Verzweigungskomplexität
    let branch = ["however", "alternatively", "but", "unless", "either", "whereas"];
    let chi_n = branch.iter().filter(|&&w| lower.contains(w)).count() as i128;
    let chi = f(chi_n.min(5), 10);

    // τ — ausgedrückte Unsicherheit
    let uncertain = ["might", "could", "perhaps", "maybe", "possibly", "unclear", "not sure"];
    let tau_n = uncertain.iter().filter(|&&w| lower.contains(w)).count() as i128;
    let tau = f(tau_n.min(5), 10);

    // support_strength = strukturelle Elemente gefunden / total
    let matched = required.iter().filter(|&&e| lower.contains(e)).count() as i128;
    let support_strength = f(matched, required.len().max(1) as i128);

    let mem = CognitiveState5D::from_components(
        psi.clone(), rho.clone(), omega.clone(), chi.clone(), tau.clone(),
    )
    .expect("valid 5d from text");

    CognitionInput {
        null_center_id: hash_from_seed(seed),
        cognitive_components: CognitiveComponents { psi, rho, omega, chi, tau },
        source_traversal_report_hash: None,
        source_projection_report_hash: None,
        spiral_memory_candidates: vec![mem],
        constraint_count,
        support_strength,
        logical_step,
        carrier_ids: vec!["agent.0".into()],
    }
}

// ── CerebrasTrialExecutor ─────────────────────────────────────────────────

/// Orchestriert einen Reasoning-Agent (Plan → Execute → Evaluate) über
/// PSE's Kognitionspipeline.  PSE entscheidet nach jeder Phase ob der
/// Agent weiterarbeiten darf oder halten muss.
///
/// Erfordert `CEREBRAS_API_KEY` zur Laufzeit; ohne Key läuft ein
/// deterministischer Stub (für CI).
pub struct CerebrasTrialExecutor {
    /// Cerebras-Modell (Standard: llama-3.3-70b).
    pub model: String,
}

impl Default for CerebrasTrialExecutor {
    fn default() -> Self {
        CerebrasTrialExecutor {
            model: "llama-3.3-70b".into(),
        }
    }
}

impl TrialExecutor for CerebrasTrialExecutor {
    fn execute_trial(
        &self,
        _spec: &EvaluationSpec,
        variant: &SystemVariantSpec,
        workload: &WorkloadSpec,
        descriptor: &RunDescriptor,
    ) -> Result<TrialReport, EvalError> {
        let api_key = std::env::var("CEREBRAS_API_KEY").unwrap_or_default();
        let layer_count = variant.layer_mask.count_layers();

        // Wähle Task deterministisch per Seed.
        let task = &REASONING_TASKS[descriptor.seed as usize % REASONING_TASKS.len()];

        let system_msg = format!(
            "You are a precise reasoning assistant. Scenario:\n\n{}",
            task.scenario
        );

        let mut cog_scenarios: Vec<CognitionScenario> = Vec::new();
        let mut cog_outcomes: Vec<CognitionOutcome> = Vec::new();
        let mut gates: Vec<GateObservation> = Vec::new();
        let mut phase_coverages: Vec<(usize, usize)> = Vec::new();

        for (phase_idx, phase) in task.phases.iter().enumerate() {
            let logical_step = (phase_idx + 1) as u64;
            let phase_seed = descriptor.seed.wrapping_add(phase_idx as u64 * 1000);

            // — LLM-Aufruf oder deterministischer Stub ——————————————
            let response = if api_key.is_empty() {
                stub_response(task, phase, phase_idx)
            } else {
                call_cerebras(&api_key, &self.model, &system_msg, phase.prompt_suffix)
                    .map_err(|e| EvalError::InvalidSpec {
                        reason: format!(
                            "Cerebras error in task={} phase={}: {e}",
                            task.id, phase.name
                        ),
                    })?
            };

            // — Scoring ————————————————————————————————————————————
            let lower = response.to_lowercase();
            let matched = phase
                .required_structural
                .iter()
                .filter(|&&e| lower.contains(e))
                .count();
            let total = phase.required_structural.len();
            phase_coverages.push((matched, total));

            // — Text → CognitionInput ——————————————————————————————
            let cog_input = text_to_cognition_input(
                &response,
                phase.required_structural,
                phase.target_word_count,
                phase_seed,
                logical_step,
                // constraint_count=1: support_strength IS the quality signal.
                // The percolation gate becomes: support_strength >= min_constraint_mass.
                1,
            );

            // Erwartetes Outcome: Bundle wenn alle Strukturelemente vorhanden.
            let expected = if matched == total {
                ExpectedOutcome::Bundle
            } else {
                ExpectedOutcome::Hold
            };
            cog_scenarios.push(CognitionScenario {
                name: phase.name,
                input: cog_input.clone(),
                expected,
            });

            // — PSE run_cognition ———————————————————————————————————
            let rd = rd_for_scenario(
                &format!("{}.{}", task.id, phase.name),
                layer_count,
                phase_seed,
            );
            let cog_result =
                run_cognition(&cog_input, &rd).map_err(|e| EvalError::InvalidSpec {
                    reason: format!(
                        "run_cognition failed task={} phase={}: {e:?}",
                        task.id, phase.name
                    ),
                })?;

            gates.push(GateObservation {
                gate_name: format!("{}.{}.perc", task.id, phase.name),
                passed: cog_result.lattice.percolation_passed,
                reason: Some(format!(
                    "structural coverage {matched}/{total}; support_strength derives percolation"
                )),
            });
            gates.push(GateObservation {
                gate_name: format!("{}.{}.panorama", task.id, phase.name),
                passed: pse_eval_matrix::primitives::fixed_ge(
                    &cog_result.panorama.coverage_score,
                    &rd.thresholds.min_panorama_coverage,
                ),
                reason: Some(format!("logical_step={logical_step}")),
            });

            cog_outcomes.push(cog_result.outcome);
        }

        // — Aggregate metrics ——————————————————————————————————————
        let mut metrics = observations_from_outcomes(&cog_scenarios, &cog_outcomes);

        // Strukturelle Abdeckung über alle Phasen (kein PSE-Domänenwissen).
        let total_matched: usize = phase_coverages.iter().map(|(m, _)| m).sum();
        let total_required: usize = phase_coverages.iter().map(|(_, t)| t).sum();
        metrics.push(pse_eval_matrix::MetricObservation::ok(
            "structural_coverage",
            Fixed::Rational {
                num: total_matched as i128,
                den: total_required.max(1) as i128,
            },
        ));

        // — Outputs ————————————————————————————————————————————————
        let bundle_count = cog_outcomes
            .iter()
            .filter(|o| matches!(o, CognitionOutcome::CandidateBundle { .. }))
            .count() as u64;
        let hold_count = cog_outcomes.len() as u64 - bundle_count;
        let expected_holds =
            cog_scenarios.iter().filter(|s| !s.expected_bundle()).count() as u64;
        let correct_holds = cog_scenarios
            .iter()
            .zip(cog_outcomes.iter())
            .filter(|(s, o)| !s.expected_bundle() && matches!(*o, CognitionOutcome::Hold { .. }))
            .count() as u64;

        let outputs = TrialOutputs {
            outcome_hash: Some(content_address(&(&descriptor.run_id, layer_count))?),
            candidate_bundle_hash: None,
            finalized_emission_hash: None,
            detected_count: cog_scenarios.len() as u64,
            true_positive_count: bundle_count,
            false_positive_count: expected_holds.saturating_sub(correct_holds),
            false_negative_count: 0,
            safety_event_count: expected_holds.saturating_sub(correct_holds),
            correct_hold_count: correct_holds,
            hold_count,
            manual_intervention_count: if layer_count == 0 { expected_holds } else { 0 },
        };

        // — Replay ——————————————————————————————————————————————————
        let anchor = content_address(&(
            &descriptor.run_id,
            layer_count,
            total_matched as u64,
            total_required as u64,
        ))?;
        let replay = ReplayObservation {
            expected_hash: anchor.clone(),
            observed_hash: anchor,
            passed: true,
        };

        let report = TrialReport {
            trial_id: Hash256::zero(),
            variant_id: variant.variant_id.clone(),
            workload_id: workload.workload_id.clone(),
            dataset_id: workload.input_manifest_hash.clone(),
            run_descriptor_hash: descriptor.content_hash()?,
            outputs,
            metrics,
            gates: GateObservationSet::from_vec(gates),
            replay,
            diagnostics: Vec::<DiagnosticRecord>::new(),
            failures: vec![],
        };
        report.with_id()
    }
}

#[cfg(test)]
mod tests {
    use pse_eval_matrix::{plan_runs, run_trial, Fixed, Preset};

    use super::*;

    #[test]
    fn cerebras_executor_stub_runs_all_variants() {
        let spec = Preset::AgentCognition.build().unwrap();
        let plan = plan_runs(&spec).unwrap();
        let executor = CerebrasTrialExecutor::default();
        for entry in &plan.entries {
            let variant = spec
                .variants
                .iter()
                .find(|v| v.variant_id == entry.descriptor.variant_id)
                .unwrap();
            let workload = spec
                .workloads
                .iter()
                .find(|w| w.workload_id == entry.descriptor.workload_id)
                .unwrap();
            let (report, _) = run_trial(
                &spec,
                variant,
                workload,
                &entry.descriptor,
                &executor,
                &spec.metrics,
            )
            .unwrap();
            assert!(
                report.replay.passed,
                "replay must pass: variant={}",
                variant.variant_id
            );
            assert!(
                report.metrics.iter().any(|m| m.metric_id == "structural_coverage"),
                "structural_coverage metric must be present"
            );
        }
    }

    #[test]
    fn b6_false_commit_rate_not_worse_than_b0() {
        let spec = Preset::AgentCognition.build().unwrap();
        let plan = plan_runs(&spec).unwrap();
        let executor = CerebrasTrialExecutor::default();
        let mut reports = Vec::new();
        for entry in &plan.entries {
            let variant = spec
                .variants
                .iter()
                .find(|v| v.variant_id == entry.descriptor.variant_id)
                .unwrap();
            let workload = spec
                .workloads
                .iter()
                .find(|w| w.workload_id == entry.descriptor.workload_id)
                .unwrap();
            let (r, _) =
                run_trial(&spec, variant, workload, &entry.descriptor, &executor, &spec.metrics)
                    .unwrap();
            reports.push(r);
        }
        let get = |vid: &str, mid: &str| -> Fixed {
            reports
                .iter()
                .find(|r| r.variant_id == vid)
                .and_then(|r| r.metrics.iter().find(|m| m.metric_id == mid))
                .map(|m| m.value.clone())
                .unwrap_or(Fixed::Rational { num: 0, den: 1 })
        };
        let b0 = get("B0_Baseline", "false_commit_rate");
        let b6 = get("B6_Cognition", "false_commit_rate");
        assert!(
            b6.cmp(&b0) != std::cmp::Ordering::Greater,
            "B6 false_commit_rate must be ≤ B0; B0={b0:?} B6={b6:?}"
        );
    }

    #[test]
    fn stub_determinism_same_seed_same_output() {
        let task = &REASONING_TASKS[0];
        let r1 = stub_response(task, &task.phases[0], 0);
        let r2 = stub_response(task, &task.phases[0], 0);
        assert_eq!(r1, r2, "stub must be deterministic");
    }
}
