//! Fabric-H: `ResonanceFabricState` and `EphemeralResonanceLink`
//! (PHASEMATRIX-HIVEMIND-03.1 §5).
//!
//! Fabric-H holds the **ephemeral** resonance constellation derived from
//! the most recent `CellSubstrateCycleReport`.  It is *read-only* from
//! the Stitcher's perspective — it must never mutate Fabric-T directly
//! (Invariant 1).

use serde::{Deserialize, Serialize};

use super::pipeline::CellSubstrateCycleReport;
use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, PhaseError};
use super::stitcher::StitchRunDescriptor;

/// A transient resonance link between two cells derived from Fabric-H
/// (§5.1). Links are ephemeral: they carry a `ttl_ticks` budget and are
/// never written to Fabric-T as-is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EphemeralResonanceLink {
    /// Content address of this link.
    pub link_id: Hash256,
    /// Source cell hash.
    pub source_cell: Hash256,
    /// Target cell hash.
    pub target_cell: Hash256,
    /// Float-free resonance score ∈ [0, 1].
    pub resonance_score: Fixed,
    /// Float-free phase alignment ∈ [0, 1].
    pub phase_alignment: Fixed,
    /// Remaining ticks this link is valid.
    pub ttl_ticks: u64,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl EphemeralResonanceLink {
    /// Recompute `link_id`.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        let mut probe = self.clone();
        probe.link_id = Hash256::zero();
        self.link_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Fabric-H snapshot (§5.1).
///
/// Invariant (§5.2): Fabric-H MUST always carry a `trace_hash` that
/// explains which cluster reports, pulses and convergence fields the
/// link candidates originate from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResonanceFabricState {
    /// Content address of this Fabric-H snapshot.
    pub fabric_h_id: Hash256,
    /// Content address of the source `CellSubstrateCycleReport`.
    pub source_cycle_report_hash: Hash256,
    /// Hashes of the active `ResonanceCluster`s captured in this snapshot.
    pub active_cluster_hashes: Vec<Hash256>,
    /// Hashes of the `ResonancePulse`s used to derive links.
    pub resonance_pulse_hashes: Vec<Hash256>,
    /// Ephemeral links (never persisted to Fabric-T directly).
    pub transient_links: Vec<EphemeralResonanceLink>,
    /// Hashes of the `ConvergenceField`s contributing to this snapshot.
    pub convergence_field_hashes: Vec<Hash256>,
    /// Provenance trace — explains the derivation of every link candidate.
    pub trace_hash: Hash256,
    /// Evidence references.
    pub evidence_refs: Vec<EvidenceRef>,
}

impl ResonanceFabricState {
    /// Recompute `fabric_h_id`.
    ///
    /// Sorts `active_cluster_hashes`, `resonance_pulse_hashes`,
    /// `convergence_field_hashes` and `transient_links` before hashing.
    pub fn with_id(mut self) -> Result<Self, PhaseError> {
        self.active_cluster_hashes.sort();
        self.resonance_pulse_hashes.sort();
        self.convergence_field_hashes.sort();
        self.transient_links.sort();
        let mut probe = self.clone();
        probe.fabric_h_id = Hash256::zero();
        self.fabric_h_id = content_address(&probe)?;
        Ok(self)
    }
}

/// Build `ResonanceFabricState` from a completed cell-substrate cycle
/// (pipeline step 4 in §11.1).
///
/// The function is deterministic: same `rd` + same `report` → same
/// `fabric_h_id`.
pub fn build_resonance_fabric(
    rd: &StitchRunDescriptor,
    report: &CellSubstrateCycleReport,
) -> Result<ResonanceFabricState, PhaseError> {
    // Cluster hashes.
    let active_cluster_hashes = vec![report.cluster.cluster_id.clone()];

    // Pulse hashes — derived from convergence field input pulse list.
    let resonance_pulse_hashes: Vec<Hash256> = {
        let mut v = report.convergence.input_pulses.clone();
        v.sort();
        v
    };

    // Convergence field hash.
    let convergence_field_hashes = vec![report.convergence.field_id.clone()];

    // Derive transient links from cluster member pairs.
    // Pairs are sorted so derivation is order-independent.
    let mut members = report.cluster.member_cells.clone();
    members.sort();

    let coherence = &report.cluster.coherence_score;
    let phase_alignment = &report.cluster.cluster_trident.temporal_phase;

    let mut transient_links = Vec::new();
    for i in 0..members.len() {
        for j in (i + 1)..members.len() {
            let link = EphemeralResonanceLink {
                link_id: Hash256::zero(),
                source_cell: members[i].clone(),
                target_cell: members[j].clone(),
                resonance_score: coherence.clone(),
                phase_alignment: phase_alignment.clone(),
                ttl_ticks: rd.thresholds.max_updates_per_cycle,
                evidence_refs: vec![EvidenceRef {
                    kind: "cluster.trace".into(),
                    address: report.trace.trace_id.clone(),
                    relation: "derived_from".into(),
                }],
            }
            .with_id()?;
            transient_links.push(link);
        }
    }

    // Trace hash: hash of the provenance inputs.
    let trace_hash = content_address(&(
        "fabric_h.trace",
        &report.trace.trace_id,
        &report.cluster.cluster_id,
        &rd.run_id,
    ))?;

    ResonanceFabricState {
        fabric_h_id: Hash256::zero(),
        source_cycle_report_hash: report.report_id.clone(),
        active_cluster_hashes,
        resonance_pulse_hashes,
        transient_links,
        convergence_field_hashes,
        trace_hash,
        evidence_refs: vec![EvidenceRef {
            kind: "cell_substrate.cycle_report".into(),
            address: report.report_id.clone(),
            relation: "source_of".into(),
        }],
    }
    .with_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::pipeline::{
        run_cell_substrate_cycle, CellSubstrateInput, CellSubstrateOutcome,
    };
    use crate::cell::run_descriptor::{
        CellSubstratePolicies, CellSubstrateThresholds, PhaseMatrixRunDescriptorV3,
    };
    use std::collections::BTreeMap;

    fn make_rd() -> (PhaseMatrixRunDescriptorV3, StitchRunDescriptor) {
        let pm_rd = PhaseMatrixRunDescriptorV3 {
            run_id: "stitch.test".into(),
            matrix_id: Hash256::zero(),
            parent_profile_hash: None,
            operator_versions: BTreeMap::new(),
            canonicalization_version: "phase-matrix-v0.3".into(),
            thresholds: CellSubstrateThresholds::permissive(),
            policies: CellSubstratePolicies::strict(),
            feature_flags: Default::default(),
            evidence_refs: Vec::new(),
        };
        let stitch_rd = StitchRunDescriptor::default_for(&pm_rd).unwrap();
        (pm_rd, stitch_rd)
    }

    fn make_report(pm_rd: &PhaseMatrixRunDescriptorV3) -> CellSubstrateCycleReport {
        let input = CellSubstrateInput::synthetic(pm_rd.matrix_id.clone(), 3, 3).unwrap();
        match run_cell_substrate_cycle(pm_rd, &input).unwrap() {
            CellSubstrateOutcome::Completed(r) => r,
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[test]
    fn resonance_fabric_is_content_addressed() {
        let (pm_rd, stitch_rd) = make_rd();
        let report = make_report(&pm_rd);
        let f1 = build_resonance_fabric(&stitch_rd, &report).unwrap();
        let f2 = build_resonance_fabric(&stitch_rd, &report).unwrap();
        assert_eq!(
            f1.fabric_h_id, f2.fabric_h_id,
            "fabric_h must be deterministic"
        );
    }

    #[test]
    fn fabric_h_has_trace_hash() {
        let (pm_rd, stitch_rd) = make_rd();
        let report = make_report(&pm_rd);
        let fabric = build_resonance_fabric(&stitch_rd, &report).unwrap();
        assert_ne!(
            fabric.trace_hash,
            Hash256::zero(),
            "Fabric-H must carry a non-zero trace_hash"
        );
    }
}
