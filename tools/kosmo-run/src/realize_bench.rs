//! The realization benchmark — does the generative loop actually work?
//!
//! Every other gate in this repository is deterministic and offline: the
//! Prüfstand judges the scaffolder, the unit suites judge pure logic. None
//! of them touches the one non-deterministic boundary — the real LLM call
//! that does the actual *generating*. This bench is the instrument that
//! does: given a curated corpus of behavioural wishes the deterministic
//! scaffolder **cannot** satisfy (a `Run` facet needs real logic, not a
//! stub), it drives each one through the same provider-backed descent the
//! operator uses and measures what fraction reaches REALIZED — judged by
//! **execution**, never by the model's word.
//!
//! It is provider-agnostic by construction: it arms whatever
//! `build_synthesizer` arms, so a cloud API (`--provider claude|cerebras`)
//! and a local OpenAI-compatible model (`--provider env` +
//! `KOSMO_LLM_BASE_URL`) run the *same* corpus and produce comparable
//! numbers. Mock is refused — a benchmark of the scaffolder measures
//! nothing the Prüfstand doesn't already prove.
//!
//! The corpus is **tiered by difficulty** so a single run yields a spread,
//! not one number: the *floor* (echo, add — does the loop conduct and a
//! model clear trivial targets?), the *rung* (palindrome, base conversion,
//! ROT13 — moderate logic), and the *ceiling* (Roman numerals, precedence
//! expression evaluation, run-length encoding — where a real engine is
//! discriminated from a weak one). Every task carries **multiple probes**
//! that resist hard-coding: a program that prints one memorized answer
//! fails the others, so the model must generalize.
//!
//! What this measures: synthesis-to-spec — given a precise, multi-probe
//! behavioural target, can the model produce code that an executing witness
//! accepts? What it does NOT measure: prose→spec compilation (a separate
//! axis), nor the true paradigm ceiling (whole multi-component systems) —
//! these tiers are *harder rungs*, not the summit. The corpus carries
//! well-formed facets directly, isolating the generative claim.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use kosmo_core::{Digest, Wish, WishClosureStatus, WishFacet, WishPredicate};
use kosmo_synthesizer::{ActionSynthesizer, SynthesisError, SynthesisRequest, SynthesisResult};

use crate::descend_to_wish;

/// A pass-through synthesizer that sums the token cost of every call it
/// forwards — the bench's honest cost meter. Delegates verdicts unchanged.
struct CountingSynthesizer {
    inner: Arc<dyn ActionSynthesizer>,
    tokens: AtomicU32,
}

impl CountingSynthesizer {
    fn new(inner: Arc<dyn ActionSynthesizer>) -> Self {
        Self {
            inner,
            tokens: AtomicU32::new(0),
        }
    }
    fn total(&self) -> u32 {
        self.tokens.load(Ordering::Relaxed)
    }
}

impl ActionSynthesizer for CountingSynthesizer {
    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisResult, SynthesisError> {
        let result = self.inner.synthesize(request)?;
        self.tokens.fetch_add(result.tokens_used, Ordering::Relaxed);
        Ok(result)
    }
    fn name(&self) -> &str {
        self.inner.name()
    }
}

/// Difficulty tier — the spread a single run reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Tier {
    /// Trivial: does the loop conduct and a model clear easy targets?
    Floor,
    /// Moderate logic — a competent model should clear most.
    Rung,
    /// Hard — discriminates a real engine from a weak one.
    Ceiling,
}

impl Tier {
    pub fn label(&self) -> &'static str {
        match self {
            Tier::Floor => "floor",
            Tier::Rung => "rung",
            Tier::Ceiling => "ceiling",
        }
    }
}

/// One behavioural task: a named intent, its tier, and the argv→stdout
/// probes whose satisfaction (by execution) defines success. The expected
/// outputs are self-contained ground truths, trivial to verify by hand.
pub struct RealizeTask {
    pub name: &'static str,
    pub tier: Tier,
    /// Human-readable intent (documentation; the wish is built from probes).
    pub intent: &'static str,
    /// `(argv, expected_stdout)` — at least two, to resist hard-coding.
    pub probes: &'static [(&'static [&'static str], &'static str)],
}

/// The reference corpus, tiered by difficulty. Each task is a wish of
/// budgeted `Run` facets the scaffolder cannot satisfy, so the descent must
/// call the real provider. Ground truths are verifiable by inspection.
pub fn reference_corpus() -> Vec<RealizeTask> {
    use Tier::*;
    vec![
        // ── Floor: does the loop conduct, can a model hit trivial targets? ──
        RealizeTask {
            name: "echo",
            tier: Floor,
            intent: "print the single argument unchanged",
            probes: &[
                (&["ping"], "ping"),
                (&["kosmos"], "kosmos"),
                (&["42"], "42"),
            ],
        },
        RealizeTask {
            name: "add",
            tier: Floor,
            intent: "print the sum of two integer arguments",
            probes: &[
                (&["3", "4"], "7"),
                (&["10", "32"], "42"),
                (&["0", "0"], "0"),
            ],
        },
        RealizeTask {
            name: "maximum",
            tier: Floor,
            intent: "print the larger of two integer arguments",
            probes: &[(&["3", "9"], "9"), (&["42", "7"], "42"), (&["5", "5"], "5")],
        },
        RealizeTask {
            name: "reverse",
            tier: Floor,
            intent: "print the single argument reversed",
            probes: &[(&["abc"], "cba"), (&["hello"], "olleh"), (&["x"], "x")],
        },
        RealizeTask {
            name: "uppercase",
            tier: Floor,
            intent: "print the single argument uppercased",
            probes: &[(&["abc"], "ABC"), (&["Hello"], "HELLO")],
        },
        RealizeTask {
            name: "count-vowels",
            tier: Floor,
            intent: "print the number of vowels (a,e,i,o,u) in the argument",
            probes: &[(&["hello"], "2"), (&["xyz"], "0"), (&["aeiou"], "5")],
        },
        RealizeTask {
            name: "factorial",
            tier: Floor,
            intent: "print the factorial of a non-negative integer argument",
            probes: &[(&["5"], "120"), (&["0"], "1"), (&["6"], "720")],
        },
        RealizeTask {
            name: "fibonacci",
            tier: Floor,
            intent: "print the nth Fibonacci number (fib(0)=0, fib(1)=1)",
            probes: &[(&["10"], "55"), (&["0"], "0"), (&["7"], "13")],
        },
        RealizeTask {
            name: "gcd",
            tier: Floor,
            intent: "print the greatest common divisor of two integer arguments",
            probes: &[
                (&["12", "18"], "6"),
                (&["7", "13"], "1"),
                (&["100", "80"], "20"),
            ],
        },
        RealizeTask {
            name: "sum-list",
            tier: Floor,
            intent: "print the sum of all integer arguments",
            probes: &[
                (&["1", "2", "3"], "6"),
                (&["10", "20"], "30"),
                (&["5"], "5"),
            ],
        },
        // ── Rung: moderate logic — a competent model should clear most. ──
        RealizeTask {
            name: "sum-digits",
            tier: Rung,
            intent: "print the sum of the decimal digits of the argument",
            probes: &[(&["12345"], "15"), (&["99"], "18"), (&["7"], "7")],
        },
        RealizeTask {
            name: "palindrome",
            tier: Rung,
            intent: "print \"true\" if the argument reads the same backwards, else \"false\"",
            probes: &[
                (&["racecar"], "true"),
                (&["hello"], "false"),
                (&["noon"], "true"),
            ],
        },
        RealizeTask {
            name: "lcm",
            tier: Rung,
            intent: "print the least common multiple of two integer arguments",
            probes: &[
                (&["4", "6"], "12"),
                (&["3", "5"], "15"),
                (&["6", "8"], "24"),
            ],
        },
        RealizeTask {
            name: "rot13",
            tier: Rung,
            intent: "print the argument with each letter rotated 13 places (ROT13)",
            probes: &[(&["hello"], "uryyb"), (&["abc"], "nop"), (&["xyz"], "klm")],
        },
        RealizeTask {
            name: "to-binary",
            tier: Rung,
            intent: "print the non-negative integer argument in binary",
            probes: &[(&["10"], "1010"), (&["255"], "11111111"), (&["0"], "0")],
        },
        RealizeTask {
            name: "to-hex",
            tier: Rung,
            intent: "print the non-negative integer argument in lowercase hexadecimal",
            probes: &[(&["255"], "ff"), (&["16"], "10"), (&["10"], "a")],
        },
        RealizeTask {
            name: "anagram",
            tier: Rung,
            intent: "print \"true\" if the two arguments are anagrams, else \"false\"",
            probes: &[
                (&["listen", "silent"], "true"),
                (&["hello", "world"], "false"),
                (&["abc", "cab"], "true"),
            ],
        },
        RealizeTask {
            name: "collatz",
            tier: Rung,
            intent: "print the number of Collatz steps to reach 1 from the argument",
            probes: &[(&["6"], "8"), (&["1"], "0"), (&["27"], "111")],
        },
        // ── Ceiling: hard — discriminates a real engine from a weak one. ──
        RealizeTask {
            name: "nth-prime",
            tier: Ceiling,
            intent: "print the nth prime number (1-indexed: the 1st prime is 2)",
            probes: &[(&["10"], "29"), (&["1"], "2"), (&["25"], "97")],
        },
        RealizeTask {
            name: "roman",
            tier: Ceiling,
            intent: "print the positive integer argument as an uppercase Roman numeral",
            probes: &[(&["1994"], "MCMXCIV"), (&["4"], "IV"), (&["49"], "XLIX")],
        },
        RealizeTask {
            name: "balanced",
            tier: Ceiling,
            intent: "print \"valid\" if the argument's parentheses are balanced, else \"invalid\"",
            probes: &[
                (&["(())"], "valid"),
                (&["(()"], "invalid"),
                (&["()()"], "valid"),
            ],
        },
        RealizeTask {
            name: "run-length",
            tier: Ceiling,
            intent: "run-length encode the argument as char+count pairs (aaabb -> a3b2)",
            probes: &[
                (&["aaabb"], "a3b2"),
                (&["abc"], "a1b1c1"),
                (&["xxxx"], "x4"),
            ],
        },
        RealizeTask {
            name: "expr-eval",
            tier: Ceiling,
            intent: "evaluate the integer +,-,* expression with normal operator precedence",
            probes: &[(&["2+3*4"], "14"), (&["10-2*3"], "4"), (&["2*3+4"], "10")],
        },
    ]
}

/// Build the task's wish: one budgeted `Run` facet per probe —
/// `"args=>exit:0,out~expected,ms<60000"`. Evidence-bound to the task's
/// name and probes (content-addressed, deterministic).
pub fn wish_for(task: &RealizeTask) -> Wish {
    let probes: Vec<(Vec<String>, String)> = task
        .probes
        .iter()
        .map(|(argv, exp)| {
            (
                argv.iter().map(|s| s.to_string()).collect(),
                exp.to_string(),
            )
        })
        .collect();
    let evidence = Digest::of(&(task.name, &probes));
    let predicates = probes.iter().map(|(argv, exp)| {
        WishPredicate::require(WishFacet::run(format!(
            "{}=>exit:0,out~{},ms<60000",
            argv.join(","),
            exp
        )))
    });
    Wish::new(
        format!("realize {}: {}", task.name, task.intent),
        predicates,
        Digest::ZERO,
        evidence,
    )
}

/// One task's measured outcome.
#[derive(Debug, serde::Serialize)]
pub struct TaskOutcome {
    pub name: &'static str,
    pub tier: Tier,
    pub wish_id: String,
    pub realized: bool,
    pub iterations: usize,
    pub probes: usize,
    pub tokens: u32,
}

/// The whole bench's measurement — content-addressed over its own body
/// (tamper-evident; not a reproducibility claim, since model output is
/// non-deterministic).
#[derive(Debug, serde::Serialize)]
pub struct RealizeBenchReport {
    pub provider: String,
    pub model: Option<String>,
    pub outcomes: Vec<TaskOutcome>,
}

impl RealizeBenchReport {
    pub fn attempted(&self) -> usize {
        self.outcomes.len()
    }
    pub fn realized(&self) -> usize {
        self.outcomes.iter().filter(|o| o.realized).count()
    }
    pub fn total_tokens(&self) -> u64 {
        self.outcomes.iter().map(|o| u64::from(o.tokens)).sum()
    }
    pub fn total_iterations(&self) -> usize {
        self.outcomes.iter().map(|o| o.iterations).sum()
    }
    /// `(realized, attempted)` within one tier.
    pub fn tier_counts(&self, tier: Tier) -> (usize, usize) {
        let in_tier: Vec<&TaskOutcome> = self.outcomes.iter().filter(|o| o.tier == tier).collect();
        (in_tier.iter().filter(|o| o.realized).count(), in_tier.len())
    }
    fn rate_bp(realized: usize, attempted: usize) -> u32 {
        if attempted == 0 {
            return 0;
        }
        ((realized as u64 * 10_000) / attempted as u64) as u32
    }
    /// Overall realization rate in basis points (integer — the body stays
    /// float-free; the rendered percentage is exact).
    pub fn overall_bp(&self) -> u32 {
        Self::rate_bp(self.realized(), self.attempted())
    }

    pub fn to_json(&self) -> String {
        let body = serde_json::to_value(self).unwrap_or_default();
        let report_id = Digest::of(&body);
        let tiers: Vec<serde_json::Value> = [Tier::Floor, Tier::Rung, Tier::Ceiling]
            .iter()
            .map(|t| {
                let (r, a) = self.tier_counts(*t);
                serde_json::json!({ "tier": t.label(), "realized": r, "attempted": a, "rate_bp": Self::rate_bp(r, a) })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "report_id": report_id.to_hex(),
            "realized": self.realized(),
            "attempted": self.attempted(),
            "rate_bp": self.overall_bp(),
            "tiers": tiers,
            "total_tokens": self.total_tokens(),
            "report": body,
        }))
        .unwrap_or_default()
    }

    pub fn render(&self, color: bool) -> String {
        let (green, red, dim, bold, reset) = if color {
            ("\x1b[32m", "\x1b[31m", "\x1b[2m", "\x1b[1m", "\x1b[0m")
        } else {
            ("", "", "", "", "")
        };
        let model = self.model.as_deref().unwrap_or("(default)");
        let mut out = format!(
            "{bold}Kosmocrates realization benchmark{reset}  {dim}provider {} · model {}{reset}\n",
            self.provider, model
        );
        let pct = |bp: u32| format!("{}.{:02}%", bp / 100, bp % 100);
        for tier in [Tier::Floor, Tier::Rung, Tier::Ceiling] {
            let (r, a) = self.tier_counts(tier);
            if a == 0 {
                continue;
            }
            out.push_str(&format!(
                "  {bold}{:<8}{reset} {r}/{a}  ({})\n",
                tier.label(),
                pct(Self::rate_bp(r, a))
            ));
            for o in self.outcomes.iter().filter(|o| o.tier == tier) {
                let (mark, col) = if o.realized {
                    ("\u{2713}", green)
                } else {
                    ("\u{2717}", red)
                };
                out.push_str(&format!(
                    "    {col}{mark}{reset} {:<12} {} probe(s) · {} iter · {} tokens\n",
                    o.name, o.probes, o.iterations, o.tokens
                ));
            }
        }
        out.push_str(&format!(
            "  {bold}realized {}/{}{reset} ({}) · {} iterations · {} tokens total\n",
            self.realized(),
            self.attempted(),
            pct(self.overall_bp()),
            self.total_iterations(),
            self.total_tokens(),
        ));
        out
    }
}

/// A throwaway bin workspace with an empty `main` — the blank slate each
/// task is forged onto.
fn scratch_workspace(name: &str) -> std::io::Result<std::path::PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("kosmo-realize-{name}-{nanos}"));
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"realized\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n")?;
    Ok(root)
}

/// Run the benchmark: per task, build the wish, forge it onto a blank
/// workspace via the provider-backed descent, and record whether the
/// executing witness accepted it — with the real token cost.
pub fn run_realize_bench(
    armed: Arc<dyn ActionSynthesizer>,
    provider: &str,
    model: Option<String>,
) -> RealizeBenchReport {
    // The per-task descent budget. Defaults to 8; overridable via
    // KOSMO_REALIZE_MAX_ITERS for constrained environments (a lower budget
    // yields a conservative, lower-bound realization rate).
    let max_iters: u32 = std::env::var("KOSMO_REALIZE_MAX_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(8);
    // Optionally skip the first N tasks (KOSMO_REALIZE_SKIP) to run a slice of
    // the corpus — e.g. just one tier, or to resume after an interruption.
    let skip: usize = std::env::var("KOSMO_REALIZE_SKIP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let corpus = reference_corpus();
    let total = corpus.len();
    let mut outcomes = Vec::new();
    for (idx, task) in corpus.into_iter().enumerate().skip(skip) {
        let wish = wish_for(&task);
        let counter = Arc::new(CountingSynthesizer::new(armed.clone()));
        let (realized, iterations) = match scratch_workspace(task.name) {
            Ok(root) => {
                let descent = descend_to_wish(
                    root.to_str().unwrap_or("."),
                    &wish,
                    wish.evidence_bundle_id,
                    false,
                    max_iters,
                    Some(counter.as_ref()),
                    None,
                );
                std::fs::remove_dir_all(&root).ok();
                match descent {
                    Ok(session) => (
                        session
                            .latest()
                            .is_some_and(|a| matches!(a.status, WishClosureStatus::Realized)),
                        session.iterations(),
                    ),
                    Err(_) => (false, 0),
                }
            }
            Err(_) => (false, 0),
        };
        // Live progress so a long run is observable and partial results
        // survive an interrupted run: one line per task as it resolves.
        eprintln!(
            "  [{:>2}/{}] {:<12} {}  ({} iter · {} tok)",
            idx + 1,
            total,
            task.name,
            if realized { "\u{2713}" } else { "\u{2717}" },
            iterations,
            counter.total(),
        );
        outcomes.push(TaskOutcome {
            name: task.name,
            tier: task.tier,
            wish_id: wish.id.to_hex(),
            realized,
            iterations,
            probes: task.probes.len(),
            tokens: counter.total(),
        });
    }
    RealizeBenchReport {
        provider: provider.to_string(),
        model,
        outcomes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_is_wellformed_and_tiered_and_resists_hardcoding() {
        let corpus = reference_corpus();
        assert!(corpus.len() >= 20, "a corpus worth measuring");
        let mut names: Vec<&str> = corpus.iter().map(|t| t.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), corpus.len(), "task names are unique");
        // Every tier is populated — the spread is real, not a single number.
        for tier in [Tier::Floor, Tier::Rung, Tier::Ceiling] {
            assert!(
                corpus.iter().filter(|t| t.tier == tier).count() >= 5,
                "tier {} needs enough tasks to mean something",
                tier.label()
            );
        }
        for t in &corpus {
            assert!(
                t.probes.len() >= 2,
                "{}: multiple probes resist hard-coding",
                t.name
            );
            for (argv, exp) in t.probes {
                assert!(!argv.is_empty(), "{}: a probe has no argv", t.name);
                assert!(!exp.is_empty(), "{}: a probe has no expectation", t.name);
                // The Run-facet grammar is comma/arrow-delimited; no probe may
                // carry a token that would break the key it becomes.
                for a in *argv {
                    assert!(
                        !a.contains(',') && !a.contains("=>"),
                        "{}: argv survives the grammar",
                        t.name
                    );
                }
                assert!(
                    !exp.contains(',') && !exp.contains("=>"),
                    "{}: expectation survives the grammar",
                    t.name
                );
            }
        }
    }

    #[test]
    fn the_wish_is_budgeted_runtime_expectations_bound_to_the_probes() {
        let corpus = reference_corpus();
        let task = corpus.iter().find(|t| t.name == "add").expect("add task");
        let wish = wish_for(task);
        assert_eq!(wish.predicate_count(), 3);
        let want = WishFacet::run("3,4=>exit:0,out~7,ms<60000");
        assert!(
            wish.predicates.iter().any(|p| p.facet == want),
            "the 3+4=7 probe is a budgeted runtime expectation"
        );
        assert!(wish.is_evidence_bound(), "bound to the probes");
        assert_eq!(
            wish_for(task).id,
            wish.id,
            "same probes, same wish identity"
        );
    }

    #[test]
    fn report_aggregates_tiers_rate_tokens_and_serializes() {
        let mk = |name: &'static str, tier: Tier, realized: bool, tokens: u32| TaskOutcome {
            name,
            tier,
            wish_id: "00".into(),
            realized,
            iterations: 3,
            probes: 3,
            tokens,
        };
        let report = RealizeBenchReport {
            provider: "test".into(),
            model: Some("m".into()),
            outcomes: vec![
                mk("a", Tier::Floor, true, 1000),
                mk("b", Tier::Floor, true, 1000),
                mk("c", Tier::Rung, true, 500),
                mk("d", Tier::Rung, false, 800),
                mk("e", Tier::Ceiling, false, 900),
            ],
        };
        assert_eq!(report.attempted(), 5);
        assert_eq!(report.realized(), 3);
        assert_eq!(report.overall_bp(), 6000, "3/5");
        assert_eq!(report.tier_counts(Tier::Floor), (2, 2));
        assert_eq!(report.tier_counts(Tier::Rung), (1, 2));
        assert_eq!(report.tier_counts(Tier::Ceiling), (0, 1));
        assert_eq!(report.total_tokens(), 4200);
        let text = report.render(false);
        assert!(text.contains("floor    2/2  (100.00%)"), "{text}");
        assert!(text.contains("ceiling  0/1  (0.00%)"), "{text}");
        assert!(text.contains("realized 3/5 (60.00%)"), "{text}");
        let json = report.to_json();
        assert!(json.contains("\"report_id\""), "{json}");
        assert!(json.contains("\"tier\": \"ceiling\""), "{json}");
        assert!(json.contains("\"total_tokens\": 4200"), "{json}");
    }
}
