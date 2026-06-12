# Reforge — the external-empiricism bench

Every other harness in this repository judges the wish-to-system loop
against a corpus this repository wrote. The reforge judges it against
**binaries this repository did not write**: real POSIX/coreutils tools on
your machine become the oracles.

## The protocol (one command)

```sh
ANTHROPIC_API_KEY=sk-... cargo run -p kosmo-run -- --reforge --reforge-report reforge.json
```

(or `CEREBRAS_API_KEY=...` with `--provider cerebras`.)

For each target — `expr` (addition), `factor`, `basename` — the harness:

1. **Collects ground truth externally.** It runs the *real* tool on a
   fixed probe set and records its answers. Nothing is hard-coded: the
   expectations come from the binary on *your* machine, at run time. If a
   tool is missing, its target is skipped honestly — truth is never
   invented.
2. **Builds the wish.** One budgeted runtime expectation per probe:
   `args => exit:0, out~<oracle answer>, ms<60000`. The wish is
   content-addressed and evidence-bound to the collected truth.
3. **Re-forges.** Starting from an empty crate, the loop descends: the
   deterministic scaffolder erects the probe harness, the provider-backed
   synthesizer implements the behaviour, and every iteration the forged
   binary is **executed under the sandbox witness** against the oracle's
   answers — exit code, output, and time budget, fail-closed.
4. **Reports.** A content-addressed JSON report (`report_id` =
   SHA-256 of the report body) records, per target: the oracle, the
   collected probe truths, the wish id, the verdict, and the observation
   count. `✓ re-forged` is never claimed — it is the measured state of a
   binary that ran.

Exit code 0 means: no attempted target failed (skips are honest).
Exit code 5 means: at least one attempted forge did not reach behavioural
equivalence — the report says which, with the evidence.

## What this does and does not prove

- **Proves**: the loop can take expectations whose source is the outside
  world and drive an empty workspace to a binary that *demonstrably*
  meets them — with the whole evidence chain (truth → wish id → witness)
  inspectable in the report.
- **Does not prove**: provider-independence (the implementing step is the
  one sanctioned non-deterministic worker; your provider and model
  matter) or generality beyond the target class (argv-pure,
  single-line-output tools — the class grows like every vocabulary here).

## What CI pins (offline, no key)

The harness itself: oracle probing against real binaries (`echo`),
refusal of missing/uncarryable oracles (multi-line truths are rejected,
never approximated), wish construction (budgeted, evidence-bound,
deterministic), report shape, and the provider requirement (keyless and
`--provider mock` invocations are refused — forging theater).
