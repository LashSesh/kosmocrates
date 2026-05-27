# pse-llm-demo

PSE as a cognitive substrate for LLMs — cross-session memory demo

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

PSE × LLM — Cognitive Substrate Demo

Proves the core PSE claim end-to-end across three stages:

  **Session 1 (cold start)**
    LLM response → PSE observations → SemanticCrystals → saved to disk
    Crystal provenance (source sentences) stored alongside each crystal.

  **Session 2 (warm start + replay proof)**
    Load session-1 crystals into PatternMemory.
    Replay session-1 text → 100 % memory hits (topology identical).
    New LLM question → additional crystals accumulate.

  **Session 3+ (A/B proof)**
    Render accumulated crystal records into LLM-readable context.
    Call the LLM twice with the *same question*:
      – Baseline:  standard system prompt only
      – Augmented: system prompt + PSE crystal context injected
    Compare domain-keyword coverage → quantifies the substrate benefit.

Works with any OpenAI-compatible API endpoint:
  Cerebras, OpenAI, Groq, Together AI, Fireworks, Ollama, LM Studio, etc.

Configuration (environment variables):
  `PSE_LLM_BASE_URL`  API base (default: `https://api.cerebras.ai/v1`)
  `PSE_LLM_API_KEY`   API key (required)
  `PSE_LLM_MODEL`     Model name (default: `llama3.1-8b`)
  `PSE_LLM_MEMORY`    Path to memory file (default: `pse-llm-memory.json`)

Quickstart (Cerebras):
```text
  PSE_LLM_API_KEY=YOUR_KEY cargo run --release -p pse-llm-demo   # session 1
  PSE_LLM_API_KEY=YOUR_KEY cargo run --release -p pse-llm-demo   # session 2: replay
  PSE_LLM_API_KEY=YOUR_KEY cargo run --release -p pse-llm-demo   # session 3: A/B
```

## Run

```bash
# From the workspace root:
cargo run --release -p pse-llm-demo
# or, after `cargo install --path .`:
pse-llm-demo
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
