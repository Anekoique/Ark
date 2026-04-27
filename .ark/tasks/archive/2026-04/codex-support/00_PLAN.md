# `codex-support` PLAN `00`

> Status: Draft
> Feature: `codex-support`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Add OpenAI Codex CLI as a first-class Ark platform alongside Claude Code. The current code path hard-codes `[ARK_TEMPLATES, CLAUDE_TEMPLATES]` in `init`/`upgrade`/`unload`, and `ark-context`'s settings-hook helper is shaped to Claude's `.claude/settings.json` JSON path. This PLAN introduces a static `Platform` registry (Trellis-pattern, 2 entries today: Claude + Codex) and parameterizes the settings-hook helper over JSON pointer + identity key, so both platforms run through the same surgical-hook machinery.

Three new Codex artifacts ship: `.codex/prompts/ark-{quick,design,archive}.md` (mechanical translations of the Claude command bodies), `.codex/hooks.json` (SessionStart entry calling `ark context --scope session --format json`), and `.codex/config.toml` (`project_doc_fallback_filenames = ["AGENTS.md"]`). `init` becomes interactive — first run with no platform flags shows a TTY prompt; non-TTY without flags installs both. `AGENTS.md` gets a managed block parallel to `CLAUDE.md`'s.

A parity test asserts every `templates/claude/commands/ark/<name>.md` has a sibling `templates/codex/prompts/ark-<name>.md`, making it a build-time failure to add a Claude slash command without a Codex twin.

## Log `None in 00_PLAN`

---

## Spec `Core specification`

[**Goals**]

- G-1: `Platform` registry is the single source of truth for per-platform installation. `init` / `upgrade` / `unload` / `load` / `remove` iterate the same `&[&Platform]` slice; adding a third platform later is a registry entry, not a refactor of the command bodies.
- G-2: Two platforms ship in this task: `claude-code` and `codex`. Each carries a static template tree (`include_dir!`), a config-dir name, a managed-block target file, an optional `HookFileSpec` descriptor, and a `cli_flag` string.
- G-3: `ark init` accepts `--claude` / `--codex` / `--no-claude` / `--no-codex` flags. With no platform flags, on a TTY, an interactive prompt asks which platforms to install; both checked by default. With no flags on a non-TTY, both platforms install (matches `ConflictPolicy::Interactive` non-TTY safe-default precedent in upgrade C-7). Explicit `--no-X` flags suppress prompting and install only what's selected.
- G-4: `init` ships Codex artifacts at canonical paths: `.codex/prompts/ark-quick.md`, `.codex/prompts/ark-design.md`, `.codex/prompts/ark-archive.md`, `.codex/hooks.json`, `.codex/config.toml`. The three prompt bodies are mechanical translations of `templates/claude/commands/ark/{quick,design,archive}.md`: YAML frontmatter dropped (Codex prompts don't support it), inline `/ark:foo` references rewritten to `/ark-foo`. Authored by hand at template-shipping time; not generated at install time.
- G-5: `init` installs an `AGENTS.md` managed block (marker `ARK`) when Codex is selected, parallel to `CLAUDE.md`'s. File created if absent. Body identical to `CLAUDE.md`'s (reuses `MANAGED_BLOCK_BODY`).
- G-6: `update_settings_hook` / `remove_settings_hook` / `read_settings_hook` are renamed `update_hook_file` / `remove_hook_file` / `read_hook_file` and parameterized over `(json_pointer: &str, identity_key: &str)`. The Claude entry continues to navigate `/hooks/SessionStart` with `command` identity; the Codex entry navigates `/hooks/SessionStart` with `command` identity (Codex's `hooks.json` happens to use the same shape). Per-platform constants `ark_session_start_hook_entry()` (Claude) and `ark_codex_hook_entry()` (Codex) build the canonical entries.
- G-7: `.codex/hooks.json` and `.codex/config.toml` are NOT hash-tracked. Re-applied unconditionally on every `init` / `load` / `upgrade`. Mirrors the Claude `settings.json` precedent (ark-context C-17, ark-upgrade C-8).
- G-8: `.codex/hooks.json` supports sibling user hooks. Ark's entry is identified by `command == "ark context --scope session --format json"`; surgical upserts and removals via `update_hook_file` / `remove_hook_file` preserve any other entries the user adds. Same contract as `.claude/settings.json`.
- G-9: `Snapshot::hook_bodies` captures the Codex hook entry on `unload` alongside the Claude one. `load` re-applies both. Round-trip: install both platforms → unload → load → byte-identical (modulo timestamps). Older `.ark.db` files without a Codex entry deserialize to `vec![]` for that slot via the existing `#[serde(default)]` (ark-context C-27).
- G-10: `remove` removes both platforms' SessionStart hook entries (surgically, leaving sibling user entries) and both managed blocks (`CLAUDE.md` and `AGENTS.md`). Manifest-driven — `remove` reads `manifest.managed_blocks` to know which blocks to remove; new code must record the `AGENTS.md` block in `init` so this works.
- G-11: `upgrade` re-applies `CLAUDE.md` block, `AGENTS.md` block, Claude SessionStart hook, Codex SessionStart hook, and `.codex/config.toml` on every run. None of the five are hash-tracked. Sibling user content (other CLAUDE.md sections, other hooks.json entries) is preserved.
- G-12: A test (`templates_codex_prompts_match_claude_commands`) asserts that for every `templates/claude/commands/ark/*.md`, there exists a matching `templates/codex/prompts/ark-*.md`. Adding a new slash command without a Codex twin is a compile-time / test-time failure, enforcing the parity invariant.
- G-13: `Layout` gains `codex_dir()`, `codex_prompts_dir()`, `codex_hooks_file()`, `codex_config_file()`, `agents_md()`. `owned_dirs()` returns `.ark`, `.claude/commands/ark`, `.codex` (the last only relevant to Codex-installed projects; on a Claude-only install, the dir simply doesn't exist and `walk_files` yields empty).

[**Non-goals**]

- NG-1: No Codex `.codex/agents/*.toml` custom subagents. Trellis ships them; Ark has no Claude-side equivalent. Defer.
- NG-2: No Codex `.codex/skills/*/SKILL.md` auto-routed skills. Different invocation model from prompts; not needed for parity.
- NG-3: No third platform (Cursor, OpenCode, Gemini). Registry must leave room, but no other platform ships in this task.
- NG-4: No per-prompt user customization at install time (which prompts to skip).
- NG-5: No shared-body templating with includes. Trellis confirmed parallel template trees with manual sync is the right call; G-12 enforces sync existence, parity of body content is checked in code review.
- NG-6: No detection of which CLI tools are installed on the user's machine to drive default selection. The interactive prompt presents both options; the user picks.
- NG-7: No `AGENTS.md` rewrite to make it the canonical project doc. Stays as-is; `CLAUDE.md` keeps its own managed block. The two managed blocks are independent.
- NG-8: No change to `ark context`'s output. The same `--scope session --format json` invocation feeds both platforms; the SessionStart envelope shape is identical for Claude and Codex (per Codex hook docs and Trellis confirmation).
- NG-9: No changes to slash-command (`/ark:quick`, `/ark:design`, `/ark:archive`) Claude bodies in this task. Their Codex twins are derived; the originals stand.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                       — InitArgs gains 4 platform flags + StdioPrompter
│                                                interactive selection
└── ark-core/src/
    ├── lib.rs                                 — re-exports Platform registry types
    ├── platforms.rs                           — NEW. Platform struct + PLATFORMS slice +
    │                                             CLAUDE_PLATFORM / CODEX_PLATFORM consts
    ├── layout.rs                              — adds codex_dir, codex_prompts_dir,
    │                                             codex_hooks_file, codex_config_file,
    │                                             agents_md, AGENTS_MD const,
    │                                             CODEX_DIR / CODEX_HOOKS_FILE /
    │                                             CODEX_CONFIG_FILE consts; owned_dirs
    │                                             extended to 3 entries
    ├── io/
    │   └── fs.rs                              — settings-hook helpers renamed +
    │                                             parameterized over json_pointer +
    │                                             identity_key. New: ark_codex_hook_entry().
    │                                             Existing ark_session_start_hook_entry()
    │                                             retained (Claude). Old function names
    │                                             kept as `pub use` aliases for one release.
    ├── state/
    │   └── snapshot.rs                        — unchanged shape; hook_bodies already
    │                                             carries arbitrary path/identity tuples
    ├── commands/
    │   ├── init.rs                            — accepts InitOptions::platforms; iterates
    │   │                                        selected platforms instead of two-arm loop
    │   ├── upgrade.rs                          — collect_desired_templates iterates
    │   │                                        manifest-recorded platform set; hook +
    │   │                                        config refresh runs per platform
    │   ├── unload.rs                           — captures hook entries from every platform
    │   │                                        present in the live install
    │   ├── load.rs                             — re-applies every captured hook_body via
    │   │                                        update_hook_file
    │   └── remove.rs                           — removes hook entries from every platform
    │                                             that has a hook_file
    └── templates.rs                            — adds CODEX_TEMPLATES static
templates/
├── ark/                                        — unchanged
├── claude/                                     — unchanged
└── codex/                                      — NEW
    ├── prompts/
    │   ├── ark-quick.md                       — mechanical translation of claude/commands/ark/quick.md
    │   ├── ark-design.md                       — ditto for design.md
    │   └── ark-archive.md                      — ditto for archive.md
    ├── hooks.json                              — SessionStart entry calling ark context
    └── config.toml                             — project_doc_fallback_filenames = ["AGENTS.md"]
```

**Module coupling.** `init`/`upgrade`/`unload`/`load`/`remove` depend on `platforms.rs` only. `platforms.rs` depends on `layout.rs` and `templates.rs`. The settings-hook helpers in `io/fs.rs` are leaf utilities used by both `platforms.rs`-installed plumbing and direct call sites in commands. No platform-specific code lives outside `platforms.rs` and the platform-keyed template trees.

**Call graph for `init` (post-refactor):**

```
init(opts)
  ├── Manifest::new()
  ├── extract(ARK_TEMPLATES, layout.ark_dir(), …)        — unchanged; platform-neutral
  ├── for platform in opts.platforms:
  │     ├── extract(platform.templates, dest_root, …)    — writes commands/prompts/hooks/config
  │     ├── if platform.managed_block_target:
  │     │     update_managed_block(target, "ARK", BODY)
  │     │     manifest.record_block(target, "ARK")
  │     ├── if platform.hook_file:
  │     │     update_hook_file(platform.hook_file.path,
  │     │                      (platform.hook_file.entry_builder)(),
  │     │                      platform.hook_file.json_pointer,
  │     │                      platform.hook_file.identity_key)
  │     │     (NOT recorded in manifest — re-applied unconditionally)
  ├── EMPTY_DIRS.ensure_dir each
  ├── manifest.write()
```

**Call graph for `unload` (post-refactor):**

```
unload(opts)
  ├── walk every owned_dir, capture into Snapshot::files
  ├── for block in manifest.managed_blocks:                — captures both CLAUDE.md and AGENTS.md
  │     read_managed_block + remove_managed_block
  ├── for platform in PLATFORMS:                           — capture hook entries
  │     if platform has hook_file AND file exists:
  │         read_hook_file → snapshot.add_hook_body
  │         remove_hook_file
  ├── snapshot.write
  └── delete owned_dirs
```

**Call graph for `update_hook_file` (parameterized):**

```
update_hook_file(path, entry, json_pointer, identity_key) -> Result<bool>
  ├── read_hook_or_empty(path) → serde_json::Value (or {} if missing/empty)
  ├── navigate path components from json_pointer (creating intermediates if absent)
  ├── ensure terminal node is an array
  ├── find entry whose entry.hooks[*][identity_key] == identity_value (or top-level fallback)
  ├── replace if found, append if not
  ├── serialize back (pretty, BTreeMap-ordered)
  └── write iff bytes differ
  → Ok(true) if a write happened, Ok(false) if idempotent no-op
```

[**Data Structure**]

```rust
// ark-core/src/platforms.rs (NEW)

use include_dir::Dir;
use crate::io::HookFileSpec;

/// A Platform names a coding-agent integration target. Each entry is the
/// single source of truth for that integration's installation surface:
/// where its templates live, where its config dir is on disk, what hook
/// file (if any) carries its SessionStart entry, and what managed block
/// (if any) it installs in a project doc.
#[derive(Debug, Clone, Copy)]
pub struct Platform {
    /// Stable string id, used in CLI flags and snapshot tags. ASCII, hyphen-
    /// separated, lowercase. e.g. `"claude-code"`, `"codex"`.
    pub id: &'static str,

    /// Embedded template tree, extracted under `dest_dir` of the project root.
    /// Tree-relative paths join under `layout.resolve(dest_dir)`.
    pub templates: &'static Dir<'static>,

    /// Project-relative directory where `templates` extracts. e.g. `.claude`,
    /// `.codex`. Resolves via `Layout::resolve(dest_dir)`.
    pub dest_dir: &'static str,

    /// CLI flag stem: `--<flag>` enables, `--no-<flag>` disables. e.g.
    /// `"claude"` → `--claude` / `--no-claude`.
    pub cli_flag: &'static str,

    /// Optional managed-block target. If `Some`, `init` calls
    /// `update_managed_block(layout.resolve(file), "ARK", MANAGED_BLOCK_BODY)`
    /// and records the block in the manifest. `unload` reads it via the
    /// manifest; `remove` removes it.
    pub managed_block_target: Option<&'static str>,

    /// Optional SessionStart hook descriptor. If `Some`, `init` / `load` /
    /// `upgrade` call `update_hook_file` with these parameters. If `None`,
    /// the platform has no hook surface (e.g. a future Cursor entry that
    /// uses a different mechanism).
    pub hook_file: Option<HookFileSpec>,
}

/// Static slice of all known platforms. Order is insertion order (used as
/// canonical iteration order in init / upgrade / unload).
pub const PLATFORMS: &[&Platform] = &[&CLAUDE_PLATFORM, &CODEX_PLATFORM];

pub const CLAUDE_PLATFORM: Platform = Platform {
    id: "claude-code",
    templates: &crate::templates::CLAUDE_TEMPLATES,
    dest_dir: ".claude",
    cli_flag: "claude",
    managed_block_target: Some("CLAUDE.md"),
    hook_file: Some(HookFileSpec {
        path: ".claude/settings.json",
        json_pointer: "/hooks/SessionStart",
        identity_key: "command",
        identity_value: ARK_CONTEXT_HOOK_COMMAND,
        entry_builder: ark_session_start_hook_entry,
    }),
};

pub const CODEX_PLATFORM: Platform = Platform {
    id: "codex",
    templates: &crate::templates::CODEX_TEMPLATES,
    dest_dir: ".codex",
    cli_flag: "codex",
    managed_block_target: Some("AGENTS.md"),
    hook_file: Some(HookFileSpec {
        path: ".codex/hooks.json",
        json_pointer: "/hooks/SessionStart",
        identity_key: "command",
        identity_value: ARK_CONTEXT_HOOK_COMMAND, // same — same binary call
        entry_builder: ark_codex_hook_entry,
    }),
};

impl Platform {
    pub fn by_id(id: &str) -> Option<&'static Platform> {
        PLATFORMS.iter().copied().find(|p| p.id == id)
    }
    pub fn by_cli_flag(flag: &str) -> Option<&'static Platform> {
        PLATFORMS.iter().copied().find(|p| p.cli_flag == flag)
    }
}
```

```rust
// ark-core/src/io/fs.rs additions

/// Specification for a JSON-array hook region in a config file (e.g.
/// `.claude/settings.json`'s `hooks.SessionStart`, `.codex/hooks.json`'s ditto).
#[derive(Debug, Clone, Copy)]
pub struct HookFileSpec {
    /// Project-relative path to the JSON file.
    pub path: &'static str,
    /// JSON Pointer to the array of entries (per RFC 6901).
    pub json_pointer: &'static str,
    /// Field name used to identify Ark's entry within the array.
    pub identity_key: &'static str,
    /// Value of `identity_key` Ark uses to find its own entry.
    pub identity_value: &'static str,
    /// Builds the canonical Ark entry. Called by `init` / `load` / `upgrade`.
    pub entry_builder: fn() -> serde_json::Value,
}

pub fn ark_codex_hook_entry() -> serde_json::Value {
    // Codex hook contract is the same envelope shape as Claude:
    // {matcher, hooks: [{type:"command", command, timeout}]}.
    // 5000ms timeout matches the Claude side (per ark-context C-15).
    serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": ARK_CONTEXT_HOOK_COMMAND,
                "timeout": 5000,
            }
        ],
    })
}

/// Parameterized over `(json_pointer, identity_key)`. The Claude-shaped
/// helpers (`update_settings_hook` etc.) become thin aliases retained one
/// release for downstream library consumers, then removed.
pub fn update_hook_file(
    path: impl AsRef<Path>,
    entry: serde_json::Value,
    json_pointer: &str,
    identity_key: &str,
) -> Result<bool>;

pub fn remove_hook_file(
    path: impl AsRef<Path>,
    identity_value: &str,
    json_pointer: &str,
    identity_key: &str,
) -> Result<bool>;

pub fn read_hook_file(
    path: impl AsRef<Path>,
    identity_value: &str,
    json_pointer: &str,
    identity_key: &str,
) -> Result<Option<serde_json::Value>>;

// Aliases (deprecated, removed next minor release):
// pub use update_hook_file as update_settings_hook;
// pub use remove_hook_file as remove_settings_hook;
// pub use read_hook_file as read_settings_hook;
```

```rust
// ark-core/src/commands/init.rs changes

#[derive(Debug, Clone)]
pub struct InitOptions {
    pub project_root: PathBuf,
    pub mode: WriteMode,
    /// Platforms to install. Empty = error. Default = all `PLATFORMS`.
    pub platforms: Vec<&'static Platform>,
}

impl InitOptions {
    pub fn new(project_root: impl Into<PathBuf>) -> Self;     // platforms = PLATFORMS.to_vec()
    pub fn with_mode(mut self, mode: WriteMode) -> Self;
    pub fn with_platforms(mut self, platforms: Vec<&'static Platform>) -> Self;
}
```

```rust
// ark-core/src/layout.rs additions

pub const CODEX_DIR: &str = ".codex";
pub const CODEX_PROMPTS_DIR: &str = ".codex/prompts";
pub const CODEX_HOOKS_FILE: &str = ".codex/hooks.json";
pub const CODEX_CONFIG_FILE: &str = ".codex/config.toml";
pub const AGENTS_MD: &str = "AGENTS.md";

impl Layout {
    pub fn codex_dir(&self) -> PathBuf;            // .codex/
    pub fn codex_prompts_dir(&self) -> PathBuf;    // .codex/prompts/
    pub fn codex_hooks_file(&self) -> PathBuf;     // .codex/hooks.json
    pub fn codex_config_file(&self) -> PathBuf;    // .codex/config.toml
    pub fn agents_md(&self) -> PathBuf;            // AGENTS.md
}

// owned_dirs returns 3 entries:
// [.ark, .claude/commands/ark, .codex]
// On a Claude-only install, .codex doesn't exist; walk_files returns [].
```

```rust
// ark-cli/src/main.rs additions

#[derive(Args)]
struct InitArgs {
    #[command(flatten)]
    target: TargetArgs,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    claude: bool,
    #[arg(long)]
    codex: bool,
    #[arg(long)]
    no_claude: bool,
    #[arg(long)]
    no_codex: bool,
}

// Resolution rules (in `resolve_platforms`):
//   1. Start with all PLATFORMS.
//   2. Apply --no-X removals.
//   3. If any --X positive flag was given, narrow to just those (overriding 1).
//   4. If no positive AND no negative flag was given AND stdin is a TTY:
//        run interactive prompt, defaulting both to checked.
//      If no flags AND stdin is non-TTY: install all PLATFORMS (safe default).
//   5. If the resolved set is empty: error "init requires at least one platform".

fn interactive_select_platforms() -> Result<Vec<&'static Platform>>;
```

[**API Surface**]

Library re-exports from `ark-core/src/lib.rs`:

```rust
pub use platforms::{Platform, PLATFORMS, CLAUDE_PLATFORM, CODEX_PLATFORM};
pub use io::{
    HookFileSpec, ark_codex_hook_entry,
    update_hook_file, remove_hook_file, read_hook_file,
    // aliases retained one release:
    update_settings_hook, remove_settings_hook, read_settings_hook,
};
```

CLI surface (visible in `ark --help`):

```
ark init [--force] [--claude] [--codex] [--no-claude] [--no-codex] [--dir <path>]
ark upgrade [--force | --skip-modified | --create-new] [--allow-downgrade]   (unchanged)
ark unload                                                                    (unchanged)
ark load [--force]                                                             (unchanged)
ark remove                                                                     (unchanged)
ark context --scope {session|phase} ...                                        (unchanged)
```

`ark init --help` mentions interactive selection and the four platform flags.

[**Constraints**]

- C-1: `Platform` registry (`PLATFORMS` slice) is the only place new platforms are defined. Adding a third platform requires (a) a new template tree under `templates/<name>/`, (b) a `pub const <NAME>_PLATFORM: Platform = ...` in `platforms.rs`, (c) the const added to `PLATFORMS`. No edits required to `init`/`upgrade`/`unload`/`load`/`remove`.
- C-2: `init` accepts an explicit `Vec<&'static Platform>` via `InitOptions::with_platforms`. CLI translates flags via `resolve_platforms(args, stdin_is_tty)`. Library never touches stdin or stdout.
- C-3: Interactive platform selection is a CLI-layer concern. The library's `init` is non-interactive. Tests cover library directly with explicit `Vec<&'static Platform>`; CLI integration tests cover flag resolution.
- C-4: Settings-hook helpers (`update_hook_file` / `remove_hook_file` / `read_hook_file`) accept the JSON pointer and identity key as parameters. Internal call sites in `init`/`load`/`upgrade`/`unload`/`remove` use the parameterized form via `Platform::hook_file`.
- C-5: `ark_codex_hook_entry()` and `ark_session_start_hook_entry()` are independent functions with independent test coverage. They are NOT generated from a shared template — different platforms can diverge their entries (timeout, type, custom matcher) without one breaking the other.
- C-6: Codex prompt bodies live as static authored files under `templates/codex/prompts/`. They are NOT generated at install time from the Claude bodies. The parity test (G-12) only asserts file *existence* matches; body parity is enforced by code review at template-edit time (per the Trellis precedent).
- C-7: `templates/codex/prompts/ark-{quick,design,archive}.md` carry no YAML frontmatter (Codex prompts don't support it). Body content is byte-for-byte identical to the matching Claude command except: (a) frontmatter stripped, (b) any `/ark:foo` reference within the body rewritten to `/ark-foo`. Verified once at authoring time; no automated content-diff check (would over-constrain future divergence).
- C-8: `Snapshot` schema is **unchanged**. `hook_bodies` already accommodates arbitrary `(path, json_pointer, identity_key, identity_value, entry)` tuples — Codex entries fit without schema migration. `#[serde(default)]` already covers older snapshots written before this task.
- C-9: `unload` iterates `PLATFORMS`. For each platform with a `hook_file`, if the file exists on disk and contains an Ark entry, it's captured + surgically removed. Sibling user entries survive. Mirrors `unload`'s current Claude-only path (ark-context C-18).
- C-10: `load` iterates `snapshot.hook_bodies` and, for each, calls `update_hook_file(body.path, body.entry, body.json_pointer, body.identity_key)`. Entries from a snapshot whose path doesn't correspond to any current `Platform` are still restored — `load` is path-driven, not platform-driven, for forward compatibility.
- C-11: `upgrade` re-applies all platforms' hook entries unconditionally per `Platform::hook_file`. None of the per-platform hook files (`.claude/settings.json`, `.codex/hooks.json`) are hash-tracked. `.codex/config.toml` is similarly not hash-tracked — re-applied unconditionally as a whole-file write (it's tiny and Ark-owned). Mirrors ark-upgrade C-8.
- C-12: `remove` iterates `PLATFORMS`. For each platform with a `hook_file`, if the file exists, the Ark entry is removed surgically. For each platform with a `managed_block_target`, the managed block is removed. Manifest is consulted for the canonical list of installed blocks; falls back to per-platform defaults if the manifest is absent (mirrors current unload behavior).
- C-13: `init` interactive prompt implementation: reuse `dialoguer` if it's available as a dep; else hand-roll a stdin checkbox prompt. CLI-only; not exposed via the library API.
- C-14: A test (`templates_codex_prompts_match_claude_commands`) walks `templates/claude/commands/ark/` at compile time via `include_dir!` and asserts that for every `<name>.md`, `templates/codex/prompts/ark-<name>.md` exists. Mismatch → test fails. The test does NOT check body parity.
- C-15: `Layout::owned_dirs()` returns a fixed `[PathBuf; 3]` covering `.ark`, `.claude/commands/ark`, `.codex`. `walk_files` on a non-existent dir returns `Ok(vec![])`, so a Claude-only install has no `.codex/` entries to walk and is silently a no-op. No `Option`-typed entries.
- C-16: `init` recording of hash-tracked files is unchanged in shape — it iterates the per-platform `templates` and records each. No platform-specific bookkeeping inside `Manifest`.
- C-17: `Platform::dest_dir` is a project-relative string. All path composition flows through `Layout::resolve(platform.dest_dir)` so per-platform absolute paths derive from the same root as the rest of the layout. No platform code touches `project_root` directly.
- C-18: All filesystem access in `platforms.rs` and the platform-iteration sites in `init`/`upgrade`/`unload`/`load`/`remove` routes through `io::PathExt` / `io::fs` helpers. No bare `std::fs::*`. New source-scan test `platforms_source_no_bare_std_fs` mirrors `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`.
- C-19: `update_hook_file` accepts only an absolute JSON pointer (starting with `/`). Pointers without a leading slash error with `Error::Io { path: <synth>, source: io::Error::other("invalid JSON pointer") }`. The pointer parser handles `/foo/bar/0` style paths, including escaping (`~0` for `~`, `~1` for `/`) per RFC 6901.
- C-20: Platform iteration order (per `PLATFORMS` slice) is canonical. `init` extracts in that order; `unload` captures hook entries in that order. Tests that compare snapshots across runs rely on stable iteration.
- C-21: `Platform::by_id` and `Platform::by_cli_flag` give the CLI a typed lookup path so flag resolution doesn't string-match. The CLI's `resolve_platforms` produces `Vec<&'static Platform>` from `InitArgs`.

## Runtime `runtime logic`

[**Main Flow — `init`**]

1. CLI parses `InitArgs`. `resolve_platforms(args, stdin_is_tty)` produces `Vec<&'static Platform>`.
2. CLI calls `init(InitOptions::new(target).with_mode(mode).with_platforms(platforms))`.
3. Library extracts `ARK_TEMPLATES` under `layout.ark_dir()` (platform-neutral; always installed).
4. For each platform in `opts.platforms`:
   - Extract `platform.templates` under `layout.resolve(platform.dest_dir)`.
   - If `platform.managed_block_target.is_some()`: install managed block; record in manifest.
   - If `platform.hook_file.is_some()`: call `update_hook_file` with descriptor fields; do NOT record in manifest.
5. `EMPTY_DIRS.ensure_dir`.
6. `manifest.write()`.

[**Main Flow — `upgrade`**]

1. Read `Manifest`; validate paths.
2. Compute `desired = collect_desired_templates(layout)`. Covers `ARK_TEMPLATES` + every platform whose templates appear in `manifest.files`. Platforms a project never opted into don't enter the desired set.
3. Reconcile managed blocks (existing logic, unchanged).
4. `plan_actions` produces `Vec<PlannedAction>` (existing logic, unchanged).
5. Apply writes; refresh hashes; create `.new` files; record preserves.
6. Re-apply *every* platform's managed block + hook file that the project has installed (manifest-driven). For each `Platform` whose `dest_dir` appears in `manifest.files`:
   - If `managed_block_target.is_some()`: `update_managed_block(target, "ARK", BODY)`.
   - If `hook_file.is_some()`: `update_hook_file(...)`.
7. Manifest version bump + write.
8. Apply deletions; final manifest write if mutated.

[**Main Flow — `unload`**]

1. Walk every `owned_dir`; capture into `Snapshot::files`.
2. For each managed block in `manifest.managed_blocks` (covers both `CLAUDE.md` and `AGENTS.md`): read + remove.
3. For each `Platform` in `PLATFORMS` with `hook_file.is_some()`: if the hook file exists and contains an Ark entry, capture into `snapshot.hook_bodies` + remove surgically.
4. Persist snapshot.
5. Delete `owned_dirs`. Prune empty parents.

[**Main Flow — `load`**]

1. Read snapshot.
2. Restore `snapshot.files` (writing each at its captured project-relative path).
3. Re-apply each managed block via `update_managed_block`.
4. For each `snapshot.hook_bodies` entry: `update_hook_file(body.path, body.entry, body.json_pointer, body.identity_key)`.
5. Manifest is regenerated by re-running canonical `init`-style hashing on the restored files (existing logic).

[**Failure Flow**]

1. `init --no-claude --no-codex` → error "init requires at least one platform" (resolved set empty).
2. `init` interactive prompt with both platforms unchecked → error "init requires at least one platform" (same path).
3. `init` non-TTY without flags → both platforms install (safe default; no error).
4. Codex hook write fails (permission denied on `.codex/hooks.json`) → `Error::Io`; manifest left in inconsistent state (Codex prompts written, hook not registered). Same failure-mode profile as Claude today; no transactional rollback.
5. `unload` on a project with `.codex/` but no Codex entry in the registered platforms (e.g. snapshot from a future Ark version) → `walk_files` captures dir contents into `snapshot.files`. Hook capture iterates `PLATFORMS`, so unregistered platforms' hook files are NOT captured into `hook_bodies`. **Risk: round-trip lossy for unknown platforms.** Mitigation: documented as a known limitation; future platforms ship with their own Ark version.
6. `upgrade` on a project that has Codex installed but the user removed `.codex/hooks.json` manually → upgrade re-creates the file with only the Ark entry (idempotent path-create-or-update via `update_hook_file`).
7. Old snapshot loaded into a new Ark would lack hook entries on disk if `load` re-applies only `hook_bodies`. **Therefore: legacy `load` calls `update_hook_file` for every Platform whose `dest_dir` appears in `snapshot.files`** (manifest-derived recovery for legacy snapshots).

[**State Transitions**]

- Project state ∈ {NotLoaded, ClaudeOnly, CodexOnly, Both}. Determined by presence of `manifest.files` entries under `.claude/` and `.codex/`.
- `init` transitions NotLoaded → {ClaudeOnly | CodexOnly | Both} per `opts.platforms`.
- `init` re-run on existing project preserves the existing platform set unless `--force` (existing semantics) or new platform flags expand it. **Out of scope:** adding a platform to an already-initialized project. The user runs `init --force` or `init --codex` (additive) to install a new platform; this PLAN does NOT cover the "expand selectively" path explicitly — it's a follow-up.

## Implementation `split task into phases`

[**Phase 1 — Layout and templates**]

1. Add `templates/codex/{prompts/,hooks.json,config.toml}` with mechanical translations of the three Claude commands.
2. Add `CODEX_TEMPLATES` static in `templates.rs`.
3. Extend `Layout` with `codex_*` and `agents_md` getters; add `CODEX_DIR` / `CODEX_HOOKS_FILE` / `CODEX_CONFIG_FILE` / `AGENTS_MD` consts.
4. Extend `owned_dirs` to include `.codex`. Update existing `owned_dirs`-using tests.

[**Phase 2 — Platform registry and hook-helper parameterization**]

1. Create `platforms.rs` with `Platform` + `PLATFORMS` + `CLAUDE_PLATFORM` + `CODEX_PLATFORM`.
2. Add `HookFileSpec` to `io/fs.rs`.
3. Rename `update_settings_hook` → `update_hook_file` and parameterize over `(json_pointer, identity_key)`. Same for `remove_*`, `read_*`. Old names retained as `pub use` aliases for one release.
4. Add `ark_codex_hook_entry()`.
5. Update existing tests in `io/fs.rs` to use the new names. The Claude-aliased path stays test-covered via one alias-test per renamed function.

[**Phase 3 — Refactor commands to drive from `PLATFORMS`**]

1. `init.rs`: add `InitOptions::platforms`; default to `PLATFORMS.iter().copied().collect()`. Replace the two-arm tuple loop with `for platform in opts.platforms`. Per-platform managed-block and hook-file installation.
2. `upgrade.rs`: extend `collect_desired_templates` to walk every `Platform::templates`. Re-apply hook entries per-platform after the manifest write.
3. `unload.rs`: replace the hard-coded Claude hook capture with a `for platform in PLATFORMS` loop.
4. `load.rs`: re-apply hook entries from `snapshot.hook_bodies` (already path-driven; minimal change).
5. `remove.rs`: replace hard-coded Claude hook removal with a `for platform in PLATFORMS` loop. Manifest-driven managed block removal already iterates `manifest.managed_blocks`.
6. CLI: add `InitArgs.{claude, codex, no_claude, no_codex}`. Implement `resolve_platforms(args, stdin_is_tty) -> Vec<&'static Platform>`. Add interactive prompt for the no-flags + TTY case.

[**Phase 4 — Tests + workflow doc updates**]

1. Update every existing init/upgrade/unload/load/remove test that asserts file presence to either (a) install both platforms and assert sibling Codex paths, or (b) explicitly opt into Claude-only via `with_platforms(vec![&CLAUDE_PLATFORM])` and assert the same things as before.
2. Add `templates_codex_prompts_match_claude_commands` parity test under `templates.rs` tests.
3. Add round-trip test: install both → unload → load → byte-identical filesystem and `manifest.files` contents.
4. Add CLI flag-resolution tests: `--codex` only, `--no-claude` only, `--no-codex --no-claude` errors, etc.
5. Update `.ark/workflow.md` with one note that Codex is supported; slash command bodies (Claude side) unchanged.

## Trade-offs `ask reviewer for advice`

- T-1: **Static `&[&Platform]` vs dynamic `Vec<Platform>`.** Static is zero-cost, compile-time-checked, and matches existing `EMPTY_DIRS` precedent. Loses the ability to conditional-compile platforms in/out. Recommendation: static. Trellis uses an object literal in TS; in Rust, the static `const` idiom is the closest equivalent.

- T-2: **Parameterize `update_settings_hook` vs add a sibling `update_codex_hooks_json`.** Parameterizing means one helper, one test surface, clear seam. Sibling means zero call-site churn but two near-identical implementations. Recommendation: parameterize. Mitigate the rename with one-release `pub use` aliases.

- T-3: **Default `init` to install both platforms vs prompt.** Both-by-default matches the parity stance. Prompt asks the user. Confirmed: prompt.

- T-4: **Codex prompt bodies: mechanical translation now vs reauthor for Codex idioms.** Mechanical is cheapest and preserves parity. Reauthored is better Codex UX but 2-3× cost. Confirmed: mechanical.

- T-5: **`AGENTS.md` ownership: managed block (Q5b) vs leave alone vs `@AGENTS.md`-include from `CLAUDE.md`.** Confirmed: managed block.

- T-6: **`Platform` as trait vs struct.** Struct is closed-set, enumerable, debuggable. Trait would allow third-party platforms but adds dyn dispatch and ceremony. Recommendation: struct.

- T-7: **Parity test home: `tests/`, `platforms.rs`, or `templates.rs`?** Reads both template trees via `include_dir!` — `templates.rs` is the natural home next to `templates_have_expected_structure`.

## Validation `test design`

[**Unit Tests**]

- V-UT-1 (G-1, G-2, C-1): `platforms_registry_has_two_entries_in_canonical_order`. `PLATFORMS.len() == 2`; `PLATFORMS[0].id == "claude-code"`, `PLATFORMS[1].id == "codex"`.
- V-UT-2 (G-2, C-1): `platform_by_id_resolves_known_platforms`. Lookup hits and the unknown-id miss.
- V-UT-3 (C-1): `platform_by_cli_flag_resolves`. Lookup by flag.
- V-UT-4 (G-6, C-4): `update_hook_file_is_idempotent_with_explicit_pointer`. Same checks as today's `update_settings_hook` round-trip, calling the parameterized form.
- V-UT-5 (C-19): `update_hook_file_rejects_pointer_without_leading_slash`.
- V-UT-6 (C-19): `update_hook_file_handles_rfc6901_escapes`.
- V-UT-7 (G-6, C-5): `ark_codex_hook_entry_carries_canonical_command`. `hooks[0].command == ARK_CONTEXT_HOOK_COMMAND` and `timeout == 5000`.
- V-UT-8 (G-13, C-17): `layout_codex_paths_resolve_under_root`.
- V-UT-9 (G-13, C-15): `layout_owned_dirs_includes_codex`.

[**Integration Tests**]

- V-IT-1 (G-3, G-4, G-5): `init_with_both_platforms_writes_full_tree`.
- V-IT-2 (G-3): `init_claude_only_omits_codex_paths`.
- V-IT-3 (G-3): `init_codex_only_omits_claude_paths`.
- V-IT-4 (G-9, C-9, C-10): `unload_load_round_trip_preserves_both_platforms`.
- V-IT-5 (G-8, C-9): `unload_preserves_sibling_codex_hook_entries`.
- V-IT-6 (G-11, C-11): `upgrade_re_applies_codex_hook_entry`.
- V-IT-7 (G-11): `upgrade_codex_hooks_json_idempotent`.
- V-IT-8 (G-10, C-12): `remove_clears_both_platforms`.
- V-IT-9 (G-12, C-14): `templates_codex_prompts_match_claude_commands`.
- V-IT-10 (G-3, C-2, C-3): `cli_resolve_platforms_no_flags_tty_prompts`.
- V-IT-11 (G-3): `cli_resolve_platforms_no_flags_non_tty_installs_both`.
- V-IT-12 (G-3): `cli_resolve_platforms_no_x_excludes`.
- V-IT-13 (G-5, G-10): `agents_md_managed_block_round_trip`.
- V-IT-14 (G-9, C-8): `older_snapshot_loads_without_codex_hook_body`.

[**Failure / Robustness Validation**]

- V-F-1 (G-3): `init_with_both_no_flags_errors`. CLI receives `--no-claude --no-codex` → exits non-zero with a clear error.
- V-F-2 (Failure flow 4): `init_partial_failure_codex_hook_write_fails`.
- V-F-3 (Failure flow 7): `legacy_snapshot_without_hook_bodies_loads_clean`.

[**Edge Case Validation**]

- V-E-1 (G-8): `update_hook_file_preserves_unrelated_top_level_keys`.
- V-E-2 (C-7): `codex_prompt_bodies_drop_yaml_frontmatter`. Each `.codex/prompts/ark-*.md` does NOT begin with `---\n`.
- V-E-3 (C-15): `walk_files_on_missing_codex_dir_returns_empty`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-IT-1 |
| G-2 | V-UT-1, V-UT-2, V-UT-8 |
| G-3 | V-IT-1, V-IT-2, V-IT-3, V-IT-10, V-IT-11, V-IT-12, V-F-1 |
| G-4 | V-IT-1, V-IT-2 |
| G-5 | V-IT-1, V-IT-13 |
| G-6 | V-UT-4, V-UT-7, V-IT-6 |
| G-7 | V-IT-6, V-IT-7 |
| G-8 | V-IT-5, V-E-1 |
| G-9 | V-IT-4, V-IT-14 |
| G-10 | V-IT-8, V-IT-13 |
| G-11 | V-IT-6, V-IT-7 |
| G-12 | V-IT-9 |
| G-13 | V-UT-8, V-UT-9 |
| C-1 | V-UT-1, V-UT-2, V-UT-3 |
| C-2 | V-IT-10 |
| C-3 | V-IT-10, V-IT-11 |
| C-4 | V-UT-4 |
| C-5 | V-UT-7 |
| C-6 | V-IT-9 |
| C-7 | V-E-2 |
| C-8 | V-IT-14 |
| C-9 | V-IT-4, V-IT-5 |
| C-10 | V-IT-4 |
| C-11 | V-IT-6, V-IT-7 |
| C-12 | V-IT-8 |
| C-13 | V-IT-10 |
| C-14 | V-IT-9 |
| C-15 | V-UT-9, V-E-3 |
| C-16 | (regression-only; existing init manifest tests cover) |
| C-17 | V-UT-8 |
| C-18 | (source-scan test `platforms_source_no_bare_std_fs`) |
| C-19 | V-UT-5, V-UT-6 |
| C-20 | V-UT-1 |
| C-21 | V-UT-2, V-UT-3 |
