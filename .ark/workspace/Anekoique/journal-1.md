## Session 1: Workspace journals with deferred-slot SHA recording

**Date**: 2026-05-02
**Slug**: workspace
**Branch**: `feat/workspace`
**Base Branch**: `main`
**Start Head**: `73d46ba10f39`
**Closing Commit**: <PENDING:workspace>

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
| _(none)_ |   |
