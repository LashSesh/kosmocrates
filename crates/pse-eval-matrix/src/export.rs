//! Markdown / JSON export of the summary report (§14, §15.1 `report`).
//!
//! `report` MAY emit Markdown / JSON / PDF, but per §15.2 it MUST NOT
//! recompute metrics. These renderers only read summary fields.

use crate::primitives::{canonical_bytes, EvalError, Fixed};
use crate::reports::EvaluationSummaryReport;

/// Render a deterministic markdown summary. The output is plain ASCII;
/// no wall-clock, no platform-dependent paths.
pub fn render_markdown_summary(report: &EvaluationSummaryReport) -> String {
    let mut out = String::new();
    out.push_str("# PSE-EVAL-MATRIX-01 — Evaluation Summary\n\n");
    out.push_str(&format!("- report_id: `{}`\n", report.report_id.hex()));
    out.push_str(&format!(
        "- evaluation_spec_hash: `{}`\n",
        report.evaluation_spec_hash.hex()
    ));
    out.push_str(&format!(
        "- generated_from_ledger: `{}`\n",
        report.generated_from_ledger.hex()
    ));
    out.push_str(&format!(
        "- invalid runs: {}\n\n",
        report.invalid_runs.len()
    ));

    out.push_str("## Variant summaries\n\n");
    out.push_str(
        "| variant | trials (ok / invalid) | safety_events | task_success | false_commit_rate |\n",
    );
    out.push_str("|---|---|---|---|---|\n");
    for v in &report.variant_summaries {
        let task = v
            .metric_values
            .get("task_success")
            .map(fixed_to_str)
            .unwrap_or_else(|| "—".into());
        let fcr = v
            .metric_values
            .get("false_commit_rate")
            .map(fixed_to_str)
            .unwrap_or_else(|| "—".into());
        out.push_str(&format!(
            "| `{}` | {} / {} | {} | {} | {} |\n",
            v.variant_id, v.completed_trials, v.invalid_trials, v.safety_event_count, task, fcr
        ));
    }
    out.push('\n');

    out.push_str("## Capability profiles\n\n");
    out.push_str("| variant | task | replay | safety | cognition | SAU |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for c in &report.capability_profiles {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            c.variant_id,
            fixed_to_str(&c.task_utility),
            fixed_to_str(&c.replay_utility),
            fixed_to_str(&c.safety_utility),
            fixed_to_str(&c.cognition_utility),
            fixed_to_str(&c.safety_adjusted_utility),
        ));
    }
    out.push('\n');

    if !report.ablation_summaries.is_empty() {
        out.push_str("## Ablations\n\n");
        out.push_str("| base | ablated | LMU(task_success) | safety_regression | conclusion |\n");
        out.push_str("|---|---|---|---|---|\n");
        for a in &report.ablation_summaries {
            out.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {:?} |\n",
                a.base_variant,
                a.ablated_variant,
                fixed_to_str(&a.layer_marginal_utility),
                a.safety_regression,
                a.conclusion,
            ));
        }
        out.push('\n');
    }

    if !report.statistical_summaries.is_empty() {
        out.push_str("## Statistical summaries\n\n");
        out.push_str("| label | n | confidence | ci_low | ci_high | significant |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for s in &report.statistical_summaries {
            out.push_str(&format!(
                "| {} | {} | {}% | {} | {} | {} |\n",
                s.label,
                s.n,
                s.confidence_percent,
                fixed_to_str(&s.ci_low),
                fixed_to_str(&s.ci_high),
                s.significant
            ));
        }
        out.push('\n');
    }

    if !report.conclusion_flags.is_empty() {
        out.push_str("## Conclusion flags\n\n");
        for f in &report.conclusion_flags {
            out.push_str(&format!("- `{:?}`\n", f));
        }
    }
    out
}

/// Render the summary as canonical JSON (JCS bytes).
pub fn render_json_summary(report: &EvaluationSummaryReport) -> Result<Vec<u8>, EvalError> {
    canonical_bytes(report)
}

fn fixed_to_str(f: &Fixed) -> String {
    match f {
        Fixed::FixedI64 { raw, scale } => {
            format!("{}/{}", raw, 10i128.pow(*scale))
        }
        Fixed::Rational { num, den } => format!("{num}/{den}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::Hash256;

    #[test]
    fn markdown_summary_includes_variant_table() {
        let r = EvaluationSummaryReport {
            report_id: Hash256::zero(),
            evaluation_spec_hash: Hash256::zero(),
            generated_from_ledger: Hash256::zero(),
            variant_summaries: vec![],
            workload_summaries: vec![],
            ablation_summaries: vec![],
            capability_profiles: vec![],
            statistical_summaries: vec![],
            invalid_runs: vec![],
            conclusion_flags: vec![],
        };
        let md = render_markdown_summary(&r);
        assert!(md.contains("Variant summaries"));
        assert!(md.contains("Capability profiles"));
    }
}
