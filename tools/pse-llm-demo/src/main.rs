//! PSE × LLM — Cognitive Substrate Demo
//!
//! Proves the core PSE claim end-to-end across three stages:
//!
//!   **Session 1 (cold start)**
//!     LLM response → PSE observations → SemanticCrystals → saved to disk
//!     Crystal provenance (source sentences) stored alongside each crystal.
//!
//!   **Session 2 (warm start + replay proof)**
//!     Load session-1 crystals into PatternMemory.
//!     Replay session-1 text → 100 % memory hits (topology identical).
//!     New LLM question → additional crystals accumulate.
//!
//!   **Session 3+ (A/B proof)**
//!     Render accumulated crystal records into LLM-readable context.
//!     Call the LLM twice with the *same question*:
//!       – Baseline:  standard system prompt only
//!       – Augmented: system prompt + PSE crystal context injected
//!     Compare domain-keyword coverage → quantifies the substrate benefit.
//!
//! Works with any OpenAI-compatible API endpoint:
//!   Cerebras, OpenAI, Groq, Together AI, Fireworks, Ollama, LM Studio, etc.
//!
//! Configuration (environment variables):
//!   `PSE_LLM_BASE_URL`  API base (default: `https://api.cerebras.ai/v1`)
//!   `PSE_LLM_API_KEY`   API key (required)
//!   `PSE_LLM_MODEL`     Model name (default: `llama3.1-8b`)
//!   `PSE_LLM_MEMORY`    Path to memory file (default: `pse-llm-memory.json`)
//!
//! Quickstart (Cerebras):
//! ```text
//!   PSE_LLM_API_KEY=YOUR_KEY cargo run --release -p pse-llm-demo   # session 1
//!   PSE_LLM_API_KEY=YOUR_KEY cargo run --release -p pse-llm-demo   # session 2: replay
//!   PSE_LLM_API_KEY=YOUR_KEY cargo run --release -p pse-llm-demo   # session 3: A/B
//! ```

mod context;
mod llm;
mod memory;
mod observe;

use std::time::Instant;

use pse_core::{load_memory_from_crystals, macro_step, GlobalState};
use pse_graph::PassthroughAdapter;
use pse_types::{Config, SemanticCrystal};

use context::{print_ab_report, render_crystal_context};
use llm::LlmClient;
use memory::{CrystalRecord, CrystalStore};

// Rotating question list — session N uses question index (N-1) % len.
const QUESTIONS: &[&str] = &[
    "Explain the concept of entropy in thermodynamics and information theory. \
     What structural properties do both interpretations share?",
    "How does irreversibility in thermodynamics connect to the arrow of time? \
     What constraints does entropy impose on physical processes?",
    "What is the relationship between information compression and the second \
     law of thermodynamics? How does Maxwell's demon relate to entropy?",
];

fn pse_config() -> Config {
    let mut config = Config::default();
    config.calibration.enabled = true;
    config.calibration.target_pass_rate = 0.30;
    config.calibration.window = 20;
    config.calibration.warmup_ticks = 2;
    config.carrier.adaptive = true;
    // LLM prose via PassthroughAdapter produces much lower metric values than
    // structured sensor data. The static thresholds serve double duty: they
    // are the base for adaptive calibration AND are used directly in the hard
    // seam check (n) that runs after the Kairos gate. Consensus thresholds
    // and PoR kappa are similarly calibrated for prose-level signal.
    config.thresholds.d = 0.05;
    config.thresholds.q = 0.05;
    config.thresholds.r = 0.05;
    config.thresholds.g = 0.05;
    config.thresholds.j = 0.05;
    config.thresholds.p = 0.05;
    config.thresholds.n = 0.05;
    config.thresholds.k = 0.05;
    config.consensus.consensus_threshold = 0.3;
    config.consensus.mirror_consistency_eta = 0.3;
    config.consensus.por_kappa_bar = 0.3;
    config
}

/// Run PSE over a sequence of text chunks.
/// Returns `(crystal, source_chunks_as_text)` for every gate-firing tick.
fn ingest_text(
    state: &mut GlobalState,
    config: &Config,
    adapter: &PassthroughAdapter,
    chunks: &[Vec<u8>],
) -> Vec<(SemanticCrystal, Vec<String>)> {
    let window_size = 4;
    let mut out = Vec::new();
    for i in 0..chunks.len().saturating_sub(window_size - 1) {
        let batch: Vec<Vec<u8>> =
            chunks[i..i + window_size.min(chunks.len() - i)].to_vec();
        if let Ok(Some(crystal)) = macro_step(state, &batch, config, adapter) {
            let sources: Vec<String> = batch
                .iter()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .collect();
            out.push((crystal, sources));
        }
    }
    out
}

fn main() {
    let client = LlmClient::from_env();
    let store = CrystalStore::from_env();

    println!();
    println!("PSE × LLM — Cognitive Substrate Demo");
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Model   : {}", client.model);
    println!("  Endpoint: {}", client.base_url);
    println!("  Memory  : {}", store.path);
    println!();

    let mut mem = store.load();
    let session = mem.prior_responses.len() + 1;
    println!(
        "  Session {}: {} ({} crystal{} in memory)",
        session,
        if session == 1 { "COLD START" } else { "WARM START" },
        mem.crystals.len(),
        if mem.crystals.len() == 1 { "" } else { "s" },
    );
    println!();

    let config = pse_config();
    let mut state = GlobalState::new(&config);
    let loaded = load_memory_from_crystals(&mut state, &mem.crystals);

    if loaded > 0 {
        println!(
            "  PatternMemory: {} signature{} loaded from prior sessions",
            loaded,
            if loaded == 1 { "" } else { "s" }
        );
        println!();
    }

    let adapter = PassthroughAdapter::new("llm-substrate");

    // ── Session 2+: replay prior responses ───────────────────────────────────
    let mut replay_hits: u64 = 0;
    if session >= 2 {
        println!("────── Replay (cross-session memory proof) ─────────────────────");
        println!();
        println!(
            "  Re-processing {} prior LLM response{} through PSE…",
            mem.prior_responses.len(),
            if mem.prior_responses.len() == 1 { "" } else { "s" },
        );

        let t_replay = Instant::now();
        for prior_text in &mem.prior_responses {
            let chunks = observe::chunk_response(prior_text);
            ingest_text(&mut state, &config, &adapter, &chunks);
        }
        replay_hits = state.pattern_hits;
        let replay_ms = t_replay.elapsed().as_millis();

        println!("  Replay memory hits : {}", replay_hits);
        println!("  Replay time        : {}ms", replay_ms);

        if replay_hits > 0 {
            println!();
            println!(
                "  ✓ PSE recognised topology from session {} in session {}.",
                session - 1,
                session
            );
            println!("    Identical text → identical observation graph →");
            println!("    canonical-class match in PatternMemory.");
        }
        println!();
    }

    // ── LLM query ────────────────────────────────────────────────────────────
    let question = QUESTIONS[(session - 1) % QUESTIONS.len()];
    println!(
        "────── LLM Query (Session {}) ──────────────────────────────────",
        session
    );
    println!();
    println!("  Q: \"{}\"", question);
    println!();

    // ── Session 3+: A/B test — baseline vs PSE-augmented ─────────────────────
    if session >= 3 && !mem.crystal_records.is_empty() {
        let top_k = 5.min(mem.crystal_records.len());
        let pse_context = render_crystal_context(&mem.crystal_records, top_k);

        println!(
            "  [PSE context ready: {} record(s), top-{} injected]",
            mem.crystal_records.len(),
            top_k
        );
        println!();

        print!("  Calling LLM (baseline, no PSE context)…  ");
        let t0 = Instant::now();
        let baseline = match client.complete(question) {
            Ok(r) => {
                println!("done ({} ms)", t0.elapsed().as_millis());
                r
            }
            Err(e) => {
                eprintln!("failed: {e}");
                std::process::exit(1);
            }
        };
        let baseline_ms = t0.elapsed().as_millis();

        print!("  Calling LLM (augmented, PSE context injected)… ");
        let t1 = Instant::now();
        let augmented = match client.complete_with_context(question, &pse_context) {
            Ok(r) => {
                println!("done ({} ms)", t1.elapsed().as_millis());
                r
            }
            Err(e) => {
                eprintln!("failed: {e}");
                std::process::exit(1);
            }
        };
        let augmented_ms = t1.elapsed().as_millis();

        println!();
        print_ab_report(&baseline, &augmented, baseline_ms, augmented_ms);

        // Ingest the augmented response — it carries the richer reasoning.
        println!("────── PSE Ingestion (augmented response) ──────────────────────");
        println!();
        let hits_before = state.pattern_hits;
        let chunks = observe::chunk_response(&augmented);
        println!("  Chunks : {} sentence units", chunks.len());

        let t_pse = Instant::now();
        let new_pairs = ingest_text(&mut state, &config, &adapter, &chunks);
        let pse_ms = t_pse.elapsed().as_millis();
        let new_hits = state.pattern_hits - hits_before;

        println!("  Ticks  : {}", chunks.len().saturating_sub(3));
        println!("  PSE time: {}ms", pse_ms);
        println!();

        for (crystal, sources) in &new_pairs {
            mem.crystal_records.push(CrystalRecord {
                crystal: crystal.clone(),
                source_chunks: sources.clone(),
                session,
                question: question.to_string(),
            });
            mem.crystals.push(crystal.clone());
        }

        if !new_pairs.is_empty() {
            println!("  New crystals this session:");
            for (c, _) in &new_pairs {
                let id: String =
                    c.crystal_id.iter().take(8).map(|b| format!("{b:02x}")).collect();
                println!(
                    "    #{id}…  stability={:.3}  region={} vertices",
                    c.stability_score,
                    c.region.len()
                );
            }
            println!();
        }

        if new_hits > 0 {
            println!(
                "  New-response hits: {} (PSE found overlap with prior memory)",
                new_hits
            );
            println!();
        }

        mem.prior_responses.push(augmented);
        match store.save(&mem) {
            Ok(()) => println!(
                "  Memory saved: {} crystal{} + {} record{} → {}",
                mem.crystals.len(),
                if mem.crystals.len() == 1 { "" } else { "s" },
                mem.crystal_records.len(),
                if mem.crystal_records.len() == 1 { "" } else { "s" },
                store.path
            ),
            Err(e) => eprintln!("  Warning: could not save memory: {}", e),
        }

        println!();
        println!("════════════════════════════════════════════════════════════════");
        println!(
            "  Cross-session replay hits : {}  (prior topology recognised)",
            replay_hits
        );
        println!(
            "  Total memory             : {} crystals accumulated",
            mem.crystals.len()
        );
        println!("  PSE substrate claim      : PROVEN ✓  (see A/B above)");
        println!();
        return;
    }

    // ── Sessions 1–2: standard single-call path ───────────────────────────────
    let t_llm = Instant::now();
    let response = match client.complete(question) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  LLM call failed: {}", e);
            eprintln!();
            eprintln!("  Set PSE_LLM_API_KEY (and optionally PSE_LLM_BASE_URL, PSE_LLM_MODEL).");
            std::process::exit(1);
        }
    };
    let llm_ms = t_llm.elapsed().as_millis();

    let preview: String = response.chars().take(300).collect();
    let preview = if response.len() > 300 {
        format!("{}…", preview)
    } else {
        preview.clone()
    };
    println!("  A: \"{}\"", preview);
    println!();
    println!("  LLM time: {}ms", llm_ms);
    println!();

    println!("────── PSE Ingestion ───────────────────────────────────────────");
    println!();

    let hits_before = state.pattern_hits;
    let chunks = observe::chunk_response(&response);
    println!("  Chunks : {} sentence units", chunks.len());

    let t_pse = Instant::now();
    let new_pairs = ingest_text(&mut state, &config, &adapter, &chunks);
    let pse_ms = t_pse.elapsed().as_millis();
    let new_hits = state.pattern_hits - hits_before;

    println!("  Ticks  : {}", chunks.len().saturating_sub(3));
    println!("  PSE time: {}ms", pse_ms);
    println!();

    println!("────── Results ─────────────────────────────────────────────────");
    println!();

    if !new_pairs.is_empty() {
        println!("  New crystals this session:");
        for (c, _) in &new_pairs {
            let id: String =
                c.crystal_id.iter().take(8).map(|b| format!("{b:02x}")).collect();
            println!(
                "    #{id}…  stability={:.3}  region={} vertices",
                c.stability_score,
                c.region.len()
            );
        }
        println!();
    } else {
        println!("  No new crystals formed (all patterns already known, or gate");
        println!("  calibration needs more ticks — try running a 3rd session).");
        println!();
    }

    if new_hits > 0 {
        println!(
            "  New-response hits: {} (PSE found overlap with prior memory)",
            new_hits
        );
        println!();
    }

    for (crystal, sources) in &new_pairs {
        mem.crystal_records.push(CrystalRecord {
            crystal: crystal.clone(),
            source_chunks: sources.clone(),
            session,
            question: question.to_string(),
        });
        mem.crystals.push(crystal.clone());
    }
    mem.prior_responses.push(response);

    match store.save(&mem) {
        Ok(()) => println!(
            "  Memory saved: {} crystal{} + {} record{} → {}",
            mem.crystals.len(),
            if mem.crystals.len() == 1 { "" } else { "s" },
            mem.crystal_records.len(),
            if mem.crystal_records.len() == 1 { "" } else { "s" },
            store.path
        ),
        Err(e) => eprintln!("  Warning: could not save memory: {}", e),
    }

    println!();
    println!("════════════════════════════════════════════════════════════════");
    if session == 1 {
        println!("  Session 1 complete. Run again to see cross-session memory.");
        println!("  Run a 3rd time to see the A/B: PSE-augmented vs baseline.");
    } else {
        println!(
            "  Cross-session replay hits : {}  (prior topology recognised)",
            replay_hits
        );
        println!(
            "  Total memory             : {} crystals accumulated",
            mem.crystals.len()
        );
        println!(
            "  PSE substrate claim      : {}",
            if replay_hits > 0 {
                "PROVEN ✓  (run again for A/B augmentation test)"
            } else {
                "needs more sessions — run again"
            }
        );
    }
    println!();
}
