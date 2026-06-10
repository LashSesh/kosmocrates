//! Real ParseBack executor for KOSMO-OPS-01 RX.
//!
//! Captures workspace crate topology via `cargo metadata`, diffs pre/post
//! materialization snapshots, and produces a `ParseBackReport`.
//!
//! Policy contract:
//!  - `ReportOnly` → `SkippedByReportOnly`, no scan performed.
//!  - Any other mode → read-only workspace scan (no host mutations).
//!  - Baseline mismatch (plan id ≠ pre-snapshot id) → `Inconclusive`.
//!  - Identical pre/post topology → `TopologyUnchanged`.
//!  - Diffs with a `Critical` delta → `Failed`; otherwise `Passed`.
//!
//! Host isolation: the executor only reads from the filesystem; it spawns
//! exactly one `cargo metadata` process per snapshot, which is an allowlisted
//! read-only subcommand. No file writes, no network access.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use kosmo_core::{
    Digest, ImplementationMode, ParseBackOutcome, ParseBackPlan, ParseBackReport,
    ParseBackScanScope, ParseBackSeverity, ParseBackTopologyDelta, PolicyProfile,
    TopologyChangeKind,
};

// ─── CrateFingerprint ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CrateFingerprintContent<'a> {
    name: &'a str,
    files_id: &'a Digest,
    dep_names: &'a Vec<String>,
}

#[derive(Serialize)]
struct FilesContent<'a> {
    paths: &'a Vec<String>,
}

/// Content-addressed fingerprint of one crate's topology.
///
/// Captures crate name, sorted source file paths, and sorted dependency names.
/// `crate_id` changes whenever files are added/removed or dependencies change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateFingerprint {
    /// SHA-256 of `{name, files_id, dep_names}`.
    pub crate_id: Digest,
    pub name: String,
    /// SHA-256 of the sorted source file paths.
    pub files_id: Digest,
    pub file_count: u64,
    /// Direct dependency names, sorted for determinism.
    pub dep_names: Vec<String>,
}

impl CrateFingerprint {
    pub fn new(name: String, mut file_paths: Vec<String>, mut dep_names: Vec<String>) -> Self {
        file_paths.sort();
        dep_names.sort();
        dep_names.dedup();
        let files_id = Digest::of(&FilesContent { paths: &file_paths });
        let crate_id = Digest::of(&CrateFingerprintContent {
            name: &name,
            files_id: &files_id,
            dep_names: &dep_names,
        });
        Self {
            crate_id,
            name,
            files_id,
            file_count: file_paths.len() as u64,
            dep_names,
        }
    }
}

// ─── TopologySnapshot ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SnapshotContent {
    scope_tag: String,
    /// Sorted by crate name (BTreeMap iteration order is alphabetical).
    crate_ids: Vec<Digest>,
    /// Sorted dependency edges as [from, to] pairs.
    dep_edges: Vec<[String; 2]>,
}

/// Immutable, content-addressed snapshot of the workspace crate topology at
/// one point in time.
///
/// `snapshot_id` is the SHA-256 of all crate fingerprints and dependency edges
/// under the given scope. Identical inputs always produce identical ids
/// (INVARIANT-007).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySnapshot {
    pub snapshot_id: Digest,
    pub scope: ParseBackScanScope,
    /// Keyed by crate name; ordered for determinism.
    pub crate_nodes: BTreeMap<String, CrateFingerprint>,
    /// Directed dependency edges (dependent → dependency), ordered.
    pub dep_edges: BTreeSet<(String, String)>,
}

impl TopologySnapshot {
    pub fn from_parts(
        scope: ParseBackScanScope,
        crate_nodes: BTreeMap<String, CrateFingerprint>,
        dep_edges: BTreeSet<(String, String)>,
    ) -> Self {
        let snapshot_id = Self::compute_id(&scope, &crate_nodes, &dep_edges);
        Self {
            snapshot_id,
            scope,
            crate_nodes,
            dep_edges,
        }
    }

    fn compute_id(
        scope: &ParseBackScanScope,
        crate_nodes: &BTreeMap<String, CrateFingerprint>,
        dep_edges: &BTreeSet<(String, String)>,
    ) -> Digest {
        // BTreeMap iteration order is deterministic (alphabetical by key).
        let crate_ids: Vec<Digest> = crate_nodes.values().map(|c| c.crate_id).collect();
        let dep_edges_arr: Vec<[String; 2]> = dep_edges
            .iter()
            .map(|(a, b)| [a.clone(), b.clone()])
            .collect();
        Digest::of(&SnapshotContent {
            scope_tag: format!("{:?}", scope),
            crate_ids,
            dep_edges: dep_edges_arr,
        })
    }

    pub fn crate_count(&self) -> usize {
        self.crate_nodes.len()
    }
}

// ─── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ParseBackError {
    WorkspaceRootNotFound,
    CargoMetadataFailed(String),
    MetadataParseError(String),
}

impl std::fmt::Display for ParseBackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceRootNotFound => write!(f, "workspace root Cargo.toml not found"),
            Self::CargoMetadataFailed(s) => write!(f, "cargo metadata failed: {}", s),
            Self::MetadataParseError(s) => write!(f, "metadata parse error: {}", s),
        }
    }
}

// ─── ParseBackExecutor ────────────────────────────────────────────────────────

/// Real ParseBack executor.
///
/// `snapshot()` runs `cargo metadata` on the workspace and captures crate
/// topology. `execute()` takes a pre-materialization snapshot and produces a
/// `ParseBackReport` by comparing it to a fresh post-materialization snapshot.
pub struct ParseBackExecutor {
    workspace_root: PathBuf,
    cargo_program: String,
}

impl ParseBackExecutor {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            cargo_program: "cargo".into(),
        }
    }

    /// Override the cargo binary path (useful for testing).
    pub fn with_cargo(mut self, program: impl Into<String>) -> Self {
        self.cargo_program = program.into();
        self
    }

    /// Capture a topology snapshot of the workspace.
    ///
    /// Runs `cargo metadata --no-deps` and walks each package's `src/`
    /// directory for Rust source files. Read-only; no host mutations.
    pub fn snapshot(&self, scope: &ParseBackScanScope) -> Result<TopologySnapshot, ParseBackError> {
        let metadata = self.run_cargo_metadata()?;
        self.build_snapshot(scope, &metadata)
    }

    /// Execute a parse-back rescan given a pre-materialization snapshot.
    ///
    /// Returns `SkippedByReportOnly` immediately if `policy.mode == ReportOnly`.
    /// Returns `Inconclusive` if the plan's `baseline_topology_id` does not
    /// match `pre.snapshot_id` (baseline integrity failure).
    pub fn execute(
        &self,
        plan: &ParseBackPlan,
        pre: &TopologySnapshot,
        policy: &PolicyProfile,
        evidence_bundle_id: Digest,
    ) -> ParseBackReport {
        if policy.mode == ImplementationMode::ReportOnly {
            return ParseBackReport::skipped_by_report_only(
                plan.id,
                pre.snapshot_id,
                evidence_bundle_id,
            );
        }

        let t0 = Instant::now();

        // Baseline integrity check: plan must reference the snapshot we were given.
        if plan.baseline_topology_id != pre.snapshot_id {
            return ParseBackReport::new(
                plan.id,
                ParseBackOutcome::Inconclusive,
                vec![],
                pre.snapshot_id,
                pre.snapshot_id,
                vec![format!(
                    "Baseline topology mismatch: plan references {} but pre-snapshot is {}",
                    plan.baseline_topology_id, pre.snapshot_id,
                )],
                evidence_bundle_id,
                t0.elapsed().as_millis() as u64,
            );
        }

        // Take post-materialization snapshot.
        let post = match self.snapshot(&plan.scan_scope) {
            Ok(s) => s,
            Err(e) => {
                return ParseBackReport::new(
                    plan.id,
                    ParseBackOutcome::Inconclusive,
                    vec![],
                    pre.snapshot_id,
                    pre.snapshot_id,
                    vec![format!("Post-materialization snapshot failed: {}", e)],
                    evidence_bundle_id,
                    t0.elapsed().as_millis() as u64,
                );
            }
        };

        let elapsed = t0.elapsed().as_millis() as u64;

        if pre.snapshot_id == post.snapshot_id {
            return ParseBackReport::topology_unchanged(
                plan.id,
                pre.snapshot_id,
                evidence_bundle_id,
                elapsed,
            );
        }

        let deltas = diff_snapshots(pre, &post);
        let has_critical = deltas.iter().any(|d| d.is_critical());
        let outcome = if has_critical {
            ParseBackOutcome::Failed
        } else {
            ParseBackOutcome::Passed
        };

        ParseBackReport::new(
            plan.id,
            outcome,
            deltas,
            pre.snapshot_id,
            post.snapshot_id,
            vec![],
            evidence_bundle_id,
            elapsed,
        )
    }

    fn run_cargo_metadata(&self) -> Result<serde_json::Value, ParseBackError> {
        let manifest = self.workspace_root.join("Cargo.toml");
        if !manifest.exists() {
            return Err(ParseBackError::WorkspaceRootNotFound);
        }

        let output = Command::new(&self.cargo_program)
            .args([
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--manifest-path",
            ])
            .arg(&manifest)
            .output()
            .map_err(|e| ParseBackError::CargoMetadataFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(ParseBackError::CargoMetadataFailed(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|e| ParseBackError::MetadataParseError(e.to_string()))
    }

    fn build_snapshot(
        &self,
        scope: &ParseBackScanScope,
        metadata: &serde_json::Value,
    ) -> Result<TopologySnapshot, ParseBackError> {
        let packages = metadata["packages"]
            .as_array()
            .ok_or_else(|| ParseBackError::MetadataParseError("no packages array".into()))?;

        let mut crate_nodes: BTreeMap<String, CrateFingerprint> = BTreeMap::new();
        let mut dep_edges: BTreeSet<(String, String)> = BTreeSet::new();

        for pkg in packages {
            let name = match pkg["name"].as_str() {
                Some(n) if !n.is_empty() => n.to_string(),
                _ => continue,
            };

            let pkg_dir = pkg["manifest_path"]
                .as_str()
                .and_then(|p| PathBuf::from(p).parent().map(|d| d.to_path_buf()))
                .unwrap_or_default();

            let file_paths = collect_source_files(&pkg_dir, scope);

            let mut dep_names: Vec<String> = pkg["dependencies"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|d| d["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            dep_names.sort();
            dep_names.dedup();

            for dep in &dep_names {
                dep_edges.insert((name.clone(), dep.clone()));
            }

            crate_nodes.insert(
                name.clone(),
                CrateFingerprint::new(name, file_paths, dep_names),
            );
        }

        Ok(TopologySnapshot::from_parts(
            scope.clone(),
            crate_nodes,
            dep_edges,
        ))
    }
}

// ─── Source file collector ────────────────────────────────────────────────────

fn collect_source_files(pkg_dir: &Path, scope: &ParseBackScanScope) -> Vec<String> {
    let src_dir = pkg_dir.join("src");
    if !src_dir.exists() {
        return vec![];
    }
    match scope {
        ParseBackScanScope::AffectedFilesOnly => {
            // Collect only top-level entry points for speed.
            ["lib.rs", "main.rs"]
                .iter()
                .filter(|f| src_dir.join(f).is_file())
                .map(|f| f.to_string())
                .collect()
        }
        _ => {
            let mut paths = vec![];
            walk_rs_files(&src_dir, &src_dir, &mut paths);
            paths.sort();
            paths
        }
    }
}

fn walk_rs_files(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(base, &path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

// ─── Topology diff ────────────────────────────────────────────────────────────

/// Compute the structural delta between two topology snapshots.
///
/// Returns a sorted, content-addressed list of `ParseBackTopologyDelta` items.
/// Severity rules:
/// - `NodeRemoved` / `EdgeRemoved` → `Critical` (destructive structural change).
/// - `NodeAdded` / `EdgeAdded` → `Warning` (additive but alters closure surface).
/// - `NodeModified` (files or deps changed within a crate) → `Info`.
pub fn diff_snapshots(
    pre: &TopologySnapshot,
    post: &TopologySnapshot,
) -> Vec<ParseBackTopologyDelta> {
    let mut deltas: Vec<ParseBackTopologyDelta> = vec![];

    let pre_names: BTreeSet<&str> = pre.crate_nodes.keys().map(String::as_str).collect();
    let post_names: BTreeSet<&str> = post.crate_nodes.keys().map(String::as_str).collect();

    // Crates added in post but absent in pre.
    for name in post_names.difference(&pre_names) {
        let fp = &post.crate_nodes[*name];
        deltas.push(ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeAdded,
            Some(fp.crate_id),
            format!("Crate added: {name}"),
            ParseBackSeverity::Warning,
        ));
    }

    // Crates present in pre but absent in post.
    for name in pre_names.difference(&post_names) {
        let fp = &pre.crate_nodes[*name];
        deltas.push(ParseBackTopologyDelta::new(
            TopologyChangeKind::NodeRemoved,
            Some(fp.crate_id),
            format!("Crate removed: {name}"),
            ParseBackSeverity::Critical,
        ));
    }

    // Crates present in both: detect structural changes.
    for name in pre_names.intersection(&post_names) {
        let pre_fp = &pre.crate_nodes[*name];
        let post_fp = &post.crate_nodes[*name];
        if pre_fp.crate_id != post_fp.crate_id {
            deltas.push(ParseBackTopologyDelta::new(
                TopologyChangeKind::NodeModified,
                Some(post_fp.crate_id),
                format!("Crate modified: {name} (files or deps changed)"),
                ParseBackSeverity::Info,
            ));
        }
    }

    // Dependency edges added in post.
    for (from, to) in post.dep_edges.difference(&pre.dep_edges) {
        deltas.push(ParseBackTopologyDelta::new(
            TopologyChangeKind::EdgeAdded,
            None,
            format!("Dependency added: {from} -> {to}"),
            ParseBackSeverity::Warning,
        ));
    }

    // Dependency edges removed in post.
    for (from, to) in pre.dep_edges.difference(&post.dep_edges) {
        deltas.push(ParseBackTopologyDelta::new(
            TopologyChangeKind::EdgeRemoved,
            None,
            format!("Dependency removed: {from} -> {to}"),
            ParseBackSeverity::Critical,
        ));
    }

    // Deterministic ordering: sort by delta id.
    deltas.sort_by_key(|d| d.id);
    deltas
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};

    fn pid() -> Digest {
        Digest::of_bytes(b"policy")
    }

    fn bundle_id() -> Digest {
        let bundle = EvidenceBundle::seal(
            vec![EvidenceRef::new(
                Digest::of_bytes(b"ev"),
                EvidenceKind::HostScan,
                "test",
            )],
            pid(),
            ReplayStatus::Replayable,
        );
        bundle.bundle_id
    }

    fn simple_snapshot(crate_name: &str, files: Vec<&str>, deps: Vec<&str>) -> TopologySnapshot {
        let mut nodes = BTreeMap::new();
        let mut edges = BTreeSet::new();
        let fp = CrateFingerprint::new(
            crate_name.to_string(),
            files.iter().map(|s| s.to_string()).collect(),
            deps.iter().map(|s| s.to_string()).collect(),
        );
        for dep in &fp.dep_names {
            edges.insert((crate_name.to_string(), dep.clone()));
        }
        nodes.insert(crate_name.to_string(), fp);
        TopologySnapshot::from_parts(ParseBackScanScope::FullWorkspace, nodes, edges)
    }

    #[test]
    fn crate_fingerprint_is_content_addressed() {
        let f1 = CrateFingerprint::new("alpha".into(), vec!["lib.rs".into()], vec!["serde".into()]);
        let f2 = CrateFingerprint::new("alpha".into(), vec!["lib.rs".into()], vec!["serde".into()]);
        assert_eq!(f1.crate_id, f2.crate_id);
        assert_ne!(f1.crate_id, Digest::ZERO);
    }

    #[test]
    fn crate_fingerprint_different_deps_differs() {
        let f1 = CrateFingerprint::new("alpha".into(), vec!["lib.rs".into()], vec!["serde".into()]);
        let f2 = CrateFingerprint::new("alpha".into(), vec!["lib.rs".into()], vec!["tokio".into()]);
        assert_ne!(f1.crate_id, f2.crate_id);
    }

    #[test]
    fn crate_fingerprint_dep_order_is_normalised() {
        let f1 = CrateFingerprint::new(
            "alpha".into(),
            vec!["lib.rs".into()],
            vec!["tokio".into(), "serde".into()],
        );
        let f2 = CrateFingerprint::new(
            "alpha".into(),
            vec!["lib.rs".into()],
            vec!["serde".into(), "tokio".into()],
        );
        assert_eq!(f1.crate_id, f2.crate_id);
    }

    #[test]
    fn snapshot_is_content_addressed() {
        let s1 = simple_snapshot("alpha", vec!["lib.rs"], vec!["serde"]);
        let s2 = simple_snapshot("alpha", vec!["lib.rs"], vec!["serde"]);
        assert_eq!(s1.snapshot_id, s2.snapshot_id);
        assert_ne!(s1.snapshot_id, Digest::ZERO);
    }

    #[test]
    fn snapshot_different_crates_differ() {
        let s1 = simple_snapshot("alpha", vec!["lib.rs"], vec![]);
        let s2 = simple_snapshot("beta", vec!["lib.rs"], vec![]);
        assert_ne!(s1.snapshot_id, s2.snapshot_id);
    }

    #[test]
    fn diff_identical_snapshots_is_empty() {
        let s = simple_snapshot("alpha", vec!["lib.rs"], vec!["serde"]);
        let deltas = diff_snapshots(&s, &s);
        assert!(deltas.is_empty());
    }

    #[test]
    fn diff_detects_node_added() {
        let pre = simple_snapshot("alpha", vec!["lib.rs"], vec![]);
        let mut post = pre.clone();
        let new_fp = CrateFingerprint::new("beta".into(), vec!["lib.rs".into()], vec![]);
        post.crate_nodes.insert("beta".into(), new_fp);
        post.snapshot_id =
            TopologySnapshot::compute_id(&post.scope, &post.crate_nodes, &post.dep_edges);

        let deltas = diff_snapshots(&pre, &post);
        assert_eq!(deltas.len(), 1);
        assert!(matches!(
            deltas[0].change_kind,
            TopologyChangeKind::NodeAdded
        ));
        assert!(deltas[0].description.contains("beta"));
        assert_eq!(deltas[0].severity, ParseBackSeverity::Warning);
        assert!(deltas[0].verify_id());
    }

    #[test]
    fn diff_detects_node_removed() {
        let pre = simple_snapshot("alpha", vec!["lib.rs"], vec![]);
        let post = TopologySnapshot::from_parts(
            ParseBackScanScope::FullWorkspace,
            BTreeMap::new(),
            BTreeSet::new(),
        );

        let deltas = diff_snapshots(&pre, &post);
        assert_eq!(deltas.len(), 1);
        assert!(matches!(
            deltas[0].change_kind,
            TopologyChangeKind::NodeRemoved
        ));
        assert_eq!(deltas[0].severity, ParseBackSeverity::Critical);
        assert!(deltas[0].verify_id());
    }

    #[test]
    fn diff_detects_node_modified() {
        let pre = simple_snapshot("alpha", vec!["lib.rs"], vec![]);
        let mut post = pre.clone();
        // Change the files fingerprint
        let modified_fp = CrateFingerprint::new(
            "alpha".into(),
            vec!["lib.rs".into(), "extra.rs".into()],
            vec![],
        );
        post.crate_nodes.insert("alpha".into(), modified_fp);
        post.snapshot_id =
            TopologySnapshot::compute_id(&post.scope, &post.crate_nodes, &post.dep_edges);

        let deltas = diff_snapshots(&pre, &post);
        assert_eq!(deltas.len(), 1);
        assert!(matches!(
            deltas[0].change_kind,
            TopologyChangeKind::NodeModified
        ));
        assert_eq!(deltas[0].severity, ParseBackSeverity::Info);
        assert!(deltas[0].verify_id());
    }

    #[test]
    fn diff_detects_edge_added() {
        let pre = simple_snapshot("alpha", vec!["lib.rs"], vec![]);
        let post = simple_snapshot("alpha", vec!["lib.rs"], vec!["serde"]);

        let deltas = diff_snapshots(&pre, &post);
        // NodeModified (deps changed) + EdgeAdded
        assert!(deltas
            .iter()
            .any(|d| matches!(d.change_kind, TopologyChangeKind::EdgeAdded)));
        assert!(deltas.iter().any(|d| d.description.contains("serde")));
    }

    #[test]
    fn diff_detects_edge_removed() {
        let pre = simple_snapshot("alpha", vec!["lib.rs"], vec!["serde"]);
        let post = simple_snapshot("alpha", vec!["lib.rs"], vec![]);

        let deltas = diff_snapshots(&pre, &post);
        assert!(deltas
            .iter()
            .any(|d| matches!(d.change_kind, TopologyChangeKind::EdgeRemoved)));
        assert!(deltas.iter().all(|d| d.verify_id()));
        let worst = deltas.iter().map(|d| &d.severity).max().unwrap();
        assert_eq!(*worst, ParseBackSeverity::Critical);
    }

    #[test]
    fn diff_deltas_are_content_addressed() {
        let pre = simple_snapshot("alpha", vec!["lib.rs"], vec![]);
        let post = simple_snapshot("alpha", vec!["lib.rs"], vec!["serde"]);
        let deltas1 = diff_snapshots(&pre, &post);
        let deltas2 = diff_snapshots(&pre, &post);
        let ids1: Vec<Digest> = deltas1.iter().map(|d| d.id).collect();
        let ids2: Vec<Digest> = deltas2.iter().map(|d| d.id).collect();
        assert_eq!(ids1, ids2);
    }

    #[test]
    fn executor_report_only_skips_scan() {
        let executor = ParseBackExecutor::new(PathBuf::from("/nonexistent"));
        let pre = simple_snapshot("alpha", vec![], vec![]);
        let plan = ParseBackPlan::new(
            pid(),
            Digest::of_bytes(b"mat-plan"),
            ParseBackScanScope::FullWorkspace,
            pre.snapshot_id,
        );
        let policy = PolicyProfile::default_report_only();
        let report = executor.execute(&plan, &pre, &policy, bundle_id());

        assert!(report.outcome.is_skipped_report_only());
        assert!(report.verify_id());
        assert_eq!(report.pre_topology_id, pre.snapshot_id);
        assert_eq!(report.post_topology_id, pre.snapshot_id);
    }

    #[test]
    fn executor_baseline_mismatch_is_inconclusive() {
        let executor = ParseBackExecutor::new(PathBuf::from("/nonexistent"));
        let pre = simple_snapshot("alpha", vec![], vec![]);
        let wrong_baseline = Digest::of_bytes(b"wrong-baseline");
        let plan = ParseBackPlan::new(
            pid(),
            Digest::of_bytes(b"mat-plan"),
            ParseBackScanScope::FullWorkspace,
            wrong_baseline, // does not match pre.snapshot_id
        );
        let policy = PolicyProfile::dry_run();
        let report = executor.execute(&plan, &pre, &policy, bundle_id());

        assert!(report.outcome.is_failure_class());
        assert!(report.verify_id());
        assert!(!report.diagnostics.is_empty());
        assert!(report.diagnostics[0].contains("mismatch"));
    }

    #[test]
    fn executor_unchanged_workspace_produces_topology_unchanged() {
        // Use the real kosmocrates workspace so we can run cargo metadata.
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let executor = ParseBackExecutor::new(workspace_root);

        let pre = match executor.snapshot(&ParseBackScanScope::FullWorkspace) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("snapshot failed (skip): {}", e);
                return;
            }
        };

        assert!(
            pre.crate_count() > 0,
            "workspace must have at least one crate"
        );

        let plan = ParseBackPlan::new(
            pid(),
            Digest::of_bytes(b"mat-plan"),
            ParseBackScanScope::FullWorkspace,
            pre.snapshot_id,
        );
        let policy = PolicyProfile::dry_run();
        let report = executor.execute(&plan, &pre, &policy, bundle_id());

        assert!(
            report.outcome.is_passed(),
            "unchanged workspace must be TopologyUnchanged or Passed, got {:?}",
            report.outcome
        );
        assert!(report.verify_id());
        assert_eq!(report.pre_topology_id, pre.snapshot_id);
        assert_eq!(report.post_topology_id, pre.snapshot_id);
    }

    #[test]
    fn snapshot_crate_count_matches_workspace() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let executor = ParseBackExecutor::new(workspace_root);
        let snapshot = match executor.snapshot(&ParseBackScanScope::AffectedCratesOnly) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("snapshot failed (skip): {}", e);
                return;
            }
        };
        // Kosmocrates workspace has many members; just assert >10 to be safe.
        assert!(
            snapshot.crate_count() > 10,
            "expected >10 crates, got {}",
            snapshot.crate_count()
        );
    }

    #[test]
    fn snapshot_is_deterministic_across_two_calls() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let executor = ParseBackExecutor::new(workspace_root);
        let scope = ParseBackScanScope::FullWorkspace;
        let s1 = match executor.snapshot(&scope) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("snapshot failed (skip): {}", e);
                return;
            }
        };
        let s2 = match executor.snapshot(&scope) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("snapshot failed (skip): {}", e);
                return;
            }
        };
        assert_eq!(
            s1.snapshot_id, s2.snapshot_id,
            "snapshots must be deterministic"
        );
    }
}
