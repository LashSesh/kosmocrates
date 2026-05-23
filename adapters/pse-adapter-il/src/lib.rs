//! PSE → Infinity Ledger bridge.
//!
//! Converts `SemanticCrystal` + provenance into IL-compatible artifacts:
//! - `ILPayload`: a JSON representation of a `CompactTic` ledger block
//! - An 8D vector: [fixpoint[0..5], ψ, ρ, ω] normalised to unit length
//!
//! `ILStore` persists those blocks on disk in IL ledger format, provides
//! cosine-similarity search, and maintains an HDAG of all committed crystals.
//!
//! ## HDAG
//! The HDAG is always active (no feature flag required).  It implements
//! Sebastian Klemm's original HDAG v1.0 specification:
//! - Nodes are 5D resonance tensors [temporal, morphic, relational, topological, entropic]
//! - Edges are phase-gradient transitions Φ_ij = (T_j − T_i) / ‖T_j − T_i‖
//! - Acyclicity emerges from the coherence potential gate, not explicit checks
//!
//! ## QTIC
//! Every crystal committed via `ILStore::commit_with_feedback` receives a
//! `QticCertificate` classifying it as Q0–Q5 per the QTIC specification
//! ("Quasi-Zeitinformationskristalle", Sebastian Klemm, 2026).  Only Q5
//! crystals satisfy the full QTIC definition (path-invariant, seam-stable,
//! replayable information attractor).
//!
//! ## Features
//! - `il-pipeline`: drive MEFCore::process() for authoritative TIC generation
//! - `full`:        il-pipeline enabled

pub mod adapter;
pub mod agent;
pub mod causal;
pub mod cluster;
pub mod constitutional;
pub mod context;
pub mod feedback;
pub mod hdag;
pub mod health;
pub mod lifecycle;
pub mod prompt;
pub mod qtic;
pub mod retrieval;
pub mod store;

pub use adapter::{text_to_vector8, CrystalAdapter, ILPayload};
pub use agent::{AgentCausalGraph, AgentLink};
pub use causal::{CausalCause, CausalGraph, CausalLink};
pub use cluster::{BridgeCrystal, ClusterConfig, ClusteringReport, KnowledgeCluster};
pub use constitutional::{
    Constitution, ConstitutionalAuditReport, ConstitutionalFeedback, ConstitutionalRule,
    ConstitutionalReport, RulePredicate, RuleVerdict, Severity,
};
pub use context::{ContextBudget, CrystalSummary};
pub use health::{crystal_uncertainty, CrystalHealthMetrics, MemoryHealthReport};
pub use lifecycle::{
    classify_lifecycle, ConsolidationCandidate, ConsolidationReason, CrystalLifecycle,
    DecayModel, LifecycleReport, LifecycleStatus,
};
pub use prompt::{GroundedPrompt, PromptConfig};
pub use feedback::{refine_crystal, ValidationFeedback};
pub use hdag::{crystal_to_tensor, HDAGEdge, PathInvarianceResult, ResonanceTensor, HDAG};
pub use qtic::{classify as classify_qtic, mirror_consistency_index, QticCertificate, QticClass,
               QticInput, MCI_THRESHOLD};
pub use retrieval::{
    CausalRetrievalConfig, CausalRetrievalResult, CausalRole, CausallyGroundedEntry,
};
pub use store::{ILMatch, ILStore};
