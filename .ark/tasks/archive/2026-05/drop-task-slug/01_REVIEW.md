# `drop-task-slug` REVIEW `01`

> Reviewer: Opus 4.7 sub-agent
> Plan: `01_PLAN.md`
> Verdict: Approved with Revisions

## Counts
- Blocking (CRITICAL + HIGH): 2
- Non-blocking (MEDIUM + LOW): 4

## Round-1 Verification

All ten R-NNN findings from `00_REVIEW.md` ✅ resolved. Response Matrix entries match PLAN body changes (call graph, Data Structure blocks, Phase plans, error message text, NG-7 documentation, grep gate).

## New Findings

### N-1 [HIGH] G-2 / Phase 4: Discard template tri-edit missing
Shipped templates document the bare form `ark agent task discard` on a code line:
- `templates/claude/commands/ark/discard.md:35`
- `templates/codex/skills/ark-discard/SKILL.md:35`
- `templates/opencode/commands/ark/discard.md:34`

After making `--slug` required, those documented invocations exit non-zero. Add Phase 4.6: edit each of the three discard template files. Drop the bare form and trailing focus-defaults comment.

### N-2 [HIGH] Phase 2.1 / API Surface: `run_phase` signature change unspecified
PLAN says "construct `let ppid = RealPpid::new();` once at the top of each `dispatch` function" but `run_phase(a: TaskSlugArgs, f: ...)` (called by Plan/Review/Execute/Verify) has no `ppid` parameter today. To honor C-11 testability, `run_phase` must accept `&dyn Ppid`. Spell out the new signature: `fn run_phase<F>(a: TaskSlugArgs, ppid: &dyn Ppid, f: F) -> anyhow::Result<()>`. Otherwise an executor leaves `RealPpid` baked into `run_phase`.

### N-3 [MEDIUM] V-UT-11 / Phase 3.1: Cache fixture needs unique-ppid pattern + cleanup
V-UT-11 prescribes `StubPpid(42)` as a literal. Cache files live under shared `std::env::temp_dir()` keyed by `(project_hash, ppid)`. Parallel cargo-test workers using `StubPpid(42)` race. Existing tests in `cache.rs:248` use `unique_test_ppid()` (counter-derived). Adopt that pattern + cleanup guard.

### N-4 [MEDIUM] Phase 4.5: workflow.md has THREE `--slug` lines, not two
Line 141 also contains `ark agent task worktree cleanup --slug <s>` in prose. Phase 5.1.1 grep would catch it but Phase 4.5 should enumerate lines 141, 165, 195 explicitly. Also tighten line 191 prose to enumerate `task new`, `task resume`, `task discard`.

### N-5 [LOW] Order-of-operations rationale missing
Add one-line note to T-3 or new T-4: "load state first because the active-set guard requires it; `load_state` is a single TOML parse." Prevents future maintainer from "optimizing" ordering and dropping C-10.

### N-6 [LOW] worktree SPEC G-4 stale `.current` reference noted but not fixed
Defensible scope-discipline. Track as follow-up task `worktree-spec-current-cleanup`. No code change.

## Trade-off Advice
- T-1, T-2: agreed (carried from 00_REVIEW).
- T-3: agreed; add the load-state-first rationale per N-5.
