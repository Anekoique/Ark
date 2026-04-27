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
    io::{
        PathExt, WriteMode, WriteOutcome, ark_session_start_hook_entry, update_managed_block,
        update_settings_hook, write_file,
    },
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

    // ark-context C-17: re-apply the SessionStart hook unconditionally.
    // Not hash-tracked; manifest is unchanged.
    update_settings_hook(layout.claude_settings(), ark_session_start_hook_entry())?;

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

    /// V-UT-29 (carve-out): `init(target)` operates exclusively on `target`,
    /// regardless of any Arked ancestor. The CLI ensures the wrong target
    /// can never be picked via discovery; the library is consistent with
    /// that — passing a child of an Arked parent scaffolds in the child.
    #[test]
    fn init_in_subdir_of_arked_parent_scaffolds_in_subdir() {
        let parent = tempfile::tempdir().unwrap();
        init(InitOptions::new(parent.path())).unwrap();
        assert!(parent.path().join(".ark").is_dir());

        let sub = parent.path().join("nested").join("project");
        std::fs::create_dir_all(&sub).unwrap();
        init(InitOptions::new(&sub)).unwrap();
        assert!(
            sub.join(".ark").is_dir(),
            "subdir should have its own .ark/"
        );
        // Parent's .ark/ is untouched (still has its own workflow.md).
        assert!(parent.path().join(".ark/workflow.md").is_file());
    }

    /// V-IT-7: `ark init` writes the `SessionStart` hook entry in
    /// `.claude/settings.json` per ark-context G-8 / G-11.
    #[test]
    fn init_writes_session_start_hook() {
        use crate::io::ARK_CONTEXT_HOOK_COMMAND;
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        let settings = tmp.path().join(".claude/settings.json");
        assert!(settings.is_file());
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            v["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            serde_json::Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()),
        );
        assert_eq!(v["hooks"]["SessionStart"][0]["hooks"][0]["timeout"], 5000);
    }

    /// V-IT-13: `ark init` followed by deleting the Ark entry → next `init`
    /// re-adds it. (Idempotent re-application is also covered by C-29 at
    /// the upgrade layer.) This verifies the same invariant for `init`.
    #[test]
    fn init_re_adds_deleted_session_start_hook() {
        use crate::io::ARK_CONTEXT_HOOK_COMMAND;
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        let settings = tmp.path().join(".claude/settings.json");

        // User deletes the entry by emptying the array.
        std::fs::write(
            &settings,
            serde_json::to_string_pretty(&serde_json::json!({
                "hooks": {"SessionStart": []}
            }))
            .unwrap()
                + "\n",
        )
        .unwrap();

        init(InitOptions::new(tmp.path()).with_mode(WriteMode::Skip)).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            v["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            serde_json::Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()),
        );
    }
}
