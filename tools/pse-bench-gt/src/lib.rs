//! Ground-truth precision/recall benchmark suite for the Post-Symbolic Engine.
//!
//! This crate provides the *measurement apparatus* for Research Strand A.
//! It is independent of any domain adapter or detector: a detector produces
//! a stream of [`Detection`]s, a scenario carries a stream of
//! [`GroundTruthEvent`]s, and [`score_detections`] turns the pair into a
//! [`Metrics`] record with precision, recall, F1 and AUPRC.
//!
//! The scorer is the pure-math core of the suite. Domain scenarios,
//! PSE/baseline runners, and CLI bench binaries are added in later
//! increments and consume this module unchanged.

use serde::{Deserialize, Serialize};

pub mod runner;
pub mod scenarios;

// ─── Ground-Truth Event ──────────────────────────────────────────────────────

/// A canonical ground-truth event, normalized across adapters.
///
/// `start_index` is inclusive, `end_index` is exclusive. Indices are in the
/// natural tick space of the scenario (e.g. observation ordinals for seismo,
/// interleaved sample indices for vitals, candle ordinals for binance).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroundTruthEvent {
    /// Inclusive start tick.
    pub start_index: u64,
    /// Exclusive end tick.
    pub end_index: u64,
    /// Semantic label, e.g. `"mainshock"` or `"volatility_regime_shift"`.
    pub label: String,
    /// Domain-specific severity, used as a tie-breaker in AUPRC when two
    /// ground-truth events compete for the same detection.
    pub severity: f64,
}

impl GroundTruthEvent {
    /// True iff `tick` falls in this event's half-open interval.
    pub fn contains(&self, tick: u64) -> bool {
        tick >= self.start_index && tick < self.end_index
    }

    /// Half-open interval length in ticks.
    pub fn len(&self) -> u64 {
        self.end_index.saturating_sub(self.start_index)
    }

    /// True iff the interval is empty.
    pub fn is_empty(&self) -> bool {
        self.end_index <= self.start_index
    }
}

// ─── Detection ───────────────────────────────────────────────────────────────

/// A single detector output: *"something happened at tick T with score S"*.
///
/// `source` is a free-form string describing what produced the detection
/// ("pse_crystal", "pse_memory_hit", "isoforest", "stl_zscore"). It lets
/// aggregate reporting split metrics by detector channel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    /// Tick at which the detection fired.
    pub tick: u64,
    /// Detection score in [0, 1]; higher = more confident.
    pub score: f64,
    /// Free-form source identifier.
    pub source: String,
}

impl Detection {
    pub fn new(tick: u64, score: f64, source: impl Into<String>) -> Self {
        Self { tick, score, source: source.into() }
    }
}

// ─── Metrics ─────────────────────────────────────────────────────────────────

/// Precision/recall/F1/AUPRC for a detection-vs-ground-truth comparison.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Metrics {
    /// True positives: ground-truth events matched by at least one detection.
    pub tp: u64,
    /// False positives: detections that match no ground-truth event.
    pub fp: u64,
    /// False negatives: ground-truth events not matched by any detection.
    pub fn_: u64,
    /// TP / (TP + FP); 0 when there are no detections.
    pub precision: f64,
    /// TP / (TP + FN); 0 when there are no ground-truth events.
    pub recall: f64,
    /// 2 * P * R / (P + R); 0 when P = R = 0.
    pub f1: f64,
    /// Area under the precision/recall curve, trapezoidal. `None` when
    /// detections carry no score variance (constant-score input → undefined).
    pub auprc: Option<f64>,
}

// ─── Scoring ─────────────────────────────────────────────────────────────────

/// Match detections to ground-truth events and return aggregate metrics.
///
/// Matching semantics:
///  - A detection at tick `T` matches a ground-truth event `E` iff `T` lies
///    in `[E.start_index − tolerance_ticks, E.end_index + tolerance_ticks)`.
///  - Each GT event may consume **at most one** detection (TP = 1 per event).
///    Extra detections in the same window become FPs so detector "spam"
///    does not inflate precision.
///  - Within a GT window, the highest-score detection is picked as the match
///    (greedy by score); the rest are FPs.
///  - Unmatched GT events are FNs; detections not covered by any GT window
///    are FPs.
///
/// The AUPRC is computed by sweeping a threshold over detection scores from
/// high to low and evaluating (precision, recall) at each unique cut; areas
/// are summed trapezoidally. If all detection scores are identical, AUPRC
/// returns `None` (undefined — a detector with no discrimination gives no
/// meaningful curve).
pub fn score_detections(
    ground_truth: &[GroundTruthEvent],
    detections: &[Detection],
    tolerance_ticks: u64,
) -> Metrics {
    // ---- Greedy matching at the native tolerance, no score filter. ----
    let (tp, fp, fn_) = match_at_threshold(ground_truth, detections, tolerance_ticks, f64::NEG_INFINITY);
    let precision = if tp + fp == 0 { 0.0 } else { tp as f64 / (tp + fp) as f64 };
    let recall = if tp + fn_ == 0 { 0.0 } else { tp as f64 / (tp + fn_) as f64 };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    let auprc = compute_auprc(ground_truth, detections, tolerance_ticks);
    Metrics { tp, fp, fn_, precision, recall, f1, auprc }
}

/// Greedy TP/FP/FN count at a given minimum score.
fn match_at_threshold(
    ground_truth: &[GroundTruthEvent],
    detections: &[Detection],
    tolerance_ticks: u64,
    min_score: f64,
) -> (u64, u64, u64) {
    // Filter detections by score threshold.
    let active: Vec<&Detection> = detections
        .iter()
        .filter(|d| d.score >= min_score)
        .collect();

    // For each GT event, find the highest-score detection that falls in its
    // extended window. That detection is "consumed" by this match.
    let mut consumed = vec![false; active.len()];
    let mut tp = 0u64;
    let mut fn_ = 0u64;

    for ev in ground_truth {
        let lo = ev.start_index.saturating_sub(tolerance_ticks);
        let hi = ev.end_index.saturating_add(tolerance_ticks);
        // Pick the highest-score non-consumed detection in [lo, hi).
        let mut best: Option<(usize, f64)> = None;
        for (i, d) in active.iter().enumerate() {
            if consumed[i] { continue; }
            if d.tick < lo || d.tick >= hi { continue; }
            match best {
                None => best = Some((i, d.score)),
                Some((_, s)) if d.score > s => best = Some((i, d.score)),
                _ => {}
            }
        }
        match best {
            Some((i, _)) => { consumed[i] = true; tp += 1; }
            None => { fn_ += 1; }
        }
    }

    // Every unconsumed detection is a false positive.
    let fp = consumed.iter().filter(|c| !**c).count() as u64;
    (tp, fp, fn_)
}

/// Trapezoidal AUPRC by sweeping thresholds over unique detection scores.
fn compute_auprc(
    ground_truth: &[GroundTruthEvent],
    detections: &[Detection],
    tolerance_ticks: u64,
) -> Option<f64> {
    if ground_truth.is_empty() || detections.is_empty() {
        return Some(0.0);
    }

    // Unique descending scores. If there is no variance, AUPRC is undefined.
    let mut scores: Vec<f64> = detections.iter().map(|d| d.score).collect();
    scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    scores.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    if scores.len() < 2 {
        return None;
    }

    // PR curve points: start at (recall=0, precision=1) by convention.
    let mut points: Vec<(f64, f64)> = Vec::with_capacity(scores.len() + 1);
    points.push((0.0, 1.0));
    for &s in &scores {
        let (tp, fp, fn_) = match_at_threshold(ground_truth, detections, tolerance_ticks, s);
        let precision = if tp + fp == 0 { 1.0 } else { tp as f64 / (tp + fp) as f64 };
        let recall = if tp + fn_ == 0 { 0.0 } else { tp as f64 / (tp + fn_) as f64 };
        points.push((recall, precision));
    }

    // Sort by recall ascending (trapezoidal requires monotone x).
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Trapezoidal integration.
    let mut area = 0.0;
    for pair in points.windows(2) {
        let (r0, p0) = pair[0];
        let (r1, p1) = pair[1];
        area += (r1 - r0) * (p0 + p1) * 0.5;
    }
    Some(area.clamp(0.0, 1.0))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn gt(start: u64, end: u64, label: &str) -> GroundTruthEvent {
        GroundTruthEvent {
            start_index: start,
            end_index: end,
            label: label.into(),
            severity: 1.0,
        }
    }

    fn det(tick: u64, score: f64) -> Detection {
        Detection::new(tick, score, "test")
    }

    #[test]
    fn empty_inputs_give_zero_metrics() {
        let m = score_detections(&[], &[], 0);
        assert_eq!(m.tp, 0);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 0);
        assert_eq!(m.precision, 0.0);
        assert_eq!(m.recall, 0.0);
        assert_eq!(m.f1, 0.0);
    }

    #[test]
    fn gt_without_detections_is_all_false_negatives() {
        let gts = vec![gt(10, 20, "a"), gt(30, 40, "b")];
        let m = score_detections(&gts, &[], 0);
        assert_eq!(m.tp, 0);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 2);
        assert_eq!(m.recall, 0.0);
        assert_eq!(m.precision, 0.0);
    }

    #[test]
    fn detections_without_gt_are_all_false_positives() {
        let dets = vec![det(5, 0.5), det(15, 0.7)];
        let m = score_detections(&[], &dets, 0);
        assert_eq!(m.tp, 0);
        assert_eq!(m.fp, 2);
        assert_eq!(m.fn_, 0);
    }

    #[test]
    fn exact_hit_inside_window_is_tp() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(15, 0.9)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 1);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 0);
        assert_eq!(m.precision, 1.0);
        assert_eq!(m.recall, 1.0);
        assert_eq!(m.f1, 1.0);
    }

    #[test]
    fn detection_on_exact_boundary_start_is_tp() {
        // start_index is inclusive → tick == start is a match at tol=0.
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(10, 0.9)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 1);
    }

    #[test]
    fn detection_on_exact_boundary_end_is_miss_at_zero_tolerance() {
        // end_index is exclusive → tick == end is a miss at tol=0.
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(20, 0.9)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 0);
        assert_eq!(m.fp, 1);
        assert_eq!(m.fn_, 1);
    }

    #[test]
    fn tolerance_window_extends_match() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(5, 0.9), det(25, 0.5)];
        // tol = 5 makes [5, 25) match; tick 25 is still outside since exclusive.
        let m = score_detections(&gts, &dets, 5);
        assert_eq!(m.tp, 1);
        // tick 25 is OUTSIDE the extended [5,25) window → FP
        assert_eq!(m.fp, 1);
    }

    #[test]
    fn extra_detections_in_same_window_are_false_positives() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(12, 0.9), det(15, 0.7), det(17, 0.5)];
        let m = score_detections(&gts, &dets, 0);
        // One TP (highest-score pick), two FPs.
        assert_eq!(m.tp, 1);
        assert_eq!(m.fp, 2);
        assert_eq!(m.fn_, 0);
    }

    #[test]
    fn two_events_two_detections_match_disjointly() {
        let gts = vec![gt(10, 20, "a"), gt(30, 40, "b")];
        let dets = vec![det(15, 0.6), det(35, 0.8)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 2);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 0);
    }

    #[test]
    fn highest_score_wins_within_window() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(12, 0.3), det(15, 0.9), det(17, 0.6)];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 1);
        assert_eq!(m.fp, 2);
        // AUPRC should be > 0 since the high-score detection is actually a hit.
        assert!(m.auprc.unwrap_or(0.0) > 0.0);
    }

    #[test]
    fn auprc_is_one_for_perfect_separator() {
        // Two GT events, two matching high-score detections, two distractor
        // low-score detections far from any GT.
        let gts = vec![gt(10, 20, "a"), gt(30, 40, "b")];
        let dets = vec![
            det(15, 0.95),
            det(35, 0.95),
            det(100, 0.10),
            det(200, 0.05),
        ];
        let m = score_detections(&gts, &dets, 0);
        assert_eq!(m.tp, 2);
        assert_eq!(m.fn_, 0);
        let auprc = m.auprc.expect("auprc should be defined");
        assert!(auprc > 0.99, "expected near-perfect AUPRC, got {}", auprc);
    }

    #[test]
    fn auprc_undefined_for_constant_scores() {
        let gts = vec![gt(10, 20, "a")];
        let dets = vec![det(15, 0.5), det(25, 0.5)];
        let m = score_detections(&gts, &dets, 0);
        assert!(m.auprc.is_none());
    }

    #[test]
    fn ground_truth_event_contains_is_half_open() {
        let ev = gt(10, 20, "a");
        assert!(!ev.contains(9));
        assert!(ev.contains(10));
        assert!(ev.contains(19));
        assert!(!ev.contains(20));
    }
}
