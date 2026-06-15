//! The prose→spec benchmark — does natural language translate to the right facets?
//!
//! The realization bench measures synthesis (facets → working code). This
//! measures the **other axis**: the human front door. A deliberately natural
//! (sometimes filler-laden) utterance is run through the intent extractor — the
//! provider-backed `LlmIntentExtractor` when `--provider` is set, else the
//! deterministic keyword router — and the prose it yields is compiled to a
//! `Wish`. The compiled facets are scored against a hand-written ground truth:
//! did the system translate the wish into the facets it *meant*?
//!
//! It needs no realization and no workspace — it is a cheap, single-call probe
//! of the prose→spec compiler, complementary to the (expensive) realize bench.

use kosmo_core::{Digest, WishFacetKind};
use kosmo_intent::{compile_wish, ChatIntent, IntentExtractor};

/// One prose task: a natural utterance and the facets it must yield.
pub struct ProseTask {
    pub utterance: &'static str,
    /// Expected `(kind, key)` facets — the compiled wish must contain all of them.
    pub expect: &'static [(WishFacetKind, &'static str)],
}

/// The prose corpus — natural phrasings with an unambiguous intended spec, each
/// carrying fillers/synonyms ("please", "called", "named", "inside it") that a
/// bare grammar would trip on, so the front door is actually exercised.
pub fn prose_corpus() -> Vec<ProseTask> {
    use WishFacetKind::*;
    vec![
        ProseTask {
            utterance: "please make a crate called kosmo-gateway",
            expect: &[(Crate, "kosmo-gateway")],
        },
        ProseTask {
            utterance: "I'd like a module named router",
            expect: &[(Module, "router")],
        },
        ProseTask {
            utterance: "could you add a function parse_header",
            expect: &[(Symbol, "parse_header")],
        },
        ProseTask {
            utterance: "give me a crate proxy and a module forwarding",
            expect: &[(Crate, "proxy"), (Module, "forwarding")],
        },
        ProseTask {
            utterance: "a capability called http-server",
            expect: &[(Capability, "http-server")],
        },
        ProseTask {
            utterance: "create a type RequestId",
            expect: &[(Symbol, "RequestId")],
        },
        ProseTask {
            utterance: "make a module auth with a function verify_token",
            expect: &[(Module, "auth"), (Symbol, "verify_token")],
        },
        ProseTask {
            utterance: "I want a struct Config and a function load",
            expect: &[(Symbol, "Config"), (Symbol, "load")],
        },
    ]
}

/// One prose task's measured outcome.
pub struct ProseOutcome {
    pub utterance: &'static str,
    /// The prose the extractor produced (audit trail — what it understood).
    pub prose: String,
    pub expected: usize,
    pub matched: usize,
}

impl ProseOutcome {
    pub fn passed(&self) -> bool {
        self.matched == self.expected && self.expected > 0
    }
}

/// The whole prose bench's measurement.
pub struct ProseBenchReport {
    pub extractor: String,
    pub outcomes: Vec<ProseOutcome>,
}

impl ProseBenchReport {
    pub fn attempted(&self) -> usize {
        self.outcomes.len()
    }
    pub fn passed(&self) -> usize {
        self.outcomes.iter().filter(|o| o.passed()).count()
    }

    pub fn render(&self, color: bool) -> String {
        let (green, red, dim, bold, reset) = if color {
            ("\x1b[32m", "\x1b[31m", "\x1b[2m", "\x1b[1m", "\x1b[0m")
        } else {
            ("", "", "", "", "")
        };
        let mut out = format!(
            "{bold}Kosmocrates prose\u{2192}spec benchmark{reset}  {dim}extractor {}{reset}\n",
            self.extractor
        );
        for o in &self.outcomes {
            let (mark, col) = if o.passed() {
                ("\u{2713}", green)
            } else {
                ("\u{2717}", red)
            };
            out.push_str(&format!(
                "  {col}{mark}{reset} {:<46.46} {}/{} facets  {dim}\u{2192} {}{reset}\n",
                o.utterance, o.matched, o.expected, o.prose
            ));
        }
        out.push_str(&format!(
            "  {bold}translated {}/{}{reset}\n",
            self.passed(),
            self.attempted()
        ));
        out
    }
}

/// Run the prose benchmark: per task, extract → compile → score the facets
/// against the ground truth (the wish must contain every expected facet).
pub fn run_prose_bench(extractor: &dyn IntentExtractor) -> ProseBenchReport {
    let corpus = prose_corpus();
    let mut outcomes = Vec::new();
    for task in &corpus {
        let prose = match extractor.extract(task.utterance) {
            ChatIntent::MakeWish { prose } | ChatIntent::DescendWish { prose } => prose,
            // Routed to a non-wish intent — no facets, every expectation misses.
            _ => String::new(),
        };
        let wish = compile_wish(&prose, Digest::ZERO, Digest::ZERO);
        let matched = task
            .expect
            .iter()
            .filter(|(kind, key)| {
                wish.predicates
                    .iter()
                    .any(|p| p.facet.kind == *kind && p.facet.key == *key)
            })
            .count();
        eprintln!(
            "  {} {:<46.46} {}/{} facets",
            if matched == task.expect.len() {
                "\u{2713}"
            } else {
                "\u{2717}"
            },
            task.utterance,
            matched,
            task.expect.len()
        );
        outcomes.push(ProseOutcome {
            utterance: task.utterance,
            prose,
            expected: task.expect.len(),
            matched,
        });
    }
    ProseBenchReport {
        extractor: extractor.name().to_string(),
        outcomes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_intent::KeywordIntentExtractor;

    #[test]
    fn prose_corpus_is_wellformed() {
        let corpus = prose_corpus();
        assert!(corpus.len() >= 6, "a corpus worth measuring");
        for t in &corpus {
            assert!(!t.utterance.trim().is_empty(), "an utterance");
            assert!(!t.expect.is_empty(), "{}: expects facets", t.utterance);
            for (_, key) in t.expect {
                assert!(!key.is_empty(), "{}: a non-empty facet key", t.utterance);
            }
        }
    }

    #[test]
    fn deterministic_router_translates_natural_phrasings() {
        // The offline keyword router already handles fillers/synonyms; this pins
        // that and gives the prose bench a green floor without a provider. The
        // LLM extractor is measured on top (and can only do better).
        let report = run_prose_bench(&KeywordIntentExtractor);
        assert_eq!(
            report.passed(),
            report.attempted(),
            "the deterministic front door must translate every corpus utterance:\n{}",
            report.render(false)
        );
    }
}
