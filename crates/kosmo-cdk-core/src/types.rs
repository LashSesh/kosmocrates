//! Ground objects of the Con-Dragger calculus (spec ch. 5, 21, 22), reusing the
//! Kosmocrates substrate: `ContentId` is [`kosmo_core::Digest`] (content-addressed
//! canonical hash) and the fixed-point rational is [`kosmo_core::Q16`].

use kosmo_core::{Digest, Q16};
use serde::{Deserialize, Serialize};

/// The closed status enum (spec ch. 21). Invariant I2 — *gate before score*:
/// these decide; metrics only rank.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Pass,
    Warn,
    Reject,
    Defer,
}

/// The condensation metrics of a stage (spec §18.3): density, purity,
/// irreducibility, curvature, and contradiction energy — each a `Q16` in `[0,1]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageMetrics {
    pub density: Q16,
    pub purity: Q16,
    pub irreducibility: Q16,
    pub curvature: Q16,
    pub contradiction_energy: Q16,
}

impl Default for StageMetrics {
    fn default() -> Self {
        Self {
            density: Q16::ZERO,
            purity: Q16::ZERO,
            irreducibility: Q16::ZERO,
            curvature: Q16::ZERO,
            contradiction_energy: Q16::ZERO,
        }
    }
}

/// A stage `Sn = (An, Un, En, Gn, Tn, Rn, Mn, Cn)` (Def 5.5 / Listing 22.1): the
/// attractor state, its semantically-bearing units, evidence, gate status,
/// trace/replay root, residue, metrics, and certificates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    pub attractor: Digest,
    pub units: Vec<Digest>,
    pub evidence: Digest,
    pub gate_status: Status,
    pub trace_root: Digest,
    pub residue: Digest,
    pub metrics: StageMetrics,
    pub certificates: Vec<Digest>,
}

/// A stage delta `Δn = (In, Fn, Xn, Zn)` (Def 5.7): invariants carried, new
/// fragments, exclusions, and certificates — itself evidence-bound and replayable.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delta {
    pub invariants: Vec<Digest>,
    pub fragments: Vec<Digest>,
    pub exclusions: Vec<Digest>,
    pub certificates: Vec<Digest>,
}

/// A stage-embedding certificate (Listing 21.2): without it the successor is
/// merely a new version, not a valid assimilation stage (Def 8.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageEmbeddingCertificate {
    pub certificate_id: Digest,
    pub from_stage: Digest,
    pub to_stage: Digest,
    pub canonical_equivalence: bool,
    pub semantic_equivalence: bool,
    pub residue_report_id: Digest,
    pub evidence_bundle_id: Digest,
}

/// An attractor stack (Def 5.6 / Listing 21.1): a finite sequence of stages with
/// embedding certificates, an optional fold bundle, and a stack-QSR status. Not a
/// mere history — the formal carrier of the claim that each successor preserves
/// the canonical substance of its predecessor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttractorStack {
    pub stack_id: Digest,
    pub stages: Vec<Digest>,
    pub embedding_certs: Vec<Digest>,
    pub fold_bundle: Option<Digest>,
    pub qsr_status: Status,
}
