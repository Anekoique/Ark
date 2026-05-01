//! `ark agent task resume` — claim an existing active task as this session's focus.
//!
//! Idempotent: re-resuming the slug this session already focuses on is a
//! no-op. Refuses with [`Error::TaskNotFound`] when the slug is not in
//! `state.tasks.active` (i.e. the task does not exist or has been archived).

use std::{fmt, path::PathBuf};

use crate::{
    commands::agent::state::validate_slug,
    error::{Error, Result},
    layout::Layout,
    session::{
        cache::resolve_session_id,
        ppid::{Ppid, RealPpid},
    },
    state::{Session, state_mutate},
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
    /// Slug now focused by this session.
    pub slug: String,
}

impl fmt::Display for TaskResumeSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resumed `{}` for this session", self.slug)
    }
}

/// Sets this session's focus to `slug`, refusing when `slug` is not active.
///
/// # Errors
///
/// Returns [`Error::TaskNotFound`] when `slug` is not in
/// `state.tasks.active` after reconcile.
pub fn task_resume(opts: TaskResumeOptions) -> Result<TaskResumeSummary> {
    task_resume_with_ppid(opts, &RealPpid::new())
}

/// Test seam for [`task_resume`]: same flow with an injectable parent-id provider.
pub(crate) fn task_resume_with_ppid(
    opts: TaskResumeOptions,
    ppid: &dyn Ppid,
) -> Result<TaskResumeSummary> {
    validate_slug(&opts.slug)?;
    let layout = Layout::new(&opts.project_root);
    let id = resolve_session_id(&layout, ppid)?;
    state_mutate(&layout, ppid, |state| {
        if !state.tasks.active.iter().any(|s| s == &opts.slug) {
            return Err(Error::TaskNotFound {
                slug: opts.slug.clone(),
            });
        }
        state.sessions.insert(
            id.as_str().to_string(),
            Session {
                focus: opts.slug.clone(),
                pid: ppid.parent_id(),
            },
        );
        Ok(())
    })?;
    Ok(TaskResumeSummary { slug: opts.slug })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::agent::{
            state::Tier,
            task::new::{TaskNewOptions, task_new},
        },
        session::ppid::StubPpid,
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

    /// Verifies that resuming an existing active slug succeeds.
    #[test]
    fn task_resume_sets_session_focus_idempotently() {
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
        let state = load_state(&layout, &StubPpid(7)).unwrap();
        assert!(state.sessions.values().any(|sess| sess.focus == "demo"));
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
