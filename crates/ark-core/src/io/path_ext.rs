//! Extension trait that wraps stdlib `std::fs` calls with Ark's `Error::Io`.
//!
//! Every Ark module that touches the filesystem goes through this trait. The
//! goal is to remove the `map_err(|e| Error::io(path, e))` boilerplate that
//! would otherwise clutter every call site.

use std::{fs, io::ErrorKind, path::Path};

use crate::error::{Error, Result};

/// File-system helpers that automatically attach the offending path to I/O
/// errors, distinguish the "file doesn't exist" case from real failures, and
/// expose a small vocabulary of idempotent removes.
pub trait PathExt {
    /// Read file bytes, or `None` if the file doesn't exist.
    fn read_optional(&self) -> Result<Option<Vec<u8>>>;

    /// Read file as UTF-8 text, or `None` if the file doesn't exist.
    fn read_text_optional(&self) -> Result<Option<String>>;

    /// Read file bytes (errors if missing).
    fn read_bytes(&self) -> Result<Vec<u8>>;

    /// Read file as UTF-8 text (errors if missing or non-UTF-8).
    fn read_text(&self) -> Result<String>;

    /// Write bytes to the file. Creates parent directories.
    fn write_bytes(&self, contents: &[u8]) -> Result<()>;

    /// `create_dir_all` with proper error wrapping.
    fn ensure_dir(&self) -> Result<()>;

    /// Iterate entries in this directory, wrapping `std::io::Error` with path context.
    fn list_dir(&self) -> Result<fs::ReadDir>;

    /// Remove this file if it exists. Returns `true` if a file was removed.
    fn remove_if_exists(&self) -> Result<bool>;

    /// Remove this directory if it exists and is empty. Non-empty directories
    /// are a no-op (returns `false`).
    fn remove_dir_if_empty(&self) -> Result<bool>;

    /// Remove this directory tree unconditionally. Returns `true` if it existed.
    fn remove_dir_all(&self) -> Result<bool>;

    /// Rename/move this path to `dest`. Fails loud on cross-device moves; no
    /// copy+delete fallback.
    fn rename_to(&self, dest: impl AsRef<Path>) -> Result<()>;
}

impl<T: AsRef<Path> + ?Sized> PathExt for T {
    fn read_optional(&self) -> Result<Option<Vec<u8>>> {
        let path = self.as_ref();
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    fn read_text_optional(&self) -> Result<Option<String>> {
        let path = self.as_ref();
        match fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    fn read_bytes(&self) -> Result<Vec<u8>> {
        let path = self.as_ref();
        fs::read(path).map_err(|e| Error::io(path, e))
    }

    fn read_text(&self) -> Result<String> {
        let path = self.as_ref();
        fs::read_to_string(path).map_err(|e| Error::io(path, e))
    }

    fn write_bytes(&self, contents: &[u8]) -> Result<()> {
        let path = self.as_ref();
        if let Some(parent) = path.parent() {
            parent.ensure_dir()?;
        }
        fs::write(path, contents).map_err(|e| Error::io(path, e))
    }

    fn ensure_dir(&self) -> Result<()> {
        let path = self.as_ref();
        fs::create_dir_all(path).map_err(|e| Error::io(path, e))
    }

    fn list_dir(&self) -> Result<fs::ReadDir> {
        let path = self.as_ref();
        fs::read_dir(path).map_err(|e| Error::io(path, e))
    }

    fn remove_if_exists(&self) -> Result<bool> {
        let path = self.as_ref();
        match fs::remove_file(path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    fn remove_dir_if_empty(&self) -> Result<bool> {
        let path = self.as_ref();
        match fs::remove_dir(path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) if is_not_empty_error(&e) => Ok(false),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    fn remove_dir_all(&self) -> Result<bool> {
        let path = self.as_ref();
        match fs::remove_dir_all(path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    fn rename_to(&self, dest: impl AsRef<Path>) -> Result<()> {
        let src = self.as_ref();
        fs::rename(src, dest.as_ref()).map_err(|e| Error::io(src, e))
    }
}

fn is_not_empty_error(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(66 | 39 | 145)) || e.to_string().contains("not empty")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_optional_returns_none_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(tmp.path().join("absent").read_optional().unwrap().is_none());
    }

    #[test]
    fn write_bytes_creates_parents() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("a/b/c.txt");
        nested.write_bytes(b"hi").unwrap();
        assert_eq!(nested.read_bytes().unwrap(), b"hi");
    }

    #[test]
    fn remove_dir_if_empty_skips_non_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("d");
        dir.ensure_dir().unwrap();
        dir.join("x").write_bytes(b"").unwrap();
        assert!(!dir.remove_dir_if_empty().unwrap());
        assert!(dir.exists());
    }

    #[test]
    fn rename_to_moves_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.txt");
        let dst = tmp.path().join("b.txt");
        src.write_bytes(b"hello").unwrap();
        src.rename_to(&dst).unwrap();
        assert!(!src.exists());
        assert_eq!(dst.read_bytes().unwrap(), b"hello");
    }

    #[test]
    fn rename_to_errors_on_missing_source() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("missing");
        let dst = tmp.path().join("there");
        let err = src.rename_to(&dst).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    #[test]
    fn read_text_returns_utf8() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a.txt");
        path.write_bytes(b"hello").unwrap();
        assert_eq!(path.read_text().unwrap(), "hello");
    }

    #[test]
    fn read_text_errors_on_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            tmp.path().join("absent").read_text().unwrap_err(),
            Error::Io { .. }
        ));
    }

    #[test]
    fn list_dir_lists_entries() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join("a").write_bytes(b"").unwrap();
        tmp.path().join("b").write_bytes(b"").unwrap();
        let names: std::collections::BTreeSet<_> = tmp
            .path()
            .list_dir()
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains("a"));
        assert!(names.contains("b"));
    }
}
