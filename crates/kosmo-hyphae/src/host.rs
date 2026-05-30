use crate::deficiency::DeficiencyVector;
use crate::void_map::{HostVoid, HostVoidKind, TopologicalVoidMap};
use kosmo_core::{Digest, PolicyProfile, Q16};
use kosmo_workbench::workspace::{WorkspaceEntryKind, WorkspaceIndex};
use serde::{Deserialize, Serialize};

/// A binding record linking a host workspace scan to a HYPHAE run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostBinding {
    pub binding_id: Digest,
    pub workspace_index_id: Digest,
    pub policy_id: Digest,
}

impl HostBinding {
    pub fn new(workspace_index_id: Digest, policy_id: Digest) -> Self {
        let mut b = Vec::with_capacity(64);
        b.extend_from_slice(workspace_index_id.as_bytes());
        b.extend_from_slice(policy_id.as_bytes());
        let binding_id = Digest::of_bytes(&b);
        Self { binding_id, workspace_index_id, policy_id }
    }
}

/// Serialize-only for content-addressing.
#[derive(Serialize)]
struct HostCubeContent {
    binding_id: Digest,
    void_map_id: Digest,
    deficiency_vector_id: Digest,
    entry_count: u64,
    policy_id: Digest,
}

/// The HYPHAE view of a host workspace: voids and deficiency analysis.
///
/// Constructed from a `WorkspaceIndex`; no host file writes occur
/// (HYPHAE v0.3 passive run).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostCube {
    pub cube_id: Digest,
    pub binding: HostBinding,
    pub void_map: TopologicalVoidMap,
    pub deficiency_vector: DeficiencyVector,
    pub entry_count: u64,
    pub policy_id: Digest,
}

impl HostCube {
    pub fn from_workspace_index(index: &WorkspaceIndex, policy: &PolicyProfile) -> Self {
        let binding = HostBinding::new(index.index_id, policy.id);
        let voids = Self::derive_voids(index);
        let void_map = TopologicalVoidMap::from_voids(voids, policy.id);
        let deficiency_vector = DeficiencyVector::from_void_map(&void_map);

        let cube_id = Digest::of(&HostCubeContent {
            binding_id: binding.binding_id,
            void_map_id: void_map.map_id,
            deficiency_vector_id: deficiency_vector.vector_id,
            entry_count: index.entry_count,
            policy_id: policy.id,
        });

        Self {
            cube_id,
            binding,
            void_map,
            deficiency_vector,
            entry_count: index.entry_count,
            policy_id: policy.id,
        }
    }

    /// Derive topological voids from the workspace entries using structural
    /// heuristics (Phase 3: no real parser, observation-only).
    fn derive_voids(index: &WorkspaceIndex) -> Vec<HostVoid> {
        let mut voids = Vec::new();

        let source_paths: Vec<&str> = index
            .entries
            .iter()
            .filter(|e| matches!(e.kind, WorkspaceEntryKind::SourceFile))
            .map(|e| e.path.as_str())
            .collect();

        let test_paths: Vec<&str> = index
            .entries
            .iter()
            .filter(|e| matches!(e.kind, WorkspaceEntryKind::TestFile))
            .map(|e| e.path.as_str())
            .collect();

        let doc_paths: Vec<&str> = index
            .entries
            .iter()
            .filter(|e| matches!(e.kind, WorkspaceEntryKind::Documentation))
            .map(|e| e.path.as_str())
            .collect();

        for src in &source_paths {
            let stem = module_stem(src);
            let has_test = test_paths.iter().any(|t| path_contains_stem(t, stem));
            if !has_test {
                voids.push(HostVoid::new(
                    HostVoidKind::MissingTestFiber { for_module: src.to_string() },
                    // Severity: moderate — test coverage is a key quality signal.
                    Q16::ratio(1, 2).unwrap_or(Q16::ZERO),
                    src.to_string(),
                ));
            }

            let has_doc = doc_paths.iter().any(|d| path_contains_stem(d, stem));
            if !has_doc {
                voids.push(HostVoid::new(
                    HostVoidKind::MissingDocFiber { for_module: src.to_string() },
                    Q16::ratio(1, 4).unwrap_or(Q16::ZERO),
                    src.to_string(),
                ));
            }
        }

        voids
    }

    pub fn void_count(&self) -> usize {
        self.void_map.void_count()
    }

    pub fn has_deficiencies(&self) -> bool {
        !self.deficiency_vector.entries.is_empty()
    }
}

/// Extract the module stem from a source path (filename without extension).
fn module_stem(path: &str) -> &str {
    let filename = path.split('/').last().unwrap_or(path);
    filename.strip_suffix(".rs").unwrap_or(filename)
}

/// True if `path` contains `stem` as a substring (case-sensitive).
fn path_contains_stem(path: &str, stem: &str) -> bool {
    !stem.is_empty() && path.contains(stem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, PolicyProfile};
    use kosmo_workbench::workspace::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceIndex};

    fn make_index(entries: Vec<WorkspaceEntry>) -> WorkspaceIndex {
        let count = entries.len() as u64;
        let mut sorted = entries;
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        let index_id = Digest::of_bytes(b"test-index");
        WorkspaceIndex {
            index_id,
            root: "test".into(),
            entries: sorted,
            policy_id: Digest::ZERO,
            entry_count: count,
        }
    }

    fn src(path: &str) -> WorkspaceEntry {
        WorkspaceEntry {
            path: path.into(),
            digest: Digest::of_bytes(path.as_bytes()),
            size_bytes: 100,
            kind: WorkspaceEntryKind::SourceFile,
        }
    }

    fn test_file(path: &str) -> WorkspaceEntry {
        WorkspaceEntry {
            path: path.into(),
            digest: Digest::of_bytes(path.as_bytes()),
            size_bytes: 50,
            kind: WorkspaceEntryKind::TestFile,
        }
    }

    #[test]
    fn host_cube_derives_test_void_for_untested_source() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![src("src/foo.rs")]);
        let cube = HostCube::from_workspace_index(&index, &policy);
        let test_voids = cube.void_map.count_by_kind(|k| {
            matches!(k, HostVoidKind::MissingTestFiber { .. })
        });
        assert!(test_voids > 0, "untested source file must produce a MissingTestFiber void");
    }

    #[test]
    fn host_cube_no_test_void_when_companion_present() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![
            src("src/bar.rs"),
            test_file("tests/bar_test.rs"),
        ]);
        let cube = HostCube::from_workspace_index(&index, &policy);
        let test_voids = cube.void_map.count_by_kind(|k| {
            matches!(k, HostVoidKind::MissingTestFiber { .. })
        });
        assert_eq!(test_voids, 0, "source with test companion must not produce void");
    }

    #[test]
    fn host_cube_is_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![src("src/alpha.rs")]);
        let c1 = HostCube::from_workspace_index(&index, &policy);
        let c2 = HostCube::from_workspace_index(&index, &policy);
        assert_eq!(c1.cube_id, c2.cube_id);
        assert_ne!(c1.cube_id, Digest::ZERO);
    }

    #[test]
    fn host_cube_empty_workspace_has_no_voids() {
        let policy = PolicyProfile::default_report_only();
        let index = make_index(vec![]);
        let cube = HostCube::from_workspace_index(&index, &policy);
        assert_eq!(cube.void_count(), 0);
        assert!(!cube.has_deficiencies());
    }
}
