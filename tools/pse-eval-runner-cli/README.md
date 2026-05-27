# pse-eval-runner-cli

CLI for pse-eval-runner — runs the Reasoning-Agent evaluation against Cerebras or in stub mode

A command-line tool that ships with the [Kosmocrates](https://github.com/lashsesh/pse)
workspace.

## What it does

PSE Eval Runner CLI

Führt den Reasoning-Agent (Plan → Execute → Evaluate) durch die
PSE-Kognitionspipeline.  Ohne CEREBRAS_API_KEY läuft ein
deterministischer Stub.

Usage:
```text
  pse_eval_runner run   [--preset NAME] [--model MODEL]
  pse_eval_runner score --bundle bundle.json
  pse_eval_runner show  --bundle bundle.json
```

Presets: agent-cognition (default), post-symbolic-ablation, lpcm-fragment-collapse

## Run

```bash
# From the workspace root:
cargo run --release -p pse-eval-runner-cli
# or, after `cargo install --path .`:
pse_eval_runner
```

## Documentation

For the layered architecture this tool operates on, see the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md).

## License

MIT — see [`LICENSE`](../../LICENSE).
