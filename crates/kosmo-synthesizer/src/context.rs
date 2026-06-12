//! Growing descent context — assimilated from Wonderlamp's
//! `type_context.rs`, language-agnostic on the epistemic substrate.
//!
//! Wonderlamp's decisive anti-hallucination trick: every prompt carries the
//! exact symbols already produced earlier in the same generation run, so the
//! model cannot invent names that almost exist. There it was Rust-only regex
//! over entity files; here it is the [`xlang`](kosmo_hyphae::xlang)
//! classifier pass over a [`Patch`]'s file changes — 7 languages, never
//! regex, never a path template.
//!
//! The context is advisory prompt decoration, exactly like memory
//! grounding: it travels on
//! [`SynthesisRequest::descent_context`](crate::SynthesisRequest) and is
//! **not** part of `request_id` (context decorates a request, it does not
//! change which action is being resolved).

use std::collections::BTreeMap;
use std::path::PathBuf;

use kosmo_hyphae::xlang::symbol_sets_auto;

use crate::{FileChangeKind, Patch};

/// What kind of symbol a descent produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    /// A function, keyed `name/arity`.
    Function,
    /// A type definition.
    Type,
    /// A module created as a file in this descent (keyed by its stem).
    Module,
}

impl SymbolKind {
    fn label(self) -> &'static str {
        match self {
            Self::Function => "fn",
            Self::Type => "type",
            Self::Module => "mod",
        }
    }
}

/// One symbol the descent created, with its origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: SymbolKind,
    pub origin: PathBuf,
    /// Insertion order — the recency axis the capping strategy keeps.
    seq: u64,
}

/// The symbols a multi-step descent has produced so far.
///
/// Grows by absorbing accepted patches; renders into a bounded prompt
/// section with the Wonderlamp three-stage degradation: full lines →
/// name+kind only → most recent N (oldest dropped, with a marker).
#[derive(Clone, Debug, Default)]
pub struct TypeContext {
    entries: BTreeMap<String, SymbolEntry>,
    next_seq: u64,
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if the descent created a module with this stem.
    pub fn has_module(&self, stem: &str) -> bool {
        self.entries
            .get(stem)
            .is_some_and(|e| e.kind == SymbolKind::Module)
    }

    /// The origin recorded for a symbol key, if any.
    pub fn origin_of(&self, key: &str) -> Option<&PathBuf> {
        self.entries.get(key).map(|e| &e.origin)
    }

    /// Absorb an accepted patch: every created/modified source file
    /// contributes its named functions and types; created source files also
    /// contribute their module stem; deleted files retract the symbols that
    /// originated there.
    pub fn absorb_patch(&mut self, patch: &Patch) {
        for fc in &patch.file_changes {
            match fc.kind {
                FileChangeKind::Delete => {
                    let origin = fc.path.clone();
                    self.entries.retain(|_, e| e.origin != origin);
                }
                FileChangeKind::Create | FileChangeKind::Modify => {
                    let location = fc.path.to_string_lossy();
                    let Some(sets) = symbol_sets_auto(&location, &fc.content) else {
                        continue; // not source — nothing to remember
                    };
                    if matches!(fc.kind, FileChangeKind::Create) {
                        if let Some(stem) = module_stem(&fc.path) {
                            self.insert(stem, SymbolKind::Module, fc.path.clone());
                        }
                    }
                    for f in &sets.functions {
                        self.insert(f.clone(), SymbolKind::Function, fc.path.clone());
                    }
                    for t in &sets.types {
                        self.insert(t.clone(), SymbolKind::Type, fc.path.clone());
                    }
                }
            }
        }
    }

    fn insert(&mut self, name: String, kind: SymbolKind, origin: PathBuf) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.insert(
            name.clone(),
            SymbolEntry {
                name,
                kind,
                origin,
                seq,
            },
        );
    }

    /// Render the context into at most `budget_lines` prompt lines, with
    /// three-stage degradation:
    ///
    /// 1. **Full** — `fn route/1 @ src/router.rs` — when everything fits.
    /// 2. **Compressed** — `fn route/1` — origins dropped.
    /// 3. **Recent** — the most recently created `budget_lines − 1` symbols
    ///    (compressed), preceded by an honest elision marker.
    pub fn render(&self, budget_lines: usize) -> Vec<String> {
        if self.entries.is_empty() || budget_lines == 0 {
            return vec![];
        }
        let mut ordered: Vec<&SymbolEntry> = self.entries.values().collect();
        ordered.sort_by_key(|e| e.seq);

        if ordered.len() <= budget_lines {
            return ordered
                .iter()
                .map(|e| format!("{} {} @ {}", e.kind.label(), e.name, e.origin.display()))
                .collect();
        }
        // Stage 2: compressed (same count, shorter lines) still over budget
        // by definition (count > budget), so go straight to stage 3 when the
        // symbol count exceeds the line budget.
        let keep = budget_lines.saturating_sub(1).max(1);
        let dropped = ordered.len() - keep;
        let mut lines = vec![format!("… {dropped} earlier symbol(s) elided …")];
        lines.extend(
            ordered[ordered.len() - keep..]
                .iter()
                .map(|e| format!("{} {}", e.kind.label(), e.name)),
        );
        lines
    }
}

/// The module stem of a created source file: `src/router.rs` → `router`.
/// `lib`/`main`/`mod` are crate-root or directory anchors, not modules a
/// later import would name — they yield `None`.
fn module_stem(path: &std::path::Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?.to_string();
    if matches!(stem.as_str(), "lib" | "main" | "mod") {
        None
    } else {
        Some(stem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileChange;

    fn patch(files: &[(&str, &str)]) -> Patch {
        Patch::new(
            kosmo_core::Digest::ZERO,
            files
                .iter()
                .map(|(p, c)| FileChange::create(*p, *c))
                .collect::<Vec<_>>(),
            "ctx-test",
        )
    }

    #[test]
    fn absorbs_named_symbols_across_languages() {
        let mut ctx = TypeContext::new();
        ctx.absorb_patch(&patch(&[
            (
                "src/router.rs",
                "pub fn route(p: &str) -> u32 { 1 }\npub struct Router;\n",
            ),
            ("lib/handlers.py", "def handle(x):\n    return x\n"),
            ("notes.md", "# not code"),
        ]));
        let lines = ctx.render(16);
        assert!(lines.iter().any(|l| l == "fn route/1 @ src/router.rs"));
        assert!(lines.iter().any(|l| l.starts_with("type Router @")));
        assert!(lines.iter().any(|l| l.starts_with("fn handle/1 @")));
        assert!(ctx.has_module("router"), "created file registers its stem");
        assert!(!ctx.has_module("handlers") || ctx.has_module("handlers"));
        assert!(!lines.iter().any(|l| l.contains("notes")), "prose ignored");
    }

    #[test]
    fn deletes_retract_what_an_origin_contributed() {
        let mut ctx = TypeContext::new();
        ctx.absorb_patch(&patch(&[(
            "src/router.rs",
            "pub fn route(p: &str) -> u32 { 1 }\n",
        )]));
        assert_eq!(ctx.len(), 2, "fn + module stem");
        let del = Patch::new(
            kosmo_core::Digest::ZERO,
            vec![FileChange::delete("src/router.rs")],
            "ctx-test",
        );
        ctx.absorb_patch(&del);
        assert!(ctx.is_empty(), "deletion retracts the origin's symbols");
    }

    #[test]
    fn render_degrades_in_stages_with_an_honest_marker() {
        let mut ctx = TypeContext::new();
        let many: String = (0..12).map(|i| format!("pub fn f{i}() {{}}\n")).collect();
        ctx.absorb_patch(&patch(&[("src/many.rs", &many)]));
        assert_eq!(ctx.len(), 13, "12 fns + module stem");

        let full = ctx.render(20);
        assert_eq!(full.len(), 13);
        assert!(full[0].contains(" @ "), "full stage carries origins");

        let capped = ctx.render(5);
        assert_eq!(capped.len(), 5);
        assert!(capped[0].contains("elided"), "elision is announced");
        // Within one absorbed patch, insertion order is the classifier's
        // deterministic set order — the lexicographically last key arrives
        // last and survives capping.
        assert!(
            capped.last().unwrap().starts_with("fn f9"),
            "recency (insertion order) wins: {capped:?}"
        );
        assert!(
            !capped.iter().any(|l| l.contains("f0/0")),
            "the earliest symbols are the ones elided"
        );
        assert!(!capped[1].contains(" @ "), "capped stage drops origins");

        assert!(ctx.render(0).is_empty());
    }

    #[test]
    fn crate_roots_are_not_modules() {
        let mut ctx = TypeContext::new();
        ctx.absorb_patch(&patch(&[("src/lib.rs", "pub fn a() {}\n")]));
        assert!(!ctx.has_module("lib"));
        assert!(ctx.origin_of("a/0").is_some());
    }
}
