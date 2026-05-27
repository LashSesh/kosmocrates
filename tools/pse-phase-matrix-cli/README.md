# pse-phase-matrix-cli

CLI for PHASEMATRIX-HIVEMIND-03 — cell-pool / cell-resonate / cluster-form / funnel-build / morph-step / converge / intent / cluster-dissolve / cluster-cycle / cluster-replay / cluster-verify (binary: phase-matrix)

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

`phase-matrix` — CLI for PHASEMATRIX-HIVEMIND-03 / 03.1 (§22, §12).

Subcommands (v0.3 cell substrate):

```text
cell-pool        <input.json>  --rd <rd.json>  --out <pool.json>
cluster-cycle    <input.json>  --rd <rd.json>  --out <cycle.json>
cluster-replay   <cycle.json>
cluster-verify   <trace-or-dissolution.json>
```

Subcommands added by v0.3.1 Dual-Fabric Stitch Layer (§12):

```text
stitch-fabric    <cell-cycle.json>    --rd <stitch-rd.json>  --out <fabric-h.json>
stitch-candidates <fabric-h.json>    --tensor <tensor.json>  --rd <stitch-rd.json>  --out <candidates.json>
stitch-gate      <candidates.json>   --tensor <tensor.json>  --rd <stitch-rd.json>  --out <gates.json>
stitch-apply     <candidates.json>   --gates <gates.json>    --tensor <tensor.json>  --rd <stitch-rd.json>  --out <stitch.json>
stitch-cycle     <cell-cycle.json>   --tensor <tensor.json>  --rd <stitch-rd.json>  --out <stitch.json>
stitch-replay    <stitch-bundle.json>
tensor-inspect   <tensor.json>
```

## Run

```bash
# From the workspace root:
cargo run --release -p pse-phase-matrix-cli
# or, after `cargo install --path .`:
phase-matrix
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
