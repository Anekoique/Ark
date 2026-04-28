# `worktree-support` PRD

---

[**What**]
Add a `worktree` subcommand group under `ark agent task` that binds an Ark task to a dedicated git worktree, so multiple tasks can run in parallel without colliding on `.ark/tasks/.current`, branch state, or each other's edits. Worktree creation is opt-in — `ark agent task new` keeps existing behavior unchanged. Configuration lives in a new `.ark/worktree.toml` (project-tracked).

[**Why**]
Today `.ark/` assumes one task at a time per checkout: `.current` points at one slug, the working tree carries one branch's edits, and any in-flight `/ark:design --deep` blocks any other task. This forces serial work and rules out the "Multi-agent Orchestrate" / "parallel subagents" lines on the ROADMAP.

The `reference/Trellis/` codebase shows the shape of a working solution (per-task git worktrees, opt-in config, agents tracked by branch). But Trellis welds two concerns together — parallelism (worktrees) and per-developer state (workspaces) — and reinvents config parsing in 200 lines of hand-rolled YAML. We can ship the parallelism half cleanly: TOML config, typed Rust CLI, no process supervision, no journal/identity ceremony. Workspaces (per-developer journals) are deliberately deferred to a follow-up — the name `worktree.toml` (singular) reflects that scoping.

[**Outcome**]

A user with a clean `.ark/`-loaded repo can:

1. Run `ark agent task new --slug foo --tier deep` and get the existing behavior (no worktree, edits land on the current branch).
2. Run `ark agent task new --slug foo --tier deep --worktree` (or `task new` followed by `task worktree create`) and get:
   - A new git worktree at `.ark/worktrees/<branch>/` (under repo root, sibling to `.ark/tasks/`; configurable via `worktree.toml`).
   - A new branch `<type>/foo` based on the current HEAD (default `<type> = feat`, override per-invocation with `--branch-type fix|refactor|chore|ci|docs` or fully with `--branch <full>`; project-wide default in `worktree.toml`'s `branch_prefix`). Naming follows the repo's existing convention (`feat/codex-support`, `fix/workflow-…`, etc.).
   - The HEAD branch at creation time recorded as `base_branch` in `task.toml` for future PR targeting.
   - `task.toml` updated with `branch` and `worktree_path` fields.
   - Files listed in `worktree.toml` `copy` (e.g. `.env`) copied into the worktree.
   - `post_create` hooks executed in the worktree dir, in order, aborting on first failure.
3. `cd` into the worktree, run their AI agent, and have `ark context` resolve the right task automatically (worktree has its own `.ark/tasks/.current` carrying the correct slug).
4. Run `ark agent task worktree list` to see all active worktrees with `{slug, branch, worktree_path, base_branch}`.
5. Run `ark agent task worktree cleanup --slug foo` after merging to remove the worktree dir under `.ark/worktrees/` and (optionally with `--delete-branch`) prune the branch. Cleanup is **never** automatic on archive — `task archive` leaves the worktree intact for the user to clean up explicitly.
6. Pre-existing tasks created without `--worktree` continue to work — the new fields default to `None` via `#[serde(default)]`.

Verification gate: `cargo test -p ark-core -p ark-cli` passes; integration test scaffolds a temp git repo, creates two worktree-backed tasks, advances both through PLAN simultaneously, and asserts no `.current` collision.

[**Related Specs**]

- `specs/features/ark-agent-namespace/SPEC.md` — extends the `ark agent task` subcommand group with `task worktree {create, cleanup, list}`; adds `Error::WorktreeExists`/`WorktreeNotFound`/`BranchInUse`/`WorktreeConfigMissing`; adds `branch: Option<String>` and `worktree_path: Option<PathBuf>` to `TaskToml` (both `#[serde(default)]`). Respects the SPEC's "named commands, no generic setters" principle and "no process spawning beyond what `task archive` already does."
- `specs/features/ark-context/SPEC.md` — `Layout::discover_from` already walks ancestors looking for `.ark/`, so `ark context` invoked from inside a worktree finds the right project. This task verifies the worktree-side `.current` resolves correctly via the existing logic and adds a regression test; no API changes to `ark context`.
