
[**Goals**]

- G-1: Register `codeagent-cli` as 4th platform in `PLATFORMS` registry.
- G-2: Ship CodeAgent CLI artifacts at `.cac/commands/ark/*.md`, `.cac/agents/ark-*.md`, and `.cac/settings.json` `SessionStart` hook.
- G-3: Accept `--codeagent` / `--no-codeagent` in `ark init`; CodeAgent CLI joins the TTY prompt and non-TTY error message.
- G-4: Use the existing `AGENTS.md` managed block for CodeAgent CLI (shared with Codex/OpenCode; manifest dedupes on `(file, marker)`).
- G-5: Emit CodeAgent CLI hook entry with seconds for timeout (like Codex), suppressing stdout via `>/dev/null`.
- G-6: `ark upgrade` refreshes CodeAgent CLI templates alongside other installed platforms.
- G-7: `ark context --scope session` lists `codeagent-cli` as an installed platform.

[**Non-goals**]

- NG-1: No `.cac/config.toml`; CodeAgent CLI uses JSON hooks natively and needs no config file.
- NG-2: No changes to `Platform` struct fields; the existing `HookFileSpec` + `extra_files` pattern covers CodeAgent CLI.
- NG-3: No plugin or TS file; CodeAgent CLI has native JSON hooks (unlike OpenCode).

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                     (InitArgs gains 2 CodeAgent flags;
│                                              interactive prompt + non-TTY error
│                                              extend to 4 platforms)
└── ark-core/src/
    ├── lib.rs                               (re-exports CODEAGENT_PLATFORM,
    │                                          CODEAGENT_AGENT_TEMPLATES)
    ├── platforms.rs                         (adds CODEAGENT_PLATFORM; PLATFORMS grows
    │                                          to 4 entries; registry-shape test updated)
    ├── layout.rs                            (CODEAGENT_DIR, CODEAGENT_COMMANDS_DIR,
    │                                          CODEAGENT_AGENTS_DIR, CODEAGENT_SETTINGS_FILE
    │                                          consts; codeagent_dir(), codeagent_commands_dir(),
    │                                          codeagent_agents_dir(), codeagent_settings();
    │                                          owned_dirs grows via registry — no manual entry)
    ├── io/fs/hook.rs                        (ark_codeagent_hook_entry() +
    │                                          CODEAGENT_CONTEXT_HOOK_COMMAND const)
    ├── templates.rs                         (CODEAGENT_TEMPLATES static +
    │                                          CODEAGENT_AGENT_TEMPLATES static)
    ├── state/                                (UNCHANGED)
    └── commands/                             (UNCHANGED bodies; iteration over PLATFORMS
                                              picks up the new entry for free)
templates/
└── codeagent/                                (NEW)
    ├── commands/ark/{quick,design,commit,discard,record,research,resume,spec-audit,spec-extract}.md
    └── agents/{ark-researcher,ark-reviewer,ark-verifier}.md
```

Module coupling: adding CodeAgent CLI introduces no new module dependencies. One const added to `platforms.rs`, two statics + one const to `templates.rs`, four consts + four getters to `layout.rs`, one const + one function to `io/fs/hook.rs`. Identical shape to OpenCode's introduction.

Call graph for `init`: identical to existing — the `for platform in selected_platforms` loop calls `platform.apply_managed_state(&layout, &mut manifest)?` for each selected entry.

Call graph for `unload`: unchanged. The two-stage hook-capture iterates registered `PLATFORMS` entries; CodeAgent CLI contributes a `HookFileSpec` (`.cac/settings.json`, `SessionStart`, `command`).

Call graph for `load`: unchanged. Files under `.cac/` are restored from `snapshot.files`. The `for platform in PLATFORMS` canonical re-apply iterates 4 entries.

[**Data Structure**]

```rust
// ark-core/src/platforms.rs (additions)

/// CodeAgent CLI integration.
///
/// Templates extract under `.cac/`; managed block shares `AGENTS.md` with
/// Codex/OpenCode (manifest dedupes on `(file, marker)`). The `SessionStart`
/// hook lives in `.cac/settings.json` and uses **seconds** for `timeout`
/// (CodeAgent CLI schema matches Codex, not Claude's milliseconds).
pub const CODEAGENT_PLATFORM: Platform = Platform {
    id: "codeagent-cli",
    templates: &CODEAGENT_TEMPLATES,
    dest_dir: CODEAGENT_COMMANDS_DIR,
    removal_root: CODEAGENT_DIR,
    cli_flag: "codeagent",
    managed_block_target: Some(AGENTS_MD),
    hook_file: Some(HookFileSpec {
        path: CODEAGENT_SETTINGS_FILE,
        hooks_array_key: "SessionStart",
        identity_key: "command",
        identity_value: CODEAGENT_CONTEXT_HOOK_COMMAND,
        entry_builder: ark_codeagent_hook_entry,
    }),
    extra_files: &[],
    agents_templates: Some(&CODEAGENT_AGENT_TEMPLATES),
    agents_dest_dir: Some(CODEAGENT_AGENTS_DIR),
    extra_dirs: &[],
};

pub const PLATFORMS: &[&Platform] = &[
    &CLAUDE_PLATFORM,
    &CODEX_PLATFORM,
    &OPENCODE_PLATFORM,
    &CODEAGENT_PLATFORM,
];

// ark-core/src/layout.rs (additions)
pub const CODEAGENT_DIR:            &str = ".cac";
pub const CODEAGENT_COMMANDS_DIR:   &str = ".cac/commands";
pub const CODEAGENT_AGENTS_DIR:     &str = ".cac/agents";
pub const CODEAGENT_SETTINGS_FILE:  &str = ".cac/settings.json";

impl Layout {
    pub fn codeagent_dir(&self)            -> PathBuf;   // <project>/.cac/
    pub fn codeagent_commands_dir(&self)   -> PathBuf;   // <project>/.cac/commands/
    pub fn codeagent_agents_dir(&self)     -> PathBuf;   // <project>/.cac/agents/
    pub fn codeagent_settings(&self)       -> PathBuf;   // <project>/.cac/settings.json
}

// ark-core/src/templates.rs (additions)
pub static CODEAGENT_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/codeagent/commands");

pub static CODEAGENT_AGENT_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/codeagent/agents");

// ark-core/src/io/fs/hook.rs (additions)
/// CodeAgent CLI hook command. Suppresses stdout via `>/dev/null` (same
/// rationale as Codex — the platform does not hide hook stdout from the UI).
pub const CODEAGENT_CONTEXT_HOOK_COMMAND: &str =
    "ark context --scope session --format json >/dev/null";

/// Builds the canonical Ark CodeAgent CLI `SessionStart` hook entry.
///
/// Schema matches CodeAgent CLI's hooks contract (identical to Claude's
/// `{matcher, hooks: [...]}` shape). `timeout` is in **seconds** (same as
/// Codex, unlike Claude's milliseconds). 30 seconds gives `ark context`
/// sufficient budget.
pub fn ark_codeagent_hook_entry() -> serde_json::Value {
    serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": CODEAGENT_CONTEXT_HOOK_COMMAND,
                "timeout": 30,
            }
        ],
    })
}
```

[**API Surface**]

```rust
// Library re-exports from ark-core/src/lib.rs
pub use platforms::{
    Platform, PLATFORMS,
    CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM, CODEAGENT_PLATFORM,
};
pub use templates::{
    CLAUDE_AGENT_TEMPLATES, CODEX_AGENT_TEMPLATES, OPENCODE_AGENT_TEMPLATES,
    CODEAGENT_AGENT_TEMPLATES,
};
pub use io::{
    HookFileSpec, ark_codeagent_hook_entry,
    // ...existing re-exports unchanged
};

// CLI (ark-cli/src/main.rs)
struct InitArgs {
    // existing
    #[arg(long)] claude: bool,
    #[arg(long = "no-claude")] no_claude: bool,
    #[arg(long)] codex: bool,
    #[arg(long = "no-codex")] no_codex: bool,
    #[arg(long)] opencode: bool,
    #[arg(long = "no-opencode")] no_opencode: bool,
    // new
    /// Install CodeAgent CLI integration (default: prompt on TTY).
    #[arg(long)]
    codeagent: bool,
    /// Skip CodeAgent CLI integration.
    #[arg(long = "no-codeagent")]
    no_codeagent: bool,
}
```

`InitArgs::flags` extends its `match p.cli_flag` from 3 arms to 4. The non-TTY error message names every flag pair: `--claude`, `--codex`, `--opencode`, `--codeagent`. The interactive-prompt `for platform in PLATFORMS` loop already iterates the registry and grows for free.

Template file contracts:

- **Commands** (`.cac/commands/ark/*.md`): YAML frontmatter with `description` only (same as OpenCode), body uses `$ARGUMENTS` token, heading `# \`/ark:<name> $ARGUMENTS\``. No `argument-hint:` line.
- **Agents** (`.cac/agents/*.md`): YAML frontmatter (`name`, `description`, `permissionMode`, `tools` list), then markdown body. Same format as Claude's agents. Body is byte-identical to Claude/Codex/OpenCode agents after frontmatter strip + commit-command normalization.
- **Hook entry** (`.cac/settings.json`): `hooks.SessionStart` array, `{matcher: "", hooks: [{type: "command", command: ..., timeout: 30}]}`. Timeout in seconds.

Parity tests (tree-walked, not hand-listed):
- `every_claude_command_has_a_codeagent_command_sibling`: walks `CLAUDE_TEMPLATES.files()`; for each `commands/ark/<name>.md`, asserts `CODEAGENT_TEMPLATES.get_file("ark/<name>.md").is_some()`.
- `codeagent_command_bodies_have_codeagent_frontmatter`: walks `CODEAGENT_TEMPLATES.files()`; for each `.md`, asserts body starts with `---\n` and the first non-`---` line begins with `description:`; asserts no `argument-hint:` line; asserts the body contains the literal heading `` # `/ark:<name> $ARGUMENTS` ``.
- `codeagent_agent_frontmatter_shape`: for each agent, asserts `---\nname: ark-<stem>\n` opening, `description:` and `tools:` fields present.
- `codeagent_platform_shape`: asserts every field of `CODEAGENT_PLATFORM` matches the documented shape.

[**Constraints**]

- C-1: @source-scan: .cac/ @ crates/ark-core/src/commands/**/*.rs
All paths under `.cac/` route through `Layout` getters or `CODEAGENT_*` consts; no `".cac/"` literal outside `layout.rs` and `templates.rs`.
- C-2: @test-binding: codeagent_templates_rooted_at_commands
`CODEAGENT_TEMPLATES` is rooted at `templates/codeagent/commands`; agents ship via the dedicated `CODEAGENT_AGENT_TEMPLATES` static.
- C-3: @test-binding: codeagent_platform_shape
`CODEAGENT_PLATFORM.id == "codeagent-cli"`, `cli_flag == "codeagent"`, `dest_dir == CODEAGENT_COMMANDS_DIR`, `removal_root == CODEAGENT_DIR`, `managed_block_target == Some("AGENTS.md")`, `hook_file.is_some()`, `extra_files.is_empty()`, `agents_templates.is_some()`, `agents_dest_dir == Some(CODEAGENT_AGENTS_DIR)`, `extra_dirs.is_empty()`.
- C-4: @test-binding: codeagent_hook_entry_carries_canonical_command_in_seconds
`ark_codeagent_hook_entry()` produces `timeout: 30` (seconds), `command == CODEAGENT_CONTEXT_HOOK_COMMAND`.
- C-5: @test-binding: every_claude_command_has_a_codeagent_command_sibling
Every Claude command has a CodeAgent CLI sibling; every CodeAgent CLI command's frontmatter starts with `---\ndescription:` and contains no `argument-hint:` line.
- C-6: @test-binding: codeagent_command_bodies_have_codeagent_frontmatter
CodeAgent CLI command bodies are mechanical translations of Claude commands: drop Claude frontmatter, prepend `description`-only frontmatter, preserve slash-invocation idioms (`# /ark:<name> $ARGUMENTS`) verbatim.
- C-7: @test-binding: each_platform_ships_three_agents
`CODEAGENT_AGENT_TEMPLATES` ships exactly the three Ark agents; parity tested against `CLAUDE_AGENT_TEMPLATES` as canonical.
- C-8: @test-binding: agent_bodies_are_byte_identical_modulo_platform_idioms
Every CodeAgent CLI agent prompt body is byte-identical to Claude/Codex/OpenCode after frontmatter strip + commit-command normalization.
- C-9: @test-binding: codeagent_apply_managed_state_writes_block_and_hook
`CODEAGENT_PLATFORM.apply_managed_state` writes the `AGENTS.md` managed block and the `.cac/settings.json` `SessionStart` hook entry.
- C-10: @test-binding: codeagent_capture_hook_captures_then_removes_only_ark_entry
`CODEAGENT_PLATFORM.capture_hook` captures the Ark entry, removes it, preserves sibling user entries.
- C-11: @judgment
`.cac/settings.json` hook entry is not hash-tracked; re-applied unconditionally on every `init` / `load` / `upgrade`. Sibling user content preserved.
- C-12: @judgment
`CODEAGENT_PLATFORM.removal_root == ".cac"`; the directory is wholly Ark-owned.
- C-13: @test-binding: platforms_registry_has_four_entries_in_canonical_order
`PLATFORMS.len() == 4`; order is `[CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM, CODEAGENT_PLATFORM]`.
- C-14: @judgment
No changes to `Snapshot` schema, `HookFileSpec` struct, or any command body apart from registry growth and layout consts.
- C-15: @judgment
An existing Claude/Codex/OpenCode-only project stays unchanged on `ark upgrade`; adding CodeAgent CLI requires re-running `ark init --codeagent` (additive, idempotent).

---
