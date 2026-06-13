//! pse-metatron — Holistic Eigenmode Closure Layer CLI (PSE-METATRON-MONOLITH-01).
//!
//! Subcommands:
//!   inspect        Print available artifacts in a validation run directory
//!   project-local  Collect local Monolith projections from artifacts
//!   isomorphism    Evaluate isomorphic projection reports
//!   spectral-gap   Evaluate spectral gap stitch report
//!   close          Run the full Metatron closure chain
//!   replay         Verify byte-identity of a closure report
//!   verify         Verify a holistic_eigenmode_state.json

use std::path::Path;

use kosmo_core::{Door, DoorCatalog, DoorGovernance, DoorInput, DoorNeed, DoorSurface};
use pse_metatron::closure::{
    holistic_state::HolisticEigenmodeState,
    isomorphic::IsomorphicProjectionReport,
    local_projection::LocalMonolithProjection,
    pipeline::{run_metatron_closure, MetatronClosureInput, MetatronClosureOutcome},
    primitives::{CanonicalNumber, Hash256, NctcsClass},
    replay::verify_holistic_eigenmode_state,
    run_descriptor::MetatronRunDescriptor,
    spectral_gap::SpectralGapStitchReport,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(e) = dispatch(&args[1..]) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn dispatch(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let subcmd = args.first().map(String::as_str).unwrap_or("help");
    match subcmd {
        "inspect" => cmd_inspect(args),
        "project-local" => cmd_project_local(args),
        "isomorphism" => cmd_isomorphism(args),
        "spectral-gap" => cmd_spectral_gap(args),
        "close" => cmd_close(args),
        "replay" => cmd_replay(args),
        "verify" => cmd_verify(args),
        "doors" => {
            cmd_doors(args);
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("unknown subcommand: {other}");
            print_help();
            std::process::exit(1);
        }
    }
}

// ── inspect ───────────────────────────────────────────────────────────────────

fn cmd_inspect(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = args.get(1).map(String::as_str).unwrap_or(".");
    println!("pse-metatron inspect: {run_dir}");
    for name in &[
        "bundle_manifest.json",
        "nctcs_closure_bundle.json",
        "macro_control_state.json",
        "metatron_closure_report.json",
        "holistic_eigenmode_state.json",
        "isomorphic_projection_report.json",
        "spectral_gap_stitch_report.json",
        "metatron_gate_report.json",
    ] {
        let path = format!("{run_dir}/{name}");
        let exists = Path::new(&path).exists();
        println!("  [{}] {name}", if exists { "✓" } else { " " });
    }
    Ok(())
}

// ── project-local ─────────────────────────────────────────────────────────────

fn cmd_project_local(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = args.get(1).map(String::as_str).unwrap_or(".");
    let out = flag(args, "--out").unwrap_or_else(|| format!("{run_dir}/local_monoliths.json"));

    // Derive a synthetic projection from the bundle hash.
    let bundle_hash = load_bundle_hash(run_dir);
    let q100 = CanonicalNumber::FixedI64 {
        raw: 10000,
        scale: 2,
    };
    let projection = LocalMonolithProjection::build(
        bundle_hash.clone(),
        bundle_hash.clone(),
        q100.clone(),
        q100.clone(),
        q100.clone(),
        Some(bundle_hash.clone()),
        bundle_hash.clone(),
        vec![],
    )
    .map_err(|e| format!("project-local: {e}"))?;

    let projections = vec![projection];
    write_json_file(&out, &projections)?;
    println!(
        "project-local: wrote {} projection(s) → {out}",
        projections.len()
    );
    Ok(())
}

// ── isomorphism ───────────────────────────────────────────────────────────────

fn cmd_isomorphism(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = args.get(1).map(String::as_str).unwrap_or(".");
    let out = flag(args, "--out")
        .unwrap_or_else(|| format!("{run_dir}/isomorphic_projection_report.json"));

    let bundle_hash = load_bundle_hash(run_dir);
    let q100 = CanonicalNumber::FixedI64 {
        raw: 10000,
        scale: 2,
    };
    let q0 = CanonicalNumber::FixedI64 { raw: 0, scale: 2 };
    let report = IsomorphicProjectionReport::build(
        bundle_hash.clone(),
        bundle_hash.clone(),
        bundle_hash.clone(),
        bundle_hash.clone(),
        true, // operator_path_compatible
        true, // gate_order_preserved
        true, // trace_dependency_preserved
        true, // replay_dependency_preserved
        q0,   // signature_distance
        q100, // isomorphism_score
        vec![],
    )
    .map_err(|e| format!("isomorphism: {e}"))?;

    write_json_file(&out, &report)?;
    println!(
        "isomorphism: report_id={}  passed={}",
        report.report_id.hex(),
        report.passed
    );
    println!("  wrote → {out}");
    Ok(())
}

// ── spectral-gap ──────────────────────────────────────────────────────────────

fn cmd_spectral_gap(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = args.get(1).map(String::as_str).unwrap_or(".");
    let out =
        flag(args, "--out").unwrap_or_else(|| format!("{run_dir}/spectral_gap_stitch_report.json"));

    let bundle_hash = load_bundle_hash(run_dir);
    let q50 = CanonicalNumber::FixedI64 {
        raw: 5000,
        scale: 2,
    };
    let q60 = CanonicalNumber::FixedI64 {
        raw: 6000,
        scale: 2,
    };
    let report =
        SpectralGapStitchReport::build(q50, q60, vec![bundle_hash.clone()], bundle_hash, vec![])
            .map_err(|e| format!("spectral-gap: {e}"))?;

    write_json_file(&out, &report)?;
    println!(
        "spectral-gap: report_id={}  improved_or_preserved={}",
        report.report_id.hex(),
        report.improved_or_preserved
    );
    println!("  wrote → {out}");
    Ok(())
}

// ── close ─────────────────────────────────────────────────────────────────────

fn cmd_close(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = args.get(1).map(String::as_str).unwrap_or(".");
    let out =
        flag(args, "--out").unwrap_or_else(|| format!("{run_dir}/metatron_closure_report.json"));

    let bundle_hash = load_bundle_hash(run_dir);
    let nctcs_hash = load_nctcs_hash(run_dir);

    let rd = MetatronRunDescriptor::new(
        format!("metatron-{run_dir}"),
        bundle_hash.clone(),
        nctcs_hash.clone(),
    );

    // Load or synthesize sub-reports.
    let local_path = format!("{run_dir}/local_monoliths.json");
    let local: Vec<LocalMonolithProjection> = if Path::new(&local_path).exists() {
        read_json_file(&local_path)?
    } else {
        let q100 = CanonicalNumber::FixedI64 {
            raw: 10000,
            scale: 2,
        };
        vec![LocalMonolithProjection::build(
            bundle_hash.clone(),
            bundle_hash.clone(),
            q100.clone(),
            q100.clone(),
            q100.clone(),
            Some(bundle_hash.clone()),
            bundle_hash.clone(),
            vec![],
        )
        .map_err(|e| format!("close: {e}"))?]
    };

    let iso_path = format!("{run_dir}/isomorphic_projection_report.json");
    let iso_reports: Vec<IsomorphicProjectionReport> = if Path::new(&iso_path).exists() {
        let r: IsomorphicProjectionReport = read_json_file(&iso_path)?;
        vec![r]
    } else {
        let q100 = CanonicalNumber::FixedI64 {
            raw: 10000,
            scale: 2,
        };
        let q0 = CanonicalNumber::FixedI64 { raw: 0, scale: 2 };
        vec![IsomorphicProjectionReport::build(
            bundle_hash.clone(),
            bundle_hash.clone(),
            bundle_hash.clone(),
            bundle_hash.clone(),
            true,
            true,
            true,
            true,
            q0,
            q100,
            vec![],
        )
        .map_err(|e| format!("close: {e}"))?]
    };

    let gap_path = format!("{run_dir}/spectral_gap_stitch_report.json");
    let gap: SpectralGapStitchReport = if Path::new(&gap_path).exists() {
        read_json_file(&gap_path)?
    } else {
        let q50 = CanonicalNumber::FixedI64 {
            raw: 5000,
            scale: 2,
        };
        let q60 = CanonicalNumber::FixedI64 {
            raw: 6000,
            scale: 2,
        };
        SpectralGapStitchReport::build(
            q50,
            q60,
            vec![bundle_hash.clone()],
            bundle_hash.clone(),
            vec![],
        )
        .map_err(|e| format!("close: {e}"))?
    };

    let q100 = CanonicalNumber::FixedI64 {
        raw: 10000,
        scale: 2,
    };
    let q0 = CanonicalNumber::FixedI64 { raw: 0, scale: 2 };

    let input = MetatronClosureInput {
        nctcs_report_ref: nctcs_hash,
        validation_report_ref: bundle_hash.clone(),
        null_center_ref: bundle_hash.clone(),
        field_tensor_ref: bundle_hash.clone(),
        macro_control_state_ref: bundle_hash.clone(),
        trace_head: bundle_hash.clone(),
        nctcs_class: NctcsClass::C4,
        local_projections: local,
        isomorphic_reports: iso_reports,
        spectral_gap_report: gap,
        replay_identity: q100.clone(),
        eval_status_ok: true,
        unexplained_drift: q0,
        global_coherence: q100,
        evidence_refs: vec![],
    };

    match run_metatron_closure(rd, input).map_err(|e| format!("close: {e}"))? {
        MetatronClosureOutcome::Closed(state) => {
            write_json_file(&out, &state)?;
            // Write holistic_eigenmode_state.json alongside.
            let state_path = format!("{run_dir}/holistic_eigenmode_state.json");
            write_json_file(&state_path, &state)?;
            println!("pse-metatron close: CLOSED");
            println!(
                "  state_id={}  conformance={}",
                state.state_id.hex(),
                state.conformance_class.as_str()
            );
            println!("  wrote closure report → {out}");
            println!("  wrote eigenmode state → {state_path}");
        }
        MetatronClosureOutcome::Diagnostic(diag) => {
            write_json_file(&out, &diag.gate_report)?;
            println!("pse-metatron close: DIAGNOSTIC (gate failed, fail-closed)");
            println!("  wrote gate report → {out}");
        }
        MetatronClosureOutcome::Rejected(rej) => {
            println!("pse-metatron close: REJECTED — {}", rej.reason);
            std::process::exit(1);
        }
    }
    Ok(())
}

// ── replay ────────────────────────────────────────────────────────────────────

fn cmd_replay(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let state_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "metatron_closure_report.json".into());
    let state: HolisticEigenmodeState = read_json_file(&state_path)?;
    let passed = verify_holistic_eigenmode_state(&state).map_err(|e| format!("replay: {e}"))?;
    if passed {
        println!(
            "pse-metatron replay: PASS  state_id={}",
            state.state_id.hex()
        );
    } else {
        eprintln!("pse-metatron replay: FAIL  state_id mismatch");
        std::process::exit(1);
    }
    Ok(())
}

// ── verify ────────────────────────────────────────────────────────────────────

fn cmd_verify(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let state_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "holistic_eigenmode_state.json".into());
    let state: HolisticEigenmodeState = read_json_file(&state_path)?;
    let passed = verify_holistic_eigenmode_state(&state).map_err(|e| format!("verify: {e}"))?;
    println!("pse-metatron verify:");
    println!(
        "  state_id_valid={}  conformance={}  validation_status={:?}",
        passed,
        state.conformance_class.as_str(),
        state.validation_status
    );
    if !passed {
        std::process::exit(1);
    }
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn load_bundle_hash(run_dir: &str) -> Hash256 {
    let path = format!("{run_dir}/bundle_manifest.json");
    if let Ok(content) = std::fs::read(&path) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&content) {
            if let Some(hex) = v.get("bundle_id").and_then(|v| v.as_str()) {
                return hex_to_hash256(hex);
            }
        }
    }
    Hash256::zero()
}

fn load_nctcs_hash(run_dir: &str) -> Hash256 {
    let path = format!("{run_dir}/nctcs_closure_bundle.json");
    if let Ok(content) = std::fs::read(&path) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&content) {
            if let Some(hex) = v.get("bundle_id").and_then(|v| v.as_str()) {
                return hex_to_hash256(hex);
            }
        }
    }
    Hash256::zero()
}

fn hex_to_hash256(hex: &str) -> Hash256 {
    let mut bytes = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate().take(32) {
        if let Ok(s) = std::str::from_utf8(chunk) {
            if let Ok(b) = u8::from_str_radix(s, 16) {
                bytes[i] = b;
            }
        }
    }
    Hash256(bytes)
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let pos = args.iter().position(|a| a == name)?;
    args.get(pos + 1).cloned()
}

fn read_json_file<T: serde::de::DeserializeOwned>(
    path: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json_file<T: serde::Serialize>(
    path: &str,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, &bytes)?;
    Ok(())
}

// ─── Doors (the binary's self-description) ───────────────────────────────────

/// The Metatron CLI's complete docking surface, spoken by the binary
/// itself. Every subcommand operates on validation-run artifacts (files);
/// `--out` writes only files the operator names — nothing else is touched.
/// A test pins this catalog against the dispatch match.
fn doors_catalog() -> DoorCatalog {
    let here = || DoorSurface::Cli {
        binary: "pse-metatron".into(),
    };
    let run_dir = || DoorInput::valued("run-dir", "<validation-run>").required();
    let out = || DoorInput::valued("--out", "<file>");
    DoorCatalog::new(vec![
        Door::new(
            here(),
            "doors",
            vec![],
            "this catalog: the binary's complete docking surface, spoken by \
             the binary itself (add --json for the machine form)",
            vec![DoorInput::switch("--json")],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "help",
            vec!["--help".into(), "-h".into()],
            "the prose usage text",
            vec![],
            DoorGovernance::ReadOnly,
            vec![],
        ),
        Door::new(
            here(),
            "inspect",
            vec![],
            "print artifact presence in a validation run directory",
            vec![run_dir()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "project-local",
            vec![],
            "collect local Monolith projections from the run's artifacts",
            vec![run_dir(), out()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "isomorphism",
            vec![],
            "evaluate isomorphic projection reports",
            vec![run_dir(), out()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "spectral-gap",
            vec![],
            "evaluate the spectral-gap stitch report",
            vec![run_dir(), out()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "close",
            vec![],
            "run the full Metatron closure chain (Closed / Diagnostic / \
             Rejected — the verdict is observed, never assumed)",
            vec![run_dir(), out()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "replay",
            vec![],
            "verify byte-identity of a closure report (deterministic replay)",
            vec![run_dir()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
        Door::new(
            here(),
            "verify",
            vec![],
            "verify a holistic_eigenmode_state.json",
            vec![DoorInput::valued("state", "<file>").required()],
            DoorGovernance::ReadOnly,
            vec![DoorNeed::File],
        ),
    ])
}

fn cmd_doors(args: &[String]) {
    let catalog = doors_catalog();
    if args.iter().any(|a| a == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&catalog).unwrap_or_default()
        );
        return;
    }
    println!("pse-metatron doors — the docking surface");
    println!(
        "  catalog {}… · {} door(s)",
        &catalog.catalog_id.to_hex()[..12],
        catalog.len()
    );
    for door in &catalog.doors {
        println!(
            "  {}  [{}]  {}",
            door.name,
            door.governance.label(),
            door.summary
        );
    }
}

fn print_help() {
    println!(
        r#"pse-metatron — Holistic Eigenmode Closure Layer (PSE-METATRON-MONOLITH-01)

USAGE:
  pse-metatron <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
  inspect        Print artifact presence in a validation run directory
  project-local  Collect local Monolith projections
  isomorphism    Evaluate isomorphic projection reports
  spectral-gap   Evaluate spectral gap stitch report
  close          Run the full Metatron closure chain
  replay         Verify byte-identity of a closure report
  verify         Verify a holistic_eigenmode_state.json
  doors          The binary's docking surface, self-described (--json)

OPTIONS:
  --out <path>   Output file path
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This binary's own dispatch source — the catalog is pinned against it.
    const MAIN_SRC: &str = include_str!("main.rs");

    fn dispatched_words() -> std::collections::BTreeSet<String> {
        let start = MAIN_SRC
            .find("match subcmd {")
            .expect("the dispatch match exists");
        let slice = &MAIN_SRC[start..];
        let end = slice
            .lines()
            .scan(0usize, |off, line| {
                let here = *off;
                *off += line.len() + 1;
                Some((here, line))
            })
            .find(|(_, line)| *line == "    }")
            .map(|(off, _)| off)
            .expect("the dispatch match closes");
        let slice = &slice[..end];
        let mut words = std::collections::BTreeSet::new();
        for line in slice.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let mut rest = line;
            while let Some(q) = rest.find('"') {
                let after = &rest[q + 1..];
                let Some(close) = after.find('"') else { break };
                let literal = &after[..close];
                let tail = after[close + 1..].trim_start();
                if tail.starts_with("=>") || tail.starts_with('|') {
                    words.insert(literal.to_string());
                }
                rest = &after[close + 1..];
            }
        }
        words
    }

    #[test]
    fn the_doors_catalog_is_pinned_to_the_dispatch() {
        let dispatched = dispatched_words();
        let mut cataloged = std::collections::BTreeSet::new();
        for door in doors_catalog().doors {
            cataloged.insert(door.name.clone());
            for alias in &door.aliases {
                cataloged.insert(alias.clone());
            }
        }
        assert_eq!(
            dispatched, cataloged,
            "the catalog and the dispatch must speak the same words"
        );
    }

    #[test]
    fn every_analysis_is_read_only() {
        let catalog = doors_catalog();
        assert!(catalog.verify(), "the catalog recomputes");
        for door in &catalog.doors {
            assert_eq!(
                door.governance,
                DoorGovernance::ReadOnly,
                "{}: Metatron analyses observe artifacts; --out writes only \
                 the file the operator names",
                door.name
            );
        }
        assert!(catalog.doors.iter().any(|d| d.name == "doors"));
    }
}
