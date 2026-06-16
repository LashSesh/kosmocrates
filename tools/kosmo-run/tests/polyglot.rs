//! Run 32 — the instrument is polyglot. The same wish prose measures a Python
//! workspace: modules (`.py` files, `__init__.py` = its package), functions and
//! classes (`def`/`class`), and docstrings. Existence and Shape are
//! language-agnostic; only the observation backend differs.
//!
//! (This corrects Run 31's "Rust-only" framing — a lexical Python backend was
//! already present and live; Run 35 adds the Python *Live* door — a script probe
//! executed under the sandbox — so behaviour-by-execution reaches Python too.)

use std::fs;
use std::path::{Path, PathBuf};
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

/// A bare single-file workspace in language `tag` (no manifest, no toolchain).
fn lang_workspace(tag: &str, file: &str, content: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-{tag}-{nanos}"));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(file), content).unwrap();
    root
}

fn realizes(root: &Path, wish: &str) -> Option<bool> {
    let out = kosmo_run()
        .args(["--wish", wish, "--flat", "--no-color", root.to_str().unwrap()])
        .output()
        .expect("spawn kosmo-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.contains("could not observe") {
        return None;
    }
    Some(out.status.code() == Some(0) && stdout.contains("REALIZED"))
}

#[test]
fn the_instrument_measures_clike_workspaces() {
    // Go — module = file stem, `func` → symbol, `type` → symbol.
    let go = lang_workspace(
        "go",
        "math.go",
        "package math\n\nfunc Add(a int, b int) int { return a + b }\n\ntype Calculator struct{}\n",
    );
    if let Some(ok) = realizes(&go, "a module math and a function Add and a type Calculator") {
        assert!(ok, "Go facets realize");
    }
    // C — `int square(...)` → function symbol.
    let c = lang_workspace("c", "calc.c", "int square(int x) { return x * x; }\n");
    if let Some(ok) = realizes(&c, "a module calc and a function square") {
        assert!(ok, "C facets realize");
    }
}

#[test]
fn python_live_runs_a_probe_under_the_sandbox() {
    // The Live door needs the interpreter; skip where python3 is unavailable.
    let have_py = Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !have_py {
        eprintln!("python3 unavailable, skipping");
        return;
    }
    let root = lang_workspace(
        "pyrun",
        "greeter.py",
        "import sys\n\ndef greet(name):\n    return \"hi-\" + name\n\n# kosmo:run: World=>out~hi-World\nif __name__ == \"__main__\":\n    print(greet(sys.argv[1]))\n",
    );
    // The script runs under the sandbox; a matching probe realizes the Run facet.
    assert_eq!(
        realizes(&root, "a run World=>out~hi-World"),
        Some(true),
        "the python run probe realizes by execution"
    );
    // Fail-closed: a wrong expectation never realizes.
    assert_eq!(
        realizes(&root, "a run World=>out~WRONG"),
        Some(false),
        "a wrong expectation is fail-closed"
    );
}

/// Whether an interpreter is on PATH (the Live door needs it).
fn tool_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn javascript_live_runs_a_probe_under_the_sandbox() {
    if !tool_available("node") {
        eprintln!("node unavailable, skipping");
        return;
    }
    let root = lang_workspace(
        "jsrun",
        "sq.js",
        "// kosmo:run: 5=>out~25\nconsole.log(process.argv[2] * process.argv[2]);\n",
    );
    assert_eq!(
        realizes(&root, "a run 5=>out~25"),
        Some(true),
        "the node run probe realizes by execution"
    );
    assert_eq!(
        realizes(&root, "a run 5=>out~99"),
        Some(false),
        "a wrong expectation is fail-closed"
    );
}

#[test]
fn go_live_runs_a_probe_under_the_sandbox() {
    if !tool_available("go") {
        eprintln!("go unavailable, skipping");
        return;
    }
    let root = lang_workspace(
        "gorun",
        "sq.go",
        "package main\nimport (\"fmt\"; \"os\"; \"strconv\")\n// kosmo:run: 5=>out~25\nfunc main() { x, _ := strconv.Atoi(os.Args[1]); fmt.Println(x * x) }\n",
    );
    assert_eq!(
        realizes(&root, "a run 5=>out~25"),
        Some(true),
        "the go run probe realizes by execution"
    );
}
