//! The chat front door — utterances route to existing organs, never past them.
//!
//! Assimilated from Wonderlamp's `isls-chat` with its disease excised: the
//! predecessor's intents carried `affected_files` lists, a forced User
//! entity and REST assumptions — a template generator wearing a chat mask.
//! Here a [`ChatIntent`] is **type-systemically incapable** of carrying a
//! path, a file list or an entity scaffold (pinned by a source-scan test on
//! the enum): every variant maps onto an organ the substrate already has —
//! the wish door, the landscape, adoption, governance.
//!
//! Routing discipline:
//!
//! - **Total.** [`IntentExtractor::extract`] cannot fail. The deterministic
//!   [`KeywordIntentExtractor`] ends in the [`ChatIntent::MakeWish`]
//!   fallback: an utterance nobody understood flows to the *measurable*
//!   wish door (where an unparseable wish is honestly vacuous) — never to a
//!   template generator, never to an error that swallows the operator's
//!   words.
//! - **Transient.** Routing is not a durable artifact: no content
//!   addressing, no evidence binding — the routed-to organs own all of
//!   that. This is exactly why an LLM router may fall back to keywords on
//!   malformed output without violating any replay contract.

use crate::norm;

/// What the operator wants, in the system's own vocabulary. Every variant
/// maps onto an existing organ; none can name a path, file or entity —
/// structure enters the world only through measured facets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatIntent {
    /// Compile the utterance as a wish and measure the workspace against it.
    MakeWish { prose: String },
    /// Like [`ChatIntent::MakeWish`], but the operator asked to *build*:
    /// under `--apply` the wish is descended; without it, measuring plus an
    /// honest hint (chat never escalates to host writes by itself).
    DescendWish { prose: String },
    /// Project the substrate's findings into the ranked wish landscape;
    /// optionally with its spectral geometry.
    ShowLandscape { geometry: bool },
    /// Adopt the top-N open proposals as ONE wish.
    AdoptLandscape { top: u32 },
    /// Adopt ONE coherent geometry cluster (1-based) as ONE wish.
    AdoptCluster { index: u32 },
    /// The system's measured standing (the landscape summary surface).
    ShowStatus,
    /// Norm governance routing: chat carries no spec files (by type), so
    /// this maps to instructions for the explicit operator flags.
    InjectNorm,
}

/// A total utterance router. `extract` cannot fail — the keyword router
/// bottoms out in [`ChatIntent::MakeWish`], and an LLM-backed router falls
/// back to the keyword rules on malformed output.
pub trait IntentExtractor: Send + Sync {
    fn extract(&self, utterance: &str) -> ChatIntent;
    fn name(&self) -> &str;
}

/// The first integer token in the utterance, if any.
fn first_number(words: &[String]) -> Option<u32> {
    words.iter().find_map(|w| w.parse::<u32>().ok())
}

fn has(words: &[String], any_of: &[&str]) -> bool {
    words.iter().any(|w| any_of.contains(&w.as_str()))
}

/// The deterministic routing rules, ordered (first match wins):
///
/// 1. `inject` + `norm(s)` → [`ChatIntent::InjectNorm`]
/// 2. `adopt` + `cluster(s)` → [`ChatIntent::AdoptCluster`] (first number,
///    else cluster 1); bare `adopt` → [`ChatIntent::AdoptLandscape`]
///    (first number, else top 3)
/// 3. `landscape` / `map` / `proposals` → [`ChatIntent::ShowLandscape`]
///    (geometry when `geometry` / `cluster(s)` / `shape` appears)
/// 4. `status` / `standing` → [`ChatIntent::ShowStatus`]
/// 5. `build` / `descend` / `realize` / `implement` →
///    [`ChatIntent::DescendWish`] with the full utterance
/// 6. everything else → [`ChatIntent::MakeWish`] with the full utterance —
///    the totality guarantee.
pub fn route_keywords(utterance: &str) -> ChatIntent {
    let words: Vec<String> = utterance.split_whitespace().map(norm).collect();

    if has(&words, &["inject"]) && has(&words, &["norm", "norms"]) {
        return ChatIntent::InjectNorm;
    }
    if has(&words, &["adopt", "adopts"]) {
        if has(&words, &["cluster", "clusters"]) {
            return ChatIntent::AdoptCluster {
                index: first_number(&words).unwrap_or(1).max(1),
            };
        }
        return ChatIntent::AdoptLandscape {
            top: first_number(&words).unwrap_or(3).max(1),
        };
    }
    if has(&words, &["landscape", "map", "proposals"]) {
        return ChatIntent::ShowLandscape {
            geometry: has(&words, &["geometry", "cluster", "clusters", "shape"]),
        };
    }
    if has(&words, &["status", "standing"]) {
        return ChatIntent::ShowStatus;
    }
    if has(&words, &["build", "descend", "realize", "implement"]) {
        return ChatIntent::DescendWish {
            prose: utterance.trim().to_string(),
        };
    }
    ChatIntent::MakeWish {
        prose: utterance.trim().to_string(),
    }
}

/// The deterministic, offline router — total by construction.
pub struct KeywordIntentExtractor;

impl IntentExtractor for KeywordIntentExtractor {
    fn extract(&self, utterance: &str) -> ChatIntent {
        route_keywords(utterance)
    }
    fn name(&self) -> &str {
        "keyword"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totality_everything_unrecognized_is_a_wish() {
        // The CRUD-relapse guard #6: the fallback is the MEASURABLE wish
        // door, never a template generator and never an error.
        for utterance in [
            "frobnicate the quux with seventeen wombats",
            "",
            "?!?",
            "a module router and a function dispatch",
        ] {
            match route_keywords(utterance) {
                ChatIntent::MakeWish { prose } => assert_eq!(prose, utterance.trim()),
                other => panic!("{utterance:?} must fall through to MakeWish, got {other:?}"),
            }
        }
    }

    #[test]
    fn governance_and_adoption_route_with_numbers() {
        assert_eq!(
            route_keywords("inject this norm please"),
            ChatIntent::InjectNorm
        );
        assert_eq!(
            route_keywords("adopt cluster 2"),
            ChatIntent::AdoptCluster { index: 2 }
        );
        assert_eq!(
            route_keywords("adopt the cluster"),
            ChatIntent::AdoptCluster { index: 1 }
        );
        assert_eq!(
            route_keywords("adopt 5 proposals"),
            ChatIntent::AdoptLandscape { top: 5 }
        );
        assert_eq!(
            route_keywords("adopt the landscape"),
            ChatIntent::AdoptLandscape { top: 3 },
            "adoption outranks the landscape word"
        );
    }

    #[test]
    fn landscape_status_and_descent_route() {
        assert_eq!(
            route_keywords("show me the landscape"),
            ChatIntent::ShowLandscape { geometry: false }
        );
        assert_eq!(
            route_keywords("map the landscape with geometry"),
            ChatIntent::ShowLandscape { geometry: true }
        );
        assert_eq!(
            route_keywords("show the landscape clusters"),
            ChatIntent::ShowLandscape { geometry: true }
        );
        assert_eq!(
            route_keywords("what is the status?"),
            ChatIntent::ShowStatus
        );
        assert_eq!(
            route_keywords("build a module router"),
            ChatIntent::DescendWish {
                prose: "build a module router".into()
            }
        );
        assert_eq!(
            route_keywords("realize the wish"),
            ChatIntent::DescendWish {
                prose: "realize the wish".into()
            }
        );
    }

    #[test]
    fn punctuation_never_breaks_routing() {
        assert_eq!(
            route_keywords("Adopt, please, CLUSTER #2!"),
            ChatIntent::AdoptCluster { index: 2 }
        );
        assert_eq!(
            route_keywords("Landscape?"),
            ChatIntent::ShowLandscape { geometry: false }
        );
    }

    #[test]
    fn extractor_is_the_pure_router() {
        let x = KeywordIntentExtractor;
        assert_eq!(x.name(), "keyword");
        assert_eq!(x.extract("status"), ChatIntent::ShowStatus);
    }

    /// CRUD-relapse guard #6, type level: the intent vocabulary itself must
    /// be unable to carry paths, files or entities. Scans the enum's
    /// non-comment source lines.
    #[test]
    fn chat_intents_carry_no_paths_files_or_entities() {
        let src = include_str!("chat.rs");
        let body = src.split("#[cfg(test)]").next().unwrap();
        let enum_block: String = body
            .lines()
            .skip_while(|l| !l.starts_with("pub enum ChatIntent"))
            .take_while(|l| !l.starts_with('}'))
            .filter(|l| {
                let t = l.trim_start();
                !t.starts_with("///") && !t.starts_with("//")
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            enum_block.contains("MakeWish"),
            "the scan must see the enum: {enum_block}"
        );
        for forbidden in [
            "PathBuf", "Path", "path", "file", "files", "entity", "entities", "dir",
        ] {
            assert!(
                !enum_block.contains(forbidden),
                "ChatIntent must not carry '{forbidden}':\n{enum_block}"
            );
        }
    }
}
