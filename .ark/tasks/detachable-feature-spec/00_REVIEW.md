# `detachable-feature-spec` REVIEW `00`

> Status: Open
> Feature: `detachable-feature-spec`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: `0`
- Non-blocking: `9`

## Summary

The PLAN's design direction is sound: a required `[**SPEC Path**]` block, leaf-to-root INDEX upsert mirroring `specs/project/`, and a single-segment back-compat guarantee. No critical SPEC contradictions or design-breaking flaws were found, so this is not a Rejection. However, several HIGH issues require revision before EXECUTE: (1) the back-compat claim collides with the `ark-context` SCHEMA_VERSION contract because `SpecRow::path` is repurposed without a bump; (2) the bare-backticked-slug heuristic in `related_specs::extract` has no disambiguation rule and will false-positive on common prose tokens; (3) the `Layout::specs_feature_dir` signature change is under-counted at 1 call site when 4 exist on disk; (4) the recursive `parse_features_index` walk is unspecified for cycles, missing dirs, and the row-vs-filesystem source-of-truth question. Validation coverage and a few constraint/goal shape issues round out the list.

---

## Findings

### R-001 `SpecRow::path` semantic change breaks `ark-context`'s SCHEMA_VERSION contract

- **Severity:** HIGH
- **Section:** `## Spec` → `[**Data Structure**]` (SpecRow), `[**Constraints**]` (C-13), and the Outcome-5 `path` field promise.
- **Problem:** `crates/ark-core/src/commands/context/model.rs:191-201` currently defines `SpecRow.path: PathBuf` populated by `gather.rs:239` with the project-root-relative full path (`".ark/specs/features/<name>/SPEC.md"`). The PLAN repurposes `path` to `String` carrying a `features/`-relative *directory* path (`"xemu/csr"`). The wire-format type stays string, but the field's meaning silently changes. The `ark-context` SPEC's C-6 reads: "`Context` field order is the JSON schema source of truth; renames/removes bump `SCHEMA_VERSION`, adds are free." A semantic redefinition of an existing field is at least as breaking as a rename — downstream consumers that joined `path` onto `project_root` to open the SPEC now get garbage.
- **Why it matters:** This silently breaks any tool that reads `ark context` JSON and treats `specs.features[*].path` as a filesystem path. The MCP-served projection feeds slash commands and external integrations; an incompatible semantic change without bumping `SCHEMA_VERSION = 1` violates the contract `ark-context` promised.
- **Recommendation:** Either (a) introduce a new field (`feature_path` or `relative`) carrying the new value and keep `path` as the project-root-relative SPEC file path (C-6 says "adds are free"); or (b) explicitly bump `SCHEMA_VERSION` to `2`, add a `## Spec` constraint stating the bump, and add a `## Log` Removed/Changed entry that names the supersede of `ark-context` C-6's invariant. The PLAN should pick one and state it in `[**Constraints**]`.

### R-002 Bare backticked slug heuristic in `related_specs::extract` has no disambiguation rule

- **Severity:** HIGH
- **Section:** `## Spec` → `[**API Surface**]` doc-comment on `extract`, `[**Constraints**]` C-11, `## Trade-offs` (implicit).
- **Problem:** C-11 says `extract` "accepts both `specs/features/<...>/<slug>/SPEC.md` (full path) and bare backticked `<slug>` tokens; returns paths relative to `features/` without the `/SPEC.md` suffix." `[**Related Specs**]` bodies routinely contain prose with backticked code tokens that look like slugs but aren't (`` `task commit` ``, `` `feature_path` ``, `` `xemu/csr` `` qua example, `` `slug` ``). Currently `crates/ark-core/src/commands/context/related_specs.rs:71` only matches the canonical `specs/features/<slug>/SPEC.md` shape, which is unambiguous; a bare-token mode is wide open. Nothing in the PLAN distinguishes "this is a reference" from "this is incidental backticked prose."
- **Why it matters:** False-positive matches will surface spurious "related" SPECs in `ark context` projections, polluting the agent's view of the task's surface area. Worse, downstream filters that key on this list (e.g. `ark-context` C-20: "Projection filter keeps a feature row iff some related-specs path ends with the row's normalized path") will drop or include rows incorrectly.
- **Recommendation:** Define a disambiguator. Options: (a) require the backticked token to be a bullet-leading element (matches existing PRD shape `- \`xemu/csr\` — note`); (b) only accept backticked tokens that resolve to an existing `features/<path>/SPEC.md` on disk (closes the loop with the filesystem); (c) keep the back-compat surface limited to bare slugs (no `/`) and require nested references to use the full `specs/features/<...>/SPEC.md` form. Pick one and write it as a Constraint with one V-UT case proving prose tokens are rejected.

### R-003 `Layout::specs_feature_dir` signature change miscounts call sites

- **Severity:** HIGH
- **Section:** `## Implementation` → Phase 1 step 2.
- **Problem:** The Implementation says: "Change `Layout::specs_feature_dir(&self, &str)` → `(&self, &[&str])`. **Update one call site in `extract.rs`.**" In the worktree there are four production callers plus one test: `commands/agent/spec/extract.rs:96`, `commands/agent/spec/import.rs:78`, `commands/agent/task/commit.rs:175`, `commands/agent/task/archive.rs:488`, and the test in `commit.rs:830`. The implementation plan understates the change surface by 3–4×.
- **Why it matters:** An incomplete implementation phase forces the executor to discover call sites at compile time, blurs the PLAN's contract, and risks one call site (e.g. `archive.rs`) being missed under cargo's incremental compilation if the executor only follows the PLAN. The PLAN should reflect the truth on disk so the reviewer can verify completeness against a concrete list.
- **Recommendation:** Update Phase 1 step 2 to enumerate all four call sites by file:line and note which now thread `feature_path` (extract, import, commit) versus which can keep a single-segment helper (archive's `specs_feature_dir("qd")` test — needs to call the new shape). Add a Validation entry that grep-asserts no remaining `specs_feature_dir(&str)` invocations after the refactor.

### R-004 Recursive INDEX walk lacks cycle/missing-dir/source-of-truth specification

- **Severity:** HIGH
- **Section:** `## Spec` → `[**Constraints**]` C-12, `## Implementation` Phase 4 step 2.
- **Problem:** C-12 says "`gather::parse_features_index` recursively walks every `INDEX.md` under `specs/features/`" and Phase 4 says "follow `<segment>/INDEX.md` rows to subtree INDEXes, accumulate leaf rows." Two ambiguities:
  1. **Source of truth**: rows in parent INDEX vs. directories on disk. If `features/foo/INDEX.md` lists `bar/SPEC.md` but `features/foo/bar/` does not exist (stale row after a hand-edit), what does the walker do? Conversely, if `features/foo/qux/SPEC.md` exists but isn't rowed in `features/foo/INDEX.md`, does it show up in `ark context`?
  2. **Cycles**: nothing in the PLAN forbids symlinks in `features/`; a walker that follows rows is immune but a walker that walks dirs is exposed. The PLAN doesn't say which.
- **Why it matters:** Two well-intentioned implementations of "recursive walk" will diverge. The project-spec `parse_project_index` (current `gather.rs:203`) reads the GFM table — row-driven. The PLAN should pick one model and pin it. Without that, an iteration-1 PLAN could swing either way and the live behavior would be impossible to test against.
- **Recommendation:** Add a Constraint: "Recursive walk follows the canonical structure — directories under `features/` are enumerated by `read_dir`; any `INDEX.md` row that points at a missing child is dropped with a warning; any directory missing from its parent's INDEX is still surfaced (filesystem is authoritative)." Or pick row-driven and state it explicitly. Add an Edge Case V-E for the cycle case (symlink trap) and bound the recursion depth to a sensible constant (e.g. 8).

### R-005 G-2, G-3, G-5 are shape-mismatched (constraints in goal slots)

- **Severity:** MEDIUM
- **Section:** `## Spec` → `[**Goals**]` G-2, G-3, G-5.
- **Problem:** The rubric says Goals are verb-led capabilities (≤80 chars); a "procedure that controls X" is a Constraint. G-2 ("A required PRD `[**SPEC Path**]` block declares each deep-tier SPEC's home …") is a declarative invariant — Constraint shape. G-3 ("`task commit` validates the SPEC Path and writes SPEC + INDEX rows from leaf to root.") is a procedure description — Constraint shape. G-5 ("Single-segment SPEC Paths preserve the pre-existing flat layout bit-for-bit.") is an invariant — Constraint shape. By comparison, G-1 ("Allow feature SPECs to live at arbitrary depth …") and G-4 ("`ark context` and the related-specs parser surface nested paths …") are properly verb-led capability statements.
- **Why it matters:** When this PLAN's `## Spec` is promoted verbatim on commit, the resulting SPEC's Goals section will mix capability statements with procedure rules — exactly the shape the rubric was built to avoid. Authors reading the promoted SPEC will struggle to tell what the feature *does* from what it *enforces*.
- **Recommendation:** Move G-2, G-3, G-5 into `[**Constraints**]` as Constraints (they already exist there in part as C-1..C-10). Replace the freed Goal slots with verb-led capability statements, e.g.: "Deep-tier `task commit` extracts SPECs into a recursive `features/` tree." and "Nested feature paths surface in `ark context` and PRD-related-specs parsing without breaking single-segment back-compat."

### R-006 `## Spec` lacks `[**CHANGELOG**]` section required for promoted SPECs

- **Severity:** MEDIUM
- **Section:** `## Spec` (whole block).
- **Problem:** Every feature SPEC currently under `.ark/specs/features/` carries a `[**CHANGELOG**]` section (e.g. `ark-agent-namespace/SPEC.md:189`, `project-spec/SPEC.md:90`, `ark-context/SPEC.md:293`). The `## Spec` block in this PLAN omits the CHANGELOG section entirely. On commit, `spec_extract` promotes the `## Spec` body verbatim, and the new `features/detachable-feature-spec/SPEC.md` will land without a CHANGELOG slot — the absence will be observed at the first follow-up edit ("where does the entry go?"), and the iteration-N supersede protocol (`## Log` Response Matrix → CHANGELOG entry) has no landing zone.
- **Why it matters:** This iteration is iteration 00. If iteration 01 happens, the supersede entries that would have been written into `[**CHANGELOG**]` have nowhere to land. The promoted SPEC immediately drifts from the corpus's shape on day one.
- **Recommendation:** Add `[**CHANGELOG**]` (initially empty or with a single seed entry: `- <date> `<iteration-0>`: initial promotion.`) before the end of `## Spec`. Iteration-1+ PLANs replace its body with the actual log entries.

### R-007 Validation coverage gaps on Outcome-5 and new error variants

- **Severity:** MEDIUM
- **Section:** `## Validation` (Acceptance Mapping) and `## Spec` `[**Constraints**]`.
- **Problem:**
  1. Outcome 5 (PRD) promises `ark context`'s JSON output gains a `path` field on `specs.features` rows; the PLAN's Validation has no test that asserts the JSON shape (V-UT-7 covers `SpecRow` population but not the projection-mode rendering or text-mode rendering claimed in Phase 4 step 4).
  2. The new error variants `Error::FeaturePathMissing { prd_path }` and `Error::InvalidFeaturePath { prd_path, value, reason }` have no Display test. Per project-SPEC `rust/ERRORS.md` E-9, Display must be lowercase, no trailing punctuation, no `"error: "` prefix — and E-12 requires structured fields, not concatenated strings. No V-UT asserts these.
  3. C-15 ("Error messages quote the offending value") is mapped to V-F-2 / V-F-3 which assert variant matching but not the actual rendered string.
- **Why it matters:** A SPEC contract that goes untested rots silently. The text-mode render's nested-path display is part of the user-visible contract; the JSON-shape addition is part of the schema contract that drives external tools.
- **Recommendation:** Add three Validation entries: V-UT-8 asserts `ark context --scope phase --format json` carries `specs.features[*].path` matching the documented relative form; V-UT-9 asserts the Display strings of both new Error variants conform to E-9 + quote the offending value; V-E-5 asserts text-mode rendering shows the nested path (e.g. `xemu/csr`) rather than just the leaf segment.

### R-008 `spec_import` symmetric `feature_path` extension is undocumented in `## Spec`

- **Severity:** MEDIUM
- **Section:** `## Spec` → `[**Architecture**]` (the `import.rs (*)` line), `[**API Surface**]`, `[**Constraints**]`.
- **Problem:** `## Implementation` Phase 2 step 3 says: "Update `spec_import` (brownfield path) to take `feature_path` symmetrically." But `[**API Surface**]` lists only `spec_extract` / `spec_register` / `parse_spec_path` / `related_specs::extract`. `[**Constraints**]` says nothing about `spec_import`. The `ark-agent-namespace` SPEC's CHANGELOG (2026-05-08 `extract-spec-cmd`) documents the brownfield workflow with `--feature <s>`; widening that to accept nested paths is a CLI-surface change that should be captured in the promoted SPEC of this task and cross-referenced.
- **Why it matters:** Implementation phases that don't reflect into the promoted SPEC create silent CLI drift — six months from now a reader will see the `## Spec` block without ever learning that `ark agent spec import --feature` now accepts `/`-separated values.
- **Recommendation:** Add a Constraint: "`ark agent spec import --feature <path>` accepts the same `/`-separated form as the deep-tier `[**SPEC Path**]` block; existing single-segment values continue to work." Add `spec_import` to `[**API Surface**]` with its new option shape. Optionally add a Validation entry (V-UT-10) covering nested `spec_import`.

### R-009 C-14 "reserved names" wording wobbles between `INDEX.md`/`SPEC.md` and `INDEX`/`SPEC`

- **Severity:** LOW
- **Section:** `## Spec` → `[**Constraints**]` C-14.
- **Problem:** C-14 reads: "no segment may be `INDEX.md` / `SPEC.md` / contain `.`; enforced by C-2 + C-6." Tracing: C-2 forbids `.` in segments via `^[a-z0-9][a-z0-9_-]*$`, so a segment containing `.md` is rejected (the `.` fails the alphabet). C-6 rejects `.` / `..` / empty. The constraint as written is technically correct because the *file-suffixed* forms `INDEX.md` and `SPEC.md` contain `.` and are rejected. But a literal segment `INDEX` (no `.md`) would be allowed — which is arguably also a reserved name to forbid, since it would shadow the auto-created `INDEX.md` at the same level.
- **Why it matters:** Tiny clarity issue. A future author might create `features/foo/INDEX/SPEC.md` and discover the surprise.
- **Recommendation:** Either tighten C-14 to "no segment may be `index` or `spec` (case-insensitive), nor may any segment contain `.`" and add a V-UT case, or note explicitly that reserved-name protection comes from the alphabet rejecting `.md` rather than from a name blocklist.

---

## Trade-off Advice

### TR-1 `Vec<String>` vs `FeaturePath` newtype (T-1)

- **Related Plan Item:** `T-1`
- **Topic:** Type-safety vs API parsimony.
- **Reviewer Position:** Keep with clarification.
- **Advice:** The Vec choice is fine for this iteration, but the PLAN should make explicit that validation flows through *exactly one* path: `parse_spec_path` is the only constructor for a "validated" `Vec<String>`. Today's PLAN routes some validation through `parse_spec_path` and some through `Layout::specs_feature_dir(&[&str])`; the latter takes raw `&str`s, which means a malformed segment can reach the disk path if a caller constructs the Vec manually.
- **Rationale:** Even without a newtype, you can pin the validation gate by saying: "`Layout::specs_feature_dir` panics on a segment failing the alphabet check, since the only legal way to produce one is via `parse_spec_path`." Or: have `specs_feature_dir` revalidate cheaply. Either is fine; pick one.
- **Required Action:** Adopt — add one Constraint stating where validation lives, so the executor knows whether `Layout::specs_feature_dir` is a trusted-input function or a validating one.

### TR-2 Auto-create subtree INDEXes (T-3)

- **Related Plan Item:** `T-3`
- **Topic:** User ergonomics vs filesystem discipline.
- **Reviewer Position:** Prefer A (auto-create, as PLAN proposes).
- **Advice:** Endorsed as-is.
- **Rationale:** Symmetric with `specs/project/` recursive shape; the seed template carries the managed-block markers so the next upsert is idempotent. Forcing pre-creation would defeat the structural goal.
- **Required Action:** Keep with clarification — add one Constraint stating that the seed template carries the `ARK:FEATURES` markers byte-identical to the root INDEX, so the subtree managed-block parser is shared with the root parser.

### TR-3 Walk order leaf→root vs root→leaf (T-4)

- **Related Plan Item:** `T-4`
- **Topic:** Atomic-rollback granularity vs reading-order intuition.
- **Reviewer Position:** Prefer B (root→leaf) — modestly.
- **Advice:** Root→leaf walks better with the existing RollbackGuard in `task_commit`. Today the guard snapshots `features_index_path` (the root INDEX) before mutation. With a leaf→root walk, the root INDEX is written *last*, but intermediate INDEXes are written first — they're not snapshotted. If a mid-walk failure happens, intermediate INDEXes leak. Root→leaf with the guard snapshotting each level as it goes is safer.
- **Rationale:** "Reading order in the summary" is a thin justification. Atomicity guarantees are not.
- **Required Action:** Expand comparison — at minimum, add a Constraint that intermediate INDEXes are also snapshotted in the rollback guard (regardless of walk order), and a Validation entry exercising a mid-walk failure that proves no orphan INDEX files survive.

### TR-4 Strict body parsing for `[**SPEC Path**]` (T-5)

- **Related Plan Item:** `T-5`
- **Topic:** Parser strictness vs user-facing tolerance.
- **Reviewer Position:** Prefer A (strict, as PLAN proposes).
- **Advice:** Endorsed.
- **Rationale:** A SPEC-Path block whose body has multi-line or multi-token content invites ambiguity ("which line is the path?"). Failing fast with a quoted-value error message is the cheapest UX. The PRD template's placeholder is unambiguous.
- **Required Action:** Adopt — clarify in a Constraint that the body must be exactly one token (either bare path or backticked path), and add a V-UT covering "two backticked paths on one line" rejection.
