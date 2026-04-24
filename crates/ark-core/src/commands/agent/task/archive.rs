//! `ark agent task archive` — move an active task to archive; on deep tier,
//! extract and register the feature SPEC.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use crate::{
    commands::agent::{
        spec::{
            extract::{SpecExtractOptions, spec_extract},
            register::{SpecRegisterOptions, spec_register},
        },
        state::{Phase, TaskToml, Tier, check_transition, validate_slug},
    },
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

#[derive(Debug, Clone)]
pub struct TaskArchiveOptions {
    pub project_root: PathBuf,
    pub slug: String,
}

#[derive(Debug, Clone)]
pub struct TaskArchiveSummary {
    pub slug: String,
    pub tier: Tier,
    pub deep_spec_promoted: bool,
    pub archive_path: PathBuf,
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
        Ok(())
    }
}

pub fn task_archive(opts: TaskArchiveOptions) -> Result<TaskArchiveSummary> {
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

    // Rename first. Everything after this point operates on `archive_path`; if
    // a later step fails, the task is at the archive path (still recoverable)
    // rather than wedged between states with partial side effects.
    task_dir.rename_to(&archive_path)?;

    toml.phase = Phase::Archived;
    toml.archived_at = Some(now);
    toml.updated_at = now;
    toml.save(&archive_path)?;

    // Deep-tier SPEC promotion runs from the archive path. If extract/register
    // fail, the task is archived but the promotion didn't happen — the SPEC
    // file and INDEX row don't reference an unarchived task, which is the
    // invariant we care about. The user can hand-run `ark agent spec extract`
    // / `register` to complete promotion.
    let mut deep_spec_promoted = false;
    if tier == Tier::Deep {
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
        deep_spec_promoted = true;
    }

    let current_path = layout.tasks_current();
    if let Some(current_text) = current_path.read_text_optional()?
        && current_text.trim() == opts.slug
    {
        current_path.remove_if_exists()?;
    }

    Ok(TaskArchiveSummary {
        slug: opts.slug,
        tier,
        deep_spec_promoted,
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

    fn standard_at_verify(tmp_path: &std::path::Path) {
        task_new(TaskNewOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
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
        assert!(!tmp.path().join(".ark/tasks/.current").exists());
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
        })
        .unwrap();
        let err = task_archive(TaskArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::IllegalPhaseTransition { .. }));
    }
}
