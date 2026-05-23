/*!
 * MEF-Core Pipeline - Main Interface
 *
 * Complete implementation of the MEF-Core pipeline, integrating:
 * Triton (normalization) → SpiralMemory → SolveCoagula → TICCrystallizer → MEFLedger
 */

use anyhow::Result;
use mef_ingestion::TritonCore;
use mef_ledger::MEFLedger;
use mef_solvecoagula::{SolveCoagula, SolveCoagulaConfig as SCConfig};
use mef_tic::{GateConfig as TICGateConfig, TICConfig, TICCrystallizer};
use ndarray::Array1;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::spiral_memory::SpiralMemory;

/// Main MEF-Core interface
pub struct MEFCore {
    pub seed: String,
    pub config: MEFCoreConfig,
    triton: TritonCore,
    spiral: SpiralMemory,
    solvecoagula: SolveCoagula,
    crystallizer: TICCrystallizer,
    ledger: MEFLedger,
}

/// Configuration for MEF-Core pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MEFCoreConfig {
    pub seed: String,
    #[serde(default)]
    pub spiral: SpiralConfig,
    #[serde(default)]
    pub solvecoagula: SolveCoagulaConfig,
    #[serde(default)]
    pub gate: GateConfig,
    /// Directory for TIC store (default: "mef-tic-store")
    #[serde(default = "default_tic_store")]
    pub tic_store_path: String,
    /// Directory for ledger (default: "mef-ledger")
    #[serde(default = "default_ledger_path")]
    pub ledger_path: String,
}

fn default_tic_store() -> String {
    "mef-tic-store".to_string()
}
fn default_ledger_path() -> String {
    "mef-ledger".to_string()
}

/// Spiral configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiralConfig {
    #[serde(default = "default_r")]
    pub r: f64,
    #[serde(default = "default_a")]
    pub a: f64,
    #[serde(default = "default_b")]
    pub b: f64,
    #[serde(default = "default_c")]
    pub c: f64,
    #[serde(default = "default_k")]
    pub k: i32,
    #[serde(default = "default_step")]
    pub step: f64,
}

fn default_r() -> f64 {
    1.0
}
fn default_a() -> f64 {
    0.05
}
fn default_b() -> f64 {
    0.2
}
fn default_c() -> f64 {
    0.2
}
fn default_k() -> i32 {
    2
}
fn default_step() -> f64 {
    0.01
}

impl Default for SpiralConfig {
    fn default() -> Self {
        Self {
            r: default_r(),
            a: default_a(),
            b: default_b(),
            c: default_c(),
            k: default_k(),
            step: default_step(),
        }
    }
}

/// Solve-Coagula configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveCoagulaConfig {
    #[serde(default = "default_lambda")]
    pub lambda: f64,
    #[serde(default = "default_eps")]
    pub eps: f64,
    #[serde(default = "default_max_iter")]
    pub max_iter: usize,
    #[serde(default)]
    pub operators: OperatorsConfig,
}

fn default_lambda() -> f64 {
    0.8
}
fn default_eps() -> f64 {
    1e-6
}
fn default_max_iter() -> usize {
    1000
}

impl Default for SolveCoagulaConfig {
    fn default() -> Self {
        Self {
            lambda: default_lambda(),
            eps: default_eps(),
            max_iter: default_max_iter(),
            operators: OperatorsConfig::default(),
        }
    }
}

/// Operators configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperatorsConfig {
    #[serde(default)]
    pub dk: DKConfig,
    #[serde(default)]
    pub sw: SWConfig,
    #[serde(default)]
    pub pi: PIConfig,
    #[serde(default)]
    pub wt: WTConfig,
}

/// DoubleKick configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DKConfig {
    #[serde(default = "default_alpha1")]
    pub alpha1: f64,
    #[serde(default = "default_alpha2")]
    pub alpha2: f64,
}

fn default_alpha1() -> f64 {
    0.05
}
fn default_alpha2() -> f64 {
    -0.03
}

impl Default for DKConfig {
    fn default() -> Self {
        Self {
            alpha1: default_alpha1(),
            alpha2: default_alpha2(),
        }
    }
}

/// Sweep configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SWConfig {
    #[serde(default = "default_tau0")]
    pub tau0: f64,
    #[serde(default = "default_beta")]
    pub beta: f64,
    #[serde(default = "default_schedule")]
    pub schedule: String,
}

fn default_tau0() -> f64 {
    0.5
}
fn default_beta() -> f64 {
    0.1
}
fn default_schedule() -> String {
    "cosine".to_string()
}

impl Default for SWConfig {
    fn default() -> Self {
        Self {
            tau0: default_tau0(),
            beta: default_beta(),
            schedule: default_schedule(),
        }
    }
}

/// Pfadinvarianz configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PIConfig {
    #[serde(default = "default_canon")]
    pub canon: String,
    #[serde(default = "default_tol")]
    pub tol: f64,
}

fn default_canon() -> String {
    "lexicographic".to_string()
}
fn default_tol() -> f64 {
    1e-6
}

impl Default for PIConfig {
    fn default() -> Self {
        Self {
            canon: default_canon(),
            tol: default_tol(),
        }
    }
}

/// Weight-Transfer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WTConfig {
    #[serde(default = "default_gamma")]
    pub gamma: f64,
    #[serde(default = "default_levels")]
    pub levels: Vec<String>,
}

fn default_gamma() -> f64 {
    0.1
}
fn default_levels() -> Vec<String> {
    vec!["micro".to_string(), "meso".to_string(), "macro".to_string()]
}

impl Default for WTConfig {
    fn default() -> Self {
        Self {
            gamma: default_gamma(),
            levels: default_levels(),
        }
    }
}

/// Gate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    #[serde(default = "default_por_delta")]
    pub por_delta: f64,
    #[serde(default = "default_phi_star")]
    pub phi_star: f64,
    #[serde(default = "default_mci_min")]
    pub mci_min: f64,
}

fn default_por_delta() -> f64 {
    0.02
}
fn default_phi_star() -> f64 {
    0.6
}
fn default_mci_min() -> f64 {
    0.9
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            por_delta: default_por_delta(),
            phi_star: default_phi_star(),
            mci_min: default_mci_min(),
        }
    }
}

impl Default for MEFCoreConfig {
    fn default() -> Self {
        Self::with_seed("MEF_SEED_42")
    }
}

impl MEFCoreConfig {
    pub fn with_seed(seed: &str) -> Self {
        Self {
            seed: seed.to_string(),
            spiral: SpiralConfig::default(),
            solvecoagula: SolveCoagulaConfig::default(),
            gate: GateConfig::default(),
            tic_store_path: default_tic_store(),
            ledger_path: default_ledger_path(),
        }
    }
}

/// Processing result from the MEF-Core pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub snapshot_id: String,
    pub snapshot_phase: f64,
    pub por: f64,
    pub tic_id: String,
    pub converged: bool,
    pub iterations: usize,
    pub committed: bool,
    pub block_index: Option<i32>,
    pub block_hash: Option<String>,
    /// 8D normalized vector for HNSW indexing: [x5_spiral..., psi, rho, omega]
    pub vector8: Vec<f64>,
}

impl MEFCore {
    /// Initialize MEF-Core with seed and optional configuration.
    /// Paths for TIC store and ledger are taken from config.
    pub fn new(seed: &str, config: Option<MEFCoreConfig>) -> Result<Self> {
        let config = config.unwrap_or_else(|| MEFCoreConfig::with_seed(seed));

        let mut triton_cfg: HashMap<String, Value> = HashMap::new();
        triton_cfg.insert("seed".into(), Value::String(seed.to_string()));
        let triton = TritonCore::new(triton_cfg);

        let spiral = SpiralMemory::new(config.spiral.a);

        let sc_config = SCConfig {
            lambda: config.solvecoagula.lambda,
            eps: config.solvecoagula.eps,
            max_iter: config.solvecoagula.max_iter,
            ..SCConfig::default()
        };
        let solvecoagula = SolveCoagula::new(sc_config)?;

        let tic_gate = TICGateConfig {
            por_delta: config.gate.por_delta,
            phi_star: config.gate.phi_star,
            mci_min: config.gate.mci_min,
        };
        let tic_config = TICConfig {
            gate: tic_gate,
            ..TICConfig::default()
        };
        let crystallizer = TICCrystallizer::new(tic_config, &config.tic_store_path)?;

        let ledger = MEFLedger::new(&config.ledger_path)?;

        Ok(Self {
            seed: seed.to_string(),
            config,
            triton,
            spiral,
            solvecoagula,
            crystallizer,
            ledger,
        })
    }

    /// Process data through the complete MEF-Core pipeline:
    /// Triton → SpiralMemory → SolveCoagula → TICCrystallizer → MEFLedger
    ///
    /// Returns a `ProcessingResult` including the 8D HNSW vector regardless
    /// of whether the TIC was committed to the ledger.
    pub fn process(
        &mut self,
        data: Value,
        data_type: &str,
        auto_commit: bool,
    ) -> Result<ProcessingResult> {
        // Step 1: Triton normalization → 5D vector + payload hash
        let normalized = self.triton.normalize(&data, data_type)?;
        let snapshot_id = normalized.metadata.hash[..16].to_string();

        // Step 2: SpiralMemory step — window elements from normalized payload fields
        let window_elements: Vec<String> = match &normalized.normalized_payload {
            Value::Object(map) => map.values().map(|v| v.to_string()).collect(),
            other => vec![other.to_string()],
        };
        let (_, psi_val) = self.spiral.step(&window_elements, 30);
        let por_status = if !self.spiral.memory.is_empty() { "valid" } else { "invalid" };

        // Step 3: SolveCoagula fixpoint iteration on the 5D Triton vector
        let v0 = Array1::from_vec(normalized.vector.clone());
        let (fixpoint, conv_info) = self.solvecoagula.iterate_to_fixpoint(&v0, true)?;

        // Step 4: Build convergence JSON for TICCrystallizer
        let history_json: Vec<Value> = conv_info
            .history
            .as_ref()
            .map(|h| {
                h.iter()
                    .map(|s| serde_json::json!({ "norm": s.norm, "iteration": s.iteration }))
                    .collect()
            })
            .unwrap_or_default();
        let conv_json = serde_json::json!({
            "converged": conv_info.converged,
            "iterations": conv_info.iterations,
            "history": history_json,
        });

        let snapshot_json = serde_json::json!({
            "coordinates": normalized.vector,
            "metrics": {
                "resonance": psi_val,
                "stability": conv_info.converged as u8 as f64,
                "por": por_status,
            },
        });

        // Step 5: Crystallize → TIC
        let tic = self.crystallizer.create_tic(
            &fixpoint,
            &snapshot_id,
            &self.seed,
            &conv_json,
            &snapshot_json,
        )?;

        // Step 6: Gate check + optional ledger commit
        let (committed, block_index, block_hash) =
            if auto_commit && self.crystallizer.should_commit(&tic) {
                let tic_json = serde_json::to_value(&tic)?;
                let block = self.ledger.append_block(&tic_json, &snapshot_json)?;
                (true, Some(block.index), Some(block.hash))
            } else {
                (false, None, None)
            };

        // Step 7: Build 8D HNSW vector = [fixpoint[0..5], psi, rho, omega]
        let sigma_psi = psi_val.tanh().clamp(0.0, 1.0);
        let sigma_rho = (conv_info.converged as u8 as f64 * 0.5 + 0.5).clamp(0.0, 1.0);
        let sigma_omega = tic.sigma_bar.psi.clamp(0.0, 1.0);
        let vector8 = build_vector8(&fixpoint.to_vec(), sigma_psi, sigma_rho, sigma_omega);

        Ok(ProcessingResult {
            snapshot_id,
            snapshot_phase: psi_val,
            por: psi_val,
            tic_id: tic.tic_id,
            converged: conv_info.converged,
            iterations: conv_info.iterations,
            committed,
            block_index,
            block_hash,
            vector8,
        })
    }

    pub fn get_config(&self) -> &MEFCoreConfig {
        &self.config
    }
}

/// Build a normalized 8D vector from a 5D fixpoint + 3 spectral scalars.
/// z' = [x0..x4, psi, rho, omega], ẑ = z' / ||z'||₂
fn build_vector8(x5: &[f64], psi: f64, rho: f64, omega: f64) -> Vec<f64> {
    let mut z: Vec<f64> = x5.iter().copied().collect();
    z.push(psi);
    z.push(rho);
    z.push(omega);
    let norm: f64 = z.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm > 1e-10 {
        z.iter().map(|x| x / norm).collect()
    } else {
        z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mef_core() {
        let mef = MEFCore::new("MEF_SEED_42", None).unwrap();
        assert_eq!(mef.seed, "MEF_SEED_42");
        assert_eq!(mef.config.seed, "MEF_SEED_42");
    }

    #[test]
    fn test_default_config() {
        let config = MEFCoreConfig::default();
        assert_eq!(config.seed, "MEF_SEED_42");
        assert_eq!(config.spiral.r, 1.0);
        assert_eq!(config.spiral.a, 0.05);
        assert_eq!(config.solvecoagula.lambda, 0.8);
        assert_eq!(config.gate.por_delta, 0.02);
    }

    #[test]
    fn test_process_runs_full_pipeline() {
        let mut mef = MEFCore::new("MEF_SEED_42", None).unwrap();
        let data = serde_json::json!({"text": "cognitive architecture chunking", "domain": "actr"});
        let result = mef.process(data, "json", false).unwrap();

        assert!(!result.snapshot_id.is_empty());
        assert_ne!(result.snapshot_id, "placeholder");
        assert!(!result.tic_id.is_empty());
        assert_ne!(result.tic_id, "placeholder");
        assert_eq!(result.vector8.len(), 8);

        // 8D vector should be normalized
        let norm: f64 = result.vector8.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6 || norm < 1e-10);
    }

    #[test]
    fn test_process_deterministic() {
        let mut mef1 = MEFCore::new("SEED_A", None).unwrap();
        let mut mef2 = MEFCore::new("SEED_A", None).unwrap();
        let data = serde_json::json!({"x": 42});

        let r1 = mef1.process(data.clone(), "json", false).unwrap();
        let r2 = mef2.process(data, "json", false).unwrap();

        assert_eq!(r1.snapshot_id, r2.snapshot_id);
        assert_eq!(r1.tic_id, r2.tic_id);
    }

    #[test]
    fn test_build_vector8_normalized() {
        let x5 = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let v = build_vector8(&x5, 0.3, 0.5, 0.2);
        assert_eq!(v.len(), 8);
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_serialize_config() {
        let config = MEFCoreConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("MEF_SEED_42"));
    }
}
