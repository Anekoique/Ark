# Worktrees and Workspaces

Two related but separate concerns: **worktrees** isolate parallel tasks; **workspaces** journal what each developer worked on.

## Worktrees

A git worktree is a second checkout of the same repository that shares the `.git` directory. Multiple branches can be checked out simultaneously, each in its own directory.

Ark uses worktrees to run multiple tasks in parallel without the `.ark/tasks/.current` file colliding. When you pass `--worktree` at scaffold time:

```bash
ark agent task new --slug rate-limit --tier deep --worktree
```

Ark:

1. Validates the branch name via `git check-ref-format --branch`.
2. Resolves the branch: explicit `--branch <full>` wins; else `<--branch-type>/<slug>` (e.g. `fix/rate-limit`); else `<config.toml's [worktree].branch_prefix>/<slug>` (default `feat/<slug>`).
3. Refuses if the parent's `.ark/tasks/<slug>/` already exists, or if some other worktree already owns the slug, or if the target directory exists, or if the branch is already checked out elsewhere.
4. Runs `git worktree add -b <branch> <worktree_path> <base_branch>`.
5. Copies any files listed in `[worktree].copy` (e.g. `.env`).
6. Runs any commands in `[worktree].post_create` (with cwd set to the worktree).
7. Scaffolds the task dir *inside the worktree*'s `.ark/tasks/<slug>/`.

The parent checkout's `.ark/` is never modified by a `--worktree` task.

**Deep tier requires `--worktree`.** Standard and quick are opt-in.

### Working in the worktree

After scaffold, `cd .ark/worktrees/<branch>/`. Run all subsequent phase commands from there:

```bash
cd .ark/worktrees/feat/rate-limit
ark agent task plan
# ... fill 00_PLAN.md ...
ark agent task review
# ... fill 00_REVIEW.md ...
ark agent task execute
# ... edit code, commit ...
ark agent task verify
# ... fill VERIFY.md ...
ark agent task archive
```

The worktree has its own `.ark/` independent of the parent's. Slash commands run inside it act on that `.ark/`.

### Cleanup after merge

Archive does **not** auto-clean the worktree. After the branch has been merged into your default branch, run from the parent checkout:

```bash
ark agent task worktree cleanup --slug rate-limit
ark agent task worktree cleanup --slug rate-limit --delete-branch  # also delete the branch
```

`cleanup` refuses if the worktree is dirty unless you pass `--force`. `--force` also escalates `git branch -d` to `git branch -D` so unmerged-branch deletion works.

To enumerate active worktree-backed tasks:

```bash
ark agent task worktree list
```

Each row is one line: `<slug> <branch> <path>`. Empty stdout when there are zero rows.

### Configuration

`.ark/config.toml`'s `[worktree]` section:

```toml
[worktree]
worktree_dir = ".ark/worktrees"   # where worktrees go (project-relative)
branch_prefix = "feat"             # default branch prefix
copy = [".env", ".envrc"]          # files to copy into each new worktree
post_create = [                    # commands to run after `git worktree add`
  "cargo build",
  "npm install",
]
```

`worktree_dir` must stay project-relative; absolute paths and `..` traversal are rejected. `post_create` commands run sequentially with cwd set to the new worktree; the first non-zero exit aborts the whole `task new --worktree` and rolls back the worktree dir.

## Workspaces

A workspace is a per-developer journal at `.ark/workspace/<name>/`, written automatically when a task archives. The intent is: when a coworker (or future-you) opens the repo a month from now, they can read your workspace journal and reconstruct what you did and why.

### Identity

Each developer is identified by name. Identity is set in two places:

- **`.ark/.developer`** — gitignored, per-machine. Set by `ark init --developer <you>` or `ark agent workspace init --name <you>`. This is what the auto-record path reads.
- **`.ark/workspace/<you>/`** — the journal directory itself. Created the first time a developer initializes.

If `.ark/.developer` is absent, `task archive` skips the workspace record (returning `SkippedNoIdentity`) and emits a one-line stderr diagnostic. Initialize identity later with:

```bash
ark agent workspace init --name alice
```

This is idempotent for the same name. Using a different name on a project that already has a developer initialized errors with `DeveloperAlreadyInitialized`.

### Journal shape

`.ark/workspace/<dev>/`:

```
index.md              # session table + status (managed-block re-rendered each write)
journal-1.md          # entries 1..N
journal-2.md          # entries N+1..2N (rotates at the line cap)
...
```

Each session entry in a journal file looks like:

```markdown
## Session 1: rate-limit task

**Date**: 2026-04-29
**Kind**: task
**Slug**: rate-limit
**Branch**: `feat/rate-limit`

### Summary
Did the thing.

### Commits

| Hash | Message |
|------|---------|
| `abc1234` | feat: add rate-limit middleware |

### Next Steps
- Add metrics for rate-limit hits.
```

`Kind` is `task` (auto-recorded by `task archive`) or `manual` (recorded by `/ark:record`). For `manual`, `Slug` is `-` and `Branch` is whatever the cwd's git HEAD says.

### Configuration

`.ark/config.toml`'s `[workspace]` section:

```toml
[workspace]
journal_max_lines = 2000                # rotate to journal-N+1.md when this many lines reached
auto_record_on_archive = true           # set to false to disable archive auto-record
```

`journal_max_lines` must be ≥ 100 (smaller caps cause index re-render to thrash). Setting `auto_record_on_archive = false` disables the bridge entirely; `task archive` skips workspace writes without reading identity or invoking git.

### Per-checkout journals

Journals are **per-checkout**. A `record_task` call from inside a worktree writes to *that worktree's* `.ark/workspace/<dev>/`, not the parent's. The session entry rides along with the task commit on the same branch — when the branch merges, the journal entry merges with it.

This means: each developer's workspace becomes a per-branch ledger. The default branch's journal accumulates entries from completed tasks; in-flight branches show what's still in motion.

### Manual records

For non-task work — research spikes, debugging sessions, doc passes — invoke `/ark:record`:

```
/ark:record investigated SQL injection report
```

The slash command prompts for an optional summary and next-steps list, then writes a `manual` entry to the active journal. Mirrors archive's auto-record path; works whenever a developer is initialized.

## Why both?

Worktrees solve a *now* problem: parallel tasks can't share `.current`. Workspaces solve a *later* problem: looking back at what was done last month.

You can use one without the other. A solo project on a single branch needs neither. A team project with multiple in-flight features wants both.
