use kosmo_core::{Digest, EvidenceKind, EvidenceRef, PolicyViolation};
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_SCAN_DEPTH: u32 = 8;
const MAX_FILE_BYTES: u64 = 1_048_576; // 1 MB

static EXCLUDED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".nxalien",
    "pkg",
    ".cache",
];

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WorkspaceEntryKind {
    SourceFile,
    TestFile,
    ConfigFile,
    BuildScript,
    Documentation,
    Binary,
    Unknown,
}

/// One file in the workspace, content-addressed.
///
/// `content` is the raw UTF-8 source text — populated only when the workspace
/// was built via `scan_path_with_content` or constructed with content explicitly.
/// It is intentionally excluded from `index_id` content-addressing (`#[serde(skip)]`);
/// the `digest` already content-addresses the file bytes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub path: String,
    pub digest: Digest,
    pub size_bytes: u64,
    pub kind: WorkspaceEntryKind,
    /// Source text for HDAG extraction. Not included in `index_id`.
    #[serde(skip)]
    pub content: Option<String>,
}

/// Internal content struct for deterministic hashing — Serialize only.
#[derive(Serialize)]
struct IndexContent<'a> {
    root: &'a str,
    entries: &'a [WorkspaceEntry],
    policy_id: &'a Digest,
}

/// Content-addressed, deterministically sorted index of workspace files.
///
/// Entry ordering is lexicographic by path so the `index_id` is stable
/// regardless of filesystem iteration order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceIndex {
    pub index_id: Digest,
    pub root: String,
    pub entries: Vec<WorkspaceEntry>,
    pub policy_id: Digest,
    pub entry_count: u64,
}

impl WorkspaceIndex {
    /// Build from pre-supplied entries (sorts by path, computes `index_id`).
    pub fn from_entries(
        root: String,
        mut entries: Vec<WorkspaceEntry>,
        policy_id: Digest,
    ) -> Self {
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let id = Digest::of(&IndexContent {
            root: &root,
            entries: &entries,
            policy_id: &policy_id,
        });
        let count = entries.len() as u64;
        Self {
            index_id: id,
            root,
            entries,
            policy_id,
            entry_count: count,
        }
    }

    /// Scan a local directory tree and build a `WorkspaceIndex`.
    ///
    /// Safe to call in `ReportOnly` mode — reads files, never writes.
    /// Files larger than 1 MB and excluded directories are skipped.
    /// Entry `content` fields are always `None`; use `scan_path_with_content`
    /// when HDAG extraction is needed.
    pub fn scan_path(
        root: &str,
        policy_id: Digest,
    ) -> Result<Self, WorkspaceError> {
        let root_path = Path::new(root);
        if !root_path.exists() {
            return Err(WorkspaceError::PathNotFound(root.to_string()));
        }
        let entries = collect_entries(root_path, root_path, 0, false)?;
        Ok(Self::from_entries(root.to_string(), entries, policy_id))
    }

    /// Like `scan_path` but populates `entry.content` for source/test `.rs` files.
    ///
    /// Enables `HostCube::from_workspace_index` to extract `CodeHDAG`s and produce
    /// code-structure-aware void severity and SourceCube dimensions. Files that are
    /// not valid UTF-8 get `content = None`.
    pub fn scan_path_with_content(
        root: &str,
        policy_id: Digest,
    ) -> Result<Self, WorkspaceError> {
        let root_path = Path::new(root);
        if !root_path.exists() {
            return Err(WorkspaceError::PathNotFound(root.to_string()));
        }
        let entries = collect_entries(root_path, root_path, 0, true)?;
        Ok(Self::from_entries(root.to_string(), entries, policy_id))
    }

    pub fn to_evidence_ref(&self) -> EvidenceRef {
        EvidenceRef::new(
            self.index_id,
            EvidenceKind::HostScan,
            format!("workspace-index:{}", self.root),
        )
    }

    pub fn verify_id(&self) -> bool {
        let expected = Digest::of(&IndexContent {
            root: &self.root,
            entries: &self.entries,
            policy_id: &self.policy_id,
        });
        self.index_id == expected
    }

    /// Number of source (`.rs`) files — useful for summary reports.
    pub fn source_file_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.kind == WorkspaceEntryKind::SourceFile)
            .count()
    }
}

fn collect_entries(
    base: &Path,
    current: &Path,
    depth: u32,
    with_content: bool,
) -> Result<Vec<WorkspaceEntry>, WorkspaceError> {
    if depth > MAX_SCAN_DEPTH {
        return Ok(vec![]);
    }
    let mut entries = Vec::new();
    for raw in std::fs::read_dir(current).map_err(WorkspaceError::Io)? {
        let raw = raw.map_err(WorkspaceError::Io)?;
        let path = raw.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            if EXCLUDED_DIRS.contains(&name.as_str()) {
                continue;
            }
            entries.extend(collect_entries(base, &path, depth + 1, with_content)?);
        } else if path.is_file() {
            let meta = std::fs::metadata(&path).map_err(WorkspaceError::Io)?;
            if meta.len() > MAX_FILE_BYTES {
                continue;
            }
            let raw_bytes = std::fs::read(&path).map_err(WorkspaceError::Io)?;
            let digest = Digest::of_bytes(&raw_bytes);
            let rel = path
                .strip_prefix(base)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default()
                .to_string();
            let kind = classify_entry(&rel);
            let content = if with_content
                && matches!(kind, WorkspaceEntryKind::SourceFile | WorkspaceEntryKind::TestFile)
            {
                String::from_utf8(raw_bytes).ok()
            } else {
                None
            };
            entries.push(WorkspaceEntry { kind, path: rel, digest, size_bytes: meta.len(), content });
        }
    }
    Ok(entries)
}

/// Classify a workspace file by its relative path.
///
/// Recognises the substrate's cross-language source set — Rust, Python,
/// JavaScript, and Go — so a polyglot workspace yields `SourceFile`/`TestFile`
/// entries the HYPHAE layer can lift into a topology. Test detection follows each
/// language's own convention; the function is fail-closed for unknown extensions
/// (`Unknown`), never guessing. The language-extraction dispatch itself lives in
/// `kosmo-hyphae::xlang`; this layer only needs to know which files are source.
fn classify_entry(rel: &str) -> WorkspaceEntryKind {
    if rel == "build.rs" || rel.ends_with("/build.rs") {
        return WorkspaceEntryKind::BuildScript;
    }

    let name = rel.rsplit('/').next().unwrap_or(rel);
    let under_tests_dir = rel.starts_with("tests/")
        || rel.starts_with("__tests__/")
        || rel.contains("/tests/")
        || rel.contains("/__tests__/");

    // Rust
    if rel.ends_with(".rs") {
        let is_test =
            under_tests_dir || rel.ends_with("_test.rs") || rel == "tests.rs";
        return source_or_test(is_test);
    }
    // Python
    if rel.ends_with(".py") || rel.ends_with(".pyi") {
        let is_test =
            under_tests_dir || name.starts_with("test_") || name.ends_with("_test.py");
        return source_or_test(is_test);
    }
    // JavaScript
    if rel.ends_with(".js") || rel.ends_with(".mjs") || rel.ends_with(".cjs") || rel.ends_with(".jsx")
    {
        let is_test = under_tests_dir
            || name.ends_with(".test.js")
            || name.ends_with(".spec.js")
            || name.ends_with(".test.mjs")
            || name.ends_with(".spec.mjs");
        return source_or_test(is_test);
    }
    // Go
    if rel.ends_with(".go") {
        return source_or_test(name.ends_with("_test.go"));
    }

    if rel.ends_with(".toml")
        || rel.ends_with(".json")
        || rel.ends_with(".yaml")
        || rel.ends_with(".yml")
        || rel.ends_with(".lock")
    {
        WorkspaceEntryKind::ConfigFile
    } else if rel.ends_with(".md") || rel.ends_with(".txt") || rel.ends_with(".rst") {
        WorkspaceEntryKind::Documentation
    } else {
        WorkspaceEntryKind::Unknown
    }
}

fn source_or_test(is_test: bool) -> WorkspaceEntryKind {
    if is_test {
        WorkspaceEntryKind::TestFile
    } else {
        WorkspaceEntryKind::SourceFile
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("path does not exist: {0}")]
    PathNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("policy violation: {0}")]
    Policy(#[from] PolicyViolation),
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::Digest;

    fn make_entry(path: &str, content: &[u8]) -> WorkspaceEntry {
        WorkspaceEntry {
            path: path.to_string(),
            digest: Digest::of_bytes(content),
            size_bytes: content.len() as u64,
            kind: classify_entry(path),
            content: None,
        }
    }

    #[test]
    fn workspace_index_deterministic() {
        let policy_id = Digest::of_bytes(b"test-policy");
        let entries = vec![
            make_entry("src/lib.rs", b"fn main() {}"),
            make_entry("Cargo.toml", b"[package]"),
        ];
        let idx1 = WorkspaceIndex::from_entries("/repo".into(), entries.clone(), policy_id);
        let idx2 = WorkspaceIndex::from_entries("/repo".into(), entries, policy_id);
        assert_eq!(idx1.index_id, idx2.index_id);
    }

    #[test]
    fn workspace_index_sorted_regardless_of_input_order() {
        let pid = Digest::of_bytes(b"p");
        let e1 = make_entry("z/file.rs", b"z");
        let e2 = make_entry("a/file.rs", b"a");
        let fwd = WorkspaceIndex::from_entries("/r".into(), vec![e1.clone(), e2.clone()], pid);
        let rev = WorkspaceIndex::from_entries("/r".into(), vec![e2, e1], pid);
        // Entries are sorted → same id regardless of insertion order
        assert_eq!(fwd.index_id, rev.index_id);
        assert_eq!(fwd.entries[0].path, "a/file.rs");
    }

    #[test]
    fn workspace_index_verify_id() {
        let pid = Digest::of_bytes(b"p");
        let idx = WorkspaceIndex::from_entries(
            "/repo".into(),
            vec![make_entry("src/lib.rs", b"hello")],
            pid,
        );
        assert!(idx.verify_id());
    }

    #[test]
    fn workspace_classify_entries() {
        assert_eq!(classify_entry("src/lib.rs"), WorkspaceEntryKind::SourceFile);
        assert_eq!(classify_entry("tests/foo.rs"), WorkspaceEntryKind::TestFile);
        assert_eq!(classify_entry("build.rs"), WorkspaceEntryKind::BuildScript);
        assert_eq!(classify_entry("Cargo.toml"), WorkspaceEntryKind::ConfigFile);
        assert_eq!(classify_entry("README.md"), WorkspaceEntryKind::Documentation);
        assert_eq!(classify_entry("img.png"), WorkspaceEntryKind::Unknown);
    }

    #[test]
    fn workspace_classify_cross_language_sources() {
        // Python, JavaScript, Go source files are recognised as source.
        assert_eq!(classify_entry("src/fib.py"), WorkspaceEntryKind::SourceFile);
        assert_eq!(classify_entry("web/app.js"), WorkspaceEntryKind::SourceFile);
        assert_eq!(classify_entry("lib/x.mjs"), WorkspaceEntryKind::SourceFile);
        assert_eq!(classify_entry("cmd/main.go"), WorkspaceEntryKind::SourceFile);
    }

    #[test]
    fn workspace_classify_cross_language_tests() {
        assert_eq!(classify_entry("test_fib.py"), WorkspaceEntryKind::TestFile);
        assert_eq!(classify_entry("pkg/fib_test.py"), WorkspaceEntryKind::TestFile);
        assert_eq!(classify_entry("fib.test.js"), WorkspaceEntryKind::TestFile);
        assert_eq!(classify_entry("__tests__/app.js"), WorkspaceEntryKind::TestFile);
        assert_eq!(classify_entry("fib_test.go"), WorkspaceEntryKind::TestFile);
        // A plain Rust file named with a Python test prefix stays a source file:
        // test conventions are applied per language.
        assert_eq!(classify_entry("test_helpers.rs"), WorkspaceEntryKind::SourceFile);
    }

    #[test]
    #[ignore = "integration: requires real filesystem"]
    fn workspace_scan_real() {
        let pid = Digest::of_bytes(b"p");
        let idx = WorkspaceIndex::scan_path(".", pid).unwrap();
        assert!(idx.entry_count > 0);
        assert!(idx.verify_id());
    }
}
