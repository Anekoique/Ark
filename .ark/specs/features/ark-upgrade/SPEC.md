
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

- C-1: @test-binding: load_or_default_empty_when_absent
`[upgrade]` is parsed by `strategy.rs`'s own private raw-config struct off `layout.config_file()`; missing section → empty sets.
- C-2: @test-binding: load_or_default_round_trips_full_section
An `ejected` path is excluded before classification, conflict resolution, write, and removal.
- C-3: @test-binding: ejected_path_untouched_by_force
Ejection wins over `--force`, `--skip-modified`, `--create-new`, and interactive policy.
- C-4: @test-binding: validate_rejects_merged_managed_block_file
A `merged` entry naming a managed-block file fails as `UpgradeConfigInvalid`.
- C-5: @test-binding: validate_rejects_overlap_and_unsafe
A path in both `ejected` and `merged` fails as `UpgradeConfigInvalid`.
- C-6: @test-binding: validate_rejects_overlap_and_unsafe
Every `[upgrade]` path is validated via `Layout::resolve_safe` before any I/O.
- C-7: @test-binding: record_read_round_trip_and_absent
The diff3 base is the bytes Ark last wrote, read from `.ark/.upgrade-base/`; never fetched.
- C-8: @test-binding: merged_without_base_falls_back_to_conflict_skip
A `merged` path with no recorded base routes through the existing conflict pipeline.
- C-9: @test-binding: overlapping_edits_conflict_with_git_markers
A clean diff3 writes the merged bytes; a conflict writes git-style markers via `diffy::merge_bytes`.
- C-10: @test-binding: non_utf8_round_trips
Merge operates on raw bytes so non-UTF-8 content round-trips losslessly.
- C-11: @test-binding: nested_path_mirrors_under_base_dir
Base bytes are recorded only for `merged`-eligible paths, scoping sidecar size.
- C-12: @test-binding: upgrade_source_has_no_bare_std_fs_or_dot_ark_literals
`--dry-run` performs no write, delete, manifest, managed-block, hook, base, or backup mutation.
- C-13: @test-binding: upgrade_source_has_no_bare_std_fs_or_dot_ark_literals
A non-dry-run upgrade backs up each to-be-mutated/deleted file before its first write.
- C-14: @test-binding: apply_failure_rolls_back_files_and_manifest
On any apply error, the backup is restored (files + manifest) before the error propagates.
- C-15: @test-binding: restore_without_backup_errors
`--restore` restores the most recent backup or fails `NoBackupToRestore` when none exists.
- C-16: @test-binding: plan_actions_sorts_output_by_bucket_then_path
Action sort order is `Write{Add}` < `Write{AutoUpdate}` < `Write{Overwrite}` < `Merged` < `MergeConflict` < `AppendConfigSections` < `CreateNew` < `RefreshHashOnly` < `Preserve` < `EjectSkip` < `Delete` < `DropManifestEntry`, secondary key `relative`.
- C-17: @test-binding: summary_display_prints_fixed_order_even_when_zero
New `UpgradeSummary` counters print in fixed order even when zero.
- C-18: @judgment
All filesystem access routes through `io::PathExt`; all path composition through `layout::Layout`.
- C-19: @test-binding: merged_no_base_skip_stays_on_fallback
A diverged `merged` path the user never lets Ark overwrite never acquires a base and stays on fallback permanently.
- C-20: @test-binding: apply_failure_rolls_back_files_and_manifest
The backup set includes `.ark/.installed.json`; rollback restores the manifest to its pre-write bytes alongside the files.
- C-21: @test-binding: restore_after_success_returns_pre_upgrade_tree
One backup dir per non-dry-run upgrade replaces any prior backup, is retained after success, and is left untouched by an auto-rollback (so `--restore` returns the genuine pre-upgrade tree).
- C-22: @test-binding: corrupt_upgrade_section_is_upgrade_error
A malformed `[upgrade]` section fails as `UpgradeConfigCorrupt`, never as `WorktreeConfigCorrupt`.
- C-23: @test-binding: new_action_variants_sort_into_their_buckets
`Merged`/`MergeConflict` occupy write-adjacent buckets; `EjectSkip` shares `Preserve`'s position class (per C-16).
- C-24: @test-binding: dry_run_reports_and_mutates_nothing
The dry-run preview is a `Display`-able `DryRunPreview`; one render per dispatch (project convention).
- C-25: @judgment
`capture_skip_paths` includes both sidecar dirs and both `unload` walk sites consume the widened set.
- C-26: @test-binding: upgrade_appends_missing_top_level_sections_to_config_toml
`.ark/config.toml` is seed-only (never overwritten or hash-classified) but receives an additive section merge: any top-level `[name]` table the current template ships that the user's file lacks is appended verbatim (header + leading-comment block + body). The user's existing bytes are preserved as a prefix; no existing section is rewritten, reordered, or inspected. Ejection beats append. The merge is reversible via `--restore` (the file joins the backup set).

---

[**CHANGELOG**]

- 2026-05-26: replaced from 01_PLAN.md (prior body preserved in git history)
