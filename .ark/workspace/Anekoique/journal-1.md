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
| Hash | Message |
|------|---------|
| `1cc1fe2` | feat(workflow): add workspace management |
| `1078727` | fix(workflow): require --worktree for deep-tier tasks |
| `c318367` | feat(workflow): add worktree management |
| `3dbf56d` | feat(platform): add opencode support |
| `f19c8e9` | feat(platform): add codex support |
| `71f09b1` | fix(workflow): spec extract self contained |
| `1ce3e11` | feat(cli): add `ark context` command |
| `d204466` | chore: bump version to 0.1.2 |
| `15fdb46` | feat(cli): add `ark upgrade` command |
| `c2cb0d5` | feat(cli): add `ark agent` namespace |
| `7e93a49` | ci: sanitize OAuth token before use |
| `86bd796` | chore: bump version to 0.1.1 |
| `a91984f` | chore: bootstrap ark workflow in-repo |
| `48e3704` | ci: add basic ci |
| `f4860ea` | feat(cli): add basic cli framework |
| `f548489` | feat(templates): add basic workflow framework |
| `0c824ec` | feat: init project ark |

### Next Steps
- merge feat/workspace to main
