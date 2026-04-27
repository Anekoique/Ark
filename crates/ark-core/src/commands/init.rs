//! `ark init` — scaffold `.ark/` and per-platform integrations from templates.
//!
//! Writes the embedded templates into the host project, installs each selected
//! platform's managed block (e.g. `CLAUDE.md`, `AGENTS.md`), records every
//! artifact in `.ark/.installed.json` so later commands can clean up without
//! touching user work, and re-applies each platform's `SessionStart` hook
//! entry (per ark-context C-17 / codex-support C-11).

use std::{
    fmt,
    path::{Path, PathBuf},
};

use include_dir::Dir;

use crate::{
    error::Result,
    io::{PathExt, WriteMode, WriteOutcome, merge_managed_blocks, write_file},
    layout::{ARK_DIR, EMPTY_DIRS, Layout},
    platforms::{PLATFORMS, Platform},
    state::Manifest,
    templates::{ARK_TEMPLATES, walk},
};

#[derive(Debug, Clone)]
pub struct InitOptions {
    pub project_root: PathBuf,
    pub mode: WriteMode,
    /// Platforms to install. Defaults to all `PLATFORMS`. Empty selections are
    /// rejected at the CLI layer (see `resolve_platforms`); the library
    /// honors whatever the caller passes (an empty set yields a Claude-and-
    /// Codex-free install with only `.ark/` artifacts).
    pub platforms: Vec<&'static Platform>,
}

impl InitOptions {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            mode: WriteMode::default(),
            platforms: PLATFORMS.to_vec(),
        }
    }

    pub fn with_mode(mut self, mode: WriteMode) -> Self {
        self.mode = mode;
        self
    }

    /// Override the default (all-platforms) selection. Useful for tests and
    /// for the CLI's per-flag opt-in behavior.
    pub fn with_platforms(mut self, platforms: Vec<&'static Platform>) -> Self {
        self.platforms = platforms;
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

/// Scaffold a fresh Ark installation into `opts.project_root`, or refresh an
/// existing one with an additional platform.
///
/// Safe to re-run: files that already match are left untouched. Files that
/// differ are skipped unless `opts.mode == WriteMode::Force`. Each platform
/// in `opts.platforms` contributes a template tree, an optional managed
/// block, and an optional `SessionStart` hook entry.
///
/// Additive on the manifest: if a manifest already exists, this call only
/// rewrites entries under the platform-neutral `.ark/` tree and under each
/// selected platform's `dest_dir`. Other-platform entries (e.g. Claude
/// artifacts when `opts.platforms = [Codex]`) are preserved. Per
/// codex-support G-14: `ark init --codex` on a Claude-installed project
/// adds Codex without forgetting Claude.
pub fn init(opts: InitOptions) -> Result<InitSummary> {
    let layout = Layout::new(&opts.project_root);
    let mut manifest = Manifest::read(layout.root())?.unwrap_or_default();
    let mut summary = InitSummary::default();

    drop_manifest_entries_under(&mut manifest, ARK_DIR);
    extract(
        &ARK_TEMPLATES,
        &layout.ark_dir(),
        &layout,
        opts.mode,
        &mut manifest,
        &mut summary,
    )?;

    for platform in &opts.platforms {
        drop_manifest_entries_under(&mut manifest, platform.dest_dir);
        let dest_root = layout.resolve(platform.dest_dir);
        extract(
            platform.templates,
            &dest_root,
            &layout,
            opts.mode,
            &mut manifest,
            &mut summary,
        )?;
        platform.apply_managed_state(&layout, &mut manifest)?;
    }

    EMPTY_DIRS
        .iter()
        .try_for_each(|dir| layout.resolve(dir).ensure_dir())?;
    manifest.write(layout.root())?;
    Ok(summary)
}

/// Drop every manifest entry whose path starts with `prefix`. Used before
/// re-extracting a tree so templates removed between versions don't linger
/// as ghost manifest rows.
fn drop_manifest_entries_under(manifest: &mut Manifest, prefix: &str) {
    let prefix_path = Path::new(prefix);
    let stale: Vec<PathBuf> = manifest
        .files
        .iter()
        .filter(|p| p.starts_with(prefix_path))
        .cloned()
        .collect();
    for path in stale {
        manifest.drop_file(&path);
    }
}

/// Extract every file in `tree` under `dest_root`, recording each into
/// `manifest` and counting outcomes into `summary`. The shared backbone of
/// `init` — used once for `ARK_TEMPLATES` and once per selected platform.
///
/// For every template that carries an `ARK:*` managed block, the on-disk
/// block body (if any) is spliced into the template before writing. This
/// keeps `init --force` from clobbering rows that `spec register` (or any
/// other managed-block writer) put into the live file.
fn extract(
    tree: &Dir<'_>,
    dest_root: &Path,
    layout: &Layout,
    mode: WriteMode,
    manifest: &mut Manifest,
    summary: &mut InitSummary,
) -> Result<()> {
    walk(tree).try_for_each(|entry| {
        let dest = dest_root.join(entry.relative_path);
        let contents = merge_managed_blocks(&dest, entry.contents)?;
        let outcome = write_file(&dest, &contents, mode)?;
        let relative = dest
            .strip_prefix(layout.root())
            .expect("dest under project root");
        // Only record into the manifest when the on-disk content is the one
        // we want — i.e. we wrote it, or it already matched. `Skipped` means
        // the user's pre-existing content differs from canonical and
        // `WriteMode::Skip` left it alone; recording would falsely claim
        // ownership of user content.
        if outcome != WriteOutcome::Skipped {
            manifest.record_file_with_hash(relative, &contents);
        }
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
            ".codex/skills/ark-quick/SKILL.md",
            ".codex/skills/ark-design/SKILL.md",
            ".codex/skills/ark-archive/SKILL.md",
            ".codex/config.toml",
            ".codex/hooks.json",
        ] {
            assert!(tmp.path().join(expected).is_file(), "missing: {expected}");
        }

        for dir in EMPTY_DIRS {
            assert!(tmp.path().join(dir).is_dir(), "missing dir: {dir}");
        }

        let claude_md = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.contains("<!-- ARK:START -->"));
        let agents_md = std::fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
        assert!(agents_md.contains("<!-- ARK:START -->"));

        let manifest = std::fs::read_to_string(tmp.path().join(".ark/.installed.json")).unwrap();
        assert!(manifest.contains("\"files\""));
        assert!(manifest.contains("\"managed_blocks\""));

        assert!(summary.created > 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.overwritten, 0);
    }

    /// V-IT-2 (codex-support G-3): `--no-codex` (i.e. `with_platforms(&[CLAUDE])`)
    /// installs only Claude artifacts. No `.codex/` dir, no `AGENTS.md`.
    #[test]
    fn init_claude_only_omits_codex_paths() {
        use crate::CLAUDE_PLATFORM;
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path()).with_platforms(vec![&CLAUDE_PLATFORM])).unwrap();

        assert!(tmp.path().join(".claude/commands/ark/quick.md").is_file());
        assert!(tmp.path().join("CLAUDE.md").is_file());
        assert!(!tmp.path().join(".codex").exists());
        assert!(!tmp.path().join("AGENTS.md").exists());
    }

    /// V-IT-3: symmetric — `--no-claude` installs only Codex artifacts.
    #[test]
    fn init_codex_only_omits_claude_paths() {
        use crate::CODEX_PLATFORM;
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path()).with_platforms(vec![&CODEX_PLATFORM])).unwrap();

        assert!(
            tmp.path()
                .join(".codex/skills/ark-quick/SKILL.md")
                .is_file()
        );
        assert!(tmp.path().join("AGENTS.md").is_file());
        assert!(!tmp.path().join(".claude").exists());
        assert!(!tmp.path().join("CLAUDE.md").exists());
    }

    /// codex-support C-18: source-scan invariant for `init.rs`. Mirrors
    /// `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`.
    #[test]
    fn init_source_no_bare_std_fs_or_dot_path_literals() {
        crate::commands::tests_common::assert_source_clean(include_str!("init.rs"));
    }

    /// Regression: `ark init --codex` on a project that already has Claude
    /// installed must keep Claude's manifest entries intact. Pre-fix, init
    /// rebuilt the manifest from scratch, so the second call dropped Claude's
    /// `.claude/commands/ark/*` rows even though the files survived on disk.
    #[test]
    fn second_init_with_subset_keeps_other_platform_in_manifest() {
        use crate::CODEX_PLATFORM;
        let tmp = tempfile::tempdir().unwrap();
        // First install: both platforms.
        init(InitOptions::new(tmp.path())).unwrap();
        let manifest_before = Manifest::read(tmp.path()).unwrap().unwrap();
        assert!(
            manifest_before
                .files
                .iter()
                .any(|p| p.starts_with(".claude")),
            "expected Claude entries after the default install",
        );

        // Second install: Codex only. Claude entries must survive.
        init(
            InitOptions::new(tmp.path())
                .with_mode(WriteMode::Force)
                .with_platforms(vec![&CODEX_PLATFORM]),
        )
        .unwrap();

        let manifest_after = Manifest::read(tmp.path()).unwrap().unwrap();
        assert!(
            manifest_after
                .files
                .iter()
                .any(|p| p.starts_with(".claude")),
            "Claude entries must persist when a later init selects only Codex; got {:?}",
            manifest_after.files,
        );
        assert!(
            manifest_after.files.iter().any(|p| p.starts_with(".codex")),
            "Codex entries must be present after init --codex",
        );
    }

    /// Regression: `init --force` must NOT clobber managed-block bodies that
    /// other commands (e.g. `spec register`) wrote into the live file.
    /// Pre-fix, re-running `init` after registering features wiped the rows.
    #[test]
    fn init_force_preserves_existing_managed_block_rows() {
        use crate::commands::agent::spec::{SpecRegisterOptions, spec_register};

        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature: "alpha".into(),
            scope: "first".into(),
            from_task: "alpha".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        })
        .unwrap();

        // Re-run init in --force mode: the previously registered row must
        // still be present in the INDEX after the rewrite.
        init(InitOptions::new(tmp.path()).with_mode(WriteMode::Force)).unwrap();
        let index =
            std::fs::read_to_string(tmp.path().join(".ark/specs/features/INDEX.md")).unwrap();
        assert!(
            index.contains("`alpha`"),
            "init --force wiped the registered feature row:\n{index}"
        );
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
