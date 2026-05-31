# `task-concurrency-control` VERIFY

> Status: Closed
> Feature: `task-concurrency-control`
> Owner: Verifier
> Target Task: `task-concurrency-control`
> Verify Scope:
>
> - Plan Fidelity
> - Functional Correctness
> - Code Quality
> - Organization
> - Abstraction
> - SPEC Drift

---

## Verdict

- Decision: Approved
- Blocking Issues: 0
- Non-Blocking Issues: 0

> Initial pass returned Rejected (1 HIGH + 2 MEDIUM). All three findings were
> addressed in-task and re-verified; see [Resolution](#resolution).



## Summary

The implementation covers most of the final plan's architecture: task focus now lives in `.ark/.state.toml`, state reconciliation is centralized, session identity is abstracted behind PPID providers and cache files, resume/discard flows exist, worktree inventory is based on the active set, unload preserves active worktrees, and the shipped templates describe the concurrent-task workflow.

The focused test suite passes with `cargo test -p ark-core -p ark-cli`. VERIFY still cannot approve the task because archive cleanup can leave the current session cache behind after moving the task, which violates the core concurrency invariant. Two medium issues also remain: `ark context` now has a write side effect, and the command-level lifecycle paths still are not fully test-injectable for deterministic multi-session validation.



## Findings

### V-001 Archive cleanup can miss focused-session cache release

- Severity: HIGH
- Scope: Correctness
- Location: `crates/ark-core/src/commands/agent/task/archive.rs:179`
- Problem:
  `clear_state_after_archive` checks whether the current session focused the archived slug inside `state_mutate` after `archive_task_dir` has already moved `.ark/tasks/<slug>` into the archive. State mutation loads and reconciles before the closure runs, so reconcile can prune the session focus because the active task directory no longer exists. After that, the closure no longer observes `sess.focus == slug`, `released_own_focus` remains false, and `release_session_id` is skipped.
- Why it matters:
  Archive is the lifecycle endpoint for this feature. Leaving a stale session cache after archiving the focused task breaks the PRD outcome that concurrent sessions should not remain pointed at completed work, and it increases the PPID-reuse risk the session cache cleanup was meant to reduce.
- Expected:
  Determine whether the current session focused the slug before the archive rename or before reconciliation can prune it, then release the session cache independently of the post-rename active-task check. Add a regression test that archives the focused task and asserts the session cache file is removed.



### V-002 `ark context` is no longer read-only

- Severity: MEDIUM
- Scope: Correctness / SPEC Drift
- Location: `crates/ark-core/src/commands/context/gather.rs:317`, `crates/ark-core/src/session/cache.rs:56`
- Problem:
  `gather_current_task` calls `resolve_session_id`, and `resolve_session_id` creates a new UUID cache file when one does not already exist. That makes `ark context` mutate temp session state on a read path.
- Why it matters:
  The workflow defines `ark context` as a semver-stable, read-only command and invokes it from session-start orientation. A read-only context call should not create session cache files merely because no existing cache was present.
- Expected:
  Add a non-creating session lookup for read-only callers. `ark context` should report no current task when there is no existing session cache or focused session, without writing a new cache file.



### V-003 Command-level PPID injection does not cover lifecycle validation

- Severity: MEDIUM
- Scope: Plan Fidelity / Functional Correctness
- Location: `crates/ark-core/src/commands/agent/task/new.rs:386`
- Problem:
  The PPID abstraction exists below the command layer, but command entry points such as task creation still construct `RealPpid` internally. That keeps deterministic `StubPpid` tests from exercising the same public lifecycle paths used by `ark agent task new`, phase transitions, resume, discard, and archive.
- Why it matters:
  The final plan called for deterministic multi-session lifecycle validation. Testing only helper-level behavior and host OS PPID behavior leaves the highest-risk command composition under-covered.
- Expected:
  Add internal command helpers or an equivalent injection seam that accepts a `PpidProvider`, then cover two-session command flows with `StubPpid`: create/focus, resume, phase operations, archive cleanup, and stale-session reconciliation.



## Follow-ups

None. The verdict is Rejected, so these findings should be addressed in this task before archive.

## Resolution

All three findings addressed in-task. Changes summarized below; live-binary smokes verified V-001 and V-002.

### V-001 — Archive cleanup releases the focused-session cache

- New helper `state::checkout::io::clear_focus_for_slug(layout, ppid, slug)` probes own-focus by reading `.state.toml` directly *before* `state_mutate` runs reconcile. Reconcile cannot prune the answer.
- `task_archive` now calls `clear_focus_for_slug` *before* the rename, so reconcile's add-pass still observes the live `tasks/<slug>/` dir if a later step fails.
- `task_discard` now uses the same helper (replacing its own duplicated probe-and-mut logic).
- Regression test: `commands::agent::task::concurrency_tests::archive_of_focused_task_releases_session_cache` and live smoke confirm the cache file is removed when the focused task is archived.

### V-002 — `ark context` is read-only again

- Added `session::cache::lookup_session_id(layout, &dyn Ppid) -> Result<Option<SessionId>>` — the read-only counterpart to `resolve_session_id`. Returns `None` instead of materializing a fresh cache file.
- `commands::context::gather::gather_current_task` switched to `lookup_session_id`. A session that has not registered focus via `task new` / `task resume` reports `None` without writing to the temp dir.
- Tests: `session::cache::tests::lookup_returns_none_when_no_cache_present_and_does_not_create` plus `lookup_returns_cached_id_after_resolve`.

### V-003 — Command-level PPID seam covers lifecycle composition

- Added `pub(crate) fn task_*_with_ppid(opts, &dyn Ppid)` test seams for `task_new`, `task_resume`, `task_archive`, `task_discard`. Public functions stay as one-line wrappers around `RealPpid::new()`.
- New module `commands::agent::task::concurrency_tests` exercises seven deterministic multi-session scenarios via `StubPpid`: create-distinct-tasks-with-isolated-focus, resume-in-one-session-does-not-touch-the-other, archive-of-focused-task-releases-session-cache (V-001 regression), archive-in-one-session-does-not-clear-the-others-focus, discard-of-focused-task-releases-session-cache, resume-unknown-slug-errors-in-caller-session, stale-session-pruned-when-cache-file-disappears.

### Validation

- `cargo test -p ark-core -p ark-cli` — 366 ark-core unit tests + 39 ark-cli integration tests pass (was 357 + 39; nine new).
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt --all -- --check` — clean.
- V-001 live smoke: cache file present after `task new`+`task execute`, absent after `task archive`.
- V-002 live smoke: cache file count unchanged before/after `ark context`.

