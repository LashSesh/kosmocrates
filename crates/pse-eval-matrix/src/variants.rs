//! `SystemVariantSpec` and the variant ladder B0…B7 (§3.1, §10.2).
//!
//! `LayerMask` is a bitset over the post-symbolic layers; ablations
//! flip individual bits.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::calibration::CalibrationProfile;
use crate::primitives::{content_address, EvalError, Hash256};

/// Bitset over the post-symbolic layers. Each variant declares which
/// layers are active; ablations flip the corresponding bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LayerMask(pub u32);

impl LayerMask {
    /// Bit for the streaming engine + GateSnapshot + Crystal/Falsifier/Replay layer.
    pub const PSE_CORE: u32 = 1 << 0;
    /// Bit for traversal (FieldCube, DoFGraph, CollapsePlan, …).
    pub const TRAVERSAL: u32 = 1 << 1;
    /// Bit for the signature layer.
    pub const SIGNATURE: u32 = 1 << 2;
    /// Bit for the dynamics layer.
    pub const DYNAMICS: u32 = 1 << 3;
    /// Bit for the horizon layer (NullCenter, HorizonChart, …).
    pub const HORIZON: u32 = 1 << 4;
    /// Bit for the cognition layer.
    pub const COGNITION: u32 = 1 << 5;
    /// Bit for the spiral memory module.
    pub const SPIRAL_MEMORY: u32 = 1 << 6;
    /// Bit for the constraint lattice + hypercube puzzle.
    pub const CONSTRAINT_LATTICE: u32 = 1 << 7;
    /// Bit for the phase panorama.
    pub const PHASE_PANORAMA: u32 = 1 << 8;
    /// Bit for governed wormholes.
    pub const GOVERNED_WORMHOLES: u32 = 1 << 9;
    /// Bit for the self-model tensor.
    pub const SELF_MODEL: u32 = 1 << 10;
    /// Bit for adaptive calibration.
    pub const ADAPTIVE_CALIBRATION: u32 = 1 << 11;
    /// Bit for the morphodynamic resonance cell substrate
    /// (PHASEMATRIX-HIVEMIND-03).
    pub const CELL_SUBSTRATE: u32 = 1 << 12;
    /// Bit for the Dual-Fabric Field-Tensor Stitch Layer
    /// (PHASEMATRIX-HIVEMIND-03.1).
    pub const DUAL_FABRIC_STITCH: u32 = 1 << 13;

    /// `B0_Baseline` — naive baseline / classical detector.
    pub const B0_BASELINE: LayerMask = LayerMask(0);
    /// `B1_PSE_Core` — streaming engine only.
    pub const B1_PSE_CORE: LayerMask = LayerMask(Self::PSE_CORE);
    /// `B2_Traversal`.
    pub const B2_TRAVERSAL: LayerMask = LayerMask(Self::PSE_CORE | Self::TRAVERSAL);
    /// `B3_Signature`.
    pub const B3_SIGNATURE: LayerMask =
        LayerMask(Self::PSE_CORE | Self::TRAVERSAL | Self::SIGNATURE);
    /// `B4_Dynamics`.
    pub const B4_DYNAMICS: LayerMask =
        LayerMask(Self::PSE_CORE | Self::TRAVERSAL | Self::SIGNATURE | Self::DYNAMICS);
    /// `B5_Horizon`.
    pub const B5_HORIZON: LayerMask = LayerMask(
        Self::PSE_CORE | Self::TRAVERSAL | Self::SIGNATURE | Self::DYNAMICS | Self::HORIZON,
    );
    /// `B6_Cognition` — adds CanonicalCognitionState et al.
    pub const B6_COGNITION: LayerMask = LayerMask(
        Self::PSE_CORE
            | Self::TRAVERSAL
            | Self::SIGNATURE
            | Self::DYNAMICS
            | Self::HORIZON
            | Self::COGNITION
            | Self::SPIRAL_MEMORY
            | Self::CONSTRAINT_LATTICE
            | Self::PHASE_PANORAMA
            | Self::SELF_MODEL,
    );
    /// `B7_FullStack` — everything plus governed wormholes and adaptive
    /// calibration.
    pub const B7_FULL_STACK: LayerMask = LayerMask(
        Self::PSE_CORE
            | Self::TRAVERSAL
            | Self::SIGNATURE
            | Self::DYNAMICS
            | Self::HORIZON
            | Self::COGNITION
            | Self::SPIRAL_MEMORY
            | Self::CONSTRAINT_LATTICE
            | Self::PHASE_PANORAMA
            | Self::GOVERNED_WORMHOLES
            | Self::SELF_MODEL
            | Self::ADAPTIVE_CALIBRATION,
    );
    /// `B8_PhaseMatrix` — full stack plus the morphodynamic resonance
    /// cell substrate (PHASEMATRIX-HIVEMIND-03).
    pub const B8_PHASE_MATRIX: LayerMask = LayerMask(Self::B7_FULL_STACK.0 | Self::CELL_SUBSTRATE);
    /// `B9_DualFabricStitch` — B8 plus the Dual-Fabric Field-Tensor
    /// Stitch Layer (PHASEMATRIX-HIVEMIND-03.1).
    pub const B9_DUAL_FABRIC_STITCH: LayerMask =
        LayerMask(Self::B8_PHASE_MATRIX.0 | Self::DUAL_FABRIC_STITCH);

    /// True iff every bit in `bits` is set.
    pub fn has(self, bits: u32) -> bool {
        (self.0 & bits) == bits
    }

    /// Drop the given bits.
    pub fn without(self, bits: u32) -> LayerMask {
        LayerMask(self.0 & !bits)
    }

    /// Number of layers active.
    pub fn count_layers(self) -> u32 {
        self.0.count_ones()
    }
}

/// Solver profile (§10.2). Determines which solver harness the runner
/// uses for a given variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SolverProfile {
    /// Baseline classical detector / naive agent.
    Classical,
    /// Template solver from pse-traverse.
    Template,
    /// Oracle solver (for tests / golden runs).
    Oracle,
    /// Null solver (no candidates produced — useful for safety
    /// regression tests).
    Null,
}

/// `SystemVariantSpec` — a single rung on the ladder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemVariantSpec {
    /// Stable variant id (e.g. `"B6_Cognition"`).
    pub variant_id: String,
    /// Layer bitmask.
    pub layer_mask: LayerMask,
    /// Optional feature flag overrides.
    pub feature_flags: BTreeMap<String, bool>,
    /// Configuration content hash.
    pub config_hash: Hash256,
    /// RunDescriptor template hash.
    pub rd_hash: Hash256,
    /// Solver profile.
    pub solver_profile: SolverProfile,
    /// Calibration profile.
    pub calibration_profile: CalibrationProfile,
}

impl SystemVariantSpec {
    /// `B0_Baseline`.
    pub fn baseline() -> Self {
        Self::from_ladder(
            "B0_Baseline",
            LayerMask::B0_BASELINE,
            SolverProfile::Classical,
        )
    }

    /// `B1_PSE_Core`.
    pub fn pse_core() -> Self {
        Self::from_ladder(
            "B1_PSE_Core",
            LayerMask::B1_PSE_CORE,
            SolverProfile::Template,
        )
    }

    /// `B2_Traversal`.
    pub fn traversal() -> Self {
        Self::from_ladder(
            "B2_Traversal",
            LayerMask::B2_TRAVERSAL,
            SolverProfile::Template,
        )
    }

    /// `B3_Signature`.
    pub fn signature() -> Self {
        Self::from_ladder(
            "B3_Signature",
            LayerMask::B3_SIGNATURE,
            SolverProfile::Template,
        )
    }

    /// `B4_Dynamics`.
    pub fn dynamics() -> Self {
        Self::from_ladder(
            "B4_Dynamics",
            LayerMask::B4_DYNAMICS,
            SolverProfile::Template,
        )
    }

    /// `B5_Horizon`.
    pub fn horizon() -> Self {
        Self::from_ladder("B5_Horizon", LayerMask::B5_HORIZON, SolverProfile::Template)
    }

    /// `B6_Cognition`.
    pub fn cognition() -> Self {
        Self::from_ladder(
            "B6_Cognition",
            LayerMask::B6_COGNITION,
            SolverProfile::Template,
        )
    }

    /// `B7_FullStack`.
    pub fn full_stack() -> Self {
        Self::from_ladder(
            "B7_FullStack",
            LayerMask::B7_FULL_STACK,
            SolverProfile::Template,
        )
    }

    /// `B8_PhaseMatrix` — full stack plus the morphodynamic resonance
    /// cell substrate.
    pub fn phase_matrix_substrate() -> Self {
        Self::from_ladder(
            "B8_PhaseMatrix",
            LayerMask::B8_PHASE_MATRIX,
            SolverProfile::Template,
        )
    }

    /// `B9_DualFabricStitch` — B8 plus the Dual-Fabric Field-Tensor
    /// Stitch Layer (PHASEMATRIX-HIVEMIND-03.1).
    pub fn dual_fabric_stitch() -> Self {
        Self::from_ladder(
            "B9_DualFabricStitch",
            LayerMask::B9_DUAL_FABRIC_STITCH,
            SolverProfile::Template,
        )
    }

    fn from_ladder(name: &str, mask: LayerMask, solver: SolverProfile) -> Self {
        let probe = (name, mask, solver);
        let config_hash = content_address(&probe).unwrap_or_else(|_| Hash256::zero());
        let rd_hash = config_hash.clone();
        SystemVariantSpec {
            variant_id: name.to_string(),
            layer_mask: mask,
            feature_flags: BTreeMap::new(),
            config_hash,
            rd_hash,
            solver_profile: solver,
            calibration_profile: CalibrationProfile::frozen(),
        }
    }

    /// Update the variant's `config_hash` and `rd_hash` so every change
    /// to the variant fields propagates.
    pub fn refresh_hashes(mut self) -> Result<Self, EvalError> {
        let probe = (
            &self.variant_id,
            self.layer_mask,
            &self.feature_flags,
            self.solver_profile,
            &self.calibration_profile,
        );
        self.config_hash = content_address(&probe)?;
        self.rd_hash = self.config_hash.clone();
        Ok(self)
    }
}

/// Convenience constructor for the canonical full ladder B0…B7.
pub struct VariantLadder;

impl VariantLadder {
    /// Returns the canonical full B0…B7 ladder.
    pub fn full() -> Vec<SystemVariantSpec> {
        vec![
            SystemVariantSpec::baseline(),
            SystemVariantSpec::pse_core(),
            SystemVariantSpec::traversal(),
            SystemVariantSpec::signature(),
            SystemVariantSpec::dynamics(),
            SystemVariantSpec::horizon(),
            SystemVariantSpec::cognition(),
            SystemVariantSpec::full_stack(),
        ]
    }

    /// Returns the extended B0…B8 ladder including the
    /// PHASEMATRIX-HIVEMIND-03 cell-substrate rung.
    pub fn full_with_phase_matrix() -> Vec<SystemVariantSpec> {
        let mut ladder = Self::full();
        ladder.push(SystemVariantSpec::phase_matrix_substrate());
        ladder
    }

    /// Returns the extended B0…B9 ladder including both the
    /// PHASEMATRIX-HIVEMIND-03 cell-substrate rung and the
    /// PHASEMATRIX-HIVEMIND-03.1 Dual-Fabric Stitch rung.
    pub fn full_with_dual_fabric_stitch() -> Vec<SystemVariantSpec> {
        let mut ladder = Self::full_with_phase_matrix();
        ladder.push(SystemVariantSpec::dual_fabric_stitch());
        ladder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_is_monotone_in_layer_count() {
        let ladder = VariantLadder::full();
        let mut prev = 0;
        for v in &ladder {
            let c = v.layer_mask.count_layers();
            assert!(c >= prev, "ladder not monotone: {} ↘ {}", prev, c);
            prev = c;
        }
    }

    #[test]
    fn b6_implies_cognition_bits() {
        let m = LayerMask::B6_COGNITION;
        assert!(m.has(LayerMask::COGNITION));
        assert!(m.has(LayerMask::SPIRAL_MEMORY));
        assert!(m.has(LayerMask::PHASE_PANORAMA));
        assert!(m.has(LayerMask::SELF_MODEL));
    }

    #[test]
    fn ablate_drops_bits() {
        let m = LayerMask::B6_COGNITION.without(LayerMask::SPIRAL_MEMORY);
        assert!(!m.has(LayerMask::SPIRAL_MEMORY));
        assert!(m.has(LayerMask::COGNITION));
    }

    #[test]
    fn b8_phase_matrix_implies_b7_plus_cell_substrate() {
        let m = LayerMask::B8_PHASE_MATRIX;
        assert!(m.has(LayerMask::CELL_SUBSTRATE));
        assert!(m.has(LayerMask::ADAPTIVE_CALIBRATION));
        assert!(m.has(LayerMask::COGNITION));
    }

    #[test]
    fn extended_ladder_is_monotone() {
        let ladder = VariantLadder::full_with_phase_matrix();
        let mut prev = 0;
        for v in &ladder {
            let c = v.layer_mask.count_layers();
            assert!(c >= prev);
            prev = c;
        }
        assert_eq!(ladder.len(), 9);
    }
}
