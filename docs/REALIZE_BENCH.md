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

It drives a curated corpus of behavioural wishes through the **same
provider-backed descent the operator uses**. Each task is a wish of
budgeted `Run` facets the scaffolder cannot satisfy — a `Run` facet
needs real logic, not a stub — so the descent must call the real
provider. Every task uses **multiple probes** (e.g. `add`: 3+4=7,
10+32=42, 0+0=0), so a program that prints one memorized answer fails
the others: the model must generalize, not hard-code.

The corpus is **tiered by difficulty**, so one run yields a spread, not
a single number:

- **floor** (10) — echo, add, reverse, factorial …: does the loop
  conduct, and can a model hit trivial targets at all?
- **rung** (8) — palindrome, ROT13, base conversion, anagram, Collatz
  steps …: moderate logic a competent model should mostly clear.
- **ceiling** (5) — Roman numerals, nth prime, balanced parentheses,
  run-length encoding, precedence expression evaluation: where a real
  engine is discriminated from a weak one.

These tiers are *harder rungs*, not the paradigm summit (whole
multi-component systems from prose) — that is a larger instrument still
ahead.

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

The spread is the point. A run prints per tier, e.g.:

```
floor    10/10 (100.00%)
rung      6/8  (75.00%)
ceiling   2/5  (40.00%)
realized 18/23 (78.26%) · … iterations · … tokens total
```

That means: the model cleared every trivial target, most of the moderate
ones, and two of the five hard ones — each verified by **executing the
forged program on every probe**. A low ceiling with a high floor is the
honest shape of "competent but not yet at the top"; a low floor would
mean either a weak model or a loop problem (the local stand-in pre-flight
rules out the latter). The verified baseline: an *echo-only* stand-in
scores **1/23 (4.34%)** — it can do exactly one thing — so any real model
is read against that floor.

Successful tasks are cheap (the model realizes them in one to a few
descent iterations); only tasks the model genuinely cannot do burn the
full 8-iteration budget. So cost scales with difficulty, and a capable
engine spends most of its tokens on the few rungs it can't yet reach.

## One command (the fresh-session path)

When the provider is configured in the environment (full egress + a secret
`KOSMO_LLM_API_KEY`) and a **fresh session** has started so it takes effect:

```sh
scripts/fire-realize-bench.sh
```

It validates the key and model name with one tiny call first (so a typo
costs nothing), then fires the full tiered benchmark and writes `bench.json`.
Override the engine without editing anything: `KOSMO_LLM_MODEL`,
`KOSMO_LLM_PROVIDER`, `KOSMO_LLM_BASE_URL` are read from the environment
(defaults target OpenAI). The key is read from the environment and never
printed.
