//! `ark load` — bring Ark into a project.
//!
//! - Snapshot present → restore every captured file and block, then delete
//!   the snapshot.
//! - No snapshot → scaffold from embedded templates (behaves like `init`).
//! - `.ark/` already present → error unless `force = true` (then wipe first).

use std::{fmt, path::PathBuf};

use crate::{
    commands::init::{InitOptions, InitSummary, init},
    error::{Error, Result},
    io::{PathExt, WriteMode, update_managed_block, write_file},
    layout::Layout,
    platforms::{CLAUDE_PLATFORM, PLATFORMS, Platform},
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

    // ark-context C-18 / codex-support C-22: restore Ark-owned hook entries.
    // Replay each captured entry, then overwrite with the canonical shape so
    // the on-disk hook is independent of snapshot age. For legacy snapshots
    // (no `hook_bodies`) we treat Claude as installed-by-default — Claude
    // shipped first and predates the manifest-prefix invariant.
    for hb in &snapshot.hook_bodies {
        hb.apply(layout)?;
    }
    for platform in canonical_targets(&snapshot) {
        if let Some(spec) = platform.hook_file {
            spec.apply_canonical(layout)?;
        }
    }

    Snapshot::remove(layout.root())?;

    Ok(LoadSummary::Restored {
        files: snapshot.files.len(),
        blocks: snapshot.managed_blocks.len(),
    })
}

/// Platforms whose canonical hook entries to (re-)apply post-restore.
///
/// Modern snapshots: every platform with files under its `dest_dir`. Legacy
/// snapshots (no `hook_bodies`, no per-platform prefix invariant): default to
/// Claude, which shipped first.
fn canonical_targets(snapshot: &Snapshot) -> Vec<&'static Platform> {
    let modern: Vec<_> = PLATFORMS
        .iter()
        .copied()
        .filter(|p| p.is_in_snapshot(snapshot))
        .collect();
    if !modern.is_empty() || !snapshot.hook_bodies.is_empty() {
        return modern;
    }
    vec![&CLAUDE_PLATFORM]
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
        assert!(!tmp.path().join(".gitignore").exists());

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

    /// V-IT-12 (positive half): unload → load round-trip preserves the Ark
    /// hook entry in `.claude/settings.json`. Per ark-context G-11.
    #[test]
    fn roundtrip_preserves_ark_session_start_hook() {
        use crate::io::ARK_CONTEXT_HOOK_COMMAND;

        let tmp = tempfile::tempdir().unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();

        let settings = tmp.path().join(".claude/settings.json");
        let before: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            before["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            serde_json::Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()),
        );

        unload(UnloadOptions::new(tmp.path())).unwrap();
        // After unload the settings file should no longer carry the Ark
        // entry (sibling-empty arrays are fine).
        if settings.exists() {
            let mid: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
            let arr = mid["hooks"]["SessionStart"].as_array();
            assert!(
                arr.is_none_or(|a| !a.iter().any(|e| e["command"]
                    == serde_json::Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()))),
                "Ark entry should be absent after unload"
            );
        }

        load(LoadOptions::new(tmp.path())).unwrap();
        let after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            after["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            serde_json::Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()),
            "Ark entry should be restored after load"
        );
    }

    /// V-IT-15: user-added sibling hooks (e.g. `PreToolUse`) DO survive an
    /// unload → load round-trip because `unload` only surgically removes the
    /// Ark `SessionStart` entry; the rest of `.claude/settings.json` is left
    /// in place on disk. This is better behavior than the original plan
    /// documented (it expected siblings to be lost) — and falls out naturally
    /// from `remove_settings_hook` being a precise edit rather than a
    /// whole-file delete. C-18's "user siblings outside hook_bodies don't
    /// survive" applies only to *capture into the snapshot*; `unload` itself
    /// preserves them on disk.
    #[test]
    fn roundtrip_preserves_user_pretooluse_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();

        let settings = tmp.path().join(".claude/settings.json");
        let mut current: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        current["hooks"]["PreToolUse"] = serde_json::json!([
            {"type": "command", "command": "user-only-hook"}
        ]);
        std::fs::write(
            &settings,
            serde_json::to_string_pretty(&current).unwrap() + "\n",
        )
        .unwrap();

        unload(UnloadOptions::new(tmp.path())).unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();

        let after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(
            after["hooks"]["PreToolUse"][0]["command"],
            serde_json::Value::String("user-only-hook".to_string()),
            "user sibling should survive surgical unload/load",
        );
    }

    /// V-UT-30 carve-out: `load --force` from a directory without an Ark
    /// ancestor scaffolds fresh (no walk-up). The carve-out lives in the CLI
    /// (TargetArgs::resolve), but at the library level the scaffold path
    /// always operates on the explicit target.
    #[test]
    fn load_force_scaffolds_fresh_in_non_ark_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = load(LoadOptions::new(tmp.path()).with_force(true)).unwrap();
        assert!(matches!(summary, LoadSummary::Fresh(_)));
        assert!(tmp.path().join(".ark/workflow.md").is_file());
    }

    /// codex-support C-18: source-scan invariant for `load.rs`.
    #[test]
    fn load_source_no_bare_std_fs_or_dot_path_literals() {
        crate::commands::tests_common::assert_source_clean(include_str!("load.rs"));
    }

    /// V-IT-16 (codex-support G-9, C-22): after `load` replays
    /// `snapshot.hook_bodies`, the canonical re-apply phase rewrites each
    /// installed platform's hook entry to the *current* shape. Even when the
    /// snapshot carries a stale entry (e.g. older `timeout`), post-load disk
    /// state matches the current `entry_builder` output.
    #[test]
    fn load_after_replay_re_applies_canonical_entries() {
        use crate::{
            io::ARK_CONTEXT_HOOK_COMMAND,
            state::{Snapshot, SnapshotHookBody},
        };

        let tmp = tempfile::tempdir().unwrap();
        // Hand-craft a snapshot that mimics a Codex-installed project from
        // an older Ark version. The hook entry here uses a stale `timeout`
        // value (5 instead of the current canonical 30) to prove the
        // canonical re-apply normalizes it.
        let mut snap = Snapshot::new();
        snap.add_file(".codex/skills/ark-quick/SKILL.md", b"# stub\n");
        snap.add_hook_body(SnapshotHookBody {
            path: PathBuf::from(".codex/hooks.json"),
            json_pointer: "/hooks/SessionStart".to_string(),
            identity_key: "command".to_string(),
            identity_value: ARK_CONTEXT_HOOK_COMMAND.to_string(),
            entry: serde_json::json!({
                "matcher": "",
                "hooks": [{
                    "type": "command",
                    "command": ARK_CONTEXT_HOOK_COMMAND,
                    "timeout": 5,
                }],
            }),
        });
        snap.write(tmp.path()).unwrap();

        load(LoadOptions::new(tmp.path())).unwrap();

        let hooks: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(tmp.path().join(".codex/hooks.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            hooks["hooks"]["SessionStart"][0]["hooks"][0]["timeout"],
            serde_json::json!(30),
            "canonical re-apply must normalize stale timeout to current value",
        );
    }
}
