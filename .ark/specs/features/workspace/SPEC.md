
[**Goals**]

- G-1: Per-developer journal trees under `.ark/workspace/<dev>/` with sequential `journal-N.md` files, written at `/ark:commit` time as part of the same atomic commit as the work.
- G-2: Closing Commit SHA recorded deterministically without amend or chore-commit. Mechanism: write `<PENDING:<slug>>` sentinel at commit time; `ark archive` patches via collect-then-classify pickaxe — `git log -S '**Slug**: <slug>' --format=%H -- <journal-path>` (no `-n` cap), classify by count, error on 0 or >1, derive 12-char short SHA only on unambiguous success.
- G-3: Top-level `.ark/workspace/index.md` with auto-maintained Active Developers table inside `<!-- ARK:DEVELOPERS:START/END -->` markers (existing `io::fs` helpers).
- G-4: Per-developer `.ark/workspace/<dev>/index.md` with auto-maintained Session History table inside `<!-- ARK:SESSIONS:START/END -->` markers; archive patches the matching row's Closing Commit cell in lockstep with the journal patch.
- G-5: Compact, table-first journal entry shape (Trellis-derived): no long sentences, no `Files Created`, no `Testing`, no `Next Steps`, no `Package`. Auto-populated structural fields, agent-filled content delivered via `--entry-file`.
- G-6: Manual `/ark:record` entries use the same shape with `**Slug**: -` (slug-anchored pickaxe never matches them) and omit `Base Branch`, `Start Head`, `Closing Commit`, `Git Commits`. Manual mode uses the same `RecordTransaction` as task mode (R-104).
- G-7: `task.toml` gains `journal_path: Option<String>` captured at `/ark:commit` time; archive reads it directly (no re-derivation).
- G-8: Idempotent archive — re-running on a task whose slot is filled is a no-op (sentinel-presence check). Re-running on partially-archived state resumes safely.
- G-9: Identity bootstrap consolidated to `ark init --developer <name>` / `--no-developer` + interactive prompt. Identity stored in `.ark/.developer` (gitignored).
- G-10: Configuration in `.ark/config.toml`'s `[workspace]` section: `journal_max_lines` (default 2000), `developer` (optional override).
- G-11: Across all three platforms in lockstep — Claude `/ark:record`, Codex `ark-record` skill, OpenCode `/ark:record`.
- G-12: Failure modes are explicit and audit-visible: pickaxe 0 → error with `--skip-slot-patch <slug>` escape; pickaxe >1 → ambiguous error with candidate list; journal moved/missing → error with recorded path. Every skip recorded in archive commit-message body and success summary.
- G-13: **Single-owner transactional primitives, all-or-rollback with one documented exception.** `workspace_record` owns its own `RecordTransaction` — snapshots taken inside the primitive before any mutation; on partial failure rolls back internally and returns `Err`; on success returns `RecordSummary` whose snapshot bytes higher layers can adopt. `CommitGuard` (extension of the existing `RollbackGuard`) composes by adopting the `RecordTransaction`'s success-path snapshots while keeping its existing `task.toml` + (deep) SPEC + features-INDEX coverage. `ArchiveTransaction` snapshots per-task journal/index bytes + intended `mv` paths. **Documented exception (R-204):** suffix-drift detected at journal-rollback time leaves the file intact and returns `JournalDriftDetected`; this preserves concurrent-appender data (V-UT-24, V-E-5) and is an intentional choice driven by NG-2, not an atomicity violation.
- G-14: Agent content delivered via `--entry-file <path>`. Slash command renders draft → agent edits → single `ark agent task commit --entry-file <path>` consumes it.
- G-15: Skip-slot-patch is auditable. Archive commit-message body lists skipped slugs (one per line, with reason code).
- G-16: `ark upgrade` scaffolds developer dir when `.ark/.developer` exists, in addition to the top-level workspace index.
- G-17: **`ark archive` requires a clean git index.** Precondition: `git diff --cached --quiet`. If the index is dirty, archive errors with `ArchiveIndexNotEmpty { staged_paths }` and a hint to commit or stash. This eliminates the unrelated-staged-work corruption risk from 01.
- G-18: **`ark context --scope record --format json`** projection returns identity + active journal path + journal_max_lines + session_count + branch as a `RecordProjection` payload inside the existing `ProjectedContext` envelope (additive — `ScopeTag::Record` discriminator). Consumed by `/ark:record`'s draft-render step. Text mode renders the same fields in a compact `Field: value` shape.
- G-19: **`CommitGuard` covers every Ark-managed closure artifact as one rollback set** (TR-1, R-201). The guard snapshots before *each* mutation: `task.toml` bytes (existing), promoted SPEC bytes / non-existence (existing), features-INDEX bytes (existing), `RecordSnapshot` adopted from successful `workspace_record` (new), staged-path tracker (new). On rollback, restores in reverse order: `git reset HEAD -- <staged>` → `RecordSnapshot::rollback` (each, reverse adoption order) → SPEC restore → features-INDEX restore → task.toml restore.

- NG-1: Squash-merge / as-merged-SHA recording — deferred to `task-finalize`.
- NG-2: Multi-developer concurrent-write coordination beyond `O_APPEND` — preserved via suffix-checked rollback (C-19), not violated.
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
├── entry_draft.rs      # parses `--entry-file` payload into EntryDraft
├── transaction.rs      # NEW — RecordTransaction (snapshot/rollback for journal+indices)
└── record.rs           # journal append + personal-index upsert
                          (composes RecordTransaction internally)

crates/ark-core/src/commands/
├── archive.rs          # *modified* — ArchiveTransaction + clean-index precondition
├── init.rs             # *modified* — wires --developer / --no-developer / prompt
├── context/            # *modified* — adds 'record' projection
└── agent/task/
    ├── new.rs          # *unchanged*
    ├── commit.rs       # *modified* — adopts RecordTransaction snapshots
    └── transaction.rs  # NEW — CommitTransaction (composes RecordTransaction)

crates/ark-core/src/commands/agent/state.rs
└── *modified* — TaskToml gains `journal_path: Option<String>`
```

**Snapshot ordering — the key invariant** (R-101 resolution):

```
Inside workspace_record:
  ├── 1. Resolve identity, config, active journal path.
  ├── 2. RecordTransaction::begin(journal_path, personal_idx_path, top_idx_path)
  │       → snapshots BEFORE any write:
  │         - journal byte-length (for length anchor)
  │         - personal_index bytes (full)
  │         - top_level_index bytes (full)
  ├── 3. Render entry bytes (kept in memory for suffix-check rollback).
  ├── 4. Append journal bytes via PathExt::append_text.
  │       ↪ on error: tx.rollback() (no-op since no write succeeded).
  ├── 5. Update personal_index ARK:SESSIONS block.
  │       ↪ on error: tx.rollback_journal_only() (suffix-check truncate).
  ├── 6. Update top_level_index ARK:DEVELOPERS block (developer_touch).
  │       ↪ on error: tx.rollback_journal_and_personal()
  │                   (suffix-check truncate journal, restore personal idx).
  └── 7. tx.commit() — record success; return RecordSummary { snapshots, ... }.

Then in task_commit (composing layer — CommitGuard, extension of existing RollbackGuard):
  ├── 1. EntryDraft::parse(--entry-file).
  ├── 2. CommitGuard::begin(task)              # rename of existing RollbackGuard
  │       → snapshot task.toml bytes (existing behavior).
  ├── 3. (deep) snapshot SPEC bytes / non-existence (existing).
  ├── 4. (deep) snapshot features-INDEX bytes (existing).
  ├── 5. (deep) spec_extract + spec_register   (existing).
  ├── 6. record_summary = workspace_record(...)  [self-protected per above]
  ├── 7. guard.adopt_record_snapshot(record_summary.into_snapshot())   # NEW
  ├── 8. Persist task.toml.
  ├── 9. git add <work + task.toml + (deep) SPEC + features-INDEX +
  │              workspace files>; guard.track_staged_path(p) for each.   # NEW
  ├── 10. git commit.
  │        ↪ on error in 5..10: guard.rollback() — git reset HEAD --
  │          tracked paths; reverse-order RecordSnapshot::rollback for
  │          each adopted snapshot (suffix-check truncate + restore
  │          indices); restore features-INDEX; restore SPEC; restore
  │          task.toml. SPEC + features-INDEX coverage is preserved
  │          from the existing RollbackGuard (R-201 fix).
  └── 11. guard.commit().
```

**`ark archive` boundary** (R-102 resolution — clean-index precondition):

```
ark archive [--skip-slot-patch <slug>]... [--dry-run]
  ├── Precondition: git diff --cached --quiet
  │     → if dirty: error ArchiveIndexNotEmpty { staged_paths }; halt.
  ├── ArchiveTransaction::begin()
  │     snapshots: per-task journal bytes + personal_index bytes + mv intent.
  ├── for each task with phase = committed:
  │     ├── (skip decisions per Failure Flow §3)
  │     ├── resolve_closing_sha (collect-then-classify, errors halt with rollback)
  │     ├── if sentinel absent: skip patch; else patch journal + personal_index in memory.
  │     ├── write patched files atomically (temp+rename).
  │     └── execute mv.
  ├── git add <patched journals + indices + moved task dirs>     [Ark-paths only]
  ├── git commit -m "chore(archive): bulk-archive N task(s)" + audit body.
  │     ↪ on error: ArchiveTransaction::rollback()
  │       — reverse all mvs, restore all journal/index bytes,
  │         git reset HEAD -- <ark-paths-only>.
  └── ArchiveTransaction::commit().
```

**Rollback primitives:**

- *Index rollback* (full-file): `io::fs::write_atomic(path, original_bytes)` — temp+rename (new helper, C-21). Used by `RecordTransaction::rollback`, `CommitGuard::rollback` index restores, and `archive::patch_slot` writes. *Not* `io::fs::write_file`, which is content-aware Skip/Force, not temp+rename (R-205 fix).
- *Journal rollback* (suffix-checked): `RecordTransaction::rollback_journal()` opens the journal read-only, reads the last `len(appended_bytes)` bytes, byte-compares with `appended_bytes`. If equal → `OpenOptions::new().write(true).open(path).set_len(snapshot_len)`. If unequal → error `JournalDriftDetected { path, expected_suffix_len, actual_len }`; file untouched (preserves concurrent appender's data).
- *Mv rollback*: reverse `std::fs::rename` on the same filesystem (atomic).
- *Git index rollback* (targeted): `git reset HEAD -- <ark-path-1> <ark-path-2> ...`. Touches only paths this transaction added; never touches user-staged paths because of the clean-index precondition.
- *SPEC + features-INDEX rollback* (existing in `RollbackGuard`, preserved): SPEC restore = `write_atomic(spec_path, snapshot_bytes)` if it pre-existed, else `fs::remove_file(spec_path)` if the snapshot recorded non-existence. features-INDEX restore = `write_atomic(features_index_path, snapshot_bytes)`. Reverse-order: features-INDEX → SPEC → task.toml.

[**Data Structure**]

```rust
// All public structs use private fields per S-21.
// Constructors and accessors omitted from the schematic for brevity;
// each `pub struct` shown below has a `pub fn new(...) -> Self` constructor
// and `pub fn <field>(&self) -> ...` accessors for its fields.

// crates/ark-core/src/commands/agent/workspace/identity.rs
pub struct Identity {
    name: String,
}
impl Identity {
    pub fn new(name: String) -> Result<Self>;          // validates non-empty
    pub fn name(&self) -> &str;
}

// crates/ark-core/src/commands/agent/workspace/config.rs
pub struct WorkspaceConfig {
    journal_max_lines: usize,
    developer: Option<String>,
}
impl WorkspaceConfig {
    pub fn load_or_default(project_root: &Path) -> Result<Self>;
    pub fn journal_max_lines(&self) -> usize;
    pub fn developer(&self) -> Option<&str>;
}

// crates/ark-core/src/commands/agent/workspace/entry_draft.rs
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

pub struct TaskHeader<'a> { /* private fields */ }
impl<'a> TaskHeader<'a> {
    pub fn new(
        session_number: u32,
        date: NaiveDate,
        slug: &'a str,
        branch: &'a str,
        base_branch: &'a str,
        start_head_short: &'a str,
        commits_in_range: &'a [(String, String)],
    ) -> Self;
}
pub struct ManualHeader<'a> { /* private fields */ }
impl<'a> ManualHeader<'a> {
    pub fn new(session_number: u32, date: NaiveDate, branch: &'a str) -> Self;
}

// crates/ark-core/src/commands/agent/workspace/transaction.rs (NEW)
pub struct RecordTransaction {
    journal_path: PathBuf,
    journal_byte_length_before: u64,
    appended_bytes: Vec<u8>,                   // for suffix-check rollback (R-103)
    personal_index_path: PathBuf,
    personal_index_bytes_before: Vec<u8>,
    top_level_index_path: PathBuf,
    top_level_index_bytes_before: Vec<u8>,
    state: TxState,
}

enum TxState { Open, JournalAppended, PersonalUpdated, TopLevelUpdated, Committed, RolledBack }

impl RecordTransaction {
    pub fn begin(
        journal_path: PathBuf,
        personal_index_path: PathBuf,
        top_level_index_path: PathBuf,
    ) -> Result<Self>;
    pub fn record_appended_bytes(&mut self, bytes: Vec<u8>);
    pub fn mark_journal_appended(&mut self);
    pub fn mark_personal_updated(&mut self);
    pub fn mark_top_level_updated(&mut self);
    pub fn rollback(self) -> Result<()>;       // suffix-check journal, restore indices in reverse order
    pub fn commit(self) -> RecordSnapshot;     // returns adopt-able snapshot
}

pub struct RecordSnapshot {                    // returned by commit() for higher-layer adoption
    journal_path: PathBuf,
    journal_byte_length_before: u64,
    appended_bytes: Vec<u8>,
    personal_index_path: PathBuf,
    personal_index_bytes_before: Vec<u8>,
    top_level_index_path: PathBuf,
    top_level_index_bytes_before: Vec<u8>,
}
impl RecordSnapshot {
    pub fn rollback(self) -> Result<()>;       // same suffix-check + restore-index logic
}

// crates/ark-core/src/commands/agent/workspace/record.rs
pub enum RecordMode<'a> {
    Task { slug: &'a str, entry: &'a EntryDraft },
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
    snapshot: RecordSnapshot,                  // adopt-able on success
}
impl RecordSummary {
    pub fn journal_path(&self) -> &Path;
    pub fn journal_path_relative(&self) -> &str;
    pub fn session_number(&self) -> u32;
    pub fn rotated(&self) -> bool;
    pub fn into_snapshot(self) -> RecordSnapshot;
}

// crates/ark-core/src/commands/agent/task/commit.rs (existing RollbackGuard, EXTENDED — renamed CommitGuard)
//
// The existing struct ALREADY holds:
//   - task_toml_snapshot: TaskToml (pre-mutation copy)
//   - spec_snapshot: Option<SpecSnapshot> (deep tier)
//   - features_index_snapshot: Option<FeaturesIndexSnapshot> (deep tier)
//   - commits_landed: u8 (0/1, controls git reset --soft HEAD~1)
//
// This task ADDS:
//   - adopted_record_snapshots: Vec<RecordSnapshot> (NEW)
//   - staged_paths: Vec<PathBuf> (NEW — tracks paths added by this guard's
//     git add for selective rollback)
//
pub struct CommitGuard {                     // renamed from RollbackGuard
    // ... existing fields (unchanged) ...
    adopted_record_snapshots: Vec<RecordSnapshot>,   // NEW
    staged_paths: Vec<PathBuf>,                      // NEW
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

// Internal `restore()` (existing private fn) gains two reverse-order steps
// AT THE TOP of the existing sequence:
//   0a. git reset HEAD -- <staged_paths>           (NEW)
//   0b. for each adopted_record_snapshots in reverse: snapshot.rollback()  (NEW)
//   1. git reset --soft HEAD~1 if commits_landed   (existing)
//   2. SPEC restore                                (existing)
//   3. features-INDEX restore                      (existing)
//   4. task.toml restore                           (existing)

// crates/ark-core/src/commands/archive.rs
pub struct ArchiveTransaction { /* private */ }
impl ArchiveTransaction {
    pub fn begin(project_root: &Path) -> Result<Self>;       // also runs clean-index check
    pub fn record_task(&mut self, snapshot: TaskArchiveSnapshot);
    pub fn record_skip(&mut self, skip: SkipRecord);
    pub fn rollback(self) -> Result<()>;                     // reverse mvs + restore bytes + targeted git reset
    pub fn commit_message_body(&self) -> String;
    pub fn commit(self);
}

pub struct SkipRecord {
    slug: String,
    reason: SkipReason,
}
impl SkipRecord {
    pub fn new(slug: String, reason: SkipReason) -> Self;
    pub fn slug(&self) -> &str;
    pub fn reason(&self) -> SkipReason;
}

#[derive(Clone, Copy, Debug)]
pub enum SkipReason {                          // intentionally transparent (S-21 exception via S-24)
    UserRequested,
    JournalPathAbsent,
    SentinelAlreadyFilled,
}

// crates/ark-core/src/commands/agent/state.rs (modified)
// TaskToml fields stay public for serde compat; existing convention.
pub struct TaskToml {
    // ... existing fields ...
    pub start_head: Option<String>,
    pub journal_path: Option<String>,          // NEW
}

// crates/ark-core/src/commands/context/projection.rs (modified — R-202 / R-206)
//
// Existing enum, NEW Record variant added:
pub enum Scope {
    Session,
    Phase(PhaseFilter),
    Record,                                    // NEW — for `--scope record`
}

pub enum ScopeTag {
    Session,
    Phase { phase: PhaseFilter },
    Record,                                    // NEW
}

// Existing struct, NEW additive field:
pub struct ProjectedContext {
    // ... existing fields (schema 1) ...
    pub scope: ScopeTag,
    pub record: Option<RecordProjection>,      // NEW — Some only when scope == Record
}

pub struct RecordProjection {                  // NEW
    pub identity: Option<String>,              // None if .ark/.developer absent
    pub active_journal_path: Option<String>,   // project-relative; None if no entries yet
    pub journal_max_lines: usize,
    pub session_count: u32,
    pub branch: Option<String>,
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
pub use transaction::{RecordSnapshot, RecordTransaction};

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

fn ensure_clean_index(project_root: &Path) -> Result<()>;   // R-102

// io/fs/mod.rs (NEW helper, R-205)
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()>;
//   ↪ writes <path>.<pid>.<rand>.tmp in same parent dir, then std::fs::rename.
//   ↪ atomic on the same filesystem (Posix rename guarantee).
//   ↪ NOT used to replace io::fs::write_file; that helper stays content-aware.
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
ark context --scope record [--format json]                  # NEW (R-202)
```

[**Constraints**]

- C-1: Journal append uses `PathExt::append_text` (existing `O_APPEND`); per-call atomicity is the OS guarantee.
- C-2: Sentinel format is exactly `<PENDING:<slug>>`.
- C-3: `**Slug**: <slug>` is the pickaxe anchor. Each task records exactly once; manual entries use `**Slug**: -`.
- C-4: Pickaxe runs *before* archive's git commit so the closing commit is reachable.
- C-5: Short SHA = 12 chars (`git rev-parse --short=12`).
- C-6: No parent-resolution. Journal lives on the task's branch.
- C-7: Managed-block markers reuse existing `io::fs::{read,update,remove,merge}_managed_block` API.
- C-8: `ark upgrade` scaffolds top-level index, adds `[workspace]` config, scaffolds developer dir if `.ark/.developer` exists.
- C-9: `task.toml.journal_path` is project-relative POSIX-style.
- C-10: `--no-commit` mode skips `workspace_record` entirely; `journal_path` stays None.
- C-11: Workspace files staged inside `CommitTransaction`; staged set tracked for rollback.
- C-12: Identity prompt re-prompts on blank + missing env (PR #9 fix).
- C-13: Journal scans sort descending; `scan_session_count` returns max-N.
- C-14: `WorkspaceConfig::load_or_default` reads only `[workspace]` via private `RawConfig`.
- C-15: `CommitTransaction::rollback` is reverse-order: unstage workspace paths → restore top-level index → restore personal index → suffix-check truncate journal → restore `task.toml`.
- C-16: `ArchiveTransaction::rollback` is reverse-order across tasks: undo any `mv` for the in-flight task, restore its journal/index, then prior tasks in reverse, then targeted `git reset HEAD -- <ark-paths>`.
- C-17: `--entry-file` parser is strict; missing required sections → `EntryFileMalformed`.
- C-18: Skip audit body uses one line per skipped slug: `skipped slot-patch: <slug> (<reason-code>)`.
- C-19: **Suffix-checked journal rollback.** `RecordTransaction` keeps the appended bytes in memory. On rollback: read the last `len(appended_bytes)` bytes from the journal; byte-compare; if match → `set_len(snapshot_len)`; if mismatch → `JournalDriftDetected` error, file untouched. Concurrent-appender data preserved. Implements NG-2 honestly.
- C-20: **`ark archive` requires a clean git index.** Runs `git diff --cached --quiet` at start; errors `ArchiveIndexNotEmpty { staged_paths }` with hint to commit/stash. Eliminates the path-name index-snapshot strategy from 01.
- C-21: **`io::fs::write_atomic(path, bytes)` is the helper for transaction restores and archive patch writes.** Implementation: write `<path>.<pid>.<rand>.tmp` in the same parent dir, `rename` over the original (atomic on the same filesystem). `rename` follows the Posix atomic-rename guarantee. Existing `io::fs::write_file` (content-aware Skip/Force) is unchanged and used for non-rollback paths. The `rename`-based atomicity is what `ArchiveTransaction`'s patched-file writes and `RecordTransaction`'s index restores rely on (R-205).
