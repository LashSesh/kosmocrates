use kosmo_core::{AuthorityLabel, Digest, TaintLabel};
use serde::{Deserialize, Serialize};

/// The type of structural analysis a yield represents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructuralYieldKind {
    TopologyAnalysis,
    MotifProposal,
    DeficiencyFill,
    Custom(String),
}

/// Serialize-only for content-addressing.
#[derive(Serialize)]
struct YieldContent {
    kind: String,
    host_void_id: Option<Digest>,
    deficiency_kind_ref: Option<String>,
    motif_ids: Vec<Digest>,
    hdag_id: Option<Digest>,
    taint: String,
    authority: String,
    evidence_bundle_id: Digest,
    policy_id: Digest,
}

/// The output of processing a source candidate against a host void.
///
/// A `StructuralYield` must reference either `host_void_id` or
/// `deficiency_kind_ref` to be Workbench-usable (spec §2.2).
/// The `gate_trace_id` is set by `GateCascade::apply()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuralYield {
    pub yield_id: Digest,
    pub kind: StructuralYieldKind,
    pub host_void_id: Option<Digest>,
    pub deficiency_kind_ref: Option<String>,
    pub motif_candidate_ids: Vec<Digest>,
    pub hdag_id: Option<Digest>,
    pub taint: TaintLabel,
    pub authority: AuthorityLabel,
    pub evidence_bundle_id: Digest,
    /// Set after GateCascade is applied.
    pub gate_trace_id: Option<Digest>,
    pub policy_id: Digest,
}

impl StructuralYield {
    pub fn new(
        kind: StructuralYieldKind,
        host_void_id: Option<Digest>,
        deficiency_kind_ref: Option<String>,
        taint: TaintLabel,
        authority: AuthorityLabel,
        evidence_bundle_id: Digest,
        policy_id: Digest,
    ) -> Self {
        let yield_id = Self::compute_id_from(
            &kind, &host_void_id, &deficiency_kind_ref,
            &[], &None, &taint, &authority, &evidence_bundle_id, &policy_id,
        );
        Self {
            yield_id,
            kind,
            host_void_id,
            deficiency_kind_ref,
            motif_candidate_ids: vec![],
            hdag_id: None,
            taint,
            authority,
            evidence_bundle_id,
            gate_trace_id: None,
            policy_id,
        }
    }

    pub fn with_gate_trace(mut self, trace_id: Digest) -> Self {
        self.gate_trace_id = Some(trace_id);
        // Re-seal the yield_id after adding gate trace
        self.yield_id = Self::compute_id_from(
            &self.kind, &self.host_void_id, &self.deficiency_kind_ref,
            &self.motif_candidate_ids, &self.hdag_id, &self.taint,
            &self.authority, &self.evidence_bundle_id, &self.policy_id,
        );
        self
    }

    pub fn with_hdag(mut self, hdag_id: Digest) -> Self {
        self.hdag_id = Some(hdag_id);
        self
    }

    fn compute_id_from(
        kind: &StructuralYieldKind,
        host_void_id: &Option<Digest>,
        deficiency_kind_ref: &Option<String>,
        motif_ids: &[Digest],
        hdag_id: &Option<Digest>,
        taint: &TaintLabel,
        authority: &AuthorityLabel,
        evidence_bundle_id: &Digest,
        policy_id: &Digest,
    ) -> Digest {
        Digest::of(&YieldContent {
            kind: format!("{:?}", kind),
            host_void_id: *host_void_id,
            deficiency_kind_ref: deficiency_kind_ref.clone(),
            motif_ids: motif_ids.to_vec(),
            hdag_id: *hdag_id,
            taint: format!("{:?}", taint),
            authority: format!("{:?}", authority),
            evidence_bundle_id: *evidence_bundle_id,
            policy_id: *policy_id,
        })
    }

    /// A yield is Workbench-usable only if it references a void AND has
    /// passed through a GateCascade (gate_trace_id is set).
    pub fn is_workbench_usable(&self) -> bool {
        (self.host_void_id.is_some() || self.deficiency_kind_ref.is_some())
            && self.gate_trace_id.is_some()
    }

    /// Indicates whether this yield can be referred to in evidence.
    pub fn has_void_reference(&self) -> bool {
        self.host_void_id.is_some() || self.deficiency_kind_ref.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::{AuthorityLabel, Digest, TaintLabel};

    #[test]
    fn structural_yield_without_void_ref_is_not_workbench_usable() {
        let y = StructuralYield::new(
            StructuralYieldKind::TopologyAnalysis,
            None, // no void ref
            None, // no deficiency ref
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            Digest::ZERO,
            Digest::ZERO,
        );
        assert!(!y.has_void_reference(), "must not have void reference");
        assert!(!y.is_workbench_usable(), "must not be workbench-usable without void ref");
    }

    #[test]
    fn structural_yield_with_void_ref_not_usable_without_gate_trace() {
        let void_id = Digest::of_bytes(b"void");
        let y = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            Digest::ZERO,
            Digest::ZERO,
        );
        assert!(y.has_void_reference());
        assert!(!y.is_workbench_usable(), "gate_trace_id not yet set");
    }

    #[test]
    fn structural_yield_usable_after_gate_trace() {
        let void_id = Digest::of_bytes(b"void");
        let trace_id = Digest::of_bytes(b"trace");
        let y = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill,
            Some(void_id),
            None,
            TaintLabel::Clean,
            AuthorityLabel::Foundry,
            Digest::of_bytes(b"bundle"),
            Digest::ZERO,
        )
        .with_gate_trace(trace_id);
        assert!(y.is_workbench_usable());
    }

    #[test]
    fn structural_yield_id_deterministic() {
        let vid = Digest::of_bytes(b"v");
        let y1 = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill, Some(vid), None,
            TaintLabel::Clean, AuthorityLabel::Foundry, Digest::ZERO, Digest::ZERO,
        );
        let y2 = StructuralYield::new(
            StructuralYieldKind::DeficiencyFill, Some(vid), None,
            TaintLabel::Clean, AuthorityLabel::Foundry, Digest::ZERO, Digest::ZERO,
        );
        assert_eq!(y1.yield_id, y2.yield_id);
    }
}
