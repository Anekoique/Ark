# `workspace` REVIEW `00`

> Status: Open
> Feature: `workspace`
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
- Non-Blocking Issues: 3

## Summary

The plan has the right high-level direction: defer the unknowable closing SHA with a slug-scoped sentinel, keep journals branch-local, and preserve the compact Trellis-like journal shape. It is not ready to execute. The two central atomic boundaries, `/ark:commit` and `ark archive`, are underspecified in ways that can lose the PRD guarantees. The plan also leaves the agent-filled journal content path unresolved while simultaneously depending on that content being written inside the commit primitive.

## Findings

### R-001 `Agent-filled journal content has no executable delivery path`

- Severity: CRITICAL
- Section: `[**Data Structure**]`, `[**API Surface**]`, `[**Implementation**] Phase 4`, `T-2`
- Problem:
  `RecordOptions` requires agent-filled `summary` and `main_changes`, but the CLI surface only exposes `ark agent workspace record --task <slug>` / `--manual --title <t>`. Phase 4 says `task_commit` will call `workspace_record` internally, while T-2 leans toward "slash-command renders a draft entry, agent edits before staging, commit reads the journal file as-is." Those are incompatible: if `task_commit` appends the journal during the commit operation, the agent has no chance to edit the entry before it is staged and committed.
- Why it matters:
  The PRD requires compact, useful journal entries with agent-filled summary and change rows. The current plan can only produce placeholders, require a second edit/commit, or silently drop the agent-authored fields, any of which breaks the core user-facing outcome.
- Recommendation:
  Pick one concrete protocol and carry it through Data Structure, API Surface, Runtime, Implementation, and tests. Preferred: add a structured content input to `task_commit` / `workspace_record` such as `--summary <s>` plus repeatable `--change <area>=<description>`, or an `--entry-json/--entry-file` path to avoid shell escaping. Then update `/ark:commit` and the Codex/OpenCode equivalents to collect the summary/change rows before invoking `ark agent task commit`.

### R-002 `Commit-side rollback does not cover workspace writes`

- Severity: CRITICAL
- Section: `[**Architecture**] / Atomic-commit boundary at /ark:commit`, `[**Runtime**] Failure Flow`, `[**Implementation**] Phase 4`
- Problem:
  The plan inserts `workspace_record` before staging, but it does not extend the existing `task_commit` rollback model to cover the journal append, top-level workspace index, or per-developer index. The failure flow only says append failures abort before `git add`; it does not restore successful workspace writes if `task.toml` save, `git add`, or `git commit` later fails.
- Why it matters:
  The existing workflow-refactor machinery was specifically hardened so Ark-managed files are either committed atomically or restored. Adding journal/index mutations outside the rollback guard reintroduces the same partial-state class: a failed `/ark:commit` can leave a pending journal entry and session row for a task that did not actually close.
- Recommendation:
  Make workspace writes first-class rollback snapshots. Before recording, snapshot journal length or bytes, personal index bytes, top-level index bytes, and `task.toml`. On every error before successful commit, truncate/delete the appended journal entry as appropriate, restore both indices, restore `task.toml`, and targeted-unstage the workspace paths. Add failure tests for errors after journal append, after index write, after `task.toml` save, after `git add`, and after `git commit`.

### R-003 `Archive is promoted to a committing transaction without a transaction design`

- Severity: CRITICAL
- Section: `[**Architecture**] / Atomic-commit boundary at ark archive`, `[**Runtime**] Main Flow`, `[**Implementation**] Phase 5`
- Problem:
  The plan changes `ark archive` from a move operation into "patch journals + patch indices + move task dirs + git commit", but it does not define staging, commit message construction, rollback, interaction with pre-existing user-staged files, or what happens when one task in a bulk archive succeeds in memory and a later task fails before the final commit.
- Why it matters:
  The PRD says the slot patch is atomic with the archive move commit. Without a transaction model, `ark archive` can leave patched journals, moved task directories, and unstaged or partially staged Ark-managed files when `git add` or `git commit` fails. That violates the exact durability problem this task is meant to solve.
- Recommendation:
  Add an explicit `ArchiveTransaction` design. It should snapshot every touched journal/index and task-dir move, stage only Ark-managed archive files, preserve unrelated staged work, commit with the documented bulk archive message, and restore or cleanly report a recoverable state on failure. The validation matrix needs multi-task bulk archive tests, partial-failure rollback tests, and a test that user-staged non-Ark files survive a failed archive commit.

### R-004 `Ambiguous pickaxe handling is impossible with -n 1`

- Severity: HIGH
- Section: `[**Goals**] G-12`, `[**Architecture**] archive flow`, `[**Runtime**] Failure Flow`, `[**Implementation**] Phase 5`
- Problem:
  `resolve_closing_sha` is specified as `git log -S '**Slug**: <slug>' --format=%H -n 1 -- <journal-path>`, while G-12 and V-F-1 require detecting `>1` matching commits. With `-n 1`, the implementation cannot ever observe the ambiguous case.
- Why it matters:
  The plan promises a defensive ambiguity error and a candidate list, but the proposed command masks the condition. The validation cannot be implemented as written, and a real duplicate slug/history anomaly would silently pick whichever commit Git returns first.
- Recommendation:
  Define `resolve_closing_sha` as a collect-then-classify operation: run the pickaxe without `-n 1` (or with a small cap greater than 1), collect matching full SHAs, error on zero or more than one, and only then derive the 12-character short SHA via `git rev-parse --short=12 <sha>` or equivalent. Update G-2, G-12, runtime, and V-UT/V-F entries to match.

### R-005 `Related dependency is not reviewable through the declared inputs`

- Severity: MEDIUM
- Section: Plan frontmatter `Depends on`, PRD `[**Related Specs**]`
- Problem:
  The PRD depends heavily on `ark-workflow-refactor`, but the promoted feature SPEC is not present in `.ark/specs/features/` in this worktree and `ark context --scope phase --for review` returned no related feature specs. The plan frontmatter also lists no related specs or task artifacts.
- Why it matters:
  Review and execution can miss the actual staged-only commit/rollback constraints that this plan must preserve. This is already visible in R-002 and R-003.
- Recommendation:
  Either archive/promote `ark-workflow-refactor` before continuing, or explicitly add `.ark/tasks/ark-workflow-refactor/02_PLAN.md` and `02_REVIEW.md` as temporary dependencies in the PLAN until the SPEC exists. Also fix the PRD related-spec formatting if the context filter failed because the parser could not recognize it.

### R-006 `Upgrade migration omits the developer-dir behavior from the PRD`

- Severity: MEDIUM
- Section: `[**Implementation**] Phase 6`
- Problem:
  The PRD says `ark upgrade` scaffolds the developer directory when `.ark/.developer` exists. Phase 6 only scaffolds the top-level `.ark/workspace/index.md` and `[workspace]` config section.
- Why it matters:
  Fresh upgraded installs with an existing developer identity may not have the expected personal index/journal layout until a later record operation. That is a smaller issue than the commit/archive blockers, but it is still a PRD outcome mismatch.
- Recommendation:
  Add an explicit upgrade step and test: when `.ark/.developer` exists, create `.ark/workspace/<dev>/index.md` if absent without synthesizing journal entries or overwriting existing files.

### R-007 `Validation misses the new failure classes`

- Severity: MEDIUM
- Section: `[**Validation**]`
- Problem:
  The validation suite covers happy-path commit/archive and some slot failures, but it does not cover commit rollback after successful workspace writes, archive rollback after partial multi-task processing, archive `git commit` failure, preservation of user-staged non-Ark files during archive failure, or the content-delivery path for agent-filled summary/change rows.
- Why it matters:
  These are the exact places where the design is most fragile. Without tests at those boundaries, an implementation can appear green while violating the atomicity and journal-quality guarantees.
- Recommendation:
  Add validation IDs for each failure class named in R-001 through R-003 and map them back to G-1, G-2, G-5, G-8, C-4, C-10, and C-11.

## Trade-off Advice

### TR-1 `Marker-scanner sharing`

- Related Plan Item: `T-1`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer existing shared helper
- Advice:
  Use the existing `io::fs::{read_managed_block, update_managed_block, remove_managed_block}` helpers with new marker names (`ARK:DEVELOPERS`, `ARK:SESSIONS`) instead of extracting a new managed-block module or copying parser logic.
- Rationale:
  The helper is already public from `io::fs`, already supports markers like `ARK:FEATURES`, and already encodes the orphan-marker corruption behavior. A new helper module is churn; a copy risks divergent marker semantics.
- Required Action:
  Revise T-1 and Phase 2 to say the implementation reuses the existing managed-block helpers directly unless a concrete missing API is discovered.

### TR-2 `Agent-content delivery`

- Related Plan Item: `T-2`
- Topic: Flexibility vs Atomicity
- Reviewer Position: Reject current Option A / Option B split
- Advice:
  Prefer an `--entry-file` or `--entry-json` input consumed by `task_commit`, with slash commands generating the content before invoking the commit primitive.
- Rationale:
  Free-form Markdown editing before commit conflicts with an internally invoked `workspace_record`; raw `--summary` / `--main-changes` flags are easy to misquote. A file or JSON payload keeps the atomic single command while avoiding shell-escaping problems.
- Required Action:
  Replace T-2 with this protocol or justify a different end-to-end protocol that still preserves atomic commit semantics.

### TR-3 `Archive failure UX`

- Related Plan Item: `T-4`
- Topic: Safety vs Convenience
- Reviewer Position: Prefer fail-loud
- Advice:
  Keep the fail-loud behavior for pickaxe failures, but require skipped slot patches to be visible in the `ark archive` summary and commit body when `--skip-slot-patch <slug>` is used.
- Rationale:
  Silent skips make journal integrity hard to audit later. A deliberate skip is acceptable as an escape hatch only if it leaves a clear trail.
- Required Action:
  Add summary/commit-body wording and validation for skipped slot patches.
