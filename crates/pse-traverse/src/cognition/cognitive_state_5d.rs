//! Layer C1 — 5D Operational Geometry.
//!
//! Per §6 the cognitive state is a normalised 5-tuple `σ = (ψ, ρ, ω, χ, τ)`
//! plus four derived observables (potential, energy, entropy,
//! stability_index). The `SingularityDetector` collapses the spectrum
//! into a fail-closed gate: a degenerate stability operator opens the
//! "reorganisation window".

use serde::{Deserialize, Serialize};

use crate::canonical::content_address;
use crate::dynamic_state::Hash256;

use super::primitives::{
    fixed_add, fixed_ge, fixed_le, fixed_sub, max_fixed, CognitionError, EvidenceRef, Fixed,
};

/// `σ = (ψ, ρ, ω, χ, τ)` plus derived observables.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveState5D {
    pub psi_semantic_phase: Fixed,
    pub rho_coherence_density: Fixed,
    pub omega_phase_readiness: Fixed,
    pub chi_topological_curvature: Fixed,
    pub tau_entropy_symmetry: Fixed,
    pub potential: Fixed,
    pub energy: Fixed,
    pub entropy: Fixed,
    pub stability_index: Fixed,
}

impl CognitiveState5D {
    /// Zero state — useful as a deterministic baseline. `stability_index`
    /// is set to 1 so a zero state is not flagged as unstable.
    pub fn zero() -> Self {
        let zero = Fixed::zero();
        CognitiveState5D {
            psi_semantic_phase: zero.clone(),
            rho_coherence_density: zero.clone(),
            omega_phase_readiness: zero.clone(),
            chi_topological_curvature: zero.clone(),
            tau_entropy_symmetry: zero.clone(),
            potential: zero.clone(),
            energy: zero.clone(),
            entropy: zero,
            stability_index: Fixed::quantize(1.0, 9).unwrap(),
        }
    }

    /// Build a state from the five normative components and derive
    /// `potential`, `energy`, `entropy`, `stability_index`. All
    /// derivations stay in the canonical rational domain so the result
    /// is float-free.
    pub fn from_components(
        psi: Fixed,
        rho: Fixed,
        omega: Fixed,
        chi: Fixed,
        tau: Fixed,
    ) -> Result<Self, CognitionError> {
        // Energy = max(|ψ|, |ρ|, |ω|, |χ|, |τ|) — float-free L^∞.
        let energy = max_fixed(&[
            psi.clone().abs(),
            rho.clone().abs(),
            omega.clone().abs(),
            chi.clone().abs(),
            tau.clone().abs(),
        ]);
        // Entropy = |τ| (the tau component already encodes
        // entropy/symmetry per §6.1).
        let entropy = tau.clone().abs();
        // Coherent reservoir = |ρ|.
        let coherent = rho.clone().abs();
        // Potential V(σ) = entropy - coherent (per §6.3 V = E_entropic
        // - E_coherent; we use a deterministic proxy that stays in the
        // fixed-point domain).
        let potential = fixed_sub(&entropy, &coherent);
        // Stability index = 1 / (1 + |∇V proxy|) using a
        // float-free rational proxy: stability = ρ / (ρ + |τ|).
        let denom = fixed_add(&coherent, &entropy);
        let stability_index = if denom == Fixed::zero() {
            Fixed::quantize(1.0, 9).unwrap()
        } else {
            // rational division: coherent / denom
            let (cn, cd) = canonical_pair(&coherent);
            let (dn, dd) = canonical_pair(&denom);
            // (cn/cd) / (dn/dd) = (cn * dd) / (cd * dn)
            let num = cn.saturating_mul(dd);
            let den = cd.saturating_mul(dn);
            super::primitives::normalize_rational(num, den)
        };
        Ok(CognitiveState5D {
            psi_semantic_phase: psi,
            rho_coherence_density: rho,
            omega_phase_readiness: omega,
            chi_topological_curvature: chi,
            tau_entropy_symmetry: tau,
            potential,
            energy,
            entropy,
            stability_index,
        })
    }

    /// `state_hash` for cross-layer references.
    pub fn content_hash(&self) -> Result<Hash256, CognitionError> {
        Ok(Hash256(content_address(self).map_err(|e| {
            CognitionError::Canonicalization {
                reason: e.to_string(),
            }
        })?))
    }
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

/// `SingularityDetectorReport` per §6.4. A "kognitive Singularität" is a
/// degenerate decision/reorganisation window — `singular_window_open`
/// flags it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingularityDetectorReport {
    pub report_id: Hash256,
    pub state_id: Hash256,
    pub determinant_proxy: Fixed,
    pub min_eigen_proxy: Fixed,
    pub trigger_score: Fixed,
    pub singular_window_open: bool,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl SingularityDetectorReport {
    pub fn with_id(mut self) -> Result<Self, CognitionError> {
        let mut probe = self.clone();
        probe.report_id = Hash256::zero();
        let id = content_address(&probe).map_err(|e| CognitionError::Canonicalization {
            reason: e.to_string(),
        })?;
        self.report_id = Hash256(id);
        Ok(self)
    }
}

/// `G_sing = 1 ⇔ |det(T)| ≤ ε_T ∨ λ_min(T) ≤ ε_λ ∨ S_trigger ≥ θ_S`.
///
/// We use a float-free analytic proxy: `det_proxy = stability_index`,
/// `min_eigen_proxy = coherence_density`, `trigger_score = entropy`.
/// The window opens iff any of the three sub-conditions fires.
pub fn detect_singularity(
    state: &CognitiveState5D,
    epsilon_det: &Fixed,
    epsilon_eigen: &Fixed,
    theta_trigger: &Fixed,
) -> Result<SingularityDetectorReport, CognitionError> {
    let determinant_proxy = state.stability_index.clone();
    let min_eigen_proxy = state.rho_coherence_density.clone().abs();
    let trigger_score = state.entropy.clone();
    let det_low = fixed_le(&determinant_proxy, epsilon_det);
    let eigen_low = fixed_le(&min_eigen_proxy, epsilon_eigen);
    let trigger_high = fixed_ge(&trigger_score, theta_trigger);
    let singular_window_open = det_low || eigen_low || trigger_high;
    let state_id = state.content_hash()?;
    let report = SingularityDetectorReport {
        report_id: Hash256::zero(),
        state_id,
        determinant_proxy,
        min_eigen_proxy,
        trigger_score,
        singular_window_open,
        evidence_refs: Vec::new(),
    };
    report.with_id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_components_normalises_derived_values() {
        let s = CognitiveState5D::from_components(
            Fixed::quantize(0.5, 9).unwrap(),
            Fixed::quantize(0.5, 9).unwrap(),
            Fixed::quantize(0.5, 9).unwrap(),
            Fixed::quantize(0.5, 9).unwrap(),
            Fixed::quantize(0.5, 9).unwrap(),
        )
        .unwrap();
        // entropy = |τ| = 0.5; coherent = |ρ| = 0.5; potential = 0.
        assert_eq!(s.potential, Fixed::quantize(0.0, 9).unwrap());
    }

    #[test]
    fn potential_gradient_proxy_is_deterministic() {
        let a = CognitiveState5D::from_components(
            Fixed::quantize(0.1, 9).unwrap(),
            Fixed::quantize(0.2, 9).unwrap(),
            Fixed::quantize(0.3, 9).unwrap(),
            Fixed::quantize(0.4, 9).unwrap(),
            Fixed::quantize(0.5, 9).unwrap(),
        )
        .unwrap();
        let b = CognitiveState5D::from_components(
            Fixed::quantize(0.1, 9).unwrap(),
            Fixed::quantize(0.2, 9).unwrap(),
            Fixed::quantize(0.3, 9).unwrap(),
            Fixed::quantize(0.4, 9).unwrap(),
            Fixed::quantize(0.5, 9).unwrap(),
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn singularity_detector_opens_on_degenerate_spectrum() {
        // Force ρ to zero so min_eigen_proxy is zero and ε_eigen is
        // also zero ⇒ eigen_low fires.
        let s = CognitiveState5D::from_components(
            Fixed::quantize(0.1, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
            Fixed::quantize(0.0, 9).unwrap(),
        )
        .unwrap();
        let report = detect_singularity(
            &s,
            &Fixed::quantize(0.0, 9).unwrap(),
            &Fixed::quantize(0.0, 9).unwrap(),
            &Fixed::quantize(2.0, 9).unwrap(),
        )
        .unwrap();
        assert!(report.singular_window_open);
    }
}
