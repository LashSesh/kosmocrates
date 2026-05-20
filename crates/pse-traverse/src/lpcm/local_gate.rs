//! Hysteretic 51% local majority gate (§10.3, §10.4).
//!
//! Gate passes iff ZU = max(wᵢ) >= theta_activate and no tie exists.
//! Tie policy: TieNoCondense by default (§6.4).

use serde::{Deserialize, Serialize};

use super::primitives::{fixed_ge, Fixed, Hash256};
use super::run_descriptor::{LpcmThresholds, TiePolicy};
use super::support_mass::SupportMassVector;

/// The hysteretic condensation bit for a fragment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalCondensationBit {
    /// Content address of this bit.
    pub bit_id: Hash256,
    /// Fragment this bit belongs to.
    pub fragment_id: Hash256,
    /// Winning candidate id (if active).
    pub winning_candidate_id: Option<Hash256>,
    /// Z_max: max normalized weight.
    pub z_max: Fixed,
    /// Runner-up weight.
    pub runner_up: Fixed,
    /// Dominance margin = z_max - runner_up.
    pub dominance_margin: Fixed,
    /// Whether the gate is currently active (condensed).
    pub active: bool,
    /// Condensation status.
    pub status: LocalCondensationStatus,
    /// Failure kind if not active.
    pub failure: Option<super::LpcmFailureKind>,
}

/// Status of the local condensation gate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LocalCondensationStatus {
    /// Below deactivation threshold.
    Dormant,
    /// Local majority gate fired.
    CondensedLocal,
    /// Tie detected — not condensed per TieNoCondense policy.
    TieNoCondense,
    /// No candidates produced non-zero support.
    ZeroSupport,
    /// Topology guard rejected the patch.
    TopologyRejected,
    /// Missing provenance on required evidence.
    ProvenanceRejected,
}

/// An accepted local condensation candidate (emitted when gate is active).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalCondensationCandidate {
    pub local_candidate_id: Hash256,
    pub fragment_id: Hash256,
    pub patch_id: Hash256,
    pub candidate_direction_id: Hash256,
    pub support_vector_id: Hash256,
    pub bit_id: Hash256,
    pub evidence_refs: Vec<super::primitives::EvidenceRef>,
}

/// Evaluate the hysteretic local majority gate for one fragment.
pub fn evaluate_local_condensation(
    fragment_id: &Hash256,
    support: &SupportMassVector,
    thresholds: &LpcmThresholds,
    tie_policy: &TiePolicy,
    previous_active: bool,
) -> LocalCondensationBit {
    use super::primitives::lpcm_content_address;
    use super::primitives::fixed_sub;

    if !support.valid {
        let bit_id = lpcm_content_address(&(fragment_id, "zero-support"))
            .unwrap_or(Hash256::zero());
        return LocalCondensationBit {
            bit_id,
            fragment_id: fragment_id.clone(),
            winning_candidate_id: None,
            z_max: Fixed::zero(),
            runner_up: Fixed::zero(),
            dominance_margin: Fixed::zero(),
            active: false,
            status: LocalCondensationStatus::ZeroSupport,
            failure: Some(super::LpcmFailureKind::ZeroAdmissibleSupport),
        };
    }

    // Sort weights by candidate_id for determinism.
    let mut sorted: Vec<(&Hash256, &Fixed)> = support.weights.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);

    let z_max = sorted
        .iter()
        .map(|(_, w)| *w)
        .max()
        .cloned()
        .unwrap_or(Fixed::zero());

    // Find winner and runner-up.
    let mut winners: Vec<&Hash256> = sorted
        .iter()
        .filter(|(_, w)| **w == z_max)
        .map(|(k, _)| *k)
        .collect();

    let runner_up = sorted
        .iter()
        .filter(|(_, w)| **w != z_max)
        .map(|(_, w)| *w)
        .max()
        .cloned()
        .unwrap_or(Fixed::zero());

    let dominance_margin = fixed_sub(&z_max, &runner_up);

    // Tie detection: more than one candidate with identical z_max.
    let tie = winners.len() > 1;
    if tie {
        if let TiePolicy::TieNoCondense = tie_policy {
            let bit_id = lpcm_content_address(&(fragment_id, "tie"))
                .unwrap_or(Hash256::zero());
            return LocalCondensationBit {
                bit_id,
                fragment_id: fragment_id.clone(),
                winning_candidate_id: None,
                z_max,
                runner_up,
                dominance_margin,
                active: false,
                status: LocalCondensationStatus::TieNoCondense,
                failure: Some(super::LpcmFailureKind::TieNoCondense),
            };
        }
        // LexicographicId tie-break: pick smallest id.
        winners.sort();
        winners.truncate(1);
    }

    let winning_id = winners.into_iter().next().cloned();

    // Hysteretic gate: if previously active, require z >= theta_deactivate to stay active.
    // If previously inactive, require z >= theta_activate to become active.
    let (passes, status) = if previous_active {
        if fixed_ge(&z_max, &thresholds.theta_deactivate) {
            (true, LocalCondensationStatus::CondensedLocal)
        } else {
            (false, LocalCondensationStatus::Dormant)
        }
    } else {
        if fixed_ge(&z_max, &thresholds.theta_activate) {
            (true, LocalCondensationStatus::CondensedLocal)
        } else {
            (false, LocalCondensationStatus::Dormant)
        }
    };

    // Check dominance margin requirement.
    let passes = passes && {
        let min_margin = &thresholds.min_dominance_margin;
        if *min_margin == Fixed::zero() {
            true
        } else {
            fixed_ge(&dominance_margin, min_margin)
        }
    };

    let bit_id = lpcm_content_address(&(fragment_id, &z_max, passes))
        .unwrap_or(Hash256::zero());

    LocalCondensationBit {
        bit_id,
        fragment_id: fragment_id.clone(),
        winning_candidate_id: if passes { winning_id } else { None },
        z_max,
        runner_up,
        dominance_margin,
        active: passes,
        status,
        failure: if passes { None } else { Some(super::LpcmFailureKind::NoMajority) },
    }
}

/// Build a LocalCondensationCandidate from an active condensation bit.
pub fn build_local_candidate(
    _rd: &super::run_descriptor::LpcmRunDescriptor,
    bit: &LocalCondensationBit,
    candidates: &[super::candidate::CandidateDirection],
) -> super::primitives::LpcmResult<LocalCondensationCandidate> {
    use super::primitives::lpcm_content_address;

    let winning_dir = bit
        .winning_candidate_id
        .as_ref()
        .and_then(|wid| candidates.iter().find(|c| &c.candidate_id == wid));

    let candidate_direction_id = winning_dir
        .map(|d| d.candidate_id.clone())
        .unwrap_or(Hash256::zero());

    let patch_id = lpcm_content_address(&(&bit.fragment_id, "patch"))
        .unwrap_or(Hash256::zero());
    let support_vector_id = lpcm_content_address(&(&bit.fragment_id, "sv"))
        .unwrap_or(Hash256::zero());
    let local_candidate_id =
        lpcm_content_address(&(&bit.bit_id, &candidate_direction_id))?;

    let evidence_refs = winning_dir
        .map(|d| d.evidence_refs.clone())
        .unwrap_or_default();

    Ok(LocalCondensationCandidate {
        local_candidate_id,
        fragment_id: bit.fragment_id.clone(),
        patch_id,
        candidate_direction_id,
        support_vector_id,
        bit_id: bit.bit_id.clone(),
        evidence_refs,
    })
}
