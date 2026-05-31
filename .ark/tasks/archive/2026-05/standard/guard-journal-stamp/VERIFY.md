# `guard-journal-stamp` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `guard-journal-stamp`
> Target Task: `guard-journal-stamp`
> Tier: `standard`

---

## Project Spec Compliance

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: N/A — task makes no `specs/project/` changes; existing index unaffected.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: N/A — task touches no project SPEC.
  - `LAYOUT.md`
  - `rust/COMMENTS.md`
  - `rust/STYLE.md`
  - `rust/ERRORS.md`

## Related Feature Spec Compliance

- [x] specs/features/workspace/SPEC.md: PASS — CHANGELOG entry for `guard-journal-stamp` appended at the foot of `[**CHANGELOG**]` block.
- [x] specs/features/ark-agent-namespace/SPEC.md: PASS — new error variant joins the documented `task commit` failure-mode set; no SPEC body change required (failure enumeration is implementation-detail-stable per PRD).
- [x] rust/ERRORS.md: PASS — new `Error::JournalSessionHeadingMissing` variant uses structured fields (`journal_path: PathBuf`, `slug: String`), `#[error(...)]` actionable message, no `unwrap`/`expect` in production code paths.
- [x] rust/STYLE.md: PASS — `assert_unstamped` helper is 18 LOC (under the ≤30 LOC C-1 cap), single-responsibility, no mutation of input arguments.

## PRD Constraints

- [x] `task commit` aborts with `Error::JournalSessionHeadingMissing` when last `## Session N:` heading is already stamped: PASS — `cargo test --workspace stamp_task_refuses_when_heading_already_stamped` + `stamp_manual_refuses_when_heading_already_stamped` + `record_errors_when_last_heading_already_stamped` all pass.
- [x] Error message names journal path, slug, and the retry command: PASS — `journal_session_heading_missing_message_is_actionable` asserts the message contains path, slug, "## Session", and "ark agent task commit".
- [x] Guard fires before file mutation; failed call leaves journal and personal index untouched: PASS — `record_errors_when_last_heading_already_stamped` asserts post-call journal byte-equality with pre-call bytes; asserts personal/top-level index unchanged.
- [x] Happy path unchanged: PASS — existing `stamp_task_inserts_all_auto_fields`, `stamp_manual_omits_task_only_fields`, `stamp_targets_last_session_when_multiple_present`, `record_task_stamps_auto_fields_and_updates_indices`, `record_manual_uses_dash_slug_and_no_sentinel` all still pass byte-identical output.
- [x] Three (actually four) new tests added: PASS — 3 in `stamp.rs::tests` (`stamp_task_refuses...`, `stamp_manual_refuses...`, `journal_session_heading_missing_message_is_actionable`) + 1 in `record.rs::tests` (`record_errors_when_last_heading_already_stamped`). Total: stamp 5→8, record 4→5.
- [x] No slash-command template changes: PASS — `git diff --stat` shows no `templates/` changes.
- [x] No empty-Git-Commits-table rendering change: PASS — `render_git_commits_block` is unchanged; the `(none)` row stays.
- [x] No historical journal-file changes: PASS — `.ark/workspace/` is untouched in this commit.

## Plan Fidelity

- [x] G-1: `stamp_task` refuses with new variant when heading already stamped: PASS — `stamp_task_refuses_when_heading_already_stamped` asserts `matches!(err, Error::JournalSessionHeadingMissing { .. })`.
- [x] G-2: `stamp_manual` refuses identically: PASS — `stamp_manual_refuses_when_heading_already_stamped` asserts same variant with `slug == "-"`.
- [x] G-3: Error variant has `journal_path: PathBuf` + `slug: String`; message actionable: PASS — `error.rs:347-360` defines the struct fields; `journal_session_heading_missing_message_is_actionable` asserts the rendered message contains path, slug, and the retry command.
- [x] G-4: No write on failure: PASS — `record_errors_when_last_heading_already_stamped` asserts journal byte-equality after the failed call; `stamp_*_refuses_when_heading_already_stamped` also assert file unchanged.
- [x] G-5: Happy path byte-identical: PASS — all five pre-existing `stamp::tests` and `record::tests` continue to pass; cargo test reports 461 → 465 (+4 new, 0 regressions).
- [x] G-6: workspace SPEC CHANGELOG entry: PASS — `[**CHANGELOG**]` block at `specs/features/workspace/SPEC.md` now ends with the `2026-05-11 guard-journal-stamp` entry naming the new error variant.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: PASS — `specs/features/workspace/SPEC.md` is the only modified feature SPEC; its CHANGELOG carries the new entry.

## Findings

### V-001 Wrong-path edits during EXECUTE required mid-flight recovery

- **Severity:** MEDIUM
- **Location:** cross-file: my Edit tool calls during EXECUTE used absolute parent-checkout paths (`/Users/anekoique/Agent/Ark/crates/...`) instead of worktree paths (`/Users/anekoique/Agent/Ark/.ark/worktrees/feat/guard-journal-stamp/crates/...`).
- **Problem:** All four code/spec edits landed in the parent checkout, not the worktree. `cargo build` and `cargo test` ran in the worktree and reported success against the *pre-change* source, masking the mistake until I noticed the test count was unchanged from baseline.
- **Why it matters:** The worktree-per-task model exists precisely so multiple tasks can be in flight without colliding. Bypassing it via absolute parent paths defeats that isolation. A future agent running a parallel task would have seen unrelated dirty files in their parent checkout.
- **Recommendation:** When operating from a worktree, **never** use absolute paths that bypass it. Either (a) always pass paths relative to the worktree's working dir, or (b) when an absolute path is needed, build it from the worktree root, not the parent's. Worth a project-SPEC rule or a CLAUDE.md addendum.
- **Resolution:** FIXED in this EXECUTE phase — `git stash push` in the parent (scoped to the four files, preserving the user's in-flight `templates/ark/workflow.md`), `git stash apply` in the worktree, `git stash drop`. Parent verified clean (`git status` shows only the pre-existing dirty `workflow.md` + untracked `reference/`). Worktree carries all four edits + 4 new tests, all passing.

## Notes

- **Total LOC added**: ~95 (1 error variant ~14 LOC + `assert_unstamped` helper ~18 LOC + 2 stamp guard wires ~2 LOC + 4 tests ~60 LOC + SPEC CHANGELOG ~1 LOC). Under the C-1 cap on `assert_unstamped` (18 ≤ 30).
- **Detection rule** (next non-blank line begins with `**Date**`) is implemented and tested. False-positive risk (hand-edited journals with stray `**Date**` lines) is acknowledged in PLAN's Trade-offs and is out of scope here.
- **`task commit` will fire this guard against itself** when invoked from this worktree, because the worktree's `.ark/workspace/` is empty (no journal file → existing `EntryFileMalformed` path fires first, not the new guard). The guard's mainline failure path will be exercised by future tasks that touch a previously-stamped journal.
- **The bug that motivated this task** (rfc001-arkos's `cd50a33` journal damage) is the canonical fixture shape for `stamp_task_refuses_when_heading_already_stamped`. If the test ever regresses, that prior commit is the reference.
- Workspace journal stamping is now enforced. The slash-command prompts (`templates/{claude,codex,opencode}/...`) still describe the contract advisorily; per PRD's non-goal-2, that's intentional. If the CLI feedback proves insufficient over time, a follow-up task can tighten the prompts.
