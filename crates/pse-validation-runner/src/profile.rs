//! ValidationProfile and related configuration types (§7.1, §5.2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::primitives::{content_address, Hash256, ValidationError};

/// Conformance class of the validation run (§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ValidationConformanceClass {
    /// Repository-Snapshot and Quality Gate run deterministically.
    VR0,
    /// VR0 plus Core-Benchmarks and deterministic Replay check.
    VR1,
    /// VR1 plus internal Eval-Matrix-Presets and Layer-Ablation.
    VR2,
    /// VR2 plus TPT/PhaseMatrix/Dual-Fabric presets if present.
    VR3,
    /// VR3 plus real offline domain with Calibration/Validation/Test-Split.
    VR4,
    /// VR4 plus comparative product validation and reproducible Final Dossier.
    VR5,
}

/// Quality-gate configuration (L0).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityGateConfig {
    pub run_fmt: bool,
    pub run_clippy: bool,
    pub run_build: bool,
    pub run_tests: bool,
    pub run_doc: bool,
    /// If true, continue to later phases even when L0 fails (report marked).
    pub continue_after_quality_fail: bool,
    pub extra_rustflags: Vec<String>,
}

impl Default for QualityGateConfig {
    fn default() -> Self {
        QualityGateConfig {
            run_fmt: true,
            run_clippy: true,
            run_build: true,
            run_tests: true,
            run_doc: true,
            continue_after_quality_fail: false,
            extra_rustflags: vec!["-D".into(), "warnings".into()],
        }
    }
}

/// Benchmark configuration (L1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub run_bench_full: bool,
    pub run_pse_demo: bool,
    pub run_bench_gt: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        BenchmarkConfig {
            run_bench_full: true,
            run_pse_demo: true,
            run_bench_gt: true,
        }
    }
}

/// Eval-Matrix run configuration (L2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalMatrixRunConfig {
    /// Preset names to run. Empty = use default set.
    pub presets: Vec<String>,
    /// If true, report MissingPreset for topology-tpt-mtl rather than skip.
    pub report_missing_topology_preset: bool,
}

impl Default for EvalMatrixRunConfig {
    fn default() -> Self {
        EvalMatrixRunConfig {
            presets: vec![
                "agent-cognition".into(),
                "streaming-event-detection".into(),
                "post-symbolic-ablation".into(),
                "phase-matrix-substrate".into(),
                "dual-fabric-stitch".into(),
            ],
            report_missing_topology_preset: true,
        }
    }
}

/// Domain validation configuration (L3, optional).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainValidationConfig {
    pub dataset_manifest_path: String,
    pub domain_name: String,
}

/// Timeout configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub quality_seconds: u64,
    pub benchmark_seconds: u64,
    pub eval_preset_seconds: u64,
    pub domain_seconds: u64,
    pub default_seconds: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        TimeoutConfig {
            quality_seconds: 600,
            benchmark_seconds: 300,
            eval_preset_seconds: 120,
            domain_seconds: 600,
            default_seconds: 120,
        }
    }
}

impl TimeoutConfig {
    /// CI-friendly shorter timeouts.
    pub fn ci() -> Self {
        TimeoutConfig {
            quality_seconds: 300,
            benchmark_seconds: 120,
            eval_preset_seconds: 60,
            domain_seconds: 300,
            default_seconds: 60,
        }
    }
}

/// Output policy for the artifact directory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPolicy {
    pub keep_raw_logs: bool,
    pub compress_logs: bool,
    pub emit_markdown: bool,
    pub emit_json: bool,
}

impl Default for OutputPolicy {
    fn default() -> Self {
        OutputPolicy {
            keep_raw_logs: true,
            compress_logs: false,
            emit_markdown: true,
            emit_json: true,
        }
    }
}

/// Full validation profile (§7.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationProfile {
    pub profile_id: Hash256,
    pub profile_name: String,
    pub conformance_target: ValidationConformanceClass,
    pub quality: QualityGateConfig,
    pub benchmarks: BenchmarkConfig,
    pub eval_matrix: EvalMatrixRunConfig,
    pub domain: Option<DomainValidationConfig>,
    pub timeouts: TimeoutConfig,
    pub output_policy: OutputPolicy,
    /// Extra environment variables to inject into all commands.
    pub extra_env: BTreeMap<String, String>,
}

impl ValidationProfile {
    /// Smoke profile: minimal fast run.
    pub fn smoke() -> Result<Self, ValidationError> {
        let partial = ValidationProfile {
            profile_id: Hash256::zero(),
            profile_name: "smoke".into(),
            conformance_target: ValidationConformanceClass::VR1,
            quality: QualityGateConfig {
                run_fmt: true,
                run_clippy: false,
                run_build: true,
                run_tests: true,
                run_doc: false,
                continue_after_quality_fail: false,
                extra_rustflags: vec![],
            },
            benchmarks: BenchmarkConfig {
                run_bench_full: false,
                run_pse_demo: false,
                run_bench_gt: false,
            },
            eval_matrix: EvalMatrixRunConfig {
                presets: vec!["agent-cognition".into()],
                report_missing_topology_preset: false,
            },
            domain: None,
            timeouts: TimeoutConfig::ci(),
            output_policy: OutputPolicy::default(),
            extra_env: BTreeMap::new(),
        };
        let id = content_address(&partial)?;
        Ok(ValidationProfile { profile_id: id, ..partial })
    }

    /// Structural profile: complete internal validation without real data.
    pub fn structural() -> Result<Self, ValidationError> {
        let partial = ValidationProfile {
            profile_id: Hash256::zero(),
            profile_name: "structural".into(),
            conformance_target: ValidationConformanceClass::VR3,
            quality: QualityGateConfig::default(),
            benchmarks: BenchmarkConfig {
                run_bench_full: false,
                run_pse_demo: false,
                run_bench_gt: false,
            },
            eval_matrix: EvalMatrixRunConfig::default(),
            domain: None,
            timeouts: TimeoutConfig::default(),
            output_policy: OutputPolicy::default(),
            extra_env: BTreeMap::new(),
        };
        let id = content_address(&partial)?;
        Ok(ValidationProfile { profile_id: id, ..partial })
    }

    /// Full profile: structural plus all available benchmarks.
    pub fn full() -> Result<Self, ValidationError> {
        let partial = ValidationProfile {
            profile_id: Hash256::zero(),
            profile_name: "full".into(),
            conformance_target: ValidationConformanceClass::VR3,
            quality: QualityGateConfig::default(),
            benchmarks: BenchmarkConfig::default(),
            eval_matrix: EvalMatrixRunConfig::default(),
            domain: None,
            timeouts: TimeoutConfig::default(),
            output_policy: OutputPolicy::default(),
            extra_env: BTreeMap::new(),
        };
        let id = content_address(&partial)?;
        Ok(ValidationProfile { profile_id: id, ..partial })
    }

    /// Domain profile: full plus real offline domain.
    pub fn domain(manifest_path: String, domain_name: String) -> Result<Self, ValidationError> {
        let partial = ValidationProfile {
            profile_id: Hash256::zero(),
            profile_name: "domain".into(),
            conformance_target: ValidationConformanceClass::VR4,
            quality: QualityGateConfig::default(),
            benchmarks: BenchmarkConfig::default(),
            eval_matrix: EvalMatrixRunConfig::default(),
            domain: Some(DomainValidationConfig { dataset_manifest_path: manifest_path, domain_name }),
            timeouts: TimeoutConfig::default(),
            output_policy: OutputPolicy::default(),
            extra_env: BTreeMap::new(),
        };
        let id = content_address(&partial)?;
        Ok(ValidationProfile { profile_id: id, ..partial })
    }

    /// CI profile: short timeouts, machine-readable exit code.
    pub fn ci() -> Result<Self, ValidationError> {
        let partial = ValidationProfile {
            profile_id: Hash256::zero(),
            profile_name: "ci".into(),
            conformance_target: ValidationConformanceClass::VR2,
            quality: QualityGateConfig::default(),
            benchmarks: BenchmarkConfig {
                run_bench_full: false,
                run_pse_demo: false,
                run_bench_gt: false,
            },
            eval_matrix: EvalMatrixRunConfig {
                presets: vec!["agent-cognition".into(), "streaming-event-detection".into()],
                report_missing_topology_preset: true,
            },
            domain: None,
            timeouts: TimeoutConfig::ci(),
            output_policy: OutputPolicy::default(),
            extra_env: BTreeMap::new(),
        };
        let id = content_address(&partial)?;
        Ok(ValidationProfile { profile_id: id, ..partial })
    }

    /// Build from a named profile string.
    pub fn from_name(name: &str) -> Result<Self, ValidationError> {
        match name {
            "smoke" => Self::smoke(),
            "structural" => Self::structural(),
            "full" => Self::full(),
            "ci" => Self::ci(),
            other => Err(ValidationError::CommandFailed {
                reason: format!("unknown profile: {other}"),
            }),
        }
    }

    pub fn content_hash(&self) -> Result<Hash256, ValidationError> {
        content_address(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_serializes_canonically() {
        let p = ValidationProfile::smoke().unwrap();
        let h1 = p.content_hash().unwrap();
        let h2 = p.content_hash().unwrap();
        assert_eq!(h1, h2);
        assert_ne!(h1, Hash256::zero());
    }

    #[test]
    fn profile_smoke_is_vr1() {
        let p = ValidationProfile::smoke().unwrap();
        assert_eq!(p.conformance_target, ValidationConformanceClass::VR1);
    }

    #[test]
    fn profile_structural_is_vr3() {
        let p = ValidationProfile::structural().unwrap();
        assert_eq!(p.conformance_target, ValidationConformanceClass::VR3);
    }
}
