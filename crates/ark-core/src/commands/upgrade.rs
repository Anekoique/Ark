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
    io::{PathExt, hash_bytes, merge_managed_blocks},
    layout::Layout,
    platforms::{self, PLATFORMS},
    state::{Manifest, manifest::MANIFEST_RELATIVE_PATH},
    templates::{ARK_TEMPLATES, walk},
};

/// How to resolve a conflict when the user has modified a template locally
/// AND the template's canonical content has changed between versions.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    Overwrite,
    Skip,
    CreateNew,
}

/// Callback invoked for each user-modified file when the policy is
/// [`ConflictPolicy::Interactive`]. The library never reads stdin itself.
pub trait Prompter {
    fn prompt(&mut self, relative_path: &Path) -> Result<ConflictChoice>;
}

#[derive(Debug, Clone)]
pub struct UpgradeOptions {
    pub project_root: PathBuf,
    pub conflict_policy: ConflictPolicy,
    pub allow_downgrade: bool,
}

impl UpgradeOptions {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            conflict_policy: ConflictPolicy::default(),
            allow_downgrade: false,
        }
    }

    pub fn with_policy(mut self, policy: ConflictPolicy) -> Self {
        self.conflict_policy = policy;
        self
    }

    pub fn with_allow_downgrade(mut self, allow: bool) -> Self {
        self.allow_downgrade = allow;
        self
    }
}

/// Per-outcome counters produced by [`upgrade`].
#[derive(Debug, Default, Clone)]
pub struct UpgradeSummary {
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub modified_preserved: usize,
    pub overwritten: usize,
    pub created_new: usize,
    pub deleted: usize,
    pub orphaned: usize,
    pub version_from: String,
    pub version_to: String,
}

impl UpgradeSummary {
    fn segments(&self) -> [(&'static str, usize); 8] {
        [
            ("added", self.added),
            ("updated", self.updated),
            ("unchanged", self.unchanged),
            ("modified-preserved", self.modified_preserved),
            ("overwritten", self.overwritten),
            (".new-copied", self.created_new),
            ("deleted", self.deleted),
            ("orphaned", self.orphaned),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    Add,
    Unchanged { refresh_hash: bool },
    AutoUpdate,
    UserModified,
    AmbiguousNoHash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemovalClassification {
    SafeRemove,
    Orphaned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WriteKind {
    Add,
    AutoUpdate,
    Overwrite,
}

/// A planned mutation. `Preserve` is a variant (not just a counter bump) so
/// the sorted apply pass can report it alongside real writes. Counter-only
/// `Unchanged{refresh_hash=false}` cases are tallied inline during planning
/// and never emit a `PlannedAction`.
#[derive(Debug, Clone)]
enum PlannedAction {
    Write {
        relative: PathBuf,
        contents: Vec<u8>,
        kind: WriteKind,
    },
    RefreshHashOnly {
        relative: PathBuf,
        contents: Vec<u8>,
    },
    CreateNew {
        relative: PathBuf,
        contents: Vec<u8>,
    },
    Preserve {
        relative: PathBuf,
    },
    Delete {
        relative: PathBuf,
    },
    DropManifestEntry {
        relative: PathBuf,
    },
}

impl PlannedAction {
    /// C-19 bucket order — writes before the manifest flush barrier, deletions
    /// after. `WriteKind`'s declared order (`Add < AutoUpdate < Overwrite`)
    /// sub-orders the write bucket.
    fn sort_key(&self) -> (u8, Option<WriteKind>, &Path) {
        match self {
            PlannedAction::Write { kind, relative, .. } => (0, Some(*kind), relative),
            PlannedAction::CreateNew { relative, .. } => (1, None, relative),
            PlannedAction::RefreshHashOnly { relative, .. } => (2, None, relative),
            PlannedAction::Preserve { relative } => (3, None, relative),
            PlannedAction::Delete { relative } => (4, None, relative),
            PlannedAction::DropManifestEntry { relative } => (5, None, relative),
        }
    }
}

fn is_exempted(relative: &Path) -> bool {
    relative == Path::new(MANIFEST_RELATIVE_PATH)
}

/// Walk the embedded template trees and produce project-relative keys (per
/// C-18). This mirrors `init.rs`'s extraction shape so the keys are byte-equal
/// to what `manifest.files` stores.
///
/// Per codex-support G-14: only platforms whose `dest_dir` already appears in
/// `manifest.files` are included. A Claude-only project upgraded by a CLI
/// that knows about Codex stays Claude-only. To opt in, the user re-runs
/// `ark init --codex`.
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

/// Splice on-disk managed-block bodies into every desired template that
/// carries one. Without this step, upgrade would hash-classify the divergent
/// (template vs on-disk) bytes as "user-modified" and prompt to overwrite —
/// which would destroy rows that `spec register` (and similar) wrote.
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

/// C-17: normalize every `manifest.files` entry through `Layout::resolve_safe`.
fn validate_manifest_paths(layout: &Layout, files: &[PathBuf]) -> Result<()> {
    for path in files {
        layout.resolve_safe(path).map_err(remap_unsafe_path)?;
    }
    Ok(())
}

/// Rebadge a `resolve_safe` failure as a manifest-trust-boundary failure. The
/// underlying reason strings come from `Layout::classify_unsafe`, unchanged.
fn remap_unsafe_path(e: Error) -> Error {
    match e {
        Error::UnsafeSnapshotPath { path, reason } => Error::UnsafeManifestPath { path, reason },
        other => other,
    }
}

fn classify(desired: &[u8], on_disk: Option<&[u8]>, recorded: Option<&str>) -> Classification {
    let Some(current) = on_disk else {
        return Classification::Add;
    };
    let desired_hash = hash_bytes(desired);
    let current_hash = hash_bytes(current);

    if current_hash == desired_hash {
        let refresh = recorded != Some(current_hash.as_str());
        return Classification::Unchanged {
            refresh_hash: refresh,
        };
    }

    match recorded {
        Some(r) if r == current_hash => Classification::AutoUpdate,
        Some(_) => Classification::UserModified,
        None => Classification::AmbiguousNoHash,
    }
}

fn classify_removal(on_disk: &[u8], recorded: Option<&str>) -> RemovalClassification {
    let current_hash = hash_bytes(on_disk);
    match recorded {
        Some(r) if r == current_hash => RemovalClassification::SafeRemove,
        _ => RemovalClassification::Orphaned,
    }
}

fn resolve_conflict(
    relative: &Path,
    policy: ConflictPolicy,
    prompter: &mut dyn Prompter,
) -> Result<ConflictChoice> {
    match policy {
        ConflictPolicy::Force => Ok(ConflictChoice::Overwrite),
        ConflictPolicy::Skip => Ok(ConflictChoice::Skip),
        ConflictPolicy::CreateNew => Ok(ConflictChoice::CreateNew),
        ConflictPolicy::Interactive => prompter.prompt(relative),
    }
}

struct Plan {
    actions: Vec<PlannedAction>,
    inline_unchanged: usize,
}

fn plan_actions(
    layout: &Layout,
    manifest: &Manifest,
    desired: &[(PathBuf, Cow<'static, [u8]>)],
    policy: ConflictPolicy,
    prompter: &mut dyn Prompter,
) -> Result<Plan> {
    let mut actions: Vec<PlannedAction> = Vec::new();
    let mut inline_unchanged = 0usize;

    let desired_keys: std::collections::BTreeSet<&Path> =
        desired.iter().map(|(p, _)| p.as_path()).collect();

    for (relative, contents) in desired {
        if is_exempted(relative) {
            continue;
        }
        let absolute = layout.resolve(relative);
        let on_disk = absolute.read_optional()?;
        let recorded = manifest.hash_for(relative);
        match classify(contents, on_disk.as_deref(), recorded) {
            Classification::Add => actions.push(PlannedAction::Write {
                relative: relative.clone(),
                contents: contents.clone().into_owned(),
                kind: WriteKind::Add,
            }),
            Classification::Unchanged {
                refresh_hash: false,
            } => {
                inline_unchanged += 1;
            }
            Classification::Unchanged { refresh_hash: true } => {
                // Split responsibility: the counter bump happens inline (same as
                // refresh_hash=false), and the RefreshHashOnly action updates the
                // in-memory manifest hash without touching any counter. Do NOT bump
                // `summary.unchanged` from the RefreshHashOnly handler — double-count.
                let bytes = on_disk.expect("unchanged requires file present");
                inline_unchanged += 1;
                actions.push(PlannedAction::RefreshHashOnly {
                    relative: relative.clone(),
                    contents: bytes,
                });
            }
            Classification::AutoUpdate => actions.push(PlannedAction::Write {
                relative: relative.clone(),
                contents: contents.clone().into_owned(),
                kind: WriteKind::AutoUpdate,
            }),
            Classification::UserModified | Classification::AmbiguousNoHash => {
                let choice = resolve_conflict(relative, policy, prompter)?;
                actions.push(match choice {
                    ConflictChoice::Overwrite => PlannedAction::Write {
                        relative: relative.clone(),
                        contents: contents.clone().into_owned(),
                        kind: WriteKind::Overwrite,
                    },
                    ConflictChoice::Skip => PlannedAction::Preserve {
                        relative: relative.clone(),
                    },
                    ConflictChoice::CreateNew => PlannedAction::CreateNew {
                        relative: relative.clone(),
                        contents: contents.clone().into_owned(),
                    },
                });
            }
        }
    }

    for manifest_path in &manifest.files {
        if is_exempted(manifest_path) {
            continue;
        }
        if desired_keys.contains(manifest_path.as_path()) {
            continue;
        }
        let absolute = layout.resolve(manifest_path);
        match absolute.read_optional()? {
            None => actions.push(PlannedAction::DropManifestEntry {
                relative: manifest_path.clone(),
            }),
            Some(bytes) => match classify_removal(&bytes, manifest.hash_for(manifest_path)) {
                RemovalClassification::SafeRemove => actions.push(PlannedAction::Delete {
                    relative: manifest_path.clone(),
                }),
                RemovalClassification::Orphaned => actions.push(PlannedAction::DropManifestEntry {
                    relative: manifest_path.clone(),
                }),
            },
        }
    }

    // C-19: deterministic order.
    actions.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

    Ok(Plan {
        actions,
        inline_unchanged,
    })
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

    // C-17: path safety runs before any semantic check.
    validate_manifest_paths(&layout, &manifest.files)?;
    check_version(&manifest.version, &cli_version, opts.allow_downgrade)?;

    let mut desired = collect_desired_templates(&layout, &manifest);
    // C-17 symmetry note: desired paths come from `include_dir!` joined under
    // `layout.ark_dir()` / `layout.claude_dir()`, so they are safe by
    // construction. V-UT-17 asserts parity against `init.rs::extract`. No
    // runtime check needed here.

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
    // applied on every upgrade, not hash-tracked. Per ark-upgrade C-8 /
    // ark-context C-17 / codex-support G-11. Only platforms already in the
    // manifest are touched (preserves G-14: Claude-only stays Claude-only).
    for platform in PLATFORMS {
        if platform.is_installed(&manifest) {
            platform.apply_managed_state(&layout, &mut manifest)?;
        }
    }

    // R-004: durable manifest write BEFORE any delete can fail.
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

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

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
    fn classify_add_when_file_missing() {
        assert_eq!(classify(b"x", None, None), Classification::Add);
    }

    #[test]
    fn classify_unchanged_no_refresh_when_hash_matches_current_and_desired() {
        let hash = hash_bytes(b"same");
        assert_eq!(
            classify(b"same", Some(b"same"), Some(&hash)),
            Classification::Unchanged {
                refresh_hash: false
            }
        );
    }

    #[test]
    fn classify_unchanged_refresh_when_content_matches_desired_but_hash_stale() {
        assert_eq!(
            classify(b"same", Some(b"same"), Some("stale")),
            Classification::Unchanged { refresh_hash: true }
        );
    }

    #[test]
    fn classify_unchanged_refresh_when_content_matches_desired_but_hash_missing() {
        assert_eq!(
            classify(b"same", Some(b"same"), None),
            Classification::Unchanged { refresh_hash: true }
        );
    }

    #[test]
    fn classify_auto_update_when_hash_matches_current() {
        let hash = hash_bytes(b"old");
        assert_eq!(
            classify(b"new", Some(b"old"), Some(&hash)),
            Classification::AutoUpdate
        );
    }

    #[test]
    fn classify_user_modified_when_hash_mismatches_recorded() {
        assert_eq!(
            classify(b"new", Some(b"old"), Some("different-stored-hash")),
            Classification::UserModified
        );
    }

    #[test]
    fn classify_ambiguous_no_hash_when_content_differs_without_record() {
        assert_eq!(
            classify(b"new", Some(b"old"), None),
            Classification::AmbiguousNoHash
        );
    }

    #[test]
    fn classify_removal_safe_when_hash_matches() {
        let hash = hash_bytes(b"x");
        assert_eq!(
            classify_removal(b"x", Some(&hash)),
            RemovalClassification::SafeRemove
        );
    }

    #[test]
    fn classify_removal_orphaned_otherwise() {
        assert_eq!(
            classify_removal(b"x", None),
            RemovalClassification::Orphaned
        );
        assert_eq!(
            classify_removal(b"x", Some("stale")),
            RemovalClassification::Orphaned
        );
    }

    #[test]
    fn is_exempted_only_matches_manifest_file() {
        assert!(is_exempted(Path::new(MANIFEST_RELATIVE_PATH)));
        assert!(!is_exempted(Path::new(".ark/workflow.md")));
        assert!(!is_exempted(Path::new(".claude/commands/ark/quick.md")));
        assert!(!is_exempted(Path::new(".ark/specs/INDEX.md")));
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
    fn validate_manifest_paths_accepts_safe_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let files = vec![
            PathBuf::from(".ark/workflow.md"),
            PathBuf::from(".claude/commands/ark/quick.md"),
        ];
        assert!(validate_manifest_paths(&layout, &files).is_ok());
    }

    #[test]
    fn validate_manifest_paths_rejects_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = layout_for(&tmp);
        let files = vec![PathBuf::from("../escape.md")];
        assert!(matches!(
            validate_manifest_paths(&layout, &files),
            Err(Error::UnsafeManifestPath { .. })
        ));
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
        // V-UT-18: enforces C-12 (no bare std::fs::*) and C-13 (no `.ark/` literal
        // path composition) at compile time. Line-by-line scan, excluding the
        // tests module itself and `//` comments.
        let source = include_str!("upgrade.rs");
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
                "line {} contains bare std::fs::: {line}",
                idx + 1
            );
            assert!(
                !code.contains("\".ark/"),
                "line {} contains hand-joined .ark/ literal: {line}",
                idx + 1
            );
            assert!(
                !code.contains("\".claude/"),
                "line {} contains hand-joined .claude/ literal: {line}",
                idx + 1
            );
        }
    }

    #[test]
    fn plan_actions_sorts_output_by_bucket_then_path() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let layout = layout_for(&tmp);
        let manifest = Manifest::read(tmp.path()).unwrap().unwrap();
        let desired = collect_desired_templates(&layout, &manifest);
        let mut prompter = PanicPrompter;
        let plan = plan_actions(
            &layout,
            &manifest,
            &desired,
            ConflictPolicy::Skip,
            &mut prompter,
        )
        .unwrap();
        let keys: Vec<_> = plan
            .actions
            .iter()
            .map(|a| {
                let (bucket, kind, path) = a.sort_key();
                (bucket, kind, path.to_path_buf())
            })
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "actions must be sort-key-ordered");
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

    /// V-IT-14 / C-29: running `ark upgrade` twice produces a byte-identical
    /// `.claude/settings.json`. The hook re-application is unconditional but
    /// idempotent at the helper level; this asserts the integration is
    /// drift-free.
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

    /// V-IT-13: deleting the Ark hook entry then running `ark upgrade`
    /// re-adds it (no prompt, no hash check).
    #[test]
    fn upgrade_re_adds_deleted_session_start_hook() {
        use crate::io::ARK_CONTEXT_HOOK_COMMAND;
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
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let mut m = Manifest::read(tmp.path()).unwrap().unwrap();
        m.hashes = BTreeMap::new();
        m.write(tmp.path()).unwrap();
        let mut prompter = PanicPrompter;
        upgrade(UpgradeOptions::new(tmp.path()), &mut prompter).unwrap();
        let after = Manifest::read(tmp.path()).unwrap().unwrap();
        assert_eq!(after.hashes.len(), after.files.len());
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

    /// V-IT-15 (codex-support G-14): a Claude-only project upgraded with the
    /// new CLI version remains Claude-only — `ark upgrade` does NOT install
    /// `.codex/` artifacts or write the AGENTS.md managed block.
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
