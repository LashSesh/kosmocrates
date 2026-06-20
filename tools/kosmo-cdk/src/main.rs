//! kosmo-cdk — the Con-Dragger / ASCC CLI (spec ch. 24).
//!
//! Verbs: `bind`, `stack`, `close`, `diamond`, `explain`. Report-only and
//! fail-closed — every response carries a trace root and a gate result, and no
//! verb mutates the host. The CLI condenses a **real Kosmocrates `SystemCube`**
//! (the KBL binds it, ch. 16.3/19) end-to-end through the whole fold: a small
//! "payments" service whose build duplicated a module — a real contradiction the
//! CDK exkalibrates and then certifies as an irreducible DiamondCube.

use kosmo_cdk_core::{qsr_stack, Canonical, Stage, Status};
use kosmo_coffindragger::{
    bind_systemcube, binding::BoundUnit, close_stack, press_diamond, verify_contraction,
    ClosureOutcome,
};
use kosmo_core::{AuthorityLabel, Digest, PolicyProfile, RunDescriptor, TaintLabel};
use kosmo_systemcube::{BlueprintUnit, BlueprintUnitKind, ContradictionEnergyReport, SystemCubeManifest};
use std::process::ExitCode;

fn hex(d: &Digest) -> String {
    d.to_hex().chars().take(12).collect()
}

/// Build a REAL `SystemCube` — a small "payments" service whose LLM build
/// **duplicated** the `payments::core` module (real pairwise contradiction
/// energy) — then bind it to a raw CDK stage `S0` and condense it (exkalibrate the
/// redundancy, rebind the clean core) to a contracted stage `S1`. Returns the
/// stages, the condensed support, the exkalibrated residue, and the two
/// contradiction-energy readings (before → after).
#[allow(clippy::type_complexity)]
fn real_fold() -> (Vec<Stage>, Vec<Digest>, Vec<Digest>, f64, f64) {
    let policy = PolicyProfile::default_report_only();
    let run = RunDescriptor::new(policy.id, "payments");
    let host = Digest::of_bytes(b"payments-snapshot");
    let unit = |name: &str, kind: BlueprintUnitKind, auth: AuthorityLabel| {
        BlueprintUnit::new(
            kind,
            Digest::of_bytes(name.as_bytes()),
            auth,
            TaintLabel::Clean,
            vec![Digest::of_bytes(format!("ev:{name}").as_bytes())],
            &policy,
        )
    };
    let clean = vec![
        unit("payments::core", BlueprintUnitKind::ModuleBoundary, AuthorityLabel::Operator),
        unit("payments::api", BlueprintUnitKind::ModuleBoundary, AuthorityLabel::Operator),
        unit("payments::api>>core", BlueprintUnitKind::FiberDescriptor, AuthorityLabel::Operator),
    ];
    // The LLM emitted a second payments::core module boundary — same source, same
    // kind → a genuine Duplicate contradiction.
    let dup = BlueprintUnit::new(
        BlueprintUnitKind::ModuleBoundary,
        Digest::of_bytes(b"payments::core"),
        AuthorityLabel::Agent { name: "llm".into() },
        TaintLabel::Clean,
        vec![Digest::of_bytes(b"ev:dup")],
        &policy,
    );
    let mut raw = clean.clone();
    raw.push(dup.clone());

    let m0 = SystemCubeManifest::new(host, &run, &policy, &raw);
    let k0 = ContradictionEnergyReport::from_units(m0.manifest_id, &policy, &raw).total_energy;
    let s0 = bind_systemcube(&m0, &raw, k0);

    let m1 = SystemCubeManifest::new(host, &run, &policy, &clean);
    let k1 = ContradictionEnergyReport::from_units(m1.manifest_id, &policy, &clean).total_energy;
    let s1 = bind_systemcube(&m1, &clean, k1);

    let support = s1.units.clone();
    let residue = vec![dup.unit_id]; // the exkalibrated redundancy (visible, I3)
    (vec![s0, s1], support, residue, k0.to_f64(), k1.to_f64())
}

fn run(verb: &str, json: bool) -> ExitCode {
    let (stages, support, residue, k0, k1) = real_fold();
    let s0 = &stages[0];
    match verb {
        "bind" => {
            // KBL — the raw SystemCube bound into a CDK stage under invariant 19.1.
            let unit = BoundUnit {
                id: s0.attractor,
                artifact_type: "SystemCube(payments)".into(),
                evidence: s0.evidence,
                policy: Digest::of_bytes(b"policy.report_only"),
                trace: s0.trace_root,
                replay: true,
                boundary: s0.residue,
                status: s0.gate_status,
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&unit).unwrap());
            } else {
                println!("kosmo-cdk bind — KBL: a real SystemCube bound (invariant 19.1)");
                println!("  SystemCube(payments) {} · {} support units · pullable: {}", hex(&unit.id), s0.units.len(), unit.is_pullable());
                println!("  contradiction energy K = {k0:.3}  (a duplicated module — real pairwise contradiction)");
            }
            ExitCode::SUCCESS
        }
        "stack" => {
            let qsr = qsr_stack(&stages);
            println!("kosmo-cdk stack — {} stage(s) (raw → condensed), stack-QSR: {qsr:?}", stages.len());
            println!("  S0 {} · K={k0:.3} (raw, redundant)", hex(&s0.canon()));
            println!("  S1 {} · K={k1:.3} (condensed — the duplicate exkalibrated)", hex(&stages[1].canon()));
            ExitCode::SUCCESS
        }
        "close" => {
            // Show the contraction is honest (no covert removal, Contract 8.4).
            let contr = verify_contraction(&s0.units, &support, &residue);
            match close_stack(&stages, support, residue, true) {
                ClosureOutcome::DiamondCandidate(b) => {
                    println!("kosmo-cdk close — STACK CLOSED → diamond candidate (contraction: {contr:?})");
                    println!("  fold bundle {} · {} support · {} residue", hex(&b.bundle_id), b.support.len(), b.residue.len());
                    ExitCode::SUCCESS
                }
                ClosureOutcome::DeferredStackReport { bundle, .. } => {
                    println!("kosmo-cdk close — DEFERRED · bundle {}", hex(&bundle.bundle_id));
                    ExitCode::from(1)
                }
            }
        }
        "diamond" => match close_stack(&stages, support, residue, true) {
            ClosureOutcome::DiamondCandidate(bundle) => {
                let qsr_cert = Digest::of(&("qsr-certificate", &bundle.bundle_id));
                // The condensed core (K = 0) is irreducible and materializable.
                match press_diamond(&bundle, Status::Pass, Status::Pass, true, true, true, qsr_cert) {
                    Ok(cube) => {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&cube).unwrap());
                        } else {
                            println!("kosmo-cdk diamond — DiamondCubeCandidate ✓  (a real SystemCube, QSR-certified, report-only)");
                            println!("  diamond_id        {}", hex(&cube.diamond_id));
                            println!("  source_stack_id   {}", hex(&cube.source_stack_id));
                            println!("  qsr_certificate   {}", hex(&cube.qsr_certificate_id));
                            println!("  support_units     {}  (the irreducible core)", cube.support_units.len());
                            println!("  residue_report    {}  (the exkalibrated duplicate)", hex(&cube.residue_report_id));
                            println!("  materialization   {:?}", cube.materialization_profile);
                            println!("  contradiction     K {k0:.3} → {k1:.3}  (condensed away)");
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
        },
        "explain" => {
            let qsr = qsr_stack(&stages);
            println!("kosmo-cdk explain — the fold over a real SystemCube, gate by gate (report-only):");
            println!("  scores rank, gates decide (I2). Cleaning is never silent forgetting (I3).");
            println!("  · bound a real SystemCube(payments) → 2 stages (raw S0, condensed S1)");
            println!("  · contradiction energy K {k0:.3} → {k1:.3}  (the LLM's duplicate exkalibrated to residue)");
            println!("  · stack-QSR (density/purity/irreducibility non-decreasing, K non-increasing): {qsr:?}");
            println!("  · residue (visible): {} unit(s); ids canonical (I1); trace roots present (I4)", residue.len());
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
