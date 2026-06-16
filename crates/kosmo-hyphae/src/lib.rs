//! Kosmocrates HYPHAE v0.3/v0.4 — run-local topology assimilation.
//!
//! Phase 3 passive run: workspace scan → void map → deficiency vector →
//! frontier graph → gate cascade → assimilation decisions.
//!
//! Phase 4 CubeSwarm MVP: source cubes → swarm workers → composite support
//! cube → host target delta (planning only).
//!
//! Phase 6 Metatron v0.4.1 M1/M2: region lift → fingerprint → diagnose.
//!
//! Phase 8 LPCM v0.4.2: fragment field → support mass → seam graph →
//! monotone contractive filter → passive DoF contraction report.
//!
//! No host file writes. All outputs are observations and decisions only.
//! Default mode: `PolicyProfile::default_report_only()`.

pub mod alchemy;
pub mod assimilation;
pub mod code_hdag;
pub mod codematrix;
pub mod collapse;
pub mod corpus;
pub mod crystal;
pub mod cube;
pub mod deficiency;
pub mod delta;
pub mod frontier;
pub mod gates;
pub mod host;
pub mod lpcm;
pub mod metatron;
pub mod motif;
pub mod norm;
pub mod norm_genome;
pub mod norm_learning;
pub mod norm_schema;
pub mod run;
pub mod structural_yield;
pub mod surgery;
pub mod swarm;
pub mod void_map;
pub mod xlang;

pub use alchemy::{combine, primitives, Element, Inventory, StructuralCounts};
pub use assimilation::{
    AssimilationDecision, AssimilationLedger, AssimilationOutcome, NegativeEvidenceRecord,
};
pub use code_hdag::{CodeHDAG, CodeObservation, HDAGEdge, HDAGEdgeKind, HDAGNode, ObservationKind};
pub use collapse::{
    CollapseAction, CollapsePlanStatus, CollapseStep, HostTargetCollapsePlan,
    MorphogenicCorpusUpdate,
};
pub use corpus::{
    CartographyPrecheck, CorpusCartography, CorpusCartographyUpdate, CorpusEntity,
    CorpusEntityKind, CorpusRelation, MotifIndex, NegativeEvidenceIndex, RelationKind,
    ReplayManifest, SourceCubeIndex,
};
pub use crystal::{
    AssimilationCertificate, CertificationStatus, Constraint, ConstraintKind, ConstraintProgram,
    DualFabricGateCascade, ReplayProof, Resonite, StructuralCrystalCandidate,
    StructuralCrystalRecord,
};
pub use cube::{CubeDimensionProfile, RepositoryCube, SourceCube};
pub use deficiency::{DeficiencyEntry, DeficiencyKind, DeficiencyVector};
pub use delta::{DeltaAction, DeltaStatus, HostTargetDelta, VoidFillDelta};
pub use frontier::{SourceEvidence, SourceFrontierGraph, SourceIntent, SourceIntentKind};
pub use gates::{GateCascade, GateCheckRecord, GateKind, GateTrace};
pub use host::{HostBinding, HostCube};
pub use lpcm::{
    monotone_contractive_filter, CandidateDirection, CandidateDirectionReason,
    DoFContractionReport, Fragment, FragmentField, FragmentKind, LocalCondensationCandidate,
    LpcmPassiveReport, MonotoneFilterOutcome, SeamEdge, SeamGraph, SupportMassVector,
};
pub use metatron::{
    diagnose_micrograph, lift_region, AmbiguityKind, AnomalyKind, AnomalyRecord,
    ComplementVoidHypothesis, ExtractionMethod, MetatronMicrograph, MetatronRegionFingerprint,
    MicroTopologyDiagnostic, MicroTopologyIndex, MicrographLiftReport, ProjectionKind,
    ProjectionProfile, RegionExtractionProfile, SemanticLossRecord, TopologyAmbiguityProfile,
    TopologyRegionRef,
};
pub use motif::MotifCandidate;
pub use norm::{FitnessObservation, NormFitnessTrace, NormGeneCandidate};
pub use norm_genome::{
    cluster_genes, compose, default_gene_threshold, relate, ComposeError, NormActivation, NormGene,
    NormRelation,
};
pub use norm_learning::{
    abstract_bundle, abstract_subject, promotable, FacetBundleObservation, NormLearningConfig,
    NormProposal,
};
pub use norm_schema::{
    Norm, NormFacetTemplate, NormInjectionSpec, NormLevel, NormOrigin, NormValidationError,
    NAME_PLACEHOLDER,
};
pub use run::{passive_run, passive_run_augmented, HyphaeRunResult};
pub use structural_yield::{StructuralYield, StructuralYieldKind};
pub use surgery::{
    SurgeryBackedCollapseStep, SurgeryEffect, SurgeryPrecondition, SurgeryRisk, SurgeryTaskStatus,
    SurgeryWorkbenchTask, TopologicalSurgeryKind, TopologicalSurgeryOption,
};
pub use swarm::{CompositeSupportCube, CubeMandorla, CubeSwarm, SourceCubeWorker, WorkerStatus};
pub use void_map::{HostVoid, HostVoidKind, TopologicalVoidMap};
pub use xlang::{
    is_test_path, symbol_sets, symbol_sets_auto, CrossLanguageFingerprint, SourceLanguage,
    SymbolSets,
};
