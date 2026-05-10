//! Platform registry — single source of truth for per-platform installation.
//!
//! Each [`Platform`] entry pairs a static template tree (`include_dir!`) with
//! the project-relative paths Ark writes for that platform: where its templates
//! extract, what managed-block file (if any) it owns, what hook file (if any)
//! carries its `SessionStart` entry, and what CLI flag stem (`--<flag>` /
//! `--no-<flag>`) controls per-platform install.
//!
//! Adding a new platform is a registry entry (a `pub const` here plus an
//! addition to [`PLATFORMS`]) plus a new template tree. The command bodies
//! (`init`/`upgrade`/`unload`/`load`/`remove`) iterate this slice via the
//! behavior methods below — they do not grow new arms per platform.

use std::path::Path;

use include_dir::Dir;
use serde_json::Value;

use crate::{
    error::Result,
    io::{
        ARK_CONTEXT_HOOK_COMMAND, HookFileSpec, PathExt, ark_codex_hook_entry,
        ark_session_start_hook_entry, read_hook_file, remove_hook_file, update_hook_file,
        update_managed_block,
    },
    layout::{
        AGENTS_MD, CLAUDE_AGENTS_DIR, CLAUDE_COMMANDS_ARK_DIR, CLAUDE_DIR, CLAUDE_MD,
        CLAUDE_SETTINGS_FILE, CODEX_AGENTS_DIR, CODEX_CONFIG_FILE, CODEX_DIR, CODEX_HOOKS_FILE,
        CODEX_SKILLS_DIR, Layout, MANAGED_BLOCK_BODY, OPENCODE_AGENTS_DIR, OPENCODE_COMMANDS_DIR,
        OPENCODE_DIR, OPENCODE_PLUGIN_FILE,
    },
    state::{Manifest, Snapshot, SnapshotHookBody},
    templates::{
        CLAUDE_AGENT_TEMPLATES, CLAUDE_TEMPLATES, CODEX_AGENT_TEMPLATES, CODEX_CONFIG_TOML,
        CODEX_TEMPLATES, OPENCODE_AGENT_TEMPLATES, OPENCODE_ARK_CONTEXT_TS, OPENCODE_TEMPLATES,
    },
};

/// Describes one coding-agent integration target.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct Platform {
    /// Stores the stable platform id.
    pub id: &'static str,
    /// Embedded template tree, extracted under `dest_dir` of the project root.
    pub templates: &'static Dir<'static>,
    /// Stores the project-relative template extraction directory.
    pub dest_dir: &'static str,
    /// Stores the project-relative directory `remove` wipes wholesale.
    pub removal_root: &'static str,
    /// CLI flag stem: `--<flag>` enables, `--no-<flag>` disables.
    pub cli_flag: &'static str,
    /// Stores the optional managed-block target.
    pub managed_block_target: Option<&'static str>,
    /// Stores the optional `SessionStart` hook descriptor.
    pub hook_file: Option<HookFileSpec>,
    /// Stores whole-file writes that are not hash-tracked.
    pub extra_files: &'static [(&'static str, &'static str)],
    /// Optional embedded agents-template tree, extracted under [`agents_dest_dir`].
    ///
    /// `None` for platforms whose subagent runtime is not yet wired up. Set on
    /// every platform that ships Ark agents. Files extracted from this tree
    /// are written by [`Self::apply_managed_state`] (unconditional, regardless
    /// of `WriteMode`), so reserved Ark-agent stems are always overwritten and
    /// the C-26 reserved-stem invariant holds even on default `init`.
    ///
    /// [`agents_dest_dir`]: Self::agents_dest_dir
    pub agents_templates: Option<&'static Dir<'static>>,
    /// Project-relative directory where [`agents_templates`] extracts.
    ///
    /// `None` iff [`agents_templates`] is `None`.
    ///
    /// [`agents_templates`]: Self::agents_templates
    pub agents_dest_dir: Option<&'static str>,
    /// Project-relative directories owned by this platform that lie OUTSIDE
    /// [`removal_root`].
    ///
    /// Concatenated by [`crate::layout::Layout::owned_dirs`] so `unload` /
    /// `load` round-trip them, and walked by [`Self::remove_dir`] so `remove`
    /// wipes them. Empty for platforms whose extras already nest under
    /// `removal_root`. Claude's narrow `removal_root` (`.claude/commands/ark/`)
    /// requires `[".claude/agents"]` here.
    ///
    /// [`removal_root`]: Self::removal_root
    pub extra_dirs: &'static [&'static str],
}

impl Platform {
    /// Looks up a platform by its stable id (e.g. `"claude-code"`).
    pub fn by_id(id: &str) -> Option<&'static Platform> {
        PLATFORMS.iter().copied().find(|p| p.id == id)
    }

    /// Looks up a platform by its CLI flag stem (e.g. `"claude"`).
    pub fn by_cli_flag(flag: &str) -> Option<&'static Platform> {
        PLATFORMS.iter().copied().find(|p| p.cli_flag == flag)
    }

    /// Returns `true` iff the manifest records this platform's files.
    pub fn is_installed(&self, manifest: &Manifest) -> bool {
        let prefix = Path::new(self.dest_dir);
        manifest.files.iter().any(|p| p.starts_with(prefix))
    }

    /// Returns `true` iff the snapshot contains this platform's files.
    ///
    /// Used by `load` to decide which platforms still need a canonical
    /// hook re-apply post-restore.
    pub fn is_in_snapshot(&self, snapshot: &Snapshot) -> bool {
        let prefix = Path::new(self.dest_dir);
        snapshot.files.iter().any(|f| f.path.starts_with(prefix))
    }

    /// Re-applies this platform's managed state.
    ///
    /// Idempotent — callable from every `init`, `upgrade`, and `load` step
    /// that needs to converge to the canonical shape. Managed blocks, hook
    /// entries, and `extra_files` are not hash-tracked. Agent files (when
    /// `agents_templates` is `Some`) are written unconditionally and recorded
    /// in `manifest.{files, hashes}` so `is_installed` and `is_in_snapshot`
    /// see them; the unconditional write bypasses the upgrade conflict
    /// pipeline (agents are excluded from `collect_desired_templates`).
    pub fn apply_managed_state(&self, layout: &Layout, manifest: &mut Manifest) -> Result<()> {
        if let Some(target) = self.managed_block_target {
            let path = layout.resolve(target);
            if update_managed_block(&path, layout.managed_marker(), MANAGED_BLOCK_BODY)? {
                manifest.record_block(target, layout.managed_marker());
            }
        }
        if let Some(spec) = self.hook_file {
            spec.apply_canonical(layout)?;
        }
        if let (Some(tree), Some(dest)) = (self.agents_templates, self.agents_dest_dir) {
            let dest_root = layout.resolve(dest);
            for entry in crate::templates::walk(tree) {
                let path = dest_root.join(entry.relative_path);
                path.write_bytes(entry.contents)?;
                let relative = path
                    .strip_prefix(layout.root())
                    .expect("agent dest under project root");
                manifest.record_file_with_hash(relative, entry.contents);
            }
        }
        for (rel, body) in self.extra_files {
            layout.resolve(rel).write_bytes(body.as_bytes())?;
        }
        Ok(())
    }

    /// Captures and removes this platform's Ark-owned hook entry.
    ///
    /// Returns the on-disk path for the caller to track in dedupe sets.
    pub fn capture_hook(
        &self,
        layout: &Layout,
        snapshot: &mut Snapshot,
    ) -> Result<Option<std::path::PathBuf>> {
        let Some(spec) = self.hook_file else {
            return Ok(None);
        };
        let absolute = layout.resolve(spec.path);
        let Some(entry) = spec.read(layout)? else {
            return Ok(None);
        };
        snapshot.add_hook_body(SnapshotHookBody {
            path: std::path::PathBuf::from(spec.path),
            json_pointer: format!("/hooks/{}", spec.hooks_array_key),
            identity_key: spec.identity_key.to_string(),
            identity_value: spec.identity_value.to_string(),
            entry,
        });
        spec.remove(layout)?;
        Ok(Some(absolute))
    }

    /// Removes this platform's Ark-owned hook entry.
    ///
    /// Sibling user entries are left in place.
    ///
    /// Returns `Ok(true)` iff an entry was found and removed.
    pub fn remove_hook(&self, layout: &Layout) -> Result<bool> {
        match self.hook_file {
            Some(spec) => spec.remove(layout),
            None => Ok(false),
        }
    }

    /// Wipes this platform's `removal_root` plus the Ark-owned files inside
    /// each `extra_dirs` entry.
    ///
    /// `removal_root` is wholly Ark-owned and removed wholesale. `extra_dirs`
    /// entries (e.g. `.claude/agents/`) are shared with the user; only files
    /// that exactly match an Ark-shipped template inside `agents_templates`
    /// are unlinked, then the directory is removed iff it ends up empty.
    /// User-authored siblings at non-reserved stems survive.
    ///
    /// Returns `Ok(true)` iff any directory or file was removed.
    pub fn remove_dir(&self, layout: &Layout) -> Result<bool> {
        let mut removed = layout.resolve(self.removal_root).remove_dir_all()?;
        for extra in self.extra_dirs {
            let dir = layout.resolve(extra);
            if let Some(tree) = self.agents_templates {
                for entry in crate::templates::walk(tree) {
                    if dir.join(entry.relative_path).remove_if_exists()? {
                        removed = true;
                    }
                }
            }
            if dir.remove_dir_if_empty()? {
                removed = true;
            }
        }
        Ok(removed)
    }
}

impl HookFileSpec {
    /// Builds the canonical Ark entry from this spec.
    pub fn canonical_entry(&self) -> Value {
        (self.entry_builder)()
    }

    /// Inserts or replaces the canonical Ark entry in this hook file.
    pub fn apply_canonical(&self, layout: &Layout) -> Result<bool> {
        update_hook_file(
            layout.resolve(self.path),
            self.canonical_entry(),
            self.hooks_array_key,
            self.identity_key,
        )
    }

    /// Reads the Ark-owned hook entry, if present.
    pub fn read(&self, layout: &Layout) -> Result<Option<Value>> {
        read_hook_file(
            layout.resolve(self.path),
            self.identity_value,
            self.hooks_array_key,
            self.identity_key,
        )
    }

    /// Surgically removes the Ark-owned entry.
    ///
    /// Returns `Ok(true)` iff one was found.
    pub fn remove(&self, layout: &Layout) -> Result<bool> {
        remove_hook_file(
            layout.resolve(self.path),
            self.identity_value,
            self.hooks_array_key,
            self.identity_key,
        )
    }
}

impl SnapshotHookBody {
    /// Replays this captured entry verbatim onto disk.
    ///
    /// Uses [`update_hook_file`] with the captured identity fields.
    ///
    /// The trailing segment of `json_pointer` is the array key; falls
    /// back to `"SessionStart"` for malformed historical pointers.
    pub fn apply(&self, layout: &Layout) -> Result<bool> {
        let array_key = self
            .json_pointer
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("SessionStart");
        update_hook_file(
            layout.resolve_safe(&self.path)?,
            self.entry.clone(),
            array_key,
            &self.identity_key,
        )
    }
}

/// All known platforms, in canonical iteration order.
///
/// Used by `init` / `upgrade` / `unload` / `load` / `remove` to drive
/// per-platform plumbing.
pub const PLATFORMS: &[&Platform] = &[&CLAUDE_PLATFORM, &CODEX_PLATFORM, &OPENCODE_PLATFORM];

/// Iterates platforms whose templates appear in `manifest.files` — i.e.
/// the project has opted into them.
///
/// Preserves the Claude-only-stays-Claude-only invariant for `upgrade`.
pub fn installed<'a>(manifest: &'a Manifest) -> impl Iterator<Item = &'static Platform> + 'a {
    PLATFORMS
        .iter()
        .copied()
        .filter(|p| p.is_installed(manifest))
}

/// Claude Code integration.
///
/// Templates extract under `.claude/`; managed block lives in `CLAUDE.md`;
/// the `SessionStart` hook lives in `.claude/settings.json` and uses
/// milliseconds for `timeout`.
pub const CLAUDE_PLATFORM: Platform = Platform {
    id: "claude-code",
    templates: &CLAUDE_TEMPLATES,
    dest_dir: CLAUDE_DIR,
    removal_root: CLAUDE_COMMANDS_ARK_DIR,
    cli_flag: "claude",
    managed_block_target: Some(CLAUDE_MD),
    hook_file: Some(HookFileSpec {
        path: CLAUDE_SETTINGS_FILE,
        hooks_array_key: "SessionStart",
        identity_key: "command",
        identity_value: ARK_CONTEXT_HOOK_COMMAND,
        entry_builder: ark_session_start_hook_entry,
    }),
    extra_files: &[],
    // Claude's `templates` tree is rooted at `templates/claude/` and physically
    // contains both `commands/` and `agents/`. The main extract path filters
    // out files whose dest falls under `agents_dest_dir` so agents are NOT
    // written through `WriteMode::Skip`-honoring `init::extract`; instead
    // `apply_managed_state` writes them unconditionally via `path.write_bytes`,
    // so the C-26 reserved-stem invariant holds on default `init`.
    agents_templates: Some(&CLAUDE_AGENT_TEMPLATES),
    agents_dest_dir: Some(CLAUDE_AGENTS_DIR),
    // `.claude/agents/` lies outside `removal_root` (`.claude/commands/ark/`),
    // so it must be tracked here for `unload`/`load`/`remove` round-trip.
    extra_dirs: &[CLAUDE_AGENTS_DIR],
};

/// OpenAI Codex CLI integration.
///
/// Templates extract under `.codex/`; managed block lives in `AGENTS.md`;
/// the `SessionStart` hook lives in `.codex/hooks.json` and uses **seconds**
/// for `timeout` (Codex schema differs from Claude).
pub const CODEX_PLATFORM: Platform = Platform {
    id: "codex",
    templates: &CODEX_TEMPLATES,
    dest_dir: CODEX_SKILLS_DIR,
    removal_root: CODEX_DIR,
    cli_flag: "codex",
    managed_block_target: Some(AGENTS_MD),
    hook_file: Some(HookFileSpec {
        path: CODEX_HOOKS_FILE,
        hooks_array_key: "SessionStart",
        identity_key: "command",
        identity_value: ARK_CONTEXT_HOOK_COMMAND,
        entry_builder: ark_codex_hook_entry,
    }),
    extra_files: &[(CODEX_CONFIG_FILE, CODEX_CONFIG_TOML)],
    // Codex's main `templates` tree is rooted at `templates/codex/skills/`,
    // so the agents subtree needs a dedicated static.
    agents_templates: Some(&CODEX_AGENT_TEMPLATES),
    agents_dest_dir: Some(CODEX_AGENTS_DIR),
    // `.codex/agents/` lives under `removal_root = .codex`, so no extra entry.
    extra_dirs: &[],
};

/// OpenCode integration.
///
/// Templates extract under `.opencode/`; the managed block shares
/// `AGENTS.md` with Codex (manifest dedupes on `(file, marker)`).
/// `SessionStart`-equivalent context injection rides a Bun-loaded TS
/// plugin shipped via `extra_files`; OpenCode has no native JSON hook
/// surface.
pub const OPENCODE_PLATFORM: Platform = Platform {
    id: "opencode",
    templates: &OPENCODE_TEMPLATES,
    dest_dir: OPENCODE_COMMANDS_DIR,
    removal_root: OPENCODE_DIR,
    cli_flag: "opencode",
    managed_block_target: Some(AGENTS_MD),
    hook_file: None,
    extra_files: &[(OPENCODE_PLUGIN_FILE, OPENCODE_ARK_CONTEXT_TS)],
    // OpenCode's main `templates` tree is rooted at
    // `templates/opencode/commands/`, so the agents subtree needs a dedicated
    // static.
    agents_templates: Some(&OPENCODE_AGENT_TEMPLATES),
    agents_dest_dir: Some(OPENCODE_AGENTS_DIR),
    // `.opencode/agents/` lives under `removal_root = .opencode`, so no extra
    // entry.
    extra_dirs: &[],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that the registry has three entries in canonical order.
    #[test]
    fn platforms_registry_has_three_entries_in_canonical_order() {
        assert_eq!(PLATFORMS.len(), 3);
        assert_eq!(PLATFORMS[0].id, "claude-code");
        assert_eq!(PLATFORMS[1].id, "codex");
        assert_eq!(PLATFORMS[2].id, "opencode");
    }

    /// Verifies `by_id` resolution and miss behavior.
    #[test]
    fn platform_by_id_resolves_known_platforms() {
        assert_eq!(
            Platform::by_id("claude-code").map(|p| p.id),
            Some("claude-code")
        );
        assert_eq!(Platform::by_id("codex").map(|p| p.id), Some("codex"));
        assert_eq!(Platform::by_id("opencode").map(|p| p.id), Some("opencode"));
        assert!(Platform::by_id("unknown").is_none());
    }

    /// Verifies that `by_cli_flag` resolves all three platforms.
    #[test]
    fn platform_by_cli_flag_resolves() {
        assert_eq!(
            Platform::by_cli_flag("claude").map(|p| p.id),
            Some("claude-code")
        );
        assert_eq!(Platform::by_cli_flag("codex").map(|p| p.id), Some("codex"));
        assert_eq!(
            Platform::by_cli_flag("opencode").map(|p| p.id),
            Some("opencode")
        );
        assert!(Platform::by_cli_flag("nope").is_none());
    }

    /// Verifies the [`OPENCODE_PLATFORM`] shape.
    ///
    /// `dest_dir` is the commands subtree (parallel to Codex's
    /// `.codex/skills/` so [`Platform::templates`] extracts cleanly under
    /// it); `removal_root` is the parent `.opencode/` so removal wipes
    /// both `commands/` and `plugins/`.
    #[test]
    fn opencode_platform_shape() {
        assert_eq!(OPENCODE_PLATFORM.id, "opencode");
        assert_eq!(OPENCODE_PLATFORM.cli_flag, "opencode");
        assert_eq!(OPENCODE_PLATFORM.dest_dir, OPENCODE_COMMANDS_DIR);
        assert_eq!(OPENCODE_PLATFORM.removal_root, OPENCODE_DIR);
        assert_eq!(OPENCODE_PLATFORM.managed_block_target, Some(AGENTS_MD));
        assert!(OPENCODE_PLATFORM.hook_file.is_none());
        assert_eq!(OPENCODE_PLATFORM.extra_files.len(), 1);
        assert_eq!(OPENCODE_PLATFORM.extra_files[0].0, OPENCODE_PLUGIN_FILE);
        assert_eq!(OPENCODE_PLATFORM.extra_files[0].1, OPENCODE_ARK_CONTEXT_TS);
    }

    /// Verifies OpenCode managed-state application.
    #[test]
    fn opencode_apply_managed_state_writes_block_and_plugin() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let mut manifest = Manifest::new();

        OPENCODE_PLATFORM
            .apply_managed_state(&layout, &mut manifest)
            .unwrap();

        // Managed block in AGENTS.md, recorded in the manifest.
        let agents = std::fs::read_to_string(layout.agents_md()).unwrap();
        assert!(agents.contains("<!-- ARK:START -->"));
        assert!(
            manifest
                .managed_blocks
                .iter()
                .any(|b| b.marker == "ARK" && b.file.ends_with(AGENTS_MD))
        );

        // Plugin file byte-identical to the embedded constant.
        let plugin = std::fs::read_to_string(layout.opencode_plugin_file()).unwrap();
        assert_eq!(plugin, OPENCODE_ARK_CONTEXT_TS);

        // No hook file plumbing was triggered.
        assert!(OPENCODE_PLATFORM.hook_file.is_none());
    }

    /// Verifies shared `AGENTS.md` block dedupe.
    #[test]
    fn shared_agents_block_deduped_when_both_platforms_apply() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let mut manifest = Manifest::new();

        CODEX_PLATFORM
            .apply_managed_state(&layout, &mut manifest)
            .unwrap();
        OPENCODE_PLATFORM
            .apply_managed_state(&layout, &mut manifest)
            .unwrap();

        let agents_blocks: Vec<_> = manifest
            .managed_blocks
            .iter()
            .filter(|b| b.marker == "ARK" && b.file.ends_with(AGENTS_MD))
            .collect();
        assert_eq!(
            agents_blocks.len(),
            1,
            "block must be recorded exactly once"
        );

        // The on-disk file contains exactly one `<!-- ARK:START -->` marker.
        let agents = std::fs::read_to_string(layout.agents_md()).unwrap();
        let starts: Vec<_> = agents.match_indices("<!-- ARK:START -->").collect();
        assert_eq!(starts.len(), 1, "AGENTS.md must contain one block");
    }

    /// Verifies that `capture_hook` returns `None` for OpenCode.
    #[test]
    fn opencode_capture_hook_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let mut snapshot = Snapshot::new();
        assert_eq!(
            OPENCODE_PLATFORM
                .capture_hook(&layout, &mut snapshot)
                .unwrap(),
            None,
        );
        assert!(snapshot.hook_bodies.is_empty());
    }

    /// Verifies that `remove_hook` returns `false` for OpenCode.
    #[test]
    fn opencode_remove_hook_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert!(!OPENCODE_PLATFORM.remove_hook(&layout).unwrap());
    }

    /// Verifies the `remove_dir` round-trip.
    #[test]
    fn opencode_remove_dir_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert!(!OPENCODE_PLATFORM.remove_dir(&layout).unwrap());

        let mut manifest = Manifest::new();
        OPENCODE_PLATFORM
            .apply_managed_state(&layout, &mut manifest)
            .unwrap();
        assert!(layout.opencode_dir().exists());
        assert!(OPENCODE_PLATFORM.remove_dir(&layout).unwrap());
        assert!(!layout.opencode_dir().exists());
    }

    /// Verifies the canonical Codex hook entry.
    #[test]
    fn ark_codex_hook_entry_carries_canonical_command_in_seconds() {
        let entry = ark_codex_hook_entry();
        assert_eq!(
            entry["hooks"][0]["command"],
            Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()),
        );
        assert_eq!(entry["hooks"][0]["timeout"], serde_json::json!(30));
    }

    /// Verifies Codex managed-state application.
    #[test]
    fn codex_apply_managed_state_writes_block_hook_and_extras() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let mut manifest = Manifest::new();

        CODEX_PLATFORM
            .apply_managed_state(&layout, &mut manifest)
            .unwrap();

        // Managed block in AGENTS.md, recorded in the manifest.
        let agents = std::fs::read_to_string(layout.agents_md()).unwrap();
        assert!(agents.contains("<!-- ARK:START -->"));
        assert!(
            manifest
                .managed_blocks
                .iter()
                .any(|b| b.marker == "ARK" && b.file.ends_with(AGENTS_MD))
        );

        // Hook file carries the canonical SessionStart entry.
        let hooks: Value =
            serde_json::from_str(&std::fs::read_to_string(layout.codex_hooks_file()).unwrap())
                .unwrap();
        assert_eq!(
            hooks["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()),
        );

        // extra_files written verbatim.
        let cfg = std::fs::read_to_string(layout.codex_config_file()).unwrap();
        assert_eq!(cfg, CODEX_CONFIG_TOML);
    }

    /// Verifies Codex hook capture and removal.
    #[test]
    fn codex_capture_hook_captures_then_removes_only_ark_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());

        // Set up a hook file with the Ark entry plus a user sibling.
        let hooks_path = layout.codex_hooks_file();
        std::fs::create_dir_all(hooks_path.parent().unwrap()).unwrap();
        std::fs::write(
            &hooks_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "hooks": {
                    "SessionStart": [
                        ark_codex_hook_entry(),
                        {
                            "matcher": "",
                            "hooks": [{ "type": "command", "command": "user-sibling" }]
                        }
                    ]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let mut snapshot = Snapshot::new();
        let captured_path = CODEX_PLATFORM.capture_hook(&layout, &mut snapshot).unwrap();
        assert_eq!(captured_path, Some(hooks_path.clone()));
        assert_eq!(snapshot.hook_bodies.len(), 1);
        assert_eq!(
            snapshot.hook_bodies[0].identity_value,
            ARK_CONTEXT_HOOK_COMMAND
        );

        // The Ark entry is gone from disk; the user sibling survives.
        let after: Value =
            serde_json::from_str(&std::fs::read_to_string(&hooks_path).unwrap()).unwrap();
        let arr = after["hooks"]["SessionStart"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["hooks"][0]["command"], "user-sibling");
    }

    /// Verifies hook capture miss behavior.
    #[test]
    fn capture_hook_is_none_when_file_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let mut snapshot = Snapshot::new();
        assert_eq!(
            CODEX_PLATFORM.capture_hook(&layout, &mut snapshot).unwrap(),
            None
        );
        assert!(snapshot.hook_bodies.is_empty());
    }

    /// Verifies `remove_dir` return values.
    #[test]
    fn remove_dir_returns_whether_anything_was_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert!(!CODEX_PLATFORM.remove_dir(&layout).unwrap());

        std::fs::create_dir_all(layout.codex_dir().join("skills/ark-quick")).unwrap();
        std::fs::write(
            layout.codex_dir().join("skills/ark-quick/SKILL.md"),
            b"stub\n",
        )
        .unwrap();
        assert!(CODEX_PLATFORM.remove_dir(&layout).unwrap());
        assert!(!layout.codex_dir().exists());
    }

    /// Verifies `remove_hook` return values.
    #[test]
    fn remove_hook_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert!(!CODEX_PLATFORM.remove_hook(&layout).unwrap());

        let hooks_path = layout.codex_hooks_file();
        std::fs::create_dir_all(hooks_path.parent().unwrap()).unwrap();
        std::fs::write(
            &hooks_path,
            serde_json::to_string_pretty(&serde_json::json!({
                "hooks": { "SessionStart": [ark_codex_hook_entry()] }
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(CODEX_PLATFORM.remove_hook(&layout).unwrap());
    }

    /// Enforces the source-scan invariant for `platforms.rs`.
    #[test]
    fn platforms_source_no_bare_std_fs_or_dot_paths() {
        crate::commands::tests_common::assert_source_clean(include_str!("platforms.rs"));
    }

    /// Verifies that `agents_templates` and `agents_dest_dir` are paired:
    /// either both `Some` or both `None`.
    #[test]
    fn agents_templates_and_dest_dir_are_paired() {
        for p in PLATFORMS {
            assert_eq!(
                p.agents_templates.is_some(),
                p.agents_dest_dir.is_some(),
                "platform `{}`: `agents_templates` and `agents_dest_dir` must be both Some or \
                 both None",
                p.id,
            );
        }
    }

    /// Verifies that `Layout::owned_dirs()` is derived from `PLATFORMS`.
    ///
    /// The expected set is `{ark_dir} ∪ {p.removal_root for p in PLATFORMS} ∪
    /// {extra for p in PLATFORMS for extra in p.extra_dirs}`. A regression
    /// where a maintainer hard-codes the entries while keeping the registry
    /// fields unchanged would still pass round-trip tests; this test catches
    /// that drift directly.
    #[test]
    fn owned_dirs_derives_from_registry() {
        let layout = Layout::new(std::path::Path::new("/proj"));
        let actual: std::collections::BTreeSet<_> = layout.owned_dirs().into_iter().collect();
        let mut expected = std::collections::BTreeSet::new();
        expected.insert(layout.ark_dir());
        for p in PLATFORMS {
            expected.insert(layout.resolve(p.removal_root));
            for extra in p.extra_dirs {
                expected.insert(layout.resolve(extra));
            }
        }
        assert_eq!(
            actual, expected,
            "`Layout::owned_dirs()` must equal the registry-derived set"
        );
    }

    /// Verifies that Claude's `extra_dirs` carries `.claude/agents/`.
    ///
    /// Claude's `removal_root` is the narrow `.claude/commands/ark/`, so
    /// `.claude/agents/` would otherwise escape `unload`/`load`/`remove`
    /// round-trip. This is the exact regression that motivated `extra_dirs`.
    #[test]
    fn claude_platform_lists_agents_dir_in_extra_dirs() {
        assert!(
            CLAUDE_PLATFORM.extra_dirs.contains(&CLAUDE_AGENTS_DIR),
            "CLAUDE_PLATFORM.extra_dirs must include CLAUDE_AGENTS_DIR ({CLAUDE_AGENTS_DIR})"
        );
    }

    /// Verifies that `remove_dir` spares user-authored agents at non-reserved
    /// stems while still wiping every reserved Ark agent. Closes the data-loss
    /// gap where `.claude/agents/` was previously `remove_dir_all`'d.
    #[test]
    fn remove_dir_preserves_user_authored_agents_at_non_reserved_stems() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());

        // Set up a Claude install with both Ark agents and a user agent.
        let agents = layout.resolve(CLAUDE_AGENTS_DIR);
        agents.ensure_dir().unwrap();
        layout
            .resolve(CLAUDE_COMMANDS_ARK_DIR)
            .ensure_dir()
            .unwrap();
        agents
            .join("ark-researcher.md")
            .write_bytes(b"ark")
            .unwrap();
        agents.join("ark-reviewer.md").write_bytes(b"ark").unwrap();
        agents.join("ark-verifier.md").write_bytes(b"ark").unwrap();
        let user_path = agents.join("my-custom.md");
        user_path.write_bytes(b"USER OWNED").unwrap();

        assert!(CLAUDE_PLATFORM.remove_dir(&layout).unwrap());
        assert!(
            user_path.exists(),
            "user-authored agent at non-reserved stem must survive `remove`"
        );
        assert_eq!(std::fs::read(&user_path).unwrap(), b"USER OWNED");
        for stem in ["ark-researcher.md", "ark-reviewer.md", "ark-verifier.md"] {
            assert!(
                !agents.join(stem).exists(),
                "reserved Ark agent `{stem}` must be removed"
            );
        }
    }
}
