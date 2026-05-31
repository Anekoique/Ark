# Worktrees

A git worktree is a second checkout of the same repository that shares the `.git` directory. Multiple branches can be checked out simultaneously, each in its own directory.

Ark uses worktrees to run multiple tasks in parallel without the per-checkout `.ark/` state file colliding. When you pass `--worktree` at scaffold time:

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

## Working in the worktree

After scaffold, `cd .ark/worktrees/<branch>/`. Run all subsequent phase commands from there:

```bash
cd .ark/worktrees/feat/rate-limit
ark agent task plan
# ... fill PLAN.md ...
ark agent task review
# ... fill REVIEW.md, then fold its findings into PLAN.md in place ...
ark agent task execute
# ... edit code, commit ...
ark agent task verify
# ... fill VERIFY.md ...
ark agent task commit
ark agent task archive
```

The worktree has its own `.ark/` independent of the parent's. Slash commands run inside it act on that `.ark/`.

## Cleanup after merge

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

## Configuration

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
