//! Wings of Samael — the radial projection-arm layer (L3, spec ch. 11–12).
//!
//! A wing is the operator path `anchor ∘ condense ∘ harvest ∘ orbit ∘ probe`
//! (Def. 11.1). Its defining law is the **condensation invariant** (Inv. 11.2):
//! an admissible wing reduces degrees of freedom or isolates their relevance.
//! The [`wing_gate`] enforces it fail-closed, together with I3 (boundary over
//! absorption — no covert removal; every dropped unit is visible residue).

use kosmo_cdk_core::{GateError, Status};
use kosmo_core::Digest;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A structure a wing operates on. `dim` is the raw degrees of freedom;
/// `dim_relevant` the invariant-bearing ones. Identity is content-addressed (I1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Structure {
    pub units: Vec<Digest>,
    pub dim: u32,
    pub dim_relevant: u32,
}

impl Structure {
    pub fn new(units: Vec<Digest>, dim: u32, dim_relevant: u32) -> Self {
        Self {
            units,
            dim,
            dim_relevant,
        }
    }
    pub fn id(&self) -> Digest {
        Digest::of(self)
    }
}

/// The output of a wing application: the (condensed) structure, the exkalibrated
/// `residue` (I3 — non-integrable mass made visible, never silently swallowed),
/// and a trace root (I4 — trace and replay).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WingOutput {
    pub structure: Structure,
    pub residue: Vec<Digest>,
    pub trace_root: Digest,
}

/// The six wing kinds (§11.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WingKind {
    Scout,
    Resonance,
    Crystal,
    Materialization,
    Validation,
    Residue,
}

/// A single wing (Def. 11.1). Every admissible wing **condenses** (Inv. 11.2).
pub trait Wing {
    fn kind(&self) -> WingKind;
    /// `w = anchor ∘ condense ∘ harvest ∘ orbit ∘ probe`.
    fn apply(&self, x: &Structure) -> Result<WingOutput, GateError>;
    /// Inv. 11.2: `dim(out) ≤ dim(x)` OR `dim_relevant(out) < dim_relevant(x)`.
    fn condenses(&self, x: &Structure, out: &WingOutput) -> bool {
        out.structure.dim <= x.dim || out.structure.dim_relevant < x.dim_relevant
    }
}

/// The WingGate (§25, fail-closed): a wing output is admissible only if it
/// condenses (Inv. 11.2) **and** every removed unit is visible as residue (I3 —
/// no covert removal; negative tests 1 & 12).
pub fn wing_gate(w: &dyn Wing, x: &Structure, out: &WingOutput) -> Status {
    if !w.condenses(x, out) {
        return Status::Reject; // exploration may enlarge, but the accepted output must condense
    }
    // Negative test 8 — a wing must not integrate structure without evidence: any
    // unit in the output not present in the input requires a non-null trace root.
    let input: HashSet<&Digest> = x.units.iter().collect();
    let integrates_foreign = out.structure.units.iter().any(|u| !input.contains(u));
    if integrates_foreign && out.trace_root == Digest::ZERO {
        return Status::Reject;
    }
    let kept: HashSet<&Digest> = out.structure.units.iter().collect();
    let resid: HashSet<&Digest> = out.residue.iter().collect();
    for u in &x.units {
        if !kept.contains(u) && !resid.contains(u) {
            return Status::Reject; // covert removal → boundary violation (I3)
        }
    }
    Status::Pass
}

/// A CrystalWing extracts the irreducible core: it keeps the first `keep` units as
/// the relevant core and exkalibrates the rest to the boundary as residue.
pub struct CrystalWing {
    pub keep: usize,
}

impl Wing for CrystalWing {
    fn kind(&self) -> WingKind {
        WingKind::Crystal
    }
    fn apply(&self, x: &Structure) -> Result<WingOutput, GateError> {
        let keep = self.keep.min(x.units.len());
        let core = x.units[..keep].to_vec();
        let residue = x.units[keep..].to_vec();
        Ok(WingOutput {
            structure: Structure::new(core, keep as u32, keep as u32),
            residue,
            trace_root: Digest::of_bytes(b"crystalwing"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn structure(n: usize) -> Structure {
        let units: Vec<Digest> = (0..n).map(|i| Digest::of_bytes(&[i as u8])).collect();
        Structure::new(units, n as u32, n as u32)
    }

    #[test]
    fn a_crystal_wing_condenses_and_passes_the_gate() {
        let x = structure(5);
        let w = CrystalWing { keep: 2 };
        let out = w.apply(&x).unwrap();
        assert!(w.condenses(&x, &out), "Inv 11.2: the core has fewer dims");
        assert_eq!(out.residue.len(), 3, "the rest is exkalibrated as residue (I3)");
        assert_eq!(wing_gate(&w, &x, &out), Status::Pass);
    }

    #[test]
    fn a_non_condensing_output_is_rejected() {
        // A wing that GROWS the structure violates Inv 11.2 — the accepted output
        // must condense, never enlarge.
        let x = structure(2);
        let bad = WingOutput {
            structure: Structure::new(
                vec![Digest::of_bytes(&[0]), Digest::of_bytes(&[1]), Digest::of_bytes(&[2])],
                3,
                3,
            ),
            residue: vec![],
            trace_root: Digest::ZERO,
        };
        let w = CrystalWing { keep: 2 };
        assert_eq!(wing_gate(&w, &x, &bad), Status::Reject);
    }

    #[test]
    fn integrating_foreign_structure_without_evidence_is_rejected() {
        // Negative test 8 — a wing integrates a unit not in the input, with no
        // trace root (no evidence) → rejected.
        let x = structure(2);
        let foreign = WingOutput {
            structure: Structure::new(vec![x.units[0], Digest::of_bytes(b"foreign")], 2, 2),
            residue: vec![x.units[1]],
            trace_root: Digest::ZERO, // no evidence for the integrated structure
        };
        let w = CrystalWing { keep: 2 };
        assert_eq!(wing_gate(&w, &x, &foreign), Status::Reject, "integration needs evidence");
    }

    #[test]
    fn covert_removal_is_rejected() {
        // A unit that left the structure but is NOT in residue is silent
        // forgetting — a boundary violation (I3, negative tests 1 & 12).
        let x = structure(3);
        let covert = WingOutput {
            structure: Structure::new(vec![x.units[0]], 1, 1),
            residue: vec![x.units[1]], // unit[2] vanished without a trace
            trace_root: Digest::ZERO,
        };
        let w = CrystalWing { keep: 1 };
        assert_eq!(wing_gate(&w, &x, &covert), Status::Reject, "no covert removal");
    }
}
