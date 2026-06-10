use crate::digest::Digest;
use serde::{Deserialize, Serialize};

/// Implementation mode — controls what operations are permitted.
///
/// The default and mandatory starting mode is `ReportOnly`.
/// Escalation requires an explicit `PolicyProfile` and operator action.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum ImplementationMode {
    /// Scan, report, and produce diagnostics only. No host writes. No execution.
    #[default]
    ReportOnly,
    /// May execute checks in an isolated sandbox but does not write host files.
    DryRun,
    /// May write host files and materialize artifacts, but only with operator approval.
    OperatorApproved,
    /// May act within pre-approved bounds without per-action operator confirmation.
    AutonomousBounded,
}


/// Internal struct used for content-addressing `PolicyProfile`.
/// Excludes the `id` field to avoid self-reference in digest computation.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PolicyContent {
    mode: ImplementationMode,
    allow_network: bool,
    allow_external_acquisition: bool,
    allow_acquired_repo_execution: bool,
    allow_host_write: bool,
    allow_context_injection_from_external: bool,
    allow_synthetic_sourcecube: bool,
    allow_metatron_surgery_planning: bool,
    allow_lpcm_materialization: bool,
    allow_systemcube_materialization: bool,
    allow_memory_promotion: bool,
    require_foundry_for_executable_effects: bool,
    require_parseback_for_topology_changes: bool,
    require_operator_approval_for_materialization: bool,
}

/// Policy profile governing all HYPHAE / Workbench / SystemCube operations.
///
/// The `id` field is the content-addressed digest of the policy's content
/// (all fields except `id` itself). Use `PolicyProfile::default_report_only()`
/// to construct the safe default, or `PolicyProfile::new(...)` for custom profiles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyProfile {
    pub id: Digest,
    pub mode: ImplementationMode,

    pub allow_network: bool,
    pub allow_external_acquisition: bool,
    pub allow_acquired_repo_execution: bool,
    pub allow_host_write: bool,
    pub allow_context_injection_from_external: bool,

    pub allow_synthetic_sourcecube: bool,
    pub allow_metatron_surgery_planning: bool,
    pub allow_lpcm_materialization: bool,
    pub allow_systemcube_materialization: bool,
    pub allow_memory_promotion: bool,

    pub require_foundry_for_executable_effects: bool,
    pub require_parseback_for_topology_changes: bool,
    pub require_operator_approval_for_materialization: bool,
}

impl PolicyProfile {
    /// Construct the safe default: `ReportOnly`, all `allow_*` = false,
    /// all `require_*` = true. The default for the entire system.
    pub fn default_report_only() -> Self {
        Self::from_content(PolicyContent {
            mode: ImplementationMode::ReportOnly,
            allow_network: false,
            allow_external_acquisition: false,
            allow_acquired_repo_execution: false,
            allow_host_write: false,
            allow_context_injection_from_external: false,
            allow_synthetic_sourcecube: false,
            allow_metatron_surgery_planning: false,
            allow_lpcm_materialization: false,
            allow_systemcube_materialization: false,
            allow_memory_promotion: false,
            require_foundry_for_executable_effects: true,
            require_parseback_for_topology_changes: true,
            require_operator_approval_for_materialization: true,
        })
    }

    /// Construct a `DryRun` profile with no additional permissions.
    pub fn dry_run() -> Self {
        Self::from_content(PolicyContent {
            mode: ImplementationMode::DryRun,
            allow_network: false,
            allow_external_acquisition: false,
            allow_acquired_repo_execution: false,
            allow_host_write: false,
            allow_context_injection_from_external: false,
            allow_synthetic_sourcecube: false,
            allow_metatron_surgery_planning: false,
            allow_lpcm_materialization: false,
            allow_systemcube_materialization: false,
            allow_memory_promotion: false,
            require_foundry_for_executable_effects: true,
            require_parseback_for_topology_changes: true,
            require_operator_approval_for_materialization: true,
        })
    }

    /// Construct an `OperatorApproved` profile for Phase 11 materialization.
    ///
    /// Enables `allow_host_write` but keeps all `require_*` guards active.
    /// Foundry validation and parse-back topology are still mandatory.
    /// No network access, no memory promotion, no synthetic source cubes.
    pub fn operator_approved() -> Self {
        Self::from_content(PolicyContent {
            mode: ImplementationMode::OperatorApproved,
            allow_network: false,
            allow_external_acquisition: false,
            allow_acquired_repo_execution: false,
            allow_host_write: true,
            allow_context_injection_from_external: false,
            allow_synthetic_sourcecube: false,
            allow_metatron_surgery_planning: true,
            allow_lpcm_materialization: false,
            allow_systemcube_materialization: false,
            allow_memory_promotion: false,
            require_foundry_for_executable_effects: true,
            require_parseback_for_topology_changes: true,
            require_operator_approval_for_materialization: true,
        })
    }

    /// Operator-approved profile with SystemCube materialization enabled.
    ///
    /// Use when a `SystemCube` must be exported to a real `.kcube` archive on
    /// disk.  All other gates remain closed: no network, no memory promotion,
    /// no synthetic source cubes, Foundry and ParseBack still required.
    pub fn operator_approved_with_systemcube() -> Self {
        Self::from_content(PolicyContent {
            mode: ImplementationMode::OperatorApproved,
            allow_network: false,
            allow_external_acquisition: false,
            allow_acquired_repo_execution: false,
            allow_host_write: true,
            allow_context_injection_from_external: false,
            allow_synthetic_sourcecube: false,
            allow_metatron_surgery_planning: true,
            allow_lpcm_materialization: false,
            allow_systemcube_materialization: true,
            allow_memory_promotion: false,
            require_foundry_for_executable_effects: true,
            require_parseback_for_topology_changes: true,
            require_operator_approval_for_materialization: true,
        })
    }

    fn from_content(c: PolicyContent) -> Self {
        let id = Digest::of(&c);
        Self {
            id,
            mode: c.mode,
            allow_network: c.allow_network,
            allow_external_acquisition: c.allow_external_acquisition,
            allow_acquired_repo_execution: c.allow_acquired_repo_execution,
            allow_host_write: c.allow_host_write,
            allow_context_injection_from_external: c.allow_context_injection_from_external,
            allow_synthetic_sourcecube: c.allow_synthetic_sourcecube,
            allow_metatron_surgery_planning: c.allow_metatron_surgery_planning,
            allow_lpcm_materialization: c.allow_lpcm_materialization,
            allow_systemcube_materialization: c.allow_systemcube_materialization,
            allow_memory_promotion: c.allow_memory_promotion,
            require_foundry_for_executable_effects: c.require_foundry_for_executable_effects,
            require_parseback_for_topology_changes: c.require_parseback_for_topology_changes,
            require_operator_approval_for_materialization: c
                .require_operator_approval_for_materialization,
        }
    }

    fn to_content(&self) -> PolicyContent {
        PolicyContent {
            mode: self.mode.clone(),
            allow_network: self.allow_network,
            allow_external_acquisition: self.allow_external_acquisition,
            allow_acquired_repo_execution: self.allow_acquired_repo_execution,
            allow_host_write: self.allow_host_write,
            allow_context_injection_from_external: self.allow_context_injection_from_external,
            allow_synthetic_sourcecube: self.allow_synthetic_sourcecube,
            allow_metatron_surgery_planning: self.allow_metatron_surgery_planning,
            allow_lpcm_materialization: self.allow_lpcm_materialization,
            allow_systemcube_materialization: self.allow_systemcube_materialization,
            allow_memory_promotion: self.allow_memory_promotion,
            require_foundry_for_executable_effects: self.require_foundry_for_executable_effects,
            require_parseback_for_topology_changes: self.require_parseback_for_topology_changes,
            require_operator_approval_for_materialization: self
                .require_operator_approval_for_materialization,
        }
    }

    /// Verify that `id` matches the content of this profile.
    pub fn verify_id(&self) -> bool {
        self.id == Digest::of(&self.to_content())
    }

    pub fn is_report_only(&self) -> bool {
        self.mode == ImplementationMode::ReportOnly
    }

    pub fn is_at_least_dry_run(&self) -> bool {
        self.mode >= ImplementationMode::DryRun
    }

    /// Fail-closed check: reject if host write is not permitted.
    pub fn check_host_write(&self) -> Result<(), PolicyViolation> {
        if self.allow_host_write {
            Ok(())
        } else {
            Err(PolicyViolation::HostWriteDenied)
        }
    }

    /// Fail-closed check: reject if network access is not permitted.
    pub fn check_network(&self) -> Result<(), PolicyViolation> {
        if self.allow_network {
            Ok(())
        } else {
            Err(PolicyViolation::NetworkDenied)
        }
    }

    /// Fail-closed check: reject if external acquisition is not permitted.
    pub fn check_external_acquisition(&self) -> Result<(), PolicyViolation> {
        if self.allow_external_acquisition {
            Ok(())
        } else {
            Err(PolicyViolation::ExternalAcquisitionDenied)
        }
    }
}

impl Default for PolicyProfile {
    fn default() -> Self {
        Self::default_report_only()
    }
}

/// A policy enforcement failure — returned when a guarded action is attempted
/// without the required permission in the current `PolicyProfile`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PolicyViolation {
    #[error("host write denied by policy (ReportOnly or allow_host_write=false)")]
    HostWriteDenied,
    #[error("network access denied by policy")]
    NetworkDenied,
    #[error("external acquisition denied by policy")]
    ExternalAcquisitionDenied,
    #[error("acquired repository execution denied by policy")]
    AcquiredRepoExecutionDenied,
    #[error("synthetic SourceCube disabled by policy")]
    SyntheticSourceCubeDenied,
    #[error("SystemCube materialization denied by policy")]
    SystemCubeMaterializationDenied,
    #[error("memory promotion denied by policy")]
    MemoryPromotionDenied,
    #[error("operation requires operator approval")]
    OperatorApprovalRequired,
    #[error("operation requires Foundry validation")]
    FoundryRequired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_report_only() {
        let p = PolicyProfile::default_report_only();
        assert_eq!(p.mode, ImplementationMode::ReportOnly);
        assert!(p.is_report_only());
    }

    #[test]
    fn all_allow_flags_false_by_default() {
        let p = PolicyProfile::default_report_only();
        assert!(!p.allow_network);
        assert!(!p.allow_external_acquisition);
        assert!(!p.allow_acquired_repo_execution);
        assert!(!p.allow_host_write);
        assert!(!p.allow_context_injection_from_external);
        assert!(!p.allow_synthetic_sourcecube);
        assert!(!p.allow_metatron_surgery_planning);
        assert!(!p.allow_lpcm_materialization);
        assert!(!p.allow_systemcube_materialization);
        assert!(!p.allow_memory_promotion);
    }

    #[test]
    fn all_require_flags_true_by_default() {
        let p = PolicyProfile::default_report_only();
        assert!(p.require_foundry_for_executable_effects);
        assert!(p.require_parseback_for_topology_changes);
        assert!(p.require_operator_approval_for_materialization);
    }

    #[test]
    fn policy_id_deterministic() {
        let p1 = PolicyProfile::default_report_only();
        let p2 = PolicyProfile::default_report_only();
        assert_eq!(p1.id, p2.id);
    }

    #[test]
    fn policy_id_verified() {
        let p = PolicyProfile::default_report_only();
        assert!(p.verify_id());
    }

    #[test]
    fn policy_id_differs_for_different_mode() {
        let r = PolicyProfile::default_report_only();
        let d = PolicyProfile::dry_run();
        assert_ne!(r.id, d.id);
    }

    #[test]
    fn host_write_denied_by_default() {
        let p = PolicyProfile::default();
        assert!(p.check_host_write().is_err());
    }

    #[test]
    fn network_denied_by_default() {
        let p = PolicyProfile::default();
        assert!(p.check_network().is_err());
    }

    #[test]
    fn cross_001_default_is_report_only() {
        // CROSS-001: Run without policy profile defaults to ReportOnly.
        let p = PolicyProfile::default();
        assert_eq!(p.mode, ImplementationMode::ReportOnly);
    }

    #[test]
    fn cross_002_host_mutation_blocked_by_default() {
        // CROSS-002: Host mutation impossible without PolicyProfile allowing it.
        let p = PolicyProfile::default();
        assert!(p.check_host_write().is_err());
    }
}
