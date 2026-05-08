//! Self-healing migration from the legacy `tasks/.current` pointer to
//! `.ark/.state.toml`.
//!
//! [`synthesize_from_legacy`] is the read path: an in-memory `StateFile`
//! built from whichever legacy files exist. [`delete_legacy_files`] is the
//! write-side finalize step, called by `io::state_mutate` only after a
//! successful save — pure reads never delete legacy files (callers in
//! not-yet-migrated installs see the same files on the next invocation).

use crate::{
    error::Result,
    io::PathExt,
    layout::Layout,
    state::checkout::model::{StateFile, Tasks},
};

/// Builds a `StateFile` from `.ark/tasks/.current`.
///
/// Returns `Ok(StateFile::default())` when the legacy file is absent;
/// callers blend the result with on-disk task dirs via reconcile.
pub fn synthesize_from_legacy(layout: &Layout) -> Result<StateFile> {
    let active = read_legacy_current(layout)?.into_iter().collect();
    Ok(StateFile {
        tasks: Tasks { active },
        focus: None,
    })
}

/// Removes the legacy file. Idempotent.
///
/// Called by `io::state_mutate` after a successful state-file save so a
/// migrated install never sees the old file again.
pub fn delete_legacy_files(layout: &Layout) -> Result<()> {
    layout.tasks_current().remove_if_exists()?;
    Ok(())
}

/// Reads `.ark/tasks/.current` as a single slug, returning the trimmed value.
fn read_legacy_current(layout: &Layout) -> Result<Option<String>> {
    let Some(text) = layout.tasks_current().read_text_optional()? else {
        return Ok(None);
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that an empty checkout synthesizes an empty state.
    #[test]
    fn synthesize_returns_default_when_no_legacy() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let state = synthesize_from_legacy(&layout).unwrap();
        assert!(state.tasks.active.is_empty());
    }

    /// Verifies that legacy `.current` populates active.
    #[test]
    fn synthesize_reads_legacy_current() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.tasks_current().write_bytes(b"foo\n").unwrap();

        let state = synthesize_from_legacy(&layout).unwrap();
        assert_eq!(state.tasks.active, vec!["foo"]);
    }

    /// Verifies that `delete_legacy_files` is idempotent.
    #[test]
    fn delete_legacy_files_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.tasks_current().write_bytes(b"foo\n").unwrap();
        delete_legacy_files(&layout).unwrap();
        assert!(!layout.tasks_current().exists());
        delete_legacy_files(&layout).unwrap();
    }
}
