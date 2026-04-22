//! `.ark.db` — a portable snapshot of an Ark installation.
//!
//! Captures every file Ark owns and every managed block Ark wrote, so
//! [`unload`](crate::commands::unload) can freeze state and
//! [`load`](crate::commands::load) can restore it losslessly.
//!
//! `Snapshot` also manages the `.gitignore` entry that keeps `.ark.db` out of
//! commits — the entry is added on unload and removed on load.

use std::path::{Path, PathBuf};

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    io::path_ext::PathExt,
};

/// Relative path (from project root) where snapshots live.
pub const SNAPSHOT_FILENAME: &str = ".ark.db";

const SCHEMA_VERSION: &str = "1";
const GITIGNORE: &str = ".gitignore";
const GITIGNORE_HEADER: &str = "# Ark snapshot (managed by ark unload/load)";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: String,
    pub ark_version: String,
    pub created_at: DateTime<Utc>,
    pub files: Vec<SnapshotFile>,
    pub managed_blocks: Vec<SnapshotBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub path: PathBuf,
    pub content_b64: String,
}

impl SnapshotFile {
    /// Decode the captured bytes back into their original form.
    pub fn decode(&self) -> Result<Vec<u8>> {
        B64.decode(&self.content_b64)
            .map_err(|e| Error::SnapshotCorrupt {
                reason: e.to_string(),
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotBlock {
    pub file: PathBuf,
    pub marker: String,
    pub body: String,
}

impl Snapshot {
    pub fn new() -> Self {
        Self {
            version: SCHEMA_VERSION.to_string(),
            ark_version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: Utc::now(),
            files: Vec::new(),
            managed_blocks: Vec::new(),
        }
    }

    pub fn add_file(&mut self, relative: impl Into<PathBuf>, contents: &[u8]) {
        self.files.push(SnapshotFile {
            path: relative.into(),
            content_b64: B64.encode(contents),
        });
    }

    pub fn add_block(
        &mut self,
        file: impl Into<PathBuf>,
        marker: impl Into<String>,
        body: impl Into<String>,
    ) {
        self.managed_blocks.push(SnapshotBlock {
            file: file.into(),
            marker: marker.into(),
            body: body.into(),
        });
    }

    /// Read `.ark.db` if it exists.
    pub fn read(project_root: &Path) -> Result<Option<Self>> {
        let path = project_root.join(SNAPSHOT_FILENAME);
        let Some(text) = path.read_text_optional()? else {
            return Ok(None);
        };
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| Error::SnapshotCorrupt {
                reason: e.to_string(),
            })
    }

    pub fn write(&self, project_root: &Path) -> Result<()> {
        let path = project_root.join(SNAPSHOT_FILENAME);
        let text = serde_json::to_string_pretty(self).expect("snapshot serializes");
        path.write_bytes(text.as_bytes())
    }

    /// Delete `.ark.db` if it exists. Returns `true` if removed.
    pub fn remove(project_root: &Path) -> Result<bool> {
        project_root.join(SNAPSHOT_FILENAME).remove_if_exists()
    }

    /// Ensure `.ark.db` is listed in the project's `.gitignore`. Returns
    /// `true` if the ignore file was modified.
    ///
    /// Idempotent when the Ark-managed block is intact. Repairs the block if
    /// the header is present but the snapshot entry was removed or edited.
    pub fn ensure_ignored(project_root: &Path) -> Result<bool> {
        let path = project_root.join(GITIGNORE);
        let existing = path.read_text_optional()?.unwrap_or_default();
        if has_intact_ignore_block(&existing) {
            return Ok(false);
        }

        // Header may be present with a missing/edited entry — strip any partial
        // block first so we don't duplicate it.
        let base = strip_gitignore_block(&existing);
        let mut content = base;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(GITIGNORE_HEADER);
        content.push('\n');
        content.push_str(SNAPSHOT_FILENAME);
        content.push('\n');
        path.write_bytes(content.as_bytes())?;
        Ok(true)
    }

    /// Remove the `.ark.db` entry from `.gitignore` if present. Deletes the
    /// file if the removal leaves it empty. Returns `true` if modified.
    pub fn remove_ignored(project_root: &Path) -> Result<bool> {
        let path = project_root.join(GITIGNORE);
        let Some(existing) = path.read_text_optional()? else {
            return Ok(false);
        };
        let stripped = strip_gitignore_block(&existing);
        if stripped == existing {
            return Ok(false);
        }
        if stripped.trim().is_empty() {
            path.remove_if_exists()?;
        } else {
            path.write_bytes(stripped.as_bytes())?;
        }
        Ok(true)
    }
}

impl Default for Snapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// `true` when the Ark header is immediately followed by the snapshot entry
/// (optionally with extra entries under the same header).
fn has_intact_ignore_block(text: &str) -> bool {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line == GITIGNORE_HEADER && lines.next() == Some(SNAPSHOT_FILENAME) {
            return true;
        }
    }
    false
}

fn strip_gitignore_block(text: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if line != GITIGNORE_HEADER {
            kept.push(line);
            continue;
        }
        // Drop every non-blank line under our header (Ark-owned territory).
        while let Some(next) = lines.peek() {
            if next.is_empty() {
                break;
            }
            lines.next();
        }
        // Absorb one trailing blank line to keep surrounding content tidy.
        if matches!(lines.peek(), Some(&"")) {
            lines.next();
        }
        // And a single preceding blank if present.
        if matches!(kept.last(), Some(&"")) {
            kept.pop();
        }
    }
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let mut snap = Snapshot::new();
        let payload = b"\xff\x00hello\nworld\xee";
        snap.add_file(".ark/foo.md", payload);
        snap.add_block("CLAUDE.md", "ARK", "body content\n");

        snap.write(tmp.path()).unwrap();
        let restored = Snapshot::read(tmp.path()).unwrap().unwrap();

        assert_eq!(restored.files.len(), 1);
        assert_eq!(restored.files[0].decode().unwrap(), payload);
        assert_eq!(restored.managed_blocks[0].marker, "ARK");
    }

    #[test]
    fn read_returns_none_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Snapshot::read(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn remove_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        Snapshot::new().write(tmp.path()).unwrap();
        assert!(Snapshot::remove(tmp.path()).unwrap());
        assert!(!Snapshot::remove(tmp.path()).unwrap());
    }

    #[test]
    fn ensure_ignored_creates_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Snapshot::ensure_ignored(tmp.path()).unwrap());
        let text = std::fs::read_to_string(tmp.path().join(GITIGNORE)).unwrap();
        assert!(text.contains(GITIGNORE_HEADER));
        assert!(text.contains(SNAPSHOT_FILENAME));

        assert!(!Snapshot::ensure_ignored(tmp.path()).unwrap());
    }

    #[test]
    fn ensure_ignored_appends_to_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(GITIGNORE);
        std::fs::write(&path, "target/\nnode_modules/\n").unwrap();
        Snapshot::ensure_ignored(tmp.path()).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("target/\nnode_modules/\n"));
        assert!(text.ends_with(".ark.db\n"));
    }

    #[test]
    fn ensure_ignored_repairs_missing_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(GITIGNORE);
        std::fs::write(
            &path,
            "target/\n\n# Ark snapshot (managed by ark unload/load)\n",
        )
        .unwrap();
        assert!(Snapshot::ensure_ignored(tmp.path()).unwrap());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(GITIGNORE_HEADER));
        assert!(text.contains(SNAPSHOT_FILENAME));
        assert!(text.starts_with("target/\n"));
        // Second call is a no-op — idempotent again after repair.
        assert!(!Snapshot::ensure_ignored(tmp.path()).unwrap());
    }

    #[test]
    fn ensure_ignored_repairs_edited_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(GITIGNORE);
        std::fs::write(
            &path,
            "# Ark snapshot (managed by ark unload/load)\nwrong-name.db\n",
        )
        .unwrap();
        assert!(Snapshot::ensure_ignored(tmp.path()).unwrap());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(SNAPSHOT_FILENAME));
        assert!(!text.contains("wrong-name.db"));
    }

    #[test]
    fn remove_ignored_strips_block() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(GITIGNORE);
        std::fs::write(
            &path,
            "target/\n\n# Ark snapshot (managed by ark unload/load)\n.ark.db\n",
        )
        .unwrap();
        assert!(Snapshot::remove_ignored(tmp.path()).unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "target/\n");
    }

    #[test]
    fn remove_ignored_deletes_file_when_only_block() {
        let tmp = tempfile::tempdir().unwrap();
        Snapshot::ensure_ignored(tmp.path()).unwrap();
        assert!(Snapshot::remove_ignored(tmp.path()).unwrap());
        assert!(!tmp.path().join(GITIGNORE).exists());
    }

    #[test]
    fn remove_ignored_is_noop_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!Snapshot::remove_ignored(tmp.path()).unwrap());
    }
}
