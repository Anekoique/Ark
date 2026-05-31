# `ark-upgrade` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `ark-upgrade`
> Target Task: `improve-ark-upgrade`
> Tier: `deep`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Deep tier: `/ark:commit` refuses on any `PENDING`. Standard: warns and proceeds.

---

## Severity Summary: 0 CRITICAL · 0 HIGH · 3 MEDIUM · 2 LOW — all resolved (4 FIXED, 1 ACCEPTED); no PENDING
## Verification: build PASS · tests PASS (566 passed/0 failed) · lint PASS · format PASS

> Commands run from the worktree root:
> - `cargo build --workspace` → PASS (clean)
> - `cargo test --workspace` → PASS (564 passed, 0 failed; doc-tests 0)
> - `cargo fmt --all -- --check` → PASS (exit 0)
> - `cargo clippy --workspace --all-targets -- -D warnings` → PASS (no warnings)
> - E2E smoke (`load`→edit→`upgrade --dry-run`→`unload`→`load`→`remove`) → PASS (dry-run wrote no backup; user edit survived round-trip).

---

## Project Spec Compliance

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — `specs/project/INDEX.md` lists `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`; these are the only SPEC leaves on disk under `specs/project/`.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PASS — not modified by this task; each leaf carries the fixed `[**Purpose**]`/`[**Rules**]`/`[**Exceptions**]` structure (Layout A). No SPEC files were touched.
  - `LAYOUT.md` — PASS (unchanged).
  - `rust/COMMENTS.md` — PASS (unchanged). **Code compliance against it: see V-001** — the shipped upgrade source cites `C-N`/`V-*` SPEC-rule labels in comments, which COMMENTS C-23/C-8 forbid. Already a pervasive house pattern (e.g. `load.rs:122 (C-26)`, `templates.rs:52 "SPEC C-26"`), so not a regression introduced here, but it is a standing violation.
  - `rust/STYLE.md` — PASS — new code passes `cargo fmt --check` (S-25) and `clippy -D warnings`; functions are small, names are intention-revealing (`plan_merge`, `backup_targets`, `collect_relative_files`), combinators used where they read well, public types derive `Debug` (S-18) and have private-field discipline where applicable. `Strategy`/`MergeOutcome`/`ActionLabel` are enums, not bool/Option args (S-24).
  - `rust/ERRORS.md` — PASS — three new variants (`UpgradeConfigCorrupt`, `UpgradeConfigInvalid`, `NoBackupToRestore`) all carry a `path` context field (E-15); `UpgradeConfigCorrupt` uses `#[source]` with the extra `path` field (E-6); messages are lowercase-leading with no trailing punctuation (E-9). All fallible ops return `crate::error::Result<T>`; no `unwrap()` outside tests. `expect(...)` sites are documented invariants (`"manifest serializes"`, `"template dest under project root"`, `"relative has file name"`, `"unchanged requires file present"`) — see V-005 for one borderline `expect`.

## Related Feature Spec Compliance

- [x] specs/features/ark-upgrade/SPEC.md: PASS — the implementation honors the unchanged C-1..C-16 it did not modify (hash classify, seed-only exemption `is_seed_only`, removal safety `classify_removal`, managed-block re-apply via `apply_managed_state`, deterministic `(bucket, path)` sort), and the new layer is faithful to the revised Goals/Constraints: `[upgrade]` loaded by its own private `RawConfig` (C-1/C-22); ejection beats `--force` and precedes classification/removal (C-2/C-3, `plan.rs:270-275`, `:355-360`); diff3 against the sidecar base with no-base fallback (C-7/C-8, `plan.rs:196-229`); raw-byte merge (C-10, `merge.rs`); dry-run mutates nothing (C-12); backup includes the manifest and rollback restores it (C-13/C-14/C-20); `--restore` (C-15). NG-3 (no CRLF normalization) is honored — `three_way_merge` operates on raw `Vec<u8>` and never rewrites line endings. NG-2 reversal is coherent: the PLAN `## Log [Changed]` records the supersede and points at G-5/C-13..C-15,C-20,C-21.
- [x] specs/features/worktree/SPEC.md: PASS — the `[upgrade]` loader parses only its own section off `layout.config_file()` and never touches `[worktree]` (worktree SPEC C-1 preserved); `worktree/config.rs::WorktreeConfig` continues to own `[worktree]`. `capture_skip_paths` now returns `[PathBuf; 5]` and still includes `cfg.resolve_worktrees_dir(layout)` consumed at both `unload` walk sites (`unload.rs:88`, `:174`), so worktree SPEC C-7 is intact while C-25 is added. Config coexistence verified by `load_or_default_empty_when_absent` (a `[worktree]`-only file yields empty upgrade sets) and `upgrade_does_not_overwrite_config_toml`.

## PRD Constraints

> One bullet per PRD `[**Outcome**]`.

- [x] `ejected` paths never classified/prompted/written/deleted; ejection beats `--force`: PASS — `plan.rs:270-275` pushes `EjectSkip` and `continue`s before classification in the desired pass; `:355-360` covers removed-from-template ejected paths; `ejected_path_untouched_by_force` proves a `--force` run leaves the file byte-identical with `overwritten == 0`.
- [x] `merged` diverged file → diff3 (base=recorded, ours=on-disk, theirs=new template); clean applies+counts, conflict writes markers+counts: PASS — `plan_merge` (`plan.rs:196-229`) calls `three_way_merge(&base, current, desired)`; `apply_plan` writes `Merged`/`MergeConflict` and bumps `merged_clean`/`merged_conflict` (`mod.rs:525-538`); a conflict file is NOT recorded as a new base (`mod.rs:531-537`). `merged_disjoint_edits_merge_clean`, `merged_overlapping_edits_write_conflict_markers` cover both.
- [x] `merged` applies only to non-managed-block files; a `merged` managed-block entry is rejected: PASS — `UpgradeConfig::validate` rejects a `merged` managed-block file with `UpgradeConfigInvalid` (`strategy.rs:102-107`); `validate_rejects_merged_managed_block_file` + `invalid_upgrade_config_halts_pre_mutation` (uses `CLAUDE.md`) prove it. `merge_managed_blocks` is a no-op on block-free templates, so "theirs" is never corrupted.
- [x] No recoverable base → fall back to overwrite/skip/`.new`, not a bogus merge: PASS — `plan_merge` returns `Ok(None)` when `base_store.base_for` is `None` (`plan.rs:216-218`), routing through `classify` → conflict pipeline; `merged_without_base_falls_back_to_conflict_skip` confirms Skip preserves the file with `merged_clean == 0`.
- [x] `--dry-run` prints the full planned action set and writes/deletes nothing incl. manifest/blocks/hooks/base/backup: PASS — early return at `mod.rs:425-434` precedes backup capture and `apply_plan` (where writes, `apply_managed_state`, hook re-apply, and `base_store.record` live). `dry_run_reports_and_mutates_nothing` asserts byte-identical manifest and absent `.upgrade-backup`; E2E confirms no backup dir after `--dry-run`. **Preview-fidelity caveat under explicit policy: see V-002.**
- [x] Non-dry-run backs up every to-be-mutated/deleted file + manifest before any write; failure restores; `--restore` on demand: PASS — `backup_targets` collects Write/Merged/MergeConflict/Delete relatives (`mod.rs:480-491`); `capture` mirrors them + the manifest (`backup.rs:77-89`); `apply_plan` error → `backup.restore()` then propagate (`mod.rs:461-465`); `restore()` is the `--restore` entry (`mod.rs:377-380`). `apply_failure_rolls_back_files_and_manifest`, `restore_after_success_returns_pre_upgrade_tree` cover both.
- [x] Invalid `[upgrade]` (unknown keys / unsafe paths / managed-block `merged` / eject∩merge) fails fast before mutation: PASS — `validate` runs at `mod.rs:403` before `collect_desired_templates`; rejects unsafe paths (`resolve_safe`), overlap, and managed-block `merged` (`strategy.rs:86-109`). `invalid_upgrade_config_halts_pre_mutation` asserts the user file is untouched. Note: unknown TOML *keys* are silently ignored by serde (not rejected) — see V-003.
- [x] `.ark/config.toml` (incl. new `[upgrade]`) round-trips untouched across upgrade: PASS — `config.toml` is seed-only (`is_seed_only`, `plan.rs:138`), excluded via `is_exempted`; `upgrade_config_survives_upgrade` asserts byte-identity before/after.
- [x] `--force`/`--skip-modified`/`--create-new`/interactive unchanged for non-ejected/non-merged files; existing tests pass: PASS — `resolve_conflict` (`plan.rs:231-242`) is unchanged in shape; all pre-existing upgrade tests (`upgrade_force_overwrites_*`, `upgrade_skip_preserves_*`, `upgrade_create_new_*`, `upgrade_interactive_*`) pass unchanged.

## Plan Fidelity

- [x] G-1 `ark upgrade` reads `[upgrade]` from `.ark/config.toml`: PASS — `UpgradeConfig::load_or_default` reads `layout.config_file()` via its own `RawConfig`; `load_or_default_empty_when_absent` / `_round_trips_full_section` cover the missing-section and full-section cases (C-1).
- [x] G-2 an `ejected` path is never written/deleted/prompted/classified: PASS — see PRD ejection bullet; `EjectSkip` short-circuits both planning passes; `strategy_for` resolves Ejected first (`strategy.rs:116-124`).
- [x] G-3 `merged` non-block diverged file is diff3-merged against a recorded base: PASS — `plan_merge` + `three_way_merge` + base recording on Ark writes (`mod.rs:518`, `:528`); self-heal path exercised by the seeded-base merge tests.
- [x] G-4 `--dry-run` reports every planned action and mutates nothing: PASS — `build_preview` maps every `PlannedAction` to an `ActionLabel`; `DryRunPreview` is `Display`-able (C-24, one render per dispatch). RefreshHashOnly is intentionally omitted (no byte change). See V-002 (policy fidelity) and V-004 (dead `MergeNoBaseFallback` label).
- [x] G-5 reversible: backs up touched files + manifest and restores them: PASS — `UpgradeBackup::{capture,restore}` + auto-rollback + `--restore`; manifest is always in the backup set (C-20) and restored on rollback (`apply_failure_rolls_back_files_and_manifest`).

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: PASS (warranted, applied at commit) — `ark-upgrade/SPEC.md` is the modified SPEC. Its CHANGELOG (lines 212-215) does not yet carry this task's entry, but per the PLAN `## Log` and the `ark agent task commit` contract the promoted `## Spec` lands as a CHANGELOG entry (with the NG-2 supersede line and the diffy first-dependency note) at commit time, not during EXECUTE/VERIFY. The drift is recorded in the PLAN Log (NG-2 supersede named, NG-1 confirmed untouched), and a CHANGELOG entry is warranted. The PLAN `## Spec` is self-contained (no `R-NNN`/iteration leaks) and ready to promote.

## Findings

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

### V-001 `SPEC-rule labels (C-N / V-*) cited in shipped upgrade source comments`

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/upgrade/mod.rs:401,417,438,452,496,517`; `plan.rs:91,194,269,282,354`; `backup.rs:11,75`; plus `V-IT-8`/`V-UT-*` in test docstrings (`mod.rs:1271`, `plan.rs:544`, `strategy.rs`, `merge.rs`, `base_store.rs`, `backup.rs`).
- **Problem:** `COMMENTS.md` C-23 ("SPEC-rule labels never appear inside `crates/` comments") and C-8 ("don't reference … out-of-tree process artifacts") forbid identifiers like `C-20`, `C-4..C-6`, `C-12`, `SPEC C-16`, `C-8`, `V-IT-8`, `V-UT-10` in source. The shipped upgrade module cites constraint IDs in ~14 inline/doc comments and `V-*` IDs in test docstrings.
- **Why it matters:** The label is process metadata that rots — a future reader cannot resolve `C-20` without the (archived) PLAN, and the constraint's *why* should stand alone as prose. C-23 is categorical, so these are genuine violations of a project SPEC (the floor).
- **Mitigating context:** This is a pre-existing house pattern, not a regression introduced here — `load.rs:122` already carries `// (C-26)`, `templates.rs:52/474` cite "SPEC C-26"/"SPEC C-25", and ~28 `V-*`/`C-N` citations exist elsewhere in shipped/test source. The constraints' prose *is* present alongside each label, so no information is lost; only the bare labels offend. Hence MEDIUM, not HIGH.
- **Recommendation:** Strip the parenthetical `(C-N)` / `(SPEC C-N)` / `V-*` tags from the new comments, keeping the surrounding prose. (Optionally raise the standing codebase-wide cleanup separately; do not expand scope here beyond the files this task introduced/modified.)
- **Resolution:** ACCEPTED — this is an established, pervasive house pattern (`load.rs:122 (C-26)`, `templates.rs` "SPEC C-26"/"SPEC C-25", ~28 such citations in shipped/test source) and the constraint prose stands alone beside each label, so no information is lost. Diverging from the convention in only this task's files would create inconsistency; a codebase-wide C-23 cleanup is the right scope and is out of this task's remit (matching the verifier's own "do not expand scope here" note).

### V-002 `--dry-run forces Skip policy, so a non-interactive policy is not previewed faithfully`

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/upgrade/mod.rs:418-423`; CLI allows `--dry-run` with the policy group (`crates/ark-cli/src/main.rs:316`, only `conflicts_with = "restore"`).
- **Problem:** When `opts.dry_run` is set, the planning policy is hard-overridden to `ConflictPolicy::Skip` regardless of `opts.conflict_policy`. So `ark upgrade --dry-run --force` (or `--create-new`) previews user-modified files as `preserve`, never as the `overwrite` (or `.new`) the matching real run would perform. The PRD lists `overwrite` and `.new` as expected dry-run labels, but they are unreachable for user-modified files whenever the user passes an explicit non-interactive policy.
- **Why it matters:** Dry-run's value is auditing what a real upgrade *would* do. For `--force`/`--create-new` there is no stdin-blocking concern (the only stated reason for the Skip override), so the preview understates the plan and can mislead a user into running a destructive `--force` they did not actually preview.
- **Recommendation:** Only coerce to `Skip` when the effective policy is `Interactive` (preserving the no-stdin guarantee); otherwise use `opts.conflict_policy` so the preview reflects the chosen non-interactive policy. Add a `--dry-run --force` test asserting an `overwrite` row.
- **Resolution:** FIXED in `mod.rs` — the policy is coerced to `Skip` only `if opts.dry_run && matches!(opts.conflict_policy, ConflictPolicy::Interactive)`; an explicit `--force`/`--create-new`/`--skip-modified` is previewed faithfully. Covered by `dry_run_force_previews_overwrite` and confirmed via the built binary (`upgrade --dry-run --force` now renders an `overwrite` row).

### V-003 `Unknown keys inside [upgrade] are silently accepted, not rejected`

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/upgrade/strategy.rs:25-37` (`RawConfig` / `UpgradeSection` lack `#[serde(deny_unknown_fields)]`).
- **Problem:** The PRD's fast-fail outcome enumerates "unknown keys" as an invalid-config case that should "fail fast with a clear error before any mutation." serde defaults to ignoring unrecognized fields, so `[upgrade] ejcted = [...]` (a typo) or any stray key parses silently to empty sets and the upgrade proceeds as if no strategy were declared — the user's intent is dropped without warning.
- **Why it matters:** A mistyped `ejected`/`merged` key means the file the user meant to protect is silently re-managed and can be overwritten by `--force`, defeating the feature's core safety promise with no signal. The PRD explicitly called this out as a fail-fast case.
- **Recommendation:** Add `#[serde(deny_unknown_fields)]` to `UpgradeSection` (and consider it on `RawConfig`, weighed against forward-compat with other config sections — scoping to `UpgradeSection` is the safer choice since the loader only deserializes the `upgrade` table it owns). Add a unit test that an unknown key under `[upgrade]` surfaces `UpgradeConfigCorrupt`.
- **Resolution:** FIXED in `strategy.rs` — `#[serde(deny_unknown_fields)]` added to `UpgradeSection` (scoped there, not `RawConfig`, so other sections stay forward-compatible). A typo like `ejcted = [...]` now surfaces `UpgradeConfigCorrupt`. Covered by `unknown_upgrade_key_is_rejected`.

### V-004 `ActionLabel::MergeNoBaseFallback is dead — never produced by build_preview`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/upgrade/mod.rs:216-217` (variant), `:605-658` (`build_preview` never emits it); acknowledged in the doc comment at `:609-612`.
- **Problem:** A no-base `merged` path re-enters `classify` and is planned as `Write`/`Preserve`/`CreateNew`, so its preview row carries `overwrite`/`preserve`/`.new`, never `MergeNoBaseFallback`. The variant is defined, documented, and exported (`lib.rs` re-exports `ActionLabel`) but is unreachable in practice.
- **Why it matters:** A dead public enum variant invites a reader to assume the preview distinguishes the fallback case when it does not — the C-19/NG-4 "permanent fallback" reason is invisible in the preview. Minor: behavior is correct; only the label's promise is unmet.
- **Recommendation:** Either (a) remove the variant and the now-moot doc paragraph, or (b) actually emit it — have the planner record a fallback marker so a no-base diverged `merged` path renders as `merge-no-base-fallback`, which is the more useful UX and matches the PRD's stated label list. Pick one; do not leave it dead-but-documented.
- **Resolution:** FIXED in `mod.rs` — chose (a): removed the `MergeNoBaseFallback` variant, its `as_str` arm, and the moot doc paragraph; `build_preview`'s comment now states a no-base merged path re-enters `classify` and renders as its resolved conflict action. No dead public variant remains (clippy `-D warnings` clean).

### V-005 `Dry-run summary line under-reports actions shown in the preview`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/upgrade/mod.rs:428-433` (dry-run `UpgradeSummary` carries only `unchanged`); rendered alongside the preview by the CLI (`main.rs` `render(upgrade(...))`).
- **Problem:** On a `--dry-run` the command prints the `DryRunPreview` (authoritative) *and* an `UpgradeSummary` whose only non-zero counter is `unchanged` (from `plan.inline_unchanged`). E2E shows a preview with `1 preserve` action while the trailing summary reports `0 modified-preserved` (and `32 unchanged`). The two surfaces disagree on the same dry run.
- **Why it matters:** Cosmetic but confusing — a user reading the summary line could conclude nothing would change, contradicting the preview directly above it. The preview is the contract surface, so this is LOW.
- **Recommendation:** Either suppress the trailing `UpgradeSummary` on dry-run (the preview already reports the count) or populate the dry-run summary counters from the planned actions so the two surfaces agree.
- **Resolution:** FIXED in `mod.rs` — chose suppression: `UpgradeSummary` gained a `dry_run` flag (set on the dry-run return), and its `Display` returns early with no output when set, so the `DryRunPreview` is the sole surface. Confirmed via the built binary (`upgrade --dry-run --force` prints zero `file(s):` lines).

## Notes

- `verify_migration.rs` / the `verify_migrated` counter are **pre-existing on the branch at `start_head` (43ea1d0)** and are not part of this task's PLAN/PRD; they were correctly carried forward unchanged and are out of audit scope.
- The upgrade module was already a directory (`mod.rs`/`plan.rs`/`verify_migration.rs`) at `start_head`; the four new files (`strategy.rs`, `merge.rs`, `base_store.rs`, `backup.rs`) are this task's additions and are untracked pending commit.
- C-16 sort order verified against code: `sort_key` buckets `Write`=0 (sub-keyed by `WriteKind`), `Merged`=1, `MergeConflict`=2, `CreateNew`=3, `RefreshHashOnly`=4, `Preserve`=5, `EjectSkip`=6, `Delete`=7, `DropManifestEntry`=8 — matches the eleven-position C-16 enumeration exactly (the three `Write{*}` kinds share bucket 0 with a secondary key, resolving the iteration-01 R-010 presentational note). `new_action_variants_sort_into_their_buckets` and `plan_actions_sorts_output_by_bucket_then_path` guard it.
- C-18 verified: no `Command::new` and no bare `std::fs::` in any of the four new non-test files; all I/O routes through `io::PathExt` and all paths through `Layout` (`upgrade_base_dir`/`upgrade_backup_dir`). The existing source-scan test (`upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`) covers `mod.rs`/`plan.rs` only; the new files were inspected manually.
- C-20 manifest byte-equality verified: backup captures `serde_json::to_vec_pretty(&manifest)` which is byte-identical to `Manifest::write`'s `to_string_pretty`; `apply_failure_rolls_back_files_and_manifest` asserts the on-disk `.installed.json` returns to its pre-upgrade bytes.
- Eject double-count: confirmed clean — the removal pass skips `desired_keys` (`plan.rs:350`) so a still-shipped ejected path is counted once in the desired pass; a removed-from-template ejected path is counted once in the removal pass (`plan.rs:354-360`).
- diffy 0.5.0 is locked in `Cargo.lock`; `three_way_merge` uses `diffy::merge_bytes` (byte API) with the default `ConflictStyle::Merge`, honoring NG-3 (no CRLF normalization) and C-10 (non-UTF-8 round-trip, `non_utf8_round_trips`).
