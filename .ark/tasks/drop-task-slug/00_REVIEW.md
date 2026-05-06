# `drop-task-slug` REVIEW `00`

> Reviewer: Opus 4.7 sub-agent
> Plan: `00_PLAN.md`
> Verdict: Approved with Revisions

## Counts
- Blocking (CRITICAL + HIGH): 5
- Non-blocking (MEDIUM + LOW): 5

## Findings

### R-001 [HIGH] G-7 / Phase 4: SPEC update list omits `worktree` SPEC G-2, G-4
`worktree/SPEC.md` G-4 reads `task worktree cleanup [--slug <s>] [--delete-branch] [--force]` and G-2 cites the same flag. PLAN drops slug from `WorktreeCleanupCliArgs` but doesn't list worktree SPEC for revision. Lockstep violation. Add `worktree/SPEC.md` to Phase 4 edits; drop `[--slug <s>]` from G-4 signature and G-2 mention.

### R-002 [HIGH] G-3 / Architecture: Worktree-path resolution skips active-set membership check
Cascade returns `Ok(slug)` when `slug_from_worktree_root + task_dir.is_dir()` succeed, without verifying the slug is in `state.tasks.active`. Edge case: archived task with worktree dir still present, hand-recreated `tasks/<slug>/` shell. Tighten: also require `state.tasks.active.contains(&slug)`.

### R-003 [HIGH] G-4 / T-1: `discard` cascade is a footgun
Discard with single-active fallback silently picks `a` if user opens fresh shell at project root and runs `ark agent task discard` by accident — destructive deletion driven by implicit resolution. `--force` only guards content divergence, not target selection. Recommendation: keep `--slug` mandatory on discard (align with `resume`).

### R-004 [HIGH] Phase 3 / V-UT-10: focus-branch test not deterministic
`resolve_slug(root)` constructs `RealPpid::new()` internally. Tests can't inject `StubPpid`. Cache files in `std::env::temp_dir()` shared across parallel test workers → flaky. Fix: thread `&dyn Ppid` through `resolve_slug` signature, matching `load_state` / `lookup_session_id` upstream convention.

### R-005 [HIGH] G-5: `AmbiguousActiveTask` message inaccurate when no candidates have worktrees
Hard-coded message `"cd into a worktree or run resume --slug"` lies when neither active task has a worktree (worktrees are opt-in per `worktree/SPEC.md` G-7). Drop the cd advice; only say `run resume --slug <one of: a, b>`.

### R-006 [MEDIUM] C-1: lexical matching ignores symlinks and case
Symlinked checkouts (common on shared dev servers) and case-insensitive macOS volumes can defeat lexical matching against hardcoded lowercase `WORKTREES_DIR`. Document the limitation as NG-7, or call `fs::canonicalize` (best-effort) once before component split.

### R-007 [MEDIUM] G-8: workflow.md prose audit miss
`templates/ark/workflow.md` and `.ark/workflow.md` lines 165, 195 contain `ark agent task archive --slug <s>` and `ark agent task worktree cleanup --slug <s>` as prose examples. They lie post-flag-removal. Phase 4 must edit both files; the grep-clean step in 4.3 is wrong about no edits.

### R-008 [MEDIUM] C-1: empty-component guard documents non-existent failure
`Path::components()` collapses trailing slashes — never yields `Normal("")`. C-1's empty-component guard tests an impossible case. Replace with the real edge: `OsStr::to_str()` returning `None` for non-UTF-8. Drop V-UT-4 (redundant with V-F-2).

### R-009 [MEDIUM] Acceptance mapping: G-7 and C-7 lack programmatic validation
Manual SPEC review isn't a validation. Add Phase 5 grep gate: `grep -rn 'NoCurrentTask\|\[--slug <s>\]' .ark/specs/features/{ark-agent-namespace,task-concurrency-control,worktree}/SPEC.md` must return zero hits.

### R-010 [LOW] T-2: `lookup_session_id` Result handling
Cache-tier IO errors propagate via `?`, aborting the whole phase command on `EACCES`. Downgrade to `lookup_session_id(...).ok().flatten()` so cache failures gracefully fall through to `AmbiguousActiveTask`.

## Trade-off Advice
- T-1: agreed — keep `--slug` mandatory on `discard`.
- T-2: agreed — `lookup` over `resolve` for read paths.
- T-3: agreed lexical, but document symlink/case limitation per R-006.
