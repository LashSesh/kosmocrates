//! Top-level run orchestrator (§8, §16).
//!
//! `run()` executes all four phases (L0–L3) in order, recording each
//! command in the ValidationRunLedger, then produces FinalValidationReport
//! + BundleManifest + ReplayResult.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::bundle::{create_bundle, BundleManifest};
use crate::command_plan::{build_command_plan, ValidationPhase};
use crate::domain::{BenchGtRecord, DatasetManifest, DomainValidationSummary};
use crate::environment::EnvironmentFingerprint;
use crate::eval_matrix::{assess_presets, build_eval_summary};
use crate::executor::CommandExecutor;
use crate::executor::CommandResult;
use crate::ledger::ValidationRunLedger;
use crate::parsers::{BenchmarkSummary, QualitySummary};
use crate::primitives::{Hash256, StableId, ValidationError};
use crate::profile::ValidationProfile;
use crate::replay::{verify_replay, ReplayResult};
use crate::report::FinalValidationReport;
use crate::scoring::{derive_conclusion, ScoringInputs};
use crate::snapshot::RepositorySnapshot;

/// Complete output of a validation run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationRunOutput {
    pub run_id: StableId,
    pub snapshot: RepositorySnapshot,
    pub environment: EnvironmentFingerprint,
    pub ledger: ValidationRunLedger,
    pub report: FinalValidationReport,
    pub bundle: BundleManifest,
    pub replay: ReplayResult,
}

/// Run a full validation pass.
///
/// `repo_root`: workspace root (contains Cargo.toml / .git).
/// `out_dir`: where to write artifacts.
/// `executor`: command runner (use `ProcessExecutor` in production, `MockExecutor` in tests).
pub fn run(
    profile: &ValidationProfile,
    repo_root: &Path,
    out_dir: &Path,
    executor: &dyn CommandExecutor,
) -> Result<ValidationRunOutput, ValidationError> {
    // Create output directory structure.
    create_dirs(out_dir)?;

    // Capture environment (no wall-clock in hashes).
    let environment = EnvironmentFingerprint::collect()?;
    let environment_hash = environment.content_hash()?;

    // Capture repository snapshot.
    let snapshot = RepositorySnapshot::capture(repo_root)?;
    let snapshot_hash = snapshot.content_hash()?;
    let profile_hash = profile.content_hash()?;

    // Build deterministic command plan.
    let plan = build_command_plan(profile, repo_root, out_dir)?;

    // Generate run_id from profile + snapshot (no wall-clock).
    let run_id = make_run_id(&snapshot_hash, &profile_hash);

    // Write manifests.
    write_json(out_dir, "manifest.json", &plan)?;
    write_json(out_dir, "repo_snapshot.json", &snapshot)?;
    write_json(out_dir, "environment.json", &environment)?;
    write_json(out_dir, "command_plan.json", &plan)?;

    // Initialize ledger.
    let mut ledger =
        ValidationRunLedger::new(run_id.clone(), snapshot_hash.clone(), profile_hash.clone())?;

    // Track phase results.
    let mut quality_exits: BTreeMap<String, i32> = BTreeMap::new();
    let mut bench_exits: BTreeMap<String, i32> = BTreeMap::new();
    let mut eval_exits: BTreeMap<String, i32> = BTreeMap::new();
    let mut eval_replay_ok: BTreeMap<String, bool> = BTreeMap::new();
    let mut quality_failed = false;
    let mut domain_phase_exits: BTreeMap<ValidationPhase, i32> = BTreeMap::new();

    // Execute commands phase by phase.
    let log_dir = out_dir.to_path_buf();
    for command in &plan.commands {
        let result = match executor.execute(command, &log_dir) {
            Ok(result) => result,
            Err(_) => {
                let failed = CommandResult {
                    command_id: command.command_id.clone(),
                    exit_code: -1,
                    stdout_hash: Hash256::zero(),
                    stderr_hash: Hash256::zero(),
                    stdout_len: 0,
                    stderr_len: 0,
                    stdout_path: None,
                };
                ledger.append(command.phase, &failed, vec![])?;
                match command.phase {
                    ValidationPhase::Quality => {
                        let key = command.argv.get(1).cloned().unwrap_or_default();
                        quality_exits.insert(key, failed.exit_code);
                    }
                    ValidationPhase::Benchmark => {
                        let key = command.argv.get(4).cloned().unwrap_or_default();
                        bench_exits.insert(key, failed.exit_code);
                    }
                    ValidationPhase::DomainCalibration
                    | ValidationPhase::DomainValidation
                    | ValidationPhase::DomainTest => {
                        domain_phase_exits.insert(command.phase, failed.exit_code);
                    }
                    _ => {}
                }
                break;
            }
        };
        let satisfied = command.expected_exit.is_satisfied(result.exit_code);

        ledger.append(command.phase, &result, vec![])?;

        match command.phase {
            ValidationPhase::Quality => {
                let key = command.argv.get(1).cloned().unwrap_or_default();
                quality_exits.insert(key, result.exit_code);
                if !satisfied && !profile.quality.continue_after_quality_fail {
                    quality_failed = true;
                }
            }
            ValidationPhase::Benchmark => {
                let key = command.argv.get(4).cloned().unwrap_or_default();
                bench_exits.insert(key, result.exit_code);
            }
            ValidationPhase::EvalMatrix => {
                // Determine which preset this belongs to from output_path.
                if let Some(out_path) = command.output_paths.first() {
                    if let Some(preset) = extract_preset_from_path(out_path) {
                        // Track replay result from replay.json step.
                        if out_path.ends_with("replay.json") {
                            eval_replay_ok.insert(preset.clone(), result.exit_code == 0);
                        }
                        eval_exits.entry(preset).or_insert(result.exit_code);
                    }
                }
            }
            ValidationPhase::DomainCalibration
            | ValidationPhase::DomainValidation
            | ValidationPhase::DomainTest => {
                domain_phase_exits.insert(command.phase, result.exit_code);
            }
            _ => {}
        }

        if quality_failed {
            break;
        }
    }

    // Build summaries.
    let quality_summary = QualitySummary::from_exit_codes(
        quality_exits.get("fmt").copied(),
        quality_exits.get("clippy").copied(),
        quality_exits.get("build").copied(),
        quality_exits.get("test").copied(),
        quality_exits.get("doc").copied(),
        "",
    );
    let mut benchmark_summary = BenchmarkSummary::from_logs(
        bench_exits.get("bench_full").copied(),
        bench_exits.get("bench_gt").copied(),
        bench_exits.get("pse-demo").copied(),
        "",
    );

    let (missing_presets, eval_obligations) = assess_presets(
        &profile.eval_matrix.presets,
        profile.eval_matrix.report_missing_topology_preset,
    );
    let eval_summary = build_eval_summary(
        &eval_exits,
        &eval_replay_ok,
        missing_presets,
        eval_obligations,
    );

    let replay_identity = eval_summary.preset_results.iter().all(|r| r.replay_passed);

    // ── L3 Domain post-processing ─────────────────────────────────────────────
    let domain_dir = out_dir.join("domain");
    let (domain_summary_opt, domain_leakage_free, domain_test_completed) = if let Some(domain_cfg) =
        &profile.domain
    {
        // Load manifest from configured path.
        let manifest: Option<DatasetManifest> = load_json_opt(&domain_cfg.dataset_manifest_path);

        // Try to load bench_gt JSON outputs produced by the domain phases.
        let records: Vec<BenchGtRecord> = load_domain_records(&domain_dir);

        let leakage_free = manifest
            .as_ref()
            .map(|m| m.verify_splits().is_ok())
            .unwrap_or(false);

        let test_completed = records.iter().any(|r| r.scenario.contains("binance"))
            && domain_phase_exits
                .get(&ValidationPhase::DomainTest)
                .copied()
                .unwrap_or(1)
                == 0;

        let summary = if let Some(manifest) = &manifest {
            let manifest_hash = manifest.content_hash().unwrap_or_else(|_| Hash256::zero());
            DomainValidationSummary::build_from_records(
                domain_cfg.domain_name.clone(),
                manifest_hash,
                records,
                manifest,
            )
            .ok()
        } else {
            None
        };

        (summary, leakage_free, test_completed)
    } else {
        (None, true, false)
    };

    // Write domain summary artifacts.
    if let Some(ref summary) = domain_summary_opt {
        write_json(&domain_dir, "domain_summary.json", summary)?;
        if let Some(ref bc) = summary.baseline_comparison {
            write_json(&domain_dir, "baseline_comparison.json", bc)?;
        }
    }

    let domain_available = profile.domain.is_some() && domain_summary_opt.is_some();
    let domain_pse_metrics_present = domain_summary_opt
        .as_ref()
        .map(|s| s.test_f1.is_some() && s.scenario_metrics.iter().any(|r| r.pse_metrics.is_some()))
        .unwrap_or(false);
    let domain_pse_majority_wins = domain_summary_opt
        .as_ref()
        .and_then(|s| s.baseline_comparison.as_ref())
        .map(|b| b.pse_majority_wins)
        .unwrap_or(false);

    if !benchmark_summary.bench_gt_completed
        && domain_summary_opt
            .as_ref()
            .map(|s| !s.scenario_metrics.is_empty())
            .unwrap_or(false)
    {
        benchmark_summary.bench_gt_completed = true;
    }

    let conclusion = derive_conclusion(&ScoringInputs {
        quality: &quality_summary,
        benchmarks: &benchmark_summary,
        eval_matrix: &eval_summary,
        replay_identity,
        domain_available,
        domain_leakage_free,
        domain_test_completed,
        domain_pse_metrics_present,
        domain_pse_majority_wins,
    });

    // Write verdict.json.
    write_json(
        out_dir,
        "verdict.json",
        &serde_json::json!({
            "conclusion": conclusion.as_str(),
            "domain_available": domain_available,
            "domain_leakage_free": domain_leakage_free,
            "domain_test_completed": domain_test_completed,
            "replay_identity": replay_identity,
        }),
    )?;

    // Build final report.
    let report = FinalValidationReport::build(
        run_id.clone(),
        snapshot_hash,
        environment_hash,
        profile_hash,
        quality_summary,
        benchmark_summary,
        &eval_summary,
        domain_summary_opt.clone(),
        conclusion,
        vec![],
    )?;

    // Write final report files.
    let final_dir = out_dir.join("final");
    std::fs::create_dir_all(&final_dir).map_err(|e| ValidationError::Io {
        reason: format!("create final dir: {e}"),
    })?;
    write_json(&final_dir, "final_report.json", &report)?;
    let md = report.to_markdown(&eval_summary);
    std::fs::write(final_dir.join("final_report.md"), &md).map_err(|e| ValidationError::Io {
        reason: format!("write final_report.md: {e}"),
    })?;
    write_json(
        &final_dir,
        "conclusion.json",
        &serde_json::json!({ "conclusion": report.conclusion.as_str() }),
    )?;

    // Write open obligations.
    let mut obligations_md = String::from("# Open Obligations\n\n");
    for ob in &report.open_obligations {
        obligations_md.push_str(&format!(
            "## {}\n\n{}\n\nRecovery: {}\n\n",
            ob.obligation_id, ob.description, ob.recovery
        ));
    }
    std::fs::write(final_dir.join("open_obligations.md"), &obligations_md).map_err(|e| {
        ValidationError::Io {
            reason: format!("write open_obligations.md: {e}"),
        }
    })?;

    // Write ledger.
    write_json(out_dir, "command_log.jsonl", &ledger)?;

    // Replay verification (bundle not yet created — pass None).
    let replay = verify_replay(&ledger, None)?;
    write_json(out_dir, "replay_result.json", &replay)?;

    // Create bundle manifest after all other files are written.
    let bundle = create_bundle(out_dir, &run_id, &report.report_id)?;
    write_json(out_dir, "bundle_manifest.json", &bundle)?;

    Ok(ValidationRunOutput {
        run_id,
        snapshot,
        environment,
        ledger,
        report,
        bundle,
        replay,
    })
}

/// Try to load a JSON file; returns None on missing or parse error.
fn load_json_opt<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Load all bench_gt JSON output files from the domain directory.
fn load_domain_records(domain_dir: &Path) -> Vec<BenchGtRecord> {
    let mut records = Vec::new();
    for name in &["seismo.json", "vitals.json", "binance.json"] {
        let path = domain_dir.join(name);
        if let Some(record) = load_json_opt::<BenchGtRecord>(path.to_str().unwrap_or("")) {
            records.push(record);
        }
    }
    records
}

/// Generate a deterministic run ID from snapshot + profile hashes.
fn make_run_id(snapshot_hash: &Hash256, profile_hash: &Hash256) -> StableId {
    // Format: run_{short_snap}_{short_prof} — no wall-clock.
    let snap = &snapshot_hash.hex()[..8];
    let prof = &profile_hash.hex()[..8];
    format!("run_{snap}_{prof}")
}

fn create_dirs(out_dir: &Path) -> Result<(), ValidationError> {
    for sub in &["", "quality", "bench", "eval", "domain", "final"] {
        let dir = if sub.is_empty() {
            out_dir.to_path_buf()
        } else {
            out_dir.join(sub)
        };
        std::fs::create_dir_all(&dir).map_err(|e| ValidationError::Io {
            reason: format!("create dir {}: {e}", dir.display()),
        })?;
    }
    Ok(())
}

fn write_json<T: serde::Serialize>(
    dir: &Path,
    name: &str,
    value: &T,
) -> Result<(), ValidationError> {
    let path = dir.join(name);
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|e| ValidationError::Canonicalization {
            reason: e.to_string(),
        })?;
    std::fs::write(&path, &bytes).map_err(|e| ValidationError::Io {
        reason: format!("write {}: {e}", path.display()),
    })
}

fn extract_preset_from_path(path: &str) -> Option<String> {
    // Path format: .../eval/<preset>/xxx.json
    let normalized = path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').collect();
    let eval_pos = parts.iter().position(|s| *s == "eval")?;
    parts.get(eval_pos + 1).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{CommandExecutor, MockExecutor};

    #[test]
    fn smoke_profile_produces_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("run_out");
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let profile = ValidationProfile::smoke().unwrap();
        let executor = MockExecutor::success();
        let output = run(&profile, repo_root, &out, &executor).unwrap();

        assert!(out.join("final/final_report.json").exists());
        assert!(out.join("final/final_report.md").exists());
        assert!(out.join("bundle_manifest.json").exists());
        assert!(out.join("verdict.json").exists());
        assert!(!output.run_id.is_empty());
    }

    #[test]
    fn run_is_deterministic_for_same_inputs() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let out1 = dir1.path().join("r1");
        let out2 = dir2.path().join("r2");
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let profile = ValidationProfile::smoke().unwrap();
        let exec = MockExecutor::success();
        let o1 = run(&profile, repo_root, &out1, &exec).unwrap();
        let o2 = run(&profile, repo_root, &out2, &exec).unwrap();
        // Same profile + same repo snapshot → same run_id.
        assert_eq!(o1.run_id, o2.run_id);
        assert_eq!(o1.report.report_id, o2.report.report_id);
    }

    struct FailingExecutor;

    impl CommandExecutor for FailingExecutor {
        fn execute(
            &self,
            _command: &crate::command_plan::ValidationCommand,
            _log_dir: &std::path::Path,
        ) -> Result<crate::executor::CommandResult, ValidationError> {
            Err(ValidationError::CommandFailed {
                reason: "simulated missing executable".into(),
            })
        }
    }

    #[test]
    fn run_writes_final_artifacts_when_command_spawn_fails() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("run_out");
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let profile = ValidationProfile::smoke().unwrap();
        let output = run(&profile, repo_root, &out, &FailingExecutor).unwrap();

        assert!(out.join("command_log.jsonl").exists());
        assert!(out.join("verdict.json").exists());
        assert!(out.join("final/final_report.json").exists());
        assert_eq!(
            output.report.conclusion.as_str(),
            crate::report::ValidationConclusion::Invalid.as_str()
        );
    }
}
