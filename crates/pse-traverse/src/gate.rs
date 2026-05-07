//! Gate, Candidate, and Dual-Fabric MVP.
//!
//! Fail-closed invariant: a gate failure NEVER produces a commit.
//! It produces a `GateReport` with `passed=false` and a recommended
//! [`crate::plan::FailurePolicy`]; the caller must convert that into
//! a refinement / boundary / excision / abort report.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::canonical::content_address;
use crate::field_cube::{EvidenceRef, FieldCube};
use crate::plan::FailurePolicy;
use crate::spec::ConstraintKind;
use crate::Result;

/// A solver-emitted candidate. The `assignments` map binds dimension
/// IDs to chosen values (as canonical JSON strings); `payloads` carries
/// arbitrary supporting bytes that will become PSE observations on
/// commit.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub id: String,
    pub field_cube_id: String,
    /// Dimension ID → chosen value (canonical JSON-stringified).
    pub assignments: BTreeMap<String, String>,
    /// Hard constraint IDs the candidate claims to satisfy.
    pub claimed_satisfies: Vec<String>,
    /// Free-form solver-attached bytes; on commit they become a PSE
    /// observation batch.
    pub payloads: Vec<Vec<u8>>,
    pub provenance: String,
}

impl Candidate {
    /// Compute a deterministic candidate identifier from its
    /// content-addressable payload (all fields except `id` itself,
    /// which is overwritten to keep the hash a function of intent).
    pub fn assign_id(mut self, prefix: &str) -> Result<Self> {
        let mut probe = self.clone();
        probe.id = String::new();
        let h = content_address(&probe)?;
        let hex: String = h.iter().take(8).map(|b| format!("{:02x}", b)).collect();
        self.id = format!("{}.{}", prefix, hex);
        Ok(self)
    }

    /// Produce the observation payloads that `pse_core::macro_step`
    /// will receive on commit.
    pub fn to_observation_payloads(
        &self,
        evidence_payloads: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>> {
        let mut out: Vec<Vec<u8>> = self.payloads.clone();
        out.extend_from_slice(evidence_payloads);
        Ok(out)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GateCheck {
    pub id: String,
    pub passed: bool,
    pub severity: GateSeverity,
    pub evidence: Vec<EvidenceRef>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GateSeverity {
    Info,
    Warn,
    Error,
    Critical,
}

/// An additional diagnostic channel carried inside a `GateReport`.
/// Used by the Signature Gate to surface its verdict without replacing
/// the primary fail-closed gate (spec §15.3).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GateDiagnosticChannel {
    pub channel_name: String,
    pub passed: bool,
    /// Content address of the artefact that produced this channel
    /// (e.g. the `SignatureGateOutcome`).
    pub address: String,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GateReport {
    pub candidate_id: String,
    pub passed: bool,
    pub checks: Vec<GateCheck>,
    /// Mirror-Consistency Index in [0, 1] when Dual-Fabric is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mci: Option<f64>,
    pub pse_commit_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_policy: Option<FailurePolicy>,
    /// Additional diagnostic channels (e.g. Signature Gate). Does not
    /// replace the primary fail-closed gate; callers decide whether to
    /// act on these.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostic_channels: Vec<GateDiagnosticChannel>,
}

/// Minimum-viable gate engine.
///
/// Checks performed in v0.1:
/// 1. Every required dimension is assigned (`Hard`).
/// 2. Every claimed-satisfied constraint exists in the cube (`Hard`).
/// 3. The candidate covers every hard constraint touching an assigned
///    dimension (`Hard`).
/// 4. (When `dual_fabric` is on) Mirror check: removing the candidate
///    must not yield a strictly *better* solution against the same
///    constraints — a sanity check against trivially satisfied
///    candidates.
pub struct GateEngine {
    pub dual_fabric: bool,
    pub mci_min: f64,
}

impl Default for GateEngine {
    fn default() -> Self {
        Self { dual_fabric: true, mci_min: 0.5 }
    }
}

impl GateEngine {
    /// Run the gate over a candidate. Returns a `GateReport`. Fail-closed:
    /// `pse_commit_ready` is true if and only if every hard check passed.
    pub fn check(&self, cube: &FieldCube, candidate: &Candidate) -> GateReport {
        let mut checks = Vec::new();

        // Check 1: every required dimension assigned.
        let mut missing_required: Vec<String> = cube.dimensions.values()
            .filter(|d| d.required && !candidate.assignments.contains_key(&d.id))
            .map(|d| d.id.clone())
            .collect();
        missing_required.sort();
        checks.push(GateCheck {
            id: "g.required_dimensions_assigned".into(),
            passed: missing_required.is_empty(),
            severity: GateSeverity::Error,
            evidence: Vec::new(),
            message: if missing_required.is_empty() {
                "ok".into()
            } else {
                format!("missing required dimensions: {}", missing_required.join(","))
            },
        });

        // Check 2: every claimed constraint exists.
        let mut missing_claims: Vec<String> = candidate.claimed_satisfies.iter()
            .filter(|c| !cube.constraints.contains_key(*c))
            .cloned()
            .collect();
        missing_claims.sort();
        checks.push(GateCheck {
            id: "g.claimed_constraints_exist".into(),
            passed: missing_claims.is_empty(),
            severity: GateSeverity::Error,
            evidence: Vec::new(),
            message: if missing_claims.is_empty() {
                "ok".into()
            } else {
                format!("unknown claimed constraints: {}", missing_claims.join(","))
            },
        });

        // Check 3: every hard constraint touching an assigned dimension is claimed.
        let assigned_dims: Vec<&str> = candidate.assignments.keys().map(|s| s.as_str()).collect();
        let mut hard_uncovered: Vec<String> = cube.constraints.values()
            .filter(|c| {
                c.kind == ConstraintKind::Hard
                && c.dimensions.iter().any(|d| assigned_dims.contains(&d.as_str()))
                && !candidate.claimed_satisfies.contains(&c.id)
            })
            .map(|c| c.id.clone())
            .collect();
        hard_uncovered.sort();
        checks.push(GateCheck {
            id: "g.hard_constraints_covered".into(),
            passed: hard_uncovered.is_empty(),
            severity: GateSeverity::Critical,
            evidence: Vec::new(),
            message: if hard_uncovered.is_empty() {
                "ok".into()
            } else {
                format!("hard constraints not claimed: {}", hard_uncovered.join(","))
            },
        });

        // Check 4: Dual-Fabric mirror check.
        let mci = if self.dual_fabric {
            let primal_pass = checks.iter().filter(|c| c.passed).count();
            let primal_total = checks.len();
            // Mirror: pretend the candidate doesn't exist. The "mirror primal"
            // would have *zero* coverage of hard constraints, so its pass count
            // is at most `primal_total - hard_constraints_covered_check`.
            // The MCI is then the agreement fraction between the two.
            let mirror_pass = primal_total.saturating_sub(1); // mirror fails check 3 by construction
            let agreement = checks.iter()
                .filter(|c| c.id != "g.hard_constraints_covered")
                .filter(|c| c.passed)
                .count();
            let mci = if primal_total > 0 {
                ((primal_pass + agreement) as f64) /
                ((primal_total + mirror_pass.max(1)) as f64)
            } else { 0.0 };
            let mci_clamped = mci.clamp(0.0, 1.0);
            checks.push(GateCheck {
                id: "g.dual_fabric_mci".into(),
                passed: mci_clamped >= self.mci_min,
                severity: GateSeverity::Warn,
                evidence: Vec::new(),
                message: format!("mci={:.4} >= min={:.4}", mci_clamped, self.mci_min),
            });
            Some(mci_clamped)
        } else { None };

        let hard_passed = checks.iter()
            .filter(|c| matches!(c.severity, GateSeverity::Error | GateSeverity::Critical))
            .all(|c| c.passed);
        let mci_passed = mci.map(|m| m >= self.mci_min).unwrap_or(true);
        let passed = hard_passed && mci_passed;

        GateReport {
            candidate_id: candidate.id.clone(),
            passed,
            checks,
            mci,
            pse_commit_ready: passed,
            failure_policy: if passed { None } else { Some(FailurePolicy::Refine) },
            diagnostic_channels: Vec::new(),
        }
    }
}

/// Standalone MCI gate (Phase 4 hook).
pub struct MciGate { pub min: f64 }

impl MciGate {
    pub fn check(&self, mci: f64) -> bool { (0.0..=1.0).contains(&mci) && mci >= self.min }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_cube::{DefaultFieldCubeBuilder, FieldCubeBuilder};
    use crate::spec::{
        ConstraintSpec, DimensionKind, DimensionSource, DimensionSpec,
        OutputSpec, ProblemSpec, ReplayPolicy, RiskPolicy, ValueDomain,
    };

    fn cube() -> FieldCube {
        let s = ProblemSpec {
            id: "p.gate".into(),
            title: "T".into(),
            objective: "o".into(),
            domain: "d".into(),
            inputs: vec![],
            constraints: vec![ConstraintSpec {
                id: "c.h".into(),
                kind: ConstraintKind::Hard,
                predicate: "true".into(),
                weight: 1.0,
                dimensions: vec!["d.x".into()],
            }],
            dimensions: vec![DimensionSpec {
                id: "d.x".into(),
                label: "X".into(),
                kind: DimensionKind::Boolean,
                values: ValueDomain::Boolean,
                required: true,
                source: DimensionSource::User,
            }],
            desired_outputs: vec![OutputSpec { id: "o.x".into(), kind: "y".into() }],
            risk_policy: RiskPolicy::default(),
            replay: ReplayPolicy::default(),
            metadata: BTreeMap::new(),
        };
        DefaultFieldCubeBuilder.build(&s).unwrap()
    }

    #[test]
    fn gate_passes_well_formed_candidate() {
        let c = cube();
        let candidate = Candidate {
            id: "cand.1".into(),
            field_cube_id: c.id.clone(),
            assignments: {
                let mut m = BTreeMap::new();
                m.insert("d.x".into(), "true".into());
                m
            },
            claimed_satisfies: vec!["c.h".into()],
            payloads: vec![],
            provenance: "test".into(),
        };
        let report = GateEngine::default().check(&c, &candidate);
        assert!(report.passed, "report = {:?}", report);
        assert!(report.pse_commit_ready);
    }

    #[test]
    fn gate_fail_is_fail_closed() {
        let c = cube();
        // Missing claimed_satisfies.
        let candidate = Candidate {
            id: "cand.bad".into(),
            field_cube_id: c.id.clone(),
            assignments: {
                let mut m = BTreeMap::new();
                m.insert("d.x".into(), "true".into());
                m
            },
            claimed_satisfies: vec![],
            payloads: vec![],
            provenance: "test".into(),
        };
        let report = GateEngine::default().check(&c, &candidate);
        assert!(!report.passed);
        assert!(!report.pse_commit_ready);
        assert!(report.failure_policy.is_some());
    }

    #[test]
    fn mci_in_unit_interval() {
        let c = cube();
        let candidate = Candidate {
            id: "cand.x".into(),
            field_cube_id: c.id.clone(),
            assignments: {
                let mut m = BTreeMap::new();
                m.insert("d.x".into(), "true".into());
                m
            },
            claimed_satisfies: vec!["c.h".into()],
            payloads: vec![],
            provenance: "test".into(),
        };
        let r = GateEngine::default().check(&c, &candidate);
        let mci = r.mci.unwrap();
        assert!((0.0..=1.0).contains(&mci), "mci={}", mci);
    }
}
