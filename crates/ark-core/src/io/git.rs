//! Spawn `git` as a subprocess. The single sanctioned entry point for
//! process execution under `ark-core` (per the `ark-context` feature C-26).
//!
//! Soft-fail on non-zero exit: the caller decides whether the exit code is
//! a real failure (e.g. `git status` in a non-git directory returns 128) or
//! benign. Spawn failure (binary missing, permissions) is a hard error.

use std::{path::Path, process::Command};

use crate::error::{Error, Result};

/// Captured output of a `git` invocation.
#[derive(Debug, Clone)]
pub struct GitOutput {
    pub exit_code: i32,
    pub stdout: String,
    /// Captured for diagnostic logging; not currently read by callers.
    #[allow(dead_code)]
    pub stderr: String,
}

impl GitOutput {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Run `git <args...>` with `cwd` as the working directory. Returns
/// `Ok(GitOutput)` for any completed run including non-zero exits; spawn
/// failures yield `Error::GitSpawn`.
pub fn run_git(args: &[&str], cwd: &Path) -> Result<GitOutput> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|source| Error::GitSpawn { source })?;
    Ok(GitOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_git_in_non_git_dir_returns_nonzero_exit() {
        let tmp = tempfile::tempdir().unwrap();
        let out = run_git(&["status", "--porcelain"], tmp.path()).unwrap();
        assert!(!out.is_success(), "git status should fail in non-git dir");
        assert_ne!(out.exit_code, 0);
    }

    #[test]
    fn run_git_in_git_repo_with_unknown_arg_returns_nonzero_exit() {
        let tmp = tempfile::tempdir().unwrap();
        let init = run_git(&["init", "--quiet"], tmp.path()).unwrap();
        assert!(init.is_success());

        let out = run_git(&["--not-a-real-flag"], tmp.path()).unwrap();
        assert!(!out.is_success());
    }

    #[test]
    fn run_git_init_then_status_succeeds_in_fresh_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let init = run_git(&["init", "--quiet"], tmp.path()).unwrap();
        assert!(init.is_success(), "git init should succeed");
        let status = run_git(&["status", "--porcelain"], tmp.path()).unwrap();
        assert!(status.is_success());
        assert_eq!(status.stdout.trim(), "");
    }
}
