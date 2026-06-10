//! GateTraceAggregator — cross-layer gate result collection and merge.
//!
//! Collects `GateResult`s from all active pipeline layers and merges them
//! using worst-wins semantics: Reject > Warn > Pass.
//!
//! The aggregated result is fail-closed: a single Reject from any layer
//! causes the entire pipeline run to report Rejected.

use kosmo_core::{Digest, GateResult};
use serde::{Deserialize, Serialize};

// ─── Layer Gate Summary ───────────────────────────────────────────────────────

/// One layer's contribution to the aggregated gate result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayerGateSummary {
    pub layer_name: String,
    pub gate_trace_id: Digest,
    pub result: GateResult,
}

// ─── Aggregated Gate Result ───────────────────────────────────────────────────

/// Content for deterministic `AggregatedGateResult` ID.
#[derive(Serialize)]
struct AggregatedContent {
    policy_id: Digest,
    layer_count: u32,
    reject_count: u32,
    warn_count: u32,
    final_result: String,
}

/// Merged gate result across all pipeline layers.
///
/// Fail-closed: any `Reject` from any layer propagates to `final_result`.
/// Layer summaries are sorted by `gate_trace_id` for determinism.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregatedGateResult {
    pub aggregate_id: Digest,
    pub policy_id: Digest,
    pub final_result: GateResult,
    pub reject_count: u32,
    pub warn_count: u32,
    /// Per-layer summaries sorted by gate_trace_id.
    pub layers: Vec<LayerGateSummary>,
}

// ─── Gate Trace Aggregator ────────────────────────────────────────────────────

/// Collects gate results from multiple layers and merges them fail-closed.
pub struct GateTraceAggregator {
    policy_id: Digest,
    layers: Vec<LayerGateSummary>,
}

impl GateTraceAggregator {
    pub fn new(policy_id: Digest) -> Self {
        Self {
            policy_id,
            layers: Vec::new(),
        }
    }

    /// Record the gate result for a named pipeline layer.
    pub fn record(
        &mut self,
        layer_name: impl Into<String>,
        gate_trace_id: Digest,
        result: GateResult,
    ) {
        self.layers.push(LayerGateSummary {
            layer_name: layer_name.into(),
            gate_trace_id,
            result,
        });
    }

    /// Consume the aggregator and produce the merged `AggregatedGateResult`.
    ///
    /// Worst-wins merge: Reject > Warn > Pass.
    pub fn aggregate(mut self) -> AggregatedGateResult {
        self.layers.sort_by_key(|l| l.gate_trace_id);

        let reject_count = self
            .layers
            .iter()
            .filter(|l| l.result.is_rejected())
            .count() as u32;
        let warn_count = self.layers.iter().filter(|l| l.result.is_warn()).count() as u32;

        let final_result = self
            .layers
            .iter()
            .fold(GateResult::Pass, |acc, l| acc.merge(l.result.clone()));

        let aggregate_id = Digest::of(&AggregatedContent {
            policy_id: self.policy_id,
            layer_count: self.layers.len() as u32,
            reject_count,
            warn_count,
            final_result: format!("{:?}", final_result),
        });

        AggregatedGateResult {
            aggregate_id,
            policy_id: self.policy_id,
            final_result,
            reject_count,
            warn_count,
            layers: self.layers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::GateResult;

    fn pid() -> Digest {
        Digest::of_bytes(b"policy")
    }

    fn tid(tag: &[u8]) -> Digest {
        Digest::of_bytes(tag)
    }

    #[test]
    fn aggregate_all_pass_gives_pass() {
        let mut agg = GateTraceAggregator::new(pid());
        agg.record("hyphae", tid(b"h"), GateResult::Pass);
        agg.record("lpcm", tid(b"l"), GateResult::Pass);
        let r = agg.aggregate();
        assert!(r.final_result.is_pass());
        assert_eq!(r.reject_count, 0);
        assert_eq!(r.warn_count, 0);
    }

    #[test]
    fn aggregate_one_reject_gives_reject() {
        let mut agg = GateTraceAggregator::new(pid());
        agg.record("hyphae", tid(b"h"), GateResult::Pass);
        agg.record(
            "lpcm",
            tid(b"l"),
            GateResult::Reject {
                reason: "bad".into(),
            },
        );
        let r = agg.aggregate();
        assert!(r.final_result.is_rejected());
        assert_eq!(r.reject_count, 1);
    }

    #[test]
    fn aggregate_warn_without_reject_gives_warn() {
        let mut agg = GateTraceAggregator::new(pid());
        agg.record(
            "hyphae",
            tid(b"h"),
            GateResult::Warn {
                message: "low".into(),
            },
        );
        agg.record("systemcube", tid(b"s"), GateResult::Pass);
        let r = agg.aggregate();
        assert!(r.final_result.is_warn());
        assert_eq!(r.warn_count, 1);
        assert_eq!(r.reject_count, 0);
    }

    #[test]
    fn aggregate_layers_sorted_by_trace_id() {
        let mut agg = GateTraceAggregator::new(pid());
        agg.record("z-layer", tid(b"zzz"), GateResult::Pass);
        agg.record("a-layer", tid(b"aaa"), GateResult::Pass);
        let r = agg.aggregate();
        let ids: Vec<Digest> = r.layers.iter().map(|l| l.gate_trace_id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "layers must be sorted by gate_trace_id");
    }

    #[test]
    fn aggregate_is_content_addressed() {
        let mut a1 = GateTraceAggregator::new(pid());
        a1.record("hyphae", tid(b"h"), GateResult::Pass);
        let mut a2 = GateTraceAggregator::new(pid());
        a2.record("hyphae", tid(b"h"), GateResult::Pass);
        assert_eq!(a1.aggregate().aggregate_id, a2.aggregate().aggregate_id);
    }

    #[test]
    fn empty_aggregator_gives_pass() {
        let agg = GateTraceAggregator::new(pid());
        let r = agg.aggregate();
        assert!(r.final_result.is_pass());
        assert_eq!(r.layers.len(), 0);
    }
}
