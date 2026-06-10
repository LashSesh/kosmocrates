use crate::code_hdag::CodeHDAG;
use crate::deficiency::DeficiencyVector;
use crate::void_map::{HostVoid, HostVoidKind, TopologicalVoidMap};
use crate::xlang::CrossLanguageFingerprint;
use kosmo_core::{Digest, PolicyProfile, Q16, TaintLabel};
use kosmo_workbench::workspace::{WorkspaceEntryKind, WorkspaceIndex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    hdag_count: u64,
    fingerprint_count: u64,
    policy_id: Digest,
}

/// The HYPHAE view of a host workspace: voids, deficiency analysis, and
/// optional code-structure HDAGs extracted from source content.
///
/// Constructed from a `WorkspaceIndex`; no host file writes occur
/// (HYPHAE v0.3 passive run).
///
/// `hdag_by_void_id` is populated when workspace entries carry source content
/// (via `WorkspaceIndex::scan_path_with_content`). The map enables pipeline
/// layers to look up the structural graph for each void's originating source file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostCube {
    pub cube_id: Digest,
    pub binding: HostBinding,
    pub void_map: TopologicalVoidMap,
    pub deficiency_vector: DeficiencyVector,
    pub entry_count: u64,
    /// HDAG keyed by `void.void_id`. Each source file may contribute HDAGs for
    /// multiple voids (MissingTestFiber, MissingDocFiber, etc.).
    /// Empty when entries lack content.
    pub hdag_by_void_id: BTreeMap<Digest, CodeHDAG>,
    /// Cross-language structural fingerprint keyed by `void.void_id`, populated
    /// alongside `hdag_by_void_id` for every supported language (Rust, Python,
    /// JavaScript, Go). This is the language-independent comparability artifact
    /// the pipeline uses to rank cubes by cross-language structural resonance.
    pub fingerprint_by_void_id: BTreeMap<Digest, CrossLanguageFingerprint>,
    pub policy_id: Digest,
}

impl HostCube {
    pub fn from_workspace_index(index: &WorkspaceIndex, policy: &PolicyProfile) -> Self {
        let binding = HostBinding::new(index.index_id, policy.id);
        let (voids, hdag_by_void_id, fingerprint_by_void_id) = Self::derive_voids_and_hdags(index);
        let void_map = TopologicalVoidMap::from_voids(voids, policy.id);
        let deficiency_vector = DeficiencyVector::from_void_map(&void_map);
        let hdag_count = hdag_by_void_id.len() as u64;
        let fingerprint_count = fingerprint_by_void_id.len() as u64;

        let cube_id = Digest::of(&HostCubeContent {
            binding_id: binding.binding_id,
            void_map_id: void_map.map_id,
            deficiency_vector_id: deficiency_vector.vector_id,
            entry_count: index.entry_count,
            hdag_count,
            fingerprint_count,
            policy_id: policy.id,
        });

        Self {
            cube_id,
            binding,
            void_map,
            deficiency_vector,
            entry_count: index.entry_count,
            hdag_by_void_id,
            fingerprint_by_void_id,
            policy_id: policy.id,
        }
    }

    /// Derive topological voids and, when entry content is available, CodeHDAGs
    /// and cross-language structural fingerprints.
    ///
    /// When `entry.content` is `Some`, a CodeHDAG is extracted from the source text
    /// and stored under each void_id created for that file. This lets downstream
    /// layers (pipeline SourceCube construction) enrich energy assessments with
    /// `rho_coherence` (test-to-definition ratio) and `omega_phase` (complexity signal).
    ///
    /// A [`CrossLanguageFingerprint`] is computed in lockstep and stored under the
    /// same void_ids, giving the pipeline a language-independent structural profile
    /// per void for cross-language resonance ranking.
    ///
    /// Void severity is also modulated by HDAG structure when content is available:
    /// `MissingTestFiber` severity scales with definition count (more definitions →
    /// higher urgency for test coverage).
    #[allow(clippy::type_complexity)]
    fn derive_voids_and_hdags(
        index: &WorkspaceIndex,
    ) -> (
        Vec<HostVoid>,
        BTreeMap<Digest, CodeHDAG>,
        BTreeMap<Digest, CrossLanguageFingerprint>,
    ) {
        let mut voids: Vec<HostVoid> = Vec::new();
        let mut hdag_by_void_id: BTreeMap<Digest, CodeHDAG> = BTreeMap::new();
        let mut fingerprint_by_void_id: BTreeMap<Digest, CrossLanguageFingerprint> = BTreeMap::new();

        let source_entries: Vec<_> = index
            .entries
            .iter()
            .filter(|e| matches!(e.kind, WorkspaceEntryKind::SourceFile))
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

        for entry in &source_entries {
            let src = entry.path.as_str();
            let stem = module_stem(src);

            // Extract HDAG when source content is available. Language is detected
            // from the path, so Rust, Python, JavaScript, and Go all lift into the
            // same content-addressed graph (fail-closed: unknown extension → no HDAG).
            let hdag_opt: Option<CodeHDAG> = entry.content.as_deref().and_then(|text| {
                CodeHDAG::extract_auto(entry.digest, src, text, TaintLabel::Unverified)
            });

            // The language-independent structural fingerprint, in lockstep with the HDAG.
            let fp_opt: Option<CrossLanguageFingerprint> = entry
                .content
                .as_deref()
                .and_then(|text| CrossLanguageFingerprint::from_auto(entry.digest, src, text));

            let has_test = test_paths.iter().any(|t| path_contains_stem(t, stem));
            if !has_test {
                // Severity scales with definition count: more definitions → higher urgency.
                // Without HDAG: HALF. With HDAG and N definitions: HALF + HALF*min(N,8)/8.
                let severity = if let Some(ref hdag) = hdag_opt {
                    let defs = hdag.definition_count().min(8) as u64;
                    let bonus = Q16::ratio(defs, 16).unwrap_or(Q16::ZERO);
                    let base = Q16::ratio(1, 2).unwrap_or(Q16::ZERO);
                    Q16::from_raw((base.raw() + bonus.raw()).min(Q16::ONE.raw()))
                } else {
                    Q16::ratio(1, 2).unwrap_or(Q16::ZERO)
                };
                let void = HostVoid::new(
                    HostVoidKind::MissingTestFiber { for_module: src.to_string() },
                    severity,
                    src.to_string(),
                );
                if let Some(ref hdag) = hdag_opt {
                    hdag_by_void_id.insert(void.void_id, hdag.clone());
                }
                if let Some(ref fp) = fp_opt {
                    fingerprint_by_void_id.insert(void.void_id, fp.clone());
                }
                voids.push(void);
            }

            let has_doc = doc_paths.iter().any(|d| path_contains_stem(d, stem));
            if !has_doc {
                let void = HostVoid::new(
                    HostVoidKind::MissingDocFiber { for_module: src.to_string() },
                    Q16::ratio(1, 4).unwrap_or(Q16::ZERO),
                    src.to_string(),
                );
                if let Some(ref hdag) = hdag_opt {
                    hdag_by_void_id.insert(void.void_id, hdag.clone());
                }
                if let Some(ref fp) = fp_opt {
                    fingerprint_by_void_id.insert(void.void_id, fp.clone());
                }
                voids.push(void);
            }
        }

        (voids, hdag_by_void_id, fingerprint_by_void_id)
    }

    pub fn void_count(&self) -> usize {
        self.void_map.void_count()
    }

    pub fn has_deficiencies(&self) -> bool {
        !self.deficiency_vector.entries.is_empty()
    }
}

/// Extract the module stem from a source path (filename without its extension).
///
/// Language-agnostic: strips the final extension so `fib.py`, `fib.go`, and
/// `fib.rs` all share the stem `fib`, letting test/doc companion detection work
/// uniformly across languages.
fn module_stem(path: &str) -> &str {
    let filename = path.rsplit('/').next().unwrap_or(path);
    match filename.rfind('.') {
        Some(dot) if dot > 0 => &filename[..dot],
        _ => filename,
    }
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
            content: None,
        }
    }

    fn src_with_content(path: &str, text: &str) -> WorkspaceEntry {
        WorkspaceEntry {
            path: path.into(),
            digest: Digest::of_bytes(text.as_bytes()),
            size_bytes: text.len() as u64,
            kind: WorkspaceEntryKind::SourceFile,
            content: Some(text.to_string()),
        }
    }

    fn test_file(path: &str) -> WorkspaceEntry {
        WorkspaceEntry {
            path: path.into(),
            digest: Digest::of_bytes(path.as_bytes()),
            size_bytes: 50,
            kind: WorkspaceEntryKind::TestFile,
            content: None,
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

    #[test]
    fn host_cube_hdag_empty_without_content() {
        let policy = PolicyProfile::default_report_only();
        // Entry has no content — HDAGs should not be populated.
        let index = make_index(vec![src("src/lib.rs")]);
        let cube = HostCube::from_workspace_index(&index, &policy);
        assert!(cube.hdag_by_void_id.is_empty(), "no HDAGs expected without source content");
    }

    #[test]
    fn host_cube_hdag_extracted_when_content_present() {
        let policy = PolicyProfile::default_report_only();
        let source = "pub fn foo() {}\npub fn bar() {}\n";
        let index = make_index(vec![src_with_content("src/lib.rs", source)]);
        let cube = HostCube::from_workspace_index(&index, &policy);
        // Two voids (TestFiber + DocFiber) should each have an HDAG entry.
        assert_eq!(cube.hdag_by_void_id.len(), 2, "both voids must carry the HDAG");
        for hdag in cube.hdag_by_void_id.values() {
            assert!(hdag.definition_count() >= 2, "expected at least 2 fn definitions");
        }
    }

    #[test]
    fn host_cube_severity_scales_with_definition_count() {
        let policy = PolicyProfile::default_report_only();
        // 0 definitions → severity should be HALF (base only).
        let sparse = make_index(vec![src_with_content("src/sparse.rs", "// no fns")]);
        let cube_sparse = HostCube::from_workspace_index(&sparse, &policy);

        // 8+ definitions → severity should be > HALF.
        let dense_src = (0..8)
            .map(|i| format!("pub fn f{}() {{}}\n", i))
            .collect::<String>();
        let dense = make_index(vec![src_with_content("src/dense.rs", &dense_src)]);
        let cube_dense = HostCube::from_workspace_index(&dense, &policy);

        let severity_sparse = cube_sparse.void_map.voids.iter()
            .find(|v| matches!(&v.kind, crate::void_map::HostVoidKind::MissingTestFiber { .. }))
            .map(|v| v.severity)
            .expect("sparse must have MissingTestFiber");

        let severity_dense = cube_dense.void_map.voids.iter()
            .find(|v| matches!(&v.kind, crate::void_map::HostVoidKind::MissingTestFiber { .. }))
            .map(|v| v.severity)
            .expect("dense must have MissingTestFiber");

        assert!(
            severity_dense.raw() > severity_sparse.raw(),
            "more definitions must produce higher MissingTestFiber severity"
        );
    }

    #[test]
    fn host_cube_extracts_hdag_from_python_source() {
        // Cross-language hypercube materialization: a Python file with no test
        // companion produces a MissingTestFiber void carrying a real CodeHDAG,
        // extracted by the polyglot path — not the Rust extractor.
        let policy = PolicyProfile::default_report_only();
        let py = "import os\n\ndef alpha(x):\n    return x\n\ndef beta(y):\n    return y\n";
        let index = make_index(vec![src_with_content("pkg/fib.py", py)]);
        let cube = HostCube::from_workspace_index(&index, &policy);
        assert!(!cube.hdag_by_void_id.is_empty(), "Python source must yield an HDAG");
        for hdag in cube.hdag_by_void_id.values() {
            assert!(
                hdag.definition_count() >= 2,
                "expected at least two Python defs, got {}",
                hdag.definition_count()
            );
        }
    }

    #[test]
    fn host_cube_stores_cross_language_fingerprint() {
        let policy = PolicyProfile::default_report_only();
        let py = "import os\n\ndef alpha(x):\n    return x\n";
        let index = make_index(vec![src_with_content("pkg/mod.py", py)]);
        let cube = HostCube::from_workspace_index(&index, &policy);
        assert!(!cube.fingerprint_by_void_id.is_empty(), "fingerprint must be stored");
        assert_eq!(
            cube.fingerprint_by_void_id.len(),
            cube.hdag_by_void_id.len(),
            "fingerprint and HDAG maps populate in lockstep"
        );
        for fp in cube.fingerprint_by_void_id.values() {
            assert!(fp.verify_id(), "stored fingerprint must be content-addressed");
            assert!(fp.structural_count >= 2, "one import + one def");
        }
    }

    #[test]
    fn host_cube_id_differs_with_and_without_hdag() {
        let policy = PolicyProfile::default_report_only();
        let index_no_content = make_index(vec![src("src/lib.rs")]);
        let index_with_content = make_index(vec![src_with_content("src/lib.rs", "pub fn foo() {}")]);
        let cube_no = HostCube::from_workspace_index(&index_no_content, &policy);
        let cube_with = HostCube::from_workspace_index(&index_with_content, &policy);
        // hdag_count participates in cube_id — so HDAG-enriched cube has a different id.
        assert_ne!(
            cube_no.cube_id, cube_with.cube_id,
            "cube_id must differ when HDAG extraction changes hdag_count"
        );
    }
}
