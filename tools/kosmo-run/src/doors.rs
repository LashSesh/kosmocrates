//! The doors of kosmo-run — this binary's self-description.
//!
//! `--doors` answers the operator's standing question — *what, exactly,
//! could I operate right now?* — from inside the binary itself: every mode
//! is a [`Door`] with its companion inputs, its write power (governance as
//! data) and its needs, content-addressed into one deterministic catalog.
//!
//! The catalog cannot drift from the mechanism: a test scans this binary's
//! own `parse_args` source and asserts that the set of flag literals the
//! parser matches is **exactly** the set of names the catalog speaks —
//! a new flag that isn't cataloged, or a cataloged flag that isn't parsed,
//! fails the build. The description is pinned to the door it describes.

use kosmo_core::{Door, DoorCatalog, DoorGovernance, DoorInput, DoorNeed, DoorSurface};

fn here() -> DoorSurface {
    DoorSurface::Cli {
        binary: "kosmo-run".into(),
    }
}

/// Output shaping, accepted by every door.
fn output_inputs() -> Vec<DoorInput> {
    vec![DoorInput::switch("--json"), DoorInput::switch("--no-color")]
}

/// The synthesis armament shared by every descending door: an optional real
/// provider, model override, consensus ensemble, and anchored memory.
fn armament_inputs() -> Vec<DoorInput> {
    vec![
        DoorInput::valued("--provider", "claude|cerebras|mock|env"),
        DoorInput::valued("--model", "<slug>"),
        DoorInput::valued("--swarm", "<n>"),
        DoorInput::valued("--ledger", "<path>"),
        DoorInput::valued("--ground-top", "<n>"),
    ]
}

fn workspace_input() -> DoorInput {
    DoorInput::valued("path", "<workspace>")
}

/// The complete door inventory of this binary — kosmo-run speaking for
/// itself. Deterministic; the catalog_id changes iff the surface does.
pub fn catalog() -> DoorCatalog {
    let doors = vec![
        Door::new(
            here(),
            "agent",
            vec![],
            "the default door: run the closed agent loop (pipeline → synthesize → \
         validate → materialize → feedback) over a workspace; dry-run unless \
         --apply",
            [
                vec![
                    DoorInput::valued("--max-steps", "<n>"),
                    DoorInput::valued("--min-confidence", "<pct>"),
                    DoorInput::switch("--all"),
                    DoorInput::valued("--capacity", "<n>"),
                    DoorInput::switch("--apply"),
                    DoorInput::switch("--commit"),
                    workspace_input(),
                ],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::WritesWorkspace,
            vec![DoorNeed::Workspace, DoorNeed::Cargo, DoorNeed::Provider],
        ),
        Door::new(
            here(),
            "--wish",
            vec![],
            "compile a plain-prose wish and measure the workspace against it \
         (offline, deterministic); under --apply, descend until realized. \
         --layers renders it as a hypercube whose strata fill from transparent \
         to solid; --staged descends Solve\u{2192}Gate\u{2192}Coagula, coagulating \
         stratum by stratum; --mesh reads the two gears (wish solidity vs. the \
         workspace's observed structural density), surfacing over-fit",
            [
                vec![
                    DoorInput::switch("--validated"),
                    DoorInput::switch("--scaffold"),
                    DoorInput::switch("--layers"),
                    DoorInput::switch("--staged"),
                    DoorInput::switch("--mesh"),
                    DoorInput::switch("--flat"),
                    DoorInput::valued("--wish-session", "<file>"),
                    DoorInput::valued("--since", "<session>"),
                    DoorInput::switch("--apply"),
                    DoorInput::valued("--norms", "<dir>"),
                    workspace_input(),
                ],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::WritesWorkspace,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--pruefstand",
            vec!["--testbench".into()],
            "descend the built-in reference corpus of known-good and broken \
         systems and report fidelity (--validated runs the behavioural \
         suites too); the operator's workspace is never touched",
            [vec![DoorInput::switch("--validated")], output_inputs()].concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--wishlist",
            vec![],
            "measure a file of prose wishes (one per line; # comments and blank \
         lines ignored) against the workspace as a project definition-of-done, \
         into an aggregate realization gauge; read-only unless --apply, which \
         descends every wish to close the project (writes the workspace); exit 0 \
         only when every wish is realized; --since <reading> diffs against a prior \
         --json snapshot and exits 2 on any project regression",
            [
                vec![
                    DoorInput::switch("--validated"),
                    DoorInput::switch("--scaffold"),
                    DoorInput::switch("--apply"),
                    DoorInput::valued("--since", "<reading>"),
                    workspace_input(),
                ],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::WritesWorkspace,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--vocab",
            vec![],
            "print the wish vocabulary \u{2014} the prose forms for each stratum \
         (existence, shape, wiring, verified, live), by example; how to phrase a \
         wish. Needs no workspace",
            output_inputs(),
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "--landscape",
            vec![],
            "project every substrate finding into a ranked wish-proposal \
         landscape with honest standing; --geometry adds the spectral shape; \
         adoption turns a slice into ONE wish (descends under --apply)",
            [
                vec![
                    DoorInput::switch("--geometry"),
                    DoorInput::valued("--adopt", "<n>"),
                    DoorInput::valued("--adopt-cluster", "<i>"),
                    DoorInput::switch("--all"),
                    DoorInput::valued("--capacity", "<n>"),
                    DoorInput::switch("--apply"),
                    DoorInput::valued("--norms", "<dir>"),
                    workspace_input(),
                ],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::WritesWorkspace,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--chat",
            vec![],
            "route one plain utterance onto an existing door (wish / descend / \
         landscape / adopt / status / norm hints); routing is total and never \
         escalates past --apply",
            [
                vec![
                    DoorInput::switch("--all"),
                    DoorInput::valued("--capacity", "<n>"),
                    DoorInput::switch("--apply"),
                    DoorInput::valued("--norms", "<dir>"),
                    workspace_input(),
                ],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::WritesWorkspace,
            vec![DoorNeed::Workspace],
        ),
        Door::new(
            here(),
            "--atelier",
            vec![],
            "shape a durable wish draft over rounds (--chat carries each \
         utterance); machine proposals stay pending until accepted; \
         'realize' descends (writes only under --apply)",
            [
                vec![
                    DoorInput::valued("--chat", "<utterance>").required(),
                    DoorInput::switch("--apply"),
                    DoorInput::valued("--norms", "<dir>"),
                    workspace_input(),
                ],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::WritesWorkspace,
            vec![DoorNeed::Workspace, DoorNeed::File],
        ),
        Door::new(
            here(),
            "--venture",
            vec![],
            "orchestrate a whole system of dependent wish stages, fail-closed \
         and resumable; stages descend in dependency order under --apply",
            [
                vec![
                    DoorInput::valued("--venture-session", "<file>"),
                    DoorInput::switch("--apply"),
                    DoorInput::valued("--norms", "<dir>"),
                    workspace_input(),
                ],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::WritesWorkspace,
            vec![DoorNeed::Workspace, DoorNeed::Cargo, DoorNeed::File],
        ),
        Door::new(
            here(),
            "--reforge",
            vec![],
            "the external-empiricism bench: collect ground truth from oracle \
         binaries this repo did not write, re-forge them in scratch \
         workspaces, judge by execution; requires a real provider",
            [
                vec![DoorInput::valued("--reforge-report", "<file>")],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Provider, DoorNeed::Network, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--realize-bench",
            vec![],
            "the realization benchmark: drive a curated corpus of behavioural \
         wishes through the real provider descent and measure the fraction \
         that reach REALIZED (judged by execution) plus iterations and token \
         cost — provider-agnostic, mock refused; a measurement, not a gate",
            [
                vec![DoorInput::valued("--realize-bench-report", "<file>")],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Provider, DoorNeed::Network, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--realize-service",
            vec![],
            "the service-dimension realization smoke: drive one HTTP-service \
         wish through the real provider descent — the artifact is started as a \
         server and probed over HTTP, judged by it answering — proving the loop \
         realizes running services, not only CLIs; a measurement, not a gate",
            [armament_inputs(), output_inputs()].concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Provider, DoorNeed::Network, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--prose-bench",
            vec![],
            "the prose->spec benchmark: run natural-language utterances through \
         the intent extractor and wish compiler and score the facets against a \
         ground truth — offline via the keyword router, or via the LLM extractor \
         with a provider; a measurement, not a gate",
            [armament_inputs(), output_inputs()].concat(),
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "--realize-multicrate",
            vec![],
            "the multi-crate realization smoke: drive one Run wish onto a \
         two-crate workspace (a bin plus the library it calls) and realize the \
         logic across the crate boundary, judged by executing the bin; a real \
         provider, a measurement, not a gate",
            [armament_inputs(), output_inputs()].concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Provider, DoorNeed::Network, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--steward",
            vec![],
            "self-husbandry: survey the workspace's own landscape and, under \
         --apply, descend the open chores inside the operator-named fence \
         (nothing is fenced by default)",
            [
                vec![
                    DoorInput::valued("--fence", "<classes>"),
                    DoorInput::valued("--steward-max", "<n>"),
                    DoorInput::valued("--steward-report", "<file>"),
                    DoorInput::switch("--all"),
                    DoorInput::valued("--capacity", "<n>"),
                    DoorInput::switch("--apply"),
                    DoorInput::valued("--norms", "<dir>"),
                    workspace_input(),
                ],
                armament_inputs(),
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::WritesWorkspace,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--inject-norm",
            vec![],
            "operator governance: add a norm from a JSON spec (facet templates \
         only — path-like templates are rejected); it arrives unarmed",
            vec![
                DoorInput::valued("--norms", "<dir>").required(),
                DoorInput::valued("--inject-norm", "<file>").required(),
            ],
            DoorGovernance::GovernanceAct,
            vec![DoorNeed::Store, DoorNeed::File],
        ),
        Door::new(
            here(),
            "--promote-norm",
            vec![],
            "operator governance: arm a stored norm's prose trigger (the \
         explicit flag IS the approval); reserved grammar words are refused",
            vec![
                DoorInput::valued("--norms", "<dir>").required(),
                DoorInput::valued("--trigger", "<word>").required(),
            ],
            DoorGovernance::GovernanceAct,
            vec![DoorNeed::Store],
        ),
        Door::new(
            here(),
            "--foundry",
            vec![],
            "run the loop's own gate executor alone: allowlisted cargo checks \
         (build,test,lint,typecheck) with worst-wins outcome and \
         content-addressed evidence; sources untouched (target/ cache aside)",
            [
                vec![
                    DoorInput::valued("--foundry", "<kinds: build,test,lint,typecheck>").required(),
                    workspace_input(),
                ],
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--witness",
            vec![],
            "execute the workspace's binary once under the sandbox witness \
         (cwd-confined, 60s budget, output capped but digest-complete) and \
         print the content-addressed evidence of the run",
            [
                vec![
                    DoorInput::valued("--witness", "<argv,comma,separated>").required(),
                    workspace_input(),
                ],
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--parseback",
            vec![],
            "capture the workspace's content-addressed topology snapshot \
         (crates, files, dependency edges); with a baseline file, report \
         severity-ranked drift — the baseline is never silently replaced",
            [
                vec![
                    DoorInput::valued("--parseback-baseline", "<file>"),
                    workspace_input(),
                ],
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--kcube",
            vec![],
            "export the workspace's SystemCube blueprint as a real, \
         roundtrip-verified .kcube archive into the directory you name \
         (refuses silent overwrite; the explicit flag is the approval)",
            [
                vec![
                    DoorInput::valued("--kcube", "<out-dir>").required(),
                    DoorInput::valued("--capacity", "<n>"),
                    workspace_input(),
                ],
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace, DoorNeed::Cargo],
        ),
        Door::new(
            here(),
            "--codematrix",
            vec![],
            "per-source 5D fingerprints (relationality, cohesion, topology, \
         symmetry, entropy) and the most resonant pairs — strictly advisory: \
         it ranks, it never gates",
            [vec![workspace_input()], output_inputs()].concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace],
        ),
        Door::new(
            here(),
            "--alchemy",
            vec![],
            "the combine lab: seed structural elements from the workspace \
         (deduped by the cross-language novelty gate), drive combine() to a \
         fixpoint, and report the discovered catalog and its frontier — the \
         alchemy / \"Doodle God\" laboratory over real code; strictly advisory",
            [
                vec![
                    workspace_input(),
                    DoorInput::valued("--threshold", "<0..1>"),
                ],
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::ReadOnly,
            vec![DoorNeed::Workspace],
        ),
        Door::new(
            here(),
            "--doors",
            vec![],
            "this catalog: the binary's complete docking surface, spoken by the \
         binary itself — content-addressed, deterministic, pinned by test \
         against the parser; --doors-merge federates other surfaces' emitted \
         catalogs into one ecosystem inventory (each file verified by content \
         address before it is trusted)",
            [
                vec![DoorInput::valued("--doors-merge", "<catalogs,comma>")],
                output_inputs(),
            ]
            .concat(),
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "--help",
            vec!["-h".into()],
            "the prose usage text (the human-shaped companion of --doors)",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
    ];

    DoorCatalog::new(doors)
}

fn need_label(need: &DoorNeed) -> &'static str {
    match need {
        DoorNeed::Provider => "provider",
        DoorNeed::Cargo => "cargo",
        DoorNeed::Network => "network",
        DoorNeed::Workspace => "workspace",
        DoorNeed::Store => "store",
        DoorNeed::File => "file",
    }
}

/// The surface's owner — the binary a door belongs to.
fn surface_owner(surface: &DoorSurface) -> &str {
    match surface {
        DoorSurface::Cli { binary } => binary,
        DoorSurface::Http { binary, .. } => binary,
    }
}

/// The operator-shaped rendering of the catalog (the JSON form is the
/// machine truth; this is its readable face). A single surface renders as
/// that binary's doors; a federated catalog renders as the ecosystem
/// inventory, each door prefixed with its owner.
pub fn render(catalog: &DoorCatalog, color: bool) -> String {
    let (bold, dim, yellow, reset) = if color {
        ("\x1b[1m", "\x1b[2m", "\x1b[33m", "\x1b[0m")
    } else {
        ("", "", "", "")
    };
    let owners: std::collections::BTreeSet<&str> = catalog
        .doors
        .iter()
        .map(|d| surface_owner(&d.surface))
        .collect();
    let federated = owners.len() > 1;
    let mut out = if federated {
        format!("{bold}Kosmocrates doors — the federated docking surface{reset}\n")
    } else {
        format!(
            "{bold}Kosmocrates doors — the docking surface of {}{reset}\n",
            owners.iter().next().copied().unwrap_or("kosmo-run")
        )
    };
    out.push_str(&format!(
        "  catalog {dim}{}…{reset} · {} door(s){}\n",
        &catalog.catalog_id.to_hex()[..12],
        catalog.len(),
        if federated {
            format!(
                " · surfaces: {}",
                owners.iter().copied().collect::<Vec<_>>().join(", ")
            )
        } else {
            String::new()
        }
    ));
    for door in &catalog.doors {
        let aliases = if door.aliases.is_empty() {
            String::new()
        } else {
            format!(" (alias {})", door.aliases.join(", "))
        };
        let label = match &door.surface {
            DoorSurface::Cli { .. } => door.name.clone(),
            DoorSurface::Http { method, .. } => format!("{method} {}", door.name),
        };
        let owner = if federated {
            format!("{dim}{}{reset} ", surface_owner(&door.surface))
        } else {
            String::new()
        };
        out.push_str(&format!(
            "\n  {owner}{bold}{label}{reset}{aliases}  {yellow}[{}]{reset}\n",
            door.governance.label()
        ));
        out.push_str(&format!("      {}\n", door.summary));
        if !door.inputs.is_empty() {
            let inputs: Vec<String> = door
                .inputs
                .iter()
                .map(|i| {
                    let mut s = i.name.clone();
                    if let Some(v) = &i.value {
                        s.push(' ');
                        s.push_str(v);
                    }
                    if i.required {
                        s.push_str(" (required)");
                    }
                    s
                })
                .collect();
            out.push_str(&format!(
                "      {dim}inputs:{reset} {}\n",
                inputs.join(" · ")
            ));
        }
        if !door.needs.is_empty() {
            let needs: Vec<&str> = door.needs.iter().map(need_label).collect();
            out.push_str(&format!("      {dim}needs:{reset}  {}\n", needs.join(", ")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// This binary's own parser source — the catalog is pinned against it.
    const MAIN_SRC: &str = include_str!("main.rs");

    /// Every flag literal the parser actually matches: a `"-…"` string
    /// literal followed by `=>` or `|` (a match arm or arm alternative).
    fn parsed_flags() -> BTreeSet<String> {
        let mut flags = BTreeSet::new();
        for line in MAIN_SRC.lines() {
            if line.trim_start().starts_with("//") {
                continue; // prose about flags is not a parse arm
            }
            let mut rest = line;
            while let Some(start) = rest.find("\"-") {
                let after = &rest[start + 1..];
                let Some(end) = after.find('"') else { break };
                let literal = &after[..end];
                let tail = after[end + 1..].trim_start();
                if tail.starts_with("=>") || tail.starts_with('|') {
                    flags.insert(literal.to_string());
                }
                rest = &after[end + 1..];
            }
        }
        flags
    }

    /// Every name the catalog speaks (door names, aliases, input flags) that
    /// is flag-shaped.
    fn cataloged_flags() -> BTreeSet<String> {
        let mut flags = BTreeSet::new();
        for door in catalog().doors {
            for name in std::iter::once(&door.name).chain(door.aliases.iter()) {
                if name.starts_with('-') {
                    flags.insert(name.clone());
                }
            }
            for input in &door.inputs {
                if input.name.starts_with('-') {
                    flags.insert(input.name.clone());
                }
            }
        }
        flags
    }

    #[test]
    fn the_catalog_is_pinned_to_the_parser() {
        let parsed = parsed_flags();
        let cataloged = cataloged_flags();
        let unparsed: Vec<&String> = cataloged.difference(&parsed).collect();
        assert!(
            unparsed.is_empty(),
            "the catalog speaks flags the parser does not match: {unparsed:?}"
        );
        let undescribed: Vec<&String> = parsed.difference(&cataloged).collect();
        assert!(
            undescribed.is_empty(),
            "the parser matches flags the catalog does not speak — \
             a door without a description is a hole in the docking surface: \
             {undescribed:?}"
        );
    }

    #[test]
    fn wish_door_speaks_layers_and_staged_flags() {
        let catalog = catalog();
        let wish = catalog
            .doors
            .iter()
            .find(|d| d.name == "--wish")
            .expect("a --wish door");
        let inputs: BTreeSet<&str> = wish.inputs.iter().map(|i| i.name.as_str()).collect();
        assert!(inputs.contains("--layers"), "the wish door renders strata");
        assert!(inputs.contains("--staged"), "the wish door descends staged");
    }

    #[test]
    fn every_door_is_wellformed() {
        let catalog = catalog();
        assert!(catalog.len() >= 13, "all modes are doors");
        let mut names = BTreeSet::new();
        for door in &catalog.doors {
            assert!(!door.summary.is_empty(), "{} has no summary", door.name);
            assert!(
                names.insert(door.name.clone()),
                "door names are unique: {}",
                door.name
            );
        }
        // The catalog describes itself, and the governance vocabulary is in
        // honest use: decrees and read-only doors exist alongside writers.
        assert!(catalog.doors.iter().any(|d| d.name == "--doors"));
        assert!(catalog
            .doors
            .iter()
            .any(|d| d.governance == DoorGovernance::GovernanceAct));
        assert!(catalog
            .doors
            .iter()
            .any(|d| d.governance == DoorGovernance::ReadOnly));
    }

    #[test]
    fn the_catalog_is_deterministic() {
        assert_eq!(catalog().catalog_id, catalog().catalog_id);
        let text = render(&catalog(), false);
        assert!(text.contains("--steward"), "{text}");
        assert!(text.contains("[governance-act]"), "{text}");
    }
}
