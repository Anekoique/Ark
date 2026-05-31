# `ark-cleanup` PLAN

> Status: Draft
> Feature: `ark-cleanup`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`

---

## Summary

Add one stable top-level CLI verb — `ark cleanup` — that sits alongside `ark archive` and `ark context` and bridges the gap left by worktree feature NG-2 ("no automatic cleanup on archive; cleanup is a separate user step"). The command enumerates worktrees whose backing task is Committed, Archived, or whose branch is gone, and removes them on `--apply` by reusing the existing `worktree_cleanup` per slug. No new abstractions in core, only one new entrypoint and CLI glue.

## Log

[**Removed**]

- Originally this PLAN paired `ark cleanup` with `ark work` (a path printer for the focused task's worktree). Dropped during EXECUTE: `task new --worktree` writes focus into the *worktree's* state, not the parent's, so a "current focus, optional --slug" lookup makes `ark work` only useful from inside the worktree — defeating the parent-side `cd "$(ark work)"` use case the feature was meant to enable. Dropping `ark work` here removed two `Error` variants (`SlugNotActive`, `NoWorktreeForTask`) and the `commands/work.rs` module from the planned tree.

---

## Spec

[**Goals**]

- G-1: `ark cleanup` lists prunable worktrees in dry-run, removes them on `--apply`.
- G-2: `ark cleanup` is a stable top-level CLI peer to `ark archive` and `ark context`.
- G-3: Cleanup detects three reasons: `committed`, `archived`, `branch-gone`.

[**Non-goals**]

- NG-1: No remote-aware merge detection; local `git branch --list` only.
- NG-2: No new `task.toml` fields; reuses `worktree_path`, `branch`, `phase`.
- NG-3: No mutation of state, focus, or task.toml; cleanup removes the worktree dir and (on `--delete-branch`) the branch only.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                      (Command::Cleanup, CleanupArgs)
└── ark-core/src/
    ├── lib.rs                               (re-exports cleanup API)
    └── commands/
        ├── mod.rs                           (pub mod cleanup)
        └── cleanup.rs                       (NEW; `cleanup(opts) -> CleanupSummary`)
```

Module coupling: `commands::cleanup` depends on `state::load_state`, `commands::agent::state::{Phase, TaskToml}`, `commands::agent::task::worktree::{discovery, cleanup as wt_cleanup, config}`, plus a parent-side archive-dir walk. No new shells; `git branch --list` goes through `io::git::run_git`.

Call graph for `ark cleanup`:

```
cleanup(opts)
  ├── layout = Layout::new(opts.project_root)
  ├── cfg = WorktreeConfig::load_or_default(&layout)
  ├── worktrees_dir = cfg.resolve_worktrees_dir(&layout)
  ├── archived: HashSet<slug>           ← walk layout.tasks_archive_dir()/<YYYY-MM>/<slug>/
  ├── local_branches: HashSet<branch>   ← run_git(["branch","--list","--format=%(refname:short)"])
  ├── candidates = []
  ├── for entry in parse_git_worktree_list(layout.root())?:
  │     if !is_under(&entry.path, &worktrees_dir): continue
  │     wt_layout = Layout::new(&entry.path)
  │     state = load_state(&wt_layout).ok()? ; else skip silently
  │     for slug in state.tasks.active:
  │         if --slug is set and != slug: continue
  │         toml = TaskToml::load(wt_layout.task_dir(slug)).ok()?  ; else skip silently
  │         branch = toml.branch.or(entry.branch)
  │         reason = classify(slug, toml.phase, branch, &archived, &local_branches)
  │         if reason.is_some():
  │             candidates.push(Row { slug, branch, path, reason })
  ├── if !opts.apply:
  │     return CleanupSummary { dry_run: true, planned: candidates, .. }
  └── for row in candidates:
        match worktree_cleanup(WorktreeCleanupOptions { slug, delete_branch, force }):
            Ok(s)  => summary.successes.push(s)
            Err(e) => summary.failures.push((slug, e.to_string()))
        // worktree_cleanup already prunes empty parents (worktree feature C-15).
```

`classify` priority is `archived → committed → branch-gone → None`. Branch-gone applies only when the worktree carries a branch in `task.toml.branch` or the porcelain entry's branch and that name is not in `local_branches`. Worktrees where every detection rule says "not prunable" are omitted.

Worktree-backed tasks live entirely inside their worktree (the parent's `tasks/` dir does not contain the slug). Cleanup therefore reads each worktree's *own* `task.toml` for the phase check — no parent-side live-phase walk is needed. Archive lookup remains parent-side: archives are global to the project root.

[**Data Structure**]

```rust
// ark-core/src/commands/cleanup.rs
#[derive(Debug, Clone)]
pub struct CleanupOptions {
    pub project_root: PathBuf,
    pub slug: Option<String>,
    pub apply: bool,
    pub delete_branch: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupReason {
    Committed,
    Archived,
    BranchGone,
}

#[derive(Debug, Clone)]
pub struct CleanupRow {
    pub slug: String,
    pub branch: Option<String>,
    pub worktree_path: PathBuf,
    pub reason: CleanupReason,
}

#[derive(Debug, Clone, Default)]
pub struct CleanupSummary {
    pub dry_run: bool,
    pub planned: Vec<CleanupRow>,
    pub successes: Vec<WorktreeCleanupSummary>,
    pub failures: Vec<(String, String)>,
}
// Display:
//   dry-run, empty:   "ark cleanup: nothing to prune"
//   dry-run, rows:    "ark cleanup --dry-run: N candidate(s)" + per-row "  {slug} {branch} [{reason}] {path}"
//   apply, rows:      "ark cleanup: K removed, M failed"      + per-row success/failure lines
//   apply, empty:     "ark cleanup: nothing to prune"
```

No new `Error` variants. Existing `Error::WorktreeDirty`, `Error::WorktreeConfigCorrupt`, `Error::Io`, etc. cover every failure mode.

[**API Surface**]

```rust
pub fn cleanup(opts: CleanupOptions) -> Result<CleanupSummary>;

// Library re-exports added in lib.rs:
//   cleanup, CleanupOptions, CleanupSummary, CleanupRow, CleanupReason

// CLI shape (ark-cli/src/main.rs)
enum Command {
    // ...existing variants unchanged...
    Cleanup(CleanupArgs),
}

#[derive(clap::Args)]
struct CleanupArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(long)] slug: Option<String>,
    #[arg(long, default_value_t = false)] apply: bool,
    #[arg(long = "delete-branch", requires = "apply", default_value_t = false)] delete_branch: bool,
    #[arg(long, requires = "apply", default_value_t = false)] force: bool,
}
```

[**Constraints**]

- C-1: `ark cleanup` defaults to dry-run; `--apply` is required to mutate. `--delete-branch` and `--force` require `--apply` (clap `requires = "apply"`).
- C-2: Discovery iterates `parse_git_worktree_list(root)` filtered by `is_under(path, worktrees_dir)`. Third-party worktrees with no readable `.ark/` state or `task.toml` are silently skipped — same rule as worktree feature C-20.
- C-3: Reason classification priority is `Archived` > `Committed` > `BranchGone`; one row per worktree, never duplicated across reasons.
- C-4: The candidate's branch is resolved from `task.toml.branch`, falling back to the porcelain `branch` field; rows with neither are not eligible for the `BranchGone` bucket.
- C-5: The Committed phase is read from each worktree's own `<wt>/.ark/tasks/<slug>/task.toml`, not the parent's. Worktree-backed tasks never appear in the parent's `tasks/` dir.
- C-6: The Archived slug set is read from the parent's `.ark/tasks/archive/<YYYY-MM>/<slug>/` directory names (no `task.toml` parse needed for membership).
- C-7: `--apply` invokes `worktree_cleanup` per row with the user's `delete_branch` and `force`; per-row failures are collected, never abort the loop. Empty parent dir pruning is delegated to `worktree_cleanup` (worktree feature C-15).
- C-8: All git invocations route through `io::git::run_git`. No `Command::new` in `commands/cleanup.rs`.
- C-9: All filesystem reads in `commands/cleanup.rs` route through `io::PathExt::list_dir` or `Path::is_dir`; no bare `std::fs::*` outside `#[cfg(test)]`.
- C-10: The CLI exits 1 only when at least one row failed under `--apply`; exit 0 in every other path (mirrors `ark archive`).
- C-11: `ark cleanup --slug <s>` filters at enumeration time; no separate `validate_slug` call is made (state membership is the only check).
- C-12: `ark cleanup` does NOT mutate `.ark/.state.toml`, the focus pointer, or any `task.toml`. The only state side effect is whatever `worktree_cleanup` already performs (none in current implementation).
- C-13: Workflow doc update is scoped: the §"CLI surfaces" block lists `ark cleanup` alongside `ark archive`; the §"Worktrees" block points to `ark cleanup` instead of `ark agent task worktree cleanup` for the post-merge step. No other doc rewrites.

---

## Runtime

[**Main Flow**]

1. `Command::Cleanup` dispatches with `--dir` resolved via `resolve_with_discovery`.
2. `cleanup(opts)` builds the archived slug set + the local branch set.
3. For each git worktree under `worktrees_dir`, classify each active slug from the worktree's own `task.toml`.
4. Filter by `--slug` if given; on dry-run return `CleanupSummary { dry_run: true, planned, .. }`.
5. On `--apply`, invoke `worktree_cleanup` per row, collect outcomes.
6. CLI exits 0 on success or full dry-run; exit 1 only when at least one row failed in `--apply`.

[**Failure Flow**]

1. Enumeration errors (`Error::Io` from `read_dir`, `Error::Git*` from `run_git`, `Error::WorktreeConfigCorrupt`) abort the run before any mutation.
2. Per-row `worktree_cleanup` failures are collected; the CLI exits 1 if any row failed, but every other row is still attempted.

[**State Transitions**]

- None. `cleanup --apply` mutates the filesystem (worktrees) and optionally branches via `git`. It does not touch `.ark/.state.toml` or `task.toml`.

---

## Implementation

[**Phase 1 — Core skeleton**]

1. Create `ark-core/src/commands/cleanup.rs` with the four public structs (`CleanupOptions`, `CleanupReason`, `CleanupRow`, `CleanupSummary`) and a `cleanup(opts)` body returning `CleanupSummary::default()`. Wire `pub mod cleanup;` in `commands/mod.rs`. Re-export the public surface from `lib.rs`.

[**Phase 2 — Enumeration + dry-run**]

1. Implement archived slug set (walk `layout.tasks_archive_dir()/*/*/` directory names).
2. Implement local branch set via `run_git(["branch","--list","--format=%(refname:short)"], root)`.
3. Implement `classify` (Phase + branch + archived + local_branches → `Option<CleanupReason>`).
4. Implement per-worktree iteration; wire `--slug` filter at iteration time.
5. Tests: classify happy paths (active/committed/archived/branch-gone/no-branch), dry-run end-to-end (empty repo, active task skipped, committed surfaced, slug filter narrows).

[**Phase 3 — Apply + CLI wiring**]

1. Implement `--apply` loop calling `worktree_cleanup` per row, collecting `Ok`/`Err`.
2. Add `Command::Cleanup` and `CleanupArgs` to `ark-cli/src/main.rs`, including clap `requires` annotations.
3. Wire the dispatch arm with `render`. Exit 1 on `--apply` partial failure.
4. Tests: apply removes worktree dir, second invocation prints `nothing to prune`, `--delete-branch` deletes the branch, per-row failure collected without aborting.
5. Update `.ark/templates/ark/workflow.md` per C-13 (CLI surfaces table + Worktrees post-merge step).

---

## Trade-offs

- T-1: **Place `cleanup.rs` directly under `commands/` (chosen) vs. nest under `commands/agent/task/worktree/`.** Top-level placement reflects the user-visible CLI surface (`ark cleanup`, not `ark agent task cleanup`); `ark agent` is the hidden agent-namespace verb space. Nesting would force re-export plumbing and confuse the ownership rule already documented in `worktree/mod.rs:1-5` ("Worktree creation lives in `task::new`; this module owns cleanup and list").
- T-2: **Branch-gone detection: local-only `git branch --list` (chosen) vs. `git branch --merged main` vs. `git ls-remote`.** Local-only is fast, deterministic, and offline. The user explicitly chose this in design — it catches the dominant case (deleted local branch after merge) without depending on remote shape or merge analysis. The cost is missing "branch still exists locally but the PR was merged"; that case is left to the existing `ark agent task worktree cleanup --slug <s>`.
- T-3: **Dry-run by default (chosen) vs. apply by default with `--dry-run`.** `ark archive` defaults to apply, but archive's failure mode is benign (a directory move). `cleanup --apply` runs `git worktree remove` and (with `--delete-branch`) `git branch -D`; an accidental run on a worktree with uncommitted scratch is genuinely lossy. Dry-run default matches workflow.md NG-2's "cleanup is a deliberate step" framing.
- T-4: **Reuse `worktree_cleanup` per row (chosen) vs. inline a bulk loop that bundles git calls.** Reuse keeps every removal traversing the existing rollback-aware path (dirty checks, parent pruning, branch deletion modes). Per-row overhead is acceptable; cleanup is rare and its row count tiny.
- T-5: **Dropped `ark work` (chosen, mid-flight) vs. ship it as a worktree-only tool.** Without cross-checkout focus resolution, `ark work` from the parent fails with `NoFocus` for deep-tier tasks — the exact ergonomic the feature targeted. Shipping it half-functional would mislead users; deferring lets cross-checkout focus be designed properly in a follow-up.

---

## Validation

[**Unit Tests**]

- V-UT-1: `classify` returns `None` for an Active task with a live branch.
- V-UT-2: `classify` returns `Committed` for `phase = Committed` with a live branch.
- V-UT-3: `classify` returns `Archived` when the slug is in the archived set, even if `phase = Committed` (precedence test).
- V-UT-4: `classify` returns `BranchGone` for a non-Committed phase whose branch is missing from the local set.
- V-UT-5: `classify` returns `None` when there is no branch field and the task is active (BranchGone needs a branch).

[**Integration Tests**]

- V-IT-1: `cleanup` against an empty repo (no worktrees) returns dry-run + empty `planned`, Display reads `ark cleanup: nothing to prune`.
- V-IT-2: `cleanup` against a worktree-backed Active task returns empty `planned` (active tasks are not surfaced).
- V-IT-3: `cleanup` against a worktree-backed Committed task surfaces one row with reason `Committed`.
- V-IT-4: `cleanup` with `--slug <s>` narrows the planned list to one row.
- V-IT-5: `cleanup --apply --force` removes the worktree dir; a second `cleanup` invocation prints `nothing to prune`.
- V-IT-6: `cleanup --apply --delete-branch --force` removes both worktree and branch (`git branch --list <b>` empty after).

[**Failure / Robustness**]

- V-F-1: `cleanup --apply` with one dirty + one clean Committed worktree collects exactly one failure (the dirty one) and one success (the clean one); the loop never aborts.

[**Edge Cases**]

- V-E-1: `cleanup` invoked from inside a worktree (not the parent): same behaviour — `parse_git_worktree_list` returns every worktree regardless of cwd.
- V-E-2: `cleanup --apply --force` with an unmerged branch + `--delete-branch`: branch is force-deleted (relies on existing `worktree_cleanup` semantics).
- V-E-3: Source-scan invariant: production code in `commands/cleanup.rs` contains no bare `std::fs::*` calls and no hand-joined `.ark/` path literals.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-3, V-IT-5 |
| G-2 | V-IT-1, V-IT-3 (CLI peer behaviour: top-level dispatch + Display) |
| G-3 | V-UT-2, V-UT-3, V-UT-4 |
| C-1 | V-IT-2 vs V-IT-5 (default vs `--apply`); clap `requires` enforced at CLI parse time |
| C-2 | manual review of `enumerate_candidates` (silent skip on `load_state.ok()?` and `TaskToml::load.ok()?`) |
| C-3 | V-UT-3 |
| C-4 | V-UT-4, V-UT-5 |
| C-5 | V-IT-3 (Committed worktree surfaced even though parent's `tasks/` is empty) |
| C-6 | manual review of `enumerate_archived` walk depth |
| C-7 | V-F-1 |
| C-8 | V-E-3 (source scan); manual review |
| C-9 | V-E-3 |
| C-10 | manual review of CLI dispatch + `summary.failures.is_empty()` exit-code check |
| C-11 | manual review (no `validate_slug` call) |
| C-12 | manual review (no `state_mutate` / `clear_focus_for_slug` call in `cleanup.rs`) |
| C-13 | manual review during VERIFY |
