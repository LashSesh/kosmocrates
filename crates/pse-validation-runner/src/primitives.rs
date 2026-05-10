//! Shared primitives for the validation runner.
//!
//! Re-exports `Hash256` from pse-traverse. Provides `content_address`
//! (JCS + SHA-256) and the crate-wide error type.

use serde::{Deserialize, Serialize};

pub use pse_traverse::dynamic_state::Hash256;

/// Content-address `value` via JCS serialisation + SHA-256.
///
/// Wall-clock time MUST NOT appear in any value passed here.
pub fn content_address<T: Serialize>(value: &T) -> Result<Hash256, ValidationError> {
    pse_traverse::canonical::content_address(value)
        .map(Hash256)
        .map_err(|e| ValidationError::Canonicalization {
            reason: e.to_string(),
        })
}

/// Stable run identifier — human-readable, not part of any audit hash.
pub type StableId = String;

/// Total error surface for the validation runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationError {
    CommandFailed { reason: String },
    Timeout { command: String },
    MissingBinary { binary: String },
    MissingPreset { preset: String },
    ReplayMismatch { reason: String },
    LedgerChainBroken { reason: String },
    DirtyRepository,
    CalibrationLeakage { reason: String },
    TestSplitMissing,
    MetricMissing { metric: String },
    ReportParseFailed { reason: String },
    NonDeterministicOutput { reason: String },
    BundleVerificationFailed { reason: String },
    Canonicalization { reason: String },
    Io { reason: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::CommandFailed { reason } => write!(f, "command failed: {reason}"),
            ValidationError::Timeout { command } => write!(f, "timeout: {command}"),
            ValidationError::MissingBinary { binary } => write!(f, "missing binary: {binary}"),
            ValidationError::MissingPreset { preset } => write!(f, "missing preset: {preset}"),
            ValidationError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
            ValidationError::LedgerChainBroken { reason } => {
                write!(f, "ledger chain broken: {reason}")
            }
            ValidationError::DirtyRepository => write!(f, "dirty repository"),
            ValidationError::CalibrationLeakage { reason } => {
                write!(f, "calibration leakage: {reason}")
            }
            ValidationError::TestSplitMissing => write!(f, "test split missing"),
            ValidationError::MetricMissing { metric } => write!(f, "metric missing: {metric}"),
            ValidationError::ReportParseFailed { reason } => {
                write!(f, "report parse failed: {reason}")
            }
            ValidationError::NonDeterministicOutput { reason } => {
                write!(f, "non-deterministic output: {reason}")
            }
            ValidationError::BundleVerificationFailed { reason } => {
                write!(f, "bundle verification failed: {reason}")
            }
            ValidationError::Canonicalization { reason } => {
                write!(f, "canonicalization: {reason}")
            }
            ValidationError::Io { reason } => write!(f, "io: {reason}"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// The machine-readable failure kind corresponding to `ValidationError`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationFailureKind {
    CommandFailed,
    Timeout,
    MissingBinary,
    MissingPreset,
    ReplayMismatch,
    LedgerChainBroken,
    DirtyRepository,
    CalibrationLeakage,
    TestSplitMissing,
    MetricMissing,
    ReportParseFailed,
    NonDeterministicOutput,
    BundleVerificationFailed,
}

impl ValidationFailureKind {
    pub fn recovery_instruction(&self) -> &'static str {
        match self {
            ValidationFailureKind::CommandFailed => {
                "Re-run the failing command manually; check exit code and stderr."
            }
            ValidationFailureKind::Timeout => {
                "Increase timeout_seconds in ValidationProfile or reduce workload."
            }
            ValidationFailureKind::MissingBinary => {
                "Run `cargo build --release` to build required binaries."
            }
            ValidationFailureKind::MissingPreset => {
                "Add the missing preset to pse-eval-matrix or mark as open obligation."
            }
            ValidationFailureKind::ReplayMismatch => {
                "Verify determinism: rebuild from identical commit/inputs and re-run replay."
            }
            ValidationFailureKind::LedgerChainBroken => {
                "Ledger was modified after recording; restore from backup or restart run."
            }
            ValidationFailureKind::DirtyRepository => {
                "Commit or stash all changes before running validation."
            }
            ValidationFailureKind::CalibrationLeakage => {
                "Calibration must use only the calibration split; verify dataset split config."
            }
            ValidationFailureKind::TestSplitMissing => {
                "Provide a DatasetManifest with calibration, validation, and test splits."
            }
            ValidationFailureKind::MetricMissing => {
                "Register the missing metric in the EvaluationSpec before scoring."
            }
            ValidationFailureKind::ReportParseFailed => {
                "Check that the benchmark/eval tool outputs match the expected format."
            }
            ValidationFailureKind::NonDeterministicOutput => {
                "Two runs produced different hashes; check for platform-float or PRNG usage."
            }
            ValidationFailureKind::BundleVerificationFailed => {
                "Bundle was corrupted or tampered; re-create from a clean run."
            }
        }
    }
}
