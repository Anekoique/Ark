# `task-concurrency-control` REVIEW `01`

> Status: Closed
> Feature: `task-concurrency-control`
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
- Blocking Issues: 3
- Non-Blocking Issues: 1

## Summary

PLAN 01 resolves the four REVIEW 00 blockers at the intent level: parent-id is now a real platform shim, reconcile is two-way, unload has an explicit state-file policy, and worktree inventory no longer depends on session focus.

However, the revised design still is not executable as written. The module dependency rules contradict the proposed GC API, `task new` now observes its own just-created task during reconcile and therefore warns incorrectly, and the archive ordering is inconsistent with deep-tier SPEC promotion. These are structural enough that execution should not start from this plan.

## Findings

### R-001 `load_state` GC violates the stated module dependency graph

- Severity: HIGH
- Section: `[**Architecture**]`, `[**API Surface**]`, `[**Runtime**]`
- Problem:
  PLAN 01 says `state_file` and `session` MUST NOT import each other, and lists `session` dependencies as only `io::PathExt`, `io::hash_bytes`, `layout`, and `error` (`01_PLAN.md:168-178`). But the API defines `session::gc::prune_dead_sessions(layout, state: &mut StateFile)` (`01_PLAN.md:351-352`), and `load_state` is required to run GC before returning (`01_PLAN.md:400-402`, `01_PLAN.md:460-462`). That requires either `session` to import `StateFile` from `state_file`, or `state_file::io::load_state` to import `session::gc`.
- Why it matters:
  The architecture cannot be implemented while preserving its own dependency rule. This is exactly the kind of cross-module ownership ambiguity that will lead to either circular imports, misplaced logic, or an undocumented dependency reversal.
- Recommendation:
  Choose one dependency direction and make it explicit. The cleanest shape is to move GC into `state_file::reconcile` and have `session` expose only a stateless predicate/helper such as `session::cache_matches(layout, pid, uuid) -> bool`, or else allow `state_file -> session` and remove the "MUST NOT import each other" rule. Update the module coupling, API surface, and tests accordingly.

### R-002 `task new` warns on the first task after two-way reconcile

- Severity: HIGH
- Section: `[**Architecture**]`, `[**Runtime**]`, `[**Validation**]`
- Problem:
  The `task new` flow creates the task directory and writes `task.toml` before calling `state_mutate` (`01_PLAN.md:183-189`). `state_mutate` then runs two-way reconcile before the closure (`01_PLAN.md:397-403`). Because C-19 adds every non-archived task dir to `state.tasks.active` (`01_PLAN.md:382-383`), the newly-created task is already in `active` when the closure checks `if !state.tasks.active.is_empty()` (`01_PLAN.md:190-193`). A fresh first task will therefore emit the "active task(s)" warning, and the closure then pushes the same slug again before dedup-on-save.
- Why it matters:
  G-8 says the warning is only for a non-empty active set *before the new task is appended* (`01_PLAN.md:86`). The current sequence violates that user-visible contract and makes V-IT-2 under-specified: the "first task silent" assertion will fail unless the implementation adds special-case logic not present in the plan.
- Recommendation:
  Specify how `task_new` computes the pre-existing active set. Options: capture `had_other_active = state.tasks.active.iter().any(|s| s != opts.slug)` after reconcile; move the state mutation before scaffolding and handle rollback; or make reconcile accept a just-created slug exclusion for this path. Add validation that a first task is silent and a second distinct task warns.

### R-003 Deep archive ordering is internally inconsistent

- Severity: HIGH
- Section: `[**Architecture**]`, `[**Runtime**]`, `[**Implementation**]`
- Problem:
  The archive call graph runs `spec_extract + spec_register` before `state_mutate`, before saving `phase = Archived`, and before renaming the task directory (`01_PLAN.md:201-220`, `01_PLAN.md:422-432`). Current archive code promotes deep SPECs from `archive_path` after the rename and after saving archived task metadata; its comment explicitly protects the invariant that a promoted SPEC does not reference an unarchived task. PLAN 01 says the SPEC promotion is "unchanged," but the archived path does not exist at that point and any later lock or rename failure would leave promoted SPEC side effects for an active task.
- Why it matters:
  This can corrupt feature SPEC state and break `ark-agent-namespace`'s archive contract for deep tasks. Two-way reconcile fixes the active-set recovery problem; it does not roll back SPEC files or `specs/features/INDEX.md`.
- Recommendation:
  Re-specify archive ordering as a single coherent sequence. At minimum: reserve destination, acquire/update state if that is still the chosen trade-off, mark task archived, rename, then run deep SPEC extract/register from the actual archived path; or keep rename-first and only clear state after successful rename. Add a failure-flow test for deep archive where SPEC promotion cannot happen unless the task is durably archived.

### R-004 Session-provider behavior needs a test seam

- Severity: MEDIUM
- Section: `[**Validation**]`, `[**Implementation**]`
- Problem:
  V-IT-1 depends on "two distinct simulated PPIDs" (`01_PLAN.md:592`), and V-F-5 depends on forcing Windows toolhelp failure but allows skipping if mocking is hard (`01_PLAN.md:607`). The implementation API exposes only `session::ppid::parent_id() -> u32` (`01_PLAN.md:288-298`) and does not define any test-only provider override or lower-level Windows helper that can be unit-tested deterministically.
- Why it matters:
  Without a seam, the most important session behavior will be covered only by smoke tests or brittle process-topology tricks. That is weak coverage for the feature's central concurrency contract.
- Recommendation:
  Add a narrow test seam: for example, keep production `parent_id()` public but factor Windows lookup into an injectable/helper function, and allow session cache tests to pass an explicit ppid into lower-level helpers. Keep the production CLI path unchanged.

## Trade-off Advice

### TR-1 Keep two-way reconcile, but make creation paths explicit

- Related Plan Item: `T-3`, `C-19`
- Topic: Safety vs Simplicity
- Reviewer Position: Prefer Option A with Revisions
- Advice:
  Two-way reconcile is the right recovery primitive, but creation paths need to distinguish "already active before this command" from "created by this command before reconcile ran."
- Rationale:
  Recovery and user-facing warning semantics need different views of the active set.
- Required Action:
  Add explicit `task_new` sequencing or filtering rules before execution.

### TR-2 Do not let archive cleanup ordering leak into SPEC promotion

- Related Plan Item: `T-3`
- Topic: Compatibility vs Safety
- Reviewer Position: Need More Justification
- Advice:
  Treat deep-tier SPEC promotion as a separate side-effect class from state-file cleanup. It should only occur once the task is archived at the path used for extraction.
- Rationale:
  State reconciliation can recover `.state.toml`; it cannot recover or unwrite promoted feature SPEC files.
- Required Action:
  Rewrite the archive flow and validation around deep-tier failure modes.

### TR-3 Resolve ownership of session GC in the design, not during implementation

- Related Plan Item: `Architecture`, `API Surface`
- Topic: Module Boundaries
- Reviewer Position: Need More Justification
- Advice:
  Pick whether `state_file` owns session pruning or `session` owns state mutation helpers, then make the dependency graph match that choice.
- Rationale:
  The current "neither imports the other" rule is incompatible with the public API.
- Required Action:
  Update module coupling, public functions, and tests in PLAN 02.
