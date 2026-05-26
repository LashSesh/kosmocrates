#![allow(clippy::needless_range_loop)]
//! `pse-demo` — 30-second runnable showcase of the Kosmocrates.
//!
//! Drives a synthetic structured stream (a damped oscillator with a
//! mid-stream regime shift) through PSE's full pipeline using a sliding
//! window. Reports throughput, crystal formation rate, **bottleneck gate**
//! (which Kairos threshold blocks the most ticks), and the SHA-256 of the
//! first crystal — so a fresh user can see the engine produce real
//! verifiable artifacts in under a minute.
//!
//! Usage:
//!
//! ```bash
//! cargo run --release -p pse-demo
//! RUST_LOG=pse_core=debug cargo run --release -p pse-demo  # gate diagnostics
//! ```

use std::time::Instant;

use pse_bench_gt::runner::EventScopedAdapter;
use pse_core::{macro_step, GlobalState};
use pse_types::{Config, GateSnapshot, KairosCalibrationConfig};

const N_TICKS: usize = 600;
const WINDOW_SIZE: usize = 8;
/// Regime boundaries (tick indices). Each boundary swaps frequency and amplitude
/// of the synthetic stream — designed to inject genuine novelty into d, q, k.
const REGIME_BOUNDARIES: &[usize] = &[150, 300, 450];

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .try_init();

    println!("PSE Demo — synthetic damped oscillator with regime shift");
    println!("=========================================================\n");
    println!("Ticks: {}, sliding window: {}", N_TICKS, WINDOW_SIZE);

    // Use the streaming preset: Strand-J adaptive carrier + rolling
    // Kairos calibration (top 5% of ticks, 500-tick window, 100-tick warmup).
    // PSE_DEMO_ADAPTIVE=0 falls back to the static-threshold preset so
    // users can compare both modes.
    let adaptive = std::env::var("PSE_DEMO_ADAPTIVE")
        .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(true);
    let mut config = if adaptive {
        let mut c = Config::preset_streaming();
        c.calibration = KairosCalibrationConfig {
            enabled: true,
            target_pass_rate: 0.30,
            window: 150,
            warmup_ticks: 50,
        };
        // Metric d (deformation) decays structurally to near-zero in
        // sliding-window mode (it measures change from one window to the next,
        // which converges as the window stabilises). Setting threshold=0.0
        // makes d always pass; the other seven gates do the discriminating.
        // This is the same fix applied in Config::preset_anomaly_detection().
        c.thresholds.d = 0.0;
        // The cascade PI operator produces a near-zero primal_score on this
        // synthetic stream. consensus_threshold=0.6 (default) would block every
        // crystal even when the Kairos gate passes. Lower to 0.0 so the
        // 8-metric gate is the sole discriminant.
        c.consensus.consensus_threshold = 0.0;
        c.consensus.mirror_consistency_eta = 0.0;
        c
    } else {
        Config::default()
    };
    config.carrier.adaptive = true;
    let state = GlobalState::new(&config);
    let mut state = state;
    if adaptive {
        println!("Mode: adaptive Kairos (target_pass_rate=0.30, window=150, warmup=50)");
    } else {
        println!("Mode: static thresholds (adaptive disabled via PSE_DEMO_ADAPTIVE=0)");
    }
    // Strand-I semantic phase extractor: the synthetic stream embeds the
    // tick index and regime in each payload, so we can recover the
    // *actual* oscillator phase rather than the avalanche-hash phase the
    // engine would otherwise compute from the SHA digest.
    let adapter = EventScopedAdapter::new("demo").with_phase_fn(|raw: &[u8]| {
        let v: serde_json::Value = serde_json::from_slice(raw).ok()?;
        let tick = v.get("tick")?.as_u64()? as f64;
        let regime = v.get("regime")?.as_u64()? as usize;
        let regimes_omega = [0.21_f64, 0.55, 0.18, 0.40];
        let omega = regimes_omega.get(regime).copied()?;
        Some((omega * tick).rem_euclid(std::f64::consts::TAU))
    });

    let payloads = generate_stream(N_TICKS);

    let mut crystals = Vec::new();
    let mut gate_history: Vec<GateSnapshot> = Vec::with_capacity(N_TICKS);
    let mut kairos_passes = 0u64;

    let start = Instant::now();
    for k in 0..payloads.len() {
        let lo = k.saturating_sub(WINDOW_SIZE - 1);
        let window = &payloads[lo..=k];
        match macro_step(&mut state, window, &config, &adapter) {
            Ok(Some(crystal)) => crystals.push(crystal),
            Ok(None) => {}
            Err(e) => eprintln!("tick {} error: {}", k, e),
        }
        if let Some(g) = state.last_gate.clone() {
            if g.kairos {
                kairos_passes += 1;
            }
            gate_history.push(g);
        }
    }
    let elapsed = start.elapsed();

    let total_obs: usize = (0..payloads.len())
        .map(|k| (k + 1) - k.saturating_sub(WINDOW_SIZE - 1))
        .sum();
    let obs_per_sec = total_obs as f64 / elapsed.as_secs_f64();
    let crystal_rate = crystals.len() as f64 / elapsed.as_secs_f64();

    println!("\nResults");
    println!("-------");
    println!("Wall time:          {:.3}s", elapsed.as_secs_f64());
    println!(
        "Observations:       {} (across {} macro_steps)",
        total_obs, N_TICKS
    );
    println!("Throughput:         {:.0} obs/sec", obs_per_sec);
    println!(
        "Kairos passes:      {} / {}",
        kairos_passes,
        gate_history.len()
    );
    println!("Crystals formed:    {}", crystals.len());
    println!("Crystal rate:       {:.2} /sec", crystal_rate);
    if let Some(c) = crystals.first() {
        let id_hex: String = c
            .crystal_id
            .iter()
            .take(8)
            .map(|b| format!("{:02x}", b))
            .collect();
        println!("First crystal SHA:  {}…", id_hex);
    }

    let report = bottleneck_report(&gate_history, &config);
    println!("\nGate diagnostics ({} ticks observed)", gate_history.len());
    println!("------------------------------------");
    println!("{}", report.summary_table());
    if let Some(b) = &report.bottleneck {
        println!("\nBottleneck gate:    {}", b);
        println!("(consider lowering its threshold or checking its metric)");
    } else if !crystals.is_empty() {
        println!("\nNo bottleneck — gate consistently passes.");
    }

    let json = serde_json::json!({
        "wall_seconds": elapsed.as_secs_f64(),
        "total_observations": total_obs,
        "throughput_obs_per_sec": obs_per_sec,
        "crystals": crystals.len(),
        "crystal_rate_per_sec": crystal_rate,
        "bottleneck_gate": report.bottleneck,
        "gate_means": report.means_map(),
        "gate_thresholds": {
            "d": config.thresholds.d, "q": config.thresholds.q,
            "r": config.thresholds.r, "g": config.thresholds.g,
            "j": config.thresholds.j, "p": config.thresholds.p,
            "n": config.thresholds.n, "k": config.thresholds.k,
        },
    });
    std::fs::write("pse-demo.json", serde_json::to_vec_pretty(&json).unwrap())
        .expect("write summary");
    println!("\nWrote pse-demo.json");
}

/// Synthetic stream with multiple regime shifts. Each segment has a
/// distinct (frequency, amplitude) pair; the shifts at `REGIME_BOUNDARIES`
/// inject genuine novelty into d (deformation), q (coherence relative to
/// the active carrier), and k (crystal score). Inside a segment the
/// signal is structured enough that the data helix locks onto a carrier.
fn generate_stream(n: usize) -> Vec<Vec<u8>> {
    let regimes = [(0.21_f64, 1.0_f64), (0.55, 0.5), (0.18, 1.5), (0.40, 0.8)];
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let t = k as f64;
        let regime_idx = REGIME_BOUNDARIES
            .iter()
            .filter(|b| k >= **b)
            .count()
            .min(regimes.len() - 1);
        let (omega, amp_scale) = regimes[regime_idx];
        let value = amp_scale * (omega * t).sin() + 0.10 * (omega * 3.0 * t).cos();
        let payload = serde_json::json!({
            "tick": k,
            "value": value,
            "regime": regime_idx,
        });
        out.push(serde_json::to_vec(&payload).expect("serializes"));
    }
    out
}

const NAMES: [&str; 8] = ["d", "q", "r", "g", "j", "p", "n", "k"];

struct GateReport {
    means: [f64; 8],
    fails: [u64; 8],
    thresholds: [f64; 8],
    bottleneck: Option<String>,
}

impl GateReport {
    fn summary_table(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "{:<6} {:>10} {:>10} {:>8}\n",
            "metric", "mean", "threshold", "fails"
        ));
        for i in 0..8 {
            s.push_str(&format!(
                "{:<6} {:>10.4} {:>10.4} {:>8}\n",
                NAMES[i], self.means[i], self.thresholds[i], self.fails[i]
            ));
        }
        s
    }

    fn means_map(&self) -> serde_json::Value {
        let mut m = serde_json::Map::new();
        for i in 0..8 {
            m.insert(NAMES[i].to_string(), serde_json::json!(self.means[i]));
        }
        serde_json::Value::Object(m)
    }
}

fn bottleneck_report(history: &[GateSnapshot], config: &Config) -> GateReport {
    let thresholds = [
        config.thresholds.d,
        config.thresholds.q,
        config.thresholds.r,
        config.thresholds.g,
        config.thresholds.j,
        config.thresholds.p,
        config.thresholds.n,
        config.thresholds.k,
    ];
    if history.is_empty() {
        return GateReport {
            means: [0.0; 8],
            fails: [0; 8],
            thresholds,
            bottleneck: None,
        };
    }
    let mut sums = [0f64; 8];
    let mut fails = [0u64; 8];
    for g in history {
        let vals = [g.d, g.q, g.r, g.g, g.j, g.p, g.n, g.k];
        for i in 0..8 {
            sums[i] += vals[i];
            if vals[i] < thresholds[i] {
                fails[i] += 1;
            }
        }
    }
    let n = history.len() as f64;
    let means = [
        sums[0] / n,
        sums[1] / n,
        sums[2] / n,
        sums[3] / n,
        sums[4] / n,
        sums[5] / n,
        sums[6] / n,
        sums[7] / n,
    ];
    let mut max_fails = 0u64;
    let mut bottleneck_idx: Option<usize> = None;
    for i in 0..8 {
        if fails[i] > max_fails {
            max_fails = fails[i];
            bottleneck_idx = Some(i);
        }
    }
    let bottleneck = bottleneck_idx.map(|i| {
        format!(
            "{} (failed {}/{} ticks; mean={:.4}, threshold={:.4})",
            NAMES[i],
            fails[i],
            history.len(),
            means[i],
            thresholds[i],
        )
    });
    GateReport {
        means,
        fails,
        thresholds,
        bottleneck,
    }
}
