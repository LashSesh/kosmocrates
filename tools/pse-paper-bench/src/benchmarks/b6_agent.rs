use pse_eval_matrix::agent_exoskeleton::run_agent_exoskeleton_benchmark;
use serde::Serialize;

#[derive(Serialize, Clone, Default)]
pub struct B6Results {
    pub pse_hit3: f64,
    pub baseline_hit3: f64,
    pub pse_mrr: f64,
    pub baseline_mrr: f64,
    pub pse_false_focus: f64,
    pub baseline_false_focus: f64,
    pub scenarios_with_pse_gain: usize,
    pub total_scenarios: usize,
}

pub fn run() -> B6Results {
    let report = run_agent_exoskeleton_benchmark();
    let agg = &report.aggregate;
    B6Results {
        pse_hit3: agg.pse_hit_at_3_mean,
        baseline_hit3: agg.baseline_hit_at_3_mean,
        pse_mrr: agg.pse_mrr_causal_item_mean,
        baseline_mrr: agg.baseline_mrr_causal_item_mean,
        pse_false_focus: agg.pse_false_focus_mean,
        baseline_false_focus: agg.baseline_false_focus_mean,
        scenarios_with_pse_gain: agg.scenarios_with_pse_gain,
        total_scenarios: agg.scenario_count,
    }
}
