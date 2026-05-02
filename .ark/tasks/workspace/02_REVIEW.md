# `workspace` REVIEW `02`

> Status: Open
> Feature: `workspace`
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

- Decision: Approved with Revisions
- Blocking Issues: 1
- Non-Blocking Issues: 5

## Summary

Iteration 02 closes the prior round's core blockers. `workspace_record` now owns its own `RecordTransaction`, archive has a clean-index precondition, rollback is suffix-checked under concurrent appends, and manual recording shares the same transactional primitive. The remaining blocker is a regression in the composed `/ark:commit` transaction: the plan replaces the current commit rollback model but does not explicitly carry forward deep-tier SPEC and features-INDEX snapshots. Fix that before execution. The rest are API/spec alignment issues that can be handled as revisions.

## Findings

### R-201 `CommitTransaction drops SPEC and features-INDEX rollback coverage`

- Severity: CRITICAL
- Section: `[**Architecture**] task_commit composition, `[**Data Structure**] CommitTransaction, `[**Runtime**] Main Flow step 2, `[**Implementation**] Phase 4, validation
- Problem:
  `CommitTransaction::begin` is specified as snapshotting `task.toml` only, and its data model adopts `Vec<RecordSnapshot>` for workspace rollback. The deep-tier `spec_extract` step still runs before the git commit, but the plan no longer explicitly snapshots/restores `.ark/specs/features/<slug>/SPEC.md` and `.ark/specs/features/INDEX.md`. The comment `// workspace; (deep) spec snapshots if any` is not backed by a concrete type, method, order, or validation.
- Why it matters:
  The existing workflow-refactor commit path deliberately protects SPEC promotion and features INDEX because a failed `/ark:commit` after `spec_extract` must not leave promoted SPEC files dirty for a task that did not close. This plan reintroduces the partial-state class that earlier reviews fixed.
- Recommendation:
  Extend `CommitTransaction` with explicit `SpecSnapshot` / `FeaturesIndexSnapshot` support, snapshot both files before `spec_extract`, and restore/delete them on any later failure. Add validation for failures after `spec_extract`, after `spec_register`, after workspace record, and after git commit, asserting task.toml, journal/indices, SPEC, and features INDEX all return to their pre-commit state.

### R-202 `ark context --for record conflicts with the existing context CLI contract`

- Severity: HIGH
- Section: G-18, Architecture `context/`, API Surface, Runtime step 1, Implementation Phase 3 step 9
- Problem:
  The plan proposes `ark context --for record --format json`. The current `ark context` contract and CLI accept `--for` only when `--scope=phase`; using `--for` with default `--scope=session` is explicitly rejected. `record` is also not a lifecycle phase, so adding it as another `PhaseFilter` would blur the existing semantics.
- Why it matters:
  `ark context` is a visible, semver-covered command. The proposed shape breaks or muddies an established argument invariant from the `ark-context` SPEC and current implementation.
- Recommendation:
  Use an additive, semantically clear shape. Preferred: add `--scope record` and a `Scope::Record` projection, so the command becomes `ark context --scope record --format json`. Alternatively, add a dedicated visible subcommand if you want record context outside the scope model. Update G-18, CLI surface, runtime, tests, and docs accordingly.

### R-203 `PRD still describes the old record CLI shape`

- Severity: MEDIUM
- Section: PRD Outcome item 6 and item 13 vs PLAN G-14 / API Surface
- Problem:
  The PLAN now uses `--entry-file` for task and manual recording, but the PRD still says task mode is `--task <slug>` and manual mode is `--manual --title <t>`, with `/ark:commit` gaining an internal `record --task <slug>` step. That no longer matches the planned `ark agent workspace record --task <slug> --entry-file <path>` / `--manual --entry-file <path>` protocol.
- Why it matters:
  PRD and PLAN disagree on the user/tool contract for a core feature. Implementation or verification could follow either wording and produce incompatible wrappers.
- Recommendation:
  Update PRD Outcome items 6 and 13 to mention `--entry-file` and the draft-render workflow for both task and manual modes.

### R-204 `JournalDriftDetected is a documented exception to atomicity but G-13 still claims all-or-rollback`

- Severity: MEDIUM
- Section: G-13, C-19, Runtime Failure Flow §14, Validation V-E-5
- Problem:
  G-13 says `workspace_record` is all-or-rollback. C-19 and Failure Flow §14 correctly say suffix drift leaves the journal untouched and returns `JournalDriftDetected`, which can leave this transaction's appended entry in the file if another process appended after it.
- Why it matters:
  The design choice is reasonable, but the goal language over-promises. VERIFY will otherwise treat this as an atomicity violation even though it is the intentional no-lock behavior.
- Recommendation:
  Reword G-13 to say "all-or-rollback except suffix-drift, which preserves concurrent appends and returns `JournalDriftDetected` for manual reconciliation." Map that exception explicitly to V-UT-24 / V-E-5.

### R-205 `write_file is described as atomic temp+rename but currently is not`

- Severity: MEDIUM
- Section: Rollback primitives, ArchiveTransaction, project code reality
- Problem:
  The plan says index rollback uses `io::fs::write_file(path, bytes)` as "atomic temp+rename." In the current codebase, `io::fs::write_file` delegates to `PathExt::write_bytes`, not a temp-file rename protocol.
- Why it matters:
  The transaction design relies on write semantics when restoring indices and patched archive files. If the plan wants atomic temp+rename, it must add or change a helper and test it; otherwise the wording should not promise stronger behavior than the code provides.
- Recommendation:
  Either add a new atomic-write helper and use it for transaction restores/patch writes, or revise the plan to use the existing content-aware write semantics and rely on snapshots plus rollback rather than temp+rename atomicity. Add a focused test for the chosen helper.

### R-206 `Context projection payload needs schema placement`

- Severity: LOW
- Section: G-18, Data Structure, API Surface
- Problem:
  The plan lists the record projection JSON fields but does not state whether it reuses the schema-1 `ProjectedContext` envelope or returns a separate shape. It also does not state whether text mode is supported or rejected.
- Why it matters:
  `ark context` has a stable JSON schema and common renderer path. A projection-specific ad hoc JSON object could bypass that contract accidentally.
- Recommendation:
  Define the record projection as an additive field inside the existing projected-context schema, e.g. `scope = "record"` plus `record = { identity, active_journal_path, journal_max_lines, session_count, branch }`, and specify text-mode behavior.

## Trade-off Advice

### TR-1 `Commit transaction composition`

- Related Plan Item: `CommitTransaction`, Phase 4
- Topic: Safety vs Scope
- Reviewer Position: Require one composed transaction for all Ark-managed closure artifacts
- Advice:
  Treat SPEC files, features INDEX, task.toml, workspace journals, and workspace indices as one rollback set owned by `CommitTransaction`.
- Rationale:
  `/ark:commit` is the user-visible atomic closure boundary. Even if `workspace_record` is self-protected internally, the outer transaction must still cover every Ark-managed artifact mutated before the final git commit.
- Required Action:
  Add SPEC/INDEX snapshots to the transaction design and validation.

### TR-2 `Record context command shape`

- Related Plan Item: G-18
- Topic: Compatibility vs Convenience
- Reviewer Position: Prefer `--scope record`
- Advice:
  Use `ark context --scope record --format json`, not `ark context --for record`.
- Rationale:
  `record` is a projection scope, not a lifecycle phase. This keeps the existing `--for` invariant intact and makes the addition obvious in help text.
- Required Action:
  Revise the CLI/API/docs/tests to this shape unless you choose a dedicated subcommand.
