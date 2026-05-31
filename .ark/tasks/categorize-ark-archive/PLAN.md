# `categorize-ark-archive` PLAN

> Status: Draft
> Feature: `categorize-ark-archive`
> Iteration: `0`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none

---

## Summary

Insert a `<tier>/` level into the archive layout so closed tasks live at
`tasks/archive/<YYYY-MM>/<tier>/<slug>/` instead of `tasks/archive/<YYYY-MM>/<slug>/`.
A `Tier::dir_name()` helper supplies the segment; the single write site
(`task_archive_move`) and the two read sites (`context::gather_archive`,
`cleanup::enumerate_archived`) gain one extra directory level of walking.
`ark archive --dry-run` path string is updated in lockstep. The 32 existing
archived dirs are migrated by an `ark archive`-equivalent move (done as part of
EXECUTE, by hand via the same path math, not a new CLI verb). A generated
`tasks/archive/INDEX.md` groups every archived task by tier, and `workflow.md`'s
layout convention is updated.

## Log `None in 00_PLAN`

[**Added**]

- N/A (iteration 0)

[**Changed**]

- N/A

[**Removed**]

- N/A

[**Unresolved**]

- N/A

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| — | — | — | (no prior review; standard tier) |

---

## Spec

[**Goals**]

- G-1: Archive tasks at `tasks/archive/<YYYY-MM>/<tier>/<slug>/`.
- G-2: `ark archive` writes the tier-segmented path for new archives.
- G-3: `ark context` and `ark cleanup` read the tier-segmented layout.
- G-4: `tasks/archive/INDEX.md` lists every archived task grouped by tier.
- G-5: Migrate all existing archived tasks into the new layout.
- G-6: `ark archive` regenerates `tasks/archive/INDEX.md` after a real run.

[**Non-goals**]

- NG-1: No change to `task.toml` schema or to how tier is recorded.
- NG-2: `--dry-run` does not rewrite `INDEX.md` (stays read-only).

[**Architecture**]

```
crates/ark-core/src/
├── commands/agent/state.rs        # + Tier::dir_name() -> &'static str
├── commands/agent/task/archive.rs # task_archive_move: insert <tier> segment
├── commands/archive.rs            # --dry-run dest path: insert <tier> segment;
│                                  #   + regenerate INDEX.md after a real run
├── commands/archive_index.rs      # NEW — render_archive_index + write_archive_index
├── commands/context/gather.rs     # gather_archive: walk month/<tier>/<slug>
└── commands/cleanup.rs            # enumerate_archived: walk month/<tier>/<slug>
.ark/tasks/archive/
├── INDEX.md                       # tier-grouped index (generated; rewritten by ark archive)
└── <YYYY-MM>/<tier>/<slug>/       # migrated layout
.ark/workflow.md                   # layout convention line updated
```

[**Data Structure**]

```rust
// state.rs — no new types; one helper on the existing enum.
impl Tier {
    /// Lowercase directory segment for the archive layout.
    pub fn dir_name(self) -> &'static str; // "quick" | "standard" | "deep" | "research"
}
```

[**API Surface**]

```rust
// archive.rs — TaskArchiveMoveSummary.archive_path now ends in
// <month>/<tier>/<slug>; no signature change. Options unchanged
// (tier is read from the task's own task.toml, not passed in).
pub fn task_archive_move(opts: TaskArchiveMoveOptions) -> Result<TaskArchiveMoveSummary>;

// archive_index.rs — pure renderer + writer, driven off the on-disk tree.
pub fn render_archive_index(layout: &Layout) -> Result<String>; // full Markdown body
pub fn write_archive_index(layout: &Layout) -> Result<()>;      // render + atomic write to archive/INDEX.md
```

[**Constraints**]

- C-1: @source-scan: `join(&opts.archive_month).join(&opts.slug)` @ `crates/ark-core/src/**/*.rs`
  No archive path is built as `<month>/<slug>`; the `<tier>` segment sits between them.
- C-2: @test-binding: ark_archive_writes_tier_segment
  `task_archive_move` places a task at `archive/<month>/<tier>/<slug>/`, tier from its `task.toml`.
- C-3: @test-binding: gather_archive_reads_tier_layout
  `ark context` lists an archived task stored under the three-level tier layout.
- C-4: @test-binding: enumerate_archived_reads_tier_layout
  `ark cleanup` membership set includes a slug stored under the three-level tier layout.
- C-5: @judgment
  `INDEX.md` contains every archived slug exactly once under its recorded tier; layout dirs unchanged.
- C-6: @test-binding: ark_archive_regenerates_index
  A real `ark archive` run rewrites `archive/INDEX.md` to match the post-move tree; `--dry-run` does not.

---

## Runtime

[**Main Flow**]

1. `task_archive_move` loads `task.toml`, reads `toml.tier`, builds
   `tasks_archive_dir()/<month>/<tier.dir_name()>/<slug>`, ensures parents, renames.
2. `ark context` / `ark cleanup` walk three levels (month → tier → slug) instead of two.
3. `ark archive`, after all moves succeed (real run only), calls `write_archive_index`
   which renders the whole `INDEX.md` from the on-disk tree and atomically writes it.

[**Failure Flow**]

1. Same-slug collision in destination → `Error::TaskAlreadyExists` (unchanged path-string, now tier-qualified).
2. Migration of an existing dir whose destination already exists → skip + report (idempotent).

[**State Transitions**]

- Archive dir: `<month>/<slug>/` → `<month>/<tier>/<slug>/` (one-time migration + all future writes).

---

## Implementation

[**Phase 1 — code: write + read sites]**

- Add `Tier::dir_name()` to `state.rs`.
- `task_archive_move`: `archive_parent = tasks_archive_dir().join(&month).join(tier.dir_name())`.
- `archive.rs` `--dry-run`: same insertion for the planned dest string.
- `gather_archive` + `enumerate_archived`: add one nested `list_dir()` loop (month → tier → slug).
- Update existing tests that assert `archive/2026-05/<slug>` to the tier-qualified path; add C-2/C-3/C-4 tests.

[**Phase 2 — migrate the 32 existing dirs]**

- For each `archive/<month>/<slug>/`, read its `task.toml` tier, move to `archive/<month>/<tier>/<slug>/`.
- Use `git mv` via the sanctioned path? No — Rust code uses `PathExt::rename_to`; the migration here is a one-off
  performed in the working tree with the build's own move semantics (documented in VERIFY), then `git add -A`.

[**Phase 3 — INDEX.md + docs]**

- Generate `tasks/archive/INDEX.md`: per-tier section (deep/standard/quick/research), count header,
  rows `| Month | [slug](relative/path) | Title |` sorted by month then slug.
- Update `workflow.md` line 64 (`├── tasks/archive/YYYY-MM/`) and §5 reopen note to the tier-qualified layout.

[**Phase 4 — auto-regenerate INDEX.md in `ark archive`]**

- New `commands/archive_index.rs`: `render_archive_index(layout)` produces the exact same Markdown
  the static generator did, driven off the on-disk tree (walk month → tier → slug, read each
  `task.toml` for title; sort sections deep/standard/quick/research, rows month-then-slug; omit empty
  tiers). `write_archive_index(layout)` renders + `write_atomic` to `archive/INDEX.md`.
- `ark_archive`: after the move loop, on a real (non-dry-run) run, call `write_archive_index(&layout)`.
  Dry-run returns before any write (NG-2).
- Tests: `render_archive_index_matches_tree`, `ark_archive_regenerates_index` (C-6),
  `ark_archive_dry_run_does_not_write_index`.

---

## Trade-offs

- T-1: `<month>/<tier>/<slug>` (chosen) vs `<tier>/<month>/<slug>` vs flat `<tier>/<slug>`.
  Month-first keeps chronology as the primary axis (matches today's mental model and the
  `committed_at`-derived bucket), adds tier as a secondary fan-out. Tier-first would reorder the
  whole tree; flat would drop chronology. Month-first is the least disruptive to existing contracts.
- T-2: `INDEX.md` generated once vs auto-maintained by `ark archive` (chosen: auto). A static index
  goes stale the next time a task is archived, putting the burden on the user to remember to
  regenerate. Auto-regeneration in `ark archive` makes the index a guaranteed-correct projection of
  the tree at the cost of a new write path + tests. Full-rewrite (not a managed block) is chosen
  because the whole file is generated content — there is no user-authored region to preserve.
  `--dry-run` is exempt to keep its read-only contract (NG-2).
- T-3: `Tier::dir_name()` helper vs inlining `serde`/`format!("{:?}").to_lowercase()`. A named helper
  is the single source of truth and avoids `Debug`-format coupling.

---

## Validation

[**Unit Tests**]

- V-UT-1: `Tier::dir_name()` returns `quick`/`standard`/`deep`/`research`.
- V-UT-2: `ark_archive_writes_tier_segment` — moved task lands at `<month>/<tier>/<slug>/`.
- V-UT-3: `render_archive_index_matches_tree` — renderer output reflects a seeded multi-tier tree.

[**Integration Tests**]

- V-IT-1: `gather_archive_reads_tier_layout` — `ark context` surfaces a task in the new layout.
- V-IT-2: `enumerate_archived_reads_tier_layout` — cleanup membership includes a tier-layout slug.
- V-IT-3: `ark_archive_regenerates_index` — a real run writes `INDEX.md` listing the just-archived task.

[**Failure / Robustness**]

- V-F-1: re-archiving / re-migrating an already-moved slug is a no-op (no panic, reported skip).

[**Edge Cases**]

- V-E-1: empty archive → `INDEX.md` lists zero rows per tier; gather/cleanup return empty sets (no panic).
- V-E-2: `ark_archive_dry_run_does_not_write_index` — `--dry-run` leaves `INDEX.md` untouched/absent.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-2, V-F-1 |
| G-2 | V-UT-2 |
| G-3 | V-IT-1, V-IT-2 |
| G-4 | C-5 (manual: open INDEX.md), V-E-1 |
| G-5 | C-5 (manual: 32 dirs moved), smoke test |
| G-6 | V-IT-3, V-E-2 |
| C-1 | source scan after edit |
| C-2 | V-UT-2 |
| C-3 | V-IT-1 |
| C-4 | V-IT-2 |
| C-5 | manual review of INDEX.md + tree |
| C-6 | V-IT-3, V-E-2 |
