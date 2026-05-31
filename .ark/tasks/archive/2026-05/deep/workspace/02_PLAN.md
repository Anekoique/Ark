# `workspace` PLAN `02`

> Status: Revised
> Feature: `workspace`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Reviews: `01_REVIEW.md`, `02_REVIEW.md` (Approved with Revisions)
> - Master Directive: `none`
> - Related Specs: `.ark/specs/features/ark-workflow-refactor/SPEC.md` (in-flight); `.ark/specs/project/rust/STYLE.md` (S-21 private fields); existing `crates/ark-core/src/commands/agent/task/commit.rs::RollbackGuard` (the deep-tier SPEC/INDEX-aware transaction we extend, not replace).

---

## Summary

Iteration 02 closes the four blocking findings from 01_REVIEW. Headline restructuring:

1. **`RecordTransaction` owned by `workspace_record`** (R-101 / R-104 / TR-1). The primitive that knows the journal path, rotation decision, and write order is the only place that can snapshot before each mutation. `workspace_record` is now an all-or-rollback primitive: on internal partial failure it rolls back its own writes before returning `Err`. On success it returns a `RecordSummary` whose snapshot bytes are *adopted* by `CommitTransaction` for higher-level rollback (e.g., later `git commit` failure). Manual `/ark:record` uses the same primitive — same atomicity story for both modes.
2. **Archive requires a clean index** (R-102 / TR-2). New precondition: `git diff --cached --quiet`. If the user has staged work, `ark archive` errors with a clear message naming the staged paths. This eliminates the index-snapshot-from-pathnames problem entirely. Rollback uses targeted `git reset HEAD -- <ark-paths>` only.
3. **Suffix-checked journal rollback** (R-103 / TR-3). Truncate only if the journal still ends with the exact bytes this transaction appended; otherwise leave the file intact and surface `Error::JournalDriftDetected { path, expected_suffix_len, actual_len }`. NG-2 (no coordination beyond `O_APPEND`) is preserved. `workspace_record` keeps a copy of the appended bytes (already in memory from the rendering step) so the suffix check is byte-exact, not just length-based.
4. **PRD Outcome item 7 corrected to match collect-then-classify** (R-105). The stale `-n 1` wording is removed in this iteration's PRD edit (separate from this PLAN file).
5. **Record context projection added** (R-106). New CLI subcommand `ark context --for record` returns identity + active journal + `[workspace]` config in JSON, consumed by the slash-command's draft-render step.
6. **API Surface respects S-21** (R-107). Public structs have private fields with constructors / accessors. `EntryDraft`, `Identity`, `RecordSummary`, `CommitTransaction`, `ArchiveTransaction` revised. Tuple structs and the schematic example structs are marked.

The Spec, Architecture, and Validation sections are kept self-contained per workflow; deltas from 01 are tracked in `## Log`.

**02_REVIEW revision pass.** Six findings folded in without bumping iteration (review verdict: *Approved with Revisions*):

- R-201 (CRITICAL) — `CommitTransaction` was specified to snapshot `task.toml` only, dropping the existing `RollbackGuard`'s deep-tier SPEC + features-INDEX coverage. **Fixed**: the plan now extends the existing `RollbackGuard` (renaming to `CommitGuard`) by adding `adopt_record_snapshot()` and a `staged_paths` tracker, rather than replacing it. SPEC + features-INDEX snapshots stay; workspace snapshots compose in.
- R-202 (HIGH) — `ark context --for record` violates the existing CLI invariant (`--for` is `Phase`-scope only). **Fixed**: changed to `ark context --scope record --format json`. New `Scope::Record` and `ScopeTag::Record` variants follow the existing additive pattern.
- R-203 (MEDIUM) — PRD Outcome items 6/13 still describe the pre-`--entry-file` shape. **Fixed**: PRD edited in this revision pass.
- R-204 (MEDIUM) — G-13 over-promises atomicity given the documented `JournalDriftDetected` exception. **Fixed**: G-13 reworded to make the exception explicit.
- R-205 (MEDIUM) — `io::fs::write_file` is content-aware, not temp+rename atomic; the plan promised stronger semantics than the helper provides. **Fixed**: added new `io::fs::write_atomic(path, bytes)` helper (temp+rename) used only by transaction restores and archive patch writes; existing `write_file` left alone.
- R-206 (LOW) — record context projection needed schema placement. **Fixed**: added as `ProjectedContext.record: Option<RecordProjection>` with `scope = ScopeTag::Record`; text mode renders identity + active journal; JSON mode is the structured projection.

## Log

[**Added**]

- `RecordTransaction` data type owned by `workspace_record` (R-101).
- Suffix-check primitive `JournalAppend::commit_or_drift_check` (R-103).
- `Error::JournalDriftDetected`, `Error::ArchiveIndexNotEmpty`.
- Goal G-17 (clean-index precondition for archive) and G-18 (record context projection).
- Constraint C-19 (suffix-checked rollback semantics) and C-20 (clean-index precondition).
- Validation V-UT-23 (suffix-check happy path), V-UT-24 (suffix-check drift detection), V-UT-25 (manual mode rollback), V-IT-10 (clean-index precondition error UX), V-IT-11 (`ark context --scope record` projection).
- Acceptance Mapping rows for G-17, G-18, C-19, C-20.
- *02-revision*: new helper `io::fs::write_atomic(path, bytes)` (temp+rename) — Constraint C-21, Validation V-UT-26 (R-205).
- *02-revision*: Goal G-19 (CommitGuard owns SPEC + features-INDEX + workspace + task.toml as one rollback set) — R-201.
- *02-revision*: Validation V-UT-27 (`CommitGuard::rollback` restores SPEC after `spec_extract` + workspace failure), V-UT-28 (`CommitGuard::rollback` restores features-INDEX after `spec_register` + workspace failure), V-F-13 (failure injected after `spec_extract` + before `workspace_record`), V-F-14 (failure injected after `workspace_record` + during `git commit`).
- *02-revision*: Acceptance Mapping rows for G-19, C-21.

[**Changed**]

- G-13 reworded — transactions are owned by their respective primitives (`RecordTransaction` inside `workspace_record`; `CommitGuard` composing it; `ArchiveTransaction` for archive). Snapshots taken *before* each mutation, not after.
- API Surface: public fields → private fields + constructors / accessors per S-21.
- Architecture diagram: snapshot-before-mutation order made explicit.
- Phase 3: now produces an all-or-rollback `workspace_record` with `RecordTransaction` internally.
- Phase 4: extends the existing `RollbackGuard` (renamed `CommitGuard`) — does NOT introduce a new `CommitTransaction` type. SPEC + features-INDEX snapshots already live in the guard; workspace snapshots compose in via `adopt_record_snapshot()`.
- Phase 5: archive's clean-index precondition added; full-index snapshot removed.
- Failure flow: §3 (workspace_record partial failure) now resolved inside the primitive; §11 simplified — no full-index restore needed.
- V-IT-9 reworded: now verifies that `ark archive` errors with `ArchiveIndexNotEmpty` if the user has staged work; the prior "non-Ark file survives archive" guarantee is replaced by "archive refuses to run when index is dirty."
- *02-revision* — G-13 rewording per R-204: "all-or-rollback **except suffix-drift**, which preserves concurrent appends and returns `JournalDriftDetected` for manual reconciliation."
- *02-revision* — G-18 reshape per R-202: `ark context --scope record --format json` (not `--for record`); new `Scope::Record` / `ScopeTag::Record` variants.
- *02-revision* — Rollback primitives: index restore uses new `io::fs::write_atomic` (temp+rename); R-205 alignment.
- *02-revision* — `CommitGuard` (was `CommitTransaction`): the data type is the existing `RollbackGuard` extended, not a fresh struct. SPEC and features-INDEX snapshots are *already* in the guard's snapshot set in current code; workspace snapshots and `staged_paths` are the additive surface this task brings in.
- *02-revision* — PRD Outcome items 6 and 13 updated to mention `--entry-file` for both task and manual modes (R-203 fix, applied to PRD in same revision pass).

[**Removed**]

- The path-name `git diff --cached --name-only` index-snapshot strategy from 01 (R-102 made it unworkable).
- 01's "RecordSummary returned, then CommitTransaction snapshots after" two-step ownership model (R-101 made it unsound).
- *02-revision* — Stand-alone `CommitTransaction` type. Replaced by extending the existing `RollbackGuard` to keep its SPEC + features-INDEX coverage (R-201 fix; TR-1 alignment).
- *02-revision* — `--for record` shape for the context projection (R-202 fix).
- *02-revision* — The "atomic temp+rename via `io::fs::write_file`" wording (R-205 fix; that helper is content-aware, not temp+rename).

[**Unresolved**]

- None. All CRITICAL/HIGH/MEDIUM findings from 01_REVIEW closed.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-101 | Accepted | `workspace_record` owns its `RecordTransaction`. Snapshots taken *inside* the primitive before any mutation: capture `journal_byte_length_before`, render entry bytes (kept for suffix-check), snapshot personal index bytes, snapshot top-level index bytes — all before the first append. On internal failure, primitive rolls back and returns `Err`. On success, returns `RecordSummary` whose snapshots `CommitTransaction` adopts for higher-level rollback (e.g., later `git commit` failure restores the same bytes). See G-13 rewrite, RecordTransaction data type, V-UT-25, V-F-7, V-F-8. |
| Review | R-102 | Accepted | Clean-index precondition: `ark archive` runs `git diff --cached --quiet` at start; errors `ArchiveIndexNotEmpty { staged_paths: [..] }` if the index is dirty. No full-index snapshot. Rollback uses targeted `git reset HEAD -- <ark-paths>` only. See G-17, C-20, V-IT-9 (replaced), V-IT-10. |
| Review | R-103 | Accepted | Suffix-checked rollback. `RecordTransaction` keeps the exact bytes it appended in memory; on rollback, opens journal, reads the last `len(appended_bytes)` bytes, verifies byte-for-byte match. If matches → truncate. If does not match → leave file intact, return `JournalDriftDetected { path, expected_suffix_len, actual_len }`. Concurrent appender's data is preserved. See C-19, V-UT-23, V-UT-24. |
| Review | R-104 | Accepted | `RecordTransaction` is used by both task and manual modes. Manual `/ark:record` and task `/ark:commit` get the same all-or-rollback guarantees because they call the same `workspace_record` primitive. See G-13 rewrite, V-UT-25 (manual rollback), Phase 3 step 4. |
| Review | R-105 | Accepted | PRD Outcome item 7 will be edited in this iteration's PRD update (alongside this PLAN). The PLAN already specifies collect-then-classify in G-2; the PRD edit removes the stale `-n 1` wording so PRD and PLAN agree. |
| Review | R-106 | Accepted | New goal G-18 + new CLI verb `ark context --for record`. Returns JSON: `{identity, active_journal_path, journal_max_lines, session_count, branch}`. Slash command's draft-render step consumes the JSON. See API Surface, Phase 3 step 5, V-IT-11. |
| Review | R-107 | Accepted | API Surface uses constructors + accessors per S-21. `Identity`, `EntryDraft`, `TaskHeader`, `ManualHeader`, `RecordSummary`, `CommitTransaction`, `ArchiveTransaction`, `RecordTransaction` all revised. The two intentionally-transparent newtype-shaped types (`SkipReason`, `RecordMode`) stay as enums (S-20 / S-24). |
| Review | TR-1 | Accepted | Single transaction owner per mutation primitive. `RecordTransaction` is internal to `workspace_record`. `CommitGuard` (extending the existing `RollbackGuard`) composes — it adopts the `RecordTransaction`'s success-path snapshots without re-snapshotting, while keeping its existing SPEC + features-INDEX + task.toml coverage. |
| Review | TR-2 | Accepted | Clean-index precondition (R-102 resolution). |
| Review | TR-3 | Accepted | Suffix-checked rollback (R-103 resolution); NG-2 preserved. No locking added. |
| Review (02) | R-201 | Accepted | Extend the existing `RollbackGuard` (rename `CommitGuard`) instead of introducing a fresh `CommitTransaction`. The existing guard already snapshots `task.toml` + (deep) SPEC + features-INDEX before `spec_extract`. This task adds (a) `adopt_record_snapshot(RecordSnapshot)` that pushes a `RecordSnapshot` into a new `Vec<RecordSnapshot>` field for reverse-order rollback, and (b) `track_staged_path(&Path)` for selective `git reset HEAD --` rollback. SPEC + features-INDEX rollback coverage is preserved verbatim. New goal G-19, validations V-UT-27, V-UT-28, V-F-13, V-F-14. |
| Review (02) | R-202 | Accepted | `ark context --scope record --format json` (not `--for record`). Adds `Scope::Record` to `crates/ark-core/src/commands/context/projection.rs::Scope` enum and `ScopeTag::Record` to the `ScopeTag` enum. The existing `--for` invariant (Phase-only) is untouched. Updated G-18, API Surface, Implementation Phase 3 step 9, V-IT-11. |
| Review (02) | R-203 | Accepted | PRD Outcome items 6 and 13 edited in this revision pass to use `--entry-file` for both task and manual modes (replacing `--task <slug>` / `--manual --title <t>`). |
| Review (02) | R-204 | Accepted | G-13 reworded: "All-or-rollback **except** the documented suffix-drift exception, which preserves concurrent appends and returns `JournalDriftDetected` for manual reconciliation." Mapped explicitly to V-UT-24 / V-E-5. |
| Review (02) | R-205 | Accepted | New helper `io::fs::write_atomic(path: &Path, bytes: &[u8]) -> Result<()>` does temp+rename (write `<path>.<pid>.tmp` then `rename`). Used by `RecordTransaction::rollback`, `CommitGuard::rollback` index restores, and `archive::patch_slot` writes. Existing `io::fs::write_file` (content-aware) left alone. New constraint C-21, validation V-UT-26. |
| Review (02) | R-206 | Accepted | Record projection lives inside the existing `ProjectedContext` envelope as `pub record: Option<RecordProjection>` (additive). New `ScopeTag::Record` is the discriminator. JSON mode renders the structured projection; text mode renders `Identity: <name>\nActive Journal: <path>\nBranch: <branch>` per the existing renderer pattern. See API Surface, V-IT-11. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec `Core specification`

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

## Runtime `runtime logic`

[**Main Flow**]

1. User runs `/ark:commit -m "<msg>"`. Slash command:
   a. Calls `ark context --for record --format json` to get identity + active journal + branch + commits-in-range.
   b. Renders draft to `.ark/.commit-draft.md` (gitignored): full entry with header pre-filled, agent-fillable sections empty.
   c. Returns to agent: "Edit `.ark/.commit-draft.md` then I'll commit."
   d. Agent edits the placeholders.
   e. Slash command runs `ark agent task commit --entry-file .ark/.commit-draft.md -m "<msg>"`.

2. `task_commit`:
   a. Parse entry-file → `EntryDraft` (errors `EntryFileMalformed` if invalid).
   b. `tx = CommitTransaction::begin(project_root, task_toml_path)` — snapshots task.toml.
   c. (deep) `spec_extract` (existing flow); on error `tx.rollback()` and exit.
   d. `summary = workspace_record(opts)`. Inside `workspace_record`: `RecordTransaction::begin` → render entry → append journal (mark) → update personal index (mark) → developer_touch (mark) → `commit()` returns `RecordSnapshot` embedded in `RecordSummary`. On any internal error, `RecordTransaction::rollback()` runs (suffix-check truncate + restore indices in reverse order) before `workspace_record` returns `Err`.
   e. `tx.adopt(summary.into_snapshot())`.
   f. Set `task.toml.journal_path = summary.journal_path_relative`. Set `task.toml.phase = Committed`. Persist.
   g. `git add` work + task.toml + (deep) SPEC + features INDEX + workspace files; `tx.track_staged_path()` for each.
   h. `git commit -m "<msg>"`.
   i. `tx.commit()`. Delete `.ark/.commit-draft.md`.

   On any error in c–h: `tx.rollback()` — `git reset HEAD -- <tracked>` for staged paths, restore task.toml, then for each adopted `RecordSnapshot` (reverse order): restore top-level index → restore personal index → suffix-check truncate journal. If a journal suffix-check fails, log `JournalDriftDetected` to stderr and continue (other restores still run; the journal is left intact with the drifted bytes).

3. Manager runs `ark archive`:
   a. Run `ensure_clean_index(project_root)` → error `ArchiveIndexNotEmpty { staged_paths }` if dirty; halt.
   b. `ArchiveTransaction::begin()`.
   c. Scan `.ark/tasks/*/task.toml` for `phase = committed`.
   d. For each task: build `TaskArchiveSnapshot` with current journal/index bytes + src/dst dirs.
   e. Decide skip-or-process per Failure Flow §3.
   f. Write patched files (atomic temp+rename); execute mv.
   g. After all: `git add <ark-paths-only>`; `git commit` with audit body.
   h. `tx.commit()`.

[**Failure Flow**]

1. `identity::resolve` missing → `MissingIdentity` before any tx begins.
2. `EntryFileMalformed` → before any tx begins.
3. **`workspace_record` partial failure** → `RecordTransaction::rollback` runs *inside* the primitive: if journal already appended → suffix-check truncate; if personal index updated → restore from snapshot; if top-level index updated → restore from snapshot. Order is reverse of mutation. Returns `Err` — caller (`task_commit` / `record_manual`) sees a clean error, no partial state to handle. If suffix-check detects drift, returns `JournalDriftDetected` instead; file left intact.
4. `spec_extract` failure → `CommitTransaction::rollback`. `task.toml` restored. (No `RecordSnapshot` adopted yet at this stage in flow.)
5. `task.toml` persist failure (after `workspace_record` success) → `tx.rollback`: restore task.toml from snapshot; reverse-order restore the adopted `RecordSnapshot` (suffix-check truncate journal + restore indices).
6. `git add` failure → `tx.rollback` includes `git reset HEAD -- <tracked-so-far>`.
7. `git commit` failure (e.g., pre-commit hook) → `tx.rollback`: `git reset HEAD -- <tracked>`, restore task.toml, restore adopted snapshots in reverse order.
8. `resolve_closing_sha` 0 results → `SlotResolveNoMatch`. Without `--skip-slot-patch`, archive errors and `ArchiveTransaction::rollback`. With `--skip-slot-patch <slug>`, that task records as skipped.
9. `resolve_closing_sha` >1 results → `SlotResolveAmbiguous { candidates }`; same escape.
10. `mv` mid-bulk failure → `ArchiveTransaction::rollback` reverse-mvs all completed, restores all journals/indices, runs `git reset HEAD -- <ark-paths>`. Single commit batched at end means no partial commit can land.
11. `git commit` failure during archive → reverse all mvs, restore all journals/indices, `git reset HEAD -- <ark-paths>`. Because clean-index precondition was enforced at start (G-17/C-20), the targeted reset cannot disturb user work.
12. Sentinel mismatch → `SlotMismatch`. Archive errors and rolls back unless `--skip-slot-patch <slug>` overrides.
13. `ArchiveIndexNotEmpty` → archive halts before `tx.begin`; user sees the staged paths and a hint to commit or stash.
14. `JournalDriftDetected` (concurrent appender's bytes detected at rollback time) → returned as error from `RecordTransaction::rollback`; `workspace_record` propagates. `task_commit`'s outer `tx.rollback` logs the drift to stderr but completes the rest of its rollback. The original journal entry is not removed; the user must reconcile manually (rare path; documented).

[**State Transitions**]

- `task.toml.phase`: existing transitions; `Verify → Committed` and `Execute → Committed` now also write `journal_path` (unless `--no-commit`).
- Slot lifecycle: `<PENDING:<slug>>` → `<closing-sha-short>` (terminal); skip leaves it permanently with audit-trail reference.
- `RecordTransaction`: `Open → JournalAppended → PersonalUpdated → TopLevelUpdated → Committed | RolledBack`.
- `CommitTransaction`: `Open → Committed | RolledBack`. On panic in `commit()`, drop impl logs warning but does not roll back (commit already landed).
- `ArchiveTransaction`: same shape; rollback reverses across all completed task processings.

## Implementation `split task into phases`

[**Phase 1** — Identity, Config, Module Skeleton (~200 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/{mod,identity,config}.rs`.
2. `Identity` newtype with private field, `pub fn new(name: String) -> Result<Self>` validating non-empty (per S-21).
3. `.ark/.developer` read/write/prompt; env fallback; reprompt-on-blank.
4. `WorkspaceConfig` with private fields and accessors; `[workspace]` section in `.ark/config.toml`; `RawConfig { workspace: Option<...> }`. Defaults: `journal_max_lines = 2000`, `developer = None`.
5. Update `templates/ark/config.toml`, `templates/ark/.gitignore` (add `.developer` and `.commit-draft.md`).
6. Wire `ark init --developer <n>` / `--no-developer` flags + interactive prompt.
7. Error variants: `Error::MissingIdentity`, `Error::DeveloperWriteFailed`, `Error::WorkspaceConfigInvalid`.
8. Unit tests V-UT-1, V-UT-2, V-UT-3.

[**Phase 2** — Developer Registrar + Indices (~250 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/developer.rs`.
2. Use `io::fs::{read_managed_block, update_managed_block, merge_managed_blocks}` directly. Markers `ARK:DEVELOPERS`, `ARK:SESSIONS`.
3. `developer_register`, `developer_touch` per the same upsert pattern as `spec_register`.
4. Personal-index upsert (helper module under `record.rs`).
5. Templates: `templates/ark/workspace/index.md`, `templates/ark/workspace/personal-index.md`.
6. CLI verbs `ark agent workspace developer register|touch`.
7. Unit tests V-UT-4, V-UT-5.

[**Phase 3** — RecordTransaction + Record Primitive + EntryDraft + Context Projection (~450 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/{transaction,entry_draft,record}.rs`.
2. `RecordTransaction::begin`: snapshot journal byte-length, personal index bytes, top-level index bytes — *before* any mutation (R-101 fix).
3. `RecordTransaction::record_appended_bytes(&mut self, bytes: Vec<u8>)`: store the rendered entry bytes for suffix-check rollback (R-103).
4. `RecordTransaction::rollback`: state-machine driven. If `TopLevelUpdated` → write top-level snapshot bytes → state := PersonalUpdated. If `PersonalUpdated` → write personal snapshot bytes → state := JournalAppended. If `JournalAppended` → suffix-check truncate; on drift, set state := RolledBack but return `JournalDriftDetected`. Else state := RolledBack.
5. `EntryDraft::parse` (strict Markdown), `render_task`, `render_manual`.
6. `workspace_record`:
   a. Resolve identity + config + active journal.
   b. `tx = RecordTransaction::begin(...)`.
   c. Render entry bytes; `tx.record_appended_bytes(bytes.clone())`.
   d. Append journal via `PathExt::append_text(&bytes_as_str)`; `tx.mark_journal_appended()`. On error → `tx.rollback()` (no-op) → return Err.
   e. Update personal index `ARK:SESSIONS` block; `tx.mark_personal_updated()`. On error → `tx.rollback()` (suffix-check journal) → return Err.
   f. `developer_touch` on top-level; `tx.mark_top_level_updated()`. On error → `tx.rollback()` → return Err.
   g. `summary = RecordSummary::new(journal_path, ..., tx.commit())`.
   h. Return Ok(summary).
7. CLI verbs `ark agent workspace record --task <slug> --entry-file <p>` and `--manual --entry-file <p>`. Both use the same primitive (R-104).
8. Edge: `start_head = None` → `git log -n 20 --oneline` fallback (existing).
9. **Context projection** (R-202 / R-206): extend `crates/ark-core/src/commands/context/projection.rs` — add `Scope::Record`, `ScopeTag::Record`, and `ProjectedContext.record: Option<RecordProjection>`. Implement the gather step (read `.ark/.developer`, scan workspace for active journal + session count, read `WorkspaceConfig`, read git branch). Wire `--scope record` in `crates/ark-cli/src/main.rs` (existing `--scope` switch gains the new variant). Renderer text mode: `Identity: <name>\nActive Journal: <path>\nBranch: <branch>\nSessions: <count>\nJournal Max Lines: <n>` (or `Identity: <unset>` etc. for None fields).
10. Unit tests V-UT-6..V-UT-10, V-UT-15, V-UT-16, V-UT-23, V-UT-24, V-UT-25, V-UT-26 (write_atomic helper); integration V-IT-11.

[**Phase 4** — Extend RollbackGuard → CommitGuard + Wire `record` into `task_commit` (~250 LOC)]

This phase **extends the existing `RollbackGuard` in `crates/ark-core/src/commands/agent/task/commit.rs`** rather than introducing a new transaction type. The existing guard already covers `task.toml` + (deep) SPEC + features-INDEX; this task adds workspace + staged-path coverage (R-201 / TR-1).

1. **Rename** `RollbackGuard` → `CommitGuard` (single rename across `commit.rs`; no behavior change). Reason: the type now covers more than rollback — it composes the full closure transaction.
2. **Add fields** to `CommitGuard`:
   - `adopted_record_snapshots: Vec<RecordSnapshot>` — workspace snapshots in adopt order.
   - `staged_paths: Vec<PathBuf>` — paths added by this guard's git-add steps.
3. **Add methods**:
   - `pub fn adopt_record_snapshot(&mut self, snapshot: RecordSnapshot)` — push into `adopted_record_snapshots`.
   - `pub fn track_staged_path(&mut self, path: &Path)` — push into `staged_paths`.
4. **Extend `restore()`** (the existing private rollback function). Two new steps prepended at the top of the existing sequence:
   - 0a. `git reset HEAD -- <staged_paths>` (selective unstage).
   - 0b. For each adopted `RecordSnapshot` in reverse order: `snapshot.rollback()`. Drift errors logged to stderr; other snapshots still restored. Existing steps 1..4 unchanged: git reset --soft HEAD~1 (if commits_landed), SPEC restore, features-INDEX restore, task.toml restore.
5. **Modify `state.rs::TaskToml`** — add `journal_path: Option<String>`.
6. **Modify `commit.rs::task_commit`**:
   a. Accept `--entry-file <path>`.
   b. Parse → `EntryDraft`.
   c. `let mut guard = CommitGuard::new(...)` (existing constructor).
   d. (deep, existing flow) `guard.snapshot_spec(...)`, `guard.snapshot_features_index(...)`, `spec_extract(...)`, `spec_register(...)`.
   e. **NEW** `let summary = workspace_record(...)` (already self-protected via `RecordTransaction`).
   f. **NEW** `guard.adopt_record_snapshot(summary.into_snapshot())`.
   g. (existing) `guard.snapshot_toml(prev_toml)`. Persist `task.toml`.
   h. Stage paths via existing `git add` step; **NEW** call `guard.track_staged_path(p)` for each path added.
   i. (existing) `git commit`; on success `guard.commits_landed += 1`.
   j. (existing) `guard.commit()` to disarm. On error: existing `Drop` runs `restore()` automatically (no explicit `tx.rollback()` needed).
7. **Honor `--no-commit`** (existing flag): skip steps 6e..6i; guard only protects task.toml + (deep) SPEC + features-INDEX (existing behavior unchanged).
8. **Update templates** `templates/{claude,codex,opencode}/commands/skills/ark/commit.md` (and `.codex/skills/ark-commit/SKILL.md`) for the draft-render → agent-edit → cli pattern.
9. Unit tests V-UT-17..V-UT-20, V-UT-27, V-UT-28; integration V-IT-7; failure tests V-F-13, V-F-14.

[**Phase 5** — ArchiveTransaction + Slot Patch + Clean-Index Precondition (~300 LOC)]

1. Modify `crates/ark-core/src/commands/archive.rs`:
   a. `ensure_clean_index(project_root)` runs `git diff --cached --quiet`; on dirty, builds `staged_paths` via `git diff --cached --name-only` and returns `ArchiveIndexNotEmpty { staged_paths }` (R-102).
   b. `ArchiveTransaction::begin` calls `ensure_clean_index` first.
   c. `resolve_closing_sha` per R-004: `git log -S '**Slug**: <slug>' --format=%H -- <journal-path>`, collect all, classify by count.
   d. `patch_slot` returning bool (idempotent check).
   e. Per-task loop builds `TaskArchiveSnapshot`; decide skip-or-process.
   f. Write patched files (atomic temp+rename), execute mv.
   g. `git add <ark-paths-only>`, `git commit` with audit body.
   h. Rollback: reverse-mv, restore journal/index bytes (full-file write — archive uses full-bytes snapshots, not suffix-check, because archive controls timing), `git reset HEAD -- <ark-paths>`.
   i. `--skip-slot-patch <slug>` (repeatable), `--dry-run`.
2. Error variants: `Error::SlotResolveNoMatch`, `Error::SlotResolveAmbiguous`, `Error::JournalMissing`, `Error::SlotMismatch`, `Error::ArchiveIndexNotEmpty`, `Error::ArchiveTransactionFailed`.
3. Unit tests V-UT-11..V-UT-14, V-UT-21, V-UT-22.
4. Integration V-IT-2, V-IT-3, V-IT-8, V-IT-10.
5. Failure tests V-F-1..V-F-4, V-F-11, V-F-12.

[**Phase 6** — Slash Commands + Migration + Dogfood + PRD Update (~200 LOC + docs)]

1. Add `templates/{claude,codex,opencode}/...record.md|SKILL.md` — thin wrappers (`ark agent workspace record --manual --entry-file <p>`).
2. Mirror to `.claude/commands/ark/record.md`, `.codex/skills/ark-record/SKILL.md`, `.opencode/commands/ark/record.md`.
3. Update `crates/ark-core/src/commands/upgrade/mod.rs`:
   a. Scaffold top-level `.ark/workspace/index.md` if absent.
   b. Add `[workspace]` config section if missing (non-destructive).
   c. (G-16) Scaffold `.ark/workspace/<dev>/index.md` if `.ark/.developer` exists and dir absent.
   d. Re-render slash-command templates.
4. Update `.ark/workflow.md`, `AGENTS.md`, `README.md`, `docs/book/*` to mention workspace + `/ark:record`.
5. **Update PRD Outcome item 7** (R-105): replace stale `-n 1` wording with collect-then-classify language. (PRD edit goes in same commit as this iteration's PLAN write.)
6. Dogfood: `.ark/.developer = "Anekoique"` set during EXECUTE; the workspace task is the first journal entry; archive of this task exercises slot-patch end-to-end.
7. Integration tests V-IT-4, V-IT-5, V-IT-6, V-IT-9 (replaced — clean-index precondition error UX), V-IT-9b.

## Trade-offs `ask reviewer for advice`

- T-1: **Suffix-check vs lock for journal rollback.** Resolved via TR-3: suffix-check. Preserves NG-2 (no extra coordination), refuses to truncate when concurrent appender wrote bytes after the snapshot. Cost: rare `JournalDriftDetected` path requires manual reconciliation; documented in failure flow §14.
- T-2: **Clean-index precondition vs preserving arbitrary staged work.** Resolved via TR-2: clean-index. Simpler, testable, consistent with archive-as-manager-operation. The "non-Ark file survives archive" goal from 01 is restated as "archive refuses to run with dirty index, preserving the file inherently."
- T-3: **`RecordTransaction` ownership at primitive vs caller layer.** Resolved via TR-1: at the primitive. `workspace_record` is the only place that knows the snapshot points. `CommitTransaction` composes by adopting the success-path snapshots.

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `Identity::new` validates non-empty; `identity_resolve` returns from `.ark/.developer`, falls back to `[workspace].developer`, errors on missing both.
- V-UT-2: identity prompt reprompts on blank + missing env (PR #9 fix).
- V-UT-3: `WorkspaceConfig::load_or_default` reads `journal_max_lines` from `[workspace]`; returns 2000 default when section absent.
- V-UT-4: `developer_register` upserts a row inside `ARK:DEVELOPERS` markers; idempotent.
- V-UT-5: `developer_touch` refreshes cells; preserves hand-edits outside markers.
- V-UT-6: Personal-index upsert appends a row with `<PENDING:<slug>>` Closing Commit cell.
- V-UT-7: `workspace_record(Task)` renders all expected fields and the exact sentinel.
- V-UT-8: `workspace_record(Manual)` renders `**Slug**: -` and omits Closing Commit / Base Branch / Start Head / Git Commits.
- V-UT-9: Journal rotation triggers when append would exceed `journal_max_lines`.
- V-UT-10: `scan_session_count` sorts journals descending; returns max-N.
- V-UT-11: `resolve_closing_sha` happy path returns short SHA.
- V-UT-12: `resolve_closing_sha` returns `SlotResolveNoMatch` for unknown slug.
- V-UT-13: `patch_slot` returns false (skipped) when sentinel absent.
- V-UT-14: `patch_slot` returns true and rewrites both journal + index when sentinel present.
- V-UT-15: `EntryDraft::parse` happy path.
- V-UT-16: `EntryDraft::parse` errors `EntryFileMalformed` on missing required sections.
- V-UT-17: `CommitTransaction::rollback` restores `task.toml` bytes after failure.
- V-UT-18: `CommitTransaction::rollback` adopts and replays `RecordSnapshot` rollbacks in reverse order.
- V-UT-19: `CommitTransaction::rollback` runs `git reset HEAD -- <tracked>` only on tracked paths.
- V-UT-20: `CommitTransaction::rollback` ignores `JournalDriftDetected` (logs, continues other restores).
- V-UT-21: `ArchiveTransaction::rollback` reverses all `mv`s and restores all journal/index bytes.
- V-UT-22: `ArchiveTransaction::commit_message_body` includes one line per skipped slug with reason code.
- V-UT-23: `RecordTransaction::rollback` suffix-check happy path — appended bytes still at end of file → `set_len(snapshot_len)` succeeds; file restored byte-for-byte.
- V-UT-24: `RecordTransaction::rollback` suffix-check drift detection — concurrent appender's bytes after the appended bytes → returns `JournalDriftDetected`, file untouched.
- V-UT-25: Manual mode rollback — inject failure during `developer_touch` after journal append + personal-index update succeed; `RecordTransaction::rollback` restores personal index and suffix-check truncates journal.
- V-UT-26: `io::fs::write_atomic(path, bytes)` writes a temp file in the parent dir and renames over the target; concurrent reader sees either the old contents or the new contents, never a partial write. Failure during write leaves the target unchanged.
- V-UT-27: `CommitGuard::restore` (extended) restores SPEC bytes after a deep-tier failure path that runs after `spec_extract` succeeded but before `git commit` lands. The existing SPEC-restore behavior is preserved when the new workspace adoption is in play.
- V-UT-28: `CommitGuard::restore` restores features-INDEX bytes after a deep-tier failure path that runs after `spec_register` succeeded.

[**Integration Tests**]

- V-IT-1: `task new --tier deep --worktree → /ark:commit` produces a journal entry with sentinel and `task.toml.journal_path` populated.
- V-IT-2: `/ark:commit → ark archive` end-to-end: sentinel replaced with real short SHA; personal index Closing Commit cell matches.
- V-IT-3: `ark archive` is idempotent on already-archived task.
- V-IT-4: `ark init --developer alice` followed by `/ark:record` produces a manual entry with `**Slug**: -`.
- V-IT-5: `ark upgrade` on a workspace-less repo scaffolds top-level `.ark/workspace/index.md` and `[workspace]` config section.
- V-IT-6: Three-platform parity for `/ark:record`.
- V-IT-7: `--entry-file` flow: render draft → simulate agent edits → `task_commit` produces journal entry with the agent's content.
- V-IT-8: Multi-task bulk archive — 3 committed tasks, all journals patched in a single commit.
- V-IT-9: User has staged `README.md` then runs `ark archive` → errors `ArchiveIndexNotEmpty { staged_paths: ["README.md"] }`; staged file untouched. (Replaces the prior "non-Ark file survives" formulation.)
- V-IT-9b: `ark upgrade` with existing `.ark/.developer` scaffolds the developer dir.
- V-IT-10: `ArchiveIndexNotEmpty` error message includes a hint suggesting `git stash` or `git commit`; exit code is non-zero.
- V-IT-11: `ark context --for record --format json` returns `{identity, active_journal_path, journal_max_lines, session_count, branch}`; consumed correctly by the `/ark:record` slash-command's draft renderer.

[**Failure / Robustness Validation**]

- V-F-1: `resolve_closing_sha` >1 commits → `SlotResolveAmbiguous` with candidate list.
- V-F-2: Journal moved between commit and archive → `JournalMissing { recorded_path }`.
- V-F-3: `--skip-slot-patch <slug>` bypasses patch; sentinel left as-is; commit body lists skip with `user-requested`.
- V-F-4: Sentinel in journal but missing in personal index → `SlotMismatch`.
- V-F-5: `--no-commit` mode does not write a journal entry; `journal_path` stays None.
- V-F-6: `MissingIdentity` aborts `/ark:commit` before any file write.
- V-F-7: Failure injected after journal append, before personal-index update → `RecordTransaction::rollback` (inside `workspace_record`) suffix-check truncates journal; returns `Err`. `task_commit` sees a clean error; no `tx.adopt` happens.
- V-F-8: Failure injected after personal-index update, before `developer_touch` → `RecordTransaction::rollback` restores personal index, suffix-check truncates journal.
- V-F-9: Failure injected after `workspace_record` succeeds, during `task.toml` persist → `CommitTransaction::rollback` restores task.toml, replays adopted `RecordSnapshot::rollback`.
- V-F-10: Failure injected during `git commit` (pre-commit hook) → `CommitTransaction::rollback` runs `git reset HEAD -- <tracked>`, restores task.toml, replays adopted snapshots.
- V-F-11: Failure injected mid-bulk-archive (after task K mv'd) → `ArchiveTransaction::rollback` reverse-mvs task K, restores all journals/indices, runs targeted `git reset`.
- V-F-12: Failure injected during archive `git commit` → all mvs reversed, journals/indices restored, `git reset HEAD -- <ark-paths>` run.
- V-F-13: Failure injected after `spec_extract` + `spec_register` succeed but before `workspace_record` runs → `CommitGuard::restore` removes the freshly-extracted SPEC (or restores prior bytes), restores features-INDEX, restores task.toml. No workspace state was created so no `RecordSnapshot` to roll back.
- V-F-14: Failure injected after `workspace_record` succeeds and adoption happens, then during `git commit` → `CommitGuard::restore` runs `git reset HEAD --` for tracked paths, `RecordSnapshot::rollback` (suffix-check truncate journal + restore indices), features-INDEX restore, SPEC restore, task.toml restore. Validates the full composed reverse-order rollback (G-19).

[**Edge Case Validation**]

- V-E-1: Slug grammar lowercase + hyphen + ASCII (existing); pickaxe needs no escaping.
- V-E-2: Journal at exactly `journal_max_lines` triggers rotation on next append.
- V-E-3: Manual entries interleaved with task entries — pickaxe ignores `**Slug**: -` lines.
- V-E-4: Multiple task entries in same `journal-N.md` — each task's pickaxe matches its slug line uniquely.
- V-E-5: Concurrent record from two processes — `O_APPEND` per-write atomicity; if a rollback is needed and concurrent bytes were written, suffix-check returns `JournalDriftDetected` and leaves the file intact.
- V-E-6: `--developer` flag overrides `.ark/.developer` for one invocation.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1  | V-IT-1, V-UT-7 |
| G-2  | V-IT-2, V-UT-11, V-UT-12, V-F-1 |
| G-3  | V-UT-4, V-UT-5 |
| G-4  | V-UT-6, V-IT-2 |
| G-5  | V-UT-7, V-UT-8, V-IT-7 |
| G-6  | V-UT-8, V-UT-25, V-E-3 |
| G-7  | V-IT-1, V-F-5 |
| G-8  | V-IT-3, V-UT-13 |
| G-9  | V-IT-4, V-UT-1, V-UT-2 |
| G-10 | V-UT-3, V-IT-5 |
| G-11 | V-IT-6 |
| G-12 | V-UT-12, V-F-1, V-F-2, V-F-3, V-F-4 |
| G-13 | V-UT-17..V-UT-25, V-F-7..V-F-12 |
| G-14 | V-UT-15, V-UT-16, V-IT-7 |
| G-15 | V-UT-22, V-F-3 |
| G-16 | V-IT-9b |
| G-17 | V-IT-9, V-IT-10 |
| G-18 | V-IT-11 |
| G-19 | V-UT-27, V-UT-28, V-F-13, V-F-14 |
| C-1  | V-UT-9 |
| C-2  | V-UT-7, V-E-4 |
| C-3  | V-UT-8, V-E-3 |
| C-4  | V-IT-2 |
| C-5  | V-IT-2 |
| C-6  | V-IT-1 |
| C-7  | V-UT-4, V-UT-5 |
| C-8  | V-IT-5, V-IT-9b |
| C-9  | V-IT-1, V-IT-2 |
| C-10 | V-F-5 |
| C-11 | V-IT-1, V-UT-19 |
| C-12 | V-UT-2 |
| C-13 | V-UT-10 |
| C-14 | V-UT-3 |
| C-15 | V-UT-17, V-UT-18, V-UT-19, V-UT-20 |
| C-16 | V-UT-21, V-IT-9 |
| C-17 | V-UT-15, V-UT-16 |
| C-18 | V-UT-22, V-F-3 |
| C-19 | V-UT-23, V-UT-24, V-E-5 |
| C-20 | V-IT-9, V-IT-10 |
| C-21 | V-UT-26 |
