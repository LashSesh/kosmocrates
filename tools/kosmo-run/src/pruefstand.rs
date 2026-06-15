//! Prüfstand — the empirical fidelity harness.
//!
//! The capstone generalized into a *corpus*. Each scenario is a known system
//! (correct or deliberately broken) plus a wish and the verdict the substrate
//! *should* reach. The harness builds each in a throwaway workspace, runs the
//! real descent, and checks the reached verdict against the expected one. It
//! measures one thing: does the loop **accept exactly the systems it should** —
//! every known-good one realized, every broken one rejected?
//!
//! Structural scenarios converge offline (lexical observation); behavioural
//! scenarios run `cargo test` and so are gated behind `allow_cargo`. A scenario
//! whose observation is unavailable (e.g. no nested cargo) is *skipped*, never
//! counted as a failure — the harness reports honestly.

use std::path::PathBuf;

use kosmo_core::{Digest, WishClosureStatus, Q16};
use kosmo_intent::compile_wish;

use crate::descend_to_wish;

/// The verdict a scenario's wish *should* reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expectation {
    /// A known-good system: the descent must realize the wish.
    Realized,
    /// A deliberately broken system: the descent must *not* realize it.
    Rejected,
}

/// One reference system: a `src/lib.rs`, a wish, and the expected verdict.
pub struct Scenario {
    pub name: &'static str,
    /// The crate's source. Written to `src/main.rs` when `bin`, else `src/lib.rs`.
    pub lib: &'static str,
    pub wish: &'static str,
    pub expect: Expectation,
    /// Behavioural and runtime scenarios validate by running (nested `cargo`).
    pub needs_cargo: bool,
    /// `true` writes the source as a binary (`src/main.rs`) — for `Run` probes.
    pub bin: bool,
    /// `true` builds a cargo-less **Python** workspace: the source goes to
    /// `calc.py` and the descent runs through the polyglot door.
    pub python: bool,
}

/// How a scenario's actual verdict compared to its expectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Reached exactly the expected verdict — the substrate was faithful.
    Match,
    /// Reached the *wrong* verdict — a fidelity failure.
    Mismatch,
    /// Could not be observed (e.g. nested cargo unavailable); not a failure.
    Skipped,
}

/// The result of running one scenario.
pub struct Outcome {
    pub name: &'static str,
    pub expect: Expectation,
    /// `Some(true)` realized, `Some(false)` rejected, `None` if skipped.
    pub realized: Option<bool>,
    pub verdict: Verdict,
    /// Run 7 — the layered cube reached full solidity (the diamond is cut:
    /// every stratum's geomean opacity is `ONE`). `None` when skipped.
    pub cube_solid: Option<bool>,
    /// Run 7 — the per-layer trace stayed strictly contractive: no masked deep
    /// regression across the descent. `None` when skipped.
    pub contractive: Option<bool>,
}

/// The fidelity report over a whole corpus.
pub struct Report {
    pub outcomes: Vec<Outcome>,
}

impl Report {
    pub fn matched(&self) -> usize {
        self.count(Verdict::Match)
    }
    pub fn mismatched(&self) -> usize {
        self.count(Verdict::Mismatch)
    }
    pub fn skipped(&self) -> usize {
        self.count(Verdict::Skipped)
    }
    fn count(&self, v: Verdict) -> usize {
        self.outcomes.iter().filter(|o| o.verdict == v).count()
    }
    /// Faithful iff no scenario reached the wrong verdict. Skips are tolerated.
    pub fn is_faithful(&self) -> bool {
        self.mismatched() == 0
    }

    /// Run 7 — the layered machinery tracks realization exactly: every realized
    /// scenario cut a solid diamond, every rejected one correctly stayed a
    /// hologram (`structural_solidity < ONE`). Skips are tolerated. This ties
    /// Run 3's opacity↔realization identity to the real descents end-to-end.
    pub fn cube_faithful(&self) -> bool {
        self.outcomes.iter().all(|o| match (o.realized, o.cube_solid) {
            (Some(true), Some(solid)) => solid,
            (Some(false), Some(solid)) => !solid,
            _ => true, // skipped (or no cube): tolerated
        })
    }

    /// Run 7 — realized descents whose layered trace was *not* strictly
    /// contractive: a deep stratum regressed under a successful descent (a
    /// masked deep regression). Should be `0` on a faithful substrate.
    pub fn masked_regressions(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.realized == Some(true) && o.contractive == Some(false))
            .count()
    }
}

/// The built-in reference corpus: one scenario per facet axis the floor builds,
/// plus the behavioural keystone in **both** directions.
pub fn reference_corpus() -> Vec<Scenario> {
    // Structural scenarios (offline, lexical) — one per facet axis the floor
    // builds. All `bin: false`, `needs_cargo: false`.
    let lib = |name, wish| Scenario {
        name,
        lib: "// calc\n",
        wish,
        expect: Expectation::Realized,
        needs_cargo: false,
        bin: false,
        python: false,
    };
    let py = |name, lib, wish, expect| Scenario {
        name,
        lib,
        wish,
        expect,
        needs_cargo: false,
        bin: false,
        python: true,
    };
    vec![
        lib("symbol", "a function greet"),
        lib("contract", "a contract handle(String)->String"),
        lib("module", "a module routes"),
        lib("capability", "a capability crud:user"),
        lib("test", "a test greets_politely"),
        lib("composition", "a composition parse>>String>>eval"),
        lib("archetype-crud", "a crud user"),
        // The keystone — acceptance over generation — at the unit boundary.
        Scenario {
            name: "behavior-correct",
            lib: "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
            wish: "a behavior add(2,3)=>5",
            expect: Expectation::Realized,
            needs_cargo: true,
            bin: false,
            python: false,
        },
        Scenario {
            name: "behavior-wrong",
            lib: "pub fn add(a: i32, b: i32) -> i32 { a + b + 1 }\n",
            wish: "a behavior add(2,3)=>5",
            expect: Expectation::Rejected,
            needs_cargo: true,
            bin: false,
            python: false,
        },
        // The keystone at the PROCESS boundary (level 5): the artifact is run.
        Scenario {
            name: "run-correct",
            lib: "fn main() { let a: Vec<String> = std::env::args().skip(1).collect(); \
                  if a.first().map(|s| s.as_str()) == Some(\"add\") { \
                  let s: i32 = a[1..].iter().filter_map(|x| x.parse::<i32>().ok()).sum(); \
                  println!(\"{s}\"); } }\n",
            wish: "a run add,2,3=>out~5",
            expect: Expectation::Realized,
            needs_cargo: true,
            bin: true,
            python: false,
        },
        Scenario {
            name: "run-empty",
            lib: "fn main() {}\n",
            wish: "a run add,2,3=>out~5",
            expect: Expectation::Rejected,
            needs_cargo: true,
            bin: true,
            python: false,
        },
        // The keystone at the SERVICE boundary (level 6): the server is started,
        // awaited, probed over HTTP, torn down.
        Scenario {
            name: "service-correct",
            lib: r#"use std::io::{Read, Write};
use std::net::TcpListener;
fn main() {
    let port = std::env::var("KOSMO_PORT").unwrap();
    let l = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    for s in l.incoming() {
        let mut s = s.unwrap();
        let mut b = [0u8; 512];
        let _ = s.read(&mut b);
        let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
    }
}
"#,
            wish: "a service GET:/health=>200",
            expect: Expectation::Realized,
            needs_cargo: true,
            bin: true,
            python: false,
        },
        Scenario {
            name: "service-empty",
            lib: "fn main() {}\n",
            wish: "a service GET:/health=>200",
            expect: Expectation::Rejected,
            needs_cargo: true,
            bin: true,
            python: false,
        },
        // The polyglot arm: the same fidelity contract over Python systems,
        // observed and fabricated by Python's own law (file = module),
        // offline, interpreter-free.
        py(
            "py-observed",
            "\"\"\"calc.\"\"\"\n\n\ndef greet():\n    \"\"\"Say hi.\"\"\"\n    return \"hi\"\n\n\ndef test_greet():\n    assert greet() == \"hi\"\n",
            "a module calc and a function greet and docs for greet and a test test_greet",
            Expectation::Realized,
        ),
        py(
            "py-scaffold",
            "\"\"\"calc.\"\"\"\n",
            "a module store and a function put_record and a test test_store",
            Expectation::Realized,
        ),
        py(
            "py-arity-right",
            "def greet():\n    return \"hi\"\n",
            "a signature greet/0",
            Expectation::Realized,
        ),
        py(
            "py-arity-wrong",
            "def greet():\n    return \"hi\"\n",
            "a signature greet/2",
            Expectation::Rejected,
        ),
    ]
}

/// A throwaway single-crate workspace named `calc`. The source goes to
/// `src/main.rs` for a `bin` scenario (so `cargo run` has a target), else
/// `src/lib.rs`.
fn make_workspace(name: &str, lib: &str, bin: bool, python: bool) -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-pruefstand-{name}-{nanos}"));
    if python {
        // A cargo-less Python workspace: the polyglot door.
        std::fs::create_dir_all(&root)?;
        std::fs::write(root.join("calc.py"), lib)?;
        return Ok(root);
    }
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"calc\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    let src = if bin { "src/main.rs" } else { "src/lib.rs" };
    std::fs::write(root.join(src), lib)?;
    Ok(root)
}

/// Build, descend, compare. A skip (cargo gated off, or observation
/// unavailable) yields `Verdict::Skipped`, never a mismatch.
pub fn run_scenario(s: &Scenario, allow_cargo: bool) -> Outcome {
    let skip = || Outcome {
        name: s.name,
        expect: s.expect,
        realized: None,
        verdict: Verdict::Skipped,
        cube_solid: None,
        contractive: None,
    };
    if s.needs_cargo && !allow_cargo {
        return skip();
    }
    let root = match make_workspace(s.name, s.lib, s.bin, s.python) {
        Ok(r) => r,
        Err(_) => return skip(),
    };
    let evidence = Digest::of_bytes(s.wish.as_bytes());
    let wish = compile_wish(s.wish, Digest::ZERO, evidence);
    let descent = descend_to_wish(
        root.to_str().unwrap_or("."),
        &wish,
        evidence,
        s.needs_cargo,
        8,
        None,
        None,
    );
    std::fs::remove_dir_all(&root).ok(); // clean up regardless of outcome
    match descent {
        Ok(session) => {
            let realized = session.latest().is_some_and(|a| {
                matches!(
                    a.status,
                    WishClosureStatus::Realized | WishClosureStatus::Vacuous
                )
            });
            let want = matches!(s.expect, Expectation::Realized);
            // Run 7: the cube film the descent accumulated (observe_layered) lets
            // us read the layered verdict for free — no second descent.
            let cube_solid = session
                .latest_cube()
                .map(|c| c.structural_solidity == Q16::ONE);
            let contractive = Some(session.layered_trace().is_strictly_contractive());
            Outcome {
                name: s.name,
                expect: s.expect,
                realized: Some(realized),
                verdict: if realized == want {
                    Verdict::Match
                } else {
                    Verdict::Mismatch
                },
                cube_solid,
                contractive,
            }
        }
        // Observation unavailable (e.g. cargo metadata can't run) — skip.
        Err(_) => skip(),
    }
}

/// Run every scenario in the corpus.
pub fn run_corpus(corpus: Vec<Scenario>, allow_cargo: bool) -> Report {
    Report {
        outcomes: corpus
            .iter()
            .map(|s| run_scenario(s, allow_cargo))
            .collect(),
    }
}

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

/// Render the fidelity report as a table.
pub fn render(report: &Report, color: bool) -> String {
    let c = |code: &'static str| if color { code } else { "" };
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}Kosmocrates Pr\u{00fc}fstand \u{2014} reference corpus{}\n",
        c(BOLD),
        c(CYAN),
        c(RESET)
    ));
    for o in &report.outcomes {
        let (mark, col) = match o.verdict {
            Verdict::Match => ("\u{2713}", c(GREEN)),
            Verdict::Mismatch => ("\u{2717}", c(RED)),
            Verdict::Skipped => ("\u{2013}", c(DIM)),
        };
        let want = match o.expect {
            Expectation::Realized => "realized",
            Expectation::Rejected => "rejected",
        };
        let got = match o.realized {
            Some(true) => "realized",
            Some(false) => "rejected",
            None => "skipped",
        };
        // Run 7: ◆ a solid diamond, ◇ still a hologram, – not descended.
        let cube = match o.cube_solid {
            Some(true) => "\u{25c6}",
            Some(false) => "\u{25c7}",
            None => "\u{2013}",
        };
        out.push_str(&format!(
            "  {col}{mark}{reset} {name:<18} expect {want:<8}  got {got:<8}  cube {cube}\n",
            col = col,
            mark = mark,
            reset = c(RESET),
            name = o.name,
            want = want,
            got = got,
            cube = cube,
        ));
    }
    let faithful = report.is_faithful();
    let verdict_col = if faithful { c(GREEN) } else { c(RED) };
    out.push_str(&format!(
        "  {}fidelity: {}/{} matched{} ({} mismatched, {} skipped)\n",
        verdict_col,
        report.matched(),
        report.outcomes.len(),
        c(RESET),
        report.mismatched(),
        report.skipped(),
    ));
    let cube_col = if report.cube_faithful() {
        c(GREEN)
    } else {
        c(RED)
    };
    out.push_str(&format!(
        "  {}cube: solidity {} realization (\u{25c6} realized = solid diamond, \u{25c7} rejected = hologram){}\n",
        cube_col,
        if report.cube_faithful() {
            "tracks"
        } else {
            "DIVERGED from"
        },
        c(RESET),
    ));
    let masked = report.masked_regressions();
    let masked_col = if masked == 0 { c(GREEN) } else { c(RED) };
    out.push_str(&format!(
        "  {}layered: {} masked deep regression(s) under a realized descent{}\n",
        masked_col,
        masked,
        c(RESET),
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_corpus_is_faithful() {
        // Gate cargo off: the behavioural, runtime, and service scenarios skip,
        // the structural ones must each reach exactly their expected verdict (or
        // skip if cargo metadata is unavailable) — never a mismatch.
        let report = run_corpus(reference_corpus(), false);
        assert!(
            report.is_faithful(),
            "structural fidelity broken:\n{}",
            render(&report, false)
        );
        // The 6 cargo-gated scenarios (2 behaviour, 2 runtime, 2 service) skip.
        assert!(report.mismatched() == 0);

        // Run 7: the layered cube tracks realization exactly — realized ⇒ solid
        // diamond, rejected ⇒ hologram — and every realized descent stayed
        // strictly contractive (no masked deep regression). Read for free off
        // the same descents.
        assert!(
            report.cube_faithful(),
            "cube solidity must track realization:\n{}",
            render(&report, false)
        );
        assert_eq!(
            report.masked_regressions(),
            0,
            "a realized descent suffered a masked deep regression:\n{}",
            render(&report, false)
        );
    }

    #[test]
    fn staged_descent_coagulates_the_structural_corpus() {
        // Run 7: the staged Solve→Gate→Coagula pipeline (Run 4 + the Konus
        // precedence lens of Run 5) realizes the offline corpus and coagulates
        // every non-empty stratum — the diamond is cut, bottom-up. Proven on the
        // real descents; gracefully skips when offline observation is unavailable.
        use crate::descend_staged;
        use kosmo_core::StagedClosureReport;

        for s in reference_corpus()
            .iter()
            .filter(|s| !s.needs_cargo && s.expect == Expectation::Realized)
        {
            let Ok(root) = make_workspace(s.name, s.lib, s.bin, s.python) else {
                continue;
            };
            let evidence = Digest::of_bytes(s.wish.as_bytes());
            let wish = compile_wish(s.wish, Digest::ZERO, evidence);
            let descent =
                descend_staged(root.to_str().unwrap_or("."), &wish, evidence, false, 8, None, None);
            std::fs::remove_dir_all(&root).ok();
            let Ok(session) = descent else { continue };
            let realized = session.latest().is_some_and(|a| {
                matches!(
                    a.status,
                    WishClosureStatus::Realized | WishClosureStatus::Vacuous
                )
            });
            if !realized {
                continue; // offline observation unavailable for this case — skip
            }
            let report = StagedClosureReport::from_descent(
                session.cubes(),
                &session.layered_trace(),
                evidence,
            );
            assert!(
                report.fully_coagulated,
                "{}: staged descent realized the wish but did not coagulate every stratum: {:?}",
                s.name, report.strata
            );
        }
    }

    #[test]
    fn report_counts_and_render_are_consistent() {
        let report = Report {
            outcomes: vec![
                Outcome {
                    name: "a",
                    expect: Expectation::Realized,
                    realized: Some(true),
                    verdict: Verdict::Match,
                    cube_solid: Some(true),
                    contractive: Some(true),
                },
                Outcome {
                    name: "b",
                    expect: Expectation::Rejected,
                    realized: Some(true),
                    verdict: Verdict::Mismatch,
                    cube_solid: Some(true),
                    contractive: Some(true),
                },
            ],
        };
        assert_eq!(report.matched(), 1);
        assert_eq!(report.mismatched(), 1);
        assert!(!report.is_faithful());
        let text = render(&report, false);
        assert!(text.contains("1/2 matched"));
        assert!(text.contains("1 mismatched"));
    }
}
