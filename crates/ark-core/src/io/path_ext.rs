//! Extension trait that wraps stdlib `std::fs` calls with Ark's `Error::Io`.
//!
//! Every Ark module that touches the filesystem goes through this trait. The
//! goal is to remove the `map_err(|e| Error::io(path, e))` boilerplate that
//! would otherwise clutter every call site.

use std::{fmt::Write as _, fs, io::ErrorKind, path::Path};

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// Returns the hex-lowercase SHA-256 of `contents`.
pub fn hash_bytes(contents: &[u8]) -> String {
    let digest = Sha256::digest(contents);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        // Writing into a pre-allocated String cannot fail.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Adds Ark-specific filesystem helpers to path-like values.
pub trait PathExt {
    /// Reads file bytes, or returns `None` if the file does not exist.
    fn read_optional(&self) -> Result<Option<Vec<u8>>>;

    /// Reads the file as UTF-8 text, or returns `None` if the file does not exist.
    fn read_text_optional(&self) -> Result<Option<String>>;

    /// Reads file bytes; errors if the file is missing.
    fn read_bytes(&self) -> Result<Vec<u8>>;

    /// Reads the file as UTF-8 text; errors if the file is missing or not UTF-8.
    fn read_text(&self) -> Result<String>;

    /// Writes `contents` to this path, creating parent directories as needed.
    fn write_bytes(&self, contents: &[u8]) -> Result<()>;

    /// Creates this directory and all required parents.
    fn ensure_dir(&self) -> Result<()>;

    /// Iterates the entries in this directory.
    fn list_dir(&self) -> Result<fs::ReadDir>;

    /// Removes this file if it exists. Returns `true` if a file was removed.
    fn remove_if_exists(&self) -> Result<bool>;

    /// Removes this directory if it exists and is empty.
    fn remove_dir_if_empty(&self) -> Result<bool>;

    /// Removes this directory tree unconditionally.
    fn remove_dir_all(&self) -> Result<bool>;

    /// Renames this path to `dest`.
    fn rename_to(&self, dest: impl AsRef<Path>) -> Result<()>;

    /// Returns the file's hex-lowercase SHA-256 digest.
    fn hash_sha256(&self) -> Result<Option<String>>;

    /// Appends UTF-8 text to this file.
    ///
    /// Opens with `OpenOptions::new().create(true).append(true)`. `O_APPEND`
    /// positions each write at the current end-of-file, so concurrent
    /// appenders do not overwrite each other; the write itself is not
    /// atomic (regular files lack the `PIPE_BUF` guarantee — partial writes
    /// and signal-interrupted writes are still possible). `write_all`
    /// retries short writes; underlying I/O errors propagate as
    /// [`Error::Io`].
    fn append_text(&self, contents: &str) -> Result<()>;
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

    fn hash_sha256(&self) -> Result<Option<String>> {
        Ok(self.read_optional()?.as_deref().map(hash_bytes))
    }

    fn append_text(&self, contents: &str) -> Result<()> {
        use std::io::Write;
        let path = self.as_ref();
        if let Some(parent) = path.parent() {
            parent.ensure_dir()?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| Error::io(path, e))?;
        file.write_all(contents.as_bytes())
            .map_err(|e| Error::io(path, e))?;
        Ok(())
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
    fn hash_bytes_matches_known_vector() {
        assert_eq!(
            hash_bytes(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn hash_sha256_returns_none_for_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(tmp.path().join("absent").hash_sha256().unwrap().is_none());
    }

    #[test]
    fn hash_sha256_hashes_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a.txt");
        path.write_bytes(b"hello").unwrap();
        assert_eq!(
            path.hash_sha256().unwrap().as_deref(),
            Some(hash_bytes(b"hello").as_str())
        );
    }

    /// Verifies that `append_text` creates the file and parent dirs, then appends.
    #[test]
    fn append_text_creates_and_appends() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("nested/journal.md");
        p.append_text("first\n").unwrap();
        p.append_text("second\n").unwrap();
        assert_eq!(p.read_text().unwrap(), "first\nsecond\n");
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
