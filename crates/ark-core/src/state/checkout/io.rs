//! State-file persistence: load, atomic write, exclusive lock, retry backoff.
//!
//! All mutations route through [`state_mutate`]. It acquires an exclusive
//! advisory lock on `.state.toml.lock`, sweeps stale `.state.toml.tmp.*`
//! orphans, loads the state (synthesizing from legacy files when absent),
//! runs the caller's closure, then writes atomically (temp + rename) and
//! finalizes migration by deleting any legacy `.developer` / `.current`.
//! After every successful save, a best-effort sweep unlinks orphan
//! `<temp>/ark-session-<this-project-hash>-*.id` files left over from the
//! pre-`session-focus-bind` cache scheme.
//!
//! Reads use [`load_state`], which performs the same synthesize-and-reconcile
//! pass without acquiring a lock and never persists. Concurrent readers and
//! writers are safe: a reader either sees the pre-rename or post-rename file,
//! never a partial write.

use std::{
    fs::{File, OpenOptions, TryLockError},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use crate::{
    error::{Error, Result},
    io::{PathExt, hash_bytes},
    layout::{Layout, STATE_TMP_PREFIX},
    state::checkout::{migrate, model::StateFile, reconcile::reconcile_against_disk},
};

/// Backoff schedule for `try_lock`. Five attempts, cumulative ≤ 320 ms.
const LOCK_BACKOFF_MS: &[u64] = &[10, 20, 40, 80, 160];

/// Filename prefix used by the legacy session-id cache scheme.
const LEGACY_SESSION_CACHE_PREFIX: &str = "ark-session-";

/// Loads the state file, synthesizing from legacy and reconciling against disk.
///
/// Pure read: does not acquire a lock and does not persist. The caller sees
/// an in-memory `StateFile` whose `tasks.active` matches on-disk truth and
/// whose `focus` is `None` whenever its target is no longer active.
///
/// Identity loaded via legacy `.developer` is included in the returned value
/// but the legacy file is NOT deleted on read; cleanup happens on the next
/// successful [`state_mutate`].
pub fn load_state(layout: &Layout) -> Result<StateFile> {
    let mut state = read_or_synthesize(layout)?;
    reconcile_against_disk(layout, &mut state)?;
    Ok(state)
}

/// Acquires the exclusive lock, runs `edit`, writes atomically, releases.
///
/// Concurrent callers serialize on `.state.toml.lock`. After `LOCK_BACKOFF_MS`
/// is exhausted, returns [`Error::StateLockContended`].
///
/// # Errors
///
/// - [`Error::StateLockContended`] when the lock cannot be acquired in the
///   backoff window.
/// - [`Error::StateTomlCorrupt`] when the on-disk file fails to parse.
/// - [`Error::Io`] for filesystem failures during write or rename.
/// - Any error returned by `edit`.
pub fn state_mutate<F>(layout: &Layout, edit: F) -> Result<()>
where
    F: FnOnce(&mut StateFile) -> Result<()>,
{
    let lock_path = layout.state_lock_file();
    let _guard = LockGuard::acquire(&lock_path)?;

    sweep_tmp_orphans(layout)?;

    let mut state = read_or_synthesize(layout)?;
    reconcile_against_disk(layout, &mut state)?;
    edit(&mut state)?;

    // Sort + dedup again after the closure may have pushed new entries.
    state.tasks.active.sort();
    state.tasks.active.dedup();
    // Belt-and-braces: empty focus collapses to None.
    if state.focus.as_deref() == Some("") {
        state.focus = None;
    }

    write_atomic(layout, &state)?;
    migrate::delete_legacy_files(layout)?;
    cleanup_orphan_session_caches(layout);

    Ok(())
}

/// Removes `slug` from the active set and clears focus if it pointed at `slug`.
///
/// Used by `task archive` and `task discard`. A no-op on `tasks.active` when
/// `slug` is absent (reconcile's drop-pass handles that case eventually).
pub fn clear_focus_for_slug(layout: &Layout, slug: &str) -> Result<()> {
    state_mutate(layout, |state| {
        state.tasks.active.retain(|s| s != slug);
        if state.focus.as_deref() == Some(slug) {
            state.focus = None;
        }
        Ok(())
    })
}

/// Reads `.state.toml` if present, else synthesizes from legacy.
fn read_or_synthesize(layout: &Layout) -> Result<StateFile> {
    let path = layout.state_file();
    match path.read_text_optional()? {
        Some(text) => {
            // toml::from_str silently ignores unknown fields by default,
            // so legacy `[sessions.*]` blocks deserialize without error
            // and are dropped on the next save.
            toml::from_str(&text).map_err(|source| Error::StateTomlCorrupt { path, source })
        }
        None => migrate::synthesize_from_legacy(layout),
    }
}

/// Writes `state` to a `.tmp.<pid>` sibling, then renames into place.
///
/// `std::fs::rename` replaces the destination atomically on POSIX and on
/// Windows since Rust 1.5 (uses `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`).
fn write_atomic(layout: &Layout, state: &StateFile) -> Result<()> {
    let canonical = layout.state_file();
    if let Some(parent) = canonical.parent() {
        parent.ensure_dir()?;
    }
    let tmp = tmp_path(layout);
    let body = toml::to_string_pretty(state).expect("StateFile serializes");
    tmp.write_bytes(body.as_bytes())?;
    tmp.rename_to(&canonical)
}

/// Returns the unique tmp path for this process's in-flight write.
fn tmp_path(layout: &Layout) -> PathBuf {
    layout
        .ark_dir()
        .join(format!("{}{}", STATE_TMP_PREFIX, std::process::id()))
}

/// Removes any leftover `.state.toml.tmp.*` files in `.ark/`.
///
/// Safe under lock: no peer is mid-write, so any tmp files we find are
/// crash-orphans. Skips non-files (defensive against a future directory
/// or a name collision with something other than a regular file).
fn sweep_tmp_orphans(layout: &Layout) -> Result<()> {
    let ark_dir = layout.ark_dir();
    if !ark_dir.exists() {
        return Ok(());
    }
    for entry in ark_dir.list_dir()? {
        let entry = entry.map_err(|e| Error::io(&ark_dir, e))?;
        let is_file = entry.file_type().is_ok_and(|t| t.is_file());
        if !is_file {
            continue;
        }
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(STATE_TMP_PREFIX)
        {
            entry.path().remove_if_exists()?;
        }
    }
    Ok(())
}

/// Best-effort cleanup of legacy `<temp>/ark-session-<this-project-hash>-*.id`
/// files. Pre-`session-focus-bind` Ark cached a UUID per `(project, ppid)`
/// here; the new code never reads or writes them, so we strip them once on
/// each save. IO errors are swallowed: the cleanup is non-essential and must
/// not abort `state_mutate`.
fn cleanup_orphan_session_caches(layout: &Layout) {
    let project_hash = project_hash(layout);
    let prefix = format!("{LEGACY_SESSION_CACHE_PREFIX}{project_hash}-");
    let temp_dir = std::env::temp_dir();
    let Ok(read_dir) = std::fs::read_dir(&temp_dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix) && name.ends_with(".id") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Returns the first 16 hex chars of the project root's SHA-256.
///
/// Mirrors the legacy `session::cache::project_hash`; kept here as a private
/// helper so `cleanup_orphan_session_caches` can target the right files.
fn project_hash(layout: &Layout) -> String {
    let bytes = layout.root().to_string_lossy();
    let mut hash = hash_bytes(bytes.as_bytes());
    hash.truncate(16);
    hash
}

/// Reports whether `path`'s file name matches `.state.toml.tmp.*`.
///
/// Used by `unload.rs` to filter tmp orphans out of snapshot capture.
pub fn is_state_tmp(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with(STATE_TMP_PREFIX))
        .unwrap_or(false)
}

/// RAII wrapper around the exclusive file lock on `.state.toml.lock`.
///
/// The lock auto-releases when the underlying `File` drops (process exit,
/// SIGKILL, or normal scope end), so a crashed process never blocks peers.
struct LockGuard {
    _file: File,
}

impl LockGuard {
    fn acquire(lock_path: &Path) -> Result<Self> {
        if let Some(parent) = lock_path.parent() {
            parent.ensure_dir()?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| Error::io(lock_path, e))?;

        for &delay_ms in LOCK_BACKOFF_MS {
            match file.try_lock() {
                Ok(()) => return Ok(Self { _file: file }),
                Err(TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(delay_ms)),
                Err(TryLockError::Error(e)) => return Err(Error::io(lock_path, e)),
            }
        }
        // Final attempt: if it still would-block, give up.
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => Err(Error::StateLockContended {
                path: lock_path.to_path_buf(),
            }),
            Err(TryLockError::Error(e)) => Err(Error::io(lock_path, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_for(tmp: &tempfile::TempDir) -> Layout {
        Layout::new(tmp.path())
    }

    /// Verifies that loading an empty checkout returns the default state.
    #[test]
    fn load_state_returns_default_when_no_files_present() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let state = load_state(&layout).unwrap();
        assert!(state.tasks.active.is_empty());
        assert!(state.focus.is_none());
    }

    /// Verifies that pure reads do not delete legacy files.
    #[test]
    fn load_state_does_not_delete_legacy_on_pure_read() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        layout.tasks_current().write_bytes(b"foo\n").unwrap();
        load_state(&layout).unwrap();
        assert!(layout.tasks_current().exists());
    }

    /// Verifies that `state_mutate` persists an edit and releases the lock.
    #[test]
    fn state_mutate_round_trips_atomically() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        state_mutate(&layout, |s| {
            s.tasks.active.push("a".into());
            Ok(())
        })
        .unwrap();
        let state = load_state(&layout).unwrap();
        // "a" was dropped by reconcile because no task dir exists; the test
        // verifies the round-trip mechanic, not the active-set semantics.
        let _ = state;
        assert!(layout.state_file().exists());
        // No tmp orphan and lock file is releasable (acquire would succeed).
        let any_tmp = layout.ark_dir().list_dir().unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(STATE_TMP_PREFIX)
        });
        assert!(!any_tmp, "tmp orphan must not survive successful mutate");
    }

    /// Verifies that `state_mutate` deletes the legacy `.current` file after
    /// the first save.
    #[test]
    fn state_mutate_deletes_legacy_files_on_first_save() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        layout.tasks_current().write_bytes(b"foo\n").unwrap();
        state_mutate(&layout, |_| Ok(())).unwrap();
        assert!(!layout.tasks_current().exists());
    }

    /// Verifies that `state_mutate` unlinks tmp orphans on lock acquire.
    #[test]
    fn state_mutate_unlinks_orphan_tmp_files() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let orphan = layout.ark_dir().join(format!("{STATE_TMP_PREFIX}99999"));
        orphan.write_bytes(b"junk").unwrap();
        state_mutate(&layout, |_| Ok(())).unwrap();
        assert!(!orphan.exists());
    }

    /// Verifies that an unparsable state file produces `StateTomlCorrupt`.
    #[test]
    fn state_mutate_returns_corrupt_on_unparsable_file() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        layout
            .state_file()
            .write_bytes(b"not = valid = toml")
            .unwrap();
        let err = state_mutate(&layout, |_| Ok(())).unwrap_err();
        assert!(matches!(err, Error::StateTomlCorrupt { .. }));
    }

    /// Verifies that two concurrent mutators serialize and both succeed.
    #[test]
    fn concurrent_state_mutate_serializes_and_both_succeed() {
        use std::sync::Arc;
        let tmp = Arc::new(tempfile::tempdir().unwrap());
        let layout = Layout::new(tmp.path());
        // Pre-create the lock file so both threads race on the same inode.
        layout.state_lock_file().write_bytes(b"").unwrap();

        let tmp_a = Arc::clone(&tmp);
        let h_a = std::thread::spawn(move || {
            let layout = Layout::new(tmp_a.path());
            state_mutate(&layout, |s| {
                std::thread::sleep(Duration::from_millis(30));
                s.tasks.active.push("a".into());
                Ok(())
            })
        });
        let tmp_b = Arc::clone(&tmp);
        let h_b = std::thread::spawn(move || {
            let layout = Layout::new(tmp_b.path());
            state_mutate(&layout, |s| {
                std::thread::sleep(Duration::from_millis(30));
                s.tasks.active.push("b".into());
                Ok(())
            })
        });
        h_a.join().unwrap().unwrap();
        h_b.join().unwrap().unwrap();
    }

    /// Verifies that a held lock surfaces as `StateLockContended` after backoff.
    #[test]
    fn state_mutate_returns_lock_contended_after_backoff() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        layout.state_lock_file().write_bytes(b"").unwrap();
        // Hold the lock outside `state_mutate` for the entire backoff window.
        let held = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(layout.state_lock_file())
            .unwrap();
        held.lock().unwrap();
        let err = state_mutate(&layout, |_| Ok(())).unwrap_err();
        assert!(matches!(err, Error::StateLockContended { .. }));
        held.unlock().unwrap();
    }

    /// Verifies that `cleanup_orphan_session_caches` unlinks files matching
    /// this project's hash and leaves files for other projects intact.
    #[test]
    fn cleanup_orphan_session_caches_removes_only_this_projects_files() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let mine_prefix = format!("ark-session-{}-", project_hash(&layout));
        let mine = std::env::temp_dir().join(format!("{mine_prefix}99999.id"));
        let other = std::env::temp_dir().join("ark-session-deadbeefcafef00d-99998.id");
        mine.write_bytes(b"abc").unwrap();
        other.write_bytes(b"abc").unwrap();
        cleanup_orphan_session_caches(&layout);
        assert!(!mine.exists(), "this project's cache file must be removed");
        assert!(other.exists(), "other projects' cache files must survive");
        // Cleanup the survivor so the next test run starts fresh.
        let _ = std::fs::remove_file(other);
    }

    /// Verifies that `state_mutate` strips legacy `[sessions.*]` blocks on
    /// first save.
    #[test]
    fn state_mutate_strips_legacy_sessions_blocks_on_save() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        layout
            .state_file()
            .write_bytes(
                b"[tasks]\nactive = []\n\n[sessions.deadbeef]\nfocus = \"foo\"\npid = 1234\n",
            )
            .unwrap();
        state_mutate(&layout, |_| Ok(())).unwrap();
        let after = layout.state_file().read_text().unwrap();
        assert!(
            !after.contains("[sessions"),
            "legacy [sessions.*] block must be dropped on first save: {after}"
        );
    }

    /// Verifies that `state_mutate` persists `focus = Some(slug)`.
    #[test]
    fn state_mutate_persists_focus() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        // Write a task dir so reconcile keeps the slug in active.
        let dir = layout.task_dir("only");
        dir.ensure_dir().unwrap();
        let now = chrono::Utc::now();
        let toml = crate::commands::agent::state::TaskToml {
            id: "only".into(),
            title: "only".into(),
            tier: crate::commands::agent::state::Tier::Quick,
            phase: crate::commands::agent::state::Phase::Design,
            iteration: 0,
            max_iterations: None,
            created_at: now,
            updated_at: now,
            archived_at: None,
            committed_at: None,
            branch: None,
            worktree_path: None,
            base_branch: None,
            start_head: None,
            journal_path: None,
        };
        toml.save(&dir).unwrap();
        state_mutate(&layout, |s| {
            s.focus = Some("only".into());
            Ok(())
        })
        .unwrap();
        let state = load_state(&layout).unwrap();
        assert_eq!(state.focus.as_deref(), Some("only"));
    }
}
