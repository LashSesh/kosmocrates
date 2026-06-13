//! Reforge — the external-empiricism bench.
//!
//! The Prüfstand judges the loop against *our own* corpus; the reforge
//! judges it against the world. Each target names a binary **this
//! repository did not write** (`expr`, `factor`, `basename` — POSIX/
//! coreutils tools present on any Unix): the harness first runs that
//! oracle to collect ground truth for a fixed probe set, builds a wish
//! consisting purely of measurable runtime expectations (`Run` facets with
//! exit, output and a time budget), and then asks the loop to **re-forge**
//! a behaviourally equivalent tool from an empty workspace. The verdict is
//! observed, never claimed: the forged binary is executed under the
//! sandbox witness against the oracle's own answers.
//!
//! Honesty about the boundary: re-forging *implements behaviour*, which
//! the deterministic scaffolder cannot — so the bench requires a real
//! provider (the one sanctioned non-deterministic worker) and is therefore
//! a **protocol**, reproducible by anyone with a key in one command, not a
//! CI fixture. What CI pins offline is the harness itself: oracle probing,
//! wish construction, refusal without a provider, report shape. A missing
//! oracle skips its target honestly; a failed forge is a failure, never a
//! warning.

use std::path::PathBuf;
use std::process::Command;

use kosmo_core::{Digest, Wish, WishClosureStatus, WishFacet, WishPredicate};
use kosmo_synthesizer::ActionSynthesizer;

use crate::descend_to_wish;

/// One reforge target: an external oracle plus the argv probe set whose
/// answers define behavioural equivalence.
pub struct ReforgeTarget {
    pub name: &'static str,
    /// The oracle binary (resolved via PATH) — not written by this repo.
    pub tool: &'static str,
    /// Probe argv sets; expected outputs are read from the oracle at run
    /// time, never hard-coded (the truth stays external).
    pub probes: &'static [&'static [&'static str]],
}

/// The reference targets: argv-pure, single-line-output POSIX/coreutils
/// tools, so ground truth is collectable on any Unix without a network.
pub fn reference_targets() -> Vec<ReforgeTarget> {
    vec![
        ReforgeTarget {
            name: "expr-add",
            tool: "expr",
            probes: &[&["3", "+", "4"], &["10", "+", "32"], &["0", "+", "0"]],
        },
        ReforgeTarget {
            name: "factor",
            tool: "factor",
            probes: &[&["12"], &["30"], &["97"]],
        },
        ReforgeTarget {
            name: "basename",
            tool: "basename",
            probes: &[&["/usr/lib/libc.so"], &["/a/b/c.txt"], &["plain"]],
        },
    ]
}

/// One probe's externally collected truth.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ProbeTruth {
    pub args: Vec<String>,
    pub expected: String,
}

/// Run the oracle over every probe and collect its answers. `None` when the
/// tool is missing or any probe yields no clean single-line answer — the
/// target is then skipped honestly (no invented truth, ever).
pub fn probe_oracle(tool: &str, probes: &[&[&str]]) -> Option<Vec<ProbeTruth>> {
    let mut truths = Vec::with_capacity(probes.len());
    for probe in probes {
        let output = Command::new(tool).args(*probe).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let answer = stdout.trim();
        if answer.is_empty() || answer.contains('\n') || answer.contains(',') {
            // The Run-facet grammar is argv/single-line; a truth it cannot
            // carry must not be approximated.
            return None;
        }
        truths.push(ProbeTruth {
            args: probe.iter().map(|s| s.to_string()).collect(),
            expected: answer.to_string(),
        });
    }
    Some(truths)
}

/// Build the reforge wish: one budgeted `Run` facet per probe —
/// `"args=>exit:0,out~<oracle answer>,ms<60000"`. Constructed directly
/// (no prose grammar involved) and evidence-bound to the collected truth.
pub fn wish_for(target: &ReforgeTarget, truths: &[ProbeTruth]) -> Wish {
    let evidence = Digest::of(&(target.tool, truths));
    let predicates = truths.iter().map(|t| {
        WishPredicate::require(WishFacet::run(format!(
            "{}=>exit:0,out~{},ms<60000",
            t.args.join(","),
            t.expected
        )))
    });
    Wish::new(
        format!("reforge {} (oracle: {})", target.name, target.tool),
        predicates,
        Digest::ZERO,
        evidence,
    )
}

/// One target's outcome. `realized: None` means skipped (oracle missing).
#[derive(Debug, serde::Serialize)]
pub struct TargetOutcome {
    pub name: &'static str,
    pub tool: &'static str,
    pub wish_id: String,
    pub realized: Option<bool>,
    pub iterations: usize,
    pub probes: Vec<ProbeTruth>,
}

/// The whole bench's outcome, content-addressed over its own content.
#[derive(Debug, serde::Serialize)]
pub struct ReforgeReport {
    pub provider: String,
    pub outcomes: Vec<TargetOutcome>,
}

impl ReforgeReport {
    pub fn attempted(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.realized.is_some())
            .count()
    }
    pub fn forged(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.realized == Some(true))
            .count()
    }
    pub fn skipped(&self) -> usize {
        self.outcomes.len() - self.attempted()
    }
    /// No attempted target failed (skips are honest, not failures).
    pub fn is_faithful(&self) -> bool {
        self.outcomes.iter().all(|o| o.realized != Some(false))
    }
    /// The report as pretty JSON, prefixed by its own content address.
    pub fn to_json(&self) -> String {
        let body = serde_json::to_value(self).unwrap_or_default();
        let report_id = Digest::of(&body);
        serde_json::to_string_pretty(&serde_json::json!({
            "report_id": report_id.to_hex(),
            "report": body,
        }))
        .unwrap_or_default()
    }
    pub fn render(&self, color: bool) -> String {
        let (green, red, dim, reset) = if color {
            ("\x1b[32m", "\x1b[31m", "\x1b[2m", "\x1b[0m")
        } else {
            ("", "", "", "")
        };
        let mut out = String::from("Kosmocrates reforge — external oracles\n");
        for o in &self.outcomes {
            let line = match o.realized {
                Some(true) => format!(
                    "  {green}\u{2713}{reset} {:<10} oracle {:<9} re-forged \
                     ({} probe(s), {} observation(s))",
                    o.name,
                    o.tool,
                    o.probes.len(),
                    o.iterations
                ),
                Some(false) => format!(
                    "  {red}\u{2717}{reset} {:<10} oracle {:<9} NOT re-forged \
                     ({} probe(s), {} observation(s))",
                    o.name,
                    o.tool,
                    o.probes.len(),
                    o.iterations
                ),
                None => format!(
                    "  {dim}\u{2013} {:<10} oracle {:<9} skipped (oracle unavailable){reset}",
                    o.name, o.tool
                ),
            };
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str(&format!(
            "  re-forged: {}/{} attempted ({} skipped) \u{b7} provider {}\n",
            self.forged(),
            self.attempted(),
            self.skipped(),
            self.provider
        ));
        out
    }
}

/// A throwaway bin workspace for one forge.
fn forge_workspace(name: &str) -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-reforge-{name}-{nanos}"));
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"reforged\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n")?;
    Ok(root)
}

/// Run the bench: per target, collect the oracle's truth, build the wish,
/// and descend with the provider-backed fallback until the forged binary
/// answers like the oracle — or honestly does not.
pub fn run_reforge(fallback: &dyn ActionSynthesizer, provider: &str) -> ReforgeReport {
    let mut outcomes = Vec::new();
    for target in reference_targets() {
        let Some(truths) = probe_oracle(target.tool, target.probes) else {
            outcomes.push(TargetOutcome {
                name: target.name,
                tool: target.tool,
                wish_id: String::new(),
                realized: None,
                iterations: 0,
                probes: vec![],
            });
            continue;
        };
        let wish = wish_for(&target, &truths);
        let (realized, iterations) = match forge_workspace(target.name) {
            Ok(root) => {
                let descent = descend_to_wish(
                    root.to_str().unwrap_or("."),
                    &wish,
                    wish.evidence_bundle_id,
                    false,
                    8,
                    Some(fallback),
                    None,
                );
                std::fs::remove_dir_all(&root).ok();
                match descent {
                    Ok(session) => (
                        Some(
                            session
                                .latest()
                                .is_some_and(|a| matches!(a.status, WishClosureStatus::Realized)),
                        ),
                        session.iterations(),
                    ),
                    Err(_) => (Some(false), 0),
                }
            }
            Err(_) => (None, 0),
        };
        outcomes.push(TargetOutcome {
            name: target.name,
            tool: target.tool,
            wish_id: wish.id.to_hex(),
            realized,
            iterations,
            probes: truths,
        });
    }
    ReforgeReport {
        provider: provider.to_string(),
        outcomes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_are_wellformed_and_unique() {
        let targets = reference_targets();
        assert!(targets.len() >= 3);
        let mut names: Vec<&str> = targets.iter().map(|t| t.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), targets.len(), "target names must be unique");
        for t in &targets {
            assert!(!t.probes.is_empty(), "{} has no probes", t.name);
            for p in t.probes {
                assert!(!p.is_empty(), "{} has an empty probe", t.name);
                for arg in *p {
                    assert!(!arg.contains(','), "argv must survive the key grammar");
                }
            }
        }
    }

    #[test]
    fn probing_a_real_oracle_collects_its_truth() {
        // `echo` exists on every Unix CI runner — a stand-in oracle.
        let truths = probe_oracle("echo", &[&["hi"], &["forge"]]).expect("echo is present");
        assert_eq!(truths.len(), 2);
        assert_eq!(truths[0].expected, "hi");
        assert_eq!(truths[1].expected, "forge");
    }

    #[test]
    fn missing_or_uncarryable_oracles_are_none_never_invented() {
        assert!(
            probe_oracle("kosmo-no-such-oracle-xyz", &[&["1"]]).is_none(),
            "a missing tool yields no truth"
        );
        // `seq 1 2` answers in two lines — the Run grammar cannot carry it,
        // so the truth is refused rather than approximated.
        assert!(probe_oracle("seq", &[&["1", "2"]]).is_none());
    }

    #[test]
    fn the_wish_is_budgeted_runtime_expectations_bound_to_the_truth() {
        let target = &reference_targets()[0];
        let truths = vec![ProbeTruth {
            args: vec!["3".into(), "+".into(), "4".into()],
            expected: "7".into(),
        }];
        let wish = wish_for(target, &truths);
        assert_eq!(wish.predicate_count(), 1);
        assert_eq!(
            wish.predicates[0].facet,
            WishFacet::run("3,+,4=>exit:0,out~7,ms<60000"),
            "exit + output + time budget, all measured at execution"
        );
        assert!(wish.is_evidence_bound(), "bound to the collected truth");
        // Determinism: same truth, same wish identity.
        assert_eq!(wish_for(target, &truths).id, wish.id);
    }

    #[test]
    fn report_counts_render_and_serialize() {
        let report = ReforgeReport {
            provider: "test".into(),
            outcomes: vec![
                TargetOutcome {
                    name: "a",
                    tool: "x",
                    wish_id: "00".into(),
                    realized: Some(true),
                    iterations: 3,
                    probes: vec![],
                },
                TargetOutcome {
                    name: "b",
                    tool: "y",
                    wish_id: String::new(),
                    realized: None,
                    iterations: 0,
                    probes: vec![],
                },
            ],
        };
        assert_eq!(report.attempted(), 1);
        assert_eq!(report.forged(), 1);
        assert_eq!(report.skipped(), 1);
        assert!(report.is_faithful());
        let text = report.render(false);
        assert!(
            text.contains("re-forged: 1/1 attempted (1 skipped)"),
            "{text}"
        );
        let json = report.to_json();
        assert!(json.contains("report_id"), "{json}");
        assert!(json.contains("\"provider\": \"test\""), "{json}");
    }
}
