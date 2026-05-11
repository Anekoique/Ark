[**Goals**]

- G-1: Per-developer journal trees under `.ark/workspace/<dev>/journal-N.md`, written at `/ark:commit` time as part of the same atomic commit as the work.
- G-2: Closing-commit SHA recorded deterministically via a `<PENDING:<slug>>` sentinel patched at archive time using `git log -S '**Slug**: <slug>' --format=%H`.
- G-3: Top-level `.ark/workspace/index.md` and per-developer `index.md` carry auto-maintained tables inside `<!-- ARK:DEVELOPERS -->` and `<!-- ARK:SESSIONS -->` managed blocks.
- G-4: `ark init --developer <name>` bootstraps identity at `.ark/.developer` (gitignored).
- G-5: `ark archive` is transactional: snapshots before any mutation, rolls back on failure with one documented exception (concurrent-appender suffix drift).

[**Non-goals**]

- NG-1: Squash-merge / as-merged-SHA recording — deferred.
- NG-2: Multi-developer concurrent-write coordination beyond `O_APPEND` — preserved via suffix-checked rollback, not violated.
- NG-3: Cross-project workspace aggregation.

[**Architecture**]

```
crates/ark-core/src/commands/agent/workspace/
├── mod.rs                    (public surface — re-exports)
├── identity.rs               (.ark/.developer read/write/prompt; Identity newtype)
├── config.rs                 ([workspace] in .ark/config.toml)
├── developer.rs              (register/touch developer in top-level index)
├── entry_draft.rs            (parses --entry-file payload into EntryDraft)
├── transaction.rs            (NEW — RecordTransaction; snapshot/rollback for journal+indices)
└── record.rs                 (journal append + personal-index upsert;
                                composes RecordTransaction internally)

crates/ark-core/src/commands/
├── archive.rs                (modified — ArchiveTransaction + clean-index precondition)
├── init.rs                   (wires --developer / --no-developer / interactive prompt)
├── context/                  (adds 'record' projection — Scope::Record + RecordProjection)
└── agent/task/
    ├── new.rs                (unchanged)
    ├── commit.rs             (modified — adopts RecordTransaction snapshots)
    └── transaction.rs        (NEW — CommitTransaction wraps the existing RollbackGuard)

crates/ark-core/src/commands/agent/state.rs
└── modified — TaskToml gains `journal_path: Option<String>`
```

Snapshot ordering — the key invariant:

```
Inside workspace_record:
  ├── 1. Resolve identity, config, active journal path.
  ├── 2. RecordTransaction::begin(journal_path, personal_idx, top_idx)
  │       snapshots BEFORE any write:
  │         - journal byte-length (length anchor)
  │         - personal_index bytes (full)
  │         - top_level_index bytes (full)
  ├── 3. Render entry bytes (kept in memory for suffix-check rollback).
  ├── 4. Append journal bytes via PathExt::append_text.
  │       on error: tx.rollback() (no-op since no write succeeded)
  ├── 5. Update personal_index ARK:SESSIONS block.
  │       on error: tx.rollback_journal_only() — suffix-check truncate
  ├── 6. Update top_level_index ARK:DEVELOPERS block (developer_touch).
  │       on error: tx.rollback_journal_and_personal()
  └── 7. tx.commit() — record success; return RecordSummary { snapshots, ... }.

Then in task_commit (composing layer — CommitGuard, extension of RollbackGuard):
  ├── 1. EntryDraft::parse(--entry-file).
  ├── 2. CommitGuard::begin(task)              # rename of existing RollbackGuard
  │       snapshot task.toml bytes (existing).
  ├── 3. (deep) snapshot SPEC bytes / non-existence (existing).
  ├── 4. (deep) snapshot features-INDEX bytes (existing).
  ├── 5. (deep) spec_extract + spec_register   (existing).
  ├── 6. record_summary = workspace_record(...)        [self-protected per above]
  ├── 7. guard.adopt_record_snapshot(record_summary.into_snapshot())  # NEW
  ├── 8. Persist task.toml.
  ├── 9. git add <work + task.toml + (deep) SPEC + features-INDEX +
  │              workspace files>
  │       guard.track_staged_path(p) for each.        # NEW
  ├── 10. git commit.
  │        on error 5..10: guard.rollback() — git reset HEAD --
  │          tracked paths; reverse-order RecordSnapshot::rollback for each
  │          adopted snapshot (suffix-check truncate + restore indices);
  │          restore features-INDEX; restore SPEC; restore task.toml.
  └── 11. guard.commit().
```

`ark archive` boundary — clean-index precondition:

```
ark archive [--skip-slot-patch <slug>]... [--dry-run]
  ├── Precondition: git diff --cached --quiet
  │     dirty index: error ArchiveIndexNotEmpty { staged_paths }; halt.
  ├── ArchiveTransaction::begin()
  │     snapshots: per-task journal bytes + personal_index bytes + mv intent.
  ├── for each task with phase = committed:
  │     ├── (skip decisions per Failure Flow)
  │     ├── resolve_closing_sha (collect-then-classify; errors halt with rollback)
  │     ├── if sentinel absent: skip patch; else patch journal + personal_index in memory.
  │     ├── write patched files atomically (write_atomic — temp+rename).
  │     └── execute mv.
  ├── git add <patched journals + indices + moved task dirs>     # Ark paths only
  ├── git commit -m "chore(archive): bulk-archive N task(s)" + audit body.
  │     on error: ArchiveTransaction::rollback() — reverse all mvs,
  │       restore all journal/index bytes, git reset HEAD -- <ark-paths>.
  └── ArchiveTransaction::commit().
```

Rollback primitives:

- **Index rollback** (full-file): `io::fs::write_atomic(path, original_bytes)` — temp+rename. Used by `RecordTransaction::rollback`, `CommitGuard::rollback` index restores, and `archive::patch_slot` writes. Distinct from `io::fs::write_file` (content-aware Skip/Force, not temp+rename).
- **Journal rollback** (suffix-checked): `RecordTransaction::rollback_journal()` opens the journal read-only, reads the last `len(appended_bytes)` bytes, byte-compares with `appended_bytes`. Match → `set_len(snapshot_len)`. Mismatch → `Error::JournalDriftDetected`; file untouched (preserves concurrent appender's data).
- **Mv rollback**: reverse `std::fs::rename` on the same filesystem (atomic).
- **Git index rollback** (targeted): `git reset HEAD -- <ark-path-1> <ark-path-2> ...`. Touches only paths this transaction added; never user-staged paths because of the clean-index precondition.
- **SPEC + features-INDEX rollback** (existing in `RollbackGuard`): SPEC restore = `write_atomic(spec_path, snapshot_bytes)` if pre-existed, else `fs::remove_file` if snapshot recorded non-existence. features-INDEX restore = `write_atomic(features_index_path, snapshot_bytes)`. Reverse-order: features-INDEX → SPEC → task.toml.

[**Data Structure**]

```rust
// All public structs use private fields (S-21 Rust style); each has
// `pub fn new(...) -> Self` constructor + `pub fn <field>(&self) -> ...` accessors.

// ark-core/src/commands/agent/workspace/identity.rs
pub struct Identity { name: String }
impl Identity {
    pub fn new(name: String) -> Result<Self>;          // validates non-empty
    pub fn name(&self) -> &str;
}

// ark-core/src/commands/agent/workspace/config.rs
pub struct WorkspaceConfig {
    journal_max_lines: usize,    // default 2000
    developer: Option<String>,
}
impl WorkspaceConfig {
    pub fn load_or_default(project_root: &Path) -> Result<Self>;
    pub fn journal_max_lines(&self) -> usize;
    pub fn developer(&self) -> Option<&str>;
}

// ark-core/src/commands/agent/workspace/entry_draft.rs
pub struct EntryDraft {
    title: String,
    summary: String,
    main_changes: Vec<(String, String)>,
}
impl EntryDraft {
    pub fn parse(text: &str) -> Result<Self>;
    pub fn render_task(&self, header: &TaskHeader<'_>) -> String;
    pub fn render_manual(&self, header: &ManualHeader<'_>) -> String;
}

pub struct TaskHeader<'a>   { /* private: session_number, date, slug, branch,
                                base_branch, start_head, commits_in_range */ }
pub struct ManualHeader<'a> { /* private: session_number, date, branch */ }

// ark-core/src/commands/agent/workspace/transaction.rs (NEW)
pub struct RecordTransaction {
    journal_path: PathBuf,
    journal_byte_length_before: u64,
    appended_bytes: Vec<u8>,                  // for suffix-check rollback
    personal_index_path: PathBuf,
    personal_index_bytes_before: Vec<u8>,
    top_level_index_path: PathBuf,
    top_level_index_bytes_before: Vec<u8>,
    state: TxState,
}

enum TxState { Open, JournalAppended, PersonalUpdated, TopLevelUpdated, Committed, RolledBack }

impl RecordTransaction {
    pub fn begin(journal: PathBuf, personal_idx: PathBuf, top_idx: PathBuf) -> Result<Self>;
    pub fn record_appended_bytes(&mut self, bytes: Vec<u8>);
    pub fn mark_journal_appended(&mut self);
    pub fn mark_personal_updated(&mut self);
    pub fn mark_top_level_updated(&mut self);
    pub fn rollback(self) -> Result<()>;        // suffix-check journal, restore indices in reverse
    pub fn commit(self) -> RecordSnapshot;      // returns adopt-able snapshot
}

pub struct RecordSnapshot {                     // returned by commit() for higher-layer adoption
    journal_path: PathBuf,
    journal_byte_length_before: u64,
    appended_bytes: Vec<u8>,
    personal_index_path: PathBuf,
    personal_index_bytes_before: Vec<u8>,
    top_level_index_path: PathBuf,
    top_level_index_bytes_before: Vec<u8>,
}
impl RecordSnapshot {
    pub fn rollback(self) -> Result<()>;        // same suffix-check + restore-index logic
}

// ark-core/src/commands/agent/workspace/record.rs
pub enum RecordMode<'a> {
    Task   { slug: &'a str, entry: &'a EntryDraft },
    Manual { entry: &'a EntryDraft },
}

pub struct RecordOptions<'a> { /* private fields */ }
impl<'a> RecordOptions<'a> {
    pub fn new(project_root: &'a Path, mode: RecordMode<'a>) -> Self;
    pub fn with_identity(self, identity: &'a Identity) -> Self;
}

pub struct RecordSummary {
    journal_path: PathBuf,
    journal_path_relative: String,
    session_number: u32,
    rotated: bool,
    snapshot: RecordSnapshot,                   // adopt-able on success
}
impl RecordSummary {
    pub fn journal_path(&self) -> &Path;
    pub fn journal_path_relative(&self) -> &str;
    pub fn session_number(&self) -> u32;
    pub fn rotated(&self) -> bool;
    pub fn into_snapshot(self) -> RecordSnapshot;
}

// ark-core/src/commands/agent/task/commit.rs (existing RollbackGuard, EXTENDED)
//
// Existing fields:
//   - task_toml_snapshot: TaskToml
//   - spec_snapshot: Option<SpecSnapshot>           (deep tier)
//   - features_index_snapshot: Option<FeaturesIndexSnapshot>   (deep tier)
//   - commits_landed: u8
//
// Adds:
//   - adopted_record_snapshots: Vec<RecordSnapshot>
//   - staged_paths: Vec<PathBuf>            (paths added by this guard's git add)
pub struct CommitGuard {                     // renamed from RollbackGuard
    // existing fields...
    adopted_record_snapshots: Vec<RecordSnapshot>,
    staged_paths: Vec<PathBuf>,
}

impl CommitGuard {
    // existing methods kept verbatim:
    pub fn snapshot_toml(&mut self, toml: TaskToml);
    pub fn snapshot_spec(&mut self, spec_path: &Path) -> Result<()>;
    pub fn snapshot_features_index(&mut self, path: &Path) -> Result<()>;
    pub fn commit(self);
    // (Drop impl runs restore() on rollback path)

    // NEW additive methods:
    pub fn adopt_record_snapshot(&mut self, snapshot: RecordSnapshot);
    pub fn track_staged_path(&mut self, path: &Path);
}

// Internal `restore()` gains two reverse-order steps AT THE TOP:
//   0a. git reset HEAD -- <staged_paths>             (NEW)
//   0b. for each adopted_record_snapshots in reverse: snapshot.rollback() (NEW)
//   1.  git reset --soft HEAD~1 if commits_landed    (existing)
//   2.  SPEC restore                                  (existing)
//   3.  features-INDEX restore                        (existing)
//   4.  task.toml restore                             (existing)

// ark-core/src/commands/archive.rs
pub struct ArchiveTransaction { /* private */ }
impl ArchiveTransaction {
    pub fn begin(project_root: &Path) -> Result<Self>;       // also runs clean-index check
    pub fn record_task(&mut self, snapshot: TaskArchiveSnapshot);
    pub fn record_skip(&mut self, skip: SkipRecord);
    pub fn rollback(self) -> Result<()>;                     // reverse mvs + restore bytes + targeted git reset
    pub fn commit_message_body(&self) -> String;
    pub fn commit(self);
}

pub struct SkipRecord { slug: String, reason: SkipReason }
impl SkipRecord {
    pub fn new(slug: String, reason: SkipReason) -> Self;
    pub fn slug(&self) -> &str;
    pub fn reason(&self) -> SkipReason;
}

#[derive(Clone, Copy, Debug)]
pub enum SkipReason {
    UserRequested,
    JournalPathAbsent,
    SentinelAlreadyFilled,
}

// ark-core/src/commands/agent/state.rs (TaskToml extension)
pub struct TaskToml {
    // ...existing...
    pub start_head:   Option<String>,
    pub journal_path: Option<String>,           // NEW
}

// ark-core/src/commands/context/projection.rs (additions)
pub enum Scope {
    Session,
    Phase(PhaseFilter),
    Record,                                     // NEW — for `--scope record`
}

pub enum ScopeTag {
    Session,
    Phase { phase: PhaseFilter },
    Record,                                     // NEW
}

pub struct ProjectedContext {
    // ...existing fields (schema 1)...
    pub scope: ScopeTag,
    pub record: Option<RecordProjection>,       // Some only when scope == Record
}

pub struct RecordProjection {
    pub identity:               Option<String>,    // None if .ark/.developer absent
    pub active_journal_path:    Option<String>,    // project-relative; None if no entries yet
    pub journal_max_lines:      usize,
    pub session_count:          u32,
    pub branch:                 Option<String>,
}
```

[**API Surface**]

```rust
// public re-exports from workspace::mod.rs
pub use config::WorkspaceConfig;
pub use developer::{
    DeveloperRegisterOptions, DeveloperTouchOptions,
    developer_register, developer_touch,
};
pub use entry_draft::{EntryDraft, ManualHeader, TaskHeader};
pub use identity::{Identity, ResolveOptions, identity_resolve, identity_write};
pub use record::{RecordMode, RecordOptions, RecordSummary, workspace_record};
pub use transaction::{RecordSnapshot, RecordTransaction};

// archive.rs internal helpers
fn resolve_closing_sha(project_root: &Path, journal_path: &Path, slug: &str) -> Result<String>;
fn patch_slot(journal_path: &Path, personal_index_path: &Path, slug: &str, short_sha: &str) -> Result<bool>;
fn ensure_clean_index(project_root: &Path) -> Result<()>;

// io/fs/mod.rs (NEW helper)
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()>;
//   writes <path>.<pid>.<rand>.tmp in same parent dir, then std::fs::rename.
//   atomic on the same filesystem (Posix rename guarantee).
//   distinct from io::fs::write_file (content-aware Skip/Force).

// CLI surface
ark agent task commit --entry-file <path> [-m <msg>] [--no-commit]
ark agent workspace record --task <slug>     --entry-file <path>
ark agent workspace record --manual          --entry-file <path>
ark agent workspace developer register --name <n>
ark agent workspace developer touch    --name <n>
ark init --developer <n> | --no-developer
ark archive [--skip-slot-patch <slug>]... [--dry-run]
ark context --scope record [--format json]
```

[**Constraints**]

- C-1: Journal append uses `PathExt::append_text` (`O_APPEND`); per-call atomicity is the OS guarantee.
- C-2: Sentinel format is exactly `<PENDING:<slug>>`.
- C-3: `**Slug**: <slug>` is the pickaxe anchor; manual entries use `**Slug**: -`.
- C-4: Pickaxe runs before archive's git commit so the closing commit is reachable.
- C-5: Short SHA = 12 chars (`git rev-parse --short=12`).
- C-6: No parent-resolution; the journal lives on the task's branch.
- C-7: Managed-block markers reuse existing `io::fs::{read,update,remove,merge}_managed_block` API.
- C-8: `ark upgrade` scaffolds the top-level workspace index, adds `[workspace]` config, scaffolds the developer dir if `.ark/.developer` exists.
- C-9: `task.toml.journal_path` is project-relative POSIX-style.
- C-10: `--no-commit` skips `workspace_record` entirely; `journal_path` stays None.
- C-11: Workspace files are staged inside `CommitTransaction`; the staged set is tracked for rollback.
- C-12: Identity prompt re-prompts on blank input + missing env.
- C-13: Journal file scans sort descending; `scan_session_count` returns max-N.
- C-14: `WorkspaceConfig::load_or_default` reads only the `[workspace]` section via a private `RawConfig`.
- C-15: `CommitTransaction::rollback` is reverse-order: unstage workspace paths → restore top-level index → restore personal index → suffix-check truncate journal → restore `task.toml`.
- C-16: `ArchiveTransaction::rollback` is reverse-order across tasks: undo any `mv` for the in-flight task, restore its journal/index, then prior tasks in reverse, then targeted `git reset HEAD -- <ark-paths>`.
- C-17: `--entry-file` parser is strict; missing required sections → `Error::EntryFileMalformed`.
- C-18: Skip audit body uses one line per skipped slug: `skipped slot-patch: <slug> (<reason-code>)`.
- C-19: Suffix-checked journal rollback: `RecordTransaction` keeps appended bytes in memory. On rollback: read the last `len(appended_bytes)` bytes; byte-match → `set_len(snapshot_len)`; mismatch → `Error::JournalDriftDetected`, file untouched. Concurrent-appender data preserved.
- C-20: `ark archive` requires a clean git index; runs `git diff --cached --quiet` first; dirty index → `Error::ArchiveIndexNotEmpty { staged_paths }` with hint.
- C-21: `io::fs::write_atomic(path, bytes)` writes `<path>.<pid>.<rand>.tmp` in the same parent dir then renames (atomic on same filesystem). Used by transaction restores and archive patch writes. `io::fs::write_file` (content-aware Skip/Force) is unchanged.

[**CHANGELOG**]

- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
- 2026-05-11 `guard-journal-stamp`: CLI enforces the session-heading contract. `stamp_task` and `stamp_manual` refuse when the journal's last `## Session N:` heading is already followed by stamped auto-fields; failure surfaces as `Error::JournalSessionHeadingMissing`.
