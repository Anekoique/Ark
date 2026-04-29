//! `[workspace]` section of `.ark/config.toml` — user-editable config for
//! the workspace feature.
//!
//! Per workspace C-10: created by `ark init` from the embedded `config.toml`
//! template; `ark upgrade` does NOT overwrite. Missing file or missing
//! `[workspace]` section → defaults. Corrupt file →
//! [`Error::WorkspaceConfigCorrupt`]. `journal_max_lines < 100` →
//! [`Error::InvalidConfigField`] (workspace C-9).

use serde::Deserialize;

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Minimum allowed `journal_max_lines`. Smaller caps would rotate so often
/// that the index re-render thrashes (workspace C-9).
const MIN_JOURNAL_MAX_LINES: u32 = 100;

/// On-disk shape of `.ark/config.toml`. Mirrors the worktree-side `RawConfig`
/// but only reads the `[workspace]` section. Each feature module keeps its
/// own raw shape so adding a new section in another feature is independent.
#[derive(Debug, Clone, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    workspace: Option<WorkspaceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    /// Lines per journal file before rotation. Default 2000.
    #[serde(default = "default_journal_max_lines")]
    pub journal_max_lines: u32,

    /// When `false`, `task archive` skips auto-record and returns
    /// [`super::record::WorkspaceRecorded::SkippedDisabled`] without reading
    /// identity or invoking git (workspace G-8 / C-11).
    #[serde(default = "default_auto_record_on_archive")]
    pub auto_record_on_archive: bool,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            journal_max_lines: default_journal_max_lines(),
            auto_record_on_archive: default_auto_record_on_archive(),
        }
    }
}

impl WorkspaceConfig {
    /// Load the `[workspace]` section of `.ark/config.toml`, or return
    /// defaults if the file or the section is absent.
    pub fn load_or_default(layout: &Layout) -> Result<Self> {
        let path = layout.config_file();
        let cfg = match path.read_text_optional()? {
            Some(text) => {
                let raw: RawConfig = toml::from_str(&text)
                    .map_err(|source| Error::WorkspaceConfigCorrupt { path, source })?;
                raw.workspace.unwrap_or_default()
            }
            None => Self::default(),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if self.journal_max_lines < MIN_JOURNAL_MAX_LINES {
            return Err(Error::InvalidConfigField {
                field: "journal_max_lines",
                reason: "must be >= 100",
            });
        }
        Ok(())
    }
}

fn default_journal_max_lines() -> u32 {
    2000
}

fn default_auto_record_on_archive() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

    /// V-UT-3: missing file → defaults.
    #[test]
    fn load_or_default_returns_defaults_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let cfg = WorkspaceConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.journal_max_lines, 2000);
        assert!(cfg.auto_record_on_archive);
    }

    /// V-UT-3: corrupt TOML → WorkspaceConfigCorrupt.
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
        assert!(matches!(err, Error::WorkspaceConfigCorrupt { .. }));
    }

    /// V-UT-3 / V-E-1: journal_max_lines < 100 → InvalidConfigField.
    #[test]
    fn load_or_default_errors_on_too_small_max_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"[workspace]\njournal_max_lines = 99\n")
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

    #[test]
    fn load_or_default_round_trips_full_config() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(
                br#"
[workspace]
journal_max_lines = 500
auto_record_on_archive = false
"#,
            )
            .unwrap();
        let cfg = WorkspaceConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.journal_max_lines, 500);
        assert!(!cfg.auto_record_on_archive);
    }
}
