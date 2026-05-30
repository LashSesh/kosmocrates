use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Content-addressed SHA-256 digest over JCS-canonical serialization.
///
/// The zero value (`Digest::ZERO`) is reserved as a placeholder for
/// self-referential id fields before sealing (see `PolicyProfile::default_report_only`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(pub(crate) [u8; 32]);

impl Digest {
    pub const ZERO: Self = Self([0u8; 32]);

    /// Compute the canonical digest of any serializable value: SHA-256(JCS(value)).
    pub fn of<T: Serialize>(value: &T) -> Self {
        let bytes = canonical_bytes(value);
        Self::of_bytes(&bytes)
    }

    /// Compute the canonical digest of raw bytes: SHA-256(data).
    pub fn of_bytes(data: &[u8]) -> Self {
        use sha2::{Digest as Sha2Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        Self(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{:02x}", b);
            s
        })
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (i, pair) in s.as_bytes().chunks(2).enumerate() {
            let hi = hex_val(pair[0])?;
            let lo = hex_val(pair[1])?;
            bytes[i] = (hi << 4) | lo;
        }
        Some(Self(bytes))
    }
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// JCS (RFC 8785) canonical serialization — deterministic, key-sorted JSON.
pub fn canonical_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    serde_jcs::to_vec(value).expect("JCS serialization must not fail for canonical types")
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({}…)", &self.to_hex()[..8])
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for Digest {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(d)?;
        Self::from_hex(&hex)
            .ok_or_else(|| serde::de::Error::custom("expected 64-char lowercase hex digest"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Sample {
        z: u32,
        a: u32,
    }

    #[test]
    fn digest_deterministic() {
        let s = Sample { z: 1, a: 2 };
        let d1 = Digest::of(&s);
        let d2 = Digest::of(&s);
        assert_eq!(d1, d2);
    }

    #[test]
    fn digest_differs_for_different_input() {
        let d1 = Digest::of(&Sample { z: 1, a: 2 });
        let d2 = Digest::of(&Sample { z: 1, a: 3 });
        assert_ne!(d1, d2);
    }

    #[test]
    fn digest_hex_round_trip() {
        let d = Digest::of(&42u64);
        let hex = d.to_hex();
        assert_eq!(hex.len(), 64);
        let back = Digest::from_hex(&hex).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn digest_jcs_key_ordering() {
        // JCS sorts object keys — {z:1, a:2} and {a:2, z:1} must produce same digest.
        // struct field order != serialization order after JCS normalization.
        // Both Sample{z:1,a:2} and a manually identical BTreeMap should hash the same.
        use std::collections::BTreeMap;
        let mut m = BTreeMap::new();
        m.insert("z", 1u32);
        m.insert("a", 2u32);
        let d1 = Digest::of(&Sample { z: 1, a: 2 });
        let d2 = Digest::of(&m);
        // BTreeMap serializes as {"a":2,"z":1} (alphabetical), serde struct as {"z":1,"a":2}
        // JCS normalizes both to {"a":2,"z":1} → same digest.
        assert_eq!(d1, d2, "JCS must normalise key order");
    }

    #[test]
    fn digest_zero_is_all_zeros() {
        assert_eq!(Digest::ZERO.as_bytes(), &[0u8; 32]);
    }
}
