//! Kosmocrates core substrate types.
//!
//! Deterministic, content-addressed, fail-closed by construction.
//! Default `PolicyProfile` is `ReportOnly` with all capabilities locked.
//!
//! Layer position: kosmo-core sits directly above the existing `pse-types`
//! substrate and below all HYPHAE, Workbench, and SystemCube layers.

pub mod authority;
pub mod digest;
pub mod evidence;
pub mod fixed_point;
pub mod foundry;
pub mod parseback;
pub mod policy;
pub mod run;

pub use authority::{AuthorityLabel, Capability, CapabilityLock, LicenseStatus, TaintLabel};
pub use digest::{canonical_bytes, Digest};
pub use evidence::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};
pub use fixed_point::Q16;
pub use foundry::{
    AllowedFoundryCommand, FoundryCheckSpec, FoundryCommandPolicy, FoundryEnvironmentPolicy,
    FoundryExecutionOutcome, FoundryExecutionPlan, FoundryExecutionReport, FoundrySandboxKind,
    FoundrySandboxSpec, FoundryTimeoutPolicy, PathDigest,
};
pub use parseback::{
    ParseBackOutcome, ParseBackPlan, ParseBackReport, ParseBackScanScope, ParseBackSeverity,
    ParseBackTopologyDelta, TopologyChangeKind,
};
pub use policy::{ImplementationMode, PolicyProfile, PolicyViolation};
pub use run::{
    FoundryCheckKind, FoundryCheckResult, FoundryOutcome, GateResult, LedgerEvent,
    LedgerEventKind, RunDescriptor,
};
