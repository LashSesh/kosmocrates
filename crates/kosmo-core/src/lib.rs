//! Kosmocrates core substrate types.
//!
//! Deterministic, content-addressed, fail-closed by construction.
//! Default `PolicyProfile` is `ReportOnly` with all capabilities locked.
//!
//! Layer position: kosmo-core sits directly above the existing `pse-types`
//! substrate and below all HYPHAE, Workbench, and SystemCube layers.

pub mod acquisition;
pub mod authority;
pub mod cartography;
pub mod digest;
pub mod energy;
pub mod evaluation;
pub mod evidence;
pub mod fixed_point;
pub mod foundry;
pub mod kcube;
pub mod materialization;
pub mod parseback;
pub mod policy;
pub mod run;
pub mod validation;

pub use acquisition::{
    AcquiredSource, AcquisitionSandbox, AcquisitionSourceKind, AcquisitionTaint,
    LicenseCheckOutcome, SecretScanOutcome, SourceAcquisitionCapability,
};
pub use authority::{AuthorityLabel, Capability, CapabilityLock, LicenseStatus, TaintLabel};
pub use cartography::{
    CartographyEntryKind, CartographyIntegrityReport, CartographyIntegrityStatus,
    CartographyStoreCommit, CartographyStoreError, CartographyStorageManifest,
    CorpusCartographyStore, CorpusScope, InMemoryCartographyStore,
};
pub use digest::{canonical_bytes, Digest};
pub use energy::{
    rank_by_energy, EnergyAssessment, EnergyFactors, EnergyKernel, FoundrySurvival, TripolarEnergy,
};
pub use evidence::{EvidenceBundle, EvidenceKind, EvidenceRef, ReplayStatus};
pub use evaluation::{
    EvaluationCriteria, EvaluationDimension, EvaluationHarness, EvaluationMetrics,
    EvaluationOutcome, EvaluationRunReport, EvaluationScenario, EvaluationSuiteOutcome,
    EvaluationSuiteReport, StubEvaluationHarness,
};
pub use fixed_point::Q16;
pub use kcube::{
    KcubeArtifactKind, KcubeExportPolicy, KcubeManifestEntry, KcubePackage,
    KcubeRoundtripVerification, KcubeWriteOutcome, KcubeWriteReport,
};
pub use foundry::{
    AllowedFoundryCommand, FoundryCheckSpec, FoundryCommandPolicy, FoundryEnvironmentPolicy,
    FoundryExecutionOutcome, FoundryExecutionPlan, FoundryExecutionReport, FoundrySandboxKind,
    FoundrySandboxSpec, FoundryTimeoutPolicy, PathDigest,
};
pub use materialization::{
    IsolatedWorktreeSpec, MaterializationExecutionPlan, MaterializationExecutionReport,
    MaterializationOutcome, TaskApplicationOutcome, TaskApplicationResult,
    WorkbenchTaskApplication, WorkbenchTaskKind, WorktreeCleanupPolicy, WorktreeCreationMethod,
};
pub use parseback::{
    ParseBackOutcome, ParseBackPlan, ParseBackReport, ParseBackScanScope, ParseBackSeverity,
    ParseBackTopologyDelta, TopologyChangeKind,
};
pub use policy::{ImplementationMode, PolicyProfile, PolicyViolation};
pub use validation::{
    determine_closure_status, ValidationClosureReport, ValidationClosureStatus,
};
pub use run::{
    FoundryCheckKind, FoundryCheckResult, FoundryOutcome, GateResult, LedgerEvent,
    LedgerEventKind, RunDescriptor,
};
