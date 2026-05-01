//! `ark agent task archive` moves an active task to archive.
//!
//! On deep tier, it also extracts and registers the feature SPEC.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use crate::{
    commands::agent::{
        spec::{
            extract::{SpecExtractOptions, spec_extract},
            register::{SpecRegisterOptions, spec_register},
        },
        state::{Phase, TaskToml, Tier, check_transition, validate_slug},
        workspace::{RecordTaskOptions, WorkspaceRecorded, record_task},
    },
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
    session::ppid::{Ppid, RealPpid},
    state::clear_focus_for_slug,
};

/// Options for archiving an active task.
#[derive(Debug, Clone)]
pub struct TaskArchiveOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Task slug to archive.
    pub slug: String,
}

/// Summary of a task archive operation.
#[derive(Debug, Clone)]
pub struct TaskArchiveSummary {
    /// Archived task slug.
    pub slug: String,
    /// Workflow tier of the archived task.
    pub tier: Tier,
    /// Reports whether a deep-tier SPEC was promoted.
    pub deep_spec_promoted: bool,
    /// Destination path under the archive directory.
    pub archive_path: PathBuf,
    /// Records the auto-record outcome for the developer's workspace journal.
    ///
    /// `Recorded` on success, `SkippedNoIdentity` when `.ark/.developer`
    /// is absent, `SkippedDisabled` when
    /// `[workspace].auto_record_on_archive = false` in `.ark/config.toml`.
    pub workspace_recorded: WorkspaceRecorded,
}

impl fmt::Display for TaskArchiveSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "archived `{}` ({:?}) -> {}",
            self.slug,
            self.tier,
            self.archive_path.display()
        )?;
        if self.deep_spec_promoted {
            write!(f, " [SPEC promoted]")?;
        }
        match &self.workspace_recorded {
            WorkspaceRecorded::Recorded { session_number, .. } => {
                write!(f, " [workspace session {session_number}]")?;
            }
            WorkspaceRecorded::SkippedNoIdentity | WorkspaceRecorded::SkippedDisabled => {}
        }
        Ok(())
    }
}

/// Archives a task and promotes its SPEC when the task is deep-tier.
pub fn task_archive(opts: TaskArchiveOptions) -> Result<TaskArchiveSummary> {
    task_archive_with_ppid(opts, &RealPpid::new())
}

/// Test seam for [`task_archive`]: same flow with an injectable parent-id provider.
pub(crate) fn task_archive_with_ppid(
    opts: TaskArchiveOptions,
    ppid: &dyn Ppid,
) -> Result<TaskArchiveSummary> {
    validate_slug(&opts.slug)?;

    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);
    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    let mut toml = TaskToml::load(&task_dir)?;
    check_transition(toml.tier, toml.phase, Phase::Archived)?;
    let tier = toml.tier;

    // Reserve the archive path before any mutation. If the destination already
    // exists (same-slug re-archive in the same month), fail cleanly with the
    // task dir untouched and no partial side effects.
    let now = Utc::now();
    let yyyy_mm = now.format("%Y-%m").to_string();
    let archive_parent = layout.tasks_archive_dir().join(&yyyy_mm);
    archive_parent.ensure_dir()?;
    let archive_path = archive_parent.join(&opts.slug);
    if archive_path.exists() {
        return Err(Error::TaskAlreadyExists {
            slug: format!("archive/{yyyy_mm}/{}", opts.slug),
        });
    }

    // Clear state-file references *before* rename so the focused-session
    // probe still sees the live entry. Ordering also gives clean recovery:
    // if a later step fails, two-way reconcile re-adds the slug from the
    // surviving `tasks/<slug>/` dir (`phase != Archived` until step below).
    clear_focus_for_slug(&layout, ppid, &opts.slug)?;

    task_dir.rename_to(&archive_path)?;

    toml.phase = Phase::Archived;
    toml.archived_at = Some(now);
    toml.updated_at = now;
    toml.save(&archive_path)?;

    // Deep-tier SPEC promotion runs from the archive path. If extract/register
    // fail, the task is archived but the promotion did not happen — the SPEC
    // file and INDEX row do not reference an unarchived task. The user can
    // hand-run `ark agent spec extract` / `register` to complete promotion.
    let deep_spec_promoted = tier == Tier::Deep;
    if deep_spec_promoted {
        spec_extract(SpecExtractOptions {
            project_root: opts.project_root.clone(),
            slug: opts.slug.clone(),
            plan_override: None,
            task_dir_override: Some(archive_path.clone()),
        })?;
        spec_register(SpecRegisterOptions {
            project_root: opts.project_root.clone(),
            feature: opts.slug.clone(),
            scope: toml.title.clone(),
            from_task: opts.slug.clone(),
            date: now.date_naive(),
        })?;
    }

    // `record_task` swallows `DeveloperNotInitialized` and respects
    // `auto_record_on_archive = false`. Other errors propagate; archive is
    // not rolled back on workspace failure.
    let workspace_recorded = record_task(RecordTaskOptions {
        project_root: opts.project_root.clone(),
        slug: opts.slug.clone(),
        title: toml.title.clone(),
        tier,
        branch: toml.branch.clone(),
        base_branch: toml.base_branch.clone(),
        worktree_path: toml.worktree_path.as_ref().map(|p| layout.root().join(p)),
        archive_path: archive_path.clone(),
        archived_at: now,
    })?;

    Ok(TaskArchiveSummary {
        slug: opts.slug,
        tier,
        deep_spec_promoted,
        archive_path,
        workspace_recorded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::task::{
        new::{TaskNewOptions, task_new},
        phase::{TaskPhaseOptions, task_execute, task_plan, task_verify},
    };

    fn standard_at_verify(tmp_path: &std::path::Path) {
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
    }

    #[test]
    fn standard_archive_moves_dir_and_clears_current() {
        let tmp = tempfile::tempdir().unwrap();
        standard_at_verify(tmp.path());

        let s = task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        assert_eq!(s.tier, Tier::Standard);
        assert!(!s.deep_spec_promoted);
        assert!(!tmp.path().join(".ark/tasks/demo").exists());
        assert!(s.archive_path.exists());
        assert!(s.archive_path.join("task.toml").exists());
        // Active set in `.state.toml` no longer contains the archived slug.
        let state = crate::state::load_state(&Layout::new(tmp.path()), &RealPpid::new()).unwrap();
        assert!(!state.tasks.active.iter().any(|s| s == "demo"));
    }

    #[test]
    fn archive_twice_errors() {
        let tmp = tempfile::tempdir().unwrap();
        standard_at_verify(tmp.path());
        task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        let err = task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskNotFound { .. }));
    }

    #[test]
    fn archive_illegal_from_design_errors() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        let err = task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::IllegalPhaseTransition { .. }));
    }

    /// Verifies that absent identity causes archive to succeed with `SkippedNoIdentity`.
    #[test]
    fn archive_skips_workspace_record_when_no_identity() {
        use crate::commands::agent::workspace::record::WorkspaceRecorded;
        let tmp = tempfile::tempdir().unwrap();
        standard_at_verify(tmp.path());
        let s = task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        assert!(matches!(
            s.workspace_recorded,
            WorkspaceRecorded::SkippedNoIdentity
        ));
        assert!(!tmp.path().join(".ark/workspace").exists());
    }

    /// Standard-tier archive auto-records into the developer's journal.
    #[test]
    fn standard_tier_archive_records_session() {
        use crate::commands::agent::workspace::{
            init::{WorkspaceInitOptions, workspace_init},
            record::WorkspaceRecorded,
        };
        let tmp = tempfile::tempdir().unwrap();
        workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        standard_at_verify(tmp.path());
        let s = task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        let WorkspaceRecorded::Recorded { session_number, .. } = s.workspace_recorded else {
            panic!("expected Recorded, got {:?}", s.workspace_recorded);
        };
        assert_eq!(session_number, 1);

        let layout = Layout::new(tmp.path());
        let journal = layout.workspace_journal("alice", 1).read_text().unwrap();
        assert!(journal.contains("**Slug**: demo"));
        assert!(journal.contains("**Kind**: task"));
    }

    /// Quick-tier archive auto-records into the developer's journal.
    #[test]
    fn quick_tier_archive_records_session() {
        use crate::commands::agent::{
            task::{
                new::{TaskNewOptions, task_new},
                phase::{TaskPhaseOptions, task_execute},
            },
            workspace::{
                init::{WorkspaceInitOptions, workspace_init},
                record::WorkspaceRecorded,
            },
        };

        let tmp = tempfile::tempdir().unwrap();
        workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            title: "qd".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
        })
        .unwrap();
        let s = task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
        })
        .unwrap();
        assert_eq!(s.tier, Tier::Quick);
        assert!(matches!(
            s.workspace_recorded,
            WorkspaceRecorded::Recorded { .. }
        ));
    }

    /// Verifies that `auto_record_on_archive = false` causes archive to return `SkippedDisabled`.
    #[test]
    fn archive_skips_when_auto_record_disabled() {
        use crate::commands::agent::workspace::{
            init::{WorkspaceInitOptions, workspace_init},
            record::WorkspaceRecorded,
        };
        let tmp = tempfile::tempdir().unwrap();
        workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        let layout = Layout::new(tmp.path());
        layout
            .config_file()
            .write_bytes(b"[workspace]\nauto_record_on_archive = false\n")
            .unwrap();
        standard_at_verify(tmp.path());
        let s = task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        assert!(matches!(
            s.workspace_recorded,
            WorkspaceRecorded::SkippedDisabled
        ));
    }
}
