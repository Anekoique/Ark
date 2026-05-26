# `ark-upgrade` REVIEW `00`

> Status: Closed
> Feature: `ark-upgrade`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Rejected
- Blocking: `1`
- Non-blocking: `8`

## Summary

The design is strong: the four features are coherently phased, the sidecar/diff3/fallback trio is the right shape, the research is faithfully reflected, and the validation matrix is largely real tests rather than hand-waves. It is rejected on one mandatory gate: the plan revises the prior SPEC's `NG-2` ("No backup directory; rollback is not promised") and arguably `NG-1`, but its `## Log` `[Changed]`/`[Removed]` sections both say "N/A" — the supersede is named only in Trade-off `T-5` and the Summary, which does not satisfy the mandatory `## Log` supersede rule (R-001). Beyond that, several HIGH issues need closing before EXECUTE: the "reuse the existing `RawConfig` loader" claim is factually wrong (R-002), the backup artifact conflates two different lifecycles and its interaction with the mid-sequence manifest flush is unspecified (R-003, R-004), and the central self-healing claim has an unaddressed permanent-fallback hole for Skip-preserved diverged files (R-005). None require a structural redraft — the architecture stands — so a single revision iteration should clear them.

---

## Findings

### R-001 `NG-2 supersede not recorded in the ## Log`

- **Severity:** CRITICAL
- **Section:** `## Log` `[Changed]` / `[Removed]` (vs. `## Trade-offs` T-5, `## Summary`)
- **Problem:** The plan adds `.ark/.upgrade-backup/` and promises rollback + `--restore`, directly contradicting the existing `ark-upgrade` SPEC `NG-2` ("No backup directory; rollback is not promised") at `.ark/specs/features/ark-upgrade/SPEC.md:12`. The revision is mentioned in `T-5` and the Summary, but the `## Log` `[Changed]` and `[Removed]` blocks both read "N/A — initial plan." The mandatory review rule requires that a contradiction of an existing feature SPEC non-goal be named by an explicit `## Log` Removed/Changed entry naming the supersede. It is not.
- **Why it matters:** This is the gate's hard rule. The `## Log` is the machine-auditable supersede record that the next iteration's Response Matrix and the SPEC CHANGELOG promotion both key off; burying a non-goal reversal in prose under Trade-offs means the SPEC could be promoted without a CHANGELOG line recording that `NG-2` was reversed. Same risk applies to `NG-1` ("no migration manifest system") if the sidecar is read as migration state — clarify whether it is in-scope of `NG-1` or distinct.
- **Recommendation:** Add a `## Log` `[Changed]` entry naming the supersede explicitly, e.g. "Changed: ark-upgrade `NG-2` — was 'No backup directory; rollback is not promised'; now upgrade captures a pre-write backup and offers `--restore`. Superseded by G-5 / C-13..C-15." Confirm `NG-1` is untouched (sidecar stores bytes Ark itself wrote, not a fetched migration manifest) and say so, or log a change there too.

### R-002 `"existing RawConfig loader" does not exist as claimed`

- **Severity:** HIGH
- **Section:** `## Spec` C-1; `## Architecture` (`strategy.rs (new)`); `## Implementation` Phase 1
- **Problem:** C-1 says `[upgrade]` "is read via the existing `RawConfig` loader." There is no shared cross-feature config loader. `RawConfig` is a **private** struct inside `crates/ark-core/src/commands/agent/task/worktree/config.rs:24`, models only `worktree: Option<WorktreeConfig>`, and maps parse failures to `Error::WorktreeConfigCorrupt`. The `[workspace]` section is parsed by its own separate loader, not this `RawConfig`. So `[upgrade]` cannot be "read via" it without either making that private struct public (cross-module coupling into the worktree feature) or adding an `upgrade` field to a worktree-owned type (wrong ownership).
- **Why it matters:** Self-containment (the `## Spec` must read cleanly) and architecture soundness. As written, an executor would either misroute the dependency or surface a `WorktreeConfigCorrupt` error for a malformed `[upgrade]` section — a confusing, mislabeled failure. The plan already declares a new `UpgradeConfigInvalid` error but never says where corrupt-TOML (as opposed to semantic-invalid) errors land.
- **Recommendation:** Restate C-1 to define `strategy.rs`'s own private raw-config struct (mirroring the worktree pattern: a `#[derive(Deserialize)] struct` with `#[serde(default)] upgrade: Option<...>` read off `layout.config_file()`), and specify the corrupt-TOML error variant for `[upgrade]` (a new `UpgradeConfigCorrupt { path, source }` paralleling `WorktreeConfigCorrupt`, distinct from the semantic `UpgradeConfigInvalid`). Add a unit test that a malformed `[upgrade]` section does not surface as a worktree error.

### R-003 `Backup artifact conflates transient rollback with durable regret-restore`

- **Severity:** HIGH
- **Section:** `## Spec` C-13, C-14, C-15, G-5; `## Runtime` Main Flow 6, Failure Flow 2
- **Problem:** One `.ark/.upgrade-backup/` directory is used for two semantically different artifacts with different lifecycles: (a) the transient pre-write backup restored automatically on apply error (C-13/C-14), and (b) the durable backup the user restores on demand after a *completed* upgrade they regret (C-15/G-5, PRD `Outcome` line 26). The plan never specifies retention: is the backup deleted after a successful upgrade (then `--restore` has nothing to restore) or retained (then a failed-and-auto-restored upgrade leaves a backup that `--restore` would re-apply to an already-restored tree)? PRD explicitly flags "`--restore` of 'the most recent backup' well-defined if an upgrade half-succeeded."
- **Why it matters:** Without a defined lifecycle, `--restore` semantics are ambiguous and possibly destructive (restoring a stale backup over good state). G-5's two clauses ("backs up touched files" and "can restore the last backup") imply one mechanism but require two retention policies.
- **Recommendation:** Specify the backup lifecycle: when a backup dir is created (per non-dry-run upgrade), whether it is retained after success (yes, for the regret case), whether a prior backup is overwritten or timestamped, and what "most recent" means. State that an auto-rollback (C-14) leaves the tree pre-upgrade so a subsequent `--restore` is a no-op-or-refusal, not a second rollback. Add a validation case for "upgrade succeeds, then `--restore` returns the pre-upgrade tree" distinct from the failure-rollback case (V-F-1 and V-F-3 currently both lean on the same dir without distinguishing retention).

### R-004 `Restore-on-error does not account for the mid-sequence manifest flush`

- **Severity:** HIGH
- **Section:** `## Spec` C-14; `## Runtime` Main Flow 6-8, Failure Flow 2
- **Problem:** The existing apply flow writes the manifest durably BEFORE deferred deletions (`upgrade/mod.rs:349-350`), then performs deletions, then writes the manifest a second time if deletions mutated it (`:374-376`). C-14 says "On any apply error, the backup is restored and the manifest is left at its pre-write state," but if a *deletion* fails (Failure Flow step 2 fires after the first manifest write), restoring backed-up file bytes does not restore the already-overwritten on-disk manifest — the manifest now records post-write hashes and dropped entries while the files have been rolled back. The plan's C-13 lists backup targets as "every Write/Merged/MergeConflict/Delete target" but omits `.ark/.installed.json` itself.
- **Why it matters:** A half-rolled-back upgrade that leaves the manifest ahead of the file tree breaks the next upgrade's hash classification (every restored file now looks user-modified against the advanced manifest), defeating the recoverability goal and violating the prior SPEC's manifest-integrity invariant (C-3, C-16 ordering at `.ark/specs/features/ark-upgrade/SPEC.md:197,210`).
- **Recommendation:** Include the manifest file in the backup set (or snapshot the in-memory `Manifest` before apply and re-write it on rollback), and state in C-14 that rollback restores the manifest to its pre-write bytes alongside the files. Add a failure test where a *deletion* (post-manifest-flush) fails and assert the manifest matches its pre-upgrade content, not just the file tree.

### R-005 `Self-healing claim has an unaddressed permanent-fallback hole`

- **Severity:** HIGH
- **Section:** `## Spec` C-8, C-11; `## Runtime` State Transitions; `## Trade-offs` T-4; `## Validation` V-IT-6
- **Problem:** The motivating scenario is a user who already diverged a non-block file (e.g. `.ark/workflow.md`) and then lists it under `merged`. On the first post-feature upgrade there is no base → fallback to the conflict pipeline (C-8). Main Flow step 7 records a new base only on a `Merged`/`MergeConflict`/`Write` of a merged path. If the user picks Skip in that fallback (the natural choice for someone protecting their edits), no write occurs, so no base is ever recorded, so the merge path never activates — permanent fallback, not "self-healing." The plan inherits the research's "self-healing on the next Ark write" framing (T-4) without confronting that a Skip-preserved diverged file has no next Ark write. The plan also lists `init.rs` as recording bases (Architecture, Phase 3), but `init` only seeds fresh files — a user who diverged before adding the path to `merged` never had a base recorded at init either.
- **Why it matters:** This is the central limitation of the feature's headline capability. It is fine as a documented boundary, but the plan presents self-healing as near-universal (State Transitions: "after one Ark write → base recorded → merges next time") and validates only the healing case (V-IT-6), not the permanent-fallback case. An executor will build to the optimistic model and the gap surfaces only in the field.
- **Recommendation:** State the boundary explicitly as a constraint or non-goal: a `merged` path that the user has diverged and never lets Ark overwrite never acquires a base and stays on the fallback path permanently. Decide and document whether `--dry-run`/summary surfaces "no base — fell back" distinctly so the user understands why a `merged` file prompted (research line 132-137 recommends this; the plan's preview spec, R-007, doesn't mention it). Add a validation case asserting that a no-base merged path the user Skips stays on fallback across repeated upgrades.

### R-006 `New PlannedAction variants are unplaced in the C-16 deterministic sort`

- **Severity:** MEDIUM
- **Section:** `## Data Structure` (Merged/MergeConflict/EjectSkip variants); `## Spec` C-17
- **Problem:** The prior SPEC's `C-16` (`.ark/specs/features/ark-upgrade/SPEC.md:210`) fixes the action sort order across an exhaustive bucket numbering (`Write{Add}` < `Write{AutoUpdate}` < `Write{Overwrite}` < `CreateNew` < `RefreshHashOnly` < `Preserve` < `Delete` < `DropManifestEntry`), implemented in `plan.rs:73-82` `sort_key()`. The plan adds `Merged`, `MergeConflict`, and `EjectSkip` but never assigns them bucket positions, and the promoted `## Spec` does not carry a revised C-16. `Merged`/`MergeConflict` are writes; `EjectSkip` is counter-only like `Preserve`.
- **Why it matters:** Determinism is a load-bearing tested invariant (`plan.rs::plan_actions_sorts_output_by_bucket_then_path`). Adding variants without specifying their sort bucket leaves the ordering — and the test that guards it — unspecified, and the promoted SPEC would drop or contradict the existing C-16.
- **Recommendation:** Specify the bucket placement for the three new variants in the Data Structure (e.g. `Merged`/`MergeConflict` adjacent to the `Write` buckets, `EjectSkip` adjacent to `Preserve`) and carry a revised C-16 enumeration into the promoted `## Spec`. Extend the existing sort-order test.

### R-007 `--dry-run preview rendering is under-specified for testability`

- **Severity:** MEDIUM
- **Section:** `## Runtime` Main Flow 5; `## Spec` G-4, C-12; `## Validation` V-IT-2
- **Problem:** PRD `Outcome` (line 25) requires `--dry-run` to print "the full planned action set (per path: add / update / overwrite / merge-clean / merge-conflict / preserve / `.new` / delete / orphan / eject-skip)." The plan says "render the plan as a preview and return" but does not define the preview's shape (per-path lines? a `Display` on a new preview summary? does it reuse `UpgradeSummary`?), nor where the eleven action labels render. V-IT-2 only asserts "reports the action and leaves disk + manifest byte-identical" without pinning the format.
- **Why it matters:** "Reports every planned action" (G-4) is only testable if the preview surface is defined. Per the project convention "commands return summaries that `impl Display`" (CLAUDE.md), an ad-hoc preview print would violate the one-`render`-per-dispatch rule.
- **Recommendation:** Define the preview as a `Display`-able structure (e.g. a `DryRunPreview` summary carrying per-path `(relative, action-label)` rows, or extend `UpgradeSummary` with a preview mode) and specify the label set. Tighten V-IT-2 to assert specific per-path action labels appear, including the no-base-fallback label from R-005.

### R-008 `unload skip of sidecar dirs: mechanism and consequence unstated`

- **Severity:** MEDIUM
- **Section:** `## Architecture` (`unload.rs skips ...`); `## Spec` C-16; `## Validation` V-E-2
- **Problem:** The existing skip set is a fixed-size `[PathBuf; 3]` returned by `capture_skip_paths` (`unload.rs:193-199`) and consumed by BOTH walk sites — the `owned_dirs` capture loop (`:89-90`) and `capture_orphan_hook_entries` (`:175-176`) — per the worktree SPEC C-7 two-site rule. Adding `.ark/.upgrade-base/` and `.ark/.upgrade-backup/` changes the array arity and must update both sites; the plan's one-line "skips the sidecar + backup dirs" names neither. Separately, the plan treats the base store as throwaway local state (like `.state.toml`), but does not state the consequence: an `unload`→`load` round-trip drops all recorded bases, so every `merged` file silently reverts to fallback after a round-trip until Ark next writes it.
- **Why it matters:** Missing one walk site leaks sidecar bytes into `.ark.db` (the worktree feature hit exactly this two-site hazard). The dropped-bases-on-round-trip behavior is a legitimate design choice but interacts with R-005's permanent-fallback hole and should be a stated, tested boundary, not implicit.
- **Recommendation:** Specify that `capture_skip_paths` grows to include both new dirs and both walk sites consume the widened set (cite the worktree C-7 precedent). State the round-trip consequence explicitly (round-trip drops bases → fallback until next Ark write). V-E-2 should assert neither sidecar dir appears in the snapshot from either walk.

### R-009 `remove.rs "wipes the sidecar dirs" is a redundant no-op edit`

- **Severity:** LOW
- **Section:** `## Architecture` (`remove.rs wipes .ark/.upgrade-base/ and .ark/.upgrade-backup/`)
- **Problem:** `remove` already wipes everything under `.ark/` via `layout.ark_dir().remove_dir_all()` (`remove.rs:86`). The two sidecar dirs live under `.ark/`, so they are already removed; no `remove.rs` change is needed.
- **Why it matters:** Minor, but the listed edit implies remove plumbing that does not exist and could lead an executor to add dead code or a redundant test. Contrast with `unload`, which genuinely needs the skip (R-008) because it captures-then-removes selectively.
- **Recommendation:** Drop `remove.rs` from the touched-files list (or note "no change needed — covered by `.ark/` wipe"). Keep the `remove` wipe assertion in V-E-2 as a guard, but do not plan a code edit.

---

## Trade-off Advice

### TR-1 `Gitignored sidecar vs. manifest base64 for base storage`

- **Related Plan Item:** `T-1`
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer A (sidecar) — agree with the plan
- **Advice:** Keep the gitignored sidecar. The research conclusively shows `.ark/.installed.json` is git-tracked and not gitignored, so manifest base64 would commit and churn the template corpus in every host repo. The sidecar's extra plumbing (R-008) is the lesser cost.
- **Rationale:** Bounded blast radius on a user-visible committed file outweighs the unload/remove plumbing; consistent with how `.ark.db`, `.state.toml`, and `.developer` are treated as local-only.
- **Required Action:** Keep with clarification — close R-008's two-walk-site and round-trip-consequence gaps so the "extra plumbing" is fully specified.

### TR-2 `diffy crate vs. hand-rolled diff3`

- **Related Plan Item:** `T-2`
- **Topic:** Correctness vs Dependency Footprint
- **Reviewer Position:** Prefer A (take the crate) — agree
- **Advice:** Adopt `diffy 0.5` `merge_bytes`. diff3 false-conflict minimization is genuinely hard to get right; the crate is MIT/Apache (compatible), byte-API matches Ark's `Vec<u8>` handling, and git-style markers satisfy the PRD out of the box.
- **Rationale:** This is Ark's first diff/merge dependency, but correctness of conflict markers is non-negotiable and the crate is well-used.
- **Required Action:** Justify in the SPEC/CHANGELOG that this is a deliberate first-of-kind dependency. The research caveat (verify the exact `MergeOptions`/`ConflictStyle` builder ergonomics against docs.rs/diffy/0.5.0, and confirm `merge_bytes` not utf8 `merge` so non-UTF-8 round-trips) should be carried into Phase 3 as an implementation check; V-UT-6 already covers the non-UTF-8 assertion.

### TR-3 `One backup dir for both rollback and regret-restore`

- **Related Plan Item:** `T-5`
- **Topic:** Simplicity vs Correctness
- **Reviewer Position:** Need More Justification
- **Advice:** Either keep one dir with an explicitly specified retention/overwrite policy, or split the transient rollback staging from the durable regret-restore backup. One dir is acceptable but only once R-003's lifecycle is pinned.
- **Rationale:** The two use cases have different retention needs; collapsing them without a stated policy is the root of R-003 and the PRD's "half-succeeded" ambiguity.
- **Required Action:** Adopt with clarification — specify retention semantics (see R-003) before EXECUTE.
