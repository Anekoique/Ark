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
**Closing Commit**: 88d2c99

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
| `88d2c99` | fix(worktree): fix worktree synchronization issues |

## Session 3: Tier-aware PLAN naming

**Date**: 2026-05-05
**Slug**: tier-aware-plan-naming
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `12f975b`
**Closing Commit**: ccddfde

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
| `ccddfde` | feat(workflow): tier-aware PLAN naming |

## Session 4: Manifest-aware ark init

**Date**: 2026-05-05
**Slug**: manifest-aware-init
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `ccddfde`
**Closing Commit**: ee62980

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
| `ee62980` | feat(init): manifest-aware platform resolution |

## Session 5: Cap journal Main Changes prose

**Date**: 2026-05-05
**Slug**: prose-discipline
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `ee62980`
**Closing Commit**: 162a509

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
| `162a509` | docs(commit): cap journal Main Changes prose |

## Session 6: Extend SessionStart envelope with suppressOutput and systemMessage

**Date**: 2026-05-06
**Slug**: session-envelope
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `162a509`
**Closing Commit**: 8153c3c

### Summary

Hook output stops dumping a JSON wall in transcripts; one-line `Ark: branch=… · N active · M specs` shows instead.

### Main Changes

| Area | Description |
|------|-------------|
| context | wrap_session_start_envelope adds suppressOutput=true and systemMessage |
| context | trim additionalContext under 9,500 chars; sets truncated=true on drop |

### Git Commits

| Hash | Message |
|------|---------|
| `8153c3c` | feat(context): extend envelope |

## Session 7: Drop --slug from non-targeted task verbs

**Date**: 2026-05-06
**Slug**: drop-task-slug
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `8153c3c`
**Closing Commit**: aac0f97

### Summary

`ark agent task plan/review/execute/verify/commit/promote/archive` resolve the slug from worktree path or active set; stale session ids no longer wedge the verb.

### Main Changes

| Area | Description |
|------|-------------|
| resolver | resolve_slug cascade: worktree path + active-set membership → single active → focus map |
| error | NoCurrentTask split into NoActiveTask and AmbiguousActiveTask with accurate messages |
| cli | --slug dropped from 7 verbs; required on resume and discard (was optional on discard) |
| spec | three feature SPECs and both workflow.md copies updated in lockstep |

### Git Commits

| Hash | Message |
|------|---------|
| `aac0f97` | feat(agent): drop --slug from non-targeted task verbs |

## Session 8: tight docs

**Date**: 2026-05-08
**Slug**: doc-tighten
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `aac0f97`
**Closing Commit**: e169239

### Summary

Restructured commands, workflow.md, and templates into a CLI-driven contract; tightened all 9 feature SPECs to a uniform shape.

### Main Changes

| Area | Description |
|------|-------------|
| commands & skills | Tabular Step N: format with [AI]/[USER] markers and Failure Modes tables across Claude / Codex / OpenCode |
| workflow.md | CLI-driven (Trellis pattern): Quick Start → Lifecycle anchored to ark calls; no narration tables |
| templates | PRD/PLAN/REVIEW/VERIFY/SPEC enforce schema markers + Goal/Constraint length caps with Good/Bad examples |
| feature SPECs | All 9 rewritten — Goals 5–6 verb-led, Non-goals 3, Constraints scoped; Architecture/API surface restored |

### Git Commits

| Hash | Message |
|------|---------|
| `e169239` | feat(workflow): tighten docs |

## Session 9: Brownfield SPEC import via `/ark:extract-spec`

**Date**: 2026-05-08
**Slug**: extract-spec-cmd
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `e169239`
**Closing Commit**: 7d0ae82

### Summary

Brownfield projects can author a feature SPEC from an existing implementation through a confirm-gated discover→synthesize→import flow.

### Main Changes

| Area | Description |
|------|-------------|
| CLI | `ark agent spec import --feature/--scope/--from-file/--from-commit` writes SPEC + INDEX row in one call |
| sharing | `upsert_index_row` and `sanitize_table_field` factored out of `register.rs` so import and register share validation |
| provenance | First CHANGELOG entry stamps `extracted from <short-sha>`; INDEX row uses `from-task = "extracted"` sentinel |
| skill | `/ark:extract-spec <name> [hint]` shipped across Claude command, OpenCode command, Codex skill |

### Git Commits

| Hash | Message |
|------|---------|
| `7d0ae82` | feat(cmd): add `extract-spec` command |

## Session 10: per-checkout `[focus]` replaces session-keyed map

**Date**: 2026-05-08
**Slug**: session-focus-bind
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `7d0ae82`
**Closing Commit**: 79500fd

### Summary

Non-targeted task verbs now resolve via a single per-checkout `[focus]` field instead of a per-session focus map; the `session/` module, `Ppid` trait, and `$TMPDIR` cache scheme are gone.

### Main Changes

| Area | Description |
|------|-------------|
| state | `StateFile.focus: Option<String>`; reconcile clears stale focus instead of pruning sessions; `state_mutate`/`load_state` lose `&dyn Ppid` |
| verbs | `task new`/`resume` set focus and warn on rebind suggesting `--worktree`; `task commit`/`archive`/`discard` clear focus when slug matches |
| errors | `Error::NoFocus { project_root, candidates }` replaces `NoActiveTask` and `AmbiguousActiveTask`; `agent_cli::resolve_slug` reads `state.focus` directly |
| cleanup | First new-binary `state_mutate` strips legacy `[sessions.*]` blocks and unlinks orphan `<temp>/ark-session-<hash>-*.id` files |

### Git Commits

| Hash | Message |
|------|---------|
| `79500fd` | refactor(state): replace per-session focus map with per-checkout |


## Session 11: recursive VERIFY seeding for nested project INDEX.md

**Date**: 2026-05-08
**Slug**: recursive-verify-seeding
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `ab40c67`
**Closing Commit**: 8372470

### Summary

Seeded VERIFY's `Project Spec Compliance` walks `specs/project/INDEX.md` recursively and renders Index-integrity + Leaf-SPECs subsections instead of one flat per-leaf checklist.

### Main Changes

| Area | Description |
|------|-------------|
| seeder | `read_project_spec_tree` walks nested `INDEX.md` rows with cycle guard; missing referenced files skipped silently |
| renderer | `Project Spec Compliance` block emits `### Index integrity` (one PENDING per index) + `### Leaf SPECs` (rolled-up PENDING + traceability sublist) |
| template | `templates/ark/templates/VERIFY.md` comment describes the recursive walk and two-subsection layout |

### Git Commits

| Hash | Message |
|------|---------|
| `8372470` | feat(workflow): add recursive VERIFY seeding |

## Session 12: Scaffold workspace files during ark init

**Date**: 2026-05-10
**Slug**: fix-workspace-init
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `8372470`
**Closing Commit**: f14837c

### Summary

`ark init` now writes `.ark/workspace/index.md` (always) and the per-developer `<dev>/index.md` + Active Developers row when an identity is established.

### Main Changes

| Area | Description |
|------|-------------|
| scaffolding | `scaffold_top_index` / `scaffold_developer_dir` extract the previously inline scaffold logic so `init` can run them eagerly |
| init | `commands/init.rs` calls `scaffold_top_index` after template extraction; CLI's identity bootstrap calls `scaffold_developer_dir` + idempotent `developer_register` |
| idempotence | `bootstrap_workspace` skips `developer_register` when the row already exists, so re-init never clobbers `Last Active` / `Sessions` written by `workspace_record` |

### Git Commits

| Hash | Message |
|------|---------|
| `f14837c` | fix: init workspace correctly |

## Session 13: ark-agent support

**Date**: 2026-05-10
**Slug**: subagent-support
**Branch**: `feat/subagent-support`
**Base Branch**: `main`
**Start Head**: `8372470`
**Closing Commit**: 258f187

### Summary

Three Ark subagents (researcher / reviewer / verifier) ship across Claude / Codex / OpenCode; `/ark:design` wires them into DESIGN, REVIEW, and VERIFY.

### Main Changes

| Area | Description |
| ---- | ----------- |
| agents | `ark-{researcher,reviewer,verifier}` prompts under `templates/{claude,codex,opencode}/agents/` |
| registry | `Platform` gains `agents_templates`/`agents_dest_dir`/`extra_dirs`; `#[non_exhaustive]` |
| install | Claude routes agents through `apply_managed_state`; reserved-stem overwrites on default `init` |
| owned_dirs | derived from `PLATFORMS` so unload/load round-trip Claude's `.claude/agents/` |

### Git Commits

| Hash | Message |
|------|---------|
| `258f187` | feat(workflow): add ark-agent support |
| `f14837c` | fix: init workspace correctly |

## Session 14: ark cleanup

**Date**: 2026-05-10
**Slug**: ark-cleanup
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `f14837c`
**Closing Commit**: e0e8499

### Summary

`ark cleanup` lists prunable worktrees (Committed / Archived / branch-gone) and removes them on `--apply`.

### Main Changes

| Area | Description |
| ---- | ----------- |
| cli | new top-level `ark cleanup`, peer to `ark archive`; clap `requires = "apply"` for `--delete-branch` and `--force` |
| core | `commands/cleanup.rs` reuses `worktree_cleanup` per row; reads each worktree's own `task.toml` for phase |
| docs | workflow.md §"Worktrees" post-merge step + §"CLI surfaces" entry name the new verb |

### Git Commits

| Hash | Message |
|------|---------|
| `e0e8499` | feat(cli): add `ark cleanup` |
| `ec56fc0` | fix: fix historical workspace record |
| `258f187` | feat(workflow): add ark-agent support |

## Session 15: rfc001-arkos

**Date**: 2026-05-11
**Slug**: rfc001-arkos
**Branch**: `docs/rfc001-arkos`
**Base Branch**: `main`
**Start Head**: `e0e8499`
**Closing Commit**: cd50a33

### Summary

RFC 001 positions ArkOS as a workflow substrate for agents — peer to Ark, workload-grounded self-improvement, six first-class open questions.

### Main Changes

| Area | Description |
| ---- | ----------- |
| rfc | new `docs/rfcs/001-arkos.md` (370 lines) establishes substrate framing, layered model, self-improvement discipline |
| docs/rfcs/ | new directory; first numbered RFC; three-digit prefix convention |
| research | three persisted prior-art surveys under task `research/` (self-improving agents, recursive decomposition, self-generating specs) |

## Session 16: guard journal stamp contract

**Date**: 2026-05-11
**Slug**: guard-journal-stamp
**Branch**: `feat/guard-journal-stamp`
**Base Branch**: `main`
**Start Head**: `fcfd341`
**Closing Commit**: 8071272

### Summary

CLI now refuses `task commit` when the journal's last `## Session` heading is already stamped, with a message naming the missing block.

### Main Changes

| Area | Description |
| ---- | ----------- |
| error | new `Error::JournalSessionHeadingMissing { journal_path, slug }` variant with actionable message |
| stamp | `assert_unstamped` helper wired into `stamp_task` + `stamp_manual`; refuses before any write |
| tests | 3 new in `stamp::tests`, 1 new in `record::tests`; 461 → 465 |

### Git Commits

| Hash | Message |
|------|---------|
| `cd50a33` | feat(rfc): add rfc001-arkos |

## Session 17: support detachable feature SPEC

**Date**: 2026-05-18
**Slug**: detachable-feature-spec
**Branch**: `feat/detachable-feature-spec`
**Base Branch**: `main`
**Start Head**: `5186b2a`
**Closing Commit**: 395ba8d

### Summary

Generalize `specs/features/` into a recursive tree; deep-tier PRDs declare a required `[**SPEC Path**]` that `task commit` reads to land the SPEC and walk INDEXes leaf-to-root.

### Main Changes

| Area | Description |
| ---- | ----------- |
| spec | PRD `[**SPEC Path**]` parser + `Layout::specs_feature_dir(&[&str]) -> Result<PathBuf>`; leaf-to-root INDEX upsert with branch discriminator `<seg>/INDEX.md` |
| context | `SpecRow.feature_path` + `GatherWarning` (MissingChild / OrphanLeaf / OrphanSubtree); recursive INDEX-strict walk with depth bound |
| commit | `RollbackGuard.features_indexes: Vec<_>` snapshots every level pre-mutation; reverse-order restore |
| docs | recursive `features/` tree documented; new EXECUTE rule keeps Ark workflow context out of shipped source |

### Git Commits

| Hash | Message |
|------|---------|
| `395ba8d` | feat(spec): support detachable feature SPEC |

## Session 18: drop installed_at from manifest

**Date**: 2026-05-18
**Slug**: drop-installed-at
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `395ba8d`
**Closing Commit**: fc1078f

### Summary

Drop `installed_at` from `.installed.json` so no-op upgrades stop churning the manifest diff; serde ignores the legacy field on read.

### Main Changes

| Area | Description |
| ---- | ----------- |
| manifest | Remove `Manifest.installed_at` + `chrono::{DateTime, Utc}` import from `state/manifest.rs` |
| upgrade | Drop `manifest.installed_at = Utc::now()` and unused `chrono::Utc` import in `commands/upgrade/mod.rs` |
| spec | `features/ark-upgrade/SPEC.md` pipeline diagram + Manifest struct lose the field |

### Git Commits

| Hash | Message |
|------|---------|
| `fc1078f` | feat(manifest): drop timestamp at .installed.json |

## Session 19: add `ark research`

**Date**: 2026-05-20
**Slug**: ark-research
**Branch**: `feat/ark-research`
**Base Branch**: `main`
**Start Head**: `fc1078f`
**Closing Commit**: 60e3221

### Summary

Introduce a fourth workflow tier whose deliverable is a curated reference corpus under `<task>/research/`, not a code change. Follow-up implementation optional.

### Main Changes

| Area | Description |
| ---- | ----------- |
| tier | `Tier::Research` + `Phase::Research`; sole legal lifecycle `Research → Committed → Archived` |
| commit | research-tier branch skips VERIFY gate + SPEC extract; `ark_files_for_first_commit` stages `task.toml` + `research/**` |
| promote | `task_promote` rejects every source-or-target involving research with `Error::WrongTier` |
| platforms | `/ark:research` shipped across Claude / Codex / OpenCode (Codex applies closed-form substitution map) |

### Git Commits

| Hash | Message |
|------|---------|
| `60e3221` | feat(workflow): add `ark research` |

## Session 20: research on agent harness and agent infra

**Date**: 2026-05-21
**Slug**: agent-harness-infra
**Branch**: `main`
**Base Branch**: `main`
**Start Head**: `7b5d107`
**Closing Commit**: <PENDING:agent-harness-infra>

### Summary

78-file reference corpus mapping 2026 agent-harness / agent-infra landscape; synthesis ranks `ark-mcp` as the highest-leverage next move.

### Main Changes

| Area | Description |
| ---- | ----------- |
| corpus | 10 sections × per-topic markdown under `research/`; 13.8K lines; INDEX.md per section with cross-refs |
| prior art | 18 peer profiles (Aider, Cline, OpenHands, Devin, Cursor, Zed, agent-platforms survey, ...) using a fixed template |
| synthesis | 11 cross-cutting findings (F1–F11); A/B/C/D direction ranking; Q3'26–Q2'27 roadmap sketch |
| process | mixed dispatch + main-session writes: 9 parallel `ark-researcher` subagents; 4 stalled on watchdog; recovered via disk-as-truth |

### Git Commits

| Hash | Message |
|------|---------|
| _(none)_ |   |

