//! `ark agent task` — task-lifecycle subcommands.

/// Side-effect-free archive helper: rename + state cleanup only.
pub mod archive;
/// Atomic task-closure step: VERIFY gate, SPEC extract, journal, commit.
pub mod commit;
/// Discards an unarchived task.
pub mod discard;
/// Creates task directories and optional git worktrees.
pub mod new;
/// Moves tasks through legal lifecycle phases.
pub mod phase;
/// PRD parsers (e.g. `[**SPEC Path**]`).
pub mod prd;
/// Promotes tasks between workflow tiers.
pub mod promote;
/// Resumes an active task as this session's focus.
pub mod resume;
/// Seed-time substitution for the `VERIFY.md` template.
pub mod verify_seed;
/// Manages task-bound git worktrees.
pub mod worktree;

pub use archive::{TaskArchiveMoveOptions, TaskArchiveMoveSummary, task_archive_move};
pub use commit::{TaskCommitOptions, TaskCommitSummary, VerifyPendingCounts, task_commit};
pub use discard::{TaskDiscardOptions, TaskDiscardSummary, task_discard};
pub use new::{TaskNewOptions, TaskNewSummary, TaskNewWorktree, TaskNewWorktreeSummary, task_new};
pub use phase::{
    TaskPhaseOptions, TaskPhaseSummary, task_execute, task_plan, task_review, task_verify,
};
pub use prd::parse_spec_path;
pub use promote::{TaskPromoteOptions, TaskPromoteSummary, task_promote};
pub use resume::{TaskResumeOptions, TaskResumeSummary, task_resume};
pub use worktree::{
    WorktreeCleanupOptions, WorktreeCleanupSummary, WorktreeConfig, WorktreeListOptions,
    WorktreeListSummary, WorktreeRow, worktree_cleanup, worktree_list,
};

#[cfg(test)]
mod concurrency_tests;
