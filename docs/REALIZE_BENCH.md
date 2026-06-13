# The realization benchmark — does the generative loop actually work?

Every other gate in this repository is deterministic and offline. They
prove the substrate, the scaffolder, the wiring. **None of them answers
the one question that decides whether the whole "wish → system" claim is
real:** when a prose-level wish needs behaviour the deterministic
scaffolder cannot produce, can a real model produce code that *works*?

`kosmo-run --realize-bench` is the instrument that measures it — and
nothing was known about this empirically before it existed. This is the
gate before any GUI is even in question.

## What it does

It drives a curated corpus of behavioural wishes (`echo`, `add`,
`maximum`, `reverse`, `uppercase`, `count-vowels`, `factorial`,
`fibonacci`, `gcd`, `sum-list`) through the **same provider-backed
descent the operator uses**. Each task is a wish of budgeted `Run`
facets the scaffolder cannot satisfy — a `Run` facet needs real logic,
not a stub — so the descent must call the real provider. Every task uses
**multiple probes** (e.g. `add`: 3+4=7, 10+32=42, 0+0=0), so a program
that prints one memorized answer fails the others: the model must
generalize, not hard-code.

The verdict is **execution, never the model's word**: the forged program
is run under the sandbox witness and its output and exit code are matched
against the probes. It reports, per task and in aggregate, the
**realization rate**, the **descent iterations**, and the **real token
cost** (measured by a counting wrapper around the provider).

It is a **measurement, not a gate** — completion always exits 0; the rate
is the finding.

## Running it (provider-agnostic)

The benchmark does not care whether the model is local or remote — it
arms whatever `build_synthesizer` arms. The same corpus, the same
numbers, comparable across models.

**Cloud API:**

```sh
ANTHROPIC_API_KEY=sk-... cargo run -p kosmo-run -- \
    --provider claude --realize-bench --realize-bench-report bench.json
# or CEREBRAS_API_KEY=... --provider cerebras
```

**Local model** (Ollama, llama.cpp, vLLM, LM Studio — anything that
speaks OpenAI-compatible `/chat/completions`):

```sh
KOSMO_LLM_PROVIDER=openai \
KOSMO_LLM_BASE_URL=http://localhost:11434/v1 \
KOSMO_LLM_MODEL=qwen2.5-coder:7b \
KOSMO_LLM_API_KEY=ollama \
  cargo run -p kosmo-run -- --provider env --realize-bench \
    --realize-bench-report bench-local.json
```

`--model <slug>` overrides the model; the chosen provider and model are
recorded in the report. The report is content-addressed
(`report_id` = SHA-256 of the body) — tamper-evident, though not a
reproducibility claim, since model output is non-deterministic.

`--provider mock` is **refused**: a benchmark of the deterministic
scaffolder measures only what the Prüfstand already proves.

## What it measures, and what it does not

- **Measures:** synthesis-to-spec — given a precise, multi-probe
  behavioural target, the fraction the model realizes (by execution),
  the effort (iterations), and the cost (tokens), per model.
- **Does not measure:** prose→spec compilation. The corpus carries
  well-formed facets directly, isolating the generative claim from the
  front-door grammar / wish-LLM (a separate axis, a later instrument).
- **Does not claim:** generality beyond the corpus class
  (single-binary, argv→stdout behaviours). The corpus grows like every
  vocabulary here; the instrument is the durable part.

## How to read a result

`realized 7/10 (70.00%)` with a model means: of ten behavioural targets
the scaffolder could not touch, the model drove seven to a program that
**actually runs correctly on every probe**. A low rate is not a failure
of the bench — it is the honest news about that model on this loop, which
is exactly what has to be known before the surface is dressed up.
