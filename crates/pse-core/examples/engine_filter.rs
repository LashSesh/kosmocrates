//! Demonstriert PSEs vollständigen Engine-Filter.
//!
//! `EngineFilter` verbirgt `GlobalState` + `Config` + `macro_step` hinter
//! einer einzelnen `step()`-Methode. Synthetische Beobachtungen fließen Tick
//! für Tick hinein; gate-Werte werden periodisch ausgegeben, um zu zeigen,
//! wie der Kairos-Gate über die Zeit fluktuiert.
//!
//! Kristallformation erfordert echte Datenkonvergenz über viele Ticks hinweg.
//! Mit synthetischen Sinusdaten verweilt der Engine absichtlich im HOLD-Zustand.
//!
//! Ausführen:
//!   cargo run -p pse-core --example engine_filter

use pse_core::EngineFilter;

fn batch(tick: usize, n_sensors: usize) -> Vec<Vec<u8>> {
    (0..n_sensors)
        .map(|s| {
            serde_json::to_vec(&serde_json::json!({
                "entity": format!("sensor_{:03}", s),
                "value": ((tick as f64 * 0.1) + (s as f64 * 0.2)).sin(),
                "tick": tick,
                "phase": (tick as f64 * 0.05 + s as f64 * 0.1) % std::f64::consts::TAU,
            }))
            .unwrap()
        })
        .collect()
}

fn main() {
    let mut filter = EngineFilter::new();

    let n_ticks = 100;
    let n_sensors = 50;

    println!("PSE Engine Filter — Demo");
    println!("{n_sensors} sensors × {n_ticks} ticks  (Config::default)");
    println!();
    println!("tick   kairos  d     q     r     g     commit-rate");
    println!("────────────────────────────────────────────────────");

    for tick in 0..n_ticks {
        let obs = batch(tick, n_sensors);
        let result = filter.step(&obs).expect("step must not error");

        // Print every 10th tick so gate dynamics are visible.
        if tick % 10 == 9 || result.committed {
            let gate = match &result.gate {
                Some(g) => format!(
                    "{:<7}  {:.3}  {:.3}  {:.3}  {:.3}",
                    if g.kairos { "YES" } else { "no" },
                    g.d, g.q, g.r, g.g
                ),
                None => "—".into(),
            };
            let commit_mark = if result.committed { "  ← COMMIT" } else { "" };
            println!(
                "{:>4}   {}  {:.1}%{}",
                result.tick,
                gate,
                filter.commit_rate() * 100.0,
                commit_mark,
            );
        }
    }

    println!("────────────────────────────────────────────────────");
    println!(
        "Ergebnis: {}/{} ticks committed  ({:.1}% commit-rate)",
        filter.commits(),
        filter.ticks(),
        filter.commit_rate() * 100.0,
    );
    println!();
    println!("Hinweis: Kristallformation erfordert Datenkonvergenz über viele Ticks.");
    println!("         Synthetische Sinusdaten erfüllen die strukturellen Gate-Bedingungen");
    println!("         absichtlich nicht vollständig — das Engine bleibt im HOLD-Zustand.");
    println!("         Mit realen oder tuned-Config-Daten entstehen Kristalle.");
}
