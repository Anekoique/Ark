//! `[worktree]` section of `.ark/config.toml`.
//!
//! This is user-editable config for the worktree feature.
//!
//! Created by `ark init` from the embedded `config.toml` template;
//! `ark upgrade` does NOT overwrite. Missing file or missing `[worktree]`
//! section → defaults. Corrupt file → [`Error::WorktreeConfigCorrupt`].

use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Represents the on-disk shape of `.ark/config.toml`.
///
/// Each feature owns its own section; missing sections fall through to that
/// feature's `Default` impl.
#[derive(Debug, Clone, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    worktree: Option<WorktreeConfig>,
}

/// User-editable worktree configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeConfig {
    /// Stores the project-relative worktree root path.
    ///
    /// Defaults to `.ark/worktrees`. Absolute paths are rejected via
    /// [`crate::error::Error::InvalidConfigField`].
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,

    /// Stores the default branch prefix.
    ///
    /// Used when neither `--branch-type` nor `--branch` is passed. Defaults
    /// to `feat`.
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,

    /// Lists project-relative paths to copy into each new worktree.
    #[serde(default)]
    pub copy: Vec<String>,

    /// Lists shell commands run in the worktree dir after `git worktree add`.
    ///
    /// Commands run sequentially and abort on first non-zero exit.
    #[serde(default)]
    pub post_create: Vec<String>,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            worktree_dir: default_worktree_dir(),
            branch_prefix: default_branch_prefix(),
            copy: Vec::new(),
            post_create: Vec::new(),
        }
    }
}

impl WorktreeConfig {
    /// Loads the `[worktree]` section of `.ark/config.toml`.
    ///
    /// Returns defaults if the file or the section is absent.
    ///
    /// Distinguishes "file missing" (returns defaults) from real I/O errors.
    pub fn load_or_default(layout: &Layout) -> Result<Self> {
        let path = layout.config_file();
        let cfg = match path.read_text_optional()? {
            Some(text) => {
                let raw: RawConfig = toml::from_str(&text)
                    .map_err(|source| Error::WorktreeConfigCorrupt { path, source })?;
                raw.worktree.unwrap_or_default()
            }
            None => Self::default(),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// Resolves the absolute worktrees-storage dir.
    ///
    /// Derived from `cfg.worktree_dir` joined onto the project root. Falls
    /// back to `layout.worktrees_dir()` when the caller has only the layout;
    /// they are equal under the default config.
    pub fn resolve_worktrees_dir(&self, layout: &Layout) -> PathBuf {
        layout.root().join(&self.worktree_dir)
    }

    /// Returns the absolute path for a specific branch's worktree.
    pub fn resolve_worktree_dir(&self, layout: &Layout, branch: &str) -> PathBuf {
        self.resolve_worktrees_dir(layout).join(branch)
    }

    fn validate(&self) -> Result<()> {
        // `worktree_dir` must stay inside the project. Reject absolute paths
        // and any `..` traversal that would escape the root.
        let p = Path::new(&self.worktree_dir);
        if p.is_absolute() {
            return Err(Error::InvalidConfigField {
                field: "worktree_dir",
                reason: "must be project-relative",
            });
        }
        if p.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(Error::InvalidConfigField {
                field: "worktree_dir",
                reason: "must not contain `..`",
            });
        }
        Ok(())
    }
}

fn default_worktree_dir() -> String {
    ".ark/worktrees".into()
}

fn default_branch_prefix() -> String {
    "feat".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

    /// Verifies that a missing config file returns default values.
    #[test]
    fn load_or_default_returns_defaults_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let cfg = WorktreeConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.worktree_dir, ".ark/worktrees");
        assert_eq!(cfg.branch_prefix, "feat");
        assert!(cfg.copy.is_empty());
        assert!(cfg.post_create.is_empty());
    }

    /// Verifies that corrupt TOML returns `WorktreeConfigCorrupt`.
    #[test]
    fn load_or_default_errors_on_corrupt_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"not = valid = toml")
            .unwrap();
        let err = WorktreeConfig::load_or_default(&layout).unwrap_err();
        assert!(matches!(err, Error::WorktreeConfigCorrupt { .. }));
    }

    /// Verifies that a `worktree_dir` containing `..` returns `InvalidConfigField`.
    #[test]
    fn load_or_default_errors_on_parent_dir_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"[worktree]\nworktree_dir = \"../escape\"\n")
            .unwrap();
        let err = WorktreeConfig::load_or_default(&layout).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfigField {
                field: "worktree_dir",
                ..
            }
        ));
    }

    /// Verifies that an absolute `worktree_dir` returns `InvalidConfigField`.
    #[test]
    fn load_or_default_errors_on_absolute_worktree_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"[worktree]\nworktree_dir = \"/abs/path\"\n")
            .unwrap();
        let err = WorktreeConfig::load_or_default(&layout).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfigField {
                field: "worktree_dir",
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
[worktree]
worktree_dir = ".ark/worktrees"
branch_prefix = "fix"
copy = [".env", ".env.local"]
post_create = ["echo hi"]
"#,
            )
            .unwrap();
        let cfg = WorktreeConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.branch_prefix, "fix");
        assert_eq!(cfg.copy.len(), 2);
        assert_eq!(cfg.post_create.len(), 1);
    }

    /// Verifies that a missing `[worktree]` section returns defaults.
    #[test]
    fn missing_worktree_section_returns_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .config_file()
            .write_bytes(b"[workspace]\njournal_max_lines = 500\n")
            .unwrap();
        let cfg = WorktreeConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.worktree_dir, ".ark/worktrees");
        assert_eq!(cfg.branch_prefix, "feat");
    }
}
