//! `[upgrade]` section of `.ark/config.toml`: the per-path file strategy.
//!
//! User-editable config declaring which paths upgrade should leave alone
//! (`ejected`) and which should diff3-merge on divergence (`merged`). Owns
//! its own raw-config struct so a malformed `[upgrade]` section surfaces as
//! [`Error::UpgradeConfigCorrupt`] rather than the worktree feature's error.
//! Missing file or missing section → empty strategy sets (no behavior change).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
    state::manifest::ManagedBlock,
};

/// Raw shape of `.ark/config.toml` as far as the upgrade feature cares.
///
/// Private and upgrade-owned (mirrors the worktree feature's own private
/// `RawConfig`): each feature parses only its own section so corrupt-TOML
/// errors carry the right feature label.
#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    upgrade: Option<UpgradeSection>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpgradeSection {
    #[serde(default)]
    ejected: Vec<PathBuf>,
    #[serde(default)]
    merged: Vec<PathBuf>,
}

/// The resolved per-path strategy for one path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Standard hash-classify pipeline (overwrite / skip / `.new`).
    Default,
    /// Never written, deleted, prompted, or classified by upgrade.
    Ejected,
    /// diff3-merged against a recorded base when both sides diverge.
    Merged,
}

/// User-declared upgrade file strategy from `[upgrade]`.
#[derive(Debug, Clone, Default)]
pub struct UpgradeConfig {
    /// Paths upgrade never touches.
    pub ejected: Vec<PathBuf>,
    /// Non-managed-block paths that diff3-merge on divergence.
    pub merged: Vec<PathBuf>,
}

impl UpgradeConfig {
    /// Loads the `[upgrade]` section of `.ark/config.toml`.
    ///
    /// Returns empty sets when the file or the section is absent. Corrupt
    /// TOML surfaces as [`Error::UpgradeConfigCorrupt`].
    pub fn load_or_default(layout: &Layout) -> Result<Self> {
        let path = layout.config_file();
        let section = match path.read_text_optional()? {
            Some(text) => {
                let raw: RawConfig = toml::from_str(&text)
                    .map_err(|source| Error::UpgradeConfigCorrupt { path, source })?;
                raw.upgrade.unwrap_or_default()
            }
            None => UpgradeSection::default(),
        };
        Ok(Self {
            ejected: section.ejected,
            merged: section.merged,
        })
    }

    /// Validates the strategy sets against path safety, overlap, and blocks.
    ///
    /// - every path resolves safely under the project root (no `..`/absolute);
    /// - no path appears in both `ejected` and `merged`;
    /// - no `merged` path is a managed-block file (their block is already
    ///   preserved on upgrade, so a 3-way merge of the whole file is wrong).
    pub fn validate(&self, blocks: &[ManagedBlock], layout: &Layout) -> Result<()> {
        for path in self.ejected.iter().chain(&self.merged) {
            layout
                .resolve_safe(path)
                .map_err(|_| Error::UpgradeConfigInvalid {
                    path: path.clone(),
                    reason: "path must be project-relative with no `..` traversal",
                })?;
        }
        for path in &self.merged {
            if self.ejected.contains(path) {
                return Err(Error::UpgradeConfigInvalid {
                    path: path.clone(),
                    reason: "path is in both `ejected` and `merged`",
                });
            }
            if blocks.iter().any(|b| &b.file == path) {
                return Err(Error::UpgradeConfigInvalid {
                    path: path.clone(),
                    reason: "managed-block file cannot be `merged` (its block is preserved)",
                });
            }
        }
        Ok(())
    }

    /// Resolves the strategy for `relative`.
    ///
    /// `ejected` wins over `merged` if a path somehow appears in both (the
    /// validate pass rejects that, but resolution stays total and safe).
    pub fn strategy_for(&self, relative: &Path) -> Strategy {
        if self.ejected.iter().any(|p| p == relative) {
            Strategy::Ejected
        } else if self.merged.iter().any(|p| p == relative) {
            Strategy::Merged
        } else {
            Strategy::Default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

    fn layout_with_config(text: &str) -> (tempfile::TempDir, Layout) {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout.config_file().write_bytes(text.as_bytes()).unwrap();
        (tmp, layout)
    }

    /// V-UT-1: missing file and missing section both yield empty sets.
    #[test]
    fn load_or_default_empty_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let cfg = UpgradeConfig::load_or_default(&layout).unwrap();
        assert!(cfg.ejected.is_empty() && cfg.merged.is_empty());

        let (_t, layout) = layout_with_config("[worktree]\nbranch_prefix = \"feat\"\n");
        let cfg = UpgradeConfig::load_or_default(&layout).unwrap();
        assert!(cfg.ejected.is_empty() && cfg.merged.is_empty());
    }

    /// V-UT-1: a full `[upgrade]` section round-trips.
    #[test]
    fn load_or_default_round_trips_full_section() {
        let (_t, layout) = layout_with_config(
            "[upgrade]\nejected = [\".ark/workflow.md\"]\nmerged = \
             [\".claude/commands/ark/quick.md\"]\n",
        );
        let cfg = UpgradeConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.ejected, vec![PathBuf::from(".ark/workflow.md")]);
        assert_eq!(
            cfg.merged,
            vec![PathBuf::from(".claude/commands/ark/quick.md")]
        );
    }

    /// V-UT-9: corrupt `[upgrade]` is an upgrade error, never a worktree one.
    #[test]
    fn corrupt_upgrade_section_is_upgrade_error() {
        let (_t, layout) = layout_with_config("[upgrade]\nejected = \"not-an-array\"\n");
        let err = UpgradeConfig::load_or_default(&layout).unwrap_err();
        assert!(
            matches!(err, Error::UpgradeConfigCorrupt { .. }),
            "got {err:?}"
        );
    }

    /// V-003: an unknown key under `[upgrade]` (e.g. a typo) fails fast.
    #[test]
    fn unknown_upgrade_key_is_rejected() {
        let (_t, layout) = layout_with_config("[upgrade]\nejcted = [\".ark/workflow.md\"]\n");
        let err = UpgradeConfig::load_or_default(&layout).unwrap_err();
        assert!(
            matches!(err, Error::UpgradeConfigCorrupt { .. }),
            "got {err:?}"
        );
    }

    /// V-UT-2: a managed-block file in `merged` is rejected.
    #[test]
    fn validate_rejects_merged_managed_block_file() {
        let layout = Layout::new("/proj");
        let cfg = UpgradeConfig {
            ejected: vec![],
            merged: vec![PathBuf::from("CLAUDE.md")],
        };
        let blocks = vec![ManagedBlock {
            file: PathBuf::from("CLAUDE.md"),
            marker: "ARK".into(),
        }];
        assert!(matches!(
            cfg.validate(&blocks, &layout),
            Err(Error::UpgradeConfigInvalid { .. })
        ));
    }

    /// V-UT-3: overlap and unsafe paths are rejected.
    #[test]
    fn validate_rejects_overlap_and_unsafe() {
        let layout = Layout::new("/proj");
        let overlap = UpgradeConfig {
            ejected: vec![PathBuf::from(".ark/workflow.md")],
            merged: vec![PathBuf::from(".ark/workflow.md")],
        };
        assert!(matches!(
            overlap.validate(&[], &layout),
            Err(Error::UpgradeConfigInvalid { .. })
        ));

        let unsafe_path = UpgradeConfig {
            ejected: vec![PathBuf::from("../escape.md")],
            merged: vec![],
        };
        assert!(matches!(
            unsafe_path.validate(&[], &layout),
            Err(Error::UpgradeConfigInvalid { .. })
        ));
    }

    /// V-UT-4: `strategy_for` resolves each strategy.
    #[test]
    fn strategy_for_resolves() {
        let cfg = UpgradeConfig {
            ejected: vec![PathBuf::from(".ark/workflow.md")],
            merged: vec![PathBuf::from(".claude/commands/ark/quick.md")],
        };
        assert_eq!(
            cfg.strategy_for(Path::new(".ark/workflow.md")),
            Strategy::Ejected
        );
        assert_eq!(
            cfg.strategy_for(Path::new(".claude/commands/ark/quick.md")),
            Strategy::Merged
        );
        assert_eq!(
            cfg.strategy_for(Path::new(".ark/specs/INDEX.md")),
            Strategy::Default
        );
    }
}
