//! Finds the worktree that owns a given task slug.
//!
//! Uses `git worktree list --porcelain`. Used by
//! `task worktree {cleanup, list}`.
//!
//! Discovery walks every git worktree, reads its `.ark/.state.toml`'s
//! `tasks.active` set (or synthesizes it from legacy `.ark/tasks/.current`
//! during the migration window), and looks for a slug match. Worktrees whose
//! state file is unreadable or whose `task.toml` is missing are silently
//! skipped; that is the documented contract for non-Ark third-party worktrees.

use std::path::{Path, PathBuf};

use crate::{
    commands::agent::state::TaskToml, error::Result, io::git::run_git, layout::Layout,
    session::ppid::RealPpid, state::load_state,
};

/// Information about one git worktree from `git worktree list --porcelain`.
/// One entry from `git worktree list --porcelain`.
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    /// Absolute or git-reported path to the worktree checkout.
    pub path: PathBuf,
    /// Checked-out branch, if the worktree is not detached.
    pub branch: Option<String>,
}

/// Parses `git worktree list --porcelain` output into [`WorktreeEntry`] rows.
///
/// Format per `git-worktree(1)`: blank-line-separated records, each with
/// lines `worktree <path>`, `HEAD <sha>`, optional `branch <ref>`, optional
/// `bare` / `detached` / `locked` markers.
pub fn parse_git_worktree_list(repo_root: &Path) -> Result<Vec<WorktreeEntry>> {
    let out = run_git(&["worktree", "list", "--porcelain"], repo_root)?;
    if !out.is_success() {
        // Non-zero from `worktree list` typically means non-git dir; soft-fail.
        return Ok(Vec::new());
    }
    Ok(parse_porcelain(&out.stdout))
}

fn parse_porcelain(text: &str) -> Vec<WorktreeEntry> {
    let mut out = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    for line in text.lines() {
        if line.is_empty() {
            if let Some(path) = current_path.take() {
                out.push(WorktreeEntry {
                    path,
                    branch: current_branch.take(),
                });
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(rest));
        } else if let Some(rest) = line.strip_prefix("branch ") {
            // `branch refs/heads/feat/foo` → strip `refs/heads/`.
            current_branch = Some(rest.strip_prefix("refs/heads/").unwrap_or(rest).to_string());
        }
        // `HEAD <sha>`, `bare`, `detached`, `locked` are ignored — we only
        // need path + branch for the discovery and list contracts.
    }
    if let Some(path) = current_path {
        out.push(WorktreeEntry {
            path,
            branch: current_branch,
        });
    }
    out
}

/// Searches for a worktree whose `.ark/.state.toml` lists `slug` as active.
///
/// Loads the matching `task.toml` and returns `(worktree_path, task_toml)` for
/// the first slug match, or `None` if no worktree owns it.
///
/// Silently skips worktrees with unreadable `.ark/.state.toml` or `task.toml`
/// (e.g. third-party worktrees living under `worktrees_dir`).
///
/// Path comparison canonicalizes both sides because `git worktree list
/// --porcelain` emits canonical paths (e.g. macOS `/tmp` → `/private/tmp`).
pub fn find_worktree_for_slug(
    repo_root: &Path,
    worktrees_dir: &Path,
    slug: &str,
) -> Result<Option<(PathBuf, TaskToml)>> {
    let canonical_prefix = canonicalize_or_pass(worktrees_dir);
    let ppid = RealPpid::new();
    for entry in parse_git_worktree_list(repo_root)? {
        if !is_under(&entry.path, &canonical_prefix) {
            continue;
        }
        let wt_layout = Layout::new(&entry.path);
        let Ok(state) = load_state(&wt_layout, &ppid) else {
            continue;
        };
        if !state.tasks.active.iter().any(|s| s == slug) {
            continue;
        }
        let task_dir = wt_layout.task_dir(slug);
        let Ok(toml) = TaskToml::load(&task_dir) else {
            continue;
        };
        return Ok(Some((entry.path, toml)));
    }
    Ok(None)
}

/// Returns `true` if `path` starts with `prefix`.
///
/// Falls back to a lexical match if either side fails to canonicalize (e.g.
/// the prefix dir does not exist yet).
pub fn is_under(path: &Path, prefix: &Path) -> bool {
    let path_can = canonicalize_or_pass(path);
    let prefix_can = canonicalize_or_pass(prefix);
    path_can.starts_with(&prefix_can) || path.starts_with(prefix)
}

fn canonicalize_or_pass(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_porcelain_basic_worktrees() {
        let text = "\
worktree /repo
HEAD aaaaaa
branch refs/heads/main

worktree /repo/.ark/worktrees/feat/foo
HEAD bbbbbb
branch refs/heads/feat/foo
";
        let entries = parse_porcelain(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, PathBuf::from("/repo"));
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert_eq!(
            entries[1].path,
            PathBuf::from("/repo/.ark/worktrees/feat/foo")
        );
        assert_eq!(entries[1].branch.as_deref(), Some("feat/foo"));
    }

    #[test]
    fn parse_porcelain_handles_detached_and_no_trailing_blank() {
        let text = "\
worktree /repo
HEAD aaaaaa
detached

worktree /repo/.ark/worktrees/feat/foo
HEAD bbbbbb
branch refs/heads/feat/foo";
        let entries = parse_porcelain(text);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].branch.is_none());
        assert_eq!(entries[1].branch.as_deref(), Some("feat/foo"));
    }

    #[test]
    fn parse_porcelain_branch_without_refs_heads_prefix() {
        // Some git versions or configurations emit raw branch names.
        let text = "worktree /repo\nbranch feat/foo\n";
        let entries = parse_porcelain(text);
        assert_eq!(entries[0].branch.as_deref(), Some("feat/foo"));
    }

    /// Verifies that `find_worktree_for_slug` returns the matching worktree
    /// when its `.ark/.state.toml` lists the slug as active.
    #[test]
    fn find_worktree_for_slug_matches_active_state_entry() {
        use crate::commands::agent::{
            state::Tier,
            task::new::{TaskNewOptions, TaskNewWorktree, task_new},
        };
        let tmp = init_repo_with_commit();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "foo".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap();
        let worktrees_dir = tmp.path().join(".ark/worktrees");
        let found = find_worktree_for_slug(tmp.path(), &worktrees_dir, "foo").unwrap();
        let (wt_path, toml) = found.expect("foo's worktree must be discoverable");
        assert!(wt_path.ends_with(".ark/worktrees/feat/foo"));
        assert_eq!(toml.id, "foo");
    }

    /// Verifies that `find_worktree_for_slug` returns `None` when no worktree
    /// owns the slug.
    #[test]
    fn find_worktree_for_slug_returns_none_when_unknown() {
        let tmp = init_repo_with_commit();
        let worktrees_dir = tmp.path().join(".ark/worktrees");
        let found = find_worktree_for_slug(tmp.path(), &worktrees_dir, "ghost").unwrap();
        assert!(found.is_none());
    }

    /// Initializes a temp git repo with one commit so `git worktree add` works.
    fn init_repo_with_commit() -> tempfile::TempDir {
        use crate::io::{PathExt, git::run_git};
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
}
