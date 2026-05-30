use kosmo_core::{Digest, Q16};
use serde::{Deserialize, Serialize};

/// A named structural gap in the host topology.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostVoidKind {
    MissingTestFiber { for_module: String },
    MissingDocFiber { for_module: String },
    MissingImplementation { intent: String },
    MissingErrorHandling { location: String },
    MissingTypeAnnotation { location: String },
    IncompleteFunctionBody { location: String },
    Custom { description: String },
}

/// Serialize-only for content-addressing.
#[derive(Serialize)]
struct VoidContent<'a> {
    kind: &'a HostVoidKind,
    severity: Q16,
    location: &'a str,
}

/// A single topological void in the host codebase.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostVoid {
    pub void_id: Digest,
    pub kind: HostVoidKind,
    /// Severity in [0, 1] as Q16 — no floats in gate paths.
    pub severity: Q16,
    pub location: String,
}

impl HostVoid {
    pub fn new(kind: HostVoidKind, severity: Q16, location: String) -> Self {
        let void_id = Digest::of(&VoidContent {
            kind: &kind,
            severity,
            location: &location,
        });
        Self { void_id, kind, severity, location }
    }

    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&VoidContent {
            kind: &self.kind,
            severity: self.severity,
            location: &self.location,
        });
        self.void_id == expected
    }
}

/// Serialize-only for content-addressing TopologicalVoidMap.
#[derive(Serialize)]
struct VoidMapContent {
    voids: Vec<Digest>,
    policy_id: Digest,
}

/// The complete map of structural voids in a host codebase.
///
/// `voids` are sorted by `void_id` for determinism.
/// `map_id` is the content-addressed digest of the sorted void IDs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologicalVoidMap {
    pub map_id: Digest,
    pub voids: Vec<HostVoid>,
    pub policy_id: Digest,
}

impl TopologicalVoidMap {
    pub fn empty(policy_id: Digest) -> Self {
        let map_id = Digest::of(&VoidMapContent {
            voids: vec![],
            policy_id,
        });
        Self { map_id, voids: vec![], policy_id }
    }

    pub fn from_voids(mut voids: Vec<HostVoid>, policy_id: Digest) -> Self {
        voids.sort_by_key(|v| v.void_id);
        let map_id = Digest::of(&VoidMapContent {
            voids: voids.iter().map(|v| v.void_id).collect(),
            policy_id,
        });
        Self { map_id, voids, policy_id }
    }

    pub fn void_count(&self) -> usize {
        self.voids.len()
    }

    pub fn count_by_kind<F: Fn(&HostVoidKind) -> bool>(&self, pred: F) -> usize {
        self.voids.iter().filter(|v| pred(&v.kind)).count()
    }

    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&VoidMapContent {
            voids: self.voids.iter().map(|v| v.void_id).collect(),
            policy_id: self.policy_id,
        });
        self.map_id == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::Q16;

    #[test]
    fn host_void_id_deterministic() {
        let v1 = HostVoid::new(
            HostVoidKind::MissingTestFiber { for_module: "src/foo.rs".into() },
            Q16::HALF,
            "src/foo.rs".into(),
        );
        let v2 = HostVoid::new(
            HostVoidKind::MissingTestFiber { for_module: "src/foo.rs".into() },
            Q16::HALF,
            "src/foo.rs".into(),
        );
        assert_eq!(v1.void_id, v2.void_id);
        assert!(v1.verify_id());
    }

    #[test]
    fn void_map_sorted_deterministic() {
        let pid = Digest::of_bytes(b"p");
        let v1 = HostVoid::new(HostVoidKind::Custom { description: "z".into() }, Q16::HALF, "z".into());
        let v2 = HostVoid::new(HostVoidKind::Custom { description: "a".into() }, Q16::HALF, "a".into());
        let m1 = TopologicalVoidMap::from_voids(vec![v1.clone(), v2.clone()], pid);
        let m2 = TopologicalVoidMap::from_voids(vec![v2, v1], pid);
        assert_eq!(m1.map_id, m2.map_id, "map_id must be stable regardless of insertion order");
        assert!(m1.verify_id());
    }

    #[test]
    fn void_map_empty() {
        let vm = TopologicalVoidMap::empty(Digest::ZERO);
        assert_eq!(vm.void_count(), 0);
        assert!(vm.verify_id());
    }
}
