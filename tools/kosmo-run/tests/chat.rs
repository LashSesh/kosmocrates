//! The chat front door, end to end — one utterance, routed onto the organs.
//!
//! Pins: routing is total (gibberish becomes a measurable wish, never a
//! template and never an error), the routing decision is echoed for audit,
//! descent intent never escalates past `--apply`, and adoption-by-chat
//! rides the same read-only-by-default contract as the flags.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A minimal observable crate with one undocumented, untested module.
fn fixture(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-chat-{tag}-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod router;\n").unwrap();
    fs::write(
        root.join("src/router.rs"),
        "pub fn dispatch(p: &str) -> u32 { p.len() as u32 }\n",
    )
    .unwrap();
    root
}

#[test]
fn chat_routes_to_the_landscape_and_echoes_the_decision() {
    let root = fixture("landscape");
    let out = kosmo_run()
        .args([
            "--chat",
            "show me the landscape",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("chat[keyword] → show-landscape"),
        "{stdout}"
    );
    assert!(stdout.contains("Kosmocrates wish landscape"), "{stdout}");
    assert!(!stdout.contains("geometry:"), "no geometry unless asked");

    // …and the geometry rides along when the utterance asks for it.
    let out = kosmo_run()
        .args([
            "--chat",
            "map the landscape with geometry",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("chat[keyword] → show-landscape +geometry"),
        "{stdout}"
    );
    assert!(stdout.contains("geometry:"), "{stdout}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn chat_totality_gibberish_becomes_a_measurable_wish() {
    let root = fixture("gibberish");
    let out = kosmo_run()
        .args([
            "--chat",
            "frobnicate the quux sideways",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stderr}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert!(stdout.contains("chat[keyword] → make-wish"), "{stdout}");
    assert!(stdout.contains("Kosmocrates wish"), "{stdout}");
    assert!(
        stdout.contains("VACUOUS"),
        "an unparseable wish is honestly vacuous, never a template: {stdout}"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn chat_descent_intent_never_escalates_past_apply() {
    let root = fixture("descend");
    let before = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let out = kosmo_run()
        .args([
            "--chat",
            "build a module alpha",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stderr}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert!(stdout.contains("chat[keyword] → descend-wish"), "{stdout}");
    assert!(stdout.contains("add --apply to descend"), "{stdout}");
    assert!(stdout.contains("alpha"), "the wish is measured: {stdout}");
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        before,
        "nothing written without --apply"
    );
    assert_eq!(out.status.code(), Some(1), "unmet wish exits 1");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn chat_adopt_cluster_rides_the_read_only_contract() {
    let root = fixture("adopt");
    let out = kosmo_run()
        .args([
            "--chat",
            "adopt cluster 1",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("chat[keyword] → adopt cluster 1"),
        "{stdout}"
    );
    assert!(stdout.contains("adopted as one wish"), "{stdout}");
    assert!(stdout.contains("add --apply to descend"), "{stdout}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn chat_status_and_governance_hints() {
    let root = fixture("status");
    let norms = root.join("norms-store");
    let out = kosmo_run()
        .args([
            "--chat",
            "status",
            "--no-color",
            "--norms",
            norms.to_str().unwrap(),
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("chat[keyword] → status"), "{stdout}");
    assert!(stdout.contains("measured standing"), "{stdout}");
    // The cockpit reports every armed organ — the empty catalog included.
    assert!(stdout.contains("norms: 0 known (0 armed)"), "{stdout}");

    // First contact without any key is an invitation, not a dead end.
    let out = kosmo_run()
        .arg(root.to_str().unwrap())
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("CEREBRAS_API_KEY")
        .env_remove("KOSMO_LLM_API_KEY")
        .env_remove("KOSMO_LLM_PROVIDER")
        .env_remove("KOSMO_LLM_BASE_URL")
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("these run offline"),
        "the keyless first contact must invite: {stderr}"
    );
    assert!(stderr.contains("--landscape"), "{stderr}");

    // InjectNorm routes to instructions — chat carries no spec files.
    let out = kosmo_run()
        .args([
            "--chat",
            "inject a norm",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--inject-norm <spec.json>"), "{stdout}");
    assert!(stdout.contains("chat carries no paths"), "{stdout}");

    // --chat and --wish are mutually exclusive.
    let out = kosmo_run()
        .args([
            "--chat",
            "status",
            "--wish",
            "a crate x",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("mutually exclusive"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::remove_dir_all(&root).ok();
}
