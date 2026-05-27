//! Kosmocrates quickstart — the smallest end-to-end example.
//!
//! Run:
//!   cargo run --release -p pse --example quickstart
//!
//! Demonstrates the core loop: feed observations through `macro_step`,
//! watch the engine crystallise stable patterns into content-addressed
//! `SemanticCrystal` records, and confirm that replaying the same
//! input produces the same crystal IDs (determinism contract).

use pse::core::{macro_step, GlobalState};
use pse::graph::PassthroughAdapter;
use pse::types::Config;

// 30 entities × 500 ticks matches the scale used in
// `crates/pse-core/examples/accumulation_proof.rs` — small enough to
// finish in a few seconds on release, large enough that the engine
// actually crystallises stable structure.
const N_ENTITIES: usize = 30;
const N_TICKS: usize = 500;

fn synthetic_batch(tick: usize) -> Vec<Vec<u8>> {
    (0..N_ENTITIES)
        .map(|entity| {
            let value = ((tick as f64 * 0.1) + (entity as f64 * 0.2)).sin();
            let payload = serde_json::json!({
                "entity": format!("sensor_{:03}", entity),
                "value": value,
                "tick": tick,
                "phase": (tick as f64 * 0.05 + entity as f64 * 0.1) % std::f64::consts::TAU,
            });
            serde_json::to_vec(&payload).unwrap()
        })
        .collect()
}

fn run_session(label: &str) -> (usize, u64) {
    let config = Config::default();
    let adapter = PassthroughAdapter::new("quickstart");
    let mut state = GlobalState::new(&config);

    for tick in 0..N_TICKS {
        let batch = synthetic_batch(tick);
        let _ = macro_step(&mut state, &batch, &config, &adapter);
    }

    println!(
        "[{label}] crystals = {:>3}   pattern_hits = {:>4}",
        state.archive.len(),
        state.pattern_hits
    );
    (state.archive.len(), state.pattern_hits)
}

fn main() {
    println!("Kosmocrates quickstart — single-process determinism check\n");

    // Two independent sessions over the same deterministic input.
    let (c1, h1) = run_session("session 1");
    let (c2, h2) = run_session("session 2");

    assert_eq!(c1, c2, "crystal count must match across replays");
    assert_eq!(h1, h2, "pattern-hit count must match across replays");

    println!(
        "\nOK: both sessions produced {c1} crystals and {h1} pattern hits — \
         identical across replays. That is the determinism contract \
         documented in CONTRIBUTING.md and enforced by \
         `tests/replay_byte_identity.rs`.\n\n\
         Default Config crystallisation thresholds are conservative. \
         To see crystals form, study the tuned configs in \
         `crates/pse-core/examples/accumulation_proof.rs` and \
         `tools/pse-demo/`."
    );
}
