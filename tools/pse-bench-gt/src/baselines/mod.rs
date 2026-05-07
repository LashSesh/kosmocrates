//! Baseline anomaly detectors for fair comparison against PSE.
//!
//! Each baseline consumes a 1-D feature series (one f64 per observation
//! tick, in the same tick frame PSE produces) and emits
//! [`Detection`](crate::Detection)s with a distinct `source` string.
//! The bench-gt scorer treats them exactly like PSE detections: same
//! matching rules, same metrics.

pub mod isoforest;
pub mod stl_zscore;
