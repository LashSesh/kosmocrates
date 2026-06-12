//! The wish atelier, end to end — shaping rounds on a durable draft.
//!
//! Pins: dictation enters the wish, machine proposals never do without an
//! explicit accept (the governance symmetry with the norm organ), verdicts
//! act on the displayed numbering, realize is the only door to the
//! workspace (and writes only under --apply), and a tampered draft is
//! refused by its content address.

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
    std::env::temp_dir().join(format!("kosmo-atelier-{tag}-{nanos}"))
}

fn crate_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "").unwrap();
}

/// One atelier round; returns stdout.
fn round(draft: &Path, ws: &Path, utterance: &str, apply: bool) -> (String, Option<i32>) {
    let mut cmd = kosmo_run();
    cmd.args(["--atelier", draft.to_str().unwrap(), "--no-color"]);
    if !utterance.is_empty() {
        cmd.args(["--chat", utterance]);
    }
    if apply {
        cmd.arg("--apply");
    }
    cmd.arg(ws.to_str().unwrap());
    let out = cmd.output().expect("spawn kosmo-run");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code(),
    )
}

#[test]
fn dialogue_shapes_verdicts_apply_and_realize_descends() {
    let base = scratch("dialogue");
    let ws = base.join("ws");
    crate_fixture(&ws);
    let draft = base.join("draft.json");

    // Round 1: dictation enters the wish; companions are proposed, numbered
    // after the accepted facets.
    let (out, code) = round(
        &draft,
        &ws,
        "a module parser and a function parse_line",
        false,
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("wish so far (2)"), "{out}");
    assert!(out.contains("Module parser"), "{out}");
    assert!(out.contains("Symbol parse_line"), "{out}");
    assert!(out.contains("proposed (3)"), "{out}");
    assert!(out.contains("Test parser_smoke"), "{out}");
    assert!(out.contains("Doc parser"), "{out}");

    // Round 2: accept the test companion (display index 3), reject the docs.
    let (out, _) = round(&draft, &ws, "accept 3", false);
    assert!(out.contains("wish so far (3)"), "{out}");
    assert!(out.contains("proposed (2)"), "{out}");
    let (out, _) = round(&draft, &ws, "reject 4,5", false);
    assert!(out.contains("wish so far (3)"), "{out}");
    assert!(!out.contains("proposed ("), "all proposals resolved: {out}");

    // Round 3: a bare invocation shows the persisted state.
    let (out, _) = round(&draft, &ws, "", false);
    assert!(out.contains("dialogue (1 round(s))"), "{out}");
    assert!(out.contains("wish so far (3)"), "{out}");

    // The draft file is the durable workpiece: dialogue + identity on disk.
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&draft).unwrap()).unwrap();
    assert_eq!(saved["prose_history"].as_array().unwrap().len(), 1);
    assert!(!saved["draft_id"].as_str().unwrap_or("").is_empty());

    // Realize without --apply: measured, read-only, honest hint.
    let before = fs::read_to_string(ws.join("src/lib.rs")).unwrap();
    let (out, code) = round(&draft, &ws, "realize", false);
    assert!(out.contains("met 0/3"), "{out}");
    assert!(out.contains("read-only"), "{out}");
    assert_eq!(code, Some(1), "unmet wish exits 1: {out}");
    assert_eq!(fs::read_to_string(ws.join("src/lib.rs")).unwrap(), before);

    // Realize with --apply: the descent converges and the workspace holds
    // module + function + smoke test.
    let (out, code) = round(&draft, &ws, "realize", true);
    if out.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {out}");
        fs::remove_dir_all(&base).ok();
        return;
    }
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("wish realized"), "{out}");
    let lib = fs::read_to_string(ws.join("src/lib.rs")).unwrap();
    assert!(lib.contains("pub mod parser"), "{lib}");
    assert!(lib.contains("fn parse_line"), "{lib}");
    fs::remove_dir_all(&base).ok();
}

#[test]
fn proposals_never_enter_the_wish_without_accept() {
    // THE governance pin at the CLI seam: realize right after round 1 —
    // the wish carries exactly the two dictated facets, none of the three
    // proposals.
    let base = scratch("governance");
    let ws = base.join("ws");
    crate_fixture(&ws);
    let draft = base.join("draft.json");

    let (out, _) = round(
        &draft,
        &ws,
        "a module parser and a function parse_line",
        false,
    );
    assert!(out.contains("proposed (3)"), "{out}");
    let (out, code) = round(&draft, &ws, "realize", true);
    if out.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {out}");
        fs::remove_dir_all(&base).ok();
        return;
    }
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("met 0/2"),
        "exactly the two dictated facets descend: {out}"
    );
    let lib = fs::read_to_string(ws.join("src/lib.rs")).unwrap();
    assert!(
        !lib.contains("parser_smoke"),
        "an unaccepted proposal must never materialize: {lib}"
    );
    fs::remove_dir_all(&base).ok();
}

#[test]
fn guards_empty_drafts_tampering_and_flag_conflicts() {
    let base = scratch("guards");
    let ws = base.join("ws");
    crate_fixture(&ws);
    let draft = base.join("draft.json");

    // Realizing an empty draft is an honest refusal.
    let (out, code) = round(&draft, &ws, "realize", false);
    assert!(out.contains("no accepted facets"), "{out}");
    assert_eq!(code, Some(1));

    // Prose containing action words stays prose (fail-closed dictation).
    let (out, _) = round(&draft, &ws, "build a module alpha", false);
    assert!(out.contains("wish so far (1)"), "{out}");
    assert!(out.contains("Module alpha"), "{out}");

    // A tampered draft is refused by its content address.
    let text = fs::read_to_string(&draft).unwrap();
    fs::write(&draft, text.replace("alpha", "omega")).unwrap();
    let (out, code) = round(&draft, &ws, "", false);
    assert_ne!(code, Some(0));
    assert!(out.contains("content address"), "{out}");

    // --atelier and --wish are mutually exclusive.
    let out = kosmo_run()
        .args([
            "--atelier",
            base.join("other.json").to_str().unwrap(),
            "--wish",
            "a crate x",
            ws.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("mutually exclusive"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::remove_dir_all(&base).ok();
}
