# `ark-workflow-refactor` REVIEW `02`

> Status: Closed
> Feature: `ark-workflow-refactor`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
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
- Blocking Issues: 3
- Non-Blocking Issues: 2

## Summary

Iteration 02 fixes the major durability problems from round 01: no
`committed_head` dirty residue, targeted staging, targeted rollback, and SPEC
files included in the rollback set. The remaining blocker is traceability. The
new "recover closing SHA from `git log -n 1 -- <journal-path>`" invariant is
not reliable because workspace journals are shared files that later task or
manual records can modify. The PRD also still contains older exact-range
wording in several places, and the rollback control flow needs to explicitly
cover failures during SPEC extraction/register, not only `git commit` failure.

## Findings

### R-201 `journal-path git log does not identify a specific task commit`

- Severity: CRITICAL
- Section: [**Spec**] G-5; [**Trade-offs**] T-1/T-8; PRD Outcome item 7
- Problem:
  The plan says the closing SHA is recoverable with
  `git log -n 1 -- <journal-path>` because the journal file is touched only by
  task-closing commits. That premise is false for the current workspace model:
  `.ark/workspace/<dev>/journal-N.md` is a shared rolling journal. Manual
  `/ark:record` writes to the same file, and later task commits can append to
  the same `journal-N.md` before rotation.
- Why it matters:
  After any later journal entry, `git log -n 1 -- journal-N.md` returns the
  later journal-writing commit, not the commit that introduced this task's
  entry. That means the promised complete range
  `<start_head>..<closing-commit-sha>` is not recoverable for the task, so the
  replacement for the impossible inline SHA still fails the PRD's traceability
  goal.
- Recommendation:
  Use a recovery mechanism tied to the entry, not only the file path. Options:
  write each task closure to a task-specific journal artifact/path, require a
  unique entry marker and document a `git log -S/-G <marker> -- <journal-path>`
  lookup, persist a non-self-referential committed identifier that can map back
  to the closing commit, or revise the PRD to drop exact closing-SHA recovery.
  Add validation with a later manual `/ark:record` and a later task commit
  touching the same journal file, proving the original task's closing commit is
  still recoverable.

### R-202 `PRD still contains stale exact-range wording`

- Severity: HIGH
- Section: PRD [**What**], PRD Outcome item 2, PRD [**Related Specs**] workspace row; [**Log**] R-104 resolution
- Problem:
  `02_PLAN` says the PRD was updated to authorize the new journal shape, but the
  PRD still says the feature will "capture an exact commit range via
  `start_head`", says `/ark:commit` writes a journal entry "with exact range
  `start_head..HEAD`", and the workspace related-spec row still names a
  `commit_range = "<start_head>..<HEAD>"` field.
- Why it matters:
  REVIEW and VERIFY use the PRD as the contract. Leaving contradictory wording
  means an executor can satisfy the PLAN while still failing the PRD, and a
  verifier has no single source of truth for the journal contract.
- Recommendation:
  Update every PRD occurrence, not only Outcome item 7. The PRD should clearly
  state the chosen invariant and should stop naming an inline `commit_range`
  field unless the implementation will actually render one.

### R-203 `rollback control flow does not consistently wrap SPEC failures`

- Severity: HIGH
- Section: [**Spec**] G-3 steps 4-5/8; [**Runtime**] Failure Flow steps 4-6; [**Constraints**] C-4/C-15
- Problem:
  The rollback set snapshots SPEC files before extraction, and the failure flow
  says SPEC extraction failure triggers restore. But the main algorithm only
  describes rollback in step 8f, after staging and `git commit` failure. It does
  not explicitly state that failures from `spec_extract`, `spec_register`,
  journal append, workspace index rerender, task TOML save, or `git add` all go
  through the same restore path once their corresponding snapshots exist.
- Why it matters:
  The most important R-101 fix depends on cleanup running for partial
  `spec_extract`/`spec_register` failures, not just pre-commit hook rejection.
  If implementation follows the call graph literally, partial SPEC/INDEX writes
  can still leak.
- Recommendation:
  Define rollback as a scoped guard/state machine: after each snapshot is taken,
  any subsequent error before successful `git commit` restores the snapshots
  that exist. Name exactly which steps trigger rollback and which do not. Add
  tests for `spec_extract` failure after creating a SPEC file and
  `spec_register` failure after modifying the features INDEX.

### R-204 `post-commit clean-tree claim conflicts with staged-only behavior`

- Severity: MEDIUM
- Section: [**Summary**], G-3, C-23, Runtime Main Flow, Validation V-UT-17/V-IT-1
- Problem:
  The plan repeatedly says the working tree is clean after successful
  `/ark:commit`, but the staged-only model intentionally leaves unrelated
  pre-existing unstaged user files untouched. C-23 partially acknowledges this
  by saying user unstaged files may still show, while validation still asks for
  `git status --porcelain` to be empty.
- Why it matters:
  This creates a false success criterion. A correct implementation that protects
  unrelated unstaged files will not always leave the whole worktree clean.
- Recommendation:
  Change the invariant to "no Ark-introduced dirty files remain" unless
  `/ark:commit` also refuses to run with unrelated unstaged files. Update
  summary text, slash-command wrap-up, C-23, and validation accordingly.

### R-205 `archive helper naming is inconsistent`

- Severity: LOW
- Section: [**Log**] G-9a; [**Spec**] G-9; [**API Surface**]
- Problem:
  The plan alternates between `ark agent task archive-move` and `ark agent task
  archive` for the hidden helper. The API surface shows
  `Archive(TaskArchiveMoveCliArgs)`, while the log calls the subcommand
  `archive-move`.
- Why it matters:
  This is small, but CLI naming is copied into templates, docs, and tests. A
  mismatch here can produce failing command examples or duplicate hidden verbs.
- Recommendation:
  Pick one hidden subcommand name and use it consistently. If preserving the
  `ark-agent-namespace` verb set is the goal, prefer `ark agent task archive`
  with side-effect-free semantics and document that the Rust helper is named
  `task_archive_move`.

## Trade-off Advice

### TR-1 `task commit traceability`

- Related Plan Item: `T-1`, `T-8`
- Topic: Traceability vs Single-Commit Closure
- Reviewer Position: Need More Justification
- Advice:
  Keep the single-commit closure if desired, but do not claim task-level closing
  SHA recovery from a shared journal file path. Tie recovery to a unique entry,
  a task-specific path, or a documented weaker traceability guarantee.
- Rationale:
  The self-referential SHA problem is real, but the replacement invariant must
  survive later journal writes.
- Required Action:
  Revise G-5 and PRD Outcome item 7 around a recovery method that remains valid
  after later manual and task journal entries.

### TR-2 `rollback implementation shape`

- Related Plan Item: `T-3`
- Topic: Robustness vs Simplicity
- Reviewer Position: Prefer Scoped Rollback Guard
- Advice:
  Implement rollback as a scoped guard that accumulates snapshots and restores
  them on any pre-commit error.
- Rationale:
  A long linear sequence with rollback only shown at `git commit` failure is too
  easy to implement incompletely.
- Required Action:
  Add the guard or equivalent explicit control-flow design to the next PLAN and
  test partial failure at SPEC and INDEX mutation points.
