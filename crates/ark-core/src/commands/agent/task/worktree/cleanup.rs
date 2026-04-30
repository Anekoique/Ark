//! `ark agent task worktree cleanup` removes worktree-backed task checkouts.
//!
//! The command can optionally delete the branch. Discovery uses
//! `git worktree list --porcelain`.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use crate::{
    commands::agent::{
        state::validate_slug,
        task::worktree::{WorktreeConfig, discovery::find_worktree_for_slug},
    },
    error::{Error, Result},
    io::{PathExt, git::run_git},
    layout::Layout,
};

/// Options for removing a task-bound worktree.
#[derive(Debug, Clone)]
pub struct WorktreeCleanupOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Task slug whose worktree should be removed.
    pub slug: String,
    /// Reports whether the backing branch should also be deleted.
    pub delete_branch: bool,
    /// Reports whether dirty worktrees and branch deletion should be forced.
    pub force: bool,
}

/// Summary of a task-bound worktree cleanup.
#[derive(Debug, Clone)]
pub struct WorktreeCleanupSummary {
    /// Task slug whose worktree was removed.
    pub slug: String,
    /// Branch associated with the removed worktree.
    pub branch: Option<String>,
    /// Reports whether the branch was deleted.
    pub branch_deleted: bool,
    /// Path to the removed worktree checkout.
    pub worktree_path: PathBuf,
}

impl fmt::Display for WorktreeCleanupSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "removed worktree for task `{}` at {}",
            self.slug,
            self.worktree_path.display()
        )?;
        if self.branch_deleted
            && let Some(b) = &self.branch
        {
            write!(f, " (deleted branch `{b}`)")?;
        }
        Ok(())
    }
}

/// Removes a task-bound worktree and optionally deletes its branch.
pub fn worktree_cleanup(opts: WorktreeCleanupOptions) -> Result<WorktreeCleanupSummary> {
    validate_slug(&opts.slug)?;

    let layout = Layout::new(&opts.project_root);
    let cfg = WorktreeConfig::load_or_default(&layout)?;
    let worktrees_dir = cfg.resolve_worktrees_dir(&layout);

    // Discovery uses `git worktree list --porcelain`. Missing maps to
    // `WorktreeNotFound`, which keeps repeated cleanup script-safe.
    let Some((wt_path, toml)) = find_worktree_for_slug(layout.root(), &worktrees_dir, &opts.slug)?
    else {
        return Err(Error::WorktreeNotFound { slug: opts.slug });
    };

    // Dirty check unless --force. A non-zero exit (broken repo,
    // permission issue, stale path) must NOT be treated as clean — surface
    // it instead of silently proceeding to remove the worktree.
    if !opts.force {
        let status = run_git(&["status", "--porcelain"], &wt_path)?;
        if !status.is_success() {
            return Err(git_error(
                &wt_path,
                "git status --porcelain",
                &status.stderr,
            ));
        }
        if !status.stdout.trim().is_empty() {
            return Err(Error::WorktreeDirty { path: wt_path });
        }
    }

    let wt_str = wt_path
        .to_str()
        .expect("worktree paths are utf-8 by construction");
    let mut remove_args: Vec<&str> = vec!["worktree", "remove"];
    if opts.force {
        remove_args.push("--force");
    }
    remove_args.push(wt_str);
    let remove_out = run_git(&remove_args, layout.root())?;
    if !remove_out.is_success() {
        return Err(git_error(
            &wt_path,
            "git worktree remove",
            &remove_out.stderr,
        ));
    }

    // Surface git's error for unmerged branches; user re-runs with
    // --force to escalate to `-D`.
    let mut branch_deleted = false;
    if opts.delete_branch
        && let Some(branch) = toml.branch.as_deref()
    {
        let flag = if opts.force { "-D" } else { "-d" };
        let out = run_git(&["branch", flag, branch], layout.root())?;
        if !out.is_success() {
            return Err(git_error(
                layout.root(),
                &format!("git branch {flag} {branch}"),
                &out.stderr,
            ));
        }
        branch_deleted = true;
    }

    // Prune empty parent dirs up to (not including) worktrees_dir().
    prune_empty_parents(&wt_path, &worktrees_dir)?;

    Ok(WorktreeCleanupSummary {
        slug: opts.slug,
        branch: toml.branch,
        branch_deleted,
        worktree_path: wt_path,
    })
}

/// Wraps a non-zero git exit as [`Error::Io`] carrying the trimmed stderr.
fn git_error(path: &Path, action: &str, stderr: &str) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(format!("{action} failed: {}", stderr.trim())),
    }
}

/// Removes empty parent directories until `stop_at`.
///
/// Walks up from `start.parent()` toward `stop_at` without removing
/// `stop_at` itself. Stops as soon as a dir is non-empty or reaches
/// `stop_at`.
///
/// macOS canonicalizes `/tmp` → `/private/tmp`; `git worktree remove` reports
/// the canonical path while `Layout` paths are not canonicalized. Comparing
/// canonical forms keeps the loop bounded.
fn prune_empty_parents(start: &Path, stop_at: &Path) -> Result<()> {
    let canonicalize = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let stop = canonicalize(stop_at);

    let mut cursor = start.parent();
    while let Some(dir) = cursor {
        if canonicalize(dir) == stop {
            break;
        }
        dir.remove_dir_if_empty()?;
        if dir.exists() {
            break;
        }
        cursor = dir.parent();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::{
        state::Tier,
        task::{
            new::{TaskNewOptions, TaskNewWorktree, task_new},
            worktree::list::{WorktreeListOptions, worktree_list},
        },
    };

    fn init_repo() -> tempfile::TempDir {
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
        tmp
    }

    fn create_worktree(tmp: &std::path::Path, slug: &str) {
        task_new(TaskNewOptions {
            project_root: tmp.to_path_buf(),
            slug: slug.into(),
            title: format!("{slug} task"),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap();
    }

    /// Commits scaffolding so cleanup without `--force` succeeds.
    ///
    /// Mirrors the user's typical workflow: scaffold → commit → work → merge
    /// → cleanup.
    fn commit_scaffolding(tmp: &std::path::Path, slug: &str) {
        let wt = tmp.join(".ark/worktrees/feat").join(slug);
        run_git(&["add", "."], &wt).unwrap();
        run_git(&["commit", "-m", "scaffold", "--quiet"], &wt).unwrap();
    }

    /// Verifies cleanup with `--delete-branch`.
    ///
    /// The worktree dir is removed, empty parent dirs are pruned up to
    /// `.ark/worktrees/`, and the branch is deleted.
    #[test]
    fn cleanup_happy_path_with_delete_branch() {
        let tmp = init_repo();
        create_worktree(tmp.path(), "foo");
        commit_scaffolding(tmp.path(), "foo");
        let wt = tmp.path().join(".ark/worktrees/feat/foo");
        assert!(wt.is_dir());

        let summary = worktree_cleanup(WorktreeCleanupOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            delete_branch: true,
            force: true, // unmerged branch → -d would refuse; use -D.
        })
        .unwrap();

        assert!(!wt.exists(), "worktree dir should be gone");
        assert!(summary.branch_deleted);
        // Empty parent `feat/` pruned, but `worktrees/` stays.
        assert!(!tmp.path().join(".ark/worktrees/feat").exists());
        assert!(tmp.path().join(".ark/worktrees").exists());
        // Branch gone.
        let branches = run_git(&["branch", "--list", "feat/foo"], tmp.path()).unwrap();
        assert!(branches.stdout.trim().is_empty());
    }

    /// Verifies that cleanup of a dirty worktree without `--force` is rejected.
    ///
    /// The scaffolding from `task new --worktree` is uncommitted, which alone trips
    /// the dirty check (the contract is: commit or `--force`).
    #[test]
    fn cleanup_on_dirty_without_force_rejected() {
        let tmp = init_repo();
        create_worktree(tmp.path(), "foo");
        let wt = tmp.path().join(".ark/worktrees/feat/foo");

        let err = worktree_cleanup(WorktreeCleanupOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            delete_branch: false,
            force: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::WorktreeDirty { .. }));
        assert!(wt.is_dir(), "dirty cleanup should not have removed");
    }

    /// Verifies repeated cleanup returns `WorktreeNotFound`.
    #[test]
    fn cleanup_idempotent_via_second_run() {
        let tmp = init_repo();
        create_worktree(tmp.path(), "foo");
        commit_scaffolding(tmp.path(), "foo");

        worktree_cleanup(WorktreeCleanupOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            delete_branch: true,
            force: true,
        })
        .unwrap();

        // Second run: discovery miss → WorktreeNotFound (script-safe).
        let err = worktree_cleanup(WorktreeCleanupOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            delete_branch: false,
            force: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::WorktreeNotFound { .. }));
    }

    /// Verifies that `worktree list` skips non-worktree tasks.
    #[test]
    fn worktree_list_round_trip() {
        let tmp = init_repo();
        create_worktree(tmp.path(), "alpha");
        create_worktree(tmp.path(), "beta");

        // Plus a non-worktreed task on the parent.
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "no-wt".into(),
            title: "plain".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();

        let summary = worktree_list(WorktreeListOptions {
            project_root: tmp.path().to_path_buf(),
        })
        .unwrap();
        assert_eq!(summary.rows.len(), 2);
        let slugs: Vec<&str> = summary.rows.iter().map(|r| r.slug.as_str()).collect();
        assert!(slugs.contains(&"alpha"));
        assert!(slugs.contains(&"beta"));
    }

    /// Verifies that zero rows produces empty `Display` output (no header).
    #[test]
    fn worktree_list_zero_rows_silent() {
        let tmp = init_repo();
        let summary = worktree_list(WorktreeListOptions {
            project_root: tmp.path().to_path_buf(),
        })
        .unwrap();
        assert!(summary.rows.is_empty());
        assert_eq!(format!("{summary}"), "");
    }

    /// Verifies that archive leaves worktree state intact.
    ///
    /// `worktree cleanup` frees the worktree dir and branch; `archive` never
    /// does.
    #[test]
    fn archive_leaves_worktree_intact() {
        use crate::commands::agent::task::{
            archive::{TaskArchiveOptions, task_archive},
            phase::{TaskPhaseOptions, task_execute, task_plan, task_verify},
        };

        let tmp = init_repo();
        create_worktree(tmp.path(), "foo");
        let wt = tmp.path().join(".ark/worktrees/feat/foo");
        // Advance phases inside the worktree (which is its own project root).
        task_plan(TaskPhaseOptions {
            project_root: wt.clone(),
            slug: "foo".into(),
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: wt.clone(),
            slug: "foo".into(),
        })
        .unwrap();
        task_verify(TaskPhaseOptions {
            project_root: wt.clone(),
            slug: "foo".into(),
        })
        .unwrap();
        task_archive(TaskArchiveOptions {
            project_root: wt.clone(),
            slug: "foo".into(),
        })
        .unwrap();

        // Worktree dir + branch survive archive.
        assert!(wt.is_dir(), "worktree dir must survive archive");
        let branches = run_git(&["branch", "--list", "feat/foo"], tmp.path()).unwrap();
        assert!(
            !branches.stdout.trim().is_empty(),
            "branch must survive archive"
        );
    }
}
