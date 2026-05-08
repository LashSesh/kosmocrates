//! Layer C3 — Spiral Memory and Resonance Addressing (§8).
//!
//! `SpiralMemoryAddress` records the (θ, R, U, V) projection of a 5D
//! state plus a resonance/interference score. A `SpiralMemoryHitSet`
//! is the result of querying memory for states close to a query — hits
//! are sorted by resonance ascending then hash, so the canonical bytes
//! are stable under any caller-side reordering.

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::cognitive_state_5d::CognitiveState5D;
use super::primitives::{fixed_abs_diff, fixed_add, fixed_le, CognitionError, EvidenceRef, Fixed};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpiralMemoryAddress {
    pub address_id: Hash256,
    pub spiral_id: Hash256,
    pub segment_id: Hash256,
    pub theta: Fixed,
    pub radius: Fixed,
    pub internal_u: Fixed,
    pub internal_v: Fixed,
    pub resonance_score: Fixed,
    pub interference_score: Fixed,
    pub utilization: Fixed,
}

impl SpiralMemoryAddress {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.address_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.address_id = Hash256(id);
        Ok(self)
    }
}

impl PartialOrd for SpiralMemoryAddress {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpiralMemoryAddress {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Lower resonance score = stronger hit. Ties broken by
        // address_id so the order is total and stable.
        self.resonance_score
            .cmp(&other.resonance_score)
            .then(self.address_id.cmp(&other.address_id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpiralSegment {
    pub segment_id: Hash256,
    pub from_state: Hash256,
    pub to_state: Hash256,
    pub segment_weight: Fixed,
    pub resonance_distance: Fixed,
    pub branchable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpiralMemoryHitSet {
    pub hitset_id: Hash256,
    pub query_state_id: Hash256,
    pub hits: Vec<SpiralMemoryAddress>,
    pub best_hit: Option<Hash256>,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl SpiralMemoryHitSet {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        // Hits are always sorted by (resonance, address_id) before
        // hashing so the canonical bytes are stable.
        self.hits.sort();
        self.best_hit = self.hits.first().map(|h| h.address_id.clone());
        let mut probe = self.clone();
        probe.hitset_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.hitset_id = Hash256(id);
        Ok(self)
    }
}

/// Resonance distance per §8.3:
///
/// `d_R(s, s') = w_ψ |ψ(s)−ψ(s')| + w_ρ |ρ(s)−ρ(s')| + w_ω |ω(s)−ω(s')|`.
pub fn resonance_distance(
    s: &CognitiveState5D,
    sp: &CognitiveState5D,
    w_psi: &Fixed,
    w_rho: &Fixed,
    w_omega: &Fixed,
) -> Fixed {
    let dpsi = fixed_abs_diff(&s.psi_semantic_phase, &sp.psi_semantic_phase);
    let drho = fixed_abs_diff(&s.rho_coherence_density, &sp.rho_coherence_density);
    let domega = fixed_abs_diff(&s.omega_phase_readiness, &sp.omega_phase_readiness);
    let term_psi = mul_fixed(w_psi, &dpsi);
    let term_rho = mul_fixed(w_rho, &drho);
    let term_omega = mul_fixed(w_omega, &domega);
    fixed_add(&fixed_add(&term_psi, &term_rho), &term_omega)
}

fn mul_fixed(a: &Fixed, b: &Fixed) -> Fixed {
    let (an, ad) = canonical_pair(a);
    let (bn, bd) = canonical_pair(b);
    super::primitives::normalize_rational(an.saturating_mul(bn), ad.saturating_mul(bd))
}

fn canonical_pair(x: &Fixed) -> (i128, i128) {
    match x {
        Fixed::FixedI64 { raw, scale } => {
            let den = 10i128.pow(*scale);
            (*raw as i128, den)
        }
        Fixed::Rational { num, den } => (*num, *den),
    }
}

/// Build a hit-set from a query state and a candidate set, retaining
/// only those hits whose resonance distance falls under `epsilon_r`.
pub fn spiral_memory_query(
    query: &CognitiveState5D,
    candidates: &[CognitiveState5D],
    epsilon_r: &Fixed,
) -> Result<SpiralMemoryHitSet, CognitionError> {
    let w = Fixed::quantize(1.0, 9).unwrap();
    let mut hits = Vec::new();
    let query_state_id = query.content_hash()?;
    for (i, c) in candidates.iter().enumerate() {
        let dr = resonance_distance(query, c, &w, &w, &w);
        if !fixed_le(&dr, epsilon_r) {
            continue;
        }
        let candidate_hash = c.content_hash()?;
        let address = SpiralMemoryAddress {
            address_id: Hash256::zero(),
            spiral_id: candidate_hash.clone(),
            segment_id: candidate_hash.clone(),
            theta: c.psi_semantic_phase.clone(),
            radius: c.rho_coherence_density.clone(),
            internal_u: c.omega_phase_readiness.clone(),
            internal_v: c.chi_topological_curvature.clone(),
            resonance_score: dr,
            interference_score: Fixed::Rational {
                num: i as i128,
                den: 100,
            },
            utilization: Fixed::quantize(1.0, 9).unwrap(),
        }
        .with_id()?;
        hits.push(address);
    }
    let mut hitset = SpiralMemoryHitSet {
        hitset_id: Hash256::zero(),
        query_state_id,
        hits,
        best_hit: None,
        evidence_refs: Vec::new(),
    };
    hitset = hitset.with_id()?;
    Ok(hitset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(p: f64, r: f64, o: f64) -> CognitiveState5D {
        CognitiveState5D::from_components(
            Fixed::quantize(p, 9).unwrap(),
            Fixed::quantize(r, 9).unwrap(),
            Fixed::quantize(o, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn resonance_distance_is_deterministic() {
        let a = s(0.1, 0.2, 0.3);
        let b = s(0.4, 0.5, 0.6);
        let w = Fixed::quantize(1.0, 9).unwrap();
        let d1 = resonance_distance(&a, &b, &w, &w, &w);
        let d2 = resonance_distance(&a, &b, &w, &w, &w);
        assert_eq!(d1, d2);
    }

    #[test]
    fn hitset_sorts_by_resonance_then_hash() {
        let q = s(0.0, 0.0, 0.0);
        let c1 = s(0.5, 0.5, 0.5); // farther
        let c2 = s(0.1, 0.1, 0.1); // closer
        let hs = spiral_memory_query(&q, &[c1, c2], &Fixed::quantize(10.0, 9).unwrap()).unwrap();
        // Closer one (c2) should come first.
        assert!(hs.hits.len() >= 2);
        assert!(hs.hits[0].resonance_score < hs.hits[1].resonance_score);
    }
}
