# `ark-upgrade` REVIEW `01`

> Status: Closed
> Feature: `ark-upgrade`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved
- Blocking: `0`
- Non-blocking: `1`

## Summary

The revision clears every iteration-00 finding, and each Response-Matrix claim holds up against the source it touches. The mandatory gate is satisfied: the `NG-2` supersede is now an explicit `## Log [Changed]` entry (and `NG-1` is confirmed untouched in `[Removed]`), so the SPEC can promote with a faithful CHANGELOG line. The substantive HIGHs — the bogus "reuse RawConfig" claim, the backup lifecycle, the manifest-in-backup gap, and the permanent-fallback hole — are now real constraints (C-19..C-22) each carrying a distinct validation, and they match how `worktree/config.rs`, `upgrade/mod.rs`, `plan.rs`, and `unload.rs` actually behave. The design is implementable as written; the one open item is LOW presentational polish that does not block EXECUTE.

---

## Findings

### R-010 `Data Structure bucket comment compresses the 11-position C-16 order into "0..=7"`

- **Severity:** LOW
- **Section:** `## Data Structure` (plan.rs comment, lines ~134-137) vs. `## Spec` C-16
- **Problem:** The Data Structure block annotates the new variants as "write buckets 0..=2", "buckets 3..=5", "buckets 6..=7" — eight bucket-classes — while C-16 enumerates eleven distinct ordered positions (`Write{Add}` < `Write{AutoUpdate}` < `Write{Overwrite}` < `Merged` < `MergeConflict` < `CreateNew` < `RefreshHashOnly` < `Preserve` < `EjectSkip` < `Delete` < `DropManifestEntry`). The existing `sort_key` collapses the three `Write{*}` kinds into a single primary bucket (0) disambiguated by a secondary `WriteKind` key (`plan.rs:73-82`), so the comment's compressed numbering is reconcilable with both C-16 and the code — but the two surfaces use different counting and a reader must cross-check to see they agree.
- **Why it matters:** Purely presentational. C-16 is the authoritative, unambiguous ordering and V-UT-10 guards it; the executor will implement against C-16 and the existing `sort_key` shape regardless. No behavioral risk.
- **Recommendation:** When implementing, either restate the Data Structure comment in C-16's eleven-position terms or note explicitly that the three `Write{*}` kinds share primary bucket 0 with a secondary `WriteKind` key (mirroring `plan.rs:73-82`). No plan re-spin required; resolve inline in EXECUTE.

---

## Trade-off Advice

### TR-1 `Gitignored sidecar vs. manifest base64 for base storage`

- **Related Plan Item:** `T-1`
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer A (sidecar) — agree, and the plumbing gap is now closed
- **Advice:** Keep the gitignored sidecar. C-25 now pins both `unload` walk sites (`capture_skip_paths` widened from `[PathBuf; 3]`, consumed at `unload.rs:88` and `:174`) and states the round-trip-drops-bases consequence, which was the only open cost of this choice.
- **Rationale:** `.ark/.installed.json` is git-tracked (research-confirmed), so manifest base64 would commit and churn the corpus per host; the sidecar keeps bytes local consistent with `.state.toml` / `.developer` handling. The remaining cost (round-trip reverts merged paths to fallback) is now a stated, tested boundary (V-E-2, V-IT-8) rather than a silent surprise.
- **Required Action:** Adopt — no further action.

### TR-2 `diffy crate vs. hand-rolled diff3`

- **Related Plan Item:** `T-2`
- **Topic:** Correctness vs Dependency Footprint
- **Reviewer Position:** Prefer A (take the crate) — agree
- **Advice:** Adopt `diffy 0.5 merge_bytes`; Phase 3 still carries the docs.rs ergonomics check (`MergeOptions` / `ConflictStyle`, byte API not utf8), which is the right place for it.
- **Rationale:** diff3 false-conflict minimization is hard to reimplement; byte API matches Ark's `Vec<u8>` flow; the SPEC CHANGELOG note records the deliberate first-of-kind dependency.
- **Required Action:** Adopt — verify builder names against `docs.rs/diffy/0.5.0` during Phase 3 as planned.

### TR-3 `One backup dir for both rollback and regret-restore`

- **Related Plan Item:** `T-5`
- **Topic:** Simplicity vs Correctness
- **Reviewer Position:** Prefer A (one dir) — agree now that retention is specified
- **Advice:** Keep the single backup dir. C-21 now pins the lifecycle (one dir per non-dry-run upgrade, replaces prior, retained after success, untouched by auto-rollback) and C-20 puts the manifest in the set, which jointly resolve the "half-succeeded `--restore`" ambiguity the PRD flagged.
- **Rationale:** Restoring pre-upgrade bytes onto an already-rolled-back tree is idempotent, so a post-auto-rollback `--restore` is a harmless no-op/refusal (V-IT-9) — the two use cases coexist on one mechanism without a destructive interaction.
- **Required Action:** Adopt — no further action.

---

## Resolution of iteration-00 findings

- R-001 (CRITICAL): **Resolved.** `## Log [Changed]` names the ark-upgrade SPEC `NG-2` supersede ("was 'No backup directory; rollback is not promised'") and points to G-5 / C-13..C-15, C-20, C-21; `[Removed]` confirms `NG-1` (no migration manifest) is untouched and clarifies the sidecar holds bytes Ark wrote, not fetched/migration data. The mandatory supersede-record gate is satisfied.
- R-002 (HIGH): **Resolved.** C-1 drops the false "reuse RawConfig" claim; C-22 + the Data Structure define an own private `RawUpgradeConfig` struct off `layout.config_file()` and a distinct `UpgradeConfigCorrupt` error; V-UT-9 asserts a malformed `[upgrade]` section does not surface as `WorktreeConfigCorrupt`. Matches the private, worktree-only `RawConfig` at `config.rs:24`.
- R-003 (HIGH): **Resolved.** C-21 pins creation (per non-dry-run upgrade), prior-backup replacement, post-success retention, and "most recent" via auto-rollback leaving the backup untouched; V-IT-9 distinguishes the success-then-restore case from the auto-rollback case.
- R-004 (HIGH): **Resolved.** C-20 adds `.ark/.installed.json` (and the in-memory `Manifest`) to the backup set; Main Flow step 6 captures it before the writes and the mid-sequence flush at `mod.rs:350`; V-F-5 asserts a deletion-phase failure restores the manifest, not just the tree.
- R-005 (HIGH): **Resolved.** NG-4 + C-19 state the permanent-fallback boundary for a Skip-preserved diverged merged path; V-IT-8 asserts it stays on fallback across repeated upgrades; the preview's `MergeNoBaseFallback` label (C-24, ActionLabel) surfaces the reason.
- R-006 (MEDIUM): **Resolved.** C-23 + the revised C-16 place `Merged`/`MergeConflict` in write-adjacent buckets and `EjectSkip` with `Preserve`, consistent with the existing `sort_key` buckets at `plan.rs:73-82`; V-UT-10 extends the sort test. (See R-010 for a LOW presentational note on the bucket comment.)
- R-007 (MEDIUM): **Resolved.** `DryRunPreview` is a `Display`-able type with an `ActionLabel` enum (eleven labels incl. `MergeNoBaseFallback`); C-24 ties it to the one-render-per-dispatch convention; V-IT-2 is tightened to assert specific per-path labels and byte-identical disk + manifest.
- R-008 (MEDIUM): **Resolved.** C-25 widens `capture_skip_paths` to both sidecar dirs and requires both `unload` walk sites consume the widened set; the round-trip-drops-bases consequence is stated in State Transitions; V-E-2 asserts both dirs are absent from the snapshot.
- R-009 (LOW): **Resolved.** `remove.rs` is dropped from the touched-files list (Architecture comment confirms the `.ark/` wipe at `remove.rs:86` covers both sidecars); V-E-2 keeps the wipe as a guard assertion only.

No new CRITICAL or HIGH issues were introduced by the revisions. The `## Spec` (Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints) is self-contained — no `iteration 00` or `R-NNN` references leak into the Spec proper; finding labels appear only in the `## Log` Response Matrix. Zero open blocking findings: approved to EXECUTE.
