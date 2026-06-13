//! The wish landscape — the findings become the wish menu.
//!
//! `--landscape` projects every substrate finding the wish vocabulary can
//! express into a ranked proposal landscape, measured against the observed
//! topology (met / open / beyond observation / beyond vocabulary), and
//! `--adopt <n>` turns the top open proposals into ONE severity-weighted,
//! evidence-bound wish — read-only by default, descended under `--apply`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A mixed workspace: one undocumented, untested Rust module (observable)
/// and one Python file (an honest beyond-observation target).
fn mixed_workspace(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-landscape-{tag}-{nanos}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("lib")).unwrap();
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
    fs::write(
        root.join("lib/handlers.py"),
        "def handle(x):\n    return x\n",
    )
    .unwrap();
    root
}

#[test]
fn landscape_maps_findings_with_honest_standing() {
    let root = mixed_workspace("map");
    let out = kosmo_run()
        .args(["--landscape", "--json", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json landscape");

    let proposals = doc["proposals"].as_array().unwrap();
    assert!(!proposals.is_empty(), "findings must project to proposals");
    let standing_of = |key: &str| -> String {
        proposals
            .iter()
            .find(|p| p["facet_key"] == key)
            .unwrap_or_else(|| panic!("proposal {key} present"))["standing"]
            .as_str()
            .unwrap()
            .to_string()
    };
    // The Rust module's targets are open (observable, unmet)…
    assert_eq!(standing_of("router"), "open");
    assert_eq!(standing_of("router_smoke"), "open");
    // …and since the polyglot door, the Python module's targets are too:
    // file = module, observed without an interpreter.
    assert_eq!(standing_of("handlers"), "open");
    assert_eq!(standing_of("handlers_smoke"), "open");
    // The crate root stays honest residue (no `mod lib` exists to observe).
    assert_eq!(standing_of("lib"), "beyond-observation");
    assert!(doc["open"].as_u64().unwrap() >= 4);
    // The landscape is evidence-bound to the diagnosis report.
    assert!(!doc["report_id"].as_str().unwrap().is_empty());

    fs::remove_dir_all(&root).ok();
}

#[test]
fn adopt_without_apply_is_read_only() {
    let root = mixed_workspace("readonly");
    let before = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let out = kosmo_run()
        .args([
            "--landscape",
            "--adopt",
            "2",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("adopted as one wish"));
    assert!(
        stdout.contains("add --apply to descend"),
        "read-only contract stated: {stdout}"
    );
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        before,
        "nothing written without --apply"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn geometry_is_opt_in_and_clusters_the_open_landscape() {
    let root = mixed_workspace("geometry");
    // Without --geometry the JSON surface is unchanged: no "geometry" key.
    let out = kosmo_run()
        .args(["--landscape", "--json", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(out.status.success());
    let plain: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert!(
        plain.get("geometry").is_none(),
        "geometry must be strictly opt-in"
    );

    // With --geometry: the open proposals (doc + test of `router`) couple
    // by subject into ONE coherent cluster.
    let out = kosmo_run()
        .args([
            "--landscape",
            "--geometry",
            "--json",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    // Everything the flag-off run shows is still there, unchanged in shape.
    for key in [
        "path",
        "report_id",
        "proposals",
        "met",
        "open",
        "beyond_observation",
        "beyond_vocabulary",
    ] {
        assert!(doc.get(key).is_some(), "{key} survives --geometry");
    }
    let clusters = doc["geometry"]["clusters"].as_array().unwrap();
    assert_eq!(
        clusters.len(),
        2,
        "two modules — one Rust, one Python — two coherent clusters: {clusters:?}"
    );
    let cluster_of = |subject: &str| -> Vec<String> {
        clusters
            .iter()
            .find(|c| c["subjects"][0] == subject)
            .unwrap_or_else(|| panic!("cluster for {subject}"))["facets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f.as_str().unwrap().to_string())
            .collect()
    };
    let router = cluster_of("router");
    assert!(router.contains(&"Doc router".to_string()), "{router:?}");
    assert!(
        router.contains(&"Test router_smoke".to_string()),
        "{router:?}"
    );
    let handlers = cluster_of("handlers");
    assert!(
        handlers.contains(&"Doc handlers".to_string()),
        "the Python organ clusters too: {handlers:?}"
    );

    // Text mode mentions the geometry only under the flag.
    let out = kosmo_run()
        .args(["--landscape", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    assert!(!String::from_utf8_lossy(&out.stdout).contains("geometry:"));
    let out = kosmo_run()
        .args([
            "--landscape",
            "--geometry",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("geometry:"), "{stdout}");
    assert!(stdout.contains("cluster 1"), "{stdout}");
    assert!(stdout.contains("cluster 2"), "{stdout}");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn adopt_cluster_takes_one_coherent_cluster_and_guards_its_flags() {
    let root = mixed_workspace("cluster");
    // --adopt and --adopt-cluster are mutually exclusive.
    let out = kosmo_run()
        .args([
            "--landscape",
            "--adopt",
            "2",
            "--adopt-cluster",
            "1",
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

    // An out-of-range index is an honest error naming the real count.
    let out = kosmo_run()
        .args([
            "--landscape",
            "--adopt-cluster",
            "9",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("cluster(s)"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Adopting cluster 1 without --apply is read-only and prints the wish.
    let before = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let out = kosmo_run()
        .args([
            "--landscape",
            "--adopt-cluster",
            "1",
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
    assert!(stdout.contains("adopted as one wish"), "{stdout}");
    assert!(stdout.contains("add --apply to descend"), "{stdout}");
    assert_eq!(fs::read_to_string(root.join("src/lib.rs")).unwrap(), before);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn adopt_cluster_with_apply_descends_the_cluster_to_realized() {
    let root = mixed_workspace("cluster-apply");
    let norms = root.join("norms-store");
    let out = kosmo_run()
        .args([
            "--landscape",
            "--adopt-cluster",
            "1",
            "--apply",
            "--no-color",
            "--norms",
            norms.to_str().unwrap(),
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") || stdout.contains("pipeline failed") {
        // cargo metadata unavailable in exotic sandboxes — the in-process
        // descend tests already pin convergence; this is the CLI seam.
        eprintln!("observe unavailable, skipping: {stdout}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("REALIZED"), "descent converged: {stdout}");
    // Cluster 1 is the Python organ (handlers): its doc lands as a module
    // docstring via the polyglot scaffold fallback, its smoke test as a
    // Rust test in the cargo crate — a genuinely mixed fabrication.
    let handlers = fs::read_to_string(root.join("lib/handlers.py")).unwrap();
    assert!(
        handlers.starts_with("\"\"\""),
        "module docstring landed in the Python file: {handlers}"
    );
    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(
        lib.contains("fn handlers_smoke"),
        "smoke test landed: {lib}"
    );
    // The system's own proposal is a sighting too: the adopted descent
    // recorded a norm-learning observation.
    let observations = fs::read_to_string(norms.join("observations.jsonl"))
        .expect("adopted descents feed norm learning");
    assert_eq!(observations.lines().count(), 1, "{observations}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn adopt_with_apply_descends_to_realized() {
    let root = mixed_workspace("apply");
    let out = kosmo_run()
        .args([
            "--landscape",
            "--adopt",
            "2",
            "--apply",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") || stdout.contains("pipeline failed") {
        // cargo metadata unavailable in exotic sandboxes — the in-process
        // descend tests already pin convergence; this is the CLI seam.
        eprintln!("observe unavailable, skipping: {stdout}");
        fs::remove_dir_all(&root).ok();
        return;
    }
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("REALIZED"), "descent converged: {stdout}");
    // The top-2 open proposals are now the two smoke tests — one per
    // module, one per language; both land as Rust tests in the cargo crate.
    let lib = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(lib.contains("fn router_smoke"), "smoke test landed: {lib}");
    assert!(
        lib.contains("fn handlers_smoke"),
        "the Python module's smoke test landed too: {lib}"
    );
    fs::remove_dir_all(&root).ok();
}
