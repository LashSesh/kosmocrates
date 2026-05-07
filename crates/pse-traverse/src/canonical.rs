//! Canonical serialisation and content addressing for traversal artefacts.
//!
//! Wraps `serde_jcs` (RFC 8785) so the encoding is byte-identical across
//! runs, platforms, and Rust versions. SHA-256 is the address function;
//! collision means equality.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::TraverseError;

/// Canonicalise a serializable value via JCS (RFC 8785).
pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, TraverseError> {
    serde_jcs::to_vec(value).map_err(|e| TraverseError::Canonical(e.to_string()))
}

/// Compute the SHA-256 content address of `value` after canonical encoding.
pub fn content_address<T: Serialize>(value: &T) -> Result<[u8; 32], TraverseError> {
    let bytes = canonical_bytes(value)?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(h.finalize().into())
}

/// Hex-encoded content address (64 lower-case hex chars).
pub fn hex_address<T: Serialize>(value: &T) -> Result<String, TraverseError> {
    let h = content_address(value)?;
    let mut s = String::with_capacity(64);
    for b in h {
        s.push_str(&format!("{:02x}", b));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn canonical_same_input_same_hash() {
        let mut m = BTreeMap::new();
        m.insert("b", 2);
        m.insert("a", 1);
        let h1 = content_address(&m).unwrap();
        let h2 = content_address(&m).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn canonical_map_order_stable() {
        // Two maps with the same logical contents must canonicalise identically
        // regardless of insertion order. BTreeMap already enforces this; we
        // assert it anyway to lock the contract.
        let mut a = BTreeMap::new();
        a.insert("x", 10);
        a.insert("y", 20);
        let mut b = BTreeMap::new();
        b.insert("y", 20);
        b.insert("x", 10);
        assert_eq!(canonical_bytes(&a).unwrap(), canonical_bytes(&b).unwrap());
    }

    #[test]
    fn hex_address_length_64() {
        let s = hex_address(&"hello").unwrap();
        assert_eq!(s.len(), 64);
        assert!(s
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }
}
