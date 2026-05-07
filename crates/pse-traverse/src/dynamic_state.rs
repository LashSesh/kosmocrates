//! Dynamic state primitives for the traversal dynamics layer.
//!
//! HASH-01: no wall-clock time, HashMap iteration, or non-seeded randomness in hashes.
//! NUMBER-01: all report-relevant numbers MUST be CanonicalNumber before hashing.

use serde::{Deserialize, Serialize};

use crate::field_cube::EvidenceRef;

// ───────────────────────────────────────────────────────────────────────────────
// i128 serialization helpers (serialize as decimal string to avoid JSON range issues)
// ───────────────────────────────────────────────────────────────────────────────

fn ser_i128_str<S: serde::Serializer>(v: &i128, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

fn de_i128_str<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i128, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<i128>().map_err(serde::de::Error::custom)
}

// ───────────────────────────────────────────────────────────────────────────────
// DynamicError
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum DynamicError {
    CanonicalizationFailed { reason: String },
    InvalidDimension { expected: usize, got: usize },
    NonFiniteNumber { field: String },
    ReplayMismatch { expected: Hash256, actual: Hash256 },
    OperatorVersionMismatch { expected: String, actual: String },
    StorageFailure { reason: String },
}

impl std::fmt::Display for DynamicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicError::CanonicalizationFailed { reason } => {
                write!(f, "canonicalization failed: {}", reason)
            }
            DynamicError::InvalidDimension { expected, got } => {
                write!(f, "invalid dimension: expected {}, got {}", expected, got)
            }
            DynamicError::NonFiniteNumber { field } => {
                write!(f, "non-finite number in field: {}", field)
            }
            DynamicError::ReplayMismatch { expected, actual } => write!(
                f,
                "replay mismatch: expected {}, actual {}",
                expected.hex(),
                actual.hex()
            ),
            DynamicError::OperatorVersionMismatch { expected, actual } => write!(
                f,
                "operator version mismatch: expected {}, actual {}",
                expected, actual
            ),
            DynamicError::StorageFailure { reason } => write!(f, "storage failure: {}", reason),
        }
    }
}

impl std::error::Error for DynamicError {}

// ───────────────────────────────────────────────────────────────────────────────
// CanonicalNumber
// ───────────────────────────────────────────────────────────────────────────────

/// Canonical fixed-point scalar for all content-addressed values.
/// NUMBER-01: all report-relevant numbers MUST be CanonicalNumber before hashing.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CanonicalNumber {
    FixedI64 {
        raw: i64,
        scale: u32,
    },
    Rational {
        #[serde(serialize_with = "ser_i128_str", deserialize_with = "de_i128_str")]
        num: i128,
        #[serde(serialize_with = "ser_i128_str", deserialize_with = "de_i128_str")]
        den: i128,
    },
}

impl PartialEq for CanonicalNumber {
    fn eq(&self, other: &Self) -> bool {
        // Compare by converting to a common form: both as rational
        let (n1, d1) = self.as_rational();
        let (n2, d2) = other.as_rational();
        // n1/d1 == n2/d2 iff n1*d2 == n2*d1 (d always positive)
        n1 * d2 == n2 * d1
    }
}

impl Eq for CanonicalNumber {}

impl PartialOrd for CanonicalNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CanonicalNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (n1, d1) = self.as_rational();
        let (n2, d2) = other.as_rational();
        // n1/d1 vs n2/d2 → n1*d2 vs n2*d1 (d always positive)
        (n1 * d2).cmp(&(n2 * d1))
    }
}

impl std::hash::Hash for CanonicalNumber {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            CanonicalNumber::FixedI64 { raw, scale } => {
                0u8.hash(state);
                raw.hash(state);
                scale.hash(state);
            }
            CanonicalNumber::Rational { num, den } => {
                1u8.hash(state);
                num.hash(state);
                den.hash(state);
            }
        }
    }
}

impl CanonicalNumber {
    fn as_rational(&self) -> (i128, i128) {
        match self {
            CanonicalNumber::FixedI64 { raw, scale } => {
                let den = 10i128.pow(*scale);
                (*raw as i128, den)
            }
            CanonicalNumber::Rational { num, den } => (*num, *den),
        }
    }

    /// Quantize f64 to CanonicalNumber with given scale.
    /// Uses round-ties-to-even (banker's rounding).
    pub fn quantize(value: f64, scale: u32) -> Result<Self, DynamicError> {
        if !value.is_finite() {
            return Err(DynamicError::NonFiniteNumber {
                field: format!("quantize({})", value),
            });
        }
        let factor = 10f64.powi(scale as i32);
        let scaled = value * factor;
        // Banker's rounding (round half to even)
        let raw = banker_round(scaled);
        Ok(CanonicalNumber::FixedI64 { raw, scale })
    }

    /// Quantize with default scale 9.
    pub fn quantize_default(value: f64) -> Result<Self, DynamicError> {
        Self::quantize(value, 9)
    }

    /// Zero value.
    pub fn zero() -> Self {
        CanonicalNumber::FixedI64 { raw: 0, scale: 9 }
    }

    /// Convert to f64.
    pub fn to_f64(&self) -> f64 {
        let (n, d) = self.as_rational();
        n as f64 / d as f64
    }

    /// Absolute value.
    pub fn abs(&self) -> Self {
        match self {
            CanonicalNumber::FixedI64 { raw, scale } => CanonicalNumber::FixedI64 {
                raw: raw.abs(),
                scale: *scale,
            },
            CanonicalNumber::Rational { num, den } => CanonicalNumber::Rational {
                num: num.abs(),
                den: *den,
            },
        }
    }
}

/// Banker's rounding: round half to even.
fn banker_round(x: f64) -> i64 {
    let floor = x.floor();
    let frac = x - floor;
    if (frac - 0.5).abs() < 1e-10 {
        // Exactly half — round to even
        let floor_i = floor as i64;
        if floor_i % 2 == 0 {
            floor_i
        } else {
            floor_i + 1
        }
    } else {
        x.round() as i64
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// Hash256
// ───────────────────────────────────────────────────────────────────────────────

/// 256-bit hash, serialized as 64-char lowercase hex string.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    pub fn zero() -> Self {
        Hash256([0u8; 32])
    }

    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.0 {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    pub fn from_hex(s: &str) -> Result<Self, DynamicError> {
        if s.len() != 64 {
            return Err(DynamicError::CanonicalizationFailed {
                reason: format!("expected 64 hex chars, got {}", s.len()),
            });
        }
        let mut bytes = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hi = hex_nibble(chunk[0])
                .map_err(|e| DynamicError::CanonicalizationFailed { reason: e })?;
            let lo = hex_nibble(chunk[1])
                .map_err(|e| DynamicError::CanonicalizationFailed { reason: e })?;
            bytes[i] = (hi << 4) | lo;
        }
        Ok(Hash256(bytes))
    }
}

fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex char: {}", b as char)),
    }
}

impl Serialize for Hash256 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for Hash256 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Hash256::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// StableId
// ───────────────────────────────────────────────────────────────────────────────

/// Stable content-addressed identifier wrapping Hash256.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StableId(pub Hash256);

// ───────────────────────────────────────────────────────────────────────────────
// stable_id_of helper
// ───────────────────────────────────────────────────────────────────────────────

pub fn stable_id_of<T: Serialize>(value: &T) -> Result<Hash256, DynamicError> {
    crate::canonical::content_address(value)
        .map(Hash256)
        .map_err(|e| DynamicError::CanonicalizationFailed {
            reason: e.to_string(),
        })
}

// ───────────────────────────────────────────────────────────────────────────────
// AuxiliaryKind
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuxiliaryKind {
    LogicalTick,
    Phase,
    SequenceIndex,
    PolicyEpoch,
}

// ───────────────────────────────────────────────────────────────────────────────
// LiftProfile
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LiftProfile {
    pub profile_id: Hash256,
    pub base_dimension: usize,
    pub lifted_dimension: usize,
    pub auxiliary_kind: AuxiliaryKind,
}

impl LiftProfile {
    pub fn new(base_dimension: usize, auxiliary_kind: AuxiliaryKind) -> Result<Self, DynamicError> {
        let lifted_dimension = base_dimension + 1;
        let mut profile = LiftProfile {
            profile_id: Hash256::zero(),
            base_dimension,
            lifted_dimension,
            auxiliary_kind,
        };
        // Compute profile_id from (base_dimension, lifted_dimension, auxiliary_kind)
        let probe = (base_dimension, lifted_dimension, &profile.auxiliary_kind);
        profile.profile_id = stable_id_of(&probe)?;
        Ok(profile)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// BaseState
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseState {
    pub state_id: Hash256,
    pub coords: Vec<CanonicalNumber>,
    pub weight: CanonicalNumber,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl BaseState {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.state_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.state_id = id;
        Ok(self)
    }

    pub fn zero(dims: usize) -> Result<Self, DynamicError> {
        let coords = vec![CanonicalNumber::zero(); dims];
        let s = BaseState {
            state_id: Hash256::zero(),
            coords,
            weight: CanonicalNumber::zero(),
            evidence_refs: vec![],
        };
        s.with_id()
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// LiftedState
// ───────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiftedState {
    pub state_id: Hash256,
    pub base_state_id: Hash256,
    pub coords: Vec<CanonicalNumber>,
    pub auxiliary: CanonicalNumber,
    pub lift_profile: LiftProfile,
}

impl LiftedState {
    pub fn with_id(mut self) -> Result<Self, DynamicError> {
        let mut probe = self.clone();
        probe.state_id = Hash256::zero();
        let id = stable_id_of(&probe)?;
        self.state_id = id;
        Ok(self)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// StateLift trait + DefaultStateLift
// ───────────────────────────────────────────────────────────────────────────────

pub trait StateLift {
    fn lift(&self, base: &BaseState, tick: u64) -> Result<LiftedState, DynamicError>;
    fn project(&self, lifted: &LiftedState) -> Result<BaseState, DynamicError>;
    fn roundtrip_error(&self, base: &BaseState, tick: u64)
        -> Result<CanonicalNumber, DynamicError>;
}

pub struct DefaultStateLift;

impl StateLift for DefaultStateLift {
    fn lift(&self, base: &BaseState, tick: u64) -> Result<LiftedState, DynamicError> {
        let auxiliary = CanonicalNumber::FixedI64 {
            raw: tick as i64,
            scale: 0,
        };
        let profile = LiftProfile::new(base.coords.len(), AuxiliaryKind::LogicalTick)?;
        let mut coords = base.coords.clone();
        coords.push(auxiliary.clone());
        let lifted = LiftedState {
            state_id: Hash256::zero(),
            base_state_id: base.state_id.clone(),
            coords,
            auxiliary,
            lift_profile: profile,
        };
        lifted.with_id()
    }

    fn project(&self, lifted: &LiftedState) -> Result<BaseState, DynamicError> {
        let base_dim = lifted.lift_profile.base_dimension;
        let coords: Vec<CanonicalNumber> = lifted.coords[..base_dim].to_vec();
        let s = BaseState {
            state_id: Hash256::zero(),
            coords,
            weight: CanonicalNumber::zero(),
            evidence_refs: vec![],
        };
        s.with_id()
    }

    fn roundtrip_error(
        &self,
        base: &BaseState,
        tick: u64,
    ) -> Result<CanonicalNumber, DynamicError> {
        let lifted = self.lift(base, tick)?;
        let projected = self.project(&lifted)?;
        // Compute L2 distance between base.coords and projected.coords
        let mut sum_sq = 0.0f64;
        for (a, b) in base.coords.iter().zip(projected.coords.iter()) {
            let diff = a.to_f64() - b.to_f64();
            sum_sq += diff * diff;
        }
        let dist = sum_sq.sqrt();
        CanonicalNumber::quantize_default(dist)
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lift_roundtrip_preserves_base() {
        let base = BaseState {
            state_id: Hash256::zero(),
            coords: vec![
                CanonicalNumber::quantize_default(1.5).unwrap(),
                CanonicalNumber::quantize_default(2.7).unwrap(),
            ],
            weight: CanonicalNumber::quantize_default(1.0).unwrap(),
            evidence_refs: vec![],
        };
        let base = base.with_id().unwrap();
        let lift = DefaultStateLift;
        let lifted = lift.lift(&base, 42).unwrap();
        let projected = lift.project(&lifted).unwrap();
        // Coords should match within quantization (all values are the same after roundtrip)
        assert_eq!(base.coords.len(), projected.coords.len());
        for (a, b) in base.coords.iter().zip(projected.coords.iter()) {
            let diff = (a.to_f64() - b.to_f64()).abs();
            assert!(
                diff < 1e-6,
                "coord mismatch: {} vs {}",
                a.to_f64(),
                b.to_f64()
            );
        }
    }

    #[test]
    fn canonical_number_quantize_is_deterministic() {
        let v = std::f64::consts::PI;
        let a = CanonicalNumber::quantize_default(v).unwrap();
        let b = CanonicalNumber::quantize_default(v).unwrap();
        assert_eq!(a, b);
        // Check via serde round-trip too
        let json_a = serde_json::to_string(&a).unwrap();
        let json_b = serde_json::to_string(&b).unwrap();
        assert_eq!(json_a, json_b);
    }

    #[test]
    fn hash256_hex_roundtrip() {
        let h = Hash256([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]);
        let hex = h.hex();
        assert_eq!(hex.len(), 64);
        let h2 = Hash256::from_hex(&hex).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn zero_hash_is_all_zeros() {
        let z = Hash256::zero();
        assert_eq!(z.0, [0u8; 32]);
        assert_eq!(
            z.hex(),
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn base_state_zero_has_all_zero_coords() {
        let s = BaseState::zero(3).unwrap();
        assert_eq!(s.coords.len(), 3);
        for c in &s.coords {
            assert_eq!(c.to_f64(), 0.0);
        }
    }
}
