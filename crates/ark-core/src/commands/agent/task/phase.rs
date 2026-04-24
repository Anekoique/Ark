//! `ark agent task {plan,review,execute,verify}` — explicit phase transitions.
//!
//! Each loads the task.toml, checks legality under tier, mutates phase, and
//! writes back. Plan/Review/Verify also seed their artifact from the
//! embedded template when missing.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use crate::{
    commands::agent::{
        state::{Phase, TaskToml, check_transition, validate_slug},
        template::copy_template,
    },
    error::{Error, Result},
    layout::Layout,
};

#[derive(Debug, Clone)]
pub struct TaskPhaseOptions {
    pub project_root: PathBuf,
    pub slug: String,
}

#[derive(Debug, Clone)]
pub struct TaskPhaseSummary {
    pub slug: String,
    pub from: Phase,
    pub to: Phase,
}

impl fmt::Display for TaskPhaseSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task `{}`: {:?} -> {:?}", self.slug, self.from, self.to)
    }
}

pub fn task_plan(opts: TaskPhaseOptions) -> Result<TaskPhaseSummary> {
    transition(opts, Phase::Plan)
}

pub fn task_review(opts: TaskPhaseOptions) -> Result<TaskPhaseSummary> {
    transition(opts, Phase::Review)
}

pub fn task_execute(opts: TaskPhaseOptions) -> Result<TaskPhaseSummary> {
    transition(opts, Phase::Execute)
}

pub fn task_verify(opts: TaskPhaseOptions) -> Result<TaskPhaseSummary> {
    transition(opts, Phase::Verify)
}

fn transition(opts: TaskPhaseOptions, to: Phase) -> Result<TaskPhaseSummary> {
    validate_slug(&opts.slug)?;

    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);

    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    let mut toml = TaskToml::load(&task_dir)?;
    let from = toml.phase;
    check_transition(toml.tier, from, to)?;

    // Seed the artifact before persisting the phase: if the template write
    // fails, the toml on disk still reflects the old phase and the caller can
    // retry the same transition. Saving first would advance the phase and
    // leave a missing artifact that no legal transition can re-seed.
    if let Some((template, filename)) = artifact_for(to, toml.iteration) {
        let path = task_dir.join(filename);
        if !path.exists() {
            copy_template(template, &path)?;
        }
    }

    toml.phase = to;
    toml.updated_at = Utc::now();
    toml.save(&task_dir)?;

    Ok(TaskPhaseSummary {
        slug: opts.slug,
        from,
        to,
    })
}

/// Which embedded template should be seeded when entering `phase`, and at what
/// filename under the task directory.
fn artifact_for(phase: Phase, iteration: u32) -> Option<(&'static str, String)> {
    match phase {
        Phase::Plan => Some(("PLAN", format!("{iteration:02}_PLAN.md"))),
        Phase::Review => Some(("REVIEW", format!("{iteration:02}_REVIEW.md"))),
        Phase::Verify => Some(("VERIFY", "VERIFY.md".into())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::agent::{
            state::Tier,
            task::new::{TaskNewOptions, task_new},
        },
        io::PathExt,
    };

    fn fresh(tmp_path: &std::path::Path, tier: Tier) -> String {
        task_new(TaskNewOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier,
        })
        .unwrap();
        "demo".to_string()
    }

    #[test]
    fn standard_design_to_plan_to_execute_to_verify() {
        let tmp = tempfile::tempdir().unwrap();
        let slug = fresh(tmp.path(), Tier::Standard);
        let o = |s: &str| TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: s.into(),
        };

        let s = task_plan(o(&slug)).unwrap();
        assert_eq!((s.from, s.to), (Phase::Design, Phase::Plan));
        assert!(tmp.path().join(".ark/tasks/demo/00_PLAN.md").exists());

        let s = task_execute(o(&slug)).unwrap();
        assert_eq!((s.from, s.to), (Phase::Plan, Phase::Execute));

        let s = task_verify(o(&slug)).unwrap();
        assert_eq!((s.from, s.to), (Phase::Execute, Phase::Verify));
        assert!(tmp.path().join(".ark/tasks/demo/VERIFY.md").exists());
    }

    #[test]
    fn illegal_transition_errors_and_does_not_mutate() {
        let tmp = tempfile::tempdir().unwrap();
        let slug = fresh(tmp.path(), Tier::Quick);
        let before = tmp
            .path()
            .join(".ark/tasks/demo/task.toml")
            .read_bytes()
            .unwrap();

        let err = task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: slug.clone(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::IllegalPhaseTransition { .. }));

        let after = tmp
            .path()
            .join(".ark/tasks/demo/task.toml")
            .read_bytes()
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn rejects_path_traversal_slug() {
        let tmp = tempfile::tempdir().unwrap();
        let err = task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "../escape".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidTaskField { .. }));
    }

    #[test]
    fn task_not_found_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "ghost".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskNotFound { .. }));
    }

    #[test]
    fn deep_design_to_plan_to_review() {
        let tmp = tempfile::tempdir().unwrap();
        let slug = fresh(tmp.path(), Tier::Deep);
        let o = |s: &str| TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: s.into(),
        };
        task_plan(o(&slug)).unwrap();
        let s = task_review(o(&slug)).unwrap();
        assert_eq!((s.from, s.to), (Phase::Plan, Phase::Review));
        assert!(tmp.path().join(".ark/tasks/demo/00_REVIEW.md").exists());
    }
}
