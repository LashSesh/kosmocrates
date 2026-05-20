//! MetatronGateReport — the composite gate G_meta.
//!
//! G_meta = G_nctcs ∧ G_trace ∧ G_replay ∧ G_iso ∧ G_gap ∧ G_eval ∧ G_drift
//!
//! Fail-closed: if G_meta = 0, no HolisticEigenmodeState with productive
//! status may be created. Only diagnostic reports are permitted.

use serde::{Deserialize, Serialize};

use super::isomorphic::IsomorphicProjectionReport;
use super::primitives::{content_address, EvidenceRef, Fixed, Hash256, MetatronError, NctcsClass};
use super::spectral_gap::SpectralGapStitchReport;

/// Individual gate result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_id: String,
    pub passed: bool,
    pub reason: String,
}

/// Composite MetatronGate report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetatronGateReport {
    pub gate_report_id: Hash256,
    /// G_nctcs: NCTCS conformance ≥ minimum required class.
    pub g_nctcs: GateResult,
    /// G_trace: all inputs have TraceRefs.
    pub g_trace: GateResult,
    /// G_replay: ReplayIdentity ≥ minimum threshold.
    pub g_replay: GateResult,
    /// G_iso: local monolith projections are operator-compatible with global path.
    pub g_iso: GateResult,
    /// G_gap: spectral gap is preserved or improved.
    pub g_gap: GateResult,
    /// G_eval: Eval/Validation status permits closure.
    pub g_eval: GateResult,
    /// G_drift: unexplained drift is below threshold.
    pub g_drift: GateResult,
    /// Conjunction of all individual gates.
    pub passed: bool,
    pub evidence_refs: Vec<EvidenceRef>,
}

impl MetatronGateReport {
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        nctcs_class: NctcsClass,
        min_nctcs_class: NctcsClass,
        has_all_trace_refs: bool,
        replay_identity: &Fixed,
        min_replay_identity: &Fixed,
        iso_reports: &[IsomorphicProjectionReport],
        gap_report: &SpectralGapStitchReport,
        eval_status_ok: bool,
        unexplained_drift: &Fixed,
        max_unexplained_drift: &Fixed,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Result<Self, MetatronError> {
        let g_nctcs = GateResult {
            gate_id: "G_nctcs".into(),
            passed: nctcs_class >= min_nctcs_class,
            reason: format!("NCTCS class {:?} >= min {:?}", nctcs_class, min_nctcs_class),
        };

        let g_trace = GateResult {
            gate_id: "G_trace".into(),
            passed: has_all_trace_refs,
            reason: "All inputs have TraceRefs".into(),
        };

        let g_replay = GateResult {
            gate_id: "G_replay".into(),
            passed: fixed_ge(replay_identity, min_replay_identity),
            reason: "ReplayIdentity ≥ minimum threshold".into(),
        };

        let iso_all_passed = !iso_reports.is_empty() && iso_reports.iter().all(|r| r.passed);
        let g_iso = GateResult {
            gate_id: "G_iso".into(),
            passed: iso_all_passed,
            reason: "At least one isomorphic projection report present, all passed".into(),
        };

        let g_gap = GateResult {
            gate_id: "G_gap".into(),
            passed: gap_report.improved_or_preserved,
            reason: "Spectral gap preserved or improved".into(),
        };

        let g_eval = GateResult {
            gate_id: "G_eval".into(),
            passed: eval_status_ok,
            reason: "Eval/Validation status permits closure".into(),
        };

        let g_drift = GateResult {
            gate_id: "G_drift".into(),
            passed: fixed_le(unexplained_drift, max_unexplained_drift),
            reason: "Unexplained drift below threshold".into(),
        };

        let passed = g_nctcs.passed
            && g_trace.passed
            && g_replay.passed
            && g_iso.passed
            && g_gap.passed
            && g_eval.passed
            && g_drift.passed;

        let partial = MetatronGateReport {
            gate_report_id: Hash256::zero(),
            g_nctcs,
            g_trace,
            g_replay,
            g_iso,
            g_gap,
            g_eval,
            g_drift,
            passed,
            evidence_refs,
        };
        let id = content_address(&partial)?;
        Ok(MetatronGateReport {
            gate_report_id: id,
            ..partial
        })
    }
}

fn fixed_ge(a: &Fixed, b: &Fixed) -> bool {
    use super::primitives::CanonicalNumber;
    let (CanonicalNumber::FixedI64 { raw: ra, scale: sa }, CanonicalNumber::FixedI64 { raw: rb, scale: sb }) = (a, b);
    let scale = sa.max(sb);
    ra * 10i64.pow(scale - sa) >= rb * 10i64.pow(scale - sb)
}

fn fixed_le(a: &Fixed, b: &Fixed) -> bool {
    fixed_ge(b, a)
}
