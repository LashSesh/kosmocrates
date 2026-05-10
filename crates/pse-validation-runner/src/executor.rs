//! CommandExecutor — runs ValidationCommands and records results.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::command_plan::ValidationCommand;
use crate::primitives::{Hash256, ValidationError};

/// Result of executing one command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub command_id: Hash256,
    pub exit_code: i32,
    pub stdout_hash: Hash256,
    pub stderr_hash: Hash256,
    pub stdout_len: u64,
    pub stderr_len: u64,
    /// Relative path where stdout was written (if captured to file).
    pub stdout_path: Option<String>,
}

/// Abstraction over command execution — injectable for tests.
pub trait CommandExecutor: Send + Sync {
    fn execute(
        &self,
        command: &ValidationCommand,
        log_dir: &Path,
    ) -> Result<CommandResult, ValidationError>;
}

/// Production executor: runs commands as child processes.
pub struct ProcessExecutor {
    pub dry_run: bool,
}

impl ProcessExecutor {
    pub fn new() -> Self {
        ProcessExecutor { dry_run: false }
    }
}

impl Default for ProcessExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor for ProcessExecutor {
    fn execute(
        &self,
        command: &ValidationCommand,
        log_dir: &Path,
    ) -> Result<CommandResult, ValidationError> {
        if self.dry_run {
            return Ok(CommandResult {
                command_id: command.command_id.clone(),
                exit_code: 0,
                stdout_hash: Hash256::zero(),
                stderr_hash: Hash256::zero(),
                stdout_len: 0,
                stderr_len: 0,
                stdout_path: None,
            });
        }

        let argv = &command.argv;
        if argv.is_empty() {
            return Err(ValidationError::CommandFailed { reason: "empty argv".into() });
        }

        let mut child = std::process::Command::new(&argv[0]);
        child.args(&argv[1..]);
        child.current_dir(&command.working_dir);
        for (k, v) in &command.env {
            child.env(k, v);
        }

        let timeout = Duration::from_secs(command.timeout_seconds);

        let output = run_with_timeout(child, timeout).map_err(|e| match e {
            RunError::Timeout => ValidationError::Timeout { command: argv[0].clone() },
            RunError::Io(msg) => ValidationError::CommandFailed { reason: msg },
        })?;

        let stdout_hash = sha256_hash(&output.stdout);
        let stderr_hash = sha256_hash(&output.stderr);
        let stdout_len = output.stdout.len() as u64;
        let stderr_len = output.stderr.len() as u64;

        // Write stdout to log file if log_dir exists.
        let stdout_path = write_log(log_dir, command, &output.stdout, &output.stderr);

        let exit_code = output.status.code().unwrap_or(-1);

        Ok(CommandResult {
            command_id: command.command_id.clone(),
            exit_code,
            stdout_hash,
            stderr_hash,
            stdout_len,
            stderr_len,
            stdout_path,
        })
    }
}

enum RunError {
    Timeout,
    Io(String),
}

fn run_with_timeout(
    mut cmd: std::process::Command,
    _timeout: Duration,
) -> Result<std::process::Output, RunError> {
    // Spawn and wait — timeout enforcement via OS-level timeout if available.
    // For now we do a simple blocking wait; a full timeout implementation
    // would use threads or a select loop.
    cmd.output().map_err(|e| RunError::Io(e.to_string()))
}

fn write_log(
    log_dir: &Path,
    command: &ValidationCommand,
    stdout: &[u8],
    stderr: &[u8],
) -> Option<String> {
    if !log_dir.exists() {
        return None;
    }
    // Use first output_path as the log filename, or derive from argv[0].
    let log_name = command
        .output_paths
        .first()
        .and_then(|p| std::path::Path::new(p).file_name()?.to_str().map(String::from))
        .unwrap_or_else(|| {
            command.argv.first().cloned().unwrap_or_else(|| "cmd".into()) + ".log"
        });

    let log_path = log_dir.join(&log_name);
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut content = stdout.to_vec();
    if !stderr.is_empty() {
        content.extend_from_slice(b"\n--- stderr ---\n");
        content.extend_from_slice(stderr);
    }
    std::fs::write(&log_path, &content).ok()?;
    Some(log_path.to_string_lossy().into_owned())
}

fn sha256_hash(data: &[u8]) -> Hash256 {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    Hash256::from_hex(&hex::encode(digest)).unwrap_or(Hash256::zero())
}

/// Mock executor for tests — returns configurable results.
#[cfg(any(test, feature = "validation-runner"))]
pub struct MockExecutor {
    /// Exit code to return for all commands.
    pub exit_code: i32,
}

#[cfg(any(test, feature = "validation-runner"))]
impl MockExecutor {
    pub fn success() -> Self {
        MockExecutor { exit_code: 0 }
    }

    pub fn failure() -> Self {
        MockExecutor { exit_code: 1 }
    }
}

#[cfg(any(test, feature = "validation-runner"))]
impl CommandExecutor for MockExecutor {
    fn execute(
        &self,
        command: &ValidationCommand,
        _log_dir: &Path,
    ) -> Result<CommandResult, ValidationError> {
        Ok(CommandResult {
            command_id: command.command_id.clone(),
            exit_code: self.exit_code,
            stdout_hash: Hash256::zero(),
            stderr_hash: Hash256::zero(),
            stdout_len: 0,
            stderr_len: 0,
            stdout_path: None,
        })
    }
}
