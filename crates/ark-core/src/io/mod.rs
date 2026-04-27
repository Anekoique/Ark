//! Filesystem and text-content I/O.
//!
//! - [`path_ext::PathExt`] — the low-level trait that wraps `std::fs` calls
//!   with Ark's error type and idempotent remove helpers.
//! - [`fs`] — content-aware writes, managed-block editing, settings-hook
//!   editing, and a directory walker.
//! - [`git`] — the only sanctioned `Command::new("git")` site (per
//!   ark-context C-26). Kept crate-private; callers route through
//!   [`crate::commands::context::gather`] rather than the raw helper.

pub mod fs;
pub(crate) mod git;
pub mod path_ext;

pub use fs::{
    ARK_CONTEXT_HOOK_COMMAND, WriteMode, WriteOutcome, ark_session_start_hook_entry,
    read_managed_block, read_settings_hook, remove_managed_block, remove_settings_hook,
    scan_managed_markers, splice_managed_block, update_managed_block, update_settings_hook,
    walk_files, write_file,
};
pub use path_ext::{PathExt, hash_bytes};
