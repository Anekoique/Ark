# Tiers

Ark has four tiers. Pick the smallest that fits.

| Tier     | Trigger              | Artifacts                                                              | Path through states                                  |
| -------- | -------------------- | ---------------------------------------------------------------------- | ---------------------------------------------------- |
| Quick    | `/ark:quick`         | `PRD.md`                                                               | design → execute → archived                          |
| Standard | `/ark:design`        | `PRD.md`, `PLAN.md`, `VERIFY.md`                                       | design → plan → execute → verify → archived          |
| Deep     | `/ark:design --deep` | `PRD.md`, `PLAN.md`, `REVIEW.md`, `VERIFY.md`, promoted `SPEC.md`       | design → plan → review → execute → verify → archived |
| Research | `/ark:research`      | `PRD.md`, `research/`                                                  | research → committed → archived                      |

The same triggers exist for every supported platform: Claude Code slash commands (`/ark:quick`), Codex skills (`ark-quick`), OpenCode commands (`/ark:quick`).

## When to use each

```
quick:    reversible + no new abstractions
deep:     breaking / cross-cutting / new subsystem
research: you can't write a PRD yet — the question is what to build
standard: everything else
```

**Quick** is for changes you'd land without a code review in a sane workflow: typo fixes, doc edits, dependency bumps, trivial bug fixes. One markdown file, one commit, done. No PLAN, no VERIFY.

**Standard** is the default for feature work. The PRD captures the *what and why*; the PLAN elaborates *how* with explicit Goals (G-1, G-2, …), Constraints (C-1, …), and a Validation table that maps every Goal to at least one test. VERIFY checks the shipped code against the PLAN's Validation matrix.

**Deep** adds a single REVIEW step between PLAN and EXECUTE. The agent (or a separate reviewer model) writes a verdict — *Approved* / *Approved with Revisions* / *Rejected* — and findings (R-001, …) into `REVIEW.md`. There is no loop: the author folds every CRITICAL/HIGH finding back into `PLAN.md` in place, then advances to EXECUTE. On commit, the PLAN's `## Spec` section is promoted to a permanent feature SPEC under `.ark/specs/features/<path>/`.

**Research** is for the question *before* the PRD — when you can't yet state what to build, or whether to build at all. The deliverable is a reference corpus under `research/`, not shipped code; follow-up implementation is optional. There's no PLAN, REVIEW, VERIFY, or SPEC promotion — the lifecycle is just `research → committed → archived`. See [Subagents](./subagents.md) for the distinction between a research-tier task and dispatching the embedded `ark-researcher` inside a tiered task.

## Tier promotion mid-flight

If you start standard and the change turns out to be deeper than you thought, promote without losing artifacts:

```bash
ark agent task promote --to deep
```

Demotion (`--to standard`) is also supported. The PRD and any existing PLAN survive; the only thing that changes is `task.toml.tier` and the workflow path the slash commands take from here.

**Research tier does not participate in promotion.** `ark agent task promote` is rejected whenever the source or target is research. To cross over from a finished research corpus to implementation, close the research task and start a fresh `/ark:quick` or `/ark:design` whose PRD cites the research slug in prose.

## Parallel tasks

Each checkout drives **one focused task** at a time — the slug recorded in `.ark/.state.toml`'s `[focus]`. Starting a second task in the same checkout rebinds that focus onto the new slug (you get a warning). To run two tasks side by side, give each its own checkout by passing `--worktree` at scaffold time:

```bash
ark agent task new --slug foo --tier deep --worktree
```

This creates a git worktree at `.ark/worktrees/feat/foo/` (override branch with `--branch-type fix|refactor|...` or `--branch <full>`) and scaffolds the task dir *inside the worktree*. The worktree owns its own `.ark/.state.toml`, so its focus is independent of the parent checkout's. `--worktree` is **required** for deep tier.

Configure copies and post-create hooks via `.ark/config.toml`'s `[worktree]` section. After the branch merges, run `ark agent task worktree cleanup --slug foo` from the parent to remove the directory. Archive does NOT auto-clean the worktree.

See [Worktrees](./worktrees.md) for the full surface.
