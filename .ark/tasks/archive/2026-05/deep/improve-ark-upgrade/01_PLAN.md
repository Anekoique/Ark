# `ark-upgrade` PLAN `01`

> Status: Revised
> Feature: `ark-upgrade`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

Extend `ark upgrade` with a user-declared per-path file strategy read from a new `[upgrade]` section of `.ark/config.toml`. Two strategies layer on top of the existing pipeline: `ejected` (Ark never touches the path — beats every policy including `--force`) and `merged` (a non-managed-block file changed on both sides is diff3-merged against a recorded base). The merge base is the exact bytes Ark last wrote, kept in a gitignored sidecar (`.ark/.upgrade-base/`); with no recorded base the file falls back to the existing overwrite/skip/`.new` path. `--dry-run` prints a per-path action preview and mutates nothing. Each non-dry-run upgrade snapshots every file it will write or delete — and the manifest — into a retained backup it restores on any failure; `--restore` re-applies the most recent backup on demand. This iteration resolves the iteration-00 review: it records the SPEC non-goal supersede in the Log, replaces the wrong "reuse RawConfig" claim with an own-loader design, pins the backup lifecycle and includes the manifest in the backup set, states the permanent-fallback boundary, places the new actions in the deterministic sort order, and specifies the dry-run preview surface.

## Log

[**Added**]

- C-19 (permanent-fallback boundary), C-20 (manifest in backup set), C-21 (backup retention), C-22 (own `[upgrade]` loader + `UpgradeConfigCorrupt`), C-23 (new-action sort buckets), C-24 (dry-run preview is `Display`-able), C-25 (two-walk-site unload skip).
- `DryRunPreview` type (R-007); `RestoreSummary` (already implied, now in Data Structure).
- Validation V-IT-8 (no-base merged + Skip stays on fallback across upgrades), V-F-5 (deletion failure restores manifest), V-IT-9 (`--restore` after success vs. after auto-rollback), V-UT-9 (corrupt `[upgrade]` ≠ worktree error), V-UT-10 (new actions sort-placed).

[**Changed**]

- ark-upgrade SPEC `NG-2` — was "No backup directory; rollback is not promised." Now upgrade captures a pre-write backup (files + manifest) and offers `--restore`. Superseded by G-5 / C-13..C-15, C-20, C-21. (R-001)
- C-1 — dropped the false "reuse the existing `RawConfig` loader" claim; `[upgrade]` gets its own private raw-config struct and `UpgradeConfigCorrupt` error (R-002, now C-22).
- Backup model — one retained backup dir with a defined lifecycle, manifest included; auto-rollback leaves the tree pre-upgrade so `--restore` is well-defined (R-003, R-004).
- `## Architecture` — `init.rs` no longer claims to record merged bases (init only seeds fresh files; clarified in C-19/T-4). `remove.rs` dropped from touched files — `.ark/` wipe already covers the sidecars (R-009).
- Self-healing framing — narrowed: a diverged path the user never lets Ark overwrite never acquires a base (R-005).

[**Removed**]

- The implication that the sidecar is "migration state" — clarified NG-1 (no migration manifest) is untouched: the sidecar holds bytes Ark itself wrote, not fetched/old-version data (R-001).

[**Unresolved**]

- None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | `## Log [Changed]` now names the `NG-2` supersede and `[Removed]` confirms `NG-1` untouched. |
| Review | R-002 | Accepted | C-22 defines an own private raw-config struct + `UpgradeConfigCorrupt`; V-UT-9 asserts no worktree-error mislabel. |
| Review | R-003 | Accepted | C-21 pins retention: one backup per non-dry-run upgrade, retained after success, prior backup replaced; auto-rollback leaves tree pre-upgrade. |
| Review | R-004 | Accepted | C-20 adds `.ark/.installed.json` (and in-memory `Manifest`) to the backup set; V-F-5 asserts a deletion-phase failure restores the manifest. |
| Review | R-005 | Accepted | C-19 states the permanent-fallback boundary; V-IT-8 asserts a Skip-preserved no-base merged path stays on fallback; preview labels it (C-24). |
| Review | R-006 | Accepted | C-23 places `Merged`/`MergeConflict` in the write buckets and `EjectSkip` with `Preserve`; revised C-16 enumeration carried; V-UT-10 extends the sort test. |
| Review | R-007 | Accepted | C-24 defines `DryRunPreview` (`Display`, per-path label rows); V-IT-2 tightened to assert specific labels. |
| Review | R-008 | Accepted | C-25 widens `capture_skip_paths` and requires both walk sites; round-trip-drops-bases stated; V-E-2 asserts both dirs absent from the snapshot. |
| Review | R-009 | Accepted | `remove.rs` dropped from touched files; V-E-2 keeps the wipe assertion as a guard only. |
| Review | TR-1 | Accepted | Sidecar kept; R-008 plumbing closed. |
| Review | TR-2 | Accepted | `diffy` taken; first-of-kind dependency noted in SPEC; builder ergonomics verified in Phase 3. |
| Review | TR-3 | Accepted | One backup dir kept with the C-21 retention policy now specified. |

> Every prior CRITICAL / HIGH finding appears above with explicit reasoning.

---

## Spec

[**Goals**]

- G-1: `ark upgrade` reads an `[upgrade]` strategy section from `.ark/config.toml`.
- G-2: An `ejected` path is never written, deleted, prompted, or classified by upgrade.
- G-3: A `merged` non-block file with diverged sides is diff3-merged against a recorded base.
- G-4: `ark upgrade --dry-run` reports every planned action and mutates nothing.
- G-5: `ark upgrade` is reversible: it backs up touched files plus the manifest and restores them.

[**Non-goals**]

- NG-1: No network I/O; the merge base is recorded locally, never fetched (carried; sidecar is not migration state).
- NG-2: `merged` does not apply to managed-block files; their splice strategy is unchanged.
- NG-3: No AI-assisted or interactive merge tool; merging is deterministic diff3 only.
- NG-4: A diverged path the user never lets Ark overwrite is not auto-healed onto the merge path.

[**Architecture**]

```
crates/ark-core/src/
├── commands/upgrade/
│   ├── mod.rs              upgrade(): config → plan → (dry-run preview | backup → apply → restore-on-err)
│   │                        + restore(): --restore entry; backup capture/restore helpers
│   ├── plan.rs             classify(): strategy_for() consulted before hash classification;
│   │                        Merged/MergeConflict/EjectSkip PlannedAction variants + sort buckets
│   ├── strategy.rs   (new) UpgradeConfig: own raw-config struct, load+validate [upgrade]
│   ├── merge.rs      (new) three_way_merge(): diffy::merge_bytes wrapper, git-style markers
│   ├── base_store.rs (new) UpgradeBaseStore: read/write/remove base bytes under sidecar dir
│   └── backup.rs     (new) UpgradeBackup: capture(files+manifest), restore, most_recent
├── layout.rs                + upgrade_base_dir()/upgrade_backup_dir() + consts
├── commands/unload.rs       capture_skip_paths grows to include both sidecar dirs (both walk sites)
└── error.rs                 + UpgradeConfigInvalid, UpgradeConfigCorrupt, NoBackupToRestore
templates/ark/
├── config.toml              + commented [upgrade] section (ejected/merged examples)
└── .gitignore               + .upgrade-base/ and .upgrade-backup/ rules
# init.rs: NOT touched for base recording — init seeds fresh files; bases are recorded by
#          upgrade's apply step only. remove.rs: NOT touched — `.ark/` wipe covers both dirs.
```

[**Data Structure**]

```rust
// strategy.rs — own loader, not the worktree RawConfig
#[derive(Deserialize)] struct RawUpgradeConfig { upgrade: Option<UpgradeSection> } // private
pub struct UpgradeConfig { pub ejected: Vec<PathBuf>, pub merged: Vec<PathBuf> }
impl UpgradeConfig {
    pub fn load_or_default(layout: &Layout) -> Result<Self>; // corrupt TOML → UpgradeConfigCorrupt
    pub fn validate(&self, blocks: &[ManagedBlock], layout: &Layout) -> Result<()>; // C-4,C-5,C-6
    pub fn strategy_for(&self, relative: &Path) -> Strategy;
}
pub enum Strategy { Default, Ejected, Merged }

// base_store.rs
pub struct UpgradeBaseStore<'a> { layout: &'a Layout } // .ark/.upgrade-base/<mirrored path>
impl UpgradeBaseStore<'_> {
    pub fn record(&self, relative: &Path, bytes: &[u8]) -> Result<()>;
    pub fn base_for(&self, relative: &Path) -> Result<Option<Vec<u8>>>;
}

// backup.rs
pub struct UpgradeBackup<'a> { layout: &'a Layout } // .ark/.upgrade-backup/
impl UpgradeBackup<'_> {
    pub fn capture(&self, files: &[PathBuf], manifest_bytes: &[u8]) -> Result<()>; // replaces prior
    pub fn restore(&self) -> Result<RestoreSummary>;          // files + manifest
    pub fn exists(&self) -> bool;
}

// merge.rs
pub enum MergeOutcome { Clean(Vec<u8>), Conflict(Vec<u8>) } // Conflict carries marker bytes
pub fn three_way_merge(base: &[u8], ours: &[u8], theirs: &[u8]) -> MergeOutcome;

// plan.rs — new PlannedAction variants, with sort buckets (see C-23)
//   Write{Add|AutoUpdate|Overwrite} | Merged | MergeConflict   → write buckets 0..=2 (Merged/Conflict adjacent)
//   CreateNew | RefreshHashOnly | Preserve | EjectSkip         → buckets 3..=5 (EjectSkip with Preserve)
//   Delete | DropManifestEntry                                 → buckets 6..=7

// mod.rs
pub struct DryRunPreview { pub rows: Vec<(PathBuf, ActionLabel)> } // impl Display, one line per path
pub enum ActionLabel { Add, Update, Overwrite, MergeClean, MergeConflict, Preserve,
                       CreateNew, Delete, Orphan, EjectSkip, MergeNoBaseFallback }
// UpgradeSummary gains: merged_clean, merged_conflict, ejected_skipped, merge_fallback counters
// UpgradeOptions gains: dry_run: bool, restore: bool
pub struct RestoreSummary { pub restored: usize, pub manifest_restored: bool }
```

[**API Surface**]

```rust
// mod.rs
pub fn upgrade(opts: UpgradeOptions, prompter: &mut dyn Prompter) -> Result<UpgradeSummary>;
pub fn restore(opts: UpgradeOptions) -> Result<RestoreSummary>;
impl UpgradeOptions { pub fn with_dry_run(self, b: bool) -> Self; pub fn with_restore(self, b: bool) -> Self; }
// layout.rs
impl Layout { pub fn upgrade_base_dir(&self) -> PathBuf; pub fn upgrade_backup_dir(&self) -> PathBuf; }
```

[**Constraints**]

- C-1: `[upgrade]` is parsed by `strategy.rs`'s own private raw-config struct off `layout.config_file()`; missing section → empty sets.
- C-2: An `ejected` path is excluded before classification, conflict resolution, write, and removal.
- C-3: Ejection wins over `--force`, `--skip-modified`, `--create-new`, and interactive policy.
- C-4: A `merged` entry naming a managed-block file fails as `UpgradeConfigInvalid`.
- C-5: A path in both `ejected` and `merged` fails as `UpgradeConfigInvalid`.
- C-6: Every `[upgrade]` path is validated via `Layout::resolve_safe` before any I/O.
- C-7: The diff3 base is the bytes Ark last wrote, read from `.ark/.upgrade-base/`; never fetched.
- C-8: A `merged` path with no recorded base routes through the existing conflict pipeline.
- C-9: A clean diff3 writes the merged bytes; a conflict writes git-style markers via `diffy::merge_bytes`.
- C-10: Merge operates on raw bytes so non-UTF-8 content round-trips losslessly.
- C-11: Base bytes are recorded only for `merged`-eligible paths, scoping sidecar size.
- C-12: `--dry-run` performs no write, delete, manifest, managed-block, hook, base, or backup mutation.
- C-13: A non-dry-run upgrade backs up each to-be-mutated/deleted file before its first write.
- C-14: On any apply error, the backup is restored (files + manifest) before the error propagates.
- C-15: `--restore` restores the most recent backup or fails `NoBackupToRestore` when none exists.
- C-16: Action sort order is `Write{Add}` < `Write{AutoUpdate}` < `Write{Overwrite}` < `Merged` < `MergeConflict` < `CreateNew` < `RefreshHashOnly` < `Preserve` < `EjectSkip` < `Delete` < `DropManifestEntry`, secondary key `relative`.
- C-17: New `UpgradeSummary` counters print in fixed order even when zero.
- C-18: All filesystem access routes through `io::PathExt`; all path composition through `layout::Layout`.
- C-19: A diverged `merged` path the user never lets Ark overwrite never acquires a base and stays on fallback permanently.
- C-20: The backup set includes `.ark/.installed.json`; rollback restores the manifest to its pre-write bytes alongside the files.
- C-21: One backup dir per non-dry-run upgrade replaces any prior backup, is retained after success, and is left untouched by an auto-rollback (so `--restore` returns the genuine pre-upgrade tree).
- C-22: A malformed `[upgrade]` section fails as `UpgradeConfigCorrupt`, never as `WorktreeConfigCorrupt`.
- C-23: `Merged`/`MergeConflict` occupy write-adjacent buckets; `EjectSkip` shares `Preserve`'s position class (per C-16).
- C-24: The dry-run preview is a `Display`-able `DryRunPreview`; one render per dispatch (project convention).
- C-25: `capture_skip_paths` includes both sidecar dirs and both `unload` walk sites consume the widened set.

---

## Runtime

[**Main Flow**]

1. `upgrade()` loads `Manifest`, validates manifest paths, checks version (unchanged).
2. Load `UpgradeConfig`; `validate` against `manifest.managed_blocks` + overlap + path-safety (C-4,C-5,C-6,C-22).
3. `collect_desired_templates` + `reconcile_managed_blocks` (unchanged).
4. `plan_actions`: per desired path resolve `strategy_for` first — `Ejected`→`EjectSkip`; `Merged`+diverged+base→`three_way_merge`→`Merged`/`MergeConflict`; `Merged`+no-base→existing conflict resolution (C-8); `Default`→existing classify. Removal pass skips `Ejected` paths.
5. If `dry_run`: build + render `DryRunPreview` and return; mutate nothing (C-12, C-24).
6. Capture backup of every Write/Merged/MergeConflict/Delete target plus the current manifest bytes (C-13, C-20, C-21).
7. Apply writes; for each `Merged`/`MergeConflict`/`Write` of a `merged` path, record the new base bytes.
8. Flush manifest, apply deferred deletions, re-apply managed state (unchanged ordering). On any error in 7–8: `UpgradeBackup::restore()` then propagate (C-14).

[**Failure Flow**]

1. Invalid/corrupt `[upgrade]` → `UpgradeConfigInvalid` / `UpgradeConfigCorrupt`, halt pre-mutation.
2. Any write or deletion error after backup capture → restore files + manifest, propagate original error.
3. `--restore` with no backup → `NoBackupToRestore`.

[**State Transitions**]

- Path strategy: `Default` ⊕ `Ejected` ⊕ `Merged` (config-resolved, mutually exclusive per path).
- `merged` path base: absent → fallback. Recorded only when Ark writes the path (Merged/MergeConflict/Overwrite). Skip-preserved diverged path → no write → no base → permanent fallback (C-19).
- `unload`→`load` round-trip drops `.ark/.upgrade-base/`, so every `merged` path reverts to fallback until Ark next writes it (C-25 consequence).

---

## Implementation

[**Phase 1**] Config + layout + errors. `strategy.rs` (own raw-config struct, `UpgradeConfig`, `Strategy`, `validate`), layout consts/methods, `UpgradeConfigInvalid`/`UpgradeConfigCorrupt`/`NoBackupToRestore`, `[upgrade]` template + `.gitignore` rules. Tests: V-UT-1..4, V-UT-9.

[**Phase 2**] Ejection + dry-run. Wire `strategy_for` into `plan_actions` (EjectSkip + removal skip + sort buckets C-23), `dry_run` option, `DryRunPreview` Display, counters. Tests: V-IT-1, V-IT-2, V-UT-8, V-UT-10.

[**Phase 3**] Base store + diff3. `base_store.rs`, `merge.rs` (verify diffy `merge_bytes`/`ConflictStyle` ergonomics against docs.rs first), record bases in upgrade apply, wire Merged/MergeConflict + no-base fallback. Tests: V-UT-5,6,7, V-IT-3,4,5,6,8.

[**Phase 4**] Backup + restore. `backup.rs` (capture files+manifest, replace-prior retention, restore), restore-on-error in apply, `restore()` + `RestoreSummary`, `unload` two-walk-site skip. Tests: V-F-1,2,3,5, V-IT-9, V-E-2.

[**Phase 5**] CLI wiring + docs. `--dry-run`/`--restore` flags, promoted `## Spec` into the ark-upgrade SPEC (with NG-2 CHANGELOG line + diffy first-dependency note), `workflow.md`/CLAUDE.md upgrade rows only as far as required.

---

## Trade-offs

- T-1: **Base storage — gitignored sidecar vs. manifest base64.** `.ark/.installed.json` is git-tracked and not gitignored, so manifest base64 would commit the whole template corpus into every host repo and churn it each upgrade. Sidecar keeps base bytes local at the cost of `unload` plumbing (C-25). Chosen: sidecar.
- T-2: **`diffy` crate vs. hand-rolled diff3.** `diffy` 0.5 (MIT/Apache, ~834k dl/mo) ships `merge_bytes` + git-style markers; diff3 false-conflict minimization is subtle to reimplement. Chosen: take the crate — Ark's first diff/merge dependency, justified in the SPEC CHANGELOG.
- T-3: **Scope base storage to `merged` paths.** Storing the whole corpus wastes disk for files that never merge. Chosen: record only `merged`-eligible paths.
- T-4: **No-base fallback vs. erroring.** Pre-feature installs and never-rewritten merged paths have no base; a 2-way "merge" is garbage. Chosen: fall back to overwrite/skip/`.new`, labeled distinctly (C-24), and accept the permanent-fallback boundary for Skip-preserved diverged files (C-19) rather than fabricate a base.
- T-5: **One retained backup dir for rollback + regret-restore.** Two use cases, one mechanism; collapsing them needs a stated retention policy (C-21) and the manifest in the set (C-20). Chosen: one dir with the explicit C-21 lifecycle.

---

## Validation

[**Unit Tests**]

- V-UT-1: `load_or_default` empty when `[upgrade]` absent; round-trips a full section.
- V-UT-2: validation rejects a `merged` managed-block file (`UpgradeConfigInvalid`).
- V-UT-3: validation rejects eject∩merge overlap; rejects unsafe/`..` paths.
- V-UT-4: `strategy_for` resolves Ejected / Merged / Default.
- V-UT-5: `three_way_merge` Clean for disjoint edits; Conflict with `<<<<<<<`/`=======`/`>>>>>>>` for overlapping.
- V-UT-6: `three_way_merge` round-trips non-UTF-8 bytes.
- V-UT-7: `UpgradeBaseStore` record/read round-trip; `base_for` None when absent.
- V-UT-8: summary Display prints new counters in fixed order at zero.
- V-UT-9: a corrupt `[upgrade]` section surfaces `UpgradeConfigCorrupt`, not `WorktreeConfigCorrupt`.
- V-UT-10: the sort test covers `Merged`/`MergeConflict`/`EjectSkip` bucket placement (C-16).

[**Integration Tests**]

- V-IT-1: ejected path untouched by `--force` (no overwrite/delete/prompt).
- V-IT-2: `--dry-run` after a user edit prints the specific per-path action label(s) and leaves disk + manifest byte-identical.
- V-IT-3: merged path, disjoint edits → both edits present, counted `merged_clean`.
- V-IT-4: merged path, overlapping edits → conflict markers, counted `merged_conflict`.
- V-IT-5: merged path, no base → fallback conflict pipeline (Skip preserves user file), counted `merge_fallback`.
- V-IT-6: base self-heal — after an Ark-written upgrade records a base, the next merged upgrade merges.
- V-IT-7: `[upgrade]` section survives upgrade byte-identical (config seed-only).
- V-IT-8: no-base merged path the user Skips stays on fallback across repeated upgrades (C-19).
- V-IT-9: `--restore` after a *successful* upgrade returns the pre-upgrade tree; after an auto-rollback it is a no-op/refusal, not a second rollback (C-21).

[**Failure / Robustness**]

- V-F-1: a write failure mid-apply restores every backed-up file to pre-upgrade bytes.
- V-F-2: `--restore` with no backup → `NoBackupToRestore`.
- V-F-3: `--restore` after a completed upgrade restores the prior tree.
- V-F-4: invalid `[upgrade]` config halts before any file is written.
- V-F-5: a deletion-phase failure (after the mid-sequence manifest flush) restores the manifest to its pre-upgrade content, not just the file tree (C-20).

[**Edge Cases**]

- V-E-1: empty `ejected`/`merged` behave exactly like today (no regression).
- V-E-2: `unload`→`load` ignores `.upgrade-base/`/`.upgrade-backup/` from BOTH walk sites; `remove` wipes them via the `.ark/` wipe.
- V-E-3: a `merged` path also removed-from-templates respects ejection/removal rules.
- V-E-4: legacy hash-only manifest + no sidecar deserializes and upgrades via fallback.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-IT-7 |
| G-2 | V-IT-1, V-UT-4 |
| G-3 | V-IT-3, V-IT-4, V-IT-5, V-IT-6, V-IT-8, V-UT-5, V-UT-7 |
| G-4 | V-IT-2, V-UT-8 |
| G-5 | V-F-1, V-F-2, V-F-3, V-F-5, V-IT-9 |
| C-1 | V-UT-1 |
| C-2 | V-IT-1 |
| C-3 | V-IT-1 |
| C-4 | V-UT-2 |
| C-5 | V-UT-3 |
| C-6 | V-UT-3 |
| C-7 | V-UT-7, V-IT-6 |
| C-8 | V-IT-5, V-E-4 |
| C-9 | V-UT-5, V-IT-3, V-IT-4 |
| C-10 | V-UT-6 |
| C-11 | V-UT-7 |
| C-12 | V-IT-2 |
| C-13 | V-F-1 |
| C-14 | V-F-1, V-F-5 |
| C-15 | V-F-2, V-F-3, V-IT-9 |
| C-16 | V-UT-10 |
| C-17 | V-UT-8 |
| C-18 | (source-scan test, carried) |
| C-19 | V-IT-8 |
| C-20 | V-F-5 |
| C-21 | V-IT-9 |
| C-22 | V-UT-9 |
| C-23 | V-UT-10 |
| C-24 | V-IT-2 |
| C-25 | V-E-2 |
