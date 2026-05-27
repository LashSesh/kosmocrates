# pse-eval-matrix

PSE-EVAL-MATRIX-01 — empirical benchmark matrix for post-symbolic cognition systems (variants × workloads × domains × metrics × calibration; replay-verified, fail-closed)

`pse-eval-matrix` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

PSE-EVAL-MATRIX-01 — Empirical Benchmark Matrix for Post-Symbolic
Cognition Systems.

See `specs/PSE_EVAL_MATRIX_01.pdf` for the normative specification
this crate realises.

The eval matrix is **not** a benchmark harness. It is the formal
description of:

* which scientific question is being asked
  ([`spec::EvaluationPurpose`]);
* which system variants are compared
  ([`variants::SystemVariantSpec`] over a B0 → B7 ladder);
* which workloads and domains are admissible
  ([`workloads::WorkloadSpec`] / [`datasets::DatasetManifest`]);
* which metrics count as primary, secondary or diagnostic
  ([`metrics::MetricSpec`]);
* which ablations are mandatory
  ([`ablation::AblationLadder`]);
* which gate / replay violations invalidate a result
  ([`failure_taxonomy::FailureKind`]);
* how runs are aggregated statistically
  ([`spec::StatisticalPlan`]);
* what counts as evidence sufficient for publication or product
  decisions ([`reports::EvaluationSummaryReport`] +
  [`scoring::CapabilityProfile`]).

A run is the deterministic application of `(variant × workload ×
dataset × metric × run_descriptor)` to a `TrialExecutor`. The
resulting [`reports::TrialReport`] is content-addressed; an
[`ledger::EvaluationRunLedger`] is append-only and hash-chained;
replay is byte-identical or the run is invalid; scoring runs
exclusively from the ledger and the declared metric specs.

## Hard rules

* No platform floats in score / metric / gate hashes — every
  gate-relevant scalar is a [`primitives::Fixed`]
  (`CanonicalNumber`).
* Every keyed collection is a `BTreeMap`; every unordered list is
  sorted before hashing.
* Wall-clock timestamps are forbidden in the audit pathway.
* A `ReplayMismatch` invalidates the run; an invalid run cannot
  contribute to any conclusion claim.
* `score` MUST NOT recompute metrics — only aggregate from
  recorded `MetricObservation`s.

## Add to your project

```toml
[dependencies]
pse-eval-matrix = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-eval-matrix --open`
(once published, also available on [docs.rs](https://docs.rs/pse-eval-matrix)).

## License

MIT — see [`LICENSE`](../../LICENSE).
