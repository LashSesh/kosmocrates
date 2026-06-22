//! # kosmo-wings — Wings of Samael (L3) + Ophanim (L6)
//!
//! The radial projection-arm and closed-cycle layer of the **Coffin-Dragger**
//! master spec (`specs/coffindragger master spec.pdf`), conformance class CDK-W.
//! Built on [`kosmo_cdk_core`]; report-only, content-addressed, fail-closed.
//!
//! - [`wing`] — a [`Wing`] is `anchor ∘ condense ∘ harvest ∘ orbit ∘ probe`
//!   (Def. 11.1); every admissible wing **condenses** (Inv. 11.2), enforced by
//!   [`wing_gate`] together with I3 (no covert removal — dropped mass is visible
//!   residue).
//! - [`mesh`] — the wing algebra ([`Composition`]) and the [`WingMesh`];
//!   conflicts are exkalibrated, never swallowed.
//! - [`ophanim`] — an [`OphanimCycle`] is a closed wing cycle with roundtrip
//!   self-calibration `d(Ω^t(x), A⋆) → 0`, admissible only if replayable and
//!   convergent (the OphanimRoundtripGate).
//!
//! The local symmetry oracle binding (MPK / `pse-metatron`) and the CDK run /
//! KBL / diamondization (`kosmo-coffindragger`) are the following tickets; see
//! `docs/CDK-coffindragger.md`.

pub mod mesh;
pub mod mpk_bridge;
pub mod ophanim;
pub mod wing;

pub use mesh::{Composition, WingMesh};
pub use mpk_bridge::{classify_local, mpk_projection_gate, MpkCatalogBinding, MpkClassification, MpkScope};
pub use ophanim::OphanimCycle;
pub use wing::{wing_gate, CrystalWing, Structure, Wing, WingKind, WingOutput};
