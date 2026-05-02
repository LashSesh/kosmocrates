//! Metatron Scan as a PSE workspace library.
//!
//! Vendored from the standalone `metatron-scan` crate (Sebastian Klemm,
//! MIT-licensed, same author as PSE) and adapted for in-workspace use.
//! The Metatron Scan adapter modules (cargo_deps, rust_imports, edge_list,
//! json_graph, state_machine, decompose, ingest_report) are intentionally
//! NOT vendored — PSE has its own adapter framework. Everything else
//! (scaffold, group, spectrum, platonic, catalog, analysis, scan,
//! properties, export) is preserved verbatim so the empirical findings
//! in `research/findings.md` continue to hold.
//!
//! Strand O integrates this library into PSE in five steps:
//!
//!  - **O.1 (this PR)**: stand the crate up as a workspace member,
//!    verify its tests pass, expose its public API to other PSE crates.
//!  - **O.2**: cuboctahedron phase-ladder builder in `pse-cascade`.
//!  - **O.3**: `MetatronTopologySignature` field on every `SemanticCrystal`.
//!  - **O.4**: periodic-table lookup in `pse-memory`.
//!  - **O.5**: empirical bench extensions for the H1/H2/H3 hypotheses.

pub mod error;
pub mod geometry;
pub mod scaffold;
pub mod group;
pub mod ingest;
pub mod embed;
pub mod orbit;
pub mod stabilizer;
pub mod complement;
pub mod spectrum;
pub mod platonic;
pub mod bitgraph;
pub mod bitgraph_n;
pub mod properties;
pub mod scan;
pub mod catalog;
pub mod analysis;
pub mod export;
pub mod phase_layout;

pub use error::{ScanError, ScanResult};
pub use ingest::InputGraph;
pub use scan::{scan, scan_with_cache, ScanReport, S7Cache};
pub use export::{
    catalog_to_csv, catalog_to_json, report_to_json, report_to_json_compact, write_catalog_json,
};
pub use catalog::{
    build_catalog, build_catalog_for_n, build_catalog_orderly, find_in_catalog, oeis_a000055,
    oeis_a000088, oeis_a000171, oeis_a001349, oeis_a003400, oeis_a005470, oeis_a005840,
    oeis_a033995, CatalogEntry, CatalogStats, PeriodicTable,
};
pub use bitgraph::{S7BitCache, fast_orbit};
pub use properties::{compute_properties, GraphProperties};
pub use analysis::{
    cospectral_analysis, cospectral_distinguishers, full_analysis, matrix_power_analysis,
    platonic_correlation, scaffold_invariants, self_complementary_analysis, spectral_distribution,
    CospectralAnalysis, CospectralDistinguisher, FullAnalysis, MatrixPowerAnalysis,
    PlatonicCorrelationAnalysis, ScaffoldInvariantAnalysis, SelfComplementaryAnalysis,
    SpectralDistribution,
};
pub use scaffold::MetatronScaffold;
pub use platonic::{classify_platonic, PlatonicClass};
pub use spectrum::compute_spectrum;
pub use phase_layout::metatron_phase_layout;
