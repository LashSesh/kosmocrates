# pse-traverse-lpcm-cli

CLI for the PSE LPCM Fragmented 51% Condensation Layer — inspect / partition / collapse / replay / verify / ablate

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

`pse-traverse-lpcm` — CLI for the Fragmented 51% Condensation Layer.

Subcommands (§17 Reference CLI):

```text
inspect   <window.json>                               → summary stats
partition <window.json> --out <fragments.json>        → fragmented window
collapse  <window.json> [--rd <rd.json>] --out <report.json>
replay    --rd <rd.json> --input <window.json> --report <report.json>
verify    <report.json>                               → replay identity check
ablate    <window.json> [--without-lpcm] [--with-lpcm] --out <ablation.json>
```

## Run

```bash
# From the workspace root:
cargo run --release -p pse-traverse-lpcm-cli
# or, after `cargo install --path .`:
pse-traverse-lpcm
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
