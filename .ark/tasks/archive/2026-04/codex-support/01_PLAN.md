# `codex-support` PLAN `01`

> Status: Revised
> Feature: `codex-support`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Iteration 01 addresses the four blocking findings (R-001 timeout unit, R-002 snapshot round-trip story, R-003 parameterization scope, R-004 source-scan coverage) and reshapes the Codex slash-command surface in response to **R-013, which on verification turns out to be load-bearing**: `.codex/prompts/` is a feature request (openai/codex#9848), not a supported discovery path. Current Codex CLI only auto-loads prompts from `$CODEX_HOME/prompts/` (global `~/.codex/prompts/`), not project-scope. Trellis ships skills (`.codex/skills/*/SKILL.md`) precisely because that's the actual project-scope mechanism that exists today.

So the iteration replaces `.codex/prompts/ark-{quick,design,archive}.md` with `.codex/skills/ark-{quick,design,archive}/SKILL.md`. Skills carry YAML frontmatter (`name`, `description`) that enables description-based routing; user invokes via natural language ("use ark-quick to ship a small fix") rather than a slash. This is a UX departure from Claude's `/ark:quick` slash command — accepted because the alternative (waiting on upstream Codex) isn't feasible. The G-12 parity invariant holds in the new shape: each Claude command has a Codex skill twin.

Beyond the slash-command surface change, six other revisions:
- Codex hook timeout switches to seconds-unit (`timeout = 30`, well under Codex's 600s default), with explicit doc and unit-test guard.
- `load` re-applies canonical hook entries via `Platform::hook_file.entry_builder` after replaying `snapshot.hook_bodies`, making post-load state independent of snapshot age.
- Parameterization narrows to `hooks_array_key: &str` (drops the RFC 6901 over-engineering); both shipping platforms use `SessionStart`, future platforms add a key, not a parser.
- Source-scan test extended to cover all five command files (`init`, `upgrade`, `unload`, `load`, `remove`), not just `platforms.rs`.
- Deprecation aliases become explicit `#[deprecated(since = "0.2.0")]` thin-wrapper functions, removed at 0.3.0.
- Non-TTY without platform flags errors instead of silently installing both.
- G-14 codifies "Claude-only project stays Claude-only on upgrade; opt in via `ark init --codex` rerun".

## Log

[**Added**]
- G-14 — explicit Claude-only-stays-Claude-only invariant on upgrade.
- C-22 — canonical re-apply in `load` after `hook_bodies` replay.
- C-23 — `#[deprecated]` thin-wrapper aliases with explicit removal milestone.
- C-24 — `unload` captures Ark hook entries by file-presence + identity scan, not `PLATFORMS` membership.
- C-25 — Codex hook timeout unit (seconds) with explicit comment in `ark_codex_hook_entry()`.
- V-IT-15 — `upgrade_on_claude_only_project_does_not_install_codex`.
- V-IT-16 — `load_after_replay_re_applies_canonical_entries`.
- V-IT-17 — `unload_captures_orphan_ark_hook_entries_in_unregistered_files`.
- V-UT-10 — `ark_codex_hook_entry_uses_seconds_unit` (constant guard).

[**Changed**]
- G-4 — Codex artifacts move from `.codex/prompts/ark-*.md` to `.codex/skills/ark-*/SKILL.md`. Skills use YAML frontmatter (`name`, `description`) for Codex's description-based routing.
- G-3 — non-TTY without platform flags errors instead of installing both.
- G-6 — parameterization is `hooks_array_key: &str`, not a JSON Pointer. Helper signatures simplify.
- G-12 — parity test renamed `every_claude_command_has_a_codex_skill_sibling` (was `templates_codex_prompts_match_claude_commands`); content-parity check added — after stripping Claude frontmatter, stripping Codex frontmatter, and rewriting `/ark:foo` → `ark-foo` in the Claude body, body byte-equality is asserted (per R-011).
- G-13 — `Layout` getters renamed: `codex_prompts_dir` → `codex_skills_dir`. `CODEX_PROMPTS_DIR` → `CODEX_SKILLS_DIR`.
- C-7 — describes the skill body shape (frontmatter present, contents are the slash-command body with adjustments).
- C-15 — clarified that ark-context C-15's "5000ms (Claude-side)" is Claude-only; Codex has its own seconds-unit constant.
- C-19 — replaced. New C-19: array-key validation (`[A-Za-z0-9_-]+`); RFC 6901 dropped.
- C-18 — extended to cover all five command files plus `platforms.rs`. Mirror the existing `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` shape across `init.rs`, `unload.rs`, `load.rs`, `remove.rs`.
- Failure flow #5 — replaced by C-24's identity-value scan; entries from unregistered files are captured into `hook_bodies` like any other.
- Failure flow #7 — replaced by C-22; legacy snapshots no longer need a separate recovery path.
- T-2 — re-positioned: parameterize over `hooks_array_key`, not full pointer. Justifies dropping C-19/V-UT-5/V-UT-6.

[**Removed**]
- C-19 (old) — RFC 6901 escapes. Replaced with array-key validation only.
- V-UT-5 — `update_hook_file_rejects_pointer_without_leading_slash`. No pointer.
- V-UT-6 — `update_hook_file_handles_rfc6901_escapes`. No pointer.
- The "manifest-derived recovery for legacy snapshots" framing in Failure flow #7 — single canonical re-apply path now.

[**Unresolved**]
- R-013's deeper question — should we *also* author Codex prompts under `.codex/prompts/` in anticipation of openai/codex#9848 landing? Recommendation: no. Skills are sufficient and ship today; adding prompts later, when the upstream feature lands, is a one-task follow-up that doesn't gate this PLAN.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Codex hook timeout = 30 (seconds). Comment explains unit. New V-UT-10 guards the constant. C-25 documents the unit. |
| Review | R-002 | Accepted | C-22 added: `load` runs `update_hook_file` with `(platform.hook_file.entry_builder)()` for every installed platform after `hook_bodies` replay. Post-load state is canonical regardless of snapshot age. V-IT-16 enforces. |
| Review | R-003 | Accepted | TR-2 adopted. Parameterization narrowed to `hooks_array_key: &str`. Dropped RFC 6901 parser, dropped C-19, dropped V-UT-5/V-UT-6. New C-19 just validates the array-key shape. |
| Review | R-004 | Accepted | C-18 rewrites: source-scan test covers all five command files (`init`, `upgrade`, `unload`, `load`, `remove`) plus `platforms.rs`. The existing `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` becomes a per-file pattern repeated across the five files; `fs.rs` is exempt (sanctioned site, same as the existing carve-out). |
| Review | R-005 | Accepted | C-23 added: aliases ship as `#[deprecated(since = "0.2.0", note = "Use update_hook_file")]` thin-wrapper functions in `io/fs.rs`, removed at 0.3.0. Not `pub use` aliases. |
| Review | R-006 | Accepted | C-24 added: `unload` walks the file-presence path. For each registered platform's `hook_file.path`, capture if file exists. Additionally, scan every `.json` file under `owned_dirs()` for entries whose `command == ARK_CONTEXT_HOOK_COMMAND` (in case of unregistered platforms). All captures are surgical. V-IT-17 enforces. |
| Review | R-007 | Accepted | G-3 reverses non-TTY default. Non-TTY without `--claude` / `--codex` / `--no-X` → error "init requires --claude, --codex, or both when stdin is not a TTY." Failure flow updates accordingly. V-IT-11 changes meaning. |
| Review | R-008 | Accepted | G-14 added. V-IT-15 enforces. The "expand selectively" path is just `ark init --codex` rerun on the existing project (idempotent + additive). |
| Review | R-009 | Accepted | C-19 dropped per R-003. |
| Review | R-010 | Accepted | C-8 gets one extra paragraph addressing forward direction (older binary reading newer snapshot). |
| Review | R-011 | Accepted | G-12 adds content-parity check + rename. The bytes-after-stripping-frontmatter must match modulo the `/ark:foo` → `ark-foo` rewrite. Test is mechanical; future intentional divergence is a deliberate edit + test update. |
| Review | R-012 | Accepted | Phase 1 step 4 expanded. Specifically called out: `remove.rs` line 76's `let [a, b] = layout.owned_dirs()` destructure → `for d in layout.owned_dirs()`. |
| Review | R-013 | Accepted with reshape | `.codex/prompts/` confirmed unsupported on current Codex (openai/codex#9848). Switch to `.codex/skills/`. G-4, G-12, G-13, C-7 updated. Trade-off T-4 expanded (T-8 added) to address the Codex-side UX (description-based routing vs slash invocation). |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here. ✓
> - Every Master directive must appear here. (None.)
> - Rejections must include explicit reasoning. (None rejected.)

---

## Spec `Core specification`

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

## Runtime `runtime logic`

[**Main Flow — `init`**]

1. CLI parses `InitArgs`. `resolve_platforms(args, stdin_is_tty)` produces `Vec<&'static Platform>` or `Err(InitError::NoPlatforms)`.
2. Library-side `init` is non-interactive; receives the resolved list.
3. Extract `ARK_TEMPLATES`.
4. For each platform: extract templates, install managed block (record), install hook entry (don't record).
5. Ensure empty dirs.
6. Write manifest.

[**Main Flow — `upgrade`**]

Unchanged from 00 in shape; step 6 hook re-apply now uses `hooks_array_key` parameter; Codex `.codex/config.toml` whole-file rewrite happens iff `manifest.files` contains any `.codex/*` entry.

[**Main Flow — `unload`**]

1. Walk `owned_dirs`; capture into `snapshot.files`.
2. Capture+remove each `manifest.managed_blocks` entry.
3. **Stage A**: For each `PLATFORMS` entry with `hook_file.is_some()`: read+remove the registered file's Ark entry. Track captured paths.
4. **Stage B (C-24)**: For each `*.json` file under `owned_dirs()` not already in stage A's captured-paths set: parse, find any entry with `command == ARK_CONTEXT_HOOK_COMMAND`, capture+remove surgically. Parse failures are non-fatal (warn).
5. Persist snapshot.
6. Delete owned dirs. Prune empty parents.

[**Main Flow — `load`**]

1. Read snapshot.
2. Restore `snapshot.files`.
3. Re-apply each managed block.
4. **Replay phase**: For each `snapshot.hook_bodies` entry, `update_hook_file(body.path, body.entry, array_key_from(body.json_pointer), body.identity_key)`. (Snapshot still carries `json_pointer` for portability; the helper extracts the array-key suffix; both shipping platforms have a one-segment-key pointer of the form `/hooks/<KEY>` so derivation is `path.rsplit('/').next()`.)
5. **Canonical re-apply phase (C-22)**: For each `PLATFORMS` entry with `hook_file.is_some()` whose `dest_dir` appears as a prefix in `snapshot.files`, call `update_hook_file(spec.path, (spec.entry_builder)(), spec.hooks_array_key, spec.identity_key)`. The replay-phase entry is overwritten with the canonical bytes; old snapshots upgrade to current shape transparently.
6. Regenerate manifest hashes; write manifest.

[**Failure Flow**]

1. `init --no-claude --no-codex` → `InitError::NoPlatforms` ("init requires at least one platform").
2. `init` interactive prompt with both unchecked → same error.
3. **`init` non-TTY without flags → `InitError::NoPlatforms` with a message naming the available flags** (R-007 / G-3 reversal).
4. Codex hook write fails → `Error::Io`; partial-state behavior matches today's Claude.
5. `unload` on a project with `.codex/` from a future Ark → C-24 captures any Ark-shaped hook entry in any `.json` file. No data loss for `.json`-shaped hook surfaces. Non-JSON hook surfaces (theoretical, hypothetical) remain a follow-up.
6. `upgrade` on a Codex-installed project where the user removed `.codex/hooks.json` → `update_hook_file` re-creates.
7. (Deleted — replaced by C-22.) Old snapshot loaded into new Ark: replay phase re-applies the captured entries; canonical phase normalizes to current shape. Single path, no manifest-derived recovery.

[**State Transitions**]

- Project state ∈ {NotLoaded, ClaudeOnly, CodexOnly, Both}. Determined by manifest entry prefixes.
- `init` transitions NotLoaded → {ClaudeOnly | CodexOnly | Both} per `opts.platforms`.
- `init --codex` re-run on a Claude-only project (G-14) → ClaudeOnly → Both. Idempotent on Claude artifacts; additive on Codex artifacts; manifest gains `.codex/*` entries.
- `init --no-codex` re-run on a Both project → no-op; `init` does not remove. Removal is `ark unload` / `ark remove`'s job.

## Implementation `split task into phases`

[**Phase 1 — Layout and templates**]

1. Author `templates/codex/skills/ark-{quick,design,archive}/SKILL.md`. Each carries YAML frontmatter (`name`, `description` — pulled from the matching Claude command's intent) plus the body of `templates/claude/commands/ark/<name>.md` with its own frontmatter stripped and `/ark:<name>` references rewritten to `ark-<name>`.
2. Author `templates/codex/hooks.json` with the SessionStart entry (timeout: 30) and `templates/codex/config.toml` (`project_doc_fallback_filenames = ["AGENTS.md"]`).
3. Add `CODEX_TEMPLATES` static in `templates.rs`.
4. Extend `Layout` with `codex_*` and `agents_md` getters; add consts.
5. Extend `owned_dirs` to 3 entries. **Specifically: replace the `let [a, b] = layout.owned_dirs()` destructure in `remove.rs` line 76 with slice iteration** (R-012). Update any other consumer.

[**Phase 2 — Platform registry and hook-helper parameterization**]

1. Create `platforms.rs` with `Platform` + `PLATFORMS` + `CLAUDE_PLATFORM` + `CODEX_PLATFORM`.
2. Add `HookFileSpec` to `io/fs.rs`.
3. Rename internals to `update_hook_file` etc. and parameterize over `(hooks_array_key, identity_key)`. Validate the key (C-19).
4. Add `ark_codex_hook_entry()` with seconds-unit and doc comment.
5. Add the three deprecated thin wrappers (C-23).
6. Update existing tests in `io/fs.rs` to use new names. Add an `update_hook_file_via_deprecated_alias_still_works` test to lock the alias contract.

[**Phase 3 — Refactor commands to drive from `PLATFORMS`**]

1. `init.rs`: add `InitOptions::platforms`; iterate.
2. `upgrade.rs`: extend `collect_desired_templates`; per-installed-platform hook + block + config refresh.
3. `unload.rs`: implement Stage A + Stage B (C-24).
4. `load.rs`: replay phase + canonical re-apply phase (C-22).
5. `remove.rs`: per-platform hook removal; preserve manifest-driven managed-block removal.
6. CLI: `InitArgs` flags + `resolve_platforms` + interactive prompt + non-TTY error path.

[**Phase 4 — Tests + workflow doc updates**]

1. Update existing init/upgrade/unload/load/remove tests to install both platforms by default, or explicitly opt into single-platform via `with_platforms(vec![&CLAUDE_PLATFORM])`.
2. Add `every_claude_command_has_a_codex_skill_sibling` parity test in `templates.rs` (existence + body content modulo rewrite).
3. Add round-trip test: install both → unload → load → byte-identical filesystem.
4. Add CLI flag-resolution tests including non-TTY-no-flags errors.
5. Add the new V-IT-15, V-IT-16, V-IT-17, V-UT-10.
6. Update `.ark/workflow.md` with one note that Codex is supported.

## Trade-offs `ask reviewer for advice`

- T-1: Static slice — adopted (TR-1).
- T-2: Parameterize narrowly — adopted (TR-2). `hooks_array_key: &str` not full pointer.
- T-3: Prompt on TTY, **error on non-TTY without flags** — adopted (TR-3).
- T-4: Mechanical translation — adopted, with content-parity test (TR-4).
- T-5: AGENTS.md managed block — adopted (TR-5).
- T-6: Struct — adopted (TR-6).
- T-7: Parity test in `templates.rs` — adopted (TR-7).
- **T-8 (NEW)**: Codex slash-command UX. Skills (description-routed) vs prompts (slash-invoked).
  - Adv. (skills): Project-scope discovery works on current Codex. Description routing surfaces commands when users describe the task. Trellis-confirmed pattern.
  - Disadv. (skills): No 1:1 keystroke parity with `/ark:quick`; user types prose or invokes by name.
  - Adv. (prompts): Direct UX parity with Claude's `/ark:quick` → `/ark-quick`.
  - Disadv. (prompts): Project-scope discovery not supported on current Codex (openai/codex#9848 open). Files would ship and never be found.
  - **Recommendation**: skills now; ship prompts later if openai/codex#9848 lands and the user base wants both invocation paths. The G-12 parity test is keyed on skill files; adding prompt files later is a separate task that can use the *same* G-12 test mechanism for its own files.

## Validation `test design`

Unchanged from 00 except:

[**Removed**]
- V-UT-5 (RFC 6901 leading slash) — gone with C-19 simplification.
- V-UT-6 (RFC 6901 escapes) — gone.

[**Renamed**]
- V-IT-9: `every_claude_command_has_a_codex_skill_sibling` (was `templates_codex_prompts_match_claude_commands`). Now also checks body content modulo frontmatter strip + `/ark:foo` rewrite (per R-011, G-12).

[**Updated wording**]
- V-IT-11: `cli_resolve_platforms_no_flags_non_tty_errors` (was `…installs_both`). Asserts `Err(InitError::NoPlatforms)`.
- V-F-1: same shape; the message-content assertion includes the names of available flags.

[**Added**]

- **V-UT-10 (G-6, C-25)**: `ark_codex_hook_entry_uses_seconds_unit`. Asserts `entry["hooks"][0]["timeout"] == 30` (the seconds-unit constant).
- **V-IT-15 (G-14)**: `upgrade_on_claude_only_project_does_not_install_codex`. `init` with `vec![&CLAUDE_PLATFORM]` only → `upgrade` → `.codex/` does not exist; `manifest.files` does not gain `.codex/*` entries; `AGENTS.md` block is not installed.
- **V-IT-16 (G-9, C-22)**: `load_after_replay_re_applies_canonical_entries`. Hand-craft a `.ark.db` whose `hook_bodies[0].entry` differs from the current `ark_codex_hook_entry()` (e.g. a stale `timeout`). Run `load`. Assert the on-disk `.codex/hooks.json` matches the *current* canonical entry, not the snapshot's stale value. Locks C-22.
- **V-IT-17 (C-24)**: `unload_captures_orphan_ark_hook_entries_in_unregistered_files`. Hand-place an Ark-identity hook entry inside `.codex/extras.json` (a future-version-shaped file unknown to current `PLATFORMS`). `unload` → `snapshot.hook_bodies` contains the entry; the file on disk has the entry surgically removed.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-IT-1 |
| G-2 | V-UT-1, V-UT-2, V-UT-8 |
| G-3 | V-IT-1, V-IT-2, V-IT-3, V-IT-10, **V-IT-11** (revised), V-IT-12, V-F-1 |
| G-4 | V-IT-1, V-IT-2, V-IT-9 |
| G-5 | V-IT-1, V-IT-13 |
| G-6 | V-UT-4, V-UT-7, V-UT-10, V-IT-6 |
| G-7 | V-IT-6, V-IT-7 |
| G-8 | V-IT-5, V-E-1 |
| G-9 | V-IT-4, V-IT-14, V-IT-16 |
| G-10 | V-IT-8, V-IT-13 |
| G-11 | V-IT-6, V-IT-7 |
| G-12 | V-IT-9 (existence + content-parity) |
| G-13 | V-UT-8, V-UT-9 |
| **G-14** | V-IT-15 |
| C-1 | V-UT-1, V-UT-2, V-UT-3 |
| C-2 | V-IT-10 |
| C-3 | V-IT-10, V-IT-11 |
| C-4 | V-UT-4 |
| C-5 | V-UT-7, V-UT-10 |
| C-6 | V-IT-9 |
| C-7 | V-E-2, V-IT-9 |
| C-8 | V-IT-14 |
| C-9 | V-IT-4, V-IT-5 |
| C-10 | V-IT-4, V-IT-16 |
| C-11 | V-IT-6, V-IT-7 |
| C-12 | V-IT-8 |
| C-13 | V-IT-10 |
| C-14 | V-IT-9 |
| C-15 | V-UT-9, V-E-3 |
| C-16 | (regression-only) |
| C-17 | V-UT-8 |
| C-18 (revised) | source-scan tests in each of `init.rs`, `upgrade.rs`, `unload.rs`, `load.rs`, `remove.rs`, `platforms.rs` |
| C-19 (revised) | V-UT-4 (round-trip with explicit key); new V-UT-5b `update_hook_file_rejects_invalid_array_key` |
| C-20 | V-UT-1 |
| C-21 | V-UT-2, V-UT-3 |
| **C-22** | V-IT-16 |
| **C-23** | new V-UT `deprecated_aliases_delegate_correctly` |
| **C-24** | V-IT-17 |
| **C-25** | V-UT-10 |
