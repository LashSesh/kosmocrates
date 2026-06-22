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
//! - [`diamond`] — diamondization (ch. 17): [`press_diamond`] presses a closed
//!   fold into a QSR-certified, irreducible, materializable [`DiamondCubeCandidate`]
//!   (Def. 17.1), report-only by default (ASCC-6).
//!
//! The `kosmo-cdk` CLI (bind/stack/close/diamond/explain) drives the whole fold
//! end-to-end. See `docs/CDK-coffindragger.md`.

pub mod binding;
pub mod diamond;
pub mod run;
pub mod stack;

pub use binding::{bind_systemcube, BoundUnit};
pub use diamond::{is_irreducible, press_diamond, DiamondCubeCandidate, MaterializationProfile};
pub use run::{purge, PurgeResult};
pub use stack::{
    certify_embedding, close_stack, stack_closed, verify_accretion, verify_assimilation,
    verify_contraction, verify_roundtrip, ClosureOutcome, FoldBundle,
};
