use crate::void_map::{HostVoidKind, TopologicalVoidMap};
use kosmo_core::{Digest, Q16};
use serde::{Deserialize, Serialize};

/// Category of host codebase deficiency.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DeficiencyKind {
    TestCoverage,
    Documentation,
    TypeSafety,
    ErrorHandling,
    Modularity,
    StructuralCoherence,
    Custom(String),
}

/// One entry in the deficiency vector: a kind and its Q16 severity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeficiencyEntry {
    pub kind: DeficiencyKind,
    /// Severity 0.0–1.0 as Q16 (no floats in gate paths).
    pub severity: Q16,
    pub affected_count: u64,
    pub note: String,
}

/// Serialize-only content struct for content-addressing.
#[derive(Serialize)]
struct VectorContent {
    entries: Vec<(String, i64, u64)>, // (kind as json string, severity raw, affected_count)
    policy_id: Digest,
}

/// Quantified collection of deficiencies in a host codebase.
///
/// `vector_id` is content-addressed over sorted (kind, severity, count) tuples.
/// `total_severity` is the integer-averaged Q16 severity across all entries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeficiencyVector {
    pub vector_id: Digest,
    pub entries: Vec<DeficiencyEntry>,
    pub total_severity: Q16,
    pub policy_id: Digest,
}

impl DeficiencyVector {
    pub fn empty(policy_id: Digest) -> Self {
        let vector_id = Digest::of(&VectorContent {
            entries: vec![],
            policy_id,
        });
        Self {
            vector_id,
            entries: vec![],
            total_severity: Q16::ZERO,
            policy_id,
        }
    }

    pub fn from_entries(mut entries: Vec<DeficiencyEntry>, policy_id: Digest) -> Self {
        // Sort by kind for determinism
        entries.sort_by(|a, b| a.kind.cmp(&b.kind));

        let total_severity = if entries.is_empty() {
            Q16::ZERO
        } else {
            let sum_raw: i64 = entries.iter().map(|e| e.severity.raw()).sum();
            Q16::from_raw(sum_raw / entries.len() as i64)
        };

        let content_entries: Vec<(String, i64, u64)> = entries
            .iter()
            .map(|e| (format!("{:?}", e.kind), e.severity.raw(), e.affected_count))
            .collect();

        let vector_id = Digest::of(&VectorContent {
            entries: content_entries,
            policy_id,
        });

        Self {
            vector_id,
            entries,
            total_severity,
            policy_id,
        }
    }

    /// Derive a `DeficiencyVector` from a `TopologicalVoidMap`.
    pub fn from_void_map(void_map: &TopologicalVoidMap) -> Self {
        let policy_id = void_map.policy_id;
        let total = void_map.void_count() as u64;
        if total == 0 {
            return Self::empty(policy_id);
        }

        let test_voids =
            void_map.count_by_kind(|k| matches!(k, HostVoidKind::MissingTestFiber { .. })) as u64;
        let doc_voids =
            void_map.count_by_kind(|k| matches!(k, HostVoidKind::MissingDocFiber { .. })) as u64;

        let mut entries = Vec::new();

        if test_voids > 0 {
            // severity = untested / total source files (Q16 ratio, integer arithmetic)
            let severity = Q16::ratio(test_voids, total).unwrap_or(Q16::ZERO);
            entries.push(DeficiencyEntry {
                kind: DeficiencyKind::TestCoverage,
                severity,
                affected_count: test_voids,
                note: format!("{} source file(s) without tests", test_voids),
            });
        }

        if doc_voids > 0 {
            let severity = Q16::ratio(doc_voids, total).unwrap_or(Q16::ZERO);
            entries.push(DeficiencyEntry {
                kind: DeficiencyKind::Documentation,
                severity,
                affected_count: doc_voids,
                note: format!("{} module(s) without documentation", doc_voids),
            });
        }

        Self::from_entries(entries, policy_id)
    }

    pub fn has_critical_deficiency(&self, threshold: Q16) -> bool {
        self.entries.iter().any(|e| e.severity.at_least(threshold))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::void_map::{HostVoid, HostVoidKind, TopologicalVoidMap};
    use kosmo_core::{Digest, Q16};

    #[test]
    fn deficiency_vector_empty() {
        let dv = DeficiencyVector::empty(Digest::ZERO);
        assert!(dv.entries.is_empty());
        assert_eq!(dv.total_severity, Q16::ZERO);
    }

    #[test]
    fn deficiency_from_void_map_test_coverage() {
        let pid = Digest::of_bytes(b"p");
        let voids = vec![
            HostVoid::new(
                HostVoidKind::MissingTestFiber {
                    for_module: "src/a.rs".into(),
                },
                Q16::HALF,
                "src/a.rs".into(),
            ),
            HostVoid::new(
                HostVoidKind::MissingTestFiber {
                    for_module: "src/b.rs".into(),
                },
                Q16::HALF,
                "src/b.rs".into(),
            ),
        ];
        let vm = TopologicalVoidMap::from_voids(voids, pid);
        let dv = DeficiencyVector::from_void_map(&vm);
        assert_eq!(dv.entries.len(), 1);
        assert_eq!(dv.entries[0].kind, DeficiencyKind::TestCoverage);
        assert_eq!(dv.entries[0].affected_count, 2);
        // severity = 2/2 = Q16::ONE
        assert_eq!(dv.entries[0].severity, Q16::ONE);
    }

    #[test]
    fn deficiency_vector_deterministic() {
        let pid = Digest::of_bytes(b"p");
        let e1 = DeficiencyEntry {
            kind: DeficiencyKind::TestCoverage,
            severity: Q16::HALF,
            affected_count: 1,
            note: "n".into(),
        };
        let dv1 = DeficiencyVector::from_entries(vec![e1.clone()], pid);
        let dv2 = DeficiencyVector::from_entries(vec![e1], pid);
        assert_eq!(dv1.vector_id, dv2.vector_id);
    }
}
