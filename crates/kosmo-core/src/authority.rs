use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Identifies the authoritative source of a decision, artifact, or event.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuthorityLabel {
    Human,
    Operator,
    Foundry,
    Agent { name: String },
    Unknown,
}

impl AuthorityLabel {
    pub fn is_human_or_operator(&self) -> bool {
        matches!(self, AuthorityLabel::Human | AuthorityLabel::Operator)
    }

    pub fn is_foundry(&self) -> bool {
        matches!(self, AuthorityLabel::Foundry)
    }
}

/// Taint status of an artifact.
///
/// Taint must propagate: any artifact derived from a tainted source inherits
/// at least the same taint level. Tainted artifacts cannot be promoted to
/// trusted memory without Foundry validation and operator review.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaintLabel {
    Clean,
    External,
    Synthetic,
    Unverified,
    PolicyRestricted,
    Quarantined { reason: String },
}

impl TaintLabel {
    pub fn is_clean(&self) -> bool {
        matches!(self, TaintLabel::Clean)
    }

    pub fn is_quarantined(&self) -> bool {
        matches!(self, TaintLabel::Quarantined { .. })
    }

    /// Returns the "higher" (more restrictive) of two taints.
    pub fn merge(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }
}

impl Default for TaintLabel {
    fn default() -> Self {
        TaintLabel::Unverified
    }
}

/// License status of an artifact or source.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LicenseStatus {
    Unknown,
    Permissive { spdx: String },
    Copyleft { spdx: String },
    Proprietary,
    NotApplicable,
    Unresolved,
}

impl LicenseStatus {
    pub fn is_permissive(&self) -> bool {
        matches!(self, LicenseStatus::Permissive { .. })
    }

    pub fn allows_use_by_default(&self) -> bool {
        matches!(
            self,
            LicenseStatus::Permissive { .. } | LicenseStatus::NotApplicable
        )
    }
}

impl Default for LicenseStatus {
    fn default() -> Self {
        LicenseStatus::Unknown
    }
}

/// A named capability that must be explicitly granted in `PolicyProfile`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Capability {
    NetworkAccess,
    ExternalAcquisition,
    HostWrite,
    ContextInjectionFromExternal,
    SyntheticSourceCube,
    MetatronSurgeryPlanning,
    LpcmMaterialization,
    SystemCubeMaterialization,
    MemoryPromotion,
    AcquiredRepoExecution,
}

/// Lock that restricts which capabilities are available.
///
/// When `locked = true` (default), only capabilities explicitly listed in
/// `granted` are usable. When `locked = false`, the lock is disabled
/// (use only in fully trusted, contained contexts).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityLock {
    pub locked: bool,
    pub granted: BTreeSet<Capability>,
}

impl CapabilityLock {
    /// Default: locked with no capabilities granted.
    pub fn locked_empty() -> Self {
        Self {
            locked: true,
            granted: BTreeSet::new(),
        }
    }

    /// Check whether a capability is available under this lock.
    pub fn has(&self, cap: &Capability) -> bool {
        !self.locked || self.granted.contains(cap)
    }

    /// Check whether a capability is explicitly denied.
    pub fn is_denied(&self, cap: &Capability) -> bool {
        self.locked && !self.granted.contains(cap)
    }
}

impl Default for CapabilityLock {
    fn default() -> Self {
        Self::locked_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_lock_default_denies_all() {
        let lock = CapabilityLock::default();
        assert!(lock.is_denied(&Capability::NetworkAccess));
        assert!(lock.is_denied(&Capability::HostWrite));
        assert!(lock.is_denied(&Capability::SyntheticSourceCube));
    }

    #[test]
    fn capability_lock_granted() {
        let mut lock = CapabilityLock::locked_empty();
        lock.granted.insert(Capability::NetworkAccess);
        assert!(lock.has(&Capability::NetworkAccess));
        assert!(lock.is_denied(&Capability::HostWrite));
    }

    #[test]
    fn taint_merge_max() {
        assert_eq!(
            TaintLabel::Clean.merge(TaintLabel::External),
            TaintLabel::External
        );
        assert_eq!(
            TaintLabel::Synthetic.merge(TaintLabel::External),
            TaintLabel::Synthetic
        );
    }

    #[test]
    fn taint_default_is_unverified() {
        assert_eq!(TaintLabel::default(), TaintLabel::Unverified);
    }

    #[test]
    fn license_unknown_does_not_allow_by_default() {
        assert!(!LicenseStatus::Unknown.allows_use_by_default());
        assert!(!LicenseStatus::Proprietary.allows_use_by_default());
        assert!(LicenseStatus::Permissive { spdx: "MIT".into() }.allows_use_by_default());
    }
}
