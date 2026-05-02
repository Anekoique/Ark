# `ark-workflow-refactor` REVIEW `00`

> Status: Closed
> Feature: `ark-workflow-refactor`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
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
- Blocking Issues: 5
- Non-Blocking Issues: 2

## Summary

The plan is directionally aligned with the PRD, and the lifecycle split
`verify -> commit -> archive` is the right shape. It is not executable as
written. The current commit protocol leaves post-commit mutations outside the
commit it claims is atomic, `ark archive` would re-run side effects that moved
to `/ark:commit`, and the top-level archive API cannot honor the required
`committed_at` month bucket. These are design-level problems that should be
fixed in the next PLAN before implementation starts.

## Findings

### R-001 `self-referential commit range is impossible in one commit`

- Severity: CRITICAL
- Section: [**Spec**] G-3, G-5, C-4; [**Runtime**] Main Flow; [**Trade-offs**] T-1/T-6
- Problem:
  The plan requires one task-closing commit to contain the workspace journal
  entry, and also requires that journal entry to contain the final
  `start_head..<HEAD>` range where `<HEAD>` is the hash of that same commit.
  The proposed sequence writes `<HEAD-PENDING>`, runs `git commit`, then patches
  the journal file with the real SHA after the commit lands.
- Why it matters:
  A Git commit hash is computed from the tree it records. Any post-commit patch
  to the journal file is not part of that commit and leaves the working tree
  dirty. If the patched file were included by amending, the commit hash would
  change again. As written, the plan cannot satisfy "single git commit covering
  work + journal entry" and "journal contains the exact final HEAD SHA" at the
  same time.
- Recommendation:
  Pick one invariant and make the plan explicit. Viable choices are: store a
  non-self-referential value in the committed journal entry, accept an amend,
  accept a second journal-fix commit, or make the exact SHA a post-commit
  working-tree record that is intentionally not part of the task commit. Update
  G-3, G-5, C-4, T-1/T-6, and validation to match the chosen invariant.

### R-002 `task.toml committed phase is saved after the commit`

- Severity: CRITICAL
- Section: [**Spec**] G-3; [**Runtime**] Main Flow step 4; [**Implementation**] Phase 2
- Problem:
  `task_commit` saves `task.toml.phase = Committed` and `committed_at = now`
  after running `git commit`. That means the task state transition is not part
  of the task-closing commit and the worktree remains dirty immediately after a
  successful `/ark:commit`.
- Why it matters:
  The PRD frames `/ark:commit` as the final committing action for a task. A
  successful close that leaves `task.toml` uncommitted contradicts that model
  and creates confusing follow-on behavior for review, archive, and journal
  range accounting.
- Recommendation:
  Redesign the commit protocol so task state is either intentionally included in
  the task-closing commit, or explicitly documented as a post-commit local
  mutation with a second required commit/record path. Do not leave this as an
  accidental dirty-file side effect.

### R-003 `bulk archive would duplicate SPEC promotion and journal recording`

- Severity: CRITICAL
- Section: [**Spec**] G-3, G-6, C-18; [**Architecture**] Module coupling; [**Implementation**] Phase 3
- Problem:
  The plan moves deep-tier SPEC extraction and workspace journal recording to
  `task_commit`, but also says top-level `ark archive` should call the existing
  `task_archive` helper. The existing helper still performs deep SPEC
  promotion and calls `record_task` after moving the task.
- Why it matters:
  A deep task would promote/register the same SPEC at commit time and again at
  bulk-archive time, potentially appending duplicate CHANGELOG rows. Every tier
  would also risk a second task journal entry during `ark archive`, despite the
  PRD making `/ark:commit` the closure/journal step.
- Recommendation:
  Split the archive implementation into a move/state-cleanup helper with no
  SPEC or workspace side effects, or add an explicit mode that suppresses those
  side effects when called by top-level `ark archive`. Add tests proving
  `ark archive` does not write a journal entry and does not re-promote a SPEC.

### R-004 `archive month cannot be derived from committed_at through the proposed API`

- Severity: HIGH
- Section: [**Spec**] G-6; [**Runtime**] Main Flow step 6; [**Implementation**] Phase 3
- Problem:
  `ark archive` is required to place tasks under
  `.ark/tasks/archive/YYYY-MM/<slug>/` using each task's `committed_at`
  timestamp. The plan calls the existing `task_archive(TaskArchiveOptions {
  project_root, slug })`, whose current behavior derives the archive bucket from
  `Utc::now()`. The proposed `TaskArchiveOptions` still has no timestamp,
  month, or destination override.
- Why it matters:
  Manager bulk archive can happen days or months after `/ark:commit`. Using
  archive-run time instead of commit time violates the PRD and makes archive
  layout nondeterministic for accumulated committed tasks.
- Recommendation:
  Add a manager-only archive helper/API that accepts the effective archive
  timestamp or destination month derived from `committed_at`, and update
  `task_archive` tests to cover a task committed in a previous month.

### R-005 `commit projection contradicts ark-context's no-body contract`

- Severity: HIGH
- Section: [**Spec**] C-17; Related Spec `ark-context` G-4/G-5/G-7
- Problem:
  The plan adds `ark context --scope phase --for commit` and states that the
  projection includes the latest `VERIFY.md` content body. The `ark-context`
  feature SPEC says context payloads carry paths and summaries only, never file
  bodies, and its phase projections are additive over a stable schema.
- Why it matters:
  `ark context` is a visible, semver-covered command with an additive-only JSON
  schema. Adding artifact bodies changes a core contract and increases output
  size/duplication when slash commands can read the returned path directly.
- Recommendation:
  Keep the commit projection body-free and have `/ark:commit` read `VERIFY.md`
  from the artifact path, or explicitly revise the `ark-context` SPEC/schema
  with a compatibility story. The next PLAN should name that choice.

### R-006 `JournalPatchFailed recovery path is illegal`

- Severity: MEDIUM
- Section: [**Runtime**] Failure Flow step 7; [**Spec**] G-3 phase precondition
- Problem:
  The failure flow says that if journal patching fails, the commit lands, the
  phase transitions to `Committed`, and the user may re-run
  `task_commit --no-commit`. The phase precondition for `task_commit` rejects
  `Committed`; only `Quick/Execute` and `Standard|Deep/Verify` are legal inputs.
- Why it matters:
  The documented recovery command cannot run. A patch failure would strand the
  user with a committed task and an unresolved `<HEAD-PENDING>` token unless
  they hand-edit state or journal content.
- Recommendation:
  Either remove the retry guidance and require manual repair, or define an
  idempotent repair path that is legal from `Committed` and only patches the
  journal/token state without re-running commit or SPEC extraction.

### R-007 `concurrent journal writes can duplicate session numbers`

- Severity: LOW
- Section: [**Validation**] V-E-6; Related Spec `workspace` G-4/G-5
- Problem:
  V-E-6 says concurrent `task_commit` calls are safe because journal appends are
  atomic and "the index re-render is serialized by the existing rerender lock."
  There is no workspace index/journal lock today. `write_journal_and_index`
  computes `highest_session + 1` before append, so two concurrent writers can
  pick the same session number even if both append operations are individually
  atomic.
- Why it matters:
  The journal parser treats `## Session N` as an anchor. Duplicate session
  numbers make the index ambiguous and can break the proposed
  `patch_head_pending` anchor selection.
- Recommendation:
  Either scope concurrent `/ark:commit` out and remove V-E-6, or add a real
  workspace journal lock around session-number assignment, append, and index
  re-render.

## Trade-off Advice

### TR-1 `journal exactness vs one-commit closure`

- Related Plan Item: `T-1`, `T-6`
- Topic: Traceability vs Atomicity
- Reviewer Position: Need More Justification
- Advice:
  Do not keep both "one commit contains the journal" and "journal names that
  commit's final SHA" as hard requirements. They conflict under Git's object
  model.
- Rationale:
  The current workaround gives neither a clean one-commit close nor a committed
  exact SHA. A less elegant but truthful invariant is better than a protocol
  that leaves hidden dirty state.
- Required Action:
  Revise the PLAN around the selected invariant and update validation to assert
  the resulting clean/dirty worktree behavior.

### TR-2 `archive helper reuse`

- Related Plan Item: `T-3`
- Topic: Compatibility vs Correct Side Effects
- Reviewer Position: Prefer Option B
- Advice:
  Prefer extracting a side-effect-free archive movement helper over reusing
  `task_archive` unchanged.
- Rationale:
  The old helper's value is exactly the behavior this refactor is moving out of
  archive: SPEC promotion and auto-record. Preserving that behavior creates
  duplicate writes and makes the new lifecycle harder to reason about.
- Required Action:
  Introduce an internal move/archive primitive or a narrowly-scoped mode flag,
  then document which callers are allowed to trigger SPEC/journal side effects.

### TR-3 `context projection bodies`

- Related Plan Item: `C-17`
- Topic: Stable API Simplicity vs Convenience
- Reviewer Position: Prefer Option A
- Advice:
  Keep `ark context` projections body-free and let slash commands read artifact
  files by path.
- Rationale:
  This preserves the stable context contract and avoids turning `ark context`
  into an artifact transport. The caller already runs inside the checkout and
  can read `VERIFY.md` directly.
- Required Action:
  Remove VERIFY body inclusion from the commit projection, or explicitly treat
  this as an `ark-context` SPEC revision with schema/version handling.
