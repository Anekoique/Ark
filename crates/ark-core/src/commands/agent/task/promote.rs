//! `ark agent task promote` — change a task's tier mid-flight.
//!
//! Swaps `task.toml.tier`. Refuses if the current phase would be illegal under
//! the target tier (e.g. promoting deep→quick while `phase == Review`, since
//! quick has no Review phase). Does NOT rewrite artifact bodies — the agent
//! decides what to reshape after the tier change. The one structural rename
//! happens on Standard→Deep: the lone `PLAN.md` is moved to `00_PLAN.md` so
//! deep-tier iteration semantics apply without losing the existing body.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use crate::{
    commands::agent::state::{Phase, TaskToml, Tier, validate_slug},
    error::{Error, Result},
    layout::Layout,
};

/// Options for promoting a task to another tier.
#[derive(Debug, Clone)]
pub struct TaskPromoteOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Task slug to promote.
    pub slug: String,
    /// Target workflow tier.
    pub to: Tier,
}

/// Summary of a task tier promotion.
#[derive(Debug, Clone)]
pub struct TaskPromoteSummary {
    /// Task slug that was promoted.
    pub slug: String,
    /// Previous workflow tier.
    pub from: Tier,
    /// New workflow tier.
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

/// Promotes a task to another workflow tier.
pub fn task_promote(opts: TaskPromoteOptions) -> Result<TaskPromoteSummary> {
    validate_slug(&opts.slug)?;

    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);

    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    let mut toml = TaskToml::load(&task_dir)?;
    let from = toml.tier;

    // Research is upstream of tiered implementation, not a sibling tier.
    // Cross-over between research and quick/standard/deep is by a fresh
    // `task new` that cites the research slug in its PRD, not by promote.
    if from == Tier::Research || opts.to == Tier::Research {
        return Err(Error::WrongTier {
            expected: from,
            actual: opts.to,
        });
    }

    if !phase_exists_in_tier(opts.to, toml.phase) {
        return Err(Error::IllegalPhaseTransition {
            tier: opts.to,
            from: toml.phase,
            to: toml.phase,
        });
    }

    // When promoting Standard→Deep, the lone `PLAN.md` becomes the first
    // iteration of the deep loop. Rename it to `00_PLAN.md` so subsequent
    // `task plan` calls (deep iterations) don't shadow the existing body
    // with a fresh empty template.
    if from != Tier::Deep && opts.to == Tier::Deep {
        let plain = task_dir.join("PLAN.md");
        let nn = task_dir.join("00_PLAN.md");
        if plain.is_file() && !nn.exists() {
            std::fs::rename(&plain, &nn).map_err(|e| Error::io(&plain, e))?;
        }
    }

    toml.tier = opts.to;
    toml.updated_at = Utc::now();
    // Keep `max_iterations` consistent with tier semantics: it is Deep-only
    // metadata. Seed on promote-into-Deep; clear on promote-out-of-Deep.
    toml.max_iterations = match opts.to {
        Tier::Deep => Some(toml.max_iterations.unwrap_or(3)),
        _ => None,
    };
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
            worktree: None,
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
        // Standard tier seeds the unprefixed `PLAN.md`; capture it before
        // the promote renames it to `00_PLAN.md`.
        let plan_before = tmp
            .path()
            .join(".ark/tasks/demo/PLAN.md")
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
        // Promote renamed `PLAN.md` → `00_PLAN.md`; body is preserved.
        assert!(!tmp.path().join(".ark/tasks/demo/PLAN.md").exists());
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
            worktree: None,
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
    fn deep_to_standard_clears_max_iterations() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Deep,
            worktree: None,
        })
        .unwrap();
        assert_eq!(
            TaskToml::load(&tmp.path().join(".ark/tasks/demo"))
                .unwrap()
                .max_iterations,
            Some(3),
        );
        task_promote(TaskPromoteOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            to: Tier::Standard,
        })
        .unwrap();
        assert_eq!(
            TaskToml::load(&tmp.path().join(".ark/tasks/demo"))
                .unwrap()
                .max_iterations,
            None,
        );
    }

    #[test]
    fn rejects_path_traversal_slug() {
        let tmp = tempfile::tempdir().unwrap();
        let err = task_promote(TaskPromoteOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "../escape".into(),
            to: Tier::Deep,
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidTaskField { .. }));
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

    /// Rejects promoting a research-tier task out to any other tier.
    ///
    /// Research is upstream of tiered implementation, not a sibling tier;
    /// cross-over is by a fresh `task new` whose PRD cites the research
    /// slug.
    #[test]
    fn task_promote_rejects_research_source() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Research,
            worktree: None,
        })
        .unwrap();
        let task_dir = tmp.path().join(".ark/tasks/demo");
        let before = task_dir.join("task.toml").read_bytes().unwrap();

        let err = task_promote(TaskPromoteOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            to: Tier::Standard,
        })
        .unwrap_err();
        assert!(matches!(err, Error::WrongTier { .. }), "got {err:?}");

        // task.toml byte-identical pre/post: no mutation on the rejection path.
        let after = task_dir.join("task.toml").read_bytes().unwrap();
        assert_eq!(before, after);
    }

    /// Rejects promoting any other tier into research.
    #[test]
    fn task_promote_rejects_research_target() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        let task_dir = tmp.path().join(".ark/tasks/demo");
        let before = task_dir.join("task.toml").read_bytes().unwrap();

        let err = task_promote(TaskPromoteOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            to: Tier::Research,
        })
        .unwrap_err();
        assert!(matches!(err, Error::WrongTier { .. }), "got {err:?}");

        let after = task_dir.join("task.toml").read_bytes().unwrap();
        assert_eq!(before, after);
    }
}
