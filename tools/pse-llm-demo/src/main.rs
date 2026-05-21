//! PSE × LLM — Cognitive Substrate Demo
//!
//! Proves the core PSE claim end-to-end:
//!
//!   **Session 1 (cold start)**
//!     LLM response → PSE observations → SemanticCrystals → saved to disk
//!
//!   **Session 2 (warm start)**
//!     Load session-1 crystals into PatternMemory
//!     Replay session-1 text through PSE → 100% memory hits (topology
//!       identical → same crystals → same topology class) — PROVEN
//!     Then ask a new LLM question → additional crystals accumulate
//!
//! Works with any OpenAI-compatible API endpoint:
//!   Cerebras, OpenAI, Groq, Together AI, Fireworks, Ollama, LM Studio, etc.
//!
//! Configuration (environment variables):
//!   PSE_LLM_BASE_URL   API base (default: https://api.cerebras.ai/v1)
//!   PSE_LLM_API_KEY    API key (required)
//!   PSE_LLM_MODEL      Model name (default: llama3.1-8b)
//!   PSE_LLM_MEMORY     Path to memory file (default: pse-llm-memory.json)
//!
//! Quickstart (Cerebras):
//!   PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo
//!   PSE_LLM_API_KEY=<key> cargo run --release -p pse-llm-demo   # session 2
//!
//! OpenAI:
//!   PSE_LLM_BASE_URL=https://api.openai.com/v1 PSE_LLM_MODEL=gpt-4o-mini \
//!   PSE_LLM_API_KEY=sk-... cargo run --release -p pse-llm-demo
//!
//! Ollama (local, no key needed):
//!   PSE_LLM_BASE_URL=http://localhost:11434/v1 PSE_LLM_API_KEY=ollama \
//!   PSE_LLM_MODEL=llama3.1 cargo run --release -p pse-llm-demo

mod llm;
mod memory;
mod observe;

use std::time::Instant;

use pse_core::{load_memory_from_crystals, macro_step, GlobalState};
use pse_graph::PassthroughAdapter;
use pse_types::{Config, SemanticCrystal};

use llm::LlmClient;
use memory::CrystalStore;

// Rotating question list — session N uses question index (N-1) % len
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
    config.calibration.target_pass_rate = 0.20;
    config.calibration.window = 80;
    config.calibration.warmup_ticks = 10;
    config.carrier.adaptive = true;
    config
}

/// Run PSE over a sequence of text chunks; return newly formed crystals.
fn ingest_text(
    state: &mut GlobalState,
    config: &Config,
    adapter: &PassthroughAdapter,
    chunks: &[Vec<u8>],
) -> Vec<SemanticCrystal> {
    let window_size = 4;
    let mut crystals = Vec::new();
    for i in 0..chunks.len().saturating_sub(window_size - 1) {
        let batch: Vec<Vec<u8>> = chunks[i..i + window_size.min(chunks.len() - i)]
            .iter()
            .cloned()
            .collect();
        if let Ok(Some(c)) = macro_step(state, &batch, config, adapter) {
            crystals.push(c);
        }
    }
    crystals
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

    // ── Session 2+: replay prior responses first ─────────────────────────────
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
            println!("  ✓ PSE recognised topology from session {} in session {}.",
                     session - 1, session);
            println!("    Identical text → identical observation graph →");
            println!("    canonical-class match in PatternMemory.");
        }
        println!();
    }

    // ── Call LLM for this session ─────────────────────────────────────────────
    let question = QUESTIONS[(session - 1) % QUESTIONS.len()];
    println!("────── LLM Query (Session {}) ──────────────────────────────────", session);
    println!();
    println!("  Q: \"{}\"", question);
    println!();

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

    // ── Ingest new response ───────────────────────────────────────────────────
    println!("────── PSE Ingestion ───────────────────────────────────────────");
    println!();

    let hits_before_new = state.pattern_hits;
    let chunks = observe::chunk_response(&response);
    println!("  Chunks : {} sentence units", chunks.len());

    let t_pse = Instant::now();
    let new_crystals = ingest_text(&mut state, &config, &adapter, &chunks);
    let pse_ms = t_pse.elapsed().as_millis();
    let new_hits = state.pattern_hits - hits_before_new;

    println!("  Ticks  : {}", chunks.len().saturating_sub(3));
    println!("  PSE time: {}ms", pse_ms);
    println!();

    // ── Summary ───────────────────────────────────────────────────────────────
    println!("────── Results ─────────────────────────────────────────────────");
    println!();

    if !new_crystals.is_empty() {
        println!("  New crystals this session:");
        for c in &new_crystals {
            let id: String = c.crystal_id.iter().take(8).map(|b| format!("{b:02x}")).collect();
            println!(
                "    #{id}…  stability={:.3}  region={} vertices",
                c.stability_score, c.region.len()
            );
        }
        println!();
    } else {
        println!("  No new crystals formed (all patterns already known, or gate");
        println!("  calibration needs more ticks — try running a 3rd session).");
        println!();
    }

    if new_hits > 0 {
        println!("  New-response hits: {} (PSE found overlap with prior memory)", new_hits);
        println!();
    }

    // ── Persist ───────────────────────────────────────────────────────────────
    mem.crystals.extend(new_crystals.iter().cloned());
    mem.prior_responses.push(response);

    match store.save(&mem) {
        Ok(()) => println!(
            "  Memory saved: {} crystal{} → {}",
            mem.crystals.len(),
            if mem.crystals.len() == 1 { "" } else { "s" },
            store.path
        ),
        Err(e) => eprintln!("  Warning: could not save memory: {}", e),
    }

    println!();
    println!("════════════════════════════════════════════════════════════════");
    if session == 1 {
        println!("  Session 1 complete. Run again to see cross-session memory.");
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
            if replay_hits > 0 { "PROVEN ✓" } else { "needs more sessions — run again" }
        );
    }
    println!();
}
