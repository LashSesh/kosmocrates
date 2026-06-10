//! Cross-language code topology — the polyglot extractor for the production substrate.
//!
//! ## Why this module exists
//!
//! [`CodeHDAG::extract_from_rust_source`](crate::code_hdag::CodeHDAG::extract_from_rust_source)
//! reads exactly one language. Everything downstream of it in the substrate's
//! hypercube materialization — `SourceCube` → `CubeSwarm` → `SystemCube` →
//! `.kcube` — is already language-agnostic: it consumes a [`CodeHDAG`] and the
//! ρ/ω structural poles derived from it, never the source text itself. The only
//! Rust-specific link in that whole chain is the extractor at its head.
//!
//! This module closes that gap. It realises *cross-language* hypercube
//! materialization: a Python, JavaScript, or Go file is lifted into the **same**
//! content-addressed [`CodeHDAG`] a Rust file would produce, so it flows into the
//! identical cube pipeline with no downstream change.
//!
//! ## What is mined, and what is deliberately left behind
//!
//! The language-independent structural taxonomy here is mined from the PSE-Codex
//! corpus (the "barbara" workspace: 10 algorithms × 4 languages) and its
//! `normalize` Rosetta table — the per-language knowledge of *what counts as* a
//! function, a type, an import, a test. That taxonomy is the reusable asset.
//!
//! PSE-Codex computes its signatures with tree-sitter parsing and `f64` spectral
//! / Kuramoto / Betti machinery. None of that is ported: it is float-heavy and
//! dependency-bound, which the substrate forbids by construction (CROSS-007 — no
//! floats in content-addressed or gate paths; no external network dependencies).
//! Instead the taxonomy is re-expressed as a deterministic, dependency-free
//! **lexical** extractor — the exact discipline of the existing Rust extractor,
//! generalised across languages.
//!
//! ## Invariants preserved
//!
//! * **Fail-closed.** An unrecognised file extension yields `None`
//!   ([`CodeHDAG::extract_auto`]); the extractor never guesses a language.
//! * **Content-addressed & deterministic.** Identical input yields an identical
//!   `hdag_id` and `fingerprint_id` (INVARIANT-007).
//! * **No floats.** [`CrossLanguageFingerprint`] carries `Q16` structural ratios
//!   only; [`CrossLanguageFingerprint::similarity`] is integer arithmetic.
//! * **Evidence-bound & taint-propagating.** Every observation backrefs its
//!   source evidence; taint flows source → graph (CROSS-006).

use crate::code_hdag::{CodeHDAG, CodeObservation, HDAGEdge, HDAGEdgeKind, HDAGNode, ObservationKind};
use kosmo_core::{Digest, Q16, TaintLabel};
use serde::{Deserialize, Serialize};

/// A source language the substrate can lift into a [`CodeHDAG`].
///
/// The four corpus languages of the PSE-Codex cross-language proof. The set is
/// closed on purpose: each variant carries hand-verified lexical rules, so
/// adding a language is a deliberate, tested act — never an inferred guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SourceLanguage {
    Rust,
    Python,
    JavaScript,
    Go,
}

impl SourceLanguage {
    /// Detect the language of a file from its path extension.
    ///
    /// Returns `None` for any extension not backed by verified lexical rules —
    /// the fail-closed default. Matching is case-insensitive on the extension.
    pub fn from_path(path: &str) -> Option<Self> {
        let filename = path.rsplit('/').next().unwrap_or(path);
        let ext = filename.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    /// Stable lowercase identifier, aligned with the PSE-Codex language strings.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::Go => "go",
        }
    }
}

/// True when `path` names a test file under the conventions of `language`.
///
/// Used by host scanning to classify polyglot test companions the same way the
/// substrate already classifies Rust `tests/` files.
pub fn is_test_path(language: SourceLanguage, path: &str) -> bool {
    let filename = path.rsplit('/').next().unwrap_or(path);
    let under_tests_dir = path.starts_with("tests/")
        || path.starts_with("__tests__/")
        || path.contains("/tests/")
        || path.contains("/__tests__/");
    match language {
        SourceLanguage::Rust => {
            under_tests_dir || filename.ends_with("_test.rs") || filename == "tests.rs"
        }
        SourceLanguage::Python => {
            under_tests_dir || filename.starts_with("test_") || filename.ends_with("_test.py")
        }
        SourceLanguage::JavaScript => {
            under_tests_dir
                || filename.ends_with(".test.js")
                || filename.ends_with(".spec.js")
                || filename.ends_with(".test.mjs")
                || filename.ends_with(".spec.mjs")
        }
        SourceLanguage::Go => filename.ends_with("_test.go"),
    }
}

// ─── Lexical extraction core ───────────────────────────────────────────────────

/// What a single recognised line contributes to the graph.
enum LineOutcome {
    /// Blank line, comment, decorator, or otherwise structurally inert.
    Skip,
    /// Emit one observation node plus an edge from the file root.
    Emit(ObservationKind, HDAGEdgeKind),
}

/// Mutable line-scanner state for the constructs that span lines.
#[derive(Default)]
struct LexState {
    /// Inside a Go `import ( … )` block.
    in_import_block: bool,
    /// Inside a JavaScript `/* … */` block comment.
    in_block_comment: bool,
    /// Inside a Python `""" … """` / `''' … '''` docstring.
    in_doc_string: bool,
    /// A Rust `#[test]`-style attribute was seen on the previous line.
    pending_test: bool,
}

impl CodeHDAG {
    /// Extract a [`CodeHDAG`] from source text in a known `language`.
    ///
    /// Rust is delegated verbatim to
    /// [`extract_from_rust_source`](Self::extract_from_rust_source) so existing
    /// `hdag_id`s and behaviour are byte-for-byte unchanged. Python, JavaScript,
    /// and Go use the dependency-free lexical extractor in this module.
    ///
    /// The result is the same content-addressed graph type the Rust path
    /// produces, so `rho_coherence`, `omega_phase`, and the tripolar energy
    /// kernel all apply without modification.
    pub fn extract_from_source(
        language: SourceLanguage,
        source_evidence_id: Digest,
        location: &str,
        content: &str,
        taint: TaintLabel,
    ) -> Self {
        match language {
            SourceLanguage::Rust => {
                Self::extract_from_rust_source(source_evidence_id, location, content, taint)
            }
            other => extract_lexical(
                classifier_for(other),
                source_evidence_id,
                location,
                content,
                taint,
            ),
        }
    }

    /// Detect the language from `location` and extract, or `None` for an
    /// unrecognised extension (fail-closed). This is the integration entry point
    /// for host scanning over a polyglot workspace.
    pub fn extract_auto(
        source_evidence_id: Digest,
        location: &str,
        content: &str,
        taint: TaintLabel,
    ) -> Option<Self> {
        SourceLanguage::from_path(location)
            .map(|lang| Self::extract_from_source(lang, source_evidence_id, location, content, taint))
    }
}

/// Line classifier for a language. Rust has one too, used only for fingerprint
/// counting — its canonical graph always comes from `extract_from_rust_source`.
fn classifier_for(language: SourceLanguage) -> fn(&str, &mut LexState) -> LineOutcome {
    match language {
        SourceLanguage::Rust => classify_rust_line,
        SourceLanguage::Python => classify_python_line,
        SourceLanguage::JavaScript => classify_javascript_line,
        SourceLanguage::Go => classify_go_line,
    }
}

/// The shared lexical driver: scan the source line by line, classify each line
/// under `classify`, and build the rooted, content-addressed [`CodeHDAG`].
///
/// Mirrors the structure of the Rust extractor exactly — one root
/// [`ObservationKind::ModuleDeclaration`] node for the file, every recognised
/// construct linked back to it — so the two paths produce structurally
/// comparable graphs.
fn extract_lexical(
    classify: fn(&str, &mut LexState) -> LineOutcome,
    source_evidence_id: Digest,
    location: &str,
    content: &str,
    taint: TaintLabel,
) -> CodeHDAG {
    let root_obs = CodeObservation::new(
        ObservationKind::ModuleDeclaration { name: location.to_string() },
        location.to_string(),
        Digest::of_bytes(format!("{location}:0:<root>").as_bytes()),
        taint.clone(),
        source_evidence_id,
    );
    let root_id = root_obs.obs_id;
    let mut nodes = vec![HDAGNode::from_observation(&root_obs)];
    let mut edges: Vec<HDAGEdge> = Vec::new();
    let mut state = LexState::default();

    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let LineOutcome::Emit(kind, edge_kind) = classify(line, &mut state) {
            let frag = Digest::of_bytes(format!("{location}:{}:{line}", idx + 1).as_bytes());
            let lineloc = format!("{location}:{}", idx + 1);
            push(
                &mut nodes,
                &mut edges,
                root_id,
                kind,
                lineloc,
                frag,
                &taint,
                source_evidence_id,
                edge_kind,
            );
        }
    }

    CodeHDAG::new(nodes, edges, source_evidence_id, taint)
}

/// Push one observation node and its edge from `root_id`. Local mirror of the
/// private helper in `code_hdag` (same semantics; kept here to avoid widening
/// that module's visibility).
#[allow(clippy::too_many_arguments)]
fn push(
    nodes: &mut Vec<HDAGNode>,
    edges: &mut Vec<HDAGEdge>,
    root_id: Digest,
    kind: ObservationKind,
    location: String,
    fragment_digest: Digest,
    taint: &TaintLabel,
    source_evidence_id: Digest,
    edge_kind: HDAGEdgeKind,
) {
    let obs = CodeObservation::new(kind, location, fragment_digest, taint.clone(), source_evidence_id);
    let node = HDAGNode::from_observation(&obs);
    edges.push(HDAGEdge { from_obs: root_id, to_obs: node.node_id, kind: edge_kind });
    nodes.push(node);
}

// ─── Per-language classifiers ──────────────────────────────────────────────────

/// Python: `def`/`class`/`import`/`from … import`, docstring-aware.
fn classify_python_line(line: &str, state: &mut LexState) -> LineOutcome {
    // Docstring tracking — count triple-quote markers on the line.
    let triples = line.matches("\"\"\"").count() + line.matches("'''").count();
    if state.in_doc_string {
        if triples % 2 == 1 {
            state.in_doc_string = false;
        }
        return LineOutcome::Skip;
    }
    if triples % 2 == 1 {
        state.in_doc_string = true; // opens a docstring this line does not close
        return LineOutcome::Skip;
    }

    if line.starts_with('#') || line.starts_with('@') {
        return LineOutcome::Skip;
    }

    if let Some(module) = parse_python_import(line) {
        return LineOutcome::Emit(ObservationKind::ImportStatement { module }, HDAGEdgeKind::Imports);
    }
    if let Some(name) = parse_keyword_name(line, &["class"]) {
        return LineOutcome::Emit(ObservationKind::TypeDefinition { name }, HDAGEdgeKind::Contains);
    }
    // `def name(...)` or `async def name(...)`.
    let def_body = line.strip_prefix("async ").unwrap_or(line).strip_prefix("def ");
    if let Some(rest) = def_body {
        if let Some(name) = first_ident(rest) {
            let is_test = is_test_name(&name);
            return function_or_test(name, count_params(rest), is_test);
        }
    }
    LineOutcome::Skip
}

/// Go: `func`/`type`/`import`, import-block-aware.
fn classify_go_line(line: &str, state: &mut LexState) -> LineOutcome {
    if line.starts_with("//") {
        return LineOutcome::Skip;
    }
    if state.in_import_block {
        if line.starts_with(')') {
            state.in_import_block = false;
            return LineOutcome::Skip;
        }
        if let Some(module) = first_quoted(line) {
            return LineOutcome::Emit(ObservationKind::ImportStatement { module }, HDAGEdgeKind::Imports);
        }
        return LineOutcome::Skip;
    }
    if line.starts_with("import (") {
        state.in_import_block = true;
        return LineOutcome::Skip;
    }
    if let Some(rest) = line.strip_prefix("import ") {
        if let Some(module) = first_quoted(rest) {
            return LineOutcome::Emit(ObservationKind::ImportStatement { module }, HDAGEdgeKind::Imports);
        }
    }
    if let Some(name) = parse_keyword_name(line, &["type"]) {
        return LineOutcome::Emit(ObservationKind::TypeDefinition { name }, HDAGEdgeKind::Contains);
    }
    // `func name(...)` or `func (recv) name(...)` (method).
    if let Some(rest) = line.strip_prefix("func ") {
        let rest = rest.trim_start();
        let after_recv = if rest.starts_with('(') {
            rest.find(')').map(|c| rest[c + 1..].trim_start()).unwrap_or(rest)
        } else {
            rest
        };
        if let Some(name) = first_ident(after_recv) {
            let is_test = name.starts_with("Test");
            return function_or_test(name, count_params(after_recv), is_test);
        }
    }
    LineOutcome::Skip
}

/// JavaScript: `function`/arrow/`class`/`import`/`require`, block-comment-aware.
fn classify_javascript_line(line: &str, state: &mut LexState) -> LineOutcome {
    if state.in_block_comment {
        if line.contains("*/") {
            state.in_block_comment = false;
        }
        return LineOutcome::Skip;
    }
    if line.starts_with("/*") {
        if !line.contains("*/") {
            state.in_block_comment = true;
        }
        return LineOutcome::Skip;
    }
    if line.starts_with("//") || line.starts_with('*') {
        return LineOutcome::Skip;
    }

    if let Some(module) = parse_js_import(line) {
        return LineOutcome::Emit(ObservationKind::ImportStatement { module }, HDAGEdgeKind::Imports);
    }

    // Strip leading `export` / `export default` so exported defs are seen.
    let body = line
        .strip_prefix("export default ")
        .or_else(|| line.strip_prefix("export "))
        .unwrap_or(line);

    if let Some(name) = parse_keyword_name(body, &["class"]) {
        return LineOutcome::Emit(ObservationKind::TypeDefinition { name }, HDAGEdgeKind::Contains);
    }

    // `function name(...)`, `async function name(...)`, `function* name(...)`.
    let fn_body = body
        .strip_prefix("async ")
        .unwrap_or(body)
        .strip_prefix("function")
        .map(|r| r.trim_start_matches('*').trim_start());
    if let Some(rest) = fn_body {
        if let Some(name) = first_ident(rest) {
            return function_or_test(name, count_params(rest), false);
        }
    }

    // Arrow assignment: `const name = (...) =>` / `let` / `var`, possibly async.
    if line.contains("=>") {
        for kw in ["const ", "let ", "var "] {
            if let Some(rest) = body.strip_prefix(kw) {
                if let Some(name) = first_ident(rest) {
                    return function_or_test(name, count_params(rest), false);
                }
            }
        }
    }

    // Test markers: `describe(`, `it(`, `test(` at statement start.
    for marker in ["describe(", "it(", "test("] {
        if line.starts_with(marker) {
            let name = first_quoted(line).unwrap_or_else(|| marker.trim_end_matches('(').to_string());
            return LineOutcome::Emit(ObservationKind::TestDefinition { name }, HDAGEdgeKind::Tests);
        }
    }

    LineOutcome::Skip
}

/// Rust: lexical recognition for fingerprint counting only. The canonical Rust
/// graph is always produced by
/// [`extract_from_rust_source`](crate::code_hdag::CodeHDAG::extract_from_rust_source);
/// this classifier exists so [`CrossLanguageFingerprint`] can tally Rust kinds
/// with the same uniform machinery as the other languages.
fn classify_rust_line(line: &str, state: &mut LexState) -> LineOutcome {
    if line.starts_with("//") {
        return LineOutcome::Skip;
    }
    if is_rust_test_attribute(line) {
        state.pending_test = true;
        return LineOutcome::Skip;
    }
    let was_pending = state.pending_test;
    state.pending_test = false;

    let s = strip_rust_visibility(line);
    if let Some(rest) = s.strip_prefix("use ") {
        let module = rest.trim().trim_end_matches(';').trim();
        if let Some(module) = non_empty(module) {
            return LineOutcome::Emit(ObservationKind::ImportStatement { module }, HDAGEdgeKind::Imports);
        }
    }
    for kw in ["struct ", "enum ", "trait ", "union ", "type "] {
        if let Some(rest) = s.strip_prefix(kw) {
            if let Some(name) = first_ident(rest) {
                return LineOutcome::Emit(ObservationKind::TypeDefinition { name }, HDAGEdgeKind::Contains);
            }
        }
    }
    // `fn`, allowing const/async/unsafe qualifiers between visibility and `fn`.
    let mut fn_scan = s.as_str();
    loop {
        let t = fn_scan.trim_start();
        if let Some(r) = t.strip_prefix("const ") {
            fn_scan = r;
        } else if let Some(r) = t.strip_prefix("async ") {
            fn_scan = r;
        } else if let Some(r) = t.strip_prefix("unsafe ") {
            fn_scan = r;
        } else {
            break;
        }
    }
    if let Some(rest) = fn_scan.trim_start().strip_prefix("fn ") {
        if let Some(name) = first_ident(rest) {
            return function_or_test(name, count_params(rest), was_pending);
        }
    }
    LineOutcome::Skip
}

// ─── Shared lexical helpers ─────────────────────────────────────────────────────

/// Emit a `FunctionDefinition` (Contains) or `TestDefinition` (Tests) edge.
fn function_or_test(name: String, arity: u32, is_test: bool) -> LineOutcome {
    if is_test {
        LineOutcome::Emit(ObservationKind::TestDefinition { name }, HDAGEdgeKind::Tests)
    } else {
        LineOutcome::Emit(ObservationKind::FunctionDefinition { name, arity }, HDAGEdgeKind::Contains)
    }
}

/// Python/Go test-name convention: `test`-prefixed (case-insensitive).
fn is_test_name(name: &str) -> bool {
    name.to_ascii_lowercase().starts_with("test")
}

fn is_rust_test_attribute(line: &str) -> bool {
    line == "#[test]" || (line.starts_with("#[") && line.ends_with("test]") && line.contains("test"))
}

/// Strip a leading Rust visibility qualifier (`pub`, `pub(crate)`, …).
fn strip_rust_visibility(line: &str) -> String {
    let s = line.trim();
    if let Some(rest) = s.strip_prefix("pub") {
        let rest = rest.trim_start();
        if let Some(after_paren) = rest.strip_prefix('(') {
            if let Some(close) = after_paren.find(')') {
                return after_paren[close + 1..].trim_start().to_string();
            }
        }
        return rest.to_string();
    }
    s.to_string()
}

/// Parse `kw <Name>` where the line begins with one of `keywords` followed by a
/// space — returns the first identifier of the name. Handles Go's
/// `type Name struct`/`interface` and JS/Python `class Name`.
fn parse_keyword_name(line: &str, keywords: &[&str]) -> Option<String> {
    for kw in keywords {
        if let Some(rest) = line.strip_prefix(kw) {
            if rest.starts_with(' ') {
                return first_ident(rest);
            }
        }
    }
    None
}

/// Parse a Python import target: `import a.b` / `import a as x` /
/// `from a.b import c` → the dotted module path.
fn parse_python_import(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix("from ") {
        let module = rest.split(" import ").next().unwrap_or("").trim();
        return non_empty(module);
    }
    if let Some(rest) = line.strip_prefix("import ") {
        let first = rest.split(',').next().unwrap_or("").trim();
        let module = first.split(" as ").next().unwrap_or("").trim();
        return non_empty(module);
    }
    None
}

/// Parse a JavaScript import target: `import … from "x"`, `import "x"`,
/// `… require("y")` → the quoted module specifier.
fn parse_js_import(line: &str) -> Option<String> {
    if line.starts_with("import ") || line.starts_with("import{") {
        return first_quoted(line);
    }
    if line.contains("require(") {
        let after = line.split("require(").nth(1).unwrap_or("");
        return first_quoted(after);
    }
    None
}

/// First identifier at the start of `s` (alphanumeric, `_`, or `$`).
fn first_ident(s: &str) -> Option<String> {
    let s = s.trim_start();
    let ident: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
        .collect();
    non_empty(&ident)
}

/// Count comma-separated parameters inside the first `( … )` on the line.
/// `self`/`receiver`-style first parameters are counted — a lexical
/// approximation matching the Rust extractor, not type resolution.
fn count_params(after_name: &str) -> u32 {
    match (after_name.find('('), after_name.find(')')) {
        (Some(o), Some(c)) if c > o => {
            let inner = after_name[o + 1..c].trim();
            if inner.is_empty() {
                0
            } else {
                inner.matches(',').count() as u32 + 1
            }
        }
        _ => 0,
    }
}

/// Extract the first single/double/back-quoted string literal on the line.
fn first_quoted(s: &str) -> Option<String> {
    let mut start = None;
    let mut quote = '"';
    for (i, c) in s.char_indices() {
        if c == '"' || c == '\'' || c == '`' {
            start = Some(i + c.len_utf8());
            quote = c;
            break;
        }
    }
    let start = start?;
    let rest = &s[start..];
    let end = rest.find(quote)?;
    non_empty(&rest[..end])
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

// ─── Cross-language structural fingerprint ─────────────────────────────────────

/// Tally of the four universal structural categories in one source file.
#[derive(Default, Clone, Copy)]
struct KindCounts {
    functions: u64,
    types: u64,
    imports: u64,
    tests: u64,
}

impl KindCounts {
    fn total(&self) -> u64 {
        self.functions + self.types + self.imports + self.tests
    }

    fn observe(&mut self, kind: &ObservationKind) {
        match kind {
            ObservationKind::FunctionDefinition { .. } => self.functions += 1,
            ObservationKind::TypeDefinition { .. } => self.types += 1,
            ObservationKind::ImportStatement { .. } => self.imports += 1,
            ObservationKind::TestDefinition { .. } => self.tests += 1,
            _ => {}
        }
    }
}

/// Run a language's classifier over `content` and tally the structural kinds.
fn count_kinds(language: SourceLanguage, content: &str) -> KindCounts {
    let classify = classifier_for(language);
    let mut state = LexState::default();
    let mut counts = KindCounts::default();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let LineOutcome::Emit(kind, _) = classify(line, &mut state) {
            counts.observe(&kind);
        }
    }
    counts
}

/// Serialize-only content for [`CrossLanguageFingerprint`] addressing.
#[derive(Serialize)]
struct FingerprintContent {
    language: String,
    function_density: i64,
    type_density: i64,
    import_density: i64,
    test_density: i64,
    structural_count: u64,
    source_evidence_id: Digest,
}

/// A content-addressed, `Q16`-only vector of language-independent structural
/// ratios for a single source file.
///
/// This is the cross-language comparability layer the hypercube needs: two
/// programs that do the same thing in different languages — say `fibonacci.py`
/// and `fibonacci.rs` — produce close fingerprints, because the *proportion* of
/// functions, types, imports, and tests is a property of the algorithm, not the
/// syntax. The PSE-Codex corpus is the empirical evidence for this; here it is
/// captured in integer arithmetic so it can sit inside a gate path (CROSS-007).
///
/// The four densities are taken over the total count of those four categories,
/// so they form a structural simplex (they sum to `1.0` whenever any structure
/// is present) rather than raw counts that scale with file size.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossLanguageFingerprint {
    pub fingerprint_id: Digest,
    pub language: SourceLanguage,
    /// `function_nodes / structural_count`.
    pub function_density: Q16,
    /// `type_nodes / structural_count`.
    pub type_density: Q16,
    /// `import_nodes / structural_count`.
    pub import_density: Q16,
    /// `test_nodes / structural_count`.
    pub test_density: Q16,
    /// Total number of structural observations (functions + types + imports + tests).
    pub structural_count: u64,
    pub source_evidence_id: Digest,
}

impl CrossLanguageFingerprint {
    /// Extract the fingerprint of `content` in a known `language`.
    pub fn from_source(
        language: SourceLanguage,
        source_evidence_id: Digest,
        content: &str,
    ) -> Self {
        Self::from_counts(language, count_kinds(language, content), source_evidence_id)
    }

    /// Detect the language from `location` and fingerprint, or `None` for an
    /// unrecognised extension (fail-closed). The host-scan integration entry point.
    pub fn from_auto(source_evidence_id: Digest, location: &str, content: &str) -> Option<Self> {
        SourceLanguage::from_path(location)
            .map(|lang| Self::from_source(lang, source_evidence_id, content))
    }

    fn from_counts(language: SourceLanguage, counts: KindCounts, source_evidence_id: Digest) -> Self {
        let total = counts.total();
        let density = |count: u64| -> Q16 {
            if total == 0 {
                Q16::ZERO
            } else {
                Q16::ratio(count, total).unwrap_or(Q16::ZERO)
            }
        };
        let function_density = density(counts.functions);
        let type_density = density(counts.types);
        let import_density = density(counts.imports);
        let test_density = density(counts.tests);

        let fingerprint_id = Self::compute_id(
            language,
            function_density,
            type_density,
            import_density,
            test_density,
            total,
            source_evidence_id,
        );

        Self {
            fingerprint_id,
            language,
            function_density,
            type_density,
            import_density,
            test_density,
            structural_count: total,
            source_evidence_id,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_id(
        language: SourceLanguage,
        function_density: Q16,
        type_density: Q16,
        import_density: Q16,
        test_density: Q16,
        structural_count: u64,
        source_evidence_id: Digest,
    ) -> Digest {
        Digest::of(&FingerprintContent {
            language: language.as_str().to_string(),
            function_density: function_density.raw(),
            type_density: type_density.raw(),
            import_density: import_density.raw(),
            test_density: test_density.raw(),
            structural_count,
            source_evidence_id,
        })
    }

    /// Verify the stored `fingerprint_id` matches the content.
    pub fn verify_id(&self) -> bool {
        self.fingerprint_id
            == Self::compute_id(
                self.language,
                self.function_density,
                self.type_density,
                self.import_density,
                self.test_density,
                self.structural_count,
                self.source_evidence_id,
            )
    }

    /// Language-independent structural similarity in `[0, 1]`, `Q16` integer
    /// arithmetic only (CROSS-007).
    ///
    /// `1 − mean(|Δdensity|)` over the four density axes — the fixed-point
    /// counterpart of the PSE-Codex structural-ratios similarity. `1.0` means
    /// the two profiles are structurally identical; a self-comparison is always
    /// exactly `Q16::ONE`. Symmetric by construction.
    pub fn similarity(&self, other: &Self) -> Q16 {
        let axes = [
            (self.function_density, other.function_density),
            (self.type_density, other.type_density),
            (self.import_density, other.import_density),
            (self.test_density, other.test_density),
        ];
        let mut sum_diff_raw: i64 = 0;
        for (a, b) in axes {
            sum_diff_raw = sum_diff_raw.saturating_add(a.saturating_sub(b).abs().raw());
        }
        let mean_diff = Q16::from_raw(sum_diff_raw / axes.len() as i64);
        Q16::ONE.saturating_sub(mean_diff).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{Digest, EnergyFactors, Q16, TaintLabel};

    // ── Real corpus fixtures (trimmed PSE-Codex "barbara" corpus) ──────────────

    const PY_FIB: &str = r#"
"""Fibonacci implementations."""
from typing import Dict, List, Optional
from functools import lru_cache

def fibonacci_recursive(n: int, memo: Optional[Dict[int, int]] = None) -> int:
    if n < 0:
        raise ValueError("negative")
    if n <= 1:
        return n
    return fibonacci_recursive(n - 1, memo) + fibonacci_recursive(n - 2, memo)

def fibonacci_iterative(n: int) -> int:
    if n <= 1:
        return n
    prev, curr = 0, 1
    for _ in range(2, n + 1):
        prev, curr = curr, prev + curr
    return curr

@lru_cache(maxsize=256)
def fibonacci_cached(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci_cached(n - 1) + fibonacci_cached(n - 2)

def fibonacci_sequence(count: int) -> List[int]:
    return [fibonacci_iterative(i) for i in range(count)]
"#;

    const GO_FIB: &str = r#"
package main

import "fmt"

func fibMemo(n int) int64 {
	if n <= 0 {
		return 0
	}
	return fibMemo(n-1) + fibMemo(n-2)
}

func fibIterative(n int) int64 {
	var prev, curr int64 = 0, 1
	for i := 2; i <= n; i++ {
		prev, curr = curr, prev+curr
	}
	return curr
}

func fibSequence(n int) []int64 {
	seq := make([]int64, n)
	return seq
}

func isFibonacci(n int64) bool {
	return isPerfectSquare(5*n*n + 4)
}

func main() {
	fmt.Println(fibMemo(10))
}
"#;

    const JS_FIB: &str = r#"
/**
 * Fibonacci implementations.
 * @module fibonacci
 */
function fibonacciMemo(n, memo = new Map()) {
  if (n <= 1) {
    return BigInt(n);
  }
  return fibonacciMemo(n - 1, memo) + fibonacciMemo(n - 2, memo);
}

function fibonacciIterative(n) {
  let prev = 0n;
  let curr = 1n;
  for (let i = 2; i <= n; i++) {
    curr = prev + curr;
  }
  return curr;
}

function fibonacciSequence(count) {
  const seq = [0n];
  return seq;
}

const isFibonacci = (n) => {
  return n >= 0;
};

module.exports = { fibonacciMemo, fibonacciIterative };
"#;

    const RS_FIB: &str = r#"
/// Fibonacci implementations.
use std::collections::HashMap;

fn fibonacci_memo(n: u64, cache: &mut HashMap<u64, u64>) -> u64 {
    if n <= 1 {
        return n;
    }
    fibonacci_memo(n - 1, cache) + fibonacci_memo(n - 2, cache)
}

fn fibonacci_iterative(n: u64) -> u64 {
    let mut prev = 0u64;
    let mut curr = 1u64;
    curr
}

fn fibonacci_sequence(count: usize) -> Vec<u64> {
    Vec::new()
}

fn is_fibonacci(value: u64) -> bool {
    true
}
"#;

    fn ev() -> Digest {
        Digest::of_bytes(b"ev")
    }

    // ── Language detection ─────────────────────────────────────────────────────

    #[test]
    fn detects_language_from_extension() {
        assert_eq!(SourceLanguage::from_path("src/a.rs"), Some(SourceLanguage::Rust));
        assert_eq!(SourceLanguage::from_path("a/b/c.py"), Some(SourceLanguage::Python));
        assert_eq!(SourceLanguage::from_path("x.mjs"), Some(SourceLanguage::JavaScript));
        assert_eq!(SourceLanguage::from_path("main.go"), Some(SourceLanguage::Go));
        assert_eq!(SourceLanguage::from_path("README.md"), None);
        assert_eq!(SourceLanguage::from_path("Makefile"), None);
    }

    #[test]
    fn detection_is_case_insensitive() {
        assert_eq!(SourceLanguage::from_path("A.PY"), Some(SourceLanguage::Python));
        assert_eq!(SourceLanguage::from_path("B.Go"), Some(SourceLanguage::Go));
    }

    #[test]
    fn extract_auto_is_fail_closed_for_unknown_extension() {
        assert!(CodeHDAG::extract_auto(ev(), "data.csv", "a,b,c", TaintLabel::Clean).is_none());
    }

    #[test]
    fn is_test_path_per_language() {
        assert!(is_test_path(SourceLanguage::Python, "test_fib.py"));
        assert!(is_test_path(SourceLanguage::Python, "pkg/fib_test.py"));
        assert!(!is_test_path(SourceLanguage::Python, "fib.py"));
        assert!(is_test_path(SourceLanguage::Go, "fib_test.go"));
        assert!(!is_test_path(SourceLanguage::Go, "fib.go"));
        assert!(is_test_path(SourceLanguage::JavaScript, "fib.test.js"));
        assert!(is_test_path(SourceLanguage::JavaScript, "__tests__/fib.js"));
    }

    // ── Python extraction ──────────────────────────────────────────────────────

    #[test]
    fn python_captures_imports_and_functions() {
        let h = CodeHDAG::extract_from_source(SourceLanguage::Python, ev(), "fib.py", PY_FIB, TaintLabel::Clean);
        assert_eq!(h.edges_of_kind(&HDAGEdgeKind::Imports), 2, "two `from … import` lines");
        assert!(h.edges_of_kind(&HDAGEdgeKind::Contains) >= 4, "at least four defs");
    }

    #[test]
    fn python_skips_docstrings_and_decorators() {
        let src = "\"\"\"\nimport not_a_real_import\ndef not_a_real_def():\n\"\"\"\n@decorator\ndef real(x):\n    pass\n";
        let h = CodeHDAG::extract_from_source(SourceLanguage::Python, ev(), "x.py", src, TaintLabel::Clean);
        assert_eq!(h.edges_of_kind(&HDAGEdgeKind::Imports), 0, "docstring import ignored");
        assert_eq!(h.definition_count(), 1, "only the real def, got {}", h.definition_count());
    }

    // ── Go extraction ──────────────────────────────────────────────────────────

    #[test]
    fn go_captures_single_and_block_imports() {
        let block = "package main\nimport (\n\t\"fmt\"\n\t\"os\"\n)\nfunc main() {}\n";
        let h = CodeHDAG::extract_from_source(SourceLanguage::Go, ev(), "m.go", block, TaintLabel::Clean);
        assert_eq!(h.edges_of_kind(&HDAGEdgeKind::Imports), 2, "two block imports");
        assert!(h.edges_of_kind(&HDAGEdgeKind::Contains) >= 1, "func main");
    }

    #[test]
    fn go_captures_types_and_methods() {
        let src = "type Node struct {\n\tval int\n}\nfunc (n Node) Value() int {\n\treturn n.val\n}\n";
        let h = CodeHDAG::extract_from_source(SourceLanguage::Go, ev(), "n.go", src, TaintLabel::Clean);
        assert!(h.definition_count() >= 2, "type + method, got {}", h.definition_count());
    }

    #[test]
    fn go_detects_test_functions() {
        let src = "func TestThing(t *testing.T) {\n}\nfunc Helper() {}\n";
        let h = CodeHDAG::extract_from_source(SourceLanguage::Go, ev(), "n_test.go", src, TaintLabel::Clean);
        assert_eq!(h.edges_of_kind(&HDAGEdgeKind::Tests), 1, "one Test function");
    }

    // ── JavaScript extraction ──────────────────────────────────────────────────

    #[test]
    fn javascript_captures_functions_and_arrows() {
        let h = CodeHDAG::extract_from_source(SourceLanguage::JavaScript, ev(), "fib.js", JS_FIB, TaintLabel::Clean);
        assert!(
            h.edges_of_kind(&HDAGEdgeKind::Contains) >= 4,
            "three function decls + one arrow, got {}",
            h.edges_of_kind(&HDAGEdgeKind::Contains)
        );
    }

    #[test]
    fn javascript_skips_jsdoc_block_comments() {
        let src = "/**\n * function ghost() {}\n * @param x\n */\nfunction real() {}\n";
        let h = CodeHDAG::extract_from_source(SourceLanguage::JavaScript, ev(), "x.js", src, TaintLabel::Clean);
        assert_eq!(h.definition_count(), 1, "block-comment function ignored, got {}", h.definition_count());
    }

    #[test]
    fn javascript_captures_imports() {
        let src = "import { foo } from \"./mod.js\";\nconst bar = require(\"baz\");\n";
        let h = CodeHDAG::extract_from_source(SourceLanguage::JavaScript, ev(), "x.js", src, TaintLabel::Clean);
        assert_eq!(h.edges_of_kind(&HDAGEdgeKind::Imports), 2, "es import + require");
    }

    // ── Invariants: determinism, taint, content-addressing ─────────────────────

    #[test]
    fn extraction_is_deterministic() {
        let a = CodeHDAG::extract_from_source(SourceLanguage::Python, ev(), "fib.py", PY_FIB, TaintLabel::Clean);
        let b = CodeHDAG::extract_from_source(SourceLanguage::Python, ev(), "fib.py", PY_FIB, TaintLabel::Clean);
        assert_eq!(a.hdag_id, b.hdag_id, "identical source → identical hdag_id");
    }

    #[test]
    fn taint_propagates_to_graph() {
        let h = CodeHDAG::extract_from_source(SourceLanguage::Go, ev(), "m.go", GO_FIB, TaintLabel::External);
        assert_eq!(h.taint, TaintLabel::External);
    }

    #[test]
    fn rust_path_matches_canonical_extractor() {
        // extract_from_source(Rust, …) must be byte-identical to the existing path.
        let a = CodeHDAG::extract_from_source(SourceLanguage::Rust, ev(), "f.rs", RS_FIB, TaintLabel::Clean);
        let b = CodeHDAG::extract_from_rust_source(ev(), "f.rs", RS_FIB, TaintLabel::Clean);
        assert_eq!(a.hdag_id, b.hdag_id, "Rust dispatch must equal the canonical extractor");
    }

    // ── Topology → energy bridge works for all languages ───────────────────────

    #[test]
    fn cross_language_topology_feeds_energy_kernel() {
        // A rich Python module must out-rank an empty one under the same ψ — the
        // exact property the Rust path guarantees, now language-independent.
        let rich = CodeHDAG::extract_from_source(SourceLanguage::Python, ev(), "fib.py", PY_FIB, TaintLabel::Clean);
        let empty = CodeHDAG::extract_from_source(SourceLanguage::Python, ev(), "e.py", "# nothing\n", TaintLabel::Clean);
        let psi = Q16::ONE;
        let rich_k = rich.energy_kernel(psi, EnergyFactors::all_clean());
        let empty_k = empty.energy_kernel(psi, EnergyFactors::all_clean());
        assert!(rich_k.energy().raw() > empty_k.energy().raw(), "rich module out-ranks empty");
        assert_eq!(empty_k.energy(), Q16::ZERO, "empty topology → zero energy");
    }

    // ── Cross-language fingerprint ─────────────────────────────────────────────

    #[test]
    fn fingerprint_is_content_addressed() {
        let f = CrossLanguageFingerprint::from_source(SourceLanguage::Python, ev(), PY_FIB);
        assert!(f.verify_id());
        assert_ne!(f.fingerprint_id, Digest::ZERO);
    }

    #[test]
    fn fingerprint_self_similarity_is_one() {
        let f = CrossLanguageFingerprint::from_source(SourceLanguage::Go, ev(), GO_FIB);
        assert_eq!(f.similarity(&f), Q16::ONE, "self-similarity must be exactly 1.0");
    }

    #[test]
    fn fingerprint_similarity_is_symmetric() {
        let py = CrossLanguageFingerprint::from_source(SourceLanguage::Python, ev(), PY_FIB);
        let rs = CrossLanguageFingerprint::from_source(SourceLanguage::Rust, ev(), RS_FIB);
        assert_eq!(py.similarity(&rs), rs.similarity(&py), "similarity must be symmetric");
    }

    #[test]
    fn fingerprint_cross_language_same_algorithm_is_high() {
        // The central cross-language claim: the same algorithm in four languages
        // produces structurally close fingerprints. Threshold is conservative.
        let py = CrossLanguageFingerprint::from_source(SourceLanguage::Python, ev(), PY_FIB);
        let go = CrossLanguageFingerprint::from_source(SourceLanguage::Go, ev(), GO_FIB);
        let js = CrossLanguageFingerprint::from_source(SourceLanguage::JavaScript, ev(), JS_FIB);
        let rs = CrossLanguageFingerprint::from_source(SourceLanguage::Rust, ev(), RS_FIB);
        let threshold = Q16::ratio(60, 100).unwrap();
        for (a, b, label) in [
            (&py, &rs, "py~rs"),
            (&py, &go, "py~go"),
            (&py, &js, "py~js"),
            (&go, &rs, "go~rs"),
        ] {
            let sim = a.similarity(b);
            assert!(sim.at_least(threshold), "{label} similarity {sim} below 0.60");
        }
    }

    #[test]
    fn fingerprint_empty_file_is_neutral() {
        let f = CrossLanguageFingerprint::from_source(SourceLanguage::Python, ev(), "# nothing\n");
        assert_eq!(f.structural_count, 0);
        assert_eq!(f.function_density, Q16::ZERO);
        assert!(f.verify_id());
    }
}
