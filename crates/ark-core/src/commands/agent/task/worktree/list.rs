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
    layout::Layout,
    state::load_state,
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
        // Silently skip worktrees with unreadable state or `task.toml`
        // (third-party worktrees living under this dir).
        let wt_layout = Layout::new(&entry.path);
        let Ok(state) = load_state(&wt_layout) else {
            continue;
        };
        for slug in state.tasks.active {
            let task_dir = wt_layout.task_dir(&slug);
            let Ok(toml) = TaskToml::load(&task_dir) else {
                continue;
            };
            let branch = toml
                .branch
                .clone()
                .or(entry.branch.clone())
                .unwrap_or_else(|| "(unknown)".into());
            rows.push(WorktreeRow {
                slug,
                branch,
                worktree_path: entry.path.clone(),
                updated_at: toml.updated_at,
            });
        }
    }
    rows.sort_by_key(|r| std::cmp::Reverse(r.updated_at));
    Ok(WorktreeListSummary { rows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::agent::{
            state::Tier,
            task::new::{TaskNewOptions, TaskNewWorktree, task_new},
        },
        io::{PathExt, git::run_git},
    };

    fn init_repo_with_commit() -> tempfile::TempDir {
        use crate::commands::agent::workspace::{Identity, identity_write};
        let tmp = tempfile::tempdir().unwrap();
        run_git(&["init", "--quiet"], tmp.path()).unwrap();
        run_git(&["config", "user.email", "test@example.com"], tmp.path()).unwrap();
        run_git(&["config", "user.name", "Test"], tmp.path()).unwrap();
        run_git(&["config", "commit.gpgsign", "false"], tmp.path()).unwrap();
        run_git(&["checkout", "-b", "main"], tmp.path()).unwrap();
        tmp.path()
            .join("README.md")
            .write_bytes(b"# repo\n")
            .unwrap();
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(&["commit", "-m", "init", "--quiet"], tmp.path()).unwrap();
        identity_write(tmp.path(), &Identity::new("test-dev").unwrap()).unwrap();
        tmp
    }

    /// Verifies that `worktree_list` enumerates active worktree-backed tasks.
    #[test]
    fn worktree_list_reports_active_tasks() {
        let tmp = init_repo_with_commit();
        for slug in ["alpha", "beta"] {
            task_new(TaskNewOptions {
                project_root: tmp.path().to_path_buf(),
                slug: slug.into(),
                title: slug.into(),
                tier: Tier::Standard,
                worktree: Some(TaskNewWorktree::default()),
            })
            .unwrap();
        }
        let summary = worktree_list(WorktreeListOptions {
            project_root: tmp.path().to_path_buf(),
        })
        .unwrap();
        let slugs: Vec<&str> = summary.rows.iter().map(|r| r.slug.as_str()).collect();
        assert_eq!(slugs.len(), 2);
        assert!(slugs.contains(&"alpha"));
        assert!(slugs.contains(&"beta"));
    }

    /// Verifies that `worktree_list` returns no rows when the worktree dir is empty.
    #[test]
    fn worktree_list_is_empty_when_no_worktrees_exist() {
        let tmp = init_repo_with_commit();
        let summary = worktree_list(WorktreeListOptions {
            project_root: tmp.path().to_path_buf(),
        })
        .unwrap();
        assert!(summary.rows.is_empty());
    }
}
