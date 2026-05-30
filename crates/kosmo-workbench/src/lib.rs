//! Kosmocrates Workbench — production substrate for HYPHAE and higher layers.
//!
//! Provides workspace indexing, task declaration, context packing,
//! Foundry check execution, and dry-run report generation.
//!
//! Default operating mode is `ReportOnly`: workspace is scanned and reports
//! are produced, but no host files are written and no external code is executed
//! unless the `PolicyProfile` explicitly permits it.

pub mod context_pack;
pub mod foundry;
pub mod report;
pub mod task_spec;
pub mod workspace;

pub use context_pack::{ContextEntryKind, ContextPack, ContextPackEntry, ContextPackError, PermittedUse};
pub use foundry::{FoundryCheckSpec, FoundryRunOutput, FoundryRunner};
pub use report::RunReport;
pub use task_spec::{TaskKind, TaskSpec};
pub use workspace::{WorkspaceEntry, WorkspaceEntryKind, WorkspaceError, WorkspaceIndex};
