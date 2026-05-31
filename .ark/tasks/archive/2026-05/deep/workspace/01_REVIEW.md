# `workspace` REVIEW `01`

> Status: Open
> Feature: `workspace`
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
- Non-Blocking Issues: 3

## Summary

Iteration 01 resolves the previous round's broad design gaps: `--entry-file` is a workable content-delivery protocol, the pickaxe is now collect-then-classify, and the plan recognizes that both commit and archive need transactional handling. The remaining problems are transaction-boundary details. In particular, workspace write snapshots are still acquired too late for partial failures, archive's git-index handling can commit or corrupt unrelated staged work, and journal truncation rollback is unsafe under the plan's own "no coordination beyond O_APPEND" constraint.

## Findings

### R-101 `Workspace snapshots are captured after the partial-failure point`

- Severity: CRITICAL
- Section: `[**Architecture**] / /ark:commit atomic boundary`, `[**Data Structure**] RecordSummary + CommitTransaction, `[**Runtime**] Failure Flow`, `[**Implementation**] Phase 3 and Phase 4
- Problem:
  `workspace_record` is responsible for capturing `journal_byte_length_before`, `personal_index_bytes_before`, and `top_level_index_bytes_before`, then returning them in `RecordSummary`. `CommitTransaction` only records those snapshots after `workspace_record` returns successfully. If `workspace_record` appends the journal and then fails while updating the personal index or touching the top-level index, there is no returned `RecordSummary`, so `CommitTransaction` does not have the snapshot it needs to truncate/restore.
- Why it matters:
  The failure flow promises rollback after a partial `workspace_record` failure, but the data needed to roll back is not available on that error path. A failed `/ark:commit` can still leave a pending journal entry or partially updated index for a task that did not close.
- Recommendation:
  Move snapshot ownership before mutation. Either make `CommitTransaction` resolve the target journal/index paths and snapshot them before calling `workspace_record`, then pass a transaction handle into `workspace_record`; or make `workspace_record` internally transactional and guarantee it rolls back its own partial writes before returning `Err`. Update the data structures and failure tests so a failure after journal append but before `RecordSummary` exists is explicitly covered.

### R-102 `Archive can commit or corrupt unrelated staged work`

- Severity: CRITICAL
- Section: `[**Architecture**] / ark archive atomic boundary`, `ArchiveTransaction`, `Rollback primitives`, `V-IT-9`
- Problem:
  `ArchiveTransaction` snapshots the original git index as `git diff --cached --name-only`, then the archive path runs `git add` and `git commit` at the end. A normal `git commit -m ...` commits all staged files, including unrelated files the user had staged before running `ark archive`. The proposed rollback also cannot restore the original index from path names alone; `git reset` plus `git add` would stage current working-tree contents, not the exact original staged blobs, modes, deletions, or partial hunks.
- Why it matters:
  The plan claims user-staged non-Ark files are preserved, but the current design can include them in the archive commit on success and can alter their staged content on failure. That violates both user intent and the archive transaction boundary.
- Recommendation:
  Pick a simpler, enforceable archive precondition: require `git diff --cached --quiet` before `ark archive` and fail with a clear error if the index is not empty. Then archive can stage only Ark-managed files and roll back with targeted `git reset HEAD -- <ark-files>`. If preserving an arbitrary non-empty index remains required, the plan needs a real index snapshot strategy (`git diff --cached --binary` plus apply, or a temporary index design), not a path-name list.

### R-103 `Journal truncate rollback is unsafe with concurrent appends`

- Severity: HIGH
- Section: `[**Constraints**] C-1, `Rollback primitives`, `NG-2`, `[**Edge Case Validation**] V-E-5
- Problem:
  The plan uses `truncate_to(snapshot_len)` to roll back journal appends and says this is safe because the file is append-only between snapshot and rollback. At the same time, NG-2 and V-E-5 explicitly avoid coordination beyond `O_APPEND`. If another process appends to the same journal after the snapshot and before rollback, truncating to the old byte length deletes that other entry.
- Why it matters:
  This is data loss in the exact file where workspace history is stored. `O_APPEND` protects individual writes from interleaving; it does not make later truncation safe.
- Recommendation:
  Add a per-journal lock around record/commit/archive mutations, or replace truncate rollback with suffix-checked rollback: only truncate if the file still ends with the exact bytes this transaction appended; otherwise leave the file intact and return a recoverable drift error that names the journal path. If the project deliberately rejects locking, document the suffix-check behavior and validate the concurrent-append case.

### R-104 `Manual record lacks an atomicity story`

- Severity: HIGH
- Section: `[**Goals**] G-6 and G-13, `[**API Surface**]`, `[**Runtime**] Main Flow`, `[**Implementation**] Phase 6
- Problem:
  G-13 adds transactions for `/ark:commit` and `ark archive`, but manual `ark agent workspace record --manual --entry-file <path>` still calls the same journal/index update machinery without a transaction boundary. If a manual record appends the journal and then index update fails, the plan has no rollback path.
- Why it matters:
  Manual `/ark:record` is a first-class platform feature in G-6/G-11. It should not have weaker consistency than task-driven recording, especially because it writes the same journal and indices.
- Recommendation:
  Introduce a `RecordTransaction` owned by `workspace_record` for both task and manual modes, or define `workspace_record` as an all-or-rollback primitive. `task_commit` can then compose it into `CommitTransaction` by adopting its snapshots or by invoking it under the same transaction handle.

### R-105 `PRD still specifies the rejected -n 1 pickaxe`

- Severity: MEDIUM
- Section: PRD Outcome item 7 vs PLAN G-2 / R-004 response
- Problem:
  The revised PLAN correctly changes `resolve_closing_sha` to collect all pickaxe matches without `-n`, but the PRD still says archive resolves with `git log -S '**Slug**: <slug>' --format=%H -n 1 -- <journal-path>`.
- Why it matters:
  The PRD and PLAN now disagree on a core correctness mechanism. Future implementation or verification can accidentally follow the stale PRD wording and reintroduce the ambiguity bug from the prior review.
- Recommendation:
  Update the PRD Outcome item 7 to match the collect-then-classify lookup, including the zero/multiple match behavior and 12-character short-SHA derivation.

### R-106 `Record context projection is still missing from the plan`

- Severity: MEDIUM
- Section: PRD `[**Related Specs**] ark-context row, PLAN Architecture/API/Implementation
- Problem:
  The PRD says this task adds an `ark context` record projection that bundles developer identity, active journal path, and workspace config for `/ark:record`. The revised PLAN does not include any `commands/context` architecture changes, CLI surface, implementation phase, or validation for that projection.
- Why it matters:
  Either the PRD is over-scoped or the PLAN is incomplete. As written, platform wrappers may lack the structured context they are supposed to use when preparing manual record drafts.
- Recommendation:
  Add the record projection to Goals, Architecture, API Surface, Implementation, and Validation, or explicitly amend the PRD to remove/defer it and explain how `/ark:record` gets the required seed data without `ark context`.

### R-107 `Public data models violate the project style spec`

- Severity: LOW
- Section: `[**Data Structure**]`, project spec `rust/STYLE.md` S-21
- Problem:
  The plan sketches several public structs with public fields (`Identity`, `EntryDraft`, `TaskHeader`, `ArchiveTransaction`, etc.). The Rust style spec says public structs have private fields unless intentionally transparent.
- Why it matters:
  This is easy to fix during implementation, but leaving the PLAN examples as public-field APIs invites a public surface that contradicts the project conventions.
- Recommendation:
  Mark these snippets as schematic or revise the API to private fields with constructors/accessors. Keep intentionally transparent internal-only structs `pub(crate)` if needed.

## Trade-off Advice

### TR-1 `Transaction ownership`

- Related Plan Item: `G-13`, `CommitTransaction`, `workspace_record`
- Topic: Safety vs Composition
- Reviewer Position: Prefer one transaction owner per mutation primitive
- Advice:
  Make `workspace_record` transactional at its own boundary, then let `task_commit` compose it rather than trying to reconstruct its rollback state after the fact.
- Rationale:
  The primitive that knows the journal path, rotation decision, index paths, and write order is the only place that can reliably snapshot before each mutation and roll back partial failures.
- Required Action:
  Revise the transaction architecture so snapshots exist before the writes they protect, including error paths where `workspace_record` does not return a normal summary.

### TR-2 `Archive index policy`

- Related Plan Item: `ArchiveTransaction`, `V-IT-9`
- Topic: Simplicity vs Flexibility
- Reviewer Position: Prefer clean-index precondition
- Advice:
  Require an empty staging area for `ark archive`.
- Rationale:
  Preserving arbitrary pre-existing index state, especially partial hunks and deletions, is much harder than the feature needs. A clean-index precondition is clear, testable, and consistent with archive being a manager operation.
- Required Action:
  Replace the path-name index snapshot with an empty-index precondition and targeted rollback of only Ark-managed archive paths, or fully specify a real staged-blob snapshot/restore design.

### TR-3 `Rollback under concurrency`

- Related Plan Item: `C-1`, `NG-2`, `V-E-5`
- Topic: Data Integrity vs No Locking
- Reviewer Position: Need More Justification
- Advice:
  Either add a narrow per-journal lock or use suffix-checked rollback that refuses to truncate when later bytes appeared.
- Rationale:
  `O_APPEND` and truncate-based rollback solve different problems. Without one of these extra protections, rollback can delete a valid concurrent journal entry.
- Required Action:
  Update constraints and validation to make the chosen behavior explicit.
