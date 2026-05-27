# pse-constitutional-interceptor

Constitutional enforcement layer — evaluates agent actions against nxalien rules before they touch application state

`pse-constitutional-interceptor` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Constitutional Interceptor — evaluates agent actions against nxalien rules.

This is the enforcement layer of Pfad B: governance rules are no longer
advisory Markdown — they block actions that violate the constitutional
framework *before* those actions can touch application state.

## Decision flow

```text
ActionContext ─► trigger match? ─► severity ─► Decision

  Blocking rule matched  →  Block  (hard stop, 403 on HTTP)
  Required rule matched  →  Block  in strict_mode, Warn otherwise
  Advisory rule matched  →  Allow  (surfaced in report, never blocks)
  No rule matched        →  Allow
```

## Strict mode

Strict mode is activated automatically when the EpistemicSignal of the
governing PSE instance is `Drifting` or `Diverging`.  In strict mode,
Required rules escalate to Block — the governance framework tightens as
the system's epistemic health degrades.

## Usage

```rust,ignore
use pse_constitutional_interceptor::{ActionContext, ConstitutionalEvaluator};

let evaluator = ConstitutionalEvaluator::new(rules);
let action = ActionContext::new("run_command", "rm -rf /data", "clean up disk");
let report = evaluator.evaluate(&action);
if report.decision.is_block() {
    eprintln!("Blocked: {:?}", report.decision);
}
```

## Add to your project

```toml
[dependencies]
pse-constitutional-interceptor = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-constitutional-interceptor --open`
(once published, also available on [docs.rs](https://docs.rs/pse-constitutional-interceptor)).

## License

MIT — see [`LICENSE`](../../LICENSE).
