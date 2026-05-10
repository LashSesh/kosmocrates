//! PSE-VALIDATION-RUNNER-01 — One-Button Validation Singularity.
//!
//! Orchestrates Build, Tests, Benchmarks, Eval-Matrix-Presets, Replay,
//! Scoring, Reports and Artifact-Bundling into a single deterministic
//! `ValidationSingularityBundle`.
//!
//! Invariant: no wall-clock time appears in any content-addressed hash.
//! The runner is NOT a new cognition layer; it MUST NOT duplicate
//! Eval-Matrix logic.

pub mod bundle;
pub mod command_plan;
pub mod domain;
pub mod environment;
pub mod eval_matrix;
pub mod executor;
pub mod ledger;
pub mod nctcs;
pub mod parsers;
pub mod primitives;
pub mod profile;
pub mod replay;
pub mod report;
pub mod runner;
pub mod scoring;
pub mod snapshot;

pub use primitives::{Hash256, ValidationError, ValidationFailureKind};
pub use profile::{ValidationConformanceClass, ValidationProfile};
pub use report::{FinalValidationReport, ValidationConclusion};
pub use runner::run;
