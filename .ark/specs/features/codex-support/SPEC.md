
[**Goals**]

- G-1: `Platform` registry is the single source of truth for per-platform installation. `init` / `upgrade` / `unload` / `load` / `remove` iterate the same `&[&Platform]` slice; adding a third platform later is a registry entry, not a refactor of the command bodies.
- G-2: Two platforms ship in this task: `claude-code` and `codex`. Each carries a static template tree (`include_dir!`), a config-dir name, a managed-block target file, an optional `HookFileSpec` descriptor, and a `cli_flag` string.
- G-3: `ark init` accepts `--claude` / `--codex` / `--no-claude` / `--no-codex` flags. With no platform flags, on a TTY, an interactive prompt asks which platforms to install; both checked by default. **With no flags on a non-TTY, init errors with a message naming the available flags** (per R-007). Explicit `--no-X` flags suppress prompting and install only what's selected.
- G-4: `init` ships Codex artifacts at canonical paths: `.codex/skills/ark-quick/SKILL.md`, `.codex/skills/ark-design/SKILL.md`, `.codex/skills/ark-archive/SKILL.md`, `.codex/hooks.json`, `.codex/config.toml`. Skills carry YAML frontmatter (`name: ark-quick`, `description: ...`) so Codex's description-based routing surfaces them when the user describes a quick-tier task. The skill body is a mechanical translation of the matching `templates/claude/commands/ark/<name>.md` body: drop Claude's frontmatter, prepend Codex's `name`/`description` frontmatter, rewrite any inline `/ark:foo` to `ark-foo` skill references.
- G-5: `init` installs an `AGENTS.md` managed block (marker `ARK`) when Codex is selected, parallel to `CLAUDE.md`'s. File created if absent. Body identical to `CLAUDE.md`'s (reuses `MANAGED_BLOCK_BODY`).
- G-6: `update_settings_hook` / `remove_settings_hook` / `read_settings_hook` are renamed `update_hook_file` / `remove_hook_file` / `read_hook_file` and parameterized over `hooks_array_key: &str` (the key under `hooks` whose array carries the Ark entry — both shipping platforms pass `"SessionStart"`). The `identity_key` parameter is also added (both pass `"command"`). No JSON Pointer parser. Per-platform constants `ark_session_start_hook_entry()` (Claude) and `ark_codex_hook_entry()` (Codex) build the canonical entries.
- G-7: `.codex/hooks.json` and `.codex/config.toml` are NOT hash-tracked. Re-applied unconditionally on every `init` / `load` / `upgrade`. Mirrors the Claude `settings.json` precedent (ark-context C-17, ark-upgrade C-8).
- G-8: `.codex/hooks.json` supports sibling user hooks. Ark's entry is identified by `command == "ark context --scope session --format json"`; surgical upserts and removals via `update_hook_file` / `remove_hook_file` preserve any other entries the user adds. Same contract as `.claude/settings.json`.
- G-9: `Snapshot::hook_bodies` captures the Codex hook entry on `unload` alongside the Claude one. `load` re-applies both via `update_hook_file(path, body.entry, ...)`, **then iterates `PLATFORMS` and re-applies the canonical entry per `Platform::hook_file.entry_builder`** (per C-22). Round-trip: install both platforms → unload → load → byte-identical (modulo timestamps). Older `.ark.db` files without a Codex entry deserialize to `vec![]` for that slot; the canonical re-apply restores both platforms regardless.
- G-10: `remove` removes both platforms' SessionStart hook entries (surgically, leaving sibling user entries) and both managed blocks (`CLAUDE.md` and `AGENTS.md`). Manifest-driven — `remove` reads `manifest.managed_blocks` to know which blocks to remove; new code records the `AGENTS.md` block in `init` so this works.
- G-11: `upgrade` re-applies `CLAUDE.md` block, `AGENTS.md` block (only on Codex-installed projects), Claude SessionStart hook (only on Claude-installed projects), Codex SessionStart hook (only on Codex-installed projects), and `.codex/config.toml` (only on Codex-installed projects) on every run. None are hash-tracked. Sibling user content (other CLAUDE.md sections, other hooks.json entries) is preserved. **A platform is considered "installed" iff some path under `Platform::dest_dir` appears in `manifest.files`.**
- G-12: Two parity tests pin the Claude/Codex template surfaces in lockstep. (a) `every_claude_command_has_a_codex_skill_sibling` asserts that for every `templates/claude/commands/ark/<name>.md`, the file `templates/codex/skills/ark-<name>/SKILL.md` exists — adding a Claude command without a Codex twin is a test-time failure. (b) `codex_skill_bodies_have_codex_frontmatter_not_claude_frontmatter` asserts each shipped skill begins with `---\nname: ark-` (Codex frontmatter), not Claude's `description:`/`argument-hint:` shape — a copy-paste regression where a Claude body lands in the Codex tree fails this. Body-content parity is *not* mechanically asserted: Claude bodies use slash-invocation idioms (`# /ark:quick $ARGUMENTS`) that don't translate to Codex's description-routed model, and a strict byte-equality check would over-constrain intentional divergence. The two tests together catch the failure modes that matter (missing twin, wrong frontmatter); deeper drift is policed by code review at template-edit time.
- G-13: `Layout` gains `codex_dir()`, `codex_skills_dir()`, `codex_hooks_file()`, `codex_config_file()`, `agents_md()`. `CODEX_DIR`, `CODEX_SKILLS_DIR`, `CODEX_HOOKS_FILE`, `CODEX_CONFIG_FILE`, `AGENTS_MD` consts. `owned_dirs()` returns `.ark`, `.claude/commands/ark`, `.codex` (the last only relevant to Codex-installed projects; on a Claude-only install, the dir doesn't exist and `walk_files` yields empty).
- **G-14 (NEW)**: An existing Claude-only project upgraded with the new CLI version remains Claude-only on `ark upgrade`. To add Codex, the user re-runs `ark init --codex` (additive — installs Codex artifacts + records them in the manifest; idempotent on Claude artifacts). This works because `init` is idempotent and platform-keyed iteration on a per-flag basis is a no-op for unselected platforms.

[**Non-goals**]

- NG-1: No Codex `.codex/agents/*.toml` custom subagents. Trellis ships them; Ark has no Claude-side equivalent.
- NG-2: No `.codex/prompts/` slash-command files. Not project-scope-discoverable on current Codex (openai/codex#9848 is open). Revisit when upstream lands.
- NG-3: No third platform (Cursor, OpenCode, Gemini). Registry must leave room.
- NG-4: No per-prompt user customization at install time.
- NG-5: No shared-body templating with includes. The G-12 content-parity test holds the two trees in sync byte-for-byte modulo the documented rewrite.
- NG-6: No detection of which CLI tools are installed on the user's machine to drive default selection.
- NG-7: No `AGENTS.md` rewrite to make it the canonical project doc.
- NG-8: No change to `ark context`'s output shape.
- NG-9: No changes to slash-command (`/ark:quick`, `/ark:design`, `/ark:archive`) Claude bodies in this task.
- NG-10 (NEW): No JSON Pointer parser. The hook helper takes a single key; future divergence in JSON shape is a parser ticket then, not now.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                       — InitArgs gains 4 platform flags + interactive
│                                                selection + non-TTY error path
└── ark-core/src/
    ├── lib.rs                                 — re-exports Platform registry types
    ├── platforms.rs                           — NEW. Platform struct + PLATFORMS slice +
    │                                             CLAUDE_PLATFORM / CODEX_PLATFORM consts
    ├── layout.rs                              — adds codex_dir, codex_skills_dir,
    │                                             codex_hooks_file, codex_config_file,
    │                                             agents_md, AGENTS_MD const,
    │                                             CODEX_DIR / CODEX_SKILLS_DIR /
    │                                             CODEX_HOOKS_FILE / CODEX_CONFIG_FILE consts;
    │                                             owned_dirs extended to 3 entries
    ├── io/
    │   └── fs.rs                              — settings-hook helpers renamed +
    │                                             parameterized over (hooks_array_key,
    │                                             identity_key). Old names retained as
    │                                             #[deprecated] thin wrappers (0.2.0 → 0.3.0).
    │                                             ark_codex_hook_entry() with seconds-unit
    │                                             constant 30.
    ├── state/
    │   └── snapshot.rs                        — unchanged
    ├── commands/
    │   ├── init.rs                            — accepts InitOptions::platforms
    │   ├── upgrade.rs                          — collect_desired_templates iterates
    │   │                                        manifest-recorded platform set; hook +
    │   │                                        config refresh runs per-installed-platform
    │   ├── unload.rs                           — captures hook entries by file-presence +
    │   │                                        identity scan (R-006 / C-24)
    │   ├── load.rs                             — re-applies hook_bodies, then re-applies
    │   │                                        canonical entries per PLATFORMS (C-22)
    │   └── remove.rs                           — removes hook entries from every platform;
    │                                             owned_dirs destructure replaced
    └── templates.rs                            — adds CODEX_TEMPLATES static
templates/
├── ark/                                        — unchanged
├── claude/                                     — unchanged
└── codex/                                      — NEW
    ├── skills/                                 — Codex's project-scope mechanism
    │   ├── ark-quick/SKILL.md                 — frontmatter + body of claude/quick.md
    │   ├── ark-design/SKILL.md                 — ditto
    │   └── ark-archive/SKILL.md                — ditto
    ├── hooks.json                              — SessionStart entry (timeout: 30, seconds)
    └── config.toml                             — project_doc_fallback_filenames = ["AGENTS.md"]
```

**Module coupling.** Unchanged from 00.

**Call graph for `init` (post-refactor):** Unchanged from 00.

**Call graph for `unload` (revised — C-24):**

```
unload(opts)
  ├── walk every owned_dir, capture into Snapshot::files
  ├── for block in manifest.managed_blocks:                — captures both CLAUDE.md and AGENTS.md
  │     read_managed_block + remove_managed_block
  ├── for platform in PLATFORMS:                           — capture by registered file
  │     if platform has hook_file AND file exists:
  │         read_hook_file → snapshot.add_hook_body
  │         remove_hook_file
  ├── for json_file in walk_files(owned_dirs).filter(.json):  — C-24: scan unregistered files
  │     for entry in scan_session_start_array(json_file):
  │         if entry.hooks[*].command == ARK_CONTEXT_HOOK_COMMAND
  │            AND json_file not already captured above:
  │             snapshot.add_hook_body
  │             remove the entry (surgically)
  ├── snapshot.write
  └── delete owned_dirs
```

**Call graph for `load` (revised — C-22):**

```
load(opts)
  ├── read snapshot
  ├── restore snapshot.files
  ├── re-apply each managed block via update_managed_block
  ├── for body in snapshot.hook_bodies:                    — replay captured entries
  │     update_hook_file(body.path, body.entry,
  │                      derive_array_key(body.json_pointer),
  │                      body.identity_key)
  ├── for platform in PLATFORMS:                           — C-22: canonical re-apply
  │     if platform.hook_file.is_some()
  │        AND platform.dest_dir appears in restored files:
  │         spec = platform.hook_file.unwrap()
  │         update_hook_file(layout.resolve(spec.path),
  │                          (spec.entry_builder)(),
  │                          spec.hooks_array_key,
  │                          spec.identity_key)
  ├── regenerate manifest hashes from restored files
  └── write manifest
```

**Call graph for `update_hook_file` (revised — narrowed):**

```
update_hook_file(path, entry, hooks_array_key, identity_key) -> Result<bool>
  ├── read settings file → serde_json::Value (or {} if missing/empty)
  ├── ensure root.hooks is an object (create if absent)
  ├── ensure root.hooks[hooks_array_key] is an array (create if absent)
  ├── find entry whose entry.hooks[*][identity_key] == identity_value (or top-level fallback)
  ├── replace if found, append if not
  ├── serialize back (pretty, BTreeMap-ordered)
  └── write iff bytes differ
  → Ok(true) if a write happened, Ok(false) if idempotent no-op
```

[**Data Structure**]

`Platform`, `PLATFORMS`, `CLAUDE_PLATFORM`, `CODEX_PLATFORM` — same as 00 except:

```rust
pub const CODEX_PLATFORM: Platform = Platform {
    id: "codex",
    templates: &crate::templates::CODEX_TEMPLATES,
    dest_dir: ".codex",
    cli_flag: "codex",
    managed_block_target: Some("AGENTS.md"),
    hook_file: Some(HookFileSpec {
        path: ".codex/hooks.json",
        hooks_array_key: "SessionStart",   // was: json_pointer
        identity_key: "command",
        identity_value: ARK_CONTEXT_HOOK_COMMAND,
        entry_builder: ark_codex_hook_entry,
    }),
};
```

```rust
// ark-core/src/io/fs.rs additions / changes

#[derive(Debug, Clone, Copy)]
pub struct HookFileSpec {
    pub path: &'static str,
    /// Array key under root `hooks` carrying the Ark entry. Per G-6.
    pub hooks_array_key: &'static str,
    pub identity_key: &'static str,
    pub identity_value: &'static str,
    pub entry_builder: fn() -> serde_json::Value,
}

/// Codex SessionStart hook entry. Note: `timeout` is in seconds (per
/// Codex's hooks.json schema; default if omitted is 600s). Claude uses
/// milliseconds for the same field name; do not confuse.
pub fn ark_codex_hook_entry() -> serde_json::Value {
    serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": ARK_CONTEXT_HOOK_COMMAND,
                "timeout": 30,
            }
        ],
    })
}

pub fn update_hook_file(
    path: impl AsRef<Path>,
    entry: serde_json::Value,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<bool>;

pub fn remove_hook_file(
    path: impl AsRef<Path>,
    identity_value: &str,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<bool>;

pub fn read_hook_file(
    path: impl AsRef<Path>,
    identity_value: &str,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<Option<serde_json::Value>>;

// Deprecated thin wrappers (R-005 / C-23). Removed at 0.3.0. Each delegates
// with the previously hard-coded "SessionStart" / "command" arguments.
#[deprecated(since = "0.2.0", note = "Use update_hook_file")]
pub fn update_settings_hook(path: impl AsRef<Path>, entry: serde_json::Value) -> Result<bool>;

#[deprecated(since = "0.2.0", note = "Use remove_hook_file")]
pub fn remove_settings_hook(path: impl AsRef<Path>, identity_value: &str) -> Result<bool>;

#[deprecated(since = "0.2.0", note = "Use read_hook_file")]
pub fn read_settings_hook(path: impl AsRef<Path>, identity_value: &str) -> Result<Option<serde_json::Value>>;
```

`InitOptions`, `Layout` getters, CLI args — same as 00 with renames per G-13 (`codex_skills_dir` not `codex_prompts_dir`; `CODEX_SKILLS_DIR` not `CODEX_PROMPTS_DIR`).

[**API Surface**]

Library re-exports from `ark-core/src/lib.rs`:

```rust
pub use platforms::{Platform, PLATFORMS, CLAUDE_PLATFORM, CODEX_PLATFORM};
pub use io::{
    HookFileSpec, ark_codex_hook_entry,
    update_hook_file, remove_hook_file, read_hook_file,
    // Deprecated wrappers retained one release:
    update_settings_hook, remove_settings_hook, read_settings_hook,
};
```

CLI surface — unchanged from 00.

[**Constraints**]

- C-1 through C-17 — unchanged from 00 (modulo the `codex_skills_dir` rename in C-17 and the C-7 revision below).
- C-7 (REVISED): Codex skill bodies live as static authored files under `templates/codex/skills/ark-<name>/SKILL.md`, each carrying YAML frontmatter (`name`, `description`) followed by an authored body. Bodies are mechanical translations of the matching Claude command but **diverge by design** — the Claude templates use slash-invocation idioms (`# /ark:quick $ARGUMENTS`, `Parse $ARGUMENTS:`, "the user typed `/ark:archive`") that have no equivalent in Codex's description-routed skill model. Translation rules applied at authoring time: drop Claude's frontmatter, prepend Codex frontmatter, rewrite `# /ark:<name> $ARGUMENTS` → `# ark-<name>`, rewrite inline `/ark:<name>` → `ark-<name>`, rewrite `$ARGUMENTS` → `<task description>`, soften "the user typed" phrasing. The shipped parity tests (G-12) check existence and frontmatter shape only; body content is policed by code review when either side changes.
- C-18 (REVISED): Source-scan tests cover all five command files (`init`, `upgrade`, `unload`, `load`, `remove`) plus `platforms.rs`. Each file's tests module includes a per-file scanner mirroring `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`'s shape: line-by-line scan, exclude `#[cfg(test)]` body, exclude `//` comments, assert no `std::fs::`, no `".ark/"`, no `".claude/"`, no `".codex/"`, no `"AGENTS.md"`/`"CLAUDE.md"` literals (those go through `Layout`). `fs.rs` is exempt (sanctioned site).
- C-19 (REVISED): `update_hook_file`/`remove_hook_file`/`read_hook_file` accept `hooks_array_key: &str` validated against `[A-Za-z0-9_-]+`. Empty or out-of-charset → `Error::Io { path: <synth>, source: io::Error::other("invalid hooks array key") }`. Both shipping platforms pass `"SessionStart"`.
- C-20: Platform iteration order (per `PLATFORMS` slice) is canonical. (Unchanged.)
- C-21: `Platform::by_id` and `Platform::by_cli_flag` give the CLI a typed lookup path. (Unchanged.)
- **C-22 (NEW)**: After `load` replays `snapshot.hook_bodies`, it iterates `PLATFORMS` and re-applies the canonical entry via `(platform.hook_file.entry_builder)()` for every platform whose `dest_dir` appears in the restored `files`. This makes the post-load on-disk hook state independent of snapshot age and resolves the asymmetry flagged in R-002. The replay-then-canonical pattern matches ark-context C-17's "re-apply unconditionally on every init/load/upgrade" semantics.
- **C-23 (NEW)**: The deprecated helpers (`update_settings_hook`, `remove_settings_hook`, `read_settings_hook`) ship as concrete thin-wrapper functions in `io/fs.rs`, each carrying `#[deprecated(since = "0.2.0", note = "Use update_hook_file")]`. They delegate by passing the previously hard-coded `"SessionStart"` / `"command"` arguments. Removed at the 0.3.0 release. NOT `pub use` aliases (which can't carry `#[deprecated]` reliably and lose docstrings).
- **C-24 (NEW)**: `unload`'s hook-capture path has two stages. (a) For each `PLATFORMS` entry with `hook_file.is_some()` and the file present, read+remove via `read_hook_file`/`remove_hook_file`. (b) Then walk every `*.json` file under `owned_dirs()`, parse it, and for any entry containing `command == ARK_CONTEXT_HOOK_COMMAND` that wasn't already captured in (a), capture it into `snapshot.hook_bodies` and remove it surgically. This makes round-trip lossless for hook entries written by past or future Ark versions whose platforms aren't currently registered. Failure to parse a `.json` file is non-fatal (skip + warn-to-stderr).
- **C-25 (NEW)**: Codex hook timeout in `ark_codex_hook_entry()` is `30` (seconds). Claude's `ark_session_start_hook_entry()` is `5000` (milliseconds). The two values use different units per their respective platforms' hook schemas. Both functions carry doc comments naming the unit. Deviating from these defaults is a deliberate edit and updates the matching V-UT.
- C-8 (REVISED): same as 00 + one paragraph: "Forward direction (older binary reading newer snapshot): unknown `hook_bodies` entries deserialize successfully (the field shape is stable). Older Ark's `load` does not re-apply the unrecognized entries (they have no matching `Platform`). The user upgrading the binary picks up the entry on the next `unload`/`load` cycle through C-24."

(Other unchanged constraints from 00 retained verbatim.)
