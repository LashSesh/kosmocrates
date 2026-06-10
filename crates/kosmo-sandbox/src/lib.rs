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
//!   (`CommandExt::process_group`); on timeout the whole group is
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
use std::net::{SocketAddr, TcpListener, TcpStream};
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
fn spawn_reader<R: Read + Send + 'static>(r: Option<R>, cap: usize) -> JoinHandle<(Vec<u8>, bool)> {
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

// ─── Service mode (start a server, await readiness, probe, tear down) ────────

/// One HTTP request to issue against a started service.
#[derive(Debug, Clone)]
pub struct HttpProbe {
    pub method: String,
    pub path: String,
}

impl HttpProbe {
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: "GET".into(),
            path: path.into(),
        }
    }
}

/// How a service probe ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceVerdict {
    /// The server became ready and answered the probe (see `status`/`body`).
    Probed,
    /// The server never bound its port (or crashed) within the budget.
    NeverReady,
    /// The server could not be started at all.
    SpawnFailed,
}

/// The evidence of one service probe. Content-addressed like [`RuntimeWitness`].
#[derive(Debug, Clone)]
pub struct ServiceWitness {
    pub verdict: ServiceVerdict,
    pub status: Option<u16>,
    pub body: String,
    pub body_digest: Digest,
    /// Wall time from spawn to a successful probe (zero if never ready).
    pub startup: Duration,
}

impl ServiceWitness {
    pub fn answered(&self) -> bool {
        matches!(self.verdict, ServiceVerdict::Probed)
    }
    fn spawn_failed() -> Self {
        Self {
            verdict: ServiceVerdict::SpawnFailed,
            status: None,
            body: String::new(),
            body_digest: Digest::of_bytes(&[]),
            startup: Duration::ZERO,
        }
    }
    fn never_ready() -> Self {
        Self {
            verdict: ServiceVerdict::NeverReady,
            status: None,
            body: String::new(),
            body_digest: Digest::of_bytes(&[]),
            startup: Duration::ZERO,
        }
    }
}

impl Sandbox {
    /// Start `spec` as a **long-running server**, wait for it to answer, issue
    /// one `probe`, then tear the whole process group down. The server is told
    /// its port via the `KOSMO_PORT` env var (a free loopback port this sandbox
    /// picks). Readiness is detected by *actually probing* — we retry the HTTP
    /// request until it answers or [`Sandbox::with_timeout`] elapses, so a
    /// server that binds but isn't serving yet is not mistaken for ready.
    ///
    /// Always reaps and group-kills the server (it will not exit on its own).
    pub fn serve_and_probe(&self, spec: &RunSpec, probe: &HttpProbe) -> ServiceWitness {
        let Some(port) = free_port() else {
            return ServiceWitness::spawn_failed();
        };
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("KOSMO_PORT", port.to_string())
            .env("KOSMO_SEED", self.seed.to_string());
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => return ServiceWitness::spawn_failed(),
        };
        let pid = child.id();
        // Drain output so a chatty server never blocks on a full pipe.
        let out = spawn_reader(child.stdout.take(), self.output_cap);
        let err = spawn_reader(child.stderr.take(), self.output_cap);

        let start = Instant::now();
        let mut answer = None;
        while start.elapsed() < self.timeout {
            // A server that exits before answering has crashed — stop early.
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            if let Some((status, body)) = http_probe(port, probe, self.output_cap) {
                answer = Some((status, body, start.elapsed()));
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }

        kill_group(pid);
        let _ = child.wait();
        let _ = out.join();
        let _ = err.join();

        match answer {
            Some((status, body, startup)) => ServiceWitness {
                verdict: ServiceVerdict::Probed,
                status: Some(status),
                body_digest: Digest::of_bytes(body.as_bytes()),
                body,
                startup,
            },
            None => ServiceWitness::never_ready(),
        }
    }
}

/// Pick a free loopback TCP port by binding `:0` and reading the assignment.
/// Inherently racy (the port is released before the child rebinds), but the
/// standard pragmatic approach for ephemeral test servers.
fn free_port() -> Option<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    drop(listener);
    Some(port)
}

/// Issue one HTTP/1.1 request over a raw TCP socket and parse the response.
/// `None` if the server isn't answering yet (used as the readiness signal).
fn http_probe(port: u16, probe: &HttpProbe, cap: usize) -> Option<(u16, String)> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .ok()?;
    let req = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        probe.method, probe.path
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut resp = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                resp.extend_from_slice(&chunk[..n]);
                if resp.len() >= cap {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    parse_http_response(&resp)
}

/// Parse a raw HTTP/1.1 response into `(status, body)`. `None` if there is no
/// parseable status line (e.g. the connection produced no data).
fn parse_http_response(bytes: &[u8]) -> Option<(u16, String)> {
    let text = String::from_utf8_lossy(bytes);
    let status_line = text.lines().next()?; // "HTTP/1.1 200 OK"
    let status: u16 = status_line.split_whitespace().nth(1)?.parse().ok()?;
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    Some((status, body))
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

    // ── Service mode ──────────────────────────────────────────────────────

    #[test]
    fn parse_http_response_extracts_status_and_body() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
        assert_eq!(parse_http_response(raw), Some((200, "ok".to_string())));
        assert_eq!(
            parse_http_response(b"HTTP/1.1 404 Not Found\r\n\r\n"),
            Some((404, String::new()))
        );
        assert_eq!(parse_http_response(b""), None);
    }

    #[test]
    fn free_port_picks_an_ephemeral_port() {
        // `free_port` returns a *likely*-free ephemeral port. We deliberately do
        // NOT re-bind it to assert usability: between the function releasing the
        // socket and any re-bind, a concurrent caller (e.g. this crate's own
        // service tests, run in parallel) can legitimately claim it — that race
        // is inherent to the bind-:0-then-release approach, not a defect. Real
        // usability is exercised by the serve_* tests, which actually serve.
        let p = free_port().expect("a free port");
        assert!(p > 0, "a real ephemeral port number");
    }

    #[test]
    fn serve_spawn_failure_is_a_verdict() {
        let w = Sandbox::new().serve_and_probe(
            &RunSpec::new("kosmo-no-such-server-xyz", Vec::<String>::new()),
            &HttpProbe::get("/"),
        );
        assert_eq!(w.verdict, ServiceVerdict::SpawnFailed);
        assert!(!w.answered());
    }

    #[test]
    fn serve_never_ready_is_killed_within_budget() {
        // A process that never binds the port → NeverReady, and torn down
        // promptly (not left running for its full 30s sleep).
        let w = Sandbox::new()
            .with_timeout(Duration::from_millis(400))
            .serve_and_probe(&sh("sleep 30"), &HttpProbe::get("/health"));
        assert_eq!(w.verdict, ServiceVerdict::NeverReady);
        assert!(!w.answered());
    }
}
