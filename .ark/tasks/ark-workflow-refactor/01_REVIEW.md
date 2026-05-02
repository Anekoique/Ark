# `ark-workflow-refactor` REVIEW `01`

> Status: Closed
> Feature: `ark-workflow-refactor`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Rejected
- Blocking Issues: 4
- Non-Blocking Issues: 2

## Summary

Iteration 01 resolves the main `00_PLAN` impossibility by removing the
self-referential journal SHA and splitting archive side effects. The revised
plan is still not ready for execution. The new rollback protocol does not
rollback deep-tier SPEC extraction, the commit protocol silently stages every
dirty file and destroys the caller's index shape on failure, and the new journal
shape no longer satisfies the PRD's exact task commit range requirement.

## Findings

### R-101 `commit rollback leaves deep SPEC side effects behind`

- Severity: CRITICAL
- Section: [**Spec**] G-3; [**Runtime**] Failure Flow step 6; [**Implementation**] Phase 2
- Problem:
  `task_commit` runs `spec_extract` and `spec_register` before journal append,
  task TOML save, staging, and `git commit`. On `git commit` failure, the
  rollback path restores `task.toml`, truncates the workspace journal, restores
  the workspace index, and runs `git reset`, but it does not restore
  `specs/features/<slug>/SPEC.md` or `specs/features/INDEX.md`.
- Why it matters:
  A failed deep-tier `/ark:commit` can leave promoted SPEC content and feature
  INDEX changes in the worktree while the task remains in `Verify`. That breaks
  the claimed atomic closure and can produce stale or duplicate CHANGELOG/INDEX
  edits on retry.
- Recommendation:
  Either move SPEC extraction/register after a successful commit as an explicit
  second phase, or snapshot and restore every file it can touch during rollback.
  Add failure tests proving a rejected git commit leaves feature SPEC files and
  `specs/features/INDEX.md` unchanged.

### R-102 `git add -A violates the staged-work contract`

- Severity: HIGH
- Section: [**Spec**] G-3; [**Constraints**] C-4; PRD Outcome item 2 and item 3
- Problem:
  The PRD says `/ark:commit` generates its message from the staged diff and a
  clean tree errors with "task commit requires staged work." The revised plan
  checks only `git status --porcelain`, then runs `git add -A` from `task_cwd`,
  capturing every dirty file in the checkout.
- Why it matters:
  The command can commit unrelated unstaged user edits. This is especially risky
  in a shared checkout with multiple active Ark tasks, and contradicts the
  user's stated staged-work workflow.
- Recommendation:
  Make the precondition and staging model explicit: require an existing staged
  diff for user work, stage only Ark-generated closure artifacts, and leave
  unrelated unstaged files untouched. If `git add -A` is still desired, revise
  the PRD and slash-command UX to make "commit everything dirty" the contract.

### R-103 `rollback destroys the user's original index state`

- Severity: HIGH
- Section: [**Runtime**] Failure Flow step 6; [**Constraints**] C-4
- Problem:
  On commit failure, rollback runs plain `git reset` after `git add -A`. That
  unstages everything, including files the user had deliberately staged before
  invoking `/ark:commit`.
- Why it matters:
  A failed pre-commit hook should not lose the user's staging intent. With the
  current plan, a retry after fixing the hook may commit a different set of
  files unless the user manually reconstructs the index.
- Recommendation:
  Preserve and restore the index state, or avoid broad staging in the first
  place. At minimum, add tests where the user has a staged file and an unstaged
  file before a failing hook, then assert rollback restores both worktree and
  index state exactly.

### R-104 `journal no longer records the PRD-required exact range`

- Severity: HIGH
- Section: [**Spec**] G-5; [**Trade-offs**] T-1; PRD Outcome item 7
- Problem:
  The revised journal entry records `Start Head` and `Base Branch`, and the
  commits table is computed before the task-closing commit. It omits the
  just-created commit from the table and no longer renders
  `commit_range = "<start_head>..<HEAD>"` in the journal entry.
- Why it matters:
  The PRD explicitly asks for exact journal commit range and says `/ark:commit`
  writes the journal entry with `commit_range = "<start_head>..<HEAD>"` where
  `HEAD` is the just-created commit. The plan's new invariant may be more
  implementable, but it is a scope change against the accepted PRD and should
  not be hidden inside the PLAN.
- Recommendation:
  Either update the PRD/Outcome to authorize the weaker journal shape, or choose
  a protocol that preserves an exact range without violating Git's object model
  (for example, store the exact range in committed task metadata instead of the
  journal, and revise all journal wording accordingly). The next PLAN should
  explicitly call out the PRD contract change.

### R-105 `committed_head dirty state undermines bulk archive correctness`

- Severity: MEDIUM
- Section: [**Spec**] G-2/G-3/G-6; [**Runtime**] Main Flow steps 4-6
- Problem:
  `committed_head` is written as an intentional post-commit dirty edit. The
  plan then allows time to pass and later runs `ark archive`, but does not state
  whether archive requires that dirty metadata edit to be committed, preserved,
  ignored, or included in a later PR-review commit.
- Why it matters:
  Bulk archive will move the active task directory, including an uncommitted
  `task.toml` edit, into `tasks/archive/YYYY-MM/<slug>/`. If that edit was never
  committed before the move, the archive operation itself becomes responsible
  for preserving or committing task-close metadata, which the PRD assigns to
  `/ark:commit`.
- Recommendation:
  Define the lifecycle rule for `committed_head`: either it must be committed by
  a follow-up operation before `ark archive`, it is local-only and may be absent
  from history, or it should be dropped. Add validation around `ark archive`
  behavior with a dirty `committed_head` field.

### R-106 `archive helper deletion may break internal recovery/debugging surface`

- Severity: LOW
- Section: [**Spec**] G-9; [**Trade-offs**] T-2; Related Spec `ark-agent-namespace`
- Problem:
  `01_PLAN` removes `ark agent task archive` entirely. The previous plan kept it
  hidden but available; the `ark-agent-namespace` SPEC currently lists `task
  archive` as an explicit phase transition subcommand.
- Why it matters:
  The `ark agent` namespace is not semver-stable, so removal is allowed, but it
  is still an internal workflow contract used by templates and debugging. The
  plan should state how maintainers perform a one-off archive move when
  top-level bulk archive is not the right tool.
- Recommendation:
  Either keep a hidden `ark agent task archive-move`/`archive` command that
  calls the side-effect-free helper, or explicitly revise the
  `ark-agent-namespace` SPEC expectations and document top-level `ark archive
  --month` as the only supported path.

## Trade-off Advice

### TR-1 `closure atomicity boundary`

- Related Plan Item: `T-1`
- Topic: Atomicity vs Traceability
- Reviewer Position: Need More Justification
- Advice:
  Define one durable atomic boundary and make every artifact obey it. Right now
  work + journal + pre-commit task TOML are in the closing commit, SPEC files
  are pre-commit but not rollback-covered, and `committed_head` is post-commit
  dirty metadata.
- Rationale:
  Three different durability classes make failure recovery and archive behavior
  hard to reason about.
- Required Action:
  Revise the plan so each closure artifact is either committed atomically,
  rollback-covered, or explicitly local-only with a documented later owner.

### TR-2 `staged-only workflow`

- Related Plan Item: `C-4`
- Topic: Safety vs Convenience
- Reviewer Position: Prefer staged-only
- Advice:
  Prefer requiring user work to already be staged and only staging Ark-generated
  closure files internally.
- Rationale:
  This matches the PRD language, protects unrelated edits, and gives the agent's
  generated commit message a stable input.
- Required Action:
  Replace `git add -A` with a narrower staging protocol, or revise the PRD and
  add strong warnings/tests for all-dirty-file capture.
