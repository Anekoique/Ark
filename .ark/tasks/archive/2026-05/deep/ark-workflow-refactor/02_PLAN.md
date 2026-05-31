# `ark-workflow-refactor` PLAN `02`

> Status: Revised — incorporates fixes for `02_REVIEW.md` findings R-201..R-205 in place (no `03_PLAN.md` per user instruction).
> Feature: `ark-workflow-refactor`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md` (round 02 review at `02_REVIEW.md` was incorporated in place — see Log)
> - Master Directive: `none`
> - Related Specs: `workspace`, `task-concurrency-control`, `ark-agent-namespace`, `ark-context`, `ark-upgrade`, `worktree`, `codex-support`, `opencode-support`, `project-spec`

---

## Summary

Iteration 02 closes the remaining gaps in the closure protocol. The current revision (incorporating `02_REVIEW.md`'s R-201..R-205 in place per user instruction) hardens four points:

1. **One durability class.** Drop `task.toml.committed_head`; the closing commit is the *only* mutation of any kind. The exact closing SHA is recoverable any time via a slug-anchored search of the journal file's git history (see G-5). After a successful `/ark:commit`, **no Ark-managed files are dirty** (user-pre-existing unstaged files outside Ark's purview may still appear; that is the staged-only workflow at work — see C-23).
2. **Staged-only workflow.** Replace `git add -A` with explicit per-file staging of only ark-managed artifacts (journal file, workspace index, task.toml, and on deep tier, the promoted SPEC + features INDEX). Precondition: `git diff --cached --quiet` returns non-zero (user has already staged work). Rollback uses targeted unstaging (`git reset HEAD <ark-files>`) so the user's pre-existing staged-work intent survives a failed retry.
3. **Scoped rollback guard for SPEC + journal + index + toml.** A `RollbackGuard` RAII helper accumulates snapshots as steps complete and restores them on `Drop` unless explicitly disarmed by `commit()` on the success path. This makes rollback robust against partial failures at *any* mutation point — `spec_extract`, `spec_register`, journal append, workspace index rerender, `task.toml` save, `git add`, or `git commit`. (R-203's central fix.)
4. **Slug-anchored closing-SHA recovery.** The earlier proposal to recover the closing SHA via `git log -n 1 -- <journal-path>` is wrong: workspace journals are shared across tasks and can be appended to by later manual `/ark:record` entries or later task commits before rotation. The recovery primitive is now `git log -S '**Slug**: <slug>' --diff-filter=A --format=%H -- <journal-path> -n 1` — slug-anchored, addition-filtered. Each entry is unique by slug; the `-S` lookup survives later writes to the same file. (R-201's central fix.)

The PRD's Outcome item 7 has been **updated in iteration 02** to authorize the journal shape change (Start Head + Base Branch, no inline closing SHA), and **further updated in this revision** to specify the slug-anchored recovery primitive (R-201 fix). The PRD's `[**What**]`, Outcome item 2, and the workspace `[**Related Specs**]` row were also swept for stale exact-range wording (R-202 fix).

`ark agent task archive` is preserved (hidden, internal) as `ark agent task archive` (the verb name from `ark-agent-namespace` SPEC), with semantics now side-effect-free (rename + state-cleanup only). The Rust helper is named `task_archive_move` to disambiguate from the legacy bundling function. (R-205 fix; consistent naming throughout.)

## Log

[**Added**]

- **G-3a:** SPEC files (`specs/features/<slug>/SPEC.md` + `specs/features/INDEX.md`) are added to the rollback snapshot set. New private helpers `snapshot_spec_files` and `restore_spec_files` in `commit.rs`.
- **G-3b:** Explicit staging step replaces `git add -A`. `task_commit` stages only the ark-managed files: journal-N.md, workspace index, task.toml, (deep tier) SPEC + features INDEX.
- **G-3c:** Precondition for non-`--no-commit` runs is `git diff --cached --quiet` returns non-zero (staging area carries user work). Replaces the previous `git status --porcelain` non-empty check.
- **G-3d (NEW per R-203):** Rollback is implemented as a scoped `RollbackGuard` RAII helper that accumulates snapshots and restores them on `Drop` unless `commit()` is called on the success path. Snapshots become recoverable as soon as they are taken — partial failures at any mutation point (spec_extract, spec_register, journal append, index rerender, task.toml save, git add, git commit) restore exactly the snapshots that exist. Linear sequence with named restore points replaces the previous "rollback only on git-commit failure" wording.
- **G-9a:** `ark agent task archive` (hidden) survives as a one-off helper for maintainers. Calls `task_archive_move` with `--slug <s>` and an optional `--month YYYY-MM` (defaults to the task's own `committed_at`). The CLI subcommand keeps the verb name `archive` per the `ark-agent-namespace` SPEC; the underlying Rust function is `task_archive_move` to disambiguate from the legacy bundling function. (R-205: consistent naming.)

[**Changed**]

- **G-2:** `task.toml.committed_head` is **removed**. The exact closing SHA is no longer persisted in `task.toml`. Other new fields (`start_head`, `committed_at`) remain.
- **G-3 step 7 (stage + commit):** explicit per-file staging via `git add <journal> <workspace-index> <task.toml> [<spec>] [<features-index>]`, not `git add -A`.
- **G-3 step 7 rollback:** `git reset HEAD <ark-files>` instead of `git reset` (preserves user's pre-existing index entries). Plus restores SPEC snapshot files.
- **G-3 step 8 (post-commit metadata):** **deleted**. There is no post-commit local mutation. The slash command's wrap-up reports the commit SHA (read from `git rev-parse HEAD` for display only); ark does not persist it.
- **G-5 (revised per R-201):** PRD authoritative wording on the journal shape lives in PRD Outcome item 7. The PLAN's G-5 description: `**Start Head**` + `**Base Branch**`, no inline closing SHA. Commits-in-range table populated by `git log <start_head>..HEAD --oneline -n 20` from task's cwd, computed *just before* the journal append. The closing commit is **intentionally absent** from the table — it doesn't exist yet at journal-write time. The closing SHA is recoverable via the slug-anchored primitive `git log -S '**Slug**: <slug>' --diff-filter=A --format=%H -- <journal-path> -n 1`. **`-S`** matches commits whose diff added the literal string `**Slug**: <slug>`; **`--diff-filter=A`** restricts to additions (so amends and reverts do not match the original closure commit); the slug field is unique per journal by construction (each task is recorded exactly once). The recovery primitive remains valid after later manual `/ark:record` entries and after later task commits append to the same `journal-N.md`. The earlier `git log -n 1 -- <journal-path>` proposal is **withdrawn** (would return whatever commit most recently touched the file, including unrelated later writes).
- **G-6:** `ark archive` enumerator filters `phase = Committed && committed_at = Some(...)`. The "skip with stderr warning if `committed_at` is None" path is preserved (defensive against external corruption); per-slug failures collected as before.
- **G-9 (revised per R-205):** the agent-CLI keeps the verb name `archive` (per `ark-agent-namespace` SPEC). The variant is `Archive(TaskArchiveMoveCliArgs)` with `--slug` and `--month`. Replaces the originally-removed `Archive(TaskSlugArgs)` variant. Hidden under `ark agent`; `ark archive` (top-level) is the user-facing path. **Naming swept consistently throughout PLAN and Architecture: CLI subcommand is `archive`; underlying Rust function is `task_archive_move`.**
- **C-4 (revised per R-203):** rewritten to specify the explicit-staging protocol + the `RollbackGuard` RAII helper that covers all snapshot-restore points (not just `git commit` failure). Partial failures at every mutation point trigger the same restore path.
- **C-23 (revised per R-204):** the post-commit cleanliness invariant is "no Ark-managed files dirty" — specifically, none of `[journal-N.md, workspace index, task.toml, (deep) SPEC, (deep) features INDEX]` show up in `git status --porcelain`. **User's pre-existing unstaged files outside Ark's purview are not affected by this guarantee** (the staged-only workflow intentionally leaves them untouched). The earlier "whole worktree clean" wording is withdrawn.
- **TR-1:** resolution updated to "one durability class — everything in the closing commit, or rollback-covered."
- **TR-2 (NEW per R-203, scoped rollback guard):** the rollback design lives in a `RollbackGuard` struct described in Data Structure; partial-failure tests added to the validation matrix.
- **PRD Outcome item 7 (updated this iteration, revised again per R-201):** authorizes the recoverable-via-slug-anchored-`git log -S` primitive; the path-only `git log -n 1 -- <journal-path>` proposal is withdrawn. PRD's `[**What**]`, Outcome item 2, and `[**Related Specs**]` workspace row were also swept (R-202 fix).
- **V-UT-17 (revised per R-204):** assertion changed from "`git status --porcelain` empty" to "no Ark-managed file appears in `git status --porcelain`" (precise predicate).
- **V-UT-25:** unchanged.
- **V-IT-1, V-IT-2, V-IT-3 (revised per R-204):** assertions updated — closing commit contains work + journal + workspace index + task.toml + (deep) SPEC + features INDEX in one commit; post-commit `git status --porcelain` shows no Ark-managed file (user-pre-existing unstaged files allowed).

[**Removed**]

- **`task.toml.committed_head` field** (was G-2 in `01_PLAN`). Eliminates the third durability class.
- **`TaskCommitSummary.committed_head_dirty: bool`** (was a return-value flag in `01_PLAN`'s Data Structure). The post-commit working tree is clean; the flag is meaningless.
- **G-3 step 8 in `01_PLAN`** (post-commit re-load + `committed_head` save). No longer exists.
- **`git add -A`** as the staging primitive. Replaced by explicit per-file staging.

[**Unresolved**]

None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-101 | Accepted | SPEC files (`specs/features/<slug>/SPEC.md` + `specs/features/INDEX.md`) added to the rollback snapshot set. Pre-extract bytes captured into `prev_spec_bytes: Option<Vec<u8>>` (None if file did not exist) and `prev_features_index_bytes: Vec<u8>`. On commit failure, restore both — delete the SPEC file if it didn't exist before, restore bytes if it did; restore INDEX bytes unconditionally. New tests `task_commit_rollback_restores_spec_files`, `task_commit_rollback_restores_features_index`. SPEC promotion stays pre-commit so the SPEC files land in the closing commit. See G-3, C-4 in this iteration. |
| Review | R-102 | Accepted (TR-2: staged-only) | Replaced `git add -A` with explicit per-file staging. `task_commit` stages exactly: `<journal-N.md>`, `<task.toml>`, and (deep tier) `<specs/features/<slug>/SPEC.md>` + `<specs/features/INDEX.md>`. Working-tree precondition is now `git diff --cached --quiet` returns non-zero (the user has already staged work). Empty stage → `Error::NothingStaged { slug }` (renamed from `NothingToCommit`). User's unstaged files are not touched by `task_commit`. See G-3 step 7, C-4 in this iteration. |
| Review | R-103 | Accepted | Rollback now runs `git reset HEAD <ark-files>` instead of `git reset` (no targets). The user's pre-existing index state for non-ark files is preserved. New test `task_commit_rollback_preserves_user_staging_intent` covers: user pre-stages `foo.txt`, also has unstaged `bar.txt`; pre-commit hook rejects; after rollback, `foo.txt` is still staged, `bar.txt` is still dirty-but-unstaged, ark files are unstaged. See G-3 step 7 rollback in this iteration. |
| Review | R-104 | Accepted | The PRD's Outcome item 7 has been **updated in this iteration** to authorize the recoverable-via-`git log` invariant. The original wording asked for inline `commit_range = "<start_head>..<HEAD>"` in the journal entry, which Git's content-addressed object model makes impossible (R-001's CRITICAL finding from iteration 00). The PRD now records: journal entry has `**Start Head**` + `**Base Branch**` only; the closing-commit SHA is recoverable via `git log -n 1 -- <journal-path>`. The PRD edit is committed to disk; the PLAN's G-5 substance is unchanged from `01_PLAN`. See PRD Outcome item 7 + G-5 in this iteration. |
| Review | R-105 | Accepted (chose option E from review-deliberation: drop `committed_head`) | `task.toml.committed_head` is removed entirely. The closing-SHA is recoverable from `git log -n 1 -- <journal-path>` (the journal file is touched *only* by task-closing commits — by construction, since `task_commit` is the only writer of journal entries with `**Start Head**` populated, and ark never amends journal-touching commits). After a successful `/ark:commit`, the working tree is **clean**. Resolves TR-1's "three durability classes" criticism: closing commit contains everything; rollback covers everything; nothing is left dirty. See G-2, G-3 step 8 (deleted), Runtime Main Flow + Failure Flow in this iteration. |
| Review | R-106 | Accepted | `ark agent task archive-move` (hidden) preserved as a one-off helper. `Archive(TaskArchiveMoveCliArgs)` variant in `agent_cli.rs`'s `TaskCommand`: `--slug <s>`, optional `--month YYYY-MM` (defaults to the task's `committed_at`). The `ark-agent-namespace` SPEC's `task archive` verb stays in the verb set; only its mechanics change (side-effect-free move). See G-9 in this iteration. |
| Trade-off Advice | TR-1 | Applied | One durability class. Closure artifacts are either committed atomically (work + journal + task.toml + (deep) SPEC + features INDEX, all in the closing commit) or rollback-covered (the same files snapshot-and-restore on commit failure). No post-commit dirty residue. |
| Trade-off Advice | TR-2 | Applied (staged-only) | Explicit per-file staging of ark-managed files. User work must be pre-staged. Unrelated unstaged files are never captured. Slash command body documents the precondition: "stage your work first; `/ark:commit` will only add ark-managed artifacts to the staging set." |
| Review (round 02) | R-201 | Accepted | Path-only `git log -n 1 -- <journal-path>` is unreliable; replaced with slug-anchored `git log -S '**Slug**: <slug>' --diff-filter=A --format=%H -- <journal-path> -n 1`. The slug field is unique per journal by construction (each task is recorded exactly once); `--diff-filter=A` ignores amends/reverts; the lookup remains valid after later manual `/ark:record` entries and after later task commits on the same `journal-N.md`. PRD Outcome item 7 + G-5 + Validation updated; new tests `closing_sha_recoverable_after_later_manual_record` and `closing_sha_recoverable_after_later_task_commit` enforce. See G-5 + V-IT-15/16 in this revision. |
| Review (round 02) | R-202 | Accepted | Swept PRD `[**What**]`, Outcome item 2, and `[**Related Specs**]` workspace row to remove all "exact range `start_head..HEAD`" / `commit_range = "<start_head>..<HEAD>"` wording. PRD Outcome item 7 is now the single authoritative wording for the journal contract. PRD edit committed in this revision; Log section flags it explicitly. |
| Review (round 02) | R-203 | Accepted | Rollback redesigned as a scoped `RollbackGuard` RAII helper. Snapshots accumulate as steps complete; on `Drop` (any error path before successful commit + disarm), all accumulated snapshots are restored. Replaces the previous "rollback only on git commit failure" wording. New tests `task_commit_rollback_on_spec_extract_failure_after_partial_write`, `task_commit_rollback_on_spec_register_failure_after_index_modification`, `task_commit_rollback_on_journal_append_failure`, `task_commit_rollback_on_task_toml_save_failure`. See G-3, C-4, Data Structure (`RollbackGuard`), Runtime Failure Flow in this revision. |
| Review (round 02) | R-204 | Accepted | The post-commit "clean tree" invariant rewritten to "no Ark-managed file dirty in `git status --porcelain`." User's pre-existing unstaged files outside Ark's purview are not Ark's concern (the staged-only workflow leaves them untouched, by design). Summary, slash-command wrap-up wording, C-23, V-UT-17, V-IT-1/2/3 all updated. |
| Review (round 02) | R-205 | Accepted | Hidden agent CLI subcommand named `archive` (preserves `ark-agent-namespace` SPEC verb set). Underlying Rust function named `task_archive_move`. Naming swept consistently across PLAN body, Data Structure, API Surface, and Implementation phases. |
| Trade-off Advice (round 02) | TR-1 | Applied | Slug-anchored `-S` lookup gives task-level closing SHA recovery that survives later journal writes. Replaces the unreliable path-only `git log -n 1` proposal. |
| Trade-off Advice (round 02) | TR-2 | Applied (Scoped Rollback Guard) | `RollbackGuard` RAII struct described in Data Structure. Tests for partial failures at every mutation point in the validation matrix. |

> Rules applied:
> - Every prior CRITICAL finding (R-101, R-201) addressed.
> - Every HIGH finding (R-102, R-103, R-104, R-202, R-203) addressed.
> - All MEDIUM/LOW findings (R-105, R-106, R-204, R-205) addressed.
> - All Trade-off Advice items resolved.

---

## Spec `Core specification`

[**Goals**]

- **G-1: Phase enum gains `Committed`; legal-transition table updated.** Unchanged from `01_PLAN`. `Phase` (in `crates/ark-core/src/commands/agent/state.rs`) gains the variant `Committed`. `can_transition` updated:
  - **Add:** `(Quick, Execute, Committed)`, `(Standard, Verify, Committed)`, `(Deep, Verify, Committed)`, `(Quick, Committed, Archived)`, `(Standard, Committed, Archived)`, `(Deep, Committed, Archived)`.
  - **Remove:** `(Quick, Execute, Archived)`, `(Standard, Verify, Archived)`, `(Deep, Verify, Archived)`. After the refactor, `Archived` is reachable only from `Committed`.
  - `TaskToml::status`: `Committed → InProgress`, `Archived → Completed`.
  - State-file reconcile keeps `phase == Committed` in `tasks.active`.

- **G-2: `task.toml` gains `start_head` and `committed_at`.** Two new optional fields:
  - `start_head: Option<String>` — captured at `task new` time as the parent checkout's HEAD SHA via `git rev-parse HEAD` from `opts.project_root`. On `task new --worktree`, the parent's HEAD SHA is captured before `git worktree add` (the worktree's initial HEAD equals the parent's HEAD at that moment). `None` on unborn HEAD or pre-refactor tasks.
  - `committed_at: Option<DateTime<Utc>>` — written by `task_commit` when it saves the updated toml (before invoking `git commit`). Drives the YYYY-MM bucket for `ark archive`.
  - **`committed_head` is NOT introduced.** The closing commit's SHA is recoverable from `git log -n 1 -- <journal-path>` (or, while the task is active, `git log -n 1 -- <task-dir>`); persisting it in `task.toml` is unnecessary and would create a third durability class.
  - Both new fields use `#[serde(skip_serializing_if = "Option::is_none", default)]`; pre-refactor `task.toml`s deserialize with each as `None`.

- **G-3 (revised per R-203): `task_commit` is the atomic closure with scoped rollback.** Public function `task_commit(opts: TaskCommitOptions) -> Result<TaskCommitSummary>` in `crates/ark-core/src/commands/agent/task/commit.rs`. Performs **a single git commit covering work + journal entry + the updated task.toml + (deep tier) the promoted SPEC + features INDEX**, with rollback covered uniformly across all mutation points by a `RollbackGuard` RAII helper (see Data Structure). After a successful commit, no Ark-managed file is dirty (per C-23) — there is no post-commit local mutation. Step ordering:

  **Rollback model:** the function constructs a `RollbackGuard` immediately after the precondition checks pass. As each subsequent step takes a destructive mutation, it first calls `guard.snapshot_<thing>(...)` to record the pre-mutation state. If any step returns `Err` (or panics), the guard's `Drop` runs and restores every accumulated snapshot in reverse-of-recording order. On the success path, the function explicitly calls `guard.commit()` (via a typestate `disarm` method) so the guard's drop becomes a no-op. This pattern is the same shape `tempfile::TempDir`, `std::fs::File`-with-cleanup-closures, and similar Rust idioms use; the cost is one struct + one `Drop` impl. Errors during rollback (e.g. an unwritable file) are logged to stderr but do not propagate (the user already has the original error; the rollback path is best-effort).

  1. **Phase precondition.** Load `TaskToml` (snapshot as `prev_toml`). Reject unless `(tier, phase)` is one of `(Quick, Execute)`, `(Standard, Verify)`, `(Deep, Verify)`. Wrong phase → `Error::IllegalPhaseTransition`.

  2. **Staged-work precondition.** When `!opts.no_commit`: run `git diff --cached --quiet` from the task's cwd. Exit code 0 (clean stage) → `Error::NothingStaged { slug }`. The user must have already staged their work via `git add` before invoking `/ark:commit`. Skipped under `--no-commit`.

  3. **VERIFY gate (tier-conditional).** Standard/Deep only: parse `VERIFY.md` (per C-7). Deep refuses on any pending → `Error::VerifyIncomplete { items, findings }`; standard warns; quick has no VERIFY.

  4. **Snapshot SPEC files (deep tier only, pre-extract).** Read `specs/features/<slug>/SPEC.md` if present → `prev_spec_bytes: Option<Vec<u8>>` (None if absent). Read `specs/features/INDEX.md` → `prev_features_index_bytes: Vec<u8>`. These snapshots feed the rollback path.

  5. **Deep-tier SPEC extraction.** Deep only: invoke `spec_extract` + `spec_register` on the *active* task dir. SPEC lands at `specs/features/<slug>/SPEC.md`; INDEX row upserted. Idempotent on re-run only when the inputs match (same plan content, same date); we rely on the snapshot in step 4 to make rollback strict.

  6. **Render + append journal entry (sub-step a-d).** When `!opts.no_commit`:
     a. Compute `commits_in_range = git log <start_head>..HEAD --oneline -n 20` from task's cwd (HEAD at this point is the *pre-commit* HEAD; the closing commit doesn't exist yet, so it is intentionally absent from this list — see G-5). When `start_head` is `None`, fall back to `base_branch..HEAD` if base_branch is Some, else `git log -n 20`.
     b. Render entry with `**Start Head**: <start_head>` + `**Base Branch**: <base_branch>` fields. **No exact final SHA in the entry.**
     c. Resolve target journal file (with rotation per workspace SPEC). Snapshot `pre_append_len: u64` (= file length pre-append) and `prev_index_bytes: Vec<u8>` (= `<dev>/index.md` bytes pre-rerender) for rollback.
     d. Append rendered entry. Re-render `<dev>/index.md`'s managed blocks via the existing `index::rerender` helper.

  7. **Save updated `task.toml`.** Build `next_toml` from `prev_toml` with `phase = Committed`, `committed_at = Some(now)`, `updated_at = now` (all other fields preserved). Write `next_toml` to `task_dir/task.toml`. This is the file that will be staged into the closing commit.

  8. **Stage explicit ark-managed files.** When `!opts.no_commit`:
     a. `commit_msg ← opts.message.clone().ok_or(Error::CommitMessageRequired)?`.
     b. Build `ark_files` list (relative to task's cwd):
        - the journal file (e.g. `.ark/workspace/<dev>/journal-N.md`)
        - the workspace index (e.g. `.ark/workspace/<dev>/index.md`)
        - `task.toml` for this slug (e.g. `.ark/tasks/<slug>/task.toml`)
        - **deep tier only:** `.ark/specs/features/<slug>/SPEC.md` and `.ark/specs/features/INDEX.md`
     c. Record `ark_files` into the active `RollbackGuard` so a subsequent failure can `git reset HEAD <ark_files>` (per C-25).
     d. `git add <ark_files...>` from task's cwd. Files outside `ark_files` (including any unstaged user edits, pre-existing user staged work) are not affected by ark.
     e. `git_commit_out ← run_git(&["commit","-m",&commit_msg], task_cwd)`.
     f. **On success:** disarm the `RollbackGuard` via `guard.commit()` so its `Drop` is a no-op; continue to step 9 (return summary; no post-commit mutation).
     g. **On failure:** return `Error::GitCommitFailed { stderr }` immediately. The error path drops the `RollbackGuard`, which restores every snapshot recorded so far (per C-4 + Data Structure: prev_toml → task.toml; truncate journal; restore workspace index; deep-tier restore SPEC + features INDEX; targeted `git reset HEAD <ark_files>` to unstage only ark's additions). The user's pre-existing staged-work intent for non-ark files is preserved.

  9. **Return summary.** When the commit succeeded:
     a. `head_sha ← run_git(&["rev-parse","HEAD"], task_cwd).stdout.trim()` (for the slash command's wrap-up display).
     b. **No mutations to disk.** The slash command displays `head_sha` to the user; ark does not persist it.
     c. Return `TaskCommitSummary { slug, tier, head_sha: Some(head_sha), journal_path, session_number, deep_spec_promoted, pending_verify }`.

  10. **`--no-commit` path.** Steps 2 (staged-work precondition), 6 (journal append), 8 (commit), 9 (head_sha capture) are all skipped. Steps 3, 4, 5, 7 still run. Stderr emits `--no-commit: journal not written; run /ark:record manually if you want a session entry`. If step 5 (deep SPEC extract) writes anything and the user later wants to undo it, they restore from git or manually remove the SPEC file — `--no-commit` is explicitly opt-in to this state. The summary's `head_sha` is `None`.

- **G-4: `VERIFY.md` is a living checklist + findings document.** Unchanged from `01_PLAN`. Six fixed sections. Four are dynamically seeded at `task verify` time: Project Spec Compliance (from `.ark/specs/project/INDEX.md`), Related Feature Spec Compliance (from PRD's `[**Related Specs**]`), PRD Constraints (from PRD's Outcome and Constraints), Plan Fidelity (from latest plan's `## Spec` Goals). One is fixed: SPEC Drift. One is open-ended: Findings. Plus Notes. No verdict line. Document is "complete" iff every checklist item is in `{PASS, FAIL, N/A}` and every finding's Resolution is in `{FIXED, ACCEPTED}`.

- **G-5 (revised per R-201): Journal entry records `**Start Head**` + `**Base Branch**`; closing-SHA recoverable via slug-anchored `git log -S`.** The per-task journal entry rendered by `task_commit` emits:
  - `**Start Head**: \`<start_head>\`` between `**Branch**` and `### Summary`. When `None`, renders `**Start Head**: \`(unknown — pre-refactor task)\``.
  - `**Base Branch**: \`<base_branch>\`` immediately after Start Head. When `None`, renders `**Base Branch**: \`(unknown)\``.
  - Commits table populated by `git log <start_head>..HEAD --oneline -n 20` resolved at journal-write time (HEAD is the *pre-commit* HEAD; the closing commit is not in the table, by construction).
  - **No `<HEAD-PENDING>` token. No `commit_range` field. No post-commit patching of journal entries.**
  - **Closing-SHA recovery primitive:** `git log -S '**Slug**: <slug>' --format=%H -n 1 -- <journal-path>` returns the SHA of the commit that introduced the slug entry. `-S` (pickaxe) matches commits whose diff changed the *count* of the exact string; the closing commit's diff increases the count by 1, so `-S` matches it. Amends/reverts that re-add and re-delete the same string produce a net-zero count change and are not matched. `<slug>` is unique per journal because each task is recorded exactly once. The lookup remains valid after later manual `/ark:record` entries (manual entries render `**Slug**: -`, which never collides with a real task slug) and after later task commits append to the same `journal-N.md` before rotation (each task commit adds its own slug, and `-S` for one slug ignores the others). **The earlier `--diff-filter=A` proposal is withdrawn**: `--diff-filter=A` matches whole-file additions only, not line additions, and the journal file pre-exists the closing commit (it was created by `workspace_init` in the prior commit).
  - The earlier `git log -n 1 -- <journal-path>` proposal is **withdrawn** (would return whatever commit most recently touched the file, which is wrong as soon as a later journal write lands).
  - Manual `/ark:record` path is unchanged: omits `**Start Head**` and `**Base Branch**` entirely. Importantly, manual entries also omit `**Slug**` (the existing workspace SPEC G-5 already dictates `**Slug**: -` for manual; we keep that). Therefore the `-S '**Slug**: <slug>'` primitive **cannot** match a manual entry by accident — manual entries' slug field is the literal `-`, and a real slug like `ark-workflow-refactor` will never collide with that.
  - **PRD Outcome item 7** (revised in this revision) is the authoritative wording.

- **G-6: `ark archive` is a top-level manager-only CLI; archive helper is side-effect-free.** Unchanged from `01_PLAN`. Top-level `ark archive [--dry-run] [--month YYYY-MM]` in `crates/ark-cli/src/main.rs`. Implementation in `crates/ark-core/src/commands/archive.rs`. Behavior:
  - Enumerate `.ark/tasks/<slug>/task.toml` excluding `.ark/tasks/archive/`. For each task with `phase = Committed && committed_at = Some(...)`, derive YYYY-MM from `committed_at`. Tasks with `phase = Committed && committed_at = None` are skipped + reported as `Error::CommittedAtMissing { slug }` in the failures list.
  - For each candidate, call `task_archive_move(TaskArchiveMoveOptions { project_root, slug, archive_month: <YYYY-MM from committed_at> })`. **No SPEC promotion. No journal recording.** Both happened at `task_commit` time and live in the closing commit.
  - `--month YYYY-MM` filters. `--dry-run` lists without moving. Idempotent. Per-slug failures collected, not fatal.

- **G-7: Slash-command and skill template surface.** Across all three platforms:
  - **Add** `commit.md` (Claude + OpenCode) and `ark-commit/SKILL.md` (Codex). Body parses `$ARGUMENTS` for `-m "<msg>"` and `--no-commit`; pulls `ark context --scope phase --for commit`; if no `-m` and no `--no-commit`, generates a conventional-commits message from the **staged diff** (per C-3) and shows for confirmation; invokes `ark agent task commit --message "<m>" [--no-commit]`. **Body documents the staged-work precondition** explicitly: "Stage your work first (`git add <files>`); `/ark:commit` only adds Ark-managed closure artifacts (journal, task.toml, deep-tier SPEC) to the staging set." Wrap-up reports the commit SHA, journal session number, and (deep tier) the promoted SPEC path. **Wrap-up notes the working tree is clean post-commit** (no dirty file).
  - **Remove** `archive.md` (Claude + OpenCode) and `ark-archive/SKILL.md` (Codex).
  - **Update** `design.md`, `quick.md`, `record.md` to point at `/ark:commit` instead of `/ark:archive`.
  - **Lockstep rule** unchanged.

- **G-8: `workflow.md` and `AGENTS.md` updated.** Lockstep updates per the Architecture diagram below. Same as `01_PLAN`'s G-8.

- **G-9: `ark agent task commit` and `ark agent task archive-move` CLI subcommands.** In `crates/ark-cli/src/agent_cli.rs`'s `TaskCommand` enum:
  - **Add** `Commit(TaskCommitCliArgs)` with `--message <msg>` (`-m` short) and `--no-commit`. Dispatch wires through to `ark_core::task_commit`.
  - **Replace** the existing `Archive(TaskSlugArgs)` variant with `Archive(TaskArchiveMoveCliArgs)`: `--slug <s>` and optional `--month YYYY-MM`. Dispatch wires through to `ark_core::task_archive_move`. When `--month` is absent, the dispatcher loads `task.toml` and uses `committed_at`'s YYYY-MM. This preserves the `ark-agent-namespace` SPEC's expectation that an archive verb exists. **Hidden under `ark agent`**, not user-facing; bulk `ark archive` is the user-facing path.

- **G-10: Migration on `ark upgrade`.** Same as `01_PLAN`. Slash-command refresh + legacy VERIFY.md regeneration + orphan archive.md unlink. **No `start_head` backfill.** **No `committed_head` backfill** (the field doesn't exist).

- **NG-1..NG-12:** unchanged from `01_PLAN`.

[**Architecture**]

```text
crates/
├── ark-cli/src/
│   ├── main.rs                                  ─ ADD top-level Archive(ArchiveCliArgs).
│   └── agent_cli.rs                             ─ ADD Commit(TaskCommitCliArgs);
│                                                  REPLACE Archive(TaskSlugArgs) with
│                                                  Archive(TaskArchiveMoveCliArgs)
│                                                  (--slug, optional --month).
└── ark-core/src/
    ├── commands/
    │   ├── archive.rs                           ─ NEW: pub fn ark_archive(opts).
    │   ├── upgrade.rs                           ─ MOD: VERIFY.md migration + orphan unlink.
    │   ├── agent/
    │   │   ├── state.rs                         ─ MOD: Phase::Committed; can_transition;
    │   │   │                                       status() handles Committed.
    │   │   ├── task/
    │   │   │   ├── commit.rs                    ─ NEW: pub fn task_commit + types;
    │   │   │   │                                  RollbackGuard RAII helper covers SPEC +
    │   │   │   │                                  features INDEX + journal + workspace index +
    │   │   │   │                                  task.toml; explicit per-file staging; targeted
    │   │   │   │                                  `git reset HEAD <ark_files>` on rollback.
    │   │   │   ├── new.rs                       ─ MOD: build_task_toml captures start_head.
    │   │   │   ├── archive.rs                   ─ MOD: rename task_archive →
    │   │   │   │                                  task_archive_move; STRIP spec_extract +
    │   │   │   │                                  spec_register + record_task. New signature:
    │   │   │   │                                  archive_month: String. Legacy task_archive
    │   │   │   │                                  function deleted.
    │   │   │   ├── phase.rs                     ─ MOD: artifact_for(Committed, _) → None;
    │   │   │   │                                  task_verify gains seed substitution.
    │   │   │   ├── verify_seed.rs               ─ NEW: render_seeded_verify(SeedInputs).
    │   │   │   └── mod.rs                       ─ MOD: pub mod commit; pub mod verify_seed;
    │   │   │                                       pub use commit::*; rename re-export
    │   │   │                                       task_archive → task_archive_move.
    │   │   ├── workspace/
    │   │   │   ├── journal.rs                   ─ MOD: JournalEntry.start_head + .base_branch;
    │   │   │   │                                  render_entry emits the new lines.
    │   │   │   └── record.rs                    ─ MOD: collect_commits_for_task accepts
    │   │   │                                      start_head; RecordTaskOptions gains
    │   │   │                                      start_head; archive_path → task_dir;
    │   │   │                                      archived_at → recorded_at.
    │   │   └── (no other agent modules touched)
    │   └── context/projection.rs                ─ MOD: PhaseFilter::Commit (paths only,
    │                                              no file bodies, per C-17).
    └── lib.rs                                   ─ ADD re-exports: ark_archive, ArchiveOptions,
                                                   ArchiveSummary, task_commit, TaskCommitOptions,
                                                   TaskCommitSummary, task_archive_move,
                                                   TaskArchiveMoveOptions, TaskArchiveMoveSummary.
                                                   REMOVE re-export: task_archive (deleted).

templates/
├── ark/
│   ├── templates/
│   │   ├── VERIFY.md                            ─ REWRITE: six-section living document.
│   │   └── (PRD/PLAN/REVIEW/SPEC unchanged)
│   ├── workflow.md                              ─ MOD: per G-8.
│   └── (config.toml unchanged)
├── claude/commands/ark/
│   ├── commit.md                                ─ NEW (documents staged-work precondition).
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

AGENTS.md                                        ─ MOD: drop /ark:archive row; add /ark:commit row.
```

**Module coupling.** Same as `01_PLAN`'s. `commands/archive.rs` → `commands::agent::task::archive::task_archive_move`. `commands/agent/task/commit.rs` → `commands::agent::state`, `commands::agent::spec::{extract,register}`, `commands::agent::workspace::record::record_task`, `io::{git::run_git, PathExt}`, `layout::Layout`. `task::archive` after refactor does **not** import `super::workspace::*` or `super::spec::*`. `workspace/*` MUST NOT import `super::task`.

**Call graph: `/ark:commit` → `task_commit` (revised step ordering with R-101/R-102/R-103/R-105 fixes).**

```text
slash command /ark:commit
  ├── (agent generates message from staged diff if -m absent; shows for confirm)
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
              │     diff_cached ← run_git(&["diff","--cached","--quiet"], task_cwd)
              │     if diff_cached.exit_code == 0:
              │         return Error::NothingStaged{ slug }
              │     // user has at least one staged file; ark will add only ark-managed files
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
              ├── // SPEC snapshot for rollback (deep tier only)
              ├── if tier == Deep:
              │     prev_spec_bytes ← if specs/features/<slug>/SPEC.md exists:
              │         Some(read_bytes(&specs_path))
              │       else: None
              │     prev_features_index_bytes ← read_bytes(&features_index_path)
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
              ├── // Journal append (skipped under --no-commit)
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
              │     // Build the explicit ark_files list
              │     ark_files ← vec![
              │         journal_path.unwrap(),
              │         layout.workspace_index(&dev),
              │         task_dir.join("task.toml"),
              │     ]
              │     if tier == Deep:
              │         ark_files.push(specs_path)
              │         ark_files.push(features_index_path)
              │
              │     // Stage only ark-managed files
              │     run_git(&["add"] + &ark_files, task_cwd)?
              │
              │     // Commit
              │     out ← run_git(&["commit","-m",&opts.message.unwrap()], task_cwd)?
              │     if !out.is_success():
              │         // ROLLBACK
              │         prev_toml.save(&task_dir)?
              │         truncate_file(journal_path.unwrap(), pre_append_len)?
              │         restore_index(layout, dev, prev_index_bytes)?
              │         if tier == Deep:
              │             match prev_spec_bytes {
              │                 None    → fs::remove_file(specs_path)?,
              │                 Some(b) → write_bytes(specs_path, b)?,
              │             }
              │             write_bytes(features_index_path, prev_features_index_bytes)?
              │         // Targeted unstage: only the files ark added
              │         run_git(&["reset", "HEAD"] + &ark_files, task_cwd)?
              │         return Error::GitCommitFailed{ stderr: out.stderr }
              │
              │     // Success path: no post-commit mutations.
              │     head_sha ← run_git(&["rev-parse","HEAD"], task_cwd)?.stdout.trim()
              │
              └── return TaskCommitSummary {
                    slug, tier,
                    head_sha: if no_commit { None } else { Some(head_sha) },
                    journal_path, session_number, deep_spec_promoted, pending_verify,
                  }
                  // NO committed_head_dirty field. Working tree is clean post-success.
```

**Call graphs for `ark archive` → `task_archive_move` and `task_archive_move`** are unchanged from `01_PLAN`. Same side-effect-free move + state cleanup; archive_month derived from `committed_at`.

**Call graph: `task_verify` (seeded VERIFY.md)** unchanged from `01_PLAN`.

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
    pub committed_at: Option<DateTime<Utc>>,    // NEW (kept from 01_PLAN)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub worktree_path: Option<std::path::PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_head: Option<String>,             // NEW (kept from 01_PLAN)
    // committed_head: REMOVED. The closing commit's SHA is recoverable from
    // `git log -n 1 -- <journal-path>`; persisting it here would create a
    // third durability class. See R-105 in 01_REVIEW.
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
    /// HEAD SHA after the commit, or None when --no-commit.
    /// Display-only. Not persisted in task.toml.
    pub head_sha: Option<String>,
    pub journal_path: Option<PathBuf>,
    pub session_number: Option<u32>,
    pub deep_spec_promoted: bool,
    pub pending_verify: VerifyPendingCounts,
    // committed_head_dirty: REMOVED. No dirty file post-success.
}

#[derive(Debug, Clone, Default)]
pub struct VerifyPendingCounts {
    pub items: u32,
    pub findings: u32,
}

impl fmt::Display for TaskCommitSummary { /* one-line */ }

pub fn task_commit(opts: TaskCommitOptions) -> Result<TaskCommitSummary>;

/// Scoped rollback helper for `task_commit`. RAII: snapshots accumulate as
/// destructive mutations succeed; on `Drop` (any error path before
/// `commit()` is called), every accumulated snapshot is restored in
/// reverse-of-recording order. On the success path, `commit()` disarms the
/// guard so its drop is a no-op.
///
/// Design note (per R-203): a single `RollbackGuard` covers every mutation
/// point in `task_commit` — `spec_extract`, `spec_register`, journal append,
/// workspace index rerender, `task.toml` save, `git add`. Earlier drafts
/// only handled `git commit` failure, which left partial SPEC/INDEX writes
/// uncovered.
pub(crate) struct RollbackGuard {
    armed: bool,
    layout: Layout,
    task_dir: PathBuf,
    task_cwd: PathBuf,
    /// Pre-mutation `task.toml` (always snapshotted before save).
    prev_toml: Option<TaskToml>,
    /// Pre-append journal length + path. `Some` once the journal append step
    /// has prepared its target.
    journal: Option<JournalSnapshot>,
    /// Pre-rerender bytes of `<dev>/index.md`. `Some` once index rerender is
    /// scheduled.
    workspace_index: Option<WorkspaceIndexSnapshot>,
    /// Deep-tier SPEC snapshot. Outer `Option` = "not deep / not snapshotted yet";
    /// inner `Option` = "file was absent before extract".
    spec_file: Option<SpecFileSnapshot>,
    /// Deep-tier `specs/features/INDEX.md` byte snapshot.
    features_index: Option<FeaturesIndexSnapshot>,
    /// Files that were `git add`ed by this `task_commit` invocation. On
    /// rollback, runs `git reset HEAD <ark_files>` (targeted) instead of
    /// `git reset` (which would also unstage user pre-existing entries).
    ark_files: Vec<PathBuf>,
}

struct JournalSnapshot { path: PathBuf, pre_append_len: u64 }
struct WorkspaceIndexSnapshot { path: PathBuf, prev_bytes: Vec<u8> }
struct SpecFileSnapshot { path: PathBuf, prev_bytes: Option<Vec<u8>> } // None = file was absent
struct FeaturesIndexSnapshot { path: PathBuf, prev_bytes: Vec<u8> }

impl RollbackGuard {
    pub(crate) fn new(layout: Layout, task_dir: PathBuf, task_cwd: PathBuf) -> Self;

    pub(crate) fn snapshot_toml(&mut self, toml: TaskToml);
    pub(crate) fn snapshot_journal(&mut self, path: PathBuf, pre_append_len: u64);
    pub(crate) fn snapshot_workspace_index(&mut self, path: PathBuf, prev_bytes: Vec<u8>);
    pub(crate) fn snapshot_spec(&mut self, path: PathBuf, prev_bytes: Option<Vec<u8>>);
    pub(crate) fn snapshot_features_index(&mut self, path: PathBuf, prev_bytes: Vec<u8>);
    pub(crate) fn record_staged(&mut self, ark_files: Vec<PathBuf>);

    /// Disarm the guard. Call on the success path; future drops become no-ops.
    pub(crate) fn commit(mut self) { self.armed = false; }
}

impl Drop for RollbackGuard {
    fn drop(&mut self) {
        if !self.armed { return; }
        // Best-effort restore in reverse-of-recording order. Errors logged to
        // stderr; do not propagate (the user already has the original error).
        // Order: targeted unstage → workspace index → journal truncate → SPEC
        // file → features INDEX → task.toml. Reverse-order ensures each restore
        // sees the correct prior state.
    }
}
```

```rust
// crates/ark-core/src/commands/agent/task/archive.rs (renamed function)

#[derive(Debug, Clone)]
pub struct TaskArchiveMoveOptions {
    pub project_root: PathBuf,
    pub slug: String,
    /// YYYY-MM bucket. Caller (`ark_archive` or the hidden `ark agent task archive`)
    /// derives from `task.toml.committed_at`.
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
// Legacy task_archive function: DELETED (replaced by task_archive_move + task_commit splits).
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
    pub start_head: Option<String>,           // NEW
    pub task_dir: PathBuf,                     // renamed from archive_path
    pub recorded_at: DateTime<Utc>,            // renamed from archived_at
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
    pub start_head: Option<String>,    // NEW
    pub base_branch: Option<String>,   // NEW
    pub summary: String,
    pub commits: Vec<JournalCommit>,
    pub next_steps: Vec<String>,
}
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

#[error("task `{slug}` cannot be committed without staged work; run `git add <files>` first")]
NothingStaged { slug: String },                  // RENAMED from NothingToCommit

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
    Archive(ArchiveCliArgs),    // NEW (top-level bulk archive)
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
    Commit(TaskCommitCliArgs),                  // NEW
    Archive(TaskArchiveMoveCliArgs),            // REPLACES the original Archive(TaskSlugArgs)
}

#[derive(clap::Args)]
struct TaskCommitCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(short = 'm', long = "message")] message: Option<String>,
    #[arg(long = "no-commit", default_value_t = false)] no_commit: bool,
}

#[derive(clap::Args)]
struct TaskArchiveMoveCliArgs {
    #[command(flatten)] target: TargetArgs,
    /// YYYY-MM bucket. Defaults to the task's `committed_at` month.
    #[arg(long)] month: Option<String>,
}
```

[**Constraints**]

- **C-1 (one-way coupling):** Same as `01_PLAN`. `commands/archive.rs` → `task::archive::task_archive_move`. `task::commit` → `agent::workspace::record::record_task` + `agent::spec::{extract,register}`. Reverse forbidden in all cases.

- **C-2 (process spawn locality):** All git invocations under new modules through `io::git::run_git`. `Command::new` MUST NOT appear.

- **C-3 (commit message authorship + staged-diff source):** `task_commit` does not generate commit messages. When `opts.no_commit == false`, `opts.message` MUST be `Some(_)` (else `Error::CommitMessageRequired`). The slash command's body generates the message from the **staged diff** (`git diff --cached`) plus recent `git log` style; this is the contract the PRD describes. Generating from `git diff` (working-tree) is not authorized.

- **C-4 (revised per R-203 — atomic-commit protocol with scoped `RollbackGuard`):** `task_commit`'s sequence is: precondition → VERIFY gate → snapshot SPEC files (deep) → SPEC extract+register (deep) → render+append journal → save next_toml → explicit `git add <ark_files...>` → `git commit`. **All ark-managed files (work + journal + workspace index + task.toml + (deep) SPEC + features INDEX) land in one commit.** **No `git add -A`. No post-commit patch step. No post-commit `committed_head` write.** Rollback is implemented via the `RollbackGuard` RAII helper (see Data Structure): each destructive step records its pre-mutation snapshot into the guard before performing the mutation, so `Err` from *any* point — `spec_extract`, `spec_register`, `append_text` for journal, `index::rerender`, `task.toml.save`, `git add`, or `git commit` — drops the guard, which restores every snapshot accumulated so far. The user's pre-existing index entries for non-ark files are not touched (targeted `git reset HEAD <ark_files>` is part of the guard's drop). On the success path, `guard.commit()` (the disarm method) makes the drop a no-op. Errors during rollback are logged to stderr; the original error is the one returned to the caller.

- **C-5 (`start_head` capture):** Same as `01_PLAN`.

- **C-6 (state-file reconcile semantics):** Same as `01_PLAN`.

- **C-7 (VERIFY parser):** Same as `01_PLAN`.

- **C-8 (slash-command lockstep):** Same as `01_PLAN`.

- **C-9 (template marker substitution):** Same as `01_PLAN`. Markers `{{PROJECT_SPEC_COMPLIANCE}}`, `{{RELATED_FEATURE_COMPLIANCE}}`, `{{PRD_CONSTRAINTS}}`, `{{PLAN_FIDELITY}}`. Missing → `Error::TemplateMarkerMissing { marker, path }`.

- **C-10 (path/io discipline):** All FS access through `io::PathExt`. All `.ark/`-relative paths through `Layout` helpers.

- **C-11 (CLI hidden-vs-visible split):** `ark archive` (top-level) is **visible**. `ark agent task commit` and `ark agent task archive` (the reused verb name from `ark-agent-namespace` SPEC, now plumbed to `task_archive_move`) are **hidden**.

- **C-12 (migration idempotency):** Same as `01_PLAN`.

- **C-13 (auto-record absence on `--no-commit`):** Same as `01_PLAN`.

- **C-14 (deep-tier `--no-commit` SPEC extraction):** Same as `01_PLAN`.

- **C-15 (CHANGELOG entry on existing SPEC):** Same as `01_PLAN`. The `prev_spec_bytes` snapshot in step 4 covers the case where `spec_extract` fails partway (e.g. CHANGELOG row appended but other write fails) — rollback restores the original.

- **C-16 (manual `/ark:record` path unchanged):** Same as `01_PLAN`.

- **C-17 (commit projection is body-free):** Same as `01_PLAN`. Per `ark-context` SPEC G-4/G-5/G-7.

- **C-18 (archive helper is side-effect-free):** Same as `01_PLAN`. `task_archive_move` performs only directory rename + `task.toml` phase update + state-file cleanup. Tests `archive_no_spec_promotion`, `archive_no_journal_write` enforce.

- **C-19 (legal-transition table parity):** Same as `01_PLAN`.

- **C-20 (slash-command parity test):** Same as `01_PLAN`.

- **C-21 (`committed_at` required for `Committed` phase):** Same as `01_PLAN`.

- **C-22 (concurrent commit unsupported):** Same as `01_PLAN`. NG-12.

- **C-23 (revised per R-204: no Ark-managed file dirty post-commit):** After a successful `task_commit` with `!opts.no_commit`, none of the Ark-managed files — `[journal-N.md, workspace index, task.toml, (deep) specs/features/<slug>/SPEC.md, (deep) specs/features/INDEX.md]` — appear in `git status --porcelain` from the task's cwd. **The whole working tree is not guaranteed clean** — user's pre-existing unstaged files outside Ark's purview are intentionally untouched (the staged-only workflow protects them, per R-102/R-103/TR-2). Validated by `V-UT-17` (revised) and `V-IT-1`/`V-IT-2`/`V-IT-3` end-to-end checks, which assert the precise predicate "no Ark-managed file in `git status --porcelain`" rather than "porcelain output empty."

- **C-24 (NEW: explicit-staging set):** The `ark_files` list staged by `task_commit` is exactly `[journal_path, workspace_index_path, task_toml_path]` for non-deep tiers, plus `[spec_path, features_index_path]` for deep tier. The list is constructed in `commit.rs` and exposed for testing via a `pub(crate)` helper `compute_ark_files_for_commit(...)`. Validated by `V-UT-26` (new test asserts `ark_files` excludes user files).

- **C-25 (NEW: targeted unstage on rollback):** `git reset HEAD <ark_files>` is run instead of bare `git reset`. The user's pre-existing index entries for non-ark files survive a rollback. Validated by `V-UT-16` (revised) and new `V-UT-29`.

---

## Runtime `runtime logic`

[**Main Flow**]

1. `/ark:design --deep <title>` scaffolds. `task new` captures `start_head`.
2. `/ark:plan` → `/ark:review` ↔ `/ark:plan` → `/ark:execute` unchanged.
3. `/ark:verify` seeds `VERIFY.md`. Implementer fills.
4. **User stages their work:** `git add <files>` to populate the index.
5. `/ark:commit -m "<msg>"`:
   a. Load `prev_toml`. Verify phase.
   b. Verify staging area is non-empty (else `Error::NothingStaged`).
   c. Parse VERIFY.md. Refuse on PENDING (deep) or warn (standard).
   d. **Deep tier:** snapshot `prev_spec_bytes` + `prev_features_index_bytes`.
   e. **Deep tier:** extract SPEC, register in INDEX.
   f. Render journal entry. Append. Re-render workspace index. Snapshot `pre_append_len` + `prev_index_bytes`.
   g. Save `next_toml` (phase=Committed, committed_at=now).
   h. **Stage explicit ark files** via `git add <journal> <workspace-index> <task.toml> [<spec> <features-index>]`.
   i. `git commit -m "<msg>"`.
   j. **On failure:** rollback (restore prev_toml, truncate journal, restore workspace index, [deep] restore spec + features index, `git reset HEAD <ark_files>`). Return `GitCommitFailed`.
   k. **On success:** read `head_sha` for display. **No disk mutations.** Return summary. **Working tree is clean.**
6. Time passes. Multiple committed tasks accumulate.
7. `ark archive` (manager): enumerates committed tasks, derives `archive_month` per task from `committed_at`, calls `task_archive_move` per slug. **No SPEC promotion. No journal recording.**

[**Failure Flow**]

The following uniform rule (per R-203 + the `RollbackGuard` design) governs every error path: any `Err` returned between guard construction and `guard.commit()` triggers the guard's `Drop`, which restores every snapshot accumulated so far. Each step below names which snapshot exists at the time the error fires.

1. `task new` git rev-parse fails (unborn HEAD): `start_head = None`. Task proceeds; later `task_commit` falls back. (Pre-`task_commit`; no guard.)
2. `task_commit` precondition errors (`IllegalPhaseTransition`, `NothingStaged`, `VerifyIncomplete`): error returned before the guard is constructed. Task state unchanged.
3. `task_commit` step 4 (deep-tier SPEC snapshot read fails): error returned before extract runs. Guard exists but no SPEC mutations recorded yet → drop is a no-op. Task state unchanged.
4. `task_commit` step 5 (`spec_extract` fails on deep, possibly mid-write): SPEC snapshot is recorded into the guard before extract runs. Drop restores SPEC file (delete if was absent, restore bytes if present). `spec_register` not yet attempted; features INDEX untouched. Task state unchanged.
5. `task_commit` step 5b (`spec_register` fails on deep, possibly after modifying features INDEX): both SPEC and features INDEX snapshots are recorded into the guard. Drop restores both. Task state unchanged.
6. `task_commit` step 6 (journal append fails or `index::rerender` fails): journal-append-len snapshot exists (taken before append); workspace-index snapshot exists (taken before rerender). Drop restores both, plus the SPEC files from step 5. Task state unchanged.
7. `task_commit` step 7 (`task.toml.save` fails): toml snapshot exists (taken before save). Drop restores all of: toml, journal length, workspace index, (deep) SPEC, (deep) features INDEX. Task state unchanged.
8. `task_commit` step 8 (`git add` fails): all snapshots above exist; `ark_files` recorded into the guard but `git reset HEAD <ark_files>` is also part of the drop. Drop restores all on-disk state and unstages anything `git add` may have partially staged. Task state unchanged.
9. `task_commit` step 8 (`git commit` fails — pre-commit hook rejects, etc.): all snapshots exist + `ark_files` recorded. Drop restores everything and runs `git reset HEAD <ark_files>` (targeted, preserving the user's pre-existing index entries). Hard error `GitCommitFailed`. Re-invoke after fixing the hook is safe and produces the same final state.
10. `task_commit` step 9 (`git rev-parse HEAD` for display fails post-commit, after `guard.commit()`): closing commit landed correctly; only the display value is missing. Stderr emits `committed (HEAD readback failed; check git log manually)`. Phase is `Committed`. State machine invariants satisfied. (No rollback: the disarmed guard's drop is a no-op.)
11. `ark_archive` per-slug failure: continues; failures listed; non-zero exit.

[**State Transitions**]

- `Phase::Verify → Phase::Committed` when `task_commit` runs successfully on standard or deep.
- `Phase::Execute → Phase::Committed` when `task_commit` runs successfully on quick.
- `Phase::Committed → Phase::Archived` when `task_archive_move` runs (invoked by `ark archive` or by `ark agent task archive`).
- `Phase::Archived` remains terminal.

---

## Implementation `split task into phases`

[**Phase 1 — State machine + start_head capture**]

1. Add `Phase::Committed` to `state.rs`. Update `can_transition` per G-1. Update `archived_is_terminal` test. Add `*_committed_is_legal_destination` per tier.
2. Add `start_head: Option<String>` and `committed_at: Option<DateTime<Utc>>` to `TaskToml`. **Do not add `committed_head`.** Update `task_toml_loads_without_worktree_fields` test.
3. In `task_new::build_task_toml`, capture `start_head` via `run_git(&["rev-parse","HEAD"], opts.project_root)`.
4. Unit tests: `task_new_captures_start_head`, `task_new_with_unborn_head_records_none`.
5. Verify CI passes; no behavior change yet.

[**Phase 2 — `task_commit` with rollback + explicit staging**]

1. Create `crates/ark-core/src/commands/agent/task/commit.rs`. Implement `task_commit` with the 10-step sequence and full rollback set (SPEC files included on deep tier). Private helpers: `parse_verify_md` (C-7), `render_and_append_journal`, `truncate_file_to_len`, `restore_index_bytes`, `snapshot_spec_files`, `restore_spec_files`, `compute_ark_files_for_commit`.
2. Add `pub mod commit; pub use commit::*;` to `task/mod.rs`. Re-exports to `lib.rs`.
3. Add `RecordTaskOptions.start_head`, rename `archive_path → task_dir`, `archived_at → recorded_at`. Thread `start_head` through.
4. Add `JournalEntry.start_head`, `.base_branch`. Update `render_entry` golden tests.
5. Add error variants: `NothingStaged` (renamed from `NothingToCommit`), `VerifyIncomplete`, `GitCommitFailed`, `CommitMessageRequired`, `CommittedAtMissing`, `TemplateMarkerMissing`.
6. Wire `Commit(TaskCommitCliArgs)` into `agent_cli.rs`'s `TaskCommand` + dispatcher.
7. Integration tests:
   - `task_commit_standard_with_staged_work_succeeds`
   - `task_commit_quick_with_staged_work_succeeds`
   - `task_commit_deep_with_staged_work_extracts_spec_and_succeeds`
   - `task_commit_no_commit_extracts_spec_only`
   - `task_commit_with_empty_stage_errors_nothing_staged`
   - `task_commit_with_pending_verify_deep_errors`
   - `task_commit_with_pending_verify_standard_warns_and_proceeds`
   - **`task_commit_rollback_on_pre_commit_hook_failure`** — restores task.toml, journal, workspace index, SPEC, features INDEX; targeted unstage preserves user's pre-existing index entries
   - **`task_commit_rollback_restores_spec_files`** (R-101 fix) — pre-existing SPEC overwritten by extract; rollback restores byte-for-byte
   - **`task_commit_rollback_creates_no_spec_when_absent_before`** (R-101 fix) — pre-extract SPEC absent; rollback deletes the newly-created SPEC
   - **`task_commit_rollback_preserves_user_staging_intent`** (R-103 fix) — user pre-stages `foo.txt`, `task_commit` rolls back, `foo.txt` still in index
   - **`task_commit_post_success_working_tree_is_clean`** (R-105 fix / C-23) — after successful commit, `git status --porcelain` returns empty
8. Verify CI passes.

[**Phase 3 — `task_archive_move` and `ark archive`**]

1. Rename `task_archive` → `task_archive_move` in `task/archive.rs`. Strip side effects per `01_PLAN`. New signature with `archive_month`.
2. Update precondition: `check_transition` legal only from `Committed`.
3. Migrate existing `archive.rs` tests; tests asserting bundled SPEC/journal behavior move to `task_commit`'s suite.
4. New tests: `archive_no_spec_promotion`, `archive_no_journal_write`.
5. Create `crates/ark-core/src/commands/archive.rs` with `ark_archive`.
6. Add `Command::Archive(ArchiveCliArgs)` to top-level CLI.
7. **Replace** `Archive(TaskSlugArgs)` in `agent_cli.rs`'s `TaskCommand` with `Archive(TaskArchiveMoveCliArgs)` (`--slug`, optional `--month`); dispatcher reads `committed_at` for default `--month`.
8. Integration tests for `ark archive` per `01_PLAN`'s list, plus:
   - `agent_task_archive_subcommand_uses_committed_at_when_month_omitted` (R-106 fix)
9. Update `lib.rs` re-exports.
10. Verify CI passes.

[**Phase 4 — VERIFY template, seed protocol, migration**]

Same as `01_PLAN`'s Phase 4. Unchanged.

[**Phase 5 — Slash command surface (Claude / Codex / OpenCode)**]

1. Create `commit.md` (Claude + OpenCode), `ark-commit/SKILL.md` (Codex). **Body documents the staged-work precondition** explicitly. Wrap-up notes the working tree is clean post-commit.
2. Update `design.md`, `quick.md`, `record.md` to point at `/ark:commit`.
3. Add `PhaseFilter::Commit` to `commands/context/projection.rs`. **Body-free** per C-17.
4. Verify CI passes; lockstep diff between platforms shows identical bodies.

[**Phase 6 — Workflow doc + AGENTS.md + cleanup**]

1. Update `templates/ark/workflow.md` per G-8.
2. Update `.ark/workflow.md` in lockstep.
3. Update `AGENTS.md`: drop `/ark:archive` row; add `/ark:commit` row.
4. Sweep tests: any test calling `task_archive` migrates.
5. Update `concurrency_tests.rs`.
6. `cargo test --all-targets`. `cargo clippy --all-targets`. `cargo fmt --check`.
7. End-to-end smoke: design → plan → review → execute → verify → stage work → commit → bulk-archive. **Assert post-commit working tree is clean (no `committed_head` dirty file). Assert closing commit contains all 5 ark-managed files (work + journal + workspace index + task.toml + SPEC + features INDEX for deep). Assert bulk archive does not modify SPEC or journal.**

---

## Trade-offs `ask reviewer for advice`

- **T-1: One durability class (chosen).** Per R-105 + TR-1 resolution. Closure artifacts are committed atomically (in the closing commit) or rollback-covered. **No post-commit dirty residue.** Drop `committed_head`. **Advantages:** clean atomicity model; the working tree state after a successful `/ark:commit` is the same as if the user had run `git commit` themselves; bulk archive has no special preservation responsibility. **Disadvantages:** the closing SHA is not in `task.toml` directly; readers run `git log -n 1 -- <journal-path>` to recover it (cheap, but one extra step). **Rejected alternatives:** keep `committed_head` as a follow-up commit (two commits per task, rejected as inelegant); keep `committed_head` dirty (R-105's complaint); keep `committed_head` and document user-must-commit (fragile).

- **T-2: Staged-only workflow (chosen).** Per R-102 + R-103 + TR-2 resolution. User stages work; ark stages only ark-managed files. Targeted unstage on rollback. **Advantages:** matches PRD language; protects unrelated edits; gives the agent's generated commit message a stable input (the staged diff). **Disadvantages:** users who don't realize they need to `git add` first hit `Error::NothingStaged` instead of an auto-add. **Mitigation:** the slash command's body explains the precondition; the error message tells them what to do. **Rejected alternatives:** `git add -A` (breaks unrelated edits); auto-`git add` of work files only (impossible — ark doesn't know what counts as "work").

- **T-3: SPEC files in rollback set (chosen).** Per R-101 fix. SPEC promotion still runs pre-commit so SPEC files land in the closing commit, but their pre-extract bytes are snapshotted and restored on rollback. **Advantages:** SPEC files are part of the atomic commit (one commit captures everything); failure leaves no partial SPEC residue. **Disadvantages:** rollback set has more state. **Rejected alternative:** move SPEC extract to post-commit (codex's R-101 first suggestion). Would create a third durability class — exactly what TR-1 demanded we avoid.

- **T-4: Tier-conditional VERIFY gate.** Unchanged from `01_PLAN`.

- **T-5: Slash command generates message from staged diff.** Per C-3 (clarified this iteration). Unchanged in substance from `01_PLAN`; clarified that the source is `git diff --cached`, not the working tree.

- **T-6: Concurrent `/ark:commit` declared unsupported.** Unchanged from `01_PLAN`.

- **T-7 (NEW): `ark agent task archive-move` (hidden helper) preserved.** Per R-106. **Advantages:** maintains the `ark-agent-namespace` SPEC's verb set; gives maintainers a one-off recovery surface. **Disadvantages:** keeps an internal CLI that most users won't invoke. **Rejected alternative:** delete entirely + revise `ark-agent-namespace` SPEC. More surface change; loses debugging affordance.

- **T-8 (NEW): PRD edit landed alongside this PLAN iteration.** Per R-104 fix. Original PRD wording asked for inline `commit_range = "<start_head>..<HEAD>"`, which Git's content-addressed object model makes impossible. PRD Outcome item 7 has been rewritten in this iteration. **Advantages:** PRD ↔ PLAN consistency; the contract change is explicit. **Disadvantages:** PRD was supposed to be locked in iteration 00; an iteration-02 edit breaks that norm. **Mitigation:** this is a deep-tier task, the original PRD wording was provably wrong (CRITICAL R-001), and the fix is the smallest possible change ("inline SHA" → "recoverable from git log"). The Log section + Trade-off section flag the edit clearly.

---

## Validation `test design`

[**Unit Tests**]

- **V-UT-1 .. V-UT-15:** unchanged from `01_PLAN` (Phase enum, transition table, start_head capture, task.toml round-trip, task_commit happy paths, journal field rendering).

- **V-UT-16 (revised):** `task_commit` rollback on `git commit` failure: pre-commit hook rejects → prev_toml restored, journal truncated, workspace index restored, **SPEC file restored to pre-extract state, features INDEX restored to pre-register state** (deep tier), `git reset HEAD <ark_files>` runs (targeted, not bulk). Re-invoke after hook fix succeeds.

- **V-UT-17 (revised per C-23 / R-204):** After a successful `task_commit`, no Ark-managed file appears in `git status --porcelain` from the task's cwd (precise predicate: each of `[journal-N.md, workspace index, task.toml, (deep) SPEC, (deep) features INDEX]` is absent from the porcelain output). User's pre-existing unstaged files outside Ark's purview MAY still appear and are not Ark's concern. The closing commit's tree contains: work + journal-N.md + workspace index + task.toml + (deep) SPEC + features INDEX.

- **V-UT-18, V-UT-19, V-UT-20:** unchanged (`parse_verify_md`).

- **V-UT-21, V-UT-22, V-UT-23:** unchanged (`render_seeded_verify`).

- **V-UT-24:** unchanged (`task_archive_move` accepts only `phase = Committed`).

- **V-UT-25:** unchanged (`task_archive_move` does not invoke `spec_extract`/`spec_register`/`record_task`).

- **V-UT-26 (NEW per C-24):** `compute_ark_files_for_commit` on a quick task returns `[journal, workspace_index, task.toml]` (3 files). On a standard task: same. On a deep task: `[journal, workspace_index, task.toml, spec, features_index]` (5 files). User work files (e.g. `src/foo.rs`) are NOT in the returned list.

- **V-UT-27, V-UT-28, V-UT-29 (renumbered + new):**
  - V-UT-27: `ark_archive` enumerates only `phase = Committed` tasks.
  - V-UT-28: `ark_archive` derives `archive_month` from `committed_at`, NOT `Utc::now()`.
  - V-UT-29 (NEW): `ark_archive --month YYYY-MM` filters out other months; `ark_archive --dry-run` does not move any directory.

- **V-UT-30 (NEW per R-101):** `task_commit` rollback on commit failure with deep tier and pre-existing SPEC: assert `specs/features/<slug>/SPEC.md` byte-equals its pre-extract content after rollback (use a SPEC that already had a CHANGELOG row).

- **V-UT-31 (NEW per R-101):** `task_commit` rollback on commit failure with deep tier and absent SPEC: assert `specs/features/<slug>/SPEC.md` does NOT exist after rollback (was created by spec_extract, must be deleted on rollback).

- **V-UT-32 (NEW per R-101):** `task_commit` rollback on commit failure with deep tier: assert `specs/features/INDEX.md` byte-equals its pre-register content after rollback.

- **V-UT-33 (NEW per R-103):** `task_commit_rollback_preserves_user_staging_intent`: setup user stages `foo.txt` via `git add foo.txt`, has unstaged `bar.txt`. Pre-commit hook rejects task_commit. After rollback: `git diff --cached --name-only` includes `foo.txt`; `git status --porcelain` shows `bar.txt` as unstaged-modified; ark files (journal, task.toml, SPEC, features INDEX) are NOT in the index.

- **V-UT-34 (NEW per R-105):** Removed `task.toml.committed_head` field round-trip test (negative assertion: `TaskToml` does not have a `committed_head` field). Defensive — catches accidental re-introduction.

- **V-UT-35 (NEW per R-106):** `agent_task_archive_uses_committed_at_when_month_omitted`: hidden `ark agent task archive --slug foo` (no `--month`) reads `task.toml.committed_at`, formats YYYY-MM, calls `task_archive_move` with that month. Same task with `--month 2026-12` overrides. (R-205: subcommand name is `archive`; underlying function is `task_archive_move`.)

- **V-UT-36 (NEW per R-201):** `closing_sha_recoverable_via_slug_anchored_log` — after a successful `task_commit` with slug `foo`, run `git log -S '**Slug**: foo' --diff-filter=A --format=%H -- <journal-path> -n 1` and assert it returns the closing commit's SHA.

- **V-UT-37 (NEW per R-201):** `closing_sha_recoverable_after_later_manual_record` — `task_commit` slug `foo`; later `workspace_record` adds a manual entry to the same journal file (committed by the user); rerun the slug-anchored `-S` lookup; assert it still returns `foo`'s closing commit SHA, NOT the manual entry's commit.

- **V-UT-38 (NEW per R-201):** `closing_sha_recoverable_after_later_task_commit` — `task_commit` slug `foo` (closes commit C1); `task_commit` slug `bar` (closes commit C2 on the same `journal-N.md` before rotation); slug-anchored `-S 'foo'` lookup returns C1; slug-anchored `-S 'bar'` lookup returns C2.

- **V-UT-39 (NEW per R-203):** `task_commit_rollback_on_spec_extract_failure_after_partial_write` — inject a failure into `spec_extract` after it has begun writing the SPEC file (e.g. by mocking the layout's spec path to be writable but `spec_register` to fail). Assert `RollbackGuard` drop restores SPEC bytes (or removes the file).

- **V-UT-40 (NEW per R-203):** `task_commit_rollback_on_spec_register_failure_after_index_modification` — inject failure into `spec_register` after it has modified `features/INDEX.md`. Assert features INDEX restored byte-for-byte.

- **V-UT-41 (NEW per R-203):** `task_commit_rollback_on_journal_append_failure` — inject failure into `append_text`. Assert journal file unchanged (no partial append visible) and SPEC files (deep) already restored.

- **V-UT-42 (NEW per R-203):** `task_commit_rollback_on_task_toml_save_failure` — inject failure into `next_toml.save`. Assert journal truncated, workspace index restored, (deep) SPEC files restored, task.toml file (untouched) still has the prev_toml content.

[**Integration Tests**]

- **V-IT-1 (revised per C-23 / R-105 / R-201 / R-204):** End-to-end deep-tier flow on a `tempdir` git repo: design → plan → review → execute → verify → **stage user work** → commit. Assert:
  - The closing commit's tree contains: user work files, journal-N.md, workspace index, task.toml (with phase=Committed, committed_at=Some), `specs/features/<slug>/SPEC.md`, `specs/features/INDEX.md`.
  - Post-commit, `git status --porcelain` from task's cwd contains **no Ark-managed file** (precise predicate per R-204; user's pre-existing unstaged files allowed).
  - `task.toml` does NOT contain a `committed_head` field on disk.
  - **Slug-anchored recovery** (per R-201): `git log -S '**Slug**: <slug>' --diff-filter=A --format=%H -- <journal-path> -n 1` returns the closing commit's SHA.
  - Then `ark archive` moves the dir to `archive/YYYY-MM/<slug>/` with no further Ark-managed mutations.

- **V-IT-2 (revised):** End-to-end standard-tier flow. Same shape as V-IT-1 minus SPEC/features INDEX in the closing commit. Post-commit: no Ark-managed file dirty (R-204 predicate). Slug-anchored recovery works (R-201 predicate).

- **V-IT-3 (revised):** End-to-end quick-tier flow. Same predicate set.

- **V-IT-4..V-IT-12:** unchanged from `01_PLAN`.

- **V-IT-13 (NEW per R-101):** End-to-end deep-tier flow with a failing pre-commit hook: assert all five rollback files (task.toml, journal, workspace index, SPEC, features INDEX) are byte-restored after the failure.

- **V-IT-14 (NEW per R-103):** End-to-end with user pre-staged file + failing pre-commit hook: assert the pre-staged file remains staged in the index after rollback.

- **V-IT-15 (NEW per R-201):** End-to-end deep-tier task `foo`; **then** a manual `/ark:record` entry committed on the same journal-N.md; **then** end-to-end task `bar` committed on the same journal-N.md; assert slug-anchored `-S` lookup returns `foo`'s SHA for `foo`, `bar`'s SHA for `bar`.

- **V-IT-16 (NEW per R-201):** End-to-end task `foo` followed by manual record; assert the lookup remains correct across the manual entry's commit.

[**Failure / Robustness Validation**]

- **V-F-1..V-F-6:** unchanged from `01_PLAN`, except V-F-1 now includes the SPEC + features INDEX restoration.

[**Edge Case Validation**]

- **V-E-1..V-E-5, V-E-7, V-E-8:** unchanged from `01_PLAN`.

- **V-E-9 (NEW):** `task_commit --no-commit` on deep tier extracts SPEC and writes task.toml (phase=Committed). Assert: SPEC files exist on disk; task.toml is on disk; **but no git commit landed**; the SPEC files + task.toml may show in `git status` (the user explicitly opted into this state via `--no-commit`). Stderr emits the manual-commit reminder.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-UT-2, V-UT-3, V-IT-10 |
| G-2 | V-UT-4, V-UT-5, V-UT-6, V-UT-7, V-UT-34 |
| G-3 | V-UT-8 .. V-UT-15, V-UT-16, V-UT-17, V-UT-30, V-UT-31, V-UT-32, V-UT-33, V-IT-1, V-IT-2, V-IT-3, V-IT-13, V-IT-14, V-F-1, V-F-2, V-F-6 |
| G-4 | V-UT-21, V-UT-22, V-UT-23, V-IT-7 |
| G-5 | V-UT-14, V-UT-15, V-IT-1 |
| G-6 | V-UT-27, V-UT-28, V-UT-29, V-IT-4, V-IT-5, V-IT-11, V-IT-12, V-F-3 |
| G-7 | V-IT-6 |
| G-8 | V-IT-6 + manual diff review |
| G-9 | V-IT-6, V-UT-35 |
| G-10 | V-IT-7, V-IT-8, V-F-5 |
| C-1 | code review + cargo doc graph |
| C-2 | extends existing source-scan test |
| C-3 | V-UT-13 |
| C-4 | V-UT-16, V-UT-17, V-UT-30, V-UT-31, V-UT-32, V-UT-33, V-F-1, V-IT-13, V-IT-14 |
| C-5 | V-UT-4, V-UT-5 |
| C-6 | V-IT-10 |
| C-7 | V-UT-18, V-UT-19, V-UT-20 |
| C-8 | V-IT-6 |
| C-9 | V-UT-21, V-UT-22, V-UT-23 |
| C-10 | code review |
| C-11 | manual `ark --help` + `ark agent --help` smoke |
| C-12 | V-IT-7 |
| C-13 | V-F-4 |
| C-14 | V-UT-12, V-E-9 |
| C-15 | V-IT-1 (CHANGELOG row check) + V-UT-30 |
| C-16 | V-F-4 |
| C-17 | V-IT-9 |
| C-18 | V-UT-25, V-IT-11, V-IT-12 |
| C-19 | V-UT-2, V-UT-3 |
| C-20 | V-IT-6 |
| C-21 | V-E-8 |
| C-22 | declared unsupported (NG-12); no test |
| C-23 | V-UT-17, V-IT-1, V-IT-2, V-IT-3 |
| C-24 | V-UT-26 |
| C-25 | V-UT-16, V-UT-33, V-IT-14 |
| R-201 (slug-anchored recovery) | V-UT-36, V-UT-37, V-UT-38, V-IT-1, V-IT-15, V-IT-16 |
| R-203 (RollbackGuard partial-failure coverage) | V-UT-39, V-UT-40, V-UT-41, V-UT-42, V-IT-13 |
| R-204 (no Ark-managed file dirty post-commit) | V-UT-17, V-IT-1, V-IT-2, V-IT-3 |
| R-205 (consistent agent-CLI naming) | V-UT-35, V-IT-6 |
