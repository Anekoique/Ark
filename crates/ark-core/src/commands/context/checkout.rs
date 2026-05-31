//! Detects the per-checkout location info populating [`CheckoutInfo`].
//!
//! Classifies `root_kind` by comparing `git rev-parse --show-toplevel`
//! with the parent of `git rev-parse --git-common-dir`. Failures (non-git
//! checkout, spawn error) default to [`CheckoutRootKind::Main`].

use std::path::Path;

use crate::{
    commands::context::model::{CheckoutInfo, CheckoutRootKind},
    io::git,
    layout::Layout,
    state::load_state,
};

/// Builds the [`CheckoutInfo`] for `layout`.
///
/// `branch` mirrors `GitState::branch` semantics (caller threads it in to
/// avoid a duplicate `git rev-parse --abbrev-ref` invocation).
/// `focus_slug` reads `.state.toml` `[focus]` via [`load_state`]; missing
/// file or unbound focus yields `None`.
pub fn detect_checkout(layout: &Layout, branch: &str) -> CheckoutInfo {
    let root_kind = classify_root(layout.root());
    let focus_slug = load_state(layout).ok().and_then(|s| s.focus);
    CheckoutInfo {
        root_kind,
        branch: branch.to_string(),
        focus_slug,
    }
}

/// Classifies `root` as [`CheckoutRootKind::Worktree`] when the
/// `--show-toplevel` differs from `--git-common-dir`'s parent; otherwise
/// [`CheckoutRootKind::Main`].
///
/// Any git-side failure (non-git directory, spawn error, non-zero exit)
/// falls back to [`CheckoutRootKind::Main`].
fn classify_root(root: &Path) -> CheckoutRootKind {
    let Ok(toplevel) = git::run_git(&["rev-parse", "--show-toplevel"], root) else {
        return CheckoutRootKind::Main;
    };
    if !toplevel.is_success() {
        return CheckoutRootKind::Main;
    }
    let Ok(common_dir) = git::run_git(&["rev-parse", "--git-common-dir"], root) else {
        return CheckoutRootKind::Main;
    };
    if !common_dir.is_success() {
        return CheckoutRootKind::Main;
    }
    let toplevel = toplevel.stdout.trim();
    let common_dir = common_dir.stdout.trim();
    let common_path = Path::new(common_dir);
    // `git-common-dir` returns the `.git` directory of the main checkout
    // (absolute when invoked from a worktree, relative `.git` when
    // invoked from the main checkout). Resolve to its parent so we can
    // compare against `--show-toplevel`.
    let common_parent = match common_path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        // `.git` (parent = "") means we're in the main checkout already.
        _ => return CheckoutRootKind::Main,
    };
    // Canonicalize both sides so symlinks and `..` don't mask equality.
    let canon = |p: &Path| std::fs::canonicalize(p).ok();
    match (canon(Path::new(toplevel)), canon(&common_parent)) {
        (Some(top), Some(par)) if top == par => CheckoutRootKind::Main,
        (Some(_), Some(_)) => CheckoutRootKind::Worktree,
        // If we can't canonicalize one side, fall back to a string compare.
        _ => {
            if toplevel == common_parent.to_string_lossy() {
                CheckoutRootKind::Main
            } else {
                CheckoutRootKind::Worktree
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;
    use crate::io::PathExt;

    /// Initializes a tempdir as a git repo and returns its path-bearing
    /// guard plus a [`Layout`] rooted at it.
    fn git_inited_tempdir() -> (tempfile::TempDir, Layout) {
        let tmp = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init", "--quiet", "-b", "main"])
            .current_dir(tmp.path())
            .status()
            .expect("git init");
        // Need at least one commit so `git worktree add` succeeds.
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(tmp.path())
            .status()
            .expect("git config email");
        Command::new("git")
            .args(["config", "user.name", "test"])
            .current_dir(tmp.path())
            .status()
            .expect("git config name");
        tmp.path().join(".ark").ensure_dir().unwrap();
        std::fs::write(tmp.path().join("README.md"), "test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .status()
            .expect("git add");
        Command::new("git")
            .args(["commit", "--quiet", "-m", "init"])
            .current_dir(tmp.path())
            .status()
            .expect("git commit");
        let layout = Layout::new(tmp.path());
        (tmp, layout)
    }

    /// `detect_checkout` returns `Main` when `Layout::root` is
    /// the main git checkout.
    #[test]
    fn returns_main_when_layout_is_main_checkout() {
        let (_tmp, layout) = git_inited_tempdir();
        let info = detect_checkout(&layout, "main");
        assert_eq!(info.root_kind, CheckoutRootKind::Main);
        assert_eq!(info.branch, "main");
    }

    /// `detect_checkout` returns `Worktree` when `Layout::root`
    /// is a worktree checkout created via `git worktree add`.
    #[test]
    fn returns_worktree_when_layout_is_worktree() {
        let (tmp, _main_layout) = git_inited_tempdir();
        let wt_path = tmp.path().join("wt");
        let status = Command::new("git")
            .args([
                "worktree",
                "add",
                "--quiet",
                wt_path.to_str().unwrap(),
                "-b",
                "feat/test",
            ])
            .current_dir(tmp.path())
            .status()
            .expect("git worktree add");
        assert!(status.success());
        wt_path.join(".ark").ensure_dir().unwrap();
        let wt_layout = Layout::new(&wt_path);
        let info = detect_checkout(&wt_layout, "feat/test");
        assert_eq!(info.root_kind, CheckoutRootKind::Worktree);
        assert_eq!(info.branch, "feat/test");
    }

    /// `detect_checkout` defaults to `Main` when no git
    /// repo is present (and never panics).
    #[test]
    fn defaults_to_main_in_non_git_tempdir() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join(".ark").ensure_dir().unwrap();
        let layout = Layout::new(tmp.path());
        let info = detect_checkout(&layout, "unknown");
        assert_eq!(info.root_kind, CheckoutRootKind::Main);
    }

    /// `detect_checkout` populates `focus_slug` from
    /// `.state.toml` and returns `None` when no focus is bound.
    #[test]
    fn focus_slug_reads_state_toml_when_bound() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join(".ark").ensure_dir().unwrap();
        let layout = Layout::new(tmp.path());

        // No state file yet.
        let info = detect_checkout(&layout, "main");
        assert!(info.focus_slug.is_none());

        // Plant a real task dir on disk — reconcile (inside load_state)
        // drops focus when the slug has no backing dir with a valid
        // task.toml.
        let task_dir = layout.task_dir("demo-slug");
        task_dir.ensure_dir().unwrap();
        task_dir
            .join("task.toml")
            .write_bytes(
                b"id = \"demo-slug\"\n\
                  title = \"demo\"\n\
                  tier = \"quick\"\n\
                  phase = \"execute\"\n\
                  created_at = \"2026-05-24T00:00:00Z\"\n\
                  updated_at = \"2026-05-24T00:00:00Z\"\n",
            )
            .unwrap();
        // Now bind focus.
        use crate::state::state_mutate;
        state_mutate(&layout, |state| {
            state.focus = Some("demo-slug".to_string());
            Ok(())
        })
        .unwrap();
        let info = detect_checkout(&layout, "main");
        assert_eq!(info.focus_slug.as_deref(), Some("demo-slug"));
    }
}
