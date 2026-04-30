//! `ark remove` — wipe Ark from a project, including any `.ark.db` snapshot.

use std::{collections::BTreeMap, fmt, path::PathBuf};

use crate::{
    error::Result,
    io::{PathExt, remove_managed_block},
    layout::Layout,
    platforms::PLATFORMS,
    state::{Manifest, Snapshot},
};

/// Options for removing Ark from a project.
#[derive(Debug, Clone)]
pub struct RemoveOptions {
    /// Project root where Ark should be removed.
    pub project_root: PathBuf,
}

impl RemoveOptions {
    /// Creates remove options for `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }
}

/// What was actually removed for a single platform during `remove`.
#[derive(Debug, Default, Clone, Copy)]
pub struct RemovedPlatform {
    /// Reports whether the platform destination directory was removed.
    pub dest_dir: bool,
    /// Reports whether the platform hook entry was removed.
    pub hook_entry: bool,
}

/// Summary of a remove operation.
#[derive(Debug, Default, Clone)]
pub struct RemoveSummary {
    /// Reports whether `.ark/` was removed.
    pub removed_ark_dir: bool,
    /// Reports whether `.ark.db` was removed.
    pub removed_snapshot: bool,
    /// Number of managed blocks removed.
    pub blocks_removed: usize,
    /// Per-platform removal outcomes, keyed by `Platform::id`.
    pub per_platform: BTreeMap<&'static str, RemovedPlatform>,
}

impl fmt::Display for RemoveSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.removed_ark_dir {
            parts.push(".ark/".into());
        }
        for (id, outcome) in &self.per_platform {
            if outcome.dest_dir {
                parts.push(format!("{id} dir"));
            }
            if outcome.hook_entry {
                parts.push(format!("{id} hook"));
            }
        }
        if self.removed_snapshot {
            parts.push(".ark.db".into());
        }
        if self.blocks_removed > 0 {
            parts.push(format!("{} managed block(s)", self.blocks_removed));
        }
        if parts.is_empty() {
            write!(f, "nothing to remove")
        } else {
            write!(f, "removed {}", parts.join(", "))
        }
    }
}

/// Removes all Ark artifacts from `opts.project_root`.
///
/// Unconditional: unlike `unload`, no state is preserved.
pub fn remove(opts: RemoveOptions) -> Result<RemoveSummary> {
    let layout = Layout::new(&opts.project_root);

    let blocks_removed = remove_recorded_blocks(&layout)?;
    let removed_ark_dir = layout.ark_dir().remove_dir_all()?;

    // Remove the hook entry BEFORE wiping the platform's `removal_root`.
    // For platforms (e.g. Codex) whose hook file lives inside `removal_root`,
    // running `remove_dir` first would delete the file the surgical
    // `remove_hook` needs to act on — leaving any sibling user entries lost
    // and the `RemovedPlatform.hook_entry` flag falsely `false`.
    let mut per_platform = BTreeMap::new();
    for platform in PLATFORMS {
        let hook_entry = platform.remove_hook(&layout)?;
        let dest_dir = platform.remove_dir(&layout)?;
        per_platform.insert(
            platform.id,
            RemovedPlatform {
                dest_dir,
                hook_entry,
            },
        );
    }

    layout
        .prunable_empty_parents()
        .iter()
        .try_for_each(|p| p.remove_dir_if_empty().map(|_| ()))?;

    Ok(RemoveSummary {
        removed_ark_dir,
        removed_snapshot: Snapshot::remove(layout.root())?,
        blocks_removed,
        per_platform,
    })
}

fn remove_recorded_blocks(layout: &Layout) -> Result<usize> {
    // Manifest is authoritative when present. With no manifest, fall back to
    // every shipped platform's managed-block target so partially-tracked
    // installs do not leak `AGENTS.md` (or any future platform's block) on
    // remove.
    let blocks: Vec<(PathBuf, String)> = match Manifest::read(layout.root())? {
        Some(m) => m
            .managed_blocks
            .into_iter()
            .map(|b| (b.file, b.marker))
            .collect(),
        None => PLATFORMS
            .iter()
            .filter_map(|p| {
                p.managed_block_target
                    .map(|f| (PathBuf::from(f), layout.managed_marker().to_string()))
            })
            .collect(),
    };
    let mut count = 0;
    for (file, marker) in blocks {
        if remove_managed_block(layout.resolve(&file), &marker)? {
            count += 1;
        }
    }
    Ok(count)
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
        // Each shipped platform wipes its `removal_root`.
        for p in PLATFORMS {
            assert!(
                summary.per_platform[p.id].dest_dir,
                "expected {} dest_dir to be removed",
                p.id
            );
        }
        // Managed blocks are deduped on (file, marker): Claude writes
        // CLAUDE.md; Codex and OpenCode share AGENTS.md (one block, two
        // platforms). So the count is the number of unique target files.
        let unique_targets: std::collections::BTreeSet<_> = PLATFORMS
            .iter()
            .filter_map(|p| p.managed_block_target)
            .collect();
        assert_eq!(summary.blocks_removed, unique_targets.len());
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

    /// Verifies the source-scan invariant for `remove.rs`.
    #[test]
    fn remove_source_no_bare_std_fs_or_dot_path_literals() {
        crate::commands::tests_common::assert_source_clean(include_str!("remove.rs"));
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
