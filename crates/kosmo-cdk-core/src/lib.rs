//! # kosmo-cdk-core — Con-Dragger / ASCC core (the C0 report-only foundation)
//!
//! The L1 (QSR) / L7 (Con-Dragger) calculus core of the **Coffin-Dragger & Wings
//! of Samael** master specification (`specs/coffindragger master spec.pdf`),
//! realized natively against the Kosmocrates substrate.
//!
//! This crate is the report-only **C0** layer of the staged hand-off: it defines
//! the ground objects ([`Stage`], [`AttractorStack`], [`Delta`],
//! [`StageEmbeddingCertificate`]), the condensation [`metrics`] (the tripolar
//! D-energy `D = ψ·ρ·ω` and §18 stack/cube metrics), and the [`qsr`] predicate —
//! the structural-rigidity guard. It performs **no host mutation** and reuses
//! `kosmo_core` for content addressing ([`kosmo_core::Digest`]) and fixed-point
//! arithmetic ([`kosmo_core::Q16`]).
//!
//! ## The seven core invariants (spec §2.2) this layer enforces
//! - **I1 Structure over surface** — ids derive from canonical serialization.
//! - **I2 Gate before score** — [`qsr::qsr_stage`] lets a gate decide; metrics
//!   only rank ([`metrics::objective`]).
//! - **I3 Boundary over absorption** — non-integrable mass is residue, never
//!   silently merged (the `residue` field; the closure crate enforces the
//!   accretion contract).
//! - **I4 Trace and replay** — every object is content-addressed via [`Canonical`].
//! - **I5 Fixpoint over target text** — QSR-monotone convergence to an attractor.
//! - **I6 Projection over omniscience** — (the wings/panoptic crates).
//! - **I7 Condensation over growth** — [`metrics::stage_progress`] +
//!   QSR-monotonicity (density/purity/irreducibility rise, contradiction falls).
//!
//! The wings/Ophanim layer (`kosmo-wings`), the CDK run + KBL binding +
//! diamondization (`kosmo-coffindragger`), and the `kosmo-cdk` CLI (bind / stack
//! / close / diamond / explain) build on this foundation in later tickets. See
//! `docs/CDK-coffindragger.md` for the full repository-native specification.

pub mod metrics;
pub mod qsr;
pub mod types;

pub use metrics::{d_energy, objective, stage_progress};
pub use qsr::{qsr_stack, qsr_stage};
pub use types::{
    AttractorStack, Delta, Stage, StageEmbeddingCertificate, StageMetrics, Status,
};

use kosmo_core::Digest;
use thiserror::Error;

/// Fail-closed error for the CDK calculus (all fallible operations return this,
/// so failure is never silent — invariants I3/I4).
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum GateError {
    /// A gate rejected the operation.
    #[error("gate rejected: {0}")]
    Rejected(String),
    /// Non-integrable mass was not exkalibrated to the boundary.
    #[error("boundary violation (non-integrable mass not exkalibrated): {0}")]
    Boundary(String),
    /// Replay could not be guaranteed.
    #[error("replay incomplete: {0}")]
    ReplayIncomplete(String),
}

/// Canonicalization (I1 / I4): every id-relevant object derives its identity from
/// its canonical serialization (`Digest::of` = JCS + SHA-256), never from surface.
pub trait Canonical {
    fn canon(&self) -> Digest;
}

impl Canonical for Stage {
    fn canon(&self) -> Digest {
        Digest::of(self)
    }
}

impl Canonical for AttractorStack {
    fn canon(&self) -> Digest {
        Digest::of(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kosmo_core::Q16;

    fn stage_with(units: Vec<Digest>) -> Stage {
        Stage {
            attractor: Digest::ZERO,
            units,
            evidence: Digest::ZERO,
            gate_status: Status::Pass,
            trace_root: Digest::ZERO,
            residue: Digest::ZERO,
            metrics: StageMetrics::default(),
            certificates: vec![],
        }
    }

    #[test]
    fn canon_is_deterministic_and_content_addressed() {
        // I1/I4 — identity is the canonical hash: same content → same id, different
        // content → different id.
        let a = stage_with(vec![Digest::of_bytes(b"u1")]);
        let b = stage_with(vec![Digest::of_bytes(b"u1")]);
        let c = stage_with(vec![Digest::of_bytes(b"u2")]);
        assert_eq!(a.canon(), b.canon(), "same content → same id");
        assert_ne!(a.canon(), c.canon(), "different content → different id");
    }

    #[test]
    fn the_calculus_reuses_the_kosmocrates_substrate() {
        // D-energy and QSR compose: a gated, monotone, high-D stage is accepting.
        let d = d_energy(Q16::ONE, Q16::ONE, Q16::ONE);
        assert_eq!(d, Q16::ONE);
        assert_eq!(qsr_stage(&stage_with(vec![]), None), Status::Pass);
    }
}
