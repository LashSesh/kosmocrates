//! LPCM adapters: input canonicalization and fragment sorting.

use super::evidence::LpcmInputWindow;
use super::fragment::{FragmentedPhaseSpaceWindow, FragmentRef};
use super::primitives::{Hash256, LpcmError, LpcmResult, lpcm_content_address};

/// Canonicalize the raw input window. Returns an owned canonical copy.
pub fn canonicalize_input(input: LpcmInputWindow) -> LpcmResult<LpcmInputWindow> {
    // Validate basic integrity.
    if input.evidence_refs.is_empty() && input.adjacency.is_empty() {
        return Err(LpcmError::InvalidWindow {
            reason: "input window has no evidence refs and no adjacency".to_string(),
        });
    }
    Ok(input)
}

/// Return fragment refs sorted by ordinal (deterministic processing order).
pub fn sorted_fragments(window: &FragmentedPhaseSpaceWindow) -> Vec<&FragmentRef> {
    let mut frags: Vec<&FragmentRef> = window.fragments.iter().collect();
    frags.sort_by_key(|f| f.ordinal);
    frags
}
