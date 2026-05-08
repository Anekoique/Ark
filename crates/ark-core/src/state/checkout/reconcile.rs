//! Reconciliation between `.state.toml` and the on-disk task directories.
//!
//! [`reconcile_against_disk`] runs a single fixed-order pass: enumerate task
//! dirs, add any non-archived slug missing from `tasks.active`, drop active
//! entries whose `task.toml` is missing or archived, sort and dedupe, and
//! finally clear `state.focus` if it points outside the active set.
//!
//! Add precedes drop so a brief inconsistent intermediate state never
//! collapses an active slug. Focus invalidation runs last so a slug
//! removed in the drop pass also clears any stale focus pointing at it.

use std::collections::BTreeSet;

use crate::{
    commands::agent::state::{Phase, TaskToml},
    error::Result,
    io::PathExt,
    layout::Layout,
    state::checkout::model::StateFile,
};

/// Runs the full reconcile pass.
pub fn reconcile_against_disk(layout: &Layout, state: &mut StateFile) -> Result<()> {
    let on_disk = enumerate_active_task_slugs(layout)?;

    // 1. Add slugs present on disk but missing from state.
    for slug in &on_disk {
        if !state.tasks.active.iter().any(|s| s == slug) {
            state.tasks.active.push(slug.clone());
        }
    }

    // 2. Drop slugs absent from on-disk truth (missing dir or archived phase).
    state.tasks.active.retain(|slug| on_disk.contains(slug));

    // 3. Sort + dedup for deterministic serialization.
    state.tasks.active.sort();
    state.tasks.active.dedup();

    // 4. Clear focus when it points outside the active set.
    if let Some(focus) = state.focus.as_ref()
        && !state.tasks.active.iter().any(|s| s == focus)
    {
        state.focus = None;
    }

    Ok(())
}

/// Returns the set of slugs whose task directory exists with non-archived phase.
///
/// Skips the `archive/` subdirectory and any task dir whose `task.toml` is
/// missing or unreadable (judgment: a half-scaffolded directory should not
/// count as active).
fn enumerate_active_task_slugs(layout: &Layout) -> Result<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    let tasks_dir = layout.tasks_dir();
    if !tasks_dir.exists() {
        return Ok(out);
    }
    for entry in tasks_dir.list_dir()? {
        let entry = entry.map_err(|e| crate::error::Error::io(&tasks_dir, e))?;
        // Propagate metadata errors: a transient FS hiccup must not silently
        // collapse to "not a task dir" and let reconcile drop a live slug.
        let file_type = entry
            .file_type()
            .map_err(|e| crate::error::Error::io(entry.path(), e))?;
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "archive" || name_str.starts_with('.') {
            continue;
        }
        let task_dir = entry.path();
        let Ok(toml) = TaskToml::load(&task_dir) else {
            continue;
        };
        if toml.phase != Phase::Archived {
            out.insert(name_str.into_owned());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::commands::agent::state::Tier;

    fn layout_for(tmp: &tempfile::TempDir) -> Layout {
        Layout::new(tmp.path())
    }

    fn write_task(layout: &Layout, slug: &str, phase: Phase) {
        let dir = layout.task_dir(slug);
        dir.ensure_dir().unwrap();
        let toml = TaskToml {
            id: slug.into(),
            title: slug.into(),
            tier: Tier::Quick,
            phase,
            iteration: 0,
            max_iterations: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
            committed_at: None,
            branch: None,
            worktree_path: None,
            base_branch: None,
            start_head: None,
            journal_path: None,
        };
        toml.save(&dir).unwrap();
    }

    /// Verifies that reconcile drops an active slug whose dir is missing.
    #[test]
    fn drops_active_slug_with_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let mut state = StateFile::default();
        state.tasks.active.push("ghost".into());
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert!(state.tasks.active.is_empty());
    }

    /// Verifies that reconcile drops an active slug whose phase is archived.
    #[test]
    fn drops_active_slug_with_phase_archived() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        write_task(&layout, "done", Phase::Archived);
        let mut state = StateFile::default();
        state.tasks.active.push("done".into());
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert!(state.tasks.active.is_empty());
    }

    /// Verifies that reconcile adds a slug present on disk but missing from state.
    #[test]
    fn adds_missing_active_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        write_task(&layout, "fresh", Phase::Design);
        let mut state = StateFile::default();
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert_eq!(state.tasks.active, vec!["fresh"]);
    }

    /// Verifies that reconcile clears focus when its target is no longer active.
    #[test]
    fn clears_focus_when_focus_no_longer_active() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let mut state = StateFile {
            focus: Some("ghost".into()),
            ..StateFile::default()
        };
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert!(state.focus.is_none());
    }

    /// Verifies that reconcile preserves focus when the focused slug is active.
    #[test]
    fn preserves_focus_when_focus_is_active() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        write_task(&layout, "live", Phase::Design);
        let mut state = StateFile {
            focus: Some("live".into()),
            ..StateFile::default()
        };
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert_eq!(state.focus.as_deref(), Some("live"));
    }

    /// Verifies that reconcile is idempotent: calling it twice changes nothing further.
    #[test]
    fn add_then_drop_order_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        write_task(&layout, "a", Phase::Design);
        write_task(&layout, "b", Phase::Design);
        let mut state = StateFile::default();
        reconcile_against_disk(&layout, &mut state).unwrap();
        let after_one = state.clone();
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert_eq!(after_one.tasks.active, state.tasks.active);
        assert_eq!(after_one.focus, state.focus);
    }

    /// Verifies that reconcile recovers `active` when the state file is empty
    /// but task dirs exist on disk.
    #[test]
    fn recovers_active_from_disk_when_state_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        write_task(&layout, "x", Phase::Design);
        write_task(&layout, "y", Phase::Plan);
        let mut state = StateFile::default();
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert_eq!(state.tasks.active, vec!["x".to_string(), "y".to_string()]);
    }

    /// Verifies that reconcile drops a stale active entry whose `task.toml` was
    /// flipped to `Archived` (the post-rename failure recovery case).
    #[test]
    fn archive_rename_failure_recovery_via_reconcile() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        write_task(&layout, "stuck", Phase::Archived);
        let mut state = StateFile::default();
        state.tasks.active.push("stuck".into());
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert!(state.tasks.active.is_empty());
    }

    /// Verifies that reconcile excludes the `archive/` subdirectory from enumeration.
    #[test]
    fn reconcile_with_archive_subdir_does_not_double_count() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        write_task(&layout, "active", Phase::Design);
        // Manually create a fake archived task under archive/2026-01/old.
        let arch = layout.tasks_archive_dir().join("2026-01").join("old");
        arch.ensure_dir().unwrap();
        let toml = TaskToml {
            id: "old".into(),
            title: "old".into(),
            tier: Tier::Quick,
            phase: Phase::Archived,
            iteration: 0,
            max_iterations: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: Some(Utc::now()),
            committed_at: None,
            branch: None,
            worktree_path: None,
            base_branch: None,
            start_head: None,
            journal_path: None,
        };
        toml.save(&arch).unwrap();

        let mut state = StateFile::default();
        reconcile_against_disk(&layout, &mut state).unwrap();
        assert_eq!(state.tasks.active, vec!["active"]);
    }
}
