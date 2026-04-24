//! `ark agent task promote` — change a task's tier mid-flight.
//!
//! Swaps `task.toml.tier`. Refuses if the current phase would be illegal under
//! the target tier (e.g. promoting deep→quick while `phase == Review`, since
//! quick has no Review phase). Does NOT rewrite artifacts — the agent decides
//! what to reshape after the tier change.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use crate::{
    commands::agent::state::{Phase, TaskToml, Tier},
    error::{Error, Result},
    layout::Layout,
};

#[derive(Debug, Clone)]
pub struct TaskPromoteOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub to: Tier,
}

#[derive(Debug, Clone)]
pub struct TaskPromoteSummary {
    pub slug: String,
    pub from: Tier,
    pub to: Tier,
}

impl fmt::Display for TaskPromoteSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "task `{}`: tier {:?} -> {:?}",
            self.slug, self.from, self.to
        )
    }
}

pub fn task_promote(opts: TaskPromoteOptions) -> Result<TaskPromoteSummary> {
    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);

    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    let mut toml = TaskToml::load(&task_dir)?;
    let from = toml.tier;

    if !phase_exists_in_tier(opts.to, toml.phase) {
        return Err(Error::IllegalPhaseTransition {
            tier: opts.to,
            from: toml.phase,
            to: toml.phase,
        });
    }

    toml.tier = opts.to;
    toml.updated_at = Utc::now();
    if opts.to == Tier::Deep && toml.max_iterations.is_none() {
        toml.max_iterations = Some(3);
    }
    toml.save(&task_dir)?;

    Ok(TaskPromoteSummary {
        slug: opts.slug,
        from,
        to: opts.to,
    })
}

/// `true` if `phase` is reachable under `tier`'s state machine.
fn phase_exists_in_tier(tier: Tier, phase: Phase) -> bool {
    use Phase::*;
    matches!(
        (tier, phase),
        (Tier::Quick, Design | Execute | Archived)
            | (Tier::Standard, Design | Plan | Execute | Verify | Archived)
            | (Tier::Deep, _)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::agent::task::{
            new::{TaskNewOptions, task_new},
            phase::{TaskPhaseOptions, task_plan, task_review},
        },
        io::PathExt,
    };

    #[test]
    fn legal_promotion_preserves_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();

        let prd_before = tmp
            .path()
            .join(".ark/tasks/demo/PRD.md")
            .read_bytes()
            .unwrap();
        let plan_before = tmp
            .path()
            .join(".ark/tasks/demo/00_PLAN.md")
            .read_bytes()
            .unwrap();

        let s = task_promote(TaskPromoteOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            to: Tier::Deep,
        })
        .unwrap();
        assert_eq!((s.from, s.to), (Tier::Standard, Tier::Deep));

        assert_eq!(
            prd_before,
            tmp.path()
                .join(".ark/tasks/demo/PRD.md")
                .read_bytes()
                .unwrap()
        );
        assert_eq!(
            plan_before,
            tmp.path()
                .join(".ark/tasks/demo/00_PLAN.md")
                .read_bytes()
                .unwrap()
        );

        let toml = TaskToml::load(&tmp.path().join(".ark/tasks/demo")).unwrap();
        assert_eq!(toml.tier, Tier::Deep);
        assert_eq!(toml.max_iterations, Some(3));
    }

    #[test]
    fn illegal_phase_under_target_tier_errors() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Deep,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_review(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();

        // Quick has no Review phase; refuse.
        let err = task_promote(TaskPromoteOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            to: Tier::Quick,
        })
        .unwrap_err();
        assert!(matches!(err, Error::IllegalPhaseTransition { .. }));
    }

    #[test]
    fn promote_not_found_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = task_promote(TaskPromoteOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "ghost".into(),
            to: Tier::Deep,
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskNotFound { .. }));
    }
}
