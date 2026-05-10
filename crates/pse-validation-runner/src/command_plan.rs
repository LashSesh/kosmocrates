//! CommandPlan — deterministic sequence of ValidationCommands (§7.4, §8).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, Hash256, ValidationError};
use crate::profile::ValidationProfile;

/// A phase of the validation run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ValidationPhase {
    Snapshot,
    Quality,
    Benchmark,
    EvalMatrix,
    DomainCalibration,
    DomainValidation,
    DomainTest,
    Scoring,
    Reporting,
    Bundling,
}

/// Whether a non-zero exit code is acceptable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedExit {
    Zero,
    AnyCode,
    OneOf(Vec<i32>),
}

impl ExpectedExit {
    pub fn is_satisfied(&self, code: i32) -> bool {
        match self {
            ExpectedExit::Zero => code == 0,
            ExpectedExit::AnyCode => true,
            ExpectedExit::OneOf(codes) => codes.contains(&code),
        }
    }
}

/// A single command to be executed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCommand {
    pub command_id: Hash256,
    pub phase: ValidationPhase,
    pub argv: Vec<String>,
    pub working_dir: String,
    pub env: BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub expected_exit: ExpectedExit,
    /// Relative paths (from run output dir) where this command writes artifacts.
    pub output_paths: Vec<String>,
}

impl ValidationCommand {
    pub fn new(
        phase: ValidationPhase,
        argv: Vec<String>,
        working_dir: String,
        env: BTreeMap<String, String>,
        timeout_seconds: u64,
        expected_exit: ExpectedExit,
        output_paths: Vec<String>,
    ) -> Result<Self, ValidationError> {
        let partial = ValidationCommand {
            command_id: Hash256::zero(),
            phase,
            argv,
            working_dir,
            env,
            timeout_seconds,
            expected_exit,
            output_paths,
        };
        let command_id = content_address(&partial)?;
        Ok(ValidationCommand { command_id, ..partial })
    }
}

/// Ordered list of commands for one validation run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandPlan {
    pub plan_id: Hash256,
    pub profile_id: Hash256,
    pub commands: Vec<ValidationCommand>,
}

impl CommandPlan {
    pub fn content_hash(&self) -> Result<Hash256, ValidationError> {
        content_address(self)
    }
}

/// Build the full command plan from a profile.
pub fn build_command_plan(
    profile: &ValidationProfile,
    repo_root: &Path,
    out_dir: &Path,
) -> Result<CommandPlan, ValidationError> {
    let root = repo_root.to_string_lossy().to_string();
    let mut commands = Vec::new();

    // L0: Quality Gate
    if profile.quality.run_fmt {
        commands.push(make_cmd(
            ValidationPhase::Quality,
            vec!["cargo", "fmt", "--all", "--", "--check"],
            &root,
            profile.timeouts.quality_seconds,
            &profile.extra_env,
            vec![format!("{}/quality/fmt.log", out_dir.display())],
        )?);
    }
    if profile.quality.run_clippy {
        let mut env = profile.extra_env.clone();
        env.insert("RUSTFLAGS".into(), "-D warnings".into());
        commands.push(ValidationCommand::new(
            ValidationPhase::Quality,
            vec![
                "cargo".into(),
                "clippy".into(),
                "--workspace".into(),
                "--all-targets".into(),
                "--locked".into(),
            ],
            root.clone(),
            env,
            profile.timeouts.quality_seconds,
            ExpectedExit::Zero,
            vec![format!("{}/quality/clippy.log", out_dir.display())],
        )?);
    }
    if profile.quality.run_build {
        let mut env = profile.extra_env.clone();
        env.insert("RUSTFLAGS".into(), "-D warnings".into());
        commands.push(ValidationCommand::new(
            ValidationPhase::Quality,
            vec![
                "cargo".into(),
                "build".into(),
                "--workspace".into(),
                "--all-targets".into(),
                "--locked".into(),
            ],
            root.clone(),
            env,
            profile.timeouts.quality_seconds,
            ExpectedExit::Zero,
            vec![format!("{}/quality/build.log", out_dir.display())],
        )?);
    }
    if profile.quality.run_tests {
        commands.push(make_cmd(
            ValidationPhase::Quality,
            vec!["cargo", "test", "--workspace", "--locked"],
            &root,
            profile.timeouts.quality_seconds,
            &profile.extra_env,
            vec![format!("{}/quality/test.log", out_dir.display())],
        )?);
    }
    if profile.quality.run_doc {
        let mut env = profile.extra_env.clone();
        env.insert("RUSTDOCFLAGS".into(), "-D warnings".into());
        commands.push(ValidationCommand::new(
            ValidationPhase::Quality,
            vec![
                "cargo".into(),
                "doc".into(),
                "--workspace".into(),
                "--no-deps".into(),
                "--locked".into(),
            ],
            root.clone(),
            env,
            profile.timeouts.quality_seconds,
            ExpectedExit::Zero,
            vec![format!("{}/quality/doc.log", out_dir.display())],
        )?);
    }

    // L1: Benchmarks
    if profile.benchmarks.run_bench_full {
        commands.push(make_cmd(
            ValidationPhase::Benchmark,
            vec!["cargo", "run", "--release", "--example", "bench_full", "-p", "pse-core"],
            &root,
            profile.timeouts.benchmark_seconds,
            &profile.extra_env,
            vec![format!("{}/bench/bench_full.log", out_dir.display())],
        )?);
    }
    if profile.benchmarks.run_pse_demo {
        commands.push(make_cmd(
            ValidationPhase::Benchmark,
            vec!["cargo", "run", "--release", "-p", "pse-demo"],
            &root,
            profile.timeouts.benchmark_seconds,
            &profile.extra_env,
            vec![format!("{}/bench/pse_demo.log", out_dir.display())],
        )?);
    }
    if profile.benchmarks.run_bench_gt {
        commands.push(make_cmd(
            ValidationPhase::Benchmark,
            vec!["cargo", "run", "--release", "-p", "pse-bench-gt", "--bin", "bench_gt"],
            &root,
            profile.timeouts.benchmark_seconds,
            &profile.extra_env,
            vec![format!("{}/bench/bench_gt.log", out_dir.display())],
        )?);
    }

    // L2: Eval-Matrix presets
    for preset in &profile.eval_matrix.presets {
        let eval_dir = format!("{}/eval/{preset}", out_dir.display());
        commands.push(make_cmd(
            ValidationPhase::EvalMatrix,
            vec!["pse-eval-matrix", "init", "--template", preset, "--out", &format!("{eval_dir}/spec.json")],
            &root,
            profile.timeouts.eval_preset_seconds,
            &profile.extra_env,
            vec![format!("{eval_dir}/spec.json")],
        )?);
        commands.push(make_cmd(
            ValidationPhase::EvalMatrix,
            vec!["pse-eval-matrix", "plan", "--spec", &format!("{eval_dir}/spec.json"), "--out", &format!("{eval_dir}/plan.json")],
            &root,
            profile.timeouts.eval_preset_seconds,
            &profile.extra_env,
            vec![format!("{eval_dir}/plan.json")],
        )?);
        commands.push(make_cmd(
            ValidationPhase::EvalMatrix,
            vec!["pse-eval-matrix", "run", "--spec", &format!("{eval_dir}/spec.json"), "--plan", &format!("{eval_dir}/plan.json"), "--out", &format!("{eval_dir}/bundle.json")],
            &root,
            profile.timeouts.eval_preset_seconds,
            &profile.extra_env,
            vec![format!("{eval_dir}/bundle.json")],
        )?);
        commands.push(make_cmd(
            ValidationPhase::EvalMatrix,
            vec!["pse-eval-matrix", "replay", "--bundle", &format!("{eval_dir}/bundle.json"), "--out", &format!("{eval_dir}/replay.json")],
            &root,
            profile.timeouts.eval_preset_seconds,
            &profile.extra_env,
            vec![format!("{eval_dir}/replay.json")],
        )?);
        commands.push(make_cmd(
            ValidationPhase::EvalMatrix,
            vec!["pse-eval-matrix", "score", "--spec", &format!("{eval_dir}/spec.json"), "--bundle", &format!("{eval_dir}/bundle.json"), "--out", &format!("{eval_dir}/summary.json")],
            &root,
            profile.timeouts.eval_preset_seconds,
            &profile.extra_env,
            vec![format!("{eval_dir}/summary.json")],
        )?);
        commands.push(make_cmd(
            ValidationPhase::EvalMatrix,
            vec!["pse-eval-matrix", "report", "--summary", &format!("{eval_dir}/summary.json"), "--format", "md", "--out", &format!("{eval_dir}/summary.md")],
            &root,
            profile.timeouts.eval_preset_seconds,
            &profile.extra_env,
            vec![format!("{eval_dir}/summary.md")],
        )?);
    }

    // L3: Domain (optional)
    if let Some(domain) = &profile.domain {
        let domain_dir = format!("{}/domain", out_dir.display());
        commands.push(make_cmd(
            ValidationPhase::DomainCalibration,
            vec!["true"], // domain calibration is external; runner records manifest
            &root,
            profile.timeouts.domain_seconds,
            &profile.extra_env,
            vec![format!("{domain_dir}/calibration_report.json")],
        )?);
        let _ = domain;
    }

    // Scoring and Reporting
    commands.push(make_cmd(
        ValidationPhase::Scoring,
        vec!["true"], // scoring is done internally by the runner
        &root,
        profile.timeouts.default_seconds,
        &profile.extra_env,
        vec![format!("{}/final/conclusion.json", out_dir.display())],
    )?);
    commands.push(make_cmd(
        ValidationPhase::Reporting,
        vec!["true"],
        &root,
        profile.timeouts.default_seconds,
        &profile.extra_env,
        vec![
            format!("{}/final/final_report.json", out_dir.display()),
            format!("{}/final/final_report.md", out_dir.display()),
        ],
    )?);

    let partial = CommandPlan {
        plan_id: Hash256::zero(),
        profile_id: profile.profile_id.clone(),
        commands,
    };
    let plan_id = content_address(&partial)?;
    Ok(CommandPlan { plan_id, ..partial })
}

fn make_cmd(
    phase: ValidationPhase,
    argv_str: Vec<&str>,
    working_dir: &str,
    timeout_seconds: u64,
    extra_env: &BTreeMap<String, String>,
    output_paths: Vec<String>,
) -> Result<ValidationCommand, ValidationError> {
    ValidationCommand::new(
        phase,
        argv_str.into_iter().map(String::from).collect(),
        working_dir.to_string(),
        extra_env.clone(),
        timeout_seconds,
        ExpectedExit::Zero,
        output_paths,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_plan_is_deterministic() {
        let profile = ValidationProfile::smoke().unwrap();
        let root = Path::new("/repo");
        let out = Path::new("/out/run_001");
        let p1 = build_command_plan(&profile, root, out).unwrap();
        let p2 = build_command_plan(&profile, root, out).unwrap();
        assert_eq!(p1.plan_id, p2.plan_id);
    }

    #[test]
    fn smoke_plan_has_at_least_one_command() {
        let profile = ValidationProfile::smoke().unwrap();
        let root = Path::new("/repo");
        let out = Path::new("/out/run_001");
        let plan = build_command_plan(&profile, root, out).unwrap();
        assert!(!plan.commands.is_empty());
    }
}
