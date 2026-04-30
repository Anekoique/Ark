//! `ark-core` — library that drives the `ark` CLI.
//!
//! The CLI is a thin shell over this crate. All scaffolding, file writing,
//! snapshotting, and manifest tracking lives here.

/// Command implementations and summaries.
pub mod commands;
/// Error types shared by Ark operations.
pub mod error;
/// Filesystem, content, and subprocess I/O helpers.
pub mod io;
/// Project layout constants and rooted path helpers.
pub mod layout;
/// Platform registry and platform-specific install behavior.
pub mod platforms;
/// Persisted manifest and snapshot state.
pub mod state;
/// Embedded template trees and walkers.
pub mod templates;

pub use commands::{
    ConflictChoice, ConflictPolicy, ContextOptions, ContextSummary, Format as ContextFormat,
    InitOptions, InitSummary, LoadOptions, LoadSummary, PhaseFilter, ProjectedContext, Prompter,
    RemoveOptions, RemoveSummary, Scope as ContextScope, ScopeTag, UnloadOptions, UnloadSummary,
    UpgradeOptions, UpgradeSummary,
    agent::{
        Phase, Status, TaskToml, Tier,
        spec::{
            SpecExtractOptions, SpecExtractSummary, SpecRegisterOptions, SpecRegisterSummary,
            spec_extract, spec_register,
        },
        task::{
            TaskArchiveOptions, TaskArchiveSummary, TaskNewOptions, TaskNewSummary,
            TaskNewWorktree, TaskNewWorktreeSummary, TaskPhaseOptions, TaskPhaseSummary,
            TaskPromoteOptions, TaskPromoteSummary, WorktreeCleanupOptions, WorktreeCleanupSummary,
            WorktreeConfig, WorktreeListOptions, WorktreeListSummary, WorktreeRow, task_archive,
            task_execute, task_new, task_plan, task_promote, task_review, task_verify,
            worktree_cleanup, worktree_list,
        },
        workspace::{
            RecordTaskOptions, WorkspaceConfig, WorkspaceInitOptions, WorkspaceInitSummary,
            WorkspaceRecordOptions, WorkspaceRecordSummary, WorkspaceRecorded, record_task,
            workspace_init, workspace_record,
        },
    },
    context, init, load, remove, unload, upgrade,
};
pub use error::{Error, Result};
pub use io::{PathExt, WriteMode, hash_bytes};
pub use layout::Layout;
pub use platforms::{CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM, PLATFORMS, Platform};
