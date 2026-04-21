//! Generic PSE driver that turns a stream of raw observation payloads into
//! a stream of [`Detection`]s.
//!
//! Two detection channels are emitted:
//!  - `"pse_crystal"` for every successful crystal commit (one Detection per
//!    `Ok(Some(crystal))` return); score is `crystal.stability_score`.
//!  - `"pse_memory_hit"` whenever the internal pattern memory short-circuits
//!    a tick (the `state.pattern_hits` counter ticks up); score is `0.5`
//!    since memory hits do not carry an intrinsic stability score.
//!
//! Detection `tick` is the value of `state.commit_index` **immediately after**
//! the `macro_step` call — PSE increments `commit_index` unconditionally at
//! the top of every macro-step, so this is a monotonically-increasing tick
//! index whether or not a crystal was committed.

use pse_core::{macro_step, GlobalState};
use pse_graph::ObservationAdapter;
use pse_types::Config;

use crate::Detection;

/// Drive PSE over a stream of raw payloads, one payload per macro-step.
///
/// The caller owns `state` and may inspect it after the run (for example
/// to read `state.archive.crystals()` or `state.pattern_hits`). The runner
/// only appends to the returned `Vec<Detection>`; any state mutation is
/// purely the effect of `macro_step` itself.
pub fn run_pse(
    state: &mut GlobalState,
    observations: &[Vec<u8>],
    config: &Config,
    adapter: &dyn ObservationAdapter,
) -> Vec<Detection> {
    let mut detections = Vec::new();

    for payload in observations {
        let batch = vec![payload.clone()];
        let hits_before = state.pattern_hits;

        match macro_step(state, &batch, config, adapter) {
            Ok(Some(crystal)) => {
                detections.push(Detection::new(
                    state.commit_index,
                    crystal.stability_score.clamp(0.0, 1.0),
                    "pse_crystal",
                ));
            }
            Ok(None) => {
                if state.pattern_hits > hits_before {
                    // A pattern-memory shortcut fired at this tick.
                    detections.push(Detection::new(
                        state.commit_index,
                        0.5,
                        "pse_memory_hit",
                    ));
                }
                // Otherwise: gate/seam/consensus rejection → no detection.
            }
            Err(_) => {
                // Ingestion errors do not produce detections. The caller can
                // inspect the stream for validation failures separately.
            }
        }
    }

    detections
}
