//! Run 32 — the instrument is polyglot. The same wish prose measures a Python
//! workspace: modules (`.py` files, `__init__.py` = its package), functions and
//! classes (`def`/`class`), and docstrings. Existence and Shape are
//! language-agnostic; only the observation backend differs.
//!
//! (This corrects Run 31's "Rust-only" framing — a lexical Python backend was
//! already present and live; the deeper strata that need a toolchain — behaviour,
//! run — remain Rust-specific.)

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn kosmo_run() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kosmo-run"))
}

/// A Python package: a documented module `greeter` with a documented function
/// `greet` and a class `Greeter`. No manifest, no toolchain — purely lexical.
fn py_workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-py-{nanos}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("greeter.py"),
        r#""""The greeter module."""

def greet(name):
    """Return a greeting."""
    return name

class Greeter:
    pass
"#,
    )
    .unwrap();
    root
}

#[test]
fn the_instrument_measures_a_python_workspace() {
    let root = py_workspace();
    let out = kosmo_run()
        .args([
            "--wish",
            "a module greeter and a function greet and a doc for greet and a type Greeter",
            "--flat",
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
    assert!(stdout.contains("REALIZED"), "the Python facets are all present: {stdout}");
    assert_eq!(out.status.code(), Some(0), "a realized polyglot wish exits 0: {stdout}");
}

#[test]
fn an_absent_python_facet_is_unmet() {
    let root = py_workspace();
    let out = kosmo_run()
        .args([
            "--wish",
            "a function missing",
            "--flat",
            "--no-color",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return;
    }
    assert_eq!(out.status.code(), Some(1), "an absent function is unmet: {stdout}");
    assert!(stdout.contains("missing"), "the gap is named: {stdout}");
}

/// A JavaScript workspace: a `math` module with a `add` function and a
/// `Calculator` class. No manifest, no Node — purely lexical via the shared
/// xlang extractor (Run 33).
fn js_workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-js-{nanos}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("math.js"),
        "function add(a, b) {\n  return a + b;\n}\n\nclass Calculator {}\n",
    )
    .unwrap();
    root
}

#[test]
fn the_instrument_measures_a_javascript_workspace() {
    let root = js_workspace();
    let out = kosmo_run()
        .args([
            "--wish",
            "a module math and a function add and a type Calculator",
            "--flat",
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
    assert!(stdout.contains("REALIZED"), "the JavaScript facets are present: {stdout}");
    assert_eq!(out.status.code(), Some(0), "a realized JS wish exits 0: {stdout}");
}
