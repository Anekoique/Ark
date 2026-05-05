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

`task new --worktree` now syncs developer identity from the parent into the new worktree (prompting on TTY when missing), and `WorktreeConfig::default().post_create` ships `git submodule update --init --recursive` so submodules initialize automatically.

### Main Changes

| Area | Description |
|------|-------------|
| identity-sync | new `sync_identity()` step in `scaffold_inside_worktree`: `identity_resolve(parent)` → mirror; `MissingIdentity` + TTY → `identity_prompt` → write parent then worktree; non-TTY → return `MissingIdentity` so agents surface the existing remediation message |
| post_create default | `WorktreeConfig::default().post_create` now `["git submodule update --init --recursive"]`; safe no-op without `.gitmodules`; users disable explicitly via `post_create = []` in `.ark/config.toml` |
| template parity | `templates/ark/config.toml [worktree].post_create` matches the code default; new `worktree_template_default_matches_code` test pins parity |
| spec | `.ark/specs/features/worktree/SPEC.md` G-1 amended (default change + corrected stale `.ark/worktree.toml` → `.ark/config.toml [worktree]`), G-13 added (identity-sync goal), NG-7 refined to permit documented-default submodule hook while keeping code submodule-agnostic |
| tests | shared `init_repo` helpers in `new_tests.rs` / `cleanup.rs` / `list.rs` / `discovery.rs` now seed `.ark/.developer` so all existing worktree tests still pass; added `worktree_creation_mirrors_parent_identity`, `worktree_creation_fails_on_missing_identity_when_non_tty`, `worktree_post_create_default_runs_submodule_init`, `worktree_creation_succeeds_when_user_overrides_post_create_to_empty` |

### Git Commits

| Hash | Message |
|------|---------|
| `96865d5` | fix(worktree): fix worktree synchronization issues |
