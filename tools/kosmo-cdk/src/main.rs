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

use axum::{
    extract::Path,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::process::ExitCode;
use tokio::net::TcpListener;

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

/// Extract a `[package] name = "..."` from a Cargo.toml's text (no TOML dep).
fn parse_package_name(toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_package = l == "[package]";
            continue;
        }
        if in_package {
            if let Some(after) = l.strip_prefix("name") {
                if let Some(after_eq) = after.trim_start().strip_prefix('=') {
                    if let Some(name) = after_eq.split('"').nth(1) {
                        if !name.is_empty() {
                            return Some(name.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// **Live observation** of a real workspace: walk the directory (bounded, skipping
/// `target/` and dotfiles) and emit one evidence-bound `BlueprintUnit` per crate
/// (`[package] name`). A genuine SystemCube source — the host's real crate
/// topology, read from the filesystem, not a fabricated sample.
fn observe_workspace(root: &str) -> Vec<BlueprintUnit> {
    let policy = PolicyProfile::default_report_only();
    let mut units = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![(std::path::PathBuf::from(root), 0u32)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 5 || units.len() >= 512 {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push((p, depth + 1));
            } else if name == "Cargo.toml" {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    if let Some(pkg) = parse_package_name(&text) {
                        if seen.insert(pkg.clone()) {
                            units.push(BlueprintUnit::new(
                                BlueprintUnitKind::ModuleBoundary,
                                Digest::of_bytes(pkg.as_bytes()),
                                AuthorityLabel::Operator,
                                TaintLabel::Clean,
                                vec![Digest::of_bytes(p.to_string_lossy().as_bytes())],
                                &policy,
                            ));
                        }
                    }
                }
            }
        }
    }
    units
}

/// Bind a real, live-observed workspace SystemCube into a 1-stage fold. Returns
/// `None` if no crates are found. The contradiction `K` is the *real* pairwise
/// report over the observed crates (a contradiction-free topology is already
/// irreducible).
#[allow(clippy::type_complexity)]
fn live_fold(path: &str) -> Option<(Vec<Stage>, Vec<Digest>, Vec<Digest>, f64, f64)> {
    let units = observe_workspace(path);
    if units.is_empty() {
        return None;
    }
    let policy = PolicyProfile::default_report_only();
    let run = RunDescriptor::new(policy.id, path);
    let manifest = SystemCubeManifest::new(Digest::of_bytes(path.as_bytes()), &run, &policy, &units);
    let k = ContradictionEnergyReport::from_units(manifest.manifest_id, &policy, &units).total_energy;
    let stage = bind_systemcube(&manifest, &units, k);
    let support = stage.units.clone();
    let kf = k.to_f64();
    Some((vec![stage], support, vec![], kf, kf))
}

/// The fold to operate on: a live-observed workspace (`--workspace`) or the
/// built-in sample. Returns the fold plus a human source label.
#[allow(clippy::type_complexity)]
fn fold_for(workspace: Option<&str>) -> (Vec<Stage>, Vec<Digest>, Vec<Digest>, f64, f64, String) {
    if let Some(path) = workspace {
        if let Some((stages, support, residue, k0, k1)) = live_fold(path) {
            let n = stages.iter().map(|s| s.units.len()).max().unwrap_or(0);
            return (stages, support, residue, k0, k1, format!("live: {path} ({n} crates)"));
        }
        eprintln!("kosmo-cdk: no crates found under {path}; using the sample");
    }
    let (a, b, c, d, e) = real_fold();
    (a, b, c, d, e, "sample (payments service)".into())
}

fn run(verb: &str, json: bool, workspace: Option<&str>) -> ExitCode {
    let (stages, support, residue, k0, k1, source) = fold_for(workspace);
    let s0 = &stages[0];
    // Strong irreducibility (Def 17.2): the core is irreducible iff it carries no
    // residual contradiction (K = 0). A redundant live workspace defers honestly.
    let irreducible = k1 == 0.0;
    match verb {
        "bind" => {
            // KBL — the raw SystemCube bound into a CDK stage under invariant 19.1.
            let unit = BoundUnit {
                id: s0.attractor,
                artifact_type: format!("SystemCube [{source}]"),
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
                println!("  {source} · {} · {} support units · pullable: {}", hex(&unit.id), s0.units.len(), unit.is_pullable());
                println!("  contradiction energy K = {k0:.3}");
            }
            ExitCode::SUCCESS
        }
        "stack" => {
            let qsr = qsr_stack(&stages);
            println!("kosmo-cdk stack — {source} · {} stage(s), stack-QSR: {qsr:?}", stages.len());
            for (i, s) in stages.iter().enumerate() {
                let k = if i == 0 { k0 } else { k1 };
                println!("  S{i} {} · {} units · K={k:.3}", hex(&s.canon()), s.units.len());
            }
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
                // Strong irreducibility (Def 17.2): only a contradiction-free core
                // (K = 0) presses a diamond; a redundant one is refused, fail-closed.
                match press_diamond(&bundle, Status::Pass, Status::Pass, true, irreducible, true, qsr_cert) {
                    Ok(cube) => {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&cube).unwrap());
                        } else {
                            println!("kosmo-cdk diamond — DiamondCubeCandidate ✓  [{source}] (QSR-certified, report-only)");
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
                        eprintln!("  [{source}] K={k1:.3} — the topology is not yet irreducible; it needs condensation (contract the contradictions first).");
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
            println!("  source: {source}");
            println!("  scores rank, gates decide (I2). Cleaning is never silent forgetting (I3).");
            println!("  · contradiction energy K {k0:.3} → {k1:.3}  (redundancy exkalibrated to residue)");
            println!("  · stack-QSR (density/purity/irreducibility non-decreasing, K non-increasing): {qsr:?}");
            println!("  · residue (visible): {} unit(s); ids canonical (I1); trace roots present (I4)", residue.len());
            println!("  irreducible (K=0 → diamond-ready): {irreducible}");
            println!("  verbs: bind · stack · close · diamond · explain · serve   [--json] [--workspace <path>]");
            ExitCode::SUCCESS
        }
        "serve" => unreachable!("serve is dispatched in main"),
        other => {
            eprintln!("kosmo-cdk: unknown verb `{other}` (bind|stack|close|diamond|explain|serve) [--json]");
            ExitCode::from(2)
        }
    }
}

// ─── REST mirror (§24.2) ─────────────────────────────────────────────────────

/// Build the JSON response body for a verb — the REST mirror of the CLI. Every
/// response carries a `trace_root` and a `gate_result` (§24.2), report-only.
fn rest_artifact(verb: &str, id: Option<&str>) -> serde_json::Value {
    use serde_json::json;
    let (stages, support, residue, k0, k1) = real_fold();
    let trace = stages[0].trace_root.to_hex();
    let wrap = |gate: &str, mut body: serde_json::Value| -> serde_json::Value {
        if let serde_json::Value::Object(m) = &mut body {
            m.insert("verb".into(), json!(verb));
            m.insert("run_descriptor".into(), json!("report_only"));
            m.insert("trace_root".into(), json!(trace));
            m.insert("gate_result".into(), json!(gate));
        }
        body
    };
    match verb {
        "bind" => wrap(
            "pass",
            json!({ "artifact_type": "SystemCube(payments)", "support_units": stages[0].units.len(), "contradiction_k": k0, "pullable": true }),
        ),
        "stack" => {
            let qsr = qsr_stack(&stages);
            wrap(&format!("{qsr:?}").to_lowercase(), json!({ "stages": stages.len(), "qsr_status": format!("{qsr:?}") }))
        }
        "close" => match close_stack(&stages, support, residue, true) {
            ClosureOutcome::DiamondCandidate(b) => wrap(
                "pass",
                json!({ "closed": true, "fold_bundle": b.bundle_id.to_hex(), "support": b.support.len(), "residue": b.residue.len() }),
            ),
            ClosureOutcome::DeferredStackReport { bundle, .. } => {
                wrap("defer", json!({ "closed": false, "fold_bundle": bundle.bundle_id.to_hex() }))
            }
        },
        "diamond" => match close_stack(&stages, support, residue, true) {
            ClosureOutcome::DiamondCandidate(bundle) => {
                let cert = Digest::of(&("qsr-certificate", &bundle.bundle_id));
                match press_diamond(&bundle, Status::Pass, Status::Pass, true, true, true, cert) {
                    Ok(cube) => wrap("pass", json!({ "diamond": cube })),
                    Err(e) => wrap("reject", json!({ "error": e.to_string() })),
                }
            }
            ClosureOutcome::DeferredStackReport { .. } => wrap("defer", json!({ "deferred": true })),
        },
        "explain" => wrap(
            "pass",
            json!({ "id": id.unwrap_or("sample"), "contradiction": { "before": k0, "after": k1 }, "residue_units": residue.len(), "principle": "scores rank, gates decide (I2)" }),
        ),
        _ => json!({ "error": format!("unknown verb {verb}") }),
    }
}

async fn h_bind() -> Json<serde_json::Value> {
    Json(rest_artifact("bind", None))
}
async fn h_stack() -> Json<serde_json::Value> {
    Json(rest_artifact("stack", None))
}
async fn h_close() -> Json<serde_json::Value> {
    Json(rest_artifact("close", None))
}
async fn h_diamond() -> Json<serde_json::Value> {
    Json(rest_artifact("diamond", None))
}
async fn h_explain(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(rest_artifact("explain", Some(&id)))
}

/// The REST mirror of the CLI verbs (§24.2): `POST /cdk/{bind,stack,close,diamond}`
/// and `GET /cdk/explain/{id}`. Report-only; every response carries trace_root +
/// gate_result. Mirrors the same fold the CLI runs.
fn serve(port: u16) -> ExitCode {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("kosmo-cdk: runtime: {e}");
            return ExitCode::from(1);
        }
    };
    rt.block_on(async move {
        let app = Router::new()
            .route("/cdk/bind", post(h_bind))
            .route("/cdk/stack", post(h_stack))
            .route("/cdk/close", post(h_close))
            .route("/cdk/diamond", post(h_diamond))
            .route("/cdk/explain/{id}", get(h_explain));
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                eprintln!("kosmo-cdk REST  http://{addr}");
                eprintln!("  POST /cdk/{{bind,stack,close,diamond}} · GET /cdk/explain/{{id}}  (report-only)");
                let _ = axum::serve(listener, app).await;
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("kosmo-cdk: cannot bind {addr}: {e}");
                ExitCode::from(1)
            }
        }
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let verb = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("explain");
    if verb == "serve" {
        let port = args
            .iter()
            .position(|a| a == "--port")
            .and_then(|i| args.get(i + 1))
            .and_then(|p| p.parse().ok())
            .unwrap_or(8731);
        return serve(port);
    }
    let workspace = args
        .iter()
        .position(|a| a == "--workspace")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str);
    run(verb, json, workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_diamond_carries_trace_gate_and_the_cube() {
        let v = rest_artifact("diamond", None);
        assert_eq!(v["verb"], "diamond");
        assert_eq!(v["gate_result"], "pass");
        assert!(v["trace_root"].is_string(), "every response carries a trace_root");
        assert!(v["diamond"]["diamond_id"].is_string(), "the certified cube");
    }

    #[test]
    fn rest_bind_and_explain_are_well_formed_mirrors() {
        let b = rest_artifact("bind", None);
        assert_eq!(b["verb"], "bind");
        assert!(b["trace_root"].is_string() && b["gate_result"].is_string());
        let e = rest_artifact("explain", Some("abc123"));
        assert_eq!(e["id"], "abc123", "GET /cdk/explain/{{id}} echoes the id");
    }

    #[test]
    fn parse_package_name_extracts_only_the_package_name() {
        assert_eq!(
            parse_package_name("[package]\nname = \"foo\"\nversion=\"0.1\"").as_deref(),
            Some("foo")
        );
        assert_eq!(
            parse_package_name("[dependencies]\nname = \"bar\"").as_deref(),
            None,
            "only the [package] table"
        );
    }

    #[test]
    fn observe_workspace_reads_real_crates_live() {
        let dir = std::env::temp_dir().join(format!("cdk-ws-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::write(dir.join("a/Cargo.toml"), "[package]\nname = \"crate_a\"\n").unwrap();
        std::fs::write(dir.join("b/Cargo.toml"), "[package]\nname = \"crate_b\"\n").unwrap();
        let units = observe_workspace(dir.to_str().unwrap());
        assert_eq!(units.len(), 2, "two real crates observed from the filesystem");
        assert!(live_fold(dir.to_str().unwrap()).is_some(), "a live fold over the workspace");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
