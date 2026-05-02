//! E.5: Crystals as observations.
//!
//! `CrystalAdapter` lets the engine treat its own [`SemanticCrystal`]
//! artifacts as a *new layer of observation*. A meso-resonance then
//! literally is what the engine produces when several micro-crystals
//! synchronise on a higher carrier — the pipeline becomes
//! **compositional**: crystallisation is a fixed point that can be
//! iterated.
//!
//! Each crystal becomes its own vertex in the higher-layer graph
//! because the adapter derives the per-observation `source_id` from
//! the crystal's content-addressed `crystal_id`. Two distinct crystals
//! therefore land on distinct vertices; ingesting the *same* crystal
//! twice lands on the same vertex (idempotency under content address).
//!
//! The adapter is layer-tagged ("meso", "macro", or any user label)
//! so multiple compositional levels can coexist in a single archive.

use pse_graph::{ObservationAdapter, ObserveError};
use pse_types::{
    canonical_bytes, content_address_raw, Hash256, MeasurementContext, Observation,
    ProvenanceEnvelope, SemanticCrystal,
};

/// Encode 16 bytes of a Hash256 as 32 lowercase hex characters.
fn hex16(digest: &Hash256) -> String {
    let mut s = String::with_capacity(32);
    for b in &digest[..16] {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Adapter that turns [`SemanticCrystal`] artifacts into PSE observations.
///
/// Use [`CrystalAdapter::payload_for`] to produce the canonical JCS bytes
/// for a crystal, then feed those bytes to `macro_step` together with this
/// adapter. The resulting observation is content-addressed by the crystal's
/// own bytes, but its **vertex identity** comes from the crystal's
/// content-addressed `crystal_id` — so two distinct crystals always land
/// on distinct graph vertices regardless of when or in what order they
/// were ingested.
pub struct CrystalAdapter {
    layer: String,
}

impl CrystalAdapter {
    /// Create an adapter for the given compositional layer label
    /// (e.g. `"meso"`, `"macro"`, or any user-supplied tag).
    pub fn new(layer: impl Into<String>) -> Self {
        Self { layer: layer.into() }
    }

    /// Layer label this adapter was constructed with.
    pub fn layer(&self) -> &str {
        &self.layer
    }

    /// Encode a crystal as JCS bytes ready for `macro_step` ingest.
    ///
    /// Use:
    /// ```rust,ignore
    /// let adapter = CrystalAdapter::new("meso");
    /// let payload = CrystalAdapter::payload_for(&crystal);
    /// macro_step(&mut state, &[payload], &config, &adapter)?;
    /// ```
    pub fn payload_for(crystal: &SemanticCrystal) -> Vec<u8> {
        canonical_bytes(crystal)
    }
}

impl ObservationAdapter for CrystalAdapter {
    fn source_id(&self) -> &str {
        &self.layer
    }

    fn canonicalize(
        &self,
        raw: &[u8],
        context: &MeasurementContext,
    ) -> Result<Observation, ObserveError> {
        // Parse the payload as a SemanticCrystal so we can derive a
        // per-crystal source identifier. This is what makes each crystal
        // its own vertex in the higher-layer graph.
        let crystal: SemanticCrystal = serde_json::from_slice(raw).map_err(|e| {
            ObserveError::Canonicalize(format!("payload is not a SemanticCrystal: {}", e))
        })?;
        let crystal_hex = hex16(&crystal.crystal_id);
        let source_id = format!("{}:{}", self.layer, crystal_hex);

        let payload = raw.to_vec();
        let digest: Hash256 = content_address_raw(&payload);
        Ok(Observation {
            timestamp: 0.0,
            source_id,
            provenance: ProvenanceEnvelope {
                origin: self.layer.clone(),
                chain: vec![crystal_hex],
                sig: None,
            },
            payload,
            context: context.clone(),
            digest,
            schema_version: "crystal/1.0".into(),
            phase_hint: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{macro_step, GlobalState};
    use pse_graph::derive_vertex_id;
    use pse_types::{
        CommitProof, Config, ConstraintProgram, SemanticCrystal, TopologySignature, VertexId,
    };
    use std::collections::BTreeMap;

    fn make_crystal(region: Vec<VertexId>, stability: f64) -> SemanticCrystal {
        let mut c = SemanticCrystal {
            crystal_id: [0u8; 32],
            region: region.clone(),
            constraint_program: ConstraintProgram::new(),
            stability_score: stability,
            topology_signature: TopologySignature::default(),
            betti_numbers: vec![1, 0, 0],
            evidence_chain: Vec::new(),
            commit_proof: CommitProof::default(),
            operator_versions: BTreeMap::new(),
            created_at: 1,
            free_energy: -stability,
            carrier_instance_idx: 0,
            scale_tag: "micro".into(),
            universe_id: String::new(),
            sub_crystal_ids: Vec::new(),
            parent_crystal_ids: Vec::new(),
            genesis_metadata: None,
            metatron_signature: None,
        };
        // crystal_id = SHA-256(JCS(CrystalCore subset)) — for the test we
        // just want *deterministic* but distinct ids per region.
        // pse_evidence::build_crystal_with_id would compute this for us;
        // here we mimic it with content_address over the core fields.
        #[derive(serde::Serialize)]
        struct Core<'a> {
            region: &'a Vec<VertexId>,
            stability_score: f64,
            created_at: u64,
            free_energy: f64,
            carrier_instance_idx: usize,
        }
        let core = Core {
            region: &c.region,
            stability_score: c.stability_score,
            created_at: c.created_at,
            free_energy: c.free_energy,
            carrier_instance_idx: c.carrier_instance_idx,
        };
        c.crystal_id = pse_types::content_address(&core);
        c
    }

    #[test]
    fn distinct_crystals_get_distinct_source_ids() {
        let adapter = CrystalAdapter::new("meso");
        let c1 = make_crystal(vec![1, 2, 3], 0.7);
        let c2 = make_crystal(vec![4, 5, 6], 0.7);
        assert_ne!(c1.crystal_id, c2.crystal_id);
        let p1 = CrystalAdapter::payload_for(&c1);
        let p2 = CrystalAdapter::payload_for(&c2);
        let ctx = MeasurementContext::default();
        let o1 = adapter.canonicalize(&p1, &ctx).unwrap();
        let o2 = adapter.canonicalize(&p2, &ctx).unwrap();
        assert_ne!(o1.source_id, o2.source_id);
        // Both source-ids start with the layer label.
        assert!(o1.source_id.starts_with("meso:"));
        assert!(o2.source_id.starts_with("meso:"));
    }

    #[test]
    fn same_crystal_yields_same_source_id() {
        let adapter = CrystalAdapter::new("meso");
        let c = make_crystal(vec![10, 20, 30], 0.85);
        let p = CrystalAdapter::payload_for(&c);
        let ctx = MeasurementContext::default();
        let o1 = adapter.canonicalize(&p, &ctx).unwrap();
        let o2 = adapter.canonicalize(&p, &ctx).unwrap();
        assert_eq!(o1.source_id, o2.source_id);
        assert_eq!(o1.digest, o2.digest);
    }

    #[test]
    fn different_layers_yield_different_source_ids_for_same_crystal() {
        let meso = CrystalAdapter::new("meso");
        let macro_ = CrystalAdapter::new("macro");
        let c = make_crystal(vec![1, 2, 3], 0.7);
        let p = CrystalAdapter::payload_for(&c);
        let ctx = MeasurementContext::default();
        let o_meso = meso.canonicalize(&p, &ctx).unwrap();
        let o_macro = macro_.canonicalize(&p, &ctx).unwrap();
        assert_ne!(o_meso.source_id, o_macro.source_id);
        // ...but the suffix (the crystal-derived part) is the same.
        let suffix_meso = o_meso.source_id.trim_start_matches("meso:");
        let suffix_macro = o_macro.source_id.trim_start_matches("macro:");
        assert_eq!(suffix_meso, suffix_macro);
    }

    #[test]
    fn rejects_non_crystal_payload() {
        let adapter = CrystalAdapter::new("meso");
        let ctx = MeasurementContext::default();
        let result = adapter.canonicalize(b"not a crystal at all", &ctx);
        assert!(matches!(result, Err(ObserveError::Canonicalize(_))));
    }

    #[test]
    fn distinct_crystals_create_distinct_graph_vertices() {
        // The compositional payoff: each unique crystal lands on its
        // own vertex in the higher-layer graph, so feeding many
        // crystals can build real structure.
        let config = Config::default();
        let mut state = GlobalState::new(&config);
        let adapter = CrystalAdapter::new("meso");

        let crystals: Vec<SemanticCrystal> = (0..5)
            .map(|i| make_crystal(vec![i, i + 100, i + 200], 0.7))
            .collect();
        for c in &crystals {
            let payload = CrystalAdapter::payload_for(c);
            // We do not assert anything about whether a meso crystal
            // commits — that depends on the gate dynamics which is the
            // job of E.6 + E.7. We assert only that the graph picked
            // up each crystal as its own vertex.
            let _ = macro_step(&mut state, &[payload], &config, &adapter);
        }
        // The graph must contain at least one vertex per distinct
        // crystal source_id (it may contain more if cross-batch edges
        // produced placeholder vertices).
        let mut expected_vids = std::collections::BTreeSet::new();
        for c in &crystals {
            let source = format!("meso:{}", hex16(&c.crystal_id));
            expected_vids.insert(derive_vertex_id(&source));
        }
        for vid in expected_vids {
            assert!(
                state.graph.id_map.contains_key(&vid),
                "expected vertex {} (one per crystal source_id) in the graph",
                vid
            );
        }
    }

    #[test]
    fn round_trip_through_canonical_bytes() {
        // payload_for(c) → canonicalize → digest must equal SHA-256 of
        // those same canonical bytes (Inv I3-style content-address
        // closure).
        let adapter = CrystalAdapter::new("meso");
        let c = make_crystal(vec![7, 8, 9], 0.6);
        let payload = CrystalAdapter::payload_for(&c);
        let ctx = MeasurementContext::default();
        let obs = adapter.canonicalize(&payload, &ctx).unwrap();
        assert_eq!(obs.digest, content_address_raw(&payload));
        assert_eq!(obs.payload, payload);
    }
}
