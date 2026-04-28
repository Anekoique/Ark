
[**Goals**]

- **G-1:** `PLATFORMS` registry grows from 2 entries to 3 (`claude-code`, `codex`, `opencode`) in canonical iteration order. The five command bodies (`init` / `upgrade` / `unload` / `load` / `remove`) iterate the same `&[&Platform]` slice; adding OpenCode is a registry entry plus a template tree, not a refactor of any command body. The existing test asserting registry shape (currently named `platforms_registry_has_two_entries_in_canonical_order`) is renamed to `platforms_registry_has_three_entries_in_canonical_order` and updated to assert 3 entries with `id`s `["claude-code", "codex", "opencode"]`.

- **G-2:** `OPENCODE_PLATFORM` ships with: a static template tree (`OPENCODE_TEMPLATES`, via `include_dir!`), `dest_dir = ".opencode/commands"` (parallel to Codex's `dest_dir = ".codex/skills"` so `Platform::templates` extracts cleanly under the commands subtree), `removal_root = ".opencode"` (the parent dir, so `remove` wipes both `commands/` and `plugins/`), `cli_flag = "opencode"`, `managed_block_target = Some("AGENTS.md")` (shared with Codex per G-7), `hook_file = None` (per G-8), and one entry in `extra_files` for the TS plugin (per G-9). *(Corrected during EXECUTE: 02_PLAN's initial wording had `dest_dir = ".opencode"`, which would have caused templates to extract directly under `.opencode/ark/` instead of `.opencode/commands/ark/`. Per workflow §4 fidelity rule.)*

- **G-3:** `ark init` accepts `--opencode` / `--no-opencode` flags. With no platform flags, on a TTY, the interactive prompt asks which platforms to install (Claude / Codex / OpenCode); all three checked by default. With no flags on a non-TTY, init errors with a message that names all three flag pairs: `--claude` / `--no-claude`, `--codex` / `--no-codex`, `--opencode` / `--no-opencode` (the message lists every flag explicitly so the user sees the full surface). Mutual exclusivity is **behavioral, not declarative**: when both `--<flag>` and `--no-<flag>` are passed for the same platform, the existing `resolve_platforms_pure` filter `f.on && !f.off` excludes that platform (the negative wins on per-platform conflict). This matches the existing `--claude --no-claude` and `--codex --no-codex` pairs verbatim. No `clap::ArgGroup`, no `conflicts_with`. *(Note: 01_PLAN's "positive wins" wording was incorrect against the actual code at `main.rs:132–161`; corrected during EXECUTE per workflow §4 fidelity rule.)*

- **G-4:** `init` ships OpenCode artifacts at canonical paths: `.opencode/commands/ark/quick.md`, `.opencode/commands/ark/design.md`, `.opencode/commands/ark/archive.md`, `.opencode/plugins/ark-context.ts`. Command files carry YAML frontmatter (`description: ...`) per OpenCode's slash-command spec; bodies are mechanical translations of the matching `templates/claude/commands/ark/<name>.md` body — drop Claude's frontmatter (`argument-hint`, `description`), prepend OpenCode's frontmatter (`description` only, copied verbatim from Claude's value), keep slash-invocation idioms (the backtick-quoted heading `` # `/ark:<name> $ARGUMENTS` `` per C-6) verbatim. OpenCode commands ARE slash-invoked with `$ARGUMENTS` substitution (verified against opencode docs: "Pass arguments to commands using the `$ARGUMENTS` placeholder."). If a future opencode release renames the substitution token, C-6 names the rewrite path.

- **G-5:** `init` installs the `ARK` managed block in `AGENTS.md` when OpenCode is selected. If Codex is also selected (or already installed), `Manifest::record_block` dedupes on `(file, marker)` so the block is recorded once in `manifest.managed_blocks`, written once on disk. Body identical to Claude's (reuses `MANAGED_BLOCK_BODY`). Verified at `crates/ark-core/src/state/manifest.rs:79–91`.

- **G-6:** No `Layout` extensions for sub-paths under `.opencode/commands/`. We add `OPENCODE_DIR = ".opencode"` and `OPENCODE_PLUGIN_FILE = ".opencode/plugins/ark-context.ts"` consts plus matching `Layout::opencode_dir()` / `Layout::opencode_plugin_file()` getters used by tests and by `OPENCODE_PLATFORM`'s `extra_files`. `owned_dirs()` extends from `[PathBuf; 3]` to `[PathBuf; 4]` (adds `self.opencode_dir()`).

- **G-7:** **Shared managed block.** `OPENCODE_PLATFORM.managed_block_target = Some(AGENTS_MD)` — same value as `CODEX_PLATFORM.managed_block_target`. `Manifest::record_block` already dedupes on `(file, marker)` (see G-5). Effects: (a) `init --codex --opencode` on a fresh repo writes `AGENTS.md` once and records it once in the manifest; (b) `init --opencode` then `init --codex` on the same repo is idempotent — the second call's `update_managed_block` returns `false` (no diff) and `record_block` no-ops (already present); (c) `remove` walks `manifest.managed_blocks` once; the block is removed once even if both platforms are installed.

- **G-8:** **No native session-start hook.** `OPENCODE_PLATFORM.hook_file = None`. OpenCode has no JSON-shaped hook entry analogous to Claude's `settings.json` or Codex's `hooks.json`. `apply_managed_state`'s existing `if let Some(spec) = self.hook_file` branch is a no-op for OpenCode. `capture_hook` returns `Ok(None)`. `remove_hook` returns `Ok(false)`. No new code paths in `unload` / `load` / `remove`.

- **G-9:** **Two-hook TS plugin shipped via `extra_files`.** `OPENCODE_PLATFORM.extra_files = &[(OPENCODE_PLUGIN_FILE, OPENCODE_ARK_CONTEXT_TS)]` where `OPENCODE_ARK_CONTEXT_TS: &'static str = include_str!("../../../templates/opencode/plugins/ark-context.ts")`. Mirrors Codex's `config.toml` treatment: not hash-tracked, re-applied unconditionally on every `init` / `load` / `upgrade`. Plugin contract:
  - **Source language:** TypeScript (per T-6). Bun-loaded natively (no transpile in the user's runtime path).
  - **Imports:** Node/Bun built-ins only (`node:child_process` for `execFileSync` or `node:util` for `promisify(execFile)`). No `package.json`, no npm deps.
  - **Export shape:** `export default async ({ directory, client }) => ({ "chat.message": <handler>, "experimental.chat.messages.transform": <handler> })`. Per the Trellis reference at `reference/Trellis/packages/cli/src/templates/opencode/plugins/session-start.js:350–453`.
  - **Hooks used (two, complementary):**
    - `chat.message`: read-only notification. Receives `(input)`, where `input.sessionID` identifies the session. Plugin checks a module-local `Set<string>` (processed sessions). On first hit per session, runs `ark context --scope session --format json` (cwd = `directory` from the factory args), parses the SessionStart envelope, stores the unwrapped `additionalContext` string in a module-local `Map<sessionID, string>` keyed by `sessionID`, and marks the session processed. On subsequent messages, no-op.
    - `experimental.chat.messages.transform`: mutates `output.messages`. Receives `(input, output)`. Plugin finds the last user message (`role === "user"`), looks up its `sessionID` in the pending map; if a value is present and the message has not yet been transformed, prepends `<ark-context>\n${additionalContext}\n</ark-context>\n\n---\n\n` to the first text part's `text` field, then deletes the map entry (consume-on-use).
  - **Ordering assumption:** `chat.message` fires before `experimental.chat.messages.transform` for the same user message. The Trellis reference at `reference/Trellis/packages/cli/src/templates/opencode/plugins/session-start.js` relies on the same ordering implicitly (line 387 stores in `chat.message`, line 430 reads in `transform`). If a future opencode release reverses this order, `pendingContext.get(sessionID)` returns `undefined` in transform and injection silently stops; mitigation matches C-15 — rework plugin source to populate `pendingContext` from a different hook (or to mutate inline if a single-hook mutation surface lands), no SPEC delta.
  - **Pure helpers** (named for clarity, NOT exported): `buildEnvelopePrefix(additionalContext: string): string` returns `<ark-context>\n${additionalContext}\n</ark-context>\n\n---\n\n`. `shouldInject(sessionID: string, processed: Set<string>): boolean` returns `!processed.has(sessionID)`. **Critical**: these are plain `function` declarations, NOT `export function`. OpenCode's plugin runtime invokes every named export at load time with no arguments — exporting a parameterized helper crashes plugin loading (verified empirically: `error=undefined is not an object (evaluating 'processed.has')`). Test harnesses can validate the helpers exist via the string-level `opencode_plugin_keeps_helpers_internal` test in `crates/ark-core/src/templates.rs::tests`, which asserts the helpers are defined, NOT exported, and have live consumers in the default-exported factory.
  - **Failure handling:** Best-effort. `execFileSync` runs with `timeout: 5000` (5s) to cap the worst case if `ark context` hangs — the chat hook stays responsive. On `execFileSync` throw (`ENOENT` for missing binary, `ETIMEDOUT` on hang, non-zero exit), `JSON.parse` throw, or unexpected payload shape: the plugin catches, calls `client.app.log({ body: { service: "ark", level: "warn", message: <reason> } })` and returns without modifying state. On the first swallowed failure per session (per TR-2), the plugin additionally writes a single-line stderr note: `ark-context: skipped context injection (see opencode logs)`. Subsequent failures in the same session are silent on stderr (still logged).
  - **Per-session dedupe:** module-local `Set<string>` and `Map<string, string>`. JS event loop is single-threaded; no concurrency primitive needed.
  - **Size:** ≤95 lines including license/header comment. (Original cap was 80; relaxed to 95 to accommodate the 5s `execFileSync` timeout, the consume-on-injection ordering fix for non-text first turns, and a header comment naming the timeout constant. The cap exists to prevent the plugin from growing into a meaningful TS module that would need its own tests / `package.json` / build step — 95 is still well within that spirit.)

- **G-10:** `remove` removes `.opencode/` wholesale (`removal_root = ".opencode"`) and removes the `ARK` block from `AGENTS.md` once (per G-7 the block is recorded once in `manifest.managed_blocks`, so existing `remove` walks the list once and removes it once). The plugin file `.opencode/plugins/ark-context.ts` goes with the directory. No new code in `remove.rs` — the existing per-platform iteration handles it.

- **G-11:** `upgrade` re-applies `AGENTS.md` block (only on Codex-installed OR OpenCode-installed projects), Codex `SessionStart` hook (only on Codex-installed projects), Codex `config.toml` (only on Codex-installed projects), and OpenCode `ark-context.ts` plugin (only on OpenCode-installed projects). None are hash-tracked. A platform is "installed" iff some path under `Platform::dest_dir` appears in `manifest.files`. Existing iteration logic: `installed(manifest)` filter chained with `apply_managed_state`. No new code.

- **G-12:** **Parity tests.** Tree-walked, not hand-listed. (a) `every_claude_command_has_an_opencode_command_sibling`: walk `CLAUDE_TEMPLATES.files()`; for each file at `commands/ark/<name>.md`, assert `OPENCODE_TEMPLATES.get_file("ark/<name>.md").is_some()`. Adding a Claude command file without an OpenCode twin fails the test at `cargo test` time. (b) `opencode_command_bodies_have_opencode_frontmatter`: walk `OPENCODE_TEMPLATES.files()`; for each `.md`, assert body starts with `---\n` and the first non-`---` line begins with `description:`. Assert no `argument-hint:` line appears in the frontmatter block (Claude-specific). Assert each command body contains the literal heading `` # `/ark:<name> $ARGUMENTS` `` (backtick-quoted, matching the verbatim shape in `templates/claude/commands/ark/{quick,design,archive}.md:6` — positive sanity check on slash-invocation idiom retention). Body-content drift between Claude and OpenCode is not mechanically asserted beyond these checks; deeper drift is policed by code review at template-edit time.

- **G-13:** **Layout additions.** `OPENCODE_DIR`, `OPENCODE_PLUGIN_FILE` consts; `Layout::opencode_dir()`, `Layout::opencode_plugin_file()` getters. `owned_dirs()` returns `[PathBuf; 4]` containing `[self.ark_dir(), self.claude_commands_ark_dir(), self.codex_dir(), self.opencode_dir()]`. The last two entries only exist on Codex-/OpenCode-installed projects; on a Claude-only install, the dirs don't exist and `walk_files` yields empty (this is the existing semantic for `.codex/` per `layout.rs:232–243` doc comment, extended unchanged to `.opencode/`).

- **G-14:** **Forward-compat preserved.** An existing Claude-only or Claude+Codex project upgraded with the new CLI version remains unchanged on `ark upgrade` — no `.opencode/` directory is created. To add OpenCode, the user re-runs `ark init --opencode` (additive — installs OpenCode artifacts + records them in the manifest; idempotent on Claude/Codex artifacts). `init` is idempotent and platform-keyed iteration is per-flag: a flag absent from the user's invocation triggers no work for that platform.

- **G-15:** **Plugin runtime contract.** `templates/opencode/plugins/ark-context.ts` is the **shipped artifact**. It is authored as plain TypeScript executable by Bun without compilation (Bun runs `.ts` natively per opencode plugin docs). The file has no companion `package.json` shipped — it imports only Node/Bun built-ins (`node:child_process`, `node:util`) and uses no third-party packages. The export shape and hook contract are pinned in G-9. If we ever need npm deps later, that's a SPEC-revision moment for a follow-up task.

[**Non-goals**]

- **NG-1:** No `.opencode/agents/*.md` shipped. Decided in DESIGN: agents are description-routed; Ark's three entry points are user-invoked slash commands.
- **NG-2:** No `.opencode/agents/*.md` files for sub-roles (research / implement / check / debug à la Trellis). Out of scope; Ark's pipeline doesn't have sub-agents.
- **NG-3:** No `opencode.json` at the project root. OpenCode auto-discovers `.opencode/`; the project config file is for user customization, not Ark's installation surface.
- **NG-4:** No use of `experimental.*` opencode hooks **other than** `experimental.chat.messages.transform`. The plugin is forced to use that hook because it is the only hook that mutates user messages before they reach the LLM (the read-only `chat.message` cannot, per the Trellis reference at `session-start.js:350–453`). C-15 documents the fragility and migration path. No other experimental hooks (`experimental.*`) are adopted.
- **NG-5:** No fourth platform in this task. Registry leaves room.
- **NG-6:** No npm dependencies bundled. No `package.json` shipped under `templates/opencode/`. Plugin uses Bun built-ins only.
- **NG-7:** No detection of which CLI tool (Claude / Codex / OpenCode) is installed on the user's machine to drive default selection. All three are checked by default in the interactive prompt; the user opts out per platform via `--no-<flag>`.
- **NG-8:** No re-implementation of context-gathering logic in TS. The plugin shells out to the `ark` binary; all gathering lives in Rust per the ark-context SPEC. The plugin's job is: call `ark context --scope session --format json`, parse the SessionStart envelope, prepend `additionalContext` to the first user text part on first message of each session.
- **NG-9:** No JSON Pointer parser, no new hook abstraction. `HookFileSpec` and `update_hook_file` / `remove_hook_file` / `read_hook_file` are unchanged. `OPENCODE_PLATFORM.hook_file = None` is the entire opt-out.
- **NG-10:** No changes to slash-command (`/ark:quick`, `/ark:design`, `/ark:archive`) Claude bodies in this task.
- **NG-11:** No `Snapshot` schema changes. The plugin file is restored by the existing `Snapshot::files` round-trip (it's an Ark-owned file under `.opencode/`); no `hook_bodies` entry is captured for OpenCode (G-8).

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                    — InitArgs gains 2 OpenCode flags
│                                            (bare #[arg(long)], no
│                                            conflicts_with — matches existing
│                                            --claude/--no-claude shape).
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
    │                                          owned_dirs grows to [PathBuf; 4]
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
        └── ark-context.ts                  — Bun-loaded TS plugin (≤80 lines),
                                               two-hook (chat.message +
                                               experimental.chat.messages.transform)
```

**Module coupling.** Adding OpenCode introduces no new module dependencies. One const added to `platforms.rs`, one static + one const to `templates.rs`, two consts + two getters to `layout.rs`. The ≤80-line plugin is a leaf shipped artifact, not Rust code.

**Call graph for `init` (post-OpenCode):** Identical to existing. The `for platform in selected_platforms` loop calls `platform.apply_managed_state(&layout, &mut manifest)?` for each selected entry. Adding OpenCode adds one iteration; no new branches.

**Call graph for `unload`:** Unchanged. Two-stage hook capture (registered `PLATFORMS` entries with `hook_file.is_some()`, then JSON-file scan for orphan entries) — OpenCode contributes nothing to either stage (`hook_file = None`, no `.json` files in `.opencode/`).

**Call graph for `load`:** Unchanged. Files under `.opencode/` are restored from `snapshot.files` (including the plugin TS file). No `hook_bodies` to replay for OpenCode. Then the `for platform in PLATFORMS` canonical re-apply iterates 3 entries: Claude's hook, Codex's hook + config, OpenCode's plugin file.

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
    dest_dir: OPENCODE_COMMANDS_DIR,
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

/// `<project>/.opencode/commands/` — where OpenCode slash-command markdown
/// files are extracted. `OPENCODE_TEMPLATES` is rooted parallel to this, so
/// `Platform::templates` extracts under `dest_dir = OPENCODE_COMMANDS_DIR`
/// without an extra path component (mirrors `CODEX_SKILLS_DIR` /
/// `CODEX_TEMPLATES` rooted at `templates/codex/skills/`).
pub const OPENCODE_COMMANDS_DIR: &str = ".opencode/commands";

/// `<project>/.opencode/plugins/ark-context.ts` — Bun-loaded plugin that
/// shells out to `ark context --scope session --format json` and prepends
/// the `additionalContext` payload to the first user message.
pub const OPENCODE_PLUGIN_FILE: &str = ".opencode/plugins/ark-context.ts";

impl Layout {
    /// `<project>/.opencode/`.
    pub fn opencode_dir(&self) -> PathBuf { self.resolve(OPENCODE_DIR) }
    /// `<project>/.opencode/plugins/ark-context.ts`.
    pub fn opencode_plugin_file(&self) -> PathBuf { self.resolve(OPENCODE_PLUGIN_FILE) }

    /// Directories whose full contents are captured by `unload` and restored by
    /// `load`. `walk_files` on a missing directory yields empty, so the four
    /// entries are silently no-ops on platforms not installed.
    pub fn owned_dirs(&self) -> [PathBuf; 4] {
        [
            self.ark_dir(),
            self.claude_commands_ark_dir(),
            self.codex_dir(),
            self.opencode_dir(),
        ]
    }
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

CLI surface (`ark-cli/src/main.rs`): bare `#[arg(long)]` for both flags. Mirrors existing Claude / Codex flag pairs. No `conflicts_with`, no `ArgGroup`. Resolution is in `resolve_platforms_pure` (positive-wins).

```rust
struct InitArgs {
    // existing
    #[arg(long)] pub claude: bool,
    #[arg(long = "no-claude")] pub no_claude: bool,
    #[arg(long)] pub codex: bool,
    #[arg(long = "no-codex")] pub no_codex: bool,
    // new
    #[arg(long)] pub opencode: bool,
    #[arg(long = "no-opencode")] pub no_opencode: bool,
}
```

`InitArgs::flags` extends its `match p.cli_flag` from 2 arms to 3 to map each `Platform`'s `cli_flag` to its `(self.<flag>, self.no_<flag>)` pair. Non-TTY error message (currently `"init requires --claude, --codex, or both when stdin is not a TTY (use --no-claude / --no-codex to opt out)"` at `main.rs:157–160`) extended to name `--opencode` and `--no-opencode`. The interactive-prompt `for platform in PLATFORMS` loop at `main.rs:165–177` already iterates the registry and grows for free.

[**Constraints**]

- **C-1:** All paths under `.opencode/` route through `Layout` getters or `OPENCODE_*` consts. No `".opencode/"` literal outside `layout.rs` and `templates.rs` (sanctioned sites). Enforced by extending the existing per-file source-scan tests in `platforms.rs` and `commands/{init,upgrade,unload,load,remove}.rs` to also forbid `".opencode/"` and `"opencode.json"` literals — these scanners follow the established pattern in those files.
- **C-2:** `OPENCODE_TEMPLATES` is rooted at `templates/opencode/commands` (not whole `templates/opencode/`). The plugin file is shipped via `extra_files` + `OPENCODE_ARK_CONTEXT_TS: &str = include_str!(...)`, mirroring Codex's `config.toml` pattern: `templates/codex/skills/` is the codex `templates` root and `CODEX_CONFIG_TOML` is shipped via a parallel `include_str!`. Rationale: `Platform::templates` extracts under `dest_dir/`; for Claude that's `commands/`, for Codex `skills/`, for OpenCode `commands/`. Plugin is sui generis (one file, fixed location) and rides `extra_files` because it is **not hash-tracked** (re-applied unconditionally on every `init` / `load` / `upgrade`); a user-edited plugin is reverted on the next `upgrade`, matching the "Ark-owned" intent.
- **C-3:** `OPENCODE_PLATFORM.hook_file = None`. `Platform::capture_hook` and `Platform::remove_hook` short-circuit to `Ok(None)` / `Ok(false)` for `hook_file = None` (verified at `platforms.rs:120–148`).
- **C-4:** `OPENCODE_PLATFORM.removal_root == OPENCODE_PLATFORM.dest_dir == ".opencode"`. The Claude carve-out (`removal_root = ".claude/commands/ark"`, narrower than `dest_dir = ".claude"`) exists because `.claude/settings.json` carries Ark-managed JSON alongside user content. OpenCode has no analogous setup — `.opencode/` is wholly Ark-owned (commands + plugin). Wiping the directory wholesale is correct.
- **C-5:** `Manifest::record_block` dedupe contract is `(file, marker)`. Calling it twice with `(AGENTS.md, ARK)` (once from Codex, once from OpenCode) results in one entry. Verified at `crates/ark-core/src/state/manifest.rs:79–91`. No new code or guards needed.
- **C-6:** Body translation rules (`templates/claude/commands/ark/<name>.md` → `templates/opencode/commands/ark/<name>.md`): drop Claude's frontmatter (`description`, `argument-hint`), prepend OpenCode's frontmatter (`description` only, copied verbatim from Claude's value), keep body verbatim including the backtick-quoted heading `` # `/ark:<name> $ARGUMENTS` `` and inline `` `/ark:foo` `` references (verified shape: `templates/claude/commands/ark/{quick,design,archive}.md:6`). OpenCode's slash command surface IS the same shape as Claude's: filename → command name; `/<name>` invocation; `$ARGUMENTS` substitution (verified against opencode docs: "Pass arguments to commands using the `$ARGUMENTS` placeholder."). If a future opencode release renames the substitution token (`$ARGUMENTS` → `{args}` etc.), the rewrite is one-line in each command body and one rule update in this constraint; no SPEC delta.
- **C-7:** Plugin file is hand-authored TypeScript at `templates/opencode/plugins/ark-context.ts`. Size ≤80 lines including comments. Hooks: `chat.message` (read-only gate-and-store) + `experimental.chat.messages.transform` (mutate the user message). Module-local `Set<string>` for processed sessions; module-local `Map<string, string>` for pending context per session. Best-effort error handling: failure to spawn `ark` (`ENOENT`), non-zero exit, or `JSON.parse` throw → catch, call `await client.app.log({body: {service: "ark", level: "warn", message: <reason>}})`, return without injecting. **First swallowed failure per session** additionally writes one line to stderr: `ark-context: skipped context injection (see opencode logs)`. Subsequent failures in the same session are silent on stderr (still logged). Per TR-2.
- **C-8:** Plugin's exec target is the `ark` binary on `PATH`. Same trust model as Claude/Codex hooks (which invoke `ark context …` as a shell command). If `ark` is not on PATH, the plugin logs and skips (per C-7). Future work: a config flag for an absolute path. Out of scope here.
- **C-9:** Source-scan invariants: each of `platforms.rs` and `commands/{init,upgrade,unload,load,remove}.rs` includes a per-file scanner asserting no `".opencode/"` or `"opencode.json"` literal in non-test code. The scanners follow the established line-by-line shape used in those files (skip `#[cfg(test)]` body, skip `//` comments).
- **C-10:** Parity tests are tree-walked, not hand-listed. `every_claude_command_has_an_opencode_command_sibling` reads `CLAUDE_TEMPLATES.files()` and asserts the parallel path exists in `OPENCODE_TEMPLATES`. Adding a Claude command file without an OpenCode twin fails the test at `cargo test` time.
- **C-11:** Test invariant: `OPENCODE_PLATFORM.id == "opencode"`, `cli_flag == "opencode"`, `dest_dir == ".opencode"`, `removal_root == ".opencode"`, `managed_block_target == Some("AGENTS.md")`, `hook_file.is_none()`, `extra_files.len() == 1`, `extra_files[0].0 == ".opencode/plugins/ark-context.ts"`, `extra_files[0].1 == OPENCODE_ARK_CONTEXT_TS`. Asserted by `opencode_platform_shape` in `platforms.rs::tests`.
- **C-12:** Bun-side syntax check is **developer-runs-locally guidance**, not a CI test. The recommended command is `bun build --no-bundle templates/opencode/plugins/ark-context.ts > /dev/null` (exits non-zero on parse error). Documented in Phase 5 #21 + the plugin file's head comment. No `#[ignore]`d Rust test (per TR-3 — an ignored test is documentation pretending to be a check).
- **C-13:** No changes to `Snapshot` schema, `HookFileSpec`, `update_hook_file`, `remove_hook_file`, `read_hook_file`, `apply_managed_state` body, `capture_hook` body, `remove_hook` body, `remove_dir` body, or any command body. The only diffs are: registry growth, layout consts, two parity tests, and the OpenCode template tree.
- **C-14:** **Forward-compat for old binaries reading new artifacts.** An older `ark` binary (pre-OpenCode) reading a manifest written by a newer binary that recorded `.opencode/` paths in `manifest.files`: the older binary doesn't iterate OpenCode in `PLATFORMS` (it has 2 entries) and doesn't know about `.opencode/`. `unload` from the older binary captures `.opencode/` files only if `walk_files(owned_dirs)` happens to include it — but `owned_dirs` has 3 entries on the old binary, not 4. Result: `.opencode/` files are NOT captured by the old binary's `unload` and remain on disk. This is an **accepted degradation**: the old binary is missing a feature; calling `ark upgrade` to bring the binary up to the new version is the supported path. **Documented only** — no automated test (would require a multi-version test harness, out of scope).
- **C-15:** **Experimental-hook fragility.** `experimental.chat.messages.transform` is named `experimental` by opencode for a reason: signature/availability is not API-stable. Migration plan if the hook is renamed or stabilized: the plugin source is the only artifact that needs to change (rename the hook key in the export object). No SPEC delta. If the hook is **removed** with no replacement, the plugin reverts to G-9's `chat.message`-only behavior (gate-and-store but no injection), and the integration ships a follow-up task to identify the new mutation surface. Documented in the plugin's head comment.

---
