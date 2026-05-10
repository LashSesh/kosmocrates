//! Log parsers — extract structured metrics from benchmark and test output.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Quality gate result parsed from cargo output logs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualitySummary {
    pub fmt_pass: bool,
    pub clippy_pass: bool,
    pub build_pass: bool,
    pub test_pass: bool,
    pub doc_pass: bool,
    pub test_count: Option<u64>,
    pub failed_count: Option<u64>,
    pub quality_gate_pass: bool,
}

impl QualitySummary {
    pub fn from_exit_codes(
        fmt_exit: Option<i32>,
        clippy_exit: Option<i32>,
        build_exit: Option<i32>,
        test_exit: Option<i32>,
        doc_exit: Option<i32>,
        test_log: &str,
    ) -> Self {
        let fmt_pass = fmt_exit.map(|c| c == 0).unwrap_or(true);
        let clippy_pass = clippy_exit.map(|c| c == 0).unwrap_or(true);
        let build_pass = build_exit.map(|c| c == 0).unwrap_or(true);
        let test_pass = test_exit.map(|c| c == 0).unwrap_or(true);
        let doc_pass = doc_exit.map(|c| c == 0).unwrap_or(true);

        let (test_count, failed_count) = parse_test_counts(test_log);

        let quality_gate_pass = fmt_pass && clippy_pass && build_pass && test_pass && doc_pass;

        QualitySummary {
            fmt_pass,
            clippy_pass,
            build_pass,
            test_pass,
            doc_pass,
            test_count,
            failed_count,
            quality_gate_pass,
        }
    }

    /// All gates passed.
    pub fn all_pass() -> Self {
        QualitySummary {
            fmt_pass: true,
            clippy_pass: true,
            build_pass: true,
            test_pass: true,
            doc_pass: true,
            test_count: None,
            failed_count: None,
            quality_gate_pass: true,
        }
    }
}

/// Benchmark summary parsed from bench_full and bench_gt output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub bench_full_completed: bool,
    pub bench_gt_completed: bool,
    pub pse_demo_completed: bool,
    /// Key → value (e.g. "throughput_obs_per_sec" → "2070000").
    pub metrics: BTreeMap<String, String>,
}

impl BenchmarkSummary {
    pub fn skipped() -> Self {
        BenchmarkSummary {
            bench_full_completed: false,
            bench_gt_completed: false,
            pse_demo_completed: false,
            metrics: BTreeMap::new(),
        }
    }

    pub fn from_logs(
        bench_full_exit: Option<i32>,
        bench_gt_exit: Option<i32>,
        pse_demo_exit: Option<i32>,
        bench_full_log: &str,
    ) -> Self {
        let metrics = parse_bench_metrics(bench_full_log);
        BenchmarkSummary {
            bench_full_completed: bench_full_exit.map(|c| c == 0).unwrap_or(false),
            bench_gt_completed: bench_gt_exit.map(|c| c == 0).unwrap_or(false),
            pse_demo_completed: pse_demo_exit.map(|c| c == 0).unwrap_or(false),
            metrics,
        }
    }
}

/// Open proof obligation for the final report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenObligation {
    pub obligation_id: String,
    pub description: String,
    pub failure_kind: String,
    pub recovery: String,
}

/// Parse test pass/fail counts from `cargo test` output.
fn parse_test_counts(log: &str) -> (Option<u64>, Option<u64>) {
    // Look for lines like "test result: ok. 42 passed; 0 failed"
    for line in log.lines() {
        if line.contains("test result:") {
            let passed = extract_number(line, " passed");
            let failed = extract_number(line, " failed");
            return (passed, failed);
        }
    }
    (None, None)
}

fn extract_number(line: &str, suffix: &str) -> Option<u64> {
    let idx = line.find(suffix)?;
    let before = &line[..idx];
    let num_str = before.rsplit_once([' ', ';', '.'])?;
    num_str.1.parse().ok()
}

/// Parse key metrics from bench_full output.
fn parse_bench_metrics(log: &str) -> BTreeMap<String, String> {
    let mut metrics = BTreeMap::new();
    for line in log.lines() {
        // Example: "B01a observe-only ingest: 2_070_000 obs/sec"
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().replace([' ', '-'], "_").to_lowercase();
            let val = line[pos + 1..].trim().to_string();
            if !key.is_empty() && !val.is_empty() {
                metrics.insert(key, val);
            }
        }
    }
    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_summary_from_exit_codes() {
        let s = QualitySummary::from_exit_codes(
            Some(0),
            Some(0),
            Some(0),
            Some(0),
            Some(0),
            "test result: ok. 42 passed; 0 failed",
        );
        assert!(s.quality_gate_pass);
        assert_eq!(s.test_count, Some(42));
        assert_eq!(s.failed_count, Some(0));
    }

    #[test]
    fn quality_summary_fails_if_any_gate_fails() {
        let s = QualitySummary::from_exit_codes(
            Some(1), // fmt fail
            Some(0),
            Some(0),
            Some(0),
            Some(0),
            "",
        );
        assert!(!s.quality_gate_pass);
        assert!(!s.fmt_pass);
    }
}
