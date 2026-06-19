//! Quantenstarre / QSR — the structural-rigidity predicate (spec ch. 7), the
//! condensation guard at the heart of the calculus. QSR is **fail-closed**: a
//! violation without a gate-justified exception is a `Reject`.

use crate::types::{Stage, Status};

/// Stage-level QSR (Def. 7.1) with the QSR-monotonicity invariant (Inv. 7.3).
///
/// - **Gate before score (I2):** a non-passing gate decides immediately — no
///   metric can override it (negative test 14).
/// - **Monotonicity (Inv. 7.3):** across accepted stages density and purity are
///   non-decreasing, irreducibility is non-decreasing (rising), and contradiction
///   energy is non-increasing. A regression fails QSR (negative test 4).
///
/// Returns the stage's QSR status; `Pass`/`Warn` are accepting, `Reject`/`Defer`
/// are not.
pub fn qsr_stage(s: &Stage, prev: Option<&Stage>) -> Status {
    match s.gate_status {
        Status::Reject => return Status::Reject,
        Status::Pending | Status::Defer => return Status::Defer,
        Status::Pass | Status::Warn => {}
    }
    if let Some(p) = prev {
        let (m, pm) = (&s.metrics, &p.metrics);
        let regressed = m.density < pm.density
            || m.purity < pm.purity
            || m.irreducibility < pm.irreducibility
            || m.contradiction_energy > pm.contradiction_energy;
        if regressed {
            return Status::Reject;
        }
    }
    s.gate_status // Pass or Warn carried forward
}

/// Stack-level QSR (Def. 7.2): every accepted stage passes stage-QSR against its
/// predecessor — the monotone chain — and the roundtrip returns to the attractor.
/// For the C0 report-only foundation the monotone chain is the checkable core;
/// the fixpoint/roundtrip (`Canon → A⋆`) is realized once the fold/closure crate
/// lands. Fail-closed: the first non-accepting stage propagates.
pub fn qsr_stack(stages: &[Stage]) -> Status {
    let mut prev: Option<&Stage> = None;
    for s in stages {
        match qsr_stage(s, prev) {
            Status::Pass | Status::Warn => {}
            other => return other,
        }
        prev = Some(s);
    }
    Status::Pass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::StageMetrics;
    use kosmo_core::{Digest, Q16};

    fn stage(gate: Status, density: Q16, purity: Q16, irr: Q16, contr: Q16) -> Stage {
        Stage {
            attractor: Digest::ZERO,
            units: vec![],
            evidence: Digest::ZERO,
            gate_status: gate,
            trace_root: Digest::ZERO,
            residue: Digest::ZERO,
            metrics: StageMetrics {
                density,
                purity,
                irreducibility: irr,
                curvature: Q16::ZERO,
                contradiction_energy: contr,
            },
            certificates: vec![],
        }
    }

    #[test]
    fn a_failed_gate_is_a_hard_reject_no_score_overrides_it() {
        // Negative test 14 — a score never overrides a gate. Even with perfect
        // metrics, a rejected gate fails QSR.
        let s = stage(Status::Reject, Q16::ONE, Q16::ONE, Q16::ONE, Q16::ZERO);
        assert_eq!(qsr_stage(&s, None), Status::Reject);
    }

    #[test]
    fn density_falling_against_the_predecessor_fails_qsr() {
        // Negative test 4 — support grows but density falls.
        let prev = stage(Status::Pass, Q16::HALF, Q16::HALF, Q16::HALF, Q16::ZERO);
        let next = stage(Status::Pass, Q16::ratio(1, 4).unwrap(), Q16::HALF, Q16::HALF, Q16::ZERO);
        assert_eq!(qsr_stage(&next, Some(&prev)), Status::Reject);
    }

    #[test]
    fn rising_contradiction_energy_fails_qsr() {
        let prev = stage(Status::Pass, Q16::HALF, Q16::HALF, Q16::HALF, Q16::ZERO);
        let next = stage(Status::Pass, Q16::HALF, Q16::HALF, Q16::HALF, Q16::HALF);
        assert_eq!(qsr_stage(&next, Some(&prev)), Status::Reject);
    }

    #[test]
    fn a_monotone_condensing_stage_passes() {
        let prev = stage(Status::Pass, Q16::HALF, Q16::HALF, Q16::HALF, Q16::HALF);
        let next = stage(
            Status::Pass,
            Q16::ratio(3, 4).unwrap(), // density up
            Q16::HALF,                 // purity equal
            Q16::ratio(3, 4).unwrap(), // irreducibility up
            Q16::ratio(1, 4).unwrap(), // contradiction down
        );
        assert_eq!(qsr_stage(&next, Some(&prev)), Status::Pass);
    }

    #[test]
    fn stack_qsr_holds_on_a_monotone_chain_and_fails_on_a_regression() {
        let s0 = stage(Status::Pass, Q16::ratio(1, 4).unwrap(), Q16::HALF, Q16::HALF, Q16::HALF);
        let s1 = stage(Status::Pass, Q16::HALF, Q16::HALF, Q16::HALF, Q16::ratio(1, 4).unwrap());
        let s2 = stage(Status::Pass, Q16::ratio(3, 4).unwrap(), Q16::HALF, Q16::HALF, Q16::ZERO);
        assert_eq!(qsr_stack(&[s0.clone(), s1.clone(), s2]), Status::Pass);
        // Insert a regressing stage → the chain fails QSR.
        let bad = stage(Status::Pass, Q16::ZERO, Q16::HALF, Q16::HALF, Q16::ONE);
        assert_eq!(qsr_stack(&[s0, s1, bad]), Status::Reject);
    }
}
