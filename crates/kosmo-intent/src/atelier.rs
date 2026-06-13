//! The wish atelier — a wish is shaped over rounds before it is realized.
//!
//! Today's front door compiles prose into a wish and (under `--apply`)
//! descends immediately: one shot. The atelier makes the wish a
//! **workpiece**: a durable, content-addressed [`WishDraft`] that operator
//! and machine refine together across rounds — and only the operator's
//! explicit *realize* fires the descent.
//!
//! The governance symmetry with the norm organ, applied to dialogue:
//! **the machine proposes, only the operator disposes.**
//!
//! - What the operator *dictates* (grammar-recognized facets of their own
//!   words, including their promoted norm triggers) enters the draft's
//!   `accepted` set directly — that is dictation, not suggestion.
//! - What the machine *proposes* (companion heuristics here, model
//!   suggestions via `kosmo-intent-llm`) lands in `pending` and **never**
//!   enters the wish without an explicit `accept` — pinned by test.
//! - Suggestions are [`WishFacet`]s by type: the machine can propose
//!   measurable targets only, never paths, files or stacks.
//!
//! Every state of the draft is content-addressed (`draft_id`) and
//! evidence-bound to its own dialogue history (CROSS-006: the prose
//! history *is* the evidence), so a shaped wish carries the full
//! provenance of how it came to be.

use kosmo_core::{Digest, ObservedTopology, Wish, WishFacet, WishFacetKind, WishPredicate};
use serde::{Deserialize, Serialize};

use crate::{compile_wish_with_norms, norm, NormCatalog};

/// Where a pending suggestion came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionSource {
    /// A model proposed it (label names the provider/config).
    Model { label: String },
    /// The substrate's own companion heuristics (doc/test fibers).
    Observation,
}

/// One facet the machine proposes for the wish, awaiting the operator's
/// verdict. By construction this can only carry a measurable facet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetSuggestion {
    pub facet: WishFacet,
    pub rationale: String,
    pub source: SuggestionSource,
}

/// Serialize-only content for the draft id.
#[derive(Serialize)]
struct DraftContent<'a> {
    prose_history: &'a Vec<String>,
    accepted: &'a Vec<WishFacet>,
    pending: &'a Vec<FacetSuggestion>,
    questions: &'a Vec<String>,
    policy_id: &'a Digest,
}

/// A wish in the making: the operator's dictation so far, the machine's
/// open proposals, and the dialogue that produced both.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WishDraft {
    pub draft_id: Digest,
    /// Every operator utterance that shaped this draft, in order.
    pub prose_history: Vec<String>,
    /// The wish so far — dictated or explicitly accepted. Sorted, deduped.
    pub accepted: Vec<WishFacet>,
    /// Machine proposals awaiting a verdict. Sorted by facet, deduped.
    pub pending: Vec<FacetSuggestion>,
    /// The machine's open questions from the latest refinement round.
    pub questions: Vec<String>,
    pub policy_id: Digest,
    /// Digest of the prose history — the dialogue is the evidence.
    pub evidence_bundle_id: Digest,
}

impl WishDraft {
    pub fn new(policy_id: Digest) -> Self {
        let mut d = Self {
            draft_id: Digest::ZERO,
            prose_history: vec![],
            accepted: vec![],
            pending: vec![],
            questions: vec![],
            policy_id,
            evidence_bundle_id: Digest::ZERO,
        };
        d.rehash();
        d
    }

    fn rehash(&mut self) {
        self.accepted.sort();
        self.accepted.dedup();
        self.pending.sort_by(|a, b| a.facet.cmp(&b.facet));
        self.pending.dedup_by(|a, b| a.facet == b.facet);
        self.evidence_bundle_id = Digest::of(&self.prose_history);
        self.draft_id = Digest::of(&DraftContent {
            prose_history: &self.prose_history,
            accepted: &self.accepted,
            pending: &self.pending,
            questions: &self.questions,
            policy_id: &self.policy_id,
        });
    }

    pub fn verify_id(&self) -> bool {
        let mut copy = self.clone();
        copy.rehash();
        copy.draft_id == self.draft_id && copy.evidence_bundle_id == self.evidence_bundle_id
    }

    /// One dialogue round: append the utterance and fold its
    /// grammar-recognized facets (leafs, archetypes, promoted norms) into
    /// `accepted` — the operator's own dictation. A pending suggestion the
    /// operator now dictates is thereby resolved.
    pub fn speak(mut self, prose: &str, catalog: &NormCatalog) -> Self {
        let prose = prose.trim();
        if prose.is_empty() {
            return self;
        }
        self.prose_history.push(prose.to_string());
        let dictated = compile_wish_with_norms(
            prose,
            catalog,
            self.policy_id,
            Digest::of_bytes(prose.as_bytes()),
        );
        for p in dictated.predicates {
            self.pending.retain(|s| s.facet != p.facet);
            self.accepted.push(p.facet);
        }
        self.rehash();
        self
    }

    /// Merge machine proposals into `pending`. Facets already accepted or
    /// already pending are skipped — a proposal never duplicates and NEVER
    /// enters `accepted` by itself.
    pub fn propose(mut self, suggestions: impl IntoIterator<Item = FacetSuggestion>) -> Self {
        for s in suggestions {
            if self.accepted.contains(&s.facet) || self.pending.iter().any(|p| p.facet == s.facet) {
                continue;
            }
            self.pending.push(s);
        }
        self.rehash();
        self
    }

    /// Replace the open questions with the latest refinement round's.
    pub fn with_questions(mut self, questions: Vec<String>) -> Self {
        self.questions = questions;
        self.rehash();
        self
    }

    /// Accept pending suggestions (0-based indices into `pending`):
    /// they move into the wish. Out-of-range indices are ignored.
    pub fn accept_pending(mut self, indices: &[usize]) -> Self {
        let mut chosen: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| i < self.pending.len())
            .collect();
        chosen.sort_unstable();
        chosen.dedup();
        for i in chosen.into_iter().rev() {
            let s = self.pending.remove(i);
            self.accepted.push(s.facet);
        }
        self.rehash();
        self
    }

    /// Reject pending suggestions (0-based): they are dropped.
    pub fn reject_pending(mut self, indices: &[usize]) -> Self {
        let mut chosen: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| i < self.pending.len())
            .collect();
        chosen.sort_unstable();
        chosen.dedup();
        for i in chosen.into_iter().rev() {
            self.pending.remove(i);
        }
        self.rehash();
        self
    }

    /// Retract accepted facets (0-based indices into `accepted`).
    pub fn retract_accepted(mut self, indices: &[usize]) -> Self {
        let mut chosen: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| i < self.accepted.len())
            .collect();
        chosen.sort_unstable();
        chosen.dedup();
        for i in chosen.into_iter().rev() {
            self.accepted.remove(i);
        }
        self.rehash();
        self
    }

    /// Freeze the draft into a content-addressed [`Wish`]: the accepted
    /// facets, labelled with the full dialogue, evidence-bound to it.
    /// Pending suggestions are NOT part of the wish — unanswered proposals
    /// die unanswered.
    pub fn to_wish(&self) -> Wish {
        Wish::new(
            self.prose_history.join("; "),
            self.accepted.iter().cloned().map(WishPredicate::require),
            self.policy_id,
            self.evidence_bundle_id,
        )
    }

    /// Resolve a 1-based display index (continuous over `accepted` then
    /// `pending`) to its slot.
    pub fn resolve(&self, display_index: usize) -> Option<DraftSlot> {
        if display_index == 0 {
            return None;
        }
        let i = display_index - 1;
        if i < self.accepted.len() {
            Some(DraftSlot::Accepted(i))
        } else if i - self.accepted.len() < self.pending.len() {
            Some(DraftSlot::Pending(i - self.accepted.len()))
        } else {
            None
        }
    }
}

/// Which list a display index points into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftSlot {
    Accepted(usize),
    Pending(usize),
}

/// The substrate's own companion heuristics: for every accepted structural
/// target, propose the doc/test companions the diagnosis world would
/// otherwise report as missing fibers later. Deterministic and offline;
/// companions already accepted, already pending, or already observed
/// present in the workspace are not proposed.
pub fn companion_suggestions(
    draft: &WishDraft,
    observed: Option<&ObservedTopology>,
) -> Vec<FacetSuggestion> {
    let mut out = Vec::new();
    let mut consider = |facet: WishFacet, rationale: &str| {
        if draft.accepted.contains(&facet) {
            return;
        }
        if draft.pending.iter().any(|p| p.facet == facet) {
            return;
        }
        if observed.map(|o| o.contains(&facet)).unwrap_or(false) {
            return;
        }
        out.push(FacetSuggestion {
            facet,
            rationale: rationale.to_string(),
            source: SuggestionSource::Observation,
        });
    };
    for f in &draft.accepted {
        match f.kind {
            WishFacetKind::Module | WishFacetKind::Crate => {
                consider(
                    WishFacet::doc(f.key.clone()),
                    "structural target without a doc companion",
                );
                consider(
                    WishFacet::test(format!("{}_smoke", f.key)),
                    "structural target without a test companion",
                );
            }
            WishFacetKind::Symbol => {
                consider(
                    WishFacet::doc(f.key.clone()),
                    "public symbol without a doc companion",
                );
            }
            _ => {}
        }
    }
    out
}

// ─── The atelier's round language ────────────────────────────────────────────

/// Which display indices a verdict names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexSelection {
    All,
    These(Vec<usize>),
}

/// One atelier round, parsed from an utterance. Deliberately tiny: exact
/// verdict forms and exact realize phrases; **everything else is prose**
/// (fail-closed toward dictation, never toward action).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtelierCommand {
    /// Show the draft (also the empty utterance).
    Show,
    /// Accept pending suggestions by display index.
    Accept(IndexSelection),
    /// Reject pending suggestions by display index.
    Reject(IndexSelection),
    /// Retract accepted facets by display index.
    Drop(IndexSelection),
    /// Freeze the wish and descend (under `--apply`).
    Realize,
    /// A dialogue round: new prose for the draft.
    Speak(String),
}

/// Exact whole-utterance phrases that fire realization. Anything longer is
/// prose — "build a module x" speaks, only "build it" acts.
const REALIZE_PHRASES: &[&str] = &[
    "realize",
    "realize it",
    "build it",
    "make it real",
    "go",
    "descend",
];

/// Parse the tokens after a verdict verb — **raw** tokens, since the word
/// normalizer strips the very commas that separate indices ("1,3").
fn parse_selection(raw_words: &[&str]) -> Option<IndexSelection> {
    if raw_words.is_empty() {
        return None;
    }
    if raw_words.len() == 1 && norm(raw_words[0]) == "all" {
        return Some(IndexSelection::All);
    }
    let mut indices = Vec::new();
    for w in raw_words {
        for part in w.split(',').filter(|p| !p.is_empty()) {
            indices.push(part.parse::<usize>().ok()?);
        }
    }
    if indices.is_empty() {
        None
    } else {
        Some(IndexSelection::These(indices))
    }
}

/// Parse one utterance into an [`AtelierCommand`]. Total: unrecognized
/// input is prose ([`AtelierCommand::Speak`]).
pub fn parse_atelier_command(utterance: &str) -> AtelierCommand {
    let trimmed = utterance.trim();
    let raw_words: Vec<&str> = trimmed.split_whitespace().collect();
    let words: Vec<String> = raw_words.iter().map(|w| norm(w)).collect();
    if words.is_empty()
        || (words.len() == 1 && matches!(words[0].as_str(), "show" | "state" | "status"))
    {
        return AtelierCommand::Show;
    }
    if REALIZE_PHRASES.contains(&words.join(" ").as_str()) {
        return AtelierCommand::Realize;
    }
    if let Some(verb) = words.first() {
        let selection = || parse_selection(&raw_words[1..]);
        match verb.as_str() {
            "accept" | "take" => {
                if let Some(sel) = selection() {
                    return AtelierCommand::Accept(sel);
                }
            }
            "reject" | "decline" => {
                if let Some(sel) = selection() {
                    return AtelierCommand::Reject(sel);
                }
            }
            "drop" | "retract" => {
                if let Some(sel) = selection() {
                    return AtelierCommand::Drop(sel);
                }
            }
            _ => {}
        }
    }
    AtelierCommand::Speak(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn suggestion(facet: WishFacet) -> FacetSuggestion {
        FacetSuggestion {
            facet,
            rationale: "test".into(),
            source: SuggestionSource::Observation,
        }
    }

    #[test]
    fn draft_is_content_addressed_and_evidence_bound_to_its_dialogue() {
        let a = WishDraft::new(d(b"pol")).speak("a module parser", &NormCatalog::empty());
        let b = WishDraft::new(d(b"pol")).speak("a module parser", &NormCatalog::empty());
        assert_eq!(a.draft_id, b.draft_id);
        assert!(a.verify_id());
        assert_ne!(a.evidence_bundle_id, Digest::ZERO, "CROSS-006");
        assert_eq!(a.evidence_bundle_id, Digest::of(&a.prose_history));
        let c = a
            .clone()
            .speak("and a function parse_line", &NormCatalog::empty());
        assert_ne!(c.draft_id, a.draft_id, "every round is a new state");
    }

    #[test]
    fn dictation_enters_accepted_but_proposals_never_do() {
        // THE governance pin: the machine proposes, only the operator
        // disposes — a suggestion cannot reach the wish without `accept`.
        let draft = WishDraft::new(d(b"p"))
            .speak("a module parser", &NormCatalog::empty())
            .propose([suggestion(WishFacet::test("parser_smoke"))]);
        assert_eq!(draft.accepted, vec![WishFacet::module("parser")]);
        assert_eq!(draft.pending.len(), 1);
        let wish = draft.to_wish();
        assert_eq!(
            wish.predicate_count(),
            1,
            "pending must not leak into the wish"
        );
        assert!(wish
            .predicates
            .iter()
            .all(|p| p.facet != WishFacet::test("parser_smoke")));
    }

    #[test]
    fn verdicts_move_resolve_and_retract() {
        let draft = WishDraft::new(d(b"p"))
            .speak("a module parser", &NormCatalog::empty())
            .propose([
                suggestion(WishFacet::doc("parser")),
                suggestion(WishFacet::test("parser_smoke")),
            ]);
        assert_eq!(draft.resolve(1), Some(DraftSlot::Accepted(0)));
        assert_eq!(draft.resolve(2), Some(DraftSlot::Pending(0)));
        assert_eq!(draft.resolve(4), None);
        assert_eq!(draft.resolve(0), None);

        let accepted_one = draft.clone().accept_pending(&[0]);
        assert_eq!(accepted_one.accepted.len(), 2);
        assert_eq!(accepted_one.pending.len(), 1);

        let rejected = draft.clone().reject_pending(&[0, 1]);
        assert_eq!(rejected.pending.len(), 0);
        assert_eq!(rejected.accepted.len(), 1);

        let retracted = accepted_one.retract_accepted(&[0]);
        assert_eq!(retracted.accepted.len(), 1);

        // Out-of-range indices are ignored, never panic.
        let same = draft.clone().accept_pending(&[99]);
        assert_eq!(same.pending.len(), draft.pending.len());
    }

    #[test]
    fn dictating_a_pending_suggestion_resolves_it() {
        let draft = WishDraft::new(d(b"p"))
            .speak("a module parser", &NormCatalog::empty())
            .propose([suggestion(WishFacet::test("parser_smoke"))])
            .speak("a test parser_smoke", &NormCatalog::empty());
        assert!(
            draft.pending.is_empty(),
            "dictation supersedes the proposal"
        );
        assert!(draft.accepted.contains(&WishFacet::test("parser_smoke")));
    }

    #[test]
    fn proposals_are_deduplicated_against_accepted_and_pending() {
        let draft = WishDraft::new(d(b"p"))
            .speak("a module parser", &NormCatalog::empty())
            .propose([
                suggestion(WishFacet::module("parser")), // already accepted
                suggestion(WishFacet::doc("parser")),
                suggestion(WishFacet::doc("parser")), // duplicate
            ]);
        assert_eq!(draft.pending.len(), 1);
    }

    #[test]
    fn companions_propose_doc_and_test_fibers_offline() {
        let draft = WishDraft::new(d(b"p")).speak(
            "a module parser and a function parse_line",
            &NormCatalog::empty(),
        );
        let companions = companion_suggestions(&draft, None);
        let facets: Vec<&WishFacet> = companions.iter().map(|s| &s.facet).collect();
        assert!(facets.contains(&&WishFacet::doc("parser")));
        assert!(facets.contains(&&WishFacet::test("parser_smoke")));
        assert!(facets.contains(&&WishFacet::doc("parse_line")));
        assert!(companions
            .iter()
            .all(|s| s.source == SuggestionSource::Observation));

        // Observed-present companions are not proposed.
        let observed = ObservedTopology::from_facets([WishFacet::doc("parser")]);
        let fewer = companion_suggestions(&draft, Some(&observed));
        assert!(fewer.iter().all(|s| s.facet != WishFacet::doc("parser")));

        // Accepted companions are not re-proposed.
        let with_doc = draft.speak("docs for parser", &NormCatalog::empty());
        let fewer = companion_suggestions(&with_doc, None);
        assert!(fewer.iter().all(|s| s.facet != WishFacet::doc("parser")));
    }

    #[test]
    fn to_wish_freezes_dialogue_label_and_accepted_facets() {
        let draft = WishDraft::new(d(b"p"))
            .speak("a module parser", &NormCatalog::empty())
            .speak("and docs for parser", &NormCatalog::empty());
        let wish = draft.to_wish();
        assert_eq!(wish.label, "a module parser; and docs for parser");
        assert_eq!(wish.predicate_count(), 2);
        assert_eq!(wish.evidence_bundle_id, draft.evidence_bundle_id);
        assert!(wish.is_evidence_bound());
    }

    #[test]
    fn round_language_is_exact_and_falls_closed_to_prose() {
        use AtelierCommand::*;
        assert_eq!(parse_atelier_command(""), Show);
        assert_eq!(parse_atelier_command("show"), Show);
        assert_eq!(
            parse_atelier_command("accept 1,3"),
            Accept(IndexSelection::These(vec![1, 3]))
        );
        assert_eq!(
            parse_atelier_command("accept 2 4"),
            Accept(IndexSelection::These(vec![2, 4]))
        );
        assert_eq!(
            parse_atelier_command("accept all"),
            Accept(IndexSelection::All)
        );
        assert_eq!(
            parse_atelier_command("reject 2"),
            Reject(IndexSelection::These(vec![2]))
        );
        assert_eq!(
            parse_atelier_command("drop 1"),
            Drop(IndexSelection::These(vec![1]))
        );
        assert_eq!(parse_atelier_command("realize"), Realize);
        assert_eq!(parse_atelier_command("Build it!"), Realize);
        // The decisive fail-closed cases: action words inside prose stay prose.
        assert!(matches!(
            parse_atelier_command("build a module parser"),
            Speak(_)
        ));
        assert!(matches!(
            parse_atelier_command("accept my apologies"),
            Speak(_)
        ));
        assert!(matches!(parse_atelier_command("a crate foo"), Speak(_)));
    }

    #[test]
    fn serde_round_trip_preserves_identity() {
        let draft = WishDraft::new(d(b"p"))
            .speak("a module parser", &NormCatalog::empty())
            .propose([suggestion(WishFacet::doc("parser"))])
            .with_questions(vec!["which input format?".into()]);
        let json = serde_json::to_string(&draft).unwrap();
        let back: WishDraft = serde_json::from_str(&json).unwrap();
        assert_eq!(back, draft);
        assert!(back.verify_id());
    }

    /// Type-level disease scan: the atelier vocabulary must be unable to
    /// carry paths, files or entities (same guard as ChatIntent).
    #[test]
    fn atelier_types_carry_no_paths_files_or_entities() {
        let src = include_str!("atelier.rs");
        let body = src.split("#[cfg(test)]").next().unwrap();
        for block_start in ["pub struct FacetSuggestion", "pub struct WishDraft"] {
            let block: String = body
                .lines()
                .skip_while(|l| !l.starts_with(block_start))
                .take_while(|l| !l.starts_with('}'))
                .filter(|l| {
                    let t = l.trim_start();
                    !t.starts_with("///") && !t.starts_with("//")
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!block.is_empty(), "scan must see {block_start}");
            for forbidden in ["PathBuf", "Path", "path", "file", "files", "entity", "dir"] {
                assert!(
                    !block.contains(forbidden),
                    "{block_start} must not carry '{forbidden}':\n{block}"
                );
            }
        }
    }
}
