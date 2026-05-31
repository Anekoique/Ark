# `simplify-deep-tier` PLAN

> Status: Draft
> Feature: `simplify-deep-tier`
> Owner: Executor

---

## Summary

Remove the deep-tier PLAN ⇄ REVIEW iteration loop. The deep lifecycle becomes linear — `Design → Plan → Review → Execute → Verify → Committed → Archived` — with REVIEW findings folded back into the single `PLAN.md` in place before EXECUTE. This drops the `Review → Plan` back-edge from the state machine, flattens deep-tier artifact names to `PLAN.md` / `REVIEW.md`, removes the now-vestigial `iteration` / `max_iterations` fields from `TaskToml`, strips loop vocabulary (Log / Response Matrix / `NN_` / Iteration) from the PLAN and REVIEW templates, and rewrites the workflow doc + `/ark:design` slash command to describe the linear flow.

## Log `None in 00_PLAN`

*None — first and only PLAN.*

---

## Spec

[**Goals**]

- G-1: Deep tier runs one PLAN then one REVIEW with no loop back to PLAN.
- G-2: Deep tier seeds plain `PLAN.md` / `REVIEW.md`, parallel to `VERIFY.md`.
- G-3: REVIEW findings are folded into `PLAN.md` in place before EXECUTE.
- G-4: `TaskToml` drops `iteration` / `max_iterations`; legacy files still load.
- G-5: PLAN / REVIEW templates carry no loop vocabulary.

[**Non-goals**]

- NG-1: Standard, quick, and research tiers' lifecycles are unchanged.
- NG-2: No reopen-for-second-review mechanism replaces the removed loop.

[**Architecture**]

```
crates/ark-core/src/commands/agent/
├── state.rs            # deep table: Design→Plan→Review→Execute→Verify→Committed→Archived
│                       #   TaskToml drops iteration + max_iterations fields
├── task/
│   ├── phase.rs        # artifact_for: deep PLAN→PLAN.md, REVIEW→REVIEW.md (no NN_)
│   ├── new.rs          # construct TaskToml without iteration/max_iterations
│   ├── new_tests.rs    # drop iteration/max_iterations assertions; delete deep_tier_seeds_max_iterations
│   ├── promote.rs      # remove BOTH the max_iterations block AND the PLAN.md→00_PLAN.md rename
│   └── discard.rs      # recognize plain PLAN.md / REVIEW.md as seeded artifacts
├── spec/extract.rs     # find_final_plan prefers PLAN.md (NN_ fallback kept); CHANGELOG cites resolved filename
└── commands/context/   # TaskSummary loses iteration; ArtifactKind keeps its filename-derived iteration
    ├── model.rs        #   drop TaskSummary.iteration field
    ├── gather.rs       #   stop reading toml.iteration; add flat REVIEW.md classify arm (PLAN.md already had one)
    ├── render.rs       #   drop the `iteration:` line + `iter={}` in active-task line
    ├── projection.rs   #   test fixtures drop iteration
    └── mod.rs          #   test fixtures drop iteration
crates/ark-core/src/state/checkout/{io,reconcile}.rs  # test fixtures drop iteration/max_iterations
templates/ark/templates/
├── PLAN.md             # strip Iteration / Depends-on / Log / Response Matrix
└── REVIEW.md           # strip Iteration / Target-Plan; single Verdict + Findings
templates/claude/commands/ark/design.md   # linear Phase 3 REVIEW (no loop)
templates/ark/workflow.md (applied: .ark/workflow.md) # lifecycle diagram + REVIEW section
```

> Source-first: edit `templates/ark/` and `templates/claude/`, then `cargo build` re-embeds them. The applied `.ark/` copies regenerate.

[**Data Structure**]

```rust
// TaskToml — the persisted per-task record. Carries no iteration counter.
struct TaskToml {
    id: String,
    title: String,
    tier: Tier,
    phase: Phase,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
    committed_at: Option<DateTime<Utc>>,
    branch: Option<String>,
    worktree_path: Option<PathBuf>,
    base_branch: Option<String>,
    start_head: Option<String>,
    journal_path: Option<String>,
}

// TaskSummary (ark context projection) — mirrors the live task; no iteration field.
struct TaskSummary {
    slug: String,
    title: String,
    tier: Tier,
    phase: Phase,
    path: PathBuf,
    updated_at: DateTime<Utc>,
}

// ArtifactKind (ark context) — RETAINED as-is. Its iteration is derived from the
// artifact *filename*, not from TaskToml, so legacy NN_PLAN.md / NN_REVIEW.md
// archives still render correctly. A flattened PLAN.md classifies as iteration 0.
enum ArtifactKind {
    Prd,
    Plan { iteration: u32 },
    Review { iteration: u32 },
    Verify,
    TaskToml,
}
```

[**API Surface**]

```rust
// state.rs — the complete legal deep-tier transitions (no Review→Plan back-edge):
//   Design → Plan, Plan → Review, Review → Execute,
//   Execute → Verify, Verify → Committed, Committed → Archived.
fn can_transition(tier: Tier, from: Phase, to: Phase) -> bool;

// phase.rs — artifact_for has no iteration parameter; deep and standard agree
// on flat filenames:
fn artifact_for(phase: Phase, tier: Tier) -> Option<(&'static str, String)>;
//   Plan   → ("PLAN",   "PLAN.md")     // all tiers that reach Plan
//   Review → ("REVIEW", "REVIEW.md")   // deep only
//   Verify → ("VERIFY", "VERIFY.md")
```

[**Constraints**]

- C-1: @source-scan: Phase::Review, Phase::Plan @ crates/ark-core/src/commands/agent/state.rs
The deep transition table has no `Review → Plan` arm.
- C-2: @test-binding: deep_design_to_plan_to_review
Deep tier seeds `PLAN.md` and `REVIEW.md` with no `NN_` prefix.
- C-3: @test-binding: task_toml_loads_without_optional_fields
A `task.toml` carrying `iteration` / `max_iterations` keys still deserializes.
- C-4: @source-scan: iteration @ crates/ark-core/src/commands/agent/state.rs
`TaskToml` declares neither `iteration` nor `max_iterations`.
- C-5: @judgment
PLAN and REVIEW templates contain no Iteration, Log, Response Matrix, or `NN_` text.

---

## Runtime

[**Main Flow**]

1. `task plan` (deep) → seeds `PLAN.md`; author fills it.
2. `task review` (deep) → seeds `REVIEW.md`; reviewer writes Verdict + Findings.
3. Main session edits `PLAN.md` in place to address CRITICAL/HIGH findings.
4. `task execute` → EXECUTE (no `task review` re-entry possible).

[**Failure Flow**]

1. `task plan` invoked from `Review` → `IllegalPhaseTransition` (back-edge gone).
2. A legacy archived deep task with `NN_PLAN.md` files still resolves via `find_final_plan`'s preserved `NN_` fallback.

[**State Transitions**]

- Deep: `Design → Plan → Review → Execute → Verify → Committed → Archived`.
- `Review → Plan` no longer legal for any tier.

---

## Implementation

[**Phase 1 — state machine + model**]

- `state.rs`: remove the `(Tier::Deep, Phase::Review, Phase::Plan)` arm in `can_transition`; update the doc comment; update `can_transition_deep` to assert `Review → Plan` is now illegal and `Review → Execute` legal.
- `state.rs`: remove `iteration` and `max_iterations` from `TaskToml`; drop them from `sample()` and the optional-field round-trip test; keep `task_toml_loads_without_optional_fields` (serde ignores unknown keys, so legacy files still load).
- `new.rs`: construct `TaskToml` without the two fields.
- `new_tests.rs`: drop the `loaded.iteration == 0` assertion (line ~66); delete `deep_tier_seeds_max_iterations`; drop the research `max_iterations.is_none()` assertion.
- `promote.rs`: remove the `max_iterations` reconciliation block **and** the Standard→Deep `PLAN.md → 00_PLAN.md` rename (deep now keeps `PLAN.md`); update `legal_promotion_preserves_artifacts` to assert `PLAN.md` is preserved.
- `state/checkout/{io,reconcile}.rs`: drop `iteration`/`max_iterations` from their `TaskToml` test fixtures.

[**Phase 2 — context projection (R-001 / R-002 / TR-1 / TR-2)**]

- `context/model.rs`: remove the `TaskSummary.iteration` field. Leave `ArtifactKind::Plan/Review { iteration }` intact — it is filename-derived and serves legacy `NN_` archive display.
- `context/gather.rs`: stop reading `toml.iteration` into `TaskSummary`. Add a flat `REVIEW.md` arm to `classify_artifact` (plain `PLAN.md` already classified as iteration 0; plain `REVIEW.md` previously did not, so a deep `REVIEW.md` would have been dropped from `ark context`). The `parse_iteration_artifact` / sort-key path for legacy `NN_` names is otherwise unchanged.
- `context/render.rs`: drop the `iteration: {}` line in `write_current_task` and the `iter={}` token in the active-task line.
- `context/{projection,mod}.rs`: drop `iteration` from `TaskSummary` test fixtures.

[**Phase 3 — filenames + spec extract**]

- `phase.rs::artifact_for`: drop the `iteration` parameter; deep PLAN → `PLAN.md`, REVIEW → `REVIEW.md`. Update `deep_design_to_plan_to_review` to expect `PLAN.md` / `REVIEW.md` (no `00_`).
- `spec/extract.rs`: `find_final_plan` prefers `PLAN.md`, keeps the `NN_PLAN.md` fallback for legacy archives; CHANGELOG line cites the actually-resolved `plan_path.file_name()` (R-006) so legacy provenance stays accurate; `plan_iteration_nn` retained only if still needed for the CHANGELOG, else removed.
- `discard.rs`: recognize plain `PLAN.md` / `REVIEW.md` as seeded deep artifacts.

[**Phase 4 — templates + docs**]

- `templates/ark/templates/PLAN.md`: remove Iteration / Depends-on / `## Log` / Response Matrix; keep Summary, Spec, Runtime, Implementation, Trade-offs, Validation.
- `templates/ark/templates/REVIEW.md`: remove Iteration / Target-Plan; keep Verdict + Findings + Trade-off Advice.
- `templates/ark/workflow.md`: rewrite lifecycle diagram + REVIEW section as linear; drop the hand-edited-iteration bullet under "Hand-edited operations".
- `templates/claude/commands/ark/design.md`: rewrite Phase 3 to "review → edit `PLAN.md` in place to address CRITICAL/HIGH → `task execute`"; drop the loop step. Update the front-matter description.
- Sweep `CLAUDE.md` for iteration/`NN_PLAN` references in the module map / responsibilities and adjust.

[**Phase 5 — verify**]

- `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
- Rebuild release; run the load/unload/load/remove round-trip smoke test.

---

## Trade-offs

- T-1: **Drop `iteration`/`max_iterations` entirely** vs freeze at 0. Dropping removes dead state and the confusing always-0 field; serde's default tolerance means legacy files still load, so the migration cost is nil. Chosen.
- T-2: **Flatten to `PLAN.md`/`REVIEW.md`** vs keep `00_` prefix. Flattening removes the last loop vocabulary and matches standard tier; cost is touching `extract.rs`/`discard.rs`. The legacy `NN_` fallback in `find_final_plan` is retained so archived tasks still extract. Chosen.
- T-3: **No resolution note in PLAN** vs a Response-Matrix-lite. Per user direction, REVIEW.md holds the findings and the PLAN is edited in place; an audit trail already exists in REVIEW.md + git history. Chosen.
- T-4: **Drop `TaskSummary.iteration`** vs synthesize a constant 0. `TaskToml` no longer sources it; a field hardwired to 0 is a worse contract than absence, and `TaskSummary` only describes live tasks (no legacy concern). Dropped.
- T-5: **Keep `ArtifactKind::Plan/Review { iteration }`** vs collapse to iteration-less variants. The iteration here is derived from the filename, so it still renders legacy `NN_` archived deep tasks correctly via `ark context`; removing it buys no user-visible simplification while risking an archive-display regression. Kept.

---

## Validation

[**Unit Tests**]

- V-UT-1: `can_transition_deep` asserts `Review → Plan` is illegal and the linear arms remain legal.
- V-UT-2: `task_toml_loads_without_optional_fields` confirms a `task.toml` with `iteration`/`max_iterations` keys still loads.
- V-UT-3: `artifact_for` returns `PLAN.md` / `REVIEW.md` for deep tier.
- V-UT-4: a context test asserts a deep task with plain `PLAN.md`/`REVIEW.md` classifies as `ArtifactKind::Plan/Review { iteration: 0 }` AND a legacy `01_PLAN.md` still classifies as `iteration: 1` (TR-1 regression guard).

[**Integration Tests**]

- V-IT-1: `deep_design_to_plan_to_review` asserts `PLAN.md` + `REVIEW.md` (no `00_` files) are seeded.
- V-IT-2: deep-tier `task commit` extracts the `## Spec` from `PLAN.md` and writes the feature SPEC.
- V-IT-3: `legal_promotion_preserves_artifacts` (promote.rs) asserts Standard→Deep keeps `PLAN.md` (not renamed to `00_PLAN.md`).

[**Failure / Robustness**]

- V-F-1: `task plan` from `Review` phase returns `IllegalPhaseTransition`.
- V-F-2: SPEC extraction over a legacy task dir containing only `NN_PLAN.md` still resolves the final plan via the retained fallback.

[**Edge Cases**]

- V-E-1: load/unload round-trip preserves a deep task's `PLAN.md` / `REVIEW.md`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-F-1 |
| G-2 | V-UT-3, V-IT-1, V-E-1 |
| G-3 | C-5 (templates/docs describe in-place edit) |
| G-4 | V-UT-2, V-IT-2, V-UT-4 |
| G-5 | C-5 |
| C-1 | V-UT-1 |
| C-2 | V-IT-1 |
| C-3 | V-UT-2 |
| C-4 | V-UT-2 |
| C-5 | judgment (REVIEW) |
