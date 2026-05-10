# `subagent-support` PLAN `00`

> Status: Draft
> Feature: `subagent-support`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none

---

## Summary

Ship three Ark-native subagents — `ark-researcher`, `ark-reviewer`, `ark-verifier` — across all three platforms (Claude Code, Codex, OpenCode), and wire dispatch instructions into the existing `/ark:design` slash command for the DESIGN/PLAN brainstorm (researcher) and the REVIEW/VERIFY gates (reviewer/verifier). Agents are pure prompt artifacts: they extend the platform-template trees, install via `ark init` / `ark upgrade`, and require no new CLI surface or context-schema changes. The `Platform` registry gains an optional `agents_templates` / `agents_dest_dir` pair so agent installation reuses the same iterate-the-slice pattern the existing platforms use for their command/skill trees.

## Log `None in 00_PLAN`

---

## Spec

[**Goals**]

- G-1: Three Ark subagents (`ark-researcher`, `ark-reviewer`, `ark-verifier`) ship across all installed platforms.
- G-2: `Platform` registry gains optional `agents_templates` + `agents_dest_dir` so agent install reuses the iterate-the-slice pattern.
- G-3: `/ark:design` documents when and how the main session dispatches each agent across DESIGN/PLAN/REVIEW/VERIFY.
- G-4: Each agent enforces a tight scope wall via prompt: write-allowed paths and write-forbidden paths are explicit.
- G-5: Researcher findings persist to `.ark/tasks/<slug>/research/<topic>.md`; the directory is checked into git and archives with the task.

[**Non-goals**]

- NG-1: No new CLI verbs (`ark agent task <verb>`, `ark archive`, `ark context` unchanged).
- NG-2: No `ark context` schema changes; agents call existing `--scope phase --for <phase>` projections.
- NG-3: No automatic dispatch from CLI or from slash command; main session decides per phase per workflow.md's existing prompts.
- NG-4: No agent for EXECUTE; main session retains full context there.
- NG-5: No reviewer/verifier "self-fix" mode — gates are read-only audit roles.
- NG-6: No structured-findings JSON; markdown files in `<task>/research/` and the seeded `NN_REVIEW.md` / `VERIFY.md` are the sole output contracts.

[**Architecture**]

```
crates/ark-core/src/
├── platforms.rs                              (Platform gains agents_templates + agents_dest_dir;
│                                                each PLATFORM const populates them; install loop
│                                                walks the agent tree alongside templates)
├── templates.rs                              (CLAUDE_AGENT_TEMPLATES, CODEX_AGENT_TEMPLATES,
│                                                OPENCODE_AGENT_TEMPLATES static Dirs;
│                                                tests assert agent parity across platforms)
└── layout.rs                                 (CLAUDE_AGENTS_DIR, CODEX_AGENTS_DIR,
                                                  OPENCODE_AGENTS_DIR consts;
                                                  owned_dirs unchanged — agents live under existing
                                                  removal_root for each platform)
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
├── codex-support/SPEC.md                     (CHANGELOG entry; NG-2 superseded)
└── opencode-support/SPEC.md                  (CHANGELOG entry; agent tree added)
```

Module coupling: `Platform::apply_managed_state` already iterates `templates` + `extra_files`. The new `agents_templates` extracts under `agents_dest_dir` via the same `walk + write_bytes` loop used for the main `templates` field — no new dependencies, no cross-module wiring.

[**Data Structure**]

```rust
// ark-core/src/platforms.rs (additions only — existing fields unchanged)
pub struct Platform {
    // ... existing fields ...
    /// Optional embedded agents-template tree, extracted under [`agents_dest_dir`].
    /// `None` for platforms that do not support subagents.
    pub agents_templates: Option<&'static Dir<'static>>,
    /// Project-relative directory where [`agents_templates`] extracts.
    /// `None` iff `agents_templates` is `None`.
    pub agents_dest_dir: Option<&'static str>,
}

// ark-core/src/templates.rs (additions)
pub static CLAUDE_AGENT_TEMPLATES:   Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/claude/agents");
pub static CODEX_AGENT_TEMPLATES:    Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/codex/agents");
pub static OPENCODE_AGENT_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/opencode/agents");

// ark-core/src/layout.rs (additions)
pub const CLAUDE_AGENTS_DIR:   &str = ".claude/agents";
pub const CODEX_AGENTS_DIR:    &str = ".codex/agents";
pub const OPENCODE_AGENTS_DIR: &str = ".opencode/agents";
```

Per-platform agent file contract:

- **Claude (`.claude/agents/<name>.md`)** — YAML frontmatter (`name`, `description`, `tools`, `model` optional), markdown body. Invoked via `Task(subagent_type="<name>")`.
- **Codex (`.codex/agents/<name>.toml`)** — TOML with `name`, `description`, `tools`, `prompt` keys per Trellis precedent. Invoked via Codex's subagent tool (verified at execute time; if Codex's runtime contract differs from Trellis's, the file format adapts but the agent prompt body stays unchanged).
- **OpenCode (`.opencode/agents/<name>.md`)** — frontmatter (`description`, `tools`) + markdown body. Invoked via OpenCode's subagent tool.

Body content (the prompt) is mechanically translatable across platforms; only the frontmatter format differs. Each agent ships the same prompt body across all three platforms — modulo platform-specific tool-name spellings (e.g. Claude's `Task`, OpenCode's `task`).

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
        layout.resolve(dest).join(entry.relative_path).write_bytes(entry.contents)?;
    }
}
```

Agent files are **not hash-tracked**; they are re-applied unconditionally on every `init` / `load` / `upgrade`, mirroring `extra_files` semantics. Sibling user-authored agents in `<dest>/` are preserved (writes are file-by-file, not directory-replace).

No CLI surface change. No new `ark agent` verbs.

[**Constraints**]

- C-1: `Platform::agents_templates` and `agents_dest_dir` are both `Some` or both `None`; the registry-shape test rejects half-set entries.
- C-2: Each platform ships exactly the three agents `ark-researcher`, `ark-reviewer`, `ark-verifier`; parity tested against `CLAUDE_AGENT_TEMPLATES` as canonical.
- C-3: Agent file paths route through `Layout` getters or `<PLATFORM>_AGENTS_DIR` consts; no `".claude/agents/"`, `".codex/agents/"`, `".opencode/agents/"` literal outside `layout.rs` and `templates.rs`.
- C-4: Agent files are re-applied unconditionally on `init` / `load` / `upgrade`; not hash-tracked. User-authored siblings under `<agents_dest_dir>/` are preserved.
- C-5: Each agent prompt body contains a "Recursion Guard" section forbidding the agent from spawning `ark-researcher`, `ark-reviewer`, or `ark-verifier`.
- C-6: Each agent prompt enumerates explicit "Write ALLOWED" and "Write FORBIDDEN" path lists; no other paths may be written.
- C-7: `ark-researcher` Write ALLOWED is `.ark/tasks/<slug>/research/*.md`. Write FORBIDDEN includes code, SPECs, PRD, PLAN, REVIEW, VERIFY, task.toml, all git operations.
- C-8: `ark-reviewer` Write ALLOWED is the seeded `NN_REVIEW.md` for the current iteration. Write FORBIDDEN includes the latest `NN_PLAN.md`, code, SPECs, all git operations.
- C-9: `ark-verifier` Write ALLOWED is `VERIFY.md`. Write FORBIDDEN includes code, SPECs, PLAN, all git operations.
- C-10: `ark-verifier` does not self-fix findings; FAIL items return to the main session for resolution.
- C-11: `ark-reviewer` rejects as HIGH a PLAN whose `## Spec` section references prior iterations rather than restating in full (mirrors workflow.md gate).
- C-12: `ark-reviewer` flags as CRITICAL any PLAN that contradicts an existing feature SPEC without an explicit `## Log` Removed/Changed entry naming the supersede.
- C-13: `ark-researcher` returns to the main session a list of `<task>/research/*.md` paths plus one-line summaries; never the full content. The persist-to-files contract is enforced by prompt.
- C-14: Recursive subagent spawning is forbidden by every agent's Recursion Guard section; only the main session may dispatch the three Ark subagents.
- C-15: `/ark:design` Step 1.2 / 1.4 (DESIGN brainstorm) names trigger signals for researcher dispatch (named third-party library, prior-art comparison, codebase pattern map) and instructs the main session to *announce-then-dispatch* when signals fire.
- C-16: `/ark:design` Step 3.2 (REVIEW, deep) keeps workflow.md's existing "self-review or run the reviewer?" prompt and, on the agent path, names `ark-reviewer` as the dispatch target.
- C-17: `/ark:design` Step 5.2 (VERIFY, standard + deep) keeps the same self-or-agent prompt and names `ark-verifier` on the agent path.
- C-18: `codex-support` SPEC's NG-2 ("No `.codex/agents/*.toml` custom subagents") is superseded; an explicit CHANGELOG entry records the supersede on the SPEC.
- C-19: `opencode-support` SPEC's `OPENCODE_PLATFORM` shape gains the `agents_templates` / `agents_dest_dir` fields; CHANGELOG entry records the addition.
- C-20: `codex-support` SPEC's `CODEX_PLATFORM` shape gains the `agents_templates` / `agents_dest_dir` fields; CHANGELOG entry records the addition.
- C-21: A `templates::walk` of each `<PLATFORM>_AGENT_TEMPLATES` yields exactly three top-level `.md` (Claude/OpenCode) or `.toml` (Codex) files; parity tested.
- C-22: Every agent prompt body is identical across platforms (modulo per-platform frontmatter and tool-name spellings); no platform-specific behavior in the prompt content.
- C-23: Codex agent file format is `[<key>] = <value>` TOML with at least the keys `name`, `description`, `tools`, `prompt`; the `prompt` value carries the markdown body verbatim.
- C-24: `Platform::is_installed` / `is_in_snapshot` continue to work unchanged; agent files appear under each platform's `dest_dir` prefix and inherit the existing membership semantics.
- C-25: Agent dispatch idiom is platform-specific (Claude `Task(subagent_type=…)`, Codex/OpenCode equivalent); slash-command instructions abstract the dispatch as "dispatch `ark-<name>`" without naming a specific tool, then provide a per-platform parenthetical.

---

## Runtime

[**Main Flow — researcher dispatch from DESIGN]**

1. Main session reads project SPECs and `specs.features` index per `/ark:design` Step 1.1.
2. After `ark agent task new --slug <s>` (Step 1.3), main session begins drafting PRD content and identifies external-research signals: a named third-party library/framework/tool, a comparison of prior-art approaches, or a need to map cross-cutting codebase patterns.
3. On signal, main session announces "this needs research on X — dispatching `ark-researcher`" and invokes the platform's subagent tool with a focused query referencing `<task>/research/`.
4. Researcher resolves task slug from `.state.toml`, ensures `.ark/tasks/<slug>/research/` exists, executes searches (Glob/Grep + WebSearch/WebFetch), persists each topic to `<topic>.md`, returns paths + one-line summaries to main session.
5. Main session reads the persisted findings and incorporates them into the PRD / Brainstorm / PLAN as appropriate.

[**Main Flow — reviewer dispatch from REVIEW (deep)**]

1. After PLAN authoring (Step 2.3), main session runs `ark agent task review` (Step 2.4) and refreshes phase context (Step 3.1).
2. Main session asks the user (existing workflow.md prompt): "Should I self-review, or will you run the reviewer?"
3. On agent-path: main session dispatches `ark-reviewer` with no payload (agent reads `task.toml`, latest `NN_PLAN.md`, project SPECs, related feature SPECs).
4. Reviewer fills the seeded `NN_REVIEW.md` with verdict, severity-tagged findings (`R-NNN`), trade-off advice (`TR-N`), and returns control. Main session reads the verdict and proceeds to Step 3.3 (loop) or Step 3.4 (advance).

[**Main Flow — verifier dispatch from VERIFY**]

1. After EXECUTE (Step 4.2), main session runs `ark agent task verify` (Step 4.3); CLI seeds `VERIFY.md`.
2. Main session refreshes phase context (Step 5.1) and asks (existing prompt): "Should I self-verify, or will you run the verifier?"
3. On agent-path: main session dispatches `ark-verifier`. Agent reads seeded `VERIFY.md`, latest PLAN's Goals/Constraints, PRD Outcome, project + related feature SPECs, runs read-only commands (`git diff`, `cargo test`, `cargo clippy`, `ark context`).
4. Verifier resolves every seeded item to PASS / FAIL / N/A, adds `V-NNN` Findings for cross-cutting issues, returns control without committing or fixing anything.
5. Main session reads `VERIFY.md`. PENDING items resolved → tell user to stage and run `/ark:commit`. Open Findings → halt and ask user how to proceed.

[**Failure Flow**]

1. Researcher cannot resolve current task (no `[focus]` in `.state.toml`) → returns error message asking main session to set focus first; main session does so and retries.
2. Researcher catches a network/web tool failure → records the gap in the topic file under "Caveats / Not Found" and returns a partial result; main session sees explicit gap and decides whether to retry, dispatch differently, or proceed without.
3. Reviewer cannot find the latest `NN_PLAN.md` (e.g. wrong iteration) → returns error; main session corrects `task.toml.iteration` or re-seeds, retries.
4. Verifier finds FAIL items → records them in `VERIFY.md`, returns control; main session fixes the underlying code (not the agent) and re-dispatches the verifier or runs `ark agent task verify` again.
5. Agent attempts to write outside its allowed paths → prompt-level refusal. The Recursion Guard and Write FORBIDDEN sections are the contract; out-of-scope writes are operator error caught at review.

[**State Transitions**]

- Researcher dispatched while no task focused → returns `NoFocus`-shaped error; main session resolves by `ark agent task resume --slug <s>` or `task new`.
- Researcher dispatched but `<task>/research/` does not exist → researcher creates it via `mkdir -p` (idempotent).
- Reviewer dispatched while `task.toml.phase != Review` → reviewer reads `task.toml` and refuses with a clear error; main session checks/advances phase.
- Verifier dispatched while `VERIFY.md` is unseeded → verifier refuses with the same shape; main session runs `ark agent task verify` to seed.

---

## Implementation

[**Phase 1 — agent prompts**]

1. Author the canonical agent body for `ark-researcher` (Trellis-research-derived, persist-to-files contract, recursion guard, Write ALLOWED/FORBIDDEN lists).
2. Author the canonical body for `ark-reviewer` (verdict semantics, R-NNN finding format, the C-11 / C-12 rejection rules, recursion guard, write scope).
3. Author the canonical body for `ark-verifier` (per-item PASS/FAIL/N/A discipline, V-NNN finding format, no-self-fix discipline, recursion guard, write scope).
4. Generate per-platform variants:
   - `templates/claude/agents/ark-{researcher,reviewer,verifier}.md` — YAML frontmatter + body.
   - `templates/codex/agents/ark-{researcher,reviewer,verifier}.toml` — TOML frontmatter; body in the `prompt` key as a multi-line literal string.
   - `templates/opencode/agents/ark-{researcher,reviewer,verifier}.md` — OpenCode frontmatter + body.
5. Confirm the three bodies are identical across platforms (single source-of-truth markdown, frontmatter shimmed per platform).

[**Phase 2 — registry / template wiring**]

1. Add `CLAUDE_AGENT_TEMPLATES`, `CODEX_AGENT_TEMPLATES`, `OPENCODE_AGENT_TEMPLATES` static `include_dir!`s in `templates.rs`.
2. Add `CLAUDE_AGENTS_DIR`, `CODEX_AGENTS_DIR`, `OPENCODE_AGENTS_DIR` consts in `layout.rs`.
3. Extend `Platform` struct with `agents_templates: Option<&'static Dir>` and `agents_dest_dir: Option<&'static str>`.
4. Populate the three `<PLATFORM>_PLATFORM` consts with the new fields.
5. Extend `Platform::apply_managed_state` to extract `agents_templates` under `agents_dest_dir` (file-by-file `walk + write_bytes`).
6. Re-export the three new statics from `lib.rs`.

[**Phase 3 — slash-command updates**]

1. Edit `templates/claude/commands/ark/design.md`:
   - Step 1.2 brainstorm: add a paragraph naming researcher trigger signals and the *announce-then-dispatch* rule.
   - Step 1.4 PRD: cross-reference researcher findings under `<task>/research/`.
   - Step 2.3 PLAN: same trigger-signals paragraph, scoped to architectural/data-structure decisions.
   - Step 3.2 REVIEW: keep the existing prompt; add a paragraph that, on agent-path, names `ark-reviewer` as the dispatch target.
   - Step 5.2 VERIFY: same shape as Step 3.2 but for `ark-verifier`.
2. Mirror the body changes into `templates/codex/skills/ark-design/SKILL.md` (Codex frontmatter and slash-token rewrites preserved).
3. Mirror the body changes into `templates/opencode/commands/ark/design.md` (OpenCode frontmatter preserved).
4. Sync the changed templates into the dotfile checkout copies (`.claude/`, `.codex/`, `.opencode/`) since this repo dogfoods Ark.

[**Phase 4 — SPEC supersede**]

1. Append a `[**CHANGELOG**]` entry to `.ark/specs/features/codex-support/SPEC.md` noting NG-2 supersede + `agents_templates` field addition; cite this task's slug.
2. Append a `[**CHANGELOG**]` entry to `.ark/specs/features/opencode-support/SPEC.md` noting `agents_templates` field addition.
3. (No CHANGELOG on `ark-context` or `worktree` — they are unchanged.)

[**Phase 5 — tests**]

1. Add `every_claude_agent_has_codex_and_opencode_siblings` parity test in `templates.rs::tests`.
2. Add `each_platform_ships_three_agents` test asserting `walk` of each `<PLATFORM>_AGENT_TEMPLATES` yields exactly the three expected stems.
3. Add `agent_prompts_carry_recursion_guard` test asserting every embedded agent body contains the literal "Recursion Guard" header.
4. Add `agents_dest_dir_consistency` test in `platforms.rs::tests` asserting `agents_templates` and `agents_dest_dir` are both `Some` or both `None` per platform.
5. Add `agents_install_under_dest_dir` integration-style test using a temp `Layout` to verify `apply_managed_state` writes the three agent files under each platform's `agents_dest_dir`.
6. Verify existing tests still pass: source-scan literal-bans (C-3), shape tests for `OPENCODE_PLATFORM` / `CODEX_PLATFORM`.

[**Phase 6 — manual smoke**]

1. `cargo build && cargo test` clean.
2. Run `ark init --claude --codex --opencode` against a scratch dir; verify the three agent files land per platform.
3. Run `ark upgrade` against the same scratch dir after touching one agent; verify it gets re-applied.
4. Run a real `/ark:design --deep` flow on a throwaway slug, dispatch researcher once for a known trigger ("compare TOML vs JSON for X"), verify a `<task>/research/<topic>.md` lands and main session reads it.

---

## Trade-offs

- T-1: **One generalist researcher vs. split codebase/web agents.** Chose generalist (one agent, web + repo). Adv: simpler prompt surface, single dispatch target, fewer files to maintain. Disadv: prompt is longer; tool surface is wider per agent. Mitigation: Write FORBIDDEN list is explicit; the persist-to-files contract is the same regardless. Re-split to two agents later if prompt bloats.
- T-2: **Default-dispatch vs. ask-user for REVIEW/VERIFY.** Kept ask-user (per workflow.md and user feedback). Adv: matches existing prompt and lets users self-review on small deep tasks. Disadv: friction; if the user always picks "agent", the prompt is noise. Acceptable: friction is bounded to one Y/N per phase.
- T-3: **Agent file format per platform.** Codex uses TOML, Claude/OpenCode use markdown with YAML frontmatter. Chose to ship platform-native formats rather than a single canonical format with adapters. Adv: zero translation at install time; agents look native to each platform. Disadv: three files to keep in sync per agent. Mitigation: parity tests + the prompt body itself is identical text.
- T-4: **Agent install via `agents_templates` field vs. `extra_files`.** Chose new optional fields. Adv: declarative directory tree with arbitrary file count, parallel to existing `templates`. Disadv: `Platform` struct grows by two fields. `extra_files` would force enumerating every agent in the registry, which doesn't scale. Re-evaluate if a platform needs both subagents and unrelated extras.
- T-5: **Researcher findings checked into git vs. gitignored.** Chose checked-in (per user direction). Adv: archives with task; future readers see the research record. Disadv: noise in `git status` during active research. Acceptable: single `research/` dir under the task.
- T-6: **No new slash commands (`/ark:review`, `/ark:verify`).** Chose to keep dispatch in `design.md` only. Adv: single source of truth for the workflow; no duplication. Disadv: re-running the verifier after fixes requires repeating the dispatch from inside `/ark:design`'s VERIFY phase. Acceptable: the user can re-invoke the verifier directly via the platform's subagent tool without going through the slash command.
- T-7: **EXECUTE has no agent.** Chose not to ship one. Adv: main session retains user-intent context; no information loss across the dispatch boundary. Disadv: long EXECUTE phases blow main-session context budget. Mitigation: researcher is callable mid-EXECUTE for codebase-mapping queries.
- T-8: **codex-support SPEC supersede via CHANGELOG vs. NG-2 rewrite.** Chose CHANGELOG entry. Adv: preserves historical record; future readers see why the non-goal flipped. Disadv: SPEC carries a contradicted line in the body. Acceptable: the CHANGELOG entry is the authoritative latest.

---

## Validation

[**Unit Tests**]

- V-UT-1: `each_platform_ships_three_agents` — walk each `<PLATFORM>_AGENT_TEMPLATES`; assert top-level entries are exactly `{ark-researcher, ark-reviewer, ark-verifier}` with the correct file extension.
- V-UT-2: `agent_prompts_carry_recursion_guard` — every embedded agent body contains the literal `## Recursion Guard` header.
- V-UT-3: `agent_prompts_carry_write_scope_walls` — every embedded agent body contains both `Write ALLOWED` and `Write FORBIDDEN` headers.
- V-UT-4: `agents_dest_dir_consistency` — for every `Platform` in `PLATFORMS`, `agents_templates.is_some() == agents_dest_dir.is_some()`.
- V-UT-5: `claude_agent_frontmatter_shape` — every Claude agent file starts with `---\nname: ark-` followed by `description:` and `tools:` keys.
- V-UT-6: `codex_agent_frontmatter_shape` — every Codex agent file is parseable TOML with `name`, `description`, `tools`, `prompt` keys present.
- V-UT-7: `opencode_agent_frontmatter_shape` — every OpenCode agent file starts with `---\ndescription:` and contains a `tools:` line.
- V-UT-8: `agent_bodies_are_identical_across_platforms` — extract the prompt body from each platform's `ark-researcher` (similarly for reviewer, verifier); assert string equality after normalizing whitespace.

[**Integration Tests**]

- V-IT-1: `apply_managed_state_writes_agent_files` — run `Platform::apply_managed_state` against a temp `Layout`; assert each platform's three agents land under the correct `agents_dest_dir`.
- V-IT-2: `init_installs_agents_for_selected_platforms` — `ark init --claude --codex --opencode` in a temp project; verify all nine agent files exist.
- V-IT-3: `upgrade_re_applies_modified_agents` — modify a checked-in agent, run `ark upgrade`, assert the file is restored to the embedded canonical body.
- V-IT-4: `init_skips_agents_for_unselected_platforms` — `ark init --claude --no-codex --no-opencode`; assert only `.claude/agents/` exists.

[**Failure / Robustness**]

- V-F-1: `agents_install_idempotent` — call `apply_managed_state` twice; assert second call is a no-op for already-extracted files (file content equal pre/post).
- V-F-2: `unload_captures_agent_files_in_snapshot` — `ark unload`; assert agent files appear in `Snapshot::files`; `ark load` restores them byte-identical.
- V-F-3: `remove_drops_agent_files_with_dest_dir` — `ark remove`; assert `.claude/agents/`, `.codex/agents/`, `.opencode/agents/` are gone alongside the rest of each platform's `removal_root`.
- V-F-4: `user_authored_agent_in_dest_dir_preserved` — pre-create a `.claude/agents/my-agent.md` not owned by Ark; run `ark init`; assert the user file survives and the three Ark agents land alongside it.

[**Edge Cases**]

- V-E-1: `agents_dest_dir_with_existing_subdir_tree` — agent template tree containing nested subdirs (defensively); `walk` produces correct flat layout under `agents_dest_dir`.
- V-E-2: `unicode_in_agent_body` — agent prompt includes non-ASCII bytes (rare but legal); `include_dir!` and `write_bytes` round-trip without loss.
- V-E-3: `simultaneous_init_no_corruption` — two `ark init` invocations don't corrupt agent files (same byte stream both times; last write wins safely).
- V-E-4: `ark_upgrade_when_user_added_agent_with_same_name` — user creates `.claude/agents/ark-researcher.md` by hand; `ark upgrade` overwrites with the canonical body. Documented behavior; user must rename their override.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-IT-2 |
| G-2 | V-UT-4, V-IT-1, V-F-1 |
| G-3 | manual smoke (Phase 6 step 4) — `/ark:design` body changes inspected during code review |
| G-4 | V-UT-3, V-UT-2 |
| G-5 | manual smoke + V-F-2 (research/ travels with task in snapshot/restore) |
| C-1 | V-UT-4 |
| C-2 | V-UT-1 |
| C-3 | existing source-scan literal-ban tests (extended with `.claude/agents/`, `.codex/agents/`, `.opencode/agents/`) |
| C-4 | V-IT-3, V-F-1, V-F-4 |
| C-5 | V-UT-2 |
| C-6 | V-UT-3 |
| C-7 / C-8 / C-9 | V-UT-3 + agent body inspection at code review |
| C-10 | agent body inspection at code review (V-UT-3 confirms structural presence) |
| C-11 / C-12 | agent body inspection at code review |
| C-13 | agent body inspection at code review |
| C-14 | V-UT-2 |
| C-15 / C-16 / C-17 | `/ark:design` body inspection at code review |
| C-18 / C-19 / C-20 | SPEC CHANGELOG entries inspected at code review |
| C-21 | V-UT-1 |
| C-22 | V-UT-8 |
| C-23 | V-UT-6 |
| C-24 | existing `is_installed` / `is_in_snapshot` tests (no regression) |
| C-25 | `/ark:design` body inspection at code review |
