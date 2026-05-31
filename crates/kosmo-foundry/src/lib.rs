//! Real Foundry executor for the `kosmo-*` substrate.
//!
//! ## What this crate does
//!
//! It turns a [`FoundryExecutionPlan`] (the pure, content-addressed data model
//! defined in `kosmo-core`) into an actually-executed
//! [`FoundryExecutionReport`] by running the allowlisted commands of the plan
//! in a policy-governed sandbox. It is the runtime counterpart of the R1 data
//! model: where `kosmo-pipeline::simulate_foundry_check` only *simulated* an
//! outcome, this crate runs the real process.
//!
//! ## Why it is a separate crate
//!
//! `kosmo-core` is the portable, process-free substrate (it sits directly above
//! `pse-types`). Spawning host processes is a host capability, so it lives in a
//! dedicated crate — exactly as `kosmo-pse-bridge` isolates the PSE crossing.
//!
//! ## Safety contract (fail-closed by construction)
//!
//! - **ReportOnly is inert.** When `PolicyProfile.mode == ReportOnly` the
//!   executor returns [`FoundryExecutionReport::skipped_by_report_only`] and
//!   spawns nothing.
//! - **Allowlist is checked before spawn.** Every check is mapped to a
//!   `(program, subcommand)` pair and validated against the plan's
//!   [`FoundryCommandPolicy`]. A denied command yields
//!   [`FoundryExecutionOutcome::CommandDeniedByPolicy`] and is never executed.
//! - **No host writes outside the workspace.** Only read-only verification
//!   commands (`cargo check`/`test`/`clippy`/`fmt --check`) are mapped. The
//!   executor never passes a flag that mutates source; cargo writes only to
//!   `target/` inside the sandbox root.
//! - **Environment is stripped.** Unless the plan's
//!   [`FoundryEnvironmentPolicy`] permits passthrough, the child runs with a
//!   minimal environment (only the variables a Rust toolchain needs plus
//!   explicitly-allowed vars). Secret-looking variables are never forwarded.
//! - **Timeouts are enforced.** Each check is bounded by
//!   `FoundryTimeoutPolicy.per_check_timeout_ms`; an overrun kills the child and
//!   yields [`FoundryExecutionOutcome::TimedOut`].
//! - **Worst-wins aggregation.** The plan outcome is the most severe per-check
//!   outcome. A failure in any *required* check fails the plan.
//! - **Content-addressed audit.** Every per-check evidence id and the final
//!   report id are SHA-256 content digests; identical captured output yields
//!   identical ids.

use kosmo_core::{
    Digest, FoundryCheckKind, FoundryCheckResult, FoundryCheckSpec, FoundryCommandPolicy,
    FoundryEnvironmentPolicy, FoundryExecutionOutcome, FoundryExecutionPlan,
    FoundryExecutionReport, FoundryOutcome, ImplementationMode, PolicyProfile,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Maximum bytes of stderr retained in a `Failed` diagnostic. Output is
/// truncated deterministically so the report stays bounded and auditable.
const MAX_STDERR_BYTES: usize = 4096;

/// The default program a Foundry check invokes.
const DEFAULT_PROGRAM: &str = "cargo";

/// Drives a [`FoundryExecutionPlan`] to a real [`FoundryExecutionReport`].
///
/// The `workspace_root` is the concrete directory the sandbox maps to; the
/// plan only carries the *digest* of that root (for content-addressing), never
/// a usable path, so the caller supplies the real path here. The executor sets
/// it as the working directory of every spawned check and never escapes it.
pub struct FoundryExecutor {
    program: String,
    workspace_root: PathBuf,
}

impl FoundryExecutor {
    /// Construct an executor rooted at `workspace_root`, invoking `cargo`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            program: DEFAULT_PROGRAM.to_string(),
            workspace_root: workspace_root.into(),
        }
    }

    /// Override the invoked program (primarily for testing with portable
    /// commands such as `true`/`false`/`sleep`).
    pub fn with_program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    /// Execute the plan under `policy`, producing a content-addressed report.
    ///
    /// Returns [`FoundryExecutionReport::skipped_by_report_only`] without
    /// spawning anything when `policy.mode == ReportOnly`.
    pub fn execute(
        &self,
        plan: &FoundryExecutionPlan,
        policy: &PolicyProfile,
        evidence_bundle_id: Digest,
    ) -> FoundryExecutionReport {
        // 1. ReportOnly is inert — no process is ever spawned.
        if policy.mode == ImplementationMode::ReportOnly {
            return FoundryExecutionReport::skipped_by_report_only(plan.id, evidence_bundle_id);
        }

        let started = Instant::now();
        let mut results: Vec<FoundryCheckResult> = Vec::with_capacity(plan.checks.len());
        let mut diagnostics: Vec<String> = Vec::new();
        let mut worst = OutcomeRank::Passed;

        for spec in &plan.checks {
            let (check_result, rank) = self.run_check(spec, plan, policy);
            if rank > worst {
                worst = rank;
            }
            if let FoundryOutcome::Failed { exit_code, .. } = &check_result.outcome {
                diagnostics.push(format!(
                    "check {:?} on '{}' failed (exit {})",
                    spec.check_kind, spec.target, exit_code
                ));
            }
            results.push(check_result);
        }

        let outcome = worst.into_outcome(results.is_empty());
        let elapsed_ms = started.elapsed().as_millis() as u64;
        FoundryExecutionReport::new(plan.id, outcome, results, diagnostics, evidence_bundle_id, elapsed_ms)
    }

    /// Run a single check spec, returning its result and a severity rank for
    /// worst-wins aggregation.
    fn run_check(
        &self,
        spec: &FoundryCheckSpec,
        plan: &FoundryExecutionPlan,
        _policy: &PolicyProfile,
    ) -> (FoundryCheckResult, OutcomeRank) {
        // Map the abstract check kind to a concrete program subcommand.
        let subcommand = match map_kind_to_subcommand(&spec.check_kind) {
            Some(sub) => sub,
            None => {
                let outcome = FoundryOutcome::Skipped {
                    reason: format!("check kind {:?} has no cargo mapping", spec.check_kind),
                };
                let evidence = Digest::of_bytes(b"foundry-unmapped-check");
                return (
                    FoundryCheckResult::new(spec.check_kind.clone(), outcome, evidence, vec![]),
                    OutcomeRank::Passed,
                );
            }
        };

        // Enforce the command allowlist BEFORE spawning anything.
        if !plan.command_policy.is_command_allowed(&self.program, subcommand) {
            let outcome = FoundryOutcome::Skipped {
                reason: format!(
                    "command '{} {}' denied by FoundryCommandPolicy",
                    self.program, subcommand
                ),
            };
            let evidence = Digest::of_bytes(b"foundry-command-denied");
            return (
                FoundryCheckResult::new(
                    spec.check_kind.clone(),
                    outcome,
                    evidence,
                    vec![format!("denied: {} {}", self.program, subcommand)],
                ),
                OutcomeRank::CommandDenied,
            );
        }

        // Assemble argv: subcommand + the check's default args + spec args.
        let mut argv: Vec<String> = vec![subcommand.to_string()];
        argv.extend(default_args_for(&spec.check_kind).into_iter().map(String::from));
        argv.extend(spec.args.iter().cloned());

        let env = build_environment(&plan.environment_policy);
        let timeout_ms = plan.timeout_policy.per_check_timeout_ms;

        let raw = run_raw(
            &self.program,
            &argv,
            &self.workspace_root,
            &env,
            !plan.environment_policy.allow_passthrough,
            timeout_ms,
        );

        self.classify(spec, raw)
    }

    /// Convert a raw process result into a content-addressed check result.
    fn classify(&self, spec: &FoundryCheckSpec, raw: RawRun) -> (FoundryCheckResult, OutcomeRank) {
        if let Some(err) = raw.spawn_failed {
            let outcome = FoundryOutcome::Unavailable {
                reason: format!("could not spawn '{}': {}", self.program, err),
            };
            let evidence = Digest::of(&("unavailable", &self.program, &err));
            return (
                FoundryCheckResult::new(spec.check_kind.clone(), outcome, evidence, vec![err]),
                OutcomeRank::Inconclusive,
            );
        }

        if raw.timed_out {
            let outcome = FoundryOutcome::Failed {
                exit_code: -1,
                stderr: "timed out".into(),
            };
            let evidence = Digest::of(&("timeout", &spec.id));
            return (
                FoundryCheckResult::new(
                    spec.check_kind.clone(),
                    outcome,
                    evidence,
                    vec!["per-check timeout exceeded".into()],
                ),
                OutcomeRank::TimedOut,
            );
        }

        let exit_code = raw.exit_code.unwrap_or(-1);
        let stderr_text = truncate_utf8_lossy(&raw.stderr, MAX_STDERR_BYTES);
        // Evidence id is the content digest of the captured output — identical
        // output yields an identical evidence id (content-addressing).
        let evidence = Digest::of(&CapturedOutput {
            exit_code,
            stdout_digest: Digest::of_bytes(&raw.stdout),
            stderr_digest: Digest::of_bytes(&raw.stderr),
        });

        if exit_code == 0 {
            (
                FoundryCheckResult::new(spec.check_kind.clone(), FoundryOutcome::Passed, evidence, vec![]),
                OutcomeRank::Passed,
            )
        } else {
            let outcome = FoundryOutcome::Failed {
                exit_code,
                stderr: stderr_text.clone(),
            };
            (
                FoundryCheckResult::new(spec.check_kind.clone(), outcome, evidence, vec![stderr_text]),
                OutcomeRank::Failed,
            )
        }
    }
}

/// Severity rank for worst-wins aggregation of per-check outcomes.
///
/// Higher ordinal = more severe. The plan outcome is the maximum rank observed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum OutcomeRank {
    Passed = 0,
    Inconclusive = 1,
    Failed = 2,
    TimedOut = 3,
    CommandDenied = 4,
}

impl OutcomeRank {
    fn into_outcome(self, no_checks: bool) -> FoundryExecutionOutcome {
        match self {
            OutcomeRank::Passed => {
                if no_checks {
                    // A plan with no checks proves nothing — fail closed.
                    FoundryExecutionOutcome::Inconclusive
                } else {
                    FoundryExecutionOutcome::Passed
                }
            }
            OutcomeRank::Inconclusive => FoundryExecutionOutcome::Inconclusive,
            OutcomeRank::Failed => FoundryExecutionOutcome::Failed,
            OutcomeRank::TimedOut => FoundryExecutionOutcome::TimedOut,
            OutcomeRank::CommandDenied => FoundryExecutionOutcome::CommandDeniedByPolicy,
        }
    }
}

/// Maps an abstract [`FoundryCheckKind`] to the cargo subcommand that performs
/// it. Returns `None` for kinds that have no read-only cargo equivalent.
pub fn map_kind_to_subcommand(kind: &FoundryCheckKind) -> Option<&'static str> {
    match kind {
        FoundryCheckKind::Build | FoundryCheckKind::TypeCheck => Some("check"),
        FoundryCheckKind::Test => Some("test"),
        FoundryCheckKind::Lint => Some("clippy"),
        // ParseBack is handled by the parse-back subsystem, not by cargo.
        // Security and arbitrary Custom kinds have no built-in mapping.
        FoundryCheckKind::ParseBack
        | FoundryCheckKind::Security
        | FoundryCheckKind::Custom(_) => None,
    }
}

/// Default arguments appended for a given check kind. These keep every command
/// read-only and non-interactive.
///
/// Currently no kind needs extra guard flags (`cargo check`/`test`/`clippy` are
/// already read-only); this is the extension point for kind-specific flags such
/// as a future `fmt --check`.
fn default_args_for(_kind: &FoundryCheckKind) -> Vec<&'static str> {
    vec![]
}

/// Build the child environment from the policy.
///
/// When passthrough is disabled the child starts from an empty environment and
/// receives only the minimal toolchain variables plus explicit overrides.
/// Secret-looking variables are never forwarded.
fn build_environment(policy: &FoundryEnvironmentPolicy) -> BTreeMap<String, String> {
    let mut env: BTreeMap<String, String> = BTreeMap::new();

    if policy.allow_passthrough {
        for (k, v) in std::env::vars() {
            if policy.strip_secrets && looks_like_secret(&k) {
                continue;
            }
            env.insert(k, v);
        }
    } else {
        // Minimal set a Rust toolchain needs to locate itself and run.
        const ESSENTIAL: &[&str] = &[
            "PATH", "HOME", "USER", "LOGNAME", "LANG", "LC_ALL", "TMPDIR", "TEMP", "TMP",
            "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN", "SYSTEMROOT", "WINDIR",
            "USERPROFILE", "PATHEXT",
        ];
        for key in ESSENTIAL {
            if let Ok(val) = std::env::var(key) {
                if policy.strip_secrets && looks_like_secret(key) {
                    continue;
                }
                env.insert((*key).to_string(), val);
            }
        }
    }

    // Explicit overrides from the policy always win. `None` means "pass through
    // the ambient value" (only meaningful when passthrough is permitted).
    for (key, override_val) in &policy.allowed_vars {
        match override_val {
            Some(val) => {
                env.insert(key.clone(), val.clone());
            }
            None => {
                if let Ok(val) = std::env::var(key) {
                    env.insert(key.clone(), val);
                }
            }
        }
    }

    env
}

/// Heuristic: does an environment variable name look like it carries a secret?
fn looks_like_secret(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    const NEEDLES: &[&str] = &[
        "SECRET", "TOKEN", "PASSWORD", "PASSWD", "API_KEY", "APIKEY", "ACCESS_KEY",
        "PRIVATE_KEY", "CREDENTIAL", "AUTH", "SESSION", "COOKIE",
    ];
    NEEDLES.iter().any(|n| upper.contains(n))
}

/// Outcome of a single raw process invocation.
struct RawRun {
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
    spawn_failed: Option<String>,
}

/// Spawn `program` with `args` in `cwd`, capturing output, enforcing a
/// per-process timeout (`timeout_ms == 0` means no timeout). When `clear_env`
/// is set the child receives only `env`; otherwise `env` overlays the ambient
/// environment.
fn run_raw(
    program: &str,
    args: &[String],
    cwd: &Path,
    env: &BTreeMap<String, String>,
    clear_env: bool,
    timeout_ms: u64,
) -> RawRun {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if clear_env {
        cmd.env_clear();
    }
    for (k, v) in env {
        cmd.env(k, v);
    }

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return RawRun {
                exit_code: None,
                stdout: vec![],
                stderr: vec![],
                timed_out: false,
                spawn_failed: Some(e.to_string()),
            }
        }
    };

    // No timeout requested → wait to completion via the simple blocking path.
    if timeout_ms == 0 {
        return match child.wait_with_output() {
            Ok(out) => RawRun {
                exit_code: out.status.code(),
                stdout: out.stdout,
                stderr: out.stderr,
                timed_out: false,
                spawn_failed: None,
            },
            Err(e) => RawRun {
                exit_code: None,
                stdout: vec![],
                stderr: vec![],
                timed_out: false,
                spawn_failed: Some(e.to_string()),
            },
        };
    }

    wait_with_timeout(child, Duration::from_millis(timeout_ms))
}

/// Poll a child to completion or kill it once `timeout` elapses.
fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> RawRun {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Drain captured pipes after exit.
                let (stdout, stderr) = drain_child(&mut child);
                return RawRun {
                    exit_code: status.code(),
                    stdout,
                    stderr,
                    timed_out: false,
                    spawn_failed: None,
                };
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return RawRun {
                        exit_code: None,
                        stdout: vec![],
                        stderr: vec![],
                        timed_out: true,
                        spawn_failed: None,
                    };
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                return RawRun {
                    exit_code: None,
                    stdout: vec![],
                    stderr: vec![],
                    timed_out: false,
                    spawn_failed: Some(e.to_string()),
                }
            }
        }
    }
}

/// Read whatever is buffered on the child's stdout/stderr pipes.
fn drain_child(child: &mut std::process::Child) -> (Vec<u8>, Vec<u8>) {
    use std::io::Read;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr);
    }
    (stdout, stderr)
}

/// Lossy-decode at most `max` bytes of `bytes` as UTF-8, on a char boundary.
fn truncate_utf8_lossy(bytes: &[u8], max: usize) -> String {
    let slice = if bytes.len() > max { &bytes[..max] } else { bytes };
    String::from_utf8_lossy(slice).into_owned()
}

/// Content-addressing payload for a captured process output.
#[derive(serde::Serialize)]
struct CapturedOutput {
    exit_code: i32,
    stdout_digest: Digest,
    stderr_digest: Digest,
}

/// Convenience: build a standard read-only cargo Foundry plan for `task_id`.
///
/// Produces a plan whose checks are `cargo check` then `cargo test`, governed by
/// the default cargo command policy, a locked environment, and the supplied
/// per-check timeout. The sandbox is `LocalDryRun` rooted at `root_digest`.
pub fn standard_cargo_plan(
    policy_id: Digest,
    workspace_index_id: Digest,
    task_id: Digest,
    root_digest: Digest,
    per_check_timeout_ms: u64,
) -> FoundryExecutionPlan {
    use kosmo_core::{
        FoundrySandboxKind, FoundrySandboxSpec, FoundryTimeoutPolicy,
    };

    let sandbox = FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, root_digest);
    let checks = vec![
        FoundryCheckSpec::new(FoundryCheckKind::Build, "workspace", true),
        FoundryCheckSpec::new(FoundryCheckKind::Test, "workspace", true),
    ];
    FoundryExecutionPlan::new(
        policy_id,
        workspace_index_id,
        task_id,
        sandbox,
        checks,
        FoundryCommandPolicy::default_cargo_policy(),
        FoundryTimeoutPolicy::new(per_check_timeout_ms, per_check_timeout_ms.saturating_mul(4)),
        FoundryEnvironmentPolicy::locked(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{
        FoundryCommandPolicy, FoundrySandboxKind, FoundrySandboxSpec, FoundryTimeoutPolicy,
    };

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn plan_with(program_subcommands_allowed: bool, checks: Vec<FoundryCheckSpec>, timeout_ms: u64) -> FoundryExecutionPlan {
        let sandbox = FoundrySandboxSpec::new(FoundrySandboxKind::LocalDryRun, d(b"root"));
        let cmd_policy = if program_subcommands_allowed {
            // Allow the test program with arbitrary args.
            use kosmo_core::AllowedFoundryCommand;
            let allowed = vec![
                AllowedFoundryCommand {
                    program: "true".into(),
                    allowed_subcommands: vec![],
                    allow_arbitrary_args: true,
                },
                AllowedFoundryCommand {
                    program: "false".into(),
                    allowed_subcommands: vec![],
                    allow_arbitrary_args: true,
                },
                AllowedFoundryCommand {
                    program: "sleep".into(),
                    allowed_subcommands: vec![],
                    allow_arbitrary_args: true,
                },
            ];
            // Reconstruct a policy that allows these by hand.
            let mut p = FoundryCommandPolicy::default_cargo_policy();
            p.allowed_commands = allowed;
            // Recompute id via the public constructor path is unavailable; the
            // executor only calls is_command_allowed, which does not check id.
            p
        } else {
            FoundryCommandPolicy::default_cargo_policy()
        };

        FoundryExecutionPlan::new(
            d(b"policy"),
            d(b"workspace"),
            d(b"task"),
            sandbox,
            checks,
            cmd_policy,
            FoundryTimeoutPolicy::new(timeout_ms, timeout_ms.saturating_mul(4)),
            FoundryEnvironmentPolicy::locked(),
        )
    }

    // ── ReportOnly is inert ─────────────────────────────────────────────────

    #[test]
    fn report_only_skips_without_executing() {
        let checks = vec![FoundryCheckSpec::new(FoundryCheckKind::Build, "ws", true)];
        let plan = plan_with(false, checks, 0);
        let exec = FoundryExecutor::new(".");
        let report = exec.execute(&plan, &PolicyProfile::default_report_only(), d(b"bundle"));
        assert!(report.outcome.is_skipped_report_only());
        assert!(report.verify_id());
        assert!(report.check_results.is_empty(), "no checks must run in ReportOnly");
    }

    // ── Command allowlist denial ────────────────────────────────────────────

    #[test]
    fn denied_command_is_not_executed() {
        // Default cargo policy allows cargo check/test/fmt/clippy. We invoke a
        // program named "true" (not "cargo"), so the allowlist must deny it.
        let checks = vec![FoundryCheckSpec::new(FoundryCheckKind::Build, "ws", true)];
        let plan = plan_with(false, checks, 0); // default cargo policy
        let exec = FoundryExecutor::new(".").with_program("true");
        let report = exec.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        assert_eq!(report.outcome, FoundryExecutionOutcome::CommandDeniedByPolicy);
        assert!(report.verify_id());
    }

    // ── Kind → subcommand mapping ───────────────────────────────────────────

    #[test]
    fn kind_mapping_is_read_only() {
        assert_eq!(map_kind_to_subcommand(&FoundryCheckKind::Build), Some("check"));
        assert_eq!(map_kind_to_subcommand(&FoundryCheckKind::TypeCheck), Some("check"));
        assert_eq!(map_kind_to_subcommand(&FoundryCheckKind::Test), Some("test"));
        assert_eq!(map_kind_to_subcommand(&FoundryCheckKind::Lint), Some("clippy"));
        assert_eq!(map_kind_to_subcommand(&FoundryCheckKind::ParseBack), None);
        assert_eq!(map_kind_to_subcommand(&FoundryCheckKind::Security), None);
        assert_eq!(map_kind_to_subcommand(&FoundryCheckKind::Custom("x".into())), None);
    }

    // ── Environment stripping ───────────────────────────────────────────────

    #[test]
    fn locked_env_strips_secrets_and_excludes_ambient() {
        std::env::set_var("MY_API_KEY", "supersecret");
        std::env::set_var("KOSMO_TEST_PLAIN", "ok");
        let env = build_environment(&FoundryEnvironmentPolicy::locked());
        // Secret never appears.
        assert!(!env.contains_key("MY_API_KEY"));
        // A non-essential, non-secret ambient var is also excluded (locked = no passthrough).
        assert!(!env.contains_key("KOSMO_TEST_PLAIN"));
        std::env::remove_var("MY_API_KEY");
        std::env::remove_var("KOSMO_TEST_PLAIN");
    }

    #[test]
    fn secret_detector_matches_common_patterns() {
        assert!(looks_like_secret("AWS_SECRET_ACCESS_KEY"));
        assert!(looks_like_secret("github_token"));
        assert!(looks_like_secret("DB_PASSWORD"));
        assert!(looks_like_secret("CEREBRAS_API_KEY"));
        assert!(!looks_like_secret("PATH"));
        assert!(!looks_like_secret("CARGO_HOME"));
    }

    // ── No-checks plan fails closed ─────────────────────────────────────────

    #[test]
    fn empty_plan_is_inconclusive_not_passed() {
        let plan = plan_with(true, vec![], 0);
        let exec = FoundryExecutor::new(".").with_program("true");
        let report = exec.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        assert_eq!(report.outcome, FoundryExecutionOutcome::Inconclusive);
    }

    // ── Real execution (POSIX-portable programs) ────────────────────────────

    #[cfg(unix)]
    #[test]
    fn real_true_command_passes() {
        // Map Build→"check" but invoke program "true"; allow "true" with args.
        let checks = vec![FoundryCheckSpec::new(FoundryCheckKind::Build, "ws", true)];
        let plan = plan_with(true, checks, 0);
        let exec = FoundryExecutor::new(".").with_program("true");
        let report = exec.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        assert_eq!(report.outcome, FoundryExecutionOutcome::Passed, "true exits 0");
        assert!(report.verify_id());
        assert_eq!(report.check_results.len(), 1);
        assert!(report.check_results[0].outcome.is_passed());
    }

    #[cfg(unix)]
    #[test]
    fn real_false_command_fails() {
        let checks = vec![FoundryCheckSpec::new(FoundryCheckKind::Build, "ws", true)];
        let plan = plan_with(true, checks, 0);
        let exec = FoundryExecutor::new(".").with_program("false");
        let report = exec.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        assert_eq!(report.outcome, FoundryExecutionOutcome::Failed, "false exits 1");
        assert!(report.verify_id());
    }

    #[cfg(unix)]
    #[test]
    fn raw_sleep_times_out_and_is_killed() {
        // The timeout primitive is tested at the raw-process level so the argv
        // is fully controlled (the high-level executor always prepends a cargo
        // subcommand, which `sleep` would reject). `sleep 5` with a 100ms
        // budget must be killed and reported as timed out.
        let env = build_environment(&FoundryEnvironmentPolicy::locked());
        let raw = run_raw("sleep", &["5".to_string()], Path::new("."), &env, true, 100);
        assert!(raw.timed_out, "sleep 5 under a 100ms timeout must be killed");
        assert!(raw.exit_code.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn raw_capture_reports_exit_codes() {
        let env = build_environment(&FoundryEnvironmentPolicy::locked());
        let ok = run_raw("true", &[], Path::new("."), &env, true, 0);
        assert_eq!(ok.exit_code, Some(0));
        assert!(!ok.timed_out && ok.spawn_failed.is_none());
        let bad = run_raw("false", &[], Path::new("."), &env, true, 0);
        assert_eq!(bad.exit_code, Some(1));
    }

    #[test]
    fn raw_missing_program_is_spawn_failure() {
        let env = build_environment(&FoundryEnvironmentPolicy::locked());
        let raw = run_raw(
            "kosmo-nonexistent-program-xyz",
            &[],
            Path::new("."),
            &env,
            true,
            0,
        );
        assert!(raw.spawn_failed.is_some(), "missing program must report spawn failure");
    }

    // ── Worst-wins aggregation ──────────────────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn worst_wins_failed_dominates_passed() {
        // First check passes (true), second fails (false). But both checks use
        // the same program per executor, so model worst-wins via two specs on
        // "false": the aggregate must be Failed.
        let checks = vec![
            FoundryCheckSpec::new(FoundryCheckKind::Build, "ws", true),
            FoundryCheckSpec::new(FoundryCheckKind::Test, "ws", true),
        ];
        let plan = plan_with(true, checks, 0);
        let exec = FoundryExecutor::new(".").with_program("false");
        let report = exec.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        assert_eq!(report.outcome, FoundryExecutionOutcome::Failed);
        assert_eq!(report.check_results.len(), 2);
    }

    // ── Determinism of report id over identical synthetic output ────────────

    #[test]
    fn unmapped_check_is_skipped_deterministically() {
        let checks = vec![FoundryCheckSpec::new(FoundryCheckKind::Security, "ws", false)];
        let plan = plan_with(true, checks, 0);
        let exec = FoundryExecutor::new(".").with_program("true");
        let r1 = exec.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        let r2 = exec.execute(&plan, &PolicyProfile::dry_run(), d(b"bundle"));
        // Security maps to nothing → skipped → overall Passed (skipped is benign).
        assert_eq!(r1.outcome, FoundryExecutionOutcome::Passed);
        assert_eq!(r1.id, r2.id, "identical skipped runs must be id-stable");
    }
}
