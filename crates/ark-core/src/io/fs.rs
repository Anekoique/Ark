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

use std::path::{Path, PathBuf};

use crate::{
    error::{Error, Result},
    io::path_ext::PathExt,
};

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
    Created,
    Unchanged,
    Overwritten,
    Skipped,
}

impl WriteOutcome {
    pub fn wrote(self) -> bool {
        matches!(self, Self::Created | Self::Overwritten)
    }
}

/// Write `contents` to `path`, obeying [`WriteMode`] on conflicts.
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

/// Read the body between `<!-- {marker}:START -->` and `<!-- {marker}:END -->`
/// in `path`, if both delimiters exist. Returns `Ok(None)` if the file or the
/// markers are missing.
pub fn read_managed_block(path: impl AsRef<Path>, marker: &str) -> Result<Option<String>> {
    Ok(path
        .as_ref()
        .read_text_optional()?
        .and_then(|text| Marker::new(marker).extract_body(&text)))
}

/// Insert or replace a delimited managed block in a text file. Creates the
/// file if it doesn't exist. Returns `true` once written.
///
/// Errors with [`Error::ManagedBlockCorrupt`] if the file contains a START
/// marker without a matching END — appending a fresh block in that case would
/// silently duplicate the marker and yield garbled state on subsequent reads.
pub fn update_managed_block(path: impl AsRef<Path>, marker: &str, body: &str) -> Result<bool> {
    let path = path.as_ref();
    let m = Marker::new(marker);
    let block = m.render(body);
    let new_contents = match path.read_text_optional()? {
        None => block,
        Some(text) => match m.replace_in(&text, body) {
            Some(replaced) => replaced,
            None if text.contains(&m.start()) => {
                return Err(Error::ManagedBlockCorrupt {
                    path: path.to_path_buf(),
                    marker: marker.to_string(),
                });
            }
            None => append_block(&text, &block),
        },
    };
    path.write_bytes(new_contents.as_bytes())?;
    Ok(true)
}

/// Remove a managed block from a text file if present. If the resulting file
/// would be effectively empty, deletes it so no Ark-orphaned file lingers.
/// Returns `true` if the block was present and removed.
pub fn remove_managed_block(path: impl AsRef<Path>, marker: &str) -> Result<bool> {
    let path = path.as_ref();
    let Some(stripped) = path
        .read_text_optional()?
        .and_then(|text| Marker::new(marker).strip_from(&text))
    else {
        return Ok(false);
    };
    if stripped.trim().is_empty() {
        path.remove_if_exists()?;
    } else {
        path.write_bytes(stripped.as_bytes())?;
    }
    Ok(true)
}

/// Yield every file under `root` recursively, in an unspecified order.
///
/// Directories are skipped; only regular files are reported. Returns an empty
/// vector if `root` doesn't exist.
pub fn walk_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let root = root.as_ref();
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))? {
            let path = entry.map_err(|e| Error::io(&dir, e))?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn append_block(text: &str, block: &str) -> String {
    let sep = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("{text}{sep}\n{block}")
}

// --- Internal: managed-block delimiter helpers ---

/// `<!-- NAME:START -->` / `<!-- NAME:END -->` delimiter pair. Internal helper
/// for the managed-block functions above.
#[derive(Debug, Clone, Copy)]
struct Marker<'a> {
    name: &'a str,
}

impl<'a> Marker<'a> {
    const fn new(name: &'a str) -> Self {
        Self { name }
    }

    fn start(&self) -> String {
        format!("<!-- {}:START -->", self.name)
    }

    fn end(&self) -> String {
        format!("<!-- {}:END -->", self.name)
    }

    fn render(&self, body: &str) -> String {
        format!("{}\n{}\n{}\n", self.start(), body, self.end())
    }

    fn extract_body(&self, text: &str) -> Option<String> {
        let span = self.locate(text)?;
        Some(text[span.body].trim_matches('\n').to_string())
    }

    fn replace_in(&self, text: &str, body: &str) -> Option<String> {
        let span = self.locate(text)?;
        Some(format!(
            "{prefix}{block}\n{suffix}",
            prefix = &text[..span.start],
            block = self.render(body).trim_end_matches('\n'),
            suffix = &text[span.end..],
        ))
    }

    fn strip_from(&self, text: &str) -> Option<String> {
        let span = self.locate(text)?;
        let before = text[..span.start].trim_end_matches('\n');
        let after = text[span.end..].trim_start_matches('\n');
        Some(match (before.is_empty(), after.is_empty()) {
            (true, true) => String::new(),
            (true, false) => format!("{after}\n"),
            (false, true) => format!("{before}\n"),
            (false, false) => format!("{before}\n{after}"),
        })
    }

    fn locate(&self, text: &str) -> Option<MarkerSpan> {
        let start = text.find(&self.start())?;
        let rel_end = text[start..].find(&self.end())? + start;
        let body_start = start + self.start().len();
        let line_end = text[rel_end..]
            .find('\n')
            .map_or(text.len(), |i| rel_end + i + 1);
        Some(MarkerSpan {
            start,
            end: line_end,
            body: body_start..rel_end,
        })
    }
}

struct MarkerSpan {
    start: usize,
    end: usize,
    body: std::ops::Range<usize>,
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

    #[test]
    fn managed_block_insert_and_replace() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "hello\n").unwrap();
        update_managed_block(tmp.path(), "ARK", "first").unwrap();
        let t1 = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(t1.contains("first"));

        update_managed_block(tmp.path(), "ARK", "second").unwrap();
        let t2 = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(t2.contains("second"));
        assert!(!t2.contains("first"));
    }

    #[test]
    fn managed_block_remove_deletes_file_when_only_block() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "<!-- ARK:START -->\nbody\n<!-- ARK:END -->\n").unwrap();
        assert!(remove_managed_block(tmp.path(), "ARK").unwrap());
        assert!(!tmp.path().exists());
    }

    #[test]
    fn update_managed_block_errors_on_orphan_start() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "<!-- ARK:START -->\nbody\nno-end-here\n").unwrap();
        let err = update_managed_block(tmp.path(), "ARK", "new body").unwrap_err();
        assert!(matches!(err, Error::ManagedBlockCorrupt { .. }));
    }

    #[test]
    fn read_managed_block_returns_body_or_none() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "before\n<!-- ARK:START -->\nfoo\nbar\n<!-- ARK:END -->\nafter\n",
        )
        .unwrap();
        assert_eq!(
            read_managed_block(tmp.path(), "ARK").unwrap().unwrap(),
            "foo\nbar"
        );

        std::fs::write(tmp.path(), "no markers here\n").unwrap();
        assert!(read_managed_block(tmp.path(), "ARK").unwrap().is_none());
    }

    #[test]
    fn walk_files_collects_recursively() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join("a.txt").write_bytes(b"").unwrap();
        tmp.path().join("sub/b.txt").write_bytes(b"").unwrap();
        let files = walk_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn walk_files_returns_empty_for_missing_root() {
        let tmp = tempfile::tempdir().unwrap();
        let files = walk_files(tmp.path().join("nope")).unwrap();
        assert!(files.is_empty());
    }
}
