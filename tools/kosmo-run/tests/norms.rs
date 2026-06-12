//! The norm organ, end to end — learning, governance, expansion.
//!
//! The plan's Phase-4 pin: realize the same facet shape in two workspaces
//! (three observations), watch the third run propose — and store — an
//! UNARMED norm; arm it with `--promote-norm --trigger`; watch the trigger
//! word expand in wish prose. And the inverse pins: without the store the
//! same prose compiles to nothing, and reserved grammar words refuse to be
//! triggers. The catalog starts empty, always.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("kosmo-norms-{tag}-{nanos}"))
}

/// A minimal observable crate: manifest + empty lib root.
fn crate_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "").unwrap();
}

/// Extract the hex id following `marker` in `text`.
fn id_after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    text.split(marker).nth(1)?.split_whitespace().next()
}

#[test]
fn norm_lifecycle_learn_promote_expand() {
    let base = scratch("lifecycle");
    let norms = base.join("norms");
    fs::create_dir_all(&norms).unwrap();
    let norms_dir = norms.to_str().unwrap().to_string();
    let ws1 = base.join("ws1");
    let ws2 = base.join("ws2");
    let ws3 = base.join("ws3");

    // Three realized descents of one shape — subjects differ, shape doesn't —
    // across two workspaces (alpha@ws1, beta@ws2, gamma@ws1).
    let mut learned_id = None;
    for (ws, subject) in [(&ws1, "alpha"), (&ws2, "beta"), (&ws1, "gamma")] {
        crate_fixture(ws);
        let prose = format!("a module {subject} and a function {subject}_load");
        let out = kosmo_run()
            .args([
                "--wish",
                &prose,
                "--apply",
                "--no-color",
                "--norms",
                &norms_dir,
                ws.to_str().unwrap(),
            ])
            .output()
            .expect("spawn kosmo-run");
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if !out.status.success() && stderr.contains("could not observe") {
            // cargo metadata unavailable in exotic sandboxes — the library
            // tests pin the learning math; this is the CLI seam.
            eprintln!("observe unavailable, skipping: {stderr}");
            fs::remove_dir_all(&base).ok();
            return;
        }
        assert!(
            out.status.success(),
            "descent for {subject} failed:\n{stdout}\n{stderr}"
        );
        assert!(stdout.contains("wish realized"), "{subject}: {stdout}");
        if let Some(id) = id_after(&stdout, "norm learned: ") {
            learned_id = Some(id.to_string());
        }
    }
    let learned_id = learned_id.expect("the third realized observation crosses the thresholds");

    // The learned norm is stored but UNARMED: its would-be trigger expands
    // nothing yet (no self-promotion — CRUD-relapse guard #2).
    crate_fixture(&ws3);
    let out = kosmo_run()
        .args([
            "--wish",
            "a loader delta",
            "--no-color",
            "--norms",
            &norms_dir,
            ws3.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("VACUOUS"),
        "an unarmed norm must not expand: {stdout}"
    );

    // Operator promotion arms the trigger.
    let out = kosmo_run()
        .args([
            "--norms",
            &norms_dir,
            "--promote-norm",
            &learned_id,
            "--trigger",
            "loader",
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("promoted norm"));

    // Now the word expands: "a loader delta" compiles to the learned shape.
    let out = kosmo_run()
        .args([
            "--wish",
            "a loader delta",
            "--no-color",
            "--norms",
            &norms_dir,
            ws3.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("met 0/2"), "two unmet facets: {stdout}");
    assert!(stdout.contains("delta_load"), "{stdout}");
    assert_eq!(out.status.code(), Some(1), "unrealized wish exits 1");

    // The same prose WITHOUT the store compiles to nothing — promoted norms
    // live in the caller's catalog, never in the built-in grammar.
    let out = kosmo_run()
        .args([
            "--wish",
            "a loader delta",
            "--no-color",
            ws3.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("VACUOUS"),
        "compile_wish itself stays untouched"
    );

    fs::remove_dir_all(&base).ok();
}

#[test]
fn inject_promote_and_reserved_trigger_guard() {
    let base = scratch("inject");
    let norms = base.join("norms");
    fs::create_dir_all(&norms).unwrap();
    let norms_dir = norms.to_str().unwrap().to_string();

    let spec_path = base.join("organ.json");
    fs::write(
        &spec_path,
        r#"{
            "name": "web-organ",
            "description": "a module with an organ capability",
            "level": "Molecule",
            "semantic_class": "structural",
            "template": [
                {"kind": "Module", "key_pattern": "{name}"},
                {"kind": "Capability", "key_pattern": "organ:{name}"}
            ]
        }"#,
    )
    .unwrap();

    // Injection: arrives unarmed.
    let out = kosmo_run()
        .args([
            "--norms",
            &norms_dir,
            "--inject-norm",
            spec_path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(stdout.contains("injected norm"));
    assert!(stdout.contains("unarmed"));
    let id = id_after(&stdout, "injected norm ").unwrap().to_string();

    // Reserved grammar words may not become triggers.
    let out = kosmo_run()
        .args([
            "--norms",
            &norms_dir,
            "--promote-norm",
            &id,
            "--trigger",
            "module",
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("reserved"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // A free word arms.
    let out = kosmo_run()
        .args([
            "--norms",
            &norms_dir,
            "--promote-norm",
            &id,
            "--trigger",
            "organ",
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The armed trigger expands in wish prose.
    let ws = base.join("ws");
    crate_fixture(&ws);
    let out = kosmo_run()
        .args([
            "--wish",
            "an organ heart",
            "--no-color",
            "--norms",
            &norms_dir,
            ws.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stderr}");
        fs::remove_dir_all(&base).ok();
        return;
    }
    assert!(stdout.contains("met 0/2"), "{stdout}");
    assert!(stdout.contains("organ:heart"), "{stdout}");

    fs::remove_dir_all(&base).ok();
}

#[test]
fn diseased_spec_and_missing_flags_are_refused() {
    let base = scratch("guards");
    let norms = base.join("norms");
    fs::create_dir_all(&norms).unwrap();
    let norms_dir = norms.to_str().unwrap().to_string();

    // A path-like template never enters the store.
    let spec_path = base.join("diseased.json");
    fs::write(
        &spec_path,
        r#"{
            "name": "smuggler",
            "description": "a frozen file tree",
            "level": "Atom",
            "semantic_class": "x",
            "template": [{"kind": "Module", "key_pattern": "src/models/{name}"}]
        }"#,
    )
    .unwrap();
    let out = kosmo_run()
        .args([
            "--norms",
            &norms_dir,
            "--inject-norm",
            spec_path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("path-like"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !norms.join("norms.jsonl").exists(),
        "nothing may be persisted on a refused injection"
    );

    // Governance flags demand their companions.
    let out = kosmo_run()
        .args(["--inject-norm", spec_path.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--norms"));

    let out = kosmo_run()
        .args(["--norms", &norms_dir, "--promote-norm", "abc123"])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--trigger"));

    fs::remove_dir_all(&base).ok();
}
