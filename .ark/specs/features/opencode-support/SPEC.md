[**Goals**]

- G-1: `PLATFORMS` registry grows to 3: `claude-code`, `codex`, `opencode`. Adding OpenCode is a registry entry plus a template tree.
- G-2: OpenCode artifacts ship at `.opencode/commands/ark/{quick,design,archive}.md` and `.opencode/plugins/ark-context.ts`.
- G-3: `ark init` accepts `--opencode` / `--no-opencode`; OpenCode joins the TTY prompt and the non-TTY error message.
- G-4: OpenCode uses the existing `AGENTS.md` managed block (shared with Codex; manifest dedupes on `(file, marker)`).
- G-5: A Bun-loaded TS plugin shells out to `ark context --scope session --format json` and prepends the envelope to the first user message per session.

[**Non-goals**]

- NG-1: No native session-start hook; OpenCode has no JSON-shaped hook surface.
- NG-2: No npm dependencies; plugin uses Bun built-ins only (`node:child_process`, `node:util`).
- NG-3: No fourth platform in this task; registry leaves room.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                     (InitArgs gains 2 OpenCode flags;
│                                              interactive prompt + non-TTY error
│                                              extend to 3 platforms)
└── ark-core/src/
    ├── lib.rs                               (re-exports OPENCODE_PLATFORM)
    ├── platforms.rs                         (adds OPENCODE_PLATFORM; PLATFORMS grows
    │                                          to 3 entries; registry-shape test updated)
    ├── layout.rs                            (OPENCODE_DIR, OPENCODE_COMMANDS_DIR,
    │                                          OPENCODE_PLUGIN_FILE consts;
    │                                          opencode_dir(), opencode_plugin_file();
    │                                          owned_dirs grows to [PathBuf; 4])
    ├── io/fs.rs                             (UNCHANGED — no new hook plumbing)
    ├── state/                                (UNCHANGED)
    ├── commands/                             (UNCHANGED bodies; iteration over PLATFORMS
    │                                          picks up the new entry for free)
    └── templates.rs                          (adds OPENCODE_TEMPLATES static +
                                                OPENCODE_ARK_CONTEXT_TS const)
templates/
└── opencode/                                 (NEW)
    ├── commands/ark/{quick,design,archive}.md
    └── plugins/ark-context.ts                (Bun-loaded; ≤95 lines)
```

Module coupling: adding OpenCode introduces no new module dependencies. One const added to `platforms.rs`, one static + one const to `templates.rs`, three consts + two getters to `layout.rs`. The plugin is a leaf shipped artifact, not Rust code.

Call graph for `init`: identical to existing — the `for platform in selected_platforms` loop calls `platform.apply_managed_state(&layout, &mut manifest)?` for each selected entry.

Call graph for `unload`: unchanged. The two-stage hook-capture iterates registered `PLATFORMS` entries (OpenCode contributes nothing — `hook_file = None`) then scans `*.json` under `owned_dirs` (no `.json` files in `.opencode/`).

Call graph for `load`: unchanged. Files under `.opencode/` are restored from `snapshot.files` (including the plugin TS file). The `for platform in PLATFORMS` canonical re-apply iterates 3 entries.

[**Data Structure**]

```rust
// ark-core/src/platforms.rs (additions)

/// OpenCode integration. Templates extract under `.opencode/`; managed block
/// shares `AGENTS.md` with Codex (manifest dedupes on `(file, marker)`).
/// SessionStart-equivalent context injection rides a Bun-loaded TS plugin
/// shipped via `extra_files`; OpenCode has no native JSON hook surface.
pub const OPENCODE_PLATFORM: Platform = Platform {
    id: "opencode",
    templates: &templates::OPENCODE_TEMPLATES,
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

// ark-core/src/layout.rs (additions)

/// Root directory for OpenCode integration (relative to project root).
pub const OPENCODE_DIR:           &str = ".opencode";
/// Where OpenCode slash-command markdown files are extracted.
/// `OPENCODE_TEMPLATES` is rooted parallel to this so `Platform::templates`
/// extracts under `dest_dir = OPENCODE_COMMANDS_DIR` without an extra path.
pub const OPENCODE_COMMANDS_DIR:  &str = ".opencode/commands";
/// Bun-loaded plugin that shells out to `ark context --scope session --format json`
/// and prepends `additionalContext` to the first user message per session.
pub const OPENCODE_PLUGIN_FILE:   &str = ".opencode/plugins/ark-context.ts";

impl Layout {
    pub fn opencode_dir(&self)         -> PathBuf;   // <project>/.opencode/
    pub fn opencode_plugin_file(&self) -> PathBuf;   // <project>/.opencode/plugins/ark-context.ts

    /// Directories whose full contents are captured by `unload` and restored
    /// by `load`. `walk_files` on a missing dir yields empty.
    pub fn owned_dirs(&self) -> [PathBuf; 4] {
        [self.ark_dir(),
         self.claude_commands_ark_dir(),
         self.codex_dir(),
         self.opencode_dir()]
    }
}

// ark-core/src/templates.rs (additions)
pub static OPENCODE_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/opencode/commands");

pub const OPENCODE_ARK_CONTEXT_TS: &str =
    include_str!("../../../templates/opencode/plugins/ark-context.ts");
```

Plugin contract (`templates/opencode/plugins/ark-context.ts`):

- **Source language:** TypeScript, Bun-loaded natively (no transpile step).
- **Imports:** Node/Bun built-ins only — `node:child_process` (`execFileSync`) or `node:util` (`promisify(execFile)`). No `package.json`, no npm deps.
- **Export shape:**
  ```ts
  export default async ({ directory, client }) => ({
      "chat.message": <handler>,
      "experimental.chat.messages.transform": <handler>,
  });
  ```
- **`chat.message` handler:** read-only notification. Receives `(input)` where `input.sessionID` identifies the session. On first hit per session, runs `ark context --scope session --format json` (cwd = `directory`), parses the SessionStart envelope, stores the unwrapped `additionalContext` string in a module-local `Map<sessionID, string>`, marks the session processed.
- **`experimental.chat.messages.transform` handler:** receives `(input, output)`. Finds the last user message, looks up its `sessionID` in the pending map; if a value is present and the message has not yet been transformed, prepends `<ark-context>\n${additionalContext}\n</ark-context>\n\n---\n\n` to the first text part's `text` field, then deletes the map entry (consume-on-use).
- **Pure helpers** (named for clarity, NOT exported — OpenCode invokes every named export at load time with no arguments):
  - `function buildEnvelopePrefix(additionalContext: string): string`
  - `function shouldInject(sessionID: string, processed: Set<string>): boolean`
- **Failure handling:** `execFileSync` runs with `timeout: 5000` (5 s). On `ENOENT`, `ETIMEDOUT`, non-zero exit, `JSON.parse` throw, or unexpected payload shape, the plugin catches, calls `client.app.log({ body: { service: "ark", level: "warn", message: <reason> } })`, and returns without modifying state. First swallowed failure per session writes one line to stderr: `ark-context: skipped context injection (see opencode logs)`. Subsequent failures in the same session are silent on stderr (still logged).
- **Per-session dedupe:** module-local `Set<string>` (processed) + `Map<string, string>` (pending). JS event loop is single-threaded; no concurrency primitive needed.
- **Size:** ≤ 95 lines including license/header comment.

[**API Surface**]

```rust
// Library re-exports from ark-core/src/lib.rs
pub use platforms::{
    Platform, PLATFORMS,
    CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM,
};

// CLI (ark-cli/src/main.rs)
struct InitArgs {
    // existing
    #[arg(long)] claude: bool,
    #[arg(long = "no-claude")] no_claude: bool,
    #[arg(long)] codex: bool,
    #[arg(long = "no-codex")] no_codex: bool,
    // new
    #[arg(long)] opencode: bool,
    #[arg(long = "no-opencode")] no_opencode: bool,
}
```

`InitArgs::flags` extends its `match p.cli_flag` from 2 arms to 3 to map each `Platform`'s `cli_flag` to its `(self.<flag>, self.no_<flag>)` pair. Mutually-exclusive `--<flag> --no-<flag>` resolution is behavioural via existing `resolve_platforms_pure` filter `f.on && !f.off` (negative wins on per-platform conflict). The non-TTY error message names every flag pair: `--claude` / `--no-claude`, `--codex` / `--no-codex`, `--opencode` / `--no-opencode`. The interactive-prompt `for platform in PLATFORMS` loop already iterates the registry and grows for free.

Parity tests (tree-walked, not hand-listed):
- `every_claude_command_has_an_opencode_command_sibling`: walks `CLAUDE_TEMPLATES.files()`; for each `commands/ark/<name>.md`, asserts `OPENCODE_TEMPLATES.get_file("ark/<name>.md").is_some()`.
- `opencode_command_bodies_have_opencode_frontmatter`: walks `OPENCODE_TEMPLATES.files()`; for each `.md`, asserts body starts with `---\n` and the first non-`---` line begins with `description:`; asserts no `argument-hint:` line; asserts the body contains the literal heading `` # `/ark:<name> $ARGUMENTS` ``.
- `opencode_plugin_keeps_helpers_internal`: string-level scan of `OPENCODE_ARK_CONTEXT_TS` asserting helpers are defined but NOT exported, and have live consumers in the default-exported factory.
- `opencode_platform_shape`: asserts every field of `OPENCODE_PLATFORM` matches the documented shape (`id == "opencode"`, `dest_dir == OPENCODE_COMMANDS_DIR`, `removal_root == OPENCODE_DIR`, `managed_block_target == Some("AGENTS.md")`, `hook_file.is_none()`, `extra_files.len() == 1`).

[**Constraints**]

- C-1: All paths under `.opencode/` route through `Layout` getters or `OPENCODE_*` consts; no `".opencode/"` literal outside `layout.rs` and `templates.rs`.
- C-2: `OPENCODE_TEMPLATES` is rooted at `templates/opencode/commands`; the plugin file ships via `extra_files` + `include_str!`, mirroring Codex's `config.toml`.
- C-3: `OPENCODE_PLATFORM.hook_file = None`; `Platform::capture_hook` and `remove_hook` short-circuit to `Ok(None)` / `Ok(false)`.
- C-4: `OPENCODE_PLATFORM.removal_root == dest_dir == ".opencode"`; the directory is wholly Ark-owned.
- C-5: `Manifest::record_block` dedupes on `(file, marker)`; calling it twice with `(AGENTS.md, ARK)` writes one entry.
- C-6: OpenCode command bodies are mechanical translations of Claude commands: drop Claude frontmatter, prepend OpenCode frontmatter (`description` only), preserve slash-invocation idioms (`# /ark:<name> $ARGUMENTS`) verbatim.
- C-7: Plugin file is hand-authored TypeScript at `templates/opencode/plugins/ark-context.ts`; size ≤ 95 lines including comments.
- C-8: Plugin's exec target is the `ark` binary on `PATH`; missing binary or non-zero exit → log warning + skip injection.
- C-9: `extra_files` (plugin TS) is not hash-tracked; re-applied unconditionally on every `init` / `load` / `upgrade`.
- C-10: Parity tests are tree-walked: every Claude command has an OpenCode sibling; every OpenCode command's frontmatter starts with `---\ndescription:` and contains no `argument-hint:` line.
- C-11: `OPENCODE_PLATFORM` shape asserted by `opencode_platform_shape` test in `platforms.rs::tests`.
- C-12: Bun syntax check is developer guidance, not a CI test (`bun build --no-bundle <file> > /dev/null`).
- C-13: No changes to `Snapshot` schema, `HookFileSpec`, hook helpers, or any command body apart from registry growth and layout consts.
- C-14: Forward compat: an older `ark` binary reading a manifest with `.opencode/` paths leaves them on disk during `unload`; user upgrades the binary to recover round-trip.
- C-15: `experimental.chat.messages.transform` is named experimental by OpenCode for a reason; if renamed/removed, the plugin source is the only artifact that needs to change.

[**CHANGELOG**]

- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
- 2026-05-10 `subagent-support`: OpenCode now ships `ark-researcher`, `ark-reviewer`, `ark-verifier` under `.opencode/agents/` via the new `Platform.agents_templates` + `agents_dest_dir` fields. `OPENCODE_PLATFORM.agents_templates = Some(&OPENCODE_AGENT_TEMPLATES)`, `agents_dest_dir = Some(OPENCODE_AGENTS_DIR)`, `extra_dirs = []` (the agents directory lives under the platform's existing `removal_root = .opencode`). OpenCode agent file format follows the Trellis OpenCode precedent: YAML frontmatter (`description`, `mode: subagent`, `permission:` block) + markdown body. `Platform` is now `#[non_exhaustive]`. See `specs/features/subagent-support/SPEC.md`.
