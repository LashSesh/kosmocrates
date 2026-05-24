//! JCS-based canonical hashing for nxalien types.

use pse_nxalien_types::NxAlienRunDescriptor;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Encode bytes as lowercase hex string.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compute SHA-256 of a JCS-serialized value and return as lowercase hex.
pub fn sha256_jcs<T: Serialize>(value: &T) -> String {
    let canonical = serde_jcs::to_vec(value).expect("JCS serialization must not fail");
    bytes_to_hex(&Sha256::digest(&canonical))
}

/// Compute SHA-256 of raw bytes and return as lowercase hex.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    bytes_to_hex(&Sha256::digest(bytes))
}

// ─── Replay hash chain ───────────────────────────────────────────────────────

#[derive(Serialize)]
struct ReplayChain<'a> {
    manifest_hash: &'a str,
    context_hash: &'a str,
    schema_version: &'a str,
    seed: u64,
}

/// Compute the replay chain hash from manifest + context + descriptor.
pub fn compute_replay_hash(
    manifest_hash: &str,
    context_hash: &str,
    descriptor: &NxAlienRunDescriptor,
) -> String {
    sha256_jcs(&ReplayChain {
        manifest_hash,
        context_hash,
        schema_version: &descriptor.schema_version,
        seed: descriptor.seed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pse_nxalien_types::{EvidenceRef, NxAlienPolicy, RuleAtom, ScopeRef, Severity};

    fn base_rule(id: &str, severity: Severity, evidence: Vec<EvidenceRef>) -> RuleAtom {
        RuleAtom {
            id: id.to_string(),
            scope: ScopeRef {
                files: vec!["**/*.rs".to_string()],
                ..Default::default()
            },
            trigger: vec!["test".to_string()],
            prescription: "do something".to_string(),
            severity,
            evidence,
            gate: None,
            decay: None,
            hash: String::new(),
        }
    }

    fn ev(source: &str) -> EvidenceRef {
        EvidenceRef {
            kind: "file".to_string(),
            source: source.to_string(),
        }
    }

    #[test]
    fn identical_rules_identical_hash() {
        let r1 = base_rule("r1", Severity::Required, vec![ev("Cargo.toml")]);
        let r2 = base_rule("r1", Severity::Required, vec![ev("Cargo.toml")]);
        assert_eq!(r1.compute_hash(), r2.compute_hash());
    }

    #[test]
    fn reordered_evidence_identical_hash() {
        let r1 = base_rule("r1", Severity::Required, vec![ev("file:a"), ev("file:b")]);
        let r2 = base_rule("r1", Severity::Required, vec![ev("file:b"), ev("file:a")]);
        assert_eq!(r1.compute_hash(), r2.compute_hash());
    }

    #[test]
    fn changed_severity_changes_hash() {
        let r1 = base_rule("r1", Severity::Advisory, vec![]);
        let r2 = base_rule("r1", Severity::Blocking, vec![]);
        assert_ne!(r1.compute_hash(), r2.compute_hash());
    }

    #[test]
    fn missing_evidence_downgrades_required_rule() {
        let mut rules = vec![base_rule("r1", Severity::Required, vec![])];
        crate::gate::auto_downgrade_rules(&mut rules, &NxAlienPolicy::default());
        assert_eq!(rules[0].severity, Severity::Advisory);
    }

    #[test]
    fn with_hash_roundtrips() {
        let r = base_rule("r1", Severity::Required, vec![ev("Cargo.toml")]).with_hash();
        assert!(r.verify_hash());
    }
}
