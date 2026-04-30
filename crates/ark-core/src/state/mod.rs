//! Persisted state Ark writes to disk.
//!
//! - [`crate::state::manifest::Manifest`] — `.ark/.installed.json`, the list of everything
//!   the most recent `init` / `load` produced.
//! - [`crate::state::snapshot::Snapshot`] — `.ark.db`, a portable dump of the full Ark
//!   footprint used to hibernate and restore state across `unload` / `load`.

/// Installed-artifact manifest state.
pub mod manifest;
/// Unload/load snapshot state.
pub mod snapshot;

pub use manifest::Manifest;
pub use snapshot::{SNAPSHOT_FILENAME, Snapshot, SnapshotBlock, SnapshotFile, SnapshotHookBody};
