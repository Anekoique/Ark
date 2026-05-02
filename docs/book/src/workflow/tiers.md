# Tiers

Ark has three tiers. Pick the smallest that fits.

| Tier     | Trigger              | Artifacts                                                              | Path through states                                  |
| -------- | -------------------- | ---------------------------------------------------------------------- | ---------------------------------------------------- |
| Quick    | `/ark:quick`         | `PRD.md`                                                               | design → execute → archived                          |
| Standard | `/ark:design`        | `PRD.md`, `PLAN.md`, `VERIFY.md`                                       | design → plan → execute → verify → archived          |
| Deep     | `/ark:design --deep` | `PRD.md`, `NN_PLAN.md`, `NN_REVIEW.md`, `VERIFY.md`, promoted `SPEC.md` | design → plan ⇄ review → execute → verify → archived |

The same triggers exist for every supported platform: Claude Code slash commands (`/ark:quick`), Codex skills (`ark-quick`), OpenCode commands (`/ark:quick`).

## When to use each

```
quick:    reversible + no new abstractions
deep:     breaking / cross-cutting / new subsystem
standard: everything else
```

**Quick** is for changes you'd land without a code review in a sane workflow: typo fixes, doc edits, dependency bumps, trivial bug fixes. One markdown file, one commit, done. No PLAN, no VERIFY.

**Standard** is the default for feature work. The PRD captures the *what and why*; the PLAN elaborates *how* with explicit Goals (G-1, G-2, …), Constraints (C-1, …), and a Validation table that maps every Goal to at least one test. VERIFY checks the shipped code against the PLAN's Validation matrix.

**Deep** adds a REVIEW step between PLAN and EXECUTE. The agent (or a separate reviewer model) writes a verdict — *Approved* / *Approved with Revisions* / *Rejected* — and findings (R-001, …). If revisions are needed, the agent copies the artifacts to `01_PLAN.md` / `01_REVIEW.md` and iterates. Loop continues until verdict is *Approved* with zero open CRITICAL findings. On archive, the final PLAN's `## Spec` section is promoted to a permanent feature SPEC under `.ark/specs/features/<name>/`.

## Tier promotion mid-flight

If you start standard and the change turns out to be deeper than you thought, promote without losing artifacts:

```bash
ark agent task promote --to deep
```

Demotion (`--to standard`) is also supported. The PRD and any existing PLAN survive; the only thing that changes is `task.toml.tier` and the workflow path the slash commands take from here.

## Parallel tasks

Two in-flight tasks would collide on `.ark/tasks/.current`. To run them side by side, pass `--worktree` at scaffold time:

```bash
ark agent task new --slug foo --tier deep --worktree
```

This creates a git worktree at `.ark/worktrees/feat/foo/` (override branch with `--branch-type fix|refactor|...` or `--branch <full>`) and scaffolds the task dir *inside the worktree*. The parent checkout's `.current` is untouched. `--worktree` is **required** for deep tier.

Configure copies and post-create hooks via `.ark/config.toml`'s `[worktree]` section. After the branch merges, run `ark agent task worktree cleanup --slug foo` from the parent to remove the directory. Archive does NOT auto-clean the worktree.

See [Worktrees](./worktrees.md) for the full surface.
