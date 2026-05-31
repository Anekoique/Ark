# `opencode-support` PLAN `00`

> Status: Draft
> Feature: `opencode-support`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Add OpenCode as the third entry in the `PLATFORMS` registry, alongside `claude-code` and `codex`. Reuses the `Platform` struct, `apply_managed_state`, `capture_hook`, `remove_hook`, `remove_dir`, parity-test, and source-scan plumbing established by codex-support — no new abstractions. Three concrete deltas: (1) `OPENCODE_PLATFORM` const + `templates/opencode/` tree (slash commands under `.opencode/commands/ark/`); (2) shared `AGENTS.md` managed block with Codex (`Manifest::record_block` already dedupes on `(file, marker)` — verified at `state/manifest.rs:84-90`, no manifest changes); (3) a TS plugin file `.opencode/plugins/ark-context.ts` shipped as `extra_files` (~30 lines, shells out to `ark context --scope session --format json`, mirrors how Claude/Codex hooks shell out to the same binary). OpenCode has no native JSON `SessionStart` hook, so `OPENCODE_PLATFORM.hook_file = None`; the plugin replaces the hook role.

## Log `None in 00_PLAN`

---

## Spec `Core specification`

[**Goals**]

- **G-1:** `PLATFORMS` registry grows from 2 entries to 3 (`claude-code`, `codex`, `opencode`) in canonical iteration order. The five command bodies (`init` / `upgrade` / `unload` / `load` / `remove`) iterate the same `&[&Platform]` slice; adding OpenCode is a registry entry plus a template tree, not a refactor of any command body. The codex-support test `platforms_registry_has_two_entries_in_canonical_order` is renamed and updated to assert 3 entries with `id`s `["claude-code", "codex", "opencode"]`.

- **G-2:** `OPENCODE_PLATFORM` ships with: a static template tree (`OPENCODE_TEMPLATES`, via `include_dir!`), `dest_dir = ".opencode"`, `removal_root = ".opencode"`, `cli_flag = "opencode"`, `managed_block_target = Some("AGENTS.md")` (shared with Codex per G-7), `hook_file = None` (per G-8), and one entry in `extra_files` for the TS plugin (per G-9).

- **G-3:** `ark init` accepts `--opencode` / `--no-opencode` flags. With no platform flags, on a TTY, the interactive prompt asks which platforms to install (Claude / Codex / OpenCode); all three checked by default. With no flags on a non-TTY, init errors with a message naming all three flags (continues codex-support R-007 precedent). Explicit `--no-opencode` suppresses the prompt and the install. `--opencode` and `--no-opencode` are mutually exclusive (existing clap `conflicts_with` arg-group logic handles this; no new logic).

- **G-4:** `init` ships OpenCode artifacts at canonical paths: `.opencode/commands/ark/quick.md`, `.opencode/commands/ark/design.md`, `.opencode/commands/ark/archive.md`, `.opencode/plugins/ark-context.ts`. Command files carry YAML frontmatter (`description: ...`) per OpenCode's slash-command spec; bodies are mechanical translations of the matching `templates/claude/commands/ark/<name>.md` body — drop Claude's frontmatter (`argument-hint`, `description`), prepend OpenCode's frontmatter (`description` only), keep slash-invocation idioms (`# /ark:quick $ARGUMENTS`) since OpenCode commands ARE slash-invoked. Argument substitution: OpenCode passes `$ARGUMENTS` the same as Claude (verified against the docs at planning time; if it diverges, the translation rule is "match OpenCode's arg variable", recorded in C-6).

- **G-5:** `init` installs the `ARK` managed block in `AGENTS.md` when OpenCode is selected. If Codex is also selected (or already installed), `Manifest::record_block` dedupes on `(file, marker)` so the block is recorded once in `manifest.managed_blocks`, written once on disk. Body identical to Claude's (reuses `MANAGED_BLOCK_BODY`).

- **G-6:** No `Layout` extensions for sub-paths under `.opencode/commands/`. The codex-support pattern added `codex_skills_dir`, `codex_hooks_file`, `codex_config_file` because tests and `apply_managed_state` referenced them. OpenCode's structure is simpler: `extra_files` is a single TS plugin and the commands are extracted by `Platform::templates`. We only add `OPENCODE_DIR = ".opencode"` and `OPENCODE_PLUGIN_FILE = ".opencode/plugins/ark-context.ts"` consts plus matching `Layout::opencode_dir()` / `Layout::opencode_plugin_file()` getters used exclusively by tests. `owned_dirs()` extends from 3 to 4 entries (adds `".opencode"`).

- **G-7:** **Shared managed block.** `OPENCODE_PLATFORM.managed_block_target = Some(AGENTS_MD)` — same value as `CODEX_PLATFORM.managed_block_target`. `Manifest::record_block` already dedupes on `(file, marker)` (verified: `crates/ark-core/src/state/manifest.rs:84-90`). Effects: (a) `init --codex --opencode` on a fresh repo writes `AGENTS.md` once and records it once in the manifest; (b) `init --opencode` then `init --codex` on the same repo is idempotent — the second call's `update_managed_block` returns `false` (no diff) and `record_block` no-ops (already present); (c) `remove` walks `manifest.managed_blocks` once; the block is removed once even if both platforms are installed.

- **G-8:** **No native session-start hook.** `OPENCODE_PLATFORM.hook_file = None`. OpenCode has no JSON-shaped hook entry analogous to Claude's `settings.json` or Codex's `hooks.json`. `apply_managed_state`'s existing `if let Some(spec) = self.hook_file` branch is a no-op for OpenCode. `capture_hook` returns `Ok(None)`. No new code paths in `unload` / `load` / `remove`.

- **G-9:** **TS plugin as `extra_files`.** `OPENCODE_PLATFORM.extra_files = &[(OPENCODE_PLUGIN_FILE, OPENCODE_ARK_CONTEXT_TS)]` where `OPENCODE_ARK_CONTEXT_TS: &'static str = include_str!("../../../templates/opencode/plugins/ark-context.ts")`. Mirrors Codex's `config.toml` treatment: not hash-tracked, re-applied unconditionally on every `init` / `load` / `upgrade`. Plugin is pure TypeScript, ~30 lines, depends only on `node:child_process` and `node:fs` (Bun built-ins, no `package.json`, no npm deps). Hooks `chat.message` (stable, non-experimental). Per-session dedupe via a module-local `Set<sessionID>`. Failure modes: missing `ark` binary on PATH → swallow + log via `await client.app.log` (best-effort; opencode session continues without context); JSON parse failure → swallow + log.

- **G-10:** `remove` removes `.opencode/` wholesale (`removal_root = ".opencode"`) and removes the `ARK` block from `AGENTS.md` once (per G-7 the block is recorded once in `manifest.managed_blocks`, so existing `remove` walks the list once and removes it once). The plugin file `.opencode/plugins/ark-context.ts` goes with the directory. No new code in `remove.rs` — the existing per-platform iteration handles it.

- **G-11:** `upgrade` re-applies `AGENTS.md` block (only on Codex-installed OR OpenCode-installed projects), Codex `SessionStart` hook (only on Codex-installed projects), Codex `config.toml` (only on Codex-installed projects), and OpenCode `ark-context.ts` plugin (only on OpenCode-installed projects). None are hash-tracked. A platform is "installed" iff some path under `Platform::dest_dir` appears in `manifest.files`. Existing iteration logic: `installed(manifest)` filter chained with `apply_managed_state`. No new code.

- **G-12:** **Parity tests** (continue codex-support G-12 pattern): (a) `every_claude_command_has_an_opencode_command_sibling` asserts that for every `templates/claude/commands/ark/<name>.md`, the file `templates/opencode/commands/ark/<name>.md` exists; (b) `opencode_command_bodies_have_opencode_frontmatter` asserts each shipped command's body begins with `---\ndescription:` (OpenCode frontmatter; identical key to Claude but no `argument-hint:` line). Body-content parity is policed by code review at template-edit time, not by mechanical assertion (continues codex-support C-7 REVISED rationale).

- **G-13:** **Layout additions** (G-6 narrowed): `OPENCODE_DIR`, `OPENCODE_PLUGIN_FILE` consts; `Layout::opencode_dir()`, `Layout::opencode_plugin_file()` getters. `owned_dirs()` returns `.ark`, `.claude/commands/ark`, `.codex`, `.opencode`. The last two are only relevant on Codex-/OpenCode-installed projects; on a Claude-only install, the dirs don't exist and `walk_files` yields empty (continues codex-support pattern).

- **G-14:** **Forward-compat preserved.** An existing Claude-only or Claude+Codex project upgraded with the new CLI version remains unchanged on `ark upgrade` — no `.opencode/` directory is created. To add OpenCode, the user re-runs `ark init --opencode` (additive — installs OpenCode artifacts + records them in the manifest; idempotent on Claude/Codex artifacts). This works because `init` is idempotent and platform-keyed iteration is per-flag (continues codex-support G-14).

- **G-15:** **Plugin runtime contract.** `templates/opencode/plugins/ark-context.ts` is the **shipped artifact**. It is authored as plain TypeScript executable by Bun without compilation. The file has no companion `package.json` shipped — it imports only Node/Bun built-ins (`node:child_process`, `node:fs/promises` if needed) and uses no third-party packages. (If we ever need npm deps later, that's a SPEC-revision moment for a follow-up task.)

[**Non-goals**]

- **NG-1:** No `.opencode/agents/*.md` shipped. Decided in DESIGN: agents are description-routed; Ark's three entry points are user-invoked slash commands, so commands are the right surface.
- **NG-2:** No `.opencode/agents/*.md` files for sub-roles (research / implement / check / debug à la Trellis). Out of scope; Ark's pipeline doesn't have sub-agents.
- **NG-3:** No `opencode.json` at the project root. OpenCode auto-discovers `.opencode/`; the project config file is for user customization, not Ark's installation surface.
- **NG-4:** No use of `experimental.*` opencode hooks (`experimental.chat.messages.transform` etc.). Plugin uses only stable hooks (`chat.message`).
- **NG-5:** No fourth platform in this task. Registry leaves room (codex-support NG-3 still holds).
- **NG-6:** No npm dependencies bundled. No `package.json` shipped under `templates/opencode/`. Plugin uses Bun built-ins only.
- **NG-7:** No detection of which CLI tool (Claude / Codex / OpenCode) is installed on the user's machine to drive default selection. All three are checked by default in the interactive prompt (continues codex-support NG-6).
- **NG-8:** No re-implementation of context-gathering logic in TS. The plugin shells out to the `ark` binary; all gathering lives in Rust per ark-context SPEC. The plugin's job is to call `ark context --scope session --format json`, parse the SessionStart envelope, and prepend `additionalContext` to the first user message.
- **NG-9:** No JSON Pointer parser, no new hook abstraction. `HookFileSpec` and `update_hook_file` / `remove_hook_file` / `read_hook_file` are unchanged. `OPENCODE_PLATFORM.hook_file = None` is the entire opt-out.
- **NG-10:** No changes to slash-command (`/ark:quick`, `/ark:design`, `/ark:archive`) Claude bodies in this task.
- **NG-11:** No `Snapshot` schema changes. The plugin file is restored by the existing `Snapshot::files` round-trip (it's an Ark-owned file under `.opencode/`); no `hook_bodies` entry is captured for OpenCode (G-8 — there's no entry to capture).

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                    — InitArgs gains 2 OpenCode flags
│                                            (--opencode / --no-opencode);
│                                            interactive prompt + non-TTY
│                                            error path extend to 3 platforms
└── ark-core/src/
    ├── lib.rs                              — re-exports OPENCODE_PLATFORM
    ├── platforms.rs                        — adds OPENCODE_PLATFORM const;
    │                                          PLATFORMS slice grows to 3 entries;
    │                                          test asserting registry shape updated
    ├── layout.rs                           — adds OPENCODE_DIR,
    │                                          OPENCODE_PLUGIN_FILE consts;
    │                                          Layout::opencode_dir(),
    │                                          Layout::opencode_plugin_file();
    │                                          owned_dirs grows to 4 entries
    ├── io/fs.rs                            — UNCHANGED (no new hook plumbing)
    ├── state/                              — UNCHANGED
    ├── commands/                           — UNCHANGED command bodies; iteration
    │                                          over PLATFORMS picks up the new
    │                                          entry for free
    └── templates.rs                        — adds OPENCODE_TEMPLATES static +
                                               OPENCODE_ARK_CONTEXT_TS const
templates/
├── ark/                                    — unchanged
├── claude/                                 — unchanged
├── codex/                                  — unchanged
└── opencode/                               — NEW
    ├── commands/
    │   └── ark/
    │       ├── quick.md                    — frontmatter + body of claude/quick.md
    │       ├── design.md                   — ditto
    │       └── archive.md                  — ditto
    └── plugins/
        └── ark-context.ts                  — Bun-loaded TS plugin (~30 lines)
```

**Module coupling.** Unchanged from codex-support. Adding OpenCode does not introduce any new module dependencies; it adds one const to `platforms.rs`, one static + one const to `templates.rs`, two consts + two getters to `layout.rs`. The ~30-line plugin is a leaf shipped artifact, not Rust code.

**Call graph for `init` (post-OpenCode):** Identical to codex-support's. The `for platform in selected_platforms` loop at the heart of `init` calls `platform.apply_managed_state(&layout, &mut manifest)?` for each selected entry. Adding OpenCode adds one iteration; no new branches.

**Call graph for `unload`:** Identical to codex-support's C-24 shape. The existing two-stage hook capture (registered `PLATFORMS` entries with `hook_file.is_some()`, then JSON-file scan for orphan entries) is unchanged. OpenCode contributes nothing to either stage (`hook_file = None`, no `.json` files in `.opencode/`).

**Call graph for `load`:** Identical to codex-support's C-22 shape. Files under `.opencode/` are restored from `snapshot.files` (including the plugin TS file). No `hook_bodies` to replay for OpenCode. Then the `for platform in PLATFORMS` canonical re-apply iterates 3 entries: Claude's hook, Codex's hook + config, OpenCode's plugin file.

[**Data Structure**]

```rust
// ark-core/src/platforms.rs additions

/// OpenCode integration. Templates extract under `.opencode/`; managed block
/// shares `AGENTS.md` with Codex (manifest dedupes on `(file, marker)`).
/// SessionStart-equivalent context injection rides a Bun-loaded TS plugin
/// shipped via `extra_files`; OpenCode has no native JSON hook surface.
pub const OPENCODE_PLATFORM: Platform = Platform {
    id: "opencode",
    templates: &crate::templates::OPENCODE_TEMPLATES,
    dest_dir: OPENCODE_DIR,
    removal_root: OPENCODE_DIR,
    cli_flag: "opencode",
    managed_block_target: Some(AGENTS_MD),
    hook_file: None,
    extra_files: &[(OPENCODE_PLUGIN_FILE, OPENCODE_ARK_CONTEXT_TS)],
};

pub const PLATFORMS: &[&Platform] = &[
    &CLAUDE_PLATFORM,
    &CODEX_PLATFORM,
    &OPENCODE_PLATFORM,
];
```

```rust
// ark-core/src/layout.rs additions

/// Root directory for OpenCode integration (relative to project root).
pub const OPENCODE_DIR: &str = ".opencode";

/// `<project>/.opencode/plugins/ark-context.ts` — Bun-loaded plugin that
/// shells out to `ark context --scope session --format json` and prepends
/// the `additionalContext` payload to the first user message.
pub const OPENCODE_PLUGIN_FILE: &str = ".opencode/plugins/ark-context.ts";

impl Layout {
    /// `<project>/.opencode/`.
    pub fn opencode_dir(&self) -> PathBuf { self.resolve(OPENCODE_DIR) }
    /// `<project>/.opencode/plugins/ark-context.ts`.
    pub fn opencode_plugin_file(&self) -> PathBuf { self.resolve(OPENCODE_PLUGIN_FILE) }
}

/// `owned_dirs()` returns these four; `.opencode` only exists on OpenCode-
/// installed projects.
pub fn owned_dirs(&self) -> [&'static str; 4] {
    [ARK_DIR, CLAUDE_COMMANDS_ARK_DIR, CODEX_DIR, OPENCODE_DIR]
}
```

```rust
// ark-core/src/templates.rs additions

pub static OPENCODE_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/opencode/commands");

pub const OPENCODE_ARK_CONTEXT_TS: &str =
    include_str!("../../../templates/opencode/plugins/ark-context.ts");
```

[**API Surface**]

Library re-exports from `ark-core/src/lib.rs`:

```rust
pub use platforms::{
    Platform, PLATFORMS,
    CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM,
};
```

CLI surface (`ark-cli/src/main.rs`):

```rust
struct InitArgs {
    // existing
    #[arg(long, conflicts_with = "no_claude")]    pub claude: bool,
    #[arg(long = "no-claude", conflicts_with = "claude")]    pub no_claude: bool,
    #[arg(long, conflicts_with = "no_codex")]     pub codex: bool,
    #[arg(long = "no-codex", conflicts_with = "codex")]    pub no_codex: bool,
    // new
    #[arg(long, conflicts_with = "no_opencode")]  pub opencode: bool,
    #[arg(long = "no-opencode", conflicts_with = "opencode")]    pub no_opencode: bool,
}
```

The interactive-prompt and non-TTY-error paths in `InitArgs::resolve_platforms` (or wherever this lives) extend from a 2-entry decision matrix to a 3-entry one. No new types.

[**Constraints**]

- **C-1:** All paths under `.opencode/` route through `Layout` getters or `OPENCODE_*` consts. No `".opencode/"` literal outside `layout.rs` and `templates.rs`. Enforced by `platforms.rs`'s and each command file's source-scan test (continues codex-support C-18).
- **C-2:** `OPENCODE_TEMPLATES` is rooted at `templates/opencode/commands` (not whole `templates/opencode/`). Plugin file is shipped via `extra_files` + `OPENCODE_ARK_CONTEXT_TS: &str = include_str!(...)`, mirroring Codex's `config.toml` pattern exactly. Rationale: keeps `Platform::templates` semantically uniform across platforms ("the templates tree extracts under `dest_dir`/...") — for Claude it's `commands/`, for Codex it's `skills/`, for OpenCode it's `commands/`. Plugin is sui generis (one file, fixed location) and rides `extra_files`, the same channel codex uses for `config.toml`.
- **C-3:** `OPENCODE_PLATFORM.hook_file = None`. No `HookFileSpec` value for OpenCode. `Platform::capture_hook` and `Platform::remove_hook` short-circuit to `Ok(None)` / `Ok(false)` for `hook_file = None`, which they already do (verified at `platforms.rs:120-148`).
- **C-4:** `OPENCODE_PLATFORM.removal_root == OPENCODE_PLATFORM.dest_dir == ".opencode"`. The Claude carve-out (`removal_root = ".claude/commands/ark"`, narrower than `dest_dir = ".claude"`) exists because `.claude/settings.json` carries Ark-managed JSON alongside user content. OpenCode has no analogous setup — `.opencode/` is wholly Ark-owned (commands + plugin). Wiping the directory wholesale is correct.
- **C-5:** `Manifest::record_block` dedupe contract is `(file, marker)`. Calling it twice with `(AGENTS.md, ARK)` (once from Codex, once from OpenCode) results in one entry. Verified at `crates/ark-core/src/state/manifest.rs:84-90`. No new code or guards needed.
- **C-6:** Body translation rules (`templates/claude/commands/ark/<name>.md` → `templates/opencode/commands/ark/<name>.md`): drop Claude's frontmatter (`description`, `argument-hint`), prepend OpenCode's frontmatter (`description` only, copied verbatim from Claude's `description`), keep body verbatim including `# /ark:<name> $ARGUMENTS` and inline `/ark:foo` references — OpenCode's slash command surface IS the same shape as Claude's (filename + `/` prefix; argument substitution via `$ARGUMENTS`). No deeper rewrite needed.
- **C-7:** Plugin file is hand-authored TypeScript at `templates/opencode/plugins/ark-context.ts`. Keep ≤80 lines. Hooks `chat.message` only. Module-local `Set<string>` for per-session dedupe. Best-effort error handling: failure to spawn `ark` or parse JSON logs via `await client.app.log({body: {service: "ark", level: "warn", message: ...}})` and returns without injecting (session continues normally).
- **C-8:** Plugin's exec target is the `ark` binary on `PATH`. Same trust model as Claude/Codex hooks (which invoke `ark context …` as a shell command via the platform's hook spec). If `ark` is not on PATH, the plugin logs and skips. (Future work: a config flag for an absolute path. Out of scope here.)
- **C-9:** Source-scan invariants extend to `platforms.rs`: no `".opencode/"` literal, no bare `std::fs::`. Existing per-file scanner in `commands/{init,upgrade,unload,load,remove}.rs` and `platforms.rs` extended to also forbid `".opencode/"` and `"opencode.json"` literals. (Continues codex-support C-18 REVISED.)
- **C-10:** Parity tests are tree-walked, not hand-listed. `every_claude_command_has_an_opencode_command_sibling` reads `CLAUDE_TEMPLATES.files()` and asserts the parallel path exists in `OPENCODE_TEMPLATES`. Adding a Claude command file without an OpenCode twin fails the test at `cargo test` time. (Continues codex-support G-12.)
- **C-11:** Test invariant: `OPENCODE_PLATFORM.id == "opencode"`, `cli_flag == "opencode"`, `dest_dir == ".opencode"`, `managed_block_target == Some("AGENTS.md")`, `hook_file.is_none()`, `extra_files.len() == 1`, `extra_files[0].0 == ".opencode/plugins/ark-context.ts"`. Asserted by `opencode_platform_shape` in `platforms.rs::tests`.
- **C-12:** Plugin runtime test (out-of-process): a smoke test that runs `bun --version` if available and lints the TS file with `bun check templates/opencode/plugins/ark-context.ts`. Marked `#[ignore]` (CI-skippable) to avoid making `cargo test` depend on Bun. Non-blocking; documents the expected environment.
- **C-13:** No changes to `Snapshot` schema, `HookFileSpec`, `update_hook_file`, `remove_hook_file`, `read_hook_file`, `apply_managed_state` body, `capture_hook` body, `remove_hook` body, `remove_dir` body, or any command body. The only diffs are: registry growth, layout consts, two parity tests, and the OpenCode template tree.
- **C-14:** **Forward-compat for old binaries reading new artifacts.** An older `ark` binary (pre-OpenCode) reading a manifest written by a newer binary that recorded `.opencode/` paths in `manifest.files`: the older binary doesn't iterate OpenCode in `PLATFORMS` (it has 2 entries) and doesn't know about `.opencode/`. `unload` from the older binary captures `.opencode/` files only if `walk_files(owned_dirs)` happens to include it — but `owned_dirs` has 3 entries on the old binary, not 4. Result: `.opencode/` files are NOT captured by the old binary's `unload` and remain on disk. This is an acceptable degradation: the old binary is missing a feature; calling `ark upgrade` to bring the binary up to the new version is the supported path. Documented in NG/Risks, not enforced.

---

## Runtime `runtime logic`

[**Main Flow** — `init --opencode`]

1. CLI parses `InitArgs`; `resolve_platforms` returns `[CLAUDE_PLATFORM, OPENCODE_PLATFORM]` (or `[OPENCODE_PLATFORM]` alone if `--no-claude`, etc.).
2. For each selected platform: extract `templates` under `dest_dir`, call `apply_managed_state(&layout, &mut manifest)`. For OpenCode, this:
   a. Writes `AGENTS.md` `ARK` block via `update_managed_block`. If Codex already wrote it, `update_managed_block` returns `false` (idempotent) and `record_block` no-ops.
   b. Skips the `hook_file` arm (`None`).
   c. Writes the TS plugin file via `extra_files` iteration: `layout.resolve(".opencode/plugins/ark-context.ts").write_bytes(OPENCODE_ARK_CONTEXT_TS.as_bytes())`.
3. Manifest persisted; `manifest.files` now includes `.opencode/commands/ark/{quick,design,archive}.md` (from `templates`) and `.opencode/plugins/ark-context.ts` (from `extra_files`); `manifest.managed_blocks` contains one `(AGENTS.md, ARK)` entry regardless of how many platforms wrote it.

[**Main Flow** — runtime (user side, opencode session)]

1. User runs `opencode` in a project initialized with `--opencode`.
2. OpenCode's runtime auto-discovers `.opencode/plugins/ark-context.ts` and loads it via Bun.
3. On the user's first chat message in a new session, the `chat.message` hook fires.
4. Plugin checks its module-local `Set<sessionID>`; first time → run `ark context --scope session --format json` (cwd = project root, found via the plugin context's `directory` parameter).
5. Parse the SessionStart envelope: `{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: <stringified projection>}}`. Extract `additionalContext`.
6. Find the first text part of the user message; prepend `<ark-context>\n${additionalContext}\n</ark-context>\n\n---\n\n` to it.
7. Mark the session ID as processed.
8. Subsequent messages in the same session: hook fires, dedupe set hits, no-op.

[**Failure Flow**]

1. **`ark` binary not on PATH** → spawn fails with `ENOENT` → plugin catches, calls `client.app.log({body: {service: "ark", level: "warn", message: "ark binary not found on PATH; context not injected"}})`, returns without modifying message. Session continues normally.
2. **`ark context` exits non-zero** (e.g. not in an Ark-loaded project) → stderr captured, plugin logs the stderr at `warn`, returns without injecting. Session continues normally.
3. **JSON parse failure** (binary output corrupted) → caught, logged, returns without injecting.
4. **Concurrent first messages** (race in dedupe `Set`) → JS event loop is single-threaded; race is impossible in practice. Documented as N/A.
5. **`init --opencode` on a non-TTY with no `--opencode` / `--no-opencode` flag** → existing non-TTY error path fires, listing all three flags (continues codex-support G-3).
6. **`init --opencode --no-opencode`** → clap's `conflicts_with` rejects at parse time.
7. **Codex already wrote `AGENTS.md` block, then OpenCode init** → `update_managed_block` returns `false` (no diff); `record_block` dedupes; idempotent (G-7).

[**State Transitions**]

- Project state: `(claude, codex, opencode)` triple. Any of {0,0,0} (uninstalled), {1,0,0}, {0,1,0}, {0,0,1}, {1,1,0}, {1,0,1}, {0,1,1}, {1,1,1}. `ark init --<flag>` is additive (transitions 0→1 for the named flag, idempotent on already-installed flags). `ark remove` is wholesale (transitions all to 0). `ark upgrade` re-applies only for platforms in state 1 (G-14).
- Manifest state: `manifest.managed_blocks` contains 0 or 1 entries for `(AGENTS.md, ARK)` regardless of whether 0/1/2 of {Codex, OpenCode} are installed. `manifest.files` contains paths under `.opencode/` iff state {*,*,1}.
- Plugin runtime state: per-process, per-session-ID `Set<string>`. Resets when the opencode process exits.

---

## Implementation `split task into phases`

[**Phase 1 — Layout + templates scaffolding**]

1. Add `OPENCODE_DIR`, `OPENCODE_PLUGIN_FILE` consts in `crates/ark-core/src/layout.rs`.
2. Add `Layout::opencode_dir()`, `Layout::opencode_plugin_file()` getters.
3. Extend `owned_dirs()` from 3 to 4 entries.
4. Create `templates/opencode/` directory tree:
   - `templates/opencode/commands/ark/quick.md` (translated from `templates/claude/commands/ark/quick.md` per C-6)
   - `templates/opencode/commands/ark/design.md` (translated)
   - `templates/opencode/commands/ark/archive.md` (translated)
   - `templates/opencode/plugins/ark-context.ts` (hand-authored, ≤80 lines)
5. Add `OPENCODE_TEMPLATES` static and `OPENCODE_ARK_CONTEXT_TS` const in `crates/ark-core/src/templates.rs`.
6. Update existing source-scan exclusion lists in `platforms.rs` and each `commands/*.rs` to also forbid `.opencode/` literals (per C-9).

**Validates:** `cargo build` green. Unit tests for `Layout::opencode_dir` and `Layout::opencode_plugin_file` pass. Source-scan tests pass on the existing files (no `.opencode/` literals leaking).

[**Phase 2 — Platform registry entry**]

7. Add `OPENCODE_PLATFORM` const in `crates/ark-core/src/platforms.rs`.
8. Extend `PLATFORMS` slice from 2 entries to 3.
9. Update existing test `platforms_registry_has_two_entries_in_canonical_order` → rename to `platforms_registry_has_three_entries_in_canonical_order`, assert 3 entries with ids `["claude-code", "codex", "opencode"]`.
10. Update `Platform::by_id` and `Platform::by_cli_flag` tests to cover `"opencode"` lookups.
11. Add new test `opencode_platform_shape` (per C-11).
12. Add new test `opencode_apply_managed_state_writes_block_and_plugin` (mirrors `codex_apply_managed_state_writes_block_hook_and_extras` but checks: AGENTS.md block written, plugin file written byte-for-byte equal to `OPENCODE_ARK_CONTEXT_TS`, manifest records the block once even when `apply_managed_state` is also called on `CODEX_PLATFORM` afterward).
13. Re-export `OPENCODE_PLATFORM` from `crates/ark-core/src/lib.rs`.

**Validates:** `cargo test -p ark-core` green. Manifest dedupe verified by Phase 2 test #12. The five command bodies (`init`/`upgrade`/`unload`/`load`/`remove`) are unchanged but exercised against 3 platforms in their existing tests.

[**Phase 3 — CLI flags + interactive prompt**]

14. Add `--opencode` / `--no-opencode` flags to `InitArgs` in `crates/ark-cli/src/main.rs`.
15. Extend `resolve_platforms` (or whatever the existing function name is) to handle the third flag pair: explicit `--opencode` → install, explicit `--no-opencode` → skip, neither + TTY → prompt with all 3 platforms checked, neither + non-TTY → error listing all 3 flags.
16. Update CLI tests:
    - `init_explicit_opencode_only_installs_opencode` (mirror existing `init_explicit_codex_only` if present)
    - `init_no_opencode_skips_opencode`
    - `init_non_tty_no_flags_errors_with_three_platform_message`
    - `init_interactive_prompt_offers_three_platforms` (gated on TTY mock if the existing prompt code is testable)

**Validates:** `cargo test --workspace` green. CLI smoke: `cargo run -- init --opencode` on a `tempfile::tempdir()` produces the expected tree.

[**Phase 4 — Parity tests + integration**]

17. Add `every_claude_command_has_an_opencode_command_sibling` test in `crates/ark-core/src/templates.rs::tests` (mirrors `every_claude_command_has_a_codex_skill_sibling`).
18. Add `opencode_command_bodies_have_opencode_frontmatter` test (mirrors `codex_skill_bodies_have_codex_frontmatter_not_claude_frontmatter`).
19. Add round-trip integration test (preferred home: `crates/ark-core/src/commands/load.rs::tests` or a `tests/` sibling):
    - `init --claude --codex --opencode` → assert all three trees + one shared AGENTS.md block in manifest.
    - `unload` → assert snapshot has the union of files; AGENTS.md block removed once.
    - `load` → assert all three trees restored, AGENTS.md re-applied, hooks re-applied per `PLATFORMS` iteration.
    - Byte-identical round-trip (modulo timestamps).
20. Add upgrade test: `init --claude` + `upgrade` → no `.opencode/` created (G-14). `init --opencode` afterward → `.opencode/` appears, idempotent on Claude artifacts.

**Validates:** `cargo test --workspace` fully green. `cargo build --release` produces a binary that smoke-tests cleanly via `cargo run --release -- init --opencode --codex`.

[**Phase 5 — Manual verification + docs**]

21. Run the new `ark` against a fresh tempdir on a developer machine: `ark init --opencode`, inspect `.opencode/`, run `bun --version` then `bun --check .opencode/plugins/ark-context.ts` (validates the TS file is well-formed).
22. (Manual, not in CI) Boot an actual `opencode` session in a fresh tempdir initialized with `ark init --opencode`, send a first message, verify the `<ark-context>...</ark-context>` prefix appears in the model's input. Manual because automating opencode itself is out of scope.
23. Update `.ark/workflow.md` if any phrasing needs to mention OpenCode (e.g. the §3 tier table mentions Codex skills; add an OpenCode commands column). Likely a single-row addition.
24. No README updates needed beyond what `/ark:archive` will pick up automatically.

---

## Trade-offs `ask reviewer for advice`

- **T-1: Plugin file location — `extra_files` vs. embedded in `OPENCODE_TEMPLATES`.**
  - **Option A (chosen):** Plugin file shipped via `extra_files: &[(OPENCODE_PLUGIN_FILE, OPENCODE_ARK_CONTEXT_TS)]`. `OPENCODE_TEMPLATES` is rooted at `templates/opencode/commands/`. Symmetric with Codex's `config.toml` treatment.
    - **Adv.** Re-applied unconditionally on every `init` / `load` / `upgrade` (codex-support C-7 precedent). Manifest does NOT hash-track the plugin file — user edits to `.opencode/plugins/ark-context.ts` are reverted on next `upgrade`. This matches the "Ark-owned, not user-customizable" intent.
    - **Disadv.** One extra `include_str!` macro and one extra `extra_files` entry vs. the alternative.
  - **Option B:** `OPENCODE_TEMPLATES` rooted at `templates/opencode/` whole; plugin extracted via the regular template pipeline. `extra_files = &[]`.
    - **Adv.** One fewer macro invocation. Plugin file naturally lives under `dest_dir = ".opencode"` and extracts via `Platform::templates`.
    - **Disadv.** Plugin gets hash-tracked in `manifest.hashes` (because `templates` extraction populates `manifest.files` and hashes). On `upgrade`, hash mismatches would prompt about user-customized plugin even though the plugin is Ark-owned. Diverges from Codex's `config.toml` precedent.
  - **Recommendation:** A. Mirrors Codex; preserves Ark-owned semantics for the plugin file.

- **T-2: Plugin error handling — log-and-continue vs. fail-loud.**
  - **Option A (chosen):** Log via `client.app.log` and continue silently. User's session works; context just isn't injected.
    - **Adv.** Robust to transient failures (PATH issues, project-not-loaded, race conditions). Mirrors Trellis's `try/catch` shape on the `chat.message` handler.
    - **Disadv.** User may be surprised when `ark context` is silently absent. Discoverability cost.
  - **Option B:** Throw on any error; opencode surfaces the failure.
    - **Adv.** Loud failures get fixed faster.
    - **Disadv.** Many failure modes are user-environment issues (no `ark` on PATH for that shell, etc.); breaking opencode for them is hostile.
  - **Recommendation:** A. Plus a one-line note in the plugin file head comment: "If `ark context` doesn't seem to be running, check `opencode logs` for warnings."

- **T-3: Plugin syntax test — `bun check` (#[ignore]) vs. no check.**
  - **Option A (chosen):** Add a `#[ignore]`d test that runs `bun check templates/opencode/plugins/ark-context.ts` if Bun is available. Document Bun as a dev-dependency for plugin maintainers.
    - **Adv.** Catches syntax errors in the plugin source at developer-machine time.
    - **Disadv.** Adds an external dep to the dev workflow (Bun must be installed locally to exercise the test).
  - **Option B:** No automated check; rely on manual smoke test (Phase 5).
    - **Adv.** Zero new dev-deps.
    - **Disadv.** Easy to ship a broken plugin; the parity test catches frontmatter shape but not syntax errors in the TS body.
  - **Recommendation:** A, but `#[ignore]`d so CI is unaffected. Reviewer may downgrade to B if they prefer zero external test deps.

- **T-4: Body translation — keep slash idioms verbatim or rewrite.**
  - **Option A (chosen):** Keep Claude's body verbatim (`# /ark:quick $ARGUMENTS`, inline `/ark:foo`), only swap frontmatter. Per docs, OpenCode commands ARE slash-invoked with `$ARGUMENTS` substitution — same shape.
    - **Adv.** Minimal divergence. Parity test stays simple.
    - **Disadv.** Assumes OpenCode's `$ARGUMENTS` semantics match Claude's. If they don't (e.g. opencode uses `{{args}}` or similar), bodies render with literal `$ARGUMENTS` text.
  - **Option B:** Translate slash idioms to OpenCode-specific syntax if it diverges.
    - **Adv.** Correct rendering on first ship.
    - **Disadv.** Requires Bun-side empirical verification of OpenCode's command-body templating that's hard to automate.
  - **Recommendation:** A, with Phase 5 manual verification step #22 specifically checking that `$ARGUMENTS` substitutes correctly in opencode. If it doesn't, the fix is a one-line rewrite rule (C-6) that's a follow-up commit, not a SPEC revision.

- **T-5: AGENTS.md sharing — let it happen, or scope under `<!-- ARK:CODEX -->` / `<!-- ARK:OPENCODE -->` separately.**
  - **Option A (chosen):** Both platforms write the same `<!-- ARK:START -->...<!-- ARK:END -->` block. Manifest dedupes.
    - **Adv.** Minimal: one block, identical content, one round-trip path. No new manifest fields.
    - **Disadv.** A user who wants different per-platform content in `AGENTS.md` has to deviate from Ark; not in scope (NG-7-ish).
  - **Option B:** Per-platform markers (`ARK:CODEX`, `ARK:OPENCODE`). Two blocks in the same file.
    - **Adv.** Future-proof if per-platform divergence ever matters.
    - **Disadv.** Ugly; two visually-identical blocks in `AGENTS.md`. Manifest grows by one entry per platform. New `MANAGED_BLOCK_BODY_OPENCODE` const required.
  - **Recommendation:** A. The codex-support SPEC already established sharing semantics (G-5 in codex-support); we're following it.

---

## Validation `test design`

[**Unit Tests**]

- **V-UT-1:** `OPENCODE_PLATFORM` shape: `id == "opencode"`, `cli_flag == "opencode"`, `dest_dir == ".opencode"`, `removal_root == ".opencode"`, `managed_block_target == Some(AGENTS_MD)`, `hook_file.is_none()`, `extra_files.len() == 1`, `extra_files[0] == (".opencode/plugins/ark-context.ts", OPENCODE_ARK_CONTEXT_TS)`. Maps G-2, G-7, G-8, G-9, C-3, C-11.
- **V-UT-2:** `PLATFORMS.len() == 3` and ids in canonical order `["claude-code", "codex", "opencode"]`. Maps G-1.
- **V-UT-3:** `Platform::by_id("opencode")` and `Platform::by_cli_flag("opencode")` resolve correctly. Maps G-1.
- **V-UT-4:** `OPENCODE_PLATFORM.apply_managed_state(&layout, &mut manifest)` writes `.opencode/plugins/ark-context.ts` byte-for-byte equal to `OPENCODE_ARK_CONTEXT_TS`, writes the AGENTS.md `ARK` block, records the block once. Maps G-5, G-9, C-2.
- **V-UT-5:** Calling `apply_managed_state` for `CODEX_PLATFORM` then `OPENCODE_PLATFORM` (or vice versa) results in `manifest.managed_blocks.len() == 1` (deduped on `(AGENTS.md, ARK)`) and one on-disk write of the `<!-- ARK:START -->...` block. Maps G-7, C-5.
- **V-UT-6:** `OPENCODE_PLATFORM.capture_hook(&layout, &mut snapshot)` returns `Ok(None)` and leaves `snapshot.hook_bodies` untouched (no hook to capture). Maps G-8, C-3.
- **V-UT-7:** `OPENCODE_PLATFORM.remove_hook(&layout)` returns `Ok(false)` (no hook file). Maps G-8.
- **V-UT-8:** `OPENCODE_PLATFORM.remove_dir(&layout)` returns `false` on non-existent dir, `true` after `apply_managed_state` populated `.opencode/`. Maps G-10.
- **V-UT-9:** `Layout::opencode_dir()` and `Layout::opencode_plugin_file()` return paths joined to `root` via `resolve`. Maps G-13.
- **V-UT-10:** `Layout::owned_dirs()` returns the 4-entry array `[ARK_DIR, CLAUDE_COMMANDS_ARK_DIR, CODEX_DIR, OPENCODE_DIR]`. Maps G-13.
- **V-UT-11:** Source-scan invariant: no `".opencode/"` literal in `platforms.rs` (already exempt: `layout.rs` and `templates.rs` are exempt sites for path consts; the `dest_dir`/`removal_root`/`extra_files` references go through `OPENCODE_DIR` / `OPENCODE_PLUGIN_FILE` consts). Maps C-9.
- **V-UT-12:** Source-scan invariant extension: each of `commands/{init,upgrade,unload,load,remove}.rs` has no `".opencode/"` or `"opencode.json"` literal. Maps C-9.

[**Integration Tests**]

- **V-IT-1:** Parity: `every_claude_command_has_an_opencode_command_sibling`. Walk `CLAUDE_TEMPLATES.files()`; for each `.md` file at `commands/ark/<name>.md`, assert `OPENCODE_TEMPLATES.get_file("ark/<name>.md").is_some()`. Maps G-12, C-10.
- **V-IT-2:** Parity: `opencode_command_bodies_have_opencode_frontmatter`. Walk `OPENCODE_TEMPLATES.files()`; for each `.md`, assert body starts with `---\n` and the first non-`---` line begins with `description:`. Assert no `argument-hint:` line in the frontmatter block (Claude-specific frontmatter would fail this). Maps G-12.
- **V-IT-3:** End-to-end round-trip: `init --claude --codex --opencode` on a `tempdir` → `unload` → `load` produces byte-identical disk state (modulo timestamps in `.ark.db`). Maps G-1, G-5, G-7, G-9, G-10, G-11.
- **V-IT-4:** `init --opencode` alone (no Claude, no Codex) → only `.opencode/`, `.ark/`, and `AGENTS.md` exist (no `.claude/`, no `CLAUDE.md`, no `.codex/`). Maps G-3, G-7.
- **V-IT-5:** `init --claude` then `init --opencode` (additive, two separate calls) → both `.claude/` and `.opencode/` exist, `CLAUDE.md` and `AGENTS.md` both have ARK blocks, manifest has 2 managed-block entries (one per file). Idempotent: re-running `init --opencode` is a no-op. Maps G-14.
- **V-IT-6:** Upgrade preservation: `init --claude`, then upgrade (newer binary), then `cargo run -- upgrade` → no `.opencode/` directory created. Adding `init --opencode` after upgrade works. Maps G-14.
- **V-IT-7:** Remove with shared block: `init --codex --opencode` → `remove` → `AGENTS.md` exists but the `ARK` block is gone (other content preserved); `.codex/` and `.opencode/` are gone. Maps G-10, G-7.
- **V-IT-8:** CLI flag handling: `init --opencode --no-opencode` errors at clap parse (`conflicts_with`). Maps G-3.

[**Failure / Robustness Validation**]

- **V-F-1:** `init` on non-TTY with no platform flags errors with a message naming `--claude`, `--codex`, `--opencode` (and their `--no-*` counterparts). Maps G-3.
- **V-F-2:** `init --opencode` when `templates/opencode/commands/ark/` is missing a file (compile-time concern; the parity test V-IT-1 catches it). Maps G-12 (negative case).
- **V-F-3:** Plugin runtime failure modes (documented, not unit-tested in Rust): `ark` not on PATH, `ark context` exit nonzero, JSON parse error → plugin logs and returns. Validated by reading the plugin source and the `try/catch` shape. (No mock-Bun harness; out of scope per T-3 / Phase 5.) Maps C-7.
- **V-F-4:** Manifest forward-compat: a single unit test asserts `Snapshot` deserialization is forward-compatible by adding an `.opencode/`-prefixed path to a hand-rolled `Snapshot` JSON and round-tripping through `serde_json::from_str` → `serde_json::to_string` (no field renames; existing `#[serde(default)]` on `hook_bodies` is the only forward-compat concern). Maps C-14.

[**Edge Case Validation**]

- **V-E-1:** Re-applying `apply_managed_state` for OpenCode multiple times on the same project: idempotent. Plugin file is byte-identical, AGENTS.md block is unchanged, `record_block` no-ops, second call's `update_managed_block` returns `false`. Maps G-5, G-9.
- **V-E-2:** `ark unload` on a project with `.opencode/` but a corrupt manifest (no `OPENCODE_DIR` files in `manifest.files`): `walk_files(owned_dirs)` still picks up `.opencode/` via the directory walk (independent of manifest), captured into `snapshot.files`. `load` restores them. Tests this behavior is unchanged from codex-support's robustness model.
- **V-E-3:** Empty `.opencode/` directory (created by `ark init --opencode` on a system that somehow swallowed the template extraction — synthetic case): `unload` captures nothing under `.opencode/`; `remove_dir` returns `false`. Not a real failure mode, but the path through the code is exercised.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1  | V-UT-2, V-UT-3, V-IT-3 |
| G-2  | V-UT-1 |
| G-3  | V-IT-4, V-IT-8, V-F-1 |
| G-4  | V-IT-3, V-IT-1 (existence), V-IT-2 (frontmatter) |
| G-5  | V-UT-4, V-E-1 |
| G-6  | V-UT-9, V-UT-10 |
| G-7  | V-UT-5, V-IT-3, V-IT-4, V-IT-7 |
| G-8  | V-UT-6, V-UT-7 |
| G-9  | V-UT-1, V-UT-4, V-E-1 |
| G-10 | V-UT-8, V-IT-7 |
| G-11 | V-IT-3 |
| G-12 | V-IT-1, V-IT-2, V-F-2 |
| G-13 | V-UT-9, V-UT-10 |
| G-14 | V-IT-5, V-IT-6 |
| G-15 | (manual, Phase 5 #21–#22; documented in C-7, C-12) |
| C-1  | V-UT-11, V-UT-12 |
| C-2  | V-UT-4 |
| C-3  | V-UT-1, V-UT-6 |
| C-4  | V-UT-1 |
| C-5  | V-UT-5 (existing manifest dedupe behavior; verified by source reference at planning time) |
| C-6  | V-IT-2 (frontmatter shape); body verbatim is policed by code review (C-10 rationale) |
| C-7  | (documented, not unit-tested; Phase 5 #21–#22) |
| C-8  | (documented; trust model per Claude/Codex precedent) |
| C-9  | V-UT-11, V-UT-12 |
| C-10 | V-IT-1, V-IT-2 |
| C-11 | V-UT-1 |
| C-12 | (`#[ignore]`d Bun smoke test; Phase 5 #21) |
| C-13 | (no diff to assert; absence of changes in command bodies and `Snapshot` schema verified by `cargo test --workspace` continuing to pass) |
| C-14 | V-F-4 |
