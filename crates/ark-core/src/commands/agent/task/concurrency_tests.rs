//! Multi-session command-composition tests.
//!
//! Exercises `task new` / `task resume` / `task archive` / `task discard`
//! through their `_with_ppid` test seams so two simulated shells can drive
//! parallel lifecycles in one test process. Covers the cross-session
//! invariants the helper-level unit tests cannot.

use std::path::Path;

use chrono::Utc;

use crate::{
    commands::agent::{
        state::{Phase, TaskToml, Tier},
        task::{
            archive::{TaskArchiveMoveOptions, task_archive_move_with_ppid},
            discard::{TaskDiscardOptions, task_discard_with_ppid},
            new::{TaskNewOptions, task_new_with_ppid},
            phase::{TaskPhaseOptions, task_execute},
            resume::{TaskResumeOptions, task_resume_with_ppid},
        },
    },
    error::Error,
    layout::Layout,
    session::{
        cache::{cache_file_path, lookup_session_id},
        ppid::{Ppid, StubPpid},
    },
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

/// Returns the focused slug for the session identified by `ppid`, or `None`.
fn focus_for(layout: &Layout, ppid: &dyn Ppid) -> Option<String> {
    let id = lookup_session_id(layout, ppid).ok().flatten()?;
    let state = load_state(layout, ppid).ok()?;
    state.sessions.get(id.as_str()).map(|s| s.focus.clone())
}

/// Hand-flips a task's `task.toml` to `Phase::Committed` so the side-effect-free
/// archive helper has a legal precondition. These tests don't drive
/// `task_commit` because doing so would require a real git repo with staged
/// work; they only need the precondition met.
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

/// Verifies that two sessions create distinct tasks and each sees its own focus.
#[test]
fn two_sessions_create_distinct_tasks_with_isolated_focus() {
    let tmp = tempfile::tempdir().unwrap();
    let session_a = StubPpid(91_001);
    let session_b = StubPpid(91_002);

    task_new_with_ppid(quick(tmp.path(), "alpha"), &session_a).unwrap();
    task_new_with_ppid(quick(tmp.path(), "beta"), &session_b).unwrap();

    let layout = Layout::new(tmp.path());
    assert_eq!(focus_for(&layout, &session_a).as_deref(), Some("alpha"));
    assert_eq!(focus_for(&layout, &session_b).as_deref(), Some("beta"));
}

/// Verifies that `task resume` swaps each session to the other's slug
/// without bleeding focus across sessions.
///
/// Setup: A focuses `alpha`, B focuses `beta`. Then B re-resumes `alpha`
/// and A re-resumes `beta` — the sessions end up with swapped focus. If
/// resume incorrectly broadcast across sessions, both would converge to
/// whichever resume happened last; the swap exposes that.
#[test]
fn resume_in_one_session_does_not_touch_the_other() {
    let tmp = tempfile::tempdir().unwrap();
    let session_a = StubPpid(91_010);
    let session_b = StubPpid(91_011);

    task_new_with_ppid(quick(tmp.path(), "alpha"), &session_a).unwrap();
    task_new_with_ppid(quick(tmp.path(), "beta"), &session_b).unwrap();

    task_resume_with_ppid(
        TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "alpha".into(),
        },
        &session_b,
    )
    .unwrap();
    task_resume_with_ppid(
        TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "beta".into(),
        },
        &session_a,
    )
    .unwrap();

    let layout = Layout::new(tmp.path());
    assert_eq!(
        focus_for(&layout, &session_a).as_deref(),
        Some("beta"),
        "A's later resume must not be overwritten by B's earlier one",
    );
    assert_eq!(
        focus_for(&layout, &session_b).as_deref(),
        Some("alpha"),
        "B's resume must survive A's later resume",
    );
}

/// Verifies that archiving the focused task releases the session cache.
///
/// This is V-001's regression: previously `clear_state_after_archive` ran
/// after the rename, so reconcile pruned the focused session entry before
/// the closure could observe `sess.focus == slug`, leaving the cache file
/// orphaned. The fix probes own-focus before mutating state.
#[test]
fn archive_of_focused_task_releases_session_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let session_a = StubPpid(91_020);

    task_new_with_ppid(quick(tmp.path(), "demo"), &session_a).unwrap();
    task_execute(TaskPhaseOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "demo".into(),
    })
    .unwrap();

    let layout = Layout::new(tmp.path());
    let cache_path = cache_file_path(&layout, session_a.parent_id());
    assert!(cache_path.exists(), "cache should exist after task_new");

    force_committed(tmp.path(), "demo");
    task_archive_move_with_ppid(
        TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        },
        &session_a,
    )
    .unwrap();

    assert!(
        !cache_path.exists(),
        "cache file must be released when the focused task is archived"
    );
    assert!(focus_for(&layout, &session_a).is_none());
}

/// Verifies that one session archiving its task does not disturb the other's focus.
#[test]
fn archive_in_one_session_does_not_clear_the_others_focus() {
    let tmp = tempfile::tempdir().unwrap();
    let session_a = StubPpid(91_030);
    let session_b = StubPpid(91_031);

    task_new_with_ppid(quick(tmp.path(), "alpha"), &session_a).unwrap();
    task_new_with_ppid(quick(tmp.path(), "beta"), &session_b).unwrap();
    task_execute(TaskPhaseOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "alpha".into(),
    })
    .unwrap();

    force_committed(tmp.path(), "alpha");
    task_archive_move_with_ppid(
        TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "alpha".into(),
            archive_month: "2026-05".into(),
        },
        &session_a,
    )
    .unwrap();

    let layout = Layout::new(tmp.path());
    assert!(
        focus_for(&layout, &session_a).is_none(),
        "A's focus released"
    );
    assert_eq!(
        focus_for(&layout, &session_b).as_deref(),
        Some("beta"),
        "B's focus survives A's archive",
    );
}

/// Verifies that `task discard` releases the focused session's cache.
#[test]
fn discard_of_focused_task_releases_session_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let session_a = StubPpid(91_040);

    task_new_with_ppid(quick(tmp.path(), "demo"), &session_a).unwrap();
    let layout = Layout::new(tmp.path());
    let cache_path = cache_file_path(&layout, session_a.parent_id());
    assert!(cache_path.exists());

    task_discard_with_ppid(
        TaskDiscardOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            force: false,
        },
        &session_a,
    )
    .unwrap();

    assert!(!cache_path.exists());
    assert!(focus_for(&layout, &session_a).is_none());
}

/// Verifies that resuming a non-existent slug errors regardless of session.
#[test]
fn resume_unknown_slug_errors_in_caller_session() {
    let tmp = tempfile::tempdir().unwrap();
    let session_a = StubPpid(91_050);
    let session_b = StubPpid(91_051);

    task_new_with_ppid(quick(tmp.path(), "alpha"), &session_a).unwrap();
    let err = task_resume_with_ppid(
        TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "ghost".into(),
        },
        &session_b,
    )
    .unwrap_err();
    assert!(matches!(err, Error::TaskNotFound { .. }));

    // The failed resume left no session entry for B.
    let layout = Layout::new(tmp.path());
    assert!(focus_for(&layout, &session_b).is_none());
}

/// Verifies that a stale session entry (cache file gone) is pruned on next read.
#[test]
fn stale_session_pruned_when_cache_file_disappears() {
    let tmp = tempfile::tempdir().unwrap();
    let session_a = StubPpid(91_060);

    task_new_with_ppid(quick(tmp.path(), "demo"), &session_a).unwrap();
    let layout = Layout::new(tmp.path());

    // Simulate the shell exiting (cache file gone) without releasing focus.
    let cache_path = cache_file_path(&layout, session_a.parent_id());
    std::fs::remove_file(&cache_path).unwrap();

    // Next read prunes the stale session entry.
    let state = load_state(&layout, &session_a).unwrap();
    assert!(
        state.sessions.is_empty(),
        "GC should drop the session whose cache file vanished"
    );
}
