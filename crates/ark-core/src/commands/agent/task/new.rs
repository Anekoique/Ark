//! `ark agent task new` — scaffold a new task directory.
//!
//! Creates `.ark/tasks/<slug>/`, seeds `PRD.md` from the embedded template,
//! writes `task.toml` with phase=Design + iteration=0, and points
//! `.ark/tasks/.current` at the new slug. Refuses to overwrite an existing
//! task directory.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use crate::{
    commands::agent::{
        state::{Phase, TaskToml, Tier},
        template::copy_template,
    },
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

#[derive(Debug, Clone)]
pub struct TaskNewOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub title: String,
    pub tier: Tier,
}

#[derive(Debug, Clone)]
pub struct TaskNewSummary {
    pub slug: String,
    pub tier: Tier,
    pub task_dir: PathBuf,
}

impl fmt::Display for TaskNewSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "created {:?} task `{}` at {}",
            self.tier,
            self.slug,
            self.task_dir.display()
        )
    }
}

pub fn task_new(opts: TaskNewOptions) -> Result<TaskNewSummary> {
    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);

    if task_dir.exists() {
        return Err(Error::TaskAlreadyExists { slug: opts.slug });
    }

    task_dir.ensure_dir()?;

    copy_template("PRD", &task_dir.join("PRD.md"))?;

    let now = Utc::now();
    let toml = TaskToml {
        id: opts.slug.clone(),
        title: opts.title,
        tier: opts.tier,
        phase: Phase::Design,
        iteration: 0,
        max_iterations: match opts.tier {
            Tier::Deep => Some(3),
            _ => None,
        },
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    toml.save(&task_dir)?;

    layout
        .tasks_current()
        .write_bytes(format!("{}\n", opts.slug).as_bytes())?;

    Ok(TaskNewSummary {
        slug: opts.slug,
        tier: opts.tier,
        task_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_task_dir_prd_toml_and_current() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "demo task".into(),
            tier: Tier::Standard,
        })
        .unwrap();

        let task_dir = tmp.path().join(".ark/tasks/demo");
        assert!(task_dir.is_dir());
        assert!(task_dir.join("PRD.md").is_file());
        assert!(task_dir.join("task.toml").is_file());
        assert_eq!(
            tmp.path()
                .join(".ark/tasks/.current")
                .read_text()
                .unwrap()
                .trim(),
            "demo"
        );
        assert_eq!(summary.slug, "demo");
        assert_eq!(summary.tier, Tier::Standard);

        let loaded = TaskToml::load(&task_dir).unwrap();
        assert_eq!(loaded.phase, Phase::Design);
        assert_eq!(loaded.iteration, 0);
    }

    #[test]
    fn errors_when_task_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Quick,
        })
        .unwrap();
        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Quick,
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskAlreadyExists { .. }));
    }

    #[test]
    fn deep_tier_seeds_max_iterations() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep1".into(),
            title: "t".into(),
            tier: Tier::Deep,
        })
        .unwrap();
        let toml = TaskToml::load(&tmp.path().join(".ark/tasks/deep1")).unwrap();
        assert_eq!(toml.max_iterations, Some(3));
    }
}
