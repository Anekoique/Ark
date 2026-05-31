# `ark-research` PLAN `01`

> Status: Revised
> Feature: `ark-research`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

Introduce a fourth workflow tier, `research`, whose deliverable is a curated reference corpus under `.ark/tasks/<slug>/research/` rather than a code change. Add `Tier::Research` and `Phase::Research` enum variants, two new transition-table rows (`Research → Committed`, `Committed → Archived`), and a tier-conditional branch in `task_commit` that skips the VERIFY gate and SPEC extraction while staging `research/**` recursively. `task_promote` rejects every source-or-target involving `Tier::Research` with `Error::WrongTier`; cross-over between research and tiered implementation is via a fresh `task new`, not promotion. Ship `/ark:research <topic>` slash commands on Claude / Codex / OpenCode, matching the existing per-platform variant conventions (Codex substitutes `/ark:command → ark-command`, replaces `$ARGUMENTS`); Claude and OpenCode bodies stay byte-identical. `--worktree` is opt-in. PRD.md is reused; no new artifact template.

## Log

[**Added**]

- C-18 — explicit `task_promote` policy for `Tier::Research`: both directions rejected with `Error::WrongTier { expected, actual }` (R-002).
- NG-5 — `task_promote` cross-over with research tier is out of scope (R-002 framing).
- Architecture file tree now lists `promote.rs` with the policy edit (R-002).
- C-11 rewritten to match shipping per-platform conventions: Claude/OpenCode byte-identical; Codex applies a documented substitution map (`/ark:research → ark-research`, `/ark:commit → ark-commit`, `$ARGUMENTS → <topic>`) (R-001).
- New defensive guard in `phase.rs::artifact_for`: explicit `(_, Tier::Research, _) => None` arm so research tier can never seed `*_PLAN.md` / `*_REVIEW.md` / `VERIFY.md` even if `transition()`'s ordering ever changes (R-005).
- V-IT-1 strengthened to also assert no `*_PLAN.md` / `*_REVIEW.md` / `VERIFY.md` exists in the task dir after the illegal-transition error (R-005).
- V-UT-13 covers `task_promote` rejection in both directions for `Tier::Research` (R-002).
- V-UT-14 covers the `artifact_for` defensive arm (R-005).
- V-UT-15 covers `parse_tier("research")` (R-007).
- V-IT-5 covers `task_promote --to research` end-to-end (R-002).
- Slash-command sketch under `[**API Surface**]` rewritten to mirror `/ark:quick`'s body shape: fenced bash blocks per step, per-step explanatory sentence, `## If the corpus turns into implementation` subsection, `## See Also` block (R-004).
- "Research" subsection in workflow.md now ships the field-by-field PRD remap (Outcome → "Why this corpus is the right next step"; SPEC Path → ignored; Related Specs → optional) per TR-1 required action.
- T-6 captures the promote-policy trade-off rationale (R-002 framing).

[**Changed**]

- C-13 / C-14 rewritten to describe the actual workflow.md structure (bullet list under `## Tiers`, one-line ASCII lifecycle arrow), not a fabricated "tier table" / "lifecycle diagram" (R-003).
- Phase 5 step 7 implementation wording aligned with the bullet/arrow structure (R-003).
- T-5 trade-off framing acknowledges `ark_files_for_first_commit` is private; the decision is "where the conditional lives", not an API shape choice (R-006).
- `[**API Surface**]` now distinguishes public re-exports from private helpers; `ark_files_for_first_commit` is annotated private (R-006).
- Implementation Phase 2 step 3 corrects the CLI parser description: `parse_tier` is a hand-rolled value-parser at `crates/ark-cli/src/agent_cli.rs:324`; the change is one `"research" => Ok(Tier::Research)` arm plus the error-message tier list (R-007).
- C-17 enumerates the four production `Tier::Deep` sites verified during this iteration (`promote.rs:78`, `promote.rs:91`, `commit.rs:174`, `spec/extract.rs:84`).

[**Removed**]

- The "byte-identical bodies across Claude / Codex / OpenCode" claim — never true for slash commands, and conflated with `subagent-support` C-22 which governs agent prompts, not commands (R-001).
- The "phase.rs needs no code change" claim — superseded by the defensive `Tier::Research => None` arm (R-005).

[**Unresolved**]

- None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review `00_REVIEW.md` | R-001 (HIGH) | Accepted | C-11 rewritten to a per-platform variant contract (Claude/OpenCode byte-identical; Codex substitution map: `/ark:research → ark-research`, `/ark:commit → ark-commit`, `$ARGUMENTS → <topic>`). PRD Outcome bullet 6 updated alongside the slash-command files in Phase 5. V-IT-3 widened to test the substitution map. |
| Review `00_REVIEW.md` | R-002 (HIGH) | Accepted | New Constraint C-18: `task_promote` rejects every source-or-target involving `Tier::Research` with `Error::WrongTier`. Architecture lists `promote.rs` with the matches!-arm edit. NG-5 added. V-UT-13 + V-IT-5 cover both rejection directions. Rationale: research is upstream of tiered implementation, not a sibling tier — cross-over is a fresh `task new`, not a promote. |
| Review `00_REVIEW.md` | R-003 (MEDIUM) | Accepted | C-13 / C-14 / Phase 6 step 7 rewritten to describe the actual structure (`## Tiers` bullet list, one-line ASCII lifecycle arrow) instead of a fabricated table / diagram. |
| Review `00_REVIEW.md` | R-004 (MEDIUM) | Accepted | Slash-command sketch under `[**API Surface**]` rewritten to mirror `templates/claude/commands/ark/quick.md` shape: fenced ```bash blocks per step, per-step explanatory sentence, `## If the corpus turns into implementation` subsection, `## See Also` block. The Codex variant body applies the substitution map per C-11. |
| Review `00_REVIEW.md` | R-005 (MEDIUM) | Accepted | Defensive arm `(_, Tier::Research, _) => None` added to `phase.rs::artifact_for` (Phase 1 step 1.5). V-IT-1 strengthened to assert no `*_PLAN.md` / `*_REVIEW.md` / `VERIFY.md` artifact on disk post-illegal-transition. V-UT-14 covers the defensive arm directly. The "phase.rs needs no code change" claim is withdrawn. |
| Review `00_REVIEW.md` | R-006 (LOW) | Accepted | `ark_files_for_first_commit`'s declaration moved from `[**API Surface**]` to `[**Architecture**]` (private helper). T-5 reframed as "where the conditional lives", not an API-shape decision. |
| Review `00_REVIEW.md` | R-007 (LOW) | Accepted | Phase 2 step 3 corrected: `parse_tier` at `crates/ark-cli/src/agent_cli.rs:324` is a hand-rolled value-parser; the change is one match arm plus the error-message tier list. V-UT-15 added. |
| Review `00_REVIEW.md` | TR-1 | Accepted | T-2 stands. workflow.md "Research" subsection ships the field-by-field PRD remap per TR-1's required action. |
| Review `00_REVIEW.md` | TR-2 | Accepted | T-3 stands. No PLAN edit beyond C-5's existing wording. |
| Review `00_REVIEW.md` | TR-3 | Accepted | T-4 stands. C-7 + V-F-3 already cover the empty-corpus case. |

---

## Spec

> This section is the durable design record. On deep-tier commit, it is copied verbatim into `specs/features/ark-research/SPEC.md`. The Spec is self-contained per iteration — no "see 00_PLAN" references appear here.

[**Goals**]

- G-1: `Tier::Research` is a first-class variant alongside Quick/Standard/Deep.
- G-2: `Research → Committed → Archived` is the entire legal lifecycle for tier=research.
- G-3: `task new --tier research` scaffolds PRD.md + task.toml; --worktree is opt-in.
- G-4: `task commit` on research tier skips VERIFY and SPEC extract; stages task.toml + research/**.
- G-5: `/ark:research` ships on Claude / Codex / OpenCode; Claude+OpenCode byte-identical bodies, Codex applies the documented substitution map.

[**Non-goals**]

- NG-1: No new artifact template (PRD.md is reused; no BRIEF / FINDINGS / SCOPE files).
- NG-2: No SPEC promotion: research tier never invokes `parse_spec_path` / `spec_extract` / `spec_register`.
- NG-3: No `[**Research**]` PRD block for cross-task citation; PRDs may mention research slugs informally in prose.
- NG-4: No `ark context --for research` phase projection; the existing `design` projection serves research tasks.
- NG-5: No `task_promote` cross-over with research tier; both directions are rejected (see C-18). Cross-over is by `task new`.

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
│       ├── phase.rs                   (*) defensive arm in artifact_for:
│       │                                   (_, Tier::Research, _) => None;
│       │                                   no other code change
│       ├── promote.rs                 (*) early-return in task_promote: every (from, to)
│       │                                   involving Tier::Research returns
│       │                                   Error::WrongTier { expected: from, actual: opts.to };
│       │                                   phase_exists_in_tier untouched
│       └── commit.rs                  (*) check_phase_for_commit gains (Research, Research);
│                                           task_commit short-circuits the VERIFY gate and the
│                                           deep-tier SPEC branch for Tier::Research;
│                                           private helper research_corpus_files() walks
│                                           task_dir.join("research") recursively;
│                                           ark_files_for_first_commit (private) gains a
│                                           tier: Tier parameter and appends the corpus

crates/ark-cli/src/
└── agent_cli.rs                       (*) parse_tier (hand-rolled, line 324) gains
                                            "research" => Ok(Tier::Research); error message
                                            tier list updated.

templates/
├── claude/commands/ark/research.md    (+) new slash command; mirrors quick.md shape
├── codex/skills/ark-research/SKILL.md (+) new skill; Codex substitution map applied
├── opencode/commands/ark/research.md  (+) new command; byte-identical to claude variant
└── ark/workflow.md                    (*) ## Tiers bullet list grows a Research bullet;
                                            lifecycle arrow gains a second line for research;
                                            new ## Research subsection (≤30 lines)
```

Module coupling: untouched. `state.rs`, `new.rs`, `phase.rs`, `promote.rs`, `commit.rs` each get a localized edit; no new modules, no new public types, no new `Error` variants beyond reusing the existing `Error::WrongTier { expected: Tier, actual: Tier }`.

Private helper signatures (not part of the public surface):

```rust
// crates/ark-core/src/commands/agent/task/commit.rs
fn research_corpus_files(task_dir: &Path) -> Vec<PathBuf>;
//   Returns project-root-relative paths under <task_dir>/research/, recursively.
//   Empty Vec when the directory is absent. Uses walk_files_excluding (or sibling
//   walker) already present in io/fs.

fn ark_files_for_first_commit(
    task_dir: &Path,
    tier: Tier,                                          // (+) new param replacing `deep: bool`
    spec_path: &Path,
    intermediate_indexes: &[PathBuf],
    workspace_files: &[PathBuf],
) -> Vec<PathBuf>;
//   When tier == Tier::Research: append research_corpus_files(task_dir).
//   When tier == Tier::Deep:    append spec_path + intermediate_indexes (unchanged).
//   Always:                     prepend task.toml; append workspace_files.
//   Caller in task_commit threads `prev_toml.tier`.
```

Call graph for `task new --tier research` (no worktree variant; --worktree branch is identical to other tiers):

```
task_new(opts)
  ├── validate_slug, validate_title
  ├── scaffold task dir + copy_template("PRD", PRD.md)
  ├── build_task_toml: select initial phase per tier:
  │     Tier::Research → Phase::Research
  │     _              → Phase::Design
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
  ├── ark_files = ark_files_for_first_commit(task_dir, Tier::Research, ...)
  │     ← appends every file under <task_dir>/research/ recursively
  ├── stage_files + git commit (unchanged)
  ├── guard.commit + clear_focus (unchanged)
  └── return TaskCommitSummary { tier: Research, deep_spec_promoted: false, ... }
```

Failure path: any error before `guard.commit()` triggers `RollbackGuard::Drop`, which already handles soft-reset + ark_files unstage + task.toml restore. Research tier adds no new snapshot surface (no SPEC, no INDEX).

Call graph for `task promote` involving research tier:

```
task_promote(opts)
  ├── validate_slug
  ├── load TaskToml
  ├── if from == Tier::Research || opts.to == Tier::Research:
  │     return Err(Error::WrongTier { expected: from, actual: opts.to })
  └── (existing phase_exists_in_tier check + tier swap, untouched)
```

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

No new `Error` variants. `task new --tier research` succeeds via the existing parser; misuse routes through existing `IllegalPhaseTransition` (e.g. `ark agent task plan` on a research task) or the existing `Error::WrongTier { expected, actual }` (`task promote` involving research tier).

[**API Surface**]

Public surface re-exported from `crates/ark-core/src/lib.rs`:

```rust
pub use commands::agent::{Tier, Phase, Status, TaskToml};   // unchanged signatures; enums grow
```

CLI surface: `ark agent task new --tier research` accepted by the existing hand-rolled `parse_tier` at `crates/ark-cli/src/agent_cli.rs:324`. The change is one match arm:

```rust
fn parse_tier(s: &str) -> Result<Tier, String> {
    match s {
        "quick"    => Ok(Tier::Quick),
        "standard" => Ok(Tier::Standard),
        "deep"     => Ok(Tier::Deep),
        "research" => Ok(Tier::Research),    // (+) one new arm
        other      => Err(format!(
            "expected one of quick|standard|deep|research, got `{other}`"
        )),
    }
}
```

Slash-command body shape (Claude flavor; OpenCode body is byte-identical modulo frontmatter; Codex body applies the per-platform substitution map):

```markdown
---
description: Start a research-tier task. Gather reference corpus on a topic; follow-up implementation is optional.
argument-hint: "<topic>"
---

# `/ark:research $ARGUMENTS`

Create a research-tier task: the deliverable is a curated reference corpus
under `.ark/tasks/<slug>/research/`. No PLAN, no VERIFY, no SPEC promotion.
Follow-up implementation is optional — close the task when the corpus is
sufficient.

Structural ops (task dir, phase transitions, archive moves) are owned by
`ark agent`. Do not hand-edit `task.toml` or move directories.

## Preconditions

- `.ark/` is initialized.
- The goal is to *learn*, not to ship code. If you can already write a PRD's
  What/Why/Outcome, stop and use `/ark:quick` or `/ark:design` instead — the
  embedded `ark-researcher` subagent dispatched during DESIGN/PLAN covers
  knowledge-gathering in service of a known deliverable.

## Steps

### Step 1: Pull design-phase context `[AI]`

(fenced bash: `ark context --scope phase --for design --format json`)

Returns the snapshot of git, active tasks, and project + feature specs.

### Step 2: Create the task `[AI]`

Slugify the topic (lowercase, hyphen-separated, ASCII, ≤40 chars).

(fenced bash: `ark agent task new --slug <slug> --title "<topic>" --tier research`)

Scaffolds `.ark/tasks/<slug>/{PRD.md, task.toml}`, registers the slug, sets
this session's focus. Refuses if the slug already exists. Add `--worktree`
when research will run in parallel with in-flight code work; then
`cd .ark/worktrees/<branch>/` for subsequent steps.

### Step 3: Fill the PRD `[AI]` `[USER]`

Edit `.ark/tasks/<slug>/PRD.md`. On research tier the fields map as:

- **What** — the question / direction being investigated.
- **Why** — motivation. Not required to lead to implementation.
- **Outcome** — "Why this corpus is the right next step" (optional; the corpus
  itself is the deliverable).
- **Related Specs** — feature SPECs consulted (optional).
- **SPEC Path** — ignored on research tier.

### Step 4: Iteratively dispatch `ark-researcher` `[AI]` `[USER]`

For each sub-topic, dispatch the `ark-researcher` subagent. Each call writes
one `.ark/tasks/<slug>/research/<topic>.md`. Repeat until the corpus is
sufficient. The PRD's Scope is the bound.

### Step 5: Stage and close `[USER]` then `[AI]`

User runs `git add .ark/tasks/<slug>/`. Then invoke
`/ark:commit -m "<message>"`. See `/ark:commit` for the contract.

## If the corpus turns into implementation

Research tasks do NOT promote to implementation tiers (`task promote` rejects
research↔tiered cross-over). When the corpus is ready and you want to build:

1. Close the research task as above.
2. Start a fresh `/ark:quick` or `/ark:design` task whose PRD references the
   research slug in prose (e.g. "based on the corpus at
   `tasks/archive/.../foo/research/`").

## See Also

- `workflow.md` §3 (tiers), §Research, §4 (phase contracts), §5 (lifecycle)
- `/ark:commit` — closure contract
```

Codex variant applies the substitution map: `/ark:research → ark-research`, `/ark:commit → ark-commit`, `# \`/ark:research $ARGUMENTS\`` → `# \`ark-research <topic>\``. Frontmatter shape per `templates/codex/skills/ark-quick/SKILL.md`.

[**Constraints**]

- C-1: `Tier::Research` serializes/deserializes as `"research"` via `#[serde(rename_all = "lowercase")]`; pre-existing tasks with `tier = "quick"|"standard"|"deep"` load unchanged.
- C-2: `Phase::Research` serializes/deserializes as `"research"` identically; pre-existing tasks with the other six phases load unchanged.
- C-3: `can_transition` returns `true` for exactly two new tuples involving Research: `(Research, Research, Committed)` and `(Research, Committed, Archived)`. Every other `(Research, _, _)` returns `false`.
- C-4: `task new --tier research` writes `phase = Research` (not `phase = Design`); the `build_task_toml` switch selects per-tier initial phase.
- C-5: `task new --tier research --worktree` succeeds and produces the same worktree topology as deep tier; the flag is opt-in across all tiers per `worktree` SPEC G-4.
- C-6: `task_commit` on `Tier::Research` skips `parse_verify_md` and treats `pending_verify = VerifyPendingCounts::default()`; skips `parse_spec_path`, `spec_extract`, `spec_register`; `deep_spec_promoted = false`.
- C-7: `task_commit` on `Tier::Research` stages `task.toml` plus every file under `.ark/tasks/<slug>/research/` recursively (if the directory exists); absent `research/` directory is not an error.
- C-8: `check_phase_for_commit` accepts `(Tier::Research, Phase::Research)`; every other phase under `Tier::Research` returns `Error::IllegalPhaseTransition`.
- C-9: `ark agent task plan|review|execute|verify` on a research task fails with `IllegalPhaseTransition { tier: Research, from: Research, to: X }`; no `task.toml` mutation occurs and no `*_PLAN.md` / `*_REVIEW.md` / `VERIFY.md` artifact is written.
- C-10: PRD template is reused unchanged; the slash command body documents the research-tier semantic remap (Outcome optional, SPEC Path ignored).
- C-11: `/ark:research` slash-command bodies follow shipping per-platform conventions: Claude (`templates/claude/commands/ark/research.md`) and OpenCode (`templates/opencode/commands/ark/research.md`) are byte-identical modulo their respective frontmatter; Codex (`templates/codex/skills/ark-research/SKILL.md`) applies the substitution map `/ark:research → ark-research`, `/ark:commit → ark-commit`, `$ARGUMENTS → <topic>`, and the H1 line `# \`/ark:research $ARGUMENTS\`` → `# \`ark-research <topic>\``.
- C-12: The slash command names `ark-researcher` as the dispatch target and mirrors `/ark:quick`'s `[USER]` staging step verbatim ("User runs `git add .ark/tasks/<slug>/`. Then invoke `/ark:commit -m \"<message>\"`.").
- C-13: `workflow.md`'s `## Tiers` bullet list (currently three bullets: Quick / Standard / Deep) grows a fourth bullet at the end: `- **Research** — knowledge-gathering; corpus IS the deliverable; follow-up implementation optional. Artifacts: PRD.md, research/`.
- C-14: `workflow.md`'s one-line lifecycle arrow (currently `DESIGN → PLAN → [REVIEW ⇄ PLAN] → EXECUTE → VERIFY → COMMIT → (later) ARCHIVE`) gains an immediately-following line: `RESEARCH → COMMIT → (later) ARCHIVE   research tier`. The existing arrow is not rewritten.
- C-15: `workflow.md` ships a "Research" subsection ≤30 lines covering: (a) when to use research tier vs. embedded `ark-researcher` dispatch inside a tiered task; (b) the field-by-field PRD remap (What/Why/Scope filled; Outcome optional; SPEC Path ignored; Related Specs optional); (c) follow-up implementation is optional and uses a fresh `task new`, not `task promote`.
- C-16: No `Error` enum additions; no `task.toml` schema additions; no `Layout` getter additions.
- C-17: `Tier::Research` is treated as non-deep at every production `Tier::Deep`-checking site: `promote.rs:78` (`from != Tier::Deep && opts.to == Tier::Deep`), `promote.rs:91` (`match opts.to { Tier::Deep => ... }`), `commit.rs:174` (`prev_toml.tier == Tier::Deep`), `spec/extract.rs:84` (`if toml.tier != Tier::Deep`). All four short-circuit correctly for `Tier::Research` without a Research-specific arm.
- C-18: `task_promote` rejects every invocation involving `Tier::Research` (as source OR target) with `Error::WrongTier { expected: <source_tier>, actual: <target_tier> }`. Cross-over between research and tiered implementation is by a fresh `task new`, not promotion.

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

1. `ark agent task plan|review|execute|verify` on a research task → `Error::IllegalPhaseTransition { tier: Research, from: Research, to: X }`. No state change; no artifact written (per `phase.rs::artifact_for` defensive arm).
2. `ark agent task promote --to <any>` on a research task OR `task promote --to research` on any other task → `Error::WrongTier { expected, actual }`. No state change.
3. `task_commit` invoked from `phase != Research` on `Tier::Research` → `Error::IllegalPhaseTransition` with `to = Committed`.
4. `task_commit` with empty staged index → `Error::NothingStaged` (unchanged contract).
5. `task_commit` mid-flight git failure → `RollbackGuard::Drop` restores `task.toml` and unstages Ark-managed files (research corpus files included in `ark_files_for_first_commit`'s output); user's pre-existing staged entries survive.

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

1.5. `crates/ark-core/src/commands/agent/task/phase.rs`:
   - Add a defensive arm to `artifact_for`: `(_, Tier::Research, _) => None`. Prevents any future refactor from seeding plan/review/verify artifacts for research-tier tasks even if `transition()`'s ordering ever changes.

[**Phase 2 — Task creation**]

2. `crates/ark-core/src/commands/agent/task/new.rs`:
   - In `build_task_toml`, set initial phase per tier: `Tier::Research → Phase::Research`; all other tiers → `Phase::Design`.
   - Add unit test: `task_new_with_tier_research_starts_in_phase_research`.
   - Add unit test: `task_new_research_with_worktree_succeeds`.

3. `crates/ark-cli/src/agent_cli.rs`:
   - Add `"research" => Ok(Tier::Research)` arm to the hand-rolled `parse_tier` value-parser at line 324.
   - Update the error message's tier list to `quick | standard | deep | research`.
   - Add unit test V-UT-15: `parse_tier("research")` returns `Ok(Tier::Research)`; error path contains the four canonical tier names.

[**Phase 3 — Promote policy**]

3a. `crates/ark-core/src/commands/agent/task/promote.rs`:
   - In `task_promote`, after loading `TaskToml`, add an early-return:
     ```rust
     if from == Tier::Research || opts.to == Tier::Research {
         return Err(Error::WrongTier { expected: from, actual: opts.to });
     }
     ```
   - `phase_exists_in_tier` is unchanged: it now never sees `Tier::Research` because the early-return fires first.
   - Add unit test: `task_promote_rejects_research_source`.
   - Add unit test: `task_promote_rejects_research_target`.

[**Phase 4 — Commit on research tier**]

4. `crates/ark-core/src/commands/agent/task/commit.rs`:
   - Extend `check_phase_for_commit` matrix: `(Research, Research) => Ok`.
   - In `task_commit`, branch by tier early:
     - VERIFY gate runs iff `matches!(tier, Standard | Deep)` (unchanged condition; Research falls through with `pending = default`).
     - SPEC branch runs iff `tier == Deep` (unchanged condition).
   - Introduce private helper `research_corpus_files(task_dir: &Path) -> Vec<PathBuf>`:
     - Returns empty `Vec` when `task_dir.join("research")` is absent.
     - Otherwise walks the directory via existing `io::fs::walk_files` (or equivalent), returning every file path as project-root-relative.
   - In `ark_files_for_first_commit`, replace the existing `deep: bool` parameter with `tier: Tier`; append `research_corpus_files(task_dir)` results when `tier == Research`. Single caller in `task_commit` is updated.
   - Add E2E test `research_tier_commit_stages_corpus_and_clears_focus`.
   - Add E2E test `research_tier_skips_verify_gate`.
   - Add E2E test `research_tier_commit_does_not_promote_spec`.

[**Phase 5 — Slash commands**]

5. Add three new files:
   - `templates/claude/commands/ark/research.md` (full body per `[**API Surface**]` above).
   - `templates/codex/skills/ark-research/SKILL.md` (Codex substitution map applied per C-11).
   - `templates/opencode/commands/ark/research.md` (byte-identical to claude variant modulo frontmatter).
6. Add parity assertion test for Claude+OpenCode byte-identity (modulo frontmatter); locate the test pattern in `commands/init.rs::tests` or a sibling and extend. Codex variant is excluded from byte-parity per C-11; V-IT-3 applies the inverse substitution before asserting equality.

[**Phase 6 — Documentation**]

7. `templates/ark/workflow.md`:
   - `## Tiers` bullet list: add a fourth bullet at the end per C-13.
   - Lifecycle arrow at line 75: add a second line per C-14.
   - New `### Research` subsection (≤30 lines) per C-15: when to use; PRD field remap; follow-up via fresh `task new`.
   - CLI surfaces §: extend the `ark agent task new --tier <quick|standard|deep>` example to include `research`.
8. `AGENTS.md`: one-line mention that `Tier::Research` exists under the `ark agent` responsibilities bullet. No other change.
9. Update the PRD (`.ark/tasks/ark-research/PRD.md`) Outcome bullet 6 to match C-11's revised contract: replace "byte-identical bodies modulo per-platform frontmatter" with "Claude and OpenCode bodies byte-identical modulo frontmatter; Codex applies the documented substitution map".

[**Phase 7 — Quality bar**]

10. Run `cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`.
11. Run the end-to-end smoke test from `AGENTS.md` against a fresh tempdir to confirm round-trip integrity is unaffected.

---

## Trade-offs

- T-1: **Distinct `Phase::Research` vs. reusing `Phase::Design`.** Distinct phase is +1 enum variant + 2 transition table rows but reads unambiguously in `task.toml` and avoids tier-conditional branches inside `phase.rs::transition`. Chose distinct.
- T-2: **Reuse PRD.md vs. add `BRIEF.md` / `FINDINGS.md` templates.** Reuse means PRD's structural slots (Outcome, SPEC Path) have non-canonical meaning on research tier, documented in slash command body + workflow.md. Two new templates were rejected as redundant — the corpus IS the deliverable. Chose reuse. (TR-1 affirms.)
- T-3: **`--worktree` required vs. opt-in vs. forbidden.** Opt-in. Research tasks are read-mostly and don't *require* isolation. Aligns with `worktree` SPEC G-4. (TR-2 affirms.)
- T-4: **`research/` directory pre-created at `task new` vs. lazy.** Lazy matches `subagent-support` SPEC G-4 verbatim. C-7 handles absent-`research/` as a no-op. (TR-3 affirms.)
- T-5: **Where the corpus-staging conditional lives.** Threading `tier` into the private helper `ark_files_for_first_commit` keeps the staging policy in one function; branching at the caller would split file-list construction across two sites. Chose threading. Note: `ark_files_for_first_commit` is a private helper, so this is a code-shape decision, not an API-shape one.
- T-6: **Promote-policy for `Tier::Research`.** Three options: (a) forbid both directions with `Error::WrongTier`; (b) allow research → tiered, framing research as a "junior tier" that may graduate; (c) allow nothing reachable (silent / hidden). Chose (a). Research is *upstream* of tiered work, not a sibling tier — the deliverable shapes differ (corpus vs. code), the artifact sets differ (no PLAN/REVIEW/VERIFY), and the audit gates differ (none vs. VERIFY). A graduated research → standard mid-flight would lose the corpus's identity as a closed task; better to close it cleanly and start a fresh implementation task that *cites* the research slug in PRD prose.

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
- V-UT-13: `task_promote` rejects with `Error::WrongTier` in both directions: (a) `from = Tier::Research, to = Tier::Standard` and (b) `from = Tier::Standard, to = Tier::Research`; no `task.toml` mutation. (promote.rs)
- V-UT-14: `artifact_for` returns `None` for any `(_, Tier::Research, _)` triple even if called directly (defensive arm coverage). (phase.rs)
- V-UT-15: `parse_tier("research") == Ok(Tier::Research)`; `parse_tier("nope")` error message contains the four canonical tier names. (agent_cli.rs)

[**Integration Tests**]

- V-IT-1: `task_plan` on a research task → `Error::IllegalPhaseTransition`; `task.toml` byte-identical pre/post; no `*_PLAN.md`, `*_REVIEW.md`, or `VERIFY.md` exists in the task dir after the error. (phase.rs)
- V-IT-2: `task_review` / `task_execute` / `task_verify` on a research task all err identically (same shape as V-IT-1). (phase.rs)
- V-IT-3: Slash-command parity: `templates/{claude,opencode}/commands/ark/research.md` are byte-identical after stripping frontmatter; `templates/codex/skills/ark-research/SKILL.md` differs only by the documented substitution map (the parity test applies the inverse substitution to the Codex body and asserts byte-equality with Claude's). (templates parity test)
- V-IT-4: `ark agent task new --tier research --slug X --title "T"` succeeds via the CLI parser; `task.toml` carries `tier = "research"` and `phase = "research"`. (cli_help.rs or agent_lifecycle.rs)
- V-IT-5: `ark agent task promote --to research` on an existing standard-tier task → `Error::WrongTier`; existing task unchanged. (agent_lifecycle.rs)

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
| C-9 | V-IT-1, V-IT-2, V-UT-14 |
| C-10 | V-IT-3 |
| C-11 | V-IT-3 |
| C-12 | V-IT-3 (file contents verified) |
| C-13 | doc review at VERIFY |
| C-14 | doc review at VERIFY |
| C-15 | doc review at VERIFY |
| C-16 | source-scan at VERIFY |
| C-17 | V-UT-8 + source-scan at VERIFY |
| C-18 | V-UT-13, V-IT-5 |
