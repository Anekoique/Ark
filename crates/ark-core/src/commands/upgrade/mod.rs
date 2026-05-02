//! `ark upgrade` — refresh embedded templates in an initialized project.
//!
//! Re-applies the CLI's current embedded template set to a project that was
//! previously initialized with `ark init` or a prior `ark upgrade`. User-
//! modified files are detected by SHA-256 content hashing (recorded in the
//! installation manifest at write time) and handled via a [`ConflictPolicy`]
//! or an injected [`Prompter`].
//!
//! Migrations (renames / deletes across versions) are deferred to a later
//! task; this command only refreshes template content in place.

use std::{
    borrow::Cow,
    fmt,
    path::{Path, PathBuf},
};

use chrono::Utc;

use crate::{
    error::{Error, Result},
    io::{PathExt, merge_managed_blocks},
    layout::Layout,
    platforms::{self, PLATFORMS},
    state::{Manifest, manifest::MANIFEST_RELATIVE_PATH},
    templates::{ARK_TEMPLATES, walk},
};

mod plan;
mod verify_migration;

use plan::{PlannedAction, WriteKind, plan_actions, validate_manifest_paths};
use verify_migration::migrate_in_flight_verify_files;

/// Selects conflict behavior for user-modified templates.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// Ask the caller's [`Prompter`] per file.
    #[default]
    Interactive,
    /// Always overwrite.
    Force,
    /// Always preserve the user's file.
    Skip,
    /// Always write the new content to `<path>.new` next to the user's file.
    CreateNew,
}

/// Selects a concrete action for one modified template conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    /// Replace the user-modified file with the embedded template.
    Overwrite,
    /// Preserve the user-modified file unchanged.
    Skip,
    /// Write the embedded template to a `.new` sidecar.
    CreateNew,
}

/// Prompts for each user-modified file in interactive mode.
///
/// The library never reads stdin itself.
pub trait Prompter {
    /// Prompts for how to handle one user-modified relative path.
    fn prompt(&mut self, relative_path: &Path) -> Result<ConflictChoice>;
}

/// Options for refreshing an Ark installation to the current CLI version.
#[derive(Debug, Clone)]
pub struct UpgradeOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Conflict handling policy for modified template files.
    pub conflict_policy: ConflictPolicy,
    /// Reports whether older CLI templates may replace a newer install.
    pub allow_downgrade: bool,
}

impl UpgradeOptions {
    /// Creates upgrade options for `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            conflict_policy: ConflictPolicy::default(),
            allow_downgrade: false,
        }
    }

    /// Sets conflict handling behavior.
    pub fn with_policy(mut self, policy: ConflictPolicy) -> Self {
        self.conflict_policy = policy;
        self
    }

    /// Sets whether downgrades are allowed.
    pub fn with_allow_downgrade(mut self, allow: bool) -> Self {
        self.allow_downgrade = allow;
        self
    }
}

/// Per-outcome counters produced by [`upgrade`].
#[derive(Debug, Default, Clone)]
pub struct UpgradeSummary {
    /// Number of newly added template files.
    pub added: usize,
    /// Number of template files updated automatically.
    pub updated: usize,
    /// Number of template files already up to date.
    pub unchanged: usize,
    /// Number of modified files preserved.
    pub modified_preserved: usize,
    /// Number of modified files overwritten.
    pub overwritten: usize,
    /// Number of `.new` sidecars written.
    pub created_new: usize,
    /// Number of removed template files deleted.
    pub deleted: usize,
    /// Number of removed template files left in place.
    pub orphaned: usize,
    /// Number of in-flight `VERIFY.md` files migrated from the legacy
    /// verdict-driven shape to the new living-checklist shape.
    pub verify_migrated: usize,
    /// Version recorded before upgrade.
    pub version_from: String,
    /// CLI version applied by upgrade.
    pub version_to: String,
}

impl UpgradeSummary {
    fn segments(&self) -> [(&'static str, usize); 9] {
        [
            ("added", self.added),
            ("updated", self.updated),
            ("unchanged", self.unchanged),
            ("modified-preserved", self.modified_preserved),
            ("overwritten", self.overwritten),
            (".new-copied", self.created_new),
            ("deleted", self.deleted),
            ("orphaned", self.orphaned),
            ("verify-migrated", self.verify_migrated),
        ]
    }

    fn total(&self) -> usize {
        self.segments().iter().map(|(_, n)| n).sum()
    }
}

impl fmt::Display for UpgradeSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} file(s): ", self.total())?;
        for (i, (label, n)) in self.segments().iter().enumerate() {
            if i > 0 {
                write!(f, " · ")?;
            }
            write!(f, "{n} {label}")?;
        }
        write!(f, "\n{} -> {}", self.version_from, self.version_to)
    }
}

/// Walks the embedded template trees and produces project-relative keys.
///
/// This mirrors `init.rs`'s extraction shape so the keys are byte-equal to
/// what `manifest.files` stores.
///
/// Only platforms whose `dest_dir` already appears in `manifest.files` are
/// included. A Claude-only project upgraded by a CLI that knows about Codex
/// stays Claude-only. To opt in, the user re-runs `ark init --codex`.
#[cfg(test)]
pub(super) fn collect_desired_for_test(
    layout: &Layout,
    manifest: &Manifest,
) -> Vec<(PathBuf, Cow<'static, [u8]>)> {
    collect_desired_templates(layout, manifest)
}

fn collect_desired_templates(
    layout: &Layout,
    manifest: &Manifest,
) -> Vec<(PathBuf, Cow<'static, [u8]>)> {
    let trees = std::iter::once((&ARK_TEMPLATES, layout.ark_dir()))
        .chain(platforms::installed(manifest).map(|p| (p.templates, layout.resolve(p.dest_dir))));
    trees
        .flat_map(|(tree, dest_root)| {
            walk(tree).map(move |entry| {
                let absolute = dest_root.join(entry.relative_path);
                let relative = absolute
                    .strip_prefix(layout.root())
                    .expect("template dest under project root")
                    .to_path_buf();
                (relative, Cow::Borrowed(entry.contents))
            })
        })
        .collect()
}

/// Splices on-disk managed-block bodies into desired templates.
///
/// Without this step, upgrade would hash-classify divergent managed blocks as
/// user-modified and prompt to overwrite.
///
/// Delegates to [`merge_managed_blocks`]; the loop is the only upgrade-side
/// logic.
fn reconcile_managed_blocks(
    layout: &Layout,
    desired: &mut [(PathBuf, Cow<'static, [u8]>)],
) -> Result<()> {
    for (relative, contents) in desired.iter_mut() {
        let merged = merge_managed_blocks(layout.resolve(relative), contents)?;
        if merged.as_slice() != contents.as_ref() {
            *contents = Cow::Owned(merged);
        }
    }
    Ok(())
}

fn check_version(manifest_version: &str, cli_version: &str, allow_downgrade: bool) -> Result<()> {
    let (Ok(project), Ok(cli)) = (
        semver::Version::parse(manifest_version),
        semver::Version::parse(cli_version),
    ) else {
        return Ok(());
    };
    if project > cli && !allow_downgrade {
        return Err(Error::DowngradeRefused {
            project_version: manifest_version.to_string(),
            cli_version: cli_version.to_string(),
        });
    }
    Ok(())
}

/// Re-apply the embedded template set to `opts.project_root`.
pub fn upgrade(opts: UpgradeOptions, prompter: &mut dyn Prompter) -> Result<UpgradeSummary> {
    let layout = Layout::new(&opts.project_root);
    let manifest_path = layout.resolve(MANIFEST_RELATIVE_PATH);

    let Some(mut manifest) = Manifest::read(layout.root())? else {
        return Err(Error::NotLoaded {
            path: manifest_path,
        });
    };

    let version_from = manifest.version.clone();
    let cli_version = env!("CARGO_PKG_VERSION").to_string();

    // Path safety runs before any semantic check.
    validate_manifest_paths(&layout, &manifest.files)?;
    check_version(&manifest.version, &cli_version, opts.allow_downgrade)?;

    let mut desired = collect_desired_templates(&layout, &manifest);
    // Desired paths come from `include_dir!` joined under
    // `layout.ark_dir()` / `layout.claude_dir()`, so they are safe by
    // construction; a unit test asserts parity against `init.rs::extract`.
    // No runtime check needed here.

    // Splice on-disk managed-block bodies into the desired bytes so blocks
    // written by other commands (e.g. `spec register`) are not flagged as
    // user modifications.
    reconcile_managed_blocks(&layout, &mut desired)?;

    let plan = plan_actions(&layout, &manifest, &desired, opts.conflict_policy, prompter)?;

    let mut summary = UpgradeSummary {
        version_from,
        version_to: cli_version.clone(),
        unchanged: plan.inline_unchanged,
        ..Default::default()
    };

    // apply_writes phase: Add, AutoUpdate, Overwrite, CreateNew, RefreshHashOnly, Preserve.
    // Deletions are deferred until after the manifest is flushed.
    let mut deferred: Vec<PlannedAction> = Vec::new();
    for action in plan.actions {
        match action {
            PlannedAction::Write {
                relative,
                contents,
                kind,
            } => {
                let absolute = layout.resolve(&relative);
                absolute.write_bytes(&contents)?;
                manifest.record_file_with_hash(&relative, &contents);
                match kind {
                    WriteKind::Add => summary.added += 1,
                    WriteKind::AutoUpdate => summary.updated += 1,
                    WriteKind::Overwrite => summary.overwritten += 1,
                }
            }
            PlannedAction::RefreshHashOnly { relative, contents } => {
                manifest.record_file_with_hash(&relative, &contents);
            }
            PlannedAction::CreateNew { relative, contents } => {
                let mut new_path = layout.resolve(&relative);
                let mut file_name = new_path
                    .file_name()
                    .expect("relative has file name")
                    .to_os_string();
                file_name.push(".new");
                new_path.set_file_name(file_name);
                new_path.write_bytes(&contents)?;
                summary.created_new += 1;
            }
            PlannedAction::Preserve { .. } => {
                summary.modified_preserved += 1;
            }
            action @ (PlannedAction::Delete { .. } | PlannedAction::DropManifestEntry { .. }) => {
                deferred.push(action);
            }
        }
    }

    // Per-platform managed block + SessionStart hook + extra files — re-
    // applied on every upgrade, not hash-tracked. Only platforms already in
    // the manifest are touched (Claude-only stays Claude-only).
    for platform in PLATFORMS {
        if platform.is_installed(&manifest) {
            platform.apply_managed_state(&layout, &mut manifest)?;
        }
    }

    // Durable manifest write BEFORE any delete can fail.
    manifest.version = cli_version;
    manifest.installed_at = Utc::now();
    manifest.write(layout.root())?;

    let mut manifest_mutated = false;
    for action in deferred {
        match action {
            PlannedAction::Delete { relative } => {
                let absolute = layout.resolve(&relative);
                absolute.remove_if_exists()?;
                manifest.drop_file(&relative);
                summary.deleted += 1;
                manifest_mutated = true;
            }
            PlannedAction::DropManifestEntry { relative } => {
                let absolute = layout.resolve(&relative);
                if absolute.exists() {
                    summary.orphaned += 1;
                }
                manifest.drop_file(&relative);
                manifest_mutated = true;
            }
            _ => unreachable!("only deletions are deferred"),
        }
    }

    if manifest_mutated {
        manifest.write(layout.root())?;
    }

    // In-flight task migration: any `phase ∈ {Verify, Committed}` task whose
    // `VERIFY.md` still carries the legacy `## Verdict` heading is rewritten
    // with the new living-checklist shape. Errors per-task are logged but do
    // not abort the upgrade — the template refresh above is more important.
    summary.verify_migrated = migrate_in_flight_verify_files(&layout);

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::io::{ARK_CONTEXT_HOOK_COMMAND, hash_bytes};

    struct PanicPrompter;
    impl Prompter for PanicPrompter {
        fn prompt(&mut self, _: &Path) -> Result<ConflictChoice> {
            panic!("prompter invoked unexpectedly");
        }
    }

    fn layout_for(tmp: &tempfile::TempDir) -> Layout {
        Layout::new(tmp.path())
    }

    #[test]
    fn summary_display_prints_fixed_order_even_when_zero() {
        let s = UpgradeSummary {
            version_from: "0.1.0".into(),
            version_to: "0.2.0".into(),
            ..Default::default()
        };
        let shown = format!("{s}");
        assert!(shown.contains("0 added"));
        assert!(shown.contains("0 orphaned"));
        assert!(shown.contains("0.1.0 -> 0.2.0"));
    }

    #[test]
    fn check_version_passes_on_equal() {
        assert!(check_version("0.1.1", "0.1.1", false).is_ok());
    }

    #[test]
    fn check_version_refuses_downgrade() {
        assert!(matches!(
            check_version("1.0.0", "0.9.0", false),
            Err(Error::DowngradeRefused { .. })
        ));
    }

    #[test]
    fn check_version_allows_downgrade_with_flag() {
        assert!(check_version("1.0.0", "0.9.0", true).is_ok());
    }

    #[test]
    fn check_version_passes_on_non_semver() {
        assert!(check_version("dev", "0.1.0", false).is_ok());
    }

    #[test]
    fn desired_template_keys_match_init_manifest_entries() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let manifest = Manifest::read(tmp.path()).unwrap().unwrap();
        let layout = layout_for(&tmp);
        let desired: std::collections::BTreeSet<_> = collect_desired_templates(&layout, &manifest)
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        let from_manifest: std::collections::BTreeSet<_> = manifest.files.into_iter().collect();
        assert_eq!(desired, from_manifest);
    }

    #[test]
    fn upgrade_source_has_no_bare_std_fs_or_dot_ark_literals() {
        // Enforces "no bare `std::fs::*`" and "no `.ark/` literal path
        // composition" at compile time. Scans both upgrade module files,
        // excluding the tests module itself and `//` comments.
        let sources = [
            ("mod.rs", include_str!("mod.rs")),
            ("plan.rs", include_str!("plan.rs")),
        ];
        for (name, source) in sources {
            let mut in_tests = false;
            for (idx, line) in source.lines().enumerate() {
                if line.contains("#[cfg(test)]") {
                    in_tests = true;
                }
                if in_tests {
                    continue;
                }
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("//!") {
                    continue;
                }
                let code = trimmed.split("//").next().unwrap_or(trimmed);
                assert!(
                    !code.contains("std::fs::"),
                    "{name} line {} contains bare std::fs::: {line}",
                    idx + 1
                );
                assert!(
                    !code.contains("\".ark/"),
                    "{name} line {} contains hand-joined .ark/ literal: {line}",
                    idx + 1
                );
                assert!(
                    !code.contains("\".claude/"),
                    "{name} line {} contains hand-joined .claude/ literal: {line}",
                    idx + 1
                );
            }
        }
    }

    #[test]
    fn upgrade_is_noop_right_after_init() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let mut prompter = PanicPrompter;
        let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        assert_eq!(summary.added, 0);
        assert_eq!(summary.updated, 0);
        assert_eq!(summary.overwritten, 0);
        assert_eq!(summary.created_new, 0);
        assert_eq!(summary.deleted, 0);
        assert_eq!(summary.orphaned, 0);
        assert!(summary.unchanged > 0);
    }

    /// Verifies that repeated `ark upgrade` leaves settings unchanged.
    #[test]
    fn upgrade_settings_hook_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let settings = tmp.path().join(".claude/settings.json");
        let after_init = std::fs::read(&settings).unwrap();

        let mut prompter = PanicPrompter;
        upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        let after_first = std::fs::read(&settings).unwrap();
        upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        let after_second = std::fs::read(&settings).unwrap();

        assert_eq!(after_init, after_first, "init→upgrade drifted");
        assert_eq!(after_first, after_second, "upgrade→upgrade drifted");
    }

    /// Verifies that `ark upgrade` re-adds a deleted Ark hook entry.
    #[test]
    fn upgrade_re_adds_deleted_session_start_hook() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let settings = tmp.path().join(".claude/settings.json");
        std::fs::write(
            &settings,
            serde_json::to_string_pretty(&serde_json::json!({
                "hooks": {"SessionStart": []}
            }))
            .unwrap()
                + "\n",
        )
        .unwrap();

        let mut prompter = PanicPrompter;
        upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            v["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            serde_json::Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()),
        );
    }

    #[test]
    fn upgrade_errors_when_not_initialized() {
        let tmp = tempfile::tempdir().unwrap();
        let mut prompter = PanicPrompter;
        let err = upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap_err();
        assert!(matches!(err, Error::NotLoaded { .. }));
    }

    #[test]
    fn upgrade_refuses_downgrade_without_flag() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.version = "99.0.0".into();
        m.write(tmp.path()).unwrap();
        let mut prompter = PanicPrompter;
        let err = upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap_err();
        assert!(matches!(err, Error::DowngradeRefused { .. }));
    }

    #[test]
    fn upgrade_allows_downgrade_with_flag() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.version = "99.0.0".into();
        m.write(tmp.path()).unwrap();
        let mut prompter = PanicPrompter;
        let summary = upgrade(
            UpgradeOptions::new(tmp.path()).with_allow_downgrade(true),
            &mut prompter,
        )
        .unwrap();
        assert_eq!(summary.version_from, "99.0.0");
    }

    #[test]
    fn upgrade_rejects_manifest_with_unsafe_path() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.files.push(PathBuf::from("../escape.md"));
        m.write(tmp.path()).unwrap();
        let mut prompter = PanicPrompter;
        let err = upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap_err();
        assert!(matches!(err, Error::UnsafeManifestPath { .. }));
    }

    #[test]
    fn upgrade_backfills_hashes_when_manifest_has_none() {
        use super::plan::is_exempted;
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.hashes = BTreeMap::new();
        m.write(tmp.path()).unwrap();
        let mut prompter = PanicPrompter;
        upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        let after = Manifest::read(tmp.path()).unwrap().unwrap();
        // Hashes are refreshed for every tracked file *except* seed-only paths
        // (config.toml, project specs) which upgrade intentionally ignores.
        let trackable = after.files.iter().filter(|p| !is_exempted(p)).count();
        assert_eq!(after.hashes.len(), trackable);
    }

    #[test]
    fn upgrade_force_overwrites_user_modified_file() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit").unwrap();
        let mut prompter = PanicPrompter;
        let summary = upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Force),
            &mut prompter,
        )
        .unwrap();
        assert_eq!(summary.overwritten, 1);
        assert_ne!(std::fs::read_to_string(&target).unwrap(), "user edit");
    }

    #[test]
    fn upgrade_skip_preserves_user_modified_file() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit").unwrap();
        let mut prompter = PanicPrompter;
        let summary = upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Skip),
            &mut prompter,
        )
        .unwrap();
        assert_eq!(summary.modified_preserved, 1);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
    }

    /// Default (Interactive) policy must not prompt for user-owned seed-only
    /// paths even when the embedded template's bytes have drifted from what
    /// the user has on disk.
    #[test]
    fn upgrade_does_not_prompt_for_seed_only_paths_under_interactive_policy() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let project_index = tmp.path().join(".ark/specs/project/INDEX.md");
        let config_toml = tmp.path().join(".ark/config.toml");
        let user_index =
            "# Project Specs\n\n| Spec | Scope |\n| --- | --- |\n| `LAYOUT.md` | mine |\n";
        let user_config = "[worktree]\nbranch_prefix = \"mine\"\n";
        std::fs::write(&project_index, user_index).unwrap();
        std::fs::write(&config_toml, user_config).unwrap();
        let mut prompter = PanicPrompter;
        upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        assert_eq!(std::fs::read_to_string(&project_index).unwrap(), user_index);
        assert_eq!(std::fs::read_to_string(&config_toml).unwrap(), user_config);
    }

    /// Verifies that `.ark/config.toml` is preserved across upgrade.
    #[test]
    fn upgrade_does_not_overwrite_config_toml() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/config.toml");
        assert!(target.exists(), "init should ship config.toml");
        std::fs::write(
            &target,
            "[worktree]\nbranch_prefix = \"fix\"\ncopy = \
             [\".env\"]\n\n[workspace]\njournal_max_lines = 500\n",
        )
        .unwrap();
        let mut prompter = PanicPrompter;
        upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Skip),
            &mut prompter,
        )
        .unwrap();
        let after = std::fs::read_to_string(&target).unwrap();
        assert!(after.contains("branch_prefix = \"fix\""));
        assert!(after.contains(".env"));
        assert!(after.contains("journal_max_lines = 500"));
    }

    #[test]
    fn upgrade_create_new_writes_dot_new_sidecar() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit").unwrap();
        let mut prompter = PanicPrompter;
        let summary = upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::CreateNew),
            &mut prompter,
        )
        .unwrap();
        assert_eq!(summary.created_new, 1);
        assert!(tmp.path().join(".ark/workflow.md.new").exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
        let manifest = Manifest::read(tmp.path()).unwrap().unwrap();
        assert!(
            !manifest
                .files
                .contains(&PathBuf::from(".ark/workflow.md.new")),
            ".new file must not be tracked"
        );
    }

    #[test]
    fn upgrade_deletes_removed_template_when_hash_matches() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let ghost = tmp.path().join(".ark/ghost.md");
        std::fs::write(&ghost, b"ghost content").unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.record_file_with_hash(PathBuf::from(".ark/ghost.md"), b"ghost content");
        m.write(tmp.path()).unwrap();
        let mut prompter = PanicPrompter;
        let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        assert_eq!(summary.deleted, 1);
        assert!(!ghost.exists());
        let after = Manifest::read(tmp.path()).unwrap().unwrap();
        assert!(!after.files.contains(&PathBuf::from(".ark/ghost.md")));
    }

    #[test]
    fn upgrade_leaves_orphaned_file_when_hash_mismatches() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let ghost = tmp.path().join(".ark/ghost.md");
        std::fs::write(&ghost, b"user edited ghost").unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.record_file_with_hash(PathBuf::from(".ark/ghost.md"), b"original ghost");
        m.write(tmp.path()).unwrap();
        let mut prompter = PanicPrompter;
        let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        assert_eq!(summary.orphaned, 1);
        assert!(ghost.exists());
        assert_eq!(
            std::fs::read_to_string(&ghost).unwrap(),
            "user edited ghost"
        );
    }

    #[test]
    fn upgrade_refreshes_stale_hash_when_content_matches_desired() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let relative = PathBuf::from(".ark/workflow.md");
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.hashes
            .insert(relative.clone(), "stale_hash_value".to_string());
        m.write(tmp.path()).unwrap();
        let mut prompter = PanicPrompter;
        upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        let after = Manifest::read(tmp.path()).unwrap().unwrap();
        let on_disk = std::fs::read(tmp.path().join(&relative)).unwrap();
        assert_eq!(
            after.hash_for(&relative),
            Some(hash_bytes(&on_disk).as_str())
        );
    }

    struct StubPrompter(ConflictChoice);
    impl Prompter for StubPrompter {
        fn prompt(&mut self, _: &Path) -> Result<ConflictChoice> {
            Ok(self.0)
        }
    }

    #[test]
    fn upgrade_interactive_prompts_for_ambiguous_no_hash() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit").unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.hashes = BTreeMap::new();
        m.write(tmp.path()).unwrap();
        let mut prompter = StubPrompter(ConflictChoice::Skip);
        let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        assert_eq!(summary.modified_preserved, 1);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
    }

    /// Verifies that a Claude-only project remains Claude-only after upgrade.
    #[test]
    fn upgrade_on_claude_only_project_does_not_install_codex() {
        use crate::CLAUDE_PLATFORM;
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(
            crate::commands::InitOptions::new(tmp.path()).with_platforms(vec![&CLAUDE_PLATFORM]),
        )
        .unwrap();

        let mut prompter = PanicPrompter;
        upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();

        assert!(
            !tmp.path().join(".codex").exists(),
            ".codex must not appear"
        );
        assert!(
            !tmp.path().join("AGENTS.md").exists(),
            "AGENTS.md must not appear",
        );
        let manifest = Manifest::read(tmp.path()).unwrap().unwrap();
        assert!(
            !manifest.files.iter().any(|p| p.starts_with(".codex")),
            "manifest must not gain .codex/* entries",
        );
    }
}
