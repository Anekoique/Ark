
[**Goals**]

- G-1: Three Ark subagents (`ark-researcher`, `ark-reviewer`, `ark-verifier`) ship across every installed platform whose subagent runtime contract is verified.
- G-2: `/ark:design` documents when and how the main session dispatches each agent across DESIGN/PLAN/REVIEW/VERIFY.
- G-3: Each agent's prompt enforces a tight scope wall: explicit Write-ALLOWED and Write-FORBIDDEN path lists.
- G-4: Researcher findings persist to `.ark/tasks/<slug>/research/<topic>.md`; the directory is checked into git and archives with the task.

[**Non-goals**]

- NG-1: No new CLI verbs (`ark agent task <verb>` / `ark archive` / `ark context` unchanged).
- NG-2: No `ark context` schema changes; agents call existing `--scope phase --for <phase>` projections.
- NG-3: No automatic dispatch from CLI or from slash command; main session decides per phase per workflow.md's existing prompts.
- NG-4: No agent for EXECUTE; main session retains full context there.
- NG-5: No reviewer/verifier "self-fix" mode — gates are read-only audit roles.
- NG-6: No structured-findings JSON; markdown files in `<task>/research/` and the seeded `NN_REVIEW.md` / `VERIFY.md` are the sole output contracts.

[**Architecture**]

```
crates/ark-core/src/
├── platforms.rs                              (Platform gains agents_templates, agents_dest_dir,
│                                                extra_dirs fields and #[non_exhaustive];
│                                                each PLATFORM const populates them; install loop
│                                                walks the agent tree alongside the main templates
│                                                tree and records every file in manifest.files)
├── templates.rs                              (CLAUDE_AGENT_TEMPLATES, CODEX_AGENT_TEMPLATES,
│                                                OPENCODE_AGENT_TEMPLATES static Dirs; main extract
│                                                loop skips files whose dest falls under
│                                                `agents_dest_dir` so agents are written via
│                                                `apply_managed_state` only — preserves the C-26
│                                                reserved-stem invariant on default `WriteMode::Skip`)
└── layout.rs                                 (CLAUDE_AGENTS_DIR, CODEX_AGENTS_DIR,
                                                  OPENCODE_AGENTS_DIR consts;
                                                  Layout::owned_dirs() returns Vec<PathBuf>
                                                  derived from PLATFORMS — each platform
                                                  contributes removal_root + extra_dirs)
templates/
├── claude/agents/
│   ├── ark-researcher.md
│   ├── ark-reviewer.md
│   └── ark-verifier.md
├── codex/agents/
│   ├── ark-researcher.toml
│   ├── ark-reviewer.toml
│   └── ark-verifier.toml
├── opencode/agents/
│   ├── ark-researcher.md
│   ├── ark-reviewer.md
│   └── ark-verifier.md
├── claude/commands/ark/design.md             (dispatch instructions for all three agents)
├── codex/skills/ark-design/SKILL.md          (same body as Claude design.md, Codex frontmatter)
└── opencode/commands/ark/design.md           (same body as Claude design.md, OpenCode frontmatter)
.ark/specs/features/
├── codex-support/SPEC.md                     (CHANGELOG entry; NG-2 superseded; struct fields)
└── opencode-support/SPEC.md                  (CHANGELOG entry; agent tree added; struct fields)
```

Module coupling: `Platform::apply_managed_state` extracts `agents_templates` under `agents_dest_dir` via the same file-by-file `walk + write_bytes` loop used for the main `templates` field, recording every written file into `manifest.files` (R-005 fix). `Layout::owned_dirs()` becomes a function that iterates `PLATFORMS` and concatenates each platform's `removal_root` and `extra_dirs` (R-001 fix); call sites in `unload`/`load` are unchanged (still `for owned in layout.owned_dirs()`).

[**Data Structure**]

```rust
// ark-core/src/platforms.rs (additions)
#[non_exhaustive]                                           // C-27
#[derive(Debug, Clone, Copy)]
pub struct Platform {
    pub id: &'static str,
    pub templates: &'static Dir<'static>,
    pub dest_dir: &'static str,
    pub removal_root: &'static str,
    pub cli_flag: &'static str,
    pub managed_block_target: Option<&'static str>,
    pub hook_file: Option<HookFileSpec>,
    pub extra_files: &'static [(&'static str, &'static str)],
    /// Optional embedded agents-template tree, extracted under `agents_dest_dir`.
    /// `None` for platforms whose subagent runtime contract is not yet verified.
    pub agents_templates: Option<&'static Dir<'static>>,
    /// Project-relative directory where `agents_templates` extracts.
    /// `None` iff `agents_templates` is `None`.
    pub agents_dest_dir: Option<&'static str>,
    /// Project-relative directories owned by this platform that lie OUTSIDE
    /// `removal_root`. Concatenated by `Layout::owned_dirs()` so `unload`/`load`
    /// round-trip them; emptied for platforms whose agents (or other extras)
    /// already nest under `removal_root`. Claude's narrow `removal_root`
    /// (`.claude/commands/ark/`) requires `[".claude/agents"]` here.
    pub extra_dirs: &'static [&'static str],
}

// ark-core/src/templates.rs (additions)
//
// Claude does NOT need a dedicated agent-templates static. CLAUDE_TEMPLATES
// is rooted at `templates/claude/` (covers both `commands/` and `agents/`),
// so the agent files extract via the main loop. Only Codex and OpenCode need
// dedicated statics — their main trees are rooted at `templates/codex/skills/`
// and `templates/opencode/commands/` respectively.
pub static CODEX_AGENT_TEMPLATES:    Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/codex/agents");
pub static OPENCODE_AGENT_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/opencode/agents");

// ark-core/src/layout.rs (additions)
pub const CLAUDE_AGENTS_DIR:   &str = ".claude/agents";
pub const CODEX_AGENTS_DIR:    &str = ".codex/agents";
pub const OPENCODE_AGENTS_DIR: &str = ".opencode/agents";

impl Layout {
    /// Directories whose full contents are captured by `unload` and restored
    /// by `load`. Derived from the platform registry: each `Platform` contributes
    /// `removal_root` + `extra_dirs`. The `.ark/` root is included unconditionally.
    pub fn owned_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.ark_dir()];
        for p in PLATFORMS {
            dirs.push(self.resolve(p.removal_root));
            for extra in p.extra_dirs {
                dirs.push(self.resolve(extra));
            }
        }
        dirs
    }
}
```

Per-platform agent file contract:

- **Claude (`.claude/agents/<name>.md`)** — YAML frontmatter (`name`, `description`, `tools`, optional `model`), then the markdown body. Invoked via `Task(subagent_type="<name>")`. Files ship via the dedicated `CLAUDE_AGENT_TEMPLATES` static; the main `CLAUDE_TEMPLATES` extract loop skips files under `agents_dest_dir` so agents are written via `apply_managed_state` only.
- **Codex (`.codex/agents/<name>.toml`)** — TOML with `name`, `description`, `sandbox_mode = "workspace-write"`, `developer_instructions = """ … """` (the agent body verbatim), and a `[features]` block: `multi_agent = false` and `[features.multi_agent_v2] enabled = false` (disables nested-agent spawning per Trellis's `wait_agent` self-deadlock fix). Invoked via Codex's subagent tool.
- **OpenCode (`.opencode/agents/<name>.md`)** — YAML frontmatter (`description`, `mode: subagent`, `permission` block), then the markdown body. Invoked via OpenCode's `task` tool.

Researcher findings layout:

```
.ark/tasks/<slug>/
├── PRD.md
├── PLAN.md / NN_PLAN.md
├── NN_REVIEW.md           (deep)
├── VERIFY.md              (standard + deep)
├── task.toml
└── research/              (NEW; created on first researcher dispatch; checked into git)
    └── <topic-slug>.md
```

[**API Surface**]

```rust
// ark-core/src/lib.rs re-exports
pub use platforms::{Platform, PLATFORMS, CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM};
pub use templates::{
    CLAUDE_AGENT_TEMPLATES, CODEX_AGENT_TEMPLATES, OPENCODE_AGENT_TEMPLATES,
};
```

`Platform::apply_managed_state` body grows one block, before `extra_files`:

```rust
if let (Some(tree), Some(dest)) = (self.agents_templates, self.agents_dest_dir) {
    let dest_root = layout.resolve(dest);
    for entry in crate::templates::walk(tree) {
        let path = dest_root.join(entry.relative_path);
        path.write_bytes(entry.contents)?;
        let relative = path
            .strip_prefix(layout.root())
            .expect("agent dest under project root");
        manifest.record_file_with_hash(relative, entry.contents);
    }
}
```

Agent files are hash-tracked in `manifest.{files, hashes}` so `is_installed` and `is_in_snapshot` see them. The unconditional `path.write_bytes` in `apply_managed_state` bypasses the upgrade-conflict pipeline (agents are excluded from `collect_desired_templates` and exempted in `is_agent_path`); re-application is content-idempotent but not mtime-idempotent. Sibling user-authored agents in `<dest>/` at non-reserved stems are preserved (writes are file-by-file, not directory-replace).

No CLI surface change. No new `ark agent` verbs.

[**Constraints**]

- C-1: `Platform` registry exposes `agents_templates` + `agents_dest_dir`; agent install reuses the iterate-the-slice pattern (no new arms per platform).
- C-2: Each platform ships exactly the three agents `ark-researcher`, `ark-reviewer`, `ark-verifier`; parity tested against `CLAUDE_AGENT_TEMPLATES` as canonical.
- C-3: `Layout::owned_dirs()` is derived from `PLATFORMS` (each platform contributes `removal_root` + `extra_dirs`); call sites are unchanged.
- C-4: Agent files route through `Layout` getters or `<PLATFORM>_AGENTS_DIR` consts; no `".claude/agents/"`, `".codex/agents/"`, `".opencode/agents/"` literal outside `layout.rs` and `templates.rs`.
- C-5: Agent files are re-applied unconditionally on `init` / `load` / `upgrade` via `Platform::apply_managed_state` (which calls `path.write_bytes` regardless of `WriteMode`); they are hash-tracked in `manifest.{files,hashes}` so `is_installed` and the upgrade-conflict pipeline see them, but the unconditional write in `apply_managed_state` bypasses the conflict pipeline at install time. Re-application is content-idempotent (post-write bytes equal pre-write bytes for an unchanged template) but not mtime-idempotent — every re-apply rewrites the file.
- C-6: Each agent prompt body contains a `## Recursion Guard` section forbidding the agent from spawning `ark-researcher`, `ark-reviewer`, or `ark-verifier`.
- C-7: Each agent prompt enumerates explicit `Write ALLOWED` and `Write FORBIDDEN` lists; no other paths may be written.
- C-8: `ark-researcher` Write ALLOWED is `.ark/tasks/<slug>/research/*.md`; FORBIDDEN includes code, SPECs, PRD, PLAN, REVIEW, VERIFY, `task.toml`, all git operations.
- C-9: `ark-reviewer` Write ALLOWED is the seeded `NN_REVIEW.md` for the current iteration; FORBIDDEN includes the latest `NN_PLAN.md`, code, SPECs, all git operations.
- C-10: `ark-verifier` Write ALLOWED is `VERIFY.md`; FORBIDDEN includes code, SPECs, PLAN, all git operations.
- C-11: `ark-verifier` does not self-fix findings; FAIL items return to the main session for resolution.
- C-12: `ark-reviewer` rejects as HIGH a PLAN whose `## Spec` references prior iterations rather than restating in full.
- C-13: `ark-reviewer` flags as CRITICAL any PLAN that contradicts an existing feature SPEC without an explicit `## Log` Removed/Changed entry naming the supersede.
- C-14: `ark-researcher` returns to the main session a list of `<task>/research/*.md` paths plus one-line summaries; the literal contract phrase "paths plus one-line summaries" appears in the prompt.
- C-15: Recursive subagent spawning is forbidden by every agent's Recursion Guard section; only the main session may dispatch the three Ark subagents.
- C-16: `/ark:design` Step 1.2 / 1.4 names the trigger signals for researcher dispatch (named third-party library, prior-art comparison, codebase pattern map) and instructs main session to *announce-then-dispatch* on signal.
- C-17: `/ark:design` Step 3.2 (REVIEW, deep) keeps workflow.md's "self-review or run the reviewer?" prompt; on the agent path, names `ark-reviewer` as the dispatch target.
- C-18: `/ark:design` Step 5.2 (VERIFY, standard + deep) keeps the same self-or-agent prompt; on the agent path, names `ark-verifier`.
- C-19: `codex-support` SPEC's NG-2 ("No `.codex/agents/*.toml` custom subagents") is superseded; an explicit CHANGELOG entry records the supersede + the new `Platform` fields.
- C-20: `opencode-support` SPEC's `OPENCODE_PLATFORM` shape gains the three new `Platform` fields; CHANGELOG entry records the addition.
- C-21: Each platform ships exactly the three Ark agents. For Claude, `CLAUDE_TEMPLATES.get_dir("agents")` yields three `.md` files; for Codex / OpenCode, `walk` of `<PLATFORM>_AGENT_TEMPLATES` yields three top-level `.toml` (Codex) or `.md` (OpenCode) files. Parity tested.
- C-22: Every agent prompt body is byte-identical across platforms after stripping per-platform frontmatter; V-UT-8 asserts this.
- C-23: `Platform::extra_dirs` lists project-relative directories owned by the platform that lie outside `removal_root`. Claude's `extra_dirs = [".claude/agents"]`; Codex/OpenCode's `extra_dirs = []`. `Layout::owned_dirs()` concatenates these per C-3.
- C-24: `Platform::is_installed` / `is_in_snapshot` continue to work unchanged; agent files are recorded in `manifest.files` per C-5 and inherit the existing membership semantics for all three platforms.
- C-25: Codex agent file format is TOML with the keys `name`, `description`, `sandbox_mode = "workspace-write"`, `developer_instructions` (multi-line triple-quoted body), `[features].multi_agent = false`, `[features.multi_agent_v2].enabled = false`. Authoritative example: `reference/Trellis/.codex/agents/trellis-research.toml`.
- C-26: Filenames `ark-researcher`, `ark-reviewer`, `ark-verifier` (with the platform-appropriate extension) are reserved by Ark under each platform's `agents_dest_dir`. User-authored siblings with the same stem are overwritten on `init` / `upgrade` / `load`. Documented in `/ark:design`'s VERIFY note.
- C-27: `Platform` struct is `#[non_exhaustive]`; all `Platform` literals use struct-update syntax or named-field initializers in the registry.
- C-28: After dispatching any Ark subagent, main session checks `git status`. If files were written outside the agent's documented Write-ALLOWED paths, main session reverts the out-of-scope writes via `git restore` before incorporating the agent's findings.

---
