//! Per-session id cache living under the OS temp directory.
//!
//! Each Ark CLI invocation resolves a session id keyed by `(project_root, ppid)`.
//! The id is a UUID v4 cached at `<temp>/ark-session-<project_hash>-<ppid>.id`,
//! so multiple CLI invocations from the same shell share the same id until the
//! shell exits (its PPID changes) or the cache file is deleted (e.g. by
//! [`release_session_id`] on `task archive`/`task discard`).
//!
//! The cache file is also the liveness probe: [`cache_matches`] reports whether
//! a state-file `[sessions.<uuid>]` entry corresponds to a still-running shell.

use std::path::PathBuf;

use uuid::Uuid;

use crate::{
    error::Result,
    io::{PathExt, hash_bytes},
    layout::Layout,
    session::ppid::Ppid,
};

/// Per-session opaque identity (UUID v4 string).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub String);

impl SessionId {
    /// Returns the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Returns the cache file path for `(project_root, ppid)`.
///
/// Format: `<std::env::temp_dir()>/ark-session-<project_hash>-<ppid>.id` where
/// `project_hash` is the first 16 hex chars of `hash_bytes(project_root)`.
pub fn cache_file_path(layout: &Layout, ppid: u32) -> PathBuf {
    let project_hash = project_hash(layout);
    std::env::temp_dir().join(format!("ark-session-{project_hash}-{ppid}.id"))
}

/// Returns this CLI invocation's session id, generating and caching one on miss.
///
/// Called by commands that *register* or *change* session focus
/// (`task new`, `task resume`, `task archive`, `task discard`). Subsequent
/// calls from the same shell return the cached id because the PPID is stable
/// within one shell.
///
/// Read-only callers (e.g. `ark context`) must use [`lookup_session_id`]
/// instead — it never creates a cache file.
pub fn resolve_session_id(layout: &Layout, ppid: &dyn Ppid) -> Result<SessionId> {
    if let Some(id) = lookup_session_id(layout, ppid)? {
        return Ok(id);
    }
    let id = SessionId(Uuid::new_v4().simple().to_string());
    cache_file_path(layout, ppid.parent_id()).write_bytes(id.as_str().as_bytes())?;
    Ok(id)
}

/// Returns this CLI invocation's session id when one is already cached.
///
/// Pure read; never creates a cache file. Read-only consumers like
/// `ark context` use this so a session that has not yet registered focus
/// (via `task new` / `task resume`) reports `None` instead of materializing
/// a fresh, useless cache entry.
pub fn lookup_session_id(layout: &Layout, ppid: &dyn Ppid) -> Result<Option<SessionId>> {
    let path = cache_file_path(layout, ppid.parent_id());
    Ok(path
        .read_text_optional()?
        .as_deref()
        .map(str::trim)
        .and_then(parse_uuid)
        .map(SessionId))
}

/// Removes this session's cache file. Idempotent.
///
/// Called when focus is released (`task archive` / `task discard` of the
/// session's focused slug). Future CLI invocations from the same shell will
/// generate a fresh id.
///
/// Only deletes when the cache file contents match `id` — protects against
/// a same-shell PPID race where the cache has been rewritten by a sibling
/// invocation between focus probe and release.
pub fn release_session_id(layout: &Layout, ppid: &dyn Ppid, id: &SessionId) -> Result<()> {
    let path = cache_file_path(layout, ppid.parent_id());
    let Some(text) = path.read_text_optional()? else {
        return Ok(());
    };
    if text.trim() == id.as_str() {
        path.remove_if_exists()?;
    }
    Ok(())
}

/// Reports whether the cache file for `(project, pid)` exists and holds `uuid`.
///
/// Used by `state::checkout::reconcile::prune_dead_sessions` to drop entries whose
/// shells have exited or whose pids have been recycled.
pub fn cache_matches(layout: &Layout, pid: u32, uuid: &str) -> bool {
    let path = cache_file_path(layout, pid);
    let Ok(Some(text)) = path.read_text_optional() else {
        return false;
    };
    text.trim() == uuid
}

/// Returns the first 16 hex chars of the project root's SHA-256.
fn project_hash(layout: &Layout) -> String {
    let bytes = layout.root().to_string_lossy();
    let mut hash = hash_bytes(bytes.as_bytes());
    hash.truncate(16);
    hash
}

/// Accepts a UUID string in either hyphenated or simple (no-hyphen) form.
fn parse_uuid(text: &str) -> Option<String> {
    Uuid::parse_str(text).ok().map(|u| u.simple().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ppid::StubPpid;

    fn layout_for(tmp: &tempfile::TempDir) -> Layout {
        Layout::new(tmp.path())
    }

    /// Verifies that `cache_file_path` includes both the project hash and ppid.
    #[test]
    fn cache_file_path_uses_project_hash_and_ppid() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let path = cache_file_path(&layout, 42);
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let prefix = format!("ark-session-{}-", project_hash(&layout));
        assert!(name.starts_with(&prefix), "{name} missing project hash");
        assert!(name.ends_with("-42.id"), "{name} missing ppid suffix");
    }

    /// Verifies that `lookup_session_id` returns `None` without creating a cache.
    #[test]
    fn lookup_returns_none_when_no_cache_present_and_does_not_create() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let ppid = StubPpid(unique_test_ppid());
        let id = lookup_session_id(&layout, &ppid).unwrap();
        assert!(id.is_none());
        assert!(!cache_file_path(&layout, ppid.parent_id()).exists());
    }

    /// Verifies that `lookup_session_id` returns the cached id after `resolve`.
    #[test]
    fn lookup_returns_cached_id_after_resolve() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let ppid = StubPpid(unique_test_ppid());
        let resolved = resolve_session_id(&layout, &ppid).unwrap();
        let looked_up = lookup_session_id(&layout, &ppid).unwrap();
        assert_eq!(Some(resolved.clone()), looked_up);
        release_session_id(&layout, &ppid, &resolved).unwrap();
    }

    /// Verifies that two `resolve_session_id` calls under the same ppid return
    /// the same UUID.
    #[test]
    fn resolve_round_trips_within_same_ppid() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        // Use a unique ppid so the test is hermetic across parallel runs.
        let ppid = StubPpid(unique_test_ppid());
        let first = resolve_session_id(&layout, &ppid).unwrap();
        let second = resolve_session_id(&layout, &ppid).unwrap();
        assert_eq!(first, second);
        // Cleanup so this temp file does not leak across test runs.
        release_session_id(&layout, &ppid, &first).unwrap();
    }

    /// Verifies that `release_session_id` removes the cache file and is idempotent.
    #[test]
    fn release_removes_cache_file_idempotently() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let ppid = StubPpid(unique_test_ppid());
        let id = resolve_session_id(&layout, &ppid).unwrap();
        let path = cache_file_path(&layout, ppid.parent_id());
        assert!(path.exists());
        release_session_id(&layout, &ppid, &id).unwrap();
        assert!(!path.exists());
        // Second release is a no-op.
        release_session_id(&layout, &ppid, &id).unwrap();
    }

    /// Verifies that `release_session_id` does not delete a cache file whose
    /// contents have been rewritten by a sibling invocation.
    #[test]
    fn release_does_not_delete_cache_with_mismatched_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let ppid = StubPpid(unique_test_ppid());
        let id = resolve_session_id(&layout, &ppid).unwrap();
        let path = cache_file_path(&layout, ppid.parent_id());
        // Sibling invocation rewrote the cache (different UUID).
        path.write_bytes(b"0000000000000000000000000000ffff")
            .unwrap();
        release_session_id(&layout, &ppid, &id).unwrap();
        assert!(
            path.exists(),
            "release must not delete a foreign UUID's cache"
        );
        path.remove_if_exists().unwrap();
    }

    /// Verifies that `cache_matches` returns true when the file holds the given UUID.
    #[test]
    fn cache_matches_returns_true_for_matching_uuid() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let ppid = StubPpid(unique_test_ppid());
        let id = resolve_session_id(&layout, &ppid).unwrap();
        assert!(cache_matches(&layout, ppid.parent_id(), id.as_str()));
        release_session_id(&layout, &ppid, &id).unwrap();
    }

    /// Verifies that `cache_matches` returns false when the file is missing or
    /// holds a different UUID.
    #[test]
    fn cache_matches_returns_false_when_file_missing_or_mismatched() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let ppid = StubPpid(unique_test_ppid());
        // Missing.
        assert!(!cache_matches(&layout, ppid.parent_id(), "deadbeef"));
        // Mismatched.
        let id = resolve_session_id(&layout, &ppid).unwrap();
        assert!(!cache_matches(
            &layout,
            ppid.parent_id(),
            "0123456789abcdef"
        ));
        release_session_id(&layout, &ppid, &id).unwrap();
    }

    /// Returns a per-test ppid that does not collide with other tests in this
    /// process. Uses a thread-local counter seeded on the OS pid.
    fn unique_test_ppid() -> u32 {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        (std::process::id() << 8) ^ (n + 1) ^ 0xa5a5_a5a5
    }
}
