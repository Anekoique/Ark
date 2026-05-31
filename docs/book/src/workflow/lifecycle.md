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
       │   COMMIT   │  atomic close: deep extracts SPEC → specs/features/<path>/;
       └─────┬──────┘  single git commit; rollback on any pre-commit failure
             ▼
       ┌────────────┐
       │  ARCHIVE   │  bulk-move Committed tasks to tasks/archive/YYYY-MM/<tier>/
       └────────────┘  via top-level `ark archive`
```

State is recorded in `task.toml.phase`. Each transition is mediated by a CLI command (`ark agent task plan`, `... review`, `... execute`, `... verify`, `... commit`, `... archive`); illegal transitions error out with `IllegalPhaseTransition` rather than silently corrupting state.

**Research tier is the exception.** A research task has its own short lifecycle — `research → committed → archived` — with no PLAN, REVIEW, EXECUTE, VERIFY, or SPEC promotion. `ark agent task commit` stages `task.toml`, `PRD.md`, and every file under `research/` recursively. See [Tiers → Research](./tiers.md) and [Subagents](./subagents.md).

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

**Rule.** `## Spec` must be self-contained every iteration (deltas go in `## Log`). It's copied verbatim to `specs/features/<path>/SPEC.md` on commit (deep tier).

## REVIEW — pre-execute gate (deep only, iterative)

**Purpose.** Evaluate the latest `NN_PLAN.md` against PRD and project specs; write `NN_REVIEW.md` with verdict + findings. Loop until verdict is *Approved* with zero open CRITICAL.

**Calls.**
- `ark context --scope phase --for review` — current task, latest PLAN, related feature specs, project specs.
- `ark agent task review` — transitions PLAN → REVIEW and seeds `NN_REVIEW.md`.

**Who reviews.** At REVIEW entry the slash command stops and asks you to pick the reviewer: the `ark-reviewer` subagent, a different model, or self-review. The chosen reviewer fills `NN_REVIEW.md`; it never edits the PLAN or code. See [Subagents](./subagents.md).

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

**Who verifies.** At VERIFY entry the slash command stops and asks you to pick the verifier: the `ark-verifier` subagent, a different model, or self-verify. The verifier runs the project's build / test / lint / format-check and fills `VERIFY.md`; it does **not** self-fix — FAIL items return to the main session. See [Subagents](./subagents.md).

**Gate.** Verdict *Approved* or *Approved with Follow-ups* → tell the user to run `/ark:commit`. *Rejected* → halt for user decision.

VERIFY is **single-pass**. Unlike REVIEW, it doesn't loop. If the verdict is rejected, you decide: create fix tasks, promote tier with `ark agent task promote`, accept with acknowledgement, or discard.

## COMMIT — atomic close

**Purpose.** Land all pending work plus the closing `task.toml` flip in a single git commit. Deep tier additionally extracts the final PLAN's `## Spec` to `specs/features/<path>/SPEC.md` and registers it in the features INDEX *inside the same commit*. A scoped rollback restores every touched file if any pre-commit step fails.

**Calls.**
- `ark agent task commit` — VERIFY gate, deep-tier SPEC extract + register, single git commit covering work + `task.toml` + (deep) SPEC + features INDEX. Rolls back on any pre-commit failure.

**Trigger.** `/ark:commit`. The `/ark:design` and `/ark:quick` commands deliberately stop short of commit; you decide when to close out.

## ARCHIVE — preserve as memory

**Purpose.** Move every `phase = Committed` task into `tasks/archive/YYYY-MM/<tier>/<slug>/`, where the month is derived from the task's own `committed_at` timestamp. Side-effect-free: no SPEC promotion, no git work — that already happened at COMMIT time.

**Calls.**
- `ark archive` (top-level, manager-only) — bulk-archives every committed task. `--month YYYY-MM` filters; `--dry-run` lists candidates without moving anything.
- `ark agent task archive --slug <s>` — single-task variant, used internally and as a manual escape hatch.

**Reopen.** Move the archived dir back to `.ark/tasks/<slug>/` and reset `phase = "verify"` + clear `archived_at` in `task.toml` — the task lands back at the commit-able gate. Refuses if a same-slug active task already exists.
