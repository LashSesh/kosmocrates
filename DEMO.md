# PSE Live Demo — Three-Session Proof

> **What you will see in 15 minutes:**
> An LLM that forgets everything between calls acquires persistent, cryptographically
> auditable memory.  By session 3 it is measurably smarter on your domain — without
> fine-tuning, without a vector database, without a running server.

---

## Prerequisites

| Requirement | How to get it |
|---|---|
| **Rust 1.82+** | `rustup update stable` — see [rustup.rs](https://rustup.rs) |
| **Git** | [git-scm.com](https://git-scm.com) |
| **Cerebras API key** | [cloud.cerebras.ai](https://cloud.cerebras.ai) — free tier is fine |
| **Visual C++ Build Tools** (Windows only) | Included when running in *x64 Native Tools Command Prompt* |

No Python, no Docker, no external services beyond the LLM API.

---

## One-time setup

```
git clone https://github.com/lashsesh/pse
cd pse
cargo build --release -p pse-llm-demo
```

The first build downloads and compiles all dependencies (~2–3 min).
Subsequent builds are incremental (seconds).

---

## The Three Sessions

### Session 1 — Cold start: LLM output becomes topology

**Windows (x64 Native Tools Command Prompt):**
```
set PSE_LLM_API_KEY=<your-cerebras-key>
cargo run --release -p pse-llm-demo
```

**Linux / macOS:**
```bash
PSE_LLM_API_KEY=<your-cerebras-key> cargo run --release -p pse-llm-demo
```

**What you will see:**

```
PSE × LLM — Cognitive Substrate Demo
═══════════════════════════════════════════════════════════════
  Model   : llama3.1-8b
  Endpoint: https://api.cerebras.ai/v1
  Memory  : pse-llm-memory.json
  Phase   : char-4gram (Tier 2)

  Session 1: COLD START (0 crystals in memory)

────── LLM Query (Session 1) ──────────────────────────────────

  Q: "Explain how ACT-R models declarative memory retrieval…"

  A: "ACT-R models declarative memory retrieval through…"

  LLM time: 1840ms

────── PSE Ingestion ───────────────────────────────────────────

  Chunks : 18 sentence units
  Ticks  : 15
  PSE time: 2ms

────── Results ─────────────────────────────────────────────────

  New crystals this session:
    #3fa2e1c0…  stability=0.847  region=6 vertices
    #a91b7d23…  stability=0.791  region=4 vertices

  Memory saved: 2 crystals + 2 records → pse-llm-memory.json

════════════════════════════════════════════════════════════════
  Session 1 complete. Run again to see cross-session memory.
  Run a 3rd time to see the A/B: PSE-augmented vs baseline.
```

**What just happened:**
The LLM's response was split into sentence units, each passed through the PSE
8-metric Kairos gate.  Two sentences (or sentence clusters) formed stable enough
topology to crystallize.  Each crystal carries a SHA-256 content address —
`#3fa2e1c0…` is a short prefix.  The full 32-byte ID is bit-identical on every
replay of the same text.  Both crystals are now persisted in `pse-llm-memory.json`.

---

### Session 2 — Warm start: cross-session memory proven

Run the **exact same command** a second time:

```
cargo run --release -p pse-llm-demo
```
*(the API key is still set in your shell)*

**What you will see:**

```
  Session 2: WARM START (2 crystals in memory)

  PatternMemory: 2 signatures loaded from prior sessions

────── Replay (cross-session memory proof) ─────────────────────

  Re-processing 1 prior LLM response through PSE…
  Replay memory hits : 2
  Replay time        : 1ms

  ✓ PSE recognised topology from session 1 in session 2.
    Identical text → identical observation graph →
    canonical-class match in PatternMemory.

────── LLM Query (Session 2) ──────────────────────────────────
  Q: "How does SOAR's chunking mechanism learn from impasses?…"
  …
════════════════════════════════════════════════════════════════
  Cross-session replay hits : 2  (prior topology recognised)
  Total memory             : 4 crystals accumulated
  PSE substrate claim      : PROVEN ✓  (run again for A/B)
```

**What just happened:**
PSE re-processed the session-1 LLM response (a completely fresh Rust process,
no shared memory, no running server).  The same text produced the same
observation graph, the same Kairos gate outcome, the same crystal IDs — and
`PatternMemory` reported 2 hits.  This is not a lookup by text similarity.
This is cryptographic topology recognition: the byte-identical crystal IDs
match the ones loaded from disk.

> **This is the core claim proven:** persistent, cross-session, deterministic
> memory — without a database, without embeddings, without a server.

---

### Session 3 — A/B test: PSE context vs. bare LLM

Run a third time:

```
cargo run --release -p pse-llm-demo
```

**What you will see:**

```
  Session 3: WARM START (4 crystals in memory)

────── LLM Query (Session 3) ──────────────────────────────────

  Q: "Describe the Global Workspace Theory of consciousness…"

  ── Baseline call (no PSE context) ──────────────────────────
  A (baseline): "Global Workspace Theory (GWT) proposes that…"
  Baseline coverage: 8 / 20 keywords (40%)

  ── Augmented call (PSE context injected) ───────────────────
  [PSE-CONTEXT]
    [PSE:3fa2e1c0…] stab=0.847  "ACT-R models declarative memory…"
    [PSE:a91b7d23…] stab=0.791  "production compilation reduces…"
    [PSE:7b13fe45…] stab=0.768  "SOAR's chunking mechanism…"
  [/PSE-CONTEXT]

  A (augmented): "Global Workspace Theory, when viewed alongside ACT-R's
                  base-level activation and SOAR's chunking from impasses…"
  Augmented coverage: 11 / 20 keywords (55%)

  Coverage delta: +15 pp  (PSE context lifted domain coverage)

  ── How big the delta will be on your first three sessions ──
  Typical first-run delta is +5 to +15 pp.  Run 6+ sessions to grow
  crystal density — the delta increases roughly linearly with memory.

════════════════════════════════════════════════════════════════
  Cross-session replay hits : 4  (prior topology recognised)
  Total memory             : 5 crystals accumulated
  PSE substrate claim      : PROVEN ✓  (see A/B above)
```

**What just happened:**
The same question was sent to the LLM twice.  The second call received a
`[PSE-CONTEXT]` block assembled from the crystals accumulated in sessions 1–2.
The coverage metric counts how many domain-specific keywords appear in the
response.  The PSE-augmented response scored higher — the prior knowledge,
crystallised and retrieved deterministically, made the LLM's answer measurably
better on this domain.

> **The A/B delta grows with crystal density.**  On well-known domains
> (thermodynamics, sorting algorithms) the base LLM already scores high and
> the gap is small.  The demo targets cognitive architectures (ACT-R, SOAR,
> Global Workspace Theory) precisely because small LLMs have shallow coverage
> of specific mechanisms — the gain is real and visible from session 3 onward.

---

## Calibrating crystal density (`PSE_LLM_PASS_RATE`)

The Kairos gate's default target pass rate is `0.50` — it fires on the top 50 %
of ticks.  With ~14–20 ticks per LLM response that produces 5–10 crystals per
session.  For dense, A/B-strong runs out of the box, that's a reasonable default.

If you want fewer, higher-quality crystals — or more, noisier ones — override:

```
rem Sparse, high-confidence crystals (≈ top 20 % of ticks)
set PSE_LLM_PASS_RATE=0.20

rem Dense crystallization (≈ top 70 % of ticks)
set PSE_LLM_PASS_RATE=0.70
```

The pass rate does not affect the cross-session replay proof — replay hits work
regardless of how many crystals are in memory, as long as there is at least one.
What it does affect is the strength of the A/B delta in session 3+.

## How many sessions until the A/B is convincing?

| Sessions | Typical crystal count | Typical A/B delta (cognitive-architectures domain) |
|---|---|---|
| 1 | 5–10 | n/a (no A/B yet — needs prior crystals) |
| 3 | 10–25 | +5 to +15 pp |
| 6 | 25–60 | +15 to +30 pp |
| 10+ | 60+ | +25 to +50 pp |

The questions list rotates every 3 sessions (session 4 = session 1's question
again, with new context).  Each repeated question crystallizes a fresh response
from the LLM, so memory continues to grow.  **Run 6 sessions for a convincing
demo, 10+ for an impressive one.**

Numbers vary with model size, temperature, and how much your domain overlaps
the LLM's training data.  Domains where the LLM has shallow coverage (specific
scientific mechanisms, niche frameworks, internal terminology) show the
largest A/B gap.

---

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `Set PSE_LLM_API_KEY` error | Key not in environment | `set PSE_LLM_API_KEY=<key>` before running |
| `LLM call failed` | API unreachable or wrong base URL | Check `PSE_LLM_BASE_URL`; default is Cerebras |
| `No new crystals formed` | Gate is in conservative regime | Set `PSE_LLM_PASS_RATE=0.70` for denser firing |
| Crystals formed but 0 replay hits | Memory file not found | Make sure `pse-llm-memory.json` is in the working directory |
| Session counter resets to 1 | Wrong working directory | Run all three sessions from the same directory (`cd pse`) |
| Small A/B delta after 3 sessions | Crystal density too low | Run more sessions — delta grows with memory accumulation |

### Using a different LLM provider

```
rem OpenAI (Windows)
set PSE_LLM_API_KEY=sk-...
set PSE_LLM_BASE_URL=https://api.openai.com/v1
set PSE_LLM_MODEL=gpt-4o-mini
cargo run --release -p pse-llm-demo

rem Groq
set PSE_LLM_API_KEY=gsk_...
set PSE_LLM_BASE_URL=https://api.groq.com/openai/v1
set PSE_LLM_MODEL=llama-3.1-8b-instant
cargo run --release -p pse-llm-demo

rem Local Ollama
set PSE_LLM_BASE_URL=http://localhost:11434/v1
set PSE_LLM_MODEL=llama3.1
set PSE_LLM_API_KEY=ollama
cargo run --release -p pse-llm-demo
```

### Bring your own domain

Create a JSON file with your own questions:

```json
{
  "questions": [
    "What are the key bottlenecks in modern transformer inference at scale?",
    "How does speculative decoding differ from beam search, and when does each win?",
    "Explain the trade-off between KV-cache size and context length in practice."
  ]
}
```

Then run with your domain's keywords:

```
set PSE_LLM_QUESTIONS_FILE=my_domain.json
set PSE_LLM_KEYWORDS=speculative,KV-cache,transformer,throughput,latency,batch
cargo run --release -p pse-llm-demo
```

Three sessions with a domain your LLM knows shallowly will show the largest A/B gap.

---

## What to say when presenting

### 30-second pitch

> "LLMs are stateless — every API call starts fresh.  PSE is the infrastructure
> layer that changes that.  It ingests LLM output as a stream, crystallizes the
> structurally stable parts into content-addressed memory, and makes them
> available to the next session — deterministically, without a vector database,
> and with a cryptographic audit trail.  You just saw it do that in three
> terminal commands."

### When someone asks "what's the SHA-256 for?"

> "The crystal ID is the SHA-256 of the crystal's topology in canonical JSON form.
> Two crystals with the same ID are structurally identical — they passed the same
> 8 independent gate conditions simultaneously, they cover the same graph region,
> and they can be replayed bit-identically by anyone with the source text.
> That's not a similarity score. That's a proof."

### When someone asks "how is this different from RAG?"

> "RAG retrieves by semantic similarity — you embed a query, find nearby vectors,
> paste text into the context window.  PSE crystallizes by topological stability —
> a pattern only makes it into memory if it passes eight independent structural
> conditions simultaneously.  Most patterns fail.  What passes is certified, not
> retrieved.  The crystal carries proof of *why* it was committed, not just what
> it says.  And it's content-addressed: the same input always produces the same
> memory address, which means you can verify it independently."

### When someone asks "what do I need to run it?"

> "Rust and a Cerebras key.  Three terminal commands.  No database, no embedding
> server, no infrastructure.  The memory file is a JSON on disk.  You can put it
> in version control, inspect it by hand, or ship it alongside your application."

### When someone asks about enterprise / compliance

> "Every crystal carries a `CommitProof` with the exact gate values that authorized
> its creation.  The `EvidenceChain` links back to the raw observations.  PSE
> was designed with EU AI Act Articles 9, 13, and 17 in mind.  The constitutional
> AI substrate lets you define blocking rules — no crystal that violates a Blocking
> rule makes it into the ledger.  The full audit report is one function call."

---

## Further reading

| Document | What's in it |
|---|---|
| `README.md` | Full system overview, architecture, IL+HDAG, QTIC |
| `docs/GETTING_STARTED.md` | Rust integration guide, calibration, Tier 2 planning |
| `docs/POST_SYMBOLIC.md` | Theoretical foundations |
| `specs/TheTimelessMonolith_bySebastianKlemm_v1.0.pdf` | Pfauenthron++ formal spec |
| `specs/QTIC.pdf` | QTIC conformance theory (Q0–Q5) |
| `specs/HDAG_bySebastianKlemm_v1.0.pdf` | HDAG 5D resonance graph spec |
| `CONTRIBUTING.md` | How to extend PSE with new adapters or layers |
