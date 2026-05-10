# `subagent-support` PLAN `01`

> Status: Draft
> Feature: `subagent-support`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

Iteration 01 addresses two HIGH and several MEDIUM/LOW findings from `00_REVIEW.md`. The big shift: agent install paths are reconciled with Claude's narrow `removal_root` by deriving `Layout::owned_dirs()` from the platform registry (the registry becomes the single source of truth for owned directories), and the Codex agent file format is locked to the verified Trellis prior art (TOML with `name` / `description` / `sandbox_mode` / `developer_instructions` keys, plus a `[features]` block disabling nested-agent spawning). Goal G-2 is demoted to a Constraint (Layout-A discipline). Tool-name divergence is factored into a per-platform "Dispatch" frontmatter line so prompt bodies remain truly identical. Roughly half of the "validated by code review" constraints are upgraded to string-level template tests. The `Platform` struct addition is documented as a (minor) ark-core API impact and gated by `#[non_exhaustive]` going forward.

## Log

[**Added**]

- New constraint C-1 promoted from former G-2 (Layout-A discipline; Goals are capabilities, Constraints are mechanisms).
- New constraints C-3 (Layout::owned_dirs derived from registry), C-23 (`Platform::extra_dirs` tracks any non-`removal_root` install path, including Claude agents), C-26 (agent filenames `ark-{researcher,reviewer,verifier}` are reserved; user-authored siblings with the same stem are overwritten on re-apply).
- New constraint C-25 (Codex agent TOML key set: `name`, `description`, `sandbox_mode`, `developer_instructions`, `[features].multi_agent = false`, `[features.multi_agent_v2].enabled = false`); cites `reference/Trellis/.codex/agents/trellis-research.toml` as authoritative example.
- New constraint C-27 (`Platform` is `#[non_exhaustive]`; future additions to the struct are non-breaking for the registry).
- New constraint C-28 (operator scope-violation recovery: if an agent writes outside its allowed paths, main session reverts via `git restore` before incorporating findings; `git status` after dispatch must be clean except for documented allowed paths).
- New validation rows V-UT-9 through V-UT-15 — string-level template assertions promoted from "code review".
- New validation row V-IT-5 (`unload_load_round_trips_claude_agent_files` explicitly, plus the equivalent rows for codex/opencode); V-F-2 split per platform.
- New trade-off T-9 (ark-core public-API impact / `#[non_exhaustive]`).
- New trade-off T-10 (registry-derived `owned_dirs` vs hardcoded array); explains the choice over TR-1's vec-of-trees alternative.
- Operator-side scope-violation recovery added under `## Runtime` Failure Flow.

[**Changed**]

- G-2 demoted from Goal to C-1 ("`Platform` registry exposes `agents_templates` / `agents_dest_dir`; agent install reuses the iterate-the-slice pattern.").
- G-1 reworded: "Three Ark subagents ship across all installed platforms whose subagent runtime contract has been verified" — adds the verified-contract qualifier so Codex compliance is conditional on the empirical check in Phase 1.
- C-22 reworded: prompt bodies are byte-identical across platforms after stripping per-platform frontmatter and a single `Dispatch` line; V-UT-8 updated to assert byte equality post-strip rather than whitespace-normalized.
- C-23 (now C-25) rewritten to lock the Codex TOML schema against the Trellis prior art rather than an unverified guess; cites `developer_instructions` (not `prompt`).
- C-13 reworded to specify the literal contract phrase researcher emits in its reply ("paths plus one-line summaries").
- V-F-2 split into V-F-2a/b/c per platform; V-F-3 split into V-F-3a/b/c per platform.
- Acceptance Mapping updated: 14 "code review" rows from iteration 00 reduced to 6; the rest map to V-UT-9..15 string-scan tests.
- `Architecture` block: `Layout::owned_dirs()` no longer "unchanged" — it now grows by deriving entries from `PLATFORMS[*].extra_dirs` (the new `Platform` field accommodating Claude's split-root case).

[**Removed**]

- The PLAN's claim that "agents live under existing `removal_root` for each platform" (false for Claude) — replaced by the registry-derived `owned_dirs` mechanism.
- The hand-wavy "if Codex's runtime contract differs from Trellis's, the file format adapts" hedge in Phase 1 step 4 — replaced by a concrete Codex schema citation.

[**Unresolved**]

- TR-1's "vec of trees" alternative is rejected in favor of a tighter `agents_templates` + `extra_dirs` split (rationale in T-10). Open to revisiting if a third use case for "two trees per platform" emerges.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Add `Platform::extra_dirs: &'static [&'static str]` field. `Layout::owned_dirs()` now returns a `Vec<PathBuf>` derived from `PLATFORMS` (each platform's `removal_root` plus `extra_dirs`). Claude's `extra_dirs = [".claude/agents"]`; Codex/OpenCode's `extra_dirs = []` (their `removal_root` already contains `agents/`). C-3, C-23 added; V-IT-5, V-F-2/3 split per platform. |
| Review | R-002 | Accepted | C-25 locks the Codex TOML schema to the verified Trellis prior art (`name`, `description`, `sandbox_mode`, `developer_instructions`, `[features]` with `multi_agent = false` and `multi_agent_v2.enabled = false`). Phase 1 step 4 hedge removed. The agent body lives in the `developer_instructions` triple-quoted string. |
| Review | R-003 | Accepted | Promoted 8 of the 14 "code review" rows to string-level template tests (V-UT-9 through V-UT-15 plus V-UT-1 expanded coverage). 6 rows remain "code review" — the prose-coherence ones (e.g. C-15 trigger-signal wording, C-16/17 parenthetical readability) where string-scan would only catch the structural skeleton. |
| Review | R-004 | Accepted | C-22 reworded so V-UT-8 asserts byte equality after stripping per-platform frontmatter and a single literal `Dispatch:` line; the dispatch line is the one carve-out. The agents files thus carry one platform-specific line per agent, body otherwise byte-identical. |
| Review | R-005 | Accepted | The PLAN now states explicitly: agent extraction goes through the same path as the main `templates` field — files are recorded in `manifest.files` so `is_installed` and `is_in_snapshot` work uniformly. Hash-tracking remains opt-out (C-4 unchanged), but presence-tracking is on. Phase 2 step 5 reworded accordingly. |
| Review | R-006 | Accepted | G-2 demoted to C-1. Goals list now contains 4 capability statements only. |
| Review | R-007 | Accepted | New `Failure Flow` paragraph documents operator-side scope-violation recovery (`git status` post-dispatch; `git restore` if agent wrote outside its allowed paths). New C-28 promotes this to a constraint; new manual-smoke step in Phase 6 verifies it once. |
| Review | R-008 | Accepted | C-26 promotes the V-E-4 "user-authored agent with same stem is overwritten" behavior to an explicit constraint. The slash-command updates (Phase 3) call this out once in `/ark:design`'s VERIFY-phase note so users discover it before installing custom agents. |
| Review | R-009 | Accepted | V-F-2 and V-F-3 split per platform. The acceptance mapping for G-5 now points at the platform-specific tests, not a generic "agent files" claim. |
| Review | R-010 | Accepted | State-transition bullet reworded: researcher detects no focus by inspecting `ark context`'s output; on absence, returns a textual instruction to the main session naming `ark agent task resume --slug <s>` as the recovery command. The agent does not return Rust error variants (it cannot — agents emit text only). |
| Review | R-011 | Accepted | New T-9 trade-off documents the ark-core public-API impact: `Platform` struct grows by 3 fields (`agents_templates`, `agents_dest_dir`, `extra_dirs`); the struct gains `#[non_exhaustive]` (C-27) so future additions are non-breaking; CHANGELOG entry added to both `codex-support` and `opencode-support` SPECs noting the new fields. |
| Review | TR-1 | Rejected (with documented reason) | Stays with `agents_templates` + new `extra_dirs` split rather than reshape `templates` to a vec of `(Dir, dest)` pairs. Rationale: vec-of-trees changes the most load-bearing existing field on `Platform`; the optional-second-tree path is incremental and reversible. The new `extra_dirs` field handles the round-trip side that TR-1's vec would have addressed indirectly. T-10 documents this. |
| Review | TR-2 | Accepted | T-6 stays. Added clarification: re-running the verifier outside `/ark:design` is allowed via direct subagent invocation; no new slash command. |
| Review | TR-3 | Accepted (no PLAN change needed) | Already at "checked-in" per user direction; T-5 unchanged. |

> Every prior CRITICAL / HIGH finding from `00_REVIEW.md` has been addressed (R-001 and R-002 accepted with concrete remediations).

---

## Spec

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
├── templates.rs                              (CODEX_AGENT_TEMPLATES, OPENCODE_AGENT_TEMPLATES
│                                                static Dirs; CLAUDE_TEMPLATES already covers
│                                                `templates/claude/agents/` so no Claude-specific
│                                                static is needed; tests assert agent parity,
│                                                byte-identical bodies modulo per-platform
│                                                Dispatch line, and Codex TOML schema)
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

- **Claude (`.claude/agents/<name>.md`)** — YAML frontmatter (`name`, `description`, `tools`, optional `model`), then the markdown body. Invoked via `Task(subagent_type="<name>")`. The body's `Dispatch:` line names this idiom. Files extract via the main `CLAUDE_TEMPLATES` tree (which is rooted at `templates/claude/`); no dedicated agents-templates static is needed for Claude.
- **Codex (`.codex/agents/<name>.toml`)** — TOML with `name`, `description`, `sandbox_mode = "workspace-write"`, `developer_instructions = """ … """` (the agent body verbatim), and a `[features]` block: `multi_agent = false` and `[features.multi_agent_v2] enabled = false` (disables nested-agent spawning per Trellis's `wait_agent` self-deadlock fix). Invoked via Codex's subagent tool. Authoritative example: `reference/Trellis/.codex/agents/trellis-research.toml`.
- **OpenCode (`.opencode/agents/<name>.md`)** — YAML-like frontmatter (`description`, `tools`), then the markdown body. Invoked via OpenCode's subagent tool. The body's `Dispatch:` line names this idiom.

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
    for entry in templates::walk(tree) {
        let path = layout.resolve(dest).join(entry.relative_path);
        path.write_bytes(entry.contents)?;
        manifest.record_file(path);              // record_file is infallible; takes impl Into<PathBuf>
    }
}
```

Agent files **are** registered in `manifest.files` (R-005 fix) but **not** hash-tracked: re-applied unconditionally on every `init` / `load` / `upgrade`, mirroring `extra_files` semantics for hash but `templates` semantics for presence-tracking. Sibling user-authored agents in `<dest>/` are preserved (writes are file-by-file, not directory-replace) — except those whose stem matches an Ark agent's reserved name (C-26).

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
- C-22: Every agent prompt body is byte-identical across platforms after stripping per-platform frontmatter and a single `Dispatch:` line; V-UT-8 asserts this.
- C-23: `Platform::extra_dirs` lists project-relative directories owned by the platform that lie outside `removal_root`. Claude's `extra_dirs = [".claude/agents"]`; Codex/OpenCode's `extra_dirs = []`. `Layout::owned_dirs()` concatenates these per C-3.
- C-24: `Platform::is_installed` / `is_in_snapshot` continue to work unchanged; agent files are recorded in `manifest.files` per C-5 and inherit the existing membership semantics for all three platforms.
- C-25: Codex agent file format is TOML with the keys `name`, `description`, `sandbox_mode = "workspace-write"`, `developer_instructions` (multi-line triple-quoted body), `[features].multi_agent = false`, `[features.multi_agent_v2].enabled = false`. Authoritative example: `reference/Trellis/.codex/agents/trellis-research.toml`.
- C-26: Filenames `ark-researcher`, `ark-reviewer`, `ark-verifier` (with the platform-appropriate extension) are reserved by Ark under each platform's `agents_dest_dir`. User-authored siblings with the same stem are overwritten on `init` / `upgrade` / `load`. Documented in `/ark:design`'s VERIFY note.
- C-27: `Platform` struct is `#[non_exhaustive]`; all `Platform` literals use struct-update syntax or named-field initializers in the registry.
- C-28: After dispatching any Ark subagent, main session checks `git status`. If files were written outside the agent's documented Write-ALLOWED paths, main session reverts the out-of-scope writes via `git restore` before incorporating the agent's findings.

---

## Runtime

[**Main Flow — researcher dispatch from DESIGN**]

1. Main session reads project SPECs and `specs.features` index per `/ark:design` Step 1.1.
2. After `ark agent task new --slug <s>` (Step 1.3), main session begins drafting PRD content and identifies external-research signals: a named third-party library/framework/tool, a comparison of prior-art approaches, or a need to map cross-cutting codebase patterns.
3. On signal, main session announces "this needs research on X — dispatching `ark-researcher`" and invokes the platform's subagent tool with a focused query referencing `<task>/research/`.
4. Researcher resolves task slug from `.state.toml`, ensures `.ark/tasks/<slug>/research/` exists, executes searches (Glob/Grep + WebSearch/WebFetch), persists each topic to `<topic>.md`, returns paths + one-line summaries to main session.
5. Main session runs `git status`, reverts any out-of-scope writes via `git restore`, then reads the persisted findings and incorporates them into the PRD / Brainstorm / PLAN.

[**Main Flow — reviewer dispatch from REVIEW (deep)**]

1. After PLAN authoring (Step 2.3), main session runs `ark agent task review` (Step 2.4) and refreshes phase context (Step 3.1).
2. Main session asks the user (existing workflow.md prompt): "Should I self-review, or will you run the reviewer?"
3. On agent-path: main session dispatches `ark-reviewer` with no payload (agent reads `task.toml`, latest `NN_PLAN.md`, project SPECs, related feature SPECs).
4. Reviewer fills the seeded `NN_REVIEW.md` with verdict, severity-tagged findings (`R-NNN`), trade-off advice (`TR-N`), and returns control. Main session runs `git status`, reverts any out-of-scope writes, reads the verdict, and proceeds to Step 3.3 (loop) or Step 3.4 (advance).

[**Main Flow — verifier dispatch from VERIFY**]

1. After EXECUTE (Step 4.2), main session runs `ark agent task verify` (Step 4.3); CLI seeds `VERIFY.md`.
2. Main session refreshes phase context (Step 5.1) and asks (existing prompt): "Should I self-verify, or will you run the verifier?"
3. On agent-path: main session dispatches `ark-verifier`. Agent reads seeded `VERIFY.md`, latest PLAN's Goals/Constraints, PRD Outcome, project + related feature SPECs, runs read-only commands.
4. Verifier resolves every seeded item to PASS / FAIL / N/A, adds `V-NNN` Findings for cross-cutting issues, returns control without committing or fixing anything.
5. Main session runs `git status`, reverts any out-of-scope writes, reads `VERIFY.md`. PENDING items resolved → tell user to stage and run `/ark:commit`. Open Findings → halt and ask user how to proceed.

[**Failure Flow**]

1. **Researcher cannot resolve current task.** `ark context`'s output shows no focus → researcher returns a textual instruction naming `ark agent task resume --slug <s>` as the recovery command. The agent does not return Rust error variants (it emits text only).
2. **Researcher catches a network/web tool failure.** Records the gap in the topic file under "Caveats / Not Found" and returns a partial result; main session sees explicit gap and decides whether to retry, dispatch differently, or proceed without.
3. **Reviewer cannot find the latest `NN_PLAN.md`** (e.g. wrong iteration). Reviewer returns a textual instruction naming the expected filename and the `task.toml.iteration` field; main session corrects state, retries.
4. **Verifier finds FAIL items.** Records them in `VERIFY.md`, returns control; main session fixes the underlying code (not the agent) and re-dispatches the verifier or runs `ark agent task verify` again.
5. **Agent attempts to write outside its allowed paths.** Prompt-level refusal is the first line of defense; operator-side recovery is the second. Main session runs `git status` after every agent dispatch (per C-28). Out-of-scope writes are reverted via `git restore <path>` before incorporating findings. The recursion guard and Write-FORBIDDEN sections are the contract; LLM compliance is probabilistic; the operator-side check is the enforcement.

[**State Transitions**]

- Researcher dispatched while no task focused → `ark context` shows no focus; researcher returns the recovery instruction; main session resolves by `task resume --slug <s>` or `task new`.
- Researcher dispatched but `<task>/research/` does not exist → researcher creates it via `mkdir -p` (idempotent).
- Reviewer dispatched while `task.toml.phase != Review` → reviewer reads `task.toml` and refuses textually with the current phase + expected phase; main session checks/advances phase.
- Verifier dispatched while `VERIFY.md` is unseeded → verifier refuses textually; main session runs `ark agent task verify` to seed.

---

## Implementation

[**Phase 1 — agent prompts**]

1. Author the canonical agent body for `ark-researcher` (Trellis-research-derived persist-to-files contract, Recursion Guard, Write-ALLOWED/FORBIDDEN lists, the literal contract phrase per C-14).
2. Author the canonical body for `ark-reviewer` (verdict semantics, R-NNN finding format, the C-12 / C-13 rejection rules, Recursion Guard, write scope).
3. Author the canonical body for `ark-verifier` (per-item PASS/FAIL/N/A discipline, V-NNN finding format, no-self-fix discipline per C-11, Recursion Guard, write scope).
4. Generate per-platform variants:
   - `templates/claude/agents/ark-{researcher,reviewer,verifier}.md` — YAML frontmatter (`name`, `description`, `tools`) + a single `Dispatch: Task(subagent_type="ark-<name>")` line + body.
   - `templates/codex/agents/ark-{researcher,reviewer,verifier}.toml` — TOML per C-25 (`name`, `description`, `sandbox_mode`, `developer_instructions = """ <body> """`, `[features].multi_agent = false`, `[features.multi_agent_v2].enabled = false`). The `Dispatch:` line in `developer_instructions` reads `Dispatch: <codex-subagent-token>` (verified against Trellis's example).
   - `templates/opencode/agents/ark-{researcher,reviewer,verifier}.md` — OpenCode frontmatter + `Dispatch: <opencode-subagent-token>` + body.
5. Confirm the three bodies are byte-identical across platforms after stripping per-platform frontmatter and the single `Dispatch:` line (V-UT-8).

[**Phase 2 — registry / template wiring**]

1. Add `CODEX_AGENT_TEMPLATES` and `OPENCODE_AGENT_TEMPLATES` static `include_dir!`s in `templates.rs`. (Claude does not need a dedicated static — its main `CLAUDE_TEMPLATES` tree is rooted at the platform root and already covers `agents/`.)
2. Add `CLAUDE_AGENTS_DIR`, `CODEX_AGENTS_DIR`, `OPENCODE_AGENTS_DIR` consts in `layout.rs`.
3. Extend `Platform` struct with `agents_templates`, `agents_dest_dir`, `extra_dirs`; mark `#[non_exhaustive]` (C-27).
4. Populate the three `<PLATFORM>_PLATFORM` consts with the new fields. Claude's `extra_dirs = [".claude/agents"]`. Codex's and OpenCode's `extra_dirs = []` (their `removal_root` already covers `agents/`).
5. Extend `Layout::owned_dirs()` to derive entries from `PLATFORMS` (return `Vec<PathBuf>` rather than `[PathBuf; 4]`); update call sites in `unload`/`load` (no behavioral change since they iterate).
6. Extend `Platform::apply_managed_state` to extract `agents_templates` under `agents_dest_dir` (file-by-file `walk + write_bytes`; record into `manifest.files` per R-005).
7. Re-export the three new statics from `lib.rs`.

[**Phase 3 — slash-command updates**]

1. Edit `templates/claude/commands/ark/design.md`:
   - Step 1.2 brainstorm: paragraph naming researcher trigger signals + announce-then-dispatch rule.
   - Step 1.4 PRD: cross-reference researcher findings under `<task>/research/`.
   - Step 2.3 PLAN: same trigger-signals paragraph, scoped to architectural/data-structure decisions.
   - Step 3.2 REVIEW: keep existing prompt; on agent-path name `ark-reviewer` as the dispatch target with platform parenthetical.
   - Step 5.2 VERIFY: same shape as Step 3.2 but for `ark-verifier`. Add a one-line note about the C-26 reserved-stem behavior so users know not to install custom agents under those names.
2. Mirror body changes into `templates/codex/skills/ark-design/SKILL.md` (Codex frontmatter; slash-token rewrites preserved).
3. Mirror body changes into `templates/opencode/commands/ark/design.md` (OpenCode frontmatter preserved).
4. Sync the changed templates into the dotfile checkout copies (`.claude/`, `.codex/`, `.opencode/`) since this repo dogfoods Ark.

[**Phase 4 — SPEC supersede + minor-version note**]

1. Append a `[**CHANGELOG**]` entry to `.ark/specs/features/codex-support/SPEC.md`: NG-2 superseded; `Platform` struct grows by `agents_templates`, `agents_dest_dir`, `extra_dirs`; struct now `#[non_exhaustive]`; cite this task's slug and date.
2. Append a `[**CHANGELOG**]` entry to `.ark/specs/features/opencode-support/SPEC.md`: same struct-shape note; `OPENCODE_PLATFORM.agents_templates` populated, `extra_dirs = []`.
3. Bump `ark-core` minor version (additive struct fields → minor; deprecated wrappers from codex-support's 0.2.0 → 0.3.0 still pending). Document the bump in the same CHANGELOG entries.

[**Phase 5 — tests**]

1. Existing parity tests grow: `every_claude_command_has_a_codex_skill_sibling` etc. unchanged; new `every_claude_agent_has_codex_and_opencode_siblings` parity test in `templates.rs::tests`.
2. `each_platform_ships_three_agents` (V-UT-1).
3. `agent_prompts_carry_recursion_guard` (V-UT-2).
4. `agent_prompts_carry_write_scope_walls` (V-UT-3).
5. `agents_dest_dir_consistency` in `platforms.rs::tests` (V-UT-4).
6. `claude_agent_frontmatter_shape` (V-UT-5), `codex_agent_frontmatter_shape` (V-UT-6 — TOML parse + key set per C-25), `opencode_agent_frontmatter_shape` (V-UT-7).
7. `agent_bodies_are_byte_identical_modulo_dispatch_line` (V-UT-8 — strip frontmatter + `Dispatch:` line, assert byte equality).
8. `researcher_prompt_carries_paths_summaries_contract` (V-UT-9 — string scan for the C-14 contract phrase).
9. `reviewer_prompt_carries_iteration_rejection_rule` (V-UT-10 — string scan for "references prior iterations" + HIGH).
10. `reviewer_prompt_carries_spec_contradiction_rule` (V-UT-11 — string scan for "contradicts an existing feature SPEC" + CRITICAL).
11. `verifier_prompt_carries_no_self_fix_rule` (V-UT-12 — string scan for "does not self-fix" / equivalent).
12. `design_md_names_three_agents` (V-UT-13 — `templates/claude/commands/ark/design.md` body contains literal `ark-researcher`, `ark-reviewer`, `ark-verifier` references in the right step headers).
13. `codex_support_spec_changelog_present` (V-UT-14 — string scan for date marker post NG-2 supersede).
14. `opencode_support_spec_changelog_present` (V-UT-15).
15. `apply_managed_state_writes_agent_files_and_records_them` (V-IT-1 — temp Layout; assert files written + recorded in `manifest.files`).
16. `init_installs_agents_for_selected_platforms` (V-IT-2).
17. `upgrade_re_applies_modified_agents` (V-IT-3).
18. `init_skips_agents_for_unselected_platforms` (V-IT-4).
19. `unload_load_round_trips_claude_agent_files` (V-IT-5a — explicitly Claude per R-009).
20. `unload_load_round_trips_codex_agent_files` (V-IT-5b).
21. `unload_load_round_trips_opencode_agent_files` (V-IT-5c).
22. Verify existing source-scan literal-bans extended with `.claude/agents/`, `.codex/agents/`, `.opencode/agents/` (no regression).

[**Phase 6 — manual smoke**]

1. `cargo build && cargo test` clean.
2. Run `ark init --claude --codex --opencode` against a scratch dir; verify the nine agent files land per platform and appear in the manifest.
3. Run `ark upgrade` against the same scratch dir after touching one agent; verify it gets re-applied.
4. Run a real `/ark:design --deep` flow on a throwaway slug, dispatch researcher once, verify a `<task>/research/<topic>.md` lands and `git status` shows nothing under unauthorized paths.
5. Run `ark unload && ark load` on the scratch dir; verify all nine agent files round-trip byte-identical (R-001 fix verification).

---

## Trade-offs

- T-1: **One generalist researcher vs. split codebase/web agents.** Chose generalist. Adv: simpler prompt, single dispatch target, fewer files. Disadv: prompt is longer; tool surface wider. Mitigation: Write-FORBIDDEN list explicit, persist-to-files contract identical regardless. Re-split if prompt bloats.
- T-2: **Default-dispatch vs ask-user for REVIEW/VERIFY.** Kept ask-user (per workflow.md and user direction). Adv: matches existing prompt, lets users self-review on small deep tasks. Disadv: friction; bounded to one Y/N per phase.
- T-3: **Per-platform native file format vs single canonical with adapter.** Each platform ships native (Claude `.md` + YAML, Codex `.toml`, OpenCode `.md` + YAML). Adv: zero translation at install; agents look native. Disadv: three files per agent. Mitigation: parity tests + identical body content.
- T-4: **Agent install via new `agents_templates` field vs `extra_files`.** Chose new fields. Adv: declarative directory tree, parallel to `templates`. Disadv: `Platform` struct grows. `extra_files` would force enumerating every agent in the registry (doesn't scale).
- T-5: **Researcher findings checked into git vs gitignored.** Checked-in (per user direction). Adv: archives with task. Disadv: noise during active research. Acceptable.
- T-6: **No new slash commands (`/ark:review`, `/ark:verify`).** Kept dispatch in `design.md` only. Adv: single source of truth; no duplication. Disadv: re-running the verifier after fixes requires re-dispatch from inside `/ark:design`'s VERIFY phase. Acceptable: user can re-invoke the verifier directly via the platform's subagent tool without going through the slash command.
- T-7: **EXECUTE has no agent.** No agent. Adv: main session retains user-intent context. Disadv: long EXECUTE phases blow main-session context budget. Mitigation: researcher is callable mid-EXECUTE.
- T-8: **codex-support SPEC supersede via CHANGELOG vs NG-2 rewrite.** Chose CHANGELOG. Adv: preserves history. Disadv: SPEC body carries a contradicted line. Acceptable: CHANGELOG is the authoritative latest.
- T-9: **ark-core public-API impact (`Platform` struct shape change).** Chose to mark `Platform` as `#[non_exhaustive]` (C-27) and bump the minor version. Adv: future struct-field additions are non-breaking *for downstream consumers* of `Platform`. Disadv: `#[non_exhaustive]` does not protect *intra-crate* literals — the three `<PLATFORM>_PLATFORM` consts in `platforms.rs` and any test fixtures still need every field populated on every addition. `#[non_exhaustive]` also forces struct-update syntax on external consumers; downstream test code using full positional/named literals must update once. Acceptable: ark-core is currently single-consumer (the ark-cli binary); the constraint is a forward guard. A `Default for Platform` impl is the next step if intra-crate literals grow further.
- T-10: **`Platform::extra_dirs` field vs reshape `templates` to a vec of `(Dir, dest)` pairs (TR-1).** Chose new `extra_dirs` field. Adv: incremental, reversible, parallel to `extra_files`'s precedent. Disadv: vec-of-trees would have folded extra dirs into the existing template extraction with no separate field — cleaner if we expected a third use case for "two trees per platform". Re-evaluate if a third such case emerges.

---

## Validation

[**Unit Tests**]

- V-UT-1: `each_platform_ships_three_agents` — walk each `<PLATFORM>_AGENT_TEMPLATES`; assert top-level entries are exactly `{ark-researcher, ark-reviewer, ark-verifier}` with the correct file extension.
- V-UT-2: `agent_prompts_carry_recursion_guard` — every embedded agent body contains the literal `## Recursion Guard` header.
- V-UT-3: `agent_prompts_carry_write_scope_walls` — every embedded agent body contains both `Write ALLOWED` and `Write FORBIDDEN` headers.
- V-UT-4: `agents_dest_dir_consistency` — for every `Platform` in `PLATFORMS`, `agents_templates.is_some() == agents_dest_dir.is_some()`.
- V-UT-5: `claude_agent_frontmatter_shape` — every Claude agent file starts with `---\nname: ark-` followed by `description:` and `tools:`.
- V-UT-6: `codex_agent_frontmatter_shape` — every Codex agent file is parseable TOML with `name`, `description`, `sandbox_mode`, `developer_instructions` keys present and a `[features]` block with `multi_agent = false` and `[features.multi_agent_v2].enabled = false`.
- V-UT-7: `opencode_agent_frontmatter_shape` — every OpenCode agent file starts with `---\ndescription:` and contains a `tools:` line.
- V-UT-8: `agent_bodies_are_byte_identical_modulo_dispatch_line` — extract the prompt body from each platform's `ark-<role>` (researcher/reviewer/verifier separately), strip per-platform frontmatter and the single `Dispatch:` line, assert byte equality across platforms per role.
- V-UT-9: `researcher_prompt_carries_paths_summaries_contract` — string scan for the literal phrase "paths plus one-line summaries".
- V-UT-10: `reviewer_prompt_carries_iteration_rejection_rule` — string scan for "references prior iterations" + the literal severity tag `HIGH`.
- V-UT-11: `reviewer_prompt_carries_spec_contradiction_rule` — string scan for "contradicts an existing feature SPEC" + `CRITICAL`.
- V-UT-12: `verifier_prompt_carries_no_self_fix_rule` — string scan for the literal "does not self-fix" or "no self-fix".
- V-UT-13: `design_md_names_three_agents` — `templates/claude/commands/ark/design.md` body contains literal references to `ark-researcher`, `ark-reviewer`, `ark-verifier` in the right step headers (Step 1.2/1.4 mention researcher; Step 3.2 mentions reviewer; Step 5.2 mentions verifier).
- V-UT-14: `codex_support_spec_changelog_present` — string scan of `codex-support/SPEC.md` for a CHANGELOG entry naming `subagent-support` post NG-2.
- V-UT-15: `opencode_support_spec_changelog_present` — same for `opencode-support/SPEC.md`.
- V-UT-16: `owned_dirs_derives_from_registry` — assert `Layout::owned_dirs()` equals `{ark_dir} ∪ {p.removal_root for p in PLATFORMS} ∪ {extra for p in PLATFORMS for extra in p.extra_dirs}` resolved against `layout.root()`. Catches a regression where a maintainer hard-codes the entries instead of deriving them.

[**Integration Tests**]

- V-IT-1: `apply_managed_state_writes_agent_files_and_records_them` — temp `Layout`; assert each platform's three agents land under the correct `agents_dest_dir` AND appear in `manifest.files`.
- V-IT-2: `init_installs_agents_for_selected_platforms` — `ark init --claude --codex --opencode`; verify nine agent files exist + manifest entries.
- V-IT-3: `upgrade_re_applies_modified_agents` — modify a checked-in agent, run `ark upgrade`, assert the file is restored to canonical body.
- V-IT-4: `init_skips_agents_for_unselected_platforms` — `ark init --claude --no-codex --no-opencode`; assert only `.claude/agents/` exists.
- V-IT-5a: `unload_load_round_trips_claude_agent_files` — explicit Claude agent path round-trip (R-001 fix verification).
- V-IT-5b: `unload_load_round_trips_codex_agent_files`.
- V-IT-5c: `unload_load_round_trips_opencode_agent_files`.

[**Failure / Robustness**]

- V-F-1: `agents_install_content_idempotent` — call `apply_managed_state` twice; assert post-second-call file contents are byte-identical to post-first-call contents (mtime is allowed to change since re-application is content-idempotent, not mtime-idempotent per C-5).
- V-F-2a: `unload_captures_claude_agent_files_in_snapshot`.
- V-F-2b: `unload_captures_codex_agent_files_in_snapshot`.
- V-F-2c: `unload_captures_opencode_agent_files_in_snapshot`.
- V-F-3a: `remove_drops_claude_agent_files_with_extra_dirs` — `ark remove`; assert `.claude/agents/` cleared alongside Claude's `removal_root`.
- V-F-3b: `remove_drops_codex_agent_files_with_dest_dir`.
- V-F-3c: `remove_drops_opencode_agent_files_with_dest_dir`.
- V-F-4: `user_authored_agent_in_dest_dir_preserved` — pre-create `.claude/agents/my-custom.md` (different stem from Ark's reserved set); run `ark init`; assert user file survives and three Ark agents land alongside it.

[**Edge Cases**]

- V-E-1: `agents_dest_dir_with_existing_subdir_tree` — agent template tree containing nested subdirs (defensively); `walk` produces correct flat layout under `agents_dest_dir`.
- V-E-2: `unicode_in_agent_body` — agent prompt includes non-ASCII bytes; round-trips without loss.
- V-E-3: `simultaneous_init_no_corruption` — two `ark init` invocations don't corrupt agent files.
- V-E-4: `ark_upgrade_overwrites_user_agent_with_reserved_stem` — user creates `.claude/agents/ark-researcher.md` by hand; `ark upgrade` overwrites with the canonical body. Documented behavior per C-26.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-IT-2 |
| G-2 | V-UT-13 + design.md prose review (the trigger-signal wording is judgment-call territory) |
| G-3 | V-UT-3, V-UT-2 |
| G-4 | manual smoke (Phase 6 step 4) + V-F-2a (Claude agent files round-trip; the `<task>/research/` directory lives under `.ark/tasks/<slug>/`, in `owned_dirs` already) |
| C-1 | V-UT-4 + Platform shape inspection at code review |
| C-2 | V-UT-1 |
| C-3 | V-UT-16 (derivation invariant) + V-IT-5a/b/c (round-trip per platform) |
| C-4 | extended source-scan literal-ban tests for `.claude/agents/`, `.codex/agents/`, `.opencode/agents/` |
| C-5 | V-IT-1 (manifest.files registration), V-IT-3 (re-apply), V-F-1 (idempotency), V-F-4 (sibling preservation) |
| C-6 | V-UT-2 |
| C-7 | V-UT-3 |
| C-8 / C-9 / C-10 | V-UT-3 + agent body inspection at code review (paths are file-specific) |
| C-11 | V-UT-12 |
| C-12 | V-UT-10 |
| C-13 | V-UT-11 |
| C-14 | V-UT-9 |
| C-15 | V-UT-2 |
| C-16 / C-17 / C-18 | V-UT-13 + design.md prose review |
| C-19 | V-UT-14 |
| C-20 | V-UT-15 |
| C-21 | V-UT-1 |
| C-22 | V-UT-8 |
| C-23 | V-IT-5a (Claude-specific round-trip — exercises `extra_dirs`) |
| C-24 | V-IT-1 (manifest.files) + existing `is_installed` / `is_in_snapshot` tests (no regression) |
| C-25 | V-UT-6 |
| C-26 | V-E-4 |
| C-27 | `Platform` struct attribute inspection at code review (no string scan can reliably catch attribute changes) |
| C-28 | manual smoke (Phase 6 step 4 — `git status` post-dispatch is part of the smoke check) |
