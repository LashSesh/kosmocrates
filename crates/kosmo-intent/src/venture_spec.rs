//! Venture specs — an operator-authored JSON file becomes a [`Venture`].
//!
//! A spec stage carries its wish as **prose**, compiled through the same
//! deterministic grammar as wish mode — including the operator's promoted
//! norm triggers — so a venture stage's wish is *exactly* the wish the same
//! prose would produce at the front door (pinned by test). Dependencies are
//! 0-based stage indices; the fail-closed structural checks (range, self,
//! cycle) live in [`Venture::new`] and surface here with stage labels.
//!
//! ```json
//! {
//!   "label": "tiny engine",
//!   "stages": [
//!     { "label": "core",  "wish": "a module engine and a function run_engine" },
//!     { "label": "docs",  "wish": "docs for engine", "after": [0] }
//!   ]
//! }
//! ```

use kosmo_core::{Digest, Venture, VentureStage};
use serde::Deserialize;

use crate::{compile_wish_with_norms, NormCatalog};

/// One spec stage: a label, the wish as prose, and prerequisite indices.
#[derive(Clone, Debug, Deserialize)]
pub struct VentureStageSpec {
    pub label: String,
    pub wish: String,
    #[serde(default)]
    pub after: Vec<usize>,
}

/// The operator-authored venture file.
#[derive(Clone, Debug, Deserialize)]
pub struct VentureSpec {
    pub label: String,
    pub stages: Vec<VentureStageSpec>,
}

/// Compile a venture spec (JSON text) into a content-addressed [`Venture`].
///
/// Each stage's prose runs through [`compile_wish_with_norms`] with the
/// given catalog; the per-stage evidence is the digest of its prose (the
/// wish-mode convention, so stage wishes are byte-identical to front-door
/// wishes). A stage whose prose compiles to **zero facets** is a hard error
/// — a venture must not silently carry vacuous stages. Structural errors
/// (range/self/cycle) are reported with the offending stage's label.
pub fn compile_venture(
    spec_json: &str,
    catalog: &NormCatalog,
    policy_id: Digest,
    evidence_bundle_id: Digest,
) -> Result<Venture, String> {
    let spec: VentureSpec =
        serde_json::from_str(spec_json).map_err(|e| format!("invalid venture spec: {e}"))?;
    let mut stages = Vec::with_capacity(spec.stages.len());
    for s in &spec.stages {
        let wish = compile_wish_with_norms(
            &s.wish,
            catalog,
            policy_id,
            Digest::of_bytes(s.wish.as_bytes()),
        );
        if wish.predicate_count() == 0 {
            return Err(format!(
                "stage '{}': the prose \u{201c}{}\u{201d} compiles to no measurable facet",
                s.label, s.wish
            ));
        }
        stages.push(VentureStage::new(&s.label, wish, s.after.iter().copied()));
    }
    Venture::new(spec.label, stages, policy_id, evidence_bundle_id).map_err(|e| {
        let labels: Vec<&str> = spec.stages.iter().map(|s| s.label.as_str()).collect();
        format!("invalid venture (stages {labels:?}): {e}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_wish;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    const SPEC: &str = r#"{
        "label": "tiny engine",
        "stages": [
            { "label": "core", "wish": "a module engine and a function run_engine" },
            { "label": "docs", "wish": "docs for engine", "after": [0] },
            { "label": "smoke", "wish": "a test engine_smoke", "after": [0] }
        ]
    }"#;

    #[test]
    fn spec_compiles_to_a_content_addressed_venture() {
        let v = compile_venture(SPEC, &NormCatalog::empty(), d(b"pol"), d(b"ev")).unwrap();
        assert_eq!(v.stage_count(), 3);
        assert!(v.verify_id());
        assert_eq!(v.execution_order(), vec![0, 1, 2]);
        assert_eq!(v.stages[1].after, vec![0]);
        // Determinism: same spec, same identity.
        let again = compile_venture(SPEC, &NormCatalog::empty(), d(b"pol"), d(b"ev")).unwrap();
        assert_eq!(again.venture_id, v.venture_id);
    }

    #[test]
    fn stage_wishes_are_byte_identical_to_front_door_wishes() {
        // The consistency pin: a venture stage IS the wish the same prose
        // would produce at the front door — same content address.
        let v = compile_venture(SPEC, &NormCatalog::empty(), d(b"pol"), d(b"ev")).unwrap();
        let prose = "a module engine and a function run_engine";
        let front_door = compile_wish(prose, d(b"pol"), Digest::of_bytes(prose.as_bytes()));
        assert_eq!(v.stages[0].wish.id, front_door.id);
    }

    #[test]
    fn vacuous_stages_and_structural_faults_are_hard_errors() {
        let vacuous = r#"{"label":"x","stages":[{"label":"hollow","wish":"do something nice"}]}"#;
        let err = compile_venture(vacuous, &NormCatalog::empty(), d(b"p"), d(b"e")).unwrap_err();
        assert!(err.contains("hollow"), "{err}");
        assert!(err.contains("no measurable facet"), "{err}");

        let cyclic = r#"{"label":"x","stages":[
            {"label":"a","wish":"a module a1","after":[1]},
            {"label":"b","wish":"a module b1","after":[0]}
        ]}"#;
        let err = compile_venture(cyclic, &NormCatalog::empty(), d(b"p"), d(b"e")).unwrap_err();
        assert!(err.contains("cycle"), "{err}");

        let garbled = compile_venture("not json", &NormCatalog::empty(), d(b"p"), d(b"e"));
        assert!(garbled.unwrap_err().contains("invalid venture spec"));
    }
}
