use pse_eval_matrix::agent_exoskeleton::{benchmark_scenarios, run_agent_exoskeleton_benchmark};
use pse_reasoning::{guide_beam, BeamConfig};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Serialize, Clone, Default)]
pub struct B6Results {
    pub pse_hit3: f64,
    pub baseline_hit3: f64,
    pub beam_etv_hit3: f64,
    pub pse_mrr: f64,
    pub baseline_mrr: f64,
    pub beam_etv_mrr: f64,
    pub pse_false_focus: f64,
    pub baseline_false_focus: f64,
    pub beam_etv_false_focus: f64,
    pub scenarios_with_pse_gain: usize,
    pub total_scenarios: usize,
}

pub fn run() -> B6Results {
    let report = run_agent_exoskeleton_benchmark();
    let agg = &report.aggregate;

    let beam_metrics = run_beam_baseline();

    B6Results {
        pse_hit3: agg.pse_hit_at_3_mean,
        baseline_hit3: agg.baseline_hit_at_3_mean,
        beam_etv_hit3: beam_metrics.0,
        pse_mrr: agg.pse_mrr_causal_item_mean,
        baseline_mrr: agg.baseline_mrr_causal_item_mean,
        beam_etv_mrr: beam_metrics.1,
        pse_false_focus: agg.pse_false_focus_mean,
        baseline_false_focus: agg.baseline_false_focus_mean,
        beam_etv_false_focus: beam_metrics.2,
        scenarios_with_pse_gain: agg.scenarios_with_pse_gain,
        total_scenarios: agg.scenario_count,
    }
}

/// Evaluate Beam ETV on agent relevance scenarios.
/// Returns (hit@3, mrr, false_focus_rate) averaged across all scenarios.
fn run_beam_baseline() -> (f64, f64, f64) {
    let scenarios = benchmark_scenarios();
    let beam_cfg = BeamConfig {
        beam_width: 4,
        fan_out: 3,
        max_steps: 4,
        min_amplitude: 1e-9,
        top_k_per_step: 32,
    };

    let mut hits = vec![];
    let mut mrrs = vec![];
    let mut ffs = vec![];

    for scenario in &scenarios {
        let dir = tempfile::tempdir().expect("tmpdir");
        let mut store = crate::fixtures::open_store(dir.path());

        // crystal_id_hex → work item id
        let mut hex_to_item: std::collections::HashMap<String, String> = Default::default();

        for (i, item) in scenario.work_items.iter().enumerate() {
            let crystal = crate::fixtures::build_crystal(&item.id, 0.8, 0.7);
            let text = format!("{} {}", item.source, item.text);
            let _hex = crate::fixtures::commit_crystal(&mut store, &crystal, &text, i + 1);
            let hex = hex_encode(&crystal.crystal_id);
            hex_to_item.insert(hex, item.id.clone());
        }

        let bc = guide_beam(&scenario.issue_text, &store, &beam_cfg);

        // Collect ranked item IDs in beam order (amplitude-sorted paths, depth-first)
        let mut ranked_items: Vec<String> = Vec::new();
        let mut seen = HashSet::new();
        for path in &bc.paths {
            for step in &path.steps {
                if let Some(item_id) = hex_to_item.get(&step.crystal_id_hex) {
                    if seen.insert(item_id.clone()) {
                        ranked_items.push(item_id.clone());
                    }
                }
            }
        }

        let gt = &scenario.ground_truth;
        let causal: HashSet<String> = gt
            .causal_files
            .iter()
            .chain(gt.causal_logs.iter())
            .cloned()
            .collect();
        let irrelevant: HashSet<String> = gt.irrelevant_items.iter().cloned().collect();

        hits.push(hit_at_k(&ranked_items, &causal, 3));
        mrrs.push(mrr(&ranked_items, &causal));
        ffs.push(false_focus_at_k(&ranked_items, &irrelevant, 3));
    }

    let mean = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    (mean(&hits), mean(&mrrs), mean(&ffs))
}

fn hit_at_k(ranked: &[String], causal: &HashSet<String>, k: usize) -> f64 {
    if ranked.iter().take(k).any(|id| causal.contains(id)) {
        1.0
    } else {
        0.0
    }
}

fn mrr(ranked: &[String], causal: &HashSet<String>) -> f64 {
    for (i, id) in ranked.iter().enumerate() {
        if causal.contains(id) {
            return 1.0 / (i + 1) as f64;
        }
    }
    0.0
}

fn false_focus_at_k(ranked: &[String], irrelevant: &HashSet<String>, k: usize) -> f64 {
    let top: Vec<_> = ranked.iter().take(k).collect();
    if top.is_empty() {
        return 0.0;
    }
    top.iter().filter(|&&id| irrelevant.contains(id)).count() as f64 / top.len() as f64
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
