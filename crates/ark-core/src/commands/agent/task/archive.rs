//! `ark agent task archive` (rename) — moves a committed task to the archive.
//!
//! After the workflow refactor, archive is **side-effect-free**: it performs
//! only directory rename + `task.toml` phase update + state-file cleanup.
//! SPEC promotion and journal recording happen earlier, in `task_commit`,
//! and live in the task-closing commit. Bulk archive (`ark archive`,
//! top-level CLI) iterates this helper across every `phase = Committed`
//! task.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use crate::{
    commands::agent::state::{Phase, TaskToml, Tier, check_transition, validate_slug},
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
    session::ppid::{Ppid, RealPpid},
    state::clear_focus_for_slug,
};

/// Options for moving a committed task to the archive directory.
#[derive(Debug, Clone)]
pub struct TaskArchiveMoveOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Slug of the task to archive.
    pub slug: String,
    /// `YYYY-MM` archive bucket. Caller derives this from the task's
    /// `committed_at` timestamp; passing `Utc::now()`-based values would
    /// place historical tasks in the wrong month.
    pub archive_month: String,
}

/// Summary of a single-task archive move.
#[derive(Debug, Clone)]
pub struct TaskArchiveMoveSummary {
    /// Slug of the archived task.
    pub slug: String,
    /// Workflow tier of the archived task.
    pub tier: Tier,
    /// Final on-disk path of the archived task directory.
    pub archive_path: PathBuf,
}

impl fmt::Display for TaskArchiveMoveSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "archived `{}` ({:?}) -> {}",
            self.slug,
            self.tier,
            self.archive_path.display()
        )
    }
}

/// Moves a committed task into `tasks/archive/<archive_month>/<slug>/`.
///
/// Performs no SPEC promotion or workspace journal write — those happen at
/// `task_commit` time and are already part of the task-closing commit.
///
/// # Errors
///
/// Returns [`Error::TaskNotFound`] if the slug has no active task directory,
/// [`Error::IllegalPhaseTransition`] if the task is not in `Committed`, and
/// [`Error::TaskAlreadyExists`] if a same-slug archive entry already exists
/// in the destination month.
pub fn task_archive_move(opts: TaskArchiveMoveOptions) -> Result<TaskArchiveMoveSummary> {
    task_archive_move_with_ppid(opts, &RealPpid::new())
}

/// Test seam for [`task_archive_move`]: same flow with an injectable PPID provider.
pub(crate) fn task_archive_move_with_ppid(
    opts: TaskArchiveMoveOptions,
    ppid: &dyn Ppid,
) -> Result<TaskArchiveMoveSummary> {
    validate_slug(&opts.slug)?;

    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);
    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    let mut toml = TaskToml::load(&task_dir)?;
    check_transition(toml.tier, toml.phase, Phase::Archived)?;
    let tier = toml.tier;

    let archive_parent = layout.tasks_archive_dir().join(&opts.archive_month);
    archive_parent.ensure_dir()?;
    let archive_path = archive_parent.join(&opts.slug);
    if archive_path.exists() {
        return Err(Error::TaskAlreadyExists {
            slug: format!("archive/{}/{}", opts.archive_month, opts.slug),
        });
    }

    // Clear state-file references *before* rename so the focused-session
    // probe still sees the live entry. If a later step fails, two-way
    // reconcile re-adds the slug from the surviving `tasks/<slug>/` dir
    // (the rename has not happened yet).
    clear_focus_for_slug(&layout, ppid, &opts.slug)?;

    task_dir.rename_to(&archive_path)?;

    let now = Utc::now();
    toml.phase = Phase::Archived;
    toml.archived_at = Some(now);
    toml.updated_at = now;
    toml.save(&archive_path)?;

    Ok(TaskArchiveMoveSummary {
        slug: opts.slug,
        tier,
        archive_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::task::{
        new::{TaskNewOptions, task_new},
        phase::{TaskPhaseOptions, task_execute, task_plan, task_verify},
    };

    /// Drives a fresh standard-tier task all the way to `Committed` so archive
    /// has something legal to move. The git plumbing isn't real here — we
    /// short-circuit by hand-saving `phase = Committed` for tests that only
    /// need the precondition met.
    fn standard_at_committed(tmp_path: &std::path::Path) {
        task_new(TaskNewOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_verify(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        // Force phase=Committed without going through task_commit (which would
        // require a real git repo + staged work). committed_at is set so
        // ark_archive can derive the month later if a test wants.
        let layout = Layout::new(tmp_path);
        let task_dir = layout.task_dir("demo");
        let mut toml = TaskToml::load(&task_dir).unwrap();
        toml.phase = Phase::Committed;
        let now = Utc::now();
        toml.committed_at = Some(now);
        toml.updated_at = now;
        toml.save(&task_dir).unwrap();
    }

    fn quick_at_committed(tmp_path: &std::path::Path) {
        task_new(TaskNewOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "qd".into(),
            title: "qd".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "qd".into(),
        })
        .unwrap();
        let layout = Layout::new(tmp_path);
        let task_dir = layout.task_dir("qd");
        let mut toml = TaskToml::load(&task_dir).unwrap();
        toml.phase = Phase::Committed;
        let now = Utc::now();
        toml.committed_at = Some(now);
        toml.updated_at = now;
        toml.save(&task_dir).unwrap();
    }

    #[test]
    fn standard_archive_moves_dir_and_clears_current() {
        let tmp = tempfile::tempdir().unwrap();
        standard_at_committed(tmp.path());

        let s = task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap();
        assert_eq!(s.tier, Tier::Standard);
        assert!(!tmp.path().join(".ark/tasks/demo").exists());
        assert!(s.archive_path.exists());
        assert!(s.archive_path.join("task.toml").exists());
        assert!(
            s.archive_path
                .to_string_lossy()
                .contains(".ark/tasks/archive/2026-05/demo"),
            "archive_month must select the directory: got {}",
            s.archive_path.display()
        );
        let state = crate::state::load_state(&Layout::new(tmp.path()), &RealPpid::new()).unwrap();
        assert!(!state.tasks.active.iter().any(|s| s == "demo"));
    }

    #[test]
    fn archive_twice_errors() {
        let tmp = tempfile::tempdir().unwrap();
        standard_at_committed(tmp.path());
        task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap();
        let err = task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskNotFound { .. }));
    }

    /// Verifies that `Verify → Archived` is no longer a legal transition;
    /// archive must be preceded by `task_commit`.
    #[test]
    fn archive_from_verify_errors_after_refactor() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_verify(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        let err = task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::IllegalPhaseTransition { .. }));
    }

    /// Verifies that archive does not write anything to the workspace journal.
    ///
    /// The journal entry is the responsibility of `task_commit`; bulk archive
    /// (and this single-slug helper) must not touch it.
    #[test]
    fn archive_writes_no_journal_entry() {
        use crate::commands::agent::workspace::init::{WorkspaceInitOptions, workspace_init};
        let tmp = tempfile::tempdir().unwrap();
        workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        let layout = Layout::new(tmp.path());
        let journal_path = layout.workspace_journal("alice", 1);
        let pre_bytes = journal_path.read_bytes().unwrap();
        standard_at_committed(tmp.path());
        task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap();
        let post_bytes = journal_path.read_bytes().unwrap();
        assert_eq!(pre_bytes, post_bytes, "archive must not touch the journal");
    }

    /// Verifies that archive does not promote a SPEC.
    ///
    /// Deep-tier SPEC promotion happens at `task_commit` time. Any later
    /// archive run must leave `specs/features/INDEX.md` and existing SPEC
    /// files untouched.
    #[test]
    fn archive_writes_no_spec_files() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.specs_features_dir().ensure_dir().unwrap();
        layout
            .specs_features_index()
            .write_bytes(b"# Features\n")
            .unwrap();
        let pre_index = layout.specs_features_index().read_bytes().unwrap();

        // Use a quick-tier task — no deep SPEC to begin with.
        quick_at_committed(tmp.path());
        task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap();

        let post_index = layout.specs_features_index().read_bytes().unwrap();
        assert_eq!(
            pre_index, post_index,
            "archive must not modify the features INDEX"
        );
        assert!(
            !layout.specs_feature_dir("qd").exists(),
            "archive must not create a SPEC dir for the slug"
        );
    }
}
