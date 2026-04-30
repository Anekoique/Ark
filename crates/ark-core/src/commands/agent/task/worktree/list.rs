//! `ark agent task worktree list` enumerates active worktree-backed tasks.
//!
//! Output is silent on zero rows and sorted by `task.toml.updated_at` desc.

use std::{fmt, path::PathBuf};

use chrono::{DateTime, Utc};

use crate::{
    commands::agent::{
        state::TaskToml,
        task::worktree::{
            WorktreeConfig,
            discovery::{is_under, parse_git_worktree_list},
        },
    },
    error::Result,
    io::PathExt,
    layout::Layout,
};

/// Options for listing task-bound worktrees.
#[derive(Debug, Clone)]
pub struct WorktreeListOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
}

/// One rendered row in `ark agent task worktree list`.
#[derive(Debug, Clone)]
pub struct WorktreeRow {
    /// Task slug bound to the worktree.
    pub slug: String,
    /// Branch checked out by the worktree.
    pub branch: String,
    /// Path to the worktree checkout.
    pub worktree_path: PathBuf,
    /// Task update timestamp used for sorting.
    pub updated_at: DateTime<Utc>,
}

impl fmt::Display for WorktreeRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.slug,
            self.branch,
            self.worktree_path.display()
        )
    }
}

/// Summary of task-bound worktrees.
#[derive(Debug, Clone)]
pub struct WorktreeListSummary {
    /// Rows sorted by task update time descending.
    pub rows: Vec<WorktreeRow>,
}

impl fmt::Display for WorktreeListSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Zero rows → empty stdout. Each row is one line, no header.
        for (i, row) in self.rows.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{row}")?;
        }
        Ok(())
    }
}

/// Lists task-bound worktrees under the configured worktree root.
pub fn worktree_list(opts: WorktreeListOptions) -> Result<WorktreeListSummary> {
    let layout = Layout::new(&opts.project_root);
    let cfg = WorktreeConfig::load_or_default(&layout)?;
    let worktrees_dir = cfg.resolve_worktrees_dir(&layout);

    let mut rows: Vec<WorktreeRow> = Vec::new();
    for entry in parse_git_worktree_list(layout.root())? {
        if !is_under(&entry.path, &worktrees_dir) {
            continue;
        }
        // Silently skip worktrees with missing or unreadable `.current`
        // / `task.toml` (third-party worktrees in this dir).
        let Ok(current_text) = entry.path.join(".ark/tasks/.current").read_text() else {
            continue;
        };
        let slug = current_text.trim().to_string();
        if slug.is_empty() {
            continue;
        }
        let task_dir = entry.path.join(".ark/tasks").join(&slug);
        let Ok(toml) = TaskToml::load(&task_dir) else {
            continue;
        };
        let branch = toml
            .branch
            .or(entry.branch.clone())
            .unwrap_or_else(|| "(unknown)".into());
        rows.push(WorktreeRow {
            slug,
            branch,
            worktree_path: entry.path,
            updated_at: toml.updated_at,
        });
    }
    rows.sort_by_key(|r| std::cmp::Reverse(r.updated_at));
    Ok(WorktreeListSummary { rows })
}
