//! Self-healing migration from the legacy `.developer` + `tasks/.current`
//! layout to `.ark/.state.toml`.
//!
//! [`synthesize_from_legacy`] is the read path: an in-memory `StateFile` built
//! from whichever legacy files exist. [`delete_legacy_files`] is the write-side
//! finalize step, called by `io::state_mutate` only after a successful save —
//! pure reads never delete legacy files (callers in not-yet-migrated installs
//! see the same files on the next invocation).

use chrono::{DateTime, TimeZone, Utc};

use crate::{
    error::Result,
    io::PathExt,
    layout::Layout,
    state::checkout::model::{Identity, StateFile, Tasks},
};

/// Builds a `StateFile` from `.ark/.developer` and `.ark/tasks/.current`.
///
/// Returns `Ok(StateFile::default())` when neither legacy file is present;
/// callers blend the result with on-disk task dirs via reconcile.
pub fn synthesize_from_legacy(layout: &Layout) -> Result<StateFile> {
    let identity = read_legacy_developer(layout)?;
    let active = read_legacy_current(layout)?.into_iter().collect();
    Ok(StateFile {
        identity,
        tasks: Tasks { active },
        sessions: Default::default(),
    })
}

/// Removes the legacy files. Idempotent.
///
/// Called by `io::state_mutate` after a successful state-file save so a
/// migrated install never sees the old files again.
pub fn delete_legacy_files(layout: &Layout) -> Result<()> {
    layout.developer_file().remove_if_exists()?;
    layout.tasks_current().remove_if_exists()?;
    Ok(())
}

/// Parses the line-oriented `.developer` format: `name=<x>\ninitialized_at=<RFC3339>`.
///
/// Tolerates blank lines and unknown keys for forward compatibility (mirrors
/// the original reader in `workspace::identity::read_developer_name`).
fn read_legacy_developer(layout: &Layout) -> Result<Option<Identity>> {
    let Some(text) = layout.developer_file().read_text_optional()? else {
        return Ok(None);
    };
    let mut name: Option<String> = None;
    let mut initialized_at: Option<DateTime<Utc>> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("name=") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                name = Some(trimmed.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("initialized_at=") {
            initialized_at = DateTime::parse_from_rfc3339(rest.trim())
                .ok()
                .map(|d| d.with_timezone(&Utc));
        }
    }
    let Some(name) = name else { return Ok(None) };
    // Use Unix epoch as a safe fallback when the legacy file omits the timestamp.
    let initialized_at = initialized_at.unwrap_or_else(|| {
        Utc.timestamp_opt(0, 0)
            .single()
            .expect("Unix epoch is a valid timestamp")
    });
    Ok(Some(Identity {
        name,
        initialized_at,
    }))
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
        assert!(state.identity.is_none());
        assert!(state.tasks.active.is_empty());
    }

    /// Verifies that legacy `.developer` and `.current` populate identity and active.
    #[test]
    fn synthesize_reads_legacy_developer_and_current() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout
            .developer_file()
            .write_bytes(b"name=alice\ninitialized_at=2026-01-01T00:00:00Z\n")
            .unwrap();
        layout.tasks_current().write_bytes(b"foo\n").unwrap();

        let state = synthesize_from_legacy(&layout).unwrap();
        assert_eq!(
            state.identity.as_ref().map(|i| i.name.as_str()),
            Some("alice")
        );
        assert_eq!(state.tasks.active, vec!["foo"]);
    }

    /// Verifies that missing `.developer` and present `.current` yield identity-less state.
    #[test]
    fn synthesize_handles_missing_developer_independently() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.tasks_current().write_bytes(b"foo\n").unwrap();
        let state = synthesize_from_legacy(&layout).unwrap();
        assert!(state.identity.is_none());
        assert_eq!(state.tasks.active, vec!["foo"]);
    }

    /// Verifies that present `.developer` without `.current` yields no active.
    #[test]
    fn synthesize_handles_missing_current_independently() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout
            .developer_file()
            .write_bytes(b"name=alice\ninitialized_at=2026-01-01T00:00:00Z\n")
            .unwrap();
        let state = synthesize_from_legacy(&layout).unwrap();
        assert!(state.identity.is_some());
        assert!(state.tasks.active.is_empty());
    }

    /// Verifies that `delete_legacy_files` is idempotent.
    #[test]
    fn delete_legacy_files_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout
            .developer_file()
            .write_bytes(b"name=alice\n")
            .unwrap();
        layout.tasks_current().write_bytes(b"foo\n").unwrap();
        delete_legacy_files(&layout).unwrap();
        assert!(!layout.developer_file().exists());
        assert!(!layout.tasks_current().exists());
        // Second call is a no-op.
        delete_legacy_files(&layout).unwrap();
    }
}
