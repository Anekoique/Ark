//! `.ark.db` — a portable snapshot of an Ark installation.
//!
//! Captures every file Ark owns and every managed block Ark wrote, so
//! [`unload`](crate::commands::unload()) can freeze state and
//! [`load`](crate::commands::load()) can restore it losslessly.

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

/// Portable Ark installation snapshot persisted at `.ark.db`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Snapshot schema version.
    pub version: String,
    /// Ark version that wrote the snapshot.
    pub ark_version: String,
    /// Timestamp when the snapshot was created.
    pub created_at: DateTime<Utc>,
    /// Files captured into the snapshot.
    pub files: Vec<SnapshotFile>,
    /// Managed blocks captured into the snapshot.
    pub managed_blocks: Vec<SnapshotBlock>,
    /// Per-entry hook bodies captured on unload and restored on load.
    ///
    /// Older `.ark.db` files lack this key; serde defaults it to an empty
    /// vector.
    #[serde(default)]
    pub hook_bodies: Vec<SnapshotHookBody>,
}

/// One file captured in a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    /// Project-relative file path.
    pub path: PathBuf,
    /// Base64-encoded file contents.
    pub content_b64: String,
}

impl SnapshotFile {
    /// Decodes the captured bytes back into their original form.
    pub fn decode(&self) -> Result<Vec<u8>> {
        B64.decode(&self.content_b64)
            .map_err(|e| Error::SnapshotCorrupt {
                reason: e.to_string(),
            })
    }
}

/// One managed block captured in a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotBlock {
    /// Project-relative file path carrying the block.
    pub file: PathBuf,
    /// Managed-block marker name.
    pub marker: String,
    /// Managed-block body.
    pub body: String,
}

/// One entry inside a JSON-array hook region, captured by identity key.
///
/// For example, an entry in `.claude/settings.json`'s `hooks.SessionStart`
/// array. The restore path uses `path` and `entry` to splice the entry back
/// via [`crate::io::update_hook_file`]. `json_pointer`, `identity_key`, and
/// `identity_value` are reserved for snapshot portability — external readers
/// and future restore strategies can use them to relocate the entry without
/// re-parsing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHookBody {
    /// Project-relative hook file path.
    pub path: PathBuf,
    /// JSON pointer to the hook array.
    pub json_pointer: String,
    /// Field name used to identify the entry.
    pub identity_key: String,
    /// Field value used to identify the entry.
    pub identity_value: String,
    /// Captured hook entry.
    pub entry: serde_json::Value,
}

impl Snapshot {
    /// Creates an empty snapshot for the current crate version.
    pub fn new() -> Self {
        Self {
            version: SCHEMA_VERSION.to_string(),
            ark_version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: Utc::now(),
            files: Vec::new(),
            managed_blocks: Vec::new(),
            hook_bodies: Vec::new(),
        }
    }

    /// Adds a captured hook body.
    pub fn add_hook_body(&mut self, body: SnapshotHookBody) {
        self.hook_bodies.push(body);
    }

    /// Adds a captured file to the snapshot.
    pub fn add_file(&mut self, relative: impl Into<PathBuf>, contents: &[u8]) {
        self.files.push(SnapshotFile {
            path: relative.into(),
            content_b64: B64.encode(contents),
        });
    }

    /// Adds a captured managed block to the snapshot.
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

    /// Reads `.ark.db` from `project_root`.
    ///
    /// Returns `Ok(None)` if the file does not exist.
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

    /// Writes `.ark.db` under `project_root`.
    pub fn write(&self, project_root: &Path) -> Result<()> {
        let path = project_root.join(SNAPSHOT_FILENAME);
        let text = serde_json::to_string_pretty(self).expect("snapshot serializes");
        path.write_bytes(text.as_bytes())
    }

    /// Deletes `.ark.db` from `project_root` if it exists.
    ///
    /// Returns `true` if a file was removed, `false` if it was already absent.
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

    #[test]
    fn read_accepts_older_snapshot_without_hook_bodies_key() {
        let tmp = tempfile::tempdir().unwrap();
        let legacy = serde_json::json!({
            "version": "1",
            "ark_version": "0.1.2",
            "created_at": "2026-04-24T00:00:00Z",
            "files": [],
            "managed_blocks": []
        });
        let path = tmp.path().join(SNAPSHOT_FILENAME);
        path.write_bytes(serde_json::to_string(&legacy).unwrap().as_bytes())
            .unwrap();

        let snap = Snapshot::read(tmp.path()).unwrap().expect("file present");
        assert!(snap.hook_bodies.is_empty());
    }

    #[test]
    fn add_hook_body_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let mut snap = Snapshot::new();
        snap.add_hook_body(SnapshotHookBody {
            path: PathBuf::from(".claude/settings.json"),
            json_pointer: "/hooks/SessionStart".to_string(),
            identity_key: "command".to_string(),
            identity_value: "ark context --scope session --format json".to_string(),
            entry: serde_json::json!({
                "type": "command",
                "command": "ark context --scope session --format json",
                "timeout": 5000,
            }),
        });
        snap.write(tmp.path()).unwrap();
        let restored = Snapshot::read(tmp.path()).unwrap().unwrap();
        assert_eq!(restored.hook_bodies.len(), 1);
        assert_eq!(restored.hook_bodies[0].identity_key, "command");
    }
}
