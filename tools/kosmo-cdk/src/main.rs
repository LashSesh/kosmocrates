//! kosmo-cdk — the Con-Dragger / ASCC CLI (spec ch. 24).
//!
//! Verbs: `bind`, `stack`, `close`, `diamond`, `explain`. Report-only and
//! fail-closed — every response carries a trace root and a gate result, and no
//! verb mutates the host. The CLI drives a built-in monotone sample stack through
//! the whole fold so the pipeline (bind → stack → close → diamond) runs
//! end-to-end on a report-only profile (the Definition of Done, §26.3).

use kosmo_cdk_core::{qsr_stack, Canonical, Stage, StageMetrics, Status};
use kosmo_coffindragger::{
    binding::BoundUnit, close_stack, press_diamond, ClosureOutcome,
};
use kosmo_core::{Digest, Q16};
use std::process::ExitCode;

fn d(s: &str) -> Digest {
    Digest::of_bytes(s.as_bytes())
}

fn stage(units: &[&str], density: Q16, contr: Q16) -> Stage {
    Stage {
        attractor: Digest::ZERO,
        units: units.iter().map(|u| d(u)).collect(),
        evidence: d("evidence"),
        gate_status: Status::Pass,
        trace_root: d("trace"),
        residue: Digest::ZERO,
        metrics: StageMetrics {
            density,
            purity: Q16::HALF,
            irreducibility: Q16::HALF,
            curvature: Q16::ZERO,
            contradiction_energy: contr,
        },
        certificates: vec![],
    }
}

/// A built-in monotone sample stack (density rising, contradiction falling), its
/// support, and the exkalibrated residue — enough to drive the fold end-to-end.
fn sample() -> (Vec<Stage>, Vec<Digest>, Vec<Digest>) {
    let stages = vec![
        stage(&["core.a"], Q16::ratio(1, 4).unwrap(), Q16::HALF),
        stage(&["core.a", "core.b"], Q16::HALF, Q16::ratio(1, 4).unwrap()),
    ];
    let support = vec![d("core.a"), d("core.b")];
    let residue = vec![d("foreign.x")]; // foreign mass purged to the boundary
    (stages, support, residue)
}

fn hex(d: &Digest) -> String {
    d.to_hex().chars().take(12).collect()
}

fn run(verb: &str, json: bool) -> ExitCode {
    let (stages, support, residue) = sample();
    match verb {
        "bind" => {
            // KBL — a fully bound (pullable) sample artifact under invariant 19.1.
            let unit = BoundUnit {
                id: d("WishCube:sample"),
                artifact_type: "WishCube".into(),
                evidence: d("evidence"),
                policy: d("policy.report_only"),
                trace: d("trace"),
                replay: true,
                boundary: Digest::ZERO,
                status: Status::Pass,
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&unit).unwrap());
            } else {
                println!("kosmo-cdk bind — KBL (invariant 19.1)");
                println!("  {} {} · pullable: {}", unit.artifact_type, hex(&unit.id), unit.is_pullable());
            }
            ExitCode::SUCCESS
        }
        "stack" => {
            let qsr = qsr_stack(&stages);
            if json {
                let ids: Vec<String> = stages.iter().map(|s| s.canon().to_hex()).collect();
                println!("{}", serde_json::json!({"stages": ids, "qsr_status": format!("{qsr:?}")}));
            } else {
                println!("kosmo-cdk stack — {} stage(s), stack-QSR: {qsr:?}", stages.len());
                for s in &stages {
                    println!("  stage {} · {} unit(s)", hex(&s.canon()), s.units.len());
                }
            }
            ExitCode::SUCCESS
        }
        "close" => match close_stack(&stages, support, residue, true) {
            ClosureOutcome::DiamondCandidate(b) => {
                if json {
                    println!("{}", serde_json::to_string_pretty(&b).unwrap());
                } else {
                    println!("kosmo-cdk close — STACK CLOSED → diamond candidate");
                    println!("  fold bundle {} · {} support · {} residue", hex(&b.bundle_id), b.support.len(), b.residue.len());
                }
                ExitCode::SUCCESS
            }
            ClosureOutcome::DeferredStackReport { bundle, .. } => {
                println!("kosmo-cdk close — DEFERRED (QSR/closure not satisfied) · bundle {}", hex(&bundle.bundle_id));
                ExitCode::from(1)
            }
        },
        "diamond" => {
            match close_stack(&stages, support, residue, true) {
                ClosureOutcome::DiamondCandidate(bundle) => {
                    // QSR passed → mint a QSR certificate; the sample core is an
                    // irreducible, materializable, replayable diamond.
                    let qsr_cert = Digest::of(&("qsr-certificate", &bundle.bundle_id));
                    match press_diamond(&bundle, Status::Pass, Status::Pass, true, true, true, qsr_cert) {
                        Ok(cube) => {
                            if json {
                                println!("{}", serde_json::to_string_pretty(&cube).unwrap());
                            } else {
                                println!("kosmo-cdk diamond — DiamondCubeCandidate ✓ (QSR-certified, irreducible, report-only)");
                                println!("  diamond_id        {}", hex(&cube.diamond_id));
                                println!("  source_stack_id   {}", hex(&cube.source_stack_id));
                                println!("  qsr_certificate   {}", hex(&cube.qsr_certificate_id));
                                println!("  support_units     {}", cube.support_units.len());
                                println!("  residue_report    {}", hex(&cube.residue_report_id));
                                println!("  materialization   {:?}", cube.materialization_profile);
                            }
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("kosmo-cdk diamond — refused (fail-closed): {e}");
                            ExitCode::from(1)
                        }
                    }
                }
                ClosureOutcome::DeferredStackReport { .. } => {
                    println!("kosmo-cdk diamond — DEFERRED: stack not closed under QSR");
                    ExitCode::from(1)
                }
            }
        }
        "explain" => {
            let qsr = qsr_stack(&stages);
            println!("kosmo-cdk explain — the fold, gate by gate (report-only):");
            println!("  scores rank, gates decide (I2). Cleaning is never silent forgetting (I3).");
            println!("  · stack-QSR (monotone density/purity/irreducibility, falling contradiction): {qsr:?}");
            println!("  · residue (purged foreign mass, visible): {} unit(s)", residue.len());
            println!("  · trace roots present on every stage (I4); ids are canonical (I1).");
            println!("  verbs: bind · stack · close · diamond · explain   [--json]");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("kosmo-cdk: unknown verb `{other}` (bind|stack|close|diamond|explain) [--json]");
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let verb = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("explain");
    run(verb, json)
}
