//! EnvironmentFingerprint — hardware and toolchain context (§7.3).
//!
//! Wall-clock time MUST NOT appear in any content-addressed hash.
//! The `timestamp_wallclock_excluded_from_hash` flag documents that
//! any timestamp is display-only.

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, Hash256, ValidationError};

/// Deterministic fingerprint of the execution environment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentFingerprint {
    pub os: String,
    pub arch: String,
    pub rustc_version: String,
    pub cargo_version: String,
    pub cpu_model: Option<String>,
    pub logical_cores: Option<u32>,
    pub memory_bytes: Option<u64>,
    /// Always true — signals that wallclock is display-only.
    pub timestamp_wallclock_excluded_from_hash: bool,
}

impl EnvironmentFingerprint {
    /// Collect current environment fingerprint.
    pub fn collect() -> Result<Self, ValidationError> {
        let rustc_version = run_tool("rustc", &["--version"])?;
        let cargo_version = run_tool("cargo", &["--version"])?;

        let logical_cores = std::thread::available_parallelism().ok().map(|n| n.get() as u32);

        let fp = EnvironmentFingerprint {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            rustc_version: rustc_version.trim().to_string(),
            cargo_version: cargo_version.trim().to_string(),
            cpu_model: cpu_model(),
            logical_cores,
            memory_bytes: memory_bytes(),
            timestamp_wallclock_excluded_from_hash: true,
        };
        Ok(fp)
    }

    pub fn content_hash(&self) -> Result<Hash256, ValidationError> {
        content_address(self)
    }
}

fn run_tool(binary: &str, args: &[&str]) -> Result<String, ValidationError> {
    let out = std::process::Command::new(binary)
        .args(args)
        .output()
        .map_err(|e| ValidationError::MissingBinary { binary: format!("{binary}: {e}") })?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn cpu_model() -> Option<String> {
    // Read /proc/cpuinfo on Linux; skip on other platforms.
    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/proc/cpuinfo").ok()?;
        for line in content.lines() {
            if line.starts_with("model name") {
                return line.splitn(2, ':').nth(1).map(|s| s.trim().to_string());
            }
        }
    }
    None
}

fn memory_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/proc/meminfo").ok()?;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
                return Some(kb * 1024);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_fingerprint_is_deterministic() {
        let fp = EnvironmentFingerprint::collect().unwrap();
        let h1 = fp.content_hash().unwrap();
        let h2 = fp.content_hash().unwrap();
        assert_eq!(h1, h2);
        assert!(fp.timestamp_wallclock_excluded_from_hash);
    }

    #[test]
    fn environment_fingerprint_has_os_and_arch() {
        let fp = EnvironmentFingerprint::collect().unwrap();
        assert!(!fp.os.is_empty());
        assert!(!fp.arch.is_empty());
    }
}
