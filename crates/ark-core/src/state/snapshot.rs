//! `.ark.db` — a portable snapshot of an Ark installation.
//!
//! Captures every file Ark owns and every managed block Ark wrote, so
//! [`unload`](crate::commands::unload) can freeze state and
//! [`load`](crate::commands::load) can restore it losslessly.

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
}

impl Default for Snapshot {
    fn default() -> Self {
        Self::new()
    }
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
}
