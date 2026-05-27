# pse-eval-runner

Real TrialExecutor that drives run_cognition() for agent-relevant workload families

`pse-eval-runner` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

PSE-EVAL-RUNNER — Real `TrialExecutor` for agent-relevant workloads.

`PseTrialExecutor` drives the live `run_cognition()` pipeline against
a deterministic scenario suite calibrated to the workload family and
the variant's `LayerMask`.  Every execution path is purely a function
of (variant, workload, descriptor) — no wall-clock, no OS randomness.

## Integration

```rust,no_run
use pse_eval_matrix::{plan_runs, run_trial, init_ledger, append_to_ledger,
    score_ledger, Preset};
use pse_eval_runner::PseTrialExecutor;

let spec = Preset::AgentCognition.build().unwrap();
let plan = plan_runs(&spec).unwrap();
let executor = PseTrialExecutor;
let mut ledger = init_ledger(spec.spec_id.clone()).unwrap();
let mut reports = Vec::new();
for entry in &plan.entries {
    let variant = spec.variants.iter()
        .find(|v| v.variant_id == entry.descriptor.variant_id).unwrap();
    let workload = spec.workloads.iter()
        .find(|w| w.workload_id == entry.descriptor.workload_id).unwrap();
    let (report, run_entry) = run_trial(
        &spec, variant, workload, &entry.descriptor, &executor, &spec.metrics,
    ).unwrap();
    ledger = append_to_ledger(ledger, run_entry).unwrap();
    reports.push(report);
}
```

## Add to your project

```toml
[dependencies]
pse-eval-runner = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-eval-runner --open`
(once published, also available on [docs.rs](https://docs.rs/pse-eval-runner)).

## License

MIT — see [`LICENSE`](../../LICENSE).
