//! Per-developer journal trees and identity machinery.
//!
//! Records each developer's task and manual sessions under
//! `.ark/workspace/<dev>/journal-N.md`, with a top-level `index.md`
//! listing active developers and a per-developer `index.md` listing
//! sessions. Identity lives in `.ark/.developer` (gitignored).
//!
//! Phase 1 of the workspace feature ships identity + config; Phase 2 adds
//! the developer registrar; Phase 3 adds the record primitive with
//! transaction support.

/// `[workspace]` section of `.ark/config.toml`.
pub mod config;
/// Top-level Active Developers index registrar.
pub mod developer;
/// `.ark/.developer` read/write/prompt and identity resolution.
pub mod identity;
/// Single journal-append primitive (task + manual modes).
pub mod record;
/// Auto-field stamping helper for the agent's pre-written journal entry.
pub mod stamp;
/// Atomic snapshot/rollback for journal append + index updates.
pub mod transaction;

pub use config::WorkspaceConfig;
pub use developer::{
    DeveloperRegisterOptions, DeveloperRegisterSummary, DeveloperTouchOptions, developer_register,
    developer_touch, scaffold_developer_dir, scaffold_top_index,
};
pub use identity::{Identity, ResolveOptions, identity_resolve, identity_write};
pub use record::{RecordMode, RecordOptions, RecordSummary, workspace_record};
pub use stamp::{ManualStampFields, TaskStampFields, stamp_manual, stamp_task};
pub use transaction::{RecordSnapshot, RecordTransaction};
