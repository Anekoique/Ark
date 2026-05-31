# `ark-upgrade` REVIEW `02`

> Status: Closed
> Feature: `ark-upgrade`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved
- Blocking Issues: `0`
- Non-Blocking Issues: `3`



## Summary

Iteration 02 fully resolves every finding from `01_REVIEW`. Verified walk-through:

- **R-013**: `C-17` + `Error::UnsafeManifestPath` + step-4 `validate_manifest_paths` + V-F-9 is a complete fix. Call-graph puts validation immediately after `Manifest::read` and before any filesystem mutation; the matching symmetric check on `collect_desired_templates` (per C-17 final sentence and step 5) closes the trust boundary consistently with `load.rs:87,91`.
- **R-014**: `C-18` pins the project-relative mapping to the exact `dest_root.join(entry.relative_path).strip_prefix(project_root)` shape used by `init.rs::extract` (verified at `init.rs:135-141`). `V-UT-17`'s `sorted(manifest.files) == sorted(collect_desired_templates().map(|(p,_)|p))` is a real parity assertion — it would catch any prefix divergence.
- **R-015**: `Unchanged{refresh_hash=false}` is now explicitly "counter-only, no PlannedAction" with the inline bump happening in `plan_actions`; `Preserve` is an explicit variant whose `apply_writes` handler bumps `modified_preserved`. No more overloaded states.
- **R-016**: `C-19` specifies a total, stable sort by `(bucket, relative_path)` with the bucket order enumerated; lexicographic tiebreak makes it total for any realistic input set. V-F-8's "two consecutive runs produce byte-identical state" is directly assertable by diffing the manifest JSON and `walk_files` output.
- **R-017**: Failure Flow entry 9 documents the step-14 write failure per the reviewer's recommended option (b) — the fallback "not-desired, absent → `DropManifestEntry`" path is genuinely self-healing.
- **R-018, R-019**: Parse-success for `--force --allow-downgrade` peers added to V-UT-14; V-F-6 reinstated under G-10/C-8 mapping.

Cross-referenced against the actual code: `load.rs:87,91` uses `layout.resolve_safe`; `manifest.rs` is today as the PLAN's "extension" column assumes (no `hashes` field yet, `record_file` in place); `init.rs::extract` produces project-relative paths of the shape `C-18` prescribes; `layout.rs::classify_unsafe` catches the full set of unsafe shapes; `templates.rs::walk` yields `include_dir`-safe relative paths (no `..`, no absolutes — compiled-in asset paths are well-formed, so `resolve_safe` symmetry is free).

Remaining notes are LOW polish. No blockers.



## Findings

### R-020 `installed_at refresh on no-op upgrade is a silent behavior change`

- Severity: LOW
- Section: `Runtime / Main Flow step 12`
- Problem:
  Step 12 unconditionally sets `manifest.installed_at = Utc::now()` even when every file classified `Unchanged{refresh_hash=false}` (counter-only, no PlannedAction). On a same-version zero-delta upgrade (`V-E-1` scenario) the manifest's timestamp still churns. This is a behavior change from `init`-only semantics and can confuse anyone diffing manifests or asserting install time in tests.
- Why it matters:
  Callers treating `installed_at` as "when did Ark first land here" will see the field re-asserted as "time of last upgrade". Integration tests that serialize the manifest into a golden file will flake. No correctness impact.
- Recommendation:
  Either (a) document in `G-*` / constraints that `installed_at` is "time of last successful upgrade" (not install time); or (b) only update `installed_at` when any `PlannedAction` actually ran. Option (a) is cheaper and matches the chosen two-write pattern — one sentence under `C-14` or a new `C-20`.



### R-021 `Version parse precedes path validation; minor information leak`

- Severity: LOW
- Section: `Runtime / Main Flow steps 3 vs 4`
- Problem:
  Step 3 (`DowngradeRefused`) fires before step 4 (`UnsafeManifestPath`). A hand-crafted malicious `.installed.json` carrying both a future version and an escape path produces `DowngradeRefused`, revealing that version parsing succeeded on the attacker's payload. Trust-boundary-wise, path safety is the stronger invariant and should fail first.
- Why it matters:
  The exposure is small (the user owns the manifest; `.installed.json` is already under their writable tree) but the ordering is inconsistent with "safety checks before semantic checks". If the PLAN ever grows additional manifest-derived semantic checks (e.g. a plugin list), the precedence order matters.
- Recommendation:
  Swap step order: run `validate_manifest_paths` immediately after `Manifest::read` and before `check_version`. Zero cost, consistent with defensive posture. No test impact — V-F-9 still passes, and V-F-2 only asserts `DowngradeRefused` happens, which it will for a safe-path downgrade manifest.



### R-022 `C-12 / C-13 acceptance entries rely on VERIFY-phase grep; cheaply mechanizable`

- Severity: LOW
- Section: `Validation / Acceptance Mapping`
- Problem:
  C-12 ("no bare `std::fs::*`") and C-13 ("no hand-joined `.ark/` literals") are marked `review-only (grep ... during VERIFY)`. These are perfect candidates for a compile-time-ish test: a `#[test]` that reads `upgrade.rs` via `include_str!` and asserts neither `std::fs::` nor `".ark/"` appears in non-comment/non-test source. Takes ~10 lines. Keeps the invariant enforced across refactors without relying on the reviewer remembering.
- Why it matters:
  The same review-only pattern in `00_PLAN` / `01_PLAN` is already inherited across three iterations; without a mechanical check, a future contributor will regress it. Same argument applies to `C-15` (dyn-compatibility — the compiler already rejects violations, but a `dyn Prompter` witness type in a test would document the intent).
- Recommendation:
  Add to Phase 2.5: "`V-UT-18`: `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` — `include_str!("upgrade.rs")` grep asserting neither appears outside `//` comments or `#[cfg(test)]`." Optional but cheap.



## Trade-off Advice

No new trade-off questions. TR-1..TR-9 from prior rounds are all accepted and reflected in the PLAN. The two-write manifest pattern (TR-8) and the `resolve_safe` extension to manifest paths (TR-9) are both materialized correctly in 02_PLAN's Main Flow, Failure Flow, and Constraints. No further advice required.
