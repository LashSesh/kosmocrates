# Coffin-Dragger / Wings of Samael — repository-native specification

Derived from `specs/coffindragger master spec.pdf` (v1.0-closed). The master spec
*itself* mandates the method (ch. 27–28): **inventory → map to existing code →
repo-native spec → only then staged, gated tickets, report-only MVP (C0) first**.
This document is that repo-native spec; nothing here claims physical realization
of any pictorial term — all codenames are formal roles.

## Governing frame

```
DiamondCube        = Diamondize( Core ∘ Fix ∘ Ω(WingsOfSamael(∂QSR)) )
DiamondCubeCandidate = KBL( ASCC( CDK(HYPHAE, LPCM, MPK, TPC, MERKABA) ) )
CDK                = Cen ∘ Probe ∘ Seed ∘ Purge ∘ Pull
```

Energy basis: the Kosmocrates tripolar `D = ψ·ρ·ω`, objective `max D − λK − μC`.
A good condensate is never the largest — it is the one of maximal target-compatible
D-density under minimal contradiction energy, certified irreducible by QSR.

## The seven core invariants (§2.2) — the fixed substance

I1 structure over surface · I2 gate before score · I3 boundary over absorption ·
I4 trace and replay · I5 fixpoint over target text · I6 projection over omniscience ·
I7 condensation over growth.

## Layer model L0–L8 (no short-circuit, Req. 4.1)

| Layer | Role | Repository home |
|---|---|---|
| L0 Null-anchor | stateless order anchor | (operator/gate reference only) |
| L1 QSR | stability/closure predicate | **`kosmo-cdk-core::qsr`** ✅ (C0) |
| L2 ANL/SRG | reflection boundary, mirror-seam | `pse-*` reflection / boundary (map) |
| L3 Wings of Samael | radial projection-arm layer | **`kosmo-wings`** (new) ← adapters/SourceCube workers |
| L4 MPK | local finite symmetry/graph oracle | **`kosmo-wings::mpk_bridge` → `pse-metatron`** (local-only, catalog-bound) ✅ |
| L5 Panoptic projection | orbit completion, phase horizons | `pse-traverse` (cognition/, horizon/) |
| L6 MERKABA runtime | scheduler/sync/gatekeeper/memory | `kosmo-synthesizer::consensus` (Ophanim/Konus/Monolith) + `pse-scheduler` |
| L7 Con-Dragger / CDK | global centralization, diamondization | **`kosmo-coffindragger`** (new) + **`kosmo-cdk-core`** ✅ |
| L8 Materialization | SystemCube/Foundry/PSE bridge/output | `kosmo-systemcube`, `kosmo-materialize`, `kosmo-pse-bridge`, `kosmo-foundry` |

## Phase 2 — formal entity ↔ repository mapping

| Formal entity | Repository mapping | Status |
|---|---|---|
| `ContentId` | `kosmo_core::Digest` (JCS+SHA-256) | reuse ✅ |
| `Q16` | `kosmo_core::Q16` (fixed-point [0,1]) | reuse ✅ |
| `Status{Pending,Pass,Warn,Reject,Defer}` | new (kin to `kosmo_core::GateResult`) | **built** ✅ |
| `Stage`, `AttractorStack`, `Delta`, `StageEmbeddingCertificate` | new | **built** ✅ (`kosmo-cdk-core`) |
| `StageMetrics` (density/purity/irr/curvature/contradiction) | new; D-energy reuses `kosmo-hyphae` tripolar | **built** ✅ |
| QSR predicate (stage/stack, Inv. 7.3) | new | **built** ✅ |
| D-energy `D=ψ·ρ·ω`, objective `D−λK−μC` | new fn; basis from `kosmo-hyphae` | **built** ✅ |
| Wing (radial arm: probe∘orbit∘harvest∘condense∘anchor) | `kosmo-wings` ← existing SourceCube/projection workers, or new trait | next (C1) |
| Ophanim (closed wing cycle, roundtrip self-calibration) | `kosmo-synthesizer::consensus` (Ophanim/Konus/Monolith) wrapped | next (C1) |
| MPK (n≤8 oracle, orbit/stabilizer/spectrum, 13598 catalog) | `kosmo-wings::mpk_bridge` → `pse-metatron` (`classify_local`, `mpk_projection_gate`) | **wired** ✅ |
| ASCC closure / fold / contraction (LPCM) | `kosmo-core::closure` (StagedClosure) + `pse-traverse::plan` (CollapsePlan = Contr) + `kosmo-hyphae` LPCM | next (C2) |
| CDK run (Pull/Purge/Seed/Probe/Cen) | `kosmo-coffindragger::run` (orchestrates the above) | next (C2) |
| KBL (bind WishCube/SystemCube/reports → CDK stages) | `kosmo-coffindragger::binding` | next (C2) |
| SystemCube / contradiction energy / BlueprintUnit | `kosmo-systemcube` — **bound via `bind_systemcube`** (real KBL→Stage) | **wired** ✅ |
| DiamondCube (QSR-certified irreducible SystemCube core) | `kosmo-coffindragger::diamond` (wrapper + QSR cert) | next (C3) |
| TPC / panoptic projection / phase horizons | `pse-traverse` (cognition/horizon) | reuse ✅ |
| Materialization / PSE bridge | `kosmo-materialize`, `kosmo-pse-bridge`, `pse-adapter-kosmo` | reuse ✅ |

## Crate plan

```
crates/kosmo-cdk-core   ✅ C0  types · metrics (D-energy) · qsr   (report-only, no host mutation)
crates/kosmo-wings      ✅ C1  wing (Inv 11.2 + WingGate) · mesh · ophanim (roundtrip gate)  [mpk_bridge: next]
crates/kosmo-coffindragger ✅ C2  binding (KBL) · stack (ASCC: embed/accrete/contract/fold/close) · run (CDK purge)  [diamond: C3]
crates/kosmo-coffindragger ✅ C3  + diamond.rs (DiamondCubeCandidate, ASCC-6)
tools/kosmo-cdk         ✅ C3+ CLI bind·stack·close·diamond·explain · serve (REST §24.2) — runs the fold end-to-end
```

## Gate cascade (§25.1, fail-closed) and the non-negotiable rule

ScopeGate → EvidenceGate → BoundaryGate → CanonicalizationGate → WingGate →
MPKProjectionGate → OphanimRoundtripGate → StackClosureGate → QSRGate →
MaterializationSafetyGate → ResidueVisibilityGate. **Req. 25.1: scores never
override gates** (`Score(x) > Score(y) ⇏ Pass(x)`).

## Conformance / MVP ladder (Definition of Done, §26.3)

| Class | Requirement | Ticket |
|---|---|---|
| ASCC-0 | terminology cleaned; stage objects defined | **C0 ✅** |
| ASCC-1 | every successor stage carries an embedding certificate | C2 ✅ |
| ASCC-2 | support accretion + residue reports | C2 ✅ |
| ASCC-3 | contractive consolidation measurable + replayable | C2 ✅ |
| ASCC-4 | stack closure emits content-addressed FoldBundle | C2 ✅ |
| ASCC-5 | ACDC roundtrip checked via stack closure | C2 ✅ |
| ASCC-6 | diamondization emits QSR-certified DiamondCube | **C3 ✅** |
| CDK-W | wing/Ophanim layer + full gate cascade | **C1 ✅** |
| KBL-1 | Kosmocrates artifacts bound under Inv. 19.1 | C2 ✅ |

DoD **MET** ✅: all CDK crates build+test green; `bind/stack/close/diamond/explain` run
end-to-end report-only as **CLI and REST** (`serve`, §24.2); 15/16 negative tests (§26.1)
covered by failing-closed tests (the 16th — LLM-as-authority — architectural: no generator
in the gate path); ASCC-0..6, CDK-W, KBL-1 demonstrated; a sample `DiamondCubeCandidate`
with QSR certificate, residue report, and replayable trace; KBL binds a **real** SystemCube;
MPK bound to **real** `pse-metatron`.

## Status (this work)

**C0 done** (`kosmo-cdk-core`, 10 tests, clippy clean, report-only, reuses
`kosmo-core`): ground objects, the D-energy kernel, and the QSR predicate with the
monotonicity invariant (Inv. 7.3) and gate-before-score (I2) — negative tests 4
and 14 covered. Next: C1 (`kosmo-wings` — Wing/Ophanim over the existing consensus
+ MPK oracle), then C2 (`kosmo-coffindragger` — KBL binding + ASCC fold + CDK run),
then C3 (diamondization + the `kosmo-cdk` CLI). No host mutation at any stage until
the materialization-safety gate + operator policy are wired (L8).
