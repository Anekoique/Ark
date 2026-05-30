//! Upgrade planning: classification of on-disk vs. desired template content.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use super::{
    ConflictChoice, ConflictPolicy, Prompter,
    base_store::UpgradeBaseStore,
    config_merge::append_missing_sections,
    merge::{MergeOutcome, three_way_merge},
    strategy::{Strategy, UpgradeConfig},
};
use crate::{
    error::{Error, Result},
    io::{PathExt, hash_bytes},
    layout::{CONFIG_FILE, Layout, SPECS_PROJECT_DIR, SPECS_PROJECT_INDEX_FILE},
    state::{Manifest, manifest::MANIFEST_RELATIVE_PATH},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Classification {
    Add,
    Unchanged { refresh_hash: bool },
    AutoUpdate,
    UserModified,
    AmbiguousNoHash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RemovalClassification {
    SafeRemove,
    Orphaned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum WriteKind {
    Add,
    AutoUpdate,
    Overwrite,
}

/// Represents one planned upgrade mutation.
///
/// `Preserve` is a variant so the sorted apply pass can report it alongside
/// real writes. Counter-only `Unchanged{refresh_hash=false}` cases are tallied
/// inline during planning and never emit a `PlannedAction`.
#[derive(Debug, Clone)]
pub(super) enum PlannedAction {
    Write {
        relative: PathBuf,
        contents: Vec<u8>,
        kind: WriteKind,
    },
    /// A clean diff3 merge of a `merged` path; `contents` is the merged result.
    Merged {
        relative: PathBuf,
        contents: Vec<u8>,
    },
    /// A conflicting diff3 merge; `contents` carries Git-style markers.
    MergeConflict {
        relative: PathBuf,
        contents: Vec<u8>,
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
    /// An `ejected` path skipped entirely; counter-only, never written.
    EjectSkip {
        relative: PathBuf,
    },
    /// Append-only merge for a seed-only path (today: `.ark/config.toml`):
    /// every section the template ships that the user is missing is appended
    /// verbatim, preserving the user's bytes byte-for-byte as a prefix. See
    /// `commands/upgrade/config_merge.rs` for the section-slicing rules.
    AppendConfigSections {
        relative: PathBuf,
        contents: Vec<u8>,
    },
    Delete {
        relative: PathBuf,
    },
    DropManifestEntry {
        relative: PathBuf,
    },
}

impl PlannedAction {
    /// Returns the deterministic action sort key.
    ///
    /// Order (per SPEC C-16): `Write{Add}` < `Write{AutoUpdate}` <
    /// `Write{Overwrite}` < `Merged` < `MergeConflict` < `AppendConfigSections`
    /// < `CreateNew` < `RefreshHashOnly` < `Preserve` < `EjectSkip` < `Delete`
    /// < `DropManifestEntry`. Writes (incl. merges and additive section
    /// appends) happen before the manifest flush barrier; deletions after.
    fn sort_key(&self) -> (u8, Option<WriteKind>, &Path) {
        match self {
            PlannedAction::Write { kind, relative, .. } => (0, Some(*kind), relative),
            PlannedAction::Merged { relative, .. } => (1, None, relative),
            PlannedAction::MergeConflict { relative, .. } => (2, None, relative),
            // AppendConfigSections is an additive write that mutates a seed-
            // only path; it lives in the merge-adjacent band so dry-run rows
            // group with the other write-class actions, just past the diff3
            // merges and before CreateNew sidecars.
            PlannedAction::AppendConfigSections { relative, .. } => (3, None, relative),
            PlannedAction::CreateNew { relative, .. } => (4, None, relative),
            PlannedAction::RefreshHashOnly { relative, .. } => (5, None, relative),
            PlannedAction::Preserve { relative } => (6, None, relative),
            PlannedAction::EjectSkip { relative } => (7, None, relative),
            PlannedAction::Delete { relative } => (8, None, relative),
            PlannedAction::DropManifestEntry { relative } => (9, None, relative),
        }
    }
}

pub(super) fn is_exempted(relative: &Path) -> bool {
    relative == Path::new(MANIFEST_RELATIVE_PATH)
        || is_seed_only(relative)
        || is_agent_path(relative)
}

/// Paths under any platform's `agents_dest_dir`.
///
/// Agent files are owned by [`crate::platforms::Platform::apply_managed_state`]
/// (re-applied unconditionally on every `init` / `load` / `upgrade`); the
/// upgrade conflict pipeline must not classify them as orphans or prompt about
/// user modifications, since `apply_managed_state` overwrites them after the
/// pipeline runs anyway. Excluded here, owned-and-re-written there.
pub(super) fn is_agent_path(relative: &Path) -> bool {
    crate::platforms::PLATFORMS
        .iter()
        .filter_map(|p| p.agents_dest_dir)
        .any(|dest| relative.starts_with(Path::new(dest)))
}

/// Paths seeded by `init` whose content becomes user-owned afterwards.
///
/// Upgrade skips them entirely: no classification, no prompt, no removal —
/// even when the embedded template's bytes change between releases. Adding a
/// path here is a behavioral contract: once shipped, a user's edits to that
/// file will never be overwritten by `ark upgrade`.
fn is_seed_only(relative: &Path) -> bool {
    if relative == Path::new(CONFIG_FILE) || relative == Path::new(SPECS_PROJECT_INDEX_FILE) {
        return true;
    }
    relative.starts_with(SPECS_PROJECT_DIR)
}

/// Normalizes every `manifest.files` entry through `Layout::resolve_safe`.
pub(super) fn validate_manifest_paths(layout: &Layout, files: &[PathBuf]) -> Result<()> {
    for path in files {
        layout.resolve_safe(path).map_err(remap_unsafe_path)?;
    }
    Ok(())
}

/// Rebadges a `resolve_safe` failure for the manifest trust boundary.
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

/// Plans the action for a `merged` path, or `None` to fall back to the
/// standard pipeline.
///
/// Returns `None` (fall back) when the file is absent, the recorded hash says
/// the user did not diverge (the standard pipeline handles Add / Unchanged /
/// AutoUpdate correctly), or no base is recorded (C-8 — a 2-way "merge" would
/// be garbage). Otherwise runs diff3 and returns `Merged` / `MergeConflict`.
fn plan_merge(
    relative: &Path,
    desired: &[u8],
    on_disk: Option<&[u8]>,
    recorded: Option<&str>,
    base_store: &UpgradeBaseStore<'_>,
) -> Result<Option<PlannedAction>> {
    // Merge only applies when both sides changed: there is an on-disk file,
    // it differs from the desired template, and the recorded hash shows the
    // user (not Ark's last write) authored the on-disk content. Everything
    // else (Add, Unchanged, AutoUpdate) is handled by `classify`.
    let Some(current) = on_disk else {
        return Ok(None);
    };
    let current_hash = hash_bytes(current);
    let diverged = current_hash != hash_bytes(desired) && recorded != Some(current_hash.as_str());
    if !diverged {
        return Ok(None);
    }
    // No recorded base → cannot diff3; fall back to the conflict pipeline.
    let Some(base) = base_store.base_for(relative)? else {
        return Ok(None);
    };
    Ok(Some(match three_way_merge(&base, current, desired) {
        MergeOutcome::Clean(bytes) => PlannedAction::Merged {
            relative: relative.to_path_buf(),
            contents: bytes,
        },
        MergeOutcome::Conflict(bytes) => PlannedAction::MergeConflict {
            relative: relative.to_path_buf(),
            contents: bytes,
        },
    }))
}

/// Plans an [`PlannedAction::AppendConfigSections`] for `.ark/config.toml`
/// when the template ships top-level sections the user's file lacks.
///
/// Returns `Ok(None)` when:
/// - the user has no `config.toml` on disk yet (the standard pipeline would
///   normally have written it via `Add`, but seed-only excludes it; absence
///   here means the user has truly never been seeded — let `init` re-seed,
///   don't synthesize a half-merge);
/// - the template ships no `config.toml`;
/// - the user already has every template-shipped section;
/// - the path is `ejected` (no merge of any kind).
///
/// Path safety: a corrupt UTF-8 file disables the merge and returns `Ok(None)`
/// rather than mutating. The append helper itself operates on `&str` and would
/// be unsound on non-text bytes.
fn plan_config_section_append(
    layout: &Layout,
    desired: &[(PathBuf, Cow<'static, [u8]>)],
    config: &UpgradeConfig,
) -> Result<Option<PlannedAction>> {
    let relative = Path::new(CONFIG_FILE);
    if config.strategy_for(relative) == Strategy::Ejected {
        return Ok(None);
    }
    let Some(template_bytes) = desired
        .iter()
        .find(|(p, _)| p == relative)
        .map(|(_, bytes)| bytes.as_ref())
    else {
        return Ok(None);
    };
    let Some(user_bytes) = layout.resolve(relative).read_optional()? else {
        return Ok(None);
    };
    let (Ok(user_text), Ok(template_text)) = (
        std::str::from_utf8(&user_bytes),
        std::str::from_utf8(template_bytes),
    ) else {
        return Ok(None);
    };
    Ok(
        append_missing_sections(user_text, template_text).map(|merged| {
            PlannedAction::AppendConfigSections {
                relative: relative.to_path_buf(),
                contents: merged.into_bytes(),
            }
        }),
    )
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

pub(super) struct Plan {
    pub(super) actions: Vec<PlannedAction>,
    pub(super) inline_unchanged: usize,
}

pub(super) fn plan_actions(
    layout: &Layout,
    manifest: &Manifest,
    desired: &[(PathBuf, Cow<'static, [u8]>)],
    policy: ConflictPolicy,
    prompter: &mut dyn Prompter,
    config: &UpgradeConfig,
) -> Result<Plan> {
    let mut actions: Vec<PlannedAction> = Vec::new();
    let mut inline_unchanged = 0usize;
    let base_store = UpgradeBaseStore::new(layout);

    // Seed-only `config.toml` gets an additive section merge: any top-level
    // table the current template ships that the user is missing is appended
    // to the user's file verbatim. Ejection wins (C-3 in spirit), so a path
    // explicitly listed under `[upgrade] ejected` produces no action. The
    // user's existing bytes are preserved as a prefix (the merge is strictly
    // additive — never rewrites or reorders existing sections).
    if let Some(action) = plan_config_section_append(layout, desired, config)? {
        actions.push(action);
    }

    let desired_keys: std::collections::BTreeSet<&Path> =
        desired.iter().map(|(p, _)| p.as_path()).collect();

    for (relative, contents) in desired {
        if is_exempted(relative) {
            continue;
        }
        // Ejected paths are removed from consideration before any
        // classification, write, or prompt (C-2, C-3 — beats every policy).
        if config.strategy_for(relative) == Strategy::Ejected {
            actions.push(PlannedAction::EjectSkip {
                relative: relative.clone(),
            });
            continue;
        }
        let absolute = layout.resolve(relative);
        let on_disk = absolute.read_optional()?;
        let recorded = manifest.hash_for(relative);

        // A `merged` path whose sides have diverged and that has a recorded
        // base is diff3-merged; with no base, it falls through to the standard
        // conflict pipeline below (C-8 fallback).
        if config.strategy_for(relative) == Strategy::Merged
            && let Some(action) = plan_merge(
                relative,
                contents,
                on_disk.as_deref(),
                recorded,
                &base_store,
            )?
        {
            actions.push(action);
            continue;
        }

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
        // Desired paths were handled in the pass above (incl. their EjectSkip);
        // the removal pass only considers manifest entries no longer shipped.
        if desired_keys.contains(manifest_path.as_path()) {
            continue;
        }
        // A removed-but-ejected file is never deleted — ejection covers
        // removal too (C-2). Count it once here (it had no desired-pass entry).
        if config.strategy_for(manifest_path) == Strategy::Ejected {
            actions.push(PlannedAction::EjectSkip {
                relative: manifest_path.clone(),
            });
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

    actions.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

    Ok(Plan {
        actions,
        inline_unchanged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Layout;

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
    fn is_exempted_matches_manifest_and_seed_only_paths() {
        assert!(is_exempted(Path::new(MANIFEST_RELATIVE_PATH)));
        assert!(is_exempted(Path::new(CONFIG_FILE)));
        assert!(is_exempted(Path::new(SPECS_PROJECT_INDEX_FILE)));
        assert!(is_exempted(Path::new(".ark/specs/project/rust/STYLE.md")));
        assert!(!is_exempted(Path::new(".ark/workflow.md")));
        assert!(!is_exempted(Path::new(".claude/commands/ark/quick.md")));
        assert!(!is_exempted(Path::new(".ark/specs/INDEX.md")));
        assert!(!is_exempted(Path::new(".ark/specs/features/INDEX.md")));
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
    fn plan_actions_sorts_output_by_bucket_then_path() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();
        let layout = layout_for(&tmp);
        let manifest = crate::state::Manifest::read(tmp.path()).unwrap().unwrap();
        let desired: Vec<(PathBuf, Cow<'static, [u8]>)> =
            super::super::collect_desired_for_test(&layout, &manifest);
        let mut prompter = PanicPrompter;
        let plan = plan_actions(
            &layout,
            &manifest,
            &desired,
            ConflictPolicy::Skip,
            &mut prompter,
            &UpgradeConfig::default(),
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

    /// V-UT-10: the new variants occupy their C-16 buckets and stay sorted.
    #[test]
    fn new_action_variants_sort_into_their_buckets() {
        let p = PathBuf::from("z.md");
        let variants = [
            PlannedAction::Write {
                relative: p.clone(),
                contents: vec![],
                kind: WriteKind::Overwrite,
            },
            PlannedAction::Merged {
                relative: p.clone(),
                contents: vec![],
            },
            PlannedAction::MergeConflict {
                relative: p.clone(),
                contents: vec![],
            },
            PlannedAction::Preserve {
                relative: p.clone(),
            },
            PlannedAction::EjectSkip {
                relative: p.clone(),
            },
            PlannedAction::Delete {
                relative: p.clone(),
            },
        ];
        let buckets: Vec<u8> = variants.iter().map(|a| a.sort_key().0).collect();
        // Strictly increasing — Merged after Write, before CreateNew;
        // EjectSkip after Preserve, before Delete. AppendConfigSections sits
        // between MergeConflict (=2) and CreateNew, bumping the trailing
        // buckets by one.
        assert_eq!(buckets, vec![0, 1, 2, 6, 7, 8]);
        let mut sorted = buckets.clone();
        sorted.sort();
        assert_eq!(buckets, sorted);
    }
}
