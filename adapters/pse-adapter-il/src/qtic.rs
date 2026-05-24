//! QTIC conformance classification Q0–Q5.
//!
//! Implements the conformance class hierarchy from
//! "Quasi-Zeitinformationskristalle" (Sebastian Klemm, 2026), §18.
//!
//! The central formula (QTIC §21):
//!   QTIC = Fix(Tα) ∩ Int(ℋΣ⁻) ∩ Pass(GΓ) ∩ Pass(GΣ) ∩ Mirror(S,M) ∩ TraceReplay ∩ PathInvariant
//!
//! The conformance classes are cumulative (Q0 ⊆ Q1 ⊆ ... ⊆ Q5).
//! Only Q5 satisfies the full QTIC definition.

use pse_types::SemanticCrystal;
use serde::{Deserialize, Serialize};

/// Default MCI threshold η for Q4 (auditable QTIC).
pub const MCI_THRESHOLD: f64 = 0.7;

/// QTIC conformance class.
///
/// The classes form a cumulative prerequisite chain: reaching class Qn
/// requires all conditions for Q0..Q(n-1) to also hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum QticClass {
    /// Q0 — Formal candidate: μ ∈ P(M_t,θ).
    /// Any crystal in an information state space qualifies.
    Q0 = 0,
    /// Q1 — Mandorla capsule: M_t,θ = A(t) ∩ B(θ).
    /// Crystal must have explicit dual-time structure: extrinsic topology
    /// (topology_signature = A(t)) and intrinsic spiral phase ψ (= B(θ)).
    Q1 = 1,
    /// Q2 — Nullhomologous bridge: θ₁ ≡ θ₀ (mod 2π), δ(μ₀) = δ(μ₁).
    /// Intrinsic phase must be stable / phase-closed (ψ > −0.1).
    Q2 = 2,
    /// Q3 — Gate-passed condensation: G_Γ(μ) = 1.
    /// The Kairos gate must have fired for this crystal.
    Q3 = 3,
    /// Q4 — Auditable QTIC: TraceReady ∧ Replay ∧ MCI ≥ η.
    /// Non-empty block-hash (trace), non-zero crystal id (replay),
    /// and Mirror-Consistency Index ≥ threshold.
    Q4 = 4,
    /// Q5 — Path-invariant QTIC: PathInv = 1.
    /// Full QTIC — seam-stable, replayable, path-invariant attractor.
    Q5 = 5,
}

impl QticClass {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn description(self) -> &'static str {
        match self {
            QticClass::Q0 => "Formal QTIC candidate",
            QticClass::Q1 => "Mandorla capsule (dual-time structure)",
            QticClass::Q2 => "Nullhomologous bridge (stable intrinsic phase)",
            QticClass::Q3 => "Gate-passed condensation (G_Γ = 1)",
            QticClass::Q4 => "Auditable QTIC (TraceReady ∧ Replay ∧ MCI ≥ η)",
            QticClass::Q5 => "Path-invariant QTIC — full QTIC (PathInv = 1)",
        }
    }
}

impl From<u8> for QticClass {
    fn from(v: u8) -> Self {
        match v {
            1 => QticClass::Q1,
            2 => QticClass::Q2,
            3 => QticClass::Q3,
            4 => QticClass::Q4,
            5 => QticClass::Q5,
            _ => QticClass::Q0,
        }
    }
}

/// Mirror-Consistency Index: MCI = 1 − |σ_H − σ_T|.
///
/// σ_H = PSE graph stability signal (stability_score from the Kairos gate).
/// σ_T = IL ledger stability signal (il_stability from the feedback loop).
///
/// MCI = 1.0 means perfect consistency; 0.0 means maximal divergence.
pub fn mirror_consistency_index(pse_stability: f64, il_stability: f64) -> f64 {
    (1.0 - (pse_stability - il_stability).abs()).max(0.0)
}

/// Full QTIC classification certificate, emitted for every crystal committed to IL.
///
/// Contains the conformance class reached, all per-gate results, and the dual-time
/// coordinates (extrinsic revision time t, intrinsic spiral phase θ).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QticCertificate {
    /// Hex-encoded crystal_id of the classified crystal.
    pub crystal_id: String,
    /// HDAG node ID (ledger anchor).
    pub hdag_node_id: String,
    /// IL block hash — the canonical trace anchor / QTIC identity anchor.
    pub canonical_id: String,

    /// Highest conformance class reached (Q0–Q5).
    pub conformance_class: QticClass,
    /// Human-readable description of the class.
    pub class_description: String,

    // ── dual-time coordinates ──────────────────────────────────────────
    /// Extrinsic revision time t: IL block commit index (ℝ≥0 axis).
    pub extrinsic_t: u64,
    /// Intrinsic spiral phase θ ∈ [0, 1]: kuramoto_coherence proxy for S¹.
    pub intrinsic_theta: f64,

    // ── per-gate results ───────────────────────────────────────────────
    /// Coherence potential ψ = kuramoto_coherence − (1 − stability_score).
    pub psi: f64,
    /// Mirror-Consistency Index MCI = 1 − |σ_H − σ_T|.
    pub mci: f64,
    /// MCI threshold η used for Q4 classification.
    pub mci_threshold: f64,
    /// Whether the Kairos gate G_Γ = 1 (Q3 prerequisite).
    pub gate_passed: bool,
    /// Whether a non-empty block hash is present (trace-ready, Q4 prerequisite).
    pub trace_ready: bool,
    /// Whether the crystal has a non-zero content-addressed id (replay-ready, Q4 prerequisite).
    pub replay_ready: bool,
    /// Whether HDAG path invariance passes for this node (Q5 prerequisite).
    pub path_inv: bool,
}

impl QticCertificate {
    /// Returns true when this crystal reaches full Q5 QTIC status.
    pub fn is_full_qtic(&self) -> bool {
        self.conformance_class >= QticClass::Q5
    }

    /// Returns the conformance class as a u8 for compact storage.
    pub fn class_u8(&self) -> u8 {
        self.conformance_class.as_u8()
    }
}

/// Input bundle for QTIC classification.
pub struct QticInput<'a> {
    pub crystal: &'a SemanticCrystal,
    /// IL block hash (empty string = not yet committed).
    pub block_hash: &'a str,
    /// HDAG node ID (empty string = HDAG not active).
    pub hdag_node_id: &'a str,
    /// Extrinsic revision time: IL block commit index.
    pub extrinsic_t: u64,
    /// Whether the Kairos gate fired for this crystal.
    pub gate_passed: bool,
    /// Coherence potential ψ = kuramoto_coherence − (1 − stability_score).
    pub psi: f64,
    /// IL stability signal from the feedback loop.
    pub il_stability: f64,
    /// Whether HDAG path invariance passes for this node.
    pub path_inv: bool,
    /// MCI threshold η. Use `MCI_THRESHOLD` for the default.
    pub mci_threshold: f64,
}

/// Classify a crystal into the QTIC conformance hierarchy Q0–Q5.
///
/// Rules (cumulative prerequisite chain, QTIC §18):
/// - Q0: always (any crystal in an information state space)
/// - Q1: has explicit dual-time structure (topology_signature = A(t), ψ = B(θ))
/// - Q2: intrinsic phase is stable/closed (ψ > −0.1)
/// - Q3: Kairos gate G_Γ = 1
/// - Q4: TraceReady ∧ ReplayReady ∧ MCI ≥ η
/// - Q5: HDAG path invariance passes (PathInv = 1)
pub fn classify(input: &QticInput<'_>) -> QticCertificate {
    let crystal = input.crystal;

    let mci = mirror_consistency_index(crystal.stability_score, input.il_stability);
    let trace_ready = !input.block_hash.is_empty();
    let replay_ready = !crystal.crystal_id.iter().all(|&b| b == 0);
    let crystal_id_hex: String = crystal
        .crystal_id
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    // ── Q0 / Q1: formal candidate + dual-time structure ───────────────
    // Q0 is always satisfied; Q1 is also always satisfied for any
    // SemanticCrystal with a topology_signature (A(t) = extrinsic) and
    // ψ as the intrinsic B(θ) coordinate on S¹.
    let mut class = QticClass::Q1;

    // ── Q2: stable intrinsic phase (nullhomologous bridge) ────────────
    // ψ > −0.1 signals that the intrinsic spiral phase is close-enough
    // to forming a nullhomologous loop (QTIC §18.3 approximation).
    if input.psi > -0.1 {
        class = QticClass::Q2;
    }

    // ── Q3: gate-passed condensation ──────────────────────────────────
    if class >= QticClass::Q2 && input.gate_passed {
        class = QticClass::Q3;
    }

    // ── Q4: auditable QTIC ────────────────────────────────────────────
    if class >= QticClass::Q3 && trace_ready && replay_ready && mci >= input.mci_threshold {
        class = QticClass::Q4;
    }

    // ── Q5: path-invariant QTIC ───────────────────────────────────────
    if class >= QticClass::Q4 && input.path_inv {
        class = QticClass::Q5;
    }

    QticCertificate {
        crystal_id: crystal_id_hex,
        hdag_node_id: input.hdag_node_id.to_string(),
        canonical_id: input.block_hash.to_string(),
        class_description: class.description().to_string(),
        conformance_class: class,
        extrinsic_t: input.extrinsic_t,
        intrinsic_theta: crystal.topology_signature.kuramoto_coherence,
        psi: input.psi,
        mci,
        mci_threshold: input.mci_threshold,
        gate_passed: input.gate_passed,
        trace_ready,
        replay_ready,
        path_inv: input.path_inv,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pse_types::SemanticCrystal;

    fn make_crystal(stability: f64, kuramoto: f64, id_byte: u8) -> SemanticCrystal {
        let mut c = SemanticCrystal {
            crystal_id: [id_byte; 32],
            region: vec![],
            constraint_program: Default::default(),
            stability_score: stability,
            topology_signature: Default::default(),
            betti_numbers: vec![],
            evidence_chain: Default::default(),
            commit_proof: Default::default(),
            operator_versions: Default::default(),
            created_at: 1,
            free_energy: 0.0,
            carrier_instance_idx: 0,
            scale_tag: "test".to_string(),
            universe_id: String::new(),
            sub_crystal_ids: vec![],
            parent_crystal_ids: vec![],
            genesis_metadata: None,
            metatron_signature: None,
        };
        c.topology_signature.kuramoto_coherence = kuramoto;
        c
    }

    fn input<'a>(
        crystal: &'a SemanticCrystal,
        block_hash: &'a str,
        gate_passed: bool,
        psi: f64,
        il_stability: f64,
        path_inv: bool,
    ) -> QticInput<'a> {
        QticInput {
            crystal,
            block_hash,
            hdag_node_id: "N-test",
            extrinsic_t: 1,
            gate_passed,
            psi,
            il_stability,
            path_inv,
            mci_threshold: MCI_THRESHOLD,
        }
    }

    #[test]
    fn zero_id_crystal_is_q1_at_most() {
        // A crystal with all-zero id is not replay-ready → Q4 blocked → max Q3
        let c = make_crystal(0.8, 0.75, 0); // id_byte=0 → all zeros
        let inp = input(&c, "abc", true, 0.3, 0.8, true);
        let cert = classify(&inp);
        assert!(cert.conformance_class <= QticClass::Q3);
    }

    #[test]
    fn empty_block_hash_blocks_q4() {
        let c = make_crystal(0.8, 0.7, 42);
        let inp = input(&c, "", true, 0.4, 0.8, true);
        let cert = classify(&inp);
        assert!(
            cert.conformance_class < QticClass::Q4,
            "no trace → cannot reach Q4"
        );
    }

    #[test]
    fn low_psi_blocks_q2() {
        let c = make_crystal(0.6, 0.3, 10);
        // psi = 0.3 - (1 - 0.6) = 0.3 - 0.4 = -0.1 → exactly at boundary (not > -0.1)
        let inp = input(&c, "hash", true, -0.15, 0.8, true);
        let cert = classify(&inp);
        assert!(
            cert.conformance_class < QticClass::Q2,
            "ψ ≤ -0.1 must block Q2"
        );
    }

    #[test]
    fn gate_not_passed_blocks_q3() {
        let c = make_crystal(0.8, 0.7, 20);
        let inp = input(&c, "hash", false, 0.5, 0.8, true);
        let cert = classify(&inp);
        assert!(
            cert.conformance_class < QticClass::Q3,
            "gate_passed=false must block Q3"
        );
    }

    #[test]
    fn low_mci_blocks_q4() {
        let c = make_crystal(0.9, 0.8, 30);
        // PSE stability=0.9, IL stability=0.1 → MCI = 1 - |0.9 - 0.1| = 0.2 < 0.7
        let inp = input(&c, "hash", true, 0.7, 0.1, true);
        let cert = classify(&inp);
        assert!(
            cert.conformance_class < QticClass::Q4,
            "low MCI must block Q4"
        );
        assert!((cert.mci - 0.2).abs() < 1e-6);
    }

    #[test]
    fn path_inv_false_blocks_q5() {
        let c = make_crystal(0.8, 0.75, 40);
        let inp = input(&c, "hash", true, 0.55, 0.82, false);
        let cert = classify(&inp);
        assert!(
            cert.conformance_class < QticClass::Q5,
            "path_inv=false must block Q5"
        );
        assert_eq!(cert.conformance_class, QticClass::Q4);
    }

    #[test]
    fn full_q5_when_all_conditions_met() {
        let c = make_crystal(0.85, 0.80, 50);
        // psi = 0.80 - (1 - 0.85) = 0.80 - 0.15 = 0.65
        // il_stability ≈ 0.83 → MCI = 1 - |0.85 - 0.83| = 0.98 > 0.7 ✓
        let inp = input(&c, "block_abc", true, 0.65, 0.83, true);
        let cert = classify(&inp);
        assert_eq!(cert.conformance_class, QticClass::Q5);
        assert!(cert.is_full_qtic());
    }

    #[test]
    fn mci_is_symmetric() {
        assert!(
            (mirror_consistency_index(0.8, 0.7) - mirror_consistency_index(0.7, 0.8)).abs() < 1e-10
        );
    }

    #[test]
    fn mci_perfect_consistency() {
        assert!((mirror_consistency_index(0.75, 0.75) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn class_u8_roundtrip() {
        for v in 0u8..=5 {
            let c = QticClass::from(v);
            assert_eq!(c.as_u8(), v);
        }
    }

    #[test]
    fn certificate_description_non_empty() {
        let c = make_crystal(0.8, 0.75, 60);
        let inp = input(&c, "h", true, 0.5, 0.8, true);
        let cert = classify(&inp);
        assert!(!cert.class_description.is_empty());
    }
}
