# pse-metatron

Metatron-scaffold topology scanner and Periodic-Table-of-Graphs library for PSE (vendored from metatron-scan, MIT)

`pse-metatron` is part of the [Kosmocrates](https://github.com/lashsesh/pse) workspace —
the post-symbolic multi-layer epistemic operating system. See the project
[`README.md`](../../README.md) and [`docs/OVERVIEW.md`](../../docs/OVERVIEW.md)
for the layered architecture this crate slots into.

## What it does

Metatron Scan as a PSE workspace library.

Vendored from the standalone `metatron-scan` crate (Sebastian Klemm,
MIT-licensed, same author as PSE) and adapted for in-workspace use.
The Metatron Scan adapter modules (cargo_deps, rust_imports, edge_list,
json_graph, state_machine, decompose, ingest_report) are intentionally
NOT vendored — PSE has its own adapter framework. Everything else
(scaffold, group, spectrum, platonic, catalog, analysis, scan,
properties, export) is preserved verbatim so the empirical findings
in `research/findings.md` continue to hold.

Strand O integrates this library into PSE in five steps:

 - **O.1 (this PR)**: stand the crate up as a workspace member,
   verify its tests pass, expose its public API to other PSE crates.
 - **O.2**: cuboctahedron phase-ladder builder in `pse-cascade`.
 - **O.3**: `MetatronTopologySignature` field on every `SemanticCrystal`.
 - **O.4**: periodic-table lookup in `pse-memory`.
 - **O.5**: empirical bench extensions for the H1/H2/H3 hypotheses.

## Add to your project

```toml
[dependencies]
pse-metatron = "0.1.0"
```

## Documentation

API reference: `cargo doc -p pse-metatron --open`
(once published, also available on [docs.rs](https://docs.rs/pse-metatron)).

## License

MIT — see [`LICENSE`](../../LICENSE).
