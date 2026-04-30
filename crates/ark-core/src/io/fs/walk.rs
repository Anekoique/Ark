//! Recursive directory walkers.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Yields every file under `root` recursively, in an unspecified order.
///
/// Directories are skipped; only regular files are reported. Returns an empty
/// vector if `root` does not exist.
pub fn walk_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    walk_files_excluding::<&Path>(root, &[])
}

/// Enumerates files under `root` recursively while pruning skipped subtrees.
///
/// Both `root` and `skip_under` entries are expected to be absolute paths.
/// No symlink canonicalization is performed — a prefix that resolves through
/// a symlink will not match. Callers in `unload.rs` get absolute paths for
/// free via `Layout` helpers.
pub fn walk_files_excluding<P: AsRef<Path>>(
    root: impl AsRef<Path>,
    skip_under: &[P],
) -> Result<Vec<PathBuf>> {
    let root = root.as_ref();
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))? {
            let path = entry.map_err(|e| Error::io(&dir, e))?.path();
            if skip_under.iter().any(|p| path.starts_with(p.as_ref())) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::path_ext::PathExt;

    #[test]
    fn walk_files_collects_recursively() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join("a.txt").write_bytes(b"").unwrap();
        tmp.path().join("sub/b.txt").write_bytes(b"").unwrap();
        let files = walk_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    /// Skip prefix prunes the matching subtree.
    #[test]
    fn walk_files_excluding_prunes_skip_subtree() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join("keep/a.txt").write_bytes(b"").unwrap();
        tmp.path().join("skip/b.txt").write_bytes(b"").unwrap();
        tmp.path()
            .join("skip/nested/c.txt")
            .write_bytes(b"")
            .unwrap();
        let skip = tmp.path().join("skip");
        let files = walk_files_excluding(tmp.path(), &[&skip]).unwrap();
        assert_eq!(files.len(), 1, "{files:?}");
        assert!(files[0].ends_with("keep/a.txt"));
    }

    /// Verifies that relative skip paths do not prune absolute roots.
    ///
    /// Callers must pass absolute paths because the match is lexical.
    #[test]
    fn walk_files_excluding_skip_is_lexical_not_canonicalized() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join("skip/b.txt").write_bytes(b"").unwrap();
        let relative_skip = std::path::PathBuf::from("skip");
        let files = walk_files_excluding(tmp.path(), &[&relative_skip]).unwrap();
        assert_eq!(files.len(), 1, "relative skip prefix should not match");
    }

    #[test]
    fn walk_files_excluding_with_empty_skip_matches_walk_files() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join("a.txt").write_bytes(b"").unwrap();
        tmp.path().join("sub/b.txt").write_bytes(b"").unwrap();
        let plain = walk_files(tmp.path()).unwrap();
        let excluding = walk_files_excluding::<&Path>(tmp.path(), &[]).unwrap();
        assert_eq!(plain.len(), excluding.len());
    }

    #[test]
    fn walk_files_returns_empty_for_missing_root() {
        let tmp = tempfile::tempdir().unwrap();
        let files = walk_files(tmp.path().join("nope")).unwrap();
        assert!(files.is_empty());
    }
}
