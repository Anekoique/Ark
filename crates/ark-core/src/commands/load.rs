//! `ark load` — bring Ark into a project.
//!
//! - Snapshot present → restore every captured file and block, delete the
//!   snapshot, and strip its `.gitignore` entry.
//! - No snapshot → scaffold from embedded templates (behaves like `init`).
//! - `.ark/` already present → error unless `force = true` (then wipe first).

use std::{fmt, path::PathBuf};

use crate::{
    commands::init::{InitOptions, InitSummary, init},
    error::{Error, Result},
    io::{PathExt, WriteMode, update_managed_block, write_file},
    layout::Layout,
    state::Snapshot,
};

#[derive(Debug, Clone)]
pub struct LoadOptions {
    pub project_root: PathBuf,
    pub force: bool,
}

impl LoadOptions {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            force: false,
        }
    }

    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

/// Outcome of `load`. Each variant carries its own relevant counters.
#[derive(Debug, Clone, Copy)]
pub enum LoadSummary {
    /// Fresh scaffold from embedded templates (no snapshot was present).
    Fresh(InitSummary),
    /// Restored from a pre-existing `.ark.db` snapshot.
    Restored { files: usize, blocks: usize },
}

impl fmt::Display for LoadSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fresh(init) => write!(f, "scaffolded from templates\n{init}"),
            Self::Restored { files, blocks } => write!(
                f,
                "restored from snapshot: {files} file(s), {blocks} managed block(s)",
            ),
        }
    }
}

/// Load Ark into `opts.project_root`.
pub fn load(opts: LoadOptions) -> Result<LoadSummary> {
    let layout = Layout::new(&opts.project_root);
    let ark_dir = layout.ark_dir();

    if ark_dir.exists() {
        if !opts.force {
            return Err(Error::AlreadyLoaded { path: ark_dir });
        }
        // --force: wipe the live footprint so either path below writes cleanly.
        layout
            .owned_dirs()
            .iter()
            .try_for_each(|d| d.remove_dir_all().map(|_| ()))?;
    }

    match Snapshot::read(layout.root())? {
        Some(snapshot) => restore(&layout, snapshot),
        None => fresh(&layout),
    }
}

fn fresh(layout: &Layout) -> Result<LoadSummary> {
    init(InitOptions::new(layout.root()).with_mode(WriteMode::Force)).map(LoadSummary::Fresh)
}

fn restore(layout: &Layout, snapshot: Snapshot) -> Result<LoadSummary> {
    snapshot.files.iter().try_for_each(|f| {
        let target = layout.resolve_safe(&f.path)?;
        write_file(target, &f.decode()?, WriteMode::Force).map(|_| ())
    })?;
    snapshot.managed_blocks.iter().try_for_each(|b| {
        let target = layout.resolve_safe(&b.file)?;
        update_managed_block(target, &b.marker, &b.body).map(|_| ())
    })?;

    Snapshot::remove(layout.root())?;
    Snapshot::remove_ignored(layout.root())?;

    Ok(LoadSummary::Restored {
        files: snapshot.files.len(),
        blocks: snapshot.managed_blocks.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::unload::{UnloadOptions, unload},
        state::SNAPSHOT_FILENAME,
    };

    #[test]
    fn first_load_scaffolds_from_templates() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = load(LoadOptions::new(tmp.path())).unwrap();
        assert!(matches!(summary, LoadSummary::Fresh(_)));
        assert!(tmp.path().join(".ark/workflow.md").is_file());
    }

    #[test]
    fn load_restores_from_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();

        let user_file = tmp.path().join(".ark/tasks/mine/PRD.md");
        std::fs::create_dir_all(user_file.parent().unwrap()).unwrap();
        std::fs::write(&user_file, "user work\n").unwrap();
        unload(UnloadOptions::new(tmp.path())).unwrap();
        assert!(!tmp.path().join(".ark").exists());
        assert!(tmp.path().join(SNAPSHOT_FILENAME).exists());

        let summary = load(LoadOptions::new(tmp.path())).unwrap();
        assert!(matches!(summary, LoadSummary::Restored { .. }));
        assert!(tmp.path().join(".ark/workflow.md").is_file());
        assert_eq!(std::fs::read_to_string(&user_file).unwrap(), "user work\n");
        assert!(!tmp.path().join(SNAPSHOT_FILENAME).exists());

        let gi = tmp.path().join(".gitignore");
        if gi.exists() {
            assert!(!std::fs::read_to_string(&gi).unwrap().contains(".ark.db"));
        }

        let claude = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
        assert!(claude.contains("<!-- ARK:START -->"));
    }

    #[test]
    fn load_errors_when_already_loaded() {
        let tmp = tempfile::tempdir().unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();
        let err = load(LoadOptions::new(tmp.path())).unwrap_err();
        assert!(matches!(err, Error::AlreadyLoaded { .. }));
    }

    #[test]
    fn load_force_replaces_existing() {
        let tmp = tempfile::tempdir().unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();
        let workflow = tmp.path().join(".ark/workflow.md");
        std::fs::write(&workflow, "mangled\n").unwrap();

        let summary = load(LoadOptions::new(tmp.path()).with_force(true)).unwrap();
        assert!(matches!(summary, LoadSummary::Fresh(_)));
        assert_ne!(std::fs::read_to_string(&workflow).unwrap(), "mangled\n");
    }

    #[test]
    fn load_rejects_snapshot_with_absolute_file_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mut snap = Snapshot::new();
        snap.add_file("/tmp/ark-pwned", b"bad");
        snap.write(tmp.path()).unwrap();

        let err = load(LoadOptions::new(tmp.path())).unwrap_err();
        assert!(matches!(err, Error::UnsafeSnapshotPath { .. }));
        assert!(!PathBuf::from("/tmp/ark-pwned").exists());
    }

    #[test]
    fn load_rejects_snapshot_with_parent_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let mut snap = Snapshot::new();
        snap.add_file("../escaped.txt", b"bad");
        snap.write(tmp.path()).unwrap();

        let err = load(LoadOptions::new(tmp.path())).unwrap_err();
        assert!(matches!(err, Error::UnsafeSnapshotPath { .. }));
        assert!(!tmp.path().parent().unwrap().join("escaped.txt").exists());
    }

    #[test]
    fn load_rejects_snapshot_with_unsafe_managed_block_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mut snap = Snapshot::new();
        snap.add_block("/etc/hosts", "ARK", "pwn");
        snap.write(tmp.path()).unwrap();

        let err = load(LoadOptions::new(tmp.path())).unwrap_err();
        assert!(matches!(err, Error::UnsafeSnapshotPath { .. }));
    }

    #[test]
    fn roundtrip_preserves_edited_and_added_claude_commands() {
        let tmp = tempfile::tempdir().unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();

        let quick = tmp.path().join(".claude/commands/ark/quick.md");
        std::fs::write(&quick, "# edited quick\n").unwrap();
        let custom = tmp.path().join(".claude/commands/ark/plan.md");
        std::fs::write(&custom, "# user plan\n").unwrap();

        unload(UnloadOptions::new(tmp.path())).unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();

        assert_eq!(std::fs::read_to_string(&quick).unwrap(), "# edited quick\n");
        assert_eq!(std::fs::read_to_string(&custom).unwrap(), "# user plan\n");
    }
}
