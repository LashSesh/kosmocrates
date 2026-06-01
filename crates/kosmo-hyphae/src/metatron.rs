//! Metatron v0.4.1 M1/M2 microtopology skeleton.
//!
//! M1 (lift): HostVoidRegion → TopologyRegionRef → project → SemanticLossRecord
//!            → MetatronMicrograph → MetatronRegionFingerprint → MicrographLiftReport
//! M2 (diagnose): MetatronMicrograph + fingerprint → MicroTopologyDiagnostic
//!                (anomalies, ambiguities, ComplementVoidHypotheses)
//!
//! Phase 6 skeleton: no real graph traversal. Structural approximation only.
//! All artifacts preserve source_evidence_id backref (CROSS-006).
//! Fingerprint equality does NOT imply semantic equivalence.

use crate::void_map::HostVoidKind;
use kosmo_core::{
    Digest, EnergyAssessment, EnergyFactors, EnergyKernel, FoundrySurvival,
    GateResult, LicenseStatus, PolicyProfile, Q16, TaintLabel, TripolarEnergy,
};
use serde::{Deserialize, Serialize};

// ─── Topology Region Reference ────────────────────────────────────────────────

/// Serialize-only for TopologyRegionRef content-addressing.
#[derive(Serialize)]
struct RegionRefContent {
    host_void_id: Digest,
    node_ids: Vec<Digest>,
    radius: u32,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// A reference to a specific region of the host HDAG topology.
///
/// `node_ids` are sorted for determinism. `radius` is the BFS depth used
/// during extraction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologyRegionRef {
    pub region_id: Digest,
    pub host_void_id: Digest,
    pub node_ids: Vec<Digest>,
    pub radius: u32,
    pub evidence_bundle_id: Digest,
    pub policy_id: Digest,
}

impl TopologyRegionRef {
    pub fn new(
        host_void_id: Digest,
        mut node_ids: Vec<Digest>,
        radius: u32,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        node_ids.sort();
        let region_id = Digest::of(&RegionRefContent {
            host_void_id,
            node_ids: node_ids.clone(),
            radius,
            evidence_bundle_id,
            policy_id,
        });
        Self { region_id, host_void_id, node_ids, radius, evidence_bundle_id, policy_id }
    }

    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }
}

// ─── Extraction / Projection Profiles ────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionMethod {
    BreadthFirst,
    DepthFirst,
    Custom(String),
}

/// Serialize-only for RegionExtractionProfile content-addressing.
#[derive(Serialize)]
struct ExtractionProfileContent {
    method: String,
    max_depth: u32,
    max_nodes: u32,
}

/// Describes how a region was extracted from the HDAG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionExtractionProfile {
    pub profile_id: Digest,
    pub extraction_method: ExtractionMethod,
    pub max_depth: u32,
    pub max_nodes: u32,
}

impl RegionExtractionProfile {
    pub fn standard() -> Self {
        let extraction_method = ExtractionMethod::BreadthFirst;
        let max_depth = 3u32;
        let max_nodes = 32u32;
        let profile_id = Digest::of(&ExtractionProfileContent {
            method: format!("{:?}", extraction_method),
            max_depth,
            max_nodes,
        });
        Self { profile_id, extraction_method, max_depth, max_nodes }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionKind {
    Structural,
    Semantic,
    Hybrid,
}

/// Serialize-only for ProjectionProfile content-addressing.
#[derive(Serialize)]
struct ProjectionProfileContent {
    kind: String,
    dimension_count: u32,
    policy_id: Digest,
}

/// Describes how a HDAG region is projected for fingerprinting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectionProfile {
    pub profile_id: Digest,
    pub projection_kind: ProjectionKind,
    pub dimension_count: u32,
    pub policy_id: Digest,
}

impl ProjectionProfile {
    pub fn structural(dimension_count: u32, policy_id: Digest) -> Self {
        let profile_id = Digest::of(&ProjectionProfileContent {
            kind: "Structural".into(),
            dimension_count,
            policy_id,
        });
        Self { profile_id, projection_kind: ProjectionKind::Structural, dimension_count, policy_id }
    }
}

// ─── Semantic Loss Record ─────────────────────────────────────────────────────

/// Serialize-only for SemanticLossRecord content-addressing.
#[derive(Serialize)]
struct LossContent {
    region_id: Digest,
    lost_node_count: u64,
    lost_edge_count: u64,
    loss_ratio: i64,
    policy_id: Digest,
}

/// Records semantic information lost when projecting a HDAG region.
///
/// `loss_ratio` is Q16 in [0, 1] (no floats — CROSS-007).
/// `lost_node_ids` are sorted for determinism.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticLossRecord {
    pub loss_id: Digest,
    pub region_id: Digest,
    pub lost_node_ids: Vec<Digest>,
    pub lost_edge_count: u64,
    pub loss_ratio: Q16,
    pub policy_id: Digest,
}

impl SemanticLossRecord {
    pub fn new(
        region_id: Digest,
        mut lost_node_ids: Vec<Digest>,
        lost_edge_count: u64,
        total_nodes: u64,
        policy_id: Digest,
    ) -> Self {
        lost_node_ids.sort();
        let lost_node_count = lost_node_ids.len() as u64;
        let loss_ratio = if total_nodes == 0 {
            Q16::ZERO
        } else {
            Q16::ratio(lost_node_count, total_nodes).unwrap_or(Q16::ZERO)
        };
        let loss_id = Digest::of(&LossContent {
            region_id,
            lost_node_count,
            lost_edge_count,
            loss_ratio: loss_ratio.raw(),
            policy_id,
        });
        Self { loss_id, region_id, lost_node_ids, lost_edge_count, loss_ratio, policy_id }
    }

    pub fn is_lossless(&self) -> bool {
        self.loss_ratio == Q16::ZERO
    }

    /// Build an [`EnergyAssessment`] for this loss record.
    ///
    /// - ψ (meaning)   = `loss_ratio` — high ψ means high semantic loss (most lossy lifts rank first).
    /// - ρ (coherence) = `Q16::ONE` — no additional coherence data at loss record level.
    /// - ω (phase)     = `Q16::ONE` — no phase data at loss record level.
    /// - `evidence_bundle_id` = `region_id` — the HDAG region being projected (CROSS-006).
    pub fn energy_assessment(&self, gate: &GateResult) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.loss_ratio, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: Q16::ONE,
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.loss_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            self.region_id,
        )
    }
}

// ─── Metatron Micrograph ───────────────────────────────────────────────────────

/// Serialize-only for MetatronMicrograph content-addressing.
#[derive(Serialize)]
struct MicrographContent {
    region_id: Digest,
    projection_profile_id: Digest,
    loss_id: Digest,
    fingerprint_id: Digest,
    source_evidence_id: Digest,
    taint: String,
    policy_id: Digest,
}

/// A content-addressed snapshot of a small HDAG topology region.
///
/// Preserves `source_evidence_id` backref (CROSS-006). Taint propagates
/// from the source HDAG region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetatronMicrograph {
    pub micrograph_id: Digest,
    pub region_ref: TopologyRegionRef,
    pub projection_profile_id: Digest,
    pub loss_id: Digest,
    /// Pre-computed fingerprint placeholder (filled by M1 lift).
    pub fingerprint_id: Digest,
    /// Backref to source evidence (CROSS-006).
    pub source_evidence_id: Digest,
    pub taint: TaintLabel,
    pub policy_id: Digest,
}

impl MetatronMicrograph {
    pub fn new(
        region_ref: TopologyRegionRef,
        projection_profile_id: Digest,
        loss_id: Digest,
        fingerprint_id: Digest,
        source_evidence_id: Digest,
        taint: TaintLabel,
        policy_id: Digest,
    ) -> Self {
        let micrograph_id = Digest::of(&MicrographContent {
            region_id: region_ref.region_id,
            projection_profile_id,
            loss_id,
            fingerprint_id,
            source_evidence_id,
            taint: format!("{:?}", taint),
            policy_id,
        });
        Self {
            micrograph_id,
            region_ref,
            projection_profile_id,
            loss_id,
            fingerprint_id,
            source_evidence_id,
            taint,
            policy_id,
        }
    }
}

// ─── Micrograph Lift Report ────────────────────────────────────────────────────

/// Serialize-only for MicrographLiftReport content-addressing.
#[derive(Serialize)]
struct LiftReportContent {
    micrograph_id: Digest,
    lifted_node_count: u64,
    lifted_edge_count: u64,
    loss_ratio: i64,
    policy_id: Digest,
}

/// Report produced by the M1 lift operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicrographLiftReport {
    pub report_id: Digest,
    pub micrograph_id: Digest,
    pub lifted_node_count: u64,
    pub lifted_edge_count: u64,
    pub loss_ratio: Q16,
    pub policy_id: Digest,
}

impl MicrographLiftReport {
    pub fn new(
        micrograph: &MetatronMicrograph,
        lifted_node_count: u64,
        lifted_edge_count: u64,
        loss_ratio: Q16,
    ) -> Self {
        let report_id = Digest::of(&LiftReportContent {
            micrograph_id: micrograph.micrograph_id,
            lifted_node_count,
            lifted_edge_count,
            loss_ratio: loss_ratio.raw(),
            policy_id: micrograph.policy_id,
        });
        Self {
            report_id,
            micrograph_id: micrograph.micrograph_id,
            lifted_node_count,
            lifted_edge_count,
            loss_ratio,
            policy_id: micrograph.policy_id,
        }
    }

    /// Build an [`EnergyAssessment`] for this lift report.
    ///
    /// - ψ (meaning)   = `loss_ratio` — high ψ means high semantic loss (most lossy lifts rank first).
    /// - ρ (coherence) = `Q16::ONE` — no additional coherence data at lift report level.
    /// - ω (phase)     = `Q16::ONE` — no phase data at lift report level.
    /// - `evidence_bundle_id` = `micrograph_id` — the lifted micrograph is the causal source (CROSS-006).
    pub fn energy_assessment(&self, gate: &GateResult) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.loss_ratio, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: Q16::ONE,
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.report_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            self.micrograph_id,
        )
    }
}

// ─── Region Fingerprint ────────────────────────────────────────────────────────

/// Serialize-only for structural hash computation.
#[derive(Serialize)]
struct StructuralHashInput {
    region_id: Digest,
    node_count: u64,
    edge_count: u64,
}

/// Serialize-only for MetatronRegionFingerprint content-addressing.
#[derive(Serialize)]
struct FingerprintContent {
    micrograph_id: Digest,
    node_count: u64,
    edge_count: u64,
    structural_hash: Digest,
    taint: String,
    policy_id: Digest,
}

/// Content-addressed fingerprint of a MetatronMicrograph region.
///
/// Captures structural shape independent of node labels.
/// **INVARIANT**: fingerprint equality does NOT imply semantic equivalence —
/// two regions with identical shape may have entirely different semantics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetatronRegionFingerprint {
    pub fingerprint_id: Digest,
    pub micrograph_id: Digest,
    pub node_count: u64,
    pub edge_count: u64,
    pub structural_hash: Digest,
    pub taint: TaintLabel,
    pub policy_id: Digest,
}

impl MetatronRegionFingerprint {
    pub fn from_micrograph(micrograph: &MetatronMicrograph, edge_count: u64) -> Self {
        let node_count = micrograph.region_ref.node_count() as u64;
        let structural_hash = Digest::of(&StructuralHashInput {
            region_id: micrograph.region_ref.region_id,
            node_count,
            edge_count,
        });
        let fingerprint_id = Digest::of(&FingerprintContent {
            micrograph_id: micrograph.micrograph_id,
            node_count,
            edge_count,
            structural_hash,
            taint: format!("{:?}", micrograph.taint),
            policy_id: micrograph.policy_id,
        });
        Self {
            fingerprint_id,
            micrograph_id: micrograph.micrograph_id,
            node_count,
            edge_count,
            structural_hash,
            taint: micrograph.taint.clone(),
            policy_id: micrograph.policy_id,
        }
    }
}

// ─── Anomaly ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyKind {
    Disconnected,
    CyclicReference,
    MissingBackref,
    UnexpectedIsolation,
    Custom(String),
}

/// One anomaly detected in a micrograph region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnomalyRecord {
    pub anomaly_id: Digest,
    pub kind: AnomalyKind,
    pub severity: Q16,
    pub location: String,
}

impl AnomalyRecord {
    pub fn new(kind: AnomalyKind, severity: Q16, location: String) -> Self {
        let anomaly_id = Digest::of_bytes(
            format!("{:?}:{}:{}", kind, severity.raw(), location).as_bytes(),
        );
        Self { anomaly_id, kind, severity, location }
    }
}

// ─── Topology Ambiguity ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmbiguityKind {
    Structural,
    Semantic,
    Boundary,
    Authority,
}

/// Serialize-only for TopologyAmbiguityProfile content-addressing.
#[derive(Serialize)]
struct AmbiguityContent {
    micrograph_id: Digest,
    kind: String,
    candidate_interpretations: u32,
    confidence_score: i64,
    policy_id: Digest,
}

/// Represents ambiguity in how a topology region can be interpreted.
///
/// `confidence_score` is Q16 for the most likely interpretation (no floats).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologyAmbiguityProfile {
    pub profile_id: Digest,
    pub micrograph_id: Digest,
    pub ambiguity_kind: AmbiguityKind,
    pub candidate_interpretations: u32,
    pub confidence_score: Q16,
    pub policy_id: Digest,
}

impl TopologyAmbiguityProfile {
    pub fn new(
        micrograph_id: Digest,
        ambiguity_kind: AmbiguityKind,
        candidate_interpretations: u32,
        confidence_score: Q16,
        policy_id: Digest,
    ) -> Self {
        let profile_id = Digest::of(&AmbiguityContent {
            micrograph_id,
            kind: format!("{:?}", ambiguity_kind),
            candidate_interpretations,
            confidence_score: confidence_score.raw(),
            policy_id,
        });
        Self {
            profile_id,
            micrograph_id,
            ambiguity_kind,
            candidate_interpretations,
            confidence_score,
            policy_id,
        }
    }

    /// Build an [`EnergyAssessment`] for this ambiguity profile.
    ///
    /// - ψ (meaning)   = `confidence_score` — how confident the most likely interpretation is.
    /// - ρ (coherence) = `Q16::ONE` — no additional coherence data at ambiguity level.
    /// - ω (phase)     = `Q16::ONE` — no phase data at ambiguity level.
    /// - `evidence_bundle_id` = `micrograph_id` — the micrograph containing this ambiguity (CROSS-006).
    pub fn energy_assessment(&self, gate: &GateResult) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.confidence_score, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: Q16::ONE,
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        EnergyAssessment::new(
            self.profile_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            self.micrograph_id,
        )
    }
}

// ─── Complement Void Hypothesis ────────────────────────────────────────────────

/// Serialize-only for ComplementVoidHypothesis content-addressing.
#[derive(Serialize)]
struct HypothesisContent {
    micrograph_id: Digest,
    hypothesized_void_kind: String,
    confidence_score: i64,
    policy_id: Digest,
}

/// A hypothesis about a complementary void implied by a micrograph region.
///
/// A complement void is a structural gap that is *implied* to exist by what
/// is absent from the observed region. `confidence_score` is Q16.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplementVoidHypothesis {
    pub hypothesis_id: Digest,
    pub micrograph_id: Digest,
    pub hypothesized_void_kind: String,
    pub confidence_score: Q16,
    pub evidence_ids: Vec<Digest>,
    pub policy_id: Digest,
}

impl ComplementVoidHypothesis {
    pub fn new(
        micrograph_id: Digest,
        void_kind: &HostVoidKind,
        confidence_score: Q16,
        mut evidence_ids: Vec<Digest>,
        policy_id: Digest,
    ) -> Self {
        evidence_ids.sort();
        let hypothesized_void_kind = format!("{:?}", void_kind);
        let hypothesis_id = Digest::of(&HypothesisContent {
            micrograph_id,
            hypothesized_void_kind: hypothesized_void_kind.clone(),
            confidence_score: confidence_score.raw(),
            policy_id,
        });
        Self {
            hypothesis_id,
            micrograph_id,
            hypothesized_void_kind,
            confidence_score,
            evidence_ids,
            policy_id,
        }
    }

    /// Build an [`EnergyAssessment`] for this void hypothesis.
    ///
    /// - ψ (meaning)   = `confidence_score` — how confident the hypothesis is (higher = more actionable).
    /// - ρ (coherence) = `Q16::ONE` — no additional coherence data at hypothesis level.
    /// - ω (phase)     = `Q16::ONE` — no phase data at hypothesis level.
    /// - `evidence_bundle_id` = first non-ZERO entry in `evidence_ids`, else `micrograph_id` (CROSS-006).
    pub fn energy_assessment(&self, gate: &GateResult) -> EnergyAssessment {
        let tripolar = TripolarEnergy::new(self.confidence_score, Q16::ONE, Q16::ONE);
        let factors = EnergyFactors {
            gate: EnergyFactors::gate_factor(gate),
            taint: Q16::ONE,
            license: EnergyFactors::license_factor(&LicenseStatus::NotApplicable),
            foundry: EnergyFactors::foundry_factor(FoundrySurvival::Unavailable),
            seam: Q16::ONE,
            contradiction: Q16::ONE,
        };
        let evidence_bundle_id = self.evidence_ids
            .iter()
            .find(|&&id| id != Digest::ZERO)
            .copied()
            .unwrap_or(self.micrograph_id);
        EnergyAssessment::new(
            self.hypothesis_id,
            EnergyKernel::new(tripolar, factors),
            self.policy_id,
            evidence_bundle_id,
        )
    }
}

// ─── Micro Topology Diagnostic ────────────────────────────────────────────────

/// Serialize-only for MicroTopologyDiagnostic content-addressing.
#[derive(Serialize)]
struct DiagnosticContent {
    micrograph_id: Digest,
    fingerprint_id: Digest,
    anomaly_count: u64,
    ambiguity_count: u64,
    hypothesis_count: u64,
    policy_id: Digest,
}

/// M2 diagnostic result for a lifted micrograph.
///
/// Contains detected anomalies, topological ambiguities, and complement
/// void hypotheses. All sub-collections are sorted by their id.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicroTopologyDiagnostic {
    pub diagnostic_id: Digest,
    pub micrograph_id: Digest,
    pub fingerprint_id: Digest,
    pub anomalies: Vec<AnomalyRecord>,
    pub ambiguities: Vec<TopologyAmbiguityProfile>,
    pub void_hypotheses: Vec<ComplementVoidHypothesis>,
    pub policy_id: Digest,
}

impl MicroTopologyDiagnostic {
    pub fn new(
        micrograph_id: Digest,
        fingerprint_id: Digest,
        mut anomalies: Vec<AnomalyRecord>,
        mut ambiguities: Vec<TopologyAmbiguityProfile>,
        mut void_hypotheses: Vec<ComplementVoidHypothesis>,
        policy_id: Digest,
    ) -> Self {
        anomalies.sort_by_key(|a| a.anomaly_id);
        ambiguities.sort_by_key(|a| a.profile_id);
        void_hypotheses.sort_by_key(|h| h.hypothesis_id);

        let diagnostic_id = Digest::of(&DiagnosticContent {
            micrograph_id,
            fingerprint_id,
            anomaly_count: anomalies.len() as u64,
            ambiguity_count: ambiguities.len() as u64,
            hypothesis_count: void_hypotheses.len() as u64,
            policy_id,
        });

        Self {
            diagnostic_id,
            micrograph_id,
            fingerprint_id,
            anomalies,
            ambiguities,
            void_hypotheses,
            policy_id,
        }
    }

    pub fn has_anomalies(&self) -> bool {
        !self.anomalies.is_empty()
    }

    pub fn has_ambiguities(&self) -> bool {
        !self.ambiguities.is_empty()
    }
}

// ─── Micro Topology Index ─────────────────────────────────────────────────────

/// Serialize-only for MicroTopologyIndex content-addressing.
#[derive(Serialize)]
struct IndexContent {
    micrograph_ids: Vec<Digest>,
    fingerprint_ids: Vec<Digest>,
    diagnostic_ids: Vec<Digest>,
    policy_id: Digest,
}

/// An append-only index of all micrographs, fingerprints, and diagnostics
/// produced in a HYPHAE run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicroTopologyIndex {
    pub index_id: Digest,
    pub micrograph_ids: Vec<Digest>,
    pub fingerprint_ids: Vec<Digest>,
    pub diagnostic_ids: Vec<Digest>,
    pub policy_id: Digest,
}

impl MicroTopologyIndex {
    pub fn empty(policy_id: Digest) -> Self {
        let index_id = Digest::of(&IndexContent {
            micrograph_ids: vec![],
            fingerprint_ids: vec![],
            diagnostic_ids: vec![],
            policy_id,
        });
        Self {
            index_id,
            micrograph_ids: vec![],
            fingerprint_ids: vec![],
            diagnostic_ids: vec![],
            policy_id,
        }
    }

    /// Append one (micrograph, fingerprint, diagnostic) triple.
    /// Entries already present are deduplicated. Non-mutating.
    pub fn add(
        &self,
        micrograph: &MetatronMicrograph,
        fingerprint: &MetatronRegionFingerprint,
        diagnostic: &MicroTopologyDiagnostic,
    ) -> Self {
        let mut m_ids = self.micrograph_ids.clone();
        let mut f_ids = self.fingerprint_ids.clone();
        let mut d_ids = self.diagnostic_ids.clone();

        if !m_ids.contains(&micrograph.micrograph_id) {
            m_ids.push(micrograph.micrograph_id);
            m_ids.sort();
        }
        if !f_ids.contains(&fingerprint.fingerprint_id) {
            f_ids.push(fingerprint.fingerprint_id);
            f_ids.sort();
        }
        if !d_ids.contains(&diagnostic.diagnostic_id) {
            d_ids.push(diagnostic.diagnostic_id);
            d_ids.sort();
        }

        let index_id = Digest::of(&IndexContent {
            micrograph_ids: m_ids.clone(),
            fingerprint_ids: f_ids.clone(),
            diagnostic_ids: d_ids.clone(),
            policy_id: self.policy_id,
        });

        Self {
            index_id,
            micrograph_ids: m_ids,
            fingerprint_ids: f_ids,
            diagnostic_ids: d_ids,
            policy_id: self.policy_id,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.micrograph_ids.len()
    }
}

// ─── M1/M2 Pipeline ───────────────────────────────────────────────────────────

/// M1 lift: extract region → project → compute semantic loss → fingerprint.
///
/// Phase 6 skeleton: no real graph traversal; structural approximation only.
/// All artifacts preserve source_evidence_id backref (CROSS-006).
pub fn lift_region(
    host_void_id: Digest,
    node_ids: Vec<Digest>,
    source_evidence_id: Digest,
    taint: TaintLabel,
    policy: &PolicyProfile,
) -> (MetatronMicrograph, MetatronRegionFingerprint, MicrographLiftReport) {
    let extraction_profile = RegionExtractionProfile::standard();
    let projection_profile = ProjectionProfile::structural(4, policy.id);

    let region_ref = TopologyRegionRef::new(
        host_void_id,
        node_ids,
        extraction_profile.max_depth,
        source_evidence_id,
        policy.id,
    );

    let total_nodes = region_ref.node_count() as u64;
    let loss_record = SemanticLossRecord::new(
        region_ref.region_id,
        vec![], // skeleton: no nodes lost
        0,      // skeleton: no edges lost
        total_nodes,
        policy.id,
    );

    // Fingerprint placeholder: hash of region_id (real fingerprinting in Phase 8+)
    let fingerprint_placeholder = Digest::of_bytes(region_ref.region_id.as_bytes());

    let micrograph = MetatronMicrograph::new(
        region_ref,
        projection_profile.profile_id,
        loss_record.loss_id,
        fingerprint_placeholder,
        source_evidence_id,
        taint,
        policy.id,
    );

    let fingerprint = MetatronRegionFingerprint::from_micrograph(&micrograph, 0);
    let report =
        MicrographLiftReport::new(&micrograph, total_nodes, 0, loss_record.loss_ratio);

    (micrograph, fingerprint, report)
}

/// M2 diagnose: analyse a lifted micrograph for anomalies, ambiguities,
/// and complement void hypotheses.
///
/// Phase 6 skeleton: generates `Boundary` ambiguity for single-node regions
/// and one `ComplementVoidHypothesis` per supplied `void_kind`.
pub fn diagnose_micrograph(
    micrograph: &MetatronMicrograph,
    fingerprint: &MetatronRegionFingerprint,
    void_kind: Option<&HostVoidKind>,
    policy: &PolicyProfile,
) -> MicroTopologyDiagnostic {
    let mut ambiguities = Vec::new();
    let mut void_hypotheses = Vec::new();

    if fingerprint.node_count < 2 {
        ambiguities.push(TopologyAmbiguityProfile::new(
            micrograph.micrograph_id,
            AmbiguityKind::Boundary,
            1,
            Q16::HALF,
            policy.id,
        ));
    }

    if let Some(kind) = void_kind {
        void_hypotheses.push(ComplementVoidHypothesis::new(
            micrograph.micrograph_id,
            kind,
            Q16::ratio(3, 4).unwrap_or(Q16::ZERO),
            vec![micrograph.source_evidence_id],
            policy.id,
        ));
    }

    MicroTopologyDiagnostic::new(
        micrograph.micrograph_id,
        fingerprint.fingerprint_id,
        vec![], // no anomalies in Phase 6 skeleton
        ambiguities,
        void_hypotheses,
        policy.id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::void_map::HostVoidKind;
    use kosmo_core::{Digest, GateResult, PolicyProfile, Q16, TaintLabel};

    fn pid() -> Digest {
        Digest::of_bytes(b"p")
    }

    fn void_id() -> Digest {
        Digest::of_bytes(b"void")
    }

    fn ev_id() -> Digest {
        Digest::of_bytes(b"ev")
    }

    fn node_ids(n: usize) -> Vec<Digest> {
        (0..n).map(|i| Digest::of_bytes(format!("node{}", i).as_bytes())).collect()
    }

    #[test]
    fn topology_region_ref_is_content_addressed() {
        let nodes = node_ids(3);
        let r1 = TopologyRegionRef::new(void_id(), nodes.clone(), 2, ev_id(), pid());
        let r2 = TopologyRegionRef::new(void_id(), nodes.clone(), 2, ev_id(), pid());
        assert_eq!(r1.region_id, r2.region_id);
        assert_ne!(r1.region_id, Digest::ZERO);
    }

    #[test]
    fn topology_region_ref_sorts_nodes() {
        let mut nodes = node_ids(3);
        let r1 = TopologyRegionRef::new(void_id(), nodes.clone(), 2, ev_id(), pid());
        nodes.reverse();
        let r2 = TopologyRegionRef::new(void_id(), nodes, 2, ev_id(), pid());
        assert_eq!(r1.region_id, r2.region_id, "node order must not affect region_id");
    }

    #[test]
    fn semantic_loss_is_zero_for_full_capture() {
        let region_id = Digest::of_bytes(b"r");
        let loss = SemanticLossRecord::new(region_id, vec![], 0, 5, pid());
        assert!(loss.is_lossless());
        assert_eq!(loss.loss_ratio, Q16::ZERO);
    }

    #[test]
    fn semantic_loss_ratio_is_integer_arithmetic() {
        let region_id = Digest::of_bytes(b"r");
        // 1 lost out of 4 total = 0.25 = Q16::ratio(1, 4)
        let lost = vec![Digest::of_bytes(b"n")];
        let loss = SemanticLossRecord::new(region_id, lost, 0, 4, pid());
        assert_eq!(loss.loss_ratio, Q16::ratio(1, 4).unwrap());
    }

    #[test]
    fn metatron_micrograph_preserves_evidence_backref() {
        let policy = PolicyProfile::default_report_only();
        let (micrograph, _, _) = lift_region(
            void_id(),
            node_ids(2),
            ev_id(),
            TaintLabel::Synthetic,
            &policy,
        );
        assert_eq!(micrograph.source_evidence_id, ev_id(), "CROSS-006: backref must be preserved");
        assert_ne!(micrograph.micrograph_id, Digest::ZERO);
    }

    #[test]
    fn lift_region_pipeline_is_deterministic() {
        let policy = PolicyProfile::default_report_only();
        let nodes = node_ids(2);
        let (m1, f1, r1) = lift_region(void_id(), nodes.clone(), ev_id(), TaintLabel::Synthetic, &policy);
        let (m2, f2, r2) = lift_region(void_id(), nodes, ev_id(), TaintLabel::Synthetic, &policy);
        assert_eq!(m1.micrograph_id, m2.micrograph_id);
        assert_eq!(f1.fingerprint_id, f2.fingerprint_id);
        assert_eq!(r1.report_id, r2.report_id);
    }

    #[test]
    fn fingerprint_from_micrograph_is_deterministic() {
        let policy = PolicyProfile::default_report_only();
        let (micrograph, _, _) =
            lift_region(void_id(), node_ids(3), ev_id(), TaintLabel::Synthetic, &policy);
        let f1 = MetatronRegionFingerprint::from_micrograph(&micrograph, 2);
        let f2 = MetatronRegionFingerprint::from_micrograph(&micrograph, 2);
        assert_eq!(f1.fingerprint_id, f2.fingerprint_id);
    }

    #[test]
    fn diagnostic_has_boundary_ambiguity_for_single_node() {
        let policy = PolicyProfile::default_report_only();
        let (micrograph, fingerprint, _) =
            lift_region(void_id(), node_ids(1), ev_id(), TaintLabel::Synthetic, &policy);
        let diag = diagnose_micrograph(&micrograph, &fingerprint, None, &policy);
        assert!(diag.has_ambiguities(), "single-node region must produce Boundary ambiguity");
        assert_eq!(diag.ambiguities[0].ambiguity_kind, AmbiguityKind::Boundary);
    }

    #[test]
    fn diagnostic_no_ambiguity_for_multi_node() {
        let policy = PolicyProfile::default_report_only();
        let (micrograph, fingerprint, _) =
            lift_region(void_id(), node_ids(3), ev_id(), TaintLabel::Synthetic, &policy);
        let diag = diagnose_micrograph(&micrograph, &fingerprint, None, &policy);
        assert!(!diag.has_ambiguities(), "multi-node region must not produce boundary ambiguity");
    }

    #[test]
    fn complement_void_hypothesis_is_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let (micrograph, fingerprint, _) =
            lift_region(void_id(), node_ids(2), ev_id(), TaintLabel::Synthetic, &policy);
        let kind = HostVoidKind::MissingTestFiber { for_module: "src/lib.rs".into() };
        let diag = diagnose_micrograph(&micrograph, &fingerprint, Some(&kind), &policy);
        assert_eq!(diag.void_hypotheses.len(), 1);
        let h1 = &diag.void_hypotheses[0];
        // Run again — same hypothesis_id
        let diag2 = diagnose_micrograph(&micrograph, &fingerprint, Some(&kind), &policy);
        assert_eq!(h1.hypothesis_id, diag2.void_hypotheses[0].hypothesis_id);
        assert_ne!(h1.hypothesis_id, Digest::ZERO);
    }

    #[test]
    fn micro_topology_index_add_is_idempotent() {
        let policy = PolicyProfile::default_report_only();
        let (micrograph, fingerprint, _) =
            lift_region(void_id(), node_ids(2), ev_id(), TaintLabel::Synthetic, &policy);
        let diag =
            diagnose_micrograph(&micrograph, &fingerprint, None, &policy);

        let idx0 = MicroTopologyIndex::empty(policy.id);
        let idx1 = idx0.add(&micrograph, &fingerprint, &diag);
        let idx2 = idx1.add(&micrograph, &fingerprint, &diag);
        assert_eq!(idx1.index_id, idx2.index_id, "re-adding same entries must be idempotent");
        assert_eq!(idx1.entry_count(), 1);
    }

    #[test]
    fn fingerprint_equality_structural_not_semantic() {
        // INVARIANT: fingerprint equality ≠ semantic equivalence.
        // Two regions can have the same node/edge count and structure_hash
        // but entirely different semantic content. This test documents the
        // invariant; the code must never assert semantic equivalence from
        // fingerprint equality.
        let policy = PolicyProfile::default_report_only();
        let (m1, f1, _) =
            lift_region(void_id(), node_ids(2), ev_id(), TaintLabel::Synthetic, &policy);
        let different_void = Digest::of_bytes(b"other-void");
        let (m2, f2, _) =
            lift_region(different_void, node_ids(2), ev_id(), TaintLabel::Synthetic, &policy);
        // Both have 2 nodes and 0 edges, but different region_ids → different structural_hash
        assert_ne!(m1.micrograph_id, m2.micrograph_id);
        assert_ne!(f1.fingerprint_id, f2.fingerprint_id);
        // (If fingerprints were equal for different regions, that would prove
        // fingerprint equality does NOT imply semantic equivalence)
    }

    #[test]
    fn region_extraction_profile_standard_is_deterministic() {
        let p1 = RegionExtractionProfile::standard();
        let p2 = RegionExtractionProfile::standard();
        assert_eq!(p1.profile_id, p2.profile_id);
        assert_ne!(p1.profile_id, Digest::ZERO);
    }

    #[test]
    fn semantic_loss_record_energy_assessment_content_addressed() {
        let region_id = Digest::of_bytes(b"r");
        let loss = SemanticLossRecord::new(region_id, vec![Digest::of_bytes(b"n")], 0, 4, pid());
        let a1 = loss.energy_assessment(&GateResult::Pass);
        let a2 = loss.energy_assessment(&GateResult::Pass);
        assert_eq!(a1.id, a2.id, "energy_assessment must be deterministic");
        assert_eq!(a1.subject_id, loss.loss_id);
        assert_eq!(a1.evidence_bundle_id, loss.region_id, "evidence must be region_id");
        assert_ne!(a1.evidence_bundle_id, Digest::ZERO, "CROSS-006: non-ZERO evidence ref");
    }

    #[test]
    fn semantic_loss_record_reject_gate_zeroes_energy() {
        let region_id = Digest::of_bytes(b"r");
        let loss = SemanticLossRecord::new(region_id, vec![Digest::of_bytes(b"n")], 0, 1, pid());
        let a = loss.energy_assessment(&GateResult::Reject { reason: "test".into() });
        assert!(a.kernel.is_zeroed(), "Reject gate must zero loss energy");
    }

    #[test]
    fn micrograph_lift_report_energy_assessment_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let (_, _, report) = lift_region(void_id(), node_ids(2), ev_id(), TaintLabel::Synthetic, &policy);
        let a1 = report.energy_assessment(&GateResult::Pass);
        let a2 = report.energy_assessment(&GateResult::Pass);
        assert_eq!(a1.id, a2.id, "energy_assessment must be deterministic");
        assert_eq!(a1.subject_id, report.report_id);
        assert_eq!(a1.evidence_bundle_id, report.micrograph_id, "evidence must be micrograph_id");
        assert_ne!(a1.evidence_bundle_id, Digest::ZERO, "CROSS-006: non-ZERO evidence ref");
    }

    #[test]
    fn micrograph_lift_report_reject_gate_zeroes_energy() {
        let policy = PolicyProfile::default_report_only();
        let (_, _, report) = lift_region(void_id(), node_ids(2), ev_id(), TaintLabel::Synthetic, &policy);
        let a = report.energy_assessment(&GateResult::Reject { reason: "test".into() });
        assert!(a.kernel.is_zeroed(), "Reject gate must zero lift report energy");
    }

    #[test]
    fn topology_ambiguity_energy_assessment_content_addressed() {
        let policy = PolicyProfile::default_report_only();
        let (micrograph, _, _) = lift_region(void_id(), node_ids(2), ev_id(), TaintLabel::Synthetic, &policy);
        let profile = TopologyAmbiguityProfile::new(
            micrograph.micrograph_id,
            AmbiguityKind::Structural,
            2,
            Q16::HALF,
            policy.id,
        );
        let a1 = profile.energy_assessment(&GateResult::Pass);
        let a2 = profile.energy_assessment(&GateResult::Pass);
        assert_eq!(a1.id, a2.id, "energy_assessment must be deterministic");
        assert_eq!(a1.subject_id, profile.profile_id);
        assert_eq!(a1.evidence_bundle_id, profile.micrograph_id, "evidence must be micrograph_id");
        assert_ne!(a1.evidence_bundle_id, Digest::ZERO, "CROSS-006: non-ZERO evidence ref");
    }

    #[test]
    fn topology_ambiguity_reject_gate_zeroes_energy() {
        let policy = PolicyProfile::default_report_only();
        let profile = TopologyAmbiguityProfile::new(
            Digest::of_bytes(b"m"),
            AmbiguityKind::Semantic,
            3,
            Q16::ONE,
            policy.id,
        );
        let a = profile.energy_assessment(&GateResult::Reject { reason: "test".into() });
        assert!(a.kernel.is_zeroed(), "Reject gate must zero ambiguity energy");
    }

    #[test]
    fn complement_void_hypothesis_energy_assessment_with_evidence() {
        let policy = PolicyProfile::default_report_only();
        let ev1 = Digest::of_bytes(b"e1");
        let hypothesis = ComplementVoidHypothesis::new(
            Digest::of_bytes(b"m"),
            &HostVoidKind::Custom { description: "gap".into() },
            Q16::HALF,
            vec![ev1],
            policy.id,
        );
        let a = hypothesis.energy_assessment(&GateResult::Pass);
        assert_eq!(a.subject_id, hypothesis.hypothesis_id);
        assert_eq!(a.evidence_bundle_id, ev1, "first non-ZERO evidence_id must be used");
        assert_ne!(a.evidence_bundle_id, Digest::ZERO, "CROSS-006: non-ZERO evidence ref");
    }

    #[test]
    fn complement_void_hypothesis_falls_back_to_micrograph_id_when_no_evidence() {
        let policy = PolicyProfile::default_report_only();
        let micrograph_id = Digest::of_bytes(b"m");
        let hypothesis = ComplementVoidHypothesis::new(
            micrograph_id,
            &HostVoidKind::Custom { description: "gap".into() },
            Q16::HALF,
            vec![],
            policy.id,
        );
        let a = hypothesis.energy_assessment(&GateResult::Pass);
        assert_eq!(a.evidence_bundle_id, micrograph_id, "fallback to micrograph_id when no evidence");
        assert_ne!(a.evidence_bundle_id, Digest::ZERO, "CROSS-006: non-ZERO evidence ref");
    }
}
