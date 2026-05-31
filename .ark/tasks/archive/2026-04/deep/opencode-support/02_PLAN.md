# `opencode-support` PLAN `02`

> Status: Approved for Implementation
> Feature: `opencode-support`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Iteration 02 closes the single HIGH finding (R-101) and two non-blocking notes (R-102, R-104) from `01_REVIEW.md`. R-103 (V-UT-14 second clause) is intentionally retained per the reviewer's own default ("Keep as is. Cheap regression guards are net-positive.") with the role-naming wording the reviewer requested. No structural changes; the `## Spec` is unchanged from 01 except for the three surgical patches:

1. **V-IT-2 (c) substring fixed (R-101).** The Claude templates use backtick-quoted headings — `` # `/ark:quick $ARGUMENTS` `` not `# /ark:quick $ARGUMENTS` (verified at `templates/claude/commands/ark/{quick,design,archive}.md:6`). 01's substring assertion would never match. 02 uses the actual shape verbatim, keeping C-6's "verbatim body" rule honest.

2. **Hook-firing-order assumption documented (R-102).** The plugin's correctness depends on `chat.message` firing before `experimental.chat.messages.transform` for the same user message. The Trellis reference relies on the same ordering implicitly. 02 adds an explicit ordering clause to G-9 with the same migration framing as C-15.

3. **Phase 5 #22 wording (R-104).** "Gating step before EXECUTE → VERIFY transition" rephrased to "Last EXECUTE step; must pass before `ark agent task verify` is invoked." Removes the ambiguous mid-phase-gate language.

R-103 wording softened in V-UT-14 to name its regression-guard role.

## Log

[**Added**]

- Hook-firing-order ordering clause added to G-9 (per R-102). Documents that `chat.message` must fire before `experimental.chat.messages.transform` for the same user message; if a future opencode release reverses this order, injection silently stops with the same migration path as C-15.

[**Changed**]

- V-IT-2 (c) substring updated from `# /ark:<name> $ARGUMENTS` to `` # `/ark:<name> $ARGUMENTS` `` (backtick-quoted), matching the verbatim shape in `templates/claude/commands/ark/{quick,design,archive}.md:6` (per R-101). G-12 (b) and C-6 updated in lockstep so the verbatim-body rule and the parity test agree.
- V-UT-14 wording: the second clause's role is now named explicitly as a regression guard ("vacuous against the current implementation by design; activates if a future commit adds a `package.json` to `extra_files` or roots `OPENCODE_TEMPLATES` higher") per R-103 Option (a).
- Phase 5 #22 trailing sentence: "**Gating step before EXECUTE → VERIFY transition.**" → "**Last EXECUTE step; must pass before `ark agent task verify` is invoked.**" (per R-104). Removes the ambiguous mid-phase-gate language; the workflow's two real gates (PLAN gate, VERIFY gate) are the only gates.

[**Removed**]

- 01's V-IT-2 (c) bare-substring assertion (replaced with the backtick-quoted version).

[**Unresolved**]

- None new. The Phase 5 #22 manual smoke remains the only end-to-end check of the two-hook contract; this is unchanged from 01 and accepted as the cost of the experimental-hook contract (C-15).

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review (01) | R-101 | Accepted (Option a) | V-IT-2 (c) substring updated to `` # `/ark:<name> $ARGUMENTS` `` (backtick-quoted) verbatim from the Claude templates. G-12 (b) and C-6 wording aligned. The verbatim-body rule remains the source of truth. |
| Review (01) | R-102 | Accepted | G-9 gains an explicit "Ordering assumption" clause naming the `chat.message` → `experimental.chat.messages.transform` firing order. Migration framing matches C-15: rework plugin source, no SPEC delta. |
| Review (01) | R-103 | Accepted (Option a) | V-UT-14 wording softened: the on-disk clause is named as a regression guard. No deletion (the reviewer's default was Keep). |
| Review (01) | R-104 | Accepted | Phase 5 #22 wording rephrased per the reviewer's exact recommendation: "Last EXECUTE step; must pass before `ark agent task verify` is invoked." |
| Trade-off (01) | TR-1..TR-6 | Accepted as resolved in 01 | No new positions. The reviewer's TR-6 concession (TS is acceptable) is recorded; no plan change needed. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here. ✓ R-101 listed.
> - Every Master directive must appear here. ✓ None.
> - Rejections must include explicit reasoning. ✓ None rejected this iteration.

---

## Spec `Core specification`

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
  - **Pure helpers** (named for clarity, NOT exported): `buildEnvelopePrefix(additionalContext: string): string` returns `<ark-context>\n${additionalContext}\n</ark-context>\n\n---\n\n`. `shouldInject(sessionID: string, processed: Set<string>): boolean` returns `!processed.has(sessionID)`. These are plain `function` declarations, NOT `export function`. **(Corrected post-archive: the original wording required `export function`; OpenCode's plugin runtime invokes every named export at load time with no arguments, which crashes any helper that takes parameters. Verified empirically: `error=undefined is not an object (evaluating 'processed.has')`. The `opencode_plugin_keeps_helpers_internal` test in `crates/ark-core/src/templates.rs::tests` codifies the invariant.)**
  - **Failure handling:** Best-effort. On `execFileSync` throw (e.g. `ENOENT` for missing `ark` binary), `JSON.parse` throw, or non-zero exit code: the plugin catches, calls `client.app.log({ body: { service: "ark", level: "warn", message: <reason> } })` and returns without modifying state. On the first swallowed failure per session (per TR-2), the plugin additionally writes a single-line stderr note: `ark-context: skipped context injection (see opencode logs)`. Subsequent failures in the same session are silent in stderr (still logged).
  - **Per-session dedupe:** module-local `Set<string>` and `Map<string, string>`. JS event loop is single-threaded; no concurrency primitive needed.
  - **Size:** ≤80 lines including license/header comment.

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

CLI surface (`ark-cli/src/main.rs`): bare `#[arg(long)]` for both flags. Mirrors existing Claude / Codex flag pairs. No `conflicts_with`, no `ArgGroup`. Resolution is in `resolve_platforms_pure` — when both `--<flag>` and `--no-<flag>` are passed for the same platform, the negative wins on per-platform conflict via the `f.on && !f.off` filter. **(Corrected post-archive: original wording said "positive-wins"; verified against `crates/ark-cli/src/main.rs:132–161` — the actual rule is negative-wins on per-platform conflict. G-3 above already documents this correctly.)**

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

## Runtime `runtime logic`

[**Main Flow** — `init --opencode`]

1. CLI parses `InitArgs`; `resolve_platforms_pure` returns the selected slice. Positive flags win over negative flags (existing semantic).
2. For each selected platform: extract `templates` under `dest_dir`, call `apply_managed_state(&layout, &mut manifest)`. For OpenCode, this:
   a. Writes `AGENTS.md` `ARK` block via `update_managed_block`. If Codex already wrote it, `update_managed_block` returns `false` (idempotent) and `record_block` no-ops.
   b. Skips the `hook_file` arm (`None`).
   c. Writes the TS plugin file via `extra_files` iteration: `layout.resolve(".opencode/plugins/ark-context.ts").write_bytes(OPENCODE_ARK_CONTEXT_TS.as_bytes())`.
3. Manifest persisted; `manifest.files` includes `.opencode/commands/ark/{quick,design,archive}.md` (from `templates`) and `.opencode/plugins/ark-context.ts` (from `extra_files`); `manifest.managed_blocks` contains one `(AGENTS.md, ARK)` entry regardless of how many platforms wrote it.

[**Main Flow** — runtime (user side, opencode session)]

1. User runs `opencode` in a project initialized with `--opencode`.
2. OpenCode's runtime auto-discovers `.opencode/plugins/ark-context.ts` and loads it via Bun.
3. Plugin's default-export factory runs once per process, returning a handler object with two keys: `chat.message` and `experimental.chat.messages.transform`.
4. On the user's first chat message in a new session, `chat.message` fires. Plugin checks the module-local `processedSessions: Set<string>`; first time → `execFileSync("ark", ["context", "--scope", "session", "--format", "json"], {cwd: directory})`. Parse the SessionStart envelope: `{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: <stringified projection>}}`. Extract `additionalContext`. Store in `pendingContext: Map<string, string>` keyed by `sessionID`. Mark the session as processed.
5. Immediately after, opencode fires `experimental.chat.messages.transform` with the message about to be sent to the LLM. Plugin finds the last user message (`info.role === "user"`), looks up `info.sessionID` in `pendingContext`. If a value is present, prepend `<ark-context>\n${additionalContext}\n</ark-context>\n\n---\n\n` to the first text part's `text` field, then `pendingContext.delete(sessionID)` (consume-on-use).
6. Subsequent messages in the same session: `chat.message` hits the dedupe guard and no-ops; `experimental.chat.messages.transform` looks up `pendingContext.get(sessionID)`, finds nothing, no-ops.

[**Failure Flow**]

1. **`ark` binary not on PATH** → `execFileSync` throws `ENOENT` → plugin catches in `chat.message` handler, logs via `client.app.log` with the error message, writes one-shot stderr note, returns without populating `pendingContext`. `experimental.chat.messages.transform` then looks up nothing and no-ops. Session continues normally.
2. **`ark context` exits non-zero** (e.g. not in an Ark-loaded project) → `execFileSync` throws (with `stderr` on the error object) → plugin catches in `chat.message`, logs `stderr` at `warn`, one-shot stderr note, returns. Session continues normally.
3. **JSON parse failure** (binary output corrupted) → `JSON.parse` throws → caught in `chat.message`, logged, returns. Session continues normally.
4. **Concurrent first messages** (race on `processedSessions`) → JS event loop is single-threaded; impossible.
5. **`init --opencode` on a non-TTY with no `--opencode` / `--no-opencode` flag** → existing non-TTY error path fires with the extended message (per G-3).
6. **`init --opencode --no-opencode`** → opencode excluded per the existing `f.on && !f.off` filter (negative wins on per-platform conflict, mirrors `--claude --no-claude` and `--codex --no-codex` semantics). No parse error.
7. **Codex already wrote `AGENTS.md` block, then OpenCode init** → `update_managed_block` returns `false` (no diff); `record_block` dedupes; idempotent.
8. **Opencode renames `experimental.chat.messages.transform`** (future risk per C-15) → plugin's hook handler is registered under the old key, opencode does not call it, injection silently stops. `chat.message` still runs (gate-and-store works). Mitigation: monitor opencode releases; rename in plugin source when the hook stabilizes/renames; ship via `ark upgrade`.

[**State Transitions**]

- Project state: `(claude, codex, opencode)` triple. Any of {0,0,0} (uninstalled), {1,0,0}, {0,1,0}, {0,0,1}, {1,1,0}, {1,0,1}, {0,1,1}, {1,1,1}. `ark init --<flag>` is additive (transitions 0→1 for the named flag, idempotent on already-installed flags). `ark remove` is wholesale (transitions all to 0). `ark upgrade` re-applies only for platforms in state 1.
- Manifest state: `manifest.managed_blocks` contains 0 or 1 entries for `(AGENTS.md, ARK)` regardless of whether 0/1/2 of {Codex, OpenCode} are installed. `manifest.files` contains paths under `.opencode/` iff state {*,*,1}.
- Plugin runtime state: per-process, two module-local collections: `processedSessions: Set<string>` (sessions where context was fetched), `pendingContext: Map<string, string>` (session → prefix awaiting transform). Reset when the opencode process exits.

---

## Implementation `split task into phases`

[**Phase 1 — Layout + templates scaffolding**]

1. Add `OPENCODE_DIR`, `OPENCODE_PLUGIN_FILE` consts in `crates/ark-core/src/layout.rs`.
2. Add `Layout::opencode_dir()`, `Layout::opencode_plugin_file()` getters.
3. Update `Layout::owned_dirs()` from `[PathBuf; 3]` to `[PathBuf; 4]`. Verify all callers (grep for `.owned_dirs()` across the workspace) accept the new length without code change (they iterate; array length is structural). Patch any caller that hard-codes length 3 (none expected per source review).
4. Create `templates/opencode/` directory tree:
   - `templates/opencode/commands/ark/quick.md` (translated from `templates/claude/commands/ark/quick.md` per C-6)
   - `templates/opencode/commands/ark/design.md` (translated)
   - `templates/opencode/commands/ark/archive.md` (translated)
   - `templates/opencode/plugins/ark-context.ts` (hand-authored ≤80 lines, per G-9 contract)
5. Add `OPENCODE_TEMPLATES` static (rooted at `templates/opencode/commands`) and `OPENCODE_ARK_CONTEXT_TS` const in `crates/ark-core/src/templates.rs`.
6. Update existing source-scan exclusion lists in `platforms.rs` and each `commands/*.rs` to also forbid `.opencode/` and `opencode.json` literals (per C-9).

**Validates:** `cargo build` green. Unit tests for `Layout::opencode_dir` and `Layout::opencode_plugin_file` pass. Source-scan tests pass on the existing files (no `.opencode/` literals leaking).

[**Phase 2 — Platform registry entry**]

7. Add `OPENCODE_PLATFORM` const in `crates/ark-core/src/platforms.rs`.
8. Extend `PLATFORMS` slice from 2 entries to 3.
9. Update existing test `platforms_registry_has_two_entries_in_canonical_order` → rename to `platforms_registry_has_three_entries_in_canonical_order`, assert 3 entries with ids `["claude-code", "codex", "opencode"]`.
10. Update `Platform::by_id` and `Platform::by_cli_flag` tests to cover `"opencode"` lookups.
11. Add new test `opencode_platform_shape` (per C-11).
12. Add new test `opencode_apply_managed_state_writes_block_and_plugin` (mirrors `codex_apply_managed_state_writes_block_hook_and_extras`): apply OpenCode then verify (a) AGENTS.md `ARK` block present, (b) `.opencode/plugins/ark-context.ts` written byte-for-byte equal to `OPENCODE_ARK_CONTEXT_TS`, (c) `manifest.managed_blocks` contains one `(AGENTS.md, ARK)` entry. Then call `apply_managed_state` for `CODEX_PLATFORM` on the same manifest and assert `manifest.managed_blocks.len() == 1` (dedupe).
13. Re-export `OPENCODE_PLATFORM` from `crates/ark-core/src/lib.rs`.

**Validates:** `cargo test -p ark-core` green. Manifest dedupe verified. Five existing command bodies (`init`/`upgrade`/`unload`/`load`/`remove`) unchanged but exercised against 3 platforms.

[**Phase 3 — CLI flags + interactive prompt**]

14. Add `--opencode` / `--no-opencode` flags to `InitArgs` in `crates/ark-cli/src/main.rs`. Bare `#[arg(long)]`, no `conflicts_with`.
15. Extend `InitArgs::flags` `match p.cli_flag` arm to include `"opencode" => PlatformFlag { on: self.opencode, off: self.no_opencode }`. Extend the non-TTY error message at `main.rs:157–160` to name `--opencode` / `--no-opencode`.
16. Update CLI tests:
    - `init_explicit_opencode_only_installs_opencode` (mirror existing equivalent for codex if present)
    - `init_no_opencode_skips_opencode`
    - `init_non_tty_no_flags_errors_with_three_platform_message`
    - `resolve_platforms_pure_offers_all_three_when_no_flags_and_tty` — exercises the closure-injected branch at `main.rs:154–156`, asserting the closure is called once when both `any_positive` and `any_negative` are false and `is_tty == true`. Targets the existing testable seam, not the unrefactored `interactive_select_platforms` (which reads stdin directly).
    - `init_opencode_with_no_opencode_excludes_opencode` — asserts negative-wins-on-conflict behavior matches `--claude --no-claude`. (Test renamed from the original `..._resolves_to_opencode_on` post-archive; the original name was based on the incorrect "positive-wins" claim.)

**Validates:** `cargo test --workspace` green. CLI smoke: `cargo run -- init --opencode` on `tempfile::tempdir()` produces the expected tree.

[**Phase 4 — Parity tests + integration**]

17. Add `every_claude_command_has_an_opencode_command_sibling` test in `crates/ark-core/src/templates.rs::tests` (mirrors `every_claude_command_has_a_codex_skill_sibling`).
18. Add `opencode_command_bodies_have_opencode_frontmatter` test (per G-12 (b)): asserts (a) frontmatter starts with `---\ndescription:`, (b) no `argument-hint:` line in frontmatter, (c) body contains the literal backtick-quoted heading `` # `/ark:<name> $ARGUMENTS `` (e.g. for `quick.md` the body must contain the literal substring `` # `/ark:quick $ARGUMENTS` `` including surrounding backticks — see V-IT-2 for the exact assertion shape).
19. Add round-trip integration test (preferred home: `crates/ark-core/src/commands/load.rs::tests`):
    - `init --claude --codex --opencode` → assert all three trees + one shared AGENTS.md block in manifest.
    - `unload` → assert snapshot has the union of files; AGENTS.md block removed once.
    - `load` → assert all three trees restored, AGENTS.md re-applied, hooks re-applied per `PLATFORMS` iteration.
    - Byte-identical round-trip (modulo timestamps).
20. Add upgrade test: `init --claude` + `upgrade` → no `.opencode/` created (G-14). `init --opencode` afterward → `.opencode/` appears, idempotent on Claude artifacts.

**Validates:** `cargo test --workspace` fully green. `cargo build --release` produces a binary that smoke-tests cleanly via `cargo run --release -- init --opencode --codex`.

[**Phase 5 — Manual verification + docs**]

21. Run the new `ark` against a fresh tempdir on a developer machine: `ark init --opencode`, inspect `.opencode/`, run `bun --version` then `bun build --no-bundle .opencode/plugins/ark-context.ts > /dev/null` (validates the TS file parses; non-zero exit on syntax error).
22. (Manual, not in CI) Boot an actual `opencode` session in a fresh tempdir initialized with `ark init --opencode`. Send a first user message. Verify in opencode logs (`client.app.log` output) that `chat.message` ran. Verify the `<ark-context>...</ark-context>` prefix appears in the model's input via opencode's debug surface (or by adding a one-line `console.error` in the plugin during dev). Verify `$ARGUMENTS` substitution by invoking `/ark:quick test arg` and confirming the body renders `` # `/ark:quick test arg` `` (with surrounding backticks). **Last EXECUTE step; must pass before `ark agent task verify` is invoked.**
23. Update `.ark/workflow.md` if any phrasing needs to mention OpenCode (e.g. the §3 tier table mentions Codex skills; add an OpenCode commands column). Likely a single-row addition.
24. No README updates needed beyond what `/ark:archive` will pick up automatically.

---

## Trade-offs `ask reviewer for advice`

- **T-1: Plugin file location — `extra_files` vs. embedded in `OPENCODE_TEMPLATES`.**
  - **Option A (chosen, retained from 00):** Plugin file shipped via `extra_files: &[(OPENCODE_PLUGIN_FILE, OPENCODE_ARK_CONTEXT_TS)]`. `OPENCODE_TEMPLATES` rooted at `templates/opencode/commands/`. Symmetric with Codex's `config.toml`.
    - **Adv.** Re-applied unconditionally on every `init` / `load` / `upgrade`. Plugin file is NOT hash-tracked — user edits to `.opencode/plugins/ark-context.ts` are reverted on next `upgrade`, matching the Ark-owned intent.
    - **Disadv.** One extra `include_str!` macro and one extra `extra_files` entry vs. the alternative.
  - **Option B:** `OPENCODE_TEMPLATES` rooted at `templates/opencode/` whole; plugin extracted via the regular template pipeline. `extra_files = &[]`.
    - **Adv.** One fewer macro invocation.
    - **Disadv.** Plugin gets hash-tracked. On `upgrade`, hash mismatches would prompt about user-customized plugin even though the plugin is Ark-owned. Diverges from Codex's `config.toml` precedent.
  - **Recommendation:** A. Confirmed by reviewer (TR-1).

- **T-2: Plugin error handling — log-and-continue vs. fail-loud.**
  - **Option A (chosen, retained from 00 with TR-2 enhancement):** Log via `client.app.log` and continue silently, but on the **first** swallowed failure per session, additionally write one-line stderr note: `ark-context: skipped context injection (see opencode logs)`. Subsequent failures in the same session are silent on stderr (still logged).
    - **Adv.** Robust to transient failures (PATH issues, project-not-loaded, race conditions). Discoverable: user sees stderr note once and can grep `opencode logs` for details.
    - **Disadv.** None significant.
  - **Option B:** Throw on any error.
    - **Adv.** Loud failures get fixed faster.
    - **Disadv.** Most failure modes are user-environment issues; breaking opencode for them is hostile.
  - **Recommendation:** A.

- **T-4: Body translation — keep slash idioms verbatim or rewrite.**
  - **Option A (chosen, retained from 00):** Keep Claude's body verbatim, only swap frontmatter. `$ARGUMENTS` is opencode's command argument substitution token (verified per opencode docs: "Pass arguments to commands using the `$ARGUMENTS` placeholder.").
    - **Adv.** Minimal divergence. Parity test stays simple. Verified.
    - **Disadv.** Negligible.
  - **Option B:** Translate slash idioms if opencode's syntax diverges.
    - **Adv.** Defense against future-renamed substitution token.
    - **Disadv.** Speculative; not needed today.
  - **Recommendation:** A. Phase 5 #22 includes a manual `$ARGUMENTS` substitution check as the gating step before VERIFY; if it fails, C-6's rewrite path is one-line per command body.

- **T-5: AGENTS.md sharing — single block vs. per-platform markers.**
  - **Option A (chosen, retained from 00):** Both platforms write the same `<!-- ARK:START -->...<!-- ARK:END -->` block. Manifest dedupes.
    - **Adv.** Minimal: one block, identical content, one round-trip path. No new manifest fields.
    - **Disadv.** Per-platform divergence in `AGENTS.md` would require future task; clear migration path exists.
  - **Option B:** Per-platform markers (`ARK:CODEX`, `ARK:OPENCODE`).
    - **Adv.** Future-proof.
    - **Disadv.** Two visually-identical blocks. Manifest grows by one entry per platform. New `MANAGED_BLOCK_BODY_OPENCODE` const.
  - **Recommendation:** A. Confirmed by reviewer (TR-5).

- **T-6 (NEW): Plugin source — TypeScript vs. JavaScript.**
  - **Option A (chosen, per user direction in DESIGN):** TypeScript at `templates/opencode/plugins/ark-context.ts`. Bun runs it natively.
    - **Adv.** Types document the SessionStart envelope shape (`{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: string}}`) and the opencode hook input/output shapes (via `import type { Plugin } from "@opencode-ai/plugin"` if we ever need richer types — currently we don't, so no npm dep). Bun runs `.ts` natively per opencode docs; no transpile in user runtime path.
    - **Disadv.** TS-aware maintainers required; one slightly heavier syntax than JS.
  - **Option B (rejected):** JavaScript at `.opencode/plugins/ark-context.js`. Trellis reference uses JS.
    - **Adv.** No TS knowledge required for maintenance.
    - **Disadv.** Loses type-as-documentation value. Trellis precedent is external; within Ark, codex-support's `templates/codex/skills/` ships only `SKILL.md` files (no JS), so there is no internal "JS convention" to match. Reviewer's TR-6 rationale based on a misread of the codex artifact tree.
  - **Recommendation:** A, per user direction. The argument for TS is documentation-grade, not type-safety-grade — types are inline assertions about the opencode hook contract that read better than JSDoc comments would. If a maintainer wants JS-only, the migration is mechanical (rename `.ts` → `.js`, drop type annotations); preserved as a reversible call.

---

## Validation `test design`

[**Unit Tests**]

- **V-UT-1:** `OPENCODE_PLATFORM` shape per C-11. Maps G-2, G-7, G-8, G-9 (data-shape only, not runtime), C-3, C-11.
- **V-UT-2:** `PLATFORMS.len() == 3` and ids in canonical order `["claude-code", "codex", "opencode"]`. Maps G-1.
- **V-UT-3:** `Platform::by_id("opencode")` and `Platform::by_cli_flag("opencode")` resolve correctly. Maps G-1.
- **V-UT-4:** `OPENCODE_PLATFORM.apply_managed_state(&layout, &mut manifest)` writes `.opencode/plugins/ark-context.ts` byte-for-byte equal to `OPENCODE_ARK_CONTEXT_TS`, writes the AGENTS.md `ARK` block, records the block once. Maps G-5, G-9 (data-shape), C-2.
- **V-UT-5:** Calling `apply_managed_state` for `CODEX_PLATFORM` then `OPENCODE_PLATFORM` (or vice versa) results in `manifest.managed_blocks.len() == 1` (deduped on `(AGENTS.md, ARK)`) and one on-disk write of the `<!-- ARK:START -->...` block. Maps G-7, C-5.
- **V-UT-6:** `OPENCODE_PLATFORM.capture_hook(&layout, &mut snapshot)` returns `Ok(None)` and leaves `snapshot.hook_bodies` untouched. Maps G-8, C-3.
- **V-UT-7:** `OPENCODE_PLATFORM.remove_hook(&layout)` returns `Ok(false)` (no hook file). Maps G-8.
- **V-UT-8:** `OPENCODE_PLATFORM.remove_dir(&layout)` returns `false` on non-existent dir, `true` after `apply_managed_state` populated `.opencode/`. Maps G-10.
- **V-UT-9:** `Layout::opencode_dir()` and `Layout::opencode_plugin_file()` return paths joined to `root` via `resolve`. Maps G-13.
- **V-UT-10:** `Layout::owned_dirs()` returns a 4-entry `[PathBuf; 4]` rooted at `layout.root` containing `[ark_dir(), claude_commands_ark_dir(), codex_dir(), opencode_dir()]` (verified by path equality, not literal-string equality). Maps G-13.
- **V-UT-11:** Source-scan invariant: no `".opencode/"` literal in `platforms.rs` (sanctioned sites are `layout.rs` and `templates.rs`; references go through `OPENCODE_DIR` / `OPENCODE_PLUGIN_FILE`). Maps C-9.
- **V-UT-12:** Source-scan invariant extension: each of `commands/{init,upgrade,unload,load,remove}.rs` has no `".opencode/"` or `"opencode.json"` literal. Maps C-9.
- **V-UT-13 (NEW per R-010):** `OPENCODE_TEMPLATES.get_file("plugins/ark-context.ts").is_none()` and `OPENCODE_TEMPLATES.get_file("plugins/").is_none()` (the templates root is `templates/opencode/commands`, not `templates/opencode/`). The plugin file is reachable only via `OPENCODE_ARK_CONTEXT_TS`. Maps C-2.
- **V-UT-14 (NEW per R-010 in 01, role-clarified per R-103 in 02):** No `package.json` is reachable via `OPENCODE_TEMPLATES` (`OPENCODE_TEMPLATES.get_file("package.json").is_none()` and `OPENCODE_TEMPLATES.get_file("ark/package.json").is_none()`). After `apply_managed_state`, `.opencode/package.json` does not exist on disk. The on-disk assertion is a **regression guard for future template-tree changes** (vacuous against the current implementation by design; activates if a future commit adds a `package.json` to `extra_files` or roots `OPENCODE_TEMPLATES` higher). Maps G-15.

[**Integration Tests**]

- **V-IT-1:** Parity: `every_claude_command_has_an_opencode_command_sibling`. Walk `CLAUDE_TEMPLATES.files()`; for each `.md` file at `commands/ark/<name>.md`, assert `OPENCODE_TEMPLATES.get_file("ark/<name>.md").is_some()`. Maps G-12 (a), C-10.
- **V-IT-2 (REVISED per R-006 in 01, R-101 in 02):** Parity + sanity: `opencode_command_bodies_have_opencode_frontmatter_and_arguments_token`. Walk `OPENCODE_TEMPLATES.files()`; for each `.md`: (a) body starts with `---\n` and the first non-`---` line begins with `description:`; (b) frontmatter block (between `---\n` and the closing `---\n`) does NOT contain a line starting with `argument-hint:`; (c) the body contains the literal heading `` # `/ark:<name> $ARGUMENTS` `` (backtick-quoted) where `<name>` is the file's stem (e.g. for `quick.md` the body must contain the literal substring `` # `/ark:quick $ARGUMENTS` `` including surrounding backticks — matching `templates/claude/commands/ark/quick.md:6` verbatim). Maps G-12 (b).
- **V-IT-3:** End-to-end round-trip: `init --claude --codex --opencode` on a `tempdir` → `unload` → `load` produces byte-identical disk state (modulo timestamps in `.ark.db`). Maps G-1, G-5, G-7, G-9 (artifact path), G-10, G-11.
- **V-IT-4:** `init --opencode` alone (no Claude, no Codex) → only `.opencode/`, `.ark/`, and `AGENTS.md` exist (no `.claude/`, no `CLAUDE.md`, no `.codex/`). Maps G-3, G-7.
- **V-IT-5:** `init --claude` then `init --opencode` (additive, two separate calls) → both `.claude/` and `.opencode/` exist, `CLAUDE.md` and `AGENTS.md` both have ARK blocks, manifest has 2 managed-block entries (one per file). Idempotent: re-running `init --opencode` is a no-op. Maps G-14.
- **V-IT-6:** Upgrade preservation: `init --claude`, then upgrade (newer binary), then `cargo run -- upgrade` → no `.opencode/` directory created. Adding `init --opencode` after upgrade works. Maps G-14.
- **V-IT-7:** Remove with shared block: `init --codex --opencode` → `remove` → `AGENTS.md` exists but the `ARK` block is gone (other content preserved); `.codex/` and `.opencode/` are gone. Maps G-10, G-7.
- **V-IT-8 (REVISED per R-002 in 01, corrected in EXECUTE per workflow §4 fidelity rule):** CLI flag handling: `init --opencode --no-opencode` excludes opencode (per the existing `f.on && !f.off` filter — when both flags are set, the platform is filtered out). Matches existing `--claude --no-claude` and `--codex --no-codex` semantics verbatim. With another positive flag also set (e.g. `--claude --opencode --no-opencode`), only the unconflicted platform survives. Maps G-3.

[**Failure / Robustness Validation**]

- **V-F-1:** `init` on non-TTY with no platform flags errors with a message naming `--claude` / `--no-claude`, `--codex` / `--no-codex`, `--opencode` / `--no-opencode`. Maps G-3.
- **V-F-2:** `init --opencode` when `templates/opencode/commands/ark/` is missing a file (compile-time concern; the parity test V-IT-1 catches it). Maps G-12 (negative case).
- **V-F-3:** Plugin runtime failure modes (documented in C-7, not unit-tested in Rust): `ark` not on PATH, `ark context` exit nonzero, JSON parse error → plugin logs and returns; first per-session failure also writes one-line stderr note. Validated by reading the plugin source at code-review time and by Phase 5 #22 manual smoke. Maps C-7.

[**Edge Case Validation**]

- **V-E-1:** Re-applying `apply_managed_state` for OpenCode multiple times on the same project: idempotent. Plugin file is byte-identical, AGENTS.md block is unchanged, `record_block` no-ops, second call's `update_managed_block` returns `false`. Maps G-5, G-9.
- **V-E-2:** `ark unload` on a project with `.opencode/` but a corrupt manifest (no `OPENCODE_DIR` files in `manifest.files`): `walk_files(owned_dirs)` still picks up `.opencode/` via the directory walk (independent of manifest), captured into `snapshot.files`. `load` restores them. Tests this behavior is unchanged from the existing robustness model.
- **V-E-3:** Empty `.opencode/` directory (synthetic case): `unload` captures nothing under `.opencode/`; `remove_dir` returns `false`. Path through the code is exercised.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1  | V-UT-2, V-UT-3, V-IT-3 |
| G-2  | V-UT-1 |
| G-3  | V-IT-4, V-IT-8, V-F-1 |
| G-4  | V-IT-3, V-IT-1, V-IT-2 |
| G-5  | V-UT-4, V-E-1 |
| G-6  | V-UT-9, V-UT-10 |
| G-7  | V-UT-5, V-IT-3, V-IT-4, V-IT-7 |
| G-8  | V-UT-6, V-UT-7 |
| G-9  | V-UT-1, V-UT-4 (artifact path / extra_files shape); runtime contract is documented + manual (V-F-3, Phase 5 #22) — no Rust-side unit test crosses the Bun boundary by design |
| G-10 | V-UT-8, V-IT-7 |
| G-11 | V-IT-3 |
| G-12 | V-IT-1, V-IT-2, V-F-2 |
| G-13 | V-UT-9, V-UT-10 |
| G-14 | V-IT-5, V-IT-6 |
| G-15 | V-UT-14 (no package.json shipped); runtime contract via Phase 5 #21–#22 |
| C-1  | V-UT-11, V-UT-12 |
| C-2  | V-UT-4, V-UT-13 |
| C-3  | V-UT-1, V-UT-6 |
| C-4  | V-UT-1 |
| C-5  | V-UT-5 |
| C-6  | V-IT-2 (frontmatter shape + `$ARGUMENTS` retention); body verbatim is policed by code review |
| C-7  | V-F-3 (documented; Phase 5 #22 manual) |
| C-8  | (documented; trust model per Claude/Codex precedent) |
| C-9  | V-UT-11, V-UT-12 |
| C-10 | V-IT-1, V-IT-2 |
| C-11 | V-UT-1 |
| C-12 | (developer-runs-locally guidance; Phase 5 #21 — no automated test by design per TR-3) |
| C-13 | (documented; non-modification of `Snapshot` etc. is enforced by code review at PR time — `cargo test --workspace` continuing to pass is necessary but not sufficient) |
| C-14 | (documented-only; multi-version test harness out of scope — no test) |
| C-15 | (documented in plugin head comment; migration plan; no test) |
