//! Filesystem and text-content I/O.
//!
//! - [`path_ext::PathExt`] — the low-level trait that wraps `std::fs` calls
//!   with Ark's error type and idempotent remove helpers.
//! - [`fs`] — content-aware writes, managed-block editing, and a directory
//!   walker.

pub mod fs;
pub mod path_ext;

pub use fs::{
    WriteMode, WriteOutcome, read_managed_block, remove_managed_block, update_managed_block,
    walk_files, write_file,
};
pub use path_ext::PathExt;
