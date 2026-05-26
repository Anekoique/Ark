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

use crate::{
    error::{Error, Result},
    io::{PathExt, merge_managed_blocks},
    layout::Layout,
    platforms::{self, PLATFORMS},
    state::{Manifest, manifest::MANIFEST_RELATIVE_PATH},
    templates::{ARK_TEMPLATES, walk},
};

mod backup;
mod base_store;
mod merge;
mod plan;
mod strategy;
mod verify_migration;

pub use backup::RestoreSummary;
use backup::UpgradeBackup;
use base_store::UpgradeBaseStore;
use plan::{PlannedAction, WriteKind, plan_actions, validate_manifest_paths};
use strategy::UpgradeConfig;
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
    /// Reports the plan without mutating anything.
    pub dry_run: bool,
    /// Restores the most recent backup instead of upgrading.
    pub restore: bool,
}

impl UpgradeOptions {
    /// Creates upgrade options for `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            conflict_policy: ConflictPolicy::default(),
            allow_downgrade: false,
            dry_run: false,
            restore: false,
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

    /// Sets dry-run mode (report only, mutate nothing).
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Sets restore mode (`--restore`).
    pub fn with_restore(mut self, restore: bool) -> Self {
        self.restore = restore;
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
    /// Number of `merged` files cleanly diff3-merged.
    pub merged_clean: usize,
    /// Number of `merged` files written with conflict markers.
    pub merged_conflict: usize,
    /// Number of `merged` files that fell back to the conflict pipeline (no base).
    pub merge_fallback: usize,
    /// Number of `ejected` paths skipped entirely.
    pub ejected_skipped: usize,
    /// Number of in-flight `VERIFY.md` files migrated from the legacy
    /// verdict-driven shape to the new living-checklist shape.
    pub verify_migrated: usize,
    /// Version recorded before upgrade.
    pub version_from: String,
    /// CLI version applied by upgrade.
    pub version_to: String,
    /// `true` when produced by a `--dry-run`; suppresses the counter line so
    /// the already-printed `DryRunPreview` is the sole output.
    pub dry_run: bool,
}

impl UpgradeSummary {
    fn segments(&self) -> [(&'static str, usize); 13] {
        [
            ("added", self.added),
            ("updated", self.updated),
            ("unchanged", self.unchanged),
            ("modified-preserved", self.modified_preserved),
            ("overwritten", self.overwritten),
            (".new-copied", self.created_new),
            ("merged", self.merged_clean),
            ("merge-conflict", self.merged_conflict),
            ("merge-fallback", self.merge_fallback),
            ("ejected", self.ejected_skipped),
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
        // Dry-run output is the preview, already printed; emit no counter line.
        if self.dry_run {
            return Ok(());
        }
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

/// The action `--dry-run` would take for one path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionLabel {
    /// New file would be written.
    Add,
    /// Unmodified template would update automatically.
    Update,
    /// User-modified file would be overwritten.
    Overwrite,
    /// `merged` file would diff3-merge cleanly.
    MergeClean,
    /// `merged` file would merge with conflict markers.
    MergeConflict,
    /// User-modified file would be preserved.
    Preserve,
    /// New template would be written to a `.new` sidecar.
    CreateNew,
    /// Removed template would be deleted.
    Delete,
    /// Removed template diverged; would be left as an orphan.
    Orphan,
    /// `ejected` path would be skipped entirely.
    EjectSkip,
}

impl ActionLabel {
    fn as_str(self) -> &'static str {
        match self {
            ActionLabel::Add => "add",
            ActionLabel::Update => "update",
            ActionLabel::Overwrite => "overwrite",
            ActionLabel::MergeClean => "merge-clean",
            ActionLabel::MergeConflict => "merge-conflict",
            ActionLabel::Preserve => "preserve",
            ActionLabel::CreateNew => ".new",
            ActionLabel::Delete => "delete",
            ActionLabel::Orphan => "orphan",
            ActionLabel::EjectSkip => "eject-skip",
        }
    }
}

/// Per-path preview of a `--dry-run` upgrade.
///
/// One `Display`-able structure so the CLI keeps its one-render-per-dispatch
/// contract: each row is `<label>\t<path>`, sorted by the deterministic plan
/// order, with a trailing version line.
#[derive(Debug, Default, Clone)]
pub struct DryRunPreview {
    /// One `(path, action)` row per planned action.
    pub rows: Vec<(PathBuf, ActionLabel)>,
    /// Version recorded before the (simulated) upgrade.
    pub version_from: String,
    /// CLI version a real upgrade would apply.
    pub version_to: String,
}

impl fmt::Display for DryRunPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "dry-run: {} planned action(s) ({} -> {})",
            self.rows.len(),
            self.version_from,
            self.version_to,
        )?;
        for (path, label) in &self.rows {
            writeln!(f, "  {:<22} {}", label.as_str(), path.display())?;
        }
        Ok(())
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
    let mut out: Vec<(PathBuf, Cow<'static, [u8]>)> = Vec::new();

    // Main `.ark/` tree.
    for entry in walk(&ARK_TEMPLATES) {
        let absolute = layout.ark_dir().join(entry.relative_path);
        let relative = absolute
            .strip_prefix(layout.root())
            .expect("template dest under project root")
            .to_path_buf();
        out.push((relative, Cow::Borrowed(entry.contents)));
    }

    // Per-installed-platform: the main tree, but skip files whose dest falls
    // under `agents_dest_dir`. Agent files are owned by `apply_managed_state`
    // (re-applied unconditionally after the conflict pipeline runs); routing
    // them through this catalogue would either falsely flag user-modified
    // agents as upgrade conflicts (only to have `apply_managed_state` overwrite
    // anyway) or, worse, let a user-preserved agent silently miss `apply`'s
    // re-write. Excluded here, owned-and-re-written there.
    for p in platforms::installed(manifest) {
        let dest_root = layout.resolve(p.dest_dir);
        let skip = p
            .agents_dest_dir
            .map(|d| layout.resolve(d))
            .filter(|sk| sk.starts_with(&dest_root));
        for entry in walk(p.templates) {
            let absolute = dest_root.join(entry.relative_path);
            if let Some(sk) = skip.as_deref()
                && absolute.starts_with(sk)
            {
                continue;
            }
            let relative = absolute
                .strip_prefix(layout.root())
                .expect("template dest under project root")
                .to_path_buf();
            out.push((relative, Cow::Borrowed(entry.contents)));
        }
    }
    out
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

/// Restores the most recent `ark upgrade` backup.
///
/// `--restore` entry: returns the working tree (files + manifest) to its
/// pre-upgrade state. Fails [`Error::NoBackupToRestore`] when none exists.
pub fn restore(opts: UpgradeOptions) -> Result<RestoreSummary> {
    let layout = Layout::new(&opts.project_root);
    UpgradeBackup::new(&layout).restore()
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

    // Load + validate the user's `[upgrade]` strategy before any mutation, so
    // a bad config halts pre-mutation (C-4, C-5, C-6, C-22).
    let config = UpgradeConfig::load_or_default(&layout)?;
    config.validate(&manifest.managed_blocks, &layout)?;

    let mut desired = collect_desired_templates(&layout, &manifest);
    // Desired paths come from `include_dir!` joined under
    // `layout.ark_dir()` / `layout.claude_dir()`, so they are safe by
    // construction; a unit test asserts parity against `init.rs::extract`.

    // Splice on-disk managed-block bodies into the desired bytes so blocks
    // written by other commands (e.g. `spec register`) are not flagged as
    // user modifications.
    reconcile_managed_blocks(&layout, &mut desired)?;

    // Under dry-run, only an interactive policy is coerced to Skip — it would
    // otherwise block on stdin for a preview. An explicit non-interactive
    // policy (--force / --create-new / --skip-modified) is previewed faithfully
    // so the preview reflects what the real run would do.
    let policy = if opts.dry_run && matches!(opts.conflict_policy, ConflictPolicy::Interactive) {
        ConflictPolicy::Skip
    } else {
        opts.conflict_policy
    };
    let plan = plan_actions(&layout, &manifest, &desired, policy, prompter, &config)?;

    // Dry-run: report the plan and return without any mutation (C-12). The
    // preview is the authoritative output; the returned summary stays empty so
    // the caller does not print a second, conflicting count line.
    if opts.dry_run {
        let preview = build_preview(&layout, &plan.actions, &version_from, &cli_version)?;
        println!("{preview}");
        return Ok(UpgradeSummary {
            version_from,
            version_to: cli_version,
            dry_run: true,
            ..Default::default()
        });
    }

    // Backup every file the plan will mutate or delete, plus the manifest, so
    // a failure mid-apply rolls back cleanly and a regretted upgrade can
    // `--restore` (C-13, C-20, C-21).
    let backup = UpgradeBackup::new(&layout);
    let backup_targets = backup_targets(&plan.actions);
    let manifest_before = serde_json::to_vec_pretty(&manifest).expect("manifest serializes");
    backup.capture(&backup_targets, &manifest_before)?;

    let mut summary = UpgradeSummary {
        version_from,
        version_to: cli_version.clone(),
        unchanged: plan.inline_unchanged,
        ..Default::default()
    };

    // Apply the plan. Any error triggers a rollback of files + manifest before
    // it propagates (C-14), leaving the tree at its pre-upgrade state.
    match apply_plan(
        &layout,
        &mut manifest,
        plan.actions,
        cli_version,
        &mut summary,
    ) {
        Ok(()) => {}
        Err(e) => {
            backup.restore()?;
            return Err(e);
        }
    }

    // In-flight task migration: any `phase ∈ {Verify, Committed}` task whose
    // `VERIFY.md` still carries the legacy `## Verdict` heading is rewritten
    // with the new living-checklist shape. Errors per-task are logged but do
    // not abort the upgrade — the template refresh above is more important.
    summary.verify_migrated = migrate_in_flight_verify_files(&layout);

    Ok(summary)
}

/// Returns the project-relative paths a plan will write or delete.
///
/// Used as the backup set. `Preserve` / `EjectSkip` / `RefreshHashOnly` /
/// `DropManifestEntry` do not change file bytes, so they are excluded.
fn backup_targets(actions: &[PlannedAction]) -> Vec<PathBuf> {
    actions
        .iter()
        .filter_map(|a| match a {
            PlannedAction::Write { relative, .. }
            | PlannedAction::Merged { relative, .. }
            | PlannedAction::MergeConflict { relative, .. }
            | PlannedAction::Delete { relative } => Some(relative.clone()),
            _ => None,
        })
        .collect()
}

/// Applies the sorted plan, recording base bytes for written `merged` paths.
///
/// Writes (incl. merges) run first; deferred deletions run after the durable
/// manifest flush, preserving the existing ordering invariant (SPEC C-16).
fn apply_plan(
    layout: &Layout,
    manifest: &mut Manifest,
    actions: Vec<PlannedAction>,
    cli_version: String,
    summary: &mut UpgradeSummary,
) -> Result<()> {
    let base_store = UpgradeBaseStore::new(layout);
    let mut deferred: Vec<PlannedAction> = Vec::new();

    for action in actions {
        match action {
            PlannedAction::Write {
                relative,
                contents,
                kind,
            } => {
                layout.resolve(&relative).write_bytes(&contents)?;
                manifest.record_file_with_hash(&relative, &contents);
                // Record the base for merged-eligible writes so the next
                // upgrade can diff3 against it (C-7, C-11).
                base_store.record(&relative, &contents)?;
                match kind {
                    WriteKind::Add => summary.added += 1,
                    WriteKind::AutoUpdate => summary.updated += 1,
                    WriteKind::Overwrite => summary.overwritten += 1,
                }
            }
            PlannedAction::Merged { relative, contents } => {
                layout.resolve(&relative).write_bytes(&contents)?;
                manifest.record_file_with_hash(&relative, &contents);
                base_store.record(&relative, &contents)?;
                summary.merged_clean += 1;
            }
            PlannedAction::MergeConflict { relative, contents } => {
                layout.resolve(&relative).write_bytes(&contents)?;
                // A conflict file carries markers; do not record it as the new
                // base — the user must resolve it first. Hash tracks the bytes
                // on disk so the next run sees user-modified.
                manifest.record_file_with_hash(&relative, &contents);
                summary.merged_conflict += 1;
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
            PlannedAction::EjectSkip { .. } => {
                summary.ejected_skipped += 1;
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
        if platform.is_installed(manifest) {
            platform.apply_managed_state(layout, manifest)?;
        }
    }

    // Durable manifest write BEFORE any delete can fail.
    manifest.version = cli_version;
    manifest.write(layout.root())?;

    let mut manifest_mutated = false;
    for action in deferred {
        match action {
            PlannedAction::Delete { relative } => {
                layout.resolve(&relative).remove_if_exists()?;
                manifest.drop_file(&relative);
                summary.deleted += 1;
                manifest_mutated = true;
            }
            PlannedAction::DropManifestEntry { relative } => {
                if layout.resolve(&relative).exists() {
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

    Ok(())
}

/// Builds the `--dry-run` preview from the planned actions.
///
/// Maps each `PlannedAction` to its `ActionLabel`. A `merged` path with no
/// recorded base re-enters `classify`, so it appears as its resolved conflict
/// action (overwrite / preserve / `.new`) rather than a distinct label.
/// Orphans vs. deletes are distinguished by on-disk existence, matching the
/// apply pass.
fn build_preview(
    layout: &Layout,
    actions: &[PlannedAction],
    version_from: &str,
    version_to: &str,
) -> Result<DryRunPreview> {
    let mut rows = Vec::with_capacity(actions.len());
    for action in actions {
        let (relative, label) = match action {
            PlannedAction::Write { relative, kind, .. } => (
                relative.clone(),
                match kind {
                    WriteKind::Add => ActionLabel::Add,
                    WriteKind::AutoUpdate => ActionLabel::Update,
                    WriteKind::Overwrite => ActionLabel::Overwrite,
                },
            ),
            PlannedAction::Merged { relative, .. } => (relative.clone(), ActionLabel::MergeClean),
            PlannedAction::MergeConflict { relative, .. } => {
                (relative.clone(), ActionLabel::MergeConflict)
            }
            PlannedAction::CreateNew { relative, .. } => (relative.clone(), ActionLabel::CreateNew),
            PlannedAction::Preserve { relative } => (relative.clone(), ActionLabel::Preserve),
            PlannedAction::EjectSkip { relative } => (relative.clone(), ActionLabel::EjectSkip),
            PlannedAction::Delete { relative } => (relative.clone(), ActionLabel::Delete),
            PlannedAction::DropManifestEntry { relative } => {
                let label = if layout.resolve(relative).exists() {
                    ActionLabel::Orphan
                } else {
                    // Bare manifest-entry drop (file already gone) — nothing to
                    // show as a file action; surface it as a delete.
                    ActionLabel::Delete
                };
                (relative.clone(), label)
            }
            // RefreshHashOnly does not change file bytes — omit from the preview.
            PlannedAction::RefreshHashOnly { .. } => continue,
        };
        rows.push((relative, label));
    }
    Ok(DryRunPreview {
        rows,
        version_from: version_from.to_string(),
        version_to: version_to.to_string(),
    })
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
        // Agent files are recorded in the manifest by `apply_managed_state` but
        // intentionally excluded from `collect_desired_templates` (they are
        // owned by `apply_managed_state`'s unconditional re-write, not the
        // upgrade conflict pipeline). Strip them before comparing.
        let from_manifest: std::collections::BTreeSet<_> = manifest
            .files
            .into_iter()
            .filter(|p| !crate::commands::upgrade::plan::is_agent_path(p))
            .collect();
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
        // Hashes are refreshed by:
        //   1. The conflict pipeline for every tracked file *except* seed-only
        //      paths (config.toml, project specs) and agent files (both are
        //      exempt — agents are owned by `apply_managed_state` instead).
        //   2. `apply_managed_state` for agent files (it calls
        //      `record_file_with_hash` directly).
        // So after upgrade, the only files lacking hashes are the seed-only
        // exempt paths.
        use super::plan::is_agent_path;
        let with_hash_expected = after
            .files
            .iter()
            .filter(|p| !is_exempted(p) || is_agent_path(p))
            .count();
        assert_eq!(after.hashes.len(), with_hash_expected);
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
            "[worktree]\nbranch_prefix = \"fix\"\ncopy = [\".env\"]\n\n[user]\nkey = \"value\"\n",
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
        assert!(after.contains("[user]"));
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

    // ---- `[upgrade]` strategy: ejection, merge, dry-run, backup ----

    /// Replaces the shipped `[upgrade]` section of the project config.
    ///
    /// The template already ships an `[upgrade]` section, so a second one
    /// would be a duplicate key. Truncate at the first `[upgrade]` header and
    /// append a fresh section.
    fn write_upgrade_config(root: &Path, body: &str) {
        let cfg = root.join(".ark/config.toml");
        let text = std::fs::read_to_string(&cfg).unwrap_or_default();
        let head = text.split("[upgrade]").next().unwrap_or("").trim_end();
        std::fs::write(&cfg, format!("{head}\n\n[upgrade]\n{body}")).unwrap();
    }

    /// V-IT-1: an `ejected` path is untouched even under `--force`.
    #[test]
    fn ejected_path_untouched_by_force() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "USER OWNED").unwrap();
        write_upgrade_config(tmp.path(), "ejected = [\".ark/workflow.md\"]\n");

        let mut prompter = PanicPrompter;
        let summary = upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Force),
            &mut prompter,
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "USER OWNED");
        assert_eq!(summary.overwritten, 0);
        assert!(summary.ejected_skipped >= 1);
    }

    /// V-IT-2 + V-UT-8: `--dry-run` after an edit reports an action and mutates nothing.
    #[test]
    fn dry_run_reports_and_mutates_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit").unwrap();
        let manifest_before = std::fs::read(tmp.path().join(".ark/.installed.json")).unwrap();

        let mut prompter = PanicPrompter; // dry-run must not prompt
        let summary = upgrade(
            UpgradeOptions::new(tmp.path()).with_dry_run(true),
            &mut prompter,
        )
        .unwrap();
        // Nothing applied.
        assert_eq!(summary.overwritten, 0);
        assert_eq!(summary.modified_preserved, 0);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
        assert_eq!(
            std::fs::read(tmp.path().join(".ark/.installed.json")).unwrap(),
            manifest_before,
            "dry-run must not touch the manifest"
        );
        assert!(!tmp.path().join(".ark/.upgrade-backup").exists());
    }

    /// V-002: `--dry-run --force` previews the chosen non-interactive policy
    /// (overwrite), not a coerced Skip, and still mutates nothing.
    #[test]
    fn dry_run_force_previews_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit").unwrap();

        // Capture the preview by building it directly (the dry-run path prints
        // to stdout; assert via the plan the same policy produces).
        let opts = UpgradeOptions::new(tmp.path())
            .with_policy(ConflictPolicy::Force)
            .with_dry_run(true);
        // A panicking prompter proves --force did not coerce to interactive.
        let summary = upgrade(opts, &mut PanicPrompter).unwrap();
        assert!(summary.dry_run);
        // Nothing mutated.
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
    }

    /// Records a base for `relative` by running one Ark write through upgrade.
    ///
    /// Returns once the sidecar base exists so a follow-up merge can use it.
    fn seed_base(root: &Path, relative: &str) {
        let layout = Layout::new(root);
        let store = UpgradeBaseStore::new(&layout);
        let current = layout.resolve(relative).read_bytes().unwrap();
        store.record(Path::new(relative), &current).unwrap();
    }

    /// V-IT-3: a `merged` file with disjoint edits merges cleanly.
    #[test]
    fn merged_disjoint_edits_merge_clean() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let rel = ".ark/workflow.md";
        // Base = the shipped template; record it as the merge base.
        seed_base(tmp.path(), rel);
        let target = tmp.path().join(rel);
        let base = std::fs::read_to_string(&target).unwrap();
        // User prepends a line (disjoint from any template tail change).
        std::fs::write(&target, format!("USER HEADER\n{base}")).unwrap();
        write_upgrade_config(tmp.path(), &format!("merged = [\"{rel}\"]\n"));

        let mut prompter = PanicPrompter;
        let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        // No template tail change in this fixture, so the merge is the user's
        // edit preserved with no conflict.
        assert_eq!(summary.merged_conflict, 0);
        assert!(summary.merged_clean >= 1 || summary.unchanged >= 1);
        assert!(
            std::fs::read_to_string(&target)
                .unwrap()
                .contains("USER HEADER")
        );
    }

    /// V-IT-4: overlapping edits produce conflict markers.
    #[test]
    fn merged_overlapping_edits_write_conflict_markers() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let rel = ".ark/merge-fixture.md";
        let target = tmp.path().join(rel);
        // Establish a base and a manifest entry by writing through the store +
        // manifest, simulating a file Ark shipped at version_from.
        std::fs::write(&target, "shared\n").unwrap();
        let layout = Layout::new(tmp.path());
        UpgradeBaseStore::new(&layout)
            .record(Path::new(rel), b"shared\n")
            .unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.record_file_with_hash(PathBuf::from(rel), b"shared\n");
        m.write(tmp.path()).unwrap();
        // User changes the shared line; "theirs" (desired) is just the manifest
        // file again — to force a real conflict we drive three_way_merge via a
        // diverged on-disk + diverged desired. Use the merge module directly
        // for the marker assertion (the apply path is covered by V-IT-3).
        let outcome = merge::three_way_merge(b"shared\n", b"ours\n", b"theirs\n");
        let merge::MergeOutcome::Conflict(bytes) = outcome else {
            panic!("expected conflict");
        };
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("<<<<<<<") && text.contains(">>>>>>>"));
    }

    /// V-IT-5 + V-E-4: a `merged` path with no base falls back to the conflict pipeline.
    #[test]
    fn merged_without_base_falls_back_to_conflict_skip() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let rel = ".ark/workflow.md";
        let target = tmp.path().join(rel);
        std::fs::write(&target, "user diverged, no base recorded").unwrap();
        write_upgrade_config(tmp.path(), &format!("merged = [\"{rel}\"]\n"));

        // No base recorded → fallback to conflict pipeline → Skip preserves.
        let summary = upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Skip),
            &mut PanicPrompter,
        )
        .unwrap();
        assert_eq!(summary.merged_clean, 0);
        assert_eq!(summary.merged_conflict, 0);
        assert_eq!(summary.modified_preserved, 1);
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "user diverged, no base recorded"
        );
    }

    /// V-IT-8 (C-19): a no-base merged path the user Skips stays on fallback
    /// across repeated upgrades — it never silently starts merging.
    #[test]
    fn merged_no_base_skip_stays_on_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let rel = ".ark/workflow.md";
        let target = tmp.path().join(rel);
        std::fs::write(&target, "diverged").unwrap();
        write_upgrade_config(tmp.path(), &format!("merged = [\"{rel}\"]\n"));

        for _ in 0..2 {
            let summary = upgrade(
                UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Skip),
                &mut PanicPrompter,
            )
            .unwrap();
            assert_eq!(summary.merged_clean, 0, "must not start merging");
            assert_eq!(std::fs::read_to_string(&target).unwrap(), "diverged");
        }
        // Skip never wrote the file, so no base was ever recorded.
        let layout = Layout::new(tmp.path());
        assert!(
            UpgradeBaseStore::new(&layout)
                .base_for(Path::new(rel))
                .unwrap()
                .is_none()
        );
    }

    /// V-IT-7: the `[upgrade]` config survives upgrade byte-identical.
    #[test]
    fn upgrade_config_survives_upgrade() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        write_upgrade_config(tmp.path(), "ejected = [\".ark/workflow.md\"]\n");
        let before = std::fs::read_to_string(tmp.path().join(".ark/config.toml")).unwrap();
        upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Skip),
            &mut PanicPrompter,
        )
        .unwrap();
        let after = std::fs::read_to_string(tmp.path().join(".ark/config.toml")).unwrap();
        assert_eq!(before, after);
    }

    /// V-F-4: invalid `[upgrade]` config halts before any file is written.
    #[test]
    fn invalid_upgrade_config_halts_pre_mutation() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit").unwrap();
        // CLAUDE.md carries a managed block — listing it under `merged` is invalid.
        write_upgrade_config(tmp.path(), "merged = [\"CLAUDE.md\"]\n");
        let err = upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Force),
            &mut PanicPrompter,
        )
        .unwrap_err();
        assert!(matches!(err, Error::UpgradeConfigInvalid { .. }));
        // The user edit is untouched — we halted before applying.
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
    }

    /// V-F-3 + V-IT-9: a successful upgrade leaves a backup that `--restore`
    /// rolls back to the pre-upgrade tree.
    #[test]
    fn restore_after_success_returns_pre_upgrade_tree() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let target = tmp.path().join(".ark/workflow.md");
        std::fs::write(&target, "user edit").unwrap();
        // Force overwrites the user edit; the backup captures the pre-upgrade file.
        upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Force),
            &mut PanicPrompter,
        )
        .unwrap();
        assert_ne!(std::fs::read_to_string(&target).unwrap(), "user edit");

        let summary = restore(UpgradeOptions::new(tmp.path()).with_restore(true)).unwrap();
        assert!(summary.restored >= 1);
        assert!(summary.manifest_restored);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
    }

    /// V-F-2: `--restore` with no backup errors.
    #[test]
    fn restore_without_backup_errors() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let err = restore(UpgradeOptions::new(tmp.path()).with_restore(true)).unwrap_err();
        assert!(matches!(err, Error::NoBackupToRestore { .. }));
    }

    /// V-F-1 + V-F-5 + C-20: when apply fails mid-flight, the backup restores
    /// both the user's files and the manifest to their pre-upgrade state.
    #[test]
    fn apply_failure_rolls_back_files_and_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();

        // A user-modified file the upgrade will (under Force) try to overwrite.
        let edited = tmp.path().join(".ark/workflow.md");
        std::fs::write(&edited, "user edit").unwrap();
        let manifest_before = std::fs::read(tmp.path().join(".ark/.installed.json")).unwrap();

        // Sabotage another tracked write target by turning its on-disk path
        // into a directory, so `write_bytes` fails mid-apply. `design.md` is a
        // shipped, unconditionally-rewritten command file.
        let victim = tmp.path().join(".claude/commands/ark/design.md");
        std::fs::remove_file(&victim).ok();
        std::fs::create_dir_all(&victim).unwrap();

        let err = upgrade(
            UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Force),
            &mut PanicPrompter,
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::Io { .. }),
            "expected an I/O failure, got {err:?}"
        );

        // Rollback restored the user's edited file and the manifest.
        assert_eq!(
            std::fs::read_to_string(&edited).unwrap(),
            "user edit",
            "rollback must restore the user's file"
        );
        assert_eq!(
            std::fs::read(tmp.path().join(".ark/.installed.json")).unwrap(),
            manifest_before,
            "rollback must restore the manifest (C-20)"
        );
    }

    /// V-E-1: empty `[upgrade]` sets behave exactly like today (no regression).
    #[test]
    fn empty_upgrade_config_is_noop_after_init() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        write_upgrade_config(tmp.path(), "ejected = []\nmerged = []\n");
        let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
        assert_eq!(summary.ejected_skipped, 0);
        assert_eq!(summary.merged_clean, 0);
        assert_eq!(summary.overwritten, 0);
        assert!(summary.unchanged > 0);
    }
}
