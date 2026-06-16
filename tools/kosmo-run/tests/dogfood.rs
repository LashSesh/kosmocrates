//! Run 21 — kosmocrates measures itself.
//!
//! The instrument's own definition-of-done (`kosmo-run.wishes`) is checked
//! against the live kosmo-run source: a self-referential regression guard for
//! the tool's structural skeleton. The spiral through its own centre — the
//! truth-instrument turned on itself, and a stress test on a real, large crate.

use std::path::Path;
use std::process::Command;

#[test]
fn kosmo_run_realizes_its_own_definition_of_done() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let wishes = Path::new(crate_dir).join("kosmo-run.wishes");
    assert!(
        wishes.exists(),
        "the self-DoD is committed at {}",
        wishes.display()
    );

    let out = Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
        .args(["--wishlist", wishes.to_str().unwrap(), "--no-color", crate_dir])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }

    // Robust to growing the skeleton: every self-wish must be realized (exit 0,
    // no unmet ✗ line) and genuine (no over-fit suspect) — the backbone is whole
    // and real, not a hologram.
    assert!(stdout.contains("Kosmocrates wishlist"), "{stdout}");
    assert_eq!(
        out.status.code(),
        Some(0),
        "every self-wish realized → exit 0:\n{stdout}"
    );
    assert!(
        !stdout.contains('\u{2717}'),
        "no unmet wish in the self definition-of-done:\n{stdout}"
    );
    assert!(
        !stdout.contains("suspect"),
        "the self-skeleton is genuine, not a hologram:\n{stdout}"
    );
}
