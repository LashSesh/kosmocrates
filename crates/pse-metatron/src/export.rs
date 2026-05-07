use std::fs;
use std::path::Path;

use crate::catalog::PeriodicTable;
use crate::error::ScanResult;
use crate::scan::ScanReport;

/// Serialize a single ScanReport to pretty-printed JSON.
pub fn report_to_json(report: &ScanReport) -> ScanResult<String> {
    Ok(serde_json::to_string_pretty(report)?)
}

/// Serialize a single ScanReport to compact JSON.
pub fn report_to_json_compact(report: &ScanReport) -> ScanResult<String> {
    Ok(serde_json::to_string(report)?)
}

/// Serialize an entire catalog to pretty-printed JSON.
pub fn catalog_to_json(table: &PeriodicTable) -> ScanResult<String> {
    Ok(serde_json::to_string_pretty(table)?)
}

/// Write the catalog to disk as `periodic_table.json`.
pub fn write_catalog_json(table: &PeriodicTable, path: &Path) -> ScanResult<()> {
    let json = catalog_to_json(table)?;
    fs::write(path, json).map_err(|e| crate::error::ScanError::Serde(serde_json::Error::io(e)))?;
    Ok(())
}

/// Render the catalog as a compact CSV summary table. Columns:
/// `id, n, m, orbit, stab, stab_class, triangles, chromatic, diameter,
///  connected, bipartite, is_tree, self_complementary, complement_id,
///  spectral_radius, alg_connectivity, degree_sequence, canonical_hash`
pub fn catalog_to_csv(table: &PeriodicTable) -> String {
    let mut out = String::with_capacity(64 * 1024);
    out.push_str(
        "id,n,m,orbit,stab,stab_class,triangles,chromatic,diameter,connected,bipartite,is_tree,is_regular,max_clique,self_complementary,complement_id,spectral_radius,algebraic_connectivity,degree_sequence,canonical_hash\n",
    );
    for e in &table.entries {
        let r = &e.report;
        let p = &r.properties;
        let diam = match p.diameter {
            Some(d) => d.to_string(),
            None => "inf".to_string(),
        };
        let stab_class = r.stabilizer_class.as_deref().unwrap_or("");
        let comp = e.complement_id.as_deref().unwrap_or("");
        let degs = p
            .degree_sequence
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(":");
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{},{}\n",
            e.id,
            e.n,
            e.m,
            r.orbit_size,
            r.stabilizer_order,
            stab_class,
            r.triangle_count,
            p.chromatic_number,
            diam,
            p.connected,
            p.is_bipartite,
            p.is_tree,
            p.is_regular,
            p.max_clique_size,
            p.is_self_complementary,
            comp,
            r.spectral_radius,
            r.algebraic_connectivity,
            degs,
            r.canonical_hash,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::build_catalog;
    use crate::ingest::InputGraph;
    use crate::scan::scan;

    #[test]
    fn json_roundtrip() {
        let g = InputGraph::from_edges(3, &[(1, 2), (2, 3), (1, 3)]).unwrap();
        let report = scan(g).unwrap();
        let json = report_to_json(&report).unwrap();
        assert!(json.contains("\"orbit_size\""));
        assert!(json.contains("\"stabilizer_order\""));
        assert!(json.contains("\"properties\""));

        let compact = report_to_json_compact(&report).unwrap();
        assert!(compact.len() < json.len());
    }

    #[test]
    fn csv_has_header_and_rows() {
        let table = build_catalog(3).unwrap();
        let csv = catalog_to_csv(&table);
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[0].starts_with("id,n,m,"));
        // 1 header + 7 entries (1+2+4)
        assert_eq!(lines.len(), 1 + 7);
    }

    #[test]
    fn catalog_json_contains_entries() {
        let table = build_catalog(3).unwrap();
        let json = catalog_to_json(&table).unwrap();
        assert!(json.contains("\"entries\""));
        assert!(json.contains("\"stats\""));
        assert!(json.contains("\"G3-3-1\""));
    }
}
