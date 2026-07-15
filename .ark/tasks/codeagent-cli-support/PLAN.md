# `codeagent-cli-support` PLAN

> Status: Draft | Approved for Implementation
> Feature: `codeagent-cli-support`
> Owner: Executor

---

## Summary

Add CodeAgent CLI as a 4th platform in Ark's `PLATFORMS` registry. CodeAgent CLI uses `.cac/` as its root directory, JSON-based hooks in `.cac/settings.json` (same shape as Claude's, timeout in seconds like Codex), markdown commands at `.cac/commands/ark/*.md` (same frontmatter as OpenCode), and markdown agents at `.cac/agents/*.md` (same YAML-frontmatter format as Claude's agents). No `Platform` struct changes, no command body changes — purely a registry entry plus a new template tree and hook entry function.

> Deep tier: REVIEW findings are folded into this PLAN in place before EXECUTE — there is no iteration history to track here.

---

## Spec

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

## Runtime

[**Main Flow**]

1. `ark init --codeagent` resolves `CODEAGENT_PLATFORM` from the `PLATFORMS` registry.
2. `Platform::apply_managed_state` writes `AGENTS.md` managed block, `.cac/settings.json` `SessionStart` hook entry, and extracts `CODEAGENT_TEMPLATES` + `CODEAGENT_AGENT_TEMPLATES`.
3. `ark unload` captures `.cac/` files + hook entry into `.ark.db`.
4. `ark load` restores from snapshot, then re-applies canonical hook entry for `CODEAGENT_PLATFORM`.
5. `ark remove` wipes `.cac/` (removal_root), removes hook entry from `.cac/settings.json`, removes `AGENTS.md` managed block.

[**Failure Flow**]

1. `.cac/settings.json` parse failure → `update_hook_file` treats as empty `{}` and writes fresh.
2. Missing `ark` binary at hook runtime → CodeAgent CLI logs warning, session proceeds without context injection.

[**State Transitions**]

- No new state transitions. Task lifecycle unchanged.

---

## Implementation

[**Phase 1: Layout + Templates + Hook Entry**]

1. Add `CODEAGENT_DIR`, `CODEAGENT_COMMANDS_DIR`, `CODEAGENT_AGENTS_DIR`, `CODEAGENT_SETTINGS_FILE` consts to `layout.rs`.
2. Add `codeagent_dir()`, `codeagent_commands_dir()`, `codeagent_agents_dir()`, `codeagent_settings()` getters to `Layout`.
3. Add `CODEAGENT_TEMPLATES` and `CODEAGENT_AGENT_TEMPLATES` statics to `templates.rs`.
4. Add `CODEAGENT_CONTEXT_HOOK_COMMAND` const and `ark_codeagent_hook_entry()` function to `io/fs/hook.rs`.
5. Re-export `ark_codeagent_hook_entry` from `io/mod.rs` and `io/fs/mod.rs`.
6. Create `templates/codeagent/commands/ark/` directory with 9 command `.md` files (mechanical translation from OpenCode commands — same frontmatter, same body).
7. Create `templates/codeagent/agents/` directory with 3 agent `.md` files (adapted from Claude agents — same frontmatter shape as CodeAgent CLI expects: `name`, `description`, `permissionMode`, `tools`).

[**Phase 2: Platform Registry + CLI Flags**]

1. Add `CODEAGENT_PLATFORM` const to `platforms.rs`.
2. Add `&CODEAGENT_PLATFORM` to `PLATFORMS` slice.
3. Update `platforms_registry_has_three_entries_in_canonical_order` test to expect 4 entries.
4. Add `Platform::by_id("codeagent-cli")` and `Platform::by_cli_flag("codeagent")` coverage.
5. Add `--codeagent` / `--no-codeagent` flags to `InitArgs` in `ark-cli/src/main.rs`.
6. Add `"codeagent"` arm to `InitArgs::flags()` match.
7. Update non-TTY error message to list all 4 platform flag pairs.

[**Phase 3: Tests + Existing SPEC CHANGELOGs**]

1. Add `codeagent_platform_shape` test.
2. Add `every_claude_command_has_a_codeagent_command_sibling` test.
3. Add `codeagent_command_bodies_have_codeagent_frontmatter` test.
4. Add `codeagent_agent_frontmatter_shape` test.
5. Extend `each_platform_ships_three_agents` to cover `CODEAGENT_AGENT_TEMPLATES`.
6. Extend `agent_bodies_are_byte_identical_modulo_platform_idioms` to cover `"codeagent"` platform.
7. Extend `agent_prompts_carry_recursion_guard`, `agent_prompts_carry_write_scope_walls`, `researcher_prompt_carries_paths_summaries_contract`, `reviewer_prompt_carries_self_containment_rule`, `reviewer_prompt_carries_spec_contradiction_rule`, `verifier_prompt_carries_no_self_fix_rule` to cover `"codeagent"`.
8. Add `codeagent_apply_managed_state_writes_block_and_hook` test.
9. Add `codeagent_capture_hook_captures_then_removes_only_ark_entry` test.
10. Add `codeagent_hook_entry_carries_canonical_command_in_seconds` test.
11. Add CHANGELOG entries to `codex-support/SPEC.md` and `opencode-support/SPEC.md` and `subagent-support/SPEC.md` noting the 4th platform.
12. Run full test suite: `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`.

---

## Trade-offs

- T-1: **`>/dev/null` in hook command** (same as Codex). CodeAgent CLI may not implement `suppressOutput` for hook stdout in the UI; redirecting avoids noise. If a future version supports suppression, the `>/dev/null` suffix can be dropped (breaking change to `CODEAGENT_CONTEXT_HOOK_COMMAND`). The orphan-hook scan in `unload` hard-codes `ARK_CONTEXT_HOOK_COMMAND`; CodeAgent CLI's hook is registered in `PLATFORMS` so the first-stage `capture_hook` handles it via its own `identity_value` — the orphan scan need not match `CODEAGENT_CONTEXT_HOOK_COMMAND`.
- T-2: **CLI flag `--codeagent`** rather than `--cac`. The flag names the platform identity (`codeagent-cli`), not the directory (`.cac/`). This is consistent with `--claude` (not `--dot-claude`) and `--codex` (not `--dot-codex`).
- T-3: **Agent frontmatter uses `permissionMode: bypassPermissions`** with an explicit `tools` list, matching the existing `.cac/agents/ark-*.md` files already in the project. This differs from Claude's simpler `tools:` field and OpenCode's `mode: subagent` + `permission:` block, but matches CodeAgent CLI's native format.

---

## Validation

[**Unit Tests**]

- V-UT-1: `codeagent_platform_shape` — all `CODEAGENT_PLATFORM` fields match spec
- V-UT-2: `codeagent_hook_entry_carries_canonical_command_in_seconds` — timeout=30, command=CODEAGENT_CONTEXT_HOOK_COMMAND
- V-UT-3: `codeagent_apply_managed_state_writes_block_and_hook` — managed block in AGENTS.md + hook entry in .cac/settings.json
- V-UT-4: `codeagent_capture_hook_captures_then_removes_only_ark_entry` — captures Ark entry, preserves sibling
- V-UT-5: `every_claude_command_has_a_codeagent_command_sibling` — 9 command files exist
- V-UT-6: `codeagent_command_bodies_have_codeagent_frontmatter` — frontmatter starts with `description:`, no `argument-hint:`, heading present
- V-UT-7: `codeagent_agent_frontmatter_shape` — frontmatter has `name`, `description`, `permissionMode`, `tools`
- V-UT-8: Extended parity tests — agents identical after frontmatter strip, recursion guard, write scope walls, contract phrases

[**Integration Tests**]

- V-IT-1: `ark init --codeagent --dir $TMP` creates `.cac/commands/ark/*.md`, `.cac/agents/ark-*.md`, `.cac/settings.json` with `SessionStart` hook, and `AGENTS.md` managed block
- V-IT-2: `ark unload --dir $TMP && ark load --dir $TMP` round-trips CodeAgent CLI artifacts losslessly
- V-IT-3: `ark remove --dir $TMP` cleans up all `.cac/` artifacts and AGENTS.md managed block
- V-IT-4: `ark context --scope session --format json` lists `codeagent-cli` in installed platforms
- V-IT-5: `ark upgrade --dir $TMP` refreshes `.cac/` command and agent templates when codeagent-cli is installed

[**Failure / Robustness**]

- V-F-1: Corrupt `.cac/settings.json` → `update_hook_file` treats as empty and writes fresh
- V-F-2: Missing `ark` binary at hook runtime → hook logs warning, session proceeds

[**Edge Cases**]

- V-E-1: `.cac/settings.json` with existing user `SessionStart` entries → Ark appends, preserves siblings
- V-E-2: Shared `AGENTS.md` block when both Codex and CodeAgent CLI are installed → manifest records one block entry
- V-E-3: User-authored agents at non-reserved stems in `.cac/agents/` survive `remove`

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-IT-1 |
| G-2 | V-UT-5, V-UT-6, V-UT-7, V-IT-1 |
| G-3 | V-IT-1 (CLI flags) |
| G-4 | V-UT-3, V-E-2 |
| G-5 | V-UT-2 |
| G-6 | V-IT-5 |
| G-7 | V-IT-4 |
| C-1 | source-scan test |
| C-2 | V-UT-5 (templates rooted at commands/) |
| C-3 | V-UT-1 |
| C-4 | V-UT-2 |
| C-5 | V-UT-5 |
| C-6 | V-UT-6 |
| C-7 | V-UT-8 |
| C-8 | V-UT-8 |
| C-9 | V-UT-3 |
| C-10 | V-UT-4 |
| C-11 | V-UT-3 (unconditional re-apply) |
| C-12 | V-UT-1 (removal_root == .cac) |
| C-13 | V-UT-1 (4 entries) |
| C-14 | @judgment: review confirms no Snapshot/HookFileSpec/command-body changes |
| C-15 | V-IT-1 (additive) |
