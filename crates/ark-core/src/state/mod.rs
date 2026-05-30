//! Persisted state Ark writes to disk.
//!
//! - [`crate::state::manifest::Manifest`] — `.ark/.installed.json`, the list of everything
//!   the most recent `init` / `load` produced.
//! - [`crate::state::snapshot::Snapshot`] — `.ark.db`, a portable dump of the full Ark
//!   footprint used to hibernate and restore state across `unload` / `load`.
//! - [`crate::state::checkout::StateFile`] — `.ark/.state.toml`, the per-checkout index
//!   carrying the active-task slug set and a single focus pointer.

/// Per-checkout state index (`.ark/.state.toml`).
pub mod checkout;
/// Installed-artifact manifest state.
pub mod manifest;
/// Unload/load snapshot state.
pub mod snapshot;

pub use checkout::{
    StateFile, Tasks, clear_focus_for_slug, is_state_tmp, load_state, reconcile_against_disk,
    resolve_focus_slug, state_mutate, synthesize_from_legacy,
};
pub use manifest::Manifest;
pub use snapshot::{SNAPSHOT_FILENAME, Snapshot, SnapshotBlock, SnapshotFile, SnapshotHookBody};
