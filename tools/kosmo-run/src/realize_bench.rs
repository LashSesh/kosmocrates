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
//! What this measures: synthesis-to-spec — given a precise, multi-probe
//! behavioural target, can the model produce code that an executing witness
//! accepts? Multiple probes per task resist hard-coding (a program that
//! prints one memorized answer fails the others, so the model must
//! generalize). What it does NOT measure: prose→spec compilation (the
//! front-door grammar / wish-LLM is a separate axis) — the corpus carries
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

/// One behavioural task: a named intent and the argv→stdout probes whose
/// satisfaction (by execution) defines success. The expected outputs are
/// self-contained ground truths, trivial to verify by inspection.
pub struct RealizeTask {
    pub name: &'static str,
    /// Human-readable intent (documentation; the wish is built from probes).
    pub intent: &'static str,
    /// `(argv, expected_stdout)` — at least two, to resist hard-coding.
    pub probes: &'static [(&'static [&'static str], &'static str)],
}

/// The reference corpus: deterministic, self-contained behavioural tasks of
/// rising difficulty. Each is a wish of budgeted `Run` facets the
/// scaffolder cannot satisfy, so the descent must call the real provider.
pub fn reference_corpus() -> Vec<RealizeTask> {
    vec![
        RealizeTask {
            name: "echo",
            intent: "print the single argument unchanged",
            probes: &[
                (&["ping"], "ping"),
                (&["kosmos"], "kosmos"),
                (&["42"], "42"),
            ],
        },
        RealizeTask {
            name: "add",
            intent: "print the sum of two integer arguments",
            probes: &[
                (&["3", "4"], "7"),
                (&["10", "32"], "42"),
                (&["0", "0"], "0"),
            ],
        },
        RealizeTask {
            name: "maximum",
            intent: "print the larger of two integer arguments",
            probes: &[(&["3", "9"], "9"), (&["42", "7"], "42"), (&["5", "5"], "5")],
        },
        RealizeTask {
            name: "reverse",
            intent: "print the single argument reversed",
            probes: &[(&["abc"], "cba"), (&["hello"], "olleh"), (&["x"], "x")],
        },
        RealizeTask {
            name: "uppercase",
            intent: "print the single argument uppercased",
            probes: &[(&["abc"], "ABC"), (&["Hello"], "HELLO")],
        },
        RealizeTask {
            name: "count-vowels",
            intent: "print the number of vowels (a,e,i,o,u) in the argument",
            probes: &[(&["hello"], "2"), (&["xyz"], "0"), (&["aeiou"], "5")],
        },
        RealizeTask {
            name: "factorial",
            intent: "print the factorial of a non-negative integer argument",
            probes: &[(&["5"], "120"), (&["0"], "1"), (&["6"], "720")],
        },
        RealizeTask {
            name: "fibonacci",
            intent: "print the nth Fibonacci number (fib(0)=0, fib(1)=1)",
            probes: &[(&["10"], "55"), (&["0"], "0"), (&["7"], "13")],
        },
        RealizeTask {
            name: "gcd",
            intent: "print the greatest common divisor of two integer arguments",
            probes: &[
                (&["12", "18"], "6"),
                (&["7", "13"], "1"),
                (&["100", "80"], "20"),
            ],
        },
        RealizeTask {
            name: "sum-list",
            intent: "print the sum of all integer arguments",
            probes: &[
                (&["1", "2", "3"], "6"),
                (&["10", "20"], "30"),
                (&["5"], "5"),
            ],
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
    /// Realization rate in basis points (per ten-thousand) — integer, so the
    /// rendered percentage is exact and the body stays float-free.
    pub fn rate_bp(&self) -> u32 {
        if self.outcomes.is_empty() {
            return 0;
        }
        ((self.realized() as u64 * 10_000) / self.attempted() as u64) as u32
    }

    pub fn to_json(&self) -> String {
        let body = serde_json::to_value(self).unwrap_or_default();
        let report_id = Digest::of(&body);
        serde_json::to_string_pretty(&serde_json::json!({
            "report_id": report_id.to_hex(),
            "realized": self.realized(),
            "attempted": self.attempted(),
            "rate_bp": self.rate_bp(),
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
        for o in &self.outcomes {
            let (mark, col) = if o.realized {
                ("\u{2713}", green)
            } else {
                ("\u{2717}", red)
            };
            out.push_str(&format!(
                "  {col}{mark}{reset} {:<13} {} probe(s) · {} iter · {} tokens\n",
                o.name, o.probes, o.iterations, o.tokens
            ));
        }
        let rate = self.rate_bp();
        out.push_str(&format!(
            "  {bold}realized {}/{}{reset} ({}.{:02}%) · {} iterations · {} tokens total\n",
            self.realized(),
            self.attempted(),
            rate / 100,
            rate % 100,
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
    let mut outcomes = Vec::new();
    for task in reference_corpus() {
        let wish = wish_for(&task);
        let counter = Arc::new(CountingSynthesizer::new(armed.clone()));
        let (realized, iterations) = match scratch_workspace(task.name) {
            Ok(root) => {
                let descent = descend_to_wish(
                    root.to_str().unwrap_or("."),
                    &wish,
                    wish.evidence_bundle_id,
                    false,
                    8,
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
        outcomes.push(TaskOutcome {
            name: task.name,
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
    fn corpus_is_wellformed_and_resists_hardcoding() {
        let corpus = reference_corpus();
        assert!(corpus.len() >= 8, "a corpus worth measuring");
        let mut names: Vec<&str> = corpus.iter().map(|t| t.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), corpus.len(), "task names are unique");
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
                    !exp.contains(','),
                    "{}: expectation survives the grammar",
                    t.name
                );
            }
        }
    }

    #[test]
    fn the_wish_is_budgeted_runtime_expectations_bound_to_the_probes() {
        let task = &reference_corpus()[1]; // "add"
        assert_eq!(task.name, "add");
        let wish = wish_for(task);
        assert_eq!(wish.predicate_count(), 3);
        // Each probe becomes a budgeted Run facet — argv + expected output +
        // time budget, all measured at execution (order-independent).
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
    fn report_aggregates_rate_tokens_and_serializes() {
        let report = RealizeBenchReport {
            provider: "test".into(),
            model: Some("m".into()),
            outcomes: vec![
                TaskOutcome {
                    name: "a",
                    wish_id: "00".into(),
                    realized: true,
                    iterations: 2,
                    probes: 3,
                    tokens: 1500,
                },
                TaskOutcome {
                    name: "b",
                    wish_id: "01".into(),
                    realized: false,
                    iterations: 8,
                    probes: 2,
                    tokens: 4096,
                },
                TaskOutcome {
                    name: "c",
                    wish_id: "02".into(),
                    realized: true,
                    iterations: 1,
                    probes: 2,
                    tokens: 900,
                },
            ],
        };
        assert_eq!(report.attempted(), 3);
        assert_eq!(report.realized(), 2);
        assert_eq!(report.rate_bp(), 6666, "2/3 in basis points");
        assert_eq!(report.total_tokens(), 6496);
        assert_eq!(report.total_iterations(), 11);
        let text = report.render(false);
        assert!(text.contains("realized 2/3 (66.66%)"), "{text}");
        let json = report.to_json();
        assert!(json.contains("\"report_id\""), "{json}");
        assert!(json.contains("\"rate_bp\": 6666"), "{json}");
        assert!(json.contains("\"total_tokens\": 6496"), "{json}");
    }
}
