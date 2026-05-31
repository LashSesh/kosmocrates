//! Unified tripolar energy kernel — `D = ψ · ρ · ω`.
//!
//! This is the single selection core for the `kosmo-*` substrate. Before this
//! module, ranking was fragmented: `SourceCube.support_score`, the LPCM support
//! mass majority test, the SystemCube D-density ratio, and `NormGene` fitness
//! were each a separate heuristic with its own scale. The [`EnergyKernel`]
//! unifies them into one deterministic, float-free, content-addressed tripolar
//! energy, modulated by structural-safety factors.
//!
//! ## The tripolar core
//!
//! Kosmocrates uses the Pfauenthron tripolar energy `D = ψ · ρ · ω`, where
//!
//! * `ψ` (psi)   — **meaning**: semantic goal-fit of a structure,
//! * `ρ` (rho)   — **coherence**: structural stability of a structure,
//! * `ω` (omega) — **phase**: temporal / topological coherence (HDAG fit).
//!
//! Each component is a [`Q16`] value clamped to `[0, 1]`. The product is computed
//! in integer Q16 arithmetic — no floats ever appear in this path (CROSS-007).
//! This is the integer-exact sibling of the `f64` `score_tripolar` that the PSE
//! retrieval layer (`pse-adapter-il`) uses; the kosmo substrate keeps it in Q16
//! because the value participates in gate-adjacent selection.
//!
//! ## Modulating factors
//!
//! The bare tripolar product is multiplied by a set of `[0, 1]` safety factors:
//! `gate`, `taint`, `license`, `foundry`, `seam`, `contradiction`. Every factor
//! can only **reduce** energy, never raise it; a factor of zero collapses the
//! energy to zero. The factors are derived fail-closed from the substrate's own
//! authority types ([`GateResult`], [`TaintLabel`], [`LicenseStatus`]).
//!
//! ## The non-bypass invariant (CROSS-010)
//!
//! **Energy ranks; it never gates.** A high `D` does not authorise anything a
//! gate forbids. This module exposes *no* method that turns an energy value into
//! a decision, a policy escalation, or an `Accept`. The only way energy interacts
//! with a gate is through the `gate` factor, which is a floor enforcer: a rejected
//! [`GateResult`] forces that factor to [`Q16::ZERO`], so a rejected candidate's
//! energy is zero and it can never out-rank a passing one. Energy alone can never
//! flip a `Reject` into an `Accept` — the decision authority stays entirely with
//! the gate cascade. Selection by energy is therefore always a choice *among*
//! candidates that have already independently passed their gates.

use crate::authority::{LicenseStatus, TaintLabel};
use crate::digest::Digest;
use crate::fixed_point::Q16;
use crate::run::GateResult;
use serde::{Deserialize, Serialize};

/// Clamp a `Q16` into the unit interval `[0, 1]`. Negative values floor to zero;
/// values above one saturate to one. Energy components and factors are always
/// unit-scaled so the tripolar product stays in `[0, 1]`.
fn clamp_unit(q: Q16) -> Q16 {
    if q.raw() < 0 {
        Q16::ZERO
    } else if q.raw() > Q16::ONE.raw() {
        Q16::ONE
    } else {
        q
    }
}

/// The three poles of the tripolar energy, each clamped to `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TripolarEnergy {
    /// ψ — meaning / semantic goal-fit.
    pub psi: Q16,
    /// ρ — coherence / structural stability.
    pub rho: Q16,
    /// ω — phase / temporal-topological coherence.
    pub omega: Q16,
}

impl TripolarEnergy {
    /// Construct from three components, clamping each into `[0, 1]`.
    pub fn new(psi: Q16, rho: Q16, omega: Q16) -> Self {
        Self {
            psi: clamp_unit(psi),
            rho: clamp_unit(rho),
            omega: clamp_unit(omega),
        }
    }

    /// A fully-saturated tripolar energy (`ψ = ρ = ω = 1`), `D = 1`.
    pub fn unit() -> Self {
        Self {
            psi: Q16::ONE,
            rho: Q16::ONE,
            omega: Q16::ONE,
        }
    }

    /// The tripolar product `D = ψ · ρ · ω`, in integer Q16 arithmetic.
    ///
    /// All three poles are in `[0, 1]`, so the product is in `[0, 1]` and cannot
    /// overflow; the `unwrap_or(ZERO)` is a fail-closed guard only.
    pub fn d(&self) -> Q16 {
        let pr = self.psi.checked_mul(self.rho).unwrap_or(Q16::ZERO);
        pr.checked_mul(self.omega).unwrap_or(Q16::ZERO)
    }
}

/// Outcome of a Foundry validation, as it feeds the energy `foundry` factor.
///
/// Kept as a local enum so this module has no dependency on the executor crate
/// (`kosmo-foundry`); callers map their real `FoundryExecutionReport` outcome
/// onto this three-state summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundrySurvival {
    /// All required Foundry checks passed.
    Passed,
    /// Foundry was not run (no environment / not required yet).
    Unavailable,
    /// At least one required Foundry check failed.
    Failed,
}

/// The six `[0, 1]` modulating factors applied to the bare tripolar `D`.
///
/// Each factor can only reduce energy. The factors are multiplied together
/// (`product`) and then multiplied into `D`. A single zero factor collapses the
/// whole energy to zero — fail-closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergyFactors {
    /// Gate outcome floor: `Reject` → 0 (the non-bypass enforcer).
    pub gate: Q16,
    /// Taint penalty: `Clean` → 1, `Quarantined` → 0.
    pub taint: Q16,
    /// License penalty: permissive → 1, proprietary → 0.
    pub license: Q16,
    /// Foundry survival: `Passed` → 1, `Failed` → 0.
    pub foundry: Q16,
    /// Seam coherence (caller-supplied, clamped).
    pub seam: Q16,
    /// Contradiction tolerance: `1 − contradiction_penalty` (clamped).
    pub contradiction: Q16,
}

impl EnergyFactors {
    /// Map a [`GateResult`] to its energy floor factor.
    ///
    /// `Pass` → 1, `Warn` → 1/2, `Downgrade` → 1/4 (penalised but not eliminated),
    /// `Reject` → 0. The mapping follows the gate severity order
    /// `Reject > Downgrade > Warn > Pass`; the `Reject → 0` mapping is the
    /// structural non-bypass guarantee.
    pub fn gate_factor(gate: &GateResult) -> Q16 {
        match gate {
            GateResult::Pass => Q16::ONE,
            GateResult::Warn { .. } => Q16::HALF,
            GateResult::Downgrade { .. } => Q16::ratio(1, 4).unwrap_or(Q16::ZERO),
            GateResult::Reject { .. } => Q16::ZERO,
        }
    }

    /// Map a [`TaintLabel`] to its energy factor (more taint → lower factor).
    pub fn taint_factor(taint: &TaintLabel) -> Q16 {
        match taint {
            TaintLabel::Clean => Q16::ONE,
            TaintLabel::External => Q16::ratio(3, 4).unwrap_or(Q16::ZERO),
            TaintLabel::Synthetic | TaintLabel::Unverified => Q16::HALF,
            TaintLabel::PolicyRestricted => Q16::ratio(1, 4).unwrap_or(Q16::ZERO),
            TaintLabel::Quarantined { .. } => Q16::ZERO,
        }
    }

    /// Map a [`LicenseStatus`] to its energy factor.
    pub fn license_factor(license: &LicenseStatus) -> Q16 {
        match license {
            LicenseStatus::Permissive { .. } | LicenseStatus::NotApplicable => Q16::ONE,
            LicenseStatus::Copyleft { .. } => Q16::ratio(3, 4).unwrap_or(Q16::ZERO),
            LicenseStatus::Unknown => Q16::HALF,
            LicenseStatus::Unresolved => Q16::ratio(1, 4).unwrap_or(Q16::ZERO),
            LicenseStatus::Proprietary => Q16::ZERO,
        }
    }

    /// Map a [`FoundrySurvival`] to its energy factor.
    ///
    /// `Unavailable` is penalised (1/2) but not eliminated — a structure that has
    /// not yet been Foundry-validated is uncertain, not condemned. `Failed` → 0.
    pub fn foundry_factor(survival: FoundrySurvival) -> Q16 {
        match survival {
            FoundrySurvival::Passed => Q16::ONE,
            FoundrySurvival::Unavailable => Q16::HALF,
            FoundrySurvival::Failed => Q16::ZERO,
        }
    }

    /// Derive a contradiction factor from a contradiction penalty in `[0, 1]`:
    /// `factor = 1 − penalty` (clamped). Higher contradiction → lower factor.
    pub fn contradiction_factor(penalty: Q16) -> Q16 {
        clamp_unit(Q16::ONE.saturating_sub(clamp_unit(penalty)))
    }

    /// All factors clean (`= 1`): the bare tripolar product is unmodulated.
    pub fn all_clean() -> Self {
        Self {
            gate: Q16::ONE,
            taint: Q16::ONE,
            license: Q16::ONE,
            foundry: Q16::ONE,
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        }
    }

    /// Build the factor set from the substrate's real authority/validation types.
    pub fn derive(
        gate: &GateResult,
        taint: &TaintLabel,
        license: &LicenseStatus,
        foundry: FoundrySurvival,
        seam_coherence: Q16,
        contradiction_penalty: Q16,
    ) -> Self {
        Self {
            gate: Self::gate_factor(gate),
            taint: Self::taint_factor(taint),
            license: Self::license_factor(license),
            foundry: Self::foundry_factor(foundry),
            seam: clamp_unit(seam_coherence),
            contradiction: Self::contradiction_factor(contradiction_penalty),
        }
    }

    /// The product of all six factors, in `[0, 1]`.
    pub fn product(&self) -> Q16 {
        [
            self.gate,
            self.taint,
            self.license,
            self.foundry,
            self.seam,
            self.contradiction,
        ]
        .iter()
        .fold(Q16::ONE, |acc, f| acc.checked_mul(*f).unwrap_or(Q16::ZERO))
    }
}

/// The energy kernel: a tripolar core plus its modulating factors.
///
/// `energy()` is the final selection scalar: `D · ∏ factors`. It is in `[0, 1]`
/// and is the *only* number any layer should rank candidates by. The kernel has
/// no decision method — see the module-level non-bypass invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergyKernel {
    pub tripolar: TripolarEnergy,
    pub factors: EnergyFactors,
}

impl EnergyKernel {
    pub fn new(tripolar: TripolarEnergy, factors: EnergyFactors) -> Self {
        Self { tripolar, factors }
    }

    /// The bare tripolar `D = ψ · ρ · ω`, before factor modulation.
    pub fn bare_d(&self) -> Q16 {
        self.tripolar.d()
    }

    /// The final modulated selection energy `D · ∏ factors`, in `[0, 1]`.
    pub fn energy(&self) -> Q16 {
        self.bare_d()
            .checked_mul(self.factors.product())
            .unwrap_or(Q16::ZERO)
    }

    /// Whether this kernel's gate factor was zeroed by a `Reject` (or any other
    /// hard-zero factor). A zeroed kernel can never be selected.
    pub fn is_zeroed(&self) -> bool {
        self.energy().is_zero()
    }
}

/// Serialize-only content for the deterministic [`EnergyAssessment`] id.
#[derive(Serialize)]
struct AssessmentContent<'a> {
    subject_id: &'a Digest,
    psi_raw: i64,
    rho_raw: i64,
    omega_raw: i64,
    gate_raw: i64,
    taint_raw: i64,
    license_raw: i64,
    foundry_raw: i64,
    seam_raw: i64,
    contradiction_raw: i64,
    energy_raw: i64,
    policy_id: &'a Digest,
    evidence_bundle_id: &'a Digest,
}

/// A content-addressed, evidence-bound energy assessment of one subject.
///
/// This is the durable form of an [`EnergyKernel`] evaluation. `subject_id` is
/// the content digest of whatever was scored (a `SourceCube`, a `BlueprintUnit`,
/// a candidate motif, …). `energy` is cached so a stored assessment can be ranked
/// without recomputation, and `verify_id()` re-derives the digest to detect
/// tampering. Evidence binding (`evidence_bundle_id`) is mandatory (CROSS-006).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergyAssessment {
    pub id: Digest,
    pub subject_id: Digest,
    pub kernel: EnergyKernel,
    /// Cached final energy `D · ∏ factors` (also recomputable from `kernel`).
    pub energy: Q16,
    pub policy_id: Digest,
    pub evidence_bundle_id: Digest,
}

impl EnergyAssessment {
    pub fn new(
        subject_id: Digest,
        kernel: EnergyKernel,
        policy_id: Digest,
        evidence_bundle_id: Digest,
    ) -> Self {
        let energy = kernel.energy();
        let mut a = Self {
            id: Digest::ZERO,
            subject_id,
            kernel,
            energy,
            policy_id,
            evidence_bundle_id,
        };
        a.id = a.compute_id();
        a
    }

    fn compute_id(&self) -> Digest {
        Digest::of(&AssessmentContent {
            subject_id: &self.subject_id,
            psi_raw: self.kernel.tripolar.psi.raw(),
            rho_raw: self.kernel.tripolar.rho.raw(),
            omega_raw: self.kernel.tripolar.omega.raw(),
            gate_raw: self.kernel.factors.gate.raw(),
            taint_raw: self.kernel.factors.taint.raw(),
            license_raw: self.kernel.factors.license.raw(),
            foundry_raw: self.kernel.factors.foundry.raw(),
            seam_raw: self.kernel.factors.seam.raw(),
            contradiction_raw: self.kernel.factors.contradiction.raw(),
            energy_raw: self.energy.raw(),
            policy_id: &self.policy_id,
            evidence_bundle_id: &self.evidence_bundle_id,
        })
    }

    pub fn verify_id(&self) -> bool {
        self.id == self.compute_id()
    }

    /// Whether this assessment's energy is zero (gate-zeroed or otherwise inert).
    pub fn is_zeroed(&self) -> bool {
        self.energy.is_zero()
    }
}

/// Rank assessments by energy, descending, with a deterministic tie-break.
///
/// Ties on energy are broken by `subject_id` (ascending) so the ordering is a
/// total, replay-stable order. Zero-energy assessments sort to the end but are
/// **not** removed — a caller that wants to exclude them must filter explicitly;
/// the ranking itself never silently drops a candidate.
pub fn rank_by_energy(assessments: &[EnergyAssessment]) -> Vec<EnergyAssessment> {
    let mut out: Vec<EnergyAssessment> = assessments.to_vec();
    out.sort_by(|a, b| {
        b.energy
            .raw()
            .cmp(&a.energy.raw())
            .then_with(|| a.subject_id.cmp(&b.subject_id))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(seed: &[u8]) -> Digest {
        Digest::of_bytes(seed)
    }

    fn pol() -> Digest {
        d(b"policy")
    }
    fn bundle() -> Digest {
        d(b"bundle")
    }

    // ─── TripolarEnergy ─────────────────────────────────────────────────────

    #[test]
    fn tripolar_unit_d_is_one() {
        assert_eq!(TripolarEnergy::unit().d(), Q16::ONE);
    }

    #[test]
    fn tripolar_product_is_integer_exact() {
        // 0.5 · 0.5 · 0.5 = 0.125
        let t = TripolarEnergy::new(Q16::HALF, Q16::HALF, Q16::HALF);
        assert_eq!(t.d(), Q16::ratio(1, 8).unwrap());
    }

    #[test]
    fn tripolar_clamps_out_of_range_components() {
        let t = TripolarEnergy::new(Q16::from_i64(5), -Q16::ONE, Q16::ONE);
        assert_eq!(t.psi, Q16::ONE, "psi above 1 clamps to 1");
        assert_eq!(t.rho, Q16::ZERO, "negative rho clamps to 0");
        assert_eq!(t.d(), Q16::ZERO, "any zero pole zeroes D");
    }

    #[test]
    fn tripolar_zero_pole_zeroes_energy() {
        let t = TripolarEnergy::new(Q16::ONE, Q16::ZERO, Q16::ONE);
        assert_eq!(t.d(), Q16::ZERO);
    }

    // ─── EnergyFactors ──────────────────────────────────────────────────────

    #[test]
    fn gate_factor_reject_is_zero() {
        assert_eq!(
            EnergyFactors::gate_factor(&GateResult::Reject { reason: "x".into() }),
            Q16::ZERO
        );
        assert_eq!(EnergyFactors::gate_factor(&GateResult::Pass), Q16::ONE);
        assert_eq!(
            EnergyFactors::gate_factor(&GateResult::Warn { message: "m".into() }),
            Q16::HALF
        );
    }

    #[test]
    fn taint_factor_clean_one_quarantined_zero() {
        assert_eq!(EnergyFactors::taint_factor(&TaintLabel::Clean), Q16::ONE);
        assert_eq!(
            EnergyFactors::taint_factor(&TaintLabel::Quarantined { reason: "r".into() }),
            Q16::ZERO
        );
    }

    #[test]
    fn license_factor_permissive_one_proprietary_zero() {
        assert_eq!(
            EnergyFactors::license_factor(&LicenseStatus::Permissive { spdx: "MIT".into() }),
            Q16::ONE
        );
        assert_eq!(
            EnergyFactors::license_factor(&LicenseStatus::Proprietary),
            Q16::ZERO
        );
    }

    #[test]
    fn foundry_factor_failed_is_zero() {
        assert_eq!(
            EnergyFactors::foundry_factor(FoundrySurvival::Failed),
            Q16::ZERO
        );
        assert_eq!(
            EnergyFactors::foundry_factor(FoundrySurvival::Passed),
            Q16::ONE
        );
    }

    #[test]
    fn contradiction_factor_inverts_penalty() {
        assert_eq!(EnergyFactors::contradiction_factor(Q16::ZERO), Q16::ONE);
        assert_eq!(EnergyFactors::contradiction_factor(Q16::ONE), Q16::ZERO);
        assert_eq!(EnergyFactors::contradiction_factor(Q16::HALF), Q16::HALF);
    }

    #[test]
    fn factors_product_all_clean_is_one() {
        assert_eq!(EnergyFactors::all_clean().product(), Q16::ONE);
    }

    #[test]
    fn factors_single_zero_collapses_product() {
        let mut f = EnergyFactors::all_clean();
        f.taint = Q16::ZERO;
        assert_eq!(f.product(), Q16::ZERO);
    }

    // ─── EnergyKernel ───────────────────────────────────────────────────────

    #[test]
    fn kernel_unmodulated_equals_bare_d() {
        let k = EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean());
        assert_eq!(k.energy(), k.bare_d());
        assert_eq!(k.energy(), Q16::ONE);
    }

    /// The core safety property: a rejected gate zeroes the energy regardless of
    /// how high the tripolar poles are. Energy ranks; it never bypasses a gate.
    #[test]
    fn energy_ranks_but_never_bypasses_gate() {
        let perfect_tripolar = TripolarEnergy::unit(); // D = 1, maximal
        let factors = EnergyFactors::derive(
            &GateResult::Reject { reason: "missing evidence".into() },
            &TaintLabel::Clean,
            &LicenseStatus::Permissive { spdx: "MIT".into() },
            FoundrySurvival::Passed,
            Q16::ONE,
            Q16::ZERO,
        );
        let k = EnergyKernel::new(perfect_tripolar, factors);
        assert!(k.is_zeroed(), "a Reject gate must zero the energy");
        assert_eq!(k.energy(), Q16::ZERO);
        // And a passing kernel with even a tiny D out-ranks the rejected one.
        let passing = EnergyKernel::new(
            TripolarEnergy::new(Q16::ratio(1, 100).unwrap(), Q16::ONE, Q16::ONE),
            EnergyFactors::all_clean(),
        );
        assert!(passing.energy().raw() > k.energy().raw());
    }

    #[test]
    fn quarantine_zeroes_energy() {
        let k = EnergyKernel::new(
            TripolarEnergy::unit(),
            EnergyFactors::derive(
                &GateResult::Pass,
                &TaintLabel::Quarantined { reason: "untrusted".into() },
                &LicenseStatus::Permissive { spdx: "MIT".into() },
                FoundrySurvival::Passed,
                Q16::ONE,
                Q16::ZERO,
            ),
        );
        assert!(k.is_zeroed());
    }

    // ─── EnergyAssessment ───────────────────────────────────────────────────

    #[test]
    fn assessment_is_content_addressed_and_deterministic() {
        let k = EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean());
        let a1 = EnergyAssessment::new(d(b"subject"), k, pol(), bundle());
        let a2 = EnergyAssessment::new(d(b"subject"), k, pol(), bundle());
        assert_eq!(a1.id, a2.id);
        assert_ne!(a1.id, Digest::ZERO);
        assert!(a1.verify_id());
        assert_eq!(a1.energy, Q16::ONE);
    }

    #[test]
    fn assessment_evidence_is_mandatory_nonzero() {
        let k = EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean());
        let a = EnergyAssessment::new(d(b"s"), k, pol(), bundle());
        assert_ne!(a.evidence_bundle_id, Digest::ZERO);
    }

    #[test]
    fn assessment_different_energy_changes_id() {
        let k1 = EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean());
        let k2 = EnergyKernel::new(
            TripolarEnergy::new(Q16::HALF, Q16::ONE, Q16::ONE),
            EnergyFactors::all_clean(),
        );
        let a1 = EnergyAssessment::new(d(b"s"), k1, pol(), bundle());
        let a2 = EnergyAssessment::new(d(b"s"), k2, pol(), bundle());
        assert_ne!(a1.id, a2.id);
    }

    // ─── rank_by_energy ─────────────────────────────────────────────────────

    #[test]
    fn rank_orders_by_energy_descending() {
        let lo = EnergyAssessment::new(
            d(b"lo"),
            EnergyKernel::new(
                TripolarEnergy::new(Q16::ratio(1, 4).unwrap(), Q16::ONE, Q16::ONE),
                EnergyFactors::all_clean(),
            ),
            pol(),
            bundle(),
        );
        let hi = EnergyAssessment::new(
            d(b"hi"),
            EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean()),
            pol(),
            bundle(),
        );
        let ranked = rank_by_energy(&[lo.clone(), hi.clone()]);
        assert_eq!(ranked[0].subject_id, hi.subject_id);
        assert_eq!(ranked[1].subject_id, lo.subject_id);
    }

    #[test]
    fn rank_is_deterministic_on_ties() {
        // Two equal-energy assessments must order by subject_id, stably.
        let k = EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean());
        let a = EnergyAssessment::new(d(b"aaa"), k, pol(), bundle());
        let b = EnergyAssessment::new(d(b"bbb"), k, pol(), bundle());
        let r1 = rank_by_energy(&[a.clone(), b.clone()]);
        let r2 = rank_by_energy(&[b, a]);
        assert_eq!(r1[0].subject_id, r2[0].subject_id);
        assert_eq!(r1[1].subject_id, r2[1].subject_id);
    }

    #[test]
    fn rank_never_drops_zero_energy_candidates() {
        let zeroed = EnergyAssessment::new(
            d(b"z"),
            EnergyKernel::new(
                TripolarEnergy::unit(),
                EnergyFactors::derive(
                    &GateResult::Reject { reason: "r".into() },
                    &TaintLabel::Clean,
                    &LicenseStatus::NotApplicable,
                    FoundrySurvival::Passed,
                    Q16::ONE,
                    Q16::ZERO,
                ),
            ),
            pol(),
            bundle(),
        );
        let live = EnergyAssessment::new(
            d(b"l"),
            EnergyKernel::new(TripolarEnergy::unit(), EnergyFactors::all_clean()),
            pol(),
            bundle(),
        );
        let ranked = rank_by_energy(&[zeroed, live]);
        assert_eq!(ranked.len(), 2, "ranking must preserve every candidate");
        assert!(ranked[1].is_zeroed(), "zero-energy sorts last, not dropped");
    }
}
