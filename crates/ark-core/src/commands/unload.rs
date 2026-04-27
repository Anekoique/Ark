//! `ark unload` — freeze Ark state into `.ark.db` and remove live artifacts.
//!
//! Captures every file under Ark-owned directories and every managed block
//! Ark installed, then deletes the live footprint. Ignoring `.ark.db` in
//! version control is the user's responsibility.
//!
//! Pair with `ark load` to restore. `ark remove` discards `.ark.db` entirely.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use crate::{
    error::{Error, Result},
    io::{ARK_CONTEXT_HOOK_COMMAND, PathExt, read_managed_block, remove_managed_block, walk_files},
    layout::Layout,
    platforms::PLATFORMS,
    state::{Manifest, Snapshot, SnapshotHookBody},
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
    pub hook_bodies_captured: usize,
}

impl fmt::Display for UnloadSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "captured {} file(s), {} managed block(s), and {} hook entries into .ark.db",
            self.files_captured, self.blocks_captured, self.hook_bodies_captured,
        )
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

    // 3. Capture Ark-owned hook entries from every platform (Stage A) and
    //    from any unregistered `*.json` file under owned dirs (Stage B per
    //    codex-support C-24). Sibling user entries stay on disk; owned dirs
    //    are about to be wiped, so Stage B is capture-only.
    //
    //    Stage A surgically removes each platform's known entry from disk
    //    before Stage B reads — so when Stage B scans the same file, only
    //    additional Ark-identity entries (under other event arrays, etc.)
    //    remain to be captured. No path-level dedupe needed.
    for platform in PLATFORMS {
        platform.capture_hook(&layout, &mut snapshot)?;
    }
    capture_orphan_hook_entries(&layout, &mut snapshot)?;
    summary.hook_bodies_captured = snapshot.hook_bodies.len();

    // 4. Persist the snapshot before destroying anything else.
    snapshot.write(layout.root())?;

    // 5. Delete the live Ark footprint.
    layout
        .owned_dirs()
        .iter()
        .try_for_each(|d| d.remove_dir_all().map(|_| ()))?;
    for parent in layout.prunable_empty_parents() {
        parent.remove_dir_if_empty()?;
    }

    Ok(summary)
}

/// Managed blocks to capture: recorded in the manifest if present, else
/// every shipped platform's managed-block target as a fallback so a missing
/// manifest never leaves orphaned state on either Claude or Codex (or any
/// future platform with a managed-block target).
fn managed_blocks(layout: &Layout) -> Result<Vec<(PathBuf, String)>> {
    Ok(match Manifest::read(layout.root())? {
        Some(manifest) => manifest
            .managed_blocks
            .into_iter()
            .map(|b| (b.file, b.marker))
            .collect(),
        None => PLATFORMS
            .iter()
            .filter_map(|p| {
                p.managed_block_target
                    .map(|f| (PathBuf::from(f), layout.managed_marker().to_string()))
            })
            .collect(),
    })
}

/// Stage B per codex-support C-24: scan every `*.json` file under owned
/// dirs for Ark-identity entries that Stage A didn't already capture and
/// remove from disk. Each match is added to `snapshot.hook_bodies`. No
/// surgical write — owned dirs are about to be deleted. Parse failures
/// non-fatal (warn + skip).
fn capture_orphan_hook_entries(layout: &Layout, snapshot: &mut Snapshot) -> Result<()> {
    for owned in layout.owned_dirs() {
        for path in walk_files(&owned)? {
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                capture_from_orphan_file(&path, layout, snapshot)?;
            }
        }
    }
    Ok(())
}

fn capture_from_orphan_file(path: &Path, layout: &Layout, snapshot: &mut Snapshot) -> Result<()> {
    let Some(text) = path.read_text_optional()? else {
        return Ok(());
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&text) else {
        eprintln!(
            "warning: unload: skipping unparsable JSON at {}",
            path.display()
        );
        return Ok(());
    };
    let Some(hooks) = root.get("hooks").and_then(|v| v.as_object()) else {
        return Ok(());
    };
    let relative = path
        .strip_prefix(layout.root())
        .unwrap_or(path)
        .to_path_buf();
    for (array_key, array_val) in hooks {
        let Some(array) = array_val.as_array() else {
            continue;
        };
        for entry in array {
            if !crate::io::fs::entry_carries_command(entry, ARK_CONTEXT_HOOK_COMMAND, "command") {
                continue;
            }
            snapshot.add_hook_body(SnapshotHookBody {
                path: relative.clone(),
                json_pointer: format!("/hooks/{array_key}"),
                identity_key: "command".to_string(),
                identity_value: ARK_CONTEXT_HOOK_COMMAND.to_string(),
                entry: entry.clone(),
            });
        }
    }
    Ok(())
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
        // Default install covers both platforms (CLAUDE.md + AGENTS.md blocks).
        assert_eq!(summary.blocks_captured, 2);

        assert!(!tmp.path().join(".ark").exists());
        assert!(!tmp.path().join(".claude/commands/ark").exists());
        assert!(!tmp.path().join(".codex").exists());
        assert!(tmp.path().join(SNAPSHOT_FILENAME).exists());
        assert!(!tmp.path().join(".gitignore").exists());
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
        // Manifest-missing fallback now iterates every platform's
        // managed_block_target, so default install (Claude + Codex)
        // captures both `CLAUDE.md` and `AGENTS.md` blocks.
        assert_eq!(summary.blocks_captured, PLATFORMS.len());

        // Both blocks must be removed from disk (file deleted if it was
        // the only content, per `remove_managed_block` semantics).
        for target in ["CLAUDE.md", "AGENTS.md"] {
            let path = tmp.path().join(target);
            if path.exists() {
                let text = std::fs::read_to_string(&path).unwrap();
                assert!(
                    !text.contains("<!-- ARK:START -->"),
                    "{target} still has block"
                );
            }
        }

        // And captured into the snapshot so load can restore them later.
        let snap = Snapshot::read(tmp.path()).unwrap().unwrap();
        for target in ["CLAUDE.md", "AGENTS.md"] {
            assert!(
                snap.managed_blocks
                    .iter()
                    .any(|b| b.marker == "ARK" && b.file.ends_with(target)),
                "snapshot missing {target} block"
            );
        }
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

    /// codex-support C-18: source-scan invariant for `unload.rs`.
    #[test]
    fn unload_source_no_bare_std_fs_or_dot_path_literals() {
        crate::commands::tests_common::assert_source_clean(include_str!("unload.rs"));
    }

    /// V-IT-17 (codex-support C-24): Stage B captures Ark-identity hook
    /// entries living in JSON files under `owned_dirs()` that no registered
    /// platform points at — e.g. a future-version platform's `extras.json`.
    #[test]
    fn unload_captures_orphan_ark_hook_entries_in_unregistered_files() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        // Hand-place an Ark-identity hook entry in a JSON file that no
        // current PLATFORMS entry references. `.codex/extras.json` lives
        // under owned_dirs (`.codex/`).
        let extras = tmp.path().join(".codex/extras.json");
        std::fs::write(
            &extras,
            serde_json::to_string_pretty(&serde_json::json!({
                "hooks": {
                    "SomeFutureEvent": [{
                        "matcher": "",
                        "hooks": [{
                            "type": "command",
                            "command": "ark context --scope session --format json",
                            "timeout": 30
                        }]
                    }]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        unload(UnloadOptions::new(tmp.path())).unwrap();

        let snap = Snapshot::read(tmp.path()).unwrap().unwrap();
        assert!(
            snap.hook_bodies.iter().any(|hb| {
                hb.identity_value == ARK_CONTEXT_HOOK_COMMAND
                    && hb.json_pointer == "/hooks/SomeFutureEvent"
            }),
            "Stage B must capture orphan Ark hook entries: {:?}",
            snap.hook_bodies
                .iter()
                .map(|hb| (&hb.path, &hb.json_pointer))
                .collect::<Vec<_>>()
        );
    }

    /// Regression for PR#6 #5/#14: Stage B used to skip an entire file if
    /// Stage A captured anything from it, which dropped extra Ark-identity
    /// entries living under different event arrays in that same file. Now
    /// Stage B scans all `.json` files and the `SomeFutureEvent` entry must
    /// be captured even though Stage A captured the canonical SessionStart.
    #[test]
    fn unload_captures_extra_orphan_entries_in_stage_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        // Inject a second Ark-identity entry into `.codex/hooks.json` under
        // a non-canonical event array. Stage A removes the SessionStart one
        // surgically; Stage B must scan the same file and capture the rest.
        let hooks_path = tmp.path().join(".codex/hooks.json");
        let mut hooks: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path).unwrap()).unwrap();
        hooks["hooks"]["SomeFutureEvent"] = serde_json::json!([{
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": ARK_CONTEXT_HOOK_COMMAND,
                "timeout": 30,
            }]
        }]);
        std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks).unwrap()).unwrap();

        unload(UnloadOptions::new(tmp.path())).unwrap();

        let snap = Snapshot::read(tmp.path()).unwrap().unwrap();
        let pointers: Vec<&str> = snap
            .hook_bodies
            .iter()
            .filter(|hb| hb.path.ends_with("hooks.json"))
            .map(|hb| hb.json_pointer.as_str())
            .collect();
        assert!(pointers.contains(&"/hooks/SessionStart"), "{pointers:?}");
        assert!(pointers.contains(&"/hooks/SomeFutureEvent"), "{pointers:?}");
    }
}
