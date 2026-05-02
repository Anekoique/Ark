//! `[workspace]` section of `.ark/config.toml`.
//!
//! Sister to `WorktreeConfig`. Each feature owns its own section; missing
//! sections fall through to defaults. Created by `ark init` from the
//! embedded `config.toml` template; `ark upgrade` does NOT overwrite.

use serde::Deserialize;

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Default journal-rotation threshold (lines).
const DEFAULT_JOURNAL_MAX_LINES: usize = 2000;

/// Represents the on-disk shape of `.ark/config.toml`.
///
/// We define a fresh `RawConfig` here rather than reusing `worktree::config`'s
/// to keep section parsing local to each feature. Both serde-decode the same
/// TOML file independently — missing sections produce `None` and fall back
/// to defaults in `load_or_default`.
#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    workspace: Option<RawWorkspaceConfig>,
}

/// Raw deserialized form. `WorkspaceConfig` (public) wraps this with
/// validated, accessor-only fields.
#[derive(Debug, Default, Deserialize)]
struct RawWorkspaceConfig {
    #[serde(default = "default_journal_max_lines")]
    journal_max_lines: usize,
}

/// User-editable workspace configuration.
#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    journal_max_lines: usize,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            journal_max_lines: DEFAULT_JOURNAL_MAX_LINES,
        }
    }
}

impl WorkspaceConfig {
    /// Loads the `[workspace]` section of `.ark/config.toml`.
    ///
    /// Returns defaults if the file or section is absent.
    pub fn load_or_default(layout: &Layout) -> Result<Self> {
        let path = layout.config_file();
        let cfg = match path.read_text_optional()? {
            Some(text) => {
                let raw: RawConfig = toml::from_str(&text)
                    .map_err(|source| Error::WorkspaceConfigInvalid { path, source })?;
                match raw.workspace {
                    Some(ws) => Self {
                        journal_max_lines: ws.journal_max_lines,
                    },
                    None => Self::default(),
                }
            }
            None => Self::default(),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// Returns the rotation threshold (lines) before a new `journal-{N+1}.md`
    /// is created.
    pub fn journal_max_lines(&self) -> usize {
        self.journal_max_lines
    }

    fn validate(&self) -> Result<()> {
        if self.journal_max_lines == 0 {
            return Err(Error::InvalidConfigField {
                field: "journal_max_lines",
                reason: "must be greater than 0",
            });
        }
        Ok(())
    }
}

fn default_journal_max_lines() -> usize {
    DEFAULT_JOURNAL_MAX_LINES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

    #[test]
    fn load_or_default_returns_defaults_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let cfg = WorkspaceConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.journal_max_lines(), DEFAULT_JOURNAL_MAX_LINES);
    }

    #[test]
    fn load_or_default_returns_defaults_when_section_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"[other]\nkey = \"value\"\n")
            .unwrap();
        let cfg = WorkspaceConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.journal_max_lines(), DEFAULT_JOURNAL_MAX_LINES);
    }

    #[test]
    fn load_or_default_round_trips_journal_max_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"[workspace]\njournal_max_lines = 5000\n")
            .unwrap();
        let cfg = WorkspaceConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.journal_max_lines(), 5000);
    }

    #[test]
    fn load_or_default_errors_on_corrupt_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"not = valid = toml")
            .unwrap();
        let err = WorkspaceConfig::load_or_default(&layout).unwrap_err();
        assert!(matches!(err, Error::WorkspaceConfigInvalid { .. }));
    }

    #[test]
    fn load_or_default_rejects_zero_max_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"[workspace]\njournal_max_lines = 0\n")
            .unwrap();
        let err = WorkspaceConfig::load_or_default(&layout).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfigField {
                field: "journal_max_lines",
                ..
            }
        ));
    }
}
