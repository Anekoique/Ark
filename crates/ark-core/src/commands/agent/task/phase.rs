//! `ark agent task {plan,review,execute,verify}` — explicit phase transitions.
//!
//! Each loads the task.toml, checks legality under tier, mutates phase, and
//! writes back. Plan/Review/Verify also seed their artifact from the
//! embedded template when missing.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use crate::{
    commands::agent::{
        state::{Phase, TaskToml, Tier, check_transition, validate_slug},
        task::verify_seed::{
            SeedInputs, read_plan_goals, read_prd_constraints, read_project_spec_tree,
            read_related_specs, render_seeded_verify,
        },
        template::copy_template,
    },
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Options for moving a task to another lifecycle phase.
#[derive(Debug, Clone)]
pub struct TaskPhaseOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Task slug to transition.
    pub slug: String,
}

/// Summary of a task phase transition.
#[derive(Debug, Clone)]
pub struct TaskPhaseSummary {
    /// Task slug that was transitioned.
    pub slug: String,
    /// Previous phase.
    pub from: Phase,
    /// New phase.
    pub to: Phase,
}

impl fmt::Display for TaskPhaseSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task `{}`: {:?} -> {:?}", self.slug, self.from, self.to)
    }
}

/// Moves a task into the plan phase.
pub fn task_plan(opts: TaskPhaseOptions) -> Result<TaskPhaseSummary> {
    transition(opts, Phase::Plan)
}

/// Moves a task into the review phase.
pub fn task_review(opts: TaskPhaseOptions) -> Result<TaskPhaseSummary> {
    transition(opts, Phase::Review)
}

/// Moves a task into the execute phase.
pub fn task_execute(opts: TaskPhaseOptions) -> Result<TaskPhaseSummary> {
    transition(opts, Phase::Execute)
}

/// Moves a task into the verify phase.
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
    if let Some((template, filename)) = artifact_for(to, toml.tier, toml.iteration) {
        let path = task_dir.join(filename);
        if !path.exists() {
            copy_template(template, &path)?;
            // VERIFY's checklist sections are dynamically seeded from the live
            // project + PRD + plan state. The template's static body carries
            // four substitution markers; we overlay them now.
            if to == Phase::Verify {
                seed_verify_dynamic_sections(&layout, &task_dir, &path)?;
            }
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

/// Returns the template and filename to seed when entering `phase`.
///
/// Standard tier never iterates the plan, so its lone PLAN is named
/// `PLAN.md` (parallel to `VERIFY.md`). Deep tier keeps the `NN_PLAN.md`
/// form to support the iteration loop. Quick tier never enters Plan.
fn artifact_for(phase: Phase, tier: Tier, iteration: u32) -> Option<(&'static str, String)> {
    match phase {
        Phase::Plan => {
            let name = match tier {
                Tier::Deep => format!("{iteration:02}_PLAN.md"),
                _ => "PLAN.md".into(),
            };
            Some(("PLAN", name))
        }
        Phase::Review => Some(("REVIEW", format!("{iteration:02}_REVIEW.md"))),
        Phase::Verify => Some(("VERIFY", "VERIFY.md".into())),
        _ => None,
    }
}

/// Renders the four dynamic VERIFY sections by overlaying marker tokens.
///
/// Reads `.ark/specs/project/INDEX.md`, the task's `PRD.md`, and the latest
/// `NN_PLAN.md`; substitutes the four markers in the just-copied VERIFY
/// template; writes the result back to disk. Errors propagate (a missing
/// marker is a corrupt template; the caller's transition is rolled back by
/// the surrounding logic).
fn seed_verify_dynamic_sections(
    layout: &Layout,
    task_dir: &std::path::Path,
    verify_path: &std::path::Path,
) -> Result<()> {
    let template = verify_path.read_text()?;
    let inputs = SeedInputs {
        project_specs: read_project_spec_tree(&layout.specs_project_index())?,
        related_specs: read_related_specs(&task_dir.join("PRD.md"))?,
        prd_constraints: read_prd_constraints(&task_dir.join("PRD.md"))?,
        plan_goals: latest_plan_goals(task_dir)?,
    };
    let rendered = render_seeded_verify(&template, &inputs, verify_path)?;
    verify_path.write_bytes(rendered.as_bytes())
}

/// Returns the latest plan's Goals as bullet labels.
///
/// Prefers `PLAN.md` (standard tier's single plan); falls back to the
/// highest-numbered `NN_PLAN.md` (deep tier or legacy archives). Absence
/// of any plan (e.g. quick tier) yields an empty list.
fn latest_plan_goals(task_dir: &std::path::Path) -> Result<Vec<String>> {
    match locate_latest_plan(task_dir) {
        Some(path) => read_plan_goals(&path),
        None => Ok(Vec::new()),
    }
}

/// Returns the path to the latest plan in `task_dir`, if any.
///
/// Resolution order: plain `PLAN.md` first; otherwise the highest-numbered
/// `NN_PLAN.md`. Returns `None` when neither form exists.
pub(crate) fn locate_latest_plan(task_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let plain = task_dir.join("PLAN.md");
    if plain.is_file() {
        return Some(plain);
    }
    let entries = std::fs::read_dir(task_dir).ok()?;
    let mut highest: Option<(u32, std::path::PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(stem) = name.strip_suffix("_PLAN.md") else {
            continue;
        };
        let Ok(n) = stem.parse::<u32>() else { continue };
        if highest.as_ref().is_none_or(|(prev, _)| n > *prev) {
            highest = Some((n, entry.path()));
        }
    }
    highest.map(|(_, p)| p)
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
            worktree: None,
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
        // Standard tier seeds the unprefixed `PLAN.md`.
        assert!(tmp.path().join(".ark/tasks/demo/PLAN.md").exists());
        assert!(!tmp.path().join(".ark/tasks/demo/00_PLAN.md").exists());

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
        // Deep tier preserves the iterating `NN_PLAN.md` form.
        assert!(tmp.path().join(".ark/tasks/demo/00_PLAN.md").exists());
        let s = task_review(o(&slug)).unwrap();
        assert_eq!((s.from, s.to), (Phase::Plan, Phase::Review));
        assert!(tmp.path().join(".ark/tasks/demo/00_REVIEW.md").exists());
    }
}
