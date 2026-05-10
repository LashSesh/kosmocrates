//! Canonical primitives for the Metatron closure layer.
//!
//! Self-contained: uses serde_jcs + sha2 directly to avoid a cyclic
//! dependency through pse-traverse → pse-core → pse-cascade → pse-metatron.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── Hash256 ───────────────────────────────────────────────────────────────────

/// 256-bit SHA-256 content address, serialized as 64-char lowercase hex.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    pub fn zero() -> Self {
        Hash256([0u8; 32])
    }

    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl Serialize for Hash256 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for Hash256 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(d)?;
        let mut bytes = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate().take(32) {
            let s = std::str::from_utf8(chunk).map_err(serde::de::Error::custom)?;
            bytes[i] = u8::from_str_radix(s, 16).map_err(serde::de::Error::custom)?;
        }
        Ok(Hash256(bytes))
    }
}

// ── CanonicalNumber ───────────────────────────────────────────────────────────

/// Canonical fixed-point scalar — no platform floats in the audit pathway.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CanonicalNumber {
    FixedI64 { raw: i64, scale: u32 },
}

/// Alias for spec compatibility.
pub type Fixed = CanonicalNumber;

// ── Content addressing ────────────────────────────────────────────────────────

/// JCS + SHA-256 content addressing (HASH-01 compliant).
pub fn content_address<T: Serialize>(value: &T) -> Result<Hash256, MetatronError> {
    let bytes = serde_jcs::to_vec(value).map_err(|e| MetatronError::Canonicalization {
        reason: e.to_string(),
    })?;
    let mut h = Sha256::new();
    h.update(&bytes);
    let digest: [u8; 32] = h.finalize().into();
    Ok(Hash256(digest))
}

// ── Domain types ──────────────────────────────────────────────────────────────

/// Stable run identifier — human-readable, not part of any audit hash.
pub type StableId = String;

/// Content-addressed evidence reference.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub kind: String,
    pub address: Hash256,
    pub relation: String,
}

/// Simplified NCTCS class reference for the Metatron layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NctcsClass {
    C0,
    C1,
    C2,
    C3,
    C4,
}

/// Thresholds for the MetatronGate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetatronThresholds {
    pub min_nctcs_class: NctcsClass,
    pub min_global_coherence: Fixed,
    pub min_isomorphism_score: Fixed,
    pub min_spectral_gap_delta: Fixed,
    pub min_replay_identity: Fixed,
    pub max_unexplained_drift: Fixed,
    pub min_eval_confidence: Fixed,
}

impl Default for MetatronThresholds {
    fn default() -> Self {
        let q = |n: i64| CanonicalNumber::FixedI64 { raw: n, scale: 2 };
        MetatronThresholds {
            min_nctcs_class: NctcsClass::C2,
            min_global_coherence: q(60),
            min_isomorphism_score: q(80),
            min_spectral_gap_delta: q(0),
            min_replay_identity: q(10000),
            max_unexplained_drift: q(5),
            min_eval_confidence: q(60),
        }
    }
}

/// Policy flags for the closure run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetatronPolicies {
    pub require_domain_validation: bool,
    pub require_c4_for_global_eigenmode: bool,
    pub allow_diagnostic_only: bool,
    pub reject_on_missing_trace: bool,
    pub reject_on_non_materialized_inputs: bool,
}

impl Default for MetatronPolicies {
    fn default() -> Self {
        MetatronPolicies {
            require_domain_validation: false,
            require_c4_for_global_eigenmode: true,
            allow_diagnostic_only: true,
            reject_on_missing_trace: true,
            reject_on_non_materialized_inputs: true,
        }
    }
}

/// Operator versions map type alias.
pub type OperatorVersions = BTreeMap<String, String>;

/// Total error surface for the Metatron closure layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetatronError {
    Canonicalization { reason: String },
    MissingArtifact { artifact: String },
    GateFailure { gate: String, reason: String },
    InvariantViolation { rule: String },
    ReplayMismatch { reason: String },
}

impl std::fmt::Display for MetatronError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetatronError::Canonicalization { reason } => write!(f, "canonicalization: {reason}"),
            MetatronError::MissingArtifact { artifact } => {
                write!(f, "missing artifact: {artifact}")
            }
            MetatronError::GateFailure { gate, reason } => {
                write!(f, "gate failure [{gate}]: {reason}")
            }
            MetatronError::InvariantViolation { rule } => {
                write!(f, "invariant violation: {rule}")
            }
            MetatronError::ReplayMismatch { reason } => write!(f, "replay mismatch: {reason}"),
        }
    }
}

impl std::error::Error for MetatronError {}
