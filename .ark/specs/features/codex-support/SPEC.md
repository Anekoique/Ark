[**Goals**]

- G-1: `Platform` registry is the single source of truth for per-platform installation; commands iterate `&[&Platform]`.
- G-2: Two platforms ship: `claude-code` and `codex`. Adding a third later is a registry entry, not a refactor.
- G-3: `ark init` accepts `--claude` / `--no-claude` / `--codex` / `--no-codex`; TTY prompt selects per-platform; non-TTY without flags errors.
- G-4: Codex artifacts ship at `.codex/skills/ark-{quick,design,archive}/SKILL.md`, `.codex/hooks.json`, `.codex/config.toml`.
- G-5: Codex installs an `ARK` managed block in `AGENTS.md`, parallel to `CLAUDE.md` for Claude.
- G-6: Hook helpers parameterized over `(hooks_array_key, identity_key)` so future platforms reuse the JSON-shape plumbing.

[**Non-goals**]

- NG-1: No third platform in this task; registry leaves room.
- NG-2: No `.codex/agents/*.toml` custom subagents and no `.codex/prompts/` slash commands.
- NG-3: No JSON Pointer parser; the hook helper takes a single key.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                      (InitArgs gains 4 platform flags +
│                                              interactive selection + non-TTY error)
└── ark-core/src/
    ├── lib.rs                                (re-exports Platform registry types)
    ├── platforms.rs                          (NEW; Platform struct + PLATFORMS slice +
    │                                           CLAUDE_PLATFORM / CODEX_PLATFORM consts)
    ├── layout.rs                             (codex_dir, codex_skills_dir, codex_hooks_file,
    │                                           codex_config_file, agents_md;
    │                                           CODEX_DIR, CODEX_SKILLS_DIR, CODEX_HOOKS_FILE,
    │                                           CODEX_CONFIG_FILE, AGENTS_MD consts;
    │                                           owned_dirs grows to 3 entries)
    ├── io/fs.rs                              (settings-hook helpers renamed update_hook_file
    │                                           + parameterized over (hooks_array_key,
    │                                           identity_key); old names retained as
    │                                           #[deprecated] thin wrappers (0.2.0 → 0.3.0);
    │                                           ark_codex_hook_entry() with timeout 30s)
    ├── state/snapshot.rs                     (unchanged)
    └── commands/
        ├── init.rs                           (accepts InitOptions::platforms)
        ├── upgrade.rs                        (collect_desired_templates iterates manifest-
        │                                       recorded platform set; hook + config refresh
        │                                       runs per-installed-platform)
        ├── unload.rs                         (captures hook entries by file-presence +
        │                                       identity scan — C-9)
        ├── load.rs                           (re-applies hook_bodies, then re-applies
        │                                       canonical entries per PLATFORMS — C-7)
        └── remove.rs                         (removes hook entries from every platform;
                                                owned_dirs destructure replaced)
templates/
└── codex/                                    (NEW)
    ├── skills/ark-{quick,design,archive}/SKILL.md
    ├── hooks.json                            (SessionStart entry, timeout: 30 seconds)
    └── config.toml                           (project_doc_fallback_filenames = ["AGENTS.md"])
```

Call graph for `unload`:

```
unload(opts)
  ├── walk every owned_dir, capture into Snapshot::files
  ├── for block in manifest.managed_blocks:                — captures both CLAUDE.md and AGENTS.md
  │     read_managed_block + remove_managed_block
  ├── for platform in PLATFORMS:                           — capture by registered file
  │     if platform has hook_file ∧ file exists:
  │         read_hook_file → snapshot.add_hook_body
  │         remove_hook_file
  ├── for json_file in walk_files(owned_dirs).filter(.json):  — C-9: scan unregistered files
  │     for entry in scan_session_start_array(json_file):
  │         if entry.hooks[*].command == ARK_CONTEXT_HOOK_COMMAND
  │            ∧ json_file not already captured above:
  │             snapshot.add_hook_body
  │             remove the entry surgically
  ├── snapshot.write
  └── delete owned_dirs
```

Call graph for `load`:

```
load(opts)
  ├── read snapshot
  ├── restore snapshot.files
  ├── re-apply each managed block via update_managed_block
  ├── for body in snapshot.hook_bodies:                    — replay captured entries
  │     update_hook_file(body.path, body.entry,
  │                      derive_array_key(body.json_pointer),
  │                      body.identity_key)
  ├── for platform in PLATFORMS:                           — C-7: canonical re-apply
  │     if platform.hook_file.is_some()
  │        ∧ platform.dest_dir appears in restored files:
  │         spec = platform.hook_file.unwrap()
  │         update_hook_file(layout.resolve(spec.path),
  │                          (spec.entry_builder)(),
  │                          spec.hooks_array_key,
  │                          spec.identity_key)
  ├── regenerate manifest hashes from restored files
  └── write manifest
```

Call graph for `update_hook_file` (renamed and parameterized):

```
update_hook_file(path, entry, hooks_array_key, identity_key) -> Result<bool>
  ├── read settings file → serde_json::Value (or {} if missing/empty)
  ├── ensure root.hooks is an object (create if absent)
  ├── ensure root.hooks[hooks_array_key] is an array (create if absent)
  ├── find entry whose entry.hooks[*][identity_key] == identity_value
  ├── replace if found, append if not
  ├── serialize back (pretty, BTreeMap-ordered)
  └── write iff bytes differ
  → Ok(true) if a write happened, Ok(false) if idempotent no-op
```

[**Data Structure**]

```rust
// ark-core/src/platforms.rs
pub struct Platform {
    pub id: &'static str,
    pub templates: &'static include_dir::Dir<'static>,
    pub dest_dir: &'static str,
    pub cli_flag: &'static str,
    pub managed_block_target: Option<&'static str>,
    pub hook_file: Option<HookFileSpec>,
}

pub const PLATFORMS: &[&Platform] = &[&CLAUDE_PLATFORM, &CODEX_PLATFORM];

pub const CLAUDE_PLATFORM: Platform = Platform {
    id: "claude-code",
    templates: &templates::CLAUDE_TEMPLATES,
    dest_dir: ".claude",
    cli_flag: "claude",
    managed_block_target: Some("CLAUDE.md"),
    hook_file: Some(HookFileSpec {
        path: ".claude/settings.json",
        hooks_array_key: "SessionStart",
        identity_key: "command",
        identity_value: ARK_CONTEXT_HOOK_COMMAND,
        entry_builder: ark_session_start_hook_entry,
    }),
};

pub const CODEX_PLATFORM: Platform = Platform {
    id: "codex",
    templates: &templates::CODEX_TEMPLATES,
    dest_dir: ".codex",
    cli_flag: "codex",
    managed_block_target: Some("AGENTS.md"),
    hook_file: Some(HookFileSpec {
        path: ".codex/hooks.json",
        hooks_array_key: "SessionStart",
        identity_key: "command",
        identity_value: ARK_CONTEXT_HOOK_COMMAND,
        entry_builder: ark_codex_hook_entry,
    }),
};

// ark-core/src/io/fs.rs (additions)
#[derive(Debug, Clone, Copy)]
pub struct HookFileSpec {
    pub path: &'static str,
    pub hooks_array_key: &'static str,    // array key under root `hooks` carrying the Ark entry
    pub identity_key: &'static str,
    pub identity_value: &'static str,
    pub entry_builder: fn() -> serde_json::Value,
}

/// Codex SessionStart hook entry.
/// `timeout` is in **seconds** (Codex's hooks.json schema; default 600s).
/// Claude uses **milliseconds** for the same field name; do not confuse.
pub fn ark_codex_hook_entry() -> serde_json::Value {
    serde_json::json!({
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": ARK_CONTEXT_HOOK_COMMAND,
            "timeout": 30,
        }],
    })
}
```

[**API Surface**]

```rust
// ark-core/src/io/fs.rs
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

// Deprecated thin wrappers (#[deprecated(since = "0.2.0", note = "Use update_hook_file")]).
// Removed at 0.3.0. Each delegates with the previously hard-coded
// "SessionStart" / "command" arguments.
#[deprecated(since = "0.2.0", note = "Use update_hook_file")]
pub fn update_settings_hook(path: impl AsRef<Path>, entry: serde_json::Value) -> Result<bool>;
#[deprecated(since = "0.2.0", note = "Use remove_hook_file")]
pub fn remove_settings_hook(path: impl AsRef<Path>, identity_value: &str) -> Result<bool>;
#[deprecated(since = "0.2.0", note = "Use read_hook_file")]
pub fn read_settings_hook(path: impl AsRef<Path>, identity_value: &str)
    -> Result<Option<serde_json::Value>>;

// Library re-exports from ark-core/src/lib.rs
pub use platforms::{Platform, PLATFORMS, CLAUDE_PLATFORM, CODEX_PLATFORM};
pub use io::{
    HookFileSpec, ark_codex_hook_entry,
    update_hook_file, remove_hook_file, read_hook_file,
    // Deprecated (one release):
    update_settings_hook, remove_settings_hook, read_settings_hook,
};

// CLI (ark-cli/src/main.rs)
struct InitArgs {
    #[arg(long)] claude: bool,
    #[arg(long = "no-claude")] no_claude: bool,
    #[arg(long)] codex: bool,
    #[arg(long = "no-codex")] no_codex: bool,
}
```

`Layout` gains `codex_dir()`, `codex_skills_dir()`, `codex_hooks_file()`, `codex_config_file()`, `agents_md()`. `owned_dirs()` returns `[ark_dir(), claude_commands_ark_dir(), codex_dir()]`.

`InitArgs::flags` maps each `Platform`'s `cli_flag` to its `(positive, negative)` pair via `match p.cli_flag`. Mutually-exclusive `--claude --no-claude` resolution is behavioural via `resolve_platforms_pure` (filter `f.on && !f.off` — negative wins on per-platform conflict). The interactive-prompt `for platform in PLATFORMS` loop renders all installed platforms' selection state.

[**Constraints**]

- C-1: All paths under `.codex/` route through `Layout` getters or `CODEX_*` consts; no `".codex/"` literal outside `layout.rs` and `templates.rs`.
- C-2: Skill bodies are mechanical translations of Claude commands: drop Claude frontmatter, prepend Codex frontmatter (`name`, `description`), rewrite `/ark:<name>` → `ark-<name>`, rewrite `$ARGUMENTS` → `<task description>`.
- C-3: Source-scan tests cover `init`, `upgrade`, `unload`, `load`, `remove`, `platforms.rs`; each forbids `std::fs::`, `".ark/"`, `".claude/"`, `".codex/"`, `"AGENTS.md"`, `"CLAUDE.md"` literals (sanctioned site: `fs.rs`).
- C-4: `update_hook_file` / `remove_hook_file` / `read_hook_file` accept `hooks_array_key: &str` validated against `[A-Za-z0-9_-]+`; both shipping platforms pass `"SessionStart"`.
- C-5: `PLATFORMS` slice iteration order is canonical: `[CLAUDE_PLATFORM, CODEX_PLATFORM]`.
- C-6: `Platform::by_id` and `Platform::by_cli_flag` give typed lookup paths.
- C-7: `load` replays `snapshot.hook_bodies`, then iterates `PLATFORMS` and re-applies the canonical entry for every installed platform.
- C-8: Deprecated helpers (`update_settings_hook`, etc.) are concrete `#[deprecated(since = "0.2.0", ...)]` thin wrappers, not `pub use` aliases.
- C-9: `unload` captures hooks in two stages: (a) registered platforms via `read_hook_file` + `remove_hook_file`; (b) walk every `*.json` under `owned_dirs` and surgically capture/remove unrecognized Ark entries.
- C-10: Codex hook timeout is `30` (seconds); Claude is `5000` (milliseconds). Both functions document the unit.
- C-11: `.codex/hooks.json`, `.codex/config.toml`, and `AGENTS.md` managed block are not hash-tracked; re-applied unconditionally on every `init` / `load` / `upgrade`. Sibling user content preserved.
- C-12: A platform is "installed" iff some path under `Platform::dest_dir` appears in `manifest.files`.
- C-13: Parity tests assert: every Claude command has a Codex skill sibling; every shipped skill begins with `---\nname: ark-` (Codex frontmatter, not Claude's).
- C-14: An existing Claude-only project remains Claude-only on `ark upgrade`; adding Codex requires re-running `ark init --codex` (additive, idempotent).
- C-15: Snapshot forward compat: older `.ark.db` files without a Codex hook entry deserialize to `vec![]`; canonical re-apply restores both platforms regardless.

[**CHANGELOG**]

- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
- 2026-05-10 `subagent-support`: NG-2 ("No `.codex/agents/*.toml` custom subagents") is superseded; Codex now ships `ark-researcher`, `ark-reviewer`, `ark-verifier` under `.codex/agents/` via `Platform.agents_templates` and `agents_dest_dir` (new optional fields). `Platform` is now `#[non_exhaustive]`. The Codex agent file format follows `reference/Trellis/.codex/agents/trellis-research.toml`: keys `name`, `description`, `sandbox_mode`, `developer_instructions` (multi-line body), and a `[features]` block with `multi_agent = false` and `[features.multi_agent_v2].enabled = false` to disable nested-agent spawning. See `specs/features/subagent-support/SPEC.md`.
