//! Pre-write backup for `ark upgrade` rollback and `--restore`.
//!
//! Before a non-dry-run upgrade mutates anything, it copies every file it is
//! about to write or delete — plus the manifest — into
//! [`Layout::upgrade_backup_dir`]. If apply fails, [`UpgradeBackup::restore`]
//! puts the tree (and manifest) back. The backup is retained after a
//! successful upgrade so a user who regrets it can `ark upgrade --restore`.
//!
//! One backup is kept at a time: [`UpgradeBackup::capture`] wipes any prior
//! backup first, so "most recent" is unambiguous. The manifest is always part
//! of the backup set (C-20) — restoring file bytes without the manifest would
//! leave the manifest's recorded hashes ahead of the rolled-back files and
//! break the next upgrade's classification.

use std::path::{Path, PathBuf};

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
    state::manifest::MANIFEST_RELATIVE_PATH,
};

/// Subdirectory holding the mirrored file copies inside the backup root.
const FILES_SUBDIR: &str = "files";

/// Capture / restore of the single retained upgrade backup.
pub struct UpgradeBackup<'a> {
    layout: &'a Layout,
}

/// Outcome of a restore.
#[derive(Debug, Default, Clone)]
pub struct RestoreSummary {
    /// Number of files restored from the backup.
    pub restored: usize,
    /// Whether the manifest was restored.
    pub manifest_restored: bool,
}

impl std::fmt::Display for RestoreSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "restored {} file(s){}",
            self.restored,
            if self.manifest_restored {
                " and the manifest"
            } else {
                ""
            }
        )
    }
}

impl<'a> UpgradeBackup<'a> {
    /// Creates a backup handle rooted at `layout`'s backup dir.
    pub fn new(layout: &'a Layout) -> Self {
        Self { layout }
    }

    /// Returns `true` if a retained backup exists.
    pub fn exists(&self) -> bool {
        self.layout.upgrade_backup_dir().exists()
    }

    fn files_root(&self) -> PathBuf {
        self.layout.upgrade_backup_dir().join(FILES_SUBDIR)
    }

    /// Captures `files` (those that exist) plus the manifest into a fresh
    /// backup, replacing any prior one.
    ///
    /// `manifest_bytes` is the manifest content as it stands before any write,
    /// so a rollback restores it to its pre-upgrade state (C-20). Missing
    /// source files are skipped (an Add has nothing to back up).
    pub fn capture(&self, files: &[PathBuf], manifest_bytes: &[u8]) -> Result<()> {
        let root = self.layout.upgrade_backup_dir();
        root.remove_dir_all()?;
        let files_root = self.files_root();
        for relative in files {
            if let Some(bytes) = self.layout.resolve(relative).read_optional()? {
                files_root.join(relative).write_bytes(&bytes)?;
            }
        }
        root.join(MANIFEST_RELATIVE_PATH)
            .write_bytes(manifest_bytes)?;
        Ok(())
    }

    /// Restores every backed-up file and the manifest to the working tree.
    ///
    /// Returns [`Error::NoBackupToRestore`] when no backup exists.
    pub fn restore(&self) -> Result<RestoreSummary> {
        if !self.exists() {
            return Err(Error::NoBackupToRestore {
                path: self.layout.upgrade_backup_dir(),
            });
        }
        let mut summary = RestoreSummary::default();
        let files_root = self.files_root();
        for relative in collect_relative_files(&files_root)? {
            let bytes = files_root.join(&relative).read_bytes()?;
            self.layout.resolve(&relative).write_bytes(&bytes)?;
            summary.restored += 1;
        }
        let manifest_backup = self
            .layout
            .upgrade_backup_dir()
            .join(MANIFEST_RELATIVE_PATH);
        if let Some(bytes) = manifest_backup.read_optional()? {
            self.layout
                .resolve(MANIFEST_RELATIVE_PATH)
                .write_bytes(&bytes)?;
            summary.manifest_restored = true;
        }
        Ok(summary)
    }
}

/// Walks `root` and returns every contained file as a path relative to `root`.
fn collect_relative_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_into(root, root, &mut out)?;
    Ok(out)
}

fn collect_into(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in dir.list_dir()? {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_into(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(tmp: &tempfile::TempDir) -> Layout {
        Layout::new(tmp.path())
    }

    /// capture/restore round-trips file bytes and the manifest.
    #[test]
    fn capture_then_restore_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let l = layout(&tmp);
        let target = PathBuf::from(".ark/workflow.md");
        l.resolve(&target).write_bytes(b"original").unwrap();

        let backup = UpgradeBackup::new(&l);
        backup
            .capture(std::slice::from_ref(&target), b"manifest-v0")
            .unwrap();

        // Simulate a mutating upgrade that then needs rollback.
        l.resolve(&target).write_bytes(b"clobbered").unwrap();
        l.resolve(MANIFEST_RELATIVE_PATH)
            .write_bytes(b"manifest-v1")
            .unwrap();

        let summary = backup.restore().unwrap();
        assert_eq!(summary.restored, 1);
        assert!(summary.manifest_restored);
        assert_eq!(l.resolve(&target).read_bytes().unwrap(), b"original");
        assert_eq!(
            l.resolve(MANIFEST_RELATIVE_PATH).read_bytes().unwrap(),
            b"manifest-v0"
        );
    }

    /// capture replaces any prior backup (one retained at a time).
    #[test]
    fn capture_replaces_prior_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let l = layout(&tmp);
        let backup = UpgradeBackup::new(&l);

        let first = PathBuf::from(".ark/a.md");
        l.resolve(&first).write_bytes(b"a").unwrap();
        backup.capture(std::slice::from_ref(&first), b"m0").unwrap();

        let second = PathBuf::from(".ark/b.md");
        l.resolve(&second).write_bytes(b"b").unwrap();
        backup
            .capture(std::slice::from_ref(&second), b"m1")
            .unwrap();

        // The first file is no longer in the backup tree.
        assert!(
            !l.upgrade_backup_dir()
                .join(FILES_SUBDIR)
                .join(&first)
                .exists()
        );
        assert!(
            l.upgrade_backup_dir()
                .join(FILES_SUBDIR)
                .join(&second)
                .exists()
        );
    }

    /// restore with no backup is `NoBackupToRestore`.
    #[test]
    fn restore_without_backup_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let l = layout(&tmp);
        let err = UpgradeBackup::new(&l).restore().unwrap_err();
        assert!(matches!(err, Error::NoBackupToRestore { .. }));
    }
}
