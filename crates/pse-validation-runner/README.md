# pse-validation-runner

PSE-VALIDATION-RUNNER-01 — One-Button Validation Singularity library

`pse-validation-runner` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

PSE-VALIDATION-RUNNER-01 — One-Button Validation Singularity.

Orchestrates Build, Tests, Benchmarks, Eval-Matrix-Presets, Replay,
Scoring, Reports and Artifact-Bundling into a single deterministic
`ValidationSingularityBundle`.

Invariant: no wall-clock time appears in any content-addressed hash.
The runner is NOT a new cognition layer; it MUST NOT duplicate
Eval-Matrix logic.

## Add to your project

```toml
[dependencies]
pse-validation-runner = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-validation-runner --open`
(once published, also available on [docs.rs](https://docs.rs/pse-validation-runner)).

## License

MIT — see [`LICENSE`](../../LICENSE).
