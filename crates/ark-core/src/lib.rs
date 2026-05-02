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
/// Per-session identity machinery (UUID cache + parent-id provider).
pub mod session;
/// Persisted state Ark writes to disk: manifest, snapshot, and per-checkout state.
pub mod state;
/// Embedded template trees and walkers.
pub mod templates;

pub use commands::{
    ArchiveOptions, ArchiveSummary, ConflictChoice, ConflictPolicy, ContextOptions, ContextSummary,
    Format as ContextFormat, InitOptions, InitSummary, LoadOptions, LoadSummary, PhaseFilter,
    ProjectedContext, Prompter, RemoveOptions, RemoveSummary, Scope as ContextScope, ScopeTag,
    UnloadOptions, UnloadSummary, UpgradeOptions, UpgradeSummary,
    agent::{
        Phase, Status, TaskToml, Tier,
        spec::{
            SpecExtractOptions, SpecExtractSummary, SpecRegisterOptions, SpecRegisterSummary,
            spec_extract, spec_register,
        },
        task::{
            TaskArchiveMoveOptions, TaskArchiveMoveSummary, TaskCommitOptions, TaskCommitSummary,
            TaskDiscardOptions, TaskDiscardSummary, TaskNewOptions, TaskNewSummary,
            TaskNewWorktree, TaskNewWorktreeSummary, TaskPhaseOptions, TaskPhaseSummary,
            TaskPromoteOptions, TaskPromoteSummary, TaskResumeOptions, TaskResumeSummary,
            VerifyPendingCounts, WorktreeCleanupOptions, WorktreeCleanupSummary, WorktreeConfig,
            WorktreeListOptions, WorktreeListSummary, WorktreeRow, task_archive_move, task_commit,
            task_discard, task_execute, task_new, task_plan, task_promote, task_resume,
            task_review, task_verify, worktree_cleanup, worktree_list,
        },
        workspace::{
            DeveloperRegisterOptions, DeveloperRegisterSummary, DeveloperTouchOptions, Identity,
            RecordMode, RecordOptions, RecordSnapshot, RecordSummary, RecordTransaction,
            ResolveOptions as IdentityResolveOptions, WorkspaceConfig, developer_register,
            developer_touch, identity::identity_prompt, identity_resolve, identity_write,
            workspace_record,
        },
    },
    ark_archive, context, init, load, remove, unload, upgrade,
};
pub use error::{Error, Result};
pub use io::{PathExt, WriteMode, hash_bytes};
pub use layout::Layout;
pub use platforms::{CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM, PLATFORMS, Platform};
pub use session::{
    Ppid, RealPpid, SessionId, StubPpid, cache_file_path, cache_matches, lookup_session_id,
    release_session_id, resolve_session_id,
};
pub use state::{
    Session, StateFile, Tasks, clear_focus_for_slug, load_state, prune_dead_sessions,
    reconcile_against_disk, state_mutate,
};
