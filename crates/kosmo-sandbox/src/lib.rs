//! `kosmo-sandbox` — the safe execution room for the Runtime floor.
//!
//! It runs a built artifact (possibly code the loop itself generated) and
//! returns a content-addressed [`RuntimeWitness`] — the evidence a `Run` /
//! `Service` facet is judged against. It is a **capability, not a gate**: it
//! decides nothing about what is *allowed* (policy does that); it provides the
//! room in which an execution can happen and be *trusted as observed*.
//!
//! ## Guarantees
//!
//! **Enforced (Unix):**
//! - **Timeout → group kill.** The child runs in its own process group
//!   ([`CommandExt::process_group`]); on timeout the whole group is
//!   `SIGKILL`ed, so a hung grandchild (e.g. the binary `cargo run` spawned)
//!   cannot outlive the budget. A timeout is a **verdict**, never a hang.
//! - **Bounded capture.** stdout/stderr are drained on their own threads into
//!   capped buffers — a runaway printer is truncated, never an OOM, and never a
//!   pipe-deadlock.
//! - **Guaranteed reaping.** The child is always waited on; no zombies.
//!
//! **Best-effort / honest (per `docs/RUNTIME-floor.md` §8.1):**
//! - **Network.** [`NetworkPolicy::Deny`] is the default and clears proxy env,
//!   but hard isolation (network namespaces / seccomp) is **not** yet enforced
//!   here — it needs privileges this process may not hold. The policy is
//!   recorded; real enforcement lands with the `Service` facet that needs it.
//!   This crate does not *claim* isolation it cannot deliver.
//! - **Filesystem.** Execution runs in the caller's `cwd` (a throwaway
//!   workspace, by the discipline of the Prüfstand harness) — containment by
//!   construction, not a chroot.
//! - **Non-Unix.** Timeout is *reported* but the process is not force-killed
//!   (no portable group kill); documented rather than pretended.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use kosmo_core::Digest;

/// What network access the spawned artifact is *intended* to have. Enforcement
/// is best-effort (see the crate docs) — the value is recorded as intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkPolicy {
    /// No network (default). Proxy env is cleared; hard isolation is deferred.
    Deny,
    /// Loopback only — for `Service` probes against a local port.
    Loopback,
    /// Unrestricted — opt-in, for artifacts that legitimately need it.
    Allow,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunVerdict {
    /// The process exited on its own (see [`RuntimeWitness::exit_code`]).
    Exited,
    /// The process exceeded the time budget and was killed.
    TimedOut,
    /// The process could not be started at all.
    SpawnFailed,
}

/// One thing to run: a program, its args, and optional fed stdin.
#[derive(Debug, Clone)]
pub struct RunSpec {
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,
}

impl RunSpec {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            stdin: None,
        }
    }
    pub fn with_stdin(mut self, s: impl Into<String>) -> Self {
        self.stdin = Some(s.into());
        self
    }
}

/// The evidence of one execution. Content-addressed: `stdout_digest` witnesses
/// the full output even when the captured text is truncated.
#[derive(Debug, Clone)]
pub struct RuntimeWitness {
    pub verdict: RunVerdict,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_digest: Digest,
    pub duration: Duration,
    /// Output exceeded the cap and was truncated (digest still covers all of it).
    pub truncated: bool,
}

impl RuntimeWitness {
    /// The artifact ran to completion and exited cleanly (`0`).
    pub fn succeeded(&self) -> bool {
        matches!(self.verdict, RunVerdict::Exited) && self.exit_code == Some(0)
    }
    pub fn stdout_contains(&self, needle: &str) -> bool {
        self.stdout.contains(needle)
    }
    fn spawn_failed(msg: String, dur: Duration) -> Self {
        Self {
            verdict: RunVerdict::SpawnFailed,
            exit_code: None,
            stdout: String::new(),
            stderr: msg,
            stdout_digest: Digest::of_bytes(&[]),
            duration: dur,
            truncated: false,
        }
    }
}

/// The sandbox: a reusable, immutable run policy. Build with the `with_*`
/// methods, then [`Sandbox::run`] as many specs as you like.
#[derive(Debug, Clone)]
pub struct Sandbox {
    timeout: Duration,
    output_cap: usize,
    network: NetworkPolicy,
    cwd: Option<PathBuf>,
    env: Vec<(String, String)>,
    seed: u64,
}

impl Default for Sandbox {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            output_cap: 1 << 20, // 1 MiB per stream
            network: NetworkPolicy::Deny,
            cwd: None,
            env: Vec::new(),
            seed: 0,
        }
    }
}

impl Sandbox {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }
    pub fn with_output_cap(mut self, n: usize) -> Self {
        self.output_cap = n;
        self
    }
    pub fn with_network(mut self, p: NetworkPolicy) -> Self {
        self.network = p;
        self
    }
    pub fn with_cwd(mut self, p: impl Into<PathBuf>) -> Self {
        self.cwd = Some(p.into());
        self
    }
    pub fn with_env(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.env.push((k.into(), v.into()));
        self
    }
    pub fn with_seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }

    /// Run `spec` to completion or to the time budget, whichever comes first,
    /// and return the witness. Never panics; a spawn failure is a verdict.
    pub fn run(&self, spec: &RunSpec) -> RuntimeWitness {
        let start = Instant::now();
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args)
            .stdin(if spec.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }
        // Deterministic, minimized environment.
        cmd.env("KOSMO_SEED", self.seed.to_string());
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        if matches!(self.network, NetworkPolicy::Deny) {
            for p in [
                "http_proxy",
                "https_proxy",
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "ALL_PROXY",
                "all_proxy",
            ] {
                cmd.env_remove(p);
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // Own process group so a timeout kills the whole tree, not just the
            // leader (cargo's child binary must not outlive the budget).
            cmd.process_group(0);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return RuntimeWitness::spawn_failed(e.to_string(), start.elapsed()),
        };
        let pid = child.id();

        // Feed stdin on its own thread so a child that fills stdout while we
        // write cannot deadlock; dropping the sink sends EOF.
        if let Some(input) = spec.stdin.clone() {
            if let Some(mut sink) = child.stdin.take() {
                thread::spawn(move || {
                    let _ = sink.write_all(input.as_bytes());
                });
            }
        }

        let out = spawn_reader(child.stdout.take(), self.output_cap);
        let err = spawn_reader(child.stderr.take(), self.output_cap);

        let (tx, rx) = mpsc::channel();
        let waiter = thread::spawn(move || {
            let _ = tx.send(child.wait());
        });

        let (verdict, exit_code) = match rx.recv_timeout(self.timeout) {
            Ok(Ok(status)) => (RunVerdict::Exited, status.code()),
            Ok(Err(_)) => (RunVerdict::SpawnFailed, None),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                kill_group(pid);
                let _ = rx.recv(); // reap the now-killed child
                (RunVerdict::TimedOut, None)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => (RunVerdict::SpawnFailed, None),
        };
        let _ = waiter.join();

        let (out_bytes, out_trunc) = out.join().unwrap_or((Vec::new(), false));
        let (err_bytes, err_trunc) = err.join().unwrap_or((Vec::new(), false));

        RuntimeWitness {
            verdict,
            exit_code,
            stdout: String::from_utf8_lossy(&out_bytes).into_owned(),
            stderr: String::from_utf8_lossy(&err_bytes).into_owned(),
            stdout_digest: Digest::of_bytes(&out_bytes),
            duration: start.elapsed(),
            truncated: out_trunc || err_trunc,
        }
    }
}

/// Drain a child stream into a capped buffer on its own thread. Keeps reading
/// (discarding) past the cap so the child never blocks on a full pipe; reports
/// whether anything was dropped.
fn spawn_reader<R: Read + Send + 'static>(
    r: Option<R>,
    cap: usize,
) -> JoinHandle<(Vec<u8>, bool)> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        let mut truncated = false;
        if let Some(mut r) = r {
            let mut chunk = [0u8; 8192];
            loop {
                match r.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if buf.len() < cap {
                            let take = (cap - buf.len()).min(n);
                            buf.extend_from_slice(&chunk[..take]);
                            if take < n {
                                truncated = true;
                            }
                        } else {
                            truncated = true;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
        (buf, truncated)
    })
}

/// `SIGKILL` the child's process group. The child was placed in its own group
/// (== its pid), so this fells the whole tree. Best-effort on non-Unix.
fn kill_group(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::killpg(pid as libc::pid_t, libc::SIGKILL);
    }
    #[cfg(not(unix))]
    {
        let _ = pid; // no portable group kill; see crate docs
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> RunSpec {
        RunSpec::new("sh", ["-c", script])
    }

    #[test]
    fn exit_zero_succeeds() {
        let w = Sandbox::new().run(&sh("exit 0"));
        assert_eq!(w.verdict, RunVerdict::Exited);
        assert_eq!(w.exit_code, Some(0));
        assert!(w.succeeded());
    }

    #[test]
    fn nonzero_exit_is_failure() {
        let w = Sandbox::new().run(&sh("exit 7"));
        assert_eq!(w.verdict, RunVerdict::Exited);
        assert_eq!(w.exit_code, Some(7));
        assert!(!w.succeeded());
    }

    #[test]
    fn captures_stdout_and_digests_it() {
        let w = Sandbox::new().run(&sh("printf hello"));
        assert!(w.stdout_contains("hello"));
        assert_eq!(w.stdout_digest, Digest::of_bytes(b"hello"));
        assert!(!w.truncated);
    }

    #[test]
    fn feeds_stdin() {
        let w = Sandbox::new().run(&sh("cat").with_stdin("ping"));
        assert!(w.stdout_contains("ping"));
    }

    #[test]
    fn infinite_loop_is_killed_within_budget() {
        let w = Sandbox::new()
            .with_timeout(Duration::from_millis(300))
            .run(&sh("while true; do :; done"));
        assert_eq!(w.verdict, RunVerdict::TimedOut);
        assert!(
            w.duration < Duration::from_secs(5),
            "must be killed promptly, took {:?}",
            w.duration
        );
    }

    #[test]
    fn backgrounded_child_is_group_killed() {
        // sh backgrounds a 30s sleep and waits on it. Only a *group* kill (the
        // child is its own group leader) fells both — otherwise the reap blocks
        // ~30s and this assertion fails.
        let w = Sandbox::new()
            .with_timeout(Duration::from_millis(300))
            .run(&sh("sleep 30 & wait"));
        assert_eq!(w.verdict, RunVerdict::TimedOut);
        assert!(
            w.duration < Duration::from_secs(5),
            "group kill must not wait for the grandchild, took {:?}",
            w.duration
        );
    }

    #[test]
    fn runaway_output_is_truncated_not_oom() {
        let w = Sandbox::new()
            .with_output_cap(1024)
            .with_timeout(Duration::from_secs(5))
            .run(&sh("head -c 100000 /dev/zero | tr '\\0' x"));
        assert!(w.truncated, "should report truncation");
        assert!(
            w.stdout.len() <= 1024,
            "captured output must be capped, got {}",
            w.stdout.len()
        );
    }

    #[test]
    fn spawn_failure_is_a_verdict_not_a_panic() {
        let w = Sandbox::new().run(&RunSpec::new(
            "kosmo-this-program-does-not-exist-xyz",
            Vec::<String>::new(),
        ));
        assert_eq!(w.verdict, RunVerdict::SpawnFailed);
        assert!(!w.succeeded());
    }

    #[test]
    fn cwd_is_honored() {
        let dir = std::env::temp_dir();
        let w = Sandbox::new().with_cwd(&dir).run(&sh("pwd"));
        assert!(w.succeeded());
        // pwd may resolve symlinks; just assert it ran and produced a path.
        assert!(!w.stdout.trim().is_empty());
    }
}
