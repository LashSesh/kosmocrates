//! Kosmocrates promotion CLI — the operator entry point for substrate→core
//! promotion.
//!
//! Runs the full substrate pipeline on a workspace, takes the gated
//! `PseBridgeCandidate`s the pipeline emitted, and offers them to the PSE
//! crystallization engine through `pse-adapter-kosmo` — the sanctioned
//! crossing. PSE alone decides crystallization.
//!
//! ```text
//! USAGE:
//!     kosmo-promote [OPTIONS] [PATH]
//!
//! OPTIONS:
//!     --offer        Actually feed the engine (DryRun profile; in-memory, no
//!                    host writes). Default is report-only: candidates are
//!                    listed and every outcome is SkippedByReportOnly.
//!     --all-kinds    Offer all pipeline candidate kinds, not only
//!                    CertifiedCrystal (adds Structural/Topology observations).
//!     --json         Machine-readable output.
//!     -h, --help     This help.
//! ```
//!
//! Fail-closed defaults: without `--offer` the engine is never touched; the
//! adapter allowlist starts at CertifiedCrystal only.

use std::process;

use kosmo_core::{ImplementationMode, PolicyProfile};
use kosmo_pipeline::{run_workspace_pipeline, IntegrationRunOptions};
use kosmo_pse_bridge::{PseBridgeCandidateKind, PseBridgePolicy, PseBridgeRateLimit};
use pse_adapter_kosmo::{describe_crystal, offer_candidates, KosmoBridgeAdapter};
use pse_core::GlobalState;
use pse_types::Config;

struct Args {
    path: String,
    offer: bool,
    all_kinds: bool,
    json: bool,
}

const HELP: &str = "kosmo-promote — substrate→core promotion (PSE decides crystallization)\n\
\n\
USAGE:\n\
    kosmo-promote [OPTIONS] [PATH]\n\
\n\
OPTIONS:\n\
    --offer        Feed the PSE engine (DryRun profile; in-memory, no host\n\
                   writes). Default is report-only: every outcome is\n\
                   SkippedByReportOnly and the engine is never touched.\n\
    --all-kinds    Offer all pipeline candidate kinds, not only\n\
                   CertifiedCrystal.\n\
    --json         Machine-readable output.\n\
    -h, --help     This help.\n";

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        path: ".".into(),
        offer: false,
        all_kinds: false,
        json: false,
    };
    for raw in std::env::args().skip(1) {
        match raw.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                process::exit(0);
            }
            "--offer" => args.offer = true,
            "--all-kinds" => args.all_kinds = true,
            "--json" => args.json = true,
            flag if flag.starts_with('-') => {
                return Err(format!("unknown flag '{flag}'; run --help for usage"));
            }
            path => args.path = path.to_string(),
        }
    }
    Ok(args)
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    };

    // 1. Substrate pipeline — always a passive, report-only scan.
    let pipeline_policy = PolicyProfile::default_report_only();
    let options = IntegrationRunOptions::all_layers(8);
    let report = match run_workspace_pipeline(&args.path, &options, &pipeline_policy) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: pipeline failed: {e}");
            process::exit(1);
        }
    };

    // 2. Select the candidate kinds to offer (fail-closed default: certified only).
    let kinds = if args.all_kinds {
        vec![
            PseBridgeCandidateKind::CertifiedCrystal,
            PseBridgeCandidateKind::StructuralObservation,
            PseBridgeCandidateKind::TopologyObservation,
        ]
    } else {
        vec![PseBridgeCandidateKind::CertifiedCrystal]
    };
    let candidates: Vec<_> = report
        .pse_candidates
        .iter()
        .filter(|c| kinds.contains(&c.kind))
        .cloned()
        .collect();

    // 3. Promotion profile — the gate that decides whether the engine runs.
    //    Default ReportOnly: validate_candidate short-circuits, engine untouched.
    let profile = if args.offer {
        PolicyProfile {
            mode: ImplementationMode::DryRun,
            ..PolicyProfile::default()
        }
    } else {
        PolicyProfile::default() // ReportOnly
    };
    let bridge_policy =
        PseBridgePolicy::allow(pipeline_policy.id, kinds, PseBridgeRateLimit::strict());
    let adapter = KosmoBridgeAdapter::new(&args.path)
        .with_allowed_kinds(bridge_policy.allowed_candidate_kinds.clone());

    // 4. Offer — one engine step per candidate, per-candidate verdicts.
    let config = Config::default();
    let mut state = GlobalState::new(&config);
    let offers = offer_candidates(
        &mut state,
        &config,
        &profile,
        &bridge_policy,
        &candidates,
        &adapter,
    );

    // 5. Report.
    if args.json {
        let rows: Vec<serde_json::Value> = offers
            .iter()
            .zip(candidates.iter())
            .map(|(o, c)| {
                serde_json::json!({
                    "candidate_id": c.id.to_hex(),
                    "kind": format!("{:?}", c.kind),
                    "label": c.label,
                    "confidence_raw": c.confidence.raw(),
                    "outcome": o.outcome,
                    "crystal_committed": o.crystal.is_some(),
                })
            })
            .collect();
        let doc = serde_json::json!({
            "path": args.path,
            "report_id": report.report_id.to_hex(),
            "offered": candidates.len(),
            "engine_commit_index": state.commit_index,
            "mode": if args.offer { "offer" } else { "report-only" },
            "offers": rows,
        });
        println!("{}", serde_json::to_string_pretty(&doc).expect("json"));
    } else {
        println!("kosmo-promote  {}", args.path);
        println!(
            "  pipeline report {} | pse_candidates {} | selected {}",
            &report.report_id.to_hex()[..16],
            report.pse_candidates.len(),
            candidates.len()
        );
        println!(
            "  mode: {}",
            if args.offer {
                "OFFER (DryRun profile — engine runs in-memory)"
            } else {
                "report-only (engine untouched; use --offer to feed PSE)"
            }
        );
        let mut accepted = 0usize;
        let mut deferred = 0usize;
        let mut rejected = 0usize;
        let mut skipped = 0usize;
        for (offer, candidate) in offers.iter().zip(candidates.iter()) {
            use kosmo_pse_bridge::PromotionOutcome as O;
            let verdict = match &offer.outcome {
                O::Accepted => {
                    accepted += 1;
                    "ACCEPTED".to_string()
                }
                O::Deferred => {
                    deferred += 1;
                    "deferred".to_string()
                }
                O::Rejected { reason } => {
                    rejected += 1;
                    format!("rejected: {reason}")
                }
                O::SkippedByPolicy | O::SkippedByReportOnly => {
                    skipped += 1;
                    "skipped (policy)".to_string()
                }
            };
            println!("    {:<44} {}", candidate.label, verdict);
            if let Some(crystal) = &offer.crystal {
                println!("      └─ {}", describe_crystal(crystal, candidate));
            }
        }
        println!(
            "  outcomes: {accepted} accepted | {deferred} deferred | {rejected} rejected | {skipped} skipped"
        );
        println!("  engine commit_index: {}", state.commit_index);
    }
}
