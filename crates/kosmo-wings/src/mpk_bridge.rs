//! MPK bridge — the local symmetry/graph oracle (L4, spec ch. 13), bound to the
//! real `pse-metatron` Metatron-scan kernel.
//!
//! Two disciplines are enforced **fail-closed**, both from Requirement 13.2:
//! - MPK output is a **local classification, never a global proof** (negative
//!   test 9): `MpkScope::Global` is never produced by the oracle, and
//!   [`mpk_projection_gate`] rejects any result that claims it.
//! - the graph catalog (13598 graphs for n ≤ 8) MUST be bound with **version,
//!   digest, and provenance** before use (negative test 16): [`classify_local`]
//!   refuses an unbound [`MpkCatalogBinding`].
//!
//! MPK is finite and bounded — a topological microscope, not a global proof
//! engine — so classification is restricted to small graphs (n ≤ 8).

use kosmo_cdk_core::{GateError, Status};
use kosmo_core::Digest;
use pse_metatron::{properties::compute_properties_with, InputGraph};
use serde::{Deserialize, Serialize};

/// The bound graph catalog (Req. 13.2): version, content digest, and provenance.
/// Without all three the catalog MUST NOT be used (negative test 16).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MpkCatalogBinding {
    pub version: String,
    pub digest: Digest,
    pub provenance: String,
}

impl MpkCatalogBinding {
    pub fn is_bound(&self) -> bool {
        !self.version.is_empty() && self.digest != Digest::ZERO && !self.provenance.is_empty()
    }
}

/// The scope of an MPK result. The oracle only ever produces [`MpkScope::Local`]
/// (Req. 13.2). `Global` exists solely so the gate can reject a result that
/// *claims* global-proof authority (negative test 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MpkScope {
    Local,
    Global,
}

/// MPK output (Def. 13.1): a content-addressed canonical id and the local
/// classification of a small typed graph — a *local* result under a bound catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MpkClassification {
    pub canon: Digest,
    pub node_count: usize,
    pub edge_count: usize,
    pub connected: bool,
    pub chromatic_number: usize,
    pub scope: MpkScope,
    pub catalog_version: String,
}

/// Classify a small typed graph through the local MPK oracle (`pse-metatron`).
/// Fail-closed: requires a **bound** catalog (Req. 13.2 / negative test 16) and a
/// **finite** graph (n ≤ 8). The classification is always [`MpkScope::Local`].
pub fn classify_local(
    n: usize,
    edges: &[(usize, usize)],
    catalog: &MpkCatalogBinding,
) -> Result<MpkClassification, GateError> {
    if !catalog.is_bound() {
        return Err(GateError::Rejected(
            "MPK catalog not version/digest/provenance bound".into(),
        ));
    }
    if n == 0 || n > 8 {
        return Err(GateError::Rejected(
            "MPK is a finite local oracle (1 ≤ n ≤ 8), never a global proof engine".into(),
        ));
    }
    let g = InputGraph::from_edges(n, edges)
        .map_err(|e| GateError::Rejected(format!("graph rejected: {e}")))?;
    // A genuine pse-metatron classification of the small graph.
    let props = compute_properties_with(&g.adjacency, g.n, false);
    // Canonical, content-addressed id of the input (deterministic over sorted edges).
    let mut sorted: Vec<(usize, usize)> = edges
        .iter()
        .map(|&(a, b)| if a <= b { (a, b) } else { (b, a) })
        .collect();
    sorted.sort_unstable();
    sorted.dedup();
    Ok(MpkClassification {
        canon: Digest::of(&(n, &sorted)),
        node_count: g.n,
        edge_count: g.edge_count(),
        connected: props.connected,
        chromatic_number: props.chromatic_number,
        scope: MpkScope::Local,
        catalog_version: catalog.version.clone(),
    })
}

/// MPKProjectionGate (§25, fail-closed): an MPK result is admissible only if it is
/// a **local** classification (negative test 9 — never a global proof) under a
/// **version-bound** catalog (negative test 16).
pub fn mpk_projection_gate(c: &MpkClassification, catalog: &MpkCatalogBinding) -> Status {
    if !catalog.is_bound() {
        return Status::Reject; // negative test 16
    }
    if c.scope != MpkScope::Local {
        return Status::Reject; // negative test 9 — MPK output claimed as global proof
    }
    Status::Pass
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> MpkCatalogBinding {
        MpkCatalogBinding {
            version: "metatron-n8-v1".into(),
            digest: Digest::of_bytes(b"catalog-bytes"),
            provenance: "pse-metatron::catalog::build_catalog_for_n".into(),
        }
    }

    #[test]
    fn it_classifies_a_small_graph_via_pse_metatron() {
        // The triangle K3: connected, 3 edges, chromatic number 3 — a real
        // pse-metatron classification.
        let c = classify_local(3, &[(1, 2), (2, 3), (1, 3)], &catalog()).expect("classified");
        assert_eq!(c.node_count, 3);
        assert_eq!(c.edge_count, 3);
        assert!(c.connected);
        assert_eq!(c.chromatic_number, 3, "K3 needs 3 colors");
        assert_eq!(c.scope, MpkScope::Local);
        assert_eq!(mpk_projection_gate(&c, &catalog()), Status::Pass);
    }

    #[test]
    fn an_unbound_catalog_is_refused() {
        // Negative test 16 — the catalog must be version/digest/provenance bound.
        let unbound = MpkCatalogBinding {
            version: String::new(),
            digest: Digest::ZERO,
            provenance: String::new(),
        };
        assert!(classify_local(3, &[(1, 2), (2, 3)], &unbound).is_err());
        let c = classify_local(3, &[(1, 2), (2, 3)], &catalog()).unwrap();
        assert_eq!(mpk_projection_gate(&c, &unbound), Status::Reject);
    }

    #[test]
    fn a_result_claiming_global_proof_is_rejected() {
        // Negative test 9 — MPK output treated as a global proof, not a local oracle.
        let mut c = classify_local(3, &[(1, 2), (2, 3), (1, 3)], &catalog()).unwrap();
        c.scope = MpkScope::Global; // someone over-claims authority
        assert_eq!(mpk_projection_gate(&c, &catalog()), Status::Reject);
    }

    #[test]
    fn the_oracle_is_finite_n_at_most_8() {
        // Req. 13.2 — finite local oracle, never a global proof engine.
        assert!(classify_local(9, &[(1, 2)], &catalog()).is_err());
        assert!(classify_local(0, &[], &catalog()).is_err());
    }
}
