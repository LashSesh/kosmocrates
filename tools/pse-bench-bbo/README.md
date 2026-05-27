# pse-bench-bbo

Non-stationary black-box optimization benchmark for the TRITON navigator

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

Black-box optimization benchmark for the TRITON navigator (Strand G).

This crate compares TRITON — PSE's golden-angle spiral with
Fiedler-vector momentum and Betti-guard topology safety
(`pse-navigator`) — against two reference baselines on a small
battery of black-box test functions, including non-stationary
variants where the global optimum drifts during the run.

All optimizers receive the *same* fixed budget (number of
evaluations) and *same* deterministic seed. Metrics:

 - **simple regret**:    f(x*) − f_min, where x* is the best point
                         found by the optimizer at the end of the
                         budget.
 - **cumulative regret**: Σ (f(x_t) − f_min) over all t in 1..=T,
                          where x_t is the t-th evaluated point
                          and f_min is the *current* (possibly
                          drifting) global optimum at time t.

Lower is better for both metrics. For non-stationary functions,
"best point found" is evaluated at the *final* time step.

## Run

```bash
# From the workspace root:
cargo run --release -p pse-bench-bbo
# or, after `cargo install --path .`:
bench_bbo
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
