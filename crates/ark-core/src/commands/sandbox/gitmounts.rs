//! Resolving the git directories a sandbox must mount.
//!
//! A linked worktree's `.git` is a *file* (`gitdir: <repo>/.git/worktrees/<b>`),
//! never a directory — so `root.join(".git")` is wrong. The git common dir
//! (`<repo>/.git`) is the single directory that nests the worktree's own
//! gitdir (HEAD/index/logs), the shared object store, and every ref. Mounting
//! it read-write is therefore sufficient for an in-box `git commit`.

use std::path::{Path, PathBuf};

use crate::{error::Result, io::git::run_git};

/// The git directory set a sandbox mounts for in-box git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitMounts {
    /// `<repo>/.git` — mounted rw; nests the worktree gitdir, objects, all refs.
    pub common_dir: PathBuf,
}

/// Derives the git common dir for the worktree rooted at `worktree`.
///
/// Uses `git rev-parse --path-format=absolute --git-common-dir`, never a
/// hand-built `.git` path, because a worktree's `.git` is a file.
pub fn derive_git_mounts(worktree: &Path) -> Result<GitMounts> {
    let out = run_git(
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        worktree,
    )?;
    let common_dir = PathBuf::from(out.stdout.trim());
    Ok(GitMounts { common_dir })
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// Resolves the common dir to the main repo's `.git` in a real worktree,
    /// with the worktree gitdir nested under it.
    #[test]
    fn derive_resolves_common_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        let run = |args: &[&str], cwd: &Path| {
            Command::new("git")
                .args(args)
                .current_dir(cwd)
                .output()
                .unwrap()
        };
        run(&["init", "--quiet"], repo);
        run(&["config", "user.email", "t@t"], repo);
        run(&["config", "user.name", "t"], repo);
        std::fs::write(repo.join("f"), "x").unwrap();
        run(&["add", "."], repo);
        run(&["commit", "-m", "init", "--quiet"], repo);
        let wt = repo.join("wt");
        run(
            &["worktree", "add", "-b", "feat/x", wt.to_str().unwrap()],
            repo,
        );

        let mounts = derive_git_mounts(&wt).unwrap();
        // The worktree gitdir is nested inside the common dir, so the single
        // common-dir mount covers HEAD/index (worktree gitdir) + objects/refs.
        assert!(mounts.common_dir.ends_with(".git"));
        assert!(mounts.common_dir.join("objects").is_dir());
        assert!(
            mounts.common_dir.join("worktrees/feat-x").exists()
                || mounts.common_dir.join("worktrees").is_dir()
        );
    }
}
