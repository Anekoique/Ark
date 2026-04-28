//! `ark agent task` — task-lifecycle subcommands.

pub mod archive;
pub mod new;
pub mod phase;
pub mod promote;
pub mod worktree;

pub use archive::{TaskArchiveOptions, TaskArchiveSummary, task_archive};
pub use new::{TaskNewOptions, TaskNewSummary, TaskNewWorktree, TaskNewWorktreeSummary, task_new};
pub use phase::{
    TaskPhaseOptions, TaskPhaseSummary, task_execute, task_plan, task_review, task_verify,
};
pub use promote::{TaskPromoteOptions, TaskPromoteSummary, task_promote};
pub use worktree::{
    WorktreeCleanupOptions, WorktreeCleanupSummary, WorktreeConfig, WorktreeListOptions,
    WorktreeListSummary, WorktreeRow, worktree_cleanup, worktree_list,
};
