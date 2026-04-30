//! Upgrade planning: classification of on-disk vs. desired template content.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use super::{ConflictChoice, ConflictPolicy, Prompter};
use crate::{
    error::{Error, Result},
    io::{PathExt, hash_bytes},
    layout::Layout,
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
    /// Returns the deterministic action sort key.
    ///
    /// Writes happen before the manifest flush barrier, deletions after.
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

pub(super) fn is_exempted(relative: &Path) -> bool {
    relative == Path::new(MANIFEST_RELATIVE_PATH)
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
    fn is_exempted_only_matches_manifest_file() {
        assert!(is_exempted(Path::new(MANIFEST_RELATIVE_PATH)));
        assert!(!is_exempted(Path::new(".ark/workflow.md")));
        assert!(!is_exempted(Path::new(".claude/commands/ark/quick.md")));
        assert!(!is_exempted(Path::new(".ark/specs/INDEX.md")));
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
}
