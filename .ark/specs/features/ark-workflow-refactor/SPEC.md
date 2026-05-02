
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

[**CHANGELOG**]

- 2026-05-02: replaced from 02_PLAN.md (prior body preserved in git history)
