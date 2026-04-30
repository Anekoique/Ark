# Lifecycle

Every task moves through the same states. Quick skips most of them; standard and deep walk all five.

```
       ┌────────────┐
       │  /ark:*    │  slash command starts a task
       └─────┬──────┘
             ▼
       ┌────────────┐
       │  DESIGN    │  write PRD.md — What / Why / Outcome
       └─────┬──────┘
             │  (quick skips plan/review/verify)
             ▼
       ┌────────────┐
       │    PLAN    │  write NN_PLAN.md — elaborate how
       └─────┬──────┘
             │         (deep only — plan review loop)
             │         ┌──────────────┐
             ├────────►│    REVIEW    │  NN_REVIEW.md
             │         └──────┬───────┘
             │ ◄─── rejected ─┘
             ▼
       ┌────────────┐
       │  EXECUTE   │  implement; update PLAN's Spec section if gaps emerge
       └─────┬──────┘
             ▼
       ┌────────────┐
       │   VERIFY   │  single-pass gate
       └─────┬──────┘  rejected → halt for user decision
             ▼
       ┌────────────┐
       │  ARCHIVE   │  move to tasks/archive/YYYY-MM/;
       └────────────┘  deep: extract SPEC → specs/features/<name>/
```

State is recorded in `task.toml.phase`. Each transition is mediated by a CLI command (`ark agent task plan`, `... review`, `... execute`, `... verify`, `... archive`); illegal transitions error out with `IllegalPhaseTransition` rather than silently corrupting state.

## DESIGN — capture what & why

**Purpose.** Write `PRD.md` covering What / Why / Outcome / Related Specs. Brainstorm matches the tier — quick = none, standard = ≤3 clarifying questions, deep = thorough.

**Calls.**
- `ark context --scope phase --for design` — git, project specs, feature specs index, recent archive.
- `ark agent task new --slug <s> --title "<t>" --tier {quick|standard|deep} [--worktree]` — scaffolds the task dir + PRD + `task.toml`. `--worktree` binds the task to a fresh git worktree at `.ark/worktrees/<branch>/`.

**Gate.** PRD drafted, Outcome stated. Quick → EXECUTE; standard/deep → PLAN.

## PLAN — elaborate how

**Purpose.** Fill `NN_PLAN.md` from the embedded template (Spec, Runtime, Implementation, Trade-offs, Validation). Every Goal mapped to ≥1 Validation in the Acceptance Mapping table.

**Calls.**
- `ark context --scope phase --for plan` — current PRD + related feature specs (filtered to the PRD's `[**Related Specs**]`) + project specs.
- `ark agent task plan` — transitions DESIGN → PLAN and seeds `00_PLAN.md`.

**Gate.** PLAN complete; Acceptance Mapping fills every Goal. Standard → EXECUTE; deep → REVIEW.

**Rule.** `## Spec` must be self-contained every iteration (deltas go in `## Log`). It's copied verbatim to `specs/features/<name>/SPEC.md` on archive (deep tier).

## REVIEW — pre-execute gate (deep only, iterative)

**Purpose.** Evaluate the latest `NN_PLAN.md` against PRD and project specs; write `NN_REVIEW.md` with verdict + findings. Loop until verdict is *Approved* with zero open CRITICAL.

**Calls.**
- `ark context --scope phase --for review` — current task, latest PLAN, related feature specs, project specs.
- `ark agent task review` — transitions PLAN → REVIEW and seeds `NN_REVIEW.md`.

**Reject (HIGH)** if the latest PLAN's `## Spec` references prior iterations instead of restating in full.

**Iteration.** Copy `NN_PLAN.md` and `NN_REVIEW.md` to the next number, bump `task.toml.iteration`, reset `phase = "plan"`. The state machine is small enough that this is a hand-edit (the agent does it, but no `ark agent` command wraps it).

**Gate.** Verdict *Approved*, zero open CRITICAL → EXECUTE.

## EXECUTE — implement

**Purpose.** Work through the latest PLAN's Implementation phases. If implementation reveals design gaps, **update the latest PLAN's `## Spec` section** to reflect reality. Don't silently diverge.

**Calls.**
- `ark context --scope phase --for execute` — git dirty files + current task + latest PLAN + project specs.
- `ark agent task execute` — transitions to EXECUTE.

**Gate.** Implementation complete; project's checks pass; code committed.

**Worktree note.** If the task was created with `--worktree`, all phase commands operate on the *worktree's* `.ark/`. `cd .ark/worktrees/<branch>/` and run them there. After merging the branch, run `ark agent task worktree cleanup --slug <s> [--delete-branch]` from the parent to remove the dir. Archive does NOT auto-clean.

## VERIFY — post-execute gate (single-pass)

**Purpose.** Verify the shipped code against PRD's Outcome and PLAN's Validation. Apply the higher quality bar: plan fidelity, correctness, code quality, organization, abstraction, SPEC drift.

**Calls.**
- `ark context --scope phase --for verify` — current task with PRD + latest PLAN + VERIFY.md (if exists) + git state.
- `ark agent task verify` — transitions to VERIFY and seeds `VERIFY.md`.

**Gate.** Verdict *Approved* or *Approved with Follow-ups* → tell the user to run `/ark:archive`. *Rejected* → halt for user decision.

VERIFY is **single-pass**. Unlike REVIEW, it doesn't loop. If the verdict is rejected, you decide: create fix tasks, promote tier with `ark agent task promote`, accept with acknowledgement, or discard.

## ARCHIVE — preserve as memory

**Purpose.** Move the task to `tasks/archive/YYYY-MM/<slug>/`. Deep tier additionally extracts the final PLAN's `## Spec` section to `specs/features/<name>/SPEC.md` and registers it in the features INDEX.

**Calls.**
- `ark agent task archive` — moves the dir; on deep tier, internally invokes `ark agent spec extract` and `ark agent spec register`. If a workspace developer is initialized, also appends a `task` entry to that developer's journal under `.ark/workspace/<dev>/`. Disable globally via `[workspace].auto_record_on_archive = false` in `.ark/config.toml`.

**Trigger.** `/ark:archive`. The `/ark:design` and `/ark:quick` commands deliberately stop at VERIFY (or EXECUTE for quick); you decide when to close out.

**Reopen.** Move the archived dir back to `.ark/tasks/<slug>/` and reset `phase = "design"` + clear `archived_at` in `task.toml`. Refuses if a same-slug active task already exists.

For non-task work — research, debugging, doc edits — invoke `/ark:record [<title>]` to append a `manual` entry to the same journal. Mirrors the auto-record path archive uses.
