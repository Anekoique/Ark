//! `ark agent task worktree list` — enumerate active worktree-backed tasks.
//! Per worktree-support G-5 / C-14: silent on zero rows; sorted by
//! `task.toml.updated_at` desc.

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

#[derive(Debug, Clone)]
pub struct WorktreeListOptions {
    pub project_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WorktreeRow {
    pub slug: String,
    pub branch: String,
    pub worktree_path: PathBuf,
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

#[derive(Debug, Clone)]
pub struct WorktreeListSummary {
    pub rows: Vec<WorktreeRow>,
}

impl fmt::Display for WorktreeListSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // C-14: zero rows → empty stdout. Each row is one line, no header.
        for (i, row) in self.rows.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{row}")?;
        }
        Ok(())
    }
}

pub fn worktree_list(opts: WorktreeListOptions) -> Result<WorktreeListSummary> {
    let layout = Layout::new(&opts.project_root);
    let cfg = WorktreeConfig::load_or_default(&layout)?;
    let worktrees_dir = cfg.resolve_worktrees_dir(&layout);

    let mut rows: Vec<WorktreeRow> = Vec::new();
    for entry in parse_git_worktree_list(layout.root())? {
        if !is_under(&entry.path, &worktrees_dir) {
            continue;
        }
        // C-20 / R-108: silently skip worktrees with missing or unreadable
        // `.current` / `task.toml` (third-party worktrees in this dir).
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
