# The Wish-to-System Machine — CAD/CAM for Software

- **Status:** Reference — describes implemented behavior of the `kosmo-*`
  intent→materialization layer. Cross-checked against the source on
  2026-06-10.
- **Scope:** The closed loop that turns a stated intent (a *wish*) into a
  validated change to a real workspace. This is the layer *above* the
  topology substrate documented in [`../SUBSTRATE.md`](../SUBSTRATE.md).
- **Audience:** Anyone who has read `SUBSTRATE.md` and wants to know how the
  diagnostic substrate becomes a *generative* one.

> Until now this machine was documented only as dated development-log entries
> (`IMPLEMENTATION_STATUS.md`, `CHANGELOG.md`) and assumed-as-known by the
> floor specs ([`HORIZON-behavior-archetype.md`](HORIZON-behavior-archetype.md),
> [`RUNTIME-floor.md`](RUNTIME-floor.md)). This document is the consolidated
> reference for the loop as a whole.

---

## 1. The metaphor: CAD/CAM for software

Mechanical CAD/CAM has two halves. **CAD** (computer-aided *design*) captures
intent and measures a part against it. **CAM** (computer-aided *manufacturing*)
drives a machine that cuts metal until the measured part matches the design —
under interlocks that stop the spindle the instant something is out of
tolerance.

Kosmocrates applies the same split to software:

| CAD/CAM | Kosmocrates | Crates |
|---|---|---|
| **Design intent** | `Wish` — the desired topology, made explicit and measurable | `kosmo-core::wish`, `kosmo-intent`, `kosmo-intent-llm` |
| **Metrology** (measure part vs. design) | `assess_wish` — `Q16` distance + the unmet-facet gradient | `kosmo-core::wish`, `kosmo-intent` |
| **CAM toolpath** (plan the cuts) | the pipeline's energy-ranked `ActionItem` queue | `kosmo-pipeline` |
| **The cut** (remove/add material) | synthesize a `Patch`, then materialize it | `kosmo-synthesizer*`, `kosmo-materialize` |
| **In-process inspection** | sandbox execution + R1→R2→R3 validation closure | `kosmo-sandbox`, `kosmo-operator` |
| **Interlocks / E-stop** | `PolicyProfile` gates; fail-closed by default | `kosmo-core::policy` (everywhere) |
| **Closed-loop control** | re-observe, re-measure distance, iterate to *Realized* | `kosmo-agent` |

The substrate (`SUBSTRATE.md`) is the **CAD/metrology** half: it analyses
topology, finds structural voids, and ranks candidates — but writes nothing.
This document covers the **CAM** half: the governed path from a measured gap to
a validated change on disk.

The whole thing is one feedback controller over a **discrete distance
landscape**. The wish defines the target; the distance is the error signal; the
agent loop is the descent. The single non-deterministic step in the entire
machine is the optional LLM call that proposes a patch — and its output is
immediately re-hashed back into the deterministic, content-addressed world.

---

## 2. The six pieces

The loop is built from six load-bearing parts (the framing the dev log uses):

1. **Target** — `Wish` (`kosmo-core/src/wish.rs:273`). A content-addressed set
   of `WishPredicate`s over `WishFacet`s (a crate exists, a module exists, a
   symbol exists, a test passes, a program runs, a service responds).
2. **Ruler** — `assess_wish` (`kosmo-core/src/wish.rs:430`). Measures the
   `Q16` **distance** between an observed topology and the wish: the
   weighted fraction of unmet predicates. `ZERO` = realized, `ONE` = nothing met.
3. **Contract** — `WishClosureStatus` (`Realized | Approaching | Unstarted |
   Vacuous`). Drives loop control; `Realized` is the halt condition.
4. **Observation** — `kosmo-intent::observe_workspace*`
   (`kosmo-intent/src/lib.rs:64`). Reads a *real* workspace into an
   `ObservedTopology` (a facet set) via `cargo metadata`, source scan, tests,
   runtime probes, and service probes.
5. **Governance** — the `PolicyProfile` gates at every layer, and the agent
   loop's descent discipline: it acts only toward closing `unmet_facets` and
   halts when the wish is `Realized` or the step budget is exhausted.
6. **Generation** — the synthesizer (`kosmo-synthesizer*`) turns an
   `ActionItem` into a `Patch`; the materializer (`kosmo-materialize`) applies
   it under policy.

`distance` is a **ranking quantity only — it gates nothing** (the CROSS-010
doctrine applied to intent). Driving it to zero is the loop's job; it can never
bypass a policy gate.

---

## 3. Crate map (the parts of the machine)

All entry points below are verified against the source.

| Crate | Role | Main entry point | LLM? |
|---|---|---|---|
| `kosmo-intent` | Observe a workspace; compile/track a wish | `observe_workspace` (`:64`), `compile_wish` (`:1125`), `WishSession` (`:1207`) | no |
| `kosmo-intent-llm` | Prose → `Wish` via an LLM (re-determinized) | `LlmWishCompiler` (`:25`) | **yes** |
| `kosmo-llm` | Shared LLM transport (Anthropic / OpenAI-compatible) | `LlmConfig` (`:29`), `LlmConfig::complete` (`:193`) | **yes** |
| `kosmo-pipeline` | Topology scan → energy-ranked `ActionItem` queue | `run_dry_pipeline`, `IntegrationRunReport` | no |
| `kosmo-synthesizer` | `ActionItem → Patch` trait + rule-based scaffolder | `ActionSynthesizer` (`:237`), `FacetScaffolder` (`:323`) | no |
| `kosmo-synthesizer-llm` | LLM-backed `ActionSynthesizer` | `LlmSynthesizer` (`:240`) | **yes** |
| `kosmo-materialize` | Policy-gated patch application + validation/rollback | `Materializer::materialize` (`:407`), `PatchValidator` (`:110`) | no |
| `kosmo-sandbox` | Isolated execution → content-addressed `RuntimeWitness` | `Sandbox` (`:129`), `RuntimeWitness` (`:94`) | no |
| `kosmo-operator` | R1→R2→R3 validation-closure orchestrator | `OperatorExecutor::execute` (`:179`), `OperationPlan` (`:49`) | no |
| `kosmo-pse-bridge` | Offer `kosmo-*` artifacts to PSE as candidates | `PseBridgeCandidate` (`:48`) | no |
| `kosmo-agent` | The closed loop: plan → synth → validate → observe → feedback | `AgentSession::run` (`:508`), `AgentRunReport` (`:275`) | indirect |
| `tools/kosmo-run` | CLI/TUI/server driver for the whole loop | `main` | optional |

**Only the three `*-llm` crates introduce non-determinism**, and only at the one
boundary described in §5. Everything else — observation, ranking, synthesis via
the rule-based scaffolder, materialization, sandboxing, validation — is
deterministic and replayable.

---

## 4. The end-to-end pipeline

A single agent iteration (`AgentSession::run`) is:

```
   prose ──(0)──▶ Wish ──┐
                         │   (target, content-addressed)
 workspace ─(1)─▶ ObservedTopology
                         │
                  (2) assess_wish  ──▶  distance: Q16  +  unmet_facets[]
                         │
                  (3) run_dry_pipeline ──▶ IntegrationRunReport
                         │                   (HYPHAE · Metatron · LPCM · SystemCube,
                         │                    energy-ranked; see SUBSTRATE.md)
                  (4) ActionItem queue  ◀── unmet_facets prepended (wish-directed)
                         │
                  (5) ActionSynthesizer::synthesize ──▶ Patch  (+ Q16 confidence)
                         │                                  ▲ sole non-determinism (LLM)
                  (6) confidence ≥ min?  ──no──▶ skip
                         │ yes
                  (7) Materializer::materialize(patch, policy, validator)
                         │     ReportOnly → SkippedByPolicy (no I/O)
                         │     DryRun     → sandbox copy, apply, validate, discard
                         │     OperatorApproved+allow_host_write → backup, apply,
                         │                    validate, rollback on failure
                  (8) OperatorExecutor::execute  ──▶ R1 snapshot · R2 foundry · R3 diff
                         │                            → ValidationClosureReport
                  (9) re-observe ──▶ new distance ──▶ ExecutionFeedback
                         │
                 (10) Realized?  ──yes──▶ halt        ──no──▶ loop to (3)
```

Every boundary object is content-addressed (`id = SHA-256(JCS(content))`), every
gate-relevant number is `Q16` (no floats — CROSS-007), and every durable record
is evidence-bound (CROSS-006). Identical inputs replay to byte-identical outputs
— except across the LLM boundary, which is re-determinized immediately (§5).

---

## 5. The one non-deterministic boundary

The machine is deterministic everywhere except **patch synthesis** (stage 5) and
the optional **prose→`Wish`** compilation (stage 0), when an LLM backend is
selected. Three properties keep this honest:

1. **Re-determinization at the seam.** An LLM returns free text; the synthesizer
   parses it into a `Patch` whose `patch_id = SHA-256(request_id ‖ sorted file
   changes)`. From that point on the artifact is fully content-addressed — the
   model's prose never enters a hashed or gated path.
2. **Confidence is `Q16`, not a float.** A model reports an integer percentage;
   it becomes `Q16::ratio(pct, 100)`. Sampling temperature lives only in the
   request body, never in a content-addressed structure.
3. **The LLM proposes; policy disposes.** A synthesized patch is still subject
   to the full materialization gate. Under the default `ReportOnly` policy it is
   `SkippedByPolicy` — written nowhere. Energy and confidence *rank*; they never
   *gate* (CROSS-010).
4. **Memory grounds, it does not gate.** With `--ledger <path>` each request is
   grounded in the anchored Infinity-Ledger knowledge (Pfauenthron recall per
   action; `kosmo_pse_bridge::MemoryRecall`): the top crystals ride along as a
   clearly-delimited advisory prompt section, and the resulting patch **cites**
   the crystal IDs it received (`grounding_crystal_ids`) — auditable provenance
   from a generated patch back to certified knowledge. Recall failures are loud
   (a memory explicitly attached must answer), and nothing recalled ever enters
   a gate or a content-addressed identity.

Swap the LLM for `FacetScaffolder` (`--scaffold`) or `MockSynthesizer` and the
**entire** loop becomes deterministic and offline — the mode the 147-scenario
evaluation harness runs in.

---

## 6. Running it

`kosmo-run` (`tools/kosmo-run`) drives the loop. Key flags (verified):

```bash
# Deterministic, offline wish mode — compile a wish and measure distance only:
kosmo-run --wish "a crate kosmo-api and a function handle" --scaffold .

# Persist the convergence trajectory as JSON:
kosmo-run --wish "<prose>" --wish-session trajectory.json .

# Agent mode with an LLM backend (proposes patches; still policy-gated):
ANTHROPIC_API_KEY=sk-... kosmo-run --provider claude --max-steps 3 ./crate

# Offline agent with the rule-based scaffolder (no key, no network):
kosmo-run --provider mock --all .

# WRITE validated patches (descends scaffold → write → re-observe until realized):
kosmo-run --wish "<prose>" --scaffold --apply .

# Apply and land each accepted patch as its own git commit:
kosmo-run --provider mock --apply --commit .

# Ground synthesis in the anchored memory (see SUBSTRATE.md §5 — first
# `kosmo-promote --ledger` learns, then this builds with what was learned):
kosmo-run --provider claude --ledger ~/.kosmo/il --ground-top 5 .

# Prüfstand — the built-in fidelity harness over a reference corpus:
kosmo-run --pruefstand
```

`--apply` is the only flag that writes to disk, and it still runs every patch
through the materialization validator (cargo check / test) before keeping it,
rolling back on failure.

---

## 7. Where this sits in the stack

```
   Wish-to-System loop          ← this document  (the CAM half)
 ├─ Horizon (Behavior+Archetype)  docs/HORIZON-behavior-archetype.md   (next floor)
 ├─ Runtime floor                 docs/RUNTIME-floor.md                (Run/Service facets)
 ─────────────────────────────────────────────────────────────────────
   Production substrate          ← SUBSTRATE.md  (the CAD/metrology half)
 ├─ HYPHAE · Metatron · LPCM · SystemCube · cross-language xlang
 ├─ kCube materialization · Foundry · Store · ParseBack · Operator
 ─────────────────────────────────────────────────────────────────────
   PSE base engine               ← README.md / docs/OVERVIEW.md
```

- The **substrate** ([`../SUBSTRATE.md`](../SUBSTRATE.md)) is the design/metrology
  half: it measures topology and ranks candidates, fail-closed, writing nothing
  on its own. Cross-language extraction (the `xlang` module) means the metrology
  now covers Rust, Python, JavaScript, and Go.
- The **floor specs** ([`HORIZON-behavior-archetype.md`](HORIZON-behavior-archetype.md),
  [`RUNTIME-floor.md`](RUNTIME-floor.md)) extend the *vocabulary* of facets this
  loop can target — from "a symbol exists" toward "it behaves correctly" and "the
  service responds".
- The **dev log** (`../IMPLEMENTATION_STATUS.md`, `../CHANGELOG.md`) records how
  each rung of the loop was built, run by run.

---

## 8. Invariants this loop must not break

| Invariant | Meaning here |
|---|---|
| **Fail-closed by default** | Default `PolicyProfile` is `ReportOnly`: synthesis runs, materialization is `SkippedByPolicy`. A host write requires `OperatorApproved` + `allow_host_write`. |
| **Energy/distance ranks, never gates** (CROSS-010) | Wish distance and synthesis confidence reorder work; they never flip a policy decision. |
| **No floats in gate paths** (CROSS-007) | Distance, confidence, energy are all `Q16`. Temperature is request-body only. |
| **Evidence-bound** (CROSS-006) | Every `Wish`, assessment, patch, and report carries a non-zero `evidence_bundle_id`. |
| **Deterministic replay** | Identical inputs → byte-identical outputs, except the LLM seam, which is re-hashed at the boundary. |
| **Validated before kept** | An applied patch that fails `cargo check`/`test` is rolled back; the host is restored from backup. |
