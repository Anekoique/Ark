//! Per-checkout state index: `.ark/.state.toml`.
//!
//! Carries the active-task slug set and a single `focus` slug pointing at
//! the task this checkout is currently driving. Truth lives in
//! `.ark/tasks/<slug>/task.toml`; this module reconciles against that truth
//! on every read. All mutations route through [`state_mutate`], which takes
//! an exclusive file lock and writes atomically.
//!
//! Focus is set by `task new` / `task resume` and cleared by `task archive`
//! / `task discard` when the cleared slug matches. There is no per-session
//! map: one focus per checkout. Deep-tier worktrees own their own
//! `.state.toml` and so their own focus.
//!
//! Self-healing migration from the legacy `.ark/tasks/.current` file runs at
//! the read path; the legacy file is deleted only after a successful save.

pub mod io;
pub mod migrate;
pub mod model;
pub mod reconcile;

pub use io::{clear_focus_for_slug, is_state_tmp, load_state, resolve_focus_slug, state_mutate};
pub use migrate::synthesize_from_legacy;
pub use model::{StateFile, Tasks};
pub use reconcile::reconcile_against_disk;
