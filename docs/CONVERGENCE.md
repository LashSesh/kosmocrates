# Layer Convergence — the echo structures and where they meet

- **Status:** Architecture reference — describes implemented state plus the
  named convergence paths. Cross-checked against the source on 2026-06-11.
- **Scope:** The deliberately mirrored structures between the PSE cognition
  layer (`pse-*`) and the Kosmocrates production substrate (`kosmo-*`), what
  has already converged through the unification, what is scheduled to, and
  what must never converge.

The two layers instantiate the same post-symbolic paradigm
([`POST_SYMBOLIC.md`](POST_SYMBOLIC.md)) on two streams — world data and
software itself. Because both were built to the same doctrine, several
structures exist twice, once per layer. These are **echoes, not accidents**:
each pair shares a concept and diverges exactly where its stream demands.
This document is the map, so nobody mistakes an echo for duplication — or
"cleans up" a distinction that is load-bearing.

## The pairs at a glance

| Concept | PSE layer | kosmo layer | Status |
|---|---|---|---|
| Selection energy | Pfauenthron retrieval `D = ψ·ρ·ω` (`f64`) | `kosmo-core::energy` `D = ψ·ρ·ω` (`Q16`) | **converged vocabulary**, dual arithmetic by design |
| Crystal | `SemanticCrystal` (observation-born) | `StructuralCrystalRecord` (certificate-born) | **connected** — promotion path, both memory directions |
| Gate | Kairos (8-fold conjunctive) | `GateCascade` (5 gates, worst-wins) | **composed in sequence**, same doctrine |
| Metatron | `pse-metatron` (geometric/spectral scan) | `kosmo-hyphae::metatron` (code microtopology) | name echo, deliberate; no data path yet |
| HDAG | IL-HDAG (5D resonance tensors) | `CodeHDAG` (lexical structure graph) | distinct; convergence path = the Q4/Q5 lift |
| Store | `pse-store` (SQLite/Memory rows) | `kosmo-store` (JSONL append-only) | distinct durability contracts, no convergence needed |

## 1. Selection energy — already one formula

`kosmo-core::energy` cites its origin directly: *"Kosmocrates uses the
Pfauenthron tripolar energy `D = ψ · ρ · ω`"* — the same selection core the
PSE+IL retrieval uses. The vocabulary is converged; the arithmetic is
deliberately dual:

- **PSE side**: `f64` — the engine's resonance physics (spectral, Kuramoto,
  phases) lives in continuous math.
- **kosmo side**: `Q16` — no floats may appear in audit or gate-comparison
  paths (CROSS-007).

The two worlds touch at exactly one documented seam:
`pse_adapter_kosmo::phase_from_confidence` converts a candidate's `Q16`
confidence into the observation's `f64` semantic `phase_hint`. Both sides
share the doctrine **energy ranks, never gates** (CROSS-010 / the engine's
gate-then-extract order).

**Convergence verdict: complete.** Merging the implementations would destroy
the integer-auditability of the substrate or the physics of the engine.

## 2. Crystals — connected, in both directions

The substrate's `StructuralCrystalRecord` (gate-passed, constraint-certified,
replay-proofed, fingerprint-bearing, directly evidence-bound) is offered to
the engine as a `PseBridgeCandidate` of kind `CertifiedCrystal`
(`crystal_to_pse_candidate`), consumed by `pse-adapter-kosmo`, and — under the
operator's `--batch --calibration substrate` choice — commits real
`SemanticCrystal`s with QTIC Q3 certificates. The committed crystals persist
(`kosmo-promote --state`) and warm-start the engine's `PatternMemory`;
verdicts flow back as `PromotionFeedback` into the pipeline's norm fitness
(`--feedback`).

**Convergence verdict: live.** One crystal vocabulary per layer, one sanctioned
promotion path between them, memory flowing in both directions.

## 3. Gates — same doctrine, composed in sequence

Kairos is an 8-fold **conjunctive** gate over resonance metrics
(d/q/r/g/j/p/n/k — all must pass); the substrate's `GateCascade` runs five
gates with **worst-wins** merge and no short-circuit. Both are fail-closed
conjunctions; their forms differ because their objects differ (continuous
resonance vs. discrete policy/evidence checks).

Since the unification they compose **in sequence** along the promotion path:
a yield must survive the `GateCascade` (and the constraint program, and the
replay proof) to become a certified crystal, and that crystal's candidate must
then pass the fully-armed Kairos gate to become engine memory. Two gates, two
layers, one artifact — neither replaces the other.

**Convergence verdict: complete as composition.** Do not unify the
implementations; the composition *is* the design.

## 4. Metatron² — a name echo, honestly labelled

- `pse-metatron` is the vendored **Metatron Scan** library (scaffold, group
  theory, spectra, platonic solids) feeding the engine's cuboctahedron
  phase-ladder (Strand O).
- `kosmo-hyphae::metatron` is **microtopology diagnostics** over code regions
  (M1 lift → fingerprint, M2 diagnose → surgery options).

Both honor the same geometric tradition; they share no types and no data path.
A future bridge (kosmo region fingerprints feeding the spectral catalog) is
conceivable but **unscheduled** — nothing currently needs it.

**Convergence verdict: none planned.** The shared name is heritage, not debt.

## 5. HDAG² — distinct today, with a named meeting point

- The **IL-HDAG** (`pse-adapter-il::hdag`) is a hyperdimensional DAG of 5D
  resonance tensors `[temporal, morphic, relational, topological, entropic]`
  with phase-gradient edges and coherence-emergent acyclicity. It anchors
  committed crystals in the Infinity Ledger.
- The **`CodeHDAG`** (`kosmo-hyphae::code_hdag`) is a content-addressed
  lexical structure graph over source code (modules/imports/functions/types/
  tests), now polyglot via `xlang`.

They are different formalisms for different objects and should stay so. The
unification gives them a precise meeting point — **through the crystal, not a
type merge** — and that meeting point is **live**: `kosmo-promote --ledger
<path>` anchors every accepted crystal in the Infinity Ledger (block hash =
canonical trace anchor, IL-HDAG node = the resonance tensor the code-born
structure acquires, plus the path-invariance check), lifting promoted
substrate knowledge from Q3 to **Q5, full QTIC**. The anchor is idempotent
(an identical crystal re-anchors to the same block) and operator-authorized
(host write, `--offer` mode only). An unanchored promotion still reports
`trace_ready = false, path_inv = false` — the seam stays visible in every
Q3 certificate.

**Convergence verdict: live.** A `CodeHDAG`-born structure can now hold a
node in the IL-HDAG.

## 6. Stores — different durability contracts

`pse-store` persists engine runs/crystals/metrics as SQLite rows (or in
memory); `kosmo-store` is an append-only, integrity-checkable JSONL store for
the CAD library; `kosmo-promote` adds operator-authorized JSON archives for
engine memory and feedback. Each contract matches its consumer (queryable
rows vs. replayable logs vs. explicit operator files).

**Convergence verdict: none needed.** The unifying anchor, when it comes, is
the Infinity Ledger (see §5), not a shared database.

## What must never converge

These distinctions are the architecture; treat any "simplification" of them
as a regression:

1. **The dependency direction.** No `kosmo-*` crate imports `pse-*`, ever.
   The only crossing is `kosmo-pse-bridge` → `pse-adapter-kosmo` → engine.
2. **The arithmetic split.** `Q16` in every audit/gate path of the substrate;
   `f64` inside the engine's physics. One conversion seam, documented, in the
   adapter.
3. **The decision split.** The substrate certifies *structure*; PSE alone
   decides *memory*. No kosmo component constructs or commits a
   `SemanticCrystal`; no PSE component bypasses the substrate's policy gates.
4. **Fail-closed defaults on both sides.** `ReportOnly` substrate policies and
   the engine's conjunctive gate stay the defaults; every relaxation
   (`--offer`, `--calibration substrate`, …) is an explicit, logged operator
   choice.
