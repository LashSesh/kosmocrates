/// Precision, Recall, F1 for a binary or multi-class classification result.

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PrfMetrics {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}

impl PrfMetrics {
    pub fn from_counts(tp: usize, fp: usize, fn_: usize) -> Self {
        let precision = if tp + fp == 0 {
            0.0
        } else {
            tp as f64 / (tp + fp) as f64
        };
        let recall = if tp + fn_ == 0 {
            0.0
        } else {
            tp as f64 / (tp + fn_) as f64
        };
        let f1 = if precision + recall < 1e-12 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        };
        Self {
            precision,
            recall,
            f1,
        }
    }
}

/// Macro-averaged F1 over multiple classes.
pub fn macro_f1(per_class: &[PrfMetrics]) -> f64 {
    if per_class.is_empty() {
        return 0.0;
    }
    per_class.iter().map(|m| m.f1).sum::<f64>() / per_class.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perfect_classifier() {
        let m = PrfMetrics::from_counts(10, 0, 0);
        assert!((m.f1 - 1.0).abs() < 1e-10);
    }
    #[test]
    fn zero_tp() {
        let m = PrfMetrics::from_counts(0, 5, 5);
        assert_eq!(m.f1, 0.0);
    }
}
