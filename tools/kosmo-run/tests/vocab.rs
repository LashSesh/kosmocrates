//! Run 30 — `--vocab`: the wish vocabulary, by example. The biggest usability
//! gap is "how do I phrase a wish?" (it tripped even the author in Run 26), so
//! the tool can now show the prose forms for every stratum, grounded in the
//! grammar. Needs no workspace.

use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

#[test]
fn vocab_shows_a_form_for_every_stratum() {
    let out = kosmo_run()
        .args(["--vocab", "--no-color"])
        .output()
        .expect("spawn kosmo-run");
    assert!(out.status.success(), "--vocab needs no workspace and exits 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // One stratum heading and one canonical form each, shallow to deep.
    for stratum in ["Existence", "Shape", "Wiring", "Verified", "Live"] {
        assert!(stdout.contains(stratum), "missing stratum {stratum}: {stdout}");
    }
    for form in [
        "a module",
        "a doc for",
        "a signature",
        "a contract add(i32,i32)->i32",
        "a behaviour add(2,3)=>5",
        "a run add,2,3=>out~5",
    ] {
        assert!(stdout.contains(form), "missing form {form}: {stdout}");
    }
}
