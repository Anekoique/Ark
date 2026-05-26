# `ark agent`

The hidden internal namespace that drives task lifecycle and spec management.

## Stability

`ark agent` is **not** semver-stable. The contract is between the shipped binary and the shipped templates — they version together. End users should drive the workflow through slash commands (`/ark:quick`, `/ark:design`, `/ark:commit`) rather than calling `ark agent` directly. The sole reasons to reach for it manually:

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
│   ├── commit         # atomically close: SPEC extract + single git commit
│   ├── archive        # transition committed → archived (move dir)
│   ├── resume         # claim an active task as this session's focus
│   ├── discard        # remove an unarchived task (--force if it has user content)
│   ├── promote        # change tier mid-flight (rejected for research tier)
│   └── worktree
│       ├── list       # enumerate worktree-backed tasks
│       └── cleanup    # remove a worktree dir + optionally delete branch
└── spec
    ├── extract        # extract PLAN's `## Spec` to specs/features/<path>/SPEC.md
    ├── import         # author a feature SPEC from an existing codebase (brownfield)
    └── register       # upsert a row in specs/features/INDEX.md's managed block
```

## Common patterns

### Task lifecycle

`task new --tier` accepts `quick`, `standard`, `deep`, or `research`.

```bash
# Quick tier.
ark agent task new --slug fix-typo --title "fix typo" --tier quick
# ... edit code ...
ark agent task execute
ark agent task commit
# Bulk-archive Committed tasks later via `ark archive`.

# Standard.
ark agent task new --slug rate-limit --title "add rate limit" --tier standard
# ... fill PRD ...
ark agent task plan
# ... fill 00_PLAN.md ...
ark agent task execute
# ... edit code, stage work ...
ark agent task verify
# ... fill VERIFY.md ...
ark agent task commit

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
# ... edit, stage ...
ark agent task verify
ark agent task commit   # promotes SPEC, registers in features INDEX, single git commit
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

To sweep *all* stale worktrees at once (any whose backing task is Committed/Archived or whose branch is gone), use the visible top-level `ark cleanup` — dry-run by default, `--apply` to act. It calls `worktree cleanup` per row.

### Spec promotion

Normally invoked by `ark agent task commit` on deep tier; you can call it directly when reopening an archived task:

```bash
ark agent spec extract --slug auth                # extract PLAN's `## Spec` → specs/features/<path>/SPEC.md
ark agent spec register --slug auth               # upsert row in specs/features/INDEX.md
```

The destination path comes from the PRD's `[**SPEC Path**]` block — slash-separated, leaf-to-root INDEX upsert. See [Specs](../workflow/specs.md) for the recursive layout. For a feature that already exists in code but has no SPEC, `ark agent spec import` (via `/ark:extract-spec`) authors one without a deep-tier task.

## Defaults

`task new`, `task resume`, and `task discard` require `--slug`. Every other subcommand resolves the slug from this checkout's **focus** — the single slug recorded in `.ark/.state.toml`'s `[focus]` field. `task new` and `task resume` write the focus; `task commit` / `task archive` / `task discard` clear it when their slug matches. With no focus bound, a non-targeted verb errors `NoFocus` and asks you to run `task resume --slug <s>` first.

Every command takes optional `--dir <path>` to override project discovery.

## Errors

The state machine enforces legal transitions and rejects illegal ones. Common errors:

- `IllegalPhaseTransition` — running e.g. `ark agent task review` on a quick-tier task, or on a task already past plan.
- `WrongTier` — attempting a deep-only operation (extract spec) on a standard or quick task.
- `TaskExistsOnParent` — creating a `--worktree` task whose slug already exists in the parent's `.ark/tasks/`.
- `NestedWorktreeForbidden` — invoking `task new --worktree` from inside an existing `.ark/worktrees/<branch>/` checkout.
- `WorktreeDirty` — `worktree cleanup` without `--force` on a worktree with uncommitted changes.
- `NoFocus` — a non-targeted verb run with no `[focus]` bound; run `task new` or `task resume --slug <s>` first.

Hand-edits to `task.toml` are sometimes the right escape hatch (specifically: bumping `iteration`, resetting `phase` to `plan`, or reopening an archived task). The state machine is small and the file is short; the agent does this when needed and the slash commands document when.

## See also

- [Lifecycle](../workflow/lifecycle.md) — the conceptual model.
- [Worktrees](../workflow/worktrees.md) — when and why.
