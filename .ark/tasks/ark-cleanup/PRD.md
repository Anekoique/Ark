# `ark-cleanup` PRD

---

[**What**]

Add one stable top-level CLI verb: `ark cleanup` lists (and on `--apply`, removes) worktrees whose backing task is Committed, Archived, or whose backing branch is gone locally.

[**Why**]

The deliberate-but-missing user step the worktree feature SPEC calls out (NG-2, workflow.md:273): once a deep-tier task commits, its worktree dir lingers indefinitely under `.ark/worktrees/<branch>/`, and there is no batch verb for the common case "all my Committed/Archived tasks' worktrees, please". The single-slug `ark agent task worktree cleanup --slug <s>` already exists — but the user has to know each slug. A bulk dry-run + `--apply` flow makes post-merge housekeeping a one-liner.

[**Outcome**]

- `ark cleanup` (zero args) prints a dry-run report listing every prunable worktree under this checkout's `worktrees_dir`. Each row is `<slug> <branch> [<reason>] <path>`, where reason ∈ {`committed`, `archived`, `branch-gone`}. Exit 0 even when the list is empty (prints `ark cleanup: nothing to prune`). No mutation.
- `ark cleanup --apply` removes each listed worktree by reusing `worktree_cleanup` per slug; per-slug failures are collected and reported, one bad row never blocks the rest. Branch deletion is opt-in via `--delete-branch` (matches the existing `agent task worktree cleanup` flag). After removals, empty parent dirs under `worktrees_dir` are pruned (delegated to `worktree_cleanup` per worktree feature C-15).
- `--slug <s>` narrows `ark cleanup` to a single slug; useful for scripting one prunable target.
- `--force` forces removal of dirty worktrees and force-deletes unmerged branches; only meaningful with `--apply`.
- `--delete-branch` and `--force` require `--apply` (clap-enforced).
- Detection rules:
  - **committed**: this worktree's own `.ark/tasks/<slug>/task.toml` has `phase = Committed` (worktree-backed tasks live entirely inside their worktree per the existing feature contract — the parent's `tasks/` is empty for them).
  - **archived**: slug appears under the parent's `.ark/tasks/archive/YYYY-MM/<slug>/`.
  - **branch-gone**: the backing branch is missing from `git branch --list <branch>` (local check, no network, no merge analysis).
- A worktree with no readable Ark task data (third-party worktree under `worktrees_dir`) is silently skipped — same rule as `worktree_list` C-20.
- `ark cleanup` is a stable, user-visible top-level verb, peer to `ark archive` and `ark context`. It does **not** live under `ark agent`.
- Exit 1 only when at least one row failed under `--apply`; exit 0 otherwise (mirrors `ark archive`).
- The closing commit ships: CLI wiring, one new core entrypoint (`cleanup`), tests, and a workflow.md update naming the new command beside the existing worktree section.

[**Related Specs**]

- `specs/features/worktree/SPEC.md` — sole interaction. New command extends the worktree surface: `ark cleanup` reuses `worktree_cleanup` and `parse_git_worktree_list` from `task::worktree::{cleanup, discovery}`. No new SPEC variants on `task.toml`. Worktree feature C-18 (archive does not touch worktrees) stays — `ark cleanup` is the matching user-driven verb that *does*.
- `specs/features/ark-context/SPEC.md` — no interaction; `ark cleanup` does not mutate context state and does not need a phase projection.

---

[**Out of scope**]

- Originally this PRD also proposed `ark work` to print the focused task's worktree path. Dropped during EXECUTE: `task new --worktree` writes focus into the *worktree's* state, not the parent's, so a "current focus, optional --slug" lookup makes `ark work` only useful from inside the worktree — defeating the parent-side `cd "$(ark work)"` use case. A cross-checkout fallback is the only design that works, but that is a non-trivial extension to the focus model and was deferred to a separate task.
