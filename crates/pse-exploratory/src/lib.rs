//! Exploratory Ledger — hallucination as epistemic instrument.
//!
//! A crystal with coherence potential ψ < 0 is *exploratory*: it sits in a
//! high-free-energy region of the epistemic space, far from any confirmed
//! attractor.  The model produced it, but no evidence has yet grounded it.
//!
//! Instead of discarding these crystals, the exploratory ledger parks them as
//! *hypotheses* and waits for evidence to either confirm (land) or decay them.
//!
//! ## Lifecycle
//!
//! ```text
//!   commit, ψ < 0  ──►  Pending   ──►  same rule_id, new ψ ≥ 0  ──►  Landed
//!                              │
//!                              └──►  run > added_at + decay_after  ──►  Decayed
//! ```
//!
//! - **Pending** → hypothesis lives in the ledger, surfaces as `UnknownSlot`
//! - **Landed**  → evidence arrived; crystal promoted to grounded knowledge
//! - **Decayed** → evidence never came; surfaces as `Stale` `UnknownSlot`
//!
//! ## ψ as a coherence signal
//!
//! ψ = kuramoto_coherence − (1 − stability_score)
//!
//! Negative ψ means the crystal's self-organising pressure is outweighed by its
//! instability — it cannot sustain itself as an attractor.  Positive ψ means
//! the crystal has become a self-reinforcing knowledge node.
//!
//! The landing threshold is ψ ≥ 0: the crystal has crossed the attractor
//! boundary and is now contributing positive coherence to the system.

use pse_nxalien_types::{UnknownSlot, UnknownStatus};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Crystals with ψ below this value are parked in the exploratory ledger.
pub const EXPLORATORY_PSI_THRESHOLD: f64 = 0.0;

/// Default decay window: after this many runs without landing, the entry decays.
pub const DEFAULT_DECAY_AFTER_RUNS: u64 = 10;

// ── Status ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum EntryStatus {
    Pending,
    Landed {
        grounded_at_run: u64,
        /// ψ of the grounding crystal.
        grounded_psi: f64,
    },
    Decayed {
        decayed_at_run: u64,
    },
}

// ── Entry ─────────────────────────────────────────────────────────────────────

/// A single exploratory hypothesis in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploratoryEntry {
    pub rule_id: String,
    /// ψ at ingest time (always < 0).
    pub initial_psi: f64,
    /// QTIC class at ingest time (typically Q0 or Q1).
    pub initial_qtic: u8,
    /// Block hash prefix from the originating IL commit.
    pub block_hash_prefix: String,
    pub added_at_run: u64,
    /// Entry decays if still Pending after this many runs.
    pub decay_after_runs: u64,
    pub status: EntryStatus,
}

impl ExploratoryEntry {
    pub fn is_pending(&self) -> bool {
        self.status == EntryStatus::Pending
    }
}

// ── Events ────────────────────────────────────────────────────────────────────

/// A hypothesis that has been confirmed by new evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingEvent {
    pub rule_id: String,
    pub initial_psi: f64,
    pub grounded_psi: f64,
    pub runs_pending: u64,
}

/// A hypothesis that expired without being confirmed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayEvent {
    pub rule_id: String,
    pub initial_psi: f64,
    pub runs_pending: u64,
}

// ── Summary ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExploratoryLedgerSummary {
    pub total: usize,
    pub pending: usize,
    pub landed: usize,
    pub decayed: usize,
    /// Mean ψ of all currently Pending entries.
    pub mean_pending_psi: f64,
    pub pending_entries: Vec<PendingSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSummary {
    pub rule_id: String,
    pub initial_psi: f64,
    pub initial_qtic: u8,
    pub added_at_run: u64,
    /// Positive = runs remaining before decay.  Negative = already overdue.
    pub runs_until_decay: i64,
}

// ── Ledger ────────────────────────────────────────────────────────────────────

/// File-backed store for exploratory hypotheses.
///
/// Lives at `<nxalien_dir>/exploratory.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExploratoryLedger {
    pub entries: Vec<ExploratoryEntry>,
}

impl ExploratoryLedger {
    /// Load from `dir/exploratory.json`, or return an empty ledger.
    pub fn open(dir: &Path) -> Self {
        let file = dir.join("exploratory.json");
        if file.exists() {
            if let Ok(s) = std::fs::read_to_string(&file) {
                if let Ok(ledger) = serde_json::from_str(&s) {
                    return ledger;
                }
            }
        }
        Self::default()
    }

    /// Persist to `dir/exploratory.json`.
    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let s = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(dir.join("exploratory.json"), s)
    }

    /// Ingest a new exploratory candidate.
    ///
    /// No-op if `psi >= EXPLORATORY_PSI_THRESHOLD` (not exploratory).
    /// Idempotent: skips if the rule already has a Pending entry.
    pub fn ingest(
        &mut self,
        rule_id: impl Into<String>,
        psi: f64,
        qtic: u8,
        block_hash_prefix: impl Into<String>,
        run_count: u64,
        decay_after_runs: u64,
    ) {
        if psi >= EXPLORATORY_PSI_THRESHOLD {
            return;
        }
        let id = rule_id.into();
        if self
            .entries
            .iter()
            .any(|e| e.rule_id == id && e.is_pending())
        {
            return;
        }
        self.entries.push(ExploratoryEntry {
            rule_id: id,
            initial_psi: psi,
            initial_qtic: qtic,
            block_hash_prefix: block_hash_prefix.into(),
            added_at_run: run_count,
            decay_after_runs,
            status: EntryStatus::Pending,
        });
    }

    /// Check pending entries against incoming grounded results.
    ///
    /// `grounded` is a slice of `(rule_id, psi)` pairs from the current run.
    /// A pending entry "lands" when its rule_id reappears with ψ ≥ 0 —
    /// meaning the same rule has now acquired enough evidence to become a
    /// self-sustaining attractor.
    pub fn check_landings(
        &mut self,
        grounded: &[(&str, f64)],
        current_run: u64,
    ) -> Vec<LandingEvent> {
        let mut events = Vec::new();
        for entry in self.entries.iter_mut() {
            if !entry.is_pending() {
                continue;
            }
            if let Some(&(_, new_psi)) = grounded.iter().find(|(id, _)| *id == entry.rule_id) {
                if new_psi >= EXPLORATORY_PSI_THRESHOLD {
                    events.push(LandingEvent {
                        rule_id: entry.rule_id.clone(),
                        initial_psi: entry.initial_psi,
                        grounded_psi: new_psi,
                        runs_pending: current_run.saturating_sub(entry.added_at_run),
                    });
                    entry.status = EntryStatus::Landed {
                        grounded_at_run: current_run,
                        grounded_psi: new_psi,
                    };
                }
            }
        }
        events
    }

    /// Advance the decay clock.
    ///
    /// Pending entries older than their `decay_after_runs` window transition
    /// to Decayed and are returned as `DecayEvent`s.
    pub fn tick_decay(&mut self, current_run: u64) -> Vec<DecayEvent> {
        let mut events = Vec::new();
        for entry in self.entries.iter_mut() {
            if !entry.is_pending() {
                continue;
            }
            if current_run >= entry.added_at_run + entry.decay_after_runs {
                events.push(DecayEvent {
                    rule_id: entry.rule_id.clone(),
                    initial_psi: entry.initial_psi,
                    runs_pending: entry.decay_after_runs,
                });
                entry.status = EntryStatus::Decayed {
                    decayed_at_run: current_run,
                };
            }
        }
        events
    }

    /// Convert Pending and Decayed entries to `UnknownSlot`s.
    ///
    /// These slots feed the EpistemicAgenda — the agent sees them as
    /// knowledge gaps that need evidence, not as errors to suppress.
    pub fn to_unknown_slots(&self) -> Vec<UnknownSlot> {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, EntryStatus::Pending | EntryStatus::Decayed { .. }))
            .map(|e| {
                let (status, note) = match &e.status {
                    EntryStatus::Pending => (
                        UnknownStatus::Unknown,
                        format!(
                            "exploratory crystal ψ={:.3} (Q{}) — awaiting grounding evidence",
                            e.initial_psi, e.initial_qtic
                        ),
                    ),
                    EntryStatus::Decayed { decayed_at_run } => (
                        UnknownStatus::Stale,
                        format!(
                            "hypothesis decayed at run {} after {} runs without evidence",
                            decayed_at_run, e.decay_after_runs
                        ),
                    ),
                    _ => unreachable!(),
                };
                // Confidence proxy: distance from threshold (higher ψ → closer to landing).
                // ψ ∈ (-∞, 0) → confidence ∈ (0, 1) clamped.
                let confidence = (1.0 + e.initial_psi).clamp(0.0, 0.99);
                UnknownSlot {
                    name: format!("exploratory-{}", e.rule_id),
                    domain: vec!["exploratory".to_string(), e.rule_id.clone()],
                    status,
                    candidates: vec![note],
                    evidence: vec![],
                    resolution: None,
                    confidence,
                    expires: None,
                }
            })
            .collect()
    }

    pub fn pending_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_pending()).count()
    }

    pub fn landed_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, EntryStatus::Landed { .. }))
            .count()
    }

    pub fn decayed_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, EntryStatus::Decayed { .. }))
            .count()
    }

    pub fn summary(&self, current_run: u64) -> ExploratoryLedgerSummary {
        let pending: Vec<&ExploratoryEntry> =
            self.entries.iter().filter(|e| e.is_pending()).collect();

        let mean_psi = if pending.is_empty() {
            0.0
        } else {
            pending.iter().map(|e| e.initial_psi).sum::<f64>() / pending.len() as f64
        };

        ExploratoryLedgerSummary {
            total: self.entries.len(),
            pending: pending.len(),
            landed: self.landed_count(),
            decayed: self.decayed_count(),
            mean_pending_psi: mean_psi,
            pending_entries: pending
                .iter()
                .map(|e| PendingSummary {
                    rule_id: e.rule_id.clone(),
                    initial_psi: e.initial_psi,
                    initial_qtic: e.initial_qtic,
                    added_at_run: e.added_at_run,
                    runs_until_decay: (e.added_at_run + e.decay_after_runs) as i64
                        - current_run as i64,
                })
                .collect(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger_with_entry(psi: f64, run: u64, decay: u64) -> ExploratoryLedger {
        let mut l = ExploratoryLedger::default();
        l.ingest("rule-a", psi, 1, "abc123", run, decay);
        l
    }

    // ── Ingest ─────────────────────────────────────────────────────────────

    #[test]
    fn positive_psi_not_ingested() {
        let mut l = ExploratoryLedger::default();
        l.ingest("r", 0.1, 2, "x", 1, 10);
        assert_eq!(
            l.entries.len(),
            0,
            "positive ψ must not enter exploratory ledger"
        );
    }

    #[test]
    fn zero_psi_not_ingested() {
        let mut l = ExploratoryLedger::default();
        l.ingest("r", 0.0, 2, "x", 1, 10);
        assert_eq!(l.entries.len(), 0);
    }

    #[test]
    fn negative_psi_ingested() {
        let mut l = ExploratoryLedger::default();
        l.ingest("r", -0.35, 1, "abc", 1, 10);
        assert_eq!(l.entries.len(), 1);
        assert!(l.entries[0].is_pending());
    }

    #[test]
    fn ingest_is_idempotent_per_rule() {
        let mut l = ExploratoryLedger::default();
        l.ingest("r", -0.3, 1, "a", 1, 10);
        l.ingest("r", -0.4, 1, "b", 2, 10); // same rule, different run
        assert_eq!(
            l.pending_count(),
            1,
            "second ingest for same pending rule must be ignored"
        );
    }

    // ── Landing ────────────────────────────────────────────────────────────

    #[test]
    fn landing_on_positive_psi() {
        let mut l = ledger_with_entry(-0.35, 1, 10);
        let grounded = [("rule-a", 0.12)];
        let events = l.check_landings(&grounded, 3);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].rule_id, "rule-a");
        assert_eq!(events[0].runs_pending, 2);
        assert_eq!(l.landed_count(), 1);
        assert_eq!(l.pending_count(), 0);
    }

    #[test]
    fn no_landing_when_still_negative() {
        let mut l = ledger_with_entry(-0.35, 1, 10);
        let grounded = [("rule-a", -0.1)];
        let events = l.check_landings(&grounded, 2);
        assert!(events.is_empty());
        assert_eq!(l.pending_count(), 1, "still negative ψ must stay Pending");
    }

    #[test]
    fn landing_ignores_different_rule() {
        let mut l = ledger_with_entry(-0.35, 1, 10);
        let grounded = [("rule-b", 0.5)];
        let events = l.check_landings(&grounded, 2);
        assert!(events.is_empty());
        assert_eq!(l.pending_count(), 1);
    }

    #[test]
    fn already_landed_entry_not_re_landed() {
        let mut l = ledger_with_entry(-0.35, 1, 10);
        l.check_landings(&[("rule-a", 0.2)], 2);
        assert_eq!(l.landed_count(), 1);
        let events2 = l.check_landings(&[("rule-a", 0.5)], 3);
        assert!(
            events2.is_empty(),
            "already-landed entry must not fire again"
        );
        assert_eq!(l.landed_count(), 1);
    }

    // ── Decay ──────────────────────────────────────────────────────────────

    #[test]
    fn decay_fires_after_window() {
        let mut l = ledger_with_entry(-0.35, 1, 5);
        let decays = l.tick_decay(6); // run 1 + 5 = 6
        assert_eq!(decays.len(), 1);
        assert_eq!(decays[0].rule_id, "rule-a");
        assert_eq!(l.decayed_count(), 1);
        assert_eq!(l.pending_count(), 0);
    }

    #[test]
    fn decay_does_not_fire_before_window() {
        let mut l = ledger_with_entry(-0.35, 1, 5);
        let decays = l.tick_decay(5); // run 1 + 5 - 1 = 5, not yet
        assert!(decays.is_empty());
        assert_eq!(l.pending_count(), 1);
    }

    #[test]
    fn landed_entry_does_not_decay() {
        let mut l = ledger_with_entry(-0.35, 1, 3);
        l.check_landings(&[("rule-a", 0.1)], 2);
        let decays = l.tick_decay(10);
        assert!(decays.is_empty(), "landed entry must not decay");
    }

    // ── UnknownSlots ───────────────────────────────────────────────────────

    #[test]
    fn pending_entry_surfaces_as_unknown() {
        let l = ledger_with_entry(-0.35, 1, 10);
        let slots = l.to_unknown_slots();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].status, UnknownStatus::Unknown);
        assert!(slots[0].name.starts_with("exploratory-"));
    }

    #[test]
    fn decayed_entry_surfaces_as_stale() {
        let mut l = ledger_with_entry(-0.35, 1, 5);
        l.tick_decay(6);
        let slots = l.to_unknown_slots();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].status, UnknownStatus::Stale);
    }

    #[test]
    fn landed_entry_does_not_surface_as_unknown() {
        let mut l = ledger_with_entry(-0.35, 1, 10);
        l.check_landings(&[("rule-a", 0.2)], 2);
        let slots = l.to_unknown_slots();
        assert!(
            slots.is_empty(),
            "landed entries are grounded — no longer unknowns"
        );
    }

    // ── Confidence proxy ───────────────────────────────────────────────────

    #[test]
    fn confidence_increases_as_psi_approaches_zero() {
        let mut l1 = ExploratoryLedger::default();
        let mut l2 = ExploratoryLedger::default();
        l1.ingest("r1", -0.9, 0, "x", 1, 10); // very negative
        l2.ingest("r2", -0.1, 0, "x", 1, 10); // barely negative
        let s1 = l1.to_unknown_slots();
        let s2 = l2.to_unknown_slots();
        assert!(
            s2[0].confidence > s1[0].confidence,
            "confidence must be higher when ψ is closer to 0"
        );
    }

    // ── Persistence ────────────────────────────────────────────────────────

    #[test]
    fn roundtrip_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut l = ExploratoryLedger::default();
        l.ingest("r1", -0.3, 1, "abc", 1, 10);
        l.save(dir.path()).unwrap();

        let l2 = ExploratoryLedger::open(dir.path());
        assert_eq!(l2.entries.len(), 1);
        assert_eq!(l2.entries[0].rule_id, "r1");
    }
}
