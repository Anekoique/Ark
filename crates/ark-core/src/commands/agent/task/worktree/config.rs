//! `.ark/worktree.toml` model — user-editable config for the worktree feature.
//!
//! Per worktree-support C-9: created by `ark init` from the embedded template;
//! `ark upgrade` does NOT overwrite. Missing file → defaults (G-1). Corrupt
//! file → [`Error::WorktreeConfigCorrupt`].

use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeConfig {
    /// Project-relative path; default `.ark/worktrees`. Absolute paths
    /// rejected via [`Error::InvalidConfigField`] (worktree-support C-8).
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,

    /// Default branch prefix when neither `--branch-type` nor `--branch` is
    /// passed. Default `feat`.
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,

    /// Project-relative paths to copy into each new worktree on creation
    /// (e.g. `.env`).
    #[serde(default)]
    pub copy: Vec<String>,

    /// Shell commands run in the worktree dir after `git worktree add`,
    /// sequential, abort on first non-zero (worktree-support F-3).
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
    /// Load `.ark/worktree.toml` from the layout, or return defaults if absent.
    /// Distinguishes "file missing" (→ defaults) from real I/O errors.
    pub fn load_or_default(layout: &Layout) -> Result<Self> {
        let path = layout.worktree_config_file();
        let cfg = match path.read_text_optional()? {
            Some(text) => toml::from_str(&text)
                .map_err(|source| Error::WorktreeConfigCorrupt { path, source })?,
            None => Self::default(),
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// Absolute worktrees-storage dir derived from `cfg.worktree_dir` joined
    /// onto the project root. Falls back to `layout.worktrees_dir()` when the
    /// caller has only the layout — they're equal under the default config.
    pub fn resolve_worktrees_dir(&self, layout: &Layout) -> PathBuf {
        layout.root().join(&self.worktree_dir)
    }

    /// Absolute path for a specific branch's worktree under this config.
    pub fn resolve_worktree_dir(&self, layout: &Layout, branch: &str) -> PathBuf {
        self.resolve_worktrees_dir(layout).join(branch)
    }

    fn validate(&self) -> Result<()> {
        // worktree-support C-8: worktree_dir must stay inside the project. Reject
        // absolute paths and any `..` traversal that would escape the root.
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

    /// V-UT-1: missing file → defaults.
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

    /// V-UT-2: corrupt TOML → WorktreeConfigCorrupt.
    #[test]
    fn load_or_default_errors_on_corrupt_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .worktree_config_file()
            .write_bytes(b"not = valid = toml")
            .unwrap();
        let err = WorktreeConfig::load_or_default(&layout).unwrap_err();
        assert!(matches!(err, Error::WorktreeConfigCorrupt { .. }));
    }

    /// worktree_dir containing `..` → InvalidConfigField (must stay in-project).
    #[test]
    fn load_or_default_errors_on_parent_dir_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .worktree_config_file()
            .write_bytes(b"worktree_dir = \"../escape\"\n")
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

    /// V-UT-3: absolute worktree_dir → InvalidConfigField.
    #[test]
    fn load_or_default_errors_on_absolute_worktree_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .worktree_config_file()
            .write_bytes(b"worktree_dir = \"/abs/path\"\n")
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
            .worktree_config_file()
            .write_bytes(
                br#"
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
}
