# pse-core — Python Bindings

Python bindings for **PSE (Post-Symbolic Engine)** — a thermodynamic cognitive
substrate that accumulates structured memory across LLM sessions.

Built with [PyO3](https://pyo3.rs) and [maturin](https://maturin.rs).

## What you get

```python
import pse_core

# Session 1 — cold start
state = pse_core.PseState()
crystals = state.process_text(llm_response)   # ingest an LLM response
memory = state.save_memory()                  # persist to JSON

# Session 2 — warm start: prior patterns are available instantly
state = pse_core.PseState(memory_json=memory)
state.process_text(same_llm_response)
print(state.pattern_hits())  # > 0: PSE recognised prior topology
```

**`PseState`** — the substrate. Stateful, not thread-safe per instance.  
**`PseCrystal`** — a content-addressed, deterministic topology snapshot.

## Install

### From source (recommended until PyPI release)

```bash
# 1. Install maturin
pip install maturin

# 2. Build and install into your active venv (editable)
cd bindings/python
maturin develop --release

# 3. Verify
python -c "import pse_core; print(pse_core.PseState())"
```

### Build a wheel

```bash
maturin build --release
pip install target/wheels/pse_core-*.whl
```

## API

### `PseState(memory_json=None, source_name=None)`

Create a PSE state. Pass `memory_json` (from a previous `save_memory()`) for
a warm start.

| Method | Returns | Description |
|---|---|---|
| `step(data: bytes)` | `PseCrystal \| None` | One raw observation |
| `step_text(text: str)` | `PseCrystal \| None` | One text observation |
| `process_text(text: str)` | `list[PseCrystal]` | Full LLM response (recommended) |
| `save_memory()` | `str` | JSON — pass to next session's `__init__` |
| `crystals()` | `list[PseCrystal]` | All accumulated crystals |
| `pattern_hits()` | `int` | Cross-session recognition events |
| `commit_index()` | `int` | Monotone tick counter |
| `crystal_count()` | `int` | Total crystals in state |

### `PseCrystal`

| Attribute | Type | Description |
|---|---|---|
| `id` | `str` | 64-char hex — content-addressed, deterministic |
| `stability` | `float` | Stability score in [0, 1] |
| `region_size` | `int` | Topological region vertex count |
| `to_json()` | `str` | Full crystal as JSON |

## Examples

```bash
# Minimal: cold→warm cycle without an LLM
python examples/minimal.py

# Full LLM demo (Cerebras default, any OpenAI-compatible endpoint works)
PSE_LLM_API_KEY=<key> python examples/llm_session.py
PSE_LLM_API_KEY=<key> python examples/llm_session.py   # run twice: see hits!

# OpenAI
PSE_LLM_BASE_URL=https://api.openai.com/v1 \
PSE_LLM_MODEL=gpt-4o-mini \
PSE_LLM_API_KEY=sk-... \
python examples/llm_session.py

# Ollama (local, no key needed)
PSE_LLM_BASE_URL=http://localhost:11434/v1 \
PSE_LLM_API_KEY=ollama \
PSE_LLM_MODEL=llama3.1 \
python examples/llm_session.py
```

## Integration with any LLM SDK

```python
import pse_core
import anthropic  # or openai, or any other SDK

client = anthropic.Anthropic()
state  = pse_core.PseState(memory_json=load_memory())

# Every response feeds the substrate
message = client.messages.create(
    model="claude-opus-4-7",
    max_tokens=1024,
    messages=[{"role": "user", "content": question}],
)
response_text = message.content[0].text
crystals = state.process_text(response_text)

# Persist for next session
save_memory(state.save_memory())
```

## What is PSE?

PSE operates on the *topology* of information flow, not its symbolic content.
At its core it is thermodynamic computation:

- **Kairos gate** — selects the top 20 % of ticks by information density
  (a Maxwell's demon analogue)
- **SemanticCrystals** — stable low-entropy structures, analogous to physical
  phase transitions
- **PatternMemory** — a catalogue of topological macrostates, model-agnostic
  and session-persistent
- **Cross-session proof** — identical text → identical observation graph →
  same canonical class → memory hit (`pattern_hits() > 0`)

The substrate is universal: it works with any model, any domain, any language.
