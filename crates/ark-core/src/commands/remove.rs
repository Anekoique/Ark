//! `ark remove` — wipe Ark from a project, including any `.ark.db` snapshot.

use std::{fmt, path::PathBuf};

use crate::{
    error::Result,
    io::{PathExt, remove_managed_block},
    layout::Layout,
    state::{Manifest, Snapshot},
};

#[derive(Debug, Clone)]
pub struct RemoveOptions {
    pub project_root: PathBuf,
}

impl RemoveOptions {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RemoveSummary {
    pub removed_ark_dir: bool,
    pub removed_claude_commands: bool,
    pub removed_snapshot: bool,
    pub blocks_removed: usize,
}

impl fmt::Display for RemoveSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let blocks_label = format!("{} managed block(s)", self.blocks_removed);
        let parts: Vec<&str> = [
            (self.removed_ark_dir, ".ark/"),
            (self.removed_claude_commands, ".claude/commands/ark/"),
            (self.removed_snapshot, ".ark.db"),
            (self.blocks_removed > 0, blocks_label.as_str()),
        ]
        .into_iter()
        .filter_map(|(keep, label)| keep.then_some(label))
        .collect();

        if parts.is_empty() {
            write!(f, "nothing to remove")
        } else {
            write!(f, "removed {}", parts.join(", "))
        }
    }
}

/// Remove all Ark artifacts from `opts.project_root`. Unconditional: unlike
/// `unload`, no state is preserved.
pub fn remove(opts: RemoveOptions) -> Result<RemoveSummary> {
    let layout = Layout::new(&opts.project_root);
    let mut summary = RemoveSummary::default();

    // 1. Managed blocks — use the manifest when present, otherwise fall back
    //    to the default marker in CLAUDE.md.
    if let Some(manifest) = Manifest::read(layout.root())? {
        for block in &manifest.managed_blocks {
            let target = layout.resolve(&block.file);
            if remove_managed_block(&target, &block.marker)? {
                summary.blocks_removed += 1;
            }
        }
    } else if remove_managed_block(layout.claude_md(), layout.managed_marker())? {
        summary.blocks_removed += 1;
    }

    // 2. Directories.
    let [ark_dir, claude_commands] = layout.owned_dirs();
    summary.removed_ark_dir = ark_dir.remove_dir_all()?;
    summary.removed_claude_commands = claude_commands.remove_dir_all()?;
    layout
        .prunable_empty_parents()
        .iter()
        .try_for_each(|p| p.remove_dir_if_empty().map(|_| ()))?;

    // 3. Snapshot.
    summary.removed_snapshot = Snapshot::remove(layout.root())?;

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{
            init::{InitOptions, init},
            unload::{UnloadOptions, unload},
        },
        state::SNAPSHOT_FILENAME,
    };

    #[test]
    fn remove_after_init_wipes_everything() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        let summary = remove(RemoveOptions::new(tmp.path())).unwrap();
        assert!(summary.removed_ark_dir);
        assert!(summary.removed_claude_commands);
        assert_eq!(summary.blocks_removed, 1);
    }

    #[test]
    fn remove_also_nukes_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        unload(UnloadOptions::new(tmp.path())).unwrap();
        assert!(tmp.path().join(SNAPSHOT_FILENAME).exists());

        let summary = remove(RemoveOptions::new(tmp.path())).unwrap();
        assert!(summary.removed_snapshot);
    }

    #[test]
    fn remove_on_clean_repo_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = remove(RemoveOptions::new(tmp.path())).unwrap();
        assert!(!summary.removed_ark_dir);
        assert!(!summary.removed_snapshot);
        assert_eq!(summary.blocks_removed, 0);
    }
}
