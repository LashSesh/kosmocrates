use crate::{
    baselines::keyword_filter,
    datasets,
    metrics::classification::{macro_f1, PrfMetrics},
};
use pse_constitutional_interceptor::{ActionContext, ConstitutionalEvaluator, Decision};
use pse_nxalien_types::{RuleAtom, ScopeRef, Severity};
use serde::Serialize;

#[derive(Serialize, Clone, Default)]
pub struct B3Results {
    pub pse_macro_f1: f64,
    pub pse_allow: PrfMetrics,
    pub pse_warn: PrfMetrics,
    pub pse_block: PrfMetrics,
    pub keyword_macro_f1: f64,
    pub n_actions: usize,
    pub n_rules: usize,
}

pub fn run() -> B3Results {
    let ds = datasets::constitutional_dataset();

    // Build RuleAtom list from dataset
    let rules: Vec<RuleAtom> = ds
        .rules
        .iter()
        .map(|r| {
            let sev = match r.severity.as_str() {
                "Blocking" => Severity::Blocking,
                "Required" => Severity::Required,
                _ => Severity::Advisory,
            };
            RuleAtom {
                id: r.id.clone(),
                scope: ScopeRef::default(),
                trigger: r.triggers.clone(),
                prescription: format!("Ensure rule {} is satisfied.", r.id),
                severity: sev,
                evidence: vec![],
                gate: None,
                decay: None,
                hash: String::new(),
            }
        })
        .collect();

    let evaluator = ConstitutionalEvaluator::new(rules.clone());

    // Per-class TP/FP/FN for PSE
    let mut pse_tp = [0usize; 3]; // [Allow, Warn, Block]
    let mut pse_fp = [0usize; 3];
    let mut pse_fn = [0usize; 3];
    let mut kw_tp = [0usize; 2]; // keyword is binary: Allow/Block
    let mut kw_fp = [0usize; 2];
    let mut kw_fn = [0usize; 2];

    for action in &ds.actions {
        let ctx = ActionContext::new(
            action.verb.clone(),
            action.target.clone(),
            action.description.clone(),
        );
        let report = evaluator.evaluate(&ctx);
        let predicted = match &report.decision {
            Decision::Allow => "Allow",
            Decision::Block { .. } => "Block",
            Decision::Warn { .. } => "Warn",
        };
        let expected = action.expected_decision.as_str();
        update_counts(predicted, expected, &mut pse_tp, &mut pse_fp, &mut pse_fn);

        // Keyword baseline
        let kw = keyword_filter::evaluate(&action.verb, &action.target, &action.description);
        let kw_pred = if kw == keyword_filter::KeywordDecision::Block {
            "Block"
        } else {
            "Allow"
        };
        let kw_exp = if expected == "Block" {
            "Block"
        } else {
            "Allow"
        };
        if kw_pred == kw_exp {
            if kw_pred == "Block" {
                kw_tp[1] += 1;
            } else {
                kw_tp[0] += 1;
            }
        } else if kw_pred == "Block" {
            kw_fp[1] += 1;
            kw_fn[0] += 1;
        } else {
            kw_fp[0] += 1;
            kw_fn[1] += 1;
        }
    }

    let pse_allow = PrfMetrics::from_counts(pse_tp[0], pse_fp[0], pse_fn[0]);
    let pse_warn = PrfMetrics::from_counts(pse_tp[1], pse_fp[1], pse_fn[1]);
    let pse_block = PrfMetrics::from_counts(pse_tp[2], pse_fp[2], pse_fn[2]);
    let pse_macro = macro_f1(&[pse_allow.clone(), pse_warn.clone(), pse_block.clone()]);
    let kw_allow = PrfMetrics::from_counts(kw_tp[0], kw_fp[0], kw_fn[0]);
    let kw_block = PrfMetrics::from_counts(kw_tp[1], kw_fp[1], kw_fn[1]);
    let kw_macro = macro_f1(&[kw_allow, kw_block]);

    B3Results {
        pse_macro_f1: pse_macro,
        pse_allow,
        pse_warn,
        pse_block,
        keyword_macro_f1: kw_macro,
        n_actions: ds.actions.len(),
        n_rules: ds.rules.len(),
    }
}

fn class_idx(label: &str) -> Option<usize> {
    match label {
        "Allow" => Some(0),
        "Warn" => Some(1),
        "Block" => Some(2),
        _ => None,
    }
}

fn update_counts(
    pred: &str,
    expected: &str,
    tp: &mut [usize; 3],
    fp: &mut [usize; 3],
    fn_: &mut [usize; 3],
) {
    if pred == expected {
        if let Some(i) = class_idx(pred) {
            tp[i] += 1;
        }
    } else {
        if let Some(i) = class_idx(pred) {
            fp[i] += 1;
        }
        if let Some(i) = class_idx(expected) {
            fn_[i] += 1;
        }
    }
}
