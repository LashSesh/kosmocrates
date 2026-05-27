# nxalien-cli

CLI tool for the nxalien agent-context exoskeleton

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

nxalien — PSE agent-context exoskeleton CLI.

Subcommands:
  init      Create .nxalien/ with default policy
  inspect   Print detected project substrate
  compile   Full pipeline → nxalien.manifest.json + context artifacts
            --remote <url>    POST bundle to a PSE server (cross-repo attractor)
            --remote-only     skip local IL/PSE state after remote POST
  ground    Print [NXALIEN-CONTEXT] block to stdout
  handoff   Print handoff candidates JSON
  replay    Recompute hashes and compare to manifest
  verify    Run gates + replay + canon verification
  export    Generate CLAUDE.md / AGENTS.md / .rules

## Run

```bash
# From the workspace root:
cargo run --release -p nxalien-cli
# or, after `cargo install --path .`:
nxalien
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
