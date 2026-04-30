//! `ark agent workspace` — per-developer session journals.
//!
//! Identity has two bootstrap paths:
//! - `ark init` prompts for a developer name interactively (default), or
//!   takes `--developer <name>` / `--no-developer` non-interactively.
//! - `ark agent workspace init --name <x>` for already-installed projects
//!   and idempotent re-init.
//!
//! Auto-record on `task archive` is a no-op when identity is missing or
//! when `[workspace].auto_record_on_archive = false` in `.ark/config.toml`.
//!
//! Journals are **per-checkout**: a record from inside a git worktree writes
//! to that worktree's `.ark/workspace/<dev>/`, not the parent's. The session
//! entry rides along with the task commit on the same branch.
//!
//! Module shape:
//! - `config`   — `WorkspaceConfig` ([workspace] section of config.toml).
//! - `identity` — read/write `.developer`; name validation.
//! - `journal`  — `JournalEntry` render; `parse_entries`; `parse_oneline`.
//! - `index`    — `index.md` re-render via managed blocks.
//! - `init`     — `workspace_init`.
//! - `record`   — `workspace_record` (manual) + `record_task` (task bridge).

/// Loads workspace configuration from Ark config.
pub mod config;
/// Reads and writes the per-machine developer identity.
pub mod identity;
/// Renders the per-developer workspace index.
pub mod index;
/// Initializes per-developer workspace state.
pub mod init;
/// Renders, parses, and rotates workspace journals.
pub mod journal;
/// Records manual and task-archive sessions.
pub mod record;

pub use config::WorkspaceConfig;
pub use init::{WorkspaceInitOptions, WorkspaceInitSummary, workspace_init};
pub use record::{
    RecordTaskOptions, WorkspaceRecordOptions, WorkspaceRecordSummary, WorkspaceRecorded,
    record_task, workspace_record,
};
