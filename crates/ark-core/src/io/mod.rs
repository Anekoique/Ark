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
    ARK_CONTEXT_HOOK_COMMAND, HookFileSpec, WriteMode, WriteOutcome, ark_codex_hook_entry,
    ark_session_start_hook_entry, merge_managed_blocks, read_hook_file, read_managed_block,
    remove_hook_file, remove_managed_block, scan_managed_markers, splice_managed_block,
    update_hook_file, update_managed_block, walk_files, write_file,
};
#[allow(deprecated)]
pub use fs::{read_settings_hook, remove_settings_hook, update_settings_hook};
pub use path_ext::{PathExt, hash_bytes};
