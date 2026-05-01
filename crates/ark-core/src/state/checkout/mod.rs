//! Per-checkout state index: `.ark/.state.toml`.
//!
//! Carries developer identity, the active-task slug set, and a per-session
//! focus map. Truth lives in `.ark/tasks/<slug>/task.toml`; this module
//! reconciles against that truth on every read. All mutations route through
//! [`state_mutate`], which takes an exclusive file lock and writes atomically.
//!
//! Sessions register themselves on `task new` / `task resume` (no SessionStart
//! hook integration). Liveness is probed through the per-session cache file
//! under the OS temp directory; see [`crate::session::cache`].
//!
//! Self-healing migration from the legacy `.ark/.developer` and
//! `.ark/tasks/.current` files runs at the read path; legacy files are
//! deleted only after a successful save.

pub mod io;
pub mod migrate;
pub mod model;
pub mod reconcile;

pub use io::{clear_focus_for_slug, is_state_tmp, load_state, state_mutate};
pub use migrate::synthesize_from_legacy;
pub use model::{Identity, Session, StateFile, Tasks};
pub use reconcile::{prune_dead_sessions, reconcile_against_disk};
