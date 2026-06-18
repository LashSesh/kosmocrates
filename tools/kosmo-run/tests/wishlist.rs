//! Run 15 — the wishlist: a file of wishes measured as a project's
//! definition-of-done, into an aggregate realization gauge.
//!
//! Pins: the aggregate `realized N/M` is reported, the run exits 0 only when
//! every wish is realized, and `--json` emits a machine project gauge.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A crate with module `alpha` and function `f` present; module `beta` absent.
fn workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-wishlist-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\npub fn f() {}\n").unwrap();
    fs::write(root.join("src/alpha.rs"), "pub fn a() {}\n").unwrap();
    root
}

#[test]
fn wishlist_reports_aggregate_and_gates_unrealized() {
    let root = workspace();
    let list = root.join("project.wishes");
    fs::write(
        &list,
        "# the project's definition of done\na module alpha\na function f\na module beta\n",
    )
    .unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    assert!(stdout.contains("Kosmocrates wishlist"), "{stdout}");
    assert!(stdout.contains("realized 2/3"), "alpha + f met, beta missing: {stdout}");
    assert!(stdout.contains("a module beta"), "{stdout}");
    assert_eq!(out.status.code(), Some(1), "not all realized → exit 1: {stdout}");
}

// Run 45 — Stufe 2: `--wishlist --blueprint` reads the file as ONE architecture
// (a city plan), not N independent wishes — pooled into a single foundations-first
// graph, the plan standing only when every node is realized.
#[test]
fn wishlist_blueprint_reads_the_file_as_one_architecture() {
    let root = workspace();
    let list = root.join("city.wishes");
    fs::write(&list, "a module alpha\na function f\na module beta\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--blueprint",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    // One architecture, not three gauges: the blueprint render, foundations-first.
    assert!(stdout.contains("blueprint"), "renders the city plan: {stdout}");
    assert!(stdout.contains("the architecture"), "{stdout}");
    assert!(stdout.contains("2/3 realized"), "alpha + f met, beta missing: {stdout}");
    assert!(stdout.contains("incomplete"), "the plan does not yet stand: {stdout}");
    assert_eq!(out.status.code(), Some(1), "not all realized → exit 1: {stdout}");
}

#[test]
fn wishlist_blueprint_json_emits_architecture_nodes() {
    let root = workspace();
    let list = root.join("city2.wishes");
    fs::write(&list, "a module alpha\na function f\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--blueprint",
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON array");
    assert!(v.is_array(), "the blueprint machine view is an array of nodes: {stdout}");
    assert!(
        v.as_array().unwrap().iter().any(|n| n["key"] == "alpha"),
        "a node for module alpha: {stdout}"
    );
}

// Run 46 — Stufe 2 brick 47: the architecture's coverage. Two subsystems with no
// declared edges is a heap, not an architecture — a warning about the SPEC's
// shape, independent of whether the crates are observed.
#[test]
fn wishlist_blueprint_warns_on_a_heap_with_no_edges() {
    let root = workspace();
    let list = root.join("heap.wishes");
    fs::write(&list, "a crate api\na crate logging\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--blueprint",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    assert!(
        stdout.contains("heap, not an architecture"),
        "two crates, no edges → coverage warns: {stdout}"
    );
}

// Run 47 — Stufe 2 brick 48: the pse-traverse bridge. --plan renders the
// deterministic collapse plan; a heap's disconnected crates are operationally
// unreachable (formally excised — the cousin of brick 47's coverage warning).
#[test]
fn wishlist_plan_renders_collapse_plan_and_excises_a_heap() {
    let root = workspace();
    let list = root.join("heap.wishes");
    fs::write(&list, "a crate api\na crate logging\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--plan",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    assert!(stdout.contains("collapse plan"), "renders the plan: {stdout}");
    assert!(stdout.contains("pse-traverse"), "names the engine: {stdout}");
    assert!(
        stdout.contains("excised"),
        "a heap's components are operationally unreachable: {stdout}"
    );
}

// Run 48 — Stufe 3 brick 49: wiring integrity (the first cross-component
// invariant). An edge to an undeclared component is a dangling wire — a system
// inconsistency the static check catches.
#[test]
fn wishlist_blueprint_flags_a_dangling_wire() {
    let root = workspace();
    let list = root.join("dangling.wishes");
    fs::write(&list, "a crate api\na dependency api->logging\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--blueprint",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    assert!(
        stdout.contains("dangling wire"),
        "an edge to an undeclared component is flagged: {stdout}"
    );
    assert!(stdout.contains("logging"), "names the dangling endpoint: {stdout}");
}

#[test]
fn wishlist_all_realized_exits_0() {
    let root = workspace();
    let list = root.join("done.wishes");
    fs::write(&list, "a module alpha\na function f\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    assert!(stdout.contains("realized 2/2"), "{stdout}");
    assert_eq!(out.status.code(), Some(0), "all realized → exit 0: {stdout}");
}

#[test]
fn wishlist_json_emits_a_project_gauge() {
    let root = workspace();
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na module beta\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("valid JSON ({e}): {stdout}"));
    assert_eq!(v["realized"], 1, "one of two realized: {stdout}");
    assert_eq!(v["total"], 2, "{stdout}");
    assert!(v["wishes"].is_array(), "per-wish rows: {stdout}");
}

#[test]
fn wishlist_since_gates_a_project_regression() {
    let root = workspace(); // module alpha + function f present
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na function f\n").unwrap(); // both realized
    let base = root.join("base.json");

    // Snapshot the baseline reading (all realized) via --json.
    let snap = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (snapshot)");
    let snap_out = String::from_utf8_lossy(&snap.stdout);
    if snap_out.contains("could not observe") {
        return;
    }
    assert_eq!(snap.status.code(), Some(0), "baseline all realized: {snap_out}");
    fs::write(&base, snap_out.as_ref()).unwrap();

    // The change breaks module alpha. --since detects the project regression.
    fs::write(root.join("src/lib.rs"), "pub fn f() {}\n").unwrap();
    fs::remove_file(root.join("src/alpha.rs")).unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--since",
            base.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--since)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("delta since baseline"), "{stdout}");
    assert!(stdout.contains("regressed 1"), "{stdout}");
    assert!(stdout.contains("a module alpha"), "names the broken wish: {stdout}");
    assert_eq!(out.status.code(), Some(2), "a project regression exits 2: {stdout}");
}

#[test]
fn wishlist_since_reports_progress_without_regression() {
    let root = workspace(); // alpha + f present; beta absent
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na module beta\n").unwrap();
    let base = root.join("base.json");
    let snap = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (snapshot)");
    let snap_out = String::from_utf8_lossy(&snap.stdout);
    if snap_out.contains("could not observe") {
        return;
    }
    fs::write(&base, snap_out.as_ref()).unwrap();

    // Add beta → newly realized, nothing broke → all realized → exit 0.
    fs::write(root.join("src/lib.rs"), "pub mod alpha;\npub mod beta;\npub fn f() {}\n").unwrap();
    fs::write(root.join("src/beta.rs"), "pub fn b() {}\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--since",
            base.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--since)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("newly realized 1"), "{stdout}");
    assert!(stdout.contains("a module beta"), "{stdout}");
    assert_eq!(out.status.code(), Some(0), "progress, no regression → exit 0: {stdout}");
}

#[test]
fn wishlist_scaffold_previews_the_plan_without_writing() {
    let root = workspace(); // module alpha + function f present
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na module delta\n").unwrap(); // delta missing
    let before = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--scaffold",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--scaffold)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    assert!(stdout.contains("realized 1/2"), "{stdout}");
    assert!(stdout.contains("scaffold"), "the closure plan is previewed: {stdout}");
    // A dry run writes nothing.
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        before,
        "scaffold must not write"
    );
    assert!(!root.join("src/delta.rs").exists(), "no files created by a dry run");
}

#[test]
fn wishlist_apply_closes_the_project() {
    let root = workspace(); // module alpha + function f present
    let list = root.join("p.wishes");
    // gamma is missing; --apply should scaffold it (a structural facet, offline).
    fs::write(&list, "a module alpha\na module gamma\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--apply",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--apply)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        eprintln!("observe unavailable, skipping: {stdout}");
        return;
    }
    assert!(stdout.contains("realized 2/2"), "the project was closed: {stdout}");
    assert_eq!(out.status.code(), Some(0), "all realized after --apply → exit 0: {stdout}");
    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(
        lib.contains("gamma") || root.join("src/gamma.rs").exists(),
        "gamma was erected: {stdout}"
    );
}

#[test]
fn wishlist_apply_shows_a_build_account() {
    let root = workspace(); // module alpha + function f present
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na module engine\n").unwrap(); // engine missing
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--apply",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run (--apply)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    // The final gauge plus the before→after account of the build itself.
    assert!(stdout.contains("realized 2/2"), "the project was closed: {stdout}");
    assert!(stdout.contains("what --apply built"), "the build account renders: {stdout}");
    assert!(stdout.contains("newly realized 1"), "engine was closed this run: {stdout}");
    assert!(stdout.contains("a module engine"), "the account names what it built: {stdout}");
}

#[test]
fn wishlist_coverage_flags_a_structure_only_dod() {
    let root = workspace(); // module alpha + function f — existence only
    let list = root.join("p.wishes");
    fs::write(&list, "a module alpha\na function f\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    assert!(stdout.contains("coverage:"), "the DoD coverage is shown: {stdout}");
    assert!(stdout.contains("existence"), "{stdout}");
    assert!(
        stdout.contains("no behaviour or run"),
        "an existence-only DoD is honestly flagged: {stdout}"
    );
}

#[test]
fn wishlist_is_exclusive_with_wish() {
    let root = workspace();
    let list = root.join("x.wishes");
    fs::write(&list, "a module alpha\n").unwrap();
    let out = kosmo_run()
        .args([
            "--wishlist",
            list.to_str().unwrap(),
            "--wish",
            "a module alpha",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success(), "two doors in one run is refused");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("exclusive"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
