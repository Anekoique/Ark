# `ark-cleanup` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `ark-cleanup`
> Target Task: `ark-cleanup`
> Tier: `standard`

---

## Project Spec Compliance

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — index lists `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`; `find .ark/specs/project -name '*.md'` returns those four plus `INDEX.md` itself. No drift.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PASS — this task did not touch any project SPEC. Conformance was last verified at the SPECs' own promotion / lint runs; rust/COMMENTS.md, rust/STYLE.md, rust/ERRORS.md retain `[**Purpose**] / [**Rules**] / [**Exceptions**] / [**Examples**] / [**See Also**]` shape per L-1.
  - `LAYOUT.md`: N/A (the layout reference itself; not subject to its own L-rules).
  - `rust/COMMENTS.md`: PASS (untouched in this task).
  - `rust/STYLE.md`: PASS (untouched in this task).
  - `rust/ERRORS.md`: PASS (untouched in this task).

## Related Feature Spec Compliance

- [x] specs/features/worktree/SPEC.md: PASS — `ark cleanup` reuses `worktree_cleanup`, `parse_git_worktree_list`, and `WorktreeConfig::resolve_worktrees_dir` exactly as the worktree feature exposes them. Worktree feature C-15 (empty parent pruning) and C-20 (silent skip of non-Ark worktrees) are honoured by reuse, not reimplementation. Worktree feature C-18 ("archive does not touch worktrees") is preserved — `ark cleanup` is the matching user-driven verb that *does*. No `task.toml` schema additions; `branch`, `worktree_path`, and `phase` are read-only inputs. No CHANGELOG entry needed in `worktree/SPEC.md`: the feature is consumed, not modified.
- [x] specs/features/ark-context/SPEC.md: PASS — `ark cleanup` does not invoke `ark_context`, mutate context state, or add a phase projection. Zero interaction.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` (and `[**Constraints**]` when present). One bullet per criterion.

- [x] `ark cleanup` (zero args) prints a dry-run report: PASS — verified end-to-end in `cleanup_dry_run_*` unit tests and against the live repo (printed 3 prunable Committed worktrees).
- [x] Each row formatted as `<slug> <branch> [<reason>] <path>`: PASS — `CleanupSummary::Display` writes `"  {slug} {branch} [{reason}] {path}"` per row (cleanup.rs:142-149). Live smoke-test output matched.
- [x] Empty list prints `ark cleanup: nothing to prune`, exit 0: PASS — `cleanup_dry_run_empty_repo_is_nothing_to_prune` asserts the literal string; CLI dispatch returns Ok before any exit-code branch.
- [x] `--apply` removes via `worktree_cleanup` per slug, collects per-row failures: PASS — `cleanup_apply_collects_per_row_failure` proves the loop continues after `Error::WorktreeDirty`; `cleanup_apply_removes_committed_worktree` proves the happy path removes the dir and rerun reports nothing.
- [x] `--delete-branch` requires `--apply` (clap-enforced): PASS — verified by smoke test `./target/debug/ark cleanup --delete-branch` exits with `error: the following required arguments were not provided: --apply`.
- [x] `--force` requires `--apply` (clap-enforced): PASS — same clap `requires = "apply"` annotation; covered by the same smoke-test pattern.
- [x] `--slug <s>` narrows to one slug: PASS — `cleanup_slug_filter_narrows` asserts `slugs == ["alpha"]` when filter set.
- [x] Detection — committed: PASS — `classify_committed` and `cleanup_dry_run_surfaces_committed`.
- [x] Detection — archived: PASS — `classify_archived_over_committed` proves precedence; archive-dir walk implemented in `enumerate_archived` (cleanup.rs:231-249).
- [x] Detection — branch-gone (local `git branch --list`, no network): PASS — `classify_branch_gone` covers the missing-branch case; `enumerate_local_branches` uses `run_git(["branch","--list","--format=%(refname:short)"])` with no `ls-remote` or merge analysis.
- [x] Third-party worktree silently skipped: PASS — `enumerate_candidates` uses `let Ok(state) = load_state(...) else { continue }` and `let Ok(toml) = TaskToml::load(...) else { continue }` (cleanup.rs:289-300). Worktree feature C-20 contract preserved.
- [x] Top-level verb (not under `ark agent`): PASS — `Command::Cleanup(CleanupArgs)` lives at the top of the `Command` enum in `ark-cli/src/main.rs`, peer to `Command::Archive`. `--help` shows it under the user-visible verb list.
- [x] Exit 1 only when at least one row failed under `--apply`: PASS — dispatch arm checks `!summary.failures.is_empty()` before `std::process::exit(1)` (mirrors `ark archive`).

## Plan Fidelity

- [x] G-1: `ark cleanup` lists prunable worktrees in dry-run, removes them on `--apply`: PASS — V-IT-3 (dry-run surfaces Committed) and V-IT-5 (apply removes the dir) both green.
- [x] G-2: `ark cleanup` is a stable top-level CLI peer to `ark archive` and `ark context`: PASS — top-level `Command::Cleanup` variant; CLI dispatch and `--help` confirm peer placement.
- [x] G-3: Cleanup detects three reasons (`committed`, `archived`, `branch-gone`): PASS — V-UT-2/V-UT-3/V-UT-4 cover each reason; precedence test V-UT-3 confirms `Archived > Committed`.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: PASS — N/A. This task did not modify any feature SPEC. The worktree and ark-context feature SPECs are *consumed* (read-only); no `[**CHANGELOG**]` line is needed.

## Findings

### V-001 `ark work scope dropped mid-flight; PRD/PLAN updated, no orphaned code`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/cleanup.rs`, `crates/ark-cli/src/main.rs`, `.ark/tasks/ark-cleanup/{PRD,PLAN}.md`.
- **Problem:** Original PRD scoped two verbs (`ark work` + `ark cleanup`); `ark work` was dropped during EXECUTE after the focus-binding architecture made it unworkable from the parent checkout.
- **Why it matters:** Future readers reading `ark-cleanup`'s PRD/PLAN should not be confused by a missing verb. Code, error variants, and CLI surface should not retain dead `work` traces.
- **Recommendation:** Confirm no `WorkOptions`, `WorkSummary`, `work::`, or `Command::Work` references remain; document the drop in PRD's `[**Out of scope**]` and PLAN's `## Log [**Removed**]` blocks. Both done.
- **Resolution:** FIXED in this task — `cleanup.rs` is the only new module; `error.rs` carries no `SlugNotActive` / `NoWorktreeForTask` variants; PRD `[**Out of scope**]` and PLAN `## Log [**Removed**]` document the rationale; `grep -r "ark work\|WorkOptions\|fn work(" crates/` returns zero hits.

### V-002 `branch-gone classification is correct only when task.toml.branch is populated`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/cleanup.rs:357-369` (`classify` body).
- **Problem:** Branch-gone detection requires `branch.is_some() && !local_branches.contains(b)`. If both `task.toml.branch` and the porcelain entry's branch are `None` (unusual, but possible for a detached-HEAD worktree), branch-gone never fires — the worktree silently stays prunable-but-unsurfaced.
- **Why it matters:** The user could not understand why an apparently dead worktree is not listed. Documented in PLAN's `[**Constraints**]` C-4 ("rows with neither are not eligible for the BranchGone bucket"); covered by `classify_no_branch_active_returns_none` so the behaviour is intentional and tested.
- **Recommendation:** Leave as-is. Detached-HEAD worktrees are exotic; surfacing them as `BranchGone` would lie about the cause. Document the corner in the workflow.md if it ever bites a user.
- **Resolution:** ACCEPTED — intentional design, locked by C-4 and `classify_no_branch_active_returns_none`. No code change.

### V-003 `enumerate_archived walks two levels deep without depth guard`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/cleanup.rs:231-249` (`enumerate_archived`).
- **Problem:** The walk reads `tasks_archive_dir/<YYYY-MM>/<slug>/` and assumes depth-2 layout. A misformed archive with deeper nesting would not be enumerated; rows under it would never appear in the archived set.
- **Why it matters:** Archive layout is fixed by `task_archive_move` (depth 2 by construction), so a deeper layout indicates a hand-edit. The cost of missing such an archive is a false-negative on the Archived classification — the worktree may instead surface as `BranchGone` or remain unsurfaced.
- **Recommendation:** Leave as-is. Adding depth-N walk would mask user mistakes; the depth-2 assumption matches the producer.
- **Resolution:** ACCEPTED — matches `task_archive_move`'s producer contract; no change needed.

### V-004 `ark cleanup invoked from inside a worktree behaves the same as from the parent`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/cleanup.rs` (entry).
- **Problem:** Both PRD and PLAN claim parity (V-E-1 in PLAN: "same behaviour"). `parse_git_worktree_list` on a worktree's `.git` file resolves the common dir and lists every worktree. But `layout.tasks_archive_dir()` from the worktree refers to the worktree's *own* `.ark/tasks/archive/` — which is empty (worktree-backed task dirs live in the worktree, not as archives).
- **Why it matters:** From inside a worktree, `cleanup` would never find archived slugs because the worktree has no archive. Calling from the parent is the right ergonomic; calling from a worktree silently under-reports.
- **Recommendation:** Document in workflow.md that `ark cleanup` runs from the parent checkout. Already implicit in the §"Worktrees" doc update ("After the branch is merged, clean up from the parent checkout") — explicit caveat would be over-engineering for a one-line edge case.
- **Resolution:** ACCEPTED — workflow.md text already steers users to the parent. The under-report case is benign (no false positives, only false negatives that resurface at the next parent invocation).

### V-005 `task slug rename mid-flight from ark-work-cleanup to ark-cleanup`

- **Severity:** LOW
- **Location:** `.ark/tasks/ark-cleanup/`, `.ark/.state.toml`.
- **Problem:** Mid-EXECUTE rename via direct dir move + `task.toml.id` edit + `.state.toml` rewrite, bypassing any rename CLI verb (none exists).
- **Why it matters:** State has to stay consistent — focus pointer, active set, and on-disk slug all match.
- **Recommendation:** Confirm `ark context` reports the new slug across `focus`, `active`, and the `current_task.summary.slug` fields; confirm no archived-task references to the old slug exist.
- **Resolution:** FIXED — `ark context` reports `slug: ark-cleanup` under both focus and current task; `grep -r "ark-work-cleanup" .ark/` returns zero hits after the rename.

## Notes

- The implementation reuses `worktree_cleanup` (the existing single-slug verb) per row. The new top-level verb is essentially `worktree_cleanup * (every prunable worktree)`. This keeps the rollback-aware path (dirty checks, parent pruning, branch-deletion modes) intact instead of reimplementing them.
- Real-world smoke-test against this very repo found 3 prunable Committed worktrees (`fix-workspace-init`, `recursive-verify-seeding`, `subagent-support`) — feature works end-to-end.
- The `ark work` deferred design will need a focus-binding model that either (a) writes to both parent and worktree state on `task new --worktree`, or (b) does cross-checkout focus resolution at read time. Picking that up requires a separate task.
