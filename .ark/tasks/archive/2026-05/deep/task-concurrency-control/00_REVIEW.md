# `task-concurrency-control` REVIEW `00`

> Status: Closed
> Feature: `task-concurrency-control`
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
- Blocking Issues: 4
- Non-Blocking Issues: 0

## Summary

The PLAN is directionally coherent and correctly identifies the main blast radius, but it is not ready for execution. The current session-id design depends on a non-existent cross-platform std API, the state reconciliation model is one-way despite claiming `task.toml` remains the source of truth, `.state.toml` has no unload/snapshot privacy policy even though it absorbs `.developer`, and the worktree discovery rewrite would lose active worktree tasks once `.current` disappears.

The next PLAN should keep the same overall architecture only after fixing those invariants and adding tests that exercise state loss, no-session worktree listing, and unload/load behavior.

## Findings

### R-001 Session identity API is not cross-platform and does not compile

- Severity: CRITICAL
- Section: `[**Architecture**]`, `[**Runtime**]`, `[**Constraints**]`
- Problem:
  The PLAN uses `std::process::parent_id()` in the `task new` call graph and runtime flow, and defines cache files around PPID lookup (`00_PLAN.md:131`, `00_PLAN.md:402`, `00_PLAN.md:271`). On the current pinned nightly, `std::process::parent_id` is not available; the compiler suggests the Unix-only `std::os::unix::process::parent_id`. That violates the PRD's Linux/macOS/Windows outcome and the PLAN's own cross-platform claim.
- Why it matters:
  This is a compile-time blocker before correctness is even tested. Switching to the Unix extension would make Windows support impossible, while switching to process id would break the "same shell keeps focus across invocations" behavior.
- Recommendation:
  Revise the session-id design before implementation. Either define a real cross-platform parent/session provider with `cfg`-gated implementations and Windows coverage, or change the workflow substrate to a portable Ark session token. The PLAN must name the implementation API, failure behavior, and tests for Linux/macOS/Windows.

### R-002 Reconcile only drops entries, so the state file is not a true index

- Severity: HIGH
- Section: `[**Spec**]`, `[**Runtime**]`, `[**Validation**]`
- Problem:
  G-1 says truth remains in `.ark/tasks/<slug>/task.toml` and `.state.toml` is only an index reconciled on every read (`00_PLAN.md:26`). But `reconcile.rs` is specified only to drop missing/archived entries (`00_PLAN.md:59`, `00_PLAN.md:375`), and the tests only cover drops (`00_PLAN.md:570`). It never enumerates non-archived task directories to add missing active slugs.
- Why it matters:
  If `.state.toml` is deleted, corrupted, skipped during unload/load, or mutated before an archive rename fails, active task directories can become invisible to `resume`, context, and slugless commands. That contradicts the core invariant that task dirs are the source of truth.
- Recommendation:
  Make reconciliation two-way: enumerate `.ark/tasks/<slug>/task.toml` excluding archive, add every non-archived task to `tasks.active`, then drop archived/missing entries and invalid sessions. Add tests for "state missing but task dir exists", "state active omits existing task dir", and the archive-rename-failure recovery path.

### R-003 `.state.toml` has no unload/snapshot privacy policy

- Severity: HIGH
- Section: `[**Architecture**]`, `[**Implementation**]`, `[**Spec Alignment**]`
- Problem:
  The PLAN replaces `.ark/.developer` with `.ark/.state.toml` identity storage (`00_PLAN.md:29`, `00_PLAN.md:174`) and only adds `.state.toml` to `.gitignore` (`00_PLAN.md:94`, `00_PLAN.md:528`). It does not update `commands/unload.rs` or snapshot capture. The existing workspace SPEC explicitly requires skipping the developer identity file in both unload walks because capturing it would leak identity (`workspace/SPEC.md:609`).
- Why it matters:
  After this change, unload/load may capture developer identity and session UUIDs in `.ark.db`, or, if later skipped ad hoc, may lose active-task state unless R-002 is fixed. Either outcome is a contract change that the PLAN does not acknowledge.
- Recommendation:
  Add an explicit lifecycle policy for `.state.toml`: whether unload captures it, skips it, or captures a sanitized subset. Update `unload.rs` requirements, migration notes, and validation accordingly. If identity is skipped, rely on two-way reconcile to rebuild active tasks and document that session focus is intentionally ephemeral.

### R-004 Worktree discovery/list cannot be based on this session's focus

- Severity: HIGH
- Section: `[**Implementation**]`, `worktree` SPEC alignment
- Problem:
  The PLAN says `commands/agent/task/worktree/discovery.rs` and `list.rs` should replace `.current` reads with `state_file::load_state(&wt_layout)?.sessions...` per-session scans (`00_PLAN.md:511`). But `task worktree list` and cleanup discovery are inventory operations; they must work from the parent checkout even when the parent shell has no session entry inside each child worktree. The existing worktree SPEC requires listing active worktree-backed tasks by inspecting each worktree's task state (`worktree/SPEC.md:26`, `worktree/SPEC.md:38`).
- Why it matters:
  Once `.current` is removed, `task worktree list` can silently skip valid active worktree tasks whenever their original session cache is gone or belongs to another shell. Cleanup by slug becomes unreliable.
- Recommendation:
  For worktree inventory, read `state.tasks.active` from each worktree state file and then load each corresponding `task.toml`; do not depend on current-session focus. Preserve legacy `.current` only as a migration fallback. Add tests for listing/cleanup after deleting the worktree session cache file.

## Trade-off Advice

### TR-1 Rework the session substrate before choosing lock and cache details

- Related Plan Item: `T-1`, `T-5`
- Topic: Compatibility vs Simplicity
- Reviewer Position: Need More Justification
- Advice:
  Keep `File::try_lock` if it remains viable, but do not proceed with PPID-based cache naming until the parent/session id provider is real on Windows.
- Rationale:
  The lock primitive compiled locally; the parent-id primitive did not. The design risk is session identity, not the lock.
- Required Action:
  Replace or fully specify the session-id provider and update validation to prove it.

### TR-2 One state file is acceptable only with a snapshot policy

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A with Revisions
- Advice:
  A single `.state.toml` can stay, but the PLAN must specify how identity, active tasks, and sessions behave under unload/load and state loss.
- Rationale:
  Combining identity and active-task index makes the file convenient but mixes private per-machine data with workflow recovery data.
- Required Action:
  Add unload/load rules and tests before execution.

### TR-3 State-mutate-first archive needs two-way recovery

- Related Plan Item: `T-3`
- Topic: Safety vs Simplicity
- Reviewer Position: Need More Justification
- Advice:
  Do not adopt state-mutate-first archive ordering until reconcile can rebuild active entries from `task.toml`.
- Rationale:
  The chosen ordering is only safe if recovery can derive truth from the task directory after a partial failure.
- Required Action:
  Revise reconcile semantics and failure-flow validation.
