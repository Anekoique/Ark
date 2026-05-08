//! Focus-lifecycle composition tests.
//!
//! Exercises `task new` / `task resume` / `task archive` / `task discard`
//! together to assert the per-checkout `[focus]` invariants. The
//! multi-session machinery these tests previously covered (per-shell focus
//! map, `$TMPDIR` cache files, PID liveness) was removed by the
//! `session-focus-bind` task; one focus per checkout is the new contract.

use std::path::Path;

use chrono::Utc;

use crate::{
    commands::agent::{
        state::{Phase, TaskToml, Tier},
        task::{
            archive::{TaskArchiveMoveOptions, task_archive_move},
            discard::{TaskDiscardOptions, task_discard},
            new::{TaskNewOptions, task_new},
            phase::{TaskPhaseOptions, task_execute},
            resume::{TaskResumeOptions, task_resume},
        },
    },
    error::Error,
    layout::Layout,
    state::load_state,
};

/// Builds a quick-tier `TaskNewOptions` for `slug` rooted at `tmp`.
fn quick(tmp: &Path, slug: &str) -> TaskNewOptions {
    TaskNewOptions {
        project_root: tmp.to_path_buf(),
        slug: slug.into(),
        title: slug.into(),
        tier: Tier::Quick,
        worktree: None,
    }
}

/// Returns the per-checkout focus or `None`.
fn focus(layout: &Layout) -> Option<String> {
    load_state(layout).ok().and_then(|s| s.focus)
}

/// Hand-flips a task's `task.toml` to `Phase::Committed` so the
/// side-effect-free archive helper has a legal precondition.
fn force_committed(tmp: &Path, slug: &str) {
    let layout = Layout::new(tmp);
    let task_dir = layout.task_dir(slug);
    let mut toml = TaskToml::load(&task_dir).unwrap();
    toml.phase = Phase::Committed;
    let now = Utc::now();
    toml.committed_at = Some(now);
    toml.updated_at = now;
    toml.save(&task_dir).unwrap();
}

/// Verifies that `task new` binds the focus to the new slug.
#[test]
fn task_new_binds_focus() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "alpha")).unwrap();
    let layout = Layout::new(tmp.path());
    assert_eq!(focus(&layout).as_deref(), Some("alpha"));
}

/// Verifies that creating a second task overwrites focus to the newer slug.
#[test]
fn task_new_second_task_overwrites_focus() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "alpha")).unwrap();
    task_new(quick(tmp.path(), "beta")).unwrap();
    let layout = Layout::new(tmp.path());
    assert_eq!(focus(&layout).as_deref(), Some("beta"));
}

/// Verifies that `task new` reports `overwrote_focus` when rebinding.
#[test]
fn task_new_reports_overwrote_focus_on_rebind() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "alpha")).unwrap();
    let s = task_new(quick(tmp.path(), "beta")).unwrap();
    assert_eq!(s.overwrote_focus.as_deref(), Some("alpha"));
    let rendered = s.to_string();
    assert!(
        rendered.contains("focus rebound from `alpha` to `beta`"),
        "expected rebind warning: {rendered}"
    );
}

/// Verifies that the first `task new` on a fresh checkout does not warn.
#[test]
fn task_new_first_task_does_not_warn() {
    let tmp = tempfile::tempdir().unwrap();
    let s = task_new(quick(tmp.path(), "alpha")).unwrap();
    assert!(s.overwrote_focus.is_none());
    assert!(!s.to_string().contains("rebound"));
}

/// Verifies that `task resume` switches the focus to the resumed slug.
#[test]
fn task_resume_switches_focus() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "alpha")).unwrap();
    task_new(quick(tmp.path(), "beta")).unwrap();
    task_resume(TaskResumeOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "alpha".into(),
    })
    .unwrap();
    let layout = Layout::new(tmp.path());
    assert_eq!(focus(&layout).as_deref(), Some("alpha"));
}

/// Verifies that archiving the focused task clears focus to `None`.
#[test]
fn task_archive_of_focused_clears_focus() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "demo")).unwrap();
    task_execute(TaskPhaseOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "demo".into(),
    })
    .unwrap();
    force_committed(tmp.path(), "demo");
    task_archive_move(TaskArchiveMoveOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "demo".into(),
        archive_month: "2026-05".into(),
    })
    .unwrap();
    let layout = Layout::new(tmp.path());
    assert!(focus(&layout).is_none());
}

/// Verifies that archiving a non-focused slug leaves focus on the bound slug.
#[test]
fn task_archive_of_non_focused_preserves_focus() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "alpha")).unwrap();
    task_new(quick(tmp.path(), "beta")).unwrap();
    // Focus is on `beta` (latest new). Archive `alpha`.
    task_execute(TaskPhaseOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "alpha".into(),
    })
    .unwrap();
    force_committed(tmp.path(), "alpha");
    task_archive_move(TaskArchiveMoveOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "alpha".into(),
        archive_month: "2026-05".into(),
    })
    .unwrap();
    let layout = Layout::new(tmp.path());
    assert_eq!(
        focus(&layout).as_deref(),
        Some("beta"),
        "non-focused archive must not touch focus"
    );
}

/// Verifies that `task discard` of the focused task clears focus.
#[test]
fn task_discard_of_focused_clears_focus() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "demo")).unwrap();
    task_discard(TaskDiscardOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "demo".into(),
        force: false,
    })
    .unwrap();
    let layout = Layout::new(tmp.path());
    assert!(focus(&layout).is_none());
}

/// Verifies that `task discard` of a non-focused slug preserves focus.
#[test]
fn task_discard_of_non_focused_preserves_focus() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "alpha")).unwrap();
    task_new(quick(tmp.path(), "beta")).unwrap();
    // Focus is on `beta`. Discard `alpha`.
    task_discard(TaskDiscardOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "alpha".into(),
        force: false,
    })
    .unwrap();
    let layout = Layout::new(tmp.path());
    assert_eq!(focus(&layout).as_deref(), Some("beta"));
}

/// Verifies that resuming an unknown slug errors and leaves focus untouched.
#[test]
fn task_resume_unknown_slug_errors_and_preserves_focus() {
    let tmp = tempfile::tempdir().unwrap();
    task_new(quick(tmp.path(), "alpha")).unwrap();
    let err = task_resume(TaskResumeOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "ghost".into(),
    })
    .unwrap_err();
    assert!(matches!(err, Error::TaskNotFound { .. }));
    let layout = Layout::new(tmp.path());
    assert_eq!(
        focus(&layout).as_deref(),
        Some("alpha"),
        "failed resume must not clear focus"
    );
}
