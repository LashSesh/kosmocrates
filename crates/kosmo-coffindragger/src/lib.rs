//! # kosmo-coffindragger — the Con-Dragger / ASCC pipeline core (L7)
//!
//! The global attractor-centralization and stack-closure layer of the
//! **Coffin-Dragger** master spec (`specs/coffindragger master spec.pdf`), built
//! on [`kosmo_cdk_core`]. Report-only, content-addressed, fail-closed — no host
//! mutation.
//!
//! - [`binding`] — KBL, the Kosmocrates Binding Layer (ch. 19): a [`BoundUnit`]
//!   is pullable into a DiamondCube only under invariant 19.1 (evidence + trace +
//!   replay + accepting status). Conformance class KBL-1.
//! - [`stack`] — ASCC, the Attractor Stack Closure Calculus (ch. 8): canonical
//!   embedding certification (ASCC-1), support-preserving accretion with no covert
//!   removal (ASCC-2), contractive consolidation (ASCC-3), the content-addressed
//!   [`FoldBundle`] and the closure run [`close_stack`] → a `DiamondCandidate` or a
//!   `DeferredStackReport` (ASCC-4/5), fail-closed under stack-QSR.
//! - [`run`] — CDK, the Con-Dragger cycle (ch. 3.2/10): the Purge exkalibration
//!   primitive of `Cen ∘ Probe ∘ Seed ∘ Purge ∘ Pull`.
//!
//! Diamondization (`Diamondize(W) = Core(Fix(Ω(W)))` → a QSR-certified
//! `DiamondCubeCandidate`) and the `kosmo-cdk` CLI (bind/stack/close/diamond/
//! explain) are the next ticket (C3). See `docs/CDK-coffindragger.md`.

pub mod binding;
pub mod run;
pub mod stack;

pub use binding::BoundUnit;
pub use run::{purge, PurgeResult};
pub use stack::{
    certify_embedding, close_stack, stack_closed, verify_accretion, verify_contraction,
    ClosureOutcome, FoldBundle,
};
