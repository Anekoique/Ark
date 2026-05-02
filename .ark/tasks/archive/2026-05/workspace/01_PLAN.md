# `workspace` PLAN `01`

> Status: Draft
> Feature: `workspace`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none`
> - Related Specs: `.ark/specs/features/ark-workflow-refactor/SPEC.md` (in-flight, see R-005 resolution); existing `crates/ark-core/src/io/fs/managed_block.rs` (`read|update|remove|merge_managed_blocks`).

---

## Summary

Iteration 01 closes the four blocking findings from 00_REVIEW. The headline changes:

1. **Agent content is delivered as a `--entry-file <path>` payload** to `task_commit` (R-001 / TR-2). The slash command's responsibility is: render a draft entry to a temp file → let the agent edit it in-place → invoke `ark agent task commit --entry-file <path>`. The CLI parses the file as the journal entry body (everything below the auto-populated header), validates structural fields, then `workspace_record` writes the final entry. No flag-escaping. Single atomic command.
2. **Both `/ark:commit` and `ark archive` gain a `Transaction` design** (R-002 / R-003). `CommitTransaction` snapshots `task.toml` + journal byte-length + personal index bytes + top-level index bytes before any mutation; rolls back on any error before the git commit lands. `ArchiveTransaction` snapshots every touched journal/index per task + every `mv` it intends to perform; rolls back in reverse order on failure. Both use the existing `io::fs::write_file` (atomic temp+rename) for index writes and a `truncate-to-snapshot-length` primitive for journal append-rollback.
3. **Pickaxe is collect-then-classify** (R-004). `resolve_closing_sha` runs `git log -S '**Slug**: <slug>' --format=%H -- <journal-path>` (no `-n`), collects all matching SHAs, errors on 0 or >1, derives the 12-char short SHA via `git rev-parse --short=12 <sha>` only for the unambiguous-success case.
4. **Existing `io::fs::{read,update,remove,merge}_managed_block` helpers reused directly** (TR-1). T-1 dropped; Phase 2 now references the existing API.
5. **Upgrade scaffolds developer dir when `.ark/.developer` exists** (R-006). Phase 6 step explicit.
6. **Validation suite expanded** with rollback tests, multi-task archive partial-failure, user-staged-non-Ark preservation, content-delivery path, and skip-slot-patch audit-trail tests (R-007 / TR-3).
7. **Skip-slot-patch is auditable** (TR-3). When `--skip-slot-patch <slug>` is used, archive's commit message body lists the skipped slugs; `ark archive --dry-run` and the success summary print them too.

The Spec (Goals/Constraints) and Architecture sections from 00 are kept self-contained — this PLAN's Spec section is the body of the future feature SPEC. Deltas from 00 are tracked in `## Log`.

## Log

[**Added**]

- `EntryDraft` data type (parser for `--entry-file` payload).
- `CommitTransaction` and `ArchiveTransaction` data types with snapshot/rollback contracts.
- `Error::EntryFileMalformed`, `Error::CommitTransactionFailed`, `Error::ArchiveTransactionFailed` variants.
- New goals: G-13 (rollback durability), G-14 (--entry-file content protocol), G-15 (skip-slot-patch audit trail), G-16 (upgrade scaffolds developer dir).
- New validations: V-UT-15 through V-UT-22, V-IT-7 through V-IT-9b, V-F-7 through V-F-12.

[**Changed**]

- G-2: Pickaxe spec rewritten to "collect-then-classify"; explicit no-`-n` directive.
- G-12: Reworded to reference the audit-trail requirement for skips.
- T-1: Removed (resolved via reuse of existing managed-block helpers per TR-1).
- T-2: Replaced with `--entry-file` protocol (TR-2).
- T-4: Reworded to encode TR-3's audit-trail requirement.
- Phase 4: Now consumes `--entry-file`; agent edits the rendered draft before `task_commit` reads it.
- Phase 5: Now wraps moves + patches in `ArchiveTransaction`; failure tests added.
- Phase 6: Adds developer-dir scaffold step when `.ark/.developer` exists.
- C-1: Atomicity claim refined — `O_APPEND` provides per-write atomicity; rollback uses pre-append byte-length truncation.

[**Removed**]

- T-1 (managed-block share-vs-copy debate): obsolete given existing helpers.
- 00_PLAN's "decision during implementation" hedges in Phase 4 step 2a and Phase 2 step 2 — both now have concrete protocols.

[**Unresolved**]

- None. All CRITICAL/HIGH findings closed in this iteration. MEDIUMs (R-005, R-006, R-007) closed.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Added `--entry-file <path>` protocol to `task_commit`. `EntryDraft` parser splits header (auto-populated by agent commit-prep step) from body (agent-authored). Slash command renders a draft, agent edits, single `ark agent task commit --entry-file <path>` consumes it. See Spec G-14, Implementation Phase 4 step 2, V-UT-15 / V-UT-16 / V-IT-7. |
| Review | R-002 | Accepted | Added `CommitTransaction` snapshot/rollback. Snapshot before any mutation: `task.toml` bytes, journal byte-length (snapshot point for `truncate-back`), personal index bytes, top-level index bytes. Rollback restores all four on any error before the `git commit` lands. See Spec G-13, Architecture transaction diagram, V-F-7..V-F-10. |
| Review | R-003 | Accepted | Added `ArchiveTransaction` covering: per-task snapshot of journal + personal index bytes, per-task `mv` intent + reverse-mv on rollback, single staging set built up across tasks, single `git commit` at the end. Preserves user-staged non-Ark files via `git diff --cached --name-only` snapshot before archive begins; rollback restores the original index. See Architecture, Phase 5 steps 1c..1g, V-F-11 / V-F-12 / V-IT-8 / V-IT-9. |
| Review | R-004 | Accepted | `resolve_closing_sha` rewritten to run pickaxe without `-n`, collect all matching SHAs, error on 0 (`SlotResolveNoMatch`) or >1 (`SlotResolveAmbiguous { candidates: [..] }`), derive short SHA only on unambiguous success. Updated G-2, V-UT-11, V-UT-12, V-F-1. |
| Review | R-005 | Accepted | The promoted SPEC for `ark-workflow-refactor` does not yet exist (the task is `phase = committed`, awaiting `ark archive`). Since this PLAN inherits its constraints, the frontmatter `Depends on` row now lists `.ark/specs/features/ark-workflow-refactor/SPEC.md` as a future dependency and `.ark/tasks/ark-workflow-refactor/02_PLAN.md` as the temporary substitute. Once `ark archive` runs (after this task itself ships), the future SPEC will be authoritative. PRD's `[**Related Specs**]` row already lists the spec path. |
| Review | R-006 | Accepted | Phase 6 step 3 split into 3a (top-level scaffold), 3b (config patch), 3c (developer dir scaffold when `.ark/.developer` exists). New goal G-16, new validation V-IT-9b. |
| Review | R-007 | Accepted | Validation matrix expanded with V-UT-15..V-UT-22 (rollback unit tests), V-IT-7..V-IT-9b (content delivery + multi-task archive + upgrade-with-developer), V-F-7..V-F-12 (failure modes for both transactions and skip-slot-patch audit). Acceptance Mapping updated to cover G-13..G-16. |
| Review | TR-1 | Accepted | Reuse `io::fs::{read,update,remove,merge}_managed_block` directly. T-1 removed from Trade-offs. Phase 2 step 2 references the existing API by name. |
| Review | TR-2 | Accepted | `--entry-file <path>` protocol per R-001 resolution. T-2 reworded to document the chosen protocol rather than the rejected options. |
| Review | TR-3 | Accepted | Skip-slot-patch is now audit-visible: archive's commit message body lists skipped slugs (and reasons if known), `ark archive --dry-run` reports them, success summary prints them. Validation V-F-3 + V-F-12 enforce. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec `Core specification`

[**Goals**]

- G-1: Per-developer journal trees under `.ark/workspace/<dev>/` with sequential `journal-N.md` files, written at `/ark:commit` time as part of the same atomic commit as the work.
- G-2: Closing Commit SHA recorded in the journal entry, resolved deterministically without amending or chore-committing. Mechanism: write `<PENDING:<slug>>` sentinel at commit time; `ark archive` patches the sentinel via a **collect-then-classify** pickaxe — `git log -S '**Slug**: <slug>' --format=%H -- <journal-path>` (no `-n` cap), collect all matching full SHAs, error on 0 or >1, derive 12-char short SHA only on unambiguous success.
- G-3: Top-level `.ark/workspace/index.md` with auto-maintained Active Developers table inside `<!-- ARK:DEVELOPERS:START/END -->` markers (managed via existing `io::fs` helpers).
- G-4: Per-developer `.ark/workspace/<dev>/index.md` with auto-maintained Session History table inside `<!-- ARK:SESSIONS:START/END -->` markers; archive patches the matching row's `Closing Commit` cell in lockstep with the journal patch.
- G-5: Compact, table-first journal entry shape (Trellis-derived): no long sentences, no `Files Created`, no `Testing`, no `Next Steps`, no `Package`. Auto-populated structural fields, agent-filled content fields delivered via `--entry-file`.
- G-6: Manual `/ark:record` entries use the same shape with `**Slug**: -` (so the slug-anchored pickaxe never matches them) and omit `Base Branch`, `Start Head`, `Closing Commit`, `Git Commits`.
- G-7: `task.toml` gains `journal_path: Option<String>` captured at `/ark:commit` time; `archive` reads it directly to locate the journal file (no re-derivation).
- G-8: Idempotent archive — re-running on a task whose slot is already filled is a no-op (sentinel-presence check). Re-running on a partially-archived state resumes safely.
- G-9: Identity bootstrap consolidated to `ark init --developer <name>` / `--no-developer` + interactive prompt. Identity stored in `.ark/.developer` (gitignored).
- G-10: Configuration in `.ark/config.toml`'s `[workspace]` section: `journal_max_lines` (default 2000), `developer` (optional override).
- G-11: Across all three platforms in lockstep — Claude `/ark:record`, Codex `ark-record` skill, OpenCode `/ark:record`.
- G-12: Failure modes are explicit and audit-visible: pickaxe 0 → error with `--skip-slot-patch <slug>` escape; pickaxe >1 → ambiguous error with candidate list; journal moved/missing → error with recorded path. **Every skip is recorded in the archive commit message body and the success summary.**
- G-13: **Atomic transactions on both commit and archive boundaries.** `/ark:commit` snapshots `task.toml` + journal byte-length + personal index bytes + top-level index bytes before mutation; rolls back all four on any error before `git commit` lands. `ark archive` snapshots per-task journal + personal index bytes + intended `mv` paths + the original git index; rolls back in reverse order on failure.
- G-14: **Agent content delivered via `--entry-file <path>`.** Slash command renders a draft journal entry to a temp file, agent edits the agent-fillable sections in place, single `ark agent task commit --entry-file <path>` consumes the file. CLI validates structural header fields, then `workspace_record` writes the final entry. No `--summary` / `--main-changes` flags. No two-step edit-then-commit.
- G-15: **Skip-slot-patch is auditable.** When `--skip-slot-patch <slug>` is used, the archive commit message body lists the skipped slugs (one per line). `ark archive --dry-run` and the success summary print them.
- G-16: **`ark upgrade` scaffolds the developer dir** (`.ark/workspace/<dev>/index.md`) when `.ark/.developer` exists, in addition to the top-level `.ark/workspace/index.md`. Never overwrites existing journals or personal indices.

- NG-1: Squash-merge / as-merged-SHA recording — deferred to `task-finalize`.
- NG-2: Multi-developer concurrent-write coordination beyond `O_APPEND`.
- NG-3: Cross-project workspace aggregation.
- NG-4: Worktree cleanup post-archive — deferred to `task-finalize`.
- NG-5: UI / web rendering of journals.
- NG-6: Backfill of pre-workspace task entries.

[**Architecture**]

```
crates/ark-core/src/commands/agent/workspace/
├── mod.rs              # public surface (re-exports)
├── identity.rs         # `.ark/.developer` read/write/prompt; Identity newtype
├── config.rs           # `[workspace]` section in `.ark/config.toml`
├── developer.rs        # register/touch developer in top-level index
│                       # (uses io::fs::update_managed_block, marker = "ARK:DEVELOPERS")
├── record.rs           # journal append + personal-index upsert
│                       # (uses io::fs::update_managed_block, marker = "ARK:SESSIONS")
└── entry_draft.rs      # NEW — parses `--entry-file` payload into EntryDraft

crates/ark-core/src/commands/
├── archive.rs          # *modified* — adds ArchiveTransaction + slot-patch + audit
├── init.rs             # *modified* — wires --developer / --no-developer / prompt
└── agent/task/
    ├── new.rs          # *unchanged*
    ├── commit.rs       # *modified* — adds CommitTransaction; consumes --entry-file
    └── transaction.rs  # NEW — CommitTransaction snapshot/rollback primitive

crates/ark-core/src/commands/agent/state.rs
└── *modified* — TaskToml gains `journal_path: Option<String>`
```

**`/ark:commit` atomic boundary (with CommitTransaction):**

```
ark agent task commit --entry-file <path>
  ├── read entry-file → EntryDraft (validate header structure)
  ├── CommitTransaction::begin()                          # NEW
  │     snapshots: task.toml bytes, journal byte-len,
  │                personal index bytes, top-level index bytes
  ├── deep tier: spec_extract (existing — already
  │              transaction-protected by the existing flow)
  ├── workspace::record(--task <slug>, EntryDraft)        # NEW
  │     ├── identity::resolve()
  │     ├── developer::register_if_absent(<dev>)
  │     ├── journal::append(<dev>, rendered_entry)
  │     ├── personal_index::upsert_session_row(<row>)
  │     └── developer::touch(<dev>, <count>, <active>)
  ├── set task.toml.journal_path
  ├── set task.toml.phase = Committed
  ├── git add <work + task.toml + (deep) SPEC + features INDEX
  │             + workspace files>
  ├── git commit -m "<msg>"
  │     ↪ on any error after CommitTransaction::begin and before commit
  │       lands successfully: CommitTransaction::rollback() restores all
  │       snapshotted bytes and unstages workspace paths.
  └── CommitTransaction::commit()                         # mark success
```

**`ark archive` atomic boundary (with ArchiveTransaction):**

```
ark archive [--skip-slot-patch <slug>]... [--dry-run]
  ├── ArchiveTransaction::begin()                         # NEW
  │     snapshots: original git index (`git diff --cached --name-only`),
  │                per-task journal bytes + personal index bytes,
  │                per-task source dir + dest dir (mv intent).
  ├── for each task with phase = committed:
  │     ├── if slug in skip-list:
  │     │     record skip-with-reason in audit log; skip slot-patch.
  │     ├── else if task.toml.journal_path is None:
  │     │     skip slot-patch (pre-workspace task).
  │     ├── else:
  │     │     resolve_closing_sha(...) → short SHA  (errors abort with
  │     │                                            ArchiveTransaction::rollback)
  │     │     if sentinel <PENDING:<slug>> absent in journal: skip patch.
  │     │     else: in-memory patch journal + personal index.
  │     ├── write patched files (atomic temp+rename via io::fs::write_file).
  │     └── stage `mv source → dest` (intent recorded; not yet executed).
  ├── execute all `mv` operations.
  ├── git add <patched journals + patched indices + moved task dirs>
  ├── git commit -m "chore(archive): bulk-archive N task(s)\n\n
  │                  archived: <slug>...\n
  │                  skipped slot-patch: <slug>..." (G-15 audit body)
  │     ↪ on error: ArchiveTransaction::rollback() restores journals,
  │       personal indices, reverses mv operations, restores git index
  │       to its original state.
  └── ArchiveTransaction::commit()
```

**Rollback primitives:**

- *Index rollback* — write_bytes(original_bytes) using existing `io::fs::write_file` (atomic temp+rename).
- *Journal rollback* — `truncate_to(original_len)` using `std::fs::OpenOptions::set_len`. Safe because journal writes are append-only between snapshot and rollback; truncating to the snapshotted length restores the pre-mutation state byte-for-byte.
- *Mv rollback* — reverse mv using `std::fs::rename` (atomic on the same filesystem, guaranteed for `.ark/` tree).
- *Git index rollback* — `git reset` plus `git add` of the original staged set captured by `git diff --cached --name-only` at `begin()`. Preserves user-staged non-Ark files.

[**Data Structure**]

```rust
// crates/ark-core/src/commands/agent/workspace/identity.rs
pub struct Identity {
    pub name: String,
}

pub struct ResolveOptions<'a> {
    pub project_root: &'a Path,
    pub override_name: Option<&'a str>,
}

// crates/ark-core/src/commands/agent/workspace/config.rs
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorkspaceConfig {
    pub journal_max_lines: usize,
    pub developer: Option<String>,
}

// crates/ark-core/src/commands/agent/workspace/entry_draft.rs (NEW)
pub struct EntryDraft {
    pub title: String,
    pub summary: String,
    pub main_changes: Vec<(String, String)>,
}

impl EntryDraft {
    pub fn parse(text: &str) -> Result<Self>;
    pub fn render_task(&self, header: &TaskHeader) -> String;
    pub fn render_manual(&self, header: &ManualHeader) -> String;
}

pub struct TaskHeader<'a> {
    pub session_number: u32,
    pub date: NaiveDate,
    pub slug: &'a str,
    pub branch: &'a str,
    pub base_branch: &'a str,
    pub start_head_short: &'a str,
    pub commits_in_range: &'a [(String, String)],
}

pub struct ManualHeader<'a> {
    pub session_number: u32,
    pub date: NaiveDate,
    pub branch: &'a str,
}

// crates/ark-core/src/commands/agent/workspace/record.rs
pub enum RecordMode<'a> {
    Task { slug: &'a str, entry: &'a EntryDraft },
    Manual { entry: &'a EntryDraft },
}

pub struct RecordOptions<'a> {
    pub project_root: &'a Path,
    pub mode: RecordMode<'a>,
    pub identity: Option<&'a Identity>,
}

pub struct RecordSummary {
    pub journal_path: PathBuf,
    pub journal_path_relative: String,
    pub session_number: u32,
    pub rotated: bool,
    pub journal_byte_length_before: u64,
    pub personal_index_bytes_before: Vec<u8>,
    pub top_level_index_bytes_before: Vec<u8>,
}

// crates/ark-core/src/commands/agent/task/transaction.rs (NEW)
pub struct CommitTransaction {
    project_root: PathBuf,
    task_toml_bytes_before: Vec<u8>,
    workspace_snapshot: Option<RecordSnapshot>,
    staged_paths: Vec<PathBuf>,
    pre_staged: Vec<PathBuf>,
    committed: bool,
}

struct RecordSnapshot {
    journal_path: PathBuf,
    journal_byte_length_before: u64,
    personal_index_path: PathBuf,
    personal_index_bytes_before: Vec<u8>,
    top_level_index_path: PathBuf,
    top_level_index_bytes_before: Vec<u8>,
}

impl CommitTransaction {
    pub fn begin(project_root: &Path, task: &TaskToml) -> Result<Self>;
    pub fn record_workspace_snapshot(&mut self, s: RecordSnapshot);
    pub fn add_staged_path(&mut self, p: &Path);
    pub fn rollback(self) -> Result<()>;
    pub fn commit(self);
}

// crates/ark-core/src/commands/archive.rs (modified)
pub struct ArchiveTransaction {
    project_root: PathBuf,
    pre_staged: Vec<PathBuf>,
    per_task_snapshots: Vec<TaskArchiveSnapshot>,
    skipped_slugs: Vec<SkipRecord>,
    committed: bool,
}

struct TaskArchiveSnapshot {
    slug: String,
    journal_bytes_before: Option<Vec<u8>>,
    journal_path: Option<PathBuf>,
    personal_index_bytes_before: Option<Vec<u8>>,
    personal_index_path: Option<PathBuf>,
    src_dir: PathBuf,
    dst_dir: PathBuf,
    mv_executed: bool,
}

pub struct SkipRecord {
    pub slug: String,
    pub reason: SkipReason,
}

pub enum SkipReason {
    UserRequested,
    JournalPathAbsent,
    SentinelAlreadyFilled,
}

impl ArchiveTransaction {
    pub fn begin(project_root: &Path) -> Result<Self>;
    pub fn record_task(&mut self, snapshot: TaskArchiveSnapshot);
    pub fn record_skip(&mut self, skip: SkipRecord);
    pub fn rollback(self) -> Result<()>;
    pub fn commit_message_body(&self) -> String;
    pub fn commit(self);
}

// crates/ark-core/src/commands/agent/state.rs (modified)
pub struct TaskToml {
    // ... existing fields ...
    pub start_head: Option<String>,
    pub journal_path: Option<String>,              // NEW
}
```

[**API Surface**]

```rust
// public re-exports from workspace::mod.rs
pub use config::WorkspaceConfig;
pub use developer::{
    DeveloperRegisterOptions, DeveloperTouchOptions, developer_register, developer_touch,
};
pub use entry_draft::{EntryDraft, ManualHeader, TaskHeader};
pub use identity::{Identity, ResolveOptions, identity_resolve, identity_write};
pub use record::{RecordMode, RecordOptions, RecordSummary, workspace_record};

// archive.rs internal helpers
fn resolve_closing_sha(
    project_root: &Path,
    journal_path: &Path,
    slug: &str,
) -> Result<String>;

fn patch_slot(
    journal_path: &Path,
    personal_index_path: &Path,
    slug: &str,
    short_sha: &str,
) -> Result<bool>;
```

CLI surface:

```
ark agent task commit --entry-file <path> [-m <msg>] [--no-commit]
ark agent workspace record --task <slug> --entry-file <path>
ark agent workspace record --manual --entry-file <path>
ark agent workspace developer register --name <n>
ark agent workspace developer touch --name <n>
ark init --developer <n> | --no-developer
ark archive [--skip-slot-patch <slug>]... [--dry-run]
```

[**Constraints**]

- C-1: Journal append uses `PathExt::append_text` (existing `O_APPEND`). Per-call atomicity is the OS guarantee; rollback uses `truncate_to(snapshot_len)` which is byte-exact because the file is append-only between snapshot and rollback (no other process holds it open for write within `/ark:commit`).
- C-2: Sentinel format is exactly `<PENDING:<slug>>` — slug embedded.
- C-3: `**Slug**: <slug>` is the pickaxe anchor. Each task records exactly once; manual entries use `**Slug**: -`.
- C-4: Pickaxe runs *before* archive's git commit so the closing commit is reachable.
- C-5: Short SHA = 12 chars (`git rev-parse --short=12`).
- C-6: No parent-resolution. Journal lives on the task's branch.
- C-7: Managed-block markers reuse existing `io::fs::{read,update,remove,merge}_managed_block` API. Markers: `ARK:DEVELOPERS` (top-level), `ARK:SESSIONS` (personal). The helpers already encode orphan-marker rejection.
- C-8: `ark upgrade` scaffolds top-level index, adds `[workspace]` config, and (G-16) scaffolds developer dir when `.ark/.developer` exists; never overwrites existing journals or indices.
- C-9: `task.toml.journal_path` is project-relative POSIX-style.
- C-10: `--no-commit` mode skips `workspace::record` entirely; `journal_path` stays None; archive's slot-patch is skipped per G-7.
- C-11: Workspace files staged inside `CommitTransaction`; staged set tracked for rollback.
- C-12: Identity prompt re-prompts on blank + missing env (PR #9 fix).
- C-13: Journal scans sort descending; `scan_session_count` returns max-N, not count (PR #9 fix).
- C-14: `WorkspaceConfig::load_or_default` reads only `[workspace]` via private `RawConfig { workspace: Option<...> }`.
- C-15: **`CommitTransaction::rollback` is reverse-order**: unstage workspace paths → restore top-level index → restore personal index → truncate journal → restore `task.toml`.
- C-16: **`ArchiveTransaction::rollback` is reverse-order across tasks**: for the in-flight task, undo any `mv` that ran, restore its journal/index; for prior tasks already mv'd, undo their `mv`s and restore their journals/indices; finally restore the original git index.
- C-17: **`--entry-file` parser is strict**: missing required header sections → `EntryFileMalformed` error; extra sections preserved in the rendered output.
- C-18: **Skip audit body** uses one line per skipped slug: `skipped slot-patch: <slug> (<reason-code>)`. Reason codes: `user-requested`, `journal-absent`, `sentinel-already-filled`.

## Runtime `runtime logic`

[**Main Flow**]

1. User runs `/ark:commit -m "<msg>"`. Slash command:
   a. Reads `task.toml`, computes draft entry header (session N, date, slug, branch, base_branch, start_head, commits-in-range from `git log <start_head>..HEAD --oneline`).
   b. Renders draft to `.ark/.commit-draft.md` (gitignored): full entry with header pre-filled and `### Summary` / `### Main Changes` empty placeholders + comment markers `<!-- agent: fill below -->`.
   c. Returns to agent: "Edit `.ark/.commit-draft.md` then I'll commit."
   d. Agent edits the placeholders.
   e. Slash command runs `ark agent task commit --entry-file .ark/.commit-draft.md -m "<msg>"`.
2. `task_commit`:
   a. Parse entry-file → `EntryDraft` (errors `EntryFileMalformed` if structure invalid).
   b. `CommitTransaction::begin()`.
   c. (deep) `spec_extract`.
   d. `workspace::workspace_record(Task { slug, entry: &draft })` returns `RecordSummary`.
   e. `tx.record_workspace_snapshot(snapshot_from_summary)`.
   f. Set `task.toml.journal_path = summary.journal_path_relative`.
   g. Set `task.toml.phase = Committed`. Persist.
   h. `git add` work + task.toml + (deep) SPEC + features INDEX + workspace files; each path recorded via `tx.add_staged_path()`.
   i. `git commit -m "<msg>"`.
   j. `tx.commit()`. Delete `.ark/.commit-draft.md`.

   On any error in steps b–i: `tx.rollback()` restores task.toml + indices + truncates journal + unstages added paths. Working tree returns to its state before step b. Draft file preserved for retry.

3. Manager runs `ark archive`:
   a. `ArchiveTransaction::begin()` — snapshot original git index.
   b. Scan `.ark/tasks/*/task.toml` for `phase = committed`.
   c. For each task: build `TaskArchiveSnapshot` with current journal/index bytes + src/dst dirs.
   d. Decide skip-or-process:
      - slug in `--skip-slot-patch` → record skip (UserRequested), no patch, still mv.
      - `journal_path` is None → record skip (JournalPathAbsent), no patch, still mv.
      - `<PENDING:<slug>>` absent in journal → record skip (SentinelAlreadyFilled), no patch, still mv.
      - else: `resolve_closing_sha` → patch journal + personal index in memory.
   e. Write patched files (atomic temp+rename); execute mv.
   f. After all tasks: build commit message with audit body; `git add`; `git commit`.
   g. `tx.commit()`.

[**Failure Flow**]

1. `identity::resolve` missing identity → `MissingIdentity`. `task_commit` errors before `CommitTransaction::begin`. No state changes.
2. `EntryFileMalformed` → `task_commit` errors before `begin`. No state changes.
3. `workspace_record` partial failure (e.g., journal append succeeds, personal index write fails) → `tx.rollback()` truncates journal back to snapshot length, restores top-level index from snapshot, leaves task.toml unchanged.
4. `spec_extract` failure → `tx.rollback()`. Same restoration set.
5. `task.toml` save failure → `tx.rollback()` truncates journal + restores indices.
6. `git add` failure → `tx.rollback()` includes `git reset` on any paths that were partially added.
7. `git commit` failure (e.g., pre-commit hook rejects) → `tx.rollback()` includes `git reset`. Workspace files restored from snapshots; user-staged non-Ark files preserved by selectively unstaging only paths in `tx.staged_paths`.
8. `resolve_closing_sha` 0 results → `SlotResolveNoMatch`. Without `--skip-slot-patch`, archive errors and rolls back. With `--skip-slot-patch <slug>`, that task is recorded as skipped (UserRequested), archive proceeds.
9. `resolve_closing_sha` >1 results → `SlotResolveAmbiguous { candidates }`. Same escape via `--skip-slot-patch`.
10. `mv` mid-bulk failure → `ArchiveTransaction::rollback()` reverses all completed mvs, restores all journals/indices, restores git index. Single commit batched at end means no partial commit can land.
11. `git commit` failure during archive → reverse all mvs, restore all journals/indices, `git reset` to original index.
12. Sentinel mismatch (journal has it, index doesn't, or vice versa) → `SlotMismatch`. Archive errors and rolls back unless `--skip-slot-patch <slug>` overrides.

[**State Transitions**]

- `task.toml.phase`: existing transitions; `Verify → Committed` and `Execute → Committed` now also write `journal_path` (unless `--no-commit`).
- Slot lifecycle: `<PENDING:<slug>>` → `<closing-sha-short>` (terminal); skip leaves it in `<PENDING:<slug>>` permanently with audit-trail reference.
- `CommitTransaction`: `begin → committed | rolled-back`. On panic in `commit()`, drop impl logs a warning but does not roll back (the git commit already landed).
- `ArchiveTransaction`: same shape; rollback reverses across all completed task processings.

## Implementation `split task into phases`

[**Phase 1** — Identity, Config, Module Skeleton (~200 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/{mod,identity,config}.rs`.
2. Port identity logic from PR #9 history: `Identity` newtype, `.ark/.developer` read/write/prompt, env fallback (`USER` then `USERNAME`), reprompt-on-blank.
3. Port config logic: `WorkspaceConfig`, `[workspace]` section, `RawConfig { workspace: Option<...> }`. Keys: `journal_max_lines = 2000`, `developer: Option<String>`.
4. Update `templates/ark/config.toml` to include `[workspace]` block (commented defaults).
5. Update `templates/ark/.gitignore` to include `.developer` and `.commit-draft.md`.
6. Wire `ark init --developer <n>` / `--no-developer` flags + interactive prompt.
7. Add error variants: `Error::MissingIdentity`, `Error::DeveloperWriteFailed`, `Error::WorkspaceConfigInvalid`.
8. Unit tests V-UT-1, V-UT-2, V-UT-3.

[**Phase 2** — Developer Registrar + Indices (~250 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/developer.rs`.
2. Use `io::fs::{read_managed_block, update_managed_block, merge_managed_blocks}` directly (TR-1). Markers: `ARK:DEVELOPERS` (top-level), `ARK:SESSIONS` (per-developer).
3. `developer_register`: scaffolds top-level `.ark/workspace/index.md` from template if missing; upserts dev row inside `ARK:DEVELOPERS` markers.
4. `developer_touch`: refreshes the dev row (`Last Active`, `Sessions`, `Active Journal`).
5. Personal-index upsert (in `record.rs`): scaffolds `<dev>/index.md` from template if missing; upserts row inside `ARK:SESSIONS` markers.
6. Add `templates/ark/workspace/index.md` (top-level template — static prose + empty markers).
7. Add `templates/ark/workspace/personal-index.md` (per-dev template).
8. Wire `ark agent workspace developer register|touch` CLI verbs.
9. Unit tests V-UT-4, V-UT-5.

[**Phase 3** — Record Primitive + Sentinel + Entry Draft (~350 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/{record,entry_draft}.rs`.
2. Implement `EntryDraft::parse` — strict Markdown parser keyed off section headings (`## Session N: <title>`, `### Summary`, `### Main Changes`).
3. Implement `EntryDraft::render_task` and `render_manual` with the exact field set from G-5/G-6.
4. Implement `workspace_record`:
   a. Resolve identity + config.
   b. Discover active journal (`scan_session_count` returns max-N descending).
   c. Build `TaskHeader` or `ManualHeader` from caller-provided context (slug, branch, base_branch, start_head, commits-in-range).
   d. Render full entry; capture pre-mutation snapshots (journal byte-length, personal index bytes, top-level index bytes).
   e. Append via `PathExt::append_text` (rotate first if would exceed `journal_max_lines`).
   f. Update personal index `ARK:SESSIONS` block with new session row.
   g. Call `developer_touch`.
   h. Return `RecordSummary` with `journal_path_relative` + all snapshot bytes.
5. Wire `ark agent workspace record --task <slug> --entry-file <p>` and `--manual --entry-file <p>`.
6. Edge: `start_head = None` → fallback `git log -n 20 --oneline`.
7. Unit tests V-UT-6..V-UT-10, V-UT-15, V-UT-16.

[**Phase 4** — CommitTransaction + Wire `record` into `task_commit` (~250 LOC)]

1. Create `crates/ark-core/src/commands/agent/task/transaction.rs` with `CommitTransaction`.
2. Implement snapshot/restore for: `task.toml` bytes, journal byte-length (truncate-to-snapshot rollback), personal index bytes, top-level index bytes, staged-path tracking with `git reset HEAD <path>` rollback.
3. Modify `crates/ark-core/src/commands/agent/state.rs::TaskToml` — add `journal_path: Option<String>`.
4. Modify `commit.rs`:
   a. Accept `--entry-file <path>`.
   b. Parse → `EntryDraft`.
   c. `tx = CommitTransaction::begin(...)`.
   d. (deep) `spec_extract` (existing flow).
   e. `record_summary = workspace_record(...)`.
   f. `tx.record_workspace_snapshot(record_summary.into())`.
   g. Persist `task.toml`.
   h. Stage paths; `tx.add_staged_path(...)` for each.
   i. `git commit`.
   j. `tx.commit()`.
   k. On error: `tx.rollback()`.
5. Honor `--no-commit`: skip `workspace_record` and the staging/commit steps; tx only protects `task.toml` write.
6. Update `templates/{claude,codex,opencode}/commands/skills/ark/commit.md` (and `.codex/skills/ark-commit/SKILL.md`) to:
   - Render draft to `.ark/.commit-draft.md`.
   - Tell agent: "Edit the placeholders, then I'll run commit."
   - On agent confirmation, run `ark agent task commit --entry-file .ark/.commit-draft.md -m "<msg>"`.
   - Delete the draft after success.
7. Unit tests V-UT-17..V-UT-20; integration V-IT-7.

[**Phase 5** — ArchiveTransaction + Slot Patch (~300 LOC)]

1. Modify `crates/ark-core/src/commands/archive.rs`:
   a. Add `ArchiveTransaction` with snapshot/rollback.
   b. Implement `resolve_closing_sha` per R-004: collect all matches without `-n`, error on 0 or >1, derive short SHA on unambiguous success.
   c. Implement `patch_slot` returning `bool` (false → already filled, idempotent).
   d. Per-task loop builds `TaskArchiveSnapshot` and decides skip-or-process per Failure Flow §3.
   e. Stage patched files via atomic temp+rename, then run all `mv`s, then `git add`, then `git commit` with audit body.
   f. `--skip-slot-patch <slug>` flag (repeatable).
   g. `--dry-run` prints the plan + skip records and exits without mutation.
2. Add error variants: `Error::SlotResolveNoMatch`, `Error::SlotResolveAmbiguous`, `Error::JournalMissing`, `Error::SlotMismatch`, `Error::ArchiveTransactionFailed`.
3. Unit tests V-UT-11, V-UT-12, V-UT-13, V-UT-14, V-UT-21, V-UT-22.
4. Integration V-IT-2, V-IT-3, V-IT-8.
5. Failure tests V-F-1..V-F-4, V-F-11, V-F-12.

[**Phase 6** — Slash Commands + Migration + Dogfood (~200 LOC + docs)]

1. Add `templates/{claude,codex,opencode}/...record.md|SKILL.md` — thin wrappers calling `ark agent workspace record --manual --entry-file <p>`. Each platform mirrors the draft-render → agent-edit → cli pattern.
2. Mirror to `.claude/commands/ark/record.md`, `.codex/skills/ark-record/SKILL.md`, `.opencode/commands/ark/record.md`.
3. Update `crates/ark-core/src/commands/upgrade/mod.rs`:
   a. Scaffold top-level `.ark/workspace/index.md` if absent.
   b. Add `[workspace]` config section if missing (non-destructive).
   c. (G-16) Scaffold `.ark/workspace/<dev>/index.md` if `.ark/.developer` exists and the dir doesn't.
   d. Re-render slash-command templates.
4. Update `.ark/workflow.md`, `AGENTS.md`, `README.md`, `docs/book/*` to mention workspace + `/ark:record`.
5. Dogfood: this task creates `.ark/.developer = "Anekoique"` during EXECUTE; the workspace task is the first journal entry; archive of this task exercises the slot-patch end-to-end.
6. Integration tests V-IT-4, V-IT-5, V-IT-6, V-IT-9, V-IT-9b.

## Trade-offs `ask reviewer for advice`

- T-2: **Entry-file delivery protocol.** Resolved via TR-2: `--entry-file <path>` is the chosen protocol. Slash command renders draft → agent edits → CLI reads the file. Atomic single-command commit; no shell-escape headaches; agent gets full Markdown freedom. Cost: one extra file (`.ark/.commit-draft.md` in `.gitignore`); benefit: no two-step edit-then-commit workflow.
- T-3: **Sentinel format = `<PENDING:<slug>>`.** Unchanged from 00. Human-readable, slug-embedded, greppable.
- T-4: **Skip-slot-patch is fail-loud + auditable.** Resolved via TR-3: archive fails on pickaxe errors unless `--skip-slot-patch` is passed; every skip is logged in the commit message body and the success summary. Audit-trail requirement encoded in G-15.

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `identity_resolve` returns from `.ark/.developer`, falls back to `[workspace].developer`, errors on missing both with `MissingIdentity`.
- V-UT-2: `identity` prompt reprompts on blank + missing env (PR #9 fix).
- V-UT-3: `WorkspaceConfig::load_or_default` reads `journal_max_lines` from `[workspace]`; returns 2000 default when section absent.
- V-UT-4: `developer_register` upserts a row inside `ARK:DEVELOPERS` markers; idempotent on re-register.
- V-UT-5: `developer_touch` refreshes cells; preserves hand-edits outside markers.
- V-UT-6: Personal-index upsert appends a row with `<PENDING:<slug>>` Closing Commit cell.
- V-UT-7: `workspace_record(Task)` renders all expected fields and the exact sentinel `<PENDING:<slug>>`.
- V-UT-8: `workspace_record(Manual)` renders `**Slug**: -` and omits Closing Commit / Base Branch / Start Head / Git Commits.
- V-UT-9: Journal rotation triggers when append would exceed `journal_max_lines`.
- V-UT-10: `scan_session_count` sorts journals descending and returns max-N.
- V-UT-11: `resolve_closing_sha` happy path (collect-then-classify, exactly 1 match) returns short SHA.
- V-UT-12: `resolve_closing_sha` returns `SlotResolveNoMatch` for unknown slug.
- V-UT-13: `patch_slot` returns false (skipped) when sentinel absent — idempotency.
- V-UT-14: `patch_slot` returns true and rewrites both journal + index when sentinel present.
- V-UT-15: `EntryDraft::parse` happy path — accepts a draft with all required sections.
- V-UT-16: `EntryDraft::parse` errors `EntryFileMalformed` on missing `## Session`, `### Summary`, or `### Main Changes`.
- V-UT-17: `CommitTransaction::rollback` restores `task.toml` bytes after artificial failure.
- V-UT-18: `CommitTransaction::rollback` truncates journal to snapshot length after artificial failure.
- V-UT-19: `CommitTransaction::rollback` restores both indices.
- V-UT-20: `CommitTransaction::rollback` unstages workspace paths via `git reset` while preserving user-staged non-workspace paths.
- V-UT-21: `ArchiveTransaction::rollback` reverses all `mv`s in reverse order and restores all journals/indices.
- V-UT-22: `ArchiveTransaction::commit_message_body` includes one line per skipped slug with reason code (G-15 audit format).

[**Integration Tests**]

- V-IT-1: `task new --tier deep --worktree → /ark:commit` produces a journal entry with sentinel and `task.toml.journal_path` populated.
- V-IT-2: `/ark:commit → ark archive` end-to-end: sentinel replaced with real short SHA; personal index Closing Commit cell matches.
- V-IT-3: `ark archive` is idempotent on already-archived task (sentinel-presence check passes).
- V-IT-4: `ark init --developer alice` followed by `/ark:record` produces a manual entry with `**Slug**: -`.
- V-IT-5: `ark upgrade` on a workspace-less repo scaffolds top-level `.ark/workspace/index.md` and adds `[workspace]` config section.
- V-IT-6: Three-platform parity — `/ark:record` works identically across Claude / Codex / OpenCode.
- V-IT-7: `--entry-file` flow: render draft → simulate agent edits → `task_commit --entry-file <path>` produces journal entry with the agent's content.
- V-IT-8: Multi-task bulk archive — 3 committed tasks, all journals patched in a single commit.
- V-IT-9: User-staged non-Ark file (e.g., `README.md`) survives `ark archive` regardless of whether archive succeeds or fails.
- V-IT-9b: `ark upgrade` with existing `.ark/.developer` scaffolds the developer dir (G-16).

[**Failure / Robustness Validation**]

- V-F-1: `resolve_closing_sha` returns >1 commits → `SlotResolveAmbiguous` with candidate list.
- V-F-2: Journal file moved between commit and archive → `JournalMissing { recorded_path }`.
- V-F-3: `--skip-slot-patch <slug>` bypasses patch; sentinel left as-is; commit body lists the skip with `user-requested` reason code.
- V-F-4: Sentinel in journal but missing in personal index → `SlotMismatch`.
- V-F-5: `--no-commit` mode does not write a journal entry; `journal_path` stays None.
- V-F-6: `MissingIdentity` aborts `/ark:commit` before any file write.
- V-F-7: Failure injected after `workspace_record` succeeds, before `task.toml` save → `CommitTransaction::rollback` truncates journal and restores both indices.
- V-F-8: Failure injected after `task.toml` save, before `git add` → rollback restores task.toml + truncates journal + restores indices.
- V-F-9: Failure injected after `git add`, before `git commit` → rollback runs `git reset` on tracked paths + restores files.
- V-F-10: Failure injected during `git commit` (e.g., pre-commit hook) → rollback restores all snapshots; user-staged non-Ark files preserved.
- V-F-11: Failure injected mid-bulk-archive after task K mv'd but task K+1 fails → `ArchiveTransaction::rollback` reverse-mvs task K, restores all journals/indices, restores git index.
- V-F-12: Failure injected during archive `git commit` → all mvs reversed, all journals/indices restored, git index restored.

[**Edge Case Validation**]

- V-E-1: Slug grammar is lowercase + hyphen + ASCII (existing `task new` constraint); pickaxe needs no escaping.
- V-E-2: Journal at exactly `journal_max_lines` triggers rotation on next append.
- V-E-3: Manual entries interleaved with task entries — pickaxe ignores `**Slug**: -` lines.
- V-E-4: Multiple task entries in same `journal-N.md` — each task's pickaxe uniquely matches its slug line.
- V-E-5: Concurrent record from two processes — `O_APPEND` per-write atomicity; document limitation.
- V-E-6: `--developer` flag overrides `.ark/.developer` for a single invocation; file unchanged.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1  | V-IT-1, V-UT-7 |
| G-2  | V-IT-2, V-UT-11, V-UT-12, V-F-1 |
| G-3  | V-UT-4, V-UT-5 |
| G-4  | V-UT-6, V-IT-2 |
| G-5  | V-UT-7, V-UT-8, V-IT-7 |
| G-6  | V-UT-8, V-E-3 |
| G-7  | V-IT-1, V-F-5 |
| G-8  | V-IT-3, V-UT-13 |
| G-9  | V-IT-4, V-UT-1, V-UT-2 |
| G-10 | V-UT-3, V-IT-5 |
| G-11 | V-IT-6 |
| G-12 | V-UT-12, V-F-1, V-F-2, V-F-3, V-F-4 |
| G-13 | V-UT-17, V-UT-18, V-UT-19, V-UT-20, V-UT-21, V-F-7, V-F-8, V-F-9, V-F-10, V-F-11, V-F-12 |
| G-14 | V-UT-15, V-UT-16, V-IT-7 |
| G-15 | V-UT-22, V-F-3 |
| G-16 | V-IT-9b |
| C-1  | V-UT-9, V-UT-18 |
| C-2  | V-UT-7, V-E-4 |
| C-3  | V-UT-8, V-E-3 |
| C-4  | V-IT-2 |
| C-5  | V-IT-2 |
| C-6  | V-IT-1 |
| C-7  | V-UT-4, V-UT-5 |
| C-8  | V-IT-5, V-IT-9b |
| C-9  | V-IT-1, V-IT-2 |
| C-10 | V-F-5 |
| C-11 | V-IT-1, V-UT-20 |
| C-12 | V-UT-2 |
| C-13 | V-UT-10 |
| C-14 | V-UT-3 |
| C-15 | V-UT-17, V-UT-18, V-UT-19, V-UT-20 |
| C-16 | V-UT-21, V-IT-9 |
| C-17 | V-UT-15, V-UT-16 |
| C-18 | V-UT-22, V-F-3 |
