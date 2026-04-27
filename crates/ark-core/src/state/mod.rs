//! Persisted state Ark writes to disk.
//!
//! - [`manifest::Manifest`] — `.ark/.installed.json`, the list of everything
//!   the most recent `init` / `load` produced.
//! - [`snapshot::Snapshot`] — `.ark.db`, a portable dump of the full Ark
//!   footprint used to hibernate and restore state across `unload` / `load`.

pub mod manifest;
pub mod snapshot;

pub use manifest::Manifest;
pub use snapshot::{SNAPSHOT_FILENAME, Snapshot, SnapshotBlock, SnapshotFile, SnapshotHookBody};
