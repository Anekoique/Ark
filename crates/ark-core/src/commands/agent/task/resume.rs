//! `ark agent task resume` — claim an existing active task as this checkout's focus.
//!
//! Idempotent: re-resuming the slug this checkout already focuses on is a
//! no-op. Refuses with [`Error::TaskNotFound`] when the slug is not in
//! `state.tasks.active` (i.e. the task does not exist or has been archived).

use std::{fmt, path::PathBuf};

use crate::{
    commands::agent::state::validate_slug,
    error::{Error, Result},
    layout::Layout,
    state::state_mutate,
};

/// Options for resuming an active task.
#[derive(Debug, Clone)]
pub struct TaskResumeOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Slug to resume; must be present in `state.tasks.active`.
    pub slug: String,
}

/// Summary of a `task resume` operation.
#[derive(Debug, Clone)]
pub struct TaskResumeSummary {
    /// Slug now focused by this checkout.
    pub slug: String,
    /// Slug whose focus was overwritten by this `task resume`, if any.
    ///
    /// Populated when the checkout had `state.focus = Some(other)` *before*
    /// this call and `other != self.slug`. The CLI surfaces this as a
    /// stderr-rendered warning suggesting `--worktree` for parallel work.
    pub overwrote_focus: Option<String>,
}

impl fmt::Display for TaskResumeSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resumed `{}` for this checkout", self.slug)?;
        if let Some(prev) = &self.overwrote_focus {
            write!(
                f,
                "\nwarn: focus rebound from `{prev}` to `{}`. Run `ark agent task resume --slug \
                 {prev}` to restore, or use a worktree to keep both bound in parallel.",
                self.slug
            )?;
        }
        Ok(())
    }
}

/// Sets this checkout's focus to `slug`, refusing when `slug` is not active.
///
/// # Errors
///
/// Returns [`Error::TaskNotFound`] when `slug` is not in
/// `state.tasks.active` after reconcile.
pub fn task_resume(opts: TaskResumeOptions) -> Result<TaskResumeSummary> {
    validate_slug(&opts.slug)?;
    let layout = Layout::new(&opts.project_root);
    let mut overwrote: Option<String> = None;
    state_mutate(&layout, |state| {
        if !state.tasks.active.iter().any(|s| s == &opts.slug) {
            return Err(Error::TaskNotFound {
                slug: opts.slug.clone(),
            });
        }
        let prev = state.focus.clone();
        state.focus = Some(opts.slug.clone());
        overwrote = match prev {
            Some(p) if p != opts.slug => Some(p),
            _ => None,
        };
        Ok(())
    })?;
    Ok(TaskResumeSummary {
        slug: opts.slug,
        overwrote_focus: overwrote,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::agent::{
            state::Tier,
            task::new::{TaskNewOptions, task_new},
        },
        state::load_state,
    };

    fn fresh_task(tmp: &std::path::Path, slug: &str) {
        task_new(TaskNewOptions {
            project_root: tmp.to_path_buf(),
            slug: slug.into(),
            title: slug.into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
    }

    /// Verifies that resuming an existing active slug succeeds and is idempotent.
    #[test]
    fn task_resume_sets_focus_idempotently() {
        let tmp = tempfile::tempdir().unwrap();
        fresh_task(tmp.path(), "demo");
        // First resume.
        task_resume(TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        // Second resume is a no-op.
        let s = task_resume(TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        assert_eq!(s.slug, "demo");
        let layout = Layout::new(tmp.path());
        let state = load_state(&layout).unwrap();
        assert_eq!(state.focus.as_deref(), Some("demo"));
    }

    /// Verifies that resuming overwrites existing focus.
    #[test]
    fn task_resume_overwrites_existing_focus() {
        let tmp = tempfile::tempdir().unwrap();
        fresh_task(tmp.path(), "alpha");
        fresh_task(tmp.path(), "beta");
        task_resume(TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "alpha".into(),
        })
        .unwrap();
        let layout = Layout::new(tmp.path());
        let state = load_state(&layout).unwrap();
        assert_eq!(state.focus.as_deref(), Some("alpha"));
    }

    /// Verifies that resuming a different slug surfaces `overwrote_focus`.
    #[test]
    fn task_resume_reports_overwrote_focus_on_rebind() {
        let tmp = tempfile::tempdir().unwrap();
        fresh_task(tmp.path(), "alpha");
        fresh_task(tmp.path(), "beta");
        // Focus is on "beta" (latest task_new). Resume "alpha" → overwrote = Some("beta").
        let s = task_resume(TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "alpha".into(),
        })
        .unwrap();
        assert_eq!(s.overwrote_focus.as_deref(), Some("beta"));
        let rendered = s.to_string();
        assert!(
            rendered.contains("focus rebound from `beta` to `alpha`"),
            "expected rebind warning: {rendered}"
        );
    }

    /// Verifies that re-resuming the already-focused slug does not warn.
    #[test]
    fn task_resume_does_not_warn_when_focus_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        fresh_task(tmp.path(), "demo");
        let s = task_resume(TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        assert!(s.overwrote_focus.is_none());
        assert!(!s.to_string().contains("rebound"));
    }

    /// Verifies that resuming a non-existent slug returns `TaskNotFound`.
    #[test]
    fn task_resume_invalid_slug_returns_task_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let err = task_resume(TaskResumeOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "ghost".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskNotFound { .. }));
    }
}
