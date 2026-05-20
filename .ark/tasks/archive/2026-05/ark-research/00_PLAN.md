# `ark-research` PLAN `00`

> Status: Draft
> Feature: `ark-research`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`

---

## Summary

Introduce a fourth workflow tier, `research`, whose deliverable is a curated reference corpus under `.ark/tasks/<slug>/research/` rather than a code change. Add a `Tier::Research` variant and a `Phase::Research` variant with the single legal transition `Research → Committed`, scaffold `.ark/tasks/<slug>/{PRD.md, task.toml}` like every other tier (no `research/` subdirectory pre-creation; the `ark-researcher` subagent creates it on first dispatch), close research tasks via `task_commit`'s research-tier branch (skip VERIFY gate, skip SPEC extract, stage `task.toml` + `PRD.md` + `research/**`), ship `/ark:research <topic>` slash commands across Claude / Codex / OpenCode with byte-identical bodies, and document the positioning in `workflow.md`. `--worktree` is opt-in, not required; the user's flag wins. No new templates.

## Log `None in 00_PLAN`

---

## Spec

> This section is the durable design record. On deep-tier commit, it is copied verbatim into `specs/features/ark-research/SPEC.md`.

[**Goals**]

- G-1: `Tier::Research` is a first-class variant alongside Quick/Standard/Deep.
- G-2: `Research → Committed` is the only legal transition for tier=research.
- G-3: `task new --tier research` scaffolds PRD.md + task.toml; --worktree is opt-in.
- G-4: `task commit` on research tier skips VERIFY and SPEC extract; stages PRD + research/.
- G-5: `/ark:research` ships on every platform whose subagent runtime contract is verified.

[**Non-goals**]

- NG-1: No new artifact template (PRD.md is reused; no BRIEF / FINDINGS / SCOPE files).
- NG-2: No SPEC promotion: research tier never invokes `parse_spec_path` / `spec_extract` / `spec_register`.
- NG-3: No `[**Research**]` PRD block for cross-task citation in this task; PRDs may mention research slugs informally in prose.
- NG-4: No `ark context --for research` phase projection; the existing `design` projection serves research tasks.

[**Architecture**]

```
crates/ark-core/src/
├── commands/agent/
│   ├── state.rs                       (*) Tier += Research; Phase += Research;
│   │                                       can_transition += (Research, Research, Committed)
│   │                                       and (Research, Committed, Archived)
│   └── task/
│       ├── new.rs                     (*) build_task_toml: Tier::Research → Phase::Research
│       │                                   (other tiers unchanged: Phase::Design)
│       ├── phase.rs                   (*) no code change — Research tier never enters
│       │                                   Plan/Review/Execute/Verify so the artifact_for
│       │                                   match arm coverage is sufficient as-is; new
│       │                                   tests assert IllegalPhaseTransition path
│       └── commit.rs                  (*) check_phase_for_commit gains (Research, Research);
│                                           task_commit short-circuits the VERIFY gate and the
│                                           deep-tier SPEC branch for Tier::Research;
│                                           ark_files_for_first_commit stages research/**

templates/
├── claude/commands/ark/research.md    (+) new slash command
├── codex/skills/ark-research/SKILL.md (+) new skill
├── opencode/commands/ark/research.md  (+) new command
└── ark/workflow.md                    (*) tier table grows row 4; new §"Research" subsection;
                                            lifecycle diagram adds Research → Committed track
```

Module coupling: untouched. `state.rs` is the only module that grows public surface (enum variants); `commit.rs` and `new.rs` carry tier-conditional branches that mirror the existing `Tier::Deep` branches in `commit.rs`. `phase.rs` requires no edit because no Research-tier verb ever calls `transition()` legally.

Call graph for `task new --tier research --worktree?`:

```
task_new(opts)
  ├── (worktree path identical to other tiers when opts.worktree.is_some())
  ├── validate_slug, validate_title
  ├── scaffold task dir + copy_template("PRD", PRD.md)
  ├── build_task_toml: Phase::Research (not Phase::Design)
  └── state_mutate: register active + bind focus (unchanged)
```

Call graph for `task commit` on research tier:

```
task_commit(opts)
  ├── load TaskToml; check_phase_for_commit(Research, Research) → Ok
  ├── require_staged_work (unchanged; user staged PRD + research/)
  ├── pending = VerifyPendingCounts::default()    (skip parse_verify_md)
  ├── deep = false; feature_segments unused; spec_extract / spec_register skipped
  ├── workspace_record (unchanged)
  ├── save task.toml { phase = Committed, committed_at = now }
  ├── ark_files_for_first_commit returns [task.toml] + research/**
  │     ── walk research/ if present, append every file; no-op when absent
  ├── stage_files + git commit (unchanged)
  ├── guard.commit + clear_focus (unchanged)
  └── return TaskCommitSummary { tier: Research, deep_spec_promoted: false, ... }
```

Failure path: any error before `guard.commit()` triggers `RollbackGuard::Drop`, which already handles soft-reset + ark_files unstage + task.toml restore. Research tier adds no new snapshot surface (no SPEC, no INDEX).

[**Data Structure**]

```rust
// crates/ark-core/src/commands/agent/state.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Quick,
    Standard,
    Deep,
    Research,           // (+) new variant; serde tag "research"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
    Committed,
    Archived,
    Research,           // (+) new variant; serde tag "research"
}

// can_transition table additions:
//   (Research, Research, Committed) => true
//   (Research, Committed, Archived) => true
// Every other (Research, X, Y) tuple => false (default arm).
```

No `task.toml` schema additions: research tier reuses the existing fields (`id`, `title`, `tier`, `phase`, `iteration`, `created_at`, `updated_at`, `committed_at`, etc.). `iteration` is unused by research-tier code paths but persists as `0`.

No new `Error` variants. `task new --tier research` with arguments other than `--worktree` succeeds normally; misuse routes through existing `IllegalPhaseTransition` (e.g. `ark agent task plan` on a research task).

[**API Surface**]

```rust
// crates/ark-core/src/lib.rs re-exports
pub use commands::agent::{Tier, Phase, Status, TaskToml};   // unchanged signatures

// crates/ark-core/src/commands/agent/task/commit.rs
fn check_phase_for_commit(tier: Tier, phase: Phase) -> Result<()>;
//   ── adds Tier::Research + Phase::Research to the accepted matrix:
//      matches!((tier, phase),
//          (Quick, Execute) | (Standard, Verify) | (Deep, Verify) | (Research, Research))

fn ark_files_for_first_commit(
    task_dir: &Path,
    deep: bool,                     // unchanged
    spec_path: &Path,               // unused on research tier (deep=false)
    intermediate_indexes: &[PathBuf], // unused on research tier
    workspace_files: &[PathBuf],
) -> Vec<PathBuf>;
//   ── gains: when caller is research tier, append every file under
//      task_dir.join("research/") recursively (walk_files). Implementation
//      lives in a private helper `research_corpus_files(&task_dir)` that
//      returns Vec<PathBuf>; the existing function signature gains a `tier`
//      parameter so a single call site selects the right policy.
```

CLI surface: `ark agent task new --tier research` is accepted by the existing `--tier` parser (clap value-enum gains `research`). No new top-level subcommand.

Slash-command file shape (Claude flavor; Codex / OpenCode bodies are byte-identical modulo per-platform frontmatter, per `subagent-support` SPEC C-22):

```markdown
---
description: Start a research-tier task. Gather reference corpus on a topic; follow-up implementation optional.
argument-hint: "<topic>"
---

# `/ark:research $ARGUMENTS`

Create a research-tier task: the deliverable is a curated reference corpus
under `.ark/tasks/<slug>/research/`. No PLAN, no VERIFY, no SPEC promotion.
Follow-up implementation is optional — close the task when the corpus is
sufficient.

## Preconditions
- `.ark/` is initialized.
- The goal is to *learn*, not to ship code. If you can already write a PRD's
  What/Why/Outcome, stop and use `/ark:quick` or `/ark:design` instead.

## Steps

### Step 1: Pull design-phase context `[AI]`
ark context --scope phase --for design --format json

### Step 2: Create the task `[AI]`
ark agent task new --slug <slug> --title "<topic>" --tier research [--worktree]

### Step 3: Fill the PRD `[AI]` `[USER]`
What / Why / Scope. Outcome optional. SPEC Path ignored on research tier.

### Step 4: Iteratively dispatch `ark-researcher` `[AI]` `[USER]`
Each dispatch writes one .ark/tasks/<slug>/research/<topic>.md.

### Step 5: Stage and close `[USER]` then `[AI]`
git add .ark/tasks/<slug>/ ; then /ark:commit -m "<message>".
```

[**Constraints**]

- C-1: `Tier::Research` serializes/deserializes as `"research"` via `#[serde(rename_all = "lowercase")]`; pre-existing tasks with `tier = "quick"|"standard"|"deep"` load unchanged.
- C-2: `Phase::Research` serializes/deserializes as `"research"` identically; pre-existing tasks with the other six phases load unchanged.
- C-3: `can_transition` returns `true` for exactly two new tuples involving Research: `(Research, Research, Committed)` and `(Research, Committed, Archived)`. Every other `(Research, _, _)` returns `false`.
- C-4: `task new --tier research` writes `phase = Research` (not `phase = Design`); the `build_task_toml` switch selects per-tier initial phase.
- C-5: `task new --tier research --worktree` succeeds and produces the same worktree topology as deep tier; the flag is opt-in across all tiers per `worktree` SPEC G-4 (unchanged).
- C-6: `task_commit` on `Tier::Research` skips `parse_verify_md` and treats `pending_verify = VerifyPendingCounts::default()`; skips `parse_spec_path`, `spec_extract`, `spec_register`; `deep_spec_promoted = false`.
- C-7: `task_commit` on `Tier::Research` stages `task.toml` plus every file under `.ark/tasks/<slug>/research/` recursively (if the directory exists); absent `research/` directory is not an error.
- C-8: `check_phase_for_commit` accepts `(Tier::Research, Phase::Research)`; every other phase under `Tier::Research` returns `Error::IllegalPhaseTransition`.
- C-9: `ark agent task plan|review|execute|verify` on a research task fails with `IllegalPhaseTransition { tier: Research, from: Research, to: X }`; no `task.toml` mutation occurs.
- C-10: PRD template is reused unchanged; the slash command body documents the research-tier semantic remap (Outcome optional, SPEC Path ignored).
- C-11: `/ark:research` slash-command body is byte-identical across Claude / Codex / OpenCode modulo per-platform frontmatter, mirroring `subagent-support` C-22.
- C-12: The slash command names `ark-researcher` as the dispatch target and mirrors `/ark:quick`'s `[USER]` staging step verbatim ("User runs `git add .ark/tasks/<slug>/`. Then invoke `/ark:commit -m \"<message>\"`.").
- C-13: `workflow.md`'s tier table grows a Research row at the end with cell text matching the existing terse table style.
- C-14: `workflow.md`'s lifecycle diagram adds a one-line `Research → Committed → Archived` track; the existing quick/standard/deep diagram is not rewritten.
- C-15: `workflow.md` ships a "Research" subsection ≤30 lines explaining when to use research vs. embedded `ark-researcher`, the PRD-on-research semantic remap, and that follow-up implementation is optional.
- C-16: No `Error` enum additions; no `task.toml` schema additions; no `Layout` getter additions.
- C-17: `Tier::Research` is treated as non-deep everywhere `matches!(tier, Tier::Deep)` is checked; existing call sites continue to short-circuit correctly without explicit Research-tier arms.

---

## Runtime

[**Main Flow**]

1. `ark agent task new --tier research --slug <s> --title "<t>"` → scaffold `.ark/tasks/<s>/{PRD.md, task.toml}` with `phase = Research`; register active; bind focus.
2. (Optional) `cd .ark/worktrees/<branch>/` when `--worktree` was passed.
3. User + main session fill `PRD.md` (What/Why/Scope; Outcome optional).
4. Main session iteratively dispatches `ark-researcher`; each writes one `research/<topic>.md`.
5. User runs `git add .ark/tasks/<slug>/`.
6. `/ark:commit -m "<msg>"` invokes `task_commit`: phase = Committed; commit stages task.toml + research/** + PRD.md (PRD is already in the user's `git add` set).
7. Later: `ark archive` moves the committed task into `tasks/archive/YYYY-MM/<slug>/`.

[**Failure Flow**]

1. `ark agent task plan|review|execute|verify` on a research task → `Error::IllegalPhaseTransition { tier: Research, from: Research, to: X }`. No state change.
2. `task_commit` invoked from `phase != Research` on `Tier::Research` → same error code, with `to = Committed`.
3. `task_commit` with empty staged index → `Error::NothingStaged` (unchanged contract).
4. `task_commit` mid-flight git failure → `RollbackGuard::Drop` restores `task.toml` and unstages Ark-managed files (research corpus files included in `ark_files_for_first_commit`); user's pre-existing staged entries survive.

[**State Transitions**]

- Created → `phase = Research`
- Research → Committed via `task_commit` (only legal mutation)
- Committed → Archived via `ark archive` (unchanged terminal transition)

---

## Implementation

[**Phase 1 — State machine**]

1. `crates/ark-core/src/commands/agent/state.rs`:
   - Add `Tier::Research` variant.
   - Add `Phase::Research` variant.
   - Add `can_transition` arms: `(Research, Research, Committed) => true`, `(Research, Committed, Archived) => true`.
   - Extend `archived_only_reachable_from_committed` and `archived_is_terminal` tests to include `Tier::Research`.
   - Add `can_transition_research` test covering legal + illegal transitions.

[**Phase 2 — Task creation**]

2. `crates/ark-core/src/commands/agent/task/new.rs`:
   - In `build_task_toml`, set initial phase per tier: `Tier::Research → Phase::Research`; all other tiers → `Phase::Design`.
   - Add unit test: `task_new_with_tier_research_starts_in_phase_research`.
   - Add unit test: `task_new_research_with_worktree_succeeds` (mirrors existing worktree tests).

3. `crates/ark-cli/src/agent_cli.rs`:
   - Add `research` to clap value-enum for `--tier`.
   - Add CLI parse test asserting `--tier research` round-trips.

[**Phase 3 — Commit on research tier**]

4. `crates/ark-core/src/commands/agent/task/commit.rs`:
   - Extend `check_phase_for_commit` matrix: `(Research, Research) => Ok`.
   - In `task_commit`, branch by tier early:
     - VERIFY gate runs iff `matches!(tier, Standard | Deep)` (unchanged condition; Research falls through with `pending = default`).
     - SPEC branch runs iff `tier == Deep` (unchanged condition).
   - Introduce private helper `research_corpus_files(task_dir: &Path) -> Vec<PathBuf>`:
     - Returns empty `Vec` when `task_dir.join("research")` is absent.
     - Otherwise walks the directory via existing `io::fs::walk_files` (or equivalent), returning every file path as project-root-relative.
   - In `ark_files_for_first_commit`, append `research_corpus_files(task_dir)` results when `tier == Research`. Add a `tier: Tier` parameter (single caller in `task_commit`).
   - Add E2E test `research_tier_commit_stages_corpus_and_clears_focus`.
   - Add E2E test `research_tier_skips_verify_gate`.
   - Add E2E test `research_tier_commit_does_not_promote_spec`.

[**Phase 4 — Slash commands**]

5. Add three new files:
   - `templates/claude/commands/ark/research.md`
   - `templates/codex/skills/ark-research/SKILL.md`
   - `templates/opencode/commands/ark/research.md`
   Bodies byte-identical modulo platform frontmatter (per `subagent-support` C-22).
6. Add parity assertion test under `commands/init.rs::tests` or a sibling (locate the existing parity-check for `subagent-support` agents and extend it for `commands/ark/research.md`).

[**Phase 5 — Documentation**]

7. `templates/ark/workflow.md`:
   - Tier table: add Research row at the bottom: `Research — knowledge-gathering; corpus IS the deliverable; follow-up optional. Artifacts: PRD.md, research/`.
   - Lifecycle section: add one-line `Research → Committed → Archived` track.
   - New §"Research" subsection (≤30 lines).
   - CLI surfaces §: extend the `ark agent task new --tier <quick|standard|deep>` example to include `research`.
8. `AGENTS.md`: one-line mention that `Tier::Research` exists under the `ark agent` responsibilities bullet. No other change.

[**Phase 6 — Quality bar**]

9. Run `cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`.
10. Run the end-to-end smoke test from `AGENTS.md` against a fresh tempdir to confirm round-trip integrity is unaffected.

---

## Trade-offs

- T-1: **Distinct `Phase::Research` vs. reusing `Phase::Design`.** Distinct phase is +1 enum variant + 2 transition table rows but reads unambiguously in `task.toml` and avoids tier-conditional branches inside `phase.rs::transition`. Chose distinct.
- T-2: **Reuse PRD.md vs. add `BRIEF.md` / `FINDINGS.md` templates.** Reuse means PRD's structural slots (Outcome, SPEC Path) have non-canonical meaning on research tier, documented in slash command body + workflow.md. Per discussion, two new templates were rejected as redundant — the corpus IS the deliverable. Chose reuse.
- T-3: **`--worktree` required vs. opt-in vs. forbidden.** Per-user direction: opt-in. Research tasks are read-mostly and don't *require* isolation. Opt-in costs one line of doc.
- T-4: **`research/` directory pre-created at `task new` vs. lazy.** Lazy matches `subagent-support` SPEC G-4 verbatim. Chose lazy.
- T-5: **Threading `tier` into `ark_files_for_first_commit` vs. branching at the caller.** Threading keeps the staging policy in one function. Chose threading.

---

## Validation

[**Unit Tests**]

- V-UT-1: `Tier::Research` round-trips through `toml::to_string` / `toml::from_str` as `tier = "research"`. (state.rs)
- V-UT-2: `Phase::Research` round-trips identically as `phase = "research"`. (state.rs)
- V-UT-3: `can_transition(Tier::Research, Phase::Research, Phase::Committed) == true`. (state.rs)
- V-UT-4: `can_transition(Tier::Research, Phase::Committed, Phase::Archived) == true`. (state.rs)
- V-UT-5: Every other `(Research, X, Y)` tuple returns `false` (parametric cross-product). (state.rs)
- V-UT-6: `archived_only_reachable_from_committed` extended for `Tier::Research`. (state.rs)
- V-UT-7: `archived_is_terminal` extended for `Tier::Research`. (state.rs)
- V-UT-8: `check_phase_for_commit(Tier::Research, Phase::Research)` is `Ok`; every other `(Research, X)` is `Err(IllegalPhaseTransition { to: Committed })`. (commit.rs)
- V-UT-9: `task_new` with `Tier::Research` writes `phase = Research` in the resulting `task.toml`. (new.rs)
- V-UT-10: `task_new` with `Tier::Research` and `worktree: Some(_)` produces the same worktree topology as deep tier. (new.rs)
- V-UT-11: `research_corpus_files` returns empty when `research/` absent; returns every file recursively when present. (commit.rs helper)
- V-UT-12: `ark_files_for_first_commit` on `Tier::Research` includes `task.toml` plus every research corpus file; on other tiers the output is unchanged. (commit.rs)

[**Integration Tests**]

- V-IT-1: `task_plan` on a research task → `Error::IllegalPhaseTransition`; `task.toml` byte-identical pre/post. (phase.rs)
- V-IT-2: `task_review` / `task_execute` / `task_verify` on a research task all err identically. (phase.rs)
- V-IT-3: Slash-command parity: research.md / SKILL.md exist on all three platforms; bodies byte-identical after stripping frontmatter. (templates parity test)
- V-IT-4: `ark agent task new --tier research --slug X --title "T"` succeeds via the CLI parser; `task.toml` carries `tier = "research"` and `phase = "research"`. (cli_help.rs or agent_lifecycle.rs)

[**Failure / Robustness**]

- V-F-1: `task_commit` on a research task with `--no-commit` records phase transition (`Committed`) and skips `git commit`. (commit.rs e2e)
- V-F-2: `task_commit` on a research task with pre-commit hook failure rolls back: `task.toml.phase == Research` post-rollback, research corpus files unstaged. (commit.rs e2e)
- V-F-3: Research task scaffolded without `research/` directory commits cleanly when only `PRD.md` is staged. (commit.rs e2e)
- V-F-4: Research task with nested `research/<subdir>/<topic>.md` corpus stages every nested file. (commit.rs e2e)

[**Edge Cases**]

- V-E-1: Task with `phase = research` but `tier = quick` (manually edited TOML) — `task_commit` rejects via `check_phase_for_commit`. (commit.rs)
- V-E-2: `ark archive` over a committed research task moves it to `tasks/archive/YYYY-MM/<slug>/` identically to other tiers. (archive.rs)
- V-E-3: `ark context --scope phase --for design` invoked on a research task surfaces the same projection as on a freshly-seeded standard task. (context smoke test)
- V-E-4: Pre-existing `task.toml` files at tier ∈ {quick, standard, deep} continue to parse and round-trip. (state.rs)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-UT-2, V-IT-4 |
| G-2 | V-UT-3, V-UT-4, V-UT-5, V-IT-1, V-IT-2 |
| G-3 | V-UT-9, V-UT-10 |
| G-4 | V-UT-8, V-UT-11, V-UT-12, V-F-1, V-F-3, V-F-4 |
| G-5 | V-IT-3 |
| C-1 | V-UT-1 |
| C-2 | V-UT-2 |
| C-3 | V-UT-3, V-UT-4, V-UT-5 |
| C-4 | V-UT-9 |
| C-5 | V-UT-10 |
| C-6 | V-UT-11, V-UT-12, V-F-3 |
| C-7 | V-UT-11, V-UT-12, V-F-4 |
| C-8 | V-UT-8 |
| C-9 | V-IT-1, V-IT-2 |
| C-10 | V-IT-3 |
| C-11 | V-IT-3 |
| C-12 | V-IT-3 (file contents verified) |
| C-13 | doc review at VERIFY |
| C-14 | doc review at VERIFY |
| C-15 | doc review at VERIFY |
| C-16 | source-scan at VERIFY |
| C-17 | V-UT-8 + source-scan at VERIFY |
