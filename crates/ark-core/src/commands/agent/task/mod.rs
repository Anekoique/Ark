//! `ark agent task` — task-lifecycle subcommands.

/// Archives completed tasks.
pub mod archive;
/// Discards an unarchived task.
pub mod discard;
/// Creates task directories and optional git worktrees.
pub mod new;
/// Moves tasks through legal lifecycle phases.
pub mod phase;
/// Promotes tasks between workflow tiers.
pub mod promote;
/// Resumes an active task as this session's focus.
pub mod resume;
/// Manages task-bound git worktrees.
pub mod worktree;

pub use archive::{TaskArchiveOptions, TaskArchiveSummary, task_archive};
pub use discard::{TaskDiscardOptions, TaskDiscardSummary, task_discard};
pub use new::{TaskNewOptions, TaskNewSummary, TaskNewWorktree, TaskNewWorktreeSummary, task_new};
pub use phase::{
    TaskPhaseOptions, TaskPhaseSummary, task_execute, task_plan, task_review, task_verify,
};
pub use promote::{TaskPromoteOptions, TaskPromoteSummary, task_promote};
pub use resume::{TaskResumeOptions, TaskResumeSummary, task_resume};
pub use worktree::{
    WorktreeCleanupOptions, WorktreeCleanupSummary, WorktreeConfig, WorktreeListOptions,
    WorktreeListSummary, WorktreeRow, worktree_cleanup, worktree_list,
};

#[cfg(test)]
mod concurrency_tests;
