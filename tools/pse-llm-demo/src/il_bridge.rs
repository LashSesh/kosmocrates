//! Optional Infinity Ledger integration for pse-llm-demo.
//!
//! Activated by setting `PSE_IL_STORE` to a directory path.  When active:
//!   - Every new crystal is committed to the IL file-based ledger.
//!   - The IL pipeline returns a `ValidationFeedback` which is used to
//!     produce a *refined* crystal (stability blended 70% PSE + 30% IL).
//!   - On Session 3+, the LLM question is embedded as an 8D vector and used
//!     to retrieve fuzzy-similar crystals from the IL index.
//!   - The fuzzy-retrieved records supplement the deterministic PSE recall.
//!
//! When `PSE_IL_STORE` is not set the bridge is a no-op and zero overhead.

use pse_adapter_il::{
    adapter::text_to_vector8,
    feedback::refine_crystal,
    store::{ILMatch, ILStore},
};
use pse_types::SemanticCrystal;

use crate::memory::CrystalRecord;

pub struct ILBridge {
    store: Option<ILStore>,
}

/// Outcome of an IL commit — carries the refined crystal when the feedback
/// loop produced a meaningful stability correction.
pub struct CommitOutcome {
    pub block_hash: String,
    /// Some when |IL stability − PSE stability| > 0.02; None otherwise.
    pub refined: Option<SemanticCrystal>,
}

impl ILBridge {
    /// Construct from environment.  Set `PSE_IL_STORE` to a path to activate.
    pub fn from_env() -> Self {
        let store = std::env::var("PSE_IL_STORE").ok().and_then(|path| {
            match ILStore::open(&path, "pse-llm-demo") {
                Ok(s) => {
                    println!("  IL store  : {path} ({} block(s))", s.len());
                    Some(s)
                }
                Err(e) => {
                    eprintln!("  [IL] Warning: cannot open store at {path}: {e}");
                    None
                }
            }
        });
        Self { store }
    }

    /// Commit a crystal to the IL ledger and run the feedback loop.
    ///
    /// Returns `Some(CommitOutcome)` when IL is active, `None` when disabled.
    /// `CommitOutcome.refined` is `Some` when the IL signal differs from the
    /// PSE stability by more than 0.02 — the caller should store both crystals
    /// and use the refined one as the canonical record for this session.
    pub fn commit(
        &mut self,
        crystal: &SemanticCrystal,
        source_chunks: &[String],
        session: usize,
        question: &str,
    ) -> Option<CommitOutcome> {
        let store = self.store.as_mut()?;

        match store.commit_with_feedback(crystal, source_chunks, session, question) {
            Ok(feedback) => {
                let short: String = feedback.block_hash.chars().take(12).collect();
                println!(
                    "  IL commit : {}… | ψ={:.3} | converged={} | gate={}",
                    short,
                    feedback.coherence_potential,
                    feedback.converged,
                    feedback.gate_passed,
                );

                // Produce a refined crystal only when the delta is non-trivial.
                let delta = (feedback.il_stability - crystal.stability_score).abs();
                let refined = if delta > 0.02 {
                    let r = refine_crystal(crystal, &feedback, crystal.created_at + 1);
                    println!(
                        "  IL refine : stability {:.3} → {:.3} (Δ={:+.3})",
                        crystal.stability_score,
                        r.stability_score,
                        r.stability_score - crystal.stability_score,
                    );
                    Some(r)
                } else {
                    None
                };

                Some(CommitOutcome { block_hash: feedback.block_hash, refined })
            }
            Err(e) => {
                eprintln!("  [IL] commit error: {e}");
                None
            }
        }
    }

    /// Query the IL index with `question` and return the `top_k` crystal
    /// records from `all_records` that are most similar by cosine distance.
    ///
    /// Returns an empty vec when IL is disabled or when no match exceeds 0.5.
    pub fn query_similar<'a>(
        &self,
        question: &str,
        top_k: usize,
        all_records: &'a [CrystalRecord],
    ) -> Vec<(&'a CrystalRecord, f64)> {
        let store = match &self.store {
            Some(s) => s,
            None => return vec![],
        };
        if store.is_empty() {
            return vec![];
        }

        let q_vec = text_to_vector8(question);
        let hits: Vec<ILMatch> = store.search(&q_vec, top_k);

        hits.into_iter()
            .filter(|h| h.score >= 0.5)
            .filter_map(|h| {
                all_records
                    .iter()
                    .find(|r| {
                        let id_hex: String = r
                            .crystal
                            .crystal_id
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect();
                        id_hex == h.crystal_id_hex
                    })
                    .map(|r| (r, h.score))
            })
            .collect()
    }

    pub fn is_active(&self) -> bool {
        self.store.is_some()
    }
}
