# `simplify-deep-tier` REVIEW `00`

> Status: Closed
> Feature: `simplify-deep-tier`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: 0
- Non-blocking: 6

## Summary

The design intent is sound and matches the PRD: drop the `Review → Plan` back-edge, flatten deep artifacts to `PLAN.md`/`REVIEW.md`, retire `iteration`/`max_iterations`, and rely on serde tolerance for legacy load. The decisive defect is an **incomplete blast radius**: the entire `commands/context/` module reads `TaskToml.iteration` and carries iteration-bearing types, yet the PLAN's Architecture/Data-Structure sections omit it and assert "everything else unchanged." Removing `TaskToml.iteration` as written would not compile (`gather.rs:146`). This is fully addressable in one more iteration without restructuring the design, so the verdict is Approved with Revisions — but R-001 must be resolved before EXECUTE or the build breaks.

---

## Findings

### R-001 `context/ module omitted from blast radius — guaranteed compile break`

- **Severity:** HIGH
- **Section:** `## Spec` [**Architecture**] / [**Data Structure**] ("everything else unchanged")
- **Problem:** The PLAN enumerates only `state.rs`, `phase.rs`, `new.rs`, `promote.rs`, `discard.rs`, `spec/extract.rs`. It never mentions `commands/context/`, which has a hard dependency on the field being removed:
  - `context/gather.rs:146` — `iteration: toml.iteration` — references the field the PLAN deletes; this fails to compile the instant `TaskToml.iteration` is removed.
  - `context/model.rs:125-126` — `TaskSummary.iteration: u32` (non-optional) is populated from that read.
  - `context/render.rs:166` — emits `iteration: {}` in the text snapshot.
  - `context/projection.rs:521` and `context/mod.rs:407` — test fixtures construct `TaskSummary { iteration: 0, .. }`.
- **Why it matters:** The PRD's own acceptance bullet requires `cargo build`/`test`/`clippy`/`fmt` to pass. As written the change does not build, so V-* and the smoke test can never be reached. The Data-Structure note "everything else unchanged" is factually wrong and would mislead the executor.
- **Recommendation:** Add `context/{model,gather,render,projection,mod}.rs` to the Architecture tree with an explicit decision for `TaskSummary.iteration` (drop the field and its render line, or keep it as a derived/legacy display). Update the fixtures in `mod.rs`/`projection.rs`. State whether `ark context` still prints an `iteration:` line.

### R-002 `ArtifactKind::Plan/Review { iteration } left unaddressed`

- **Severity:** MEDIUM
- **Section:** `## Spec` [**Architecture**] / `## Implementation` Phase 2
- **Problem:** `context/model.rs:163-189` defines `ArtifactKind::Plan { iteration }` / `Review { iteration }`, and `gather.rs:567-607` classifies/sorts both plain `PLAN.md` (→ `iteration: 0`) and `NN_PLAN.md`/`NN_REVIEW.md`. The PLAN says nothing about whether the iteration field on these variants stays (needed for legacy `NN_` archives shown by `ark context`) or is dropped.
- **Why it matters:** This is part of the same feature surface the task is meant to simplify; leaving it undecided risks a half-migrated model where `PLAN.md` reports `iteration: 0` while the field is meaningless. It also intersects backward compat: legacy archives still surface `NN_` artifacts through `classify_artifact`.
- **Recommendation:** Decide explicitly: keep `ArtifactKind::Plan/Review { iteration }` and the `parse_iteration_artifact` path (preserves legacy-archive rendering — recommended, lowest risk), or collapse to iteration-less variants and document the legacy display loss. Add the chosen position to the PLAN and one assertion to V-IT or a context test.

### R-003 `new_tests.rs assertions on iteration/max_iterations not in scope list`

- **Severity:** MEDIUM
- **Section:** `## Implementation` Phase 1 / `## Validation`
- **Problem:** `task/new_tests.rs` asserts `loaded.iteration == 0` (line 66), `deep_tier_seeds_max_iterations` (125-136), and research `max_iterations.is_none()` (564-565). Phase 1 mentions only `new.rs`, `state.rs` `sample()`, and round-trip tests; it does not name `new_tests.rs` or the `deep_tier_seeds_max_iterations` test that must be deleted/rewritten.
- **Why it matters:** Those tests reference removed struct fields and will fail to compile; an executor following the PLAN literally would miss them and hit a build break after believing Phase 1 complete.
- **Recommendation:** Add `task/new_tests.rs` to Phase 1 with the specific edits (drop the `iteration`/`max_iterations` assertions, delete or repurpose `deep_tier_seeds_max_iterations`). Likewise sweep `agent_cli.rs:654`, `archive_index.rs`, `context/checkout.rs`, `context/gather.rs` test fixtures whose inline `task.toml` literals carry `iteration = 0` (harmless to serde but worth a deliberate decision on consistency).

### R-004 `promote.rs Standard→Deep PLAN.md → 00_PLAN.md rename contradicts the flatten`

- **Severity:** MEDIUM
- **Section:** `## Implementation` Phase 1 (promote.rs) / [**API Surface**]
- **Problem:** Phase 1 says only "drop the `max_iterations` reconciliation block" in `promote.rs`. But `promote.rs:84-94` also renames the lone `PLAN.md` to `00_PLAN.md` on Standard→Deep promotion (and `legal_promotion_preserves_artifacts` asserts `00_PLAN.md` exists, `PLAN.md` gone). Once deep tier uses plain `PLAN.md`, that rename is wrong — it would move the body to a now-orphaned `NN_` name that the new `artifact_for` never re-seeds and `find_final_plan` only finds via fallback.
- **Why it matters:** A promoted task would have its plan body stranded under `00_PLAN.md` while `task plan`/seeding expects `PLAN.md`; the promote test would also fail against the new naming.
- **Recommendation:** In Phase 1, specify that the Standard→Deep rename block is removed entirely (deep now keeps `PLAN.md` as-is) and update `legal_promotion_preserves_artifacts` to assert `PLAN.md` is preserved unchanged.

### R-005 `## Spec self-containment: API/Data sections lean on "unchanged" rather than restating`

- **Severity:** LOW
- **Section:** `## Spec` [**Data Structure**] / [**API Surface**]
- **Problem:** The Spec is the body promoted verbatim to `specs/features/simplify-deep-tier/SPEC.md`. The Data-Structure comment "everything else unchanged" and the API comment "loses one arm" describe a diff rather than the resulting contract. A future reader of the SPEC has no prior to diff against.
- **Why it matters:** Deep-tier SPECs must read standalone; diff-style phrasing degrades the durable record (and the "everything else unchanged" claim is also wrong per R-001).
- **Recommendation:** Restate the resulting `can_transition` deep arms and the final `artifact_for` signature/behavior as positive statements, not deltas.

### R-006 `CHANGELOG line for legacy NN_ updates becomes slightly inaccurate`

- **Severity:** LOW
- **Section:** `## Implementation` Phase 2 (extract.rs)
- **Problem:** Dropping `plan_iteration_nn` and hardcoding `PLAN.md` in the CHANGELOG string means a re-extraction over a legacy archive whose final plan is `01_PLAN.md` will record "replaced from PLAN.md". The body is still correct; only the provenance note is off.
- **Why it matters:** Cosmetic provenance drift on a rare legacy path; not a correctness or data-loss issue.
- **Recommendation:** Acceptable to keep simple, but note it explicitly in the PLAN (or cite the actual resolved filename via `plan_path.file_name()` in the CHANGELOG to stay accurate at near-zero cost).

---

## Trade-off Advice

### TR-1 `ArtifactKind iteration field: keep vs drop`

- **Related Plan Item:** R-002 / `context/model.rs`
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer A (keep the iteration field on `ArtifactKind::Plan/Review`)
- **Advice:** Retain the iteration-bearing variants and the `NN_` classify/sort path so `ark context` continues to render legacy archived deep tasks correctly; only change live-seeding to emit plain `PLAN.md`.
- **Rationale:** Backward compatibility is an explicit PRD constraint (legacy `NN_PLAN.md` archives must still work). The iteration field is cheap to keep and removing it buys no user-visible simplification while risking a rendering regression on archives.
- **Required Action:** Adopt and document in the next PLAN's Architecture + add one context-level assertion.

### TR-2 `TaskSummary.iteration display line`

- **Related Plan Item:** R-001 / `context/render.rs:166`
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer B (drop the `iteration:` render line and the `TaskSummary.iteration` field)
- **Advice:** Since `TaskToml` no longer carries `iteration`, there is no live source for `TaskSummary.iteration`; remove the field and the `render.rs` line rather than synthesizing a constant 0.
- **Rationale:** A field hardwired to 0 is misleading and a worse contract than absence. Active tasks are the only thing `TasksState`/`CurrentTask` describes, so no legacy concern applies here (unlike TR-1, which is about archived-artifact *display*).
- **Required Action:** Adopt; reconcile with TR-1 (the `ArtifactKind` legacy display is separate from the live `TaskSummary` field).
