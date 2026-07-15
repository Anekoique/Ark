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
/// Persisted state Ark writes to disk: manifest, snapshot, and per-checkout state.
pub mod state;
/// Embedded template trees and walkers.
pub mod templates;

pub use commands::{
    ActionLabel, ArchiveOptions, ArchiveSummary, CleanupOptions, CleanupReason, CleanupRow,
    CleanupSummary, ConflictChoice, ConflictPolicy, ContextOptions, ContextSummary, DryRunPreview,
    Format as ContextFormat, InitOptions, InitSummary, LoadOptions, LoadSummary, PhaseFilter,
    ProjectedContext, Prompter, RemoveOptions, RemoveSummary, RestoreSummary, SandboxCreateOptions,
    SandboxCreateSummary, SandboxEnterOptions, SandboxEnterSummary, SandboxListOptions,
    SandboxListSummary, SandboxRmOptions, SandboxRmSummary, SandboxRow, SandboxWarmupOptions,
    SandboxWarmupSummary, Scope as ContextScope, ScopeTag, UnloadOptions, UnloadSummary,
    UpgradeOptions, UpgradeSummary,
    agent::{
        Phase, Status, TaskToml, Tier,
        spec::{
            SpecExtractOptions, SpecExtractSummary, SpecImportOptions, SpecImportSummary,
            SpecRegisterOptions, SpecRegisterSummary, spec_extract, spec_import, spec_register,
        },
        task::{
            TaskArchiveMoveOptions, TaskArchiveMoveSummary, TaskCommitOptions, TaskCommitSummary,
            TaskDiscardOptions, TaskDiscardSummary, TaskNewOptions, TaskNewSummary,
            TaskNewWorktree, TaskNewWorktreeSummary, TaskPhaseOptions, TaskPhaseSummary,
            TaskPromoteOptions, TaskPromoteSummary, TaskResumeOptions, TaskResumeSummary,
            VerifyPendingCounts, WorktreeCleanupOptions, WorktreeCleanupSummary, WorktreeConfig,
            WorktreeListOptions, WorktreeListSummary, WorktreeRow, parse_spec_path,
            task_archive_move, task_commit, task_discard, task_execute, task_new, task_plan,
            task_promote, task_resume, task_review, task_verify, worktree_cleanup, worktree_list,
        },
        workspace::{
            DeveloperRegisterOptions, DeveloperRegisterSummary, DeveloperTouchOptions, Identity,
            RecordMode, RecordOptions, RecordSnapshot, RecordSummary, RecordTransaction,
            ResolveOptions as IdentityResolveOptions, WorkspaceConfig, developer_register,
            developer_touch, identity::identity_prompt, identity_resolve, identity_write,
            scaffold_developer_dir, workspace_record,
        },
    },
    ark_archive, cleanup, context, init, load, remove, restore, sandbox_create, sandbox_enter,
    sandbox_list, sandbox_rm, sandbox_warmup, unload, upgrade,
};
pub use error::{Error, Result};
pub use io::{PathExt, WriteMode, hash_bytes};
pub use layout::Layout;
pub use platforms::{
    CLAUDE_PLATFORM, CODEAGENT_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM, PLATFORMS, Platform,
};
pub use state::{
    Manifest, StateFile, Tasks, clear_focus_for_slug, load_state, reconcile_against_disk,
    state_mutate,
};
pub use templates::{
    CLAUDE_AGENT_TEMPLATES, CODEAGENT_AGENT_TEMPLATES, CODEX_AGENT_TEMPLATES,
    OPENCODE_AGENT_TEMPLATES,
};
