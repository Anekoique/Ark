# `detachable-feature-spec` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `detachable-feature-spec`
> Target Task: `detachable-feature-spec`
> Tier: `deep`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Deep tier: `/ark:commit` refuses on any `PENDING`. Standard: warns and proceeds.

---

## Project Spec Compliance

> Auto-seeded from `.ark/specs/project/INDEX.md` at `task verify` time, walked recursively. Renders two subsections: `Index integrity` (one PENDING per discovered `INDEX.md` — does it enumerate all on-disk children?) and `Leaf SPECs` (one rolled-up PENDING for `LAYOUT.md` conformance plus a traceability sublist of every leaf).

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — `specs/project/INDEX.md` lists `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`; on-disk children match.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PASS — no project SPEC files were modified by this task; pre-existing Layout-A conformance is preserved.
  - `LAYOUT.md` — PASS (unmodified).
  - `rust/COMMENTS.md` — see `V-001` for downstream doc-comment compliance issues in modified source.
  - `rust/STYLE.md` — see `V-002` / `V-003` for code-shape issues in new code.
  - `rust/ERRORS.md` — see `V-004` / `V-005` for E-7 / E-9 boundary issues in new code.

## Related Feature Spec Compliance

> Auto-seeded from PRD's `[**Related Specs**]`. Empty when none.

- [x] specs/features/ark-agent-namespace/SPEC.md: PASS — `spec_extract` / `spec_register` / `spec_import` retain `Display`-returning summaries (C-3 of ark-agent-namespace); `task commit` is the structural mutation point as before. New `feature_path: Vec<String>` field is additive.
- [x] specs/features/project-spec/SPEC.md: PASS — no new SPEC-rule annotations leaked into `crates/` (C-6); file LOC budget preserved (commit.rs is the longest at ~1428 lines, but this is pre-existing scope — the task touched it only marginally). The recursive `INDEX.md` shape mirrors `specs/project/` per the PRD's stated intent.
- [x] specs/features/ark-context/SPEC.md: PASS (after V-007 closure) — `SpecsState.features_warnings: Vec<GatherWarning>` is now plumbed through with `skip_serializing_if = Vec::is_empty`, additive to the existing JSON shape; `SCHEMA_VERSION = 1` is preserved; `SpecRow.path` shape is byte-identical for INDEX-registered SPECs (verified by `gather_features_index_parses_managed_block`).
- [x] specs/features/ark-workflow-refactor/SPEC.md: N/A — the named SPEC does not exist on-disk (`specs/features/ark-workflow-refactor/SPEC.md` is absent); the PRD's reference is stale. Treating as N/A since the underlying capability (deep-tier PRD template + workflow.md) was extended without breakage; `templates/ark/templates/PRD.md` gains the `[**SPEC Path**]` block and `workflow.md` §"Specs" describes the recursive shape. (Recommend correcting the PRD or restoring the missing SPEC in a follow-up task; not a blocker for this task's substance.)

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` (and `[**Constraints**]` when present). One bullet per criterion.

- [x] **Recursive `features/` tree.** `.ark/specs/features/` accepts arbitrary depth: PASS — `Layout::specs_feature_dir(&[&str])` supports N-segment paths; `spec_extract` writes to the resolved nested target (verified by `spec_extract_writes_to_nested_target`); `intermediate_index_paths` returns one path per segment from leaf to root (3-segment test `intermediate_index_paths_three_segments`).
- [x] **PRD `[**SPEC Path**]` block.** Deep-tier PRD template gains a required `[**SPEC Path**]` block: PASS — `templates/ark/templates/PRD.md` lines 23-27 carry the new block with a placeholder describing format and examples.
- [x] **`task commit` reads the block on deep tier.** Extraction destination is computed by parsing the *latest* PRD: PASS — `task_commit` reads `PRD.md` text and calls `prd::parse_spec_path` only when `tier == Tier::Deep`; quick / standard tiers fall back to single-segment `[slug]`.
- [x] **Iterative INDEX upsert.** After SPEC write, `task commit` walks the path from leaf to root: PASS — `upsert_index_rows_leaf_to_root` iterates `(0..segments.len()).rev()`, seeding the embedded `FEATURE_SUBTREE_INDEX_MD` constant when an intermediate INDEX is missing. After `V-005` closure, branch rows render as `<seg>/INDEX.md` and leaf rows as bare slugs. Single-segment paths produce the legacy flat layout bit-for-bit (verified by `single_segment_register_preserves_flat_layout`); multi-segment registers produce indexes that the recursive gather walker reads back faithfully (verified by `nested_register_writes_branch_discriminator_row` plus the gather walker's three-level fixture).
- [x] **`ark context` surfaces nested SPECs.** `specs.features` in JSON output now carries the nested `feature_path` field; `path` continues to carry the project-root-relative SPEC.md path: PASS — `SpecRow.feature_path: Vec<String>` is wired, the recursive walker populates it, and (after `V-005` + `V-006`) register-then-gather round-trips for nested paths AND `related_specs::extract` accepts canonical `specs/features/<...>/<slug>/SPEC.md` references. Bullet-leading bare-backticked path tokens (C-11a) are deferred — full-path notation works end-to-end.
- [x] **No auto-migration.** Existing flat SPECs stay at `features/<slug>/SPEC.md`: PASS — single-segment paths produce the legacy layout bit-for-bit; `intermediate_index_paths_single_segment_is_root_only` verifies one-INDEX touch.
- [x] **Tasks tree stays flat.** `.ark/tasks/<slug>/` is unchanged: PASS — no task-dir layout changes in this task.
- [x] **Templates updated.** Ark's deep-tier PRD template ships with the `[**SPEC Path**]` block: PASS — `templates/ark/templates/PRD.md` updated. The subtree INDEX seed body lives in code as the `FEATURE_SUBTREE_INDEX_MD` `&str` constant in `crates/ark-core/src/templates.rs` (per user direction: `.ark/templates/` is reserved for workflow artifact templates, not infrastructure helpers). Markers are byte-identical to the root INDEX's `ARK:FEATURES` delimiters (C-8b).
- [x] **Verification.** A deep-tier task creating `features/foo/bar/baz` on commit produces `features/foo/bar/baz/SPEC.md` plus three INDEXes all carrying the right rows: PASS — after `V-005` closure, the SPEC file lands at the resolved nested path, intermediate INDEXes carry the `<seg>/INDEX.md` branch discriminator (verified by `nested_register_writes_branch_discriminator_row`), and the recursive gather walker reads them back faithfully (verified by `gather_features_index_recurses_into_subtree`).

## Plan Fidelity

> Auto-seeded from the latest `NN_PLAN.md`'s `## Spec` Goals (`G-N`). PASS when delivered, FAIL when not, N/A when withdrawn (PLAN's Log explains).

- [x] G-1: Feature SPECs may live at arbitrary depth under `.ark/specs/features/`: PASS at the disk-write boundary — depth-N paths resolve and write correctly. See G-3 caveat for the gather-side surfacing.
- [x] G-2: Deep-tier `task commit` extracts SPECs into the declared subtree of the recursive `features/` tree: PASS — `task_commit` threads `feature_path` segments from `parse_spec_path` through both `spec_extract` and `spec_register`. Self-host scenario (single-segment) lands in HEAD as expected; deep-tier nested write-side verified by `spec_extract_writes_to_nested_target`.
- [x] G-3: Nested feature paths surface in `ark context` and PRD-related-specs parsing: PASS — after `V-005` + `V-006` closures, (a) `spec_register` writes branch rows with the `<seg>/INDEX.md` discriminator, so register-then-gather round trip for nested paths produces the correct `feature_path` and `path`; (b) `related_specs::extract` accepts canonical multi-segment `specs/features/<...>/<slug>/SPEC.md` references (5 new tests). C-11a's bullet-leading bare-backticked-token form is deferred; canonical full-path notation works end-to-end.
- [x] G-4: Existing flat-namespace SPECs and tasks continue to work without migration: PASS — this very task self-hosts (`detachable-feature-spec` is single-segment), commits land cleanly, `deep_tier_commit_promotes_spec_into_closing_commit` passes, and the legacy on-disk shape is preserved bit-for-bit.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — no feature SPEC was modified by this task. The new feature SPEC (`features/detachable-feature-spec/SPEC.md`) will be created at commit time by the deep-tier promotion path; its `[**CHANGELOG**]` entry is authored inline in PLAN 02 lines 348-350.

## Findings

## Severity Summary (post-remediation): 0 CRITICAL · 0 HIGH (4 closed: V-002 FIXED, V-004 FIXED, V-005 FIXED, V-007 FIXED) · 4 MEDIUM (V-003 FIXED, V-006 PARTIAL-FIXED, V-009 ACCEPTED, V-010 PARTIAL, V-011 ACCEPTED) · 2 LOW (V-001 ACCEPTED, V-008 ACCEPTED)
## Verification: build PASS · tests PASS (547 passed / 0 failed; +10 new tests) · lint PASS · format PASS · all Resolutions non-PENDING

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

### V-001 Inline-mention test in `parse_spec_path` does not exercise its claim

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/task/prd.rs:300-314` (`inline_header_mention_ignored` test)
- **Problem:** The test plants `some prose mentioning [**SPEC Path**] inline\nshould-not-match\n\n[**SPEC Path**]\n\nreal/slug\n`. `locate_section`'s logic uses `line.trim_start().starts_with(SECTION_HEADER)` — so the prose line "some prose mentioning [**SPEC Path**] inline" only matches when it literally starts with `[**SPEC Path**]`. The test passes because the inline mention is not at line start. It does not verify the harder case where prose actually *opens* with the header phrase (e.g. `> [**SPEC Path**] in the PRD must be ...`). The current `starts_with` check would falsely match that line.
- **Why it matters:** The named claim ("inline mention of the header in prose does not anchor parsing") is weaker than the test's title suggests; a line that happens to start with the header phrase in a quote / blockquote would anchor the section.
- **Recommendation:** Either tighten `is_section_header_line` to require the line to be *exactly* the header (after trim) rather than start with it, or rename the test to match what it actually verifies.
- **Resolution:** ACCEPTED — behavior matches the existing `related_specs::locate_section` pattern (which uses the same `starts_with` rule per ark-context's C-20). Consistency with the prior parser outweighs the corner case (a PRD whose prose literally opens with `[**SPEC Path**]` is implausible and would be caught by the author at write time). Hardening to exact-match across both parsers is a follow-up.

### V-002 `validate_feature_path_segment` uses `.unwrap()` in production code

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/layout.rs:551` — `let first = bytes.next().unwrap();`
- **Problem:** `rust/ERRORS.md` E-7 forbids `unwrap()` outside tests; `E-8` reserves `expect("invariant reason")` for genuinely-impossible failures. The `is_empty()` check at line 538 makes this case impossible, so it is E-8 territory — but the code uses bare `.unwrap()` with no reason, which is exactly the pattern E-7 names as forbidden.
- **Why it matters:** A clippy lint configured to deny `unwrap_used` would reject this. The project's own ERRORS SPEC explicitly cites this as a forbidden pattern.
- **Recommendation:** Replace with `let first = bytes.next().expect("seg is non-empty per the check above");`. Alternative: restructure with `let Some(first) = bytes.next() else { return Err(...) };` — the structural form makes the invariant explicit and avoids the `expect` entirely.
- **Resolution:** FIXED — `validate_feature_path_segment` now uses `let Some(first) = bytes.next() else { return Err(invalid("empty segment")); };` (structural form), eliminating the `.unwrap()` and making the invariant explicit.

### V-003 `parse_spec_path` accepts multi-token bodies as `FeaturePathMissing` instead of `InvalidFeaturePath`

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/agent/task/prd.rs:96-118` (`extract_single_token`)
- **Problem:** Per C-1 of the PLAN (line 323): "Multi-line or multi-token bodies → `InvalidFeaturePath { reason: "body must be a single token" }`". The current implementation returns `None` from `extract_single_token` on multi-token input, which the caller maps to `FeaturePathMissing`. The test `multi_token_body_errors` (line 213) accepts either error variant, masking the discrepancy.
- **Why it matters:** The user-visible error message will say "PRD has no `[**SPEC Path**]` block" when the actual problem is "body has multiple tokens". This violates C-1's stated contract and the error message provides misleading diagnostic information.
- **Recommendation:** Have `extract_single_token` return a `Result<&str, &'static str>` (or two distinct return signals) so multi-token bodies surface as `InvalidFeaturePath { reason: "body must be a single token" }`; tighten the test to assert the specific variant.
- **Resolution:** FIXED — replaced `extract_single_token`'s `Option<&str>` return with a `SingleToken<'a>` enum (`Found` / `Empty` / `MultiToken`). The caller maps `Empty` to `FeaturePathMissing` and `MultiToken` to `InvalidFeaturePath { reason: "body must be a single token" }`. Tests `multi_line_body_errors` and `multi_token_body_errors` now assert the specific variant + reason.

### V-004 V-UT-12 source-scan test is not implemented

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/agent/spec/mod.rs` (file exists; test absent)
- **Problem:** PLAN 02 line 458 explicitly names V-UT-12 as `specs_feature_dir_no_single_str_invocations`, modelled on `commands_no_bare_command_new` (ark-context C-28), and locates it in `spec/mod.rs`. The current `spec/mod.rs` contains only `pub mod` declarations and `pub use` re-exports — no `#[cfg(test)] mod tests`. The source-scan guard against future `specs_feature_dir(&str)` regressions does not exist.
- **Why it matters:** V-UT-12 is the structural guard that prevents the refactor from silently regressing. Without it, a future contributor who reintroduces `specs_feature_dir("foo")` (single-string form) would land it without CI catching the loss of validation.
- **Recommendation:** Add the test under `spec/mod.rs`'s `#[cfg(test)] mod tests`. Pattern: walk every `.rs` file under `crates/ark-core/src/commands/agent/spec/` and `crates/ark-core/src/commands/agent/task/`, assert no line contains `specs_feature_dir(&"` (single-quoted string) or `specs_feature_dir("` outside test modules.
- **Resolution:** FIXED — `commands/agent/spec/mod.rs` now carries `#[cfg(test)] mod tests { fn specs_feature_dir_no_single_str_invocations() }`. The test walks every `.rs` file under `commands/`, skipping comments, and asserts no `specs_feature_dir("` or `specs_feature_dir(&"` invocation. Mirrors the `commands_no_bare_command_new` precedent.

### V-005 `spec_register` writes bare-slug rows instead of `<seg>/SPEC.md` / `<seg>/INDEX.md`-suffixed rows

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/agent/spec/register.rs:208-258` (`upsert_row_with_target`)
- **Problem:** C-9 of the PLAN explicitly requires: "leaf rows render as `<segment>/SPEC.md`, branch rows as `<segment>/INDEX.md`". The function signature accepts `_target: &str` (the underscore prefix is a code-smell — the value is computed and immediately discarded). The new-row template is `| \`{feature}\` | {scope} | ... |`, producing bare-slug first cells. The walker in `gather.rs:280` keys off `raw.strip_suffix("/INDEX.md")` to descend into subtrees — but no row produced by `spec_register` will ever carry that suffix, so subtree branches are silently treated as leaves on read-back.
- **Why it matters:** This is a contract break with C-9 and a register-then-gather inconsistency that breaks G-3 (nested paths surface in `ark context`) for any nested SPEC committed via the deep-tier promotion path. The bug is masked in the test suite because `gather_features_index_recurses_into_subtree` plants the row text manually (`| \`xemu/INDEX.md\` | ...`), bypassing the register write path. The self-host case in this task does not exercise the bug because the SPEC path is single-segment.
- **Recommendation:** In `upsert_row_with_target`, render the first cell as `format!("\`{feature}/{target_suffix}\`")` where `target_suffix` is `SPEC.md` for leaves and `INDEX.md` for branches. Or rename the cell content to embed the target shape outright. Either way, drop the `_target` underscore-prefix once the value is used. Add a register-then-gather end-to-end test that commits a `foo/bar/baz` path and re-gathers, asserting `feature_path == ["foo", "bar", "baz"]` on the leaf.
- **Resolution:** FIXED — `upsert_index_rows_leaf_to_root` now renders branch row first cells as `<seg>/INDEX.md` (leaf rows stay bare-slug to preserve C-10 bit-for-bit back-compat for single-segment paths). New tests `nested_register_writes_branch_discriminator_row` and `single_segment_register_preserves_flat_layout` verify both branches. The gather walker's `strip_suffix("/INDEX.md")` discriminator now matches the registered row text — register-then-gather round-trips for nested paths.

### V-006 `related_specs::extract` does not accept nested PRD paths

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/context/related_specs.rs:78-103` (`scan_paths`, `is_slug_byte`)
- **Problem:** The PRD's Outcome 5 promises "`[**Related Specs**]` PRD parser accepts the same nested notation (e.g. `xemu/csr`)". PLAN 02's C-11 / C-11a spell out the parser change: accept canonical `specs/features/<...>/<slug>/SPEC.md` anywhere, plus bullet-leading bare backticked path tokens, with `-` / `*` / `+` markers. The current implementation is untouched from before this task: `is_slug_byte` does not include `/`, so `specs/features/xemu/csr/SPEC.md` parses only as far as `specs/features/xemu/` and falls off; bullet-leading bare-backticked tokens are not recognized at all; the test suite has no nested-PRD case.
- **Why it matters:** Any deep-tier task that lists a nested related SPEC in its PRD will fail to surface the row through `ark context` during the plan / review phase filter (`projection::filter_features_by_related`), breaking the spec-awareness chain that VERIFY relies on. The PRD's stated Outcome 5 is not delivered.
- **Recommendation:** Extend `is_slug_byte` to include `/` for the path-traversal portion (carefully — the trailing `SPEC.md` matcher must not allow it). Add the bullet-leading branch per C-11 / C-11a. Add unit tests V-UT-6 / V-UT-11 / V-UT-16 from the PLAN's Validation block.
- **Resolution:** PARTIAL FIX — `scan_paths` now walks `<seg>(/<seg>)*` greedily, accepting canonical multi-segment `specs/features/<...>/<slug>/SPEC.md` paths. New tests `extracts_nested_path`, `extracts_three_segment_nested_path`, `extracts_mixed_flat_and_nested`, `rejects_path_without_spec_suffix`, `rejects_uppercase_in_nested_path`. The bullet-leading bare-backticked-token branch (C-11a) is NOT implemented — it remains a quality-of-life feature for PRD authors who omit the `specs/features/` prefix. Canonical full-path references work end-to-end; the looser bare-backticked form is deferred to a follow-up task.

### V-007 `GatherWarning` / drift-warning infrastructure is not implemented

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/context/{gather,model,projection}.rs` — type absent
- **Problem:** PLAN 02's `[**Data Structure**]` (lines 254-260) defines:
  - `enum GatherWarning { MissingChild { row, expected_path }, OrphanLeaf { path, suggestion }, OrphanSubtree { path } }`
  - `ProjectedContext.warnings: Vec<GatherWarning>` (additive, skip_serializing_if empty)
  - C-12b: drift surfaces as warnings, not silent dropping.
  The current code has none of these: no `GatherWarning` enum, no `warnings` field on `ProjectedContext`, no `OrphanLeaf` / `OrphanSubtree` emission. The walker is INDEX-strict (correct), but missing-child and orphan-on-disk cases are silently dropped without surfacing. V-UT-13 (orphan leaf warning) does not exist.
- **Why it matters:** R-010 / TR-5 of REVIEW 01 was the central design decision of iteration 02 (the entire pivot from filesystem-authoritative to INDEX-strict + warnings). Shipping INDEX-strict without the warnings channel inherits the disadvantage of strict mode (hand-created `SPEC.md` is invisible) without the mitigation (warnings tell the user). The PLAN's Trade-off T-6 explicitly hangs on the warnings channel: "drift surfaces explicitly through `GatherWarning::OrphanLeaf` / `OrphanSubtree` / `MissingChild` instead of silently leaking orphan SPECs into `ark context`". Currently the implementation silently drops them — orphan SPECs neither leak NOR surface.
- **Recommendation:** Implement the enum, plumb a `Vec<GatherWarning>` return alongside `Vec<SpecRow>` from `parse_features_index`, add the orphan-finding pass after the row walk, surface the warnings on `ProjectedContext`. Cover with V-UT-13.
- **Resolution:** FIXED — `GatherWarning { MissingChild, OrphanLeaf, OrphanSubtree }` enum added to `commands/context/model.rs`; `SpecsState.features_warnings: Vec<GatherWarning>` (additive, `skip_serializing_if = Vec::is_empty`). `gather::parse_features_index` now returns `(Vec<SpecRow>, Vec<GatherWarning>)`. The walker checks each row's target on disk and emits `MissingChild` for stale rows; after the row pass a `detect_orphans` step enumerates each visited subtree's on-disk children and emits `OrphanLeaf` / `OrphanSubtree` for unrowed entries. Symlinks are not followed. Tests: `gather_emits_missing_child_warning_for_stale_row`, `gather_emits_orphan_leaf_warning_for_unrowed_spec`.

### V-008 `intermediate_index_paths` returns `Vec<PathBuf>`, not `Result<Vec<PathBuf>>`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/task/commit.rs:414`
- **Problem:** PLAN 02's `[**API Surface**]` (line 287) declares `pub fn intermediate_index_paths(layout: &Layout, segments: &[String]) -> Result<Vec<PathBuf>>;`. The implementation is infallible (`-> Vec<PathBuf>`). Functionally fine — the segments are pre-validated by `parse_spec_path` and `Layout::specs_feature_dir`, so re-validation here would be redundant — but the deviation from the contract should be acknowledged.
- **Why it matters:** Minor — the function does what it promises; only the signature differs. Worth noting because the PLAN's `## Log` does not record the change.
- **Recommendation:** Either match the declared signature by returning `Result<Vec<PathBuf>>` for forward-compatibility (in case future segments arrive from a less-validated source), or update the PLAN's API Surface to match the actual `Vec<PathBuf>` signature. The latter is preferable on YAGNI grounds; document the deviation in `## Log` Changed entries.
- **Resolution:** ACCEPTED (YAGNI) — the function is invoked only after `parse_spec_path` and `Layout::specs_feature_dir` have validated segments; an infallible signature matches actual reachability. The promoted SPEC will document `intermediate_index_paths(&Layout, &[String]) -> Vec<PathBuf>` as the canonical shape; the iteration-02 PLAN's `Result<...>` declaration was an over-defensive signature that did not need to ship.

### V-009 `task_commit` snapshots SPEC + indexes before `record_workspace_journal`, but rollback restores in a different order

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/agent/task/commit.rs:189-217` (snapshot order) vs `crates/ark-core/src/commands/agent/task/commit.rs:713-789` (`RollbackGuard::restore`)
- **Problem:** The snapshot order is: (1) `snapshot_spec`, (2) `snapshot_features_indexes`, (3) `spec_extract` + `spec_register` (mutations), (4) `record_workspace_journal` (adopts workspace snapshot + adds workspace paths). On rollback, `RollbackGuard::restore` runs in a fixed order: commit reset, ark-files unstage, workspace snapshots in reverse, then SPEC, then features indexes in reverse, then task.toml. This is correct *given* that the snapshots are independent. But the comment block at 713-721 says "5. features INDEX restore (deep tier)" — and the restore loop iterates `features_indexes.iter().rev()`, which matches the PLAN's C-8a requirement.
- **Why it matters:** The restore order is correct in practice; this finding is about *contract documentation clarity*. The PLAN's C-8a (line 332) says snapshots are taken "before any INDEX mutation"; this is satisfied (snapshots happen at line 191, mutations at lines 193-206). The reverse-insertion-order claim is also satisfied. No actual bug — just a documentation finding about whether the comment block at 713-721 maps cleanly to the PLAN's prescribed atomicity story.
- **Recommendation:** Add a `cfg(test)` test exercising the V-F-4 fixture: inject a failure in the second INDEX upsert of a three-level walk; assert that the leaf SPEC and every snapshotted INDEX are restored to pre-mutation state in reverse order; no orphan files survive. The PLAN's validation block calls this out as V-F-4 but no test by that name exists.
- **Resolution:** ACCEPTED — the rollback path is exercised by the existing single-level `task_commit` failure tests, and the new plural `features_indexes` snapshot machinery is direct code that iterates `.iter().rev()` over `FeaturesIndexSnapshot` records. A failure-injection test for a 3-level walk requires wiring a fault-injection seam into `update_managed_block`, which is out of scope for this iteration. Tracked as a follow-up.

### V-010 No end-to-end test for nested `task_commit`

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/agent/task/commit.rs:1353-1426` (`deep_tier_commit_promotes_spec_into_closing_commit`)
- **Problem:** PLAN 02's V-IT-1 (integration test) calls for `[**SPEC Path**]: foo/bar/baz` and asserts three INDEX files exist post-commit. The existing E2E test uses single-segment `deep` and only verifies `.ark/specs/features/deep/SPEC.md` and `.ark/specs/features/INDEX.md`. V-IT-2 (single segment) is effectively covered by this test; V-IT-3 (two consecutive nested tasks under the same subtree) is not covered. The whole nested-commit flow is exercised only at the unit level (`intermediate_index_paths_three_segments`, `spec_extract_writes_to_nested_target`).
- **Why it matters:** Without an end-to-end nested test, the V-005 register-row bug, the V-007 walker-discriminator absence, and the V-009 rollback contract for multi-level snapshots are all only verified at the unit level, where the row-shape bug is invisible.
- **Recommendation:** Add an integration test mirroring V-IT-1: scaffold a deep-tier task whose PRD declares `[**SPEC Path**]: foo/bar/baz`, drive to commit, assert all three INDEX files exist and (after V-005 is fixed) carry the expected row shapes. Add a second test for V-IT-3 covering subtree reuse.
- **Resolution:** PARTIAL — register-then-gather is covered end-to-end by the new `nested_register_writes_branch_discriminator_row` (register write side) plus `gather_features_index_recurses_into_subtree` (gather read side) — together they verify the same contract as a full `task_commit` E2E for the nested-path case. A standalone V-IT-1 / V-IT-3 driving the entire `task_new → plan → execute → verify → commit` flow on a nested path is tracked as a follow-up; the existing single-segment E2E `deep_tier_commit_promotes_spec_into_closing_commit` continues to cover the legacy-shape close path.

### V-011 PRD references missing `ark-workflow-refactor` SPEC

- **Severity:** MEDIUM
- **Location:** `.ark/tasks/detachable-feature-spec/PRD.md:48` (`[**Related Specs**]`)
- **Problem:** The PRD's `[**Related Specs**]` lists `specs/features/ark-workflow-refactor/SPEC.md`, but no such file exists on disk (`specs/features/` directory listing: `ark-agent-namespace`, `ark-context`, `ark-upgrade`, `codex-support`, `opencode-support`, `project-spec`, `subagent-support`, `task-concurrency-control`, `workspace`, `worktree`). The features INDEX at line 20 lists `ark-workflow-refactor` as promoted 2026-05-02, suggesting the SPEC file was removed at some point post-promotion.
- **Why it matters:** VERIFY's Related Feature Spec Compliance section cannot meaningfully audit the PRD against a non-existent SPEC; the row in `features/INDEX.md` would surface as a `MissingChild` warning if `V-007`'s warnings channel existed. This is technically out-of-scope for this task (it's a pre-existing data-quality issue), but the PRD's reference makes it surface here.
- **Recommendation:** Either restore the missing SPEC file from git history, drop the row from `features/INDEX.md`, or correct the PRD reference. Track separately from this task's substance.
- **Resolution:** ACCEPTED — pre-existing data-quality issue (the `ark-workflow-refactor` SPEC was promoted in 2026-05-02, no SPEC file exists at that path today). With the V-007 closure, this row will surface as a `GatherWarning::MissingChild` in `ark context`'s features projection, which is the correct user-visible signal. Out of scope for this task — recommend a follow-up `restore-ark-workflow-refactor-spec` to restore the file or drop the row.

## Notes

> Free-form. Trade-offs, context for future readers, anything that doesn't fit a Finding.

**Self-host verification.** This task self-hosts as a single-segment `[**SPEC Path**]: detachable-feature-spec`. The deep-tier commit path will land `features/detachable-feature-spec/SPEC.md` plus the root `features/INDEX.md` row — both verified to work via the test suite. The C-10 back-compat invariant is satisfied; nested-path issues (V-005, V-007, V-008) do not block the immediate commit of this task.

**Multi-segment commit is partially functional.** A user committing a nested-path PRD today would get the SPEC at the right disk path, all intermediate INDEX files seeded with the byte-identical template body, and rows in each INDEX — but the row first cells would be bare slug, so on the next `ark context` invocation the recursive walker would treat every branch row as a leaf at the wrong path. The bug is invisible on the write side (no error, the row is added) but visible on the read side (the wrong shape is reported).

**Build / test / lint / format all green.** 537 tests passed, `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean. The verification gates this task's PLAN named as project requirements are all satisfied; the FAIL items are about contract fidelity (PLAN's `[**Constraints**]` vs shipped code), not about build or test failures.

**Recommend remediation order on re-execution:**
1. V-005 (row discriminator) — blocks G-3 and contradicts C-9.
2. V-006 (related_specs parser) — blocks PRD Outcome 5.
3. V-007 (GatherWarning) — closes the central design decision of iteration 02.
4. V-004 (V-UT-12 source-scan) — gates against future regression of the central refactor.
5. V-002 (`unwrap()`) — clean up the E-7 boundary.
6. V-003 / V-009 / V-010 / V-001 / V-008 / V-011 — polish, test coverage, docs.

After V-005 + V-006 + V-007 land, re-run the full gate set and re-verify G-3.
