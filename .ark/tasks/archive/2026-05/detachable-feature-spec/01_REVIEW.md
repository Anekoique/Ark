# `detachable-feature-spec` REVIEW `01`

> Status: Open
> Feature: `detachable-feature-spec`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: `0`
- Non-blocking: `7`

## Summary

The revision lands the design squarely: every R-001..R-009 finding from iteration 00 has a concrete change in `## Spec`, the call-site enumeration in Phase 1 matches the worktree byte-for-byte, and the new filesystem-authoritative walk model is unambiguous about what drives traversal. The `## Spec` block is self-contained — no "see iteration 00" hedges — and the `[**CHANGELOG**]` slot is now present. The remaining issues are pinning, not redesign: (a) the filesystem-authoritative walk changes the *set* of `SpecRow`s emitted by `ark context`, so V-UT-8's "byte-identical to the pre-change form" claim needs scoping to "preserved fields on rows that existed before"; (b) the `RollbackGuard` extension is named in prose but not in the `## Spec` data structure; (c) subtree-INDEX scope/promoted provenance is under-specified — two readers could disagree on which INDEX row populates `SpecRow.scope`; (d) C-2a's panic-on-malformed contract should justify itself against `rust/ERRORS.md` E-1/E-10 (and resolve the tension with the precedent of `Layout::resolve_safe` returning `Result`); (e) `spec_import`'s nested-path behavior is under-specified — does the brownfield path also walk leaf-to-root or stay root-only? None of these is design-breaking; all are HIGH-or-MEDIUM clarifications best resolved before EXECUTE. Recommend the executor proceed to a quick iteration 02 to pin these, then EXECUTE.

---

## Findings

### R-010 `SpecRow` set changes under filesystem-authoritative walk — V-UT-8 "byte-identical" claim overreaches

- **Severity:** HIGH
- **Section:** `## Spec` `[**Constraints**]` C-12 + C-12a; `## Validation` V-UT-8.
- **Problem:** C-12a establishes that traversal is filesystem-authoritative: "missing rows for present directories still surface the leaf" (per the `## Trade-offs` T-6 wording). Today's `gather::parse_features_index` reads the managed block at `features/INDEX.md` — exactly the rows declared there, no more. After this change, a hand-created `features/orphan/SPEC.md` with no INDEX row becomes a new `SpecRow` in every `ark context` output. V-UT-8 asserts `specs.features[*].path` is "byte-identical to the pre-change form" — that holds *per matching row* but not *as a whole array*: extra rows appear. Downstream consumers that iterate the array see a longer list with the same field types.
- **Why it matters:** `ark-context` C-6 promises that field renames/removes bump `SCHEMA_VERSION`; adds are free. Adding `feature_path` is fine. But changing the *cardinality semantics* of the array — "features INDEX rows" → "leaves on disk" — is a contract shift the projection consumers may not expect. The projection filter `filter_features_by_related` (`commands/context/projection.rs:228`) keys on `f.path.ends_with(...)`; orphan leaves whose paths happen to match a related-spec entry would now sneak into Plan/Review-phase output even when they were never INDEX-registered.
- **Recommendation:** Either (a) tighten V-UT-8's wording to "byte-identical for SPECs registered in the root INDEX; new leaves surfaced by the filesystem walk emit `path` matching their on-disk location" and add a V-UT-13 covering the orphan-leaf case explicitly (asserting it appears with empty `scope` and `promoted: None`); or (b) gate orphan-leaf surfacing behind a flag so iteration 0 of the rollout preserves the strict INDEX-row semantics. Pick one and state it in `[**Constraints**]`.

### R-011 `RollbackGuard` data-structure extension named in prose but not in `## Spec`

- **Severity:** HIGH
- **Section:** `## Spec` `[**Data Structure**]` (RollbackGuard not shown), `[**Constraints**]` C-8a, `## Implementation` Phase 3 step 2.
- **Problem:** The current `RollbackGuard` (`commands/agent/task/commit.rs:578-595`) has `features_index: Option<FeaturesIndexSnapshot>` — a single slot. C-8a says "every intermediate INDEX path is snapshotted in `task_commit`'s `RollbackGuard`," and Phase 3 step 2 says "Compute `intermediate_index_paths(&layout, &segments) -> Vec<PathBuf>` ... Register each in `RollbackGuard`." But the existing snapshot API is `snapshot_features_index(&path)` (singular) and the field is a singleton. The PLAN does not show what shape `RollbackGuard` takes after the change (rename to `Vec<FeaturesIndexSnapshot>`? add a new field?). `[**Data Structure**]` should carry the new shape, and `[**Architecture**]` should reflect the field-level change to `commit.rs`.
- **Why it matters:** Self-containment. A reader of the promoted SPEC sees C-8a's rollback invariant but no API for how it is upheld. The executor has to guess: do they reuse `features_index` as `Option<Vec<...>>`, replace it with `features_indexes: Vec<...>`, or thread a new helper through? Each shape lands differently in the SPEC.
- **Recommendation:** Add to `[**Data Structure**]` the post-change `RollbackGuard` field (e.g. `features_indexes: Vec<FeaturesIndexSnapshot>`) and update the `snapshot_features_index` API mention in `[**API Surface**]` (or note it becomes plural). Or add a Constraint stating "the existing `RollbackGuard::features_index: Option<FeaturesIndexSnapshot>` field becomes `features_indexes: Vec<FeaturesIndexSnapshot>`; the restore loop iterates in reverse insertion order to undo writes leaf-first."

### R-012 Subtree INDEX `scope` / `promoted` provenance under-specified

- **Severity:** HIGH
- **Section:** `## Spec` `[**Constraints**]` C-9, C-12, C-13; `## Implementation` Phase 4 step 2.
- **Problem:** With a recursive tree, scope/promoted information for `features/foo/csr/SPEC.md` could plausibly live in (a) the root `features/INDEX.md` row pointing at `foo/INDEX.md` (only carries `foo`'s scope, not csr's); (b) the subtree `features/foo/INDEX.md` row pointing at `csr/SPEC.md` (carries csr's scope); or (c) both (different scopes at different levels). Phase 4 step 2 says "The root `features/INDEX.md`'s managed-block is read but only to populate the `scope` / `promoted` columns for matching leaves" — but for a leaf at `foo/csr/SPEC.md`, there is no row in the *root* INDEX (the root row is `foo/INDEX.md`, a branch row). C-9 says "leaf rows render as `<segment>/SPEC.md`, branch rows as `<segment>/INDEX.md`" — so the leaf row lives in `foo/INDEX.md`, not the root.
- **Why it matters:** `SpecRow.scope` is user-visible (it renders in `ark context --format text` and feeds the JSON projection). Two implementations of the recursive walk will disagree on what value it carries for nested leaves, and validation won't catch the ambiguity because no V-UT asserts a specific source.
- **Recommendation:** Add a Constraint: "`SpecRow.scope` and `SpecRow.promoted` for a leaf at `features/<a>/<b>/<c>/SPEC.md` are populated from the row in `features/<a>/<b>/INDEX.md` whose first cell normalizes to `c` or `c/SPEC.md`. The root `features/INDEX.md`'s scope/promoted columns describe the *subtree* (e.g. `foo`'s scope), not any individual leaf inside it." Add V-UT covering a three-level fixture and asserting which value lands in `SpecRow.scope` for the deepest leaf.

### R-013 `Layout::specs_feature_dir` panics — justify against `rust/ERRORS.md` E-1 / E-10

- **Severity:** MEDIUM
- **Section:** `## Spec` `[**Data Structure**]` (Layout impl) + `[**Constraints**]` C-2a.
- **Problem:** C-2a says `Layout::specs_feature_dir(&[&str])` "panics on a malformed segment (`parse_spec_path` is the only legal constructor)." Project SPEC `rust/ERRORS.md` E-1 reads: "Use `Result<T, E>` for recoverable errors; reserve panics for invariant violations." E-10: "Validate at boundaries; return `Err(...)` rather than panic for recoverable misuse." E-7 forbids `unwrap()` outside tests. The existing path-validating helper on `Layout` is `resolve_safe(&self, relative) -> Result<PathBuf>` (`layout.rs:232`) — it returns `Result`, not panic. Every other `Layout` method (`task_dir`, `specs_feature_dir`, etc.) takes pre-validated input and returns `PathBuf` infallibly. The PLAN introduces a *new* pattern: an infallible-looking method that panics on malformed input. The justification is "parse_spec_path is the only legal constructor" — that's a documentation invariant, not a type-system invariant. Any caller can build a `&[&str]` from raw user input and trigger the panic.
- **Why it matters:** Conformance with project SPECs is mandatory. A panic on a public Layout API conflicts with the project's error-handling discipline. Worse, since `parse_spec_path` already validates segments, the in-`Layout` revalidation is defense-in-depth — fine to keep, but should fail closed via `Result`, not via panic. Otherwise the next caller (a future `spec_move` helper) panics on user input instead of returning a typed error.
- **Recommendation:** Either (a) change C-2a so `Layout::specs_feature_dir(&[&str]) -> Result<PathBuf>` with an `Error::InvalidFeaturePath { value, reason }` variant on bad segments (consistent with `resolve_safe`); or (b) keep the panic but add an explicit Constraint citing E-1/E-10 and justifying the deviation ("the type system cannot enforce 'callers must go through `parse_spec_path`'; the panic documents an invariant the caller violated, analogous to `slice[i]` panicking on out-of-bounds — recoverable misuse goes through `parse_spec_path` which returns `Result`."). Option (a) is consistent with the surrounding code; option (b) is acceptable if you commit to it in writing.

### R-014 `spec_import` nested-path behavior under-specified — does it walk leaf-to-root?

- **Severity:** MEDIUM
- **Section:** `## Spec` `[**API Surface**]` (`spec_import`), `[**Constraints**]` C-16, `## Implementation` Phase 2 step 3.
- **Problem:** C-16 says `ark agent spec import --feature <p>` accepts `/`-separated paths. Phase 2 step 3 says: "Extend `spec_import`: `SpecImportOptions::feature_path: Vec<String>`; CLI flag parses `/`-separated input. Existing single-segment behavior preserved when called with `vec![feature]`." But `spec_import` currently calls `upsert_index_row` (`commands/agent/spec/import.rs:93`), which is the *root-INDEX-only* helper. The PLAN's C-8 says `spec_register` walks leaf-to-root; nothing in C-8 or C-16 says `spec_import` does. Yet a `spec_import --feature foo/csr` that writes to `features/foo/csr/SPEC.md` but only touches `features/INDEX.md` produces a broken tree (no `foo/INDEX.md` listing `csr`).
- **Why it matters:** Brownfield imports become the second way SPECs land in the tree; if they don't share the leaf-to-root walk, the import path silently desynchronizes the structure. A future `ark context` walk that surfaces leaves authoritatively works around it, but the INDEX rows that *agents* read for orientation are out of date.
- **Recommendation:** Add a Constraint: "`spec_import` calls the same leaf-to-root INDEX upsert as `spec_register` (`upsert_index_rows_leaf_to_root`); brownfield imports honor the recursive tree the same way deep-tier promotions do." Add V-UT-13 (or extend V-UT-10) asserting a multi-segment `spec_import` produces the expected three-level INDEX shape, not just the root row.

### R-015 C-11a bullet pattern silent on `*` / `+` bullet markers

- **Severity:** MEDIUM
- **Section:** `## Spec` `[**Constraints**]` C-11a; `## Validation` V-UT-11.
- **Problem:** C-11a reads: "Bullet-leading pattern is `^\s*-\s*` `<seg>(/<seg>)*` ``" — only the `-` bullet marker. GFM accepts `-`, `*`, and `+` interchangeably as bullet markers (https://spec.commonmark.org/0.31.2/#bullet-list-marker), and Ark's PRD templates have historically used `-` but no rule forbids `*` / `+`. The disambiguation rule's whole point is to filter inline prose tokens out; rejecting `*` and `+` while accepting `-` is a hidden gotcha for PRD authors.
- **Why it matters:** A PRD author using `* \`xemu/csr\` — note` instead of `- \`xemu/csr\` — note` would silently lose the reference. V-UT-11 covers prose-token rejection but does not assert behavior for `*` / `+` bullets.
- **Recommendation:** Either tighten C-11a to "`^\s*[-*+]\s*` ``..`` `` (any of the three GFM bullet markers)" and add a V-UT case for each marker; or explicitly state that only `-` is accepted and document why (e.g. "matches Ark's PRD template convention; PRD authors using other bullet markers will get bare-slug references silently ignored — file a follow-up if this becomes a real corner."). Pick one and write it down.

### R-016 G-4 still in Constraint shape

- **Severity:** LOW
- **Section:** `## Spec` `[**Goals**]` G-4.
- **Problem:** R-005 from iteration 00 flagged G-2/G-3/G-5 as shape-mismatched. Iteration 01 rewrote G-2 ("Deep-tier `task commit` extracts SPECs into the declared subtree...") and G-3 ("Nested feature paths surface in `ark context` and PRD-related-specs parsing.") into verb-led capability statements. But G-4 in iteration 01 reads: "Single-segment feature paths preserve the pre-existing flat layout bit-for-bit." That's a declarative invariant — same Constraint shape the R-005 finding called out for the old G-5 ("Single-segment SPEC Paths preserve the pre-existing flat layout bit-for-bit"). The wording is essentially unchanged; the relabel from G-5 to G-4 did not address the shape concern. C-10 already carries the invariant content as a Constraint.
- **Why it matters:** The promoted SPEC's Goals section is the "what does this feature do" header an agent or human scans first. A Goal that is structurally a Constraint pollutes the scan.
- **Recommendation:** Replace G-4 with a verb-led capability — e.g. "Existing flat-namespace SPECs and tasks continue to work without migration." — and let C-10 carry the byte-for-byte invariant. The capability-level statement is "back-compat is preserved"; the procedural rule for what that means lives in C-10.

### R-017 Phase 4 step 2 reading of root INDEX is unreachable for nested leaves

- **Severity:** LOW
- **Section:** `## Implementation` Phase 4 step 2.
- **Problem:** Phase 4 step 2 says: "The root `features/INDEX.md`'s managed-block is read but only to populate the `scope` / `promoted` columns for matching leaves; existence and tree shape come from the filesystem." For a single-segment leaf (`features/klib/SPEC.md`), the root INDEX has the row carrying `klib`'s scope. For a nested leaf (`features/foo/csr/SPEC.md`), the root INDEX has no row for `csr` — the matching row lives in `foo/INDEX.md` (R-012). So "reads the root managed-block ... for matching leaves" describes single-segment behavior; nested leaves need the subtree INDEX to be read too. The wording elides this.
- **Why it matters:** The PLAN's reader infers that subtree INDEX rows are *not* consulted for scope/promoted — which contradicts the practical necessity. Closely related to R-012 but distinct: R-012 asks the Constraint to specify the source; R-017 asks the Implementation step to describe the walk correctly.
- **Recommendation:** Rewrite Phase 4 step 2's middle clause: "At each subtree level, the parent `INDEX.md`'s managed-block populates the `scope` / `promoted` columns of each leaf row whose first cell normalizes to the child name." Or merge with the R-012 Constraint and reference it from Phase 4 step 2.

---

## Trade-off Advice

### TR-5 Filesystem-authoritative walk surfaces orphan leaves — narrow vs broad

- **Related Plan Item:** `T-6`
- **Topic:** Tolerance of drift vs strictness of registration.
- **Reviewer Position:** Need more justification — lean toward strict.
- **Advice:** The "filesystem-authoritative" walk has a real upside (no stale-row pitfalls) and a real downside (orphan leaves leak into `ark context`'s view, breaking the implicit "INDEX is the roster" expectation). T-6's own framing is ambivalent: "agents reading the INDEX directly may see rows whose subtrees don't exist; mitigated by `gather` being the only consumer in the projection." That mitigation is fragile — humans reading `INDEX.md` and agents reading `ark context` JSON now see different rosters. Two answers are defensible: (a) keep T-6 as-is and emit a `warning` field on orphan rows so the projection consumer can detect drift; (b) downgrade to filesystem-checked-but-INDEX-listed: only surface leaves that *both* exist on disk *and* have an INDEX row at the appropriate level — drop rows pointing at missing children with a warning, drop on-disk leaves missing from their parent INDEX with a warning.
- **Rationale:** The recursive tree is brand-new structure. Hand-edits will happen during the bedding-in period (during PR review of subtree moves, etc.). The "filesystem is authoritative" model silently rewards mis-registered SPECs; an INDEX-strict model surfaces the drift via the warning channel and forces the registrar (the `spec_register` path) to be the single source of truth. The strict choice keeps the on-disk INDEXes machine-meaningful, not just human-readable.
- **Required Action:** Expand comparison — at minimum, add a Validation entry asserting the behavior when a leaf exists on disk but no INDEX row points at it (V-E-7), and explicitly state in `[**Constraints**]` whether the JSON projection emits the orphan with empty `scope`/`promoted` or drops it. Whichever you pick, write it down.

### TR-6 `Layout::specs_feature_dir` panic vs `Result` (cf. R-013)

- **Related Plan Item:** `T-1` (new addendum)
- **Topic:** Boundary-validation discipline.
- **Reviewer Position:** Prefer B (return `Result`).
- **Advice:** Matching the precedent set by `resolve_safe` is cheaper than carving a panic exception out of E-1/E-10. The current path-validating helper on `Layout` is fallible-by-design; making `specs_feature_dir(&[&str])` join the family is a one-line `?` for every caller that already goes through `parse_spec_path` (which produces validated segments).
- **Rationale:** "parse_spec_path is the only legal constructor" is a documentation invariant; the type system cannot enforce it. Future callers (`ark agent spec move`, a hypothetical `spec_validate` verb, integration test setups that build segment vecs from fixtures) all bypass the gate. A `Result` return makes the gate the type system instead of comments.
- **Required Action:** Adopt — change C-2a to `Layout::specs_feature_dir(&[&str]) -> Result<PathBuf>` returning `Error::InvalidFeaturePath { value, reason }` on a malformed segment; remove the panic. Or justify rejection by explicitly committing to E-1/E-10 carve-out (see R-013).

### TR-7 V-UT-12 grep-test mechanism

- **Related Plan Item:** `R-003 closure / V-UT-12`
- **Topic:** Source-scan test pattern.
- **Reviewer Position:** Endorsed with one small clarification.
- **Advice:** V-UT-12's "grep-assertion in a `#[test]` ... uses `assert_source_clean` parallel pattern" is the right approach — the existing `commands_no_bare_command_new` test (per `ark-context` C-28) sets the precedent for source-scan tests living under the `commands/` module. The PLAN should name the actual test name (e.g. `specs_feature_dir_no_single_str_invocations`) and state which file under `crates/ark-core/src/` owns it.
- **Rationale:** Locating the test is half the maintenance burden; naming it concretely closes that loop.
- **Required Action:** Keep with clarification — name the test and its source location in V-UT-12.
