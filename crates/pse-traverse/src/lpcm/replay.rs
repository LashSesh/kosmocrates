//! LPCM replay and verification (§15).
//!
//! verify_lpcm_replay re-runs the pipeline and compares canonical report hashes.

use serde::{Deserialize, Serialize};

use super::primitives::{Hash256, LpcmResult};
use super::report::HierarchicalCollapseReport;
use super::LpcmFailureKind;

/// Report from a replay verification run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcmReplayReport {
    pub replay_id: Hash256,
    pub expected_report_id: Hash256,
    pub actual_report_id: Hash256,
    pub byte_identical: bool,
    pub hash_identical: bool,
    pub mismatch_kind: Option<LpcmFailureKind>,
}

/// Verify replay identity: re-run the pipeline and compare canonical report hashes.
pub fn verify_lpcm_replay(
    rd: &super::run_descriptor::LpcmRunDescriptor,
    input: &super::evidence::LpcmInputWindow,
    expected: &HierarchicalCollapseReport,
) -> LpcmResult<LpcmReplayReport> {
    use super::pipeline::run_lpcm;
    use super::primitives::lpcm_content_address;

    let actual = run_lpcm(rd.clone(), input.clone())?;
    let hash_identical = actual.report_id == expected.report_id;
    let replay_id = lpcm_content_address(&(&actual.report_id, &expected.report_id))?;

    Ok(LpcmReplayReport {
        replay_id,
        expected_report_id: expected.report_id.clone(),
        actual_report_id: actual.report_id.clone(),
        byte_identical: hash_identical,
        hash_identical,
        mismatch_kind: if hash_identical {
            None
        } else {
            Some(LpcmFailureKind::ReplayMismatch)
        },
    })
}
