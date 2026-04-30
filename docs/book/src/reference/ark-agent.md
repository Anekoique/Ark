# `ark agent`

The hidden internal namespace that drives task lifecycle, spec management, and workspace journaling.

## Stability

`ark agent` is **not** semver-stable. The contract is between the shipped binary and the shipped templates — they version together. End users should drive the workflow through slash commands (`/ark:quick`, `/ark:design`, `/ark:archive`, `/ark:record`) rather than calling `ark agent` directly. The sole reasons to reach for it manually:

- Rare lifecycle operations the slash commands don't wrap (e.g. tier promotion mid-flight).
- Debugging a stuck workflow.
- Building automation around Ark in CI.

`ark --help` does **not** list `agent`. To explore the namespace:

```bash
ark agent --help
```

The header includes a stability disclaimer.

## Subcommand tree

```
ark agent
├── task
│   ├── new            # scaffold a new task dir
│   ├── plan           # transition design → plan
│   ├── review         # transition plan → review (deep only)
│   ├── execute        # transition plan/review → execute
│   ├── verify         # transition execute → verify
│   ├── archive        # transition verify → archived (move dir)
│   ├── promote        # change tier mid-flight
│   └── worktree
│       ├── list       # enumerate worktree-backed tasks
│       └── cleanup    # remove a worktree dir + optionally delete branch
├── spec
│   ├── extract        # extract PLAN's `## Spec` to specs/features/<name>/SPEC.md
│   └── register       # add a row to specs/features/INDEX.md's managed block
└── workspace
    ├── init           # bootstrap .ark/.developer + workspace dir
    └── record         # write a manual session entry
```

## Common patterns

### Task lifecycle

```bash
# Quick tier — three commands.
ark agent task new --slug fix-typo --title "fix typo" --tier quick
# ... edit code ...
ark agent task execute
ark agent task archive

# Standard — five.
ark agent task new --slug rate-limit --title "add rate limit" --tier standard
# ... fill PRD ...
ark agent task plan
# ... fill 00_PLAN.md ...
ark agent task execute
# ... edit code, commit ...
ark agent task verify
# ... fill VERIFY.md ...
ark agent task archive

# Deep — review loop adds two more.
ark agent task new --slug auth --title "auth refactor" --tier deep --worktree
cd .ark/worktrees/feat/auth
# ... fill PRD ...
ark agent task plan
# ... fill 00_PLAN.md ...
ark agent task review
# ... fill 00_REVIEW.md ...
# If revisions needed: copy NN_PLAN.md / NN_REVIEW.md to NN+1_*, bump iteration in task.toml,
# reset phase = "plan", and run `ark agent task review` again.
ark agent task execute
# ... edit, commit ...
ark agent task verify
ark agent task archive   # promotes SPEC, registers in features INDEX
```

### Worktree management

```bash
# Create a worktree-backed task.
ark agent task new --slug rate-limit --tier deep --worktree
# Override the branch type from `feat` (default) to `fix`:
ark agent task new --slug ratelimit-bug --tier standard --worktree --branch-type fix

# List all worktree-backed tasks.
ark agent task worktree list

# Clean up after merge.
ark agent task worktree cleanup --slug rate-limit
ark agent task worktree cleanup --slug rate-limit --delete-branch
ark agent task worktree cleanup --slug rate-limit --force   # for unmerged branches
```

### Spec promotion

Normally invoked by `ark agent task archive` on deep tier; you can call it directly when reopening an archived task:

```bash
ark agent spec extract --slug auth                # extract PLAN's `## Spec` → specs/features/auth/SPEC.md
ark agent spec register --slug auth               # add row to specs/features/INDEX.md
```

### Workspace journal

```bash
# Bootstrap identity (also done by `ark init --developer alice`).
ark agent workspace init --name alice

# Manual session entry (also done by /ark:record).
ark agent workspace record --title "investigated SQL injection report" \
    --summary "Looked at lines 42-89 of db.rs; no injection vectors..." \
    --next "- Add fuzzing for the query builder."
```

## Defaults

Every subcommand that takes `--slug` defaults to `.ark/tasks/.current` when omitted. Every command takes optional `--dir <path>` to override project discovery.

## Errors

The state machine enforces legal transitions and rejects illegal ones. Common errors:

- `IllegalPhaseTransition` — running e.g. `ark agent task review` on a quick-tier task, or on a task already past plan.
- `WrongTier` — attempting a deep-only operation (extract spec) on a standard or quick task.
- `TaskExistsOnParent` — creating a `--worktree` task whose slug already exists in the parent's `.ark/tasks/`.
- `NestedWorktreeForbidden` — invoking `task new --worktree` from inside an existing `.ark/worktrees/<branch>/` checkout.
- `WorktreeDirty` — `worktree cleanup` without `--force` on a worktree with uncommitted changes.

Hand-edits to `task.toml` are sometimes the right escape hatch (specifically: bumping `iteration`, resetting `phase` to `plan`, or reopening an archived task). The state machine is small and the file is short; the agent does this when needed and the slash commands document when.

## See also

- [Lifecycle](../workflow/lifecycle.md) — the conceptual model.
- [Worktrees and Workspaces](../workflow/worktrees-and-workspaces.md) — when and why.
