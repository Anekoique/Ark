//! `ark unload` — freeze Ark state into `.ark.db` and remove live artifacts.
//!
//! Captures every file under Ark-owned directories and every managed block
//! Ark installed, then deletes the live footprint. `.ark.db` is added to
//! `.gitignore` automatically.
//!
//! Pair with `ark load` to restore. `ark remove` discards `.ark.db` entirely.

use std::{fmt, path::PathBuf};

use crate::{
    error::{Error, Result},
    io::{PathExt, read_managed_block, remove_managed_block, walk_files},
    layout::{CLAUDE_MD, Layout},
    state::{Manifest, Snapshot},
};

#[derive(Debug, Clone)]
pub struct UnloadOptions {
    pub project_root: PathBuf,
}

impl UnloadOptions {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UnloadSummary {
    pub files_captured: usize,
    pub blocks_captured: usize,
    pub gitignore_updated: bool,
}

impl fmt::Display for UnloadSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "captured {} file(s) and {} managed block(s) into .ark.db",
            self.files_captured, self.blocks_captured,
        )?;
        if self.gitignore_updated {
            write!(f, "\nadded .ark.db to .gitignore")?;
        }
        Ok(())
    }
}

/// Snapshot and remove Ark from `opts.project_root`.
///
/// Errors with [`Error::NotLoaded`] if there's no `.ark/` directory to unload.
pub fn unload(opts: UnloadOptions) -> Result<UnloadSummary> {
    let layout = Layout::new(&opts.project_root);
    let ark_dir = layout.ark_dir();
    if !ark_dir.exists() {
        return Err(Error::NotLoaded { path: ark_dir });
    }

    let mut snapshot = Snapshot::new();
    let mut summary = UnloadSummary::default();

    // 1. Capture every file under Ark-owned directories.
    for owned in layout.owned_dirs() {
        for path in walk_files(&owned)? {
            let relative = path
                .strip_prefix(layout.root())
                .expect("file from owned_dirs lies under project root");
            snapshot.add_file(relative.to_path_buf(), &path.read_bytes()?);
            summary.files_captured += 1;
        }
    }

    // 2. Capture + remove managed blocks. Prefer the manifest (authoritative
    //    record of every block Ark installed); fall back to the default
    //    CLAUDE.md marker so a missing manifest never leaves orphaned state.
    for (file, marker) in managed_blocks(&layout)? {
        let target = layout.resolve(&file);
        if let Some(body) = read_managed_block(&target, &marker)? {
            snapshot.add_block(file, &marker, body);
            summary.blocks_captured += 1;
        }
        remove_managed_block(&target, &marker)?;
    }

    // 3. Persist the snapshot before destroying anything else.
    snapshot.write(layout.root())?;

    // 4. Delete the live Ark footprint.
    layout
        .owned_dirs()
        .iter()
        .try_for_each(|d| d.remove_dir_all().map(|_| ()))?;
    for parent in layout.prunable_empty_parents() {
        parent.remove_dir_if_empty()?;
    }

    // 5. Update .gitignore.
    summary.gitignore_updated = Snapshot::ensure_ignored(layout.root())?;

    Ok(summary)
}

/// Managed blocks to capture: recorded in the manifest if present, else the
/// canonical `CLAUDE.md` block as a fallback so a missing manifest never
/// leaves orphaned state.
fn managed_blocks(layout: &Layout) -> Result<Vec<(PathBuf, String)>> {
    Ok(match Manifest::read(layout.root())? {
        Some(manifest) => manifest
            .managed_blocks
            .into_iter()
            .map(|b| (b.file, b.marker))
            .collect(),
        None => vec![(CLAUDE_MD.into(), layout.managed_marker().into())],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::init::{InitOptions, init},
        state::SNAPSHOT_FILENAME,
    };

    #[test]
    fn unload_captures_and_removes() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        let summary = unload(UnloadOptions::new(tmp.path())).unwrap();

        assert!(summary.files_captured > 0);
        assert_eq!(summary.blocks_captured, 1);
        assert!(summary.gitignore_updated);

        assert!(!tmp.path().join(".ark").exists());
        assert!(!tmp.path().join(".claude/commands/ark").exists());
        assert!(tmp.path().join(SNAPSHOT_FILENAME).exists());
        assert!(tmp.path().join(".gitignore").exists());
    }

    #[test]
    fn unload_captures_user_files_too() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        let task = tmp.path().join(".ark/tasks/mine/PRD.md");
        std::fs::create_dir_all(task.parent().unwrap()).unwrap();
        std::fs::write(&task, "user content\n").unwrap();

        unload(UnloadOptions::new(tmp.path())).unwrap();

        let snap = Snapshot::read(tmp.path()).unwrap().unwrap();
        let file = snap
            .files
            .iter()
            .find(|f| f.path.ends_with("mine/PRD.md"))
            .unwrap();
        assert_eq!(file.decode().unwrap(), b"user content\n");
    }

    #[test]
    fn unload_errors_when_not_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        let err = unload(UnloadOptions::new(tmp.path())).unwrap_err();
        assert!(matches!(err, Error::NotLoaded { .. }));
    }

    #[test]
    fn unload_captures_and_removes_block_when_manifest_missing() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        // Delete the manifest to simulate a partially-tracked install.
        std::fs::remove_file(tmp.path().join(".ark/.installed.json")).unwrap();

        let summary = unload(UnloadOptions::new(tmp.path())).unwrap();
        assert_eq!(summary.blocks_captured, 1);

        // The managed block must be removed from CLAUDE.md (file deleted if it
        // was the only content, per remove_managed_block semantics).
        let claude = tmp.path().join("CLAUDE.md");
        if claude.exists() {
            let text = std::fs::read_to_string(&claude).unwrap();
            assert!(!text.contains("<!-- ARK:START -->"));
        }

        // And captured into the snapshot so load can restore it later.
        let snap = Snapshot::read(tmp.path()).unwrap().unwrap();
        assert!(
            snap.managed_blocks
                .iter()
                .any(|b| b.marker == "ARK" && b.file.ends_with("CLAUDE.md"))
        );
    }

    #[test]
    fn unload_captures_claude_commands_including_user_edits() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        std::fs::write(
            tmp.path().join(".claude/commands/ark/quick.md"),
            "# custom quick\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join(".claude/commands/ark/plan.md"),
            "# custom plan command\n",
        )
        .unwrap();

        unload(UnloadOptions::new(tmp.path())).unwrap();

        let snap = Snapshot::read(tmp.path()).unwrap().unwrap();
        let by = |suffix: &str| -> Vec<u8> {
            snap.files
                .iter()
                .find(|f| f.path.ends_with(suffix))
                .map(|f| f.decode().unwrap())
                .unwrap_or_default()
        };
        assert_eq!(by("commands/ark/quick.md"), b"# custom quick\n");
        assert_eq!(by("commands/ark/plan.md"), b"# custom plan command\n");
    }
}
