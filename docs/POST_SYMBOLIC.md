# Post-Symbolic Computation

This document defines **post-symbolic computation** as an independent
computational paradigm — independent of the PSE codebase, of any
specific implementation, of any specific application.

It is intended for readers who want to know:

1. What the category *is* (positively, not just "not symbolic").
2. How it sits relative to the four paradigms it succeeds.
3. The five primitives that constitute it.
4. The contract it offers (input → output, with guarantees).
5. What kinds of problems it is built for, and what it explicitly is not.
6. Where it connects to existing systems.

PSE (the engine in this repository) is **one** instantiation of this
paradigm. The paradigm is bigger than the implementation.

---

## 1. The Four Computational Paradigms It Succeeds

| Paradigm | Primitive | Truth condition | Limit |
|---|---|---|---|
| **Symbolic** (Lisp, Prolog, theorem provers) | Token / formula | Provable from axioms | Brittle outside the formal model; brittle on noisy reality |
| **Statistical** (regression, Bayesian inference, classical ML) | Distribution | Likelihood / posterior probability | Assumes a stable distribution; opaque to structural change |
| **Neural** (deep learning, LLMs) | Activation gradient | Empirical loss minimised on a training set | Non-deterministic; unverifiable; non-falsifiable; non-replayable |
| **Signal-theoretic** (DSP, FFT, wavelets) | Frequency component | Spectral decomposition matches a target band | Domain-specific; struggles with non-stationary structure |

Each paradigm represents reality through a different *primitive*. Each
primitive has a class of problems for which it is the right tool — and a
class for which it is structurally wrong.

The four leave a gap. None of them produce **a verifiable, content-addressed,
replayable record of structural events in a non-stationary stream** as
their native output. Symbolic systems can't see a stream; statistical
systems flatten structure into distribution; neural systems are black-boxes;
signal-theoretic systems decompose, they don't *commit*.

Post-symbolic computation fills that gap.

---

## 2. The Definition

> **Post-symbolic computation** is a computational paradigm whose primitive
> is the **resonant configuration** of a topologically-embedded observation
> stream, and whose native output is a **content-addressed, falsifiable,
> deterministically reproducible artifact** that records *which structural
> configuration occurred, under which constraints, with which provenance*.

Five clauses, all load-bearing:

* *resonant configuration* — not a token, not a distribution, not a
  gradient, not a frequency: a topological alignment between the substrate
  carrying the stream and a reference carrier geometry.
* *topologically-embedded observation stream* — observations are not
  scalars or vectors but vertices in an evolving graph carrying a
  geometric embedding.
* *content-addressed* — the artifact's identity *is* the SHA-256 of its
  canonical (JCS) serialisation. Two artifacts with the same content
  have the same identity; two with different content cannot.
* *falsifiable* — the artifact carries an empirical p-value against a
  stated null model (shuffle, block-bootstrap, phase-randomise) over a
  configurable surrogate set.
* *deterministically reproducible* — given the same input stream and
  configuration, the engine emits bit-identical artifacts in the same
  order. Replay is verification.

A system has post-symbolic capability if and only if all five clauses are
true of its output contract. Anything missing one clause is in one of
the four predecessor paradigms.

---

## 3. The Five Primitives

A post-symbolic system is built from five composable primitives. Each
has a precise mathematical role; each can be implemented multiple ways.

### 3.1 The 5D state

Every observation is embedded into a five-dimensional state
`(p, ρ, ω, χ, η)`:

* **p — potential**: the observation's position on a normalised scalar axis.
* **ρ — density**: local density of the observation in topological neighbourhood.
* **ω — frequency**: phase rate; the observation's place in any periodic structure.
* **χ — connectivity**: degree of the observation's vertex in the substrate graph.
* **η — causality**: directional coupling strength to neighbouring observations.

The 5D state is *physically grounded in the graph*: η is not a constant,
it is computed from the local edge structure. This is the difference
between embedding and merely indexing.

### 3.2 The carrier

A reference geometry against which observations are measured for
resonance. The canonical carrier is a **helix-pair** at antiphase
(Δφ = π), forming a standing wave whose axis is the resonance axis.

Alternative carriers can be substituted (single helix, n-helix bundles,
non-helical lattices) but the *contract* — that the carrier defines a
phase reference and an amplitude reference — is invariant.

### 3.3 The Mandorla coherence κ

The interference of an observation-derived **data helix** with the
carrier:

```
κ = κ_carrier · phase_lock(Δφ_data, Δφ_carrier) · amp_match(r_data, r_carrier)
```

Each factor lies in `[0, 1]`. The product is the *fraction of the
carrier's intrinsic coherence preserved by the data*. κ is the engine's
direct measurement of "is the structure resonant".

In PSE this is `pse_cascade::mandorla_real`. Other instantiations are
free to choose different interference functions; the requirement is
only that κ is bounded, multiplicatively decomposable, and zero only
at orthogonality.

### 3.4 The conjunctive gate (Kairos)

Eight metrics, all derived from the substrate state, are combined by
**logical AND**:

* `d` — deformation (how much the embedding moved)
* `q` — coherence (κ as a fraction of carrier maximum)
* `r` — resonance (proximity to a reference resonance state)
* `g` — readiness (a linear combination of d, q, r)
* `j` — double-kick (graph-density signature)
* `p` — projection (state-norm stability)
* `n` — seam (Mandorla phase-difference coherence)
* `k` — crystal score (κ-weighted entropy bound)

The conjunction is intentional. Each metric measures a different
*aspect* of structural alignment; their AND ensures that no single
metric can carry the decision. A configuration that hits all eight
simultaneously is, by construction, structurally aligned along
multiple independent axes — which is what makes it a candidate for
crystallisation rather than noise.

### 3.5 The crystal artifact

The output. Five mandatory fields:

* **content address** — SHA-256 of the canonical (JCS) serialisation
  of the artifact, *minus the address itself*.
* **region** — the set of substrate vertices that produced the
  resonance.
* **constraint program** — the set of structural constraints active
  on the region (extracted via inverse-weave / DoF analysis).
* **provenance envelope** — origin, schema version, timestamp,
  measurement context, observation chain.
* **commit proof** — the gate snapshot at commit time, optionally
  the surrogate-falsification p-value.

A crystal is *immutable*, *byte-comparable*, *replay-verifiable*. Two
runs of the same engine on the same inputs with the same configuration
produce byte-identical sequences of crystals.

---

## 4. The Contract

A post-symbolic system offers four guarantees on its output:

| Guarantee | What it means |
|---|---|
| **Determinism** (Inv I4) | Given (input stream, configuration), the system emits a unique, byte-identical sequence of crystals. Replay = verification. |
| **Content-address integrity** | `crystal_id` ≡ `SHA-256(JCS(crystal \ {crystal_id}))`. Tampering breaks the address. |
| **Falsifiability** | Each crystal carries (or can carry) an empirical p-value against a stated null model. The truth claim is rejectable in finite time on stated evidence. |
| **Auditability** | Every crystal carries a complete provenance envelope. Every gate decision can be replayed from the input stream. |

Together these four are the **post-symbolic guarantee**: the output is
not just a number or a label, it is *a document about a structural
event* that is independently verifiable, independently falsifiable,
and independently replayable.

This is what no statistical, neural, or signal-theoretic system
provides as its native output contract.

---

## 5. What Post-Symbolic Is For

The category is built for **structural-event detection in non-stationary
streams**, with audit-grade output. Concrete problem classes where the
post-symbolic contract is the right tool:

| Domain | Question post-symbolic answers |
|---|---|
| Medical telemetry | "An arrhythmia just happened — replay the ECG context, prove it wasn't surrogate noise." |
| Financial markets | "A regime shift in volatility just occurred — the constraint program is *this*, the falsifier rejects shuffle at p < 0.01." |
| Industrial IoT | "A bearing's vibration signature just bifurcated — the crystal carries the topology of the new attractor." |
| Cybersecurity | "An attack pattern crystallised across 14 syslog sources — here is the cross-source resonance and its evidence chain." |
| Seismology | "A foreshock pattern is forming — the topology matches Crystal #FB12 from the 2023 archive (pattern-memory hit)." |
| Compliance / audit | "Show me the immutable record of every structural decision the system made, with falsifier evidence, between time T₁ and T₂." |

In each case, the question requires more than a number ("anomaly score
0.74") — it requires *a verifiable, replayable, falsifiable artifact
about a structural event*. That is exactly what post-symbolic
computation produces.

---

## 6. What Post-Symbolic Is Not

Post-symbolic computation is **not**:

* A replacement for symbolic reasoning. Theorem provers, constraint
  solvers, and formal-method verifiers are not displaced by it.
* A replacement for statistical inference. Estimating a population
  parameter is not what crystals do.
* A replacement for trained models. LLMs / CNNs / transformers are
  not post-symbolic; they fail the *deterministic* and *falsifiable*
  clauses by construction.
* A replacement for signal processing. FFT and wavelet decompositions
  remain the right tool for spectral analysis on stationary signals.
* A general-purpose programming model. It is not Turing-complete in
  the colloquial sense — it is a *measurement-and-record* model, not
  a compute-arbitrary-function model.

A useful slogan: **post-symbolic computation answers "what just
happened, structurally, and how do I prove it"** — not "what is the
right answer", "what's the next token", or "what's the population
mean".

---

## 7. Composition: The Operator Algebra

Crystals are not a dead-end output; they participate in an algebra.
Five operators close the category:

| Operator | Signature | Meaning |
|---|---|---|
| **compose** | `(C₁, C₂) → C₃` | Synchronisation product — the crystal of two streams resonating together |
| **dual** | `C → C*` | Topology-preserving inversion — yields the dual constraint program on the same region |
| **bridge** | `(C₁, C₂) → B` | Meso-bridge — the crystal of the *interaction* between two crystals sharing substrate |
| **query** | `(C, fingerprint) → bool / score` | Resonance fingerprint test — does this crystal match a stored signature? |
| **interpolate** | `(C₁, C₂, t) → C_t` | Generative interpolation — synthesise a hypothetical crystal between two known ones |

The algebra is what turns a sequence of detections into a *theory* of
the stream: composition aggregates, duality reveals symmetry, bridges
expose cross-modal structure, queries enable cross-session memory, and
interpolation enables hypothesis generation.

These operators are total over the crystal type; their composition
remains within the crystal type; therefore the category is closed.

---

## 8. Where It Connects (Anschlussfähigkeit)

Post-symbolic computation is not a closed silo. It connects upstream
and downstream to the existing computational landscape:

**Upstream (data sources).** Any sensor, log stream, market feed, or
telemetry source that produces a temporally-ordered sequence of
observations is a valid input. The adapter trait surface is two
methods — `canonicalize` and `source_id`. PSE ships ten adapters
already; writing a new one is a half-page of code.

**Sideways (statistical / ML systems).** Crystals are JSON-serialisable
content-addressed records. Any anomaly-detection or alerting system
that consumes events can consume crystals. Conversely, statistical
features can be fed into the engine via `Observation::phase_hint` —
the engine treats them as semantic phase information rather than
deriving phase from a SHA hash.

**Downstream (audit, replay, governance).** Crystals satisfy the EU
AI Act's transparency and reproducibility requirements as a matter of
construction (see `docs/COMPLIANCE.md`). They drop directly into
audit pipelines, immutable-ledger stores, or regulatory archives
without further processing.

**Across (other post-symbolic systems).** Two post-symbolic engines
that agree on canonical serialisation can exchange crystals losslessly.
The content address is the global identity. This is what makes the
paradigm composable across organisations, not just within one engine.

---

## 9. The Falsifier

A non-trivial detail of the contract: **post-symbolic systems are
falsifiable as engines, not just as outputs**.

The falsifier (`pse_core::falsify::falsify_with_surrogates`) takes
the same observation batch that just produced a crystal, generates
*k* surrogate batches under a stated null model — Shuffle (destroys
order), BlockBootstrap (destroys long-range structure but preserves
local), PhaseRandomize (destroys phase coherence but preserves
amplitude spectrum) — and re-runs the engine on each. The empirical
p-value is

```
p = (1 + #{surrogates whose max κ ≥ real-batch max κ}) / (1 + k)
```

A crystal whose `commit_proof.falsification_p_value` exceeds α (typical
0.05) is rejected. A crystal whose p-value passes is committed *with
the p-value attached* — the truth claim is published with its own
falsifiability evidence.

This is Popper's epistemology built into the output type. A
post-symbolic system that does not carry falsifier evidence on its
crystals is not in the paradigm; it is statistical inference in
crystal-shaped clothing.

---

## 10. The Smallest Possible Implementation

To make the paradigm concrete, here is the minimum surface a system
must implement to be in the post-symbolic category:

```text
Input:    ordered stream of (timestamp, payload, source_id) tuples.

State:    persistent graph G with 5D embedding;
          carrier C (helix-pair or equivalent);
          eight metrics d, q, r, g, j, p, n, k as functions of (G, C, payload);
          gate threshold vector τ.

Step:     for each batch b in the stream:
            1. Project b onto G (vertex / edge mutations).
            2. Compute κ = mandorla(C, b).
            3. Compute (d, q, r, g, j, p, n, k).
            4. If all eight ≥ their thresholds:
                 a. Extract constraints over the resonant region.
                 b. Run dual-cascade consensus on the precursor.
                 c. (Optional) Falsify against k surrogates → p-value.
                 d. If consensus + (optional) falsifier pass:
                      build crystal,
                      content-address it,
                      append to evidence chain,
                      emit.

Output:   sequence of content-addressed, falsifier-attested,
          replay-verifiable crystals.
```

That is the entire contract. Everything in PSE — the operator algebra,
the multi-scale state, the TRITON navigator, the swarm consensus, the
adapters — is *additional capability* on top of this minimum.

A correct post-symbolic system need not be large. PSE is large because
it is general; a domain-specific post-symbolic detector can be much
smaller.

---

## 11. The One-Sentence Test

If a system claims to be post-symbolic, ask:

> "Show me a crystal, recompute its SHA-256 from its canonical
> serialisation, replay the input stream that produced it, and show me
> the surrogate p-value attached to its commit proof."

A system that can do all four steps in finite time, on demand, on any
of its outputs, is post-symbolic.

A system that cannot is in one of the four predecessor paradigms,
regardless of its marketing.

---

## 12. Where to Read Further

Within this repository:

* `docs/COMPLIANCE.md` — EU AI Act formal compliance proof sketch.
* `crates/pse-core/src/lib.rs::macro_step` — the canonical
  implementation of step 1–4 above.
* `crates/pse-core/src/falsify.rs` — the surrogate-data falsifier.
* `crates/pse-evidence/src/lib.rs` — content-address and evidence
  chain.
* `crates/pse-core/src/topology_ops.rs` — the operator algebra
  (compose / dual / bridge / query / interpolate).
* `crates/pse-core/src/adaptive.rs` — the rolling-quantile threshold
  calibrator.

External references the paradigm builds on (no specific edition
mandated):

* Karl Popper — *The Logic of Scientific Discovery* (falsification).
* Mathias Fritsche & Howard Klar — phase-randomised surrogate methods.
* The IETF JCS / RFC 8785 — deterministic JSON serialisation.
* The EU AI Act, Art. 12–15 — transparency, traceability, record-keeping.
* Roger Penrose — *The Road to Reality* (helix and standing-wave geometry).

---

*This document is paradigm-level. It does not change when PSE's
implementation changes. PSE evolves; the post-symbolic contract is
fixed.*
