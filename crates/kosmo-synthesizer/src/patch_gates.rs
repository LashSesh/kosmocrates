//! Mikro/Meso patch gates — assimilated from Wonderlamp's `gates.rs`,
//! made fail-closed on the epistemic substrate.
//!
//! Wonderlamp's gates were informational: violations logged, code emitted
//! anyway. Here they are real gates over [`kosmo_core::GateResult`] with
//! worst-wins merging; the [`ContextualSynthesizer`](crate::ContextualSynthesizer)
//! delivers a merged `Reject` as `confidence = ZERO` with a
//! `gate-reject:` rationale — auditable and unmaterializable through the
//! existing `min_confidence` policy filter.
//!
//! The checks judge **filesystem truth and descent knowledge**, never an
//! upfront plan (Wonderlamp resolved imports against its HDAG layer plan —
//! a template in disguise):
//!
//! Mikro (per file change):
//! - **NonEmpty** — a Create/Modify with empty content is a `Reject`.
//! - **TopologyConformance** — Create over an existing path, or Modify of a
//!   missing path, is a `Reject` (the patch lies about the workspace).
//! - **ParseExtract** — a recognized source file whose extraction yields no
//!   structure at all is a `Warn` (suspicious, not forbidden). Unrecognized
//!   extensions are skipped: not everything is code, and fail-closed must
//!   not become over-closed.
//!
//! Meso (per patch):
//! - **DuplicateDefinition** — the same function key defined in two files
//!   of one patch is a `Reject`.
//! - **DescentImportIntegrity** — a file in this patch imports a module the
//!   descent created and this same patch deletes: `Reject`
//!   (use-after-delete). External imports are not judged — there is no
//!   manifest model here and pretending otherwise would be dishonest.
//! - **OriginShift** — redefining a symbol the descent context knows from a
//!   different origin is a `Warn` (legitimate refactors exist).

use std::collections::BTreeMap;
use std::path::Path;

use kosmo_core::GateResult;
use kosmo_hyphae::xlang::{symbol_sets_auto, SourceLanguage};

use crate::context::TypeContext;
use crate::{FileChangeKind, Patch};

/// Worst-wins merge: Reject > Warn > Pass (Downgrade treated as Warn-class;
/// these gates never emit it).
fn merge(a: GateResult, b: GateResult) -> GateResult {
    match (&a, &b) {
        (GateResult::Reject { .. }, _) => a,
        (_, GateResult::Reject { .. }) => b,
        (GateResult::Warn { .. }, _) => a,
        (_, GateResult::Warn { .. }) => b,
        _ => a,
    }
}

/// Run every Mikro and Meso check; return the worst-wins merged verdict.
/// `workspace` is the root the patch claims to describe (read-only).
pub fn gate_patch(patch: &Patch, workspace: &Path, ctx: &TypeContext) -> GateResult {
    let mut verdict = GateResult::Pass;

    // ── Mikro: per file change ───────────────────────────────────────────
    for fc in &patch.file_changes {
        let location = fc.path.to_string_lossy().to_string();
        let on_disk = workspace.join(&fc.path);
        match fc.kind {
            FileChangeKind::Create => {
                if fc.content.trim().is_empty() {
                    verdict = merge(
                        verdict,
                        GateResult::Reject {
                            reason: format!("empty create: {location}"),
                        },
                    );
                }
                if on_disk.exists() {
                    verdict = merge(
                        verdict,
                        GateResult::Reject {
                            reason: format!("create over existing path: {location}"),
                        },
                    );
                }
            }
            FileChangeKind::Modify => {
                if fc.content.trim().is_empty() {
                    verdict = merge(
                        verdict,
                        GateResult::Reject {
                            reason: format!("empty modify: {location}"),
                        },
                    );
                }
                if !on_disk.exists() {
                    verdict = merge(
                        verdict,
                        GateResult::Reject {
                            reason: format!("modify of missing path: {location}"),
                        },
                    );
                }
            }
            FileChangeKind::Delete => continue,
        }
        if SourceLanguage::from_path(&location).is_some() {
            if let Some(sets) = symbol_sets_auto(&location, &fc.content) {
                if sets.total == 0 && !fc.content.trim().is_empty() {
                    verdict = merge(
                        verdict,
                        GateResult::Warn {
                            message: format!("no structure extracted from {location}"),
                        },
                    );
                }
            }
        }
    }

    // ── Meso: across the patch ───────────────────────────────────────────
    // Duplicate function definitions across files.
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for fc in &patch.file_changes {
        if matches!(fc.kind, FileChangeKind::Delete) {
            continue;
        }
        let location = fc.path.to_string_lossy().to_string();
        let Some(sets) = symbol_sets_auto(&location, &fc.content) else {
            continue;
        };
        for f in &sets.functions {
            if let Some(prev) = seen.get(f) {
                if prev != &location {
                    verdict = merge(
                        verdict,
                        GateResult::Reject {
                            reason: format!("duplicate definition of {f} in {prev} and {location}"),
                        },
                    );
                }
            } else {
                seen.insert(f.clone(), location.clone());
            }
        }
        // OriginShift: the descent knows this symbol from elsewhere.
        for key in sets.functions.iter().chain(sets.types.iter()) {
            if let Some(origin) = ctx.origin_of(key) {
                if origin != &fc.path {
                    verdict = merge(
                        verdict,
                        GateResult::Warn {
                            message: format!(
                                "{key} redefined in {location} (descent origin: {})",
                                origin.display()
                            ),
                        },
                    );
                }
            }
        }
    }

    // DescentImportIntegrity: import of a descent-created module that this
    // very patch deletes.
    let deleted_stems: Vec<String> = patch
        .file_changes
        .iter()
        .filter(|fc| matches!(fc.kind, FileChangeKind::Delete))
        .filter_map(|fc| {
            fc.path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
        })
        .filter(|stem| ctx.has_module(stem))
        .collect();
    if !deleted_stems.is_empty() {
        for fc in &patch.file_changes {
            if matches!(fc.kind, FileChangeKind::Delete) {
                continue;
            }
            let location = fc.path.to_string_lossy().to_string();
            let Some(sets) = symbol_sets_auto(&location, &fc.content) else {
                continue;
            };
            for import in &sets.imports {
                if let Some(stem) = deleted_stems
                    .iter()
                    .find(|stem| import_names_module(import, stem))
                {
                    verdict = merge(
                        verdict,
                        GateResult::Reject {
                            reason: format!(
                                "{location} imports {import}, but this patch deletes \
                                 the descent-created module {stem}"
                            ),
                        },
                    );
                }
            }
        }
    }

    verdict
}

/// Does an import path name the module `stem` as one of its segments?
fn import_names_module(import: &str, stem: &str) -> bool {
    import
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|segment| segment == stem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileChange;

    fn ws() -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kosmo-gates-{nanos}"));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/existing.rs"), "pub fn here() {}\n").unwrap();
        dir
    }

    fn patch(changes: Vec<FileChange>) -> Patch {
        Patch::new(kosmo_core::Digest::ZERO, changes, "gate-test")
    }

    fn reject_reason(v: &GateResult) -> &str {
        match v {
            GateResult::Reject { reason } => reason,
            other => panic!("expected Reject, got {other:?}"),
        }
    }

    #[test]
    fn clean_create_passes() {
        let dir = ws();
        let v = gate_patch(
            &patch(vec![FileChange::create(
                "src/new.rs",
                "pub fn fresh() {}\n",
            )]),
            &dir,
            &TypeContext::new(),
        );
        assert_eq!(v, GateResult::Pass);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn topology_lies_are_rejected() {
        let dir = ws();
        let over = gate_patch(
            &patch(vec![FileChange::create(
                "src/existing.rs",
                "pub fn x() {}\n",
            )]),
            &dir,
            &TypeContext::new(),
        );
        assert!(reject_reason(&over).contains("create over existing"));

        let ghost = gate_patch(
            &patch(vec![FileChange::modify("src/ghost.rs", "pub fn x() {}\n")]),
            &dir,
            &TypeContext::new(),
        );
        assert!(reject_reason(&ghost).contains("modify of missing"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_content_is_rejected_and_structurelessness_warns() {
        let dir = ws();
        let empty = gate_patch(
            &patch(vec![FileChange::create("src/new.rs", "   \n")]),
            &dir,
            &TypeContext::new(),
        );
        assert!(reject_reason(&empty).contains("empty create"));

        let prose = gate_patch(
            &patch(vec![FileChange::create(
                "src/odd.rs",
                "// only a comment\n",
            )]),
            &dir,
            &TypeContext::new(),
        );
        assert!(matches!(prose, GateResult::Warn { .. }), "{prose:?}");

        // Non-code files are skipped, not judged.
        let toml = gate_patch(
            &patch(vec![FileChange::create("Cargo.toml", "[package]\n")]),
            &dir,
            &TypeContext::new(),
        );
        assert_eq!(toml, GateResult::Pass);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn duplicate_definitions_across_files_are_rejected() {
        let dir = ws();
        let v = gate_patch(
            &patch(vec![
                FileChange::create("src/a.rs", "pub fn twice() {}\n"),
                FileChange::create("src/b.rs", "pub fn twice() {}\n"),
            ]),
            &dir,
            &TypeContext::new(),
        );
        assert!(reject_reason(&v).contains("duplicate definition of twice/0"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn use_after_delete_of_descent_module_is_rejected() {
        let dir = ws();
        let mut ctx = TypeContext::new();
        ctx.absorb_patch(&patch(vec![FileChange::create(
            "src/router.rs",
            "pub fn route() {}\n",
        )]));

        let v = gate_patch(
            &patch(vec![
                FileChange::delete("src/router.rs"),
                FileChange::create("src/uses.rs", "use crate::router::route;\npub fn go() {}\n"),
            ]),
            &dir,
            &ctx,
        );
        assert!(reject_reason(&v).contains("deletes the descent-created module router"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn origin_shift_warns_but_does_not_reject() {
        let dir = ws();
        let mut ctx = TypeContext::new();
        ctx.absorb_patch(&patch(vec![FileChange::create(
            "src/router.rs",
            "pub fn route() {}\n",
        )]));
        let v = gate_patch(
            &patch(vec![FileChange::create(
                "src/other.rs",
                "pub fn route() {}\n",
            )]),
            &dir,
            &ctx,
        );
        assert!(matches!(v, GateResult::Warn { .. }), "{v:?}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
