//! The D-energy kernel and condensation metrics (spec ch. 18). All ranking
//! reuses the Kosmocrates tripolar epistemic energy `D = ψ·ρ·ω`. Scores **rank**;
//! they never decide (invariant I2 — see [`crate::qsr`]).

use crate::types::StageMetrics;
use kosmo_core::Q16;

const SCALE: i128 = 1 << 16; // Q16 fixed-point scale (65536)

/// Fixed-point product of two `Q16` values in `[0,1]`.
fn q16_mul(a: Q16, b: Q16) -> Q16 {
    Q16::from_raw(((a.raw() as i128 * b.raw() as i128) / SCALE) as i64)
}

/// Tripolar epistemic energy `D = ψ·ρ·ω` (Eq. 18.1): meaning × coherence × phase.
/// The good condensate is never the largest — it is the one of maximal
/// target-compatible D-density (invariant I7).
pub fn d_energy(psi: Q16, rho: Q16, omega: Q16) -> Q16 {
    q16_mul(q16_mul(psi, rho), omega)
}

/// The condensation objective (Eq. 18.2): `D − λK − μC` — D-density penalized by
/// contradiction energy `K` and avoidable complexity `C`. Returned as a raw `Q16`
/// difference: an **advisory rank**, never a gate (I2). Larger is better.
pub fn objective(d: Q16, k: Q16, c: Q16, lambda: Q16, mu: Q16) -> i64 {
    d.raw() - q16_mul(lambda, k).raw() - q16_mul(mu, c).raw()
}

/// Stage progress (§18.4): `Δdensity + Δpurity + Δirreducibility − Δcontradiction`
/// (unit weights for the C0 foundation). A stage is accepted iff progress `≥ 0`
/// or a gate-justified exception holds (invariant I7 — condensation over growth).
pub fn stage_progress(prev: &StageMetrics, next: &StageMetrics) -> i64 {
    (next.density.raw() - prev.density.raw())
        + (next.purity.raw() - prev.purity.raw())
        + (next.irreducibility.raw() - prev.irreducibility.raw())
        - (next.contradiction_energy.raw() - prev.contradiction_energy.raw())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d_energy_is_the_tripolar_product() {
        assert_eq!(d_energy(Q16::ONE, Q16::ONE, Q16::ONE), Q16::ONE);
        // ½·½·1 = ¼.
        let quarter = d_energy(Q16::HALF, Q16::HALF, Q16::ONE);
        assert_eq!(quarter, Q16::ratio(1, 4).unwrap());
        // Any zero pole collapses the energy (meaning/coherence/phase are AND-like).
        assert_eq!(d_energy(Q16::ZERO, Q16::ONE, Q16::ONE), Q16::ZERO);
    }

    #[test]
    fn objective_penalizes_contradiction_and_complexity() {
        // FILL-001: mass-for-mass is penalized — a high-K candidate ranks below a
        // low-K one of equal D.
        let clean = objective(Q16::HALF, Q16::ZERO, Q16::ZERO, Q16::ONE, Q16::ONE);
        let noisy = objective(Q16::HALF, Q16::HALF, Q16::ZERO, Q16::ONE, Q16::ONE);
        assert!(clean > noisy, "contradiction energy lowers the rank");
    }

    #[test]
    fn progress_is_positive_when_density_rises_purely() {
        let prev = StageMetrics::default();
        let next = StageMetrics {
            density: Q16::HALF,
            ..StageMetrics::default()
        };
        assert!(stage_progress(&prev, &next) > 0);
    }
}
