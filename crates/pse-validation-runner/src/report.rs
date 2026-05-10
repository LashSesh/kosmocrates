//! FinalValidationReport — JSON + Markdown output (§12).

use serde::{Deserialize, Serialize};

use crate::domain::DomainValidationSummary;
use crate::eval_matrix::EvalMatrixSummary;
use crate::parsers::{BenchmarkSummary, OpenObligation, QualitySummary};
use crate::primitives::{content_address, Hash256, StableId, ValidationError};

/// The overall validation conclusion (§9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationConclusion {
    Invalid,
    DiagnosticFinding,
    PartialImprovement,
    EmpiricalImprovement,
    ExternalValidationPending,
}

impl ValidationConclusion {
    pub fn as_str(self) -> &'static str {
        match self {
            ValidationConclusion::Invalid => "Invalid",
            ValidationConclusion::DiagnosticFinding => "DiagnosticFinding",
            ValidationConclusion::PartialImprovement => "PartialImprovement",
            ValidationConclusion::EmpiricalImprovement => "EmpiricalImprovement",
            ValidationConclusion::ExternalValidationPending => "ExternalValidationPending",
        }
    }
}

/// The canonical final report (§12.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalValidationReport {
    pub report_id: Hash256,
    pub run_id: StableId,
    pub repo_snapshot_hash: Hash256,
    pub environment_hash: Hash256,
    pub profile_hash: Hash256,
    pub quality_summary: QualitySummary,
    pub benchmark_summary: BenchmarkSummary,
    pub eval_summary_refs: Vec<Hash256>,
    pub domain_summary: Option<DomainValidationSummary>,
    pub conclusion: ValidationConclusion,
    pub open_obligations: Vec<OpenObligation>,
}

impl FinalValidationReport {
    pub fn build(
        run_id: StableId,
        repo_snapshot_hash: Hash256,
        environment_hash: Hash256,
        profile_hash: Hash256,
        quality_summary: QualitySummary,
        benchmark_summary: BenchmarkSummary,
        eval_matrix: &EvalMatrixSummary,
        domain_summary: Option<DomainValidationSummary>,
        conclusion: ValidationConclusion,
        mut open_obligations: Vec<OpenObligation>,
    ) -> Result<Self, ValidationError> {
        // Merge eval-matrix obligations.
        open_obligations.extend(eval_matrix.open_obligations.clone());
        open_obligations.sort_by(|a, b| a.obligation_id.cmp(&b.obligation_id));

        let eval_summary_refs: Vec<Hash256> = eval_matrix
            .preset_results
            .iter()
            .filter_map(|r| r.summary_ref.clone())
            .collect();

        let partial = FinalValidationReport {
            report_id: Hash256::zero(),
            run_id,
            repo_snapshot_hash,
            environment_hash,
            profile_hash,
            quality_summary,
            benchmark_summary,
            eval_summary_refs,
            domain_summary,
            conclusion,
            open_obligations,
        };
        let report_id = content_address(&partial)?;
        Ok(FinalValidationReport {
            report_id,
            ..partial
        })
    }

    pub fn content_hash(&self) -> Result<Hash256, ValidationError> {
        content_address(self)
    }

    /// Render the Markdown final report (§12.2).
    ///
    /// Sections (in order):
    /// 1. Executive Summary
    /// 2. Repository Snapshot
    /// 3. Quality Gate
    /// 4. Benchmarks
    /// 5. Eval-Matrix Results
    /// 6. Layer Marginal Utility
    /// 7. Replay and Determinism
    /// 8. Domain Validation (if present)
    /// 9. Conclusion
    /// 10. Open Obligations and Next Actions
    pub fn to_markdown(&self, eval_summary: &EvalMatrixSummary) -> String {
        let mut md = String::new();

        // 1. Executive Summary
        md.push_str("# PSE Validation Report\n\n");
        md.push_str("## 1. Executive Summary\n\n");
        md.push_str(&format!(
            "**Run ID:** `{}`  \n**Conclusion:** `{}`  \n**Report ID:** `{}`\n\n",
            self.run_id,
            self.conclusion.as_str(),
            self.report_id.hex()
        ));

        // 2. Repository Snapshot
        md.push_str("## 2. Repository Snapshot\n\n");
        md.push_str(&format!(
            "- Snapshot hash: `{}`\n- Environment hash: `{}`\n- Profile hash: `{}`\n\n",
            self.repo_snapshot_hash.hex(),
            self.environment_hash.hex(),
            self.profile_hash.hex()
        ));

        // 3. Quality Gate
        md.push_str("## 3. Quality Gate\n\n");
        let q = &self.quality_summary;
        md.push_str(&format!(
            "| Check | Result |\n|---|---|\n\
             | fmt | {} |\n\
             | clippy | {} |\n\
             | build | {} |\n\
             | test | {} |\n\
             | doc | {} |\n\n",
            pass(q.fmt_pass),
            pass(q.clippy_pass),
            pass(q.build_pass),
            pass(q.test_pass),
            pass(q.doc_pass),
        ));
        if let Some(n) = q.test_count {
            md.push_str(&format!("Tests: **{}** passed", n));
            if let Some(f) = q.failed_count {
                md.push_str(&format!(", **{}** failed", f));
            }
            md.push_str("\n\n");
        }

        // 4. Benchmarks
        md.push_str("## 4. Benchmarks\n\n");
        let b = &self.benchmark_summary;
        md.push_str(&format!(
            "- bench_full: {}\n- bench_gt: {}\n- pse_demo: {}\n\n",
            pass(b.bench_full_completed),
            pass(b.bench_gt_completed),
            pass(b.pse_demo_completed),
        ));
        if !b.metrics.is_empty() {
            md.push_str("| Metric | Value |\n|---|---|\n");
            for (k, v) in &b.metrics {
                md.push_str(&format!("| {k} | {v} |\n"));
            }
            md.push('\n');
        }

        // 5. Eval-Matrix Results
        md.push_str("## 5. Eval-Matrix Results\n\n");
        if eval_summary.preset_results.is_empty() {
            md.push_str("No presets were run.\n\n");
        } else {
            md.push_str("| Preset | Completed | Replay |\n|---|---|---|\n");
            for r in &eval_summary.preset_results {
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    r.preset_name,
                    pass(r.completed),
                    pass(r.replay_passed),
                ));
            }
            md.push('\n');
        }
        if !eval_summary.missing_presets.is_empty() {
            md.push_str("**Missing mandatory presets:**\n");
            for p in &eval_summary.missing_presets {
                md.push_str(&format!("- `{p}`\n"));
            }
            md.push('\n');
        }

        // 6. Layer Marginal Utility
        md.push_str("## 6. Layer Marginal Utility\n\n");
        md.push_str("Layer Marginal Utility analysis requires domain data. ");
        if self.domain_summary.is_some() {
            md.push_str("See Domain Validation section.\n\n");
        } else {
            md.push_str("No domain dataset provided.\n\n");
        }

        // 7. Replay and Determinism
        md.push_str("## 7. Replay and Determinism\n\n");
        let replay_ok = eval_summary.preset_results.iter().all(|r| r.replay_passed);
        md.push_str(&format!(
            "Replay identity: **{}**\n\n",
            if replay_ok { "PASS" } else { "FAIL" }
        ));

        // 8. Domain Validation
        if let Some(domain) = &self.domain_summary {
            md.push_str("## 8. Domain Validation\n\n");
            md.push_str(&format!(
                "Domain: **{}**  \nLeakage check: {}  \nCalibration: {}  \nTest: {}\n\n",
                domain.domain,
                pass(domain.leakage_check_passed),
                pass(domain.calibration_completed),
                pass(domain.test_completed),
            ));
        }

        // 9. Conclusion
        md.push_str("## 9. Conclusion\n\n");
        md.push_str(&format!(
            "**{}**\n\nReport ID (content-addressed): `{}`\n\n",
            self.conclusion.as_str(),
            self.report_id.hex()
        ));

        // 10. Open Obligations
        md.push_str("## 10. Open Obligations and Next Actions\n\n");
        if self.open_obligations.is_empty() {
            md.push_str("No open obligations.\n");
        } else {
            for ob in &self.open_obligations {
                md.push_str(&format!(
                    "### `{}`\n\n{}\n\n**Recovery:** {}\n\n",
                    ob.obligation_id, ob.description, ob.recovery
                ));
            }
        }

        md
    }
}

fn pass(ok: bool) -> &'static str {
    if ok {
        "✓ PASS"
    } else {
        "✗ FAIL"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval_matrix::{EvalMatrixSummary, PresetResult};
    use crate::parsers::QualitySummary;

    fn make_report() -> (FinalValidationReport, EvalMatrixSummary) {
        let eval = EvalMatrixSummary {
            preset_results: vec![PresetResult {
                preset_name: "agent-cognition".into(),
                completed: true,
                replay_passed: true,
                summary_ref: None,
                failure: None,
            }],
            missing_presets: vec![],
            open_obligations: vec![],
            all_required_passed: true,
        };
        let report = FinalValidationReport::build(
            "run_001".into(),
            Hash256::zero(),
            Hash256::zero(),
            Hash256::zero(),
            QualitySummary::all_pass(),
            BenchmarkSummary::skipped(),
            &eval,
            None,
            ValidationConclusion::ExternalValidationPending,
            vec![],
        )
        .unwrap();
        (report, eval)
    }

    #[test]
    fn json_report_is_content_addressed() {
        let (r, _) = make_report();
        let h1 = r.content_hash().unwrap();
        let h2 = r.content_hash().unwrap();
        assert_eq!(h1, h2);
        assert_ne!(h1, Hash256::zero());
    }

    #[test]
    fn markdown_report_contains_required_sections() {
        let (r, eval) = make_report();
        let md = r.to_markdown(&eval);
        assert!(md.contains("## 1. Executive Summary"));
        assert!(md.contains("## 2. Repository Snapshot"));
        assert!(md.contains("## 3. Quality Gate"));
        assert!(md.contains("## 4. Benchmarks"));
        assert!(md.contains("## 5. Eval-Matrix Results"));
        assert!(md.contains("## 6. Layer Marginal Utility"));
        assert!(md.contains("## 7. Replay and Determinism"));
        assert!(md.contains("## 9. Conclusion"));
        assert!(md.contains("## 10. Open Obligations"));
    }
}
