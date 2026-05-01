# Journal — Anekoique (Part 1)

> AI development session journal. Started: 2026-04-29

---


## Session 1: ship workspace feature

**Date**: 2026-04-29
**Kind**: manual
**Slug**: -
**Branch**: `feat/workspace`

### Summary
Designed, planned (3 iter), executed, verified, and archived per-developer session journal feature. New ark agent workspace init/record commands, /ark:record slash command, auto-record on task archive (tier-agnostic), .ark/config.toml merging worktree+workspace settings, ark init --developer flag for identity bootstrap. Late-stage rip of parent-root resolution: journal is now per-checkout (rides with the task commit on the same branch) instead of always-on-parent. Simpler, matches mental model.

### Commits
| Hash      | Message                                               |
| --------- | ----------------------------------------------------- |
| `1cc1fe2` | feat(workflow): add workspace management              |
| `1078727` | fix(workflow): require --worktree for deep-tier tasks |
| `c318367` | feat(workflow): add worktree management               |
| `3dbf56d` | feat(platform): add opencode support                  |
| `f19c8e9` | feat(platform): add codex support                     |
| `71f09b1` | fix(workflow): spec extract self contained            |
| `1ce3e11` | feat(cli): add `ark context` command                  |
| `d204466` | chore: bump version to 0.1.2                          |
| `15fdb46` | feat(cli): add `ark upgrade` command                  |
| `c2cb0d5` | feat(cli): add `ark agent` namespace                  |
| `7e93a49` | ci: sanitize OAuth token before use                   |
| `86bd796` | chore: bump version to 0.1.1                          |
| `a91984f` | chore: bootstrap ark workflow in-repo                 |
| `48e3704` | ci: add basic ci                                      |
| `f4860ea` | feat(cli): add basic cli framework                    |
| `f548489` | feat(templates): add basic workflow framework         |
| `0c824ec` | feat: init project ark                                |

### Next Steps
- merge feat/workspace to main

## Session 2: ship 0.1.3 — ark-book, comment-tag sweep, project-spec

**Date**: 2026-04-30
**Kind**: manual
**Slug**: -
**Branch**: `main`

### Summary
Combined backfill covering the post-workspace work that landed on `main` for the 0.1.3 cut. Three pieces shipped this day:

1. **ark-book (standard tier)** — stood up `docs/book/` mdBook covering install, workflow, CLI reference, platform integrations, contributor guide. GitHub Actions workflow wired to build and deploy to Pages on tag.
2. **comment-tag sweep** — stripped task-mark tags (`V-IT-*`, `V-UT-*`, `V-E-*`, `C-N`, `G-N`, `R-N`) from doc-comments and inline comments across `crates/` — 234 instances across 38 files. Comments stay; only the tag tokens go.
3. **project-spec (deep tier)** — refactored COMMENTS.md and authored STYLE.md + ERRORS.md under a new convention-SPEC layout (Layout A: `Purpose / Rules / Exceptions / Examples / See Also`), defined authoritatively in a new `specs/project/LAYOUT.md`. Followed by a `crates/` refactor: `cargo fmt` clean, `clippy -D warnings` clean, four files >800 LOC decomposed (`io/fs.rs`, `commands/upgrade.rs`, `ark-cli/src/main.rs`, `commands/agent/task/new.rs`). 321 tests pass; CLI `--help` text unchanged.

Workspace identity (`.ark/.developer`) was missing on this clone after the workspace feature shipped — every archive between 2026-04-29 and now silently emitted "no developer set; skipping workspace record". Identity bootstrapped today; future archives auto-record.

### Commits

| Hash      | Message                                     |
| --------- | ------------------------------------------- |
| `f1a94d4` | refactor(crates): comply with project specs |
| `dbb3964` | feat(specs): add project-spec               |
| `06b0267` | chore: bump version to 0.1.3                |
| `22e0045` | docs: add ark-book                          |
| `f0fbb56` | style: format all comments with SPEC        |

### Next Steps
- tag and publish 0.1.3
- consider auto-prompting for `--developer` on first archive when `.ark/.developer` is missing (gap surfaced this session)

## Session 3: exempt project/INDEX.md from upgrade prompt

**Date**: 2026-04-30
**Kind**: task
**Slug**: fix-project-index-upgrade
**Branch**: `main`

### Summary
Archived `fix-project-index-upgrade` (quick). Folded `.ark/specs/project/INDEX.md`, `.ark/config.toml`, and the rest of `.ark/specs/project/` into `is_exempted` so `ark upgrade` skips classification entirely for user-owned seed-only paths. See `.ark/tasks/archive/2026-04/fix-project-index-upgrade` for the task artifacts.

### Commits

(none — task code changes were uncommitted at archive time)

### Next Steps
- commit the seed-only-exemption change
- follow-up quick task: stop `record_task` from logging unrelated commits when `base_branch` is `None`
