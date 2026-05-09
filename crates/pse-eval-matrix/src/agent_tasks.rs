//! Agent / code-benchmark task classes + metric specs (§12).

use serde::{Deserialize, Serialize};

use crate::metrics::{
    AggregationPolicy, InvalidationRule, MetricDirection, MetricFamily, MetricSpec,
};

/// Task classes per §12.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AgentTaskClass {
    /// Simple code change with deterministic tests.
    SmallPatch,
    /// Refactor preserving API, behaviour, tests.
    RefactorWithConstraints,
    /// Spec → new module.
    SpecToModule,
    /// Bug localisation + minimal patch.
    BugLocalization,
    /// Multi-file feature with integration tests.
    MultiFileFeature,
    /// Document → spec.
    DocToSpec,
    /// Architecture / security / determinism audit.
    RepoAudit,
}

impl AgentTaskClass {
    /// Stable tag.
    pub fn tag(self) -> &'static str {
        match self {
            AgentTaskClass::SmallPatch => "small_patch",
            AgentTaskClass::RefactorWithConstraints => "refactor_with_constraints",
            AgentTaskClass::SpecToModule => "spec_to_module",
            AgentTaskClass::BugLocalization => "bug_localization",
            AgentTaskClass::MultiFileFeature => "multi_file_feature",
            AgentTaskClass::DocToSpec => "doc_to_spec",
            AgentTaskClass::RepoAudit => "repo_audit",
        }
    }
}

/// MetricSpec set for agent / code workloads (§12.2).
pub fn agent_metric_specs() -> Vec<MetricSpec> {
    let mk = |id: &str,
              family,
              direction,
              primary,
              agg: AggregationPolicy,
              rules: Vec<InvalidationRule>| MetricSpec {
        metric_id: id.into(),
        family,
        direction,
        primary,
        aggregation: agg,
        invalidation_rules: rules,
    };
    vec![
        mk(
            "tests_passed_ratio",
            MetricFamily::Agent,
            MetricDirection::HigherIsBetter,
            true,
            AggregationPolicy::Mean,
            vec![InvalidationRule::RequireReplayPass],
        ),
        mk(
            "compile_success",
            MetricFamily::Agent,
            MetricDirection::HigherIsBetter,
            false,
            AggregationPolicy::Mean,
            vec![],
        ),
        mk(
            "patch_minimality",
            MetricFamily::Agent,
            MetricDirection::HigherIsBetter,
            false,
            AggregationPolicy::Mean,
            vec![],
        ),
        mk(
            "regression_rate",
            MetricFamily::Agent,
            MetricDirection::LowerIsBetter,
            false,
            AggregationPolicy::Mean,
            vec![InvalidationRule::RequireReplayPass],
        ),
        mk(
            "spec_coverage",
            MetricFamily::Agent,
            MetricDirection::HigherIsBetter,
            false,
            AggregationPolicy::Mean,
            vec![],
        ),
        mk(
            "iteration_count",
            MetricFamily::Agent,
            MetricDirection::LowerIsBetter,
            false,
            AggregationPolicy::Mean,
            vec![],
        ),
        mk(
            "token_or_runtime_cost",
            MetricFamily::Agent,
            MetricDirection::LowerIsBetter,
            false,
            AggregationPolicy::Mean,
            vec![],
        ),
        mk(
            "human_repair_minutes",
            MetricFamily::Agent,
            MetricDirection::LowerIsBetter,
            false,
            AggregationPolicy::Mean,
            vec![],
        ),
        mk(
            "replayable_decision_trace",
            MetricFamily::Agent,
            MetricDirection::HigherIsBetter,
            true,
            AggregationPolicy::Mean,
            vec![InvalidationRule::RequireReplayPass],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_metric_specs_match_spec_table() {
        // §12.2 lists 9 metrics.
        assert_eq!(agent_metric_specs().len(), 9);
    }

    #[test]
    fn task_class_tag_round_trips() {
        assert_eq!(AgentTaskClass::SmallPatch.tag(), "small_patch");
    }
}
