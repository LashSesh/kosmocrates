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

use kosmo_core::{Digest, WishFacet, WishFacetKind, Q16};
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
        let workspace_path_hash = Digest::of(&workspace_path.to_string_lossy().to_string());
        let request_id = Digest::of(&RequestContent {
            action_id: action_item.action_id,
            workspace_path_hash,
            policy_id: action_item.policy_id,
        });
        Self {
            request_id,
            action_item,
            workspace_path,
            source_snippets: vec![],
        }
    }

    pub fn with_snippets(mut self, snippets: Vec<SourceSnippet>) -> Self {
        self.source_snippets = snippets;
        self
    }
}

// ─── Patch ────────────────────────────────────────────────────────────────────

/// Whether a [`FileChange`] creates, modifies, or removes a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileChangeKind {
    Create,
    Modify,
    Delete,
}

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
        Self {
            path: path.into(),
            kind: FileChangeKind::Create,
            content: content.into(),
        }
    }
    pub fn modify(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: FileChangeKind::Modify,
            content: content.into(),
        }
    }
    pub fn delete(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: FileChangeKind::Delete,
            content: String::new(),
        }
    }
    /// Line count of the proposed content.
    pub fn line_count(&self) -> u32 {
        if self.content.is_empty() {
            0
        } else {
            self.content.lines().count() as u32
        }
    }
}

#[derive(Serialize)]
struct PatchContent {
    request_id: Digest,
    changes_hash: Digest,
}

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
        let patch_id = Digest::of(&PatchContent {
            request_id,
            changes_hash,
        });
        Self {
            patch_id,
            request_id,
            file_changes,
            model_hint: model_hint.into(),
        }
    }

    pub fn empty(request_id: Digest) -> Self {
        Self::new(request_id, vec![], "empty")
    }

    pub fn is_empty(&self) -> bool {
        self.file_changes.is_empty()
    }

    /// Total proposed line additions across all file changes.
    pub fn total_lines(&self) -> u32 {
        self.file_changes.iter().map(|fc| fc.line_count()).sum()
    }
}

// ─── SynthesisResult ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ResultContent {
    patch_id: Digest,
    confidence_raw: i64,
}

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
    /// Downstream callers (e.g. `kosmo-agent`'s `AgentSession`) may skip low-confidence results.
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
        Self {
            message: msg.into(),
            recoverable: false,
        }
    }
    pub fn transient(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            recoverable: true,
        }
    }
}

impl fmt::Display for SynthesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "synthesis error (recoverable={}): {}",
            self.recoverable, self.message
        )
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
    fn token_budget(&self) -> u32 {
        4096
    }
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
            format!(
                "mock synthesis for action {}",
                &request.action_item.action_id.to_hex()[..8]
            ),
            self.confidence,
        ))
    }

    fn name(&self) -> &str {
        &self.label
    }
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
        // Crate-targeting: a key ending in "@<crate>" scaffolds the item INTO
        // that workspace member instead of the root crate. `@` appears in no
        // other facet key, so it is an unambiguous separator.
        if Self::crate_targetable(&facet.kind) {
            if let Some((bare, crate_name)) = facet.key.rsplit_once('@') {
                if !bare.is_empty() && !crate_name.is_empty() {
                    return Self::scaffold_into_crate(ws, &facet.kind, bare, crate_name);
                }
            }
        }
        Self::scaffold_kind(ws, &facet.kind, &facet.key)
    }

    /// Dispatch a facet to its per-kind scaffolder, treating `ws` as the crate
    /// root. Paths in the returned changes are relative to `ws`.
    fn scaffold_kind(ws: &Path, kind: &WishFacetKind, key: &str) -> Vec<FileChange> {
        match kind {
            WishFacetKind::Symbol => Self::scaffold_symbol(ws, key),
            WishFacetKind::Signature => Self::scaffold_signature(ws, key),
            WishFacetKind::Contract => Self::scaffold_contract(ws, key),
            WishFacetKind::Module => Self::scaffold_module(ws, key),
            WishFacetKind::Crate => Self::scaffold_crate(ws, key),
            WishFacetKind::Capability => Self::scaffold_capability(ws, key),
            WishFacetKind::Test => Self::scaffold_test(ws, key),
            WishFacetKind::Behavior => Self::scaffold_behavior(ws, key),
            WishFacetKind::Composition => Self::scaffold_composition(ws, key),
            WishFacetKind::Run => Self::scaffold_run(ws, key),
            WishFacetKind::Service => Self::scaffold_service(ws, key),
            WishFacetKind::Dependency => Self::scaffold_dependency(ws, key),
            // A resolution ("the bad thing is gone") has no structural scaffold.
            WishFacetKind::Resolution => vec![],
        }
    }

    /// Which facet kinds describe an *item inside a crate* (and so can be
    /// crate-targeted with `@<crate>`). `Crate` / `Dependency` / `Resolution`
    /// are workspace-level, and `Behavior` is observed by a suite-wide test run,
    /// so none of those take a crate suffix.
    fn crate_targetable(kind: &WishFacetKind) -> bool {
        matches!(
            kind,
            WishFacetKind::Symbol
                | WishFacetKind::Signature
                | WishFacetKind::Contract
                | WishFacetKind::Module
                | WishFacetKind::Capability
                | WishFacetKind::Test
        )
    }

    /// Scaffold `bare` (the key without its `@<crate>` suffix) of kind `kind`
    /// INTO the member crate `crate_name`: run the per-kind scaffolder as if the
    /// crate's directory were the root, then re-base the change paths to be
    /// workspace-root-relative. An unknown crate is an honest no-op.
    fn scaffold_into_crate(
        ws: &Path,
        kind: &WishFacetKind,
        bare: &str,
        crate_name: &str,
    ) -> Vec<FileChange> {
        let manifests = find_crate_manifests(ws);
        let Some((_, manifest)) = manifests.iter().find(|(n, _)| n == crate_name) else {
            return vec![];
        };
        let Some(crate_dir) = manifest.parent() else {
            return vec![];
        };
        let rel = crate_dir
            .strip_prefix(ws)
            .unwrap_or(crate_dir)
            .to_path_buf();
        Self::scaffold_kind(crate_dir, kind, bare)
            .into_iter()
            .map(|fc| FileChange {
                path: rel.join(&fc.path),
                kind: fc.kind,
                content: fc.content,
            })
            .collect()
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

    /// Realize a typed contract `"name(T0,T1)->R"` by appending a typed
    /// function stub with a `todo!()` body — structurally present, **honestly
    /// empty** at runtime. The dual of `kosmo-intent`'s contract observation,
    /// so scaffold → observe round-trips and the descent converges. An
    /// unparseable key (no parameter list) is an honest no-op.
    fn scaffold_contract(ws: &Path, key: &str) -> Vec<FileChange> {
        let Some((name, params, ret)) = parse_contract_key(key) else {
            return vec![];
        };
        let param_list = params
            .iter()
            .enumerate()
            .map(|(i, t)| format!("_a{i}: {t}"))
            .collect::<Vec<_>>()
            .join(", ");
        // `-> ()` is omitted: the no-arrow form observes back to the same `()`
        // return, and avoids a clippy `unused_unit` on every scaffold.
        let arrow = if ret == "()" {
            String::new()
        } else {
            format!(" -> {ret}")
        };
        Self::append_to_lib(
            ws,
            &format!("fn {name}"),
            &format!("pub fn {name}({param_list}){arrow} {{ todo!(\"kosmo: implement {name}\") }}"),
        )
    }

    /// Append several `(marker, snippet)` items to the crate's lib in a
    /// **single** change, skipping any whose marker is already present. Needed
    /// when one facet must add several items: returning two changes for the same
    /// path would have the second overwrite the first on apply.
    fn append_items_to_lib(ws: &Path, items: &[(String, String)]) -> Vec<FileChange> {
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
        let mut added = false;
        for (marker, snippet) in items {
            if content.contains(marker.as_str()) {
                continue;
            }
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(snippet);
            if !snippet.ends_with('\n') {
                content.push('\n');
            }
            added = true;
        }
        if !added {
            return vec![];
        }
        let fc = if exists {
            FileChange::modify(rel, content)
        } else {
            FileChange::create(rel, content)
        };
        vec![fc]
    }

    /// Realize a typed composition `"from>>via>>to"` by scaffolding two
    /// type-compatible stubs in one change: `pub fn from() -> via` and
    /// `pub fn to(_a0: via)`. Once both exist with matching types the observer
    /// derives the composition, so it round-trips. Honestly empty (`todo!()`);
    /// idempotent via the `fn from` / `fn to` markers; malformed key → no-op.
    fn scaffold_composition(ws: &Path, key: &str) -> Vec<FileChange> {
        let parts: Vec<&str> = key.split(">>").collect();
        if parts.len() != 3 {
            return vec![];
        }
        let (from, via, to) = (parts[0].trim(), parts[1].trim(), parts[2].trim());
        if from.is_empty() || via.is_empty() || to.is_empty() {
            return vec![];
        }
        Self::append_items_to_lib(
            ws,
            &[
                (
                    format!("fn {from}"),
                    format!("pub fn {from}() -> {via} {{ todo!(\"kosmo: implement {from}\") }}"),
                ),
                (
                    format!("fn {to}"),
                    format!("pub fn {to}(_a0: {via}) {{ todo!(\"kosmo: implement {to}\") }}"),
                ),
            ],
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

    /// Realize a behaviour spec `"name(args)=>expected"` (the keystone) by
    /// appending a spec-test that pins the input→output pair:
    ///
    /// ```text
    /// // kosmo:behavior: add(2,3)=>5
    /// #[test]
    /// fn kosmo_spec_<hash>() { assert_eq!(add(2, 3), 5); }
    /// ```
    ///
    /// The test is **red** until `add` actually returns `5` — the deterministic
    /// scaffolder cannot write the body, so a behaviour wish converges only once
    /// the implementation is correct (filled by the LLM fallback, or already
    /// present). The `// kosmo:behavior:` marker carries the spec verbatim so
    /// the observer can pair this test back to its facet (and emit it only when
    /// green). Idempotent via the marker line; an unparseable key is a no-op.
    fn scaffold_behavior(ws: &Path, key: &str) -> Vec<FileChange> {
        let Some((call, expected)) = parse_behavior_key(key) else {
            return vec![];
        };
        let hash = Digest::of_bytes(key.as_bytes()).to_hex();
        let test_name = format!("kosmo_spec_{}", &hash[..12]);
        let marker = format!("kosmo:behavior: {key}");
        Self::append_to_lib(
            ws,
            &marker,
            &format!(
                "// {marker}\n#[test]\nfn {test_name}() {{ assert_eq!({call}, {expected}); }}"
            ),
        )
    }

    /// Realize a runtime probe `"args=>expect"` by ensuring a **bin target**
    /// exists and carries a `// kosmo:run:` marker (so the observer can find and
    /// execute it). The stub `main` is **honestly inert** — red until the body
    /// is correct (the keystone, at the process boundary).
    fn scaffold_run(ws: &Path, key: &str) -> Vec<FileChange> {
        Self::scaffold_bin_probe(ws, "run", key)
    }

    /// Realize a service probe `"method:path=>expect"` by ensuring a bin target
    /// carrying a `// kosmo:service:` marker. The stub `main` exits immediately
    /// (binds nothing) — so the probe is red until a real server is written.
    fn scaffold_service(ws: &Path, key: &str) -> Vec<FileChange> {
        Self::scaffold_bin_probe(ws, "service", key)
    }

    /// Ensure a bin target carrying a `// kosmo:<kind>:` probe marker — the
    /// shared mechanism behind `Run` and `Service`. Idempotent via the marker.
    fn scaffold_bin_probe(ws: &Path, kind: &str, key: &str) -> Vec<FileChange> {
        let marker = format!("kosmo:{kind}: {key}");
        let main = ws.join("src/main.rs");
        if main.exists() {
            let mut content = std::fs::read_to_string(&main).unwrap_or_default();
            if content.contains(&marker) {
                return vec![];
            }
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(&format!("// {marker}\n"));
            vec![FileChange::modify("src/main.rs", content)]
        } else {
            vec![FileChange::create(
                "src/main.rs",
                format!(
                    "// {marker}\nfn main() {{ /* kosmo: {kind} stub — fill to satisfy the probe */ }}\n"
                ),
            )]
        }
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

    /// Realize a dependency edge `"from->to"` by adding a path dependency on
    /// `to` to `from`'s `Cargo.toml`. Both crates are located by package name
    /// within the workspace; the wish is left untouched (empty patch) if either
    /// is missing or the edge already exists.
    fn scaffold_dependency(ws: &Path, key: &str) -> Vec<FileChange> {
        let Some((from, to)) = key.split_once("->") else {
            return vec![];
        };
        let (from, to) = (from.trim(), to.trim());
        if from.is_empty() || to.is_empty() || from == to {
            return vec![];
        }
        let manifests = find_crate_manifests(ws);
        let from_path = manifests
            .iter()
            .find(|(n, _)| n == from)
            .map(|(_, p)| p.clone());
        let to_path = manifests
            .iter()
            .find(|(n, _)| n == to)
            .map(|(_, p)| p.clone());
        let (Some(from_path), Some(to_path)) = (from_path, to_path) else {
            return vec![]; // can't locate one or both crates — an honest no-op
        };
        let (Some(from_dir), Some(to_dir)) = (from_path.parent(), to_path.parent()) else {
            return vec![];
        };
        let content = std::fs::read_to_string(&from_path).unwrap_or_default();
        if dep_already_present(&content, to) {
            return vec![]; // idempotent
        }
        let rel = relative_path(from_dir, to_dir);
        let new_content = add_path_dependency(&content, to, &rel);
        let rel_manifest = from_path.strip_prefix(ws).unwrap_or(&from_path);
        vec![FileChange::modify(rel_manifest, new_content)]
    }
}

/// Parse a contract key `"name(T0,T1)->R"` into `(name, param_types, ret)`.
/// Mirrors `kosmo-intent`'s contract observation. `None` if the key has no
/// parameter list. The parameter list and `->` are matched depth-aware so
/// nested generics / closure types do not unbalance the scan.
fn parse_contract_key(key: &str) -> Option<(String, Vec<String>, String)> {
    let chars: Vec<char> = key.chars().collect();
    let open = chars.iter().position(|&c| c == '(')?;
    let name: String = chars[..open].iter().collect();
    let name = name.trim().to_string();
    if name.is_empty() {
        return None;
    }
    // Match the parameter-list close paren, treating `->` as an atom.
    let mut depth = 0i32;
    let mut close = None;
    let mut prev = ' ';
    for (i, &c) in chars.iter().enumerate().skip(open) {
        let is_arrow_gt = c == '>' && prev == '-';
        if !is_arrow_gt {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        prev = c;
    }
    let close = close?;
    let params_str: String = chars[open + 1..close].iter().collect();
    let params = split_type_list(&params_str);
    let rest: String = chars[close + 1..].iter().collect();
    let ret = rest
        .trim_start()
        .strip_prefix("->")
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| "()".to_string());
    Some((name, params, ret))
}

/// Parse a behaviour key `"name(args)=>expected"` into `(call, expected)`,
/// where `call` is the call expression `"name(a, b)"` (args re-joined with
/// `", "`) and `expected` is the verbatim expected expression. `None` if the
/// key has no `=>` separator or no parameter list.
fn parse_behavior_key(key: &str) -> Option<(String, String)> {
    // Behavioural composition (the runtime floor's sandbox-free on-ramp): a
    // piped spec `"f(x)>>g>>h=>expected"` desugars to the nested application
    // `"h(g(f(x)))"`, validated by the very same green-test judge. `>>` overloads
    // `>`, which `split_on_fat_arrow` reads as a bracket, so pipes take a
    // dedicated parser that splits on `=>` and `>>` as plain strings.
    if key.contains(">>") {
        return parse_pipeline_behavior_key(key);
    }
    let (lhs, rhs) = split_on_fat_arrow(key)?;
    let lhs = lhs.trim();
    let expected = rhs.trim().to_string();
    if expected.is_empty() {
        return None;
    }
    let chars: Vec<char> = lhs.chars().collect();
    let open = chars.iter().position(|&c| c == '(')?;
    let name: String = chars[..open].iter().collect();
    let name = name.trim().to_string();
    if name.is_empty() {
        return None;
    }
    // Match the call's close paren depth-aware.
    let mut depth = 0i32;
    let mut close = None;
    for (i, &c) in chars.iter().enumerate().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let args_str: String = chars[open + 1..close].iter().collect();
    let args = split_type_list(&args_str); // depth-aware top-level split
    Some((format!("{name}({})", args.join(", ")), expected))
}

/// Desugar a piped behaviour key `"f(x)>>g>>h=>expected"` into the composed
/// call `("h(g(f(x)))", "expected")`. The head stage carries the pipeline input
/// (`f(x)`); each later stage is a bare function the piped value flows into. The
/// expected value is taken after the last `=>` (robust to `=>` inside a string
/// literal arg). Fewer than two stages, or an empty stage/expected, is a no-op.
fn parse_pipeline_behavior_key(key: &str) -> Option<(String, String)> {
    let (call_part, expected) = key.rsplit_once("=>")?;
    let expected = expected.trim().to_string();
    if expected.is_empty() {
        return None;
    }
    let stages: Vec<&str> = call_part.split(">>").map(|s| s.trim()).collect();
    if stages.len() < 2 {
        return None;
    }
    let mut acc = stages[0].to_string();
    if acc.is_empty() {
        return None;
    }
    for stage in &stages[1..] {
        if stage.is_empty() {
            return None;
        }
        acc = format!("{stage}({acc})");
    }
    Some((acc, expected))
}

/// Find the first **top-level** `=>` (ignoring `()`, `<>`, `[]` nesting) and
/// split there. Returns `(left, right)` excluding the `=>`.
fn split_on_fat_arrow(s: &str) -> Option<(String, String)> {
    let chars: Vec<char> = s.chars().collect();
    let mut depth = 0i32;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth -= 1,
            '=' if depth == 0 && chars.get(i + 1) == Some(&'>') => {
                let left: String = chars[..i].iter().collect();
                let right: String = chars[i + 2..].iter().collect();
                return Some((left, right));
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Split a comma-separated type list on top-level commas, ignoring commas
/// nested in `()`, `<>`, `[]`; `->` is an atom. Empty segments are dropped.
fn split_type_list(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '-' if chars.peek() == Some(&'>') => {
                cur.push('-');
                cur.push(chars.next().unwrap());
            }
            '(' | '<' | '[' => {
                depth += 1;
                cur.push(c);
            }
            ')' | '>' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                let t = cur.trim().to_string();
                if !t.is_empty() {
                    out.push(t);
                }
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    let t = cur.trim().to_string();
    if !t.is_empty() {
        out.push(t);
    }
    out
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

/// Walk `ws` for `Cargo.toml` files and return `(package_name, manifest_path)`
/// for each that declares a `[package]`. Skips `target` and dotted directories.
fn find_crate_manifests(ws: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![ws.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                stack.push(path);
            } else if path.file_name().is_some_and(|f| f == "Cargo.toml") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(name) = package_name(&content) {
                        out.push((name, path));
                    }
                }
            }
        }
    }
    out
}

/// Extract `name = "…"` from a manifest's `[package]` section.
fn package_name(toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = line.strip_prefix("name") {
                if let Some(val) = rest.trim_start().strip_prefix('=') {
                    let v = val.trim().trim_matches('"');
                    if !v.is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Relative path from directory `from` to directory `to`, `/`-joined
/// (e.g. `"../b"`). Both are expected to share a common ancestor.
fn relative_path(from: &Path, to: &Path) -> String {
    let from_comps: Vec<_> = from.components().collect();
    let to_comps: Vec<_> = to.components().collect();
    let mut i = 0;
    while i < from_comps.len() && i < to_comps.len() && from_comps[i] == to_comps[i] {
        i += 1;
    }
    let mut parts: Vec<String> =
        std::iter::repeat_n("..".to_string(), from_comps.len() - i).collect();
    for c in &to_comps[i..] {
        parts.push(c.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

/// Is `dep` already declared as a dependency line in `toml`?
fn dep_already_present(toml: &str, dep: &str) -> bool {
    toml.lines().any(|l| {
        l.trim()
            .strip_prefix(dep)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    })
}

/// Add `dep = { path = "<rel>" }` to the `[dependencies]` section, creating the
/// section if it is absent.
fn add_path_dependency(toml: &str, dep: &str, rel: &str) -> String {
    let dep_line = format!("{dep} = {{ path = \"{rel}\" }}");
    let mut lines: Vec<String> = toml.lines().map(|l| l.to_string()).collect();
    if let Some(i) = lines.iter().position(|l| l.trim() == "[dependencies]") {
        lines.insert(i + 1, dep_line);
    } else {
        lines.push(String::new());
        lines.push("[dependencies]".to_string());
        lines.push(dep_line);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
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
            (
                "no structural scaffold for this action".to_string(),
                Q16::HALF,
            )
        } else {
            (
                format!("scaffold {} change(s) toward the wish", changes.len()),
                Q16::ONE,
            )
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
    use kosmo_pipeline::{run_workspace_pipeline, IntegrationRunOptions};

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
                kind: kosmo_pipeline::ActionItemKind::FillVoid {
                    void_id: Digest::ZERO,
                },
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
            kind: kosmo_pipeline::ActionItemKind::FillVoid {
                void_id: Digest::ZERO,
            },
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
        assert!(
            fc.content.contains("pub fn existing"),
            "existing content preserved"
        );
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
        assert!(res.patch.file_changes.iter().any(
            |c| c.path.to_str() == Some("src/lib.rs") && c.content.contains("pub mod widgets")
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_ignores_non_wish_actions() {
        let dir = temp_ws("pub fn x() {}\n");
        let action = ActionItem {
            action_id: Digest::ZERO,
            priority_score: Q16::ONE,
            kind: ActionItemKind::FillVoid {
                void_id: Digest::ZERO,
            },
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
    fn scaffold_contract_emits_typed_stub() {
        let dir = temp_ws("// root\n");
        let req = wish_request(
            &dir,
            WishFacet::contract("handle", &["Request"], "Response"),
        );
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let c = &res.patch.file_changes[0].content;
        assert!(
            c.contains("pub fn handle(_a0: Request) -> Response"),
            "got:\n{c}"
        );
        assert!(c.contains("todo!"), "body must be honestly empty: {c}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_contract_unit_return_omits_arrow() {
        let dir = temp_ws("// root\n");
        let req = wish_request(&dir, WishFacet::contract("tick", &[], "()"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let c = &res.patch.file_changes[0].content;
        assert!(c.contains("pub fn tick()"), "got:\n{c}");
        assert!(
            !c.contains("-> ()"),
            "unit return should omit the arrow: {c}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_contract_multi_arg_generic() {
        let dir = temp_ws("// root\n");
        let req = wish_request(
            &dir,
            WishFacet::contract("count", &["Vec<User>", "usize"], "usize"),
        );
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let c = &res.patch.file_changes[0].content;
        assert!(
            c.contains("pub fn count(_a0: Vec<User>, _a1: usize) -> usize"),
            "got:\n{c}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_contract_is_idempotent() {
        let dir = temp_ws("pub fn handle(_a0: Request) -> Response { todo!() }\n");
        let req = wish_request(
            &dir,
            WishFacet::contract("handle", &["Request"], "Response"),
        );
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res.patch.is_empty(), "existing fn name → no change");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parse_contract_key_roundtrips_through_split() {
        let (name, params, ret) = parse_contract_key("count(Vec<User>,usize)->usize").unwrap();
        assert_eq!(name, "count");
        assert_eq!(params, vec!["Vec<User>".to_string(), "usize".to_string()]);
        assert_eq!(ret, "usize");
        // No parameter list → not a contract key.
        assert!(parse_contract_key("bare_name").is_none());
    }

    #[test]
    fn scaffold_behavior_emits_marked_spec_test() {
        let dir = temp_ws("// root\n");
        let req = wish_request(&dir, WishFacet::behavior("add(2,3)=>5"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let c = &res.patch.file_changes[0].content;
        // The marker carries the spec verbatim so the observer can pair it back.
        assert!(c.contains("// kosmo:behavior: add(2,3)=>5"), "got:\n{c}");
        assert!(c.contains("#[test]"));
        assert!(c.contains("assert_eq!(add(2, 3), 5);"), "got:\n{c}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_behavior_is_idempotent() {
        let dir = temp_ws("// kosmo:behavior: add(2,3)=>5\n#[test]\nfn kosmo_spec_x() {}\n");
        let req = wish_request(&dir, WishFacet::behavior("add(2,3)=>5"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res.patch.is_empty(), "marker already present → no change");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parse_behavior_key_splits_call_and_expected() {
        let (call, expected) = parse_behavior_key("greet(\"x\",2)=>\"hi x\"").unwrap();
        assert_eq!(call, "greet(\"x\", 2)");
        assert_eq!(expected, "\"hi x\"");
        // Missing `=>` → not a behaviour key.
        assert!(parse_behavior_key("add(2,3)").is_none());
    }

    #[test]
    fn parse_pipeline_behavior_key_folds_into_nested_call() {
        // Behavioural composition: input on the head stage, piped forward.
        let (call, expected) = parse_behavior_key("parse(\"2+3\")>>eval=>5").unwrap();
        assert_eq!(call, "eval(parse(\"2+3\"))");
        assert_eq!(expected, "5");
    }

    #[test]
    fn parse_pipeline_behavior_key_chains_three_stages() {
        let (call, expected) = parse_behavior_key("a(1)>>b>>c=>9").unwrap();
        assert_eq!(call, "c(b(a(1)))");
        assert_eq!(expected, "9");
        // A lone stage (no pipe) is not a pipeline.
        assert!(parse_behavior_key("a(1)>>=>9").is_none());
    }

    #[test]
    fn scaffold_behavior_pipeline_emits_composed_assert() {
        let dir = temp_ws("// root\n");
        let req = wish_request(&dir, WishFacet::behavior("parse(\"2+3\")>>eval=>5"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let c = &res.patch.file_changes[0].content;
        assert!(
            c.contains("assert_eq!(eval(parse(\"2+3\")), 5)"),
            "composed assert missing:\n{c}"
        );
        // The marker carries the piped key verbatim, so it round-trips on observe.
        assert!(c.contains("// kosmo:behavior: parse(\"2+3\")>>eval=>5"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_composition_emits_two_compatible_stubs_in_one_change() {
        let dir = temp_ws("// root\n");
        let req = wish_request(&dir, WishFacet::composition("parse", "String", "eval"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert_eq!(
            res.patch.file_changes.len(),
            1,
            "one change for the same file, not two (the second would overwrite)"
        );
        let c = &res.patch.file_changes[0].content;
        assert!(c.contains("pub fn parse() -> String"), "got:\n{c}");
        assert!(c.contains("pub fn eval(_a0: String)"), "got:\n{c}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_composition_idempotent_when_both_present() {
        let dir =
            temp_ws("pub fn parse() -> String { todo!() }\npub fn eval(_a0: String) { todo!() }\n");
        let req = wish_request(&dir, WishFacet::composition("parse", "String", "eval"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res.patch.is_empty(), "both fns present → no change");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_run_creates_bin_with_marker() {
        let dir = temp_ws("// lib\n"); // lib-only crate: no bin yet
        let req = wish_request(&dir, WishFacet::run("add,2,3=>out~5"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let fc = &res.patch.file_changes[0];
        assert_eq!(fc.path, std::path::Path::new("src/main.rs"));
        assert!(
            fc.content.contains("// kosmo:run: add,2,3=>out~5"),
            "marker:\n{}",
            fc.content
        );
        assert!(fc.content.contains("fn main"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_run_idempotent_when_marker_present() {
        let dir = temp_ws("// lib\n");
        std::fs::write(
            dir.join("src/main.rs"),
            "// kosmo:run: add,2,3=>out~5\nfn main() {}\n",
        )
        .unwrap();
        let req = wish_request(&dir, WishFacet::run("add,2,3=>out~5"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res.patch.is_empty(), "marker present → no change");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scaffold_service_creates_bin_with_marker() {
        let dir = temp_ws("// lib\n");
        let req = wish_request(&dir, WishFacet::service("GET:/health=>200"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let fc = &res.patch.file_changes[0];
        assert_eq!(fc.path, std::path::Path::new("src/main.rs"));
        assert!(
            fc.content.contains("// kosmo:service: GET:/health=>200"),
            "marker:\n{}",
            fc.content
        );
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

    #[test]
    fn scaffold_dependency_adds_path_dep() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let ws = std::env::temp_dir().join(format!("kosmo-synth-dep-{nanos}"));
        for name in ["a", "b"] {
            std::fs::create_dir_all(ws.join("crates").join(name).join("src")).unwrap();
            std::fs::write(
                ws.join("crates").join(name).join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
            )
            .unwrap();
            std::fs::write(ws.join("crates").join(name).join("src/lib.rs"), "// x\n").unwrap();
        }
        std::fs::write(
            ws.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\nresolver = \"2\"\n",
        )
        .unwrap();

        let req = wish_request(&ws, WishFacet::dependency("a", "b"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(!res.patch.is_empty(), "should propose a manifest edit");
        let fc = &res.patch.file_changes[0];
        assert!(fc.path.ends_with("Cargo.toml"));
        assert!(
            fc.content.contains("b = { path = \"../b\" }"),
            "got:\n{}",
            fc.content
        );
        assert!(fc.content.contains("[dependencies]"));

        // Idempotent: once the edge exists, a second pass proposes nothing.
        std::fs::write(ws.join("crates/a/Cargo.toml"), &fc.content).unwrap();
        let res2 = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res2.patch.is_empty(), "second pass should be a no-op");

        std::fs::remove_dir_all(&ws).ok();
    }

    /// A multi-crate workspace with members `alpha` and `beta` under `crates/`.
    fn temp_workspace() -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let ws = std::env::temp_dir().join(format!("kosmo-synth-target-{nanos}"));
        for name in ["alpha", "beta"] {
            std::fs::create_dir_all(ws.join("crates").join(name).join("src")).unwrap();
            std::fs::write(
                ws.join("crates").join(name).join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
            )
            .unwrap();
            std::fs::write(ws.join("crates").join(name).join("src/lib.rs"), "// x\n").unwrap();
        }
        std::fs::write(
            ws.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/alpha\", \"crates/beta\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        ws
    }

    #[test]
    fn scaffold_symbol_targets_named_crate() {
        let ws = temp_workspace();
        // "helper@beta" → into crates/beta, not the workspace root.
        let req = wish_request(&ws, WishFacet::symbol("helper@beta"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let fc = &res.patch.file_changes[0];
        assert_eq!(
            fc.path,
            std::path::Path::new("crates/beta/src/lib.rs"),
            "should write into the beta crate"
        );
        assert!(fc.content.contains("pub fn helper"));
        std::fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn scaffold_contract_targets_named_crate() {
        let ws = temp_workspace();
        let req = wish_request(&ws, WishFacet::contract_key("handle(String)->String@alpha"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        let fc = &res.patch.file_changes[0];
        assert_eq!(fc.path, std::path::Path::new("crates/alpha/src/lib.rs"));
        assert!(fc.content.contains("pub fn handle(_a0: String) -> String"));
        std::fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn scaffold_into_unknown_crate_is_noop() {
        let ws = temp_workspace();
        let req = wish_request(&ws, WishFacet::symbol("helper@ghost"));
        let res = FacetScaffolder.synthesize(&req).unwrap();
        assert!(res.patch.is_empty(), "unknown crate → honest no-op");
        std::fs::remove_dir_all(&ws).ok();
    }
}
