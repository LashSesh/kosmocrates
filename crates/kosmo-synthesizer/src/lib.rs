//! Kosmocrates synthesizer layer.
//!
//! Defines the [`ActionSynthesizer`] trait and the surrounding types that
//! carry a proposed workspace change from an [`ActionItem`] to a
//! content-addressed [`Patch`].
//!
//! The trait is deliberately abstract: implementations can range from a
//! deterministic rule-based system to a full LLM (e.g. `kosmo-synthesizer-claude`).
//! The [`MockSynthesizer`] provides a zero-dependency test implementation.
//!
//! # Data flow
//!
//! ```text
//! ActionItem
//!   └─ SynthesisRequest  (context: workspace path + source snippets)
//!        └─ ActionSynthesizer::synthesize()
//!             └─ SynthesisResult
//!                  └─ Patch  (Vec<FileChange>, content-addressed)
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use kosmo_core::{Digest, Q16, WishFacet, WishFacetKind};
use kosmo_pipeline::{ActionItem, ActionItemKind};

// ─── Source context ───────────────────────────────────────────────────────────

/// A source file excerpt provided as context to the synthesizer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceSnippet {
    pub path: PathBuf,
    pub content: String,
    pub relevance_score: Q16,
}

// ─── SynthesisRequest ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RequestContent {
    action_id: Digest,
    workspace_path_hash: Digest,
    policy_id: Digest,
}

/// Everything the synthesizer needs to produce a [`Patch`] for one [`ActionItem`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthesisRequest {
    /// Content-addressed identifier. Same action on the same workspace always
    /// produces the same `request_id`, enabling dedup and caching.
    pub request_id: Digest,
    pub action_item: ActionItem,
    pub workspace_path: PathBuf,
    /// Top-N source files ranked by structural relevance to the action target.
    /// Empty when no context extraction has been performed.
    pub source_snippets: Vec<SourceSnippet>,
}

impl SynthesisRequest {
    pub fn new(action_item: ActionItem, workspace_path: impl Into<PathBuf>) -> Self {
        let workspace_path = workspace_path.into();
        let workspace_path_hash =
            Digest::of(&workspace_path.to_string_lossy().to_string());
        let request_id = Digest::of(&RequestContent {
            action_id: action_item.action_id,
            workspace_path_hash,
            policy_id: action_item.policy_id,
        });
        Self { request_id, action_item, workspace_path, source_snippets: vec![] }
    }

    pub fn with_snippets(mut self, snippets: Vec<SourceSnippet>) -> Self {
        self.source_snippets = snippets;
        self
    }
}

// ─── Patch ────────────────────────────────────────────────────────────────────

/// Whether a [`FileChange`] creates, modifies, or removes a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileChangeKind { Create, Modify, Delete }

/// A single proposed change to one file in the workspace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: FileChangeKind,
    /// Full proposed content (empty string for `Delete`).
    pub content: String,
}

impl FileChange {
    pub fn create(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self { path: path.into(), kind: FileChangeKind::Create, content: content.into() }
    }
    pub fn modify(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self { path: path.into(), kind: FileChangeKind::Modify, content: content.into() }
    }
    pub fn delete(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), kind: FileChangeKind::Delete, content: String::new() }
    }
    /// Line count of the proposed content.
    pub fn line_count(&self) -> u32 {
        if self.content.is_empty() { 0 } else { self.content.lines().count() as u32 }
    }
}

#[derive(Serialize)]
struct PatchContent { request_id: Digest, changes_hash: Digest }

/// A content-addressed batch of [`FileChange`]s produced by a synthesizer.
///
/// `patch_id` is deterministic: same `request_id` + same ordered file changes
/// always yields the same `patch_id`. An empty patch (no file changes) is
/// valid and represents "no change needed for this action."
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Patch {
    pub patch_id: Digest,
    pub request_id: Digest,
    pub file_changes: Vec<FileChange>,
    /// Human-readable tag identifying the synthesizer that produced this patch.
    /// E.g. `"claude-opus-4-8"`, `"rule-based"`, `"mock"`.
    pub model_hint: String,
}

impl Patch {
    pub fn new(
        request_id: Digest,
        mut file_changes: Vec<FileChange>,
        model_hint: impl Into<String>,
    ) -> Self {
        // Canonical order: sort by path for deterministic hashing.
        file_changes.sort_by(|a, b| a.path.cmp(&b.path));
        let changes_hash = {
            let mut s = String::new();
            for fc in &file_changes {
                s.push_str(&fc.path.to_string_lossy());
                s.push('\x00');
                s.push_str(&fc.content);
                s.push('\x00');
            }
            Digest::of(&s)
        };
        let patch_id = Digest::of(&PatchContent { request_id, changes_hash });
        Self { patch_id, request_id, file_changes, model_hint: model_hint.into() }
    }

    pub fn empty(request_id: Digest) -> Self {
        Self::new(request_id, vec![], "empty")
    }

    pub fn is_empty(&self) -> bool { self.file_changes.is_empty() }

    /// Total proposed line additions across all file changes.
    pub fn total_lines(&self) -> u32 {
        self.file_changes.iter().map(|fc| fc.line_count()).sum()
    }
}

// ─── SynthesisResult ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ResultContent { patch_id: Digest, confidence_raw: i64 }

/// The full output of one [`ActionSynthesizer::synthesize`] call.
///
/// `result_id` is content-addressed from `(patch_id, confidence)`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthesisResult {
    pub result_id: Digest,
    pub patch: Patch,
    /// Why this patch was chosen. Used for audit trails and human review.
    pub rationale: String,
    /// Synthesizer's self-assessed confidence that this patch is correct.
    /// Downstream callers (e.g. [`AgentSession`]) may skip low-confidence results.
    pub confidence: Q16,
    /// Optional hint: which test to run first to validate the patch.
    pub test_hint: Option<String>,
    /// LLM token cost. Zero for rule-based or mock synthesizers.
    pub tokens_used: u32,
}

impl SynthesisResult {
    pub fn new(patch: Patch, rationale: impl Into<String>, confidence: Q16) -> Self {
        let result_id = Digest::of(&ResultContent {
            patch_id: patch.patch_id,
            confidence_raw: confidence.raw(),
        });
        Self {
            result_id,
            patch,
            rationale: rationale.into(),
            confidence,
            test_hint: None,
            tokens_used: 0,
        }
    }
}

// ─── Error ────────────────────────────────────────────────────────────────────

/// Error returned by [`ActionSynthesizer::synthesize`].
#[derive(Clone, Debug)]
pub struct SynthesisError {
    pub message: String,
    /// If `true`, the caller may retry (e.g. rate limit). If `false`, the
    /// action item should be skipped for this run.
    pub recoverable: bool,
}

impl SynthesisError {
    pub fn permanent(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), recoverable: false }
    }
    pub fn transient(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), recoverable: true }
    }
}

impl fmt::Display for SynthesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "synthesis error (recoverable={}): {}", self.recoverable, self.message)
    }
}
impl std::error::Error for SynthesisError {}

// ─── Trait ────────────────────────────────────────────────────────────────────

/// Pluggable synthesis backend. Implement this trait to wire in any
/// code-generation strategy — LLM, rule-based, template, or mock.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// an async runtime or used in a `spawn_blocking` context.
pub trait ActionSynthesizer: Send + Sync {
    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisResult, SynthesisError>;
    fn name(&self) -> &str;
    /// Soft token budget hint. Implementations may ignore this.
    fn token_budget(&self) -> u32 { 4096 }
}

// ─── MockSynthesizer ──────────────────────────────────────────────────────────

/// Deterministic test implementation. Returns an empty patch with a fixed
/// confidence level — no LLM calls, no file I/O.
///
/// ```
/// use kosmo_synthesizer::{MockSynthesizer, ActionSynthesizer, SynthesisRequest};
/// use kosmo_pipeline::{ActionItem, IntegrationRunOptions, run_workspace_pipeline};
/// // (build a real ActionItem or use the mock directly with a synthetic request)
/// ```
pub struct MockSynthesizer {
    pub label: String,
    pub confidence: Q16,
    pub file_changes: Vec<FileChange>,
}

impl MockSynthesizer {
    /// High-confidence mock (0.90). Simulates a synthesizer that is almost
    /// certain its patch is correct.
    pub fn confident() -> Self {
        Self {
            label: "mock-confident".into(),
            confidence: Q16::ratio(90, 100).unwrap_or(Q16::ONE),
            file_changes: vec![],
        }
    }

    /// Low-confidence mock (0.30). Simulates a synthesizer that is unsure —
    /// useful for testing `min_confidence` filtering in the agent.
    pub fn uncertain() -> Self {
        Self {
            label: "mock-uncertain".into(),
            confidence: Q16::ratio(30, 100).unwrap_or(Q16::ZERO),
            file_changes: vec![],
        }
    }

    /// Mock with a specific proposed file change (useful for patch-content tests).
    pub fn with_change(mut self, change: FileChange) -> Self {
        self.file_changes.push(change);
        self
    }
}

impl ActionSynthesizer for MockSynthesizer {
    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisResult, SynthesisError> {
        let patch = Patch::new(
            request.request_id,
            self.file_changes.clone(),
            self.label.as_str(),
        );
        Ok(SynthesisResult::new(
            patch,
            format!("mock synthesis for action {}", &request.action_item.action_id.to_hex()[..8]),
            self.confidence,
        ))
    }

    fn name(&self) -> &str { &self.label }
}

// ─── FacetScaffolder ───────────────────────────────────────────────────────────

/// A deterministic synthesizer that *builds toward a wish* offline.
///
/// It acts only on [`ActionItemKind::RealizeWishFacet`] actions, scaffolding the
/// missing facet so the next observation sees it present (and the loop
/// converges) — no LLM, no network. It reads the workspace to stay idempotent
/// (re-running on an already-realized facet yields an empty patch).
///
/// - `Symbol` → append `pub fn <name>() {}` to `src/lib.rs` (or `src/main.rs`).
/// - `Module` → create `src/<name>.rs` and add `pub mod <name>;` to the crate root.
/// - `Crate`  → create `<name>/Cargo.toml` + `<name>/src/lib.rs`, and best-effort
///   register `<name>` in the root `[workspace] members`.
/// - `Capability` / `Resolution` → no structural scaffold (empty patch).
///
/// This is the deterministic counterpart to the LLM synthesizer: it can't write
/// real behaviour, only the structural skeleton, but it makes the build-toward-
/// intent loop runnable and verifiable without a model.
pub struct FacetScaffolder;

impl FacetScaffolder {
    fn scaffold(&self, ws: &Path, facet: &WishFacet) -> Vec<FileChange> {
        match facet.kind {
            WishFacetKind::Symbol => Self::scaffold_symbol(ws, &facet.key),
            WishFacetKind::Signature => Self::scaffold_signature(ws, &facet.key),
            WishFacetKind::Module => Self::scaffold_module(ws, &facet.key),
            WishFacetKind::Crate => Self::scaffold_crate(ws, &facet.key),
            WishFacetKind::Capability => Self::scaffold_capability(ws, &facet.key),
            WishFacetKind::Test => Self::scaffold_test(ws, &facet.key),
            // Dependency / Resolution have no reliable structural scaffold
            // (a dependency edit needs a path the scaffolder can't infer).
            WishFacetKind::Dependency | WishFacetKind::Resolution => vec![],
        }
    }

    /// Append `snippet` to `src/lib.rs` (or `src/main.rs`, or create lib.rs) iff
    /// `marker` is not already present. Idempotent.
    fn append_to_lib(ws: &Path, marker: &str, snippet: &str) -> Vec<FileChange> {
        let lib = ws.join("src/lib.rs");
        let main = ws.join("src/main.rs");
        let (rel, path, exists) = if lib.exists() {
            ("src/lib.rs", lib, true)
        } else if main.exists() {
            ("src/main.rs", main, true)
        } else {
            ("src/lib.rs", lib, false)
        };
        let mut content = if exists {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };
        if content.contains(marker) {
            return vec![]; // already present — idempotent
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(snippet);
        if !snippet.ends_with('\n') {
            content.push('\n');
        }
        let fc = if exists {
            FileChange::modify(rel, content)
        } else {
            FileChange::create(rel, content)
        };
        vec![fc]
    }

    fn scaffold_symbol(ws: &Path, name: &str) -> Vec<FileChange> {
        Self::append_to_lib(
            ws,
            &format!("fn {name}"),
            &format!("pub fn {name}() {{ /* scaffolded by kosmo */ }}"),
        )
    }

    fn scaffold_signature(ws: &Path, key: &str) -> Vec<FileChange> {
        let (name, arity) = match key.split_once('/') {
            Some((n, a)) => (n, a.parse::<usize>().unwrap_or(0)),
            None => (key, 0),
        };
        let params = (0..arity)
            .map(|i| format!("_a{i}: ()"))
            .collect::<Vec<_>>()
            .join(", ");
        Self::append_to_lib(
            ws,
            &format!("fn {name}"),
            &format!("pub fn {name}({params}) {{ /* scaffolded by kosmo */ }}"),
        )
    }

    fn scaffold_capability(ws: &Path, name: &str) -> Vec<FileChange> {
        Self::append_to_lib(
            ws,
            &format!("kosmo:capability: {name}"),
            &format!("// kosmo:capability: {name}"),
        )
    }

    fn scaffold_test(ws: &Path, name: &str) -> Vec<FileChange> {
        Self::append_to_lib(
            ws,
            &format!("fn {name}"),
            &format!("#[test]\nfn {name}() {{ /* scaffolded by kosmo */ }}"),
        )
    }

    fn scaffold_module(ws: &Path, name: &str) -> Vec<FileChange> {
        let mut changes = Vec::new();
        let modfile = ws.join(format!("src/{name}.rs"));
        if !modfile.exists() {
            changes.push(FileChange::create(
                format!("src/{name}.rs"),
                format!("//! Module `{name}`, scaffolded by kosmo.\n"),
            ));
        }
        let lib = ws.join("src/lib.rs");
        let decl = format!("pub mod {name};");
        if lib.exists() {
            let mut content = std::fs::read_to_string(&lib).unwrap_or_default();
            if !content.contains(&format!("mod {name}")) {
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(&decl);
                content.push('\n');
                changes.push(FileChange::modify("src/lib.rs", content));
            }
        } else {
            changes.push(FileChange::create("src/lib.rs", format!("{decl}\n")));
        }
        changes
    }

    fn scaffold_crate(ws: &Path, name: &str) -> Vec<FileChange> {
        let mut changes = Vec::new();
        if !ws.join(name).join("Cargo.toml").exists() {
            changes.push(FileChange::create(
                format!("{name}/Cargo.toml"),
                format!(
                    "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
                ),
            ));
            changes.push(FileChange::create(
                format!("{name}/src/lib.rs"),
                format!("//! Crate `{name}`, scaffolded by kosmo.\n"),
            ));
        }
        // Best-effort: register the new crate in the root workspace members.
        if let Ok(content) = std::fs::read_to_string(ws.join("Cargo.toml")) {
            if content.contains("members") && !content.contains(&format!("\"{name}\"")) {
                if let Some(updated) = register_member(&content, name) {
                    changes.push(FileChange::modify("Cargo.toml", updated));
                }
            }
        }
        changes
    }
}

/// Insert `"<name>",` at the head of the first `members = [ … ]` array in a
/// workspace manifest. Returns `None` if no `members [` is found.
fn register_member(toml: &str, name: &str) -> Option<String> {
    let members = toml.find("members")?;
    let bracket = toml[members..].find('[')? + members;
    let mut out = String::with_capacity(toml.len() + name.len() + 8);
    out.push_str(&toml[..=bracket]);
    out.push_str(&format!("\n    \"{name}\","));
    out.push_str(&toml[bracket + 1..]);
    Some(out)
}

impl ActionSynthesizer for FacetScaffolder {
    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisResult, SynthesisError> {
        let changes = match &request.action_item.kind {
            ActionItemKind::RealizeWishFacet { facet } => {
                self.scaffold(&request.workspace_path, facet)
            }
            _ => vec![],
        };
        let (rationale, confidence) = if changes.is_empty() {
            ("no structural scaffold for this action".to_string(), Q16::HALF)
        } else {
            (format!("scaffold {} change(s) toward the wish", changes.len()), Q16::ONE)
        };
        let patch = Patch::new(request.request_id, changes, "facet-scaffolder");
        Ok(SynthesisResult::new(patch, rationale, confidence))
    }

    fn name(&self) -> &str {
        "facet-scaffolder"
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{PolicyProfile, Q16};
    use kosmo_pipeline::{IntegrationRunOptions, run_workspace_pipeline};

    fn make_request() -> SynthesisRequest {
        let policy = PolicyProfile::default_report_only();
        let options = IntegrationRunOptions::report_only();
        // Use a temp dir — no .rs files so the workspace is minimal but valid.
        let tmpdir = std::env::temp_dir();
        let tmpdir_str = tmpdir.to_string_lossy().to_string();
        let report = run_workspace_pipeline(&tmpdir_str, &options, &policy).unwrap();
        let items = report.action_items();
        if let Some(item) = items.into_iter().next() {
            SynthesisRequest::new(item, tmpdir)
        } else {
            // Fallback: construct a minimal request with a zero digest
            let item = kosmo_pipeline::ActionItem {
                action_id: Digest::ZERO,
                priority_score: Q16::ONE,
                kind: kosmo_pipeline::ActionItemKind::FillVoid { void_id: Digest::ZERO },
                description: "test".into(),
                policy_id: policy.id,
            };
            SynthesisRequest::new(item, tmpdir)
        }
    }

    #[test]
    fn mock_confident_synthesizer_returns_result() {
        let s = MockSynthesizer::confident();
        let req = make_request();
        let result = s.synthesize(&req).unwrap();
        assert!(result.confidence > Q16::HALF);
        assert_eq!(result.patch.request_id, req.request_id);
    }

    #[test]
    fn mock_uncertain_synthesizer_has_low_confidence() {
        let s = MockSynthesizer::uncertain();
        let req = make_request();
        let result = s.synthesize(&req).unwrap();
        assert!(result.confidence < Q16::HALF);
    }

    #[test]
    fn patch_id_is_deterministic() {
        let req = make_request();
        let p1 = Patch::new(req.request_id, vec![], "test");
        let p2 = Patch::new(req.request_id, vec![], "test");
        assert_eq!(p1.patch_id, p2.patch_id);
    }

    #[test]
    fn patch_id_changes_with_content() {
        let req = make_request();
        let p_empty = Patch::empty(req.request_id);
        let p_file = Patch::new(
            req.request_id,
            vec![FileChange::create("src/foo.rs", "pub fn foo() {}")],
            "test",
        );
        assert_ne!(p_empty.patch_id, p_file.patch_id);
    }

    #[test]
    fn patch_file_order_is_canonical() {
        let req = make_request();
        let changes_ab = vec![
            FileChange::create("b.rs", "b"),
            FileChange::create("a.rs", "a"),
        ];
        let mut changes_ba = changes_ab.clone();
        changes_ba.reverse();
        // Patch sorts by path — both orderings produce the same patch_id.
        let pa = Patch::new(req.request_id, changes_ab, "test");
        let pb = Patch::new(req.request_id, changes_ba, "test");
        assert_eq!(pa.patch_id, pb.patch_id);
    }

    #[test]
    fn synthesis_result_id_is_deterministic() {
        let s = MockSynthesizer::confident();
        let req = make_request();
        let r1 = s.synthesize(&req).unwrap();
        let r2 = s.synthesize(&req).unwrap();
        assert_eq!(r1.result_id, r2.result_id);
    }

    #[test]
    fn request_id_depends_on_workspace_path() {
        let _s = MockSynthesizer::confident();
        let policy = PolicyProfile::default_report_only();
        let item = kosmo_pipeline::ActionItem {
            action_id: Digest::ZERO,
            priority_score: Q16::ONE,
            kind: kosmo_pipeline::ActionItemKind::FillVoid { void_id: Digest::ZERO },
            description: "test".into(),
            policy_id: policy.id,
        };
        let r1 = SynthesisRequest::new(item.clone(), "/workspace/a");
        let r2 = SynthesisRequest::new(item, "/workspace/b");
        assert_ne!(r1.request_id, r2.request_id);
    }

    #[test]
    fn file_change_line_count() {
        let fc = FileChange::create("foo.rs", "line1\nline2\nline3");
        assert_eq!(fc.line_count(), 3);
        let del = FileChange::delete("bar.rs");
        assert_eq!(del.line_count(), 0);
    }

    #[test]
    fn mock_synthesizer_with_change() {
        let s = MockSynthesizer::confident()
            .with_change(FileChange::create("src/new.rs", "pub fn new() {}"));
        let req = make_request();
        let result = s.synthesize(&req).unwrap();
        assert_eq!(result.patch.file_changes.len(), 1);
        assert!(!result.patch.is_empty());
        assert!(result.patch.total_lines() > 0);
    }

    // ── FacetScaffolder ───────────────────────────────────────────────────

    fn temp_ws(lib: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-synth-scaffold-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), lib).unwrap();
        dir
    }

    fn wish_request(ws: &std::path::Path, facet: WishFacet) -> SynthesisRequest {
        let action = ActionItem {
            action_id: Digest::ZERO,
            priority_score: Q16::ONE,
            kind: ActionItemKind::RealizeWishFacet { facet },
            description: "realize".into(),
            policy_id: Digest::ZERO,
        };
        SynthesisRequest::new(action, ws.to_str().unwrap())
    }

    #[test]
    fn scaffold_symbol_appends_pub_fn() {
        let dir = temp_ws("pub fn existing() {}\n");
        let req = wish_request(&dir, WishFacet::symbol("newfn"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let fc = &res.patch.file_changes[0];
        assert_eq!(fc.path.to_str().unwrap(), "src/lib.rs");
        assert!(fc.content.contains("pub fn newfn"));
        assert!(fc.content.contains("pub fn existing"), "existing content preserved");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_symbol_is_idempotent() {
        let dir = temp_ws("pub fn already_here() {}\n");
        let req = wish_request(&dir, WishFacet::symbol("already_here"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res.patch.is_empty(), "already-present symbol → no change");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_module_creates_file_and_declaration() {
        let dir = temp_ws("// crate root\n");
        let req = wish_request(&dir, WishFacet::module("widgets"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res
            .patch
            .file_changes
            .iter()
            .any(|c| c.path.to_str() == Some("src/widgets.rs")));
        assert!(res
            .patch
            .file_changes
            .iter()
            .any(|c| c.path.to_str() == Some("src/lib.rs") && c.content.contains("pub mod widgets")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_ignores_non_wish_actions() {
        let dir = temp_ws("pub fn x() {}\n");
        let action = ActionItem {
            action_id: Digest::ZERO,
            priority_score: Q16::ONE,
            kind: ActionItemKind::FillVoid { void_id: Digest::ZERO },
            description: "void".into(),
            policy_id: Digest::ZERO,
        };
        let req = SynthesisRequest::new(action, dir.to_str().unwrap());
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res.patch.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_signature_emits_fn_with_arity() {
        let dir = temp_ws("// root\n");
        let req = wish_request(&dir, WishFacet::signature("compute/2"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let fc = &res.patch.file_changes[0];
        assert!(fc.content.contains("pub fn compute(_a0: (), _a1: ())"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_capability_adds_marker() {
        let dir = temp_ws("// root\n");
        let req = wish_request(&dir, WishFacet::capability("http-server"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res.patch.file_changes[0]
            .content
            .contains("// kosmo:capability: http-server"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_test_emits_test_fn() {
        let dir = temp_ws("// root\n");
        let req = wish_request(&dir, WishFacet::test("it_works"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let c = &res.patch.file_changes[0].content;
        assert!(c.contains("#[test]"));
        assert!(c.contains("fn it_works"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
