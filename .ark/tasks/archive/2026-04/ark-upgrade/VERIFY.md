# `ark-upgrade` VERIFY `01`

> Status: Closed
> Feature: `ark-upgrade`
> Owner: Verifier
> Target Task: `ark-upgrade`
> Verify Scope:
>
> - Plan Fidelity        — does the code deliver what the final PLAN promised?
> - Functional Correctness — does it work under the Validation matrix?
> - Code Quality         — readability, naming, error handling, test depth
> - Organization         — module boundaries, file placement, cohesion
> - Abstraction          — appropriate abstractions; no premature, no leaky
> - SPEC Drift           — does PLAN's Spec section still match the shipped code?

---

## Verdict

- Decision: Approved with Follow-ups
- Blocking Issues: `0`
- Non-Blocking Issues: `4`

## Summary

Shipped code faithfully implements `02_PLAN`. All 126 unit tests + 18 `upgrade` integration tests + 7 CLI upgrade tests pass; `cargo clippy --workspace --all-targets -- -D warnings` is clean; `cargo fmt --check` is clean.

Every `01_REVIEW` and `02_REVIEW` finding from the Response Matrix is materialized in code:

- **R-013 / C-17** — `validate_manifest_paths` runs at `upgrade.rs:452` immediately after `Manifest::read` and before `check_version`; a symmetric safety pass runs over `collect_desired_templates` output at `upgrade.rs:456-464`. `Error::UnsafeManifestPath` is a distinct variant at `error.rs:85-86` and is re-mapped from `UnsafeSnapshotPath` via an explicit match.
- **R-014 / C-18** — `collect_desired_templates` at `upgrade.rs:241-258` uses the exact `dest_root.join(entry.relative_path).strip_prefix(project_root)` idiom that `init.rs::extract` (init.rs:135-144) uses. V-UT-17 (`desired_template_keys_match_init_manifest_entries`, upgrade.rs:724-736) asserts byte-equal parity.
- **R-015** — `Unchanged{refresh_hash=false}` is inline-bumped at `upgrade.rs:346-350` and emits no `PlannedAction`; `Preserve` is an explicit `PlannedAction` variant handled at `upgrade.rs:508-510`.
- **R-016 / C-19** — `plan_actions` sorts by `(bucket, relative_path)` at `upgrade.rs:408-413`; the `Bucket` enum derives `Ord` in the declared order. A plan_actions_sorts_output_by_bucket_then_path test (upgrade.rs:775-795) verifies this.
- **R-017** — Two-write manifest pattern is implemented: step-12 write at line 530, deferred deletions at lines 533-552, step-14 conditional write at line 554. Failure-mode 9 is handled cleanly.
- **R-020 / C-14** — `installed_at` refresh on every successful upgrade is intentional; acknowledged.
- **R-021** — Step order swapped: path validation (line 452) runs before `check_version` (line 453).
- **R-022 / V-UT-18** — `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` (upgrade.rs:739-773) enforces C-12 and C-13 mechanically. Grep confirms no violations outside the tests module.

Plan-then-apply pattern is honored: `plan_actions` is pure (no filesystem mutation), writes all execute in the sorted apply pass, deletions are deferred past the manifest flush barrier.

Non-blocking items are polish: four tests listed in the Validation matrix (V-F-4, V-F-5, V-F-6, V-F-8) do not have direct integration-test counterparts in `crates/ark-cli/tests/upgrade.rs`, and `absolute.exists()` at `upgrade.rs:544` bypasses `PathExt` by a hair. None of these change correctness at this tier; they are cataloged as follow-ups below.

## Findings

### V-001 `Missing integration tests V-F-4, V-F-5, V-F-6, V-F-8`

- Severity: LOW
- Scope: Plan Fidelity
- Location: `crates/ark-cli/tests/upgrade.rs` (absent)
- Problem:
  `02_PLAN` enumerates `V-F-4 corrupt_manifest_errors`, `V-F-5 write_failure_leaves_manifest_untouched`, `V-F-6 managed_block_corrupt_surfaced`, and `V-F-8 partial_write_then_rerun_classifies_written_files_as_unchanged` in the Failure / Robustness Validation section and cites V-F-6 in the Acceptance Mapping for G-10 / C-8 and V-F-8 for C-19. Searching the integration-test file finds none of these by name or by functional coverage:
    - V-F-4 — No test writes malformed JSON to `.installed.json` and asserts `Error::ManifestCorrupt` propagates from `upgrade`. `manifest.rs` covers the deserializer but not the upgrade surface.
    - V-F-5 — No test exercises a write-permission failure; `upgrade.rs`'s behavior on partial write is only reasoned about, not asserted.
    - V-F-6 — No test orphans a `<!-- ARK:START -->` in `CLAUDE.md` and asserts `Error::ManagedBlockCorrupt` from `upgrade`. The underlying `io::fs::tests::update_managed_block_errors_on_orphan_start` covers the primitive but not upgrade's propagation.
    - V-F-8 — No test reruns upgrade from a simulated partial-write state and asserts a deterministic, byte-identical manifest + disk state.
- Why it matters:
  The shipped code is correct in the primitives they would cover (propagation through `?`, the determinism sort in `plan_actions`), and 126 existing tests exercise the happy paths. Missing these four reduces confidence that refactors preserve the declared invariants — V-F-8 in particular is the only acceptance evidence for C-19's "determinism under recovery" half.
- Expected:
  Follow-up task to add the four tests. None block this task; the primitives they verify are already exercised.

### V-002 `absolute.exists() bypasses PathExt`

- Severity: LOW
- Scope: Quality
- Location: `crates/ark-core/src/commands/upgrade.rs:544`
- Problem:
  The `DropManifestEntry` branch calls `absolute.exists()` (stdlib `Path::exists`, which internally calls `std::fs::metadata`) to decide whether to increment `summary.orphaned`. C-12 states "all filesystem access in `commands/upgrade.rs` routes through `io::PathExt` / `io::fs` helpers". The V-UT-18 source-grep only forbids the literal string `std::fs::`, so this call slips through.
- Why it matters:
  Minor. The logic is correct and the error model is unreachable here (errors from `metadata` collapse to `false`). But the plan-level invariant is that every filesystem touch routes through a path-wrapped helper so I/O errors are captured with path context. Adding a `PathExt::exists_checked(&self) -> Result<bool>` helper would close the gap and let V-UT-18 add `.exists()` to its forbidden-string set.
- Expected:
  Follow-up task to add `PathExt::exists_checked` and tighten V-UT-18. Not blocking; the only consequence today is a silent `false` on a permissions error, which is acceptable because the deferred-apply pass already tolerates residuals (they reclassify correctly on the next run per the plan's Failure Flow entry 6).

### V-003 `collect_desired_templates strip_prefix panics on a misconfigured Layout`

- Severity: LOW
- Scope: Quality
- Location: `crates/ark-core/src/commands/upgrade.rs:250-252`
- Problem:
  `absolute.strip_prefix(project_root).expect("template dest under project root")` relies on `Layout::ark_dir()` / `Layout::claude_dir()` always being under `layout.root()`. That holds today because both helpers are `root.join(ARK_DIR | CLAUDE_DIR)`. The `.expect` is therefore load-bearing on `Layout`'s implementation details. If someone later adds a variant of `Layout::new` that accepts pre-joined paths or an override, this `.expect` becomes a panic path.
- Why it matters:
  The rust coding-style rule reserves `.expect` for invariants that are logically impossible. This one is structurally impossible only under the current `Layout` implementation. Matches the existing pattern in `init.rs::extract:140` (`expect("dest under project root")`), so this is at least consistent — but worth a one-line comment pinning the invariant or converting to a `?`-propagated safe-resolve.
- Expected:
  Not blocking; matches established pattern. Optional follow-up: replace the two `.expect`s with `layout.resolve_safe(relative)` symmetry calls that already exist and propagate `UnsafeManifestPath` on violation.

### V-004 `Counter-only Unchanged{refresh_hash=true} also emits a RefreshHashOnly PlannedAction, double-counting risk`

- Severity: LOW
- Scope: Correctness
- Location: `crates/ark-core/src/commands/upgrade.rs:351-359`
- Problem:
  In the `Unchanged { refresh_hash: true }` arm, `inline_unchanged += 1` AND a `PlannedAction::RefreshHashOnly` is pushed. The `RefreshHashOnly` handler at line 494-496 only updates the in-memory manifest hash — it does NOT touch `summary.unchanged` — so the accounting is correct. This matches the plan's state-transition table ("Unchanged{refresh_hash=true} → RefreshHashOnly"). The `inline_unchanged` counter is what flows into `summary.unchanged` at line 471.
  I verified this is not a double-count: `apply_writes` for `RefreshHashOnly` does not touch any counter. But the pattern is subtle: a future contributor reading "inline" and expecting "counter only" might add `summary.unchanged += 1` in the `RefreshHashOnly` arm, double-counting. The plan's own comment at line 169-172 says "Counter-only `Unchanged{refresh_hash=false}` cases are tallied inline during planning and never emit a `PlannedAction`", which is correct, but doesn't address the `refresh_hash=true` case where BOTH inline counter AND action are emitted.
- Why it matters:
  No current bug. Code comment drift risk. A single line of intra-file documentation ("// refresh_hash=true: counter bumped inline here; RefreshHashOnly updates the manifest hash only") at line 351 would inoculate against regression.
- Expected:
  Optional LOW follow-up: add a comment at the `Unchanged { refresh_hash: true }` arm explaining the split responsibility between the inline counter bump and the `RefreshHashOnly` action.

## Follow-ups

- FU-001 : `upgrade-test-gaps` — add integration tests for V-F-4 (corrupt manifest at upgrade surface), V-F-5 (write-permission failure leaves manifest version untouched), V-F-6 (managed-block-corrupt propagates through upgrade), V-F-8 (byte-identical determinism under partial-state rerun).
- FU-002 : `pathext-exists-checked` — add `PathExt::exists_checked(&self) -> Result<bool>` and route the `DropManifestEntry` branch through it; extend V-UT-18 to forbid `.exists()` in non-test upgrade source.
- FU-003 : `upgrade-comment-refresh-hash-true` — one-line comment at the `Unchanged { refresh_hash: true }` arm (upgrade.rs:351) disambiguating the inline-counter + RefreshHashOnly-action split.
