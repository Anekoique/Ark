//! Ark-flavored file writes, walkers, and managed-block editing.
//!
//! Low-level filesystem primitives live on [`PathExt`]. This module adds:
//!
//! - [`write_file`] — content-aware writes that distinguish new / unchanged
//!   / overwritten / skipped outcomes.
//! - [`update_managed_block`] / [`remove_managed_block`] / [`read_managed_block`]
//!   — operations on `<!-- NAME:START -->...<!-- NAME:END -->` blocks
//!   embedded in text files like `CLAUDE.md`.
//! - [`walk_files`] — recursive enumeration of files under a directory.

use std::path::Path;

use crate::{error::Result, io::path_ext::PathExt as _};

mod hook;
mod managed_block;
mod walk;

pub(crate) use hook::entry_carries_command;
pub use hook::{
    ARK_CONTEXT_HOOK_COMMAND, HookFileSpec, ark_codex_hook_entry, ark_session_start_hook_entry,
    read_hook_file, remove_hook_file, update_hook_file,
};
#[allow(deprecated)]
pub use hook::{read_settings_hook, remove_settings_hook, update_settings_hook};
pub use managed_block::{
    merge_managed_blocks, read_managed_block, remove_managed_block, scan_managed_markers,
    splice_managed_block, update_managed_block,
};
pub use walk::{walk_files, walk_files_excluding};

/// How to handle an existing file whose contents differ from what we'd write.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    /// Leave the existing file untouched.
    #[default]
    Skip,
    /// Overwrite.
    Force,
}

/// Outcome of a single write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    /// File did not exist and was created.
    Created,
    /// File already contained the requested bytes.
    Unchanged,
    /// Existing file was overwritten.
    Overwritten,
    /// Existing differing file was preserved.
    Skipped,
}

impl WriteOutcome {
    /// Reports whether bytes were written to disk.
    pub fn wrote(self) -> bool {
        matches!(self, Self::Created | Self::Overwritten)
    }
}

/// Writes `contents` to `path`, obeying [`WriteMode`] on conflicts.
///
/// Skips silently when the file already contains byte-identical content.
pub fn write_file(
    path: impl AsRef<Path>,
    contents: &[u8],
    mode: WriteMode,
) -> Result<WriteOutcome> {
    let path = path.as_ref();
    let outcome = match (path.read_optional()?, mode) {
        (None, _) => WriteOutcome::Created,
        (Some(existing), _) if existing == contents => WriteOutcome::Unchanged,
        (Some(_), WriteMode::Skip) => WriteOutcome::Skipped,
        (Some(_), WriteMode::Force) => WriteOutcome::Overwritten,
    };
    if outcome.wrote() {
        path.write_bytes(contents)?;
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_file_creates_new() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("new.txt");
        assert_eq!(
            write_file(&target, b"hi", WriteMode::Skip).unwrap(),
            WriteOutcome::Created
        );
    }

    #[test]
    fn write_file_is_unchanged_on_identical() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"same").unwrap();
        assert_eq!(
            write_file(tmp.path(), b"same", WriteMode::Force).unwrap(),
            WriteOutcome::Unchanged
        );
    }

    #[test]
    fn write_file_skip_mode_preserves() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"old").unwrap();
        assert_eq!(
            write_file(tmp.path(), b"new", WriteMode::Skip).unwrap(),
            WriteOutcome::Skipped
        );
        assert_eq!(std::fs::read(tmp.path()).unwrap(), b"old");
    }

    #[test]
    fn write_file_force_overwrites() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"old").unwrap();
        assert_eq!(
            write_file(tmp.path(), b"new", WriteMode::Force).unwrap(),
            WriteOutcome::Overwritten
        );
    }
}
