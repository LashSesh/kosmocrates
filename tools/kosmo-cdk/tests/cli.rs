//! End-to-end CLI smoke (DoD §26.3): bind/stack/close/diamond/explain run on a
//! report-only profile, and `diamond` emits a complete DiamondCubeCandidate with
//! a QSR certificate and a residue report.

use std::process::Command;

fn cdk() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-cdk"))
}

#[test]
fn every_verb_runs_end_to_end_report_only() {
    for verb in ["bind", "stack", "close", "explain"] {
        let out = cdk().arg(verb).output().expect("spawn kosmo-cdk");
        assert!(out.status.success(), "`{verb}` runs report-only: {:?}", out.status);
    }
}

#[test]
fn diamond_emits_a_qsr_certified_candidate() {
    let out = cdk().arg("diamond").output().expect("spawn kosmo-cdk");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "the sample fold closes to a diamond: {stdout}");
    assert!(stdout.contains("DiamondCubeCandidate"), "{stdout}");
    assert!(stdout.contains("qsr_certificate"), "carries a QSR certificate: {stdout}");
    assert!(stdout.contains("ReportOnly"), "L8 safety: report-only by default: {stdout}");
}

#[test]
fn diamond_json_is_a_complete_candidate() {
    let out = cdk().args(["diamond", "--json"]).output().expect("spawn kosmo-cdk");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert!(v["diamond_id"].is_string());
    assert!(v["qsr_certificate_id"].is_string(), "QSR certificate present");
    assert!(v["residue_report_id"].is_string(), "residue report present");
    assert_eq!(v["materialization_profile"], "report_only");
}
