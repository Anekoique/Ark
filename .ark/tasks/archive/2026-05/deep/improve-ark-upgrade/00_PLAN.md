# `ark-upgrade` PLAN `00`

> Status: Draft
> Feature: `ark-upgrade`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none

---

## Summary

Extend `ark upgrade` with a user-declared per-path file strategy read from a new `[upgrade]` section of `.ark/config.toml`. Two strategies are added on top of the existing managed/auto/conflict pipeline: `ejected` (Ark stops touching the path entirely — supersedes every policy including `--force`) and `merged` (when both Ark and the user changed a non-managed-block file, run a diff3 3-way merge). The 3-way merge base is sourced by recording the exact bytes Ark last wrote for each `merged` path into a gitignored sidecar (`.ark/.upgrade-base/`), since the manifest stores only a hash; when no base is recorded, the file falls back to the existing overwrite/skip/`.new` conflict path. A `--dry-run` flag prints the full planned action set and exits without mutating anything. Every non-dry-run upgrade first snapshots the files it will write or delete into a backup it restores on any failure, and the most recent backup is restorable on demand via `ark upgrade --restore`.

## Log `None in 00_PLAN`

[**Added**]

- N/A — initial plan.

[**Changed**]

- N/A

[**Removed**]

- N/A

[**Unresolved**]

- N/A

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| — | — | — | initial plan, no prior review |

---

## Spec

[**Goals**]

- G-1: `ark upgrade` reads an `[upgrade]` strategy section from `.ark/config.toml`.
- G-2: An `ejected` path is never written, deleted, prompted, or classified by upgrade.
- G-3: A `merged` non-block file with diverged sides is diff3-merged against a recorded base.
- G-4: `ark upgrade --dry-run` reports every planned action and mutates nothing.
- G-5: `ark upgrade` is reversible: it backs up touched files and can restore the last backup.

[**Non-goals**]

- NG-1: No network I/O; the merge base is recorded locally, never fetched (carried from prior SPEC).
- NG-2: `merged` does not apply to managed-block files; their existing splice strategy is unchanged.
- NG-3: No AI-assisted or interactive merge tool; merging is deterministic diff3 only.

[**Architecture**]

```
crates/ark-core/src/
├── commands/upgrade/
│   ├── mod.rs              upgrade(): strategy load → backup → plan → apply → restore-on-err
│   │                        + restore(): --restore entry; backup capture/restore helpers
│   ├── plan.rs             classify(): consults UpgradeStrategy before hash classification;
│   │                        new Merged/MergeConflict/Ejected PlannedAction variants
│   ├── strategy.rs   (new) UpgradeConfig load+validate from [upgrade]; ejected/merged sets
│   ├── merge.rs      (new) three_way_merge(): diffy::merge_bytes wrapper, git-style markers
│   └── base_store.rs (new) UpgradeBaseStore: read/write/remove base bytes under sidecar dir
├── state/manifest.rs        unchanged (base bytes live in sidecar, not the manifest)
├── layout.rs                + UPGRADE_BASE_DIR const + upgrade_base_dir()/upgrade_backup_dir()
├── commands/init.rs         records base bytes for merged-eligible writes (via shared helper)
├── commands/remove.rs       wipes .ark/.upgrade-base/ and .ark/.upgrade-backup/
├── commands/unload.rs       skips the sidecar + backup dirs (local-only, like .state.toml)
└── error.rs                 + UpgradeConfigInvalid, NoBackupToRestore
templates/ark/
├── config.toml              + commented [upgrade] section (ejected/merged examples)
└── .gitignore               + .upgrade-base/ and .upgrade-backup/ rules
```

[**Data Structure**]

```rust
// strategy.rs
pub struct UpgradeConfig {
    pub ejected: Vec<PathBuf>,   // paths upgrade never touches
    pub merged: Vec<PathBuf>,    // non-block paths that diff3-merge on divergence
}
impl UpgradeConfig {
    pub fn load_or_default(layout: &Layout) -> Result<Self>; // [upgrade] section; defaults empty
    pub fn strategy_for(&self, relative: &Path) -> Strategy;  // Ejected | Merged | Default
}
pub enum Strategy { Default, Ejected, Merged }

// base_store.rs
pub struct UpgradeBaseStore<'a> { layout: &'a Layout } // rooted at .ark/.upgrade-base/
impl UpgradeBaseStore<'_> {
    pub fn record(&self, relative: &Path, bytes: &[u8]) -> Result<()>;
    pub fn base_for(&self, relative: &Path) -> Result<Option<Vec<u8>>>;
    pub fn remove(&self, relative: &Path) -> Result<()>;
}

// merge.rs
pub enum MergeOutcome { Clean(Vec<u8>), Conflict(Vec<u8>) } // Conflict carries marker-laden bytes
pub fn three_way_merge(base: &[u8], ours: &[u8], theirs: &[u8]) -> MergeOutcome;

// plan.rs — new PlannedAction variants
//   Merged { relative, contents }          // clean diff3 result to write
//   MergeConflict { relative, contents }   // marker-laden bytes to write, reported as conflict
//   EjectSkip { relative }                 // counted, never written

// mod.rs — UpgradeSummary gains: merged_clean, merged_conflict, ejected_skipped counters
//          UpgradeOptions gains: dry_run: bool, restore: bool
```

[**API Surface**]

```rust
// mod.rs
pub fn upgrade(opts: UpgradeOptions, prompter: &mut dyn Prompter) -> Result<UpgradeSummary>;
pub fn restore(opts: UpgradeOptions) -> Result<RestoreSummary>; // --restore path

impl UpgradeOptions {
    pub fn with_dry_run(self, dry_run: bool) -> Self;
    pub fn with_restore(self, restore: bool) -> Self;
}

// layout.rs
impl Layout {
    pub fn upgrade_base_dir(&self) -> PathBuf;    // .ark/.upgrade-base
    pub fn upgrade_backup_dir(&self) -> PathBuf;  // .ark/.upgrade-backup
}
```

[**Constraints**]

- C-1: `[upgrade]` is read via the existing `RawConfig` loader; missing section → empty strategy sets.
- C-2: An `ejected` path is excluded before classification, conflict resolution, write, and removal.
- C-3: Ejection wins over `--force`, `--skip-modified`, `--create-new`, and interactive policy.
- C-4: A `merged` entry naming a managed-block file (`manifest.managed_blocks`) fails as `UpgradeConfigInvalid`.
- C-5: A path listed in both `ejected` and `merged` fails as `UpgradeConfigInvalid`.
- C-6: Every `[upgrade]` path is validated via `Layout::resolve_safe` before any I/O.
- C-7: The diff3 base is the bytes Ark last wrote, read from `.ark/.upgrade-base/`; never fetched.
- C-8: A `merged` path with no recorded base routes through the existing conflict pipeline.
- C-9: A clean diff3 writes the merged bytes; a conflicting diff3 writes git-style markers via `diffy::merge_bytes`.
- C-10: Merge operates on raw bytes so non-UTF-8 template content round-trips losslessly.
- C-11: Base bytes are recorded only for `merged`-eligible paths, scoping sidecar size.
- C-12: `--dry-run` performs no write, delete, manifest, managed-block, hook, base, or backup mutation.
- C-13: A non-dry-run upgrade backs up each to-be-mutated/deleted file before its first write.
- C-14: On any apply error, the backup is restored and the manifest is left at its pre-write state.
- C-15: `--restore` restores the most recent backup or fails `NoBackupToRestore` when none exists.
- C-16: `.ark/.upgrade-base/` and `.ark/.upgrade-backup/` are gitignored and skipped by `unload`; wiped by `remove`.
- C-17: New `UpgradeSummary` counters print in fixed order even when zero (carried convention).
- C-18: All filesystem access routes through `io::PathExt`; all path composition through `layout::Layout`.

---

## Runtime

[**Main Flow**]

1. `upgrade()` loads `Manifest`, validates manifest paths, checks version (unchanged).
2. Load `UpgradeConfig` from `[upgrade]`; validate against managed-block set and overlap (C-4, C-5).
3. `collect_desired_templates` + `reconcile_managed_blocks` (unchanged).
4. `plan_actions`: for each desired path, resolve `strategy_for(path)` first:
   - `Ejected` → `EjectSkip` (no classification).
   - `Merged` + sides diverged + base present → `three_way_merge` → `Merged` or `MergeConflict`.
   - `Merged` + no base → existing conflict resolution (fallback, C-8).
   - `Default` → existing classify() path.
   Removal pass also skips `Ejected` paths.
5. If `dry_run`: render the plan as a preview and return; mutate nothing (C-12).
6. Capture backup of every Write/Merged/MergeConflict/Delete target (C-13).
7. Apply writes; for each `Merged`/`MergeConflict`/`Write` of a `merged` path, record new base bytes.
8. Flush manifest, apply deferred deletions, re-apply managed state (unchanged ordering).

[**Failure Flow**]

1. Config invalid (unknown key, unsafe path, block-file merged, eject∩merge) → `UpgradeConfigInvalid`, halt pre-mutation.
2. Any write/delete error after backup capture → restore backup, propagate the original error (C-14).
3. `--restore` with no backup dir → `NoBackupToRestore`.

[**State Transitions**]

- Path strategy: `Default` ⊕ `Ejected` ⊕ `Merged` (config-resolved, mutually exclusive per path).
- `merged` path: no-base → conflict-fallback; after one Ark write → base recorded → merges next time (self-healing).

---

## Implementation

[**Phase 1**] Config + layout + errors. Add `strategy.rs` (`UpgradeConfig`, `Strategy`), extend `RawConfig` with `upgrade: Option<...>`, add layout consts/methods, add error variants, ship the `[upgrade]` template + `.gitignore` rules. Tests: load/default/validate, overlap + block-file rejection.

[**Phase 2**] Ejection + dry-run. Wire `strategy_for` into `plan_actions` (EjectSkip, removal skip), add `dry_run` option + plan-preview rendering, add counters. Tests: ejected beats force, dry-run mutates nothing, summary preview.

[**Phase 3**] Base store + 3-way merge. Add `base_store.rs`, `merge.rs` (diffy), record bases at the init/upgrade write sites via a shared helper, wire `Merged`/`MergeConflict` into plan + apply, no-base fallback. Tests: clean merge, conflict markers, non-UTF-8, fallback when base absent, base self-heal.

[**Phase 4**] Backup + restore. Capture-before-write, restore-on-error, `--restore` entry + `RestoreSummary`, `remove`/`unload` plumbing for the two local dirs. Tests: failed apply restores tree, `--restore` round-trip, `unload`/`remove` handle dirs.

[**Phase 5**] CLI wiring + docs. `--dry-run`/`--restore` flags in `ark-cli`, update the `ark-upgrade` SPEC body via the promoted `## Spec`, refresh `workflow.md`/CLAUDE.md upgrade rows only as far as the feature requires.

---

## Trade-offs

- T-1: **Base storage — gitignored sidecar vs. manifest base64.** Research confirmed `.ark/.installed.json` is git-tracked and the shipped `.gitignore` does not exclude it, so embedding base64 bodies would commit the whole template corpus into every host repo and churn it each upgrade. A sidecar (`.ark/.upgrade-base/`) keeps base bytes local at the cost of `unload`/`remove` plumbing. Chosen: sidecar — bounded blast radius on a user-visible committed file outweighs the small extra plumbing.
- T-2: **`diffy` crate vs. hand-rolled diff3.** `diffy` 0.5 (MIT/Apache, ~834k dl/mo, +`hashbrown`) ships `merge_bytes` and git-style markers out of the box; diff3 false-conflict minimization is subtle and error-prone to reimplement. Chosen: take the crate; first diff/merge dependency in the tree, justified by correctness.
- T-3: **Scope base storage to `merged` paths vs. all tracked files.** Storing bases for the whole corpus wastes disk for files that never merge. Chosen: record only `merged`-eligible paths, bounding sidecar size to what the user opted into.
- T-4: **No-base fallback vs. erroring.** Pre-feature installs and never-rewritten merged paths have no base. Erroring would break the first upgrade; a 2-way "merge" produces garbage. Chosen: fall back to the existing overwrite/skip/`.new` pipeline, labeled distinctly in the summary, self-healing on the next Ark write.
- T-5: **Backup as a sidecar dir vs. no rollback (prior NG-2).** The prior SPEC disclaimed rollback; the PRD now requires recoverability. A pre-write backup dir restored on error is the minimal mechanism that does not require git. Chosen: revise NG-2, add `.ark/.upgrade-backup/`.

---

## Validation

[**Unit Tests**]

- V-UT-1: `UpgradeConfig::load_or_default` returns empty sets when `[upgrade]` is absent; round-trips a full section.
- V-UT-2: validation rejects a `merged` path that is a managed-block file (`UpgradeConfigInvalid`).
- V-UT-3: validation rejects a path in both `ejected` and `merged`; rejects unsafe/`..` paths.
- V-UT-4: `strategy_for` resolves Ejected / Merged / Default correctly.
- V-UT-5: `three_way_merge` returns `Clean` for non-overlapping edits; `Conflict` with `<<<<<<<`/`=======`/`>>>>>>>` markers for overlapping edits.
- V-UT-6: `three_way_merge` round-trips non-UTF-8 bytes.
- V-UT-7: `UpgradeBaseStore` record/read/remove round-trip; `base_for` returns None when absent.
- V-UT-8: summary Display prints new counters in fixed order even at zero.

[**Integration Tests**]

- V-IT-1: ejected path is untouched by `--force` (no overwrite, no delete, no prompt).
- V-IT-2: `--dry-run` after a user edit reports the action and leaves disk + manifest byte-identical.
- V-IT-3: merged path with both sides edited non-overlapping → file contains both edits, counted `merged_clean`.
- V-IT-4: merged path with overlapping edits → file has conflict markers, counted `merged_conflict`.
- V-IT-5: merged path with no recorded base → falls back to conflict pipeline (skip preserves user file).
- V-IT-6: base self-heal — after one Ark-written upgrade, a subsequent merged upgrade merges instead of falling back.
- V-IT-7: `[upgrade]` section survives upgrade byte-identical (config is seed-only).

[**Failure / Robustness**]

- V-F-1: a write failure mid-apply restores every backed-up file to its pre-upgrade bytes; manifest unchanged.
- V-F-2: `ark upgrade --restore` with no backup → `NoBackupToRestore`.
- V-F-3: `ark upgrade --restore` after a completed upgrade restores the prior tree.
- V-F-4: invalid `[upgrade]` config halts before any file is written.

[**Edge Cases**]

- V-E-1: empty `ejected`/`merged` arrays behave exactly like today (no regression).
- V-E-2: `unload`→`load` round-trip ignores `.upgrade-base/`/`.upgrade-backup/`; `remove` wipes them.
- V-E-3: a `merged` path that is also removed-from-templates (orphan) respects ejection/removal rules.
- V-E-4: legacy hash-only manifest + no sidecar deserializes and upgrades via fallback (migration window).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-IT-7 |
| G-2 | V-IT-1, V-UT-4 |
| G-3 | V-IT-3, V-IT-4, V-IT-5, V-IT-6, V-UT-5, V-UT-7 |
| G-4 | V-IT-2, V-UT-8 |
| G-5 | V-F-1, V-F-2, V-F-3 |
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
| C-14 | V-F-1 |
| C-15 | V-F-2, V-F-3 |
| C-16 | V-E-2 |
| C-17 | V-UT-8 |
| C-18 | (source-scan test, carried) |
