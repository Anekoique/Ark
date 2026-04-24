//! `ark init` — scaffold `.ark/` and Claude Code integration from templates.
//!
//! Writes the embedded templates into the host project, installs the managed
//! block in `CLAUDE.md`, and records every artifact in `.ark/.installed.json`
//! so later commands can clean up without touching user work.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use include_dir::Dir;

use crate::{
    error::Result,
    io::{PathExt, WriteMode, WriteOutcome, update_managed_block, write_file},
    layout::{CLAUDE_MD, EMPTY_DIRS, Layout, MANAGED_BLOCK_BODY},
    state::Manifest,
    templates::{ARK_TEMPLATES, CLAUDE_TEMPLATES, walk},
};

#[derive(Debug, Clone)]
pub struct InitOptions {
    pub project_root: PathBuf,
    pub mode: WriteMode,
}

impl InitOptions {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            mode: WriteMode::default(),
        }
    }

    pub fn with_mode(mut self, mode: WriteMode) -> Self {
        self.mode = mode;
        self
    }
}

/// Counts of per-file outcomes produced by `init`.
#[derive(Debug, Default, Clone, Copy)]
pub struct InitSummary {
    pub created: usize,
    pub unchanged: usize,
    pub skipped: usize,
    pub overwritten: usize,
}

impl InitSummary {
    pub fn total(&self) -> usize {
        self.created + self.unchanged + self.skipped + self.overwritten
    }

    fn record(&mut self, outcome: WriteOutcome) {
        match outcome {
            WriteOutcome::Created => self.created += 1,
            WriteOutcome::Unchanged => self.unchanged += 1,
            WriteOutcome::Skipped => self.skipped += 1,
            WriteOutcome::Overwritten => self.overwritten += 1,
        }
    }
}

impl fmt::Display for InitSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} file(s): {} created · {} unchanged · {} skipped · {} overwritten",
            self.total(),
            self.created,
            self.unchanged,
            self.skipped,
            self.overwritten,
        )?;
        if self.skipped > 0 {
            write!(
                f,
                "\nnote: {} existing file(s) preserved; pass --force to overwrite",
                self.skipped
            )?;
        }
        Ok(())
    }
}

/// Scaffold a fresh Ark installation into `opts.project_root`.
///
/// Safe to re-run: files that already match are left untouched. Files that
/// differ are skipped unless `opts.mode == WriteMode::Force`.
pub fn init(opts: InitOptions) -> Result<InitSummary> {
    let layout = Layout::new(&opts.project_root);
    let mut manifest = Manifest::new();
    let mut summary = InitSummary::default();

    for (tree, dest_root) in [
        (&ARK_TEMPLATES, layout.ark_dir()),
        (&CLAUDE_TEMPLATES, layout.claude_dir()),
    ] {
        extract(
            tree,
            &dest_root,
            layout.root(),
            opts.mode,
            &mut manifest,
            &mut summary,
        )?;
    }

    EMPTY_DIRS
        .iter()
        .try_for_each(|dir| layout.resolve(dir).ensure_dir())?;

    if update_managed_block(
        layout.claude_md(),
        layout.managed_marker(),
        MANAGED_BLOCK_BODY,
    )? {
        manifest.record_block(CLAUDE_MD, layout.managed_marker());
    }

    manifest.write(layout.root())?;
    Ok(summary)
}

fn extract(
    tree: &Dir<'_>,
    dest_root: &Path,
    project_root: &Path,
    mode: WriteMode,
    manifest: &mut Manifest,
    summary: &mut InitSummary,
) -> Result<()> {
    walk(tree).try_for_each(|entry| {
        let dest = dest_root.join(entry.relative_path);
        let outcome = write_file(&dest, entry.contents, mode)?;
        let relative = dest
            .strip_prefix(project_root)
            .expect("dest under project root");
        manifest.record_file_with_hash(relative, entry.contents);
        summary.record(outcome);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_writes_expected_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = init(InitOptions::new(tmp.path())).unwrap();

        for expected in [
            ".ark/workflow.md",
            ".ark/templates/PRD.md",
            ".ark/templates/PLAN.md",
            ".ark/templates/REVIEW.md",
            ".ark/templates/VERIFY.md",
            ".ark/templates/SPEC.md",
            ".ark/specs/INDEX.md",
            ".ark/specs/project/INDEX.md",
            ".ark/specs/features/INDEX.md",
            ".claude/commands/ark/quick.md",
            ".claude/commands/ark/design.md",
        ] {
            assert!(tmp.path().join(expected).is_file(), "missing: {expected}");
        }

        for dir in EMPTY_DIRS {
            assert!(tmp.path().join(dir).is_dir(), "missing dir: {dir}");
        }

        let claude_md = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.contains("<!-- ARK:START -->"));

        let manifest = std::fs::read_to_string(tmp.path().join(".ark/.installed.json")).unwrap();
        assert!(manifest.contains("\"files\""));
        assert!(manifest.contains("\"managed_blocks\""));

        assert!(summary.created > 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.overwritten, 0);
    }

    #[test]
    fn manifest_stores_project_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        let manifest = Manifest::read(tmp.path()).unwrap().unwrap();
        assert!(!manifest.files.is_empty());
        for file in &manifest.files {
            assert!(file.is_relative(), "expected relative path, got {file:?}");
        }
        assert!(
            manifest
                .files
                .iter()
                .any(|p| p.ends_with(".ark/workflow.md"))
        );
    }

    #[test]
    fn second_init_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        let summary = init(InitOptions::new(tmp.path())).unwrap();
        assert_eq!(summary.created, 0);
        assert_eq!(summary.overwritten, 0);
        assert!(summary.unchanged > 0);
    }

    #[test]
    fn skip_mode_preserves_user_edits() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit\n").unwrap();
        let summary = init(InitOptions::new(tmp.path()).with_mode(WriteMode::Skip)).unwrap();
        assert!(summary.skipped > 0);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit\n");
    }

    #[test]
    fn init_populates_manifest_hashes() {
        use crate::io::hash_bytes;

        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        let manifest = Manifest::read(tmp.path()).unwrap().unwrap();
        assert!(!manifest.hashes.is_empty());
        for file in &manifest.files {
            let on_disk = std::fs::read(tmp.path().join(file)).unwrap();
            let recorded = manifest
                .hash_for(file)
                .unwrap_or_else(|| panic!("no hash for {}", file.display()));
            assert_eq!(recorded, hash_bytes(&on_disk));
        }
    }

    #[test]
    fn force_mode_overwrites_user_edits() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit\n").unwrap();
        let summary = init(InitOptions::new(tmp.path()).with_mode(WriteMode::Force)).unwrap();
        assert!(summary.overwritten > 0);
        assert_ne!(std::fs::read_to_string(&target).unwrap(), "user edit\n");
    }
}
