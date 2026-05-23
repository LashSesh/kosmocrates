//! Crystal lifecycle management: temporal decay, staleness, consolidation.
//!
//! Health monitoring (`health.rs`) answers *what is the quality of my crystals
//! right now*.  Lifecycle management answers *what should I do about it*:
//!
//! - **Decay**: how much has a crystal's relevance eroded since it was committed?
//! - **Refresh score**: which crystals are most urgently in need of re-evaluation?
//! - **Consolidation candidates**: which crystals carry structurally redundant knowledge?
//!
//! ## Decay model
//!
//! Relevance decays as a function of `age = current_index − commit_index`:
//!
//! ```text
//! Linear:      d(age) = max(0, 1 − age / half_life)
//! Exponential: d(age) = exp(−age · ln(2) / half_life)
//! Step:        d(age) = 1  if age < half_life, else 0
//! ```
//!
//! All models return `d ∈ [0, 1]` with `d(0) = 1` (freshly committed).
//! `half_life` is expressed in **blocks** (commits), not wall-clock time, so
//! the decay rate scales naturally with the store's commit cadence.
//!
//! ## Refresh score
//!
//! `refresh_score = uncertainty × (1 − decay)` ∈ [0, 1].
//!
//! A crystal with high uncertainty *and* high age is the most urgent candidate
//! for re-evaluation.  `refresh_score = 0` means fully trusted and fresh.
//! `refresh_score = 1` means fully uncertain and completely decayed.
//!
//! ## Consolidation
//!
//! Two crystals are consolidation candidates when:
//! - They share a non-empty `metatron_canonical_hash` (structural isomorphism), or
//! - Their 8D IL vectors have cosine similarity ≥ `sim_threshold` (semantic overlap).
//!
//! Consolidation candidates are returned as pairs `(id_a, id_b, reason)`.
//! The store does *not* merge automatically — callers decide which crystal
//! to retain (typically the higher-QTIC, lower-uncertainty one).

use serde::{Deserialize, Serialize};

// ─── Decay model ─────────────────────────────────────────────────────────────

/// Temporal decay function family.
///
/// `half_life` is the number of blocks after which decay reaches 0.5.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DecayModel {
    /// Linearly interpolates from 1 (age=0) to 0 (age=half_life).
    /// Simple, interpretable.  Default for most deployments.
    Linear { half_life: f64 },
    /// Exponential: `d = exp(−age · ln2 / half_life)`.
    /// Always positive; asymptotically approaches 0 but never reaches it.
    Exponential { half_life: f64 },
    /// Step function: full relevance until `half_life`, then zero.
    /// Models hard expiry (e.g., regulatory data retention windows).
    Step { half_life: f64 },
}

impl DecayModel {
    /// Compute the decay factor `d ∈ [0, 1]` for a crystal of the given age.
    ///
    /// `age = current_block_index − commit_block_index` (≥ 0).
    pub fn decay(&self, age: f64) -> f64 {
        if age < 0.0 {
            return 1.0;
        }
        match self {
            Self::Linear { half_life } => {
                let hl = (*half_life).max(1.0);
                (1.0 - age / hl).max(0.0)
            }
            Self::Exponential { half_life } => {
                let hl = (*half_life).max(1.0);
                (-age * std::f64::consts::LN_2 / hl).exp()
            }
            Self::Step { half_life } => {
                if age < *half_life { 1.0 } else { 0.0 }
            }
        }
    }

    /// Half-life in blocks.
    pub fn half_life(&self) -> f64 {
        match self {
            Self::Linear { half_life }
            | Self::Exponential { half_life }
            | Self::Step { half_life } => *half_life,
        }
    }
}

impl Default for DecayModel {
    fn default() -> Self {
        Self::Exponential { half_life: 100.0 }
    }
}

// ─── Lifecycle status ─────────────────────────────────────────────────────────

/// Coarse lifecycle status of a crystal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleStatus {
    /// Fresh and high-quality — no action required.
    Vital,
    /// Moderate decay or moderate uncertainty — monitor.
    Aging,
    /// High decay and/or high uncertainty — refresh recommended.
    Stale,
    /// Structural duplicate detected — consolidation candidate.
    Redundant,
}

impl LifecycleStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vital => "vital",
            Self::Aging => "aging",
            Self::Stale => "stale",
            Self::Redundant => "redundant",
        }
    }
}

// ─── Per-crystal lifecycle view ───────────────────────────────────────────────

/// Lifecycle view of a single crystal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystalLifecycle {
    /// First 16 hex chars of the crystal ID.
    pub crystal_id: String,
    /// Age in blocks since this crystal was committed.
    pub age_blocks: i64,
    /// Decay factor `d ∈ [0, 1]` under the applied model.
    pub decay: f64,
    /// Epistemic uncertainty `u ∈ [0, 1]` (from `health::crystal_uncertainty`).
    pub uncertainty: f64,
    /// Refresh urgency: `refresh_score = uncertainty × (1 − decay)` ∈ [0, 1].
    /// Highest when both uncertainty is high and the crystal is old.
    pub refresh_score: f64,
    /// Coarse lifecycle status.
    pub status: LifecycleStatus,
}

impl CrystalLifecycle {
    /// One-line summary.
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} age={} d={:.2} u={:.2} refresh={:.2}",
            &self.crystal_id[..self.crystal_id.len().min(8)],
            self.status.label(),
            self.age_blocks,
            self.decay,
            self.uncertainty,
            self.refresh_score,
        )
    }
}

// ─── Consolidation candidate pair ────────────────────────────────────────────

/// Why two crystals are considered consolidation candidates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsolidationReason {
    /// Identical `metatron_canonical_hash` — topologically isomorphic regions.
    MetatronIsomorphic,
    /// 8D IL vector cosine similarity ≥ the configured threshold.
    SemanticOverlap,
}

/// A pair of crystals that may be safely consolidated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationCandidate {
    /// Crystal ID prefix of the first member.
    pub id_a: String,
    /// Crystal ID prefix of the second member.
    pub id_b: String,
    /// Why these two crystals are candidates.
    pub reason: ConsolidationReason,
    /// Cosine similarity of the two 8D IL vectors.
    pub vector_similarity: f64,
    /// Suggested crystal to *retain* (the one with higher QTIC class,
    /// breaking ties by lower uncertainty).
    pub retain: String,
    /// Suggested crystal to *deprecate*.
    pub deprecate: String,
}

// ─── Store-wide lifecycle report ──────────────────────────────────────────────

/// Lifecycle report for the entire IL store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleReport {
    /// Decay model applied.
    pub model: DecayModel,
    /// Block index used as "now" for age computation.
    pub reference_index: i64,
    /// Per-crystal lifecycle view, sorted by descending `refresh_score`.
    pub crystals: Vec<CrystalLifecycle>,
    /// Crystals with status `Stale` (refresh recommended).
    pub stale_count: usize,
    /// Crystals with status `Vital`.
    pub vital_count: usize,
    /// Mean decay factor across the store.
    pub mean_decay: f64,
    /// Mean refresh score across the store.
    pub mean_refresh_score: f64,
    /// Consolidation candidates found (empty when none detected).
    pub consolidation_candidates: Vec<ConsolidationCandidate>,
}

impl LifecycleReport {
    /// One-line summary.
    pub fn summary(&self) -> String {
        let n = self.crystals.len();
        if n == 0 {
            return "LifecycleReport: empty store".to_string();
        }
        format!(
            "LifecycleReport: {n} crystals | vital={v} stale={s} | \
             mean_d={d:.2} mean_refresh={r:.2} | {c} consolidation candidate(s)",
            n = n,
            v = self.vital_count,
            s = self.stale_count,
            d = self.mean_decay,
            r = self.mean_refresh_score,
            c = self.consolidation_candidates.len(),
        )
    }

    /// True when every crystal in the store is `Vital` and no consolidation
    /// candidates exist — the lifecycle fixpoint.
    pub fn is_lifecycle_closed(&self) -> bool {
        self.stale_count == 0 && self.consolidation_candidates.is_empty()
    }
}

// ─── Lifecycle status classification ─────────────────────────────────────────

/// Classify a crystal's lifecycle status from its decay and uncertainty.
///
/// | decay | uncertainty | status  |
/// |-------|-------------|---------|
/// | ≥0.8  | ≤0.25       | Vital   |
/// | <0.2  | any         | Stale   |
/// | any   | ≥0.65       | Stale   |
/// | else  |             | Aging   |
pub fn classify_lifecycle(decay: f64, uncertainty: f64) -> LifecycleStatus {
    if decay >= 0.8 && uncertainty <= 0.25 {
        LifecycleStatus::Vital
    } else if decay < 0.2 || uncertainty >= 0.65 {
        LifecycleStatus::Stale
    } else {
        LifecycleStatus::Aging
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── DecayModel tests ───────────────────────────────────────────────────

    #[test]
    fn linear_decay_is_one_at_age_zero() {
        let m = DecayModel::Linear { half_life: 50.0 };
        assert!((m.decay(0.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn linear_decay_is_half_at_half_life() {
        let m = DecayModel::Linear { half_life: 50.0 };
        assert!((m.decay(25.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn linear_decay_clamps_to_zero_beyond_half_life() {
        let m = DecayModel::Linear { half_life: 50.0 };
        assert_eq!(m.decay(100.0), 0.0);
        assert_eq!(m.decay(200.0), 0.0);
    }

    #[test]
    fn exponential_decay_is_one_at_age_zero() {
        let m = DecayModel::Exponential { half_life: 50.0 };
        assert!((m.decay(0.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn exponential_decay_is_half_at_half_life() {
        let m = DecayModel::Exponential { half_life: 50.0 };
        assert!((m.decay(50.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn exponential_decay_never_reaches_zero() {
        let m = DecayModel::Exponential { half_life: 50.0 };
        assert!(m.decay(1000.0) > 0.0);
    }

    #[test]
    fn step_decay_is_one_before_half_life() {
        let m = DecayModel::Step { half_life: 10.0 };
        assert_eq!(m.decay(9.0), 1.0);
    }

    #[test]
    fn step_decay_is_zero_at_and_after_half_life() {
        let m = DecayModel::Step { half_life: 10.0 };
        assert_eq!(m.decay(10.0), 0.0);
        assert_eq!(m.decay(100.0), 0.0);
    }

    #[test]
    fn negative_age_clamps_to_one() {
        for model in [
            DecayModel::Linear { half_life: 50.0 },
            DecayModel::Exponential { half_life: 50.0 },
            DecayModel::Step { half_life: 50.0 },
        ] {
            assert_eq!(model.decay(-5.0), 1.0, "negative age must give decay=1 for {model:?}");
        }
    }

    // ── classify_lifecycle tests ───────────────────────────────────────────

    #[test]
    fn fresh_low_uncertainty_is_vital() {
        assert_eq!(classify_lifecycle(0.9, 0.1), LifecycleStatus::Vital);
    }

    #[test]
    fn old_decayed_crystal_is_stale() {
        assert_eq!(classify_lifecycle(0.1, 0.5), LifecycleStatus::Stale);
    }

    #[test]
    fn high_uncertainty_is_stale_regardless_of_decay() {
        assert_eq!(classify_lifecycle(0.9, 0.7), LifecycleStatus::Stale);
    }

    #[test]
    fn moderate_decay_and_uncertainty_is_aging() {
        assert_eq!(classify_lifecycle(0.5, 0.4), LifecycleStatus::Aging);
    }

    // ── Refresh score ──────────────────────────────────────────────────────

    #[test]
    fn refresh_score_is_zero_for_fresh_certain_crystal() {
        // decay=1.0 → 1 − decay = 0 → refresh_score = 0
        let decay = 1.0f64;
        let uncertainty = 0.0f64;
        let rs = uncertainty * (1.0 - decay);
        assert!(rs.abs() < 1e-9);
    }

    #[test]
    fn refresh_score_is_high_for_old_uncertain_crystal() {
        let decay = 0.05f64;
        let uncertainty = 0.9f64;
        let rs = uncertainty * (1.0 - decay);
        assert!(rs > 0.8, "old + uncertain → high refresh score, got {rs}");
    }

    // ── LifecycleReport ────────────────────────────────────────────────────

    #[test]
    fn empty_lifecycle_report_summary() {
        let report = LifecycleReport {
            model: DecayModel::default(),
            reference_index: 0,
            crystals: vec![],
            stale_count: 0,
            vital_count: 0,
            mean_decay: 0.0,
            mean_refresh_score: 0.0,
            consolidation_candidates: vec![],
        };
        assert!(report.summary().contains("empty"));
        assert!(report.is_lifecycle_closed());
    }

    #[test]
    fn lifecycle_closed_when_all_vital_and_no_candidates() {
        let report = LifecycleReport {
            model: DecayModel::default(),
            reference_index: 10,
            crystals: vec![CrystalLifecycle {
                crystal_id: "aabbccdd00112233".to_string(),
                age_blocks: 2,
                decay: 0.95,
                uncertainty: 0.1,
                refresh_score: 0.005,
                status: LifecycleStatus::Vital,
            }],
            stale_count: 0,
            vital_count: 1,
            mean_decay: 0.95,
            mean_refresh_score: 0.005,
            consolidation_candidates: vec![],
        };
        assert!(report.is_lifecycle_closed());
    }

    #[test]
    fn lifecycle_not_closed_when_stale_exists() {
        let report = LifecycleReport {
            model: DecayModel::default(),
            reference_index: 100,
            crystals: vec![CrystalLifecycle {
                crystal_id: "aabbccdd00112233".to_string(),
                age_blocks: 95,
                decay: 0.08,
                uncertainty: 0.8,
                refresh_score: 0.74,
                status: LifecycleStatus::Stale,
            }],
            stale_count: 1,
            vital_count: 0,
            mean_decay: 0.08,
            mean_refresh_score: 0.74,
            consolidation_candidates: vec![],
        };
        assert!(!report.is_lifecycle_closed());
    }
}
