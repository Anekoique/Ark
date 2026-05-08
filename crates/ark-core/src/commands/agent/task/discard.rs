//! `ark agent task discard` — remove an unarchived task in one step.
//!
//! Refuses if the task is already archived (use the archive cleanup path
//! instead) or if any seeded artifact (`PRD.md`, `NN_PLAN.md`, `NN_REVIEW.md`,
//! `VERIFY.md`) has user content. The `--force` flag bypasses the user-content
//! guard but still refuses on archived tasks.

use std::{fmt, path::PathBuf};

use crate::{
    commands::agent::state::{Phase, TaskToml, validate_slug},
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
    state::clear_focus_for_slug,
    templates::ARK_TEMPLATES,
};

/// Embedded-template file names (under `templates/`) keyed by the on-disk
/// filename pattern that maps to them. The matcher takes the on-disk
/// `filename` and returns the template name to compare against, or `None`
/// when the file is not a guarded artifact.
fn template_for(filename: &str) -> Option<&'static str> {
    if filename == "PRD.md" {
        return Some("PRD");
    }
    if filename == "VERIFY.md" {
        return Some("VERIFY");
    }
    if filename == "PLAN.md" {
        return Some("PLAN");
    }
    if filename.ends_with("_PLAN.md") && filename_has_iter_prefix(filename) {
        return Some("PLAN");
    }
    if filename.ends_with("_REVIEW.md") && filename_has_iter_prefix(filename) {
        return Some("REVIEW");
    }
    None
}

/// Returns true when `filename` starts with two ASCII digits and an underscore.
fn filename_has_iter_prefix(filename: &str) -> bool {
    let bytes = filename.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit() && bytes[2] == b'_'
}

/// Options for discarding an unarchived task.
#[derive(Debug, Clone)]
pub struct TaskDiscardOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Slug to discard.
    pub slug: String,
    /// Skip the user-content guard.
    pub force: bool,
}

/// Summary of a `task discard` operation.
#[derive(Debug, Clone)]
pub struct TaskDiscardSummary {
    /// Discarded task slug.
    pub slug: String,
    /// Removed task directory (now absent).
    pub task_dir: PathBuf,
}

impl fmt::Display for TaskDiscardSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "discarded `{}` -> removed {}",
            self.slug,
            self.task_dir.display()
        )
    }
}

/// Removes an unarchived task: clears state references and the focus pointer
/// (when it matched the discarded slug), then deletes the task dir.
///
/// # Errors
///
/// - [`Error::TaskNotFound`] if the task dir is missing or its phase is
///   `Archived` (archived tasks are removed by other paths).
/// - [`Error::TaskStillActive`] if any guarded artifact diverges from its
///   embedded template and `force` is false.
pub fn task_discard(opts: TaskDiscardOptions) -> Result<TaskDiscardSummary> {
    validate_slug(&opts.slug)?;
    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);

    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }
    let toml = TaskToml::load(&task_dir)?;
    if toml.phase == Phase::Archived {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    if !opts.force
        && let Some(file) = first_diverging_artifact(&task_dir)?
    {
        return Err(Error::TaskStillActive {
            slug: opts.slug,
            file,
        });
    }

    clear_focus_for_slug(&layout, &opts.slug)?;
    task_dir.remove_dir_all()?;

    Ok(TaskDiscardSummary {
        slug: opts.slug,
        task_dir,
    })
}

/// Returns the first guarded artifact whose contents diverge from its template.
///
/// Iteration order is `task_dir.list_dir()`'s OS-defined order; "first" here
/// means "first seen by the directory iterator," not lexicographic.
fn first_diverging_artifact(task_dir: &std::path::Path) -> Result<Option<String>> {
    for entry in task_dir.list_dir()? {
        let entry = entry.map_err(|e| Error::io(task_dir, e))?;
        let name = entry.file_name();
        let Some(filename) = name.to_str() else {
            continue;
        };
        let Some(template_name) = template_for(filename) else {
            continue;
        };
        let on_disk = entry.path().read_bytes()?;
        let template = ARK_TEMPLATES
            .get_file(format!("templates/{template_name}.md"))
            .map(|f| f.contents().to_vec())
            .unwrap_or_default();
        if on_disk != template {
            return Ok(Some(filename.to_string()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::{
        state::Tier,
        task::new::{TaskNewOptions, task_new},
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

    /// Verifies that a never-touched task discards cleanly without `--force`.
    #[test]
    fn task_discard_template_unchanged_succeeds_without_force() {
        let tmp = tempfile::tempdir().unwrap();
        fresh_task(tmp.path(), "demo");
        let s = task_discard(TaskDiscardOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            force: false,
        })
        .unwrap();
        assert_eq!(s.slug, "demo");
        assert!(!tmp.path().join(".ark/tasks/demo").exists());
    }

    /// Verifies that an edited PRD refuses without `--force`.
    #[test]
    fn task_discard_edited_prd_refuses_without_force() {
        let tmp = tempfile::tempdir().unwrap();
        fresh_task(tmp.path(), "demo");
        tmp.path()
            .join(".ark/tasks/demo/PRD.md")
            .write_bytes(b"# Real user content")
            .unwrap();
        let err = task_discard(TaskDiscardOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            force: false,
        })
        .unwrap_err();
        assert!(matches!(
            err,
            Error::TaskStillActive { ref file, .. } if file == "PRD.md"
        ));
    }

    /// Verifies that `--force` removes the task even with edits.
    #[test]
    fn task_discard_with_force_removes_edited_task() {
        let tmp = tempfile::tempdir().unwrap();
        fresh_task(tmp.path(), "demo");
        tmp.path()
            .join(".ark/tasks/demo/PRD.md")
            .write_bytes(b"edited")
            .unwrap();
        task_discard(TaskDiscardOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            force: true,
        })
        .unwrap();
        assert!(!tmp.path().join(".ark/tasks/demo").exists());
    }

    /// Verifies that discarding an unknown slug returns `TaskNotFound`.
    #[test]
    fn task_discard_missing_slug_returns_task_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let err = task_discard(TaskDiscardOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "ghost".into(),
            force: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskNotFound { .. }));
    }
}
