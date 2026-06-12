//! Kosmocrates intent layer — connect the wish ruler to the real workspace.
//!
//! Runs 1 and 2 built the *target* ([`kosmo_core::Wish`]), the *ruler*
//! ([`kosmo_core::assess_wish`]), and the *convergence contract*
//! ([`kosmo_core::WishConvergenceTrace`]) — all pure, all measured against a
//! hand-supplied [`ObservedTopology`]. This crate fills the last gap: it reads a
//! **real** workspace and turns it into that observation, and it ties the three
//! pieces together in a stateful [`WishSession`] that tracks convergence — and,
//! fail-closed, divergence — across successive iterations.
//!
//! Observation granularity: this layer observes the workspace's **crate
//! topology** via [`kosmo_parseback`] (crate presence is content-addressed and
//! name-preserving). Finer-grained `Module` / `Symbol` facets need a
//! name-preserving source extractor and are a later run; the observation API is
//! a facet *set*, so those sources merge in without an interface change.
//!
//! Like everything on the selection path, this **ranks, it never gates**: it
//! measures how far a workspace is from a wish and whether it is moving toward
//! the attractor, but grants no capability and bypasses no policy.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kosmo_core::{
    assess_wish, Digest, ObservedTopology, ParseBackScanScope, Wish, WishAssessment,
    WishConvergenceTrace, WishFacet, WishFacetKind, WishPredicate,
};
use kosmo_hyphae::xlang::{symbol_sets, SourceLanguage};
use kosmo_hyphae::Norm;
use kosmo_parseback::{ParseBackError, ParseBackExecutor, TopologySnapshot};
use serde::{Deserialize, Serialize};

pub mod atelier;
pub mod chat;
pub mod venture_spec;
pub use atelier::{
    companion_suggestions, parse_atelier_command, AtelierCommand, DraftSlot, FacetSuggestion,
    IndexSelection, SuggestionSource, WishDraft,
};
pub use chat::{route_keywords, ChatIntent, IntentExtractor, KeywordIntentExtractor};
pub use venture_spec::{compile_venture, VentureSpec, VentureStageSpec};

// ─── Observation adapter ────────────────────────────────────────────────────

/// Build the present-facet set from a crate topology snapshot.
///
/// Emits one [`WishFacet::crate_`] per crate node. (Dependency edges and
/// per-crate file fingerprints are not surfaced as facets here — a wish targets
/// *presence*, and `Module` / `Symbol` facets need a name-preserving source
/// extractor, which is a later run.)
pub fn facets_from_snapshot(snapshot: &TopologySnapshot) -> BTreeSet<WishFacet> {
    let mut facets: BTreeSet<WishFacet> = snapshot
        .crate_nodes
        .keys()
        .map(|name| WishFacet::crate_(name.clone()))
        .collect();
    // Directed dependency edges (`from->to`) — lets a wish target the dependency
    // structure, not just crate presence.
    for (from, to) in &snapshot.dep_edges {
        facets.insert(WishFacet::dependency(from, to));
    }
    facets
}

/// Build an [`ObservedTopology`] from a crate topology snapshot.
pub fn observe_snapshot(snapshot: &TopologySnapshot) -> ObservedTopology {
    ObservedTopology::from_facets(facets_from_snapshot(snapshot))
}

/// Scan a real workspace and observe its crate topology.
///
/// Read-only: runs `cargo metadata` once via [`ParseBackExecutor`], no host
/// mutations. Uses [`ParseBackScanScope::AffectedFilesOnly`] because the
/// observed facet set is the crate *names*, which are scope-independent — this
/// just avoids walking every source file.
pub fn observe_workspace(root: impl Into<PathBuf>) -> Result<ObservedTopology, ParseBackError> {
    observe_workspace_scoped(root, ParseBackScanScope::AffectedFilesOnly)
}

/// As [`observe_workspace`], with an explicit scan scope.
pub fn observe_workspace_scoped(
    root: impl Into<PathBuf>,
    scope: ParseBackScanScope,
) -> Result<ObservedTopology, ParseBackError> {
    let snapshot = ParseBackExecutor::new(root.into()).snapshot(&scope)?;
    Ok(observe_snapshot(&snapshot))
}

// ─── Source extraction (Module / Symbol facets) ─────────────────────────────

/// Strip a leading `pub` / `pub(...)` visibility, returning the rest trimmed.
fn strip_vis(line: &str) -> &str {
    let l = line.trim_start();
    if let Some(rest) = l.strip_prefix("pub") {
        if let Some(after) = rest.trim_start().strip_prefix('(') {
            if let Some(close) = after.find(')') {
                return after[close + 1..].trim_start();
            }
        }
        if rest.starts_with(char::is_whitespace) {
            return rest.trim_start();
        }
    }
    l
}

/// The leading Rust identifier in a token (letters, digits, `_`; not digit-led).
fn leading_ident(tok: &str) -> Option<String> {
    let end = tok
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(tok.len());
    if end == 0 {
        return None;
    }
    let id = &tok[..end];
    if id.chars().next()?.is_ascii_digit() {
        return None;
    }
    Some(id.to_string())
}

/// Count a function's arity from its opening line — the number of top-level
/// arguments between its parentheses (generics/arrays don't add args).
fn fn_arity(line: &str) -> usize {
    let Some(open) = line.find('(') else {
        return 0;
    };
    let mut depth = 0i32;
    let mut args = 0usize;
    let mut seen = false;
    for c in line[open..].chars() {
        match c {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            ',' if depth == 1 => args += 1,
            c if depth == 1 && !c.is_whitespace() => seen = true,
            _ => {}
        }
    }
    if seen {
        args + 1
    } else {
        0
    }
}

/// Collapse internal whitespace runs to single spaces and trim — the canonical
/// form for a captured type, shared with the contract scaffold so that
/// scaffold → observe round-trips.
fn normalize_type(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Split a string on top-level commas, ignoring commas nested inside `()`,
/// `<>`, or `[]`. `->` is treated as an atom so closure-return arrows don't
/// unbalance the depth counter.
fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '-' if chars.peek() == Some(&'>') => {
                cur.push('-');
                cur.push(chars.next().unwrap()); // the '>'
            }
            '(' | '<' | '[' => {
                depth += 1;
                cur.push(c);
            }
            ')' | '>' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// The data type of one parameter segment (`"x: T"` → `"T"`). Bare receivers
/// (`self` / `&self` / `&mut self`) and untyped segments carry no data type and
/// yield `None`.
fn param_type(seg: &str) -> Option<String> {
    let seg = seg.trim();
    let compact: String = seg.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() || compact == "self" || compact == "&self" || compact == "&mutself" {
        return None;
    }
    let colon = seg.find(':')?;
    let ty = seg[colon + 1..].trim();
    (!ty.is_empty()).then(|| normalize_type(ty))
}

/// The return type from the text *after* a function's parameter list — the
/// token(s) between `->` and the body `{` (or a `where` clause). `"()"` if the
/// function declares no return type.
fn parse_return_type(rest: &str) -> String {
    let Some(arrow) = rest.find("->") else {
        return "()".to_string();
    };
    let after = &rest[arrow + 2..];
    let end = after.find('{').unwrap_or(after.len());
    let mut seg = &after[..end];
    if let Some(w) = seg.find(" where ") {
        seg = &seg[..w];
    }
    let norm = normalize_type(seg);
    if norm.is_empty() {
        "()".to_string()
    } else {
        norm
    }
}

/// The parameter *types* and return type of a function from its opening line.
/// Mirrors the `kosmo-synthesizer` contract scaffold, so scaffold → observe
/// round-trips. Shallow types only (single tokens / simple generics).
fn parse_fn_types(line: &str) -> (Vec<String>, String) {
    let chars: Vec<char> = line.chars().collect();
    let Some(open) = chars.iter().position(|&c| c == '(') else {
        return (vec![], "()".to_string());
    };
    let mut depth = 0i32;
    let mut close = chars.len();
    for (i, &c) in chars.iter().enumerate().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let params_str: String = chars[open + 1..close.min(chars.len())].iter().collect();
    let params = split_top_level_commas(&params_str)
        .iter()
        .filter_map(|seg| param_type(seg))
        .collect();
    let rest: String = if close < chars.len() {
        chars[close + 1..].iter().collect()
    } else {
        String::new()
    };
    (params, parse_return_type(&rest))
}

/// The structural facets a public function declares: its `Symbol`, its untyped
/// `Signature` (`name/arity`), and its typed `Contract` (`name(T0,T1)->R`).
/// The `Contract` is additive — a `Signature` wish still works unchanged.
fn fn_facets(name: &str, line: &str) -> Vec<WishFacet> {
    let (params, ret) = parse_fn_types(line);
    let refs: Vec<&str> = params.iter().map(String::as_str).collect();
    vec![
        WishFacet::symbol(name.to_string()),
        WishFacet::signature(format!("{name}/{}", fn_arity(line))),
        WishFacet::contract(name, &refs, ret),
    ]
}

/// Extract the [`WishFacet`]s a single source line declares: a `Module` (`mod`,
/// public or not) or — for a **public** definition (`fn` / `struct` / `enum` /
/// `trait` / `type` / `union` / `const` / `static`) — a `Symbol`, plus a
/// `Signature` (`"name/arity"`) when it is a function.
///
/// Deterministic and lexical — it reads the opening line only, like
/// `kosmo-hyphae`'s code HDAG extractor. Symbols are keyed by their bare name;
/// `extern` items and macro-generated definitions are not captured.
fn item_facets(line: &str) -> Vec<WishFacet> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return vec![];
    }
    if trimmed.starts_with("//") {
        // Capability marker: `// kosmo:capability: <name>` (also `//!`). This is
        // how the otherwise-invisible `Capability` facet becomes observable.
        let body = trimmed
            .trim_start_matches('/')
            .trim_start_matches('!')
            .trim_start();
        if let Some(cap) = body.strip_prefix("kosmo:capability:") {
            let name = cap.trim();
            if !name.is_empty() {
                return vec![WishFacet::capability(name)];
            }
        }
        return vec![];
    }
    let is_pub = trimmed.starts_with("pub ") || trimmed.starts_with("pub(");
    let body = strip_vis(trimmed);
    let mut tokens = body.split_whitespace().peekable();

    // Unambiguous fn modifiers.
    while matches!(
        tokens.peek(),
        Some(&"async") | Some(&"unsafe") | Some(&"default")
    ) {
        tokens.next();
    }
    // `const` / `static` are item keywords, unless `const fn`.
    if matches!(tokens.peek(), Some(&"const") | Some(&"static")) {
        tokens.next();
        let is_fn = tokens.peek() == Some(&"fn");
        if is_fn {
            tokens.next();
        }
        if !is_pub {
            return vec![];
        }
        let Some(name) = tokens.next().and_then(leading_ident) else {
            return vec![];
        };
        if is_fn {
            return fn_facets(&name, line);
        }
        return vec![WishFacet::symbol(name)];
    }

    let Some(kw) = tokens.peek().copied() else {
        return vec![];
    };
    tokens.next();
    let Some(name) = tokens.next().and_then(leading_ident) else {
        return vec![];
    };
    match kw {
        "mod" => vec![WishFacet::module(name)],
        "fn" if is_pub => fn_facets(&name, line),
        "struct" | "enum" | "trait" | "type" | "union" if is_pub => vec![WishFacet::symbol(name)],
        _ => vec![],
    }
}

/// Extract `Module` and `Symbol` facets from Rust source text.
///
/// Pure and deterministic. Captures module declarations and the **public**
/// definition surface (fns, types, consts) by bare name.
pub fn facets_from_source(source: &str) -> BTreeSet<WishFacet> {
    let mut facets = BTreeSet::new();
    let mut pending_test = false;
    let mut pending_doc = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if is_test_attr(trimmed) {
            pending_test = true;
        }
        if pending_test {
            if let Some(name) = test_fn_name(trimmed) {
                facets.insert(WishFacet::test(name));
                pending_test = false;
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                pending_test = false; // not a test fn after all
            }
        }
        let item = item_facets(line);
        // A documented public item or module declaration: a doc comment
        // (`///` / `#[doc…]`) immediately above the definition (attribute
        // lines in between are fine — docs lexically precede attributes).
        // Mirrors the substrate's MissingDocFiber finding as a *measurable*
        // facet. Modules are keyed by their `mod` declaration — the same
        // place the Module facet itself is observed.
        if is_doc_line(trimmed) {
            pending_doc = true;
        } else if pending_doc {
            let symbols: Vec<String> = item
                .iter()
                .filter(|f| f.kind == WishFacetKind::Symbol || f.kind == WishFacetKind::Module)
                .map(|f| f.key.clone())
                .collect();
            if !symbols.is_empty() {
                for name in symbols {
                    facets.insert(WishFacet::doc(name));
                }
                pending_doc = false;
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                pending_doc = false; // doc block ended on a non-item line
            }
        }
        facets.extend(item);
    }
    facets
}

/// A doc-comment line: `///`, `//!`, or a `#[doc = …]` attribute.
fn is_doc_line(trimmed: &str) -> bool {
    trimmed.starts_with("///") || trimmed.starts_with("//!") || trimmed.starts_with("#[doc")
}

/// A test attribute line: `#[test]`, `#[tokio::test]`, `#[async_std::test]`, …
fn is_test_attr(trimmed: &str) -> bool {
    trimmed.starts_with("#[") && trimmed.contains("test]")
}

/// The function name on a `fn NAME(...)` line, any visibility, if present.
fn test_fn_name(trimmed: &str) -> Option<String> {
    let idx = trimmed.find("fn ")?;
    leading_ident(trimmed[idx + 3..].trim_start())
}

/// Parse a `// kosmo:behavior: <spec>` marker line into its spec string.
/// (Also accepts `//!`.) The spec is the `Behavior` facet key verbatim.
fn behavior_marker(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("//") {
        return None;
    }
    let body = trimmed
        .trim_start_matches('/')
        .trim_start_matches('!')
        .trim_start();
    body.strip_prefix("kosmo:behavior:")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Pair each scaffolded spec-test with its behaviour spec, read from
/// `// kosmo:behavior: <spec>` markers that immediately precede a `#[test]`
/// function. Returns `(test_fn_name, spec)` pairs. Pure and deterministic —
/// the lexical half of the keystone; the *passing* half is applied later, by
/// [`behavior_facets`], once the suite has run.
fn behavior_specs_from_source(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut pending_spec: Option<String> = None;
    let mut pending_test = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(spec) = behavior_marker(trimmed) {
            pending_spec = Some(spec);
            continue;
        }
        if is_test_attr(trimmed) {
            pending_test = true;
            continue;
        }
        if let Some(name) = test_fn_name(trimmed) {
            if pending_test {
                if let Some(spec) = pending_spec.take() {
                    out.push((name, spec));
                }
            }
            pending_test = false;
            pending_spec = None;
        } else if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("//") {
            // A non-attr, non-comment, non-empty line breaks the pairing.
            pending_test = false;
            pending_spec = None;
        }
    }
    out
}

/// Behaviour facets for exactly the specs whose spec-test **passed**.
/// Fail-closed: a spec whose test is absent or failing yields no facet — a
/// wished behaviour is never satisfied by an unrun or red test.
pub fn behavior_facets(
    specs: &[(String, String)],
    passing_bare_names: &BTreeSet<String>,
) -> BTreeSet<WishFacet> {
    specs
        .iter()
        .filter(|(name, _)| passing_bare_names.contains(name))
        .map(|(_, spec)| WishFacet::behavior(spec))
        .collect()
}

/// Recursively collect `.rs` file paths under `dir`, skipping `target` / `.git`.
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" {
                continue;
            }
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Read `name = "…"` from a manifest's `[package]` section (the crate's
/// package name). `None` for a `[workspace]`-only manifest.
fn manifest_package_name(toml: &str) -> Option<String> {
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

/// The package name of the crate a source `file` belongs to: the nearest
/// ancestor (up to and including `root`) with a `[package]` `Cargo.toml`.
fn crate_of(file: &Path, root: &Path) -> Option<String> {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if let Ok(content) = std::fs::read_to_string(d.join("Cargo.toml")) {
            if let Some(name) = manifest_package_name(&content) {
                return Some(name);
            }
        }
        if d == root {
            break;
        }
        dir = d.parent();
    }
    None
}

/// Parse a contract facet key `"name(T0,T1)->R"` into `(name, params, ret)` —
/// the dual of the synthesizer's contract-key parser, used to derive
/// compositions from observed contracts. `->` is treated as an atom.
fn parse_contract_facet_key(key: &str) -> Option<(String, Vec<String>, String)> {
    let chars: Vec<char> = key.chars().collect();
    let open = chars.iter().position(|&c| c == '(')?;
    let name = chars[..open].iter().collect::<String>().trim().to_string();
    if name.is_empty() {
        return None;
    }
    let mut depth = 0i32;
    let mut close = None;
    let mut prev = ' ';
    for (i, &c) in chars.iter().enumerate().skip(open) {
        let arrow_gt = c == '>' && prev == '-';
        if !arrow_gt {
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
    let params = split_top_level_commas(&params_str)
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let rest: String = chars[close + 1..].iter().collect();
    let ret = rest
        .trim_start()
        .strip_prefix("->")
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| "()".to_string());
    Some((name, params, ret))
}

/// Derive typed data-flow [`WishFacetKind::Composition`] facets from observed
/// contracts: for ordered pairs `(f, g)` where `f`'s return type equals `g`'s
/// first parameter type (and is not unit), emit `f>>T>>g`. Pure; reads only the
/// bare (non-crate-qualified) contracts so a composition is workspace-level.
fn derive_compositions(facets: &BTreeSet<WishFacet>) -> BTreeSet<WishFacet> {
    let mut ret_of: BTreeMap<String, String> = BTreeMap::new();
    let mut in0_of: BTreeMap<String, String> = BTreeMap::new();
    for f in facets {
        if f.kind != WishFacetKind::Contract || f.key.contains('@') {
            continue;
        }
        if let Some((name, params, ret)) = parse_contract_facet_key(&f.key) {
            if let Some(p0) = params.into_iter().next() {
                in0_of.insert(name.clone(), p0);
            }
            ret_of.insert(name, ret);
        }
    }
    let mut out = BTreeSet::new();
    for (f, rf) in &ret_of {
        if rf == "()" {
            continue;
        }
        for (g, ig) in &in0_of {
            if f != g && rf == ig {
                out.insert(WishFacet::composition(f, rf, g));
            }
        }
    }
    out
}

/// Walk every `.rs` file under `dir` and union their source facets.
///
/// Read-only. Heavier than the crate-level snapshot (it reads source), so it is
/// opt-in via [`observe_workspace_deep`]. Each item is emitted **twice**: by its
/// bare key, and — when the file belongs to a package — by its crate-qualified
/// key `"<key>@<crate>"`, so a crate-targeted wish (e.g. `handle@kosmo-api`)
/// round-trips against the crate the scaffolder wrote it into. Finally, typed
/// data-flow `Composition` facets are derived from the observed contracts.
pub fn facets_from_rust_dir(dir: impl AsRef<Path>) -> BTreeSet<WishFacet> {
    let root = dir.as_ref();
    let mut files = Vec::new();
    collect_rs(root, &mut files);
    files.sort();
    let mut facets = BTreeSet::new();
    for file in &files {
        if let Ok(content) = std::fs::read_to_string(file) {
            let crate_name = crate_of(file, root);
            for f in facets_from_source(&content) {
                if let Some(ref cr) = crate_name {
                    facets.insert(WishFacet::new(f.kind.clone(), format!("{}@{}", f.key, cr)));
                }
                facets.insert(f);
            }
        }
    }
    facets.extend(derive_compositions(&facets));
    facets
}

// ─── Python observation (polyglot parity) ───────────────────────────────────

/// Python's two docstring openers.
const PY_DOC_OPENERS: [&str; 2] = ["\"\"\"", "'''"];

/// Recursively collect `.py` file paths under `dir`, skipping vendor/VCS and
/// virtualenv directories.
fn collect_py(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(
                name,
                "target" | ".git" | "__pycache__" | ".venv" | "venv" | "node_modules"
            ) {
                continue;
            }
            collect_py(&path, out);
        } else if path.extension().is_some_and(|e| e == "py") {
            out.push(path);
        }
    }
}

/// The module name of a Python file: its stem — Python's own law, file =
/// module — with `__init__.py` standing for its package directory.
fn python_module_name(file: &Path) -> Option<String> {
    let stem = file.file_stem()?.to_str()?;
    if stem == "__init__" {
        file.parent()?
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from)
    } else {
        Some(stem.to_string())
    }
}

/// The name introduced by a top-level `def`/`class` line, if any.
fn python_item_name(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("def ")
        .or_else(|| line.strip_prefix("class "))?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

fn opens_python_docstring(trimmed: &str) -> bool {
    PY_DOC_OPENERS.iter().any(|q| trimmed.starts_with(q))
}

/// `Doc` facets from a Python source: the module docstring (first statement
/// is a string) keys the module; a docstring directly under a top-level
/// `def`/`class` keys that item. Lexical and fail-closed — only what is
/// positively present counts.
fn python_doc_facets(module: &str, source: &str) -> BTreeSet<WishFacet> {
    let mut facets = BTreeSet::new();
    // Module docstring: the first non-empty, non-comment line opens a string.
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if opens_python_docstring(t) {
            facets.insert(WishFacet::doc(module));
        }
        break;
    }
    // Item docstrings: a top-level def/class whose next non-empty line opens
    // a string documents that item.
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        if !(line.starts_with("def ") || line.starts_with("class ")) {
            continue;
        }
        let Some(name) = python_item_name(line) else {
            continue;
        };
        // The header may span lines until the trailing ':'.
        let mut header = line.trim_end().to_string();
        while !header.ends_with(':') {
            match lines.next() {
                Some(cont) => header.push_str(cont.trim_end()),
                None => return facets,
            }
        }
        while let Some(next) = lines.peek() {
            let t = next.trim();
            if t.is_empty() {
                lines.next();
                continue;
            }
            if opens_python_docstring(t) {
                facets.insert(WishFacet::doc(name.clone()));
            }
            break;
        }
    }
    facets
}

/// Lexical facets of every `.py` file under `dir`: one `Module` per file
/// (Python's file = module law), `Symbol` + `Signature` facets from the
/// xlang extractor (the same verified rules the substrate scans with),
/// `Test` facets for its recognized test functions, and `Doc` facets from
/// docstrings. Read-only; needs no interpreter.
pub fn facets_from_python_dir(dir: impl AsRef<Path>) -> BTreeSet<WishFacet> {
    let root = dir.as_ref();
    let mut files = Vec::new();
    collect_py(root, &mut files);
    files.sort();
    let mut facets = BTreeSet::new();
    for file in &files {
        let Some(module) = python_module_name(file) else {
            continue;
        };
        let Ok(content) = std::fs::read_to_string(file) else {
            continue;
        };
        facets.insert(WishFacet::module(module.clone()));
        let sets = symbol_sets(SourceLanguage::Python, &content);
        for key in &sets.functions {
            if let Some((name, _arity)) = key.split_once('/') {
                facets.insert(WishFacet::symbol(name));
            }
            facets.insert(WishFacet::signature(key.clone()));
        }
        for t in &sets.types {
            facets.insert(WishFacet::symbol(t.clone()));
        }
        for t in &sets.tests {
            facets.insert(WishFacet::test(t.clone()));
        }
        facets.extend(python_doc_facets(&module, &content));
    }
    facets
}

/// A cargo-less workspace that carries Python source — observed without
/// `cargo metadata`.
fn workspace_is_python(root: &Path) -> bool {
    if root.join("Cargo.toml").exists() {
        return false;
    }
    if root.join("pyproject.toml").exists() {
        return true;
    }
    let mut files = Vec::new();
    collect_py(root, &mut files);
    !files.is_empty()
}

/// Walk every `.rs` file under `dir` and collect the `(test_fn_name, spec)`
/// pairs from `// kosmo:behavior:` markers. Read-only.
fn behavior_specs_from_dir(dir: impl AsRef<Path>) -> Vec<(String, String)> {
    let mut files = Vec::new();
    collect_rs(dir.as_ref(), &mut files);
    files.sort();
    let mut out = Vec::new();
    for file in files {
        if let Ok(content) = std::fs::read_to_string(&file) {
            out.extend(behavior_specs_from_source(&content));
        }
    }
    out
}

/// Observe a workspace at **crate + module + symbol** granularity.
///
/// [`observe_workspace`] (crate facets via `cargo metadata`) merged with the
/// `Module` / `Symbol` facets lexed from every `.rs` file under `root`. This is
/// what lets a wish target finer structure than whole crates.
pub fn observe_workspace_deep(
    root: impl Into<PathBuf>,
) -> Result<ObservedTopology, ParseBackError> {
    let root = root.into();
    // A cargo-less Python workspace observes by its own law (file = module)
    // — no `cargo metadata` requirement: the polyglot door.
    if workspace_is_python(&root) {
        let mut observed = ObservedTopology::empty();
        for facet in facets_from_python_dir(&root) {
            observed.insert(facet);
        }
        return Ok(observed);
    }
    let mut observed = observe_workspace(root.clone())?;
    for facet in facets_from_rust_dir(&root) {
        observed.insert(facet);
    }
    // Polyglot observation: Python files inside a cargo workspace are
    // measurable targets too, not blind spots.
    for facet in facets_from_python_dir(&root) {
        observed.insert(facet);
    }
    Ok(observed)
}

// ─── Validated observation (green tests) ────────────────────────────────────

/// Parse libtest output into a map of `test name → passed`.
///
/// Recognizes the per-test lines `test NAME ... ok` / `... FAILED`; `ignored`
/// and the `test result:` summary line are skipped. Pure and deterministic.
pub fn parse_test_results(output: &str) -> BTreeMap<String, bool> {
    let mut results = BTreeMap::new();
    for line in output.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        let Some((name, status)) = rest.rsplit_once(" ... ") else {
            continue;
        };
        let passed = match status.trim() {
            "ok" => true,
            "FAILED" => false,
            _ => continue, // ignored / measured / …
        };
        let name = name.trim();
        if name.is_empty() || name == "result:" {
            continue;
        }
        results.insert(name.to_string(), passed);
    }
    results
}

/// Build `Test` facets for the **passing** tests only, keyed by the bare test
/// function name (the last `::` segment), matching the lexical extractor.
pub fn passing_test_facets(results: &BTreeMap<String, bool>) -> BTreeSet<WishFacet> {
    results
        .iter()
        .filter(|(_, &passed)| passed)
        .map(|(name, _)| WishFacet::test(name.rsplit("::").next().unwrap_or(name)))
        .collect()
}

/// Run `cargo test` in `root` and parse which named tests passed.
///
/// Heavy (compiles + runs the suite), so it is opt-in via
/// [`observe_workspace_validated`], not part of default observation. A non-zero
/// exit (failing tests) is not an error here — the per-test verdicts are parsed
/// from the output regardless.
pub fn run_workspace_tests(root: impl AsRef<Path>) -> std::io::Result<BTreeMap<String, bool>> {
    let manifest = root.as_ref().join("Cargo.toml");
    let output = std::process::Command::new("cargo")
        .args(["test", "--manifest-path"])
        .arg(&manifest)
        .output()?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(parse_test_results(&combined))
}

/// Observe a workspace at crate+module+symbol granularity **with validated
/// tests**: lexical `Test` facets (mere presence) are replaced by the set of
/// tests that actually pass. This makes a `Test` wish mean "a *green* test
/// named X", binding the wish to validated behaviour.
///
/// Heavier than [`observe_workspace_deep`] (it runs the suite). If the test run
/// fails to start, it falls back to the lexical `Test` facets.
pub fn observe_workspace_validated(
    root: impl Into<PathBuf>,
) -> Result<ObservedTopology, ParseBackError> {
    let root = root.into();
    let mut observed = observe_workspace_deep(&root)?;
    if let Ok(results) = run_workspace_tests(&root) {
        observed.retain(|f| f.kind != WishFacetKind::Test);
        for facet in passing_test_facets(&results) {
            observed.insert(facet);
        }
        // Behaviour facets: a spec is present only if its scaffolded spec-test
        // passed — the keystone, observed by running the suite (fail-closed).
        let passing: BTreeSet<String> = results
            .iter()
            .filter(|(_, &ok)| ok)
            .map(|(n, _)| n.rsplit("::").next().unwrap_or(n).to_string())
            .collect();
        for facet in behavior_facets(&behavior_specs_from_dir(&root), &passing) {
            observed.insert(facet);
        }
    }
    Ok(observed)
}

// ─── Runtime observation (level 5 — execute and probe) ──────────────────────

/// A runtime probe's expectation: an exit code, a stdout substring, and/or
/// a wall-clock budget in milliseconds — the first *quality axis*: speed as
/// a measurable, fail-closed facet.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunExpect {
    exit: Option<i32>,
    stdout_contains: Option<String>,
    /// The witness's duration must not exceed this many milliseconds.
    max_ms: Option<u64>,
}

/// Parse a `Run` facet key `"args=>expect"` into `(args, expect)`. `args` is
/// comma-separated (passed after `cargo run --`); `expect` is `exit:<n>`
/// and/or `out~<substr>`, optionally capped by a tail-anchored `ms<N` budget
/// (`"hi=>out~hi,ms<50"`). `None` if malformed.
fn parse_run_key(key: &str) -> Option<(Vec<String>, RunExpect)> {
    let (args_str, expect_str) = key.split_once("=>")?;
    let args = args_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let expect = parse_run_expect(expect_str.trim())?;
    Some((args, expect))
}

fn parse_run_expect(s: &str) -> Option<RunExpect> {
    // The budget atom anchors at the tail (so an `out~` substring may
    // contain anything before it): "...,ms<N", or the lone "ms<N".
    let (s, max_ms) = if let Some((head, n)) = s.rsplit_once(",ms<") {
        (head, Some(n.parse::<u64>().ok()?))
    } else if let Some(n) = s.strip_prefix("ms<") {
        ("", Some(n.parse::<u64>().ok()?))
    } else {
        (s, None)
    };
    let mut exit = None;
    let rest = if let Some(r) = s.strip_prefix("exit:") {
        let digits: String = r
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        exit = Some(digits.parse::<i32>().ok()?);
        let r2 = &r[digits.len()..];
        r2.strip_prefix(',').unwrap_or(r2)
    } else {
        s
    };
    let stdout_contains = rest.strip_prefix("out~").map(|x| x.to_string());
    if exit.is_none() && stdout_contains.is_none() && max_ms.is_none() {
        return None;
    }
    Some(RunExpect {
        exit,
        stdout_contains,
        max_ms,
    })
}

/// A clean-exit witness that matches the expectation. Fail-closed: a timeout,
/// crash, or spawn failure never matches, even if the substring was printed —
/// and a blown budget is a miss, never a warning.
fn run_matches(w: &kosmo_sandbox::RuntimeWitness, e: &RunExpect) -> bool {
    if w.verdict != kosmo_sandbox::RunVerdict::Exited {
        return false;
    }
    if let Some(code) = e.exit {
        if w.exit_code != Some(code) {
            return false;
        }
    }
    if let Some(sub) = &e.stdout_contains {
        if !w.stdout.contains(sub) {
            return false;
        }
    }
    if let Some(cap) = e.max_ms {
        if w.duration.as_millis() > u128::from(cap) {
            return false;
        }
    }
    true
}

/// Extract a `// kosmo:run:` probe key from a source line.
fn run_marker(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("//")?.trim_start();
    let key = rest.strip_prefix("kosmo:run:")?.trim();
    (!key.is_empty()).then(|| key.to_string())
}

/// Read all `// kosmo:run:` probe keys under `dir` (sorted, deduped). Read-only.
fn run_probes_from_dir(dir: impl AsRef<Path>) -> Vec<String> {
    let mut files = Vec::new();
    collect_rs(dir.as_ref(), &mut files);
    files.sort();
    let mut out = Vec::new();
    for file in files {
        if let Ok(content) = std::fs::read_to_string(&file) {
            for line in content.lines() {
                if let Some(key) = run_marker(line.trim()) {
                    out.push(key);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Observe a workspace **with runtime probes**: everything
/// [`observe_workspace_validated`] sees, plus — for each `// kosmo:run:` marker
/// — the built artifact is *executed* under the sandbox and the `Run` facet is
/// emitted iff the program exits cleanly and matches the probe (fail-closed).
///
/// The heaviest observation (it builds and runs the binary). Opt-in: used only
/// when the wish carries a `Run` facet.
pub fn observe_workspace_runtime(
    root: impl Into<PathBuf>,
) -> Result<ObservedTopology, ParseBackError> {
    use kosmo_sandbox::{RunSpec, Sandbox};
    let root = root.into();
    let mut observed = observe_workspace_validated(&root)?;
    let probes = run_probes_from_dir(&root);
    if !probes.is_empty() {
        let manifest = root.join("Cargo.toml").to_string_lossy().into_owned();
        let sandbox = Sandbox::new()
            .with_cwd(&root)
            .with_timeout(std::time::Duration::from_secs(60));
        for key in probes {
            let Some((args, expect)) = parse_run_key(&key) else {
                continue;
            };
            let mut run_args = vec![
                "run".to_string(),
                "--quiet".to_string(),
                "--manifest-path".to_string(),
                manifest.clone(),
                "--".to_string(),
            ];
            run_args.extend(args);
            let witness = sandbox.run(&RunSpec::new("cargo", run_args));
            if run_matches(&witness, &expect) {
                observed.insert(WishFacet::run(key));
            }
        }
    }
    Ok(observed)
}

// ─── Service observation (level 6 — start a server and probe it) ─────────────

/// A service probe's expectation: an HTTP status and/or a body substring.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ServiceExpect {
    status: Option<u16>,
    body_contains: Option<String>,
}

/// Parse a `Service` facet key `"method:path=>expect"` into a probe + its
/// expectation. `expect` is an HTTP status and/or `body~<substr>`.
fn parse_service_key(key: &str) -> Option<(kosmo_sandbox::HttpProbe, ServiceExpect)> {
    let (method, rest) = key.split_once(':')?;
    let (path, exp) = rest.split_once("=>")?;
    if method.trim().is_empty() || path.trim().is_empty() {
        return None;
    }
    let probe = kosmo_sandbox::HttpProbe {
        method: method.trim().to_string(),
        path: path.trim().to_string(),
    };
    Some((probe, parse_service_expect(exp.trim())?))
}

fn parse_service_expect(s: &str) -> Option<ServiceExpect> {
    let (status_part, body) = match s.split_once(",body~") {
        Some((st, b)) => (st, Some(b.to_string())),
        None => (s, None),
    };
    let status = if status_part.is_empty() {
        None
    } else {
        Some(status_part.parse::<u16>().ok()?)
    };
    if status.is_none() && body.is_none() {
        return None;
    }
    Some(ServiceExpect {
        status,
        body_contains: body,
    })
}

/// A served probe that matches. Fail-closed: a server that never answered (never
/// ready / spawn failed) never matches.
fn service_matches(w: &kosmo_sandbox::ServiceWitness, e: &ServiceExpect) -> bool {
    if w.verdict != kosmo_sandbox::ServiceVerdict::Probed {
        return false;
    }
    if let Some(st) = e.status {
        if w.status != Some(st) {
            return false;
        }
    }
    if let Some(sub) = &e.body_contains {
        if !w.body.contains(sub) {
            return false;
        }
    }
    true
}

/// Extract a `// kosmo:service:` probe key from a source line.
fn service_marker(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("//")?.trim_start();
    let key = rest.strip_prefix("kosmo:service:")?.trim();
    (!key.is_empty()).then(|| key.to_string())
}

/// Read all `// kosmo:service:` probe keys under `dir` (sorted, deduped).
fn service_probes_from_dir(dir: impl AsRef<Path>) -> Vec<String> {
    let mut files = Vec::new();
    collect_rs(dir.as_ref(), &mut files);
    files.sort();
    let mut out = Vec::new();
    for file in files {
        if let Ok(content) = std::fs::read_to_string(&file) {
            for line in content.lines() {
                if let Some(key) = service_marker(line.trim()) {
                    out.push(key);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Observe a workspace **with service probes**: everything
/// [`observe_workspace_runtime`] sees, plus — for each `// kosmo:service:`
/// marker — the built artifact is **started as a server**, awaited, probed over
/// HTTP, and torn down; the `Service` facet is emitted iff the server answers
/// and matches (fail-closed). The deepest, heaviest observation.
pub fn observe_workspace_service(
    root: impl Into<PathBuf>,
) -> Result<ObservedTopology, ParseBackError> {
    use kosmo_sandbox::{RunSpec, Sandbox};
    let root = root.into();
    let mut observed = observe_workspace_runtime(&root)?;
    let probes = service_probes_from_dir(&root);
    if !probes.is_empty() {
        let manifest = root.join("Cargo.toml").to_string_lossy().into_owned();
        // Pre-build once so each serve starts fast (the readiness budget then
        // covers startup, not a cold compile).
        let _ = Sandbox::new()
            .with_cwd(&root)
            .with_timeout(std::time::Duration::from_secs(180))
            .run(&RunSpec::new(
                "cargo",
                ["build", "--quiet", "--manifest-path", manifest.as_str()],
            ));
        let sandbox = Sandbox::new()
            .with_cwd(&root)
            .with_timeout(std::time::Duration::from_secs(20));
        for key in probes {
            let Some((probe, expect)) = parse_service_key(&key) else {
                continue;
            };
            let spec = RunSpec::new(
                "cargo",
                ["run", "--quiet", "--manifest-path", manifest.as_str()],
            );
            let witness = sandbox.serve_and_probe(&spec, &probe);
            if service_matches(&witness, &expect) {
                observed.insert(WishFacet::service(key));
            }
        }
    }
    Ok(observed)
}

// ─── Natural-language → Wish (the human front door) ─────────────────────────

/// Normalize a word to lowercase alphanumerics (strips punctuation/markup).
fn norm(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Map a trigger word to the facet kind it introduces, if any.
fn trigger_kind(word: &str) -> Option<WishFacetKind> {
    match norm(word).as_str() {
        "crate" | "crates" | "package" | "packages" => Some(WishFacetKind::Crate),
        "module" | "modules" | "mod" => Some(WishFacetKind::Module),
        "function" | "functions" | "fn" | "func" | "method" | "methods" => {
            Some(WishFacetKind::Symbol)
        }
        "type" | "types" | "struct" | "structs" | "enum" | "enums" | "trait" | "traits"
        | "interface" | "symbol" | "symbols" => Some(WishFacetKind::Symbol),
        "dependency" | "dependencies" | "depends" | "dep" => Some(WishFacetKind::Dependency),
        "signature" | "signatures" | "sig" => Some(WishFacetKind::Signature),
        "contract" | "contracts" => Some(WishFacetKind::Contract),
        "behavior" | "behaviors" | "behaviour" | "behaviours" | "spec" | "specs" => {
            Some(WishFacetKind::Behavior)
        }
        // A validated data-flow: `a flow f(x)>>g=>y` is a Behavior over the
        // composed call `g(f(x))` — the keystone applied to a level-3 wire.
        "flow" | "flows" | "pipeline" | "pipelines" => Some(WishFacetKind::Behavior),
        "composition" | "compositions" | "compose" => Some(WishFacetKind::Composition),
        // A runtime probe: `a run add,2,3=>out~5` runs the built binary and
        // checks its exit/stdout (level 5, observed by execution).
        "run" | "runs" => Some(WishFacetKind::Run),
        // A service probe: `a service GET:/health=>200` starts the binary as a
        // server and probes it over HTTP (level 6, observed by serving).
        "service" | "endpoint" => Some(WishFacetKind::Service),
        "capability" | "capabilities" | "feature" | "features" => Some(WishFacetKind::Capability),
        "test" | "tests" => Some(WishFacetKind::Test),
        "doc" | "docs" | "documented" | "documentation" | "docstring" => Some(WishFacetKind::Doc),
        _ => None,
    }
}

/// Words that may sit between a trigger and the name it introduces.
const FILLERS: &[&str] = &[
    "a", "an", "the", "called", "named", "name", "on", "of", "for",
];

/// Clean a candidate name token: strip surrounding markup/punctuation, keeping
/// identifier characters (and `-` for crate names). `None` if nothing is left.
fn clean_name(tok: &str) -> Option<String> {
    let t = tok.trim_matches(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'));
    if t.is_empty() || !t.chars().any(|c| c.is_alphanumeric()) {
        return None;
    }
    Some(t.to_string())
}

// ─── Archetypes (the breadth axis) ──────────────────────────────────────────

/// A high-level archetype: a named template that fans out into a *bundle* of
/// lower facets. One prose word → many facets — the breadth axis of the Horizon
/// floor (`docs/HORIZON-behavior-archetype.md`). Archetypes are a pure compiler
/// concept; they introduce no new facet kind and need no scaffolder change —
/// they expand into the existing leaves the substrate already builds and
/// observes. Every current archetype expands within the root crate, so the
/// deterministic scaffolder realizes the structural part offline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Archetype {
    /// `crud <name>` — a module, `create_`/`get_` typed handlers, and a marker.
    Crud,
    /// `endpoint <name>` — a typed request handler and a marker.
    Endpoint,
    /// `component <name>` — a module and a marker.
    Component,
}

/// Recognize an archetype keyword. Kept disjoint from [`trigger_kind`] so a
/// word is either a leaf trigger or an archetype, never both. The keywords are
/// deliberately distinctive (not `route`/`widget`/…) so they don't shadow words
/// commonly used as facet *names* (e.g. a module called `routes`); archetype
/// keywords are effectively reserved.
fn archetype_trigger(word: &str) -> Option<Archetype> {
    match norm(word).as_str() {
        "crud" => Some(Archetype::Crud),
        "endpoint" | "endpoints" => Some(Archetype::Endpoint),
        "component" | "components" => Some(Archetype::Component),
        _ => None,
    }
}

/// Expand an archetype + target `name` into its facet bundle. Pure and
/// deterministic; every leaf is scaffoldable and observable, so the structural
/// bundle converges offline (a behaviour leaf, if any, needs the impl). Typed
/// handlers use `String` so they compile without extra type definitions.
pub fn expand_archetype(arch: Archetype, name: &str) -> Vec<WishFacet> {
    match arch {
        Archetype::Crud => vec![
            WishFacet::module(name),
            WishFacet::contract(format!("create_{name}"), &["String"], "String"),
            WishFacet::contract(format!("get_{name}"), &["String"], "String"),
            WishFacet::capability(format!("crud:{name}")),
        ],
        Archetype::Endpoint => vec![
            WishFacet::contract(name, &["String"], "String"),
            WishFacet::capability(format!("endpoint:{name}")),
        ],
        Archetype::Component => vec![
            WishFacet::module(name),
            WishFacet::capability(format!("component:{name}")),
        ],
    }
}

/// Find the name token introduced after position `i`: skip [`FILLERS`], stop at
/// the next leaf or archetype trigger. Returns `(clean_name, index_after)`.
fn name_after(tokens: &[&str], i: usize) -> Option<(String, usize)> {
    let mut j = i + 1;
    while j < tokens.len() {
        if trigger_kind(tokens[j]).is_some() || archetype_trigger(tokens[j]).is_some() {
            return None;
        }
        if FILLERS.contains(&norm(tokens[j]).as_str()) {
            j += 1;
            continue;
        }
        break;
    }
    if j < tokens.len() {
        clean_name(tokens[j]).map(|name| (name, j + 1))
    } else {
        None
    }
}

/// Compile a prose intent statement into a content-addressed [`Wish`].
///
/// Deterministic and dependency-free: it scans the prose for **archetype**
/// keywords (`crud`/`endpoint`/`component` — each fans out into a facet bundle)
/// and structural **leaf** trigger words (`crate`/`package`, `module`/`mod`,
/// `function`/`fn`/`method`, `type`/`struct`/`trait`, `contract`, `behavior`,
/// …), turning each `keyword NAME` phrase into the required [`WishFacet`]s. The
/// `Wish` label is the prose itself, so the intent statement is part of the
/// wish's identity.
///
/// Convention: name the thing *after* the keyword — "a crud user, a crate
/// kosmo-server, a function handle_request". Free word order and fuzzier
/// phrasing are the job of an LLM-backed [`WishCompiler`]; this is the offline,
/// byte-deterministic front door.
pub fn compile_wish(prose: &str, policy_id: Digest, evidence_bundle_id: Digest) -> Wish {
    let tokens: Vec<&str> = prose.split_whitespace().collect();
    let mut facets: Vec<WishFacet> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        // Archetypes take precedence: one keyword expands into many facets.
        if let Some(arch) = archetype_trigger(tokens[i]) {
            if let Some((name, next)) = name_after(&tokens, i) {
                facets.extend(expand_archetype(arch, &name));
                i = next;
                continue;
            }
        }
        if let Some(kind) = trigger_kind(tokens[i]) {
            if let Some((name, next)) = name_after(&tokens, i) {
                facets.push(WishFacet::new(kind, name));
                i = next;
                continue;
            }
        }
        i += 1;
    }
    let predicates: Vec<WishPredicate> = facets.into_iter().map(WishPredicate::require).collect();
    Wish::new(
        prose.trim().to_string(),
        predicates,
        policy_id,
        evidence_bundle_id,
    )
}

/// Error from a [`WishCompiler`] backend (e.g. an LLM call failing).
#[derive(Clone, Debug)]
pub struct WishCompileError {
    pub message: String,
}

impl std::fmt::Display for WishCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wish compile error: {}", self.message)
    }
}
impl std::error::Error for WishCompileError {}

/// Pluggable "prose → [`Wish`]" backend. The deterministic [`RuleWishCompiler`]
/// is the reference implementation; an LLM-backed compiler (a later crate, the
/// counterpart to `kosmo-synthesizer-llm`) would slot in behind the same trait.
pub trait WishCompiler: Send + Sync {
    fn compile(
        &self,
        prose: &str,
        policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Result<Wish, WishCompileError>;
    fn name(&self) -> &str;
}

/// The deterministic, offline front door — delegates to [`compile_wish`].
pub struct RuleWishCompiler;

impl WishCompiler for RuleWishCompiler {
    fn compile(
        &self,
        prose: &str,
        policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Result<Wish, WishCompileError> {
        Ok(compile_wish(prose, policy_id, evidence_bundle_id))
    }
    fn name(&self) -> &str {
        "rule-based"
    }
}

// ─── Norm catalog (learned archetypes in the prose grammar) ─────────────────

/// True iff `word` is reserved by the deterministic wish grammar — a leaf
/// trigger word, an archetype keyword, or a filler. A promoted norm trigger
/// may never shadow any of these: the built-in grammar always wins, by
/// construction rather than by precedence.
pub fn is_reserved_wish_word(word: &str) -> bool {
    trigger_kind(word).is_some()
        || archetype_trigger(word).is_some()
        || FILLERS.contains(&norm(word).as_str())
}

/// Why a set of norms could not become a [`NormCatalog`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormCatalogError {
    /// The trigger collides with the built-in wish grammar.
    ReservedTrigger { trigger: String, norm_id: Digest },
    /// Two armed norms claim the same trigger word.
    DuplicateTrigger {
        trigger: String,
        first: Digest,
        second: Digest,
    },
    /// An armed norm fails its own anti-disease validation.
    InvalidNorm { norm_id: Digest, message: String },
}

impl std::fmt::Display for NormCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReservedTrigger { trigger, .. } => {
                write!(
                    f,
                    "norm trigger '{trigger}' is reserved by the wish grammar"
                )
            }
            Self::DuplicateTrigger { trigger, .. } => {
                write!(f, "two norms claim the trigger '{trigger}'")
            }
            Self::InvalidNorm { message, .. } => write!(f, "invalid norm: {message}"),
        }
    }
}

impl std::error::Error for NormCatalogError {}

/// The armed norms available to [`compile_wish_with_norms`], keyed by their
/// normalized trigger word. Built from a norm store's contents; norms whose
/// `trigger` is `None` are skipped (present in the store, inert in the
/// grammar — exactly the learned-but-unpromoted state).
#[derive(Debug, Default)]
pub struct NormCatalog {
    by_trigger: std::collections::BTreeMap<String, Norm>,
}

impl NormCatalog {
    /// The empty catalog — the system's permanent starting state.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Index the **armed** norms (trigger ≠ `None`) of `norms`. Every armed
    /// norm is re-validated; reserved-word and duplicate-trigger collisions
    /// are hard errors (fail closed, never silently shadow).
    pub fn from_norms<'a>(
        norms: impl IntoIterator<Item = &'a Norm>,
    ) -> Result<Self, NormCatalogError> {
        let mut by_trigger = std::collections::BTreeMap::new();
        for n in norms {
            let Some(trigger) = n.trigger.as_deref() else {
                continue;
            };
            if let Err(e) = n.validate() {
                return Err(NormCatalogError::InvalidNorm {
                    norm_id: n.norm_id,
                    message: e.to_string(),
                });
            }
            let key = norm(trigger);
            if key.is_empty() || is_reserved_wish_word(&key) {
                return Err(NormCatalogError::ReservedTrigger {
                    trigger: trigger.to_string(),
                    norm_id: n.norm_id,
                });
            }
            if let Some(prev) = by_trigger.insert(key.clone(), n.clone()) {
                return Err(NormCatalogError::DuplicateTrigger {
                    trigger: key,
                    first: prev.norm_id,
                    second: n.norm_id,
                });
            }
        }
        Ok(Self { by_trigger })
    }

    pub fn is_empty(&self) -> bool {
        self.by_trigger.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_trigger.len()
    }

    /// The norm armed on `token` (normalized like every grammar word), if any.
    pub fn expand(&self, token: &str) -> Option<&Norm> {
        self.by_trigger.get(&norm(token))
    }

    /// All armed norms, ordered by trigger.
    pub fn norms(&self) -> impl Iterator<Item = &Norm> {
        self.by_trigger.values()
    }
}

/// [`compile_wish`] extended by a catalog of **promoted** norms: a trigger
/// word in the prose expands its norm's facet templates exactly like a
/// built-in archetype (`trigger NAME` → `norm.instantiate(NAME)`).
///
/// Grammar precedence is archetype → norm → leaf, but the three trigger sets
/// are disjoint by construction ([`NormCatalog::from_norms`] rejects reserved
/// words), so no built-in phrase changes meaning. With an empty catalog the
/// output is **identical** to [`compile_wish`] — pinned by test; `compile_wish`
/// itself is untouched and remains byte-deterministic.
pub fn compile_wish_with_norms(
    prose: &str,
    catalog: &NormCatalog,
    policy_id: Digest,
    evidence_bundle_id: Digest,
) -> Wish {
    let tokens: Vec<&str> = prose.split_whitespace().collect();
    let mut facets: Vec<WishFacet> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some(arch) = archetype_trigger(tokens[i]) {
            if let Some((name, next)) = name_after(&tokens, i) {
                facets.extend(expand_archetype(arch, &name));
                i = next;
                continue;
            }
        }
        // A promoted norm is a learned archetype: same expansion seam.
        if let Some(armed) = catalog.expand(tokens[i]) {
            if let Some((name, next)) = name_after(&tokens, i) {
                facets.extend(armed.instantiate(&name));
                i = next;
                continue;
            }
        }
        if let Some(kind) = trigger_kind(tokens[i]) {
            if let Some((name, next)) = name_after(&tokens, i) {
                facets.push(WishFacet::new(kind, name));
                i = next;
                continue;
            }
        }
        i += 1;
    }
    let predicates: Vec<WishPredicate> = facets.into_iter().map(WishPredicate::require).collect();
    Wish::new(
        prose.trim().to_string(),
        predicates,
        policy_id,
        evidence_bundle_id,
    )
}

// ─── WishSession ─────────────────────────────────────────────────────────────

/// A stateful descent toward a wish-attractor across successive observations.
///
/// Ties the three layers together: each [`WishSession::observe`] measures the
/// workspace against the wish (Run 1), appends the resulting distance to the
/// trajectory, and exposes a [`WishConvergenceTrace`] (Run 2) so the caller can
/// ask whether the system is converging — or has **diverged** from the
/// attractor, which the agent loop must treat fail-closed.
///
/// Serializable so a descent can be persisted and resumed across sessions,
/// consistent with the substrate's content-addressed, replayable contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WishSession {
    wish: Wish,
    evidence_bundle_id: Digest,
    assessments: Vec<WishAssessment>,
}

impl WishSession {
    /// Start a descent toward `wish`. `evidence_bundle_id` binds every
    /// assessment and trace this session produces (CROSS-006).
    pub fn new(wish: Wish, evidence_bundle_id: Digest) -> Self {
        Self {
            wish,
            evidence_bundle_id,
            assessments: vec![],
        }
    }

    pub fn wish(&self) -> &Wish {
        &self.wish
    }

    /// Assess the wish against an observation, append it to the trajectory, and
    /// return the new assessment.
    pub fn observe(&mut self, observed: &ObservedTopology) -> &WishAssessment {
        let assessment = assess_wish(&self.wish, observed, self.evidence_bundle_id);
        self.assessments.push(assessment);
        self.assessments.last().expect("just pushed an assessment")
    }

    /// All assessments recorded so far, oldest first.
    pub fn assessments(&self) -> &[WishAssessment] {
        &self.assessments
    }

    /// The most recent assessment, if any.
    pub fn latest(&self) -> Option<&WishAssessment> {
        self.assessments.last()
    }

    /// Number of observations recorded.
    pub fn iterations(&self) -> usize {
        self.assessments.len()
    }

    /// The convergence trace accumulated so far — the attractor contract.
    pub fn trace(&self) -> WishConvergenceTrace {
        WishConvergenceTrace::from_assessments(&self.assessments, self.evidence_bundle_id)
    }

    /// The system is at the wish-attractor: the latest observation realized the wish.
    pub fn at_attractor(&self) -> bool {
        self.trace().at_attractor()
    }

    /// No observed step has increased the distance: the contraction invariant holds.
    pub fn is_contractive(&self) -> bool {
        self.trace().is_contractive()
    }

    /// The wish is realized and the descent never diverged.
    pub fn is_converged(&self) -> bool {
        self.trace().is_converged()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{AttractorStatus, Wish, WishClosureStatus, WishFacet, WishPredicate, Q16};
    use kosmo_parseback::CrateFingerprint;
    use std::collections::{BTreeMap, BTreeSet};

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    /// A synthetic crate-topology snapshot containing the given crate names.
    fn snap(names: &[&str]) -> TopologySnapshot {
        let mut nodes = BTreeMap::new();
        for n in names {
            nodes.insert(
                n.to_string(),
                CrateFingerprint::new(n.to_string(), vec!["lib.rs".into()], vec![]),
            );
        }
        TopologySnapshot::from_parts(ParseBackScanScope::FullWorkspace, nodes, BTreeSet::new())
    }

    fn wish_two() -> Wish {
        Wish::new(
            "two crates",
            [
                WishPredicate::require(WishFacet::crate_("a")),
                WishPredicate::require(WishFacet::crate_("b")),
            ],
            d(b"policy"),
            d(b"bundle"),
        )
    }

    // ── observation adapter ───────────────────────────────────────────────

    #[test]
    fn facets_from_snapshot_emits_one_crate_facet_per_node() {
        let facets = facets_from_snapshot(&snap(&["a", "b"]));
        assert_eq!(facets.len(), 2);
        assert!(facets.contains(&WishFacet::crate_("a")));
        assert!(facets.contains(&WishFacet::crate_("b")));
    }

    #[test]
    fn observe_snapshot_contains_crate_facets() {
        let observed = observe_snapshot(&snap(&["alpha"]));
        assert!(observed.contains(&WishFacet::crate_("alpha")));
        assert!(!observed.contains(&WishFacet::crate_("missing")));
    }

    // ── WishSession descent ───────────────────────────────────────────────

    #[test]
    fn session_observe_appends_assessments() {
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        assert_eq!(s.iterations(), 0);
        let a = s.observe(&observe_snapshot(&snap(&[])));
        assert_eq!(a.distance, Q16::ONE);
        assert_eq!(s.iterations(), 1);
        s.observe(&observe_snapshot(&snap(&["a", "b"])));
        assert_eq!(s.iterations(), 2);
        assert_eq!(s.latest().unwrap().distance, Q16::ZERO);
    }

    #[test]
    fn session_converges_over_observations() {
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        s.observe(&observe_snapshot(&snap(&[]))); // ONE
        s.observe(&observe_snapshot(&snap(&["a"]))); // HALF
        s.observe(&observe_snapshot(&snap(&["a", "b"]))); // ZERO
        let trace = s.trace();
        assert_eq!(trace.distances, vec![Q16::ONE, Q16::HALF, Q16::ZERO]);
        assert_eq!(trace.status, AttractorStatus::Converged);
        assert!(s.is_contractive());
        assert!(s.is_converged());
        assert!(s.at_attractor());
    }

    #[test]
    fn session_detects_divergence_fail_closed() {
        // Realize the wish, then regress: the distance rises → divergence.
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        s.observe(&observe_snapshot(&snap(&["a", "b"]))); // ZERO
        s.observe(&observe_snapshot(&snap(&["a"]))); // HALF — moved away
        let trace = s.trace();
        assert_eq!(trace.status, AttractorStatus::Diverging);
        assert_eq!(trace.first_divergence, Some(1));
        assert!(!s.is_contractive());
    }

    #[test]
    fn session_partial_observation_is_approaching() {
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        let a = s.observe(&observe_snapshot(&snap(&["a"])));
        assert_eq!(a.status, WishClosureStatus::Approaching);
        assert_eq!(a.distance, Q16::HALF);
        assert_eq!(a.unmet_facets, vec![WishFacet::crate_("b")]);
    }

    #[test]
    fn session_is_serde_round_trippable() {
        let mut s = WishSession::new(wish_two(), d(b"ev"));
        s.observe(&observe_snapshot(&snap(&["a"])));
        let json = serde_json::to_string(&s).unwrap();
        let back: WishSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back.iterations(), 1);
        assert_eq!(back.wish().id, s.wish().id);
        assert_eq!(back.latest().unwrap().distance, Q16::HALF);
    }

    // ── real-workspace observation (graceful skip if cargo unavailable) ────

    #[test]
    fn observe_workspace_reads_real_crate_topology() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let observed = match observe_workspace(&root) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("cargo metadata unavailable, skipping: {e}");
                return;
            }
        };
        // The workspace contains these crates (kosmo-intent is this crate).
        assert!(observed.contains(&WishFacet::crate_("kosmo-core")));
        assert!(observed.contains(&WishFacet::crate_("kosmo-intent")));

        // A wish over real + imaginary crates lands Approaching: 2 met, 1 unmet.
        let wish = Wish::new(
            "exists in the real workspace",
            [
                WishPredicate::require(WishFacet::crate_("kosmo-core")),
                WishPredicate::require(WishFacet::crate_("kosmo-intent")),
                WishPredicate::require(WishFacet::crate_("kosmo-does-not-exist-xyz")),
            ],
            d(b"policy"),
            d(b"bundle"),
        );
        let mut s = WishSession::new(wish, d(b"ev"));
        let a = s.observe(&observed);
        assert_eq!(a.status, WishClosureStatus::Approaching);
        assert_eq!(a.met_count, 2);
        assert_eq!(a.total_count, 3);
        assert_eq!(
            a.unmet_facets,
            vec![WishFacet::crate_("kosmo-does-not-exist-xyz")]
        );
    }

    // ── source extraction: Module / Symbol facets (Run 6) ─────────────────

    #[test]
    fn facets_from_source_extracts_public_fn() {
        let f = facets_from_source("pub fn build() -> u32 { 1 }\n");
        assert!(f.contains(&WishFacet::symbol("build")));
    }

    #[test]
    fn facets_from_source_extracts_types() {
        let src = "pub struct Widget;\npub enum Color { Red }\npub trait Draw {}\n\
                   pub type Alias = u32;\npub union U { a: u32 }\n";
        let f = facets_from_source(src);
        for name in ["Widget", "Color", "Draw", "Alias", "U"] {
            assert!(f.contains(&WishFacet::symbol(name)), "missing {name}");
        }
    }

    #[test]
    fn facets_from_source_extracts_modules() {
        let f = facets_from_source("mod internal;\npub mod routes {}\n");
        assert!(f.contains(&WishFacet::module("internal")));
        assert!(f.contains(&WishFacet::module("routes")));
    }

    #[test]
    fn facets_from_source_skips_private_fn() {
        let f = facets_from_source("fn helper() {}\n");
        assert!(
            !f.contains(&WishFacet::symbol("helper")),
            "private fns are not part of the public surface"
        );
    }

    #[test]
    fn facets_from_source_handles_fn_modifiers() {
        let f = facets_from_source(
            "pub async fn run() {}\npub const fn c() -> u32 { 0 }\npub unsafe fn u() {}\n",
        );
        for name in ["run", "c", "u"] {
            assert!(f.contains(&WishFacet::symbol(name)), "missing {name}");
        }
    }

    #[test]
    fn facets_from_source_extracts_const_and_static_items() {
        let f = facets_from_source("pub const MAX: u32 = 9;\npub static NAME: &str = \"x\";\n");
        assert!(f.contains(&WishFacet::symbol("MAX")));
        assert!(f.contains(&WishFacet::symbol("NAME")));
    }

    #[test]
    fn facets_from_source_strips_generics_and_params() {
        let f =
            facets_from_source("pub fn map<T>(x: T) -> T { x }\npub struct Holder<T> { _p: T }\n");
        assert!(f.contains(&WishFacet::symbol("map")));
        assert!(f.contains(&WishFacet::symbol("Holder")));
    }

    #[test]
    fn facets_from_source_skips_comments_and_attributes() {
        let f = facets_from_source("// pub fn fake() {}\n#[derive(Clone)]\npub struct Real;\n");
        assert!(f.contains(&WishFacet::symbol("Real")));
        assert!(!f.contains(&WishFacet::symbol("fake")));
    }

    #[test]
    fn observe_workspace_deep_includes_symbols_and_modules() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-intent-deep-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"deep_demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            "pub fn demo_fn() -> u32 { 1 }\npub mod sub {}\n",
        )
        .unwrap();

        match observe_workspace_deep(&dir) {
            Ok(observed) => {
                assert!(observed.contains(&WishFacet::crate_("deep_demo")));
                assert!(observed.contains(&WishFacet::symbol("demo_fn")));
                assert!(observed.contains(&WishFacet::module("sub")));
            }
            Err(e) => eprintln!("cargo metadata unavailable, skipping: {e}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── natural-language → Wish (Run 7) ───────────────────────────────────

    #[test]
    fn compile_wish_extracts_crate() {
        let w = compile_wish("I want a crate kosmo-server", d(b"policy"), d(b"bundle"));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::crate_("kosmo-server")));
    }

    #[test]
    fn compile_wish_extracts_module_and_function() {
        let w = compile_wish(
            "a module routes and a function handle_request",
            d(b"policy"),
            d(b"bundle"),
        );
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::module("routes")));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::symbol("handle_request")));
    }

    #[test]
    fn compile_wish_handles_backticks_and_fillers() {
        let w = compile_wish("a crate called `kosmo-foo`", d(b"policy"), d(b"bundle"));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::crate_("kosmo-foo")));
    }

    #[test]
    fn compile_wish_type_keywords_are_symbols() {
        let w = compile_wish(
            "a struct Widget and a trait Draw and an enum Color",
            d(b"policy"),
            d(b"bundle"),
        );
        for name in ["Widget", "Draw", "Color"] {
            assert!(
                w.predicates
                    .iter()
                    .any(|p| p.facet == WishFacet::symbol(name)),
                "missing {name}"
            );
        }
    }

    #[test]
    fn compile_wish_label_is_the_prose() {
        let prose = "a crate alpha";
        let w = compile_wish(prose, d(b"policy"), d(b"bundle"));
        assert_eq!(w.label, prose);
    }

    #[test]
    fn compile_wish_is_deterministic_and_content_addressed() {
        let a = compile_wish("a crate alpha, a module beta", d(b"policy"), d(b"bundle"));
        let b = compile_wish("a crate alpha, a module beta", d(b"policy"), d(b"bundle"));
        assert_eq!(a.id, b.id);
        assert!(a.verify_id());
    }

    #[test]
    fn compile_wish_no_triggers_is_vacuous() {
        let w = compile_wish("please make it nice and fast", d(b"policy"), d(b"bundle"));
        assert_eq!(w.predicate_count(), 0);
    }

    #[test]
    fn rule_compiler_trait_compiles() {
        let c = RuleWishCompiler;
        let w = c
            .compile("a crate gamma", d(b"policy"), d(b"bundle"))
            .unwrap();
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::crate_("gamma")));
        assert_eq!(c.name(), "rule-based");
    }

    #[test]
    fn compiled_wish_assesses_against_observation() {
        // The full front door: prose → Wish → measured against a workspace.
        let w = compile_wish(
            "a crate present_crate and a crate absent_crate",
            d(b"policy"),
            d(b"bundle"),
        );
        let observed = ObservedTopology::from_facets([WishFacet::crate_("present_crate")]);
        let a = assess_wish(&w, &observed, d(b"ev"));
        assert_eq!(a.met_count, 1);
        assert_eq!(a.total_count, 2);
        assert!(a.unmet_facets.contains(&WishFacet::crate_("absent_crate")));
    }

    // ── dependency facets (Run 10) ────────────────────────────────────────

    #[test]
    fn facets_from_snapshot_includes_dependency_edges() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "a".to_string(),
            CrateFingerprint::new("a".into(), vec!["lib.rs".into()], vec!["b".into()]),
        );
        nodes.insert(
            "b".to_string(),
            CrateFingerprint::new("b".into(), vec!["lib.rs".into()], vec![]),
        );
        let mut edges = BTreeSet::new();
        edges.insert(("a".to_string(), "b".to_string()));
        let snap = TopologySnapshot::from_parts(ParseBackScanScope::FullWorkspace, nodes, edges);
        let f = facets_from_snapshot(&snap);
        assert!(f.contains(&WishFacet::dependency("a", "b")));
        assert!(f.contains(&WishFacet::crate_("a")));
    }

    #[test]
    fn compile_wish_extracts_dependency() {
        let w = compile_wish("a dependency alpha->beta", d(b"p"), d(b"e"));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::dependency("alpha", "beta")));
    }

    // ── signature facets (Run 11) ─────────────────────────────────────────

    #[test]
    fn facets_from_source_emits_signature_for_pub_fn() {
        let f = facets_from_source("pub fn handle(a: u32, b: u32) -> u32 { a + b }\n");
        assert!(f.contains(&WishFacet::symbol("handle")));
        assert!(f.contains(&WishFacet::signature("handle/2")));
    }

    #[test]
    fn facets_from_source_signature_zero_arity() {
        let f = facets_from_source("pub fn now() {}\n");
        assert!(f.contains(&WishFacet::signature("now/0")));
    }

    #[test]
    fn facets_from_source_emits_typed_contract() {
        let f = facets_from_source("pub fn handle(req: Request) -> Response {}\n");
        assert!(f.contains(&WishFacet::contract("handle", &["Request"], "Response")));
        // The structural cousins are still emitted — no regression.
        assert!(f.contains(&WishFacet::symbol("handle")));
        assert!(f.contains(&WishFacet::signature("handle/1")));
    }

    #[test]
    fn facets_from_source_contract_defaults_unit_return() {
        let f = facets_from_source("pub fn tick() {}\n");
        assert!(f.contains(&WishFacet::contract("tick", &[], "()")));
    }

    #[test]
    fn facets_from_source_contract_captures_generic_and_multi_arg() {
        let f = facets_from_source("pub fn count(items: Vec<User>, max: usize) -> usize {}\n");
        assert!(f.contains(&WishFacet::contract(
            "count",
            &["Vec<User>", "usize"],
            "usize"
        )));
    }

    #[test]
    fn facets_from_source_contract_skips_self_receiver() {
        let f = facets_from_source("pub fn name(&self, id: u64) -> String {}\n");
        assert!(f.contains(&WishFacet::contract("name", &["u64"], "String")));
    }

    #[test]
    fn facets_from_source_observes_scaffold_form_for_roundtrip() {
        // The exact shape the contract scaffold emits must observe back to the
        // same key — this is the scaffold→observe round-trip that makes the
        // descent converge.
        let f = facets_from_source("pub fn add(_a0: i32, _a1: i32) -> i32 { todo!(\"x\") }\n");
        assert!(f.contains(&WishFacet::contract("add", &["i32", "i32"], "i32")));
    }

    #[test]
    fn contract_trigger_compiles_space_free_form() {
        let w = compile_wish(
            "a contract handle(Request)->Response",
            Digest::ZERO,
            Digest::ZERO,
        );
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::contract("handle", &["Request"], "Response")));
    }

    // ── Behaviour (the keystone) ──────────────────────────────────────────

    #[test]
    fn behavior_specs_pair_marker_with_test_name() {
        let src = "// kosmo:behavior: add(2,3)=>5\n#[test]\nfn kosmo_spec_abc() { assert_eq!(add(2,3),5); }\n";
        let pairs = behavior_specs_from_source(src);
        assert_eq!(
            pairs,
            vec![("kosmo_spec_abc".to_string(), "add(2,3)=>5".to_string())]
        );
    }

    #[test]
    fn behavior_marker_without_test_is_not_paired() {
        // A marker not immediately followed by a test fn pairs with nothing.
        let src = "// kosmo:behavior: add(2,3)=>5\npub fn add(a: i32, b: i32) -> i32 { a + b }\n";
        assert!(behavior_specs_from_source(src).is_empty());
    }

    #[test]
    fn behavior_facets_are_failclosed() {
        let specs = vec![
            ("kosmo_spec_green".to_string(), "add(2,3)=>5".to_string()),
            ("kosmo_spec_red".to_string(), "add(2,3)=>6".to_string()),
        ];
        let mut passing = BTreeSet::new();
        passing.insert("kosmo_spec_green".to_string()); // only the green one passed
        let facets = behavior_facets(&specs, &passing);
        assert!(facets.contains(&WishFacet::behavior("add(2,3)=>5")));
        assert!(
            !facets.contains(&WishFacet::behavior("add(2,3)=>6")),
            "a red spec-test must NOT satisfy its behaviour facet"
        );
    }

    #[test]
    fn behavior_facets_empty_when_nothing_passes() {
        let specs = vec![("kosmo_spec_x".to_string(), "f()=>1".to_string())];
        assert!(behavior_facets(&specs, &BTreeSet::new()).is_empty());
    }

    #[test]
    fn behavior_trigger_compiles_space_free_form() {
        let w = compile_wish("a behavior add(2,3)=>5", Digest::ZERO, Digest::ZERO);
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::behavior("add(2,3)=>5")));
    }

    #[test]
    fn deep_observation_does_not_emit_behaviour() {
        // Lexical observation cannot know whether a test passes, so it never
        // emits a Behavior facet — only validated observation does.
        let facets =
            facets_from_source("// kosmo:behavior: add(2,3)=>5\n#[test]\nfn kosmo_spec_abc() {}\n");
        assert!(!facets.iter().any(|f| f.kind == WishFacetKind::Behavior));
    }

    // ── Python observation (polyglot parity) ──────────────────────────────

    #[test]
    fn python_dir_observes_modules_symbols_docs_and_tests() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-pyobs-{nanos}"));
        std::fs::create_dir_all(root.join("pkg")).unwrap();
        std::fs::write(
            root.join("calc.py"),
            "\"\"\"calc module.\"\"\"\n\n\ndef greet(name):\n    \"\"\"Say hi.\"\"\"\n    return name\n\n\nclass Engine:\n    pass\n\n\ndef test_greet():\n    assert greet(\"x\")\n",
        )
        .unwrap();
        std::fs::write(root.join("pkg/__init__.py"), "def boot():\n    pass\n").unwrap();

        let facets = facets_from_python_dir(&root);
        assert!(facets.contains(&WishFacet::module("calc")), "{facets:?}");
        assert!(
            facets.contains(&WishFacet::module("pkg")),
            "__init__.py stands for its package: {facets:?}"
        );
        assert!(facets.contains(&WishFacet::symbol("greet")));
        assert!(facets.contains(&WishFacet::signature("greet/1")));
        assert!(facets.contains(&WishFacet::symbol("Engine")));
        assert!(facets.contains(&WishFacet::test("test_greet")));
        assert!(facets.contains(&WishFacet::doc("calc")), "module docstring");
        assert!(facets.contains(&WishFacet::doc("greet")), "item docstring");
        assert!(
            !facets.contains(&WishFacet::doc("Engine")),
            "an undocumented class earns no Doc facet"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn cargoless_python_workspace_observes_without_cargo_metadata() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-pyws-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("calc.py"), "def greet():\n    return 1\n").unwrap();
        let observed = observe_workspace_deep(&root).expect("the polyglot door observes");
        assert!(observed.contains(&WishFacet::module("calc")));
        assert!(observed.contains(&WishFacet::symbol("greet")));
        assert!(observed.contains(&WishFacet::signature("greet/0")));
        std::fs::remove_dir_all(&root).ok();
    }

    // ── Archetypes (the breadth axis) ─────────────────────────────────────

    #[test]
    fn archetype_crud_expands_to_bundle() {
        let facets = expand_archetype(Archetype::Crud, "user");
        assert_eq!(facets.len(), 4, "crud fans out into four facets");
        assert!(facets.contains(&WishFacet::module("user")));
        assert!(facets.contains(&WishFacet::contract("create_user", &["String"], "String")));
        assert!(facets.contains(&WishFacet::contract("get_user", &["String"], "String")));
        assert!(facets.contains(&WishFacet::capability("crud:user")));
    }

    #[test]
    fn archetype_endpoint_and_component_bundles() {
        let e = expand_archetype(Archetype::Endpoint, "handle");
        assert!(e.contains(&WishFacet::contract("handle", &["String"], "String")));
        assert!(e.contains(&WishFacet::capability("endpoint:handle")));
        let c = expand_archetype(Archetype::Component, "card");
        assert!(c.contains(&WishFacet::module("card")));
        assert!(c.contains(&WishFacet::capability("component:card")));
    }

    #[test]
    fn compile_wish_expands_archetype_to_bundle() {
        // One prose phrase → a four-facet bundle.
        let w = compile_wish("a crud user", Digest::ZERO, Digest::ZERO);
        assert_eq!(w.predicate_count(), 4);
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::module("user")));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::capability("crud:user")));
    }

    #[test]
    fn compile_wish_mixes_archetype_and_leaf() {
        let w = compile_wish("a crud user and a crate api", Digest::ZERO, Digest::ZERO);
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::crate_("api")));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::module("user")));
        assert_eq!(w.predicate_count(), 5, "4 from crud + 1 crate");
    }

    // ── Norm catalog (learned archetypes in the grammar) ───────────────────

    fn armed_norm(name: &str, trigger: &str) -> Norm {
        use kosmo_hyphae::{NormFacetTemplate, NormLevel, NormOrigin};
        Norm::new(
            name,
            "a learned loader organ",
            NormLevel::Molecule,
            "module+symbol",
            [
                NormFacetTemplate::new(WishFacetKind::Module, "{name}"),
                NormFacetTemplate::new(WishFacetKind::Symbol, "{name}_load"),
            ],
            [],
            [],
            Digest::of_bytes(b"cand"),
            Digest::of_bytes(b"ev"),
            Digest::of_bytes(b"pol"),
            NormOrigin::OperatorInjected {
                source: "test".into(),
            },
        )
        .with_trigger(trigger)
    }

    /// The Phase-4 pin: with an empty catalog, `compile_wish_with_norms` is
    /// byte-identical to the untouched `compile_wish` — same predicates, same
    /// content-addressed wish id — across the grammar's whole register.
    #[test]
    fn empty_catalog_compiles_byte_identically() {
        let corpus = [
            "a crate kosmo-api and a function handle",
            "a crud user, an endpoint login and a module billing",
            "docs for helper, a test parser_works",
            "a behavior add(2,3)=>5 and a contract route(String)->String",
            "completely free prose without any trigger at all",
            "",
        ];
        let empty = NormCatalog::empty();
        for prose in corpus {
            let plain = compile_wish(prose, Digest::ZERO, Digest::of_bytes(b"e"));
            let normed =
                compile_wish_with_norms(prose, &empty, Digest::ZERO, Digest::of_bytes(b"e"));
            assert_eq!(plain.id, normed.id, "wish id drifted for: {prose}");
            assert_eq!(plain, normed);
        }
    }

    #[test]
    fn promoted_trigger_expands_like_an_archetype() {
        let catalog = NormCatalog::from_norms([&armed_norm("loader", "loader")]).unwrap();
        let w = compile_wish_with_norms(
            "a loader gamma and a crate api",
            &catalog,
            Digest::ZERO,
            Digest::ZERO,
        );
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::module("gamma")));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::symbol("gamma_load")));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::crate_("api")));
        assert_eq!(w.predicate_count(), 3, "2 from the norm + 1 crate leaf");

        // Fillers between trigger and name are skipped, like everywhere else.
        let w2 = compile_wish_with_norms(
            "a loader called delta",
            &catalog,
            Digest::ZERO,
            Digest::ZERO,
        );
        assert!(w2
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::symbol("delta_load")));
    }

    #[test]
    fn unarmed_norms_stay_inert_in_the_catalog() {
        use kosmo_hyphae::{NormFacetTemplate, NormLevel, NormOrigin};
        let unarmed = Norm::new(
            "sleeper",
            "",
            NormLevel::Atom,
            "module",
            [NormFacetTemplate::new(WishFacetKind::Module, "{name}")],
            [],
            [],
            Digest::of_bytes(b"c"),
            Digest::of_bytes(b"e"),
            Digest::of_bytes(b"p"),
            NormOrigin::Learned {
                observation_ids: vec![Digest::of_bytes(b"o")],
            },
        );
        let catalog = NormCatalog::from_norms([&unarmed]).unwrap();
        assert!(
            catalog.is_empty(),
            "trigger=None norms never enter the grammar"
        );
        let w = compile_wish_with_norms("a sleeper x", &catalog, Digest::ZERO, Digest::ZERO);
        assert_eq!(w.predicate_count(), 0, "an unpromoted norm expands nothing");
    }

    #[test]
    fn catalog_rejects_reserved_and_duplicate_triggers() {
        // A norm may not shadow the built-in grammar ("module" is a leaf
        // trigger; "crud" is an archetype; "called" is a filler).
        for reserved in ["module", "crud", "called"] {
            assert!(is_reserved_wish_word(reserved), "{reserved}");
            let err = NormCatalog::from_norms([&armed_norm("x", reserved)]).unwrap_err();
            assert!(
                matches!(err, NormCatalogError::ReservedTrigger { .. }),
                "{reserved}: {err}"
            );
        }
        assert!(!is_reserved_wish_word("loader"));

        let a = armed_norm("a", "loader");
        let b = armed_norm("b", "loader");
        assert!(matches!(
            NormCatalog::from_norms([&a, &b]).unwrap_err(),
            NormCatalogError::DuplicateTrigger { .. }
        ));
    }

    #[test]
    fn catalog_revalidates_armed_norms() {
        // A tampered norm (id no longer matches) is refused at the catalog
        // door — the grammar never executes an unverified artifact.
        let mut tampered = armed_norm("t", "loader");
        tampered.description = "edited after hashing".into();
        assert!(matches!(
            NormCatalog::from_norms([&tampered]).unwrap_err(),
            NormCatalogError::InvalidNorm { .. }
        ));
    }

    // ── Crate-qualified observation (crate-targeting round-trip) ───────────

    #[test]
    fn manifest_package_name_reads_package_section() {
        assert_eq!(
            manifest_package_name("[package]\nname = \"kosmo-api\"\nversion = \"0.1.0\"\n"),
            Some("kosmo-api".to_string())
        );
        // A workspace-only manifest has no package name.
        assert_eq!(
            manifest_package_name("[workspace]\nmembers = [\"a\"]\n"),
            None
        );
    }

    #[test]
    fn rust_dir_emits_crate_qualified_facets() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-intent-cratequal-{nanos}"));
        std::fs::create_dir_all(root.join("crates/api/src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/api\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("crates/api/Cargo.toml"),
            "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(root.join("crates/api/src/lib.rs"), "pub fn handle() {}\n").unwrap();

        let facets = facets_from_rust_dir(&root);
        // Both the bare facet and the crate-qualified facet are present.
        assert!(
            facets.contains(&WishFacet::symbol("handle")),
            "bare missing"
        );
        assert!(
            facets.contains(&WishFacet::symbol("handle@api")),
            "crate-qualified missing"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    // ── Composition (typed data-flow, level 3) ────────────────────────────

    #[test]
    fn derive_compositions_chains_contracts_by_type() {
        let mut facets = BTreeSet::new();
        facets.insert(WishFacet::contract("parse", &["String"], "Ast"));
        facets.insert(WishFacet::contract("eval", &["Ast"], "i32"));
        facets.insert(WishFacet::contract("unrelated", &["bool"], "bool"));
        let comps = derive_compositions(&facets);
        assert!(comps.contains(&WishFacet::composition("parse", "Ast", "eval")));
        assert!(
            !comps.iter().any(|c| c.key.contains("unrelated")),
            "no spurious composition"
        );
    }

    #[test]
    fn derive_compositions_ignores_unit_return() {
        let mut facets = BTreeSet::new();
        facets.insert(WishFacet::contract("src", &[], "()"));
        facets.insert(WishFacet::contract("sink", &["()"], "()"));
        // Unit must not chain, or everything would compose with everything.
        assert!(derive_compositions(&facets).is_empty());
    }

    #[test]
    fn composition_trigger_compiles() {
        let w = compile_wish("a composition parse>>Ast>>eval", Digest::ZERO, Digest::ZERO);
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::composition("parse", "Ast", "eval")));
    }

    #[test]
    fn flow_trigger_compiles_to_piped_behavior() {
        // Behavioural composition rides the Behavior facet: the wire is in the
        // call, validated green by the same judge.
        let w = compile_wish("a flow parse(\"2+3\")>>eval=>5", Digest::ZERO, Digest::ZERO);
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::behavior("parse(\"2+3\")>>eval=>5")));
    }

    // ── Runtime probes (level 5 — execute and observe) ────────────────────

    #[test]
    fn parse_run_key_splits_args_and_expect() {
        let (args, expect) = parse_run_key("add,2,3=>out~5").unwrap();
        assert_eq!(args, vec!["add", "2", "3"]);
        assert_eq!(expect.stdout_contains.as_deref(), Some("5"));
        assert!(parse_run_key("noarrow").is_none());
    }

    #[test]
    fn parse_run_expect_forms() {
        assert_eq!(
            parse_run_expect("exit:0"),
            Some(RunExpect {
                exit: Some(0),
                stdout_contains: None,
                max_ms: None
            })
        );
        assert_eq!(
            parse_run_expect("out~hi"),
            Some(RunExpect {
                exit: None,
                stdout_contains: Some("hi".into()),
                max_ms: None
            })
        );
        assert_eq!(
            parse_run_expect("exit:0,out~hi"),
            Some(RunExpect {
                exit: Some(0),
                stdout_contains: Some("hi".into()),
                max_ms: None
            })
        );
        assert!(parse_run_expect("garbage").is_none());
    }

    #[test]
    fn parse_run_expect_budget_atom_anchors_at_the_tail() {
        // The first quality axis: speed as a measurable expectation.
        assert_eq!(
            parse_run_expect("out~hi,ms<50"),
            Some(RunExpect {
                exit: None,
                stdout_contains: Some("hi".into()),
                max_ms: Some(50)
            })
        );
        assert_eq!(
            parse_run_expect("exit:0,out~a,b,ms<200"),
            Some(RunExpect {
                exit: Some(0),
                stdout_contains: Some("a,b".into()),
                max_ms: Some(200)
            }),
            "the out~ substring keeps its commas; the budget anchors at the tail"
        );
        assert_eq!(
            parse_run_expect("ms<10"),
            Some(RunExpect {
                exit: None,
                stdout_contains: None,
                max_ms: Some(10)
            }),
            "a lone budget is a valid expectation (clean exit within budget)"
        );
        // Malformed budgets fail the whole key — never silently ignored.
        assert!(parse_run_expect("out~hi,ms<fast").is_none());
    }

    #[test]
    fn run_matches_enforces_the_budget_failclosed() {
        use kosmo_sandbox::{RunVerdict, RuntimeWitness};
        let e = parse_run_expect("out~hi,ms<50").unwrap();
        let witness = |ms: u64| RuntimeWitness {
            verdict: RunVerdict::Exited,
            exit_code: Some(0),
            stdout: "hi".into(),
            stderr: String::new(),
            stdout_digest: Digest::ZERO,
            duration: std::time::Duration::from_millis(ms),
            truncated: false,
        };
        assert!(run_matches(&witness(10), &e), "within budget matches");
        assert!(
            !run_matches(&witness(80), &e),
            "a blown budget is a miss, never a warning"
        );
    }

    #[test]
    fn run_marker_reads_probe() {
        assert_eq!(
            run_marker("// kosmo:run: add,2,3=>out~5"),
            Some("add,2,3=>out~5".to_string())
        );
        assert_eq!(run_marker("let x = 1;"), None);
    }

    #[test]
    fn run_matches_is_failclosed_on_nonexit() {
        use kosmo_sandbox::{RunVerdict, RuntimeWitness};
        let e = RunExpect {
            exit: Some(0),
            stdout_contains: Some("5".into()),
            max_ms: None,
        };
        // A timed-out run never matches — even with the right stdout printed.
        let timed = RuntimeWitness {
            verdict: RunVerdict::TimedOut,
            exit_code: None,
            stdout: "5".into(),
            stderr: String::new(),
            stdout_digest: Digest::ZERO,
            duration: std::time::Duration::ZERO,
            truncated: false,
        };
        assert!(!run_matches(&timed, &e));
        // A clean matching exit does match.
        let ok = RuntimeWitness {
            verdict: RunVerdict::Exited,
            exit_code: Some(0),
            stdout: "answer=5".into(),
            stderr: String::new(),
            stdout_digest: Digest::ZERO,
            duration: std::time::Duration::ZERO,
            truncated: false,
        };
        assert!(run_matches(&ok, &e));
    }

    #[test]
    fn run_trigger_compiles() {
        let w = compile_wish("a run add,2,3=>out~5", Digest::ZERO, Digest::ZERO);
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::run("add,2,3=>out~5")));
    }

    // ── Service probes (level 6 — start a server and probe it) ────────────

    #[test]
    fn parse_service_key_splits_method_path_expect() {
        let (probe, expect) = parse_service_key("GET:/health=>200").unwrap();
        assert_eq!(probe.method, "GET");
        assert_eq!(probe.path, "/health");
        assert_eq!(expect.status, Some(200));
        assert!(parse_service_key("noarrow").is_none());
        assert!(
            parse_service_key("GET:=>200").is_none(),
            "empty path rejected"
        );
    }

    #[test]
    fn parse_service_expect_forms() {
        assert_eq!(
            parse_service_expect("200"),
            Some(ServiceExpect {
                status: Some(200),
                body_contains: None
            })
        );
        assert_eq!(
            parse_service_expect("200,body~ok"),
            Some(ServiceExpect {
                status: Some(200),
                body_contains: Some("ok".into())
            })
        );
        assert!(parse_service_expect("xyz").is_none());
    }

    #[test]
    fn service_marker_reads_only_service_probes() {
        assert_eq!(
            service_marker("// kosmo:service: GET:/health=>200"),
            Some("GET:/health=>200".to_string())
        );
        assert_eq!(service_marker("// kosmo:run: a=>b"), None);
    }

    #[test]
    fn service_matches_is_failclosed_on_never_ready() {
        use kosmo_sandbox::{ServiceVerdict, ServiceWitness};
        let e = ServiceExpect {
            status: Some(200),
            body_contains: None,
        };
        // A server that never came up never matches.
        let never = ServiceWitness {
            verdict: ServiceVerdict::NeverReady,
            status: None,
            body: String::new(),
            body_digest: Digest::ZERO,
            startup: std::time::Duration::ZERO,
        };
        assert!(!service_matches(&never, &e));
        let ok = ServiceWitness {
            verdict: ServiceVerdict::Probed,
            status: Some(200),
            body: "ok".into(),
            body_digest: Digest::ZERO,
            startup: std::time::Duration::ZERO,
        };
        assert!(service_matches(&ok, &e));
    }

    #[test]
    fn service_trigger_compiles() {
        let w = compile_wish("a service GET:/health=>200", Digest::ZERO, Digest::ZERO);
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::service("GET:/health=>200")));
    }

    #[test]
    fn rust_dir_derives_composition_from_source() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kosmo-intent-comp-{nanos}"));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            "pub struct Ast;\npub fn parse(s: String) -> Ast { Ast }\npub fn eval(a: Ast) -> i32 { 0 }\n",
        )
        .unwrap();
        let facets = facets_from_rust_dir(&root);
        assert!(
            facets.contains(&WishFacet::composition("parse", "Ast", "eval")),
            "parse->Ast->eval should be derived"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn facets_from_source_signature_ignores_generics_in_arity() {
        let f = facets_from_source("pub fn map(x: Vec<u32>) {}\n");
        assert!(f.contains(&WishFacet::signature("map/1")));
    }

    // ── capability facets (Run 12) ────────────────────────────────────────

    #[test]
    fn facets_from_source_extracts_capability_marker() {
        let f = facets_from_source(
            "//! crate doc\n// kosmo:capability: http-server\npub fn run() {}\n",
        );
        assert!(f.contains(&WishFacet::capability("http-server")));
    }

    #[test]
    fn compile_wish_extracts_capability() {
        let w = compile_wish("a capability http-server", d(b"p"), d(b"e"));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::capability("http-server")));
    }

    // ── test facets (Run 13) ──────────────────────────────────────────────

    #[test]
    fn facets_from_source_extracts_test() {
        let f = facets_from_source("#[test]\nfn it_works() { assert!(true); }\n");
        assert!(f.contains(&WishFacet::test("it_works")));
    }

    #[test]
    fn facets_from_source_extracts_namespaced_test() {
        let f = facets_from_source("#[tokio::test]\nasync fn runs() {}\n");
        assert!(f.contains(&WishFacet::test("runs")));
    }

    #[test]
    fn compile_wish_extracts_test() {
        let w = compile_wish("a test it_works", d(b"p"), d(b"e"));
        assert!(w
            .predicates
            .iter()
            .any(|p| p.facet == WishFacet::test("it_works")));
    }

    // ── validated observation: green tests (Run 14) ───────────────────────

    #[test]
    fn parse_test_results_reads_ok_and_failed() {
        let out = "running 3 tests\n\
                   test it_fails ... FAILED\n\
                   test it_works ... ok\n\
                   test skip_me ... ignored\n\n\
                   test result: FAILED. 1 passed; 1 failed; 1 ignored;\n";
        let r = parse_test_results(out);
        assert_eq!(r.get("it_works"), Some(&true));
        assert_eq!(r.get("it_fails"), Some(&false));
        assert!(!r.contains_key("skip_me"), "ignored tests are not verdicts");
        assert!(!r.contains_key("result:"), "the summary line is skipped");
    }

    #[test]
    fn passing_test_facets_only_green_keyed_by_bare_name() {
        let mut r = BTreeMap::new();
        r.insert("tests::alpha".to_string(), true);
        r.insert("tests::beta".to_string(), false);
        let f = passing_test_facets(&r);
        assert!(f.contains(&WishFacet::test("alpha")));
        assert!(!f.contains(&WishFacet::test("beta")));
    }

    #[test]
    fn run_workspace_tests_distinguishes_green_from_red() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-intent-validated-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"validated_demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            "#[test]\nfn green() { assert_eq!(1 + 1, 2); }\n#[test]\nfn red() { assert!(false); }\n",
        )
        .unwrap();

        match run_workspace_tests(&dir) {
            Ok(results) if !results.is_empty() => {
                let facets = passing_test_facets(&results);
                assert!(
                    facets.contains(&WishFacet::test("green")),
                    "green test passes"
                );
                assert!(
                    !facets.contains(&WishFacet::test("red")),
                    "red test fails → not a facet"
                );
            }
            _ => eprintln!("cargo test unavailable, skipping"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    // ─── Doc facets: documented items are observable ─────────────────────────

    #[test]
    fn facets_from_source_extracts_documented_items() {
        let src = "/// Adds two numbers.
pub fn add(a: u32, b: u32) -> u32 { a + b }

pub fn bare() {}

/// A documented type…
#[derive(Debug)]
pub struct Routed;

#[doc = \"attr-style docs\"]
pub fn attr_documented() {}

/// docs on a private item observe nothing
fn private_helper() {}
";
        let f = facets_from_source(src);
        assert!(f.contains(&WishFacet::doc("add")), "/// above pub fn");
        assert!(
            f.contains(&WishFacet::doc("Routed")),
            "attributes between doc and item are fine"
        );
        assert!(
            f.contains(&WishFacet::doc("attr_documented")),
            "#[doc = …] counts"
        );
        assert!(
            !f.contains(&WishFacet::doc("bare")),
            "undocumented pub item has no Doc facet"
        );
        assert!(
            !f.contains(&WishFacet::doc("private_helper")),
            "private items observe no Doc facet"
        );
        // The Symbol facets remain untouched alongside.
        assert!(f.contains(&WishFacet::symbol("bare")));
    }

    #[test]
    fn compile_wish_extracts_doc_facets() {
        for prose in [
            "docs of helper",
            "docs for helper",
            "documented helper",
            "documentation for helper",
        ] {
            let w = compile_wish(prose, d(b"policy"), d(b"bundle"));
            assert!(
                w.predicates
                    .iter()
                    .any(|p| p.facet == WishFacet::doc("helper")),
                "{prose:?} must compile to a Doc facet, got {:?}",
                w.predicates
            );
        }
    }
}
