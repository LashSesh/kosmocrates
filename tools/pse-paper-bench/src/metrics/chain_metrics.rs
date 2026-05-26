use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChainMetrics {
    pub mean_d: f64,
    pub peak_d: f64,
    pub chain_length: usize,
    pub crystal_diversity: usize,
    pub grounded_fraction: f64, // fraction of steps with qtic_class >= 3
}

pub fn compute_chain_metrics(steps: &[(String, f64, u8)]) -> ChainMetrics {
    // steps: Vec<(crystal_id_hex, d_score, qtic_class)>
    if steps.is_empty() {
        return ChainMetrics {
            mean_d: 0.0,
            peak_d: 0.0,
            chain_length: 0,
            crystal_diversity: 0,
            grounded_fraction: 0.0,
        };
    }
    let d_scores: Vec<f64> = steps.iter().map(|(_, d, _)| *d).collect();
    let mean_d = d_scores.iter().sum::<f64>() / d_scores.len() as f64;
    let peak_d = d_scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let unique: HashSet<&str> = steps.iter().map(|(id, _, _)| id.as_str()).collect();
    let grounded = steps.iter().filter(|(_, _, q)| *q >= 3).count();
    ChainMetrics {
        mean_d,
        peak_d,
        chain_length: steps.len(),
        crystal_diversity: unique.len(),
        grounded_fraction: grounded as f64 / steps.len() as f64,
    }
}
