# `task-concurrency-control` REVIEW `02`

> Status: Closed
> Feature: `task-concurrency-control`
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
- Blocking Issues: 2
- Non-Blocking Issues: 1

## Summary

PLAN 02 fixes the main round-01 design problems: GC ownership is now one-way, `task new` warning semantics are explicit, archive returns to rename-first SPEC safety, and the session provider has a trait-shaped test seam.

Two execution blockers remain. First, the `Ppid` test seam is not consistently threaded to the public task operations, so the promised multi-session integration tests cannot drive deterministic PPIDs through `task_new` / phase commands. Second, archive cleanup runs after rename, but `state_mutate` reconciles before the cleanup closure; that reconcile pass can delete the very focused session entry the closure needs in order to release the cache file.

## Findings

### R-001 `Ppid` injection stops below the command APIs

- Severity: HIGH
- Section: `[**API Surface**]`, `[**Implementation**]`, `[**Validation**]`
- Problem:
  PLAN 02 says `load_state`, `state_mutate`, and `resolve_session_id` accept `&dyn Ppid` (`02_PLAN.md:361-379`), and V-IT-1 relies on `StubPpid(12345)` / `StubPpid(67890)` to simulate two sessions (`02_PLAN.md:630`). But the public task operations still construct `RealPpid` internally: `task_resume`, `task_discard`, plus existing `task_new` and `task_archive` are described as constructing `RealPpid` as a default (`02_PLAN.md:381-390`). That means integration tests cannot drive two deterministic PPIDs through the same command-level paths that create tasks, set focus, and resolve slugless commands.
- Why it matters:
  G-2's central contract is multi-session focus isolation. If the seam only exists below the command layer, tests can verify cache helpers but not the actual lifecycle commands. The plan also contradicts itself: the summary says integration tests pass stubs, while the API surface says task operations do not accept them.
- Recommendation:
  Thread `&dyn Ppid` through the core command functions or through their options in a testable way. A pragmatic shape is: exported command functions keep current signatures and call `*_with_ppid(..., &RealPpid)` internal helpers; tests call the helpers with `StubPpid`. Update API surface and validation so V-IT-1 exercises `task_new`, phase resolution, `resume`, and archive/discard through the same state path.

### R-002 Archive reconcile can remove the focused session before cache release

- Severity: HIGH
- Section: `[**Runtime**]`, `[**Constraints**]`, `[**State Transitions**]`
- Problem:
  C-22 moves archive state cleanup after rename and SPEC promotion (`02_PLAN.md:417`). `state_mutate` always runs reconciliation before the edit closure (`02_PLAN.md:430-435`, `02_PLAN.md:493`). After rename, the active task directory no longer exists under `.ark/tasks/<slug>`, so C-19's reconcile pass drops the slug from `active` and drops sessions whose focus points at that slug before the archive cleanup closure runs (`02_PLAN.md:414`). The closure then checks whether the current session focus matches the slug before calling `release_session_id`, but that session entry may already be gone (`02_PLAN.md:207-216`). Result: archive removes the session entry without deleting the temp cache file.
- Why it matters:
  The lifecycle says archiving the focused task releases the session and deletes its cache (`02_PLAN.md:497-501`), and V-IT-5 expects archive to release the cache. Leaving stale cache files also increases the PPID-recycling risk the design is trying to bound.
- Recommendation:
  Make archive cache release independent of the post-rename reconcile side effect. For example, resolve and release the current session id before entering `state_mutate`, or add a `state_mutate_with_mode` / cleanup path that captures pre-reconcile focus before pruning. Then update Scenario D, C-22, and V-IT-5/V-IT-9 to assert the cache file is deleted even when the task dir has already moved to archive.

### R-003 Archive failure after durable promotion returns an error with unclear workflow recovery

- Severity: MEDIUM
- Section: `[**Failure Flow**]`, `[**Validation**]`
- Problem:
  V-IT-9 expects `task_archive` to return `Error::StateLockContended` after rename, archived metadata save, and SPEC promotion have all succeeded (`02_PLAN.md:639`). The failure flow says state self-heals on the next read, but the command still reports failure and `record_task` is skipped. A retry against the original slug will likely see no active task dir, yet the validation only says re-running archive is "a no-op or returns already archived" without specifying a command behavior.
- Why it matters:
  This leaves the user workflow ambiguous: the archive is structurally complete enough for SPEC promotion, but the CLI says failure. The workspace journal may be missing, and the next action is not defined.
- Recommendation:
  Specify the intended summary/error semantics for post-promotion cleanup failure. Either downgrade state cleanup failure to a warning plus successful archive summary, or keep the error but document the exact recovery command and expected retry result. Adjust V-IT-9 accordingly.

## Trade-off Advice

### TR-1 Keep the trait seam, but expose it where behavior is tested

- Related Plan Item: `T-9`
- Topic: Testability vs API Simplicity
- Reviewer Position: Prefer Option A with Revisions
- Advice:
  Use helper functions or option fields to inject `Ppid` into command-level paths under tests.
- Rationale:
  The trait is the right primitive; it just stops one layer too low.
- Required Action:
  Update command APIs/helpers and V-IT-1 before execution.

### TR-2 Reconcile-before-closure is fine, but archive needs pre-reconcile session knowledge

- Related Plan Item: `C-19`, `C-22`
- Topic: Safety vs Simplicity
- Reviewer Position: Need More Justification
- Advice:
  Keep global reconcile-before-edit semantics, but do not make archive cache cleanup depend on session entries that reconcile is allowed to prune.
- Rationale:
  Rename-first archive is the right choice for SPEC safety; the cache-release logic needs to account for that ordering.
- Required Action:
  Add an explicit archive cache-release step or state mutation mode.
