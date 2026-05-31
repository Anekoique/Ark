# `simplify-deep-tier` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `simplify-deep-tier`
> Target Task: `simplify-deep-tier`
> Tier: `deep`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Deep tier: `/ark:commit` refuses on any `PENDING`. Standard: warns and proceeds.

---

## Severity Summary: 0 CRITICAL · 1 HIGH · 0 MEDIUM · 1 LOW
## Verification: build PASS · tests PASS (623 passed/0 failed, all binaries exit 0) · lint PASS · format PASS · smoke PASS

## Project Spec Compliance

> Auto-seeded from `.ark/specs/project/INDEX.md` at `task verify` time, walked recursively. Renders two subsections: `Index integrity` and `Leaf SPECs`.

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md` are all on disk and all four are rows in the index; no unlisted children.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: N/A — no SPEC under `specs/project/` was modified by this task; pre-existing conformance is out of scope for this change's audit. The task touches only Rust source, templates, and docs.
  - `LAYOUT.md` — unchanged.
  - `rust/COMMENTS.md` — unchanged. Changed code complies: new doc-comments are third-person present (`extract.rs` `find_final_plan` "Locates the plan…"; `gather.rs` test docstrings "Legacy archived deep tasks…"), C-8/C-23 honored — no task-mark labels like `R-001`/`G-N`/`V-F-2` leak into `crates/` (the one borderline `V-F-2` docstring prefix was removed under V-002).
  - `rust/STYLE.md` — unchanged. Changed code complies: `cargo fmt --check` green (S-25), no out-params, flat artifact names, combinators read cleanly.
  - `rust/ERRORS.md` — unchanged. Changed code complies: `extract.rs` filename fallback uses `unwrap_or_else` with a literal default (no `unwrap()`); `promote.rs` rename removal eliminated a `std::fs::rename` map_err path cleanly; no new bare `#[from]` or panics.

## Related Feature Spec Compliance

> Auto-seeded from PRD's `[**Related Specs**]`. Empty when none.

- [x] specs/features/ark-workflow-refactor/SPEC.md: N/A — INDEX row exists but the SPEC body file is absent on disk (pre-existing dangling row, called out in the PRD). Nothing to grade against; this task supersedes the loop portion of that feature in prose only. No CHANGELOG obligation arises because there is no body to amend.
- [x] specs/features/spec-actuators/SPEC.md: PASS — the SPEC-extraction path it governs still works under the flattened filename. `extract.rs::find_final_plan` resolves `PLAN.md` first with the `NN_` legacy fallback retained; `spec_extract` tests (fresh write, update-with-CHANGELOG, and the new `spec_extract_resolves_legacy_nn_plan`) pass. Actuator-tagged constraints in a `## Spec` still extract verbatim regardless of the source filename.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]`. One bullet per criterion.

- [x] Deep lifecycle is `Design → Plan → Review → Execute → Verify → Committed → Archived`, no `Review → Plan`: PASS — `state.rs::can_transition` deleted the `(Tier::Deep, Phase::Review, Phase::Plan)` arm; `can_transition_deep` now asserts `!can_transition(Deep, Review, Plan)` (state.rs:388).
- [x] Deep seeds plain `PLAN.md` / `REVIEW.md` (no `NN_`): PASS — `phase.rs::artifact_for` drops the iteration parameter and returns `("PLAN","PLAN.md")` / `("REVIEW","REVIEW.md")`; `deep_design_to_plan_to_review` asserts the flat files exist and the `00_` forms do not.
- [x] `TaskToml` drops `iteration`/`max_iterations`; legacy files still load: PASS — fields removed from the struct (state.rs); `task_toml_loads_without_optional_fields` loads a `task.toml` with `iteration = 2` / `max_iterations = 5` and asserts `id`/`tier` survive (serde ignores unknown keys).
- [x] PLAN/REVIEW templates carry no iteration/Log/Response-Matrix/loop vocabulary; REVIEW keeps Verdict + Findings: PASS — `templates/ark/templates/PLAN.md` stripped of `NN`/Iteration/Depends-on/`## Log`/Response Matrix; `REVIEW.md` stripped of Iteration/Target-Plan-NN and keeps Verdict + Findings + Trade-off Advice. The one residual word "iteration" in PLAN.md:13 is a negation ("no iteration history to track here"), not loop instruction — acceptable.
- [x] Workflow doc and `/ark:design` describe review → edit `PLAN.md` in place → `task execute`, no new iteration file: PASS. `templates/ark/workflow.md`, `templates/claude/commands/ark/design.md`, `templates/opencode/commands/ark/design.md`, and `templates/codex/skills/ark-design/SKILL.md` are all rewritten linear (codex fixed under V-001).
- [x] SPEC extraction reads `PLAN.md`; CHANGELOG references the resolved filename, not hardcoded `NN_PLAN.md`: PASS — `spec_extract` cites `plan_path.file_name()` (R-006 fix); flat case records "replaced from PLAN.md", legacy case "replaced from 01_PLAN.md", both asserted by tests.
- [x] All four gates + load/unload smoke pass: PASS — build 0, test 623/0 (all binaries exit 0), clippy `-D warnings` 0, fmt `--check` 0; release smoke load→unload→load→remove all ok.

## Plan Fidelity

> Auto-seeded from the latest PLAN's `## Spec` Goals (`G-N`). PASS when delivered, FAIL when not, N/A when withdrawn.

- [x] G-1: Deep tier runs one PLAN then one REVIEW with no loop back to PLAN: PASS — back-edge removed in `state.rs`; `can_transition_deep` + the negative assertion confirm `Review → Plan` is illegal. V-F-1 (`task plan` from `Review` → `IllegalPhaseTransition`) holds via the state machine.
- [x] G-2: Deep seeds plain `PLAN.md` / `REVIEW.md`, parallel to `VERIFY.md`: PASS — `artifact_for` + `deep_design_to_plan_to_review`; load/unload round-trip preserves them (smoke PASS).
- [x] G-3: REVIEW findings are folded into `PLAN.md` in place before EXECUTE: PASS (docs) — workflow.md §REVIEW and the claude/opencode/codex design entrypoints Step 3.3 describe in-place editing with no response matrix; ark-reviewer body across all three platforms states "There is no review loop: the author edits `PLAN.md` in place". (codex design SKILL fixed under V-001.)
- [x] G-4: `TaskToml` drops `iteration`/`max_iterations`; legacy files still load: PASS — see PRD-constraint above; also `promote.rs` removed the `max_iterations` reconciliation and `deep_to_standard_clears_max_iterations` was deleted; context `gather.rs` no longer reads `toml.iteration`.
- [x] G-5: PLAN/REVIEW templates carry no loop vocabulary: PASS — templates clean; the lone "iteration" mention is a negation. (The cross-platform design-SKILL divergence was a separate doc, fixed under V-001.)

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — no existing feature SPEC body was modified. The related `ark-workflow-refactor/SPEC.md` body does not exist on disk (dangling INDEX row), so there is nothing to amend; the PRD documents this. `spec-actuators/SPEC.md` was read-only context, not edited.

## Findings

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

### V-001 `codex ark-design SKILL still instructs the removed PLAN ⇄ REVIEW loop`

- **Severity:** HIGH
- **Location:** `templates/codex/skills/ark-design/SKILL.md` (lines 2, 11–12, 116–117, 125–131) — untouched by this task (empty git diff).
- **Problem:** The claude and opencode `design.md` were rewritten to the linear single-pass flow, but the codex platform's equivalent skill was not. It still says: front-matter "optional REVIEW loop on deep tier"; "Iterated PLAN ⇄ REVIEW loop"; "Read the latest `NN_PLAN.md` … fill `NN_REVIEW.md`"; "Step 3.3: Loop if revisions needed — copy to `(NN+1)_PLAN.md` … bump `iteration` … fill `## Log` Response Matrix"; and "`task.toml.max_iterations` (typically 3–5)". Every one of these references a mechanism this task removed: `iteration`/`max_iterations` no longer exist on `TaskToml`, `artifact_for` no longer seeds `NN_` files, and the PLAN template no longer has a `## Log` / Response Matrix.
- **Why it matters:** A codex-platform user running a deep task would follow instructions that contradict the shipped binary and templates. They would be told to hand-create `(NN+1)_PLAN.md` files that the CLI never re-seeds and to edit a `task.toml.iteration` field that is now an ignored unknown key, producing artifacts the flattened `find_final_plan`/`discard`/`ark context` paths treat only as legacy. The PRD outcome "Workflow doc and `/ark:design` describe … No new iteration file" is unmet for codex, and cross-platform parity (the project ships claude/codex/opencode in lockstep) is broken for the design entrypoint.
- **Recommendation:** Rewrite `templates/codex/skills/ark-design/SKILL.md` to match the linear flow already applied to `templates/claude/commands/ark/design.md` and `templates/opencode/commands/ark/design.md`: linear description, `PLAN.md` (no `00_`), Step 3.3 "fold findings into PLAN.md in place, no response matrix", and drop the `max_iterations` line. There is no automated parity test for the design entrypoint (the byte-identity test covers only the ark-reviewer/ark-verifier agent bodies), so this gap was not caught mechanically — consider whether a parity guard for the design skill/command trio is worth adding.
- **Resolution:** FIXED — `templates/codex/skills/ark-design/SKILL.md` rewritten linear (front-matter, tier blurbs, Step 2.2 `PLAN.md`, Step 2.3 no `## Log`, Phase 3 single-pass with in-place fold, no `max_iterations`). Same `NN_PLAN.md` → `PLAN.md` cleanup applied to the codex/claude/opencode `commit` and `discard` docs and the `VERIFY.md` template Plan-Fidelity note for cross-platform parity. Rebuilt; all four gates green.

### V-002 `process label "V-F-2" appears in an extract.rs test docstring`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/spec/extract.rs:294` — `/// V-F-2: SPEC extraction over a legacy task dir …`
- **Problem:** COMMENTS.md C-8 and C-23 forbid SPEC-rule / validation labels (e.g. `V-F-2`, `R-001`, `G-N`) inside `crates/` comments — "the constraint stays; its label goes." The new legacy-NN regression test leads its docstring with the PLAN's `V-F-2` validation tag. (The rest of the docstring is fine prose; only the leading label violates.)
- **Why it matters:** Minor convention drift; the label rots once the task is archived and means nothing to a future reader of the source. Not a correctness issue. Note other pre-existing tests in the repo also carry such labels, so this is consistent with surrounding (non-conformant) code rather than a new regression in kind.
- **Recommendation:** Drop the `V-F-2:` prefix; keep the descriptive sentence ("SPEC extraction over a legacy task dir whose only plan is `NN_PLAN.md` still resolves the final plan via the retained fallback."). Apply at the main session's discretion given the pre-existing pattern.
- **Resolution:** FIXED — dropped the `V-F-2:` prefix; the docstring now opens "SPEC extraction over a legacy task dir whose only plan is `NN_PLAN.md` …". No SPEC-rule label remains in the new test.

### V-003 `ark-verifier prompt referenced the removed PLAN ## Log section`

- **Severity:** MEDIUM
- **Location:** `ark-verifier` body (claude `.md`, codex `.toml`, opencode `.md`), sections B + C.
- **Problem:** Found on a follow-up template re-sweep. The verifier instructed flagging deviations "unless the PLAN's `## Log` explicitly supersedes the SPEC" and "N/A if withdrawn (the PLAN's `## Log` must explain) … without a `## Log` Changed/Removed entry are HIGH". The PLAN template no longer has a `## Log` section (it was the iteration-history block this task removed), so the verifier pointed at a section that can never exist.
- **Why it matters:** A verifier following the prompt would look for supersede/withdrawal rationale in a non-existent section and could mis-classify a legitimately-explained deviation as HIGH.
- **Resolution:** FIXED — both lines in all three platform copies now point at `## Trade-offs` (where supersede/deviation rationale lives, consistent with the reviewer's CRITICAL rule). Byte-identity test still passes.

### V-004 `ark-book still documented the PLAN ⇄ REVIEW loop`

- **Severity:** MEDIUM
- **Location:** `docs/book/src/` — `workflow/lifecycle.md`, `workflow/tiers.md`, `workflow/subagents.md`, `reference/ark-agent.md`, `reference/ark-context.md`, `getting-started/first-task.md`, `introduction.md`, `contributing/workspace-layout.md`, `workflow/worktrees.md`, and the `ark-deck.html` slide deck.
- **Problem:** The mdbook (user-facing docs) described the removed loop throughout: lifecycle diagram with a REVIEW→PLAN back-edge, `NN_PLAN.md`/`NN_REVIEW.md` filenames, "Loop until *Approved*", "bump `task.toml.iteration`", `## Log` deltas, `iterate ⇄ (≤ max)` and `looped ⟳` in the deck.
- **Why it matters:** The book is the published reference; leaving it on the old loop contradicts the shipped binary/templates for every reader. (Out of the PRD's literal scope, which named workflow.md + `/ark:design`, but the same intent — and the user requested it.)
- **Resolution:** FIXED — all pages rewritten to the linear single-pass flow (`design → plan → review → execute → verify`, flat `PLAN.md`/`REVIEW.md`, "fold findings into PLAN.md in place"); deck slides relabeled. `mdbook build` succeeds; generated `docs/book/book/` is gitignored. Legacy `NN_` mentions retained only where they describe the backward-compat fallback (`{kind, iteration?}` in the JSON-shape note).

## Notes

- Gates run from the worktree root: `cargo build --workspace` (exit 0), `cargo test --workspace` (exit 0; the captured tail shows `623 passed; 0 failed`, and the full run exited 0 so every test binary was green), `cargo clippy --workspace --all-targets -- -D warnings` (exit 0), `cargo fmt --all -- --check` (exit 0). Release smoke test load→unload→load→remove all succeeded.
- Backward compatibility verified at three layers: (1) `task_toml_loads_without_optional_fields` loads a legacy `iteration`/`max_iterations` toml; (2) `find_final_plan` resolves a legacy `NN_PLAN.md` and the new `spec_extract_resolves_legacy_nn_plan` test exercises it end-to-end including CHANGELOG provenance; (3) `gather_classifies_legacy_nn_artifacts_by_iteration` confirms `ark context` still renders archived `00_/01_PLAN.md` + `01_REVIEW.md` with their filename-derived iteration numbers, while flat `PLAN.md`/`REVIEW.md` render bare (iteration 0).
- The cross-platform ark-reviewer (3 copies) and ark-verifier (3 copies) bodies remain byte-identical modulo platform frontmatter — `agent_bodies_are_byte_identical_modulo_platform_idioms` passes. V-001 concerns the design SKILL, which that test does not cover.
- The `discard`/`commit` user-facing docs and the `VERIFY.md` template seed note were updated this round from `NN_PLAN.md`/`NN_REVIEW.md` to flat `PLAN.md`/`REVIEW.md`. The only `NN_PLAN.md` mentions that remain are in legacy-fallback code paths and their doc-comments (`find_final_plan`, `parse_nn_plan`, the `--plan` CLI help, the `ark-context` JSON `{kind, iteration?}` shape) plus deliberately-commented legacy `task.toml` test fixtures — all describing backward-compat, not the active flow.
- The previously-seeded REVIEW (`00_REVIEW.md`) findings R-001..R-006 were all addressed in this implementation: R-001 (context blast radius), R-002/TR-1 (ArtifactKind iteration kept), R-004 (promote rename removed), R-006 (CHANGELOG cites resolved filename). The implementation is sound; the only outstanding gap is the codex design SKILL that fell outside the executor's edited file set.
