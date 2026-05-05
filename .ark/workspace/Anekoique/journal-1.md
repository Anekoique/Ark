## Session 1: Workspace journals with deferred-slot SHA recording

**Date**: 2026-05-02
**Slug**: workspace
**Branch**: `feat/workspace`
**Base Branch**: `main`
**Start Head**: `73d46ba`
**Closing Commit**: 6a796a1

### Summary

Re-introduce per-developer workspace journals. The deferred-slot mechanism resolves the closing-commit-SHA chicken-and-egg without amending or chore-committing.

### Main Changes

| Area | Description |
|------|-------------|
| identity | `.ark/.developer` (gitignored) is the single source of truth; `ark init --developer <name>` bootstraps |
| transaction | `RecordTransaction` snapshots before any mutation; suffix-checked rollback preserves concurrent appender data |
| stamp | Agent appends a session block; CLI inserts auto-fields after the heading. No temp file, no `--entry-file`, no parser round-trip |
| archive | `git log -S '**Slug**: <slug>'` collect-then-classify pickaxe replaces the `<PENDING:<slug>>` sentinel with the real short SHA at archive time. Idempotent |
| precondition | `ark archive` requires a clean git index; `ArchiveIndexNotEmpty` errors otherwise |
| context | `ark context --scope record` (additive on the existing `Scope` enum) projects identity + active journal + branch for slash commands |
| guard | Existing `RollbackGuard` extended to adopt `RecordSnapshot` and track workspace paths; SPEC + features-INDEX coverage preserved |
| platforms | `/ark:record` and updated `/ark:commit` shipped across Claude / Codex / OpenCode in lockstep |

### Git Commits

| Hash | Message |
|------|---------|
| `6a796a1` | feat(workflow): add workspace support |

## Session 2: Worktree sync — auto-mirror .developer + default submodule init

**Date**: 2026-05-05
**Slug**: worktree-sync-defaults
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `1a129d8`
**Closing Commit**: <PENDING:worktree-sync-defaults>

### Summary

New worktrees inherit the parent's `.ark/.developer` and run `git submodule update --init --recursive` by default.

### Main Changes

| Area | Description |
|------|-------------|
| identity-sync | copy parent's `.ark/.developer` into the worktree; prompt on TTY |
| submodule default | default `post_create` runs `git submodule update`; set `[]` to opt out |

### Git Commits

| Hash | Message |
|------|---------|
| `96865d5` | fix(worktree): fix worktree synchronization issues |

## Session 3: Tier-aware PLAN naming

**Date**: 2026-05-05
**Slug**: tier-aware-plan-naming
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `12f975b`
**Closing Commit**: <PENDING:tier-aware-plan-naming>

### Summary

Standard/quick tasks seed `PLAN.md`; deep keeps `NN_PLAN.md`. Locators accept both so legacy archives still resolve.

### Main Changes

| Area | Description |
|------|-------------|
| seeding | tier-aware: standard/quick → `PLAN.md`, deep → `NN_PLAN.md` |
| locators | all readers accept both forms via `locate_latest_plan` |
| promote | Standard→Deep renames `PLAN.md` → `00_PLAN.md` to preserve the body |

### Git Commits

| Hash | Message |
|------|---------|
| _(none)_ |   |

## Session 4: Manifest-aware ark init

**Date**: 2026-05-05
**Slug**: manifest-aware-init
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `ccddfde`
**Closing Commit**: <PENDING:manifest-aware-init>

### Summary

Re-running `ark init` on an installed project skips the platform prompt; CLI flags still override.

### Main Changes

| Area | Description |
|------|-------------|
| resolution | derive default platform set from `.ark/.installed.json` when no flags |
| ux | stderr line announces the detected set; flags still win on conflict |

### Git Commits

| Hash | Message |
|------|---------|
| _(none)_ |   |

## Session 5: Cap journal Main Changes prose

**Date**: 2026-05-05
**Slug**: prose-discipline
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `ee62980`
**Closing Commit**: <PENDING:prose-discipline>

### Summary

Slash-command templates now require ≤ 4 short rows per session block; existing Sessions 2–4 rewritten under the new rule.

### Main Changes

| Area | Description |
|------|-------------|
| templates | added `Style — keep it tight` to all three `/ark:commit` siblings |
| journal | rewrote Sessions 2–4 in `journal-1.md` under the new caps |

### Git Commits

| Hash | Message |
|------|---------|
| _(none)_ |   |
