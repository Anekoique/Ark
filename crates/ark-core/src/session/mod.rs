//! Per-session identity machinery: a UUID v4 cached under the OS temp directory,
//! keyed by `(project_root_hash, parent_pid)`.
//!
//! [`cache`] owns the cache-file lifecycle and the [`cache_matches`] liveness
//! probe used by `state::checkout::reconcile::prune_dead_sessions`. [`ppid`] owns
//! the cross-platform parent-id provider; production passes [`RealPpid`], tests
//! pass [`StubPpid`].
//!
//! This module is a leaf: it MUST NOT import anything from `state::checkout`.
//! Dependency direction is `state::checkout → session::cache`.

pub mod cache;
pub mod ppid;

pub use cache::{
    SessionId, cache_file_path, cache_matches, lookup_session_id, release_session_id,
    resolve_session_id,
};
pub use ppid::{Ppid, RealPpid, StubPpid};
