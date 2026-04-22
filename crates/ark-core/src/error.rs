use std::{io, path::PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("manifest corrupt at {path}: {source}")]
    ManifestCorrupt {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("snapshot corrupt: {reason}")]
    SnapshotCorrupt { reason: String },

    #[error("ark is already loaded at {path}; pass --force to replace it")]
    AlreadyLoaded { path: PathBuf },

    #[error("no ark installation found at {path}")]
    NotLoaded { path: PathBuf },

    #[error("refusing unsafe snapshot path {path:?}: {reason}")]
    UnsafeSnapshotPath { path: PathBuf, reason: &'static str },
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
