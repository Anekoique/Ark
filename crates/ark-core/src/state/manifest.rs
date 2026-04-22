//! Installation manifest: `.ark/.installed.json`.
//!
//! Records every file Ark wrote and every managed block it inserted so that
//! subsequent commands can clean up without touching user-authored files.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    io::path_ext::PathExt,
};

/// Relative path to the manifest inside the host project.
pub const MANIFEST_RELATIVE_PATH: &str = ".ark/.installed.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub installed_at: DateTime<Utc>,
    pub files: Vec<PathBuf>,
    pub managed_blocks: Vec<ManagedBlock>,
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
        }
    }

    pub fn record_file(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if !self.files.contains(&path) {
            self.files.push(path);
        }
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
