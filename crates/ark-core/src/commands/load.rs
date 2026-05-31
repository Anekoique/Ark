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
    state::{Manifest, Snapshot},
};

/// Options for loading Ark into a project.
#[derive(Debug, Clone)]
pub struct LoadOptions {
    /// Project root where Ark should be loaded.
    pub project_root: PathBuf,
    /// Reports whether an existing live Ark footprint should be replaced.
    pub force: bool,
}

impl LoadOptions {
    /// Creates load options for `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            force: false,
        }
    }

    /// Sets force mode.
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

/// Outcome of `load`. Each variant carries its own relevant counters.
///
/// `Clone` only — `InitSummary` carries an owned `Option<String>` (the
/// bootstrapped developer name) so `Copy` no longer applies.
#[derive(Debug, Clone)]
pub enum LoadSummary {
    /// Fresh scaffold from embedded templates (no snapshot was present).
    Fresh(InitSummary),
    /// Restored from a pre-existing `.ark.db` snapshot.
    Restored {
        /// Number of files restored from the snapshot.
        files: usize,
        /// Number of managed blocks restored from the snapshot.
        blocks: usize,
    },
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

/// Loads Ark into `opts.project_root`.
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

    // Restore Ark-owned hook entries. Replay each captured entry, then
    // overwrite with the canonical shape so the on-disk hook is independent
    // of snapshot age. For legacy snapshots (no `hook_bodies`) we treat
    // Claude as installed-by-default — Claude shipped first and predates
    // the manifest-prefix invariant.
    for hb in &snapshot.hook_bodies {
        hb.apply(layout)?;
    }
    let mut manifest = Manifest::read(layout.root())?.unwrap_or_default();
    for platform in canonical_targets(&snapshot) {
        // `apply_managed_state` re-overwrites managed blocks, hook entries,
        // `extra_files`, and (when present) the agents subtree from the
        // embedded canonical bodies. This guarantees reserved Ark stems
        // (C-26) get the canonical text on `load` even if the snapshot
        // captured user-edited bytes.
        platform.apply_managed_state(layout, &mut manifest)?;
    }
    manifest.write(layout.root())?;

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

    /// Verifies Ark hook preservation across unload and load.
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

    /// Verifies user-added sibling hook preservation.
    ///
    /// `unload` only surgically removes the Ark `SessionStart` entry; the rest
    /// of `.claude/settings.json` is left in place on disk.
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

    /// Verifies that `load --force` without an Ark ancestor scaffolds fresh.
    #[test]
    fn load_force_scaffolds_fresh_in_non_ark_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = load(LoadOptions::new(tmp.path()).with_force(true)).unwrap();
        assert!(matches!(summary, LoadSummary::Fresh(_)));
        assert!(tmp.path().join(".ark/workflow.md").is_file());
    }

    /// Enforces the source-scan invariant for `load.rs`.
    #[test]
    fn load_source_no_bare_std_fs_or_dot_path_literals() {
        crate::commands::tests_common::assert_source_clean(include_str!("load.rs"));
    }

    /// Verifies canonical hook re-apply after snapshot replay.
    ///
    /// Even when the snapshot carries a stale entry, post-load disk state
    /// matches the current `entry_builder` output.
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

    /// Verifies the full three-platform round-trip.
    ///
    /// The restored project has all three trees, one shared `AGENTS.md` block,
    /// and a byte-identical plugin file.
    #[test]
    fn opencode_three_platform_roundtrip() {
        use crate::templates::OPENCODE_ARK_CONTEXT_TS;
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();

        // Sanity: all three trees + shared AGENTS.md block exist post-init.
        for path in [
            ".claude/commands/ark/quick.md",
            ".codex/skills/ark-quick/SKILL.md",
            ".opencode/commands/ark/quick.md",
            ".opencode/plugins/ark-context.ts",
        ] {
            assert!(tmp.path().join(path).is_file(), "missing post-init: {path}");
        }
        let agents_before = std::fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
        let starts: Vec<_> = agents_before.match_indices("<!-- ARK:START -->").collect();
        assert_eq!(starts.len(), 1, "AGENTS.md must contain exactly one block");

        let plugin_before =
            std::fs::read_to_string(tmp.path().join(".opencode/plugins/ark-context.ts")).unwrap();
        assert_eq!(plugin_before, OPENCODE_ARK_CONTEXT_TS);

        unload(UnloadOptions::new(tmp.path())).unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();

        // All three trees restored.
        for path in [
            ".claude/commands/ark/quick.md",
            ".codex/skills/ark-quick/SKILL.md",
            ".opencode/commands/ark/quick.md",
            ".opencode/plugins/ark-context.ts",
        ] {
            assert!(tmp.path().join(path).is_file(), "missing post-load: {path}");
        }

        // AGENTS.md block reapplied exactly once (no double-write from
        // Codex+OpenCode both targeting it).
        let agents_after = std::fs::read_to_string(tmp.path().join("AGENTS.md")).unwrap();
        let starts_after: Vec<_> = agents_after.match_indices("<!-- ARK:START -->").collect();
        assert_eq!(
            starts_after.len(),
            1,
            "AGENTS.md must still contain one block"
        );

        // Plugin file byte-identical (re-applied via extra_files canonical write).
        let plugin_after =
            std::fs::read_to_string(tmp.path().join(".opencode/plugins/ark-context.ts")).unwrap();
        assert_eq!(plugin_after, OPENCODE_ARK_CONTEXT_TS);
    }

    /// Verifies that `unload` then `load` round-trips agent files for every
    /// platform.
    ///
    /// Claude is the load-bearing case: its narrow `removal_root`
    /// (`.claude/commands/ark/`) does not cover `.claude/agents/`, so this
    /// test exercises the registry-derived `Layout::owned_dirs()` path.
    #[test]
    fn unload_load_round_trips_agent_files_for_every_platform() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();

        // Capture canonical bytes before unload for byte-equality assertion.
        let before: Vec<(std::path::PathBuf, Vec<u8>)> = [
            ".claude/agents/ark-researcher.md",
            ".claude/agents/ark-reviewer.md",
            ".claude/agents/ark-verifier.md",
            ".codex/agents/ark-researcher.toml",
            ".codex/agents/ark-reviewer.toml",
            ".codex/agents/ark-verifier.toml",
            ".opencode/agents/ark-researcher.md",
            ".opencode/agents/ark-reviewer.md",
            ".opencode/agents/ark-verifier.md",
        ]
        .iter()
        .map(|p| {
            let abs = tmp.path().join(p);
            (PathBuf::from(p), std::fs::read(&abs).unwrap())
        })
        .collect();

        unload(UnloadOptions::new(tmp.path())).unwrap();
        for (rel, _) in &before {
            assert!(
                !tmp.path().join(rel).exists(),
                "agent file `{}` must be removed by unload",
                rel.display(),
            );
        }

        load(LoadOptions::new(tmp.path())).unwrap();
        for (rel, expected) in &before {
            let after = std::fs::read(tmp.path().join(rel)).unwrap();
            assert_eq!(
                after,
                *expected,
                "agent file `{}` must be byte-identical after load",
                rel.display(),
            );
        }
    }

    /// Verifies that `load` restores the canonical agent body even when the
    /// snapshot captured user-edited bytes (the C-26 reserved-stem invariant
    /// must hold on the load path, not just `init`/`upgrade`).
    #[test]
    fn load_restores_canonical_agent_body_overwriting_snapshot_edits() {
        let tmp = tempfile::tempdir().unwrap();
        crate::commands::init(crate::commands::InitOptions::new(tmp.path())).unwrap();

        // Capture canonical bytes for the assertion.
        let agent = tmp.path().join(".claude/agents/ark-researcher.md");
        let canonical = std::fs::read(&agent).unwrap();

        // User edits the reserved-stem agent before unload.
        std::fs::write(&agent, b"USER EDIT BEFORE UNLOAD").unwrap();
        unload(UnloadOptions::new(tmp.path())).unwrap();
        load(LoadOptions::new(tmp.path())).unwrap();

        let restored = std::fs::read(&agent).unwrap();
        assert_eq!(
            restored, canonical,
            "load must overwrite snapshot-captured edits at reserved stems with the canonical body"
        );
    }
}
