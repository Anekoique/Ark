# `ark-workflow-refactor` PLAN `01`

> Status: Revised
> Feature: `ark-workflow-refactor`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none`
> - Related Specs: `workspace`, `task-concurrency-control`, `ark-agent-namespace`, `ark-context`, `ark-upgrade`, `worktree`, `codex-support`, `opencode-support`, `project-spec`

---

## Summary

Iteration 01 keeps the lifecycle shape from `00_PLAN` (verify → commit → bulk-archive) but rewrites the commit protocol so it stops trying to put a commit's own SHA inside that commit. The journal entry now records `start_head` and `base_branch` instead of a self-referential `commit_range`; `task.toml` is included in the task-closing commit (saved before commit, rolled back on failure); `task_archive` is split into a side-effect-free `task_archive_move` and the legacy bundling wrapper is deleted, so bulk archive cannot redo work that already happened in `task_commit`; the new top-level `ark archive` accepts a per-task `committed_at` to derive the YYYY-MM bucket; the `ark context --for commit` projection stays body-free per the `ark-context` SPEC's contract; concurrent `/ark:commit` is declared out of scope rather than papered over with a fake lock.

## Log

[**Added**]

- **G-2a / C-5a:** `task.toml.committed_head: Option<String>` — the SHA of the task-closing commit, written by `task_commit` after the commit lands. Lives in `task.toml`, not the journal entry, so it is not self-referential.
- **G-3a / C-4a:** `task.toml` is staged into the task-closing commit alongside work + journal entry. `task_commit` saves the new toml (phase=Committed, committed_at=now) **before** running `git commit`, with a rollback path that restores the prior toml + truncates the appended journal entry on commit failure.
- **G-6a / C-18a:** `task_archive_move(slug, archive_month)` is the new side-effect-free archive helper. The legacy `task_archive` is deleted (was already only a slash-command path, which the refactor removes).
- **NG-12:** Concurrent `/ark:commit` invocations on the same checkout are unsupported. The user is responsible for serializing.

[**Changed**]

- **G-3:** Step ordering revised. New ordering: precondition → working-tree check → VERIFY gate → SPEC extract (deep) → render+append journal entry → save new task.toml → `git add` work + journal + task.toml → `git commit` → on success, write `committed_head` to task.toml in a *follow-up local edit* that is intentionally not committed (the SHA is metadata, not load-bearing); on failure, rollback (restore prior task.toml, truncate journal entry, re-render index).
- **G-5:** Journal entry no longer carries `commit_range`. Replaced by `**Start Head**` + `**Base Branch**` fields. Commits-in-range table is populated by `git log <start_head>..HEAD --oneline -n 20` *before* the task-closing commit.
- **C-17:** `ark context --for commit` projection is **body-free**. Returns the path of `VERIFY.md` and the path of the latest `NN_PLAN.md`, not their contents.
- **G-6:** `ark archive` reads each candidate task's `task.toml.committed_at` and passes it through per slug. The bucket is derived from `committed_at`, not from `Utc::now()`.
- **T-1, T-6:** Collapsed into a single trade-off `T-1`. The two were two framings of the same R-001-induced problem; now there is one chosen invariant.
- **V-E-6:** Concurrent `/ark:commit` test removed; the new NG-12 declares it out of scope.

[**Removed**]

- **`HEAD_PENDING_TOKEN` constant and `patch_head_pending` function** (were planned in `00_PLAN`'s journal.rs Data Structure). No post-commit patching exists in the new protocol.
- **`Error::JournalEntryStale`, `Error::JournalPatchFailed`** error variants. The failure modes they covered no longer exist.
- **Failure Flow steps 6 + 7 of `00_PLAN`** (commit-fail with stale token; patch-fail with literal token in journal). Replaced by a clean rollback path on commit failure.
- **`task_archive` shim.** With the side-effect split, no caller needs the legacy bundle.

[**Unresolved**]

None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Drop self-referential exact-SHA from the journal entry. Journal records `start_head` + `base_branch` only; the commits-in-range table is computed pre-commit via `git log <start_head>..HEAD`. Post-commit, the SHA is recorded as `task.toml.committed_head` (a `task.toml` field, not a journal field). No patch step. See G-3, G-5, C-4 in this iteration. |
| Review | R-002 | Accepted | `task_commit` saves the updated `task.toml` (phase=Committed, committed_at=now) **before** invoking `git commit` and stages it into the same commit as work + journal. On commit failure, restores the prior toml from in-memory snapshot and truncates the appended journal entry. See G-3, Runtime Main Flow, Failure Flow in this iteration. |
| Review | R-003 | Accepted | Split `task_archive` into `task_archive_move` (move directory + state-cleanup, no SPEC, no journal) and delete the legacy bundling wrapper. SPEC promotion lives only in `task_commit` (deep tier). Journal recording for tasks lives only in `task_commit`. `ark archive` calls only `task_archive_move`. New tests assert `ark archive` writes no journal and re-promotes no SPEC. See G-6, C-18, Architecture in this iteration. |
| Review | R-004 | Accepted | `ArchiveOptions` (the new top-level helper) derives YYYY-MM from each task's `task.toml.committed_at`. `task_archive_move` accepts an explicit `archive_month: String` parameter (YYYY-MM) so the bucket is deterministic. New test `archive_uses_committed_at_month_not_now` covers the case of a task committed in a previous month. See G-6 + V-IT-N in this iteration. |
| Review | R-005 | Accepted (TR-3 Option A) | `ark context --for commit` projection includes only paths, no file bodies. Matches `ark-context` SPEC G-4/G-5/G-7. Slash commands read VERIFY.md and the latest plan from the returned paths. See C-17 in this iteration. |
| Review | R-006 | Accepted | The patch-HEAD step is gone (R-001 resolution), so JournalPatchFailed cannot occur. The error variant and recovery prose are deleted from the plan. |
| Review | R-007 | Accepted | Concurrent `/ark:commit` declared unsupported (NG-12). V-E-6 deleted. The workspace SPEC's existing best-effort journal append remains as-is; no new lock is added. |
| Trade-off Advice | TR-1 | Applied | Chose Option A from R-001's recommendations. The journal records the *start* of the range and the base branch; the *end* is the commit that contains the entry, recoverable via `git log -- <journal-path>` (the journal file is touched only by task commits) or via `task.toml.committed_head`. Validation updated to assert clean working tree post-commit save (single dirty file: `task.toml`'s `committed_head` follow-up edit). |
| Trade-off Advice | TR-2 | Applied (Option B) | `task_archive` is split. Side-effect-free move is the only entrypoint used by `ark archive`. SPEC promotion and journal recording belong to `task_commit` only. |
| Trade-off Advice | TR-3 | Applied (Option A) | Kept `ark context` projections body-free. Commit projection emits paths only. |

> Rules applied:
> - Every prior CRITICAL/HIGH finding (R-001..R-005) appears with explicit Accept reasoning.
> - All MEDIUM/LOW findings (R-006, R-007) addressed.
> - All Trade-off Advice items resolved.

---

## Spec `Core specification`

[**Goals**]

- **G-1: Phase enum gains `Committed`; legal-transition table updated.** `Phase` (in `crates/ark-core/src/commands/agent/state.rs`) gains a single new variant `Committed` (lowercase serde rename `committed`). The legal-transition table in `can_transition` is updated:
  - **Add:** `(Quick, Execute, Committed)`, `(Standard, Verify, Committed)`, `(Deep, Verify, Committed)`, `(Quick, Committed, Archived)`, `(Standard, Committed, Archived)`, `(Deep, Committed, Archived)`.
  - **Remove:** `(Quick, Execute, Archived)`, `(Standard, Verify, Archived)`, `(Deep, Verify, Archived)`. After the refactor, `Archived` is reachable only from `Committed`.
  - **Status derivation** (`TaskToml::status`): `Phase::Committed` → `Status::InProgress`; `Phase::Archived` → `Status::Completed`.
  - The state-file reconcile loop keeps a slug in `state.tasks.active` when its `task.toml.phase` is anything except `Archived`. `Committed` is included.

- **G-2: `task.toml` gains `start_head`, `committed_at`, `committed_head`.** Three new optional fields:
  - `start_head: Option<String>` — captured at `task new` time as the parent checkout's HEAD SHA via `git rev-parse HEAD` (resolved from `opts.project_root`). On `task new --worktree`, the parent's HEAD SHA is captured before `git worktree add`. `None` on unborn HEAD or pre-refactor tasks.
  - `committed_at: Option<DateTime<Utc>>` — written by `task_commit` when it saves the updated toml (before invoking `git commit`). Drives the YYYY-MM bucket for `ark archive`.
  - `committed_head: Option<String>` — written by `task_commit` **after** the commit lands, in a follow-up local edit that is intentionally not committed. Records the exact commit that closed the task. Pre-refactor tasks have `None`; tasks committed via `--no-commit` have `None`.
  - All three carry `#[serde(skip_serializing_if = "Option::is_none", default)]`.

- **G-3: `task_commit` is the atomic closure.** New public function `task_commit(opts: TaskCommitOptions) -> Result<TaskCommitSummary>` in `crates/ark-core/src/commands/agent/task/commit.rs`. Performs **a single git commit covering work + journal entry + the updated task.toml**, with documented rollback on failure. Step ordering:

  1. **Phase precondition.** Load `TaskToml` (snapshot as `prev_toml`). Reject unless `(tier, phase)` is one of `(Quick, Execute)`, `(Standard, Verify)`, `(Deep, Verify)`. Wrong phase → `Error::IllegalPhaseTransition`.
  2. **Working-tree precondition.** Run `git status --porcelain` from the task's cwd. Empty AND `opts.no_commit == false` → `Error::NothingToCommit { slug }`. Skipped under `--no-commit`.
  3. **VERIFY gate (tier-conditional).** Standard/Deep only: parse `VERIFY.md`. Deep refuses on any pending; standard warns; quick has no VERIFY.
  4. **Deep-tier SPEC extraction.** Deep only: invoke `spec_extract` + `spec_register` on the *active* task dir (not an archive path). SPEC lands at `specs/features/<slug>/SPEC.md`; INDEX row upserted. Idempotent.
  5. **Render + append journal entry.** When `!no_commit`:
     a. Compute `commits_in_range = git log <start_head>..HEAD --oneline -n 20` from task's cwd. When `start_head` is `None`, fall back to `git log -n 20`.
     b. Render entry with `**Start Head**` + `**Base Branch**` fields. **No exact final SHA in the entry.**
     c. Resolve target journal file (with rotation per workspace SPEC). Snapshot `pre_append_len` and `prev_index_bytes` for rollback.
     d. Append rendered entry. Re-render `<dev>/index.md`'s managed blocks.
  6. **Save updated `task.toml`.** Build `next_toml` with `phase = Committed`, `committed_at = Some(now)`, `updated_at = now`. Write to disk (this file will be staged into the closing commit).
  7. **Stage + commit.** When `!no_commit`:
     a. `commit_msg ← opts.message.clone().ok_or(Error::CommitMessageRequired)?`.
     b. `git add -A` from task's cwd. Documented hidden side effect: any other dirty files in the worktree are also captured. Documented in slash-command body.
     c. `git commit -m <msg>`. On failure, run rollback path (restore prev_toml, truncate journal, restore index, `git reset`). Return `Error::GitCommitFailed { stderr }`.
  8. **Record `committed_head` (post-commit metadata).** When commit succeeded:
     a. `head_sha ← git rev-parse HEAD`.
     b. Re-load task.toml; set `committed_head = Some(head_sha)`; save.
     c. **This save leaves the working tree dirty (one file changed: `task.toml`'s last field).** This is the single intentional dirty-file the protocol leaves behind, documented in C-4 and surfaced in the slash command's wrap-up.
  9. **`--no-commit` path.** Steps 2 (working-tree check), 5 (journal write), 7 (commit), 8 (committed_head) are all skipped. Steps 3, 4, 6 still run. Stderr emits `--no-commit: journal not written; run /ark:record manually if you want a session entry`.

- **G-4: `VERIFY.md` is a living checklist + findings document.** The template at `.ark/templates/VERIFY.md` is rewritten to six sections in this fixed order: `## Project Spec Compliance`, `## Related Feature Spec Compliance`, `## PRD Constraints`, `## Plan Fidelity`, `## SPEC Drift`, `## Findings`, `## Notes`. The first four sections are dynamically seeded at `ark agent task verify` time from `.ark/specs/project/INDEX.md`, the PRD's `[**Related Specs**]`, the PRD's `[**Outcome**]`, and the latest `NN_PLAN.md`'s `## Spec` Goals. Each seeded bullet renders as `- [ ] <item>: PENDING`. Section 5 is fixed-content (one bullet: `- [ ] Modified feature SPECs have CHANGELOG entries: PENDING`). Section 6 starts empty; the implementer adds `### V-NNN — <title>` blocks (Severity / Location / Problem / Why it matters / Recommendation / Resolution). Section 7 is free-form. **No verdict line.** Document is "complete" iff every checklist item is in `{PASS, FAIL: <reason>, N/A: <reason>}` and every finding's Resolution is in `{FIXED in <commit-or-section>, ACCEPTED — <reason>}`.

- **G-5: Journal entry records `start_head` + `base_branch`, not exact end SHA.** The per-task journal entry rendered by `task_commit` emits the new fields:
  - `**Start Head**: \`<start_head>\`` between the `**Branch**` field and `### Summary`. When `start_head` is `None`, renders `**Start Head**: \`(unknown — pre-refactor task)\``.
  - `**Base Branch**: \`<base_branch>\`` immediately after Start Head. When `None`, renders `**Base Branch**: \`(unknown)\``.
  - Commits table populated by `git log <start_head>..HEAD --oneline -n 20`. When `start_head` is None, falls back to `base_branch..HEAD` if base_branch is Some, else `-n 20`.
  - **No `<HEAD-PENDING>` token. No `commit_range` field. No post-commit patching of journal entries.** Readers wanting the exact closing SHA look at `task.toml.committed_head` or run `git log -- <journal-path>`.
  - Manual `/ark:record` path is unchanged: omits `**Start Head**` and `**Base Branch**` entirely.

- **G-6: `ark archive` is a top-level manager-only CLI; archive helper is side-effect-free.** New top-level subcommand `ark archive [--dry-run] [--month YYYY-MM]` in `crates/ark-cli/src/main.rs` (peer of `ark init`, `ark unload`, `ark upgrade`, `ark context`). Implementation in `crates/ark-core/src/commands/archive.rs`. Behavior:
  - Enumerate `.ark/tasks/<slug>/task.toml` excluding `.ark/tasks/archive/`. For each task with `phase = Committed`, derive YYYY-MM from `committed_at` (must be `Some` — tasks with `phase = Committed && committed_at = None` are skipped with a stderr warning + `Error::CommittedAtMissing` surfaced in failures list).
  - For each candidate, call `task_archive_move(TaskArchiveMoveOptions { project_root, slug, archive_month: <YYYY-MM from committed_at> })`. **No SPEC promotion. No journal recording.**
  - `--month YYYY-MM` filters to only archive tasks whose `committed_at` falls in the named month.
  - `--dry-run` lists what would move without performing the move.
  - Idempotent. Per-slug failures are collected, not fatal.

- **G-7: Slash-command and skill template surface.** Across all three platforms:
  - **Add** `commit.md` (Claude + OpenCode) and `ark-commit/SKILL.md` (Codex). Body parses `$ARGUMENTS` for `-m "<msg>"` and `--no-commit`; pulls `ark context --scope phase --for commit`; if no `-m` and no `--no-commit`, generates a conventional-commits message and shows for confirmation; invokes `ark agent task commit --message "<m>" [--no-commit]`. Wrap-up reports commit SHA, journal session number, deep-tier promoted SPEC path, **and notes the single dirty file (`task.toml` follow-up edit)**.
  - **Remove** `archive.md` (Claude + OpenCode) and `ark-archive/SKILL.md` (Codex).
  - **Update** `design.md`, `quick.md`, `record.md` to point at `/ark:commit` instead of `/ark:archive`.
  - **Lockstep rule:** any change to one platform lands as a parallel edit on the other two.

- **G-8: `workflow.md` and `AGENTS.md` updated.** Lockstep updates per the architecture diagram below.

- **G-9: `ark agent task commit` CLI subcommand.** In `crates/ark-cli/src/agent_cli.rs`'s `TaskCommand` enum, add `Commit(TaskCommitCliArgs)` with flags `--message <msg>` (`-m` short) and `--no-commit`. Dispatch wires through to `ark_core::task_commit`. The existing `Archive(TaskSlugArgs)` variant **is removed** — bulk archive uses the top-level `ark archive` CLI which calls `task_archive_move` directly.

- **G-10: Migration on `ark upgrade`.** One migration step:
  - **Slash-command refresh:** existing template-overwrite pass already replaces files in lockstep. Refactor changes the embedded set; upgrade machinery requires no logic change. Removed `archive.md` files become orphans; upgrade prints `removed obsolete slash command: <path>` to stderr and unlinks each.
  - **In-flight `VERIFY.md` regeneration:** for tasks with `phase ∈ {Verify, Committed}` and a `VERIFY.md` containing `## Verdict` (legacy heuristic), rewrite using the new template seeded from live PRD + project specs + plan. Preserve any V-NNN findings verbatim. Drop `## Verdict` line (logged to stderr).
  - **No `start_head` backfill.** Pre-refactor tasks keep `None`; `task_commit` falls back to `git log -n 20`.

- **NG-1:** User-defined workflow chains out of scope.
- **NG-2:** Bulk archive triggered by version bump out of scope.
- **NG-3:** REVIEW phase shape unchanged.
- **NG-4:** REVIEW iteration loop unchanged.
- **NG-5:** Ark does not generate commit messages itself; the slash command is responsible.
- **NG-6:** Once a task is `Committed`, the legacy `## Verdict` is dropped (no archive preservation).
- **NG-7:** No new reopen API for committed tasks.
- **NG-8:** No new `pre-commit`/`post-commit` ark-side hook integration.
- **NG-9:** `--no-commit` on non-deep tiers only transitions the phase; emits stderr note.
- **NG-10:** Revise commits made post-`/ark:commit` are not retroactively folded into the journal entry.
- **NG-11:** No new locking for `ark archive` concurrency; idempotent re-runs converge.
- **NG-12:** Concurrent `/ark:commit` invocations on the same checkout are unsupported. The user must serialize. The workspace journal's session-number assignment + append + index re-render is best-effort; running two `/ark:commit`s simultaneously can produce duplicate session numbers and corrupt the index. Documented in workflow doc and `--help`. (Per R-007's recommendation; per-checkout locking deferred to a follow-up.)

[**Architecture**]

```text
crates/
├── ark-cli/src/
│   ├── main.rs                                  ─ ADD top-level Archive(ArchiveCliArgs).
│   └── agent_cli.rs                             ─ ADD Commit(TaskCommitCliArgs); REMOVE
│                                                  Archive(TaskSlugArgs) (no longer user-facing;
│                                                  bulk archive uses top-level ark archive).
└── ark-core/src/
    ├── commands/
    │   ├── archive.rs                           ─ NEW: pub fn ark_archive(opts) -> Result<…>
    │   │                                          enumerates phase=Committed tasks; calls
    │   │                                          task_archive_move per slug; archive_month
    │   │                                          derived from each task's committed_at.
    │   ├── upgrade.rs                           ─ MOD: VERIFY.md migration step (G-10);
    │   │                                          orphan-slash-command unlink + stderr note.
    │   ├── agent/
    │   │   ├── state.rs                         ─ MOD: Phase::Committed; can_transition
    │   │   │                                       updated per G-1; status() handles Committed.
    │   │   ├── task/
    │   │   │   ├── commit.rs                    ─ NEW: pub fn task_commit + types;
    │   │   │   │                                  rollback-on-commit-fail protocol (G-3);
    │   │   │   │                                  parse_verify_md helper (C-7).
    │   │   │   ├── new.rs                       ─ MOD: build_task_toml captures start_head via
    │   │   │   │                                  git rev-parse HEAD on opts.project_root.
    │   │   │   ├── archive.rs                   ─ MOD: rename task_archive →
    │   │   │   │                                  task_archive_move; STRIP spec_extract +
    │   │   │   │                                  spec_register + record_task body.
    │   │   │   │                                  New signature: archive_month: String.
    │   │   │   │                                  DELETE legacy task_archive function.
    │   │   │   ├── phase.rs                     ─ MOD: artifact_for(Committed, _) → None;
    │   │   │   │                                  task_verify gains seed-time substitution
    │   │   │   │                                  (delegates to verify_seed module).
    │   │   │   ├── verify_seed.rs               ─ NEW: render_seeded_verify(SeedInputs) with
    │   │   │   │                                  marker substitution per C-9.
    │   │   │   └── mod.rs                       ─ MOD: pub mod commit; pub mod verify_seed;
    │   │   │                                       pub use commit::*; rename re-export
    │   │   │                                       task_archive → task_archive_move.
    │   │   ├── workspace/
    │   │   │   ├── journal.rs                   ─ MOD: JournalEntry.start_head: Option<String>
    │   │   │   │                                  + JournalEntry.base_branch: Option<String>;
    │   │   │   │                                  render_entry emits **Start Head** + **Base
    │   │   │   │                                  Branch** lines for task entries when Some.
    │   │   │   │                                  No commit_range field.
    │   │   │   └── record.rs                    ─ MOD: collect_commits_for_task accepts
    │   │   │                                      start_head: Option<&str>; falls back to
    │   │   │                                      base_branch range or -n 20 when None.
    │   │   │                                      RecordTaskOptions gains start_head field;
    │   │   │                                      archive_path renamed to task_dir;
    │   │   │                                      archived_at renamed to recorded_at.
    │   │   └── (no other agent modules touched)
    │   └── context/projection.rs                ─ MOD: PhaseFilter::Commit variant;
    │                                              for_phase(Commit) returns latest VERIFY path
    │                                              + latest PLAN path + project specs +
    │                                              git state. **No file bodies.** Per C-17.
    └── lib.rs                                   ─ ADD pub re-exports: ark_archive,
                                                   ArchiveOptions, ArchiveSummary, task_commit,
                                                   TaskCommitOptions, TaskCommitSummary,
                                                   task_archive_move, TaskArchiveMoveOptions,
                                                   TaskArchiveMoveSummary.
                                                   REMOVE re-export: task_archive (deleted).

templates/
├── ark/
│   ├── templates/
│   │   ├── VERIFY.md                            ─ REWRITE: six-section living document
│   │   │                                          (per G-4); no Verdict.
│   │   └── (PRD/PLAN/REVIEW/SPEC unchanged)
│   ├── workflow.md                              ─ MOD: per G-8.
│   └── (config.toml unchanged)
├── claude/commands/ark/
│   ├── commit.md                                ─ NEW
│   ├── archive.md                               ─ DELETE
│   └── design.md / quick.md / record.md         ─ MOD
├── codex/skills/
│   ├── ark-commit/SKILL.md                      ─ NEW
│   ├── ark-archive/SKILL.md                     ─ DELETE
│   └── ark-design / ark-quick / ark-record      ─ MOD
└── opencode/commands/ark/
    ├── commit.md                                ─ NEW
    ├── archive.md                               ─ DELETE
    └── design.md / quick.md / record.md         ─ MOD

AGENTS.md                                        ─ MOD: drop /ark:archive row, add /ark:commit row.
```

**Module coupling.**

- `commands/archive.rs` (new top-level) imports `commands::agent::task::archive::task_archive_move` and `commands::agent::state::{TaskToml, Phase}`. One-way: top-level `archive` → `agent::task::archive`.
- `commands/agent/task/commit.rs` imports `commands::agent::state::{Phase, TaskToml, check_transition}`, `commands::agent::spec::{extract::spec_extract, register::spec_register}`, `commands::agent::workspace::record::record_task` (and supporting types), `io::{git::run_git, PathExt}`, `layout::Layout`. Does **not** import `task::archive`.
- `commands/agent/task/archive.rs` after refactor does **not** import `super::workspace::*` or `super::spec::*` — those imports moved to `task::commit`.
- `commands/agent/workspace/record.rs`: only caller after refactor is `task::commit::task_commit`. The legacy `task::archive::task_archive` call is removed.
- `commands/upgrade.rs` imports `templates::ARK_TEMPLATES` and `state::{TaskToml, Phase}` for VERIFY migration.

**Call graph: `/ark:commit` → `task_commit`.**

```text
slash command /ark:commit
  ├── (agent generates message if -m absent; shows for confirm)
  └── ark agent task commit --message "<m>" [--no-commit] [--slug <s>]
        └── task_commit(opts)
              ├── slug ← resolve_slug(opts.slug, ppid)
              ├── layout ← Layout::new(project_root)
              ├── task_dir ← layout.task_dir(&slug)
              ├── prev_toml ← TaskToml::load(&task_dir)?              [snapshot for rollback]
              ├── check_phase_for_commit(prev_toml.tier, prev_toml.phase)?
              │     ├── Quick + Execute → ok
              │     ├── Standard/Deep + Verify → ok
              │     └── _ → Error::IllegalPhaseTransition
              │
              ├── task_cwd ← worktree_path.unwrap_or(layout.root())
              │
              ├── if !opts.no_commit:
              │     status ← run_git(&["status","--porcelain"], task_cwd)
              │     if status.stdout.is_empty(): return Error::NothingToCommit{ slug }
              │
              ├── if tier in {Standard, Deep}:
              │     verify ← parse_verify_md(&task_dir.join("VERIFY.md"))?
              │     pendings ← collect_pending(&verify)
              │     match (tier, pendings.len()) {
              │         (Deep, n) if n > 0 → Error::VerifyIncomplete { items, findings },
              │         (Standard, n) if n > 0 → eprintln!(warn for each pending),
              │         _ → ok,
              │     }
              │
              ├── if tier == Deep:
              │     spec_extract(SpecExtractOptions {
              │         project_root, slug, plan_override: None,
              │         task_dir_override: Some(task_dir.clone()),
              │     })?
              │     spec_register(SpecRegisterOptions {
              │         project_root, feature: slug.clone(),
              │         scope: prev_toml.title.clone(),
              │         from_task: slug.clone(),
              │         date: now.date_naive(),
              │     })?
              │
              ├── (journal_path, pre_append_len, prev_index_bytes) ←
              │     if !opts.no_commit {
              │         render_and_append_journal(layout, prev_toml, commits_in_range)?
              │     } else { (None, 0, vec![]) }
              │
              ├── next_toml ← prev_toml.with(phase = Committed,
              │                              committed_at = Some(now),
              │                              updated_at = now)
              ├── next_toml.save(&task_dir)?
              │
              ├── if !opts.no_commit:
              │     run_git(&["add", "-A"], task_cwd)?
              │     out ← run_git(&["commit","-m",&opts.message.unwrap()], task_cwd)?
              │     if !out.is_success():
              │         // ROLLBACK
              │         prev_toml.save(&task_dir)?
              │         truncate_file(journal_path.unwrap(), pre_append_len)?
              │         restore_index(layout, dev, prev_index_bytes)?
              │         run_git(&["reset"], task_cwd)?    // unstage
              │         return Error::GitCommitFailed{ stderr: out.stderr }
              │     head_sha ← run_git(&["rev-parse","HEAD"], task_cwd)?.stdout.trim()
              │     // committed_head follow-up edit (intentionally dirty)
              │     post_toml ← TaskToml::load(&task_dir)?
              │     post_toml.committed_head = Some(head_sha.clone())
              │     post_toml.save(&task_dir)?
              │
              └── return TaskCommitSummary {
                    slug, tier,
                    head_sha: if no_commit { None } else { Some(head_sha) },
                    journal_path, session_number, deep_spec_promoted, pending_verify,
                    committed_head_dirty: !no_commit,
                  }
```

**Call graph: `ark archive` → `task_archive_move`.**

```text
ark archive [--month YYYY-MM] [--dry-run]
  └── ark_archive(opts)
        ├── layout ← Layout::new(project_root)
        ├── candidates ← enumerate_committed(&layout)?
        │     // walks .ark/tasks/<slug>/task.toml, filters phase=Committed,
        │     // returns Vec<(slug, committed_at)>; tasks with committed_at=None
        │     // are skipped + reported as Error::CommittedAtMissing in failures.
        │
        ├── filtered ← match opts.month {
        │       Some(m) → candidates.filter(|(_, ca)| ca.format("%Y-%m") == m),
        │       None → candidates,
        │   }
        │
        ├── if opts.dry_run:
        │     for (slug, ca) in filtered:
        │         println!("{slug} -> .ark/tasks/archive/{}/{slug}", ca.format("%Y-%m"))
        │     return Ok(ArchiveSummary { dry_run: true, … })
        │
        ├── successes ← Vec::new(); failures ← Vec::new()
        ├── for (slug, ca) in filtered:
        │     let archive_month = ca.format("%Y-%m").to_string()
        │     match task_archive_move(TaskArchiveMoveOptions {
        │         project_root: opts.project_root.clone(),
        │         slug: slug.clone(),
        │         archive_month,
        │     }) {
        │         Ok(s) → successes.push(s),
        │         Err(e) → failures.push((slug, e)),
        │     }
        │
        └── return Ok(ArchiveSummary { successes, failures, dry_run: false })
```

**Call graph: `task_archive_move` (side-effect-free).**

```text
task_archive_move(opts)
  ├── layout ← Layout::new(&opts.project_root)
  ├── task_dir ← layout.task_dir(&opts.slug)
  ├── if !task_dir.exists(): return Error::TaskNotFound{ slug }
  ├── toml ← TaskToml::load(&task_dir)?
  ├── check_transition(toml.tier, toml.phase, Phase::Archived)?   // legal only from Committed
  ├── archive_parent ← layout.tasks_archive_dir().join(&opts.archive_month)
  ├── archive_parent.ensure_dir()?
  ├── archive_path ← archive_parent.join(&opts.slug)
  ├── if archive_path.exists(): return Error::TaskAlreadyExists{ slug: format!("archive/{}/{}", archive_month, slug) }
  ├── clear_focus_for_slug(&layout, ppid, &opts.slug)?
  ├── task_dir.rename_to(&archive_path)?
  ├── toml.phase = Phase::Archived
  ├── toml.archived_at = Some(now)
  ├── toml.updated_at = now
  ├── toml.save(&archive_path)?
  └── return TaskArchiveMoveSummary { slug, tier, archive_path }
       // NO spec_extract. NO spec_register. NO record_task.
```

[**Data Structure**]

```rust
// crates/ark-core/src/commands/agent/state.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
    Committed,    // NEW
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToml {
    pub id: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub iteration: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_iterations: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archived_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub committed_at: Option<DateTime<Utc>>,    // NEW
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub worktree_path: Option<std::path::PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_head: Option<String>,             // NEW
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub committed_head: Option<String>,         // NEW (post-commit metadata)
}
```

```rust
// crates/ark-core/src/commands/agent/task/commit.rs (NEW)

#[derive(Debug, Clone)]
pub struct TaskCommitOptions {
    pub project_root: PathBuf,
    pub slug: String,
    /// Commit message. Required when `no_commit == false`.
    pub message: Option<String>,
    /// When true, skip git commit + journal write; deep tier still extracts SPEC.
    pub no_commit: bool,
}

#[derive(Debug, Clone)]
pub struct TaskCommitSummary {
    pub slug: String,
    pub tier: Tier,
    pub head_sha: Option<String>,
    pub journal_path: Option<PathBuf>,
    pub session_number: Option<u32>,
    pub deep_spec_promoted: bool,
    pub pending_verify: VerifyPendingCounts,
    /// True iff `task.toml.committed_head` was written post-commit
    /// (the single intentional dirty file).
    pub committed_head_dirty: bool,
}

#[derive(Debug, Clone, Default)]
pub struct VerifyPendingCounts {
    pub items: u32,
    pub findings: u32,
}

impl fmt::Display for TaskCommitSummary { /* one-line */ }

pub fn task_commit(opts: TaskCommitOptions) -> Result<TaskCommitSummary>;
```

```rust
// crates/ark-core/src/commands/agent/task/archive.rs (renamed function)

#[derive(Debug, Clone)]
pub struct TaskArchiveMoveOptions {
    pub project_root: PathBuf,
    pub slug: String,
    /// YYYY-MM bucket. Caller (`ark_archive`) derives from `task.toml.committed_at`.
    pub archive_month: String,
}

#[derive(Debug, Clone)]
pub struct TaskArchiveMoveSummary {
    pub slug: String,
    pub tier: Tier,
    pub archive_path: PathBuf,
}

impl fmt::Display for TaskArchiveMoveSummary { /* one-line */ }

pub fn task_archive_move(opts: TaskArchiveMoveOptions) -> Result<TaskArchiveMoveSummary>;
// Legacy task_archive function: DELETED.
```

```rust
// crates/ark-core/src/commands/agent/workspace/record.rs

#[derive(Debug, Clone)]
pub struct RecordTaskOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub branch: Option<String>,
    pub base_branch: Option<String>,
    pub worktree_path: Option<PathBuf>,
    /// NEW: exact start_head from task.toml.
    pub start_head: Option<String>,
    /// Renamed from archive_path.
    pub task_dir: PathBuf,
    /// Renamed from archived_at.
    pub recorded_at: DateTime<Utc>,
}
```

```rust
// crates/ark-core/src/commands/agent/workspace/journal.rs

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub session_number: u32,
    pub title: String,
    pub date: NaiveDate,
    pub kind: JournalKind,
    pub branch: Option<String>,
    /// NEW: rendered as `**Start Head**: \`<value>\`` for task entries.
    pub start_head: Option<String>,
    /// NEW: rendered as `**Base Branch**: \`<value>\`` for task entries.
    pub base_branch: Option<String>,
    pub summary: String,
    pub commits: Vec<JournalCommit>,
    pub next_steps: Vec<String>,
}

// HEAD_PENDING_TOKEN: not introduced.
// patch_head_pending: not introduced.
```

```rust
// crates/ark-core/src/commands/archive.rs (NEW)

#[derive(Debug, Clone)]
pub struct ArchiveOptions {
    pub project_root: PathBuf,
    pub month: Option<String>,    // YYYY-MM filter
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ArchiveSummary {
    pub successes: Vec<TaskArchiveMoveSummary>,
    pub failures: Vec<(String, Error)>,
    pub dry_run: bool,
}

impl fmt::Display for ArchiveSummary { /* multi-line */ }

pub fn ark_archive(opts: ArchiveOptions) -> Result<ArchiveSummary>;
```

```rust
// crates/ark-core/src/error.rs (additions)

#[error("task `{slug}` cannot be committed with an empty working tree")]
NothingToCommit { slug: String },

#[error("VERIFY.md has {items} pending item(s) and {findings} pending finding(s); resolve before commit")]
VerifyIncomplete { items: u32, findings: u32 },

#[error("git commit failed: {stderr}")]
GitCommitFailed { stderr: String },

#[error("commit message is required (pass `-m` or generate one before invoking `task commit`)")]
CommitMessageRequired,

#[error("task `{slug}` has phase Committed but committed_at is missing")]
CommittedAtMissing { slug: String },

#[error("template marker `{marker}` is missing in {path}")]
TemplateMarkerMissing { marker: String, path: PathBuf },

// REMOVED from 00_PLAN: JournalEntryStale, JournalPatchFailed.
```

[**API Surface**]

```rust
// crates/ark-core/src/lib.rs:
pub use commands::archive::{ArchiveOptions, ArchiveSummary, ark_archive};
pub use commands::agent::task::commit::{
    TaskCommitOptions, TaskCommitSummary, VerifyPendingCounts, task_commit,
};
pub use commands::agent::task::archive::{
    TaskArchiveMoveOptions, TaskArchiveMoveSummary, task_archive_move,
};
// Removed: task_archive (legacy bundling function deleted).
```

```rust
// crates/ark-cli/src/main.rs:
enum Command {
    // ...existing...
    Archive(ArchiveCliArgs),    // NEW
}

#[derive(clap::Args)]
struct ArchiveCliArgs {
    #[arg(long)] month: Option<String>,
    #[arg(long, default_value_t = false)] dry_run: bool,
}
```

```rust
// crates/ark-cli/src/agent_cli.rs:
enum TaskCommand {
    // ...Plan, Review, Execute, Verify, New, Promote, Resume, Discard...
    Commit(TaskCommitCliArgs),    // NEW
    // Archive(TaskSlugArgs) — REMOVED.
}

#[derive(clap::Args)]
struct TaskCommitCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(short = 'm', long = "message")] message: Option<String>,
    #[arg(long = "no-commit", default_value_t = false)] no_commit: bool,
}
```

[**Constraints**]

- **C-1 (one-way coupling):** `commands/archive.rs` depends on `commands::agent::task::archive::task_archive_move`; reverse forbidden. `commands/agent/task/commit.rs` depends on `commands::agent::workspace::record::record_task` and `commands::agent::spec::{extract,register}`; reverse forbidden. `commands/agent/workspace/*` MUST NOT import `super::task`. `commands/agent/task/archive.rs` after refactor does **not** import `super::workspace::*` or `super::spec::*`.

- **C-2 (process spawn locality):** All git invocations under the new modules route through `io::git::run_git`. `Command::new` MUST NOT appear in these new sites.

- **C-3 (commit message authorship):** `task_commit` does not generate commit messages. When `opts.no_commit == false`, `opts.message` MUST be `Some(_)`; `None` → `Error::CommitMessageRequired`.

- **C-4 (atomic-commit protocol with rollback):** `task_commit` sequence: render-journal → save-next-toml → `git add -A` → `git commit`, capturing work + journal entry + task.toml in one git commit. **No post-commit patch step on the journal file. No `<HEAD-PENDING>` token.** On `git commit` failure, rollback restores `prev_toml`, truncates the journal file to `pre_append_len`, restores prior index bytes, runs `git reset` to unstage. **One intentional dirty file remains after a successful commit:** `task.toml`'s `committed_head` follow-up edit (post-commit metadata, not part of the closing commit). The slash command surfaces this in its wrap-up.

- **C-5 (`start_head` capture):** `task_new` captures `start_head` via `run_git(&["rev-parse","HEAD"], &opts.project_root)`. On unborn HEAD, `start_head = None`. On `--worktree`, the same call is made on `opts.project_root` **before** `git worktree add`; the resolved SHA is the parent HEAD, equal to the worktree's initial HEAD.

- **C-6 (state-file reconcile semantics):** Reconcile keeps `phase == Committed` slugs in `tasks.active`. No code change required.

- **C-7 (VERIFY parser):** `parse_verify_md` reads `VERIFY.md` and counts `PENDING` occurrences. A line is pending iff it matches `/^- \[ \] .+: PENDING$/` (checklist) or appears under a `### V-NNN` heading with a `Resolution: PENDING` line. Permissive on whitespace; case-sensitive on `PENDING`. Notes section never gated. Lives in `commit.rs` as a private helper.

- **C-8 (slash-command lockstep):** Any change to one of `commit.md` (Claude/OpenCode) and `ark-commit/SKILL.md` (Codex) lands as a parallel edit on the others.

- **C-9 (template ordering on disk):** `templates/ark/templates/VERIFY.md` is the source of truth. `task_verify` parses it, then overlays dynamic content via four named markers `{{PROJECT_SPEC_COMPLIANCE}}`, `{{RELATED_FEATURE_COMPLIANCE}}`, `{{PRD_CONSTRAINTS}}`, `{{PLAN_FIDELITY}}`. Missing marker → `Error::TemplateMarkerMissing { marker, path }`.

- **C-10 (path/io discipline):** All FS access in new modules through `io::PathExt`. All `.ark/`-relative paths through `Layout` helpers.

- **C-11 (CLI hidden-vs-visible split):** `ark archive` is **top-level visible** in `ark --help`. `ark agent task commit` is **hidden** under `ark agent` per `ark-agent-namespace` SPEC.

- **C-12 (migration idempotency):** VERIFY-migration step is idempotent (heuristic: `## Verdict` substring near top). Orphan-slash-command unlink is idempotent.

- **C-13 (auto-record absence on `--no-commit`):** `--no-commit` fully skips journal write. CLI emits `--no-commit: journal not written; run /ark:record manually if you want a session entry`.

- **C-14 (deep-tier `--no-commit` SPEC extraction):** `--no-commit && tier == Deep` still runs SPEC extract. SPEC promotion is the deep-tier-defining mutation.

- **C-15 (CHANGELOG entry on existing SPEC):** Existing SPEC at extract time gets a `[**CHANGELOG**]` row appended via the existing `spec_extract` protocol (per `project-spec` SPEC).

- **C-16 (manual `/ark:record` path unchanged):** `workspace_record` keeps `start_head = None`, `base_branch = None`. Renders no `**Start Head**` / `**Base Branch**`.

- **C-17 (commit projection is body-free):** `PhaseFilter::Commit` returns: current task + path of latest VERIFY.md + path of latest NN_PLAN.md + project specs + git state. **No file bodies.** Matches `ark-context` SPEC G-4/G-5/G-7's additive-only schema.

- **C-18 (archive helper is side-effect-free):** `task_archive_move` performs only directory rename + `task.toml` phase update + state-file cleanup. **No SPEC extract. No SPEC register. No record_task.** Tests `archive_no_spec_promotion`, `archive_no_journal_write` assert this.

- **C-19 (legal-transition table parity):** `archived_is_terminal` test asserts `Archived` remains terminal AND `Committed → Archived` is the only entry. New tests `*_committed_is_legal_destination` and `committed_archived_is_legal_destination` per tier.

- **C-20 (slash-command parity test):** `commit.md` (Claude+OpenCode) and `ark-commit/SKILL.md` (Codex) share the same hash on a normalized form (front-matter stripped, whitespace collapsed).

- **C-21 (`committed_at` required for `Committed` phase):** `phase = Committed && committed_at = None` is inconsistent. `ark_archive` skips with stderr warning + `Error::CommittedAtMissing { slug }` in failures (does not abort the whole run). `task_commit` always writes both fields together; this can only arise from external corruption.

- **C-22 (concurrent commit unsupported):** Two `/ark:commit` invocations on the same checkout concurrently can produce duplicate session numbers because the workspace SPEC's `write_journal_and_index` computes `highest_session + 1` without a lock. **No lock added.** Concurrent `/ark:commit` is declared unsupported per NG-12. Documented in `--help`.

---

## Runtime `runtime logic`

[**Main Flow**]

1. `/ark:design --deep <title>` scaffolds. `task new` captures `start_head` from parent HEAD into `task.toml.start_head`.
2. `/ark:plan` → `/ark:review` ↔ `/ark:plan` → `/ark:execute` unchanged.
3. `/ark:verify` seeds `VERIFY.md` with auto-populated checklist sections. Implementer fills sections during EXECUTE → COMMIT.
4. `/ark:commit -m "<msg>"`:
   a. Load `task.toml` (snapshot `prev_toml`). Verify phase.
   b. Verify working tree non-empty (unless `--no-commit`).
   c. Parse VERIFY.md. Refuse if any PENDING (deep) or warn (standard).
   d. Deep tier: extract SPEC.
   e. Render journal entry. Append. Re-render index. Snapshot pre-append length + prior index bytes.
   f. Save updated `task.toml` (phase=Committed, committed_at=now).
   g. `git add -A`.
   h. `git commit -m "<msg>"`. On failure, run rollback path. Return `GitCommitFailed`.
   i. On success, re-resolve HEAD; write `committed_head = Some(head_sha)` to task.toml. **Single dirty file.**
5. Time passes. PR review may add commits. Multiple tasks accumulate in `phase = Committed`.
6. `ark archive` (manager): enumerates committed tasks, derives `archive_month` per task from its `committed_at`, calls `task_archive_move` per slug. **No SPEC promotion. No journal recording.**

[**Failure Flow**]

1. `task new` git rev-parse fails (unborn HEAD): `start_head = None`. Task proceeds; later `task_commit` falls back to `git log -n 20`.
2. `task_commit` step 2 (working tree clean, no `--no-commit`): `Error::NothingToCommit`. Task state unchanged.
3. `task_commit` step 3 (VERIFY incomplete on deep): `Error::VerifyIncomplete`. Task state unchanged.
4. `task_commit` step 4 (spec_extract fails): hard error from spec_extract. Task state unchanged (toml not yet saved).
5. `task_commit` step 5 (journal append fails): hard error from PathExt. Task state unchanged.
6. `task_commit` step 7 (`git commit` fails): **rollback runs.** `prev_toml.save` restores phase=Verify; journal truncated to pre-append length; index restored; `git reset` unstages. Hard error `GitCommitFailed`. Task state is exactly what it was. Re-invoke after fixing the hook is safe.
7. `task_commit` step 8 (`git rev-parse HEAD` post-commit fails): closing commit landed; `committed_head` cannot be written. Stderr emits manual-recovery note. Phase still `Committed` (committed_at was in the closing commit). State machine invariants satisfied.
8. `ark_archive` per-slug failure: continues processing remaining slugs; final summary lists failures. Exit non-zero iff any failure occurred.

[**State Transitions**]

- `Phase::Verify → Phase::Committed` when `task_commit` runs successfully on standard or deep.
- `Phase::Execute → Phase::Committed` when `task_commit` runs successfully on quick.
- `Phase::Committed → Phase::Archived` when `task_archive_move` runs (invoked by `ark archive`).
- `Phase::Archived` remains terminal.

---

## Implementation `split task into phases`

[**Phase 1 — State machine + start_head capture**]

1. Add `Phase::Committed` to `state.rs`. Update `can_transition` per G-1. Update `archived_is_terminal` test. Add `*_committed_is_legal_destination` per tier.
2. Add `start_head`, `committed_at`, `committed_head` to `TaskToml`. Update `task_toml_loads_without_worktree_fields` test.
3. In `task_new::build_task_toml`, capture `start_head` via `run_git(&["rev-parse","HEAD"], opts.project_root)` (with `.ok().filter(is_success).map(stdout.trim)`).
4. Unit tests: `task_new_captures_start_head`, `task_new_with_unborn_head_records_none`.
5. Verify CI passes; no behavior change yet.

[**Phase 2 — `task_commit` and rollback protocol**]

1. Create `crates/ark-core/src/commands/agent/task/commit.rs`. Implement `task_commit` with the nine-step sequence and rollback. Private helpers: `parse_verify_md`, `render_and_append_journal`, `truncate_file_to_len`, `restore_index_bytes`.
2. Add `pub mod commit; pub use commit::*;` to `task/mod.rs`. Re-exports to `lib.rs`.
3. Add `RecordTaskOptions.start_head: Option<String>`; rename `archive_path → task_dir`, `archived_at → recorded_at`. Thread `start_head` through.
4. Add `JournalEntry.start_head` and `.base_branch`. Update `render_entry` to emit lines for task entries when Some. Update golden tests.
5. Add error variants `NothingToCommit`, `VerifyIncomplete`, `GitCommitFailed`, `CommitMessageRequired`, `CommittedAtMissing`, `TemplateMarkerMissing`.
6. Wire `Commit(TaskCommitCliArgs)` into `agent_cli.rs`'s `TaskCommand` + dispatcher.
7. Integration tests: each tier × `--no-commit` × VERIFY-pending matrix; rollback `task_commit_rollback_on_pre_commit_hook_failure`.
8. Verify CI passes.

[**Phase 3 — `task_archive_move` (rename + strip side effects) and `ark archive`**]

1. Rename `task_archive` → `task_archive_move` in `task/archive.rs`. Strip `spec_extract`/`spec_register`. Strip `record_task` call. Strip `WorkspaceRecorded` from summary. Add `archive_month: String` to options. Use `archive_month` for the bucket directory.
2. Update precondition: `check_transition(toml.tier, toml.phase, Phase::Archived)` — legal only from `Committed` per G-1.
3. Migrate existing `archive.rs` tests: tests asserting bundled SPEC/journal behavior move to `task_commit`'s suite. Remaining tests assert only move + state cleanup.
4. New tests `archive_no_spec_promotion`, `archive_no_journal_write`.
5. Create `crates/ark-core/src/commands/archive.rs`. Implement `ark_archive`. Reuse `task_archive_move` per slug.
6. Add `Command::Archive(ArchiveCliArgs)` to `ark-cli/src/main.rs`'s top-level enum + dispatcher.
7. Integration tests: `ark_archive_archives_committed_tasks`, `_skips_uncommitted_tasks`, `_dry_run_lists_only`, `_filters_by_month`, `_idempotent`, `_uses_committed_at_month_not_now`, `_does_not_promote_spec`, `_does_not_write_journal`.
8. Update `lib.rs` re-exports. **Delete** `task_archive` re-export. **Delete** `Archive(TaskSlugArgs)` from `agent_cli.rs`.
9. Verify CI passes.

[**Phase 4 — VERIFY template, seed protocol, and migration**]

1. Rewrite `templates/ark/templates/VERIFY.md` per G-4 with the four marker tokens.
2. Create `crates/ark-core/src/commands/agent/task/verify_seed.rs` with `render_seeded_verify(SeedInputs) -> Result<String>`. `pub mod verify_seed;` in `task/mod.rs`.
3. In `task/phase.rs`'s `task_verify`, after `copy_template("VERIFY", ...)`, perform marker substitution (read just-copied file → call `render_seeded_verify` → overwrite).
4. Unit tests: `render_seeded_verify_substitutes_*` for each marker; `_errors_on_missing_marker`.
5. Update `commands/upgrade.rs` for legacy VERIFY.md migration. Add `migrate_legacy_verify_md` private helper + tests.
6. Delete `templates/claude/commands/ark/archive.md`, `templates/codex/skills/ark-archive/SKILL.md`, `templates/opencode/commands/ark/archive.md`.
7. Verify CI passes.

[**Phase 5 — Slash command surface (Claude / Codex / OpenCode)**]

1. Create `templates/claude/commands/ark/commit.md`, `templates/codex/skills/ark-commit/SKILL.md`, `templates/opencode/commands/ark/commit.md`. Bodies per G-7. Wrap-up message includes the dirty-task.toml note.
2. Update `design.md` (all 3 platforms): step 5.2 → `/ark:commit -m "<message>"`.
3. Update `quick.md` (all 3 platforms): step 8 → `/ark:commit -m "<message>"`.
4. Update `record.md` (all 3 platforms): brief note that manual record is unaffected.
5. Add `PhaseFilter::Commit` to `commands/context/projection.rs`. Same shape as Verify projection: paths only.
6. Verify CI passes; lockstep diff between platforms shows identical bodies.

[**Phase 6 — Workflow doc + AGENTS.md + cleanup**]

1. Update `templates/ark/workflow.md` per G-8. Update `.ark/workflow.md` in lockstep.
2. Update `AGENTS.md`: drop `/ark:archive` row, add `/ark:commit` row.
3. Sweep tests: any test calling `task_archive` directly migrates to `task_commit` or `task_archive_move`.
4. Update `concurrency_tests.rs` to verify `Committed` tasks stay in `tasks.active`.
5. `cargo test --all-targets`. `cargo clippy --all-targets`. `cargo fmt --check`.
6. End-to-end smoke on a fresh `tempdir`-backed install: design → plan → review → execute → verify → commit → bulk-archive. Assert (a) closing commit contains work + journal entry + task.toml; (b) post-commit, only `task.toml`'s `committed_head` is dirty; (c) bulk archive moves the dir without touching SPEC files or journal.

---

## Trade-offs `ask reviewer for advice`

- **T-1: Drop self-referential SHA from journal entry (chosen).** Per R-001's recommendation. Journal records `start_head` + `base_branch` + pre-commit commits-in-range. Exact closing SHA recorded in `task.toml.committed_head` post-commit (intentionally dirty file). **Advantages:** mathematically possible; one git commit captures work + journal + task.toml; rollback is clean truncate+restore; no patch step. **Disadvantages:** closing SHA lives in `task.toml` rather than in the entry body. **Rejected alternatives:** amend (pushed-HEAD risk), two commits (inelegant), HEAD-PENDING patch (impossible).

- **T-2: Rename `task_archive` → `task_archive_move`, delete legacy bundle (chosen).** Per R-003 / TR-2 Option B. **Advantages:** explicit at call sites; no hidden mode flag; clean deletion since no in-repo caller needs the legacy version. **Disadvantages:** rename ripples through tests; out-of-tree consumers must update. **Rejected alternative:** add `suppress_side_effects` flag — adds API surface for one caller.

- **T-3: Body-free commit projection (chosen).** Per R-005 / TR-3 Option A. Preserves `ark-context` SPEC's additive-only schema. **Advantages:** small change; no new contract. **Disadvantages:** slash commands make one extra disk read for VERIFY content. **Rejected alternative:** include body — breaks contract.

- **T-4: Tier-conditional VERIFY gate (deep refuses, standard warns).** Unchanged from `00_PLAN`.

- **T-5: Slash command generates message (chosen).** Unchanged from `00_PLAN`.

- **T-6: Concurrent `/ark:commit` declared unsupported (chosen).** Per R-007. **Advantages:** small diff; honest about current concurrency. **Disadvantages:** an exotic but real failure (duplicate session numbers). **Rejected alternative:** add per-checkout `task_commit` lock — defer to follow-up.

---

## Validation `test design`

[**Unit Tests**]

- **V-UT-1:** `Phase::Committed` round-trips through TOML serde.
- **V-UT-2:** `can_transition(*, Verify, Committed)` and `can_transition(Quick, Execute, Committed)` true; legacy `(*, Verify|Execute, Archived)` false; only `(*, Committed, Archived)` reaches `Archived`.
- **V-UT-3:** `archived_is_terminal` test holds; `Committed` is not terminal.
- **V-UT-4:** `task_new` captures `start_head` via `git rev-parse HEAD`.
- **V-UT-5:** `task_new` on unborn HEAD records `start_head = None`.
- **V-UT-6:** `task.toml` round-trips with `start_head`, `committed_at`, `committed_head` all set.
- **V-UT-7:** Pre-refactor `task.toml` deserializes cleanly with each new field as `None`.
- **V-UT-8:** `task_commit` on standard tier from `Verify` with non-empty work + complete VERIFY.md transitions to `Committed`, sets `committed_at` and `committed_head`. **Asserts the closing commit's tree contains journal entry, saved task.toml with phase=Committed, and work changes.**
- **V-UT-9:** `task_commit` on deep with PENDING items returns `Error::VerifyIncomplete`.
- **V-UT-10:** `task_commit` on standard with PENDING items emits stderr warnings and proceeds.
- **V-UT-11:** `task_commit` with empty working tree returns `Error::NothingToCommit`.
- **V-UT-12:** `task_commit --no-commit` on deep extracts SPEC and transitions to `Committed`; no git commit; no journal write; `committed_head = None`.
- **V-UT-13:** `task_commit` without `-m` and `no_commit = false` returns `Error::CommitMessageRequired`.
- **V-UT-14:** `task_commit` writes journal with `**Start Head**: \`<start_head>\`` and `**Base Branch**: \`<base>\`` rendered correctly.
- **V-UT-15:** `task_commit` on a pre-refactor task (`start_head = None`) renders `**Start Head**: \`(unknown — pre-refactor task)\``.
- **V-UT-16 (new):** `task_commit` rollback on `git commit` failure: pre-commit hook rejects → prev_toml restored, journal truncated, index restored, `git reset` clean. Re-invoke after hook fix succeeds.
- **V-UT-17 (new):** `task_commit` post-commit `committed_head` write produces single-file dirty: `git status --porcelain` returns exactly `task.toml`.
- **V-UT-18:** `parse_verify_md` returns 0 pending on resolved doc.
- **V-UT-19:** `parse_verify_md` returns N items when N checklist items remain `PENDING`.
- **V-UT-20:** `parse_verify_md` counts findings whose `Resolution: PENDING`.
- **V-UT-21:** `render_seeded_verify` substitutes `{{PROJECT_SPEC_COMPLIANCE}}` with one bullet per project SPEC.
- **V-UT-22:** `render_seeded_verify` substitutes `{{RELATED_FEATURE_COMPLIANCE}}` from PRD's `[**Related Specs**]`.
- **V-UT-23:** `render_seeded_verify` errors with `TemplateMarkerMissing` if a marker is absent.
- **V-UT-24:** `task_archive_move` accepts only `phase = Committed` tasks; rejects others.
- **V-UT-25 (new):** `task_archive_move` does not invoke `spec_extract`, `spec_register`, or `record_task`. Asserted by post-conditions.
- **V-UT-26:** `ark_archive` enumerates only `phase = Committed` tasks.
- **V-UT-27 (new):** `ark_archive` derives `archive_month` from `committed_at`, NOT `Utc::now()`. Specifically: a task with `committed_at = 2026-03-15T12:00:00Z` archived to `.ark/tasks/archive/2026-03/<slug>/` even when `ark archive` runs on `2026-05-01`.
- **V-UT-28:** `ark_archive --month 2026-05` filters out other months.
- **V-UT-29:** `ark_archive --dry-run` does not move any directory.
- **V-UT-30 (new):** `ark_archive` skips a task with `phase=Committed && committed_at=None` and reports `Error::CommittedAtMissing` in failures.

[**Integration Tests**]

- **V-IT-1:** End-to-end deep-tier flow: design → plan → review → execute → verify → commit → ark_archive. Asserts archived task at `.ark/tasks/archive/YYYY-MM/<slug>/`, feature SPEC, INDEX row, journal entry with `**Start Head**` populated. **Asserts archived `task.toml` has `committed_head = Some(<sha>)`.**
- **V-IT-2:** End-to-end standard-tier (no review, no SPEC).
- **V-IT-3:** End-to-end quick-tier.
- **V-IT-4:** Multi-task `ark_archive`: three tasks committed in three different months bulk-archived to respective YYYY-MM buckets derived from each `committed_at`.
- **V-IT-5:** `ark_archive --month YYYY-MM` filters correctly.
- **V-IT-6:** Slash-command lockstep: `commit.md` (Claude+OpenCode), `ark-commit/SKILL.md` (Codex) all exist; `archive.md` and `ark-archive` absent.
- **V-IT-7:** `ark upgrade` migrates legacy `VERIFY.md`; preserves prior `## Findings`.
- **V-IT-8:** `ark upgrade` unlinks orphan `archive.md`.
- **V-IT-9:** `ark context --for commit` returns paths only, **no file body** in JSON output.
- **V-IT-10:** State-file reconcile keeps `phase = Committed` slugs in `tasks.active`; bulk archive removes only after archive.
- **V-IT-11 (new):** `ark_archive` does not write to any journal file (snapshot mtime+content before; assert unchanged).
- **V-IT-12 (new):** `ark_archive` does not modify any feature SPEC (snapshot mtime+content before; assert unchanged).

[**Failure / Robustness Validation**]

- **V-F-1:** Pre-commit hook rejects: rollback restores everything; phase remains `Verify`; re-invoke after fix succeeds.
- **V-F-2:** `git rev-parse HEAD` post-commit fails: closing commit landed; `committed_head` not written; stderr note; phase is `Committed`.
- **V-F-3:** `ark_archive` with one corrupt `task.toml`: bad slug in failures; remaining slugs proceed; non-zero exit.
- **V-F-4:** `task_commit --no-commit` followed by manual commit + manual `/ark:record`: produces journal entry without `**Start Head**`; `phase = Committed` task is picked up by `ark_archive`.
- **V-F-5:** `ark upgrade` on corrupt VERIFY.md: falls back to fresh-seed; stderr warning.
- **V-F-6:** `task_commit` on a task whose worktree has been deleted: `Error::Io` from `git status`; phase remains `Verify`.

[**Edge Case Validation**]

- **V-E-1:** Two tasks committed in same month bulk-archive into the same `archive/YYYY-MM/` under their own slugs.
- **V-E-2:** A task whose `start_head` equals current HEAD: commits-in-range table is empty; `**Start Head**` field still renders.
- **V-E-3:** Standard `task_commit` invoked without `task_verify`: phase precondition rejects with `IllegalPhaseTransition`.
- **V-E-4:** Deep task with zero `## Spec` Goals: `render_seeded_verify` emits empty Plan Fidelity. Not an error.
- **V-E-5:** Task with `branch = None`: `task_cwd` defaults to `layout.root()`.
- **V-E-7:** `--no-commit` on quick: skips commit + journal, transitions to `Committed`.
- **V-E-8 (new):** Task with `phase=Committed && committed_at=None`: `ark_archive` skips with `Error::CommittedAtMissing` in failures.
- **V-E-6 deleted per R-007:** concurrent commit case removed; declared unsupported in NG-12.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-UT-2, V-UT-3, V-IT-10 |
| G-2 | V-UT-4, V-UT-5, V-UT-6, V-UT-7, V-UT-17 |
| G-3 | V-UT-8 .. V-UT-15, V-UT-16, V-UT-17, V-IT-1, V-IT-2, V-IT-3, V-F-1, V-F-2, V-F-6 |
| G-4 | V-UT-21, V-UT-22, V-UT-23, V-IT-7 |
| G-5 | V-UT-14, V-UT-15, V-IT-1 |
| G-6 | V-UT-26, V-UT-27, V-UT-28, V-UT-29, V-UT-30, V-IT-4, V-IT-5, V-IT-11, V-IT-12, V-F-3 |
| G-7 | V-IT-6 |
| G-8 | V-IT-6 + manual diff review |
| G-9 | V-IT-6 |
| G-10 | V-IT-7, V-IT-8, V-F-5 |
| C-1 | code review + cargo doc graph |
| C-2 | extends existing source-scan test |
| C-3 | V-UT-13 |
| C-4 | V-UT-16, V-UT-17, V-F-1 |
| C-5 | V-UT-4, V-UT-5 |
| C-6 | V-IT-10 |
| C-7 | V-UT-18, V-UT-19, V-UT-20 |
| C-8 | V-IT-6 |
| C-9 | V-UT-21, V-UT-22, V-UT-23 |
| C-10 | code review |
| C-11 | manual `ark --help` + `ark agent --help` smoke |
| C-12 | V-IT-7 (re-run on already-migrated file) |
| C-13 | V-F-4 |
| C-14 | V-UT-12 |
| C-15 | V-IT-1 (CHANGELOG row check) |
| C-16 | V-F-4 |
| C-17 | V-IT-9 |
| C-18 | V-UT-25, V-IT-11, V-IT-12 |
| C-19 | V-UT-2, V-UT-3 |
| C-20 | V-IT-6 |
| C-21 | V-UT-30, V-E-8 |
| C-22 | declared unsupported (NG-12); no test |
