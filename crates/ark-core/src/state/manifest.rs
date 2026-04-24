//! Installation manifest: `.ark/.installed.json`.
//!
//! Records every file Ark wrote and every managed block it inserted so that
//! subsequent commands can clean up without touching user-authored files.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    io::{hash_bytes, path_ext::PathExt},
};

/// Relative path to the manifest inside the host project.
pub const MANIFEST_RELATIVE_PATH: &str = ".ark/.installed.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub installed_at: DateTime<Utc>,
    pub files: Vec<PathBuf>,
    pub managed_blocks: Vec<ManagedBlock>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hashes: BTreeMap<PathBuf, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedBlock {
    pub file: PathBuf,
    pub marker: String,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            installed_at: Utc::now(),
            files: Vec::new(),
            managed_blocks: Vec::new(),
            hashes: BTreeMap::new(),
        }
    }

    pub fn record_file(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if !self.files.contains(&path) {
            self.files.push(path);
        }
    }

    /// Record a file AND its content hash. Idempotent on repeat calls.
    pub fn record_file_with_hash(&mut self, path: impl Into<PathBuf>, contents: &[u8]) {
        let path = path.into();
        if !self.files.contains(&path) {
            self.files.push(path.clone());
        }
        self.hashes.insert(path, hash_bytes(contents));
    }

    pub fn hash_for(&self, path: &Path) -> Option<&str> {
        self.hashes.get(path).map(String::as_str)
    }

    pub fn clear_hash(&mut self, path: &Path) {
        self.hashes.remove(path);
    }

    /// Remove a file entry from both `files` and `hashes`.
    pub fn drop_file(&mut self, path: &Path) {
        self.files.retain(|p| p != path);
        self.hashes.remove(path);
    }

    pub fn record_block(&mut self, file: impl Into<PathBuf>, marker: impl Into<String>) {
        let block = ManagedBlock {
            file: file.into(),
            marker: marker.into(),
        };
        if !self
            .managed_blocks
            .iter()
            .any(|b| b.file == block.file && b.marker == block.marker)
        {
            self.managed_blocks.push(block);
        }
    }

    pub fn read(project_root: &Path) -> Result<Option<Self>> {
        let path = project_root.join(MANIFEST_RELATIVE_PATH);
        let Some(text) = path.read_text_optional()? else {
            return Ok(None);
        };
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|source| Error::ManifestCorrupt { path, source })
    }

    pub fn write(&self, project_root: &Path) -> Result<()> {
        let path = project_root.join(MANIFEST_RELATIVE_PATH);
        let text = serde_json::to_string_pretty(self).expect("manifest serializes");
        path.write_bytes(text.as_bytes())
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_file_with_hash_populates_both_maps_and_is_idempotent() {
        let mut m = Manifest::new();
        m.record_file_with_hash(PathBuf::from(".ark/workflow.md"), b"hello");
        m.record_file_with_hash(PathBuf::from(".ark/workflow.md"), b"hello");
        assert_eq!(m.files.len(), 1);
        assert_eq!(m.hashes.len(), 1);
        assert_eq!(
            m.hash_for(Path::new(".ark/workflow.md")),
            Some(hash_bytes(b"hello").as_str())
        );
    }

    #[test]
    fn clear_hash_removes_from_hashes_only() {
        let mut m = Manifest::new();
        m.record_file_with_hash(PathBuf::from("a"), b"1");
        m.clear_hash(Path::new("a"));
        assert!(m.files.contains(&PathBuf::from("a")));
        assert!(m.hashes.is_empty());
    }

    #[test]
    fn drop_file_removes_from_both() {
        let mut m = Manifest::new();
        m.record_file_with_hash(PathBuf::from("a"), b"1");
        m.drop_file(Path::new("a"));
        assert!(m.files.is_empty());
        assert!(m.hashes.is_empty());
    }

    #[test]
    fn hash_for_returns_none_for_unknown_path() {
        let m = Manifest::new();
        assert_eq!(m.hash_for(Path::new("nope")), None);
    }

    #[test]
    fn legacy_manifest_without_hashes_field_deserializes() {
        let json = r#"{
            "version": "0.1.0",
            "installed_at": "2026-04-24T00:00:00Z",
            "files": [".ark/workflow.md"],
            "managed_blocks": []
        }"#;
        let m: Manifest = serde_json::from_str(json).unwrap();
        assert!(m.hashes.is_empty());
        assert_eq!(m.files.len(), 1);
    }
}
