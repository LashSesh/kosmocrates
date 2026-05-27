# pse-traverse-horizon-cli

CLI for PSE-TRAVERSE-HORIZON-03 — null-centered horizon geometry (inspect / chart / rays / crossing / finalize / replay / verify)

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

`pse-traverse-horizon` — CLI for the v0.3 null-centered horizon
geometry layer.

Subcommands:

```text
inspect  <input.json>                                         → JSON dump of input + RD digest + carrier preview
chart    <input.json> --rd horizon-rd.json --out chart.json   → content-addressed HorizonChart
rays     <chart.json> --rd horizon-rd.json --out rays.json    → deterministic phase rays
crossing <input.json> --rd horizon-rd.json --out crossing.json
finalize <input.json> --rd horizon-rd.json --out final.json   → fails when G_v0.3 = 0
replay   <final-or-hold.json>                                 → byte-identity check
verify   <horizon-certificate.json>                           → certificate-chain audit
```

Manual flag parsing — keeps the binary dependency-free.

## Run

```bash
# From the workspace root:
cargo run --release -p pse-traverse-horizon-cli
# or, after `cargo install --path .`:
pse-traverse-horizon
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
