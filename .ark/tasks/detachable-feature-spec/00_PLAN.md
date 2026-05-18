# `detachable-feature-spec` PLAN `00`

> Status: Draft
> Feature: `detachable-feature-spec`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none

---

## Summary

Generalize `.ark/specs/features/` from a flat `<slug>/SPEC.md` namespace into a recursive tree mirroring `specs/project/`. Deep-tier PRDs declare a required `[**SPEC Path**]` block whose body is a `/`-separated path relative to `features/`, ending in the task slug. `task commit` parses the latest PRD, validates the path, writes the SPEC at `features/<path>/SPEC.md`, and upserts INDEXes from leaf to root (auto-creating intermediate `INDEX.md` files from a shipped template). The related-specs parser and `ark context` features projection accept the same nested notation; bare slugs continue to resolve at the root for backwards compatibility. Tasks tree stays flat; no auto-migration of existing flat SPECs.

## Log `None in 00_PLAN`

---

## Spec

[**Goals**]

- G-1: Allow feature SPECs to live at arbitrary depth under `specs/features/`.
- G-2: A required PRD `[**SPEC Path**]` block declares each deep-tier SPEC's home relative to `features/`.
- G-3: `task commit` validates the SPEC Path and writes SPEC + INDEX rows from leaf to root.
- G-4: `ark context` and the related-specs parser surface nested paths without breaking bare-slug references.
- G-5: Single-segment SPEC Paths preserve the pre-existing flat layout bit-for-bit.

[**Non-goals**]

- NG-1: No auto-migration of existing flat SPECs; single-segment paths remain valid leaves at the root.
- NG-2: No recursive layout for `.ark/tasks/`; the tasks tree stays flat.
- NG-3: No `ark agent spec move` helper; reorganization is hand-edit until a follow-up task adds one.

[**Architecture**]

```
crates/ark-core/src/
├── error.rs                          (+) FeaturePathMissing, InvalidFeaturePath
├── layout.rs                         (*) specs_feature_dir signature change
│                                          (&self, segments: &[&str]) -> PathBuf
├── commands/agent/
│   ├── spec/
│   │   ├── extract.rs                (*) opts.feature_path: Vec<String>;
│   │   │                                 target_dir = layout.specs_feature_dir(&segments)
│   │   ├── register.rs               (*) opts.feature_path: Vec<String>;
│   │   │                                 upserts INDEXes leaf→root with seeded subtree templates
│   │   ├── import.rs                 (*) accepts segments; same INDEX walk
│   │   └── mod.rs                    (re-exports unchanged shape)
│   └── task/
│       ├── commit.rs                 (*) reads PRD, calls parse_spec_path(),
│       │                                 threads segments into spec_extract + spec_register
│       └── prd.rs                    (+) NEW leaf: SPEC-path parser
│                                          fn parse_spec_path(prd: &str, slug: &str)
│                                              -> Result<Vec<String>>
└── commands/context/
    ├── related_specs.rs              (*) accepts nested + bare paths; returns canonical paths
    ├── gather.rs                     (*) parse_features_index walks subtree INDEXes; SpecRow gains `path`
    └── model.rs                      (*) SpecRow { path: String } populated from the walk

templates/ark/templates/
├── PRD.md                            (*) adds [**SPEC Path**] block placeholder
└── FEATURE_SUBTREE_INDEX.md          (+) seed for auto-created subtree INDEX.md files
```

Call graph for the touched closure path:

```
task_commit(opts)
  ├── ... existing precondition checks ...
  ├── prd::parse_spec_path(prd_text, &toml.slug)?      → segments: Vec<String>
  ├── if tier == Deep:
  │     ├── spec_extract(SpecExtractOptions { feature_path: segments.clone(), .. })
  │     │     └── target = layout.specs_feature_dir(&segments).join("SPEC.md")
  │     └── spec_register(SpecRegisterOptions { feature_path: segments, scope, .. })
  │           └── upsert_index_rows_leaf_to_root(layout, &segments, scope, from_task, date)
  │                 ├── for i in (1..=segments.len()).rev():
  │                 │     let parent = features/.join(&segments[..i-1])
  │                 │     let child  = segments[i-1]
  │                 │     ensure parent.INDEX.md exists (seed from template if missing)
  │                 │     upsert row pointing at child (leaf = SPEC.md, else INDEX.md)
  │                 └── (root level: features/INDEX.md is the existing managed block)
  └── existing git commit closure
```

[**Data Structure**]

```rust
// ark-core/src/commands/agent/spec/extract.rs
pub struct SpecExtractOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub feature_path: Vec<String>,     // segments relative to specs/features/, last == slug
    pub plan_override: Option<PathBuf>,
    pub task_dir_override: Option<PathBuf>,
}

pub struct SpecExtractSummary {
    pub slug: String,
    pub feature_path: Vec<String>,
    pub target_path: PathBuf,
    pub was_update: bool,
}

// ark-core/src/commands/agent/spec/register.rs
pub struct SpecRegisterOptions {
    pub project_root: PathBuf,
    pub feature_path: Vec<String>,     // segments relative to specs/features/
    pub scope: String,
    pub from_task: String,
    pub date: NaiveDate,
}

pub struct SpecRegisterSummary {
    pub feature_path: Vec<String>,
    pub indexes_touched: Vec<PathBuf>,  // ordered leaf→root for tests + logs
    pub was_update: bool,               // true iff the leaf row was replaced
}

// ark-core/src/commands/agent/task/prd.rs (new leaf module)
/// Parses the PRD's `[**SPEC Path**]` block into validated kebab-case segments
/// relative to `specs/features/`. Last segment must equal `slug`.
pub fn parse_spec_path(prd: &str, slug: &str) -> Result<Vec<String>>;

// ark-core/src/layout.rs
impl Layout {
    /// Resolves a feature SPEC directory from validated segments.
    /// `segments = ["xemu", "csr"]` → `<root>/.ark/specs/features/xemu/csr`.
    pub fn specs_feature_dir(&self, segments: &[&str]) -> PathBuf;
}

// ark-core/src/commands/context/model.rs
pub struct SpecRow {
    pub name: String,    // last path segment (slug)
    pub path: String,    // path relative to specs/features/ (e.g. "xemu/csr")
    pub scope: String,
}

// ark-core/src/error.rs (additions)
Error::FeaturePathMissing  { prd_path: PathBuf },
Error::InvalidFeaturePath  { prd_path: PathBuf, value: String, reason: &'static str },
```

[**API Surface**]

```rust
// ark-core/src/commands/agent/spec/
pub fn spec_extract(opts: SpecExtractOptions) -> Result<SpecExtractSummary>;
pub fn spec_register(opts: SpecRegisterOptions) -> Result<SpecRegisterSummary>;

// ark-core/src/commands/agent/task/prd.rs (new)
pub fn parse_spec_path(prd: &str, slug: &str) -> Result<Vec<String>>;

// ark-core/src/commands/context/related_specs.rs
/// Returns paths relative to `specs/features/` (no `/SPEC.md` suffix).
/// Recognizes `specs/features/<...>/<slug>/SPEC.md` (canonical) and bare
/// backticked `<slug>` tokens (back-compat; resolves to root-level).
pub fn extract(prd_text: &str) -> Vec<String>;
```

CLI shape is unchanged. `ark agent task commit`'s behavior changes (deep tier now requires a SPEC Path in the PRD); the binary's argv surface does not.

Template additions:

```
templates/ark/templates/PRD.md
    + [**SPEC Path**]
    +
    + <path relative to specs/features/, slash-separated, ending in the task slug.
    +  Examples: `xemu/csr`, `klib`, `core/runtime/scheduler`. Required for deep tier.>

templates/ark/templates/FEATURE_SUBTREE_INDEX.md   (new file, seeded body)
    # `<subtree>` Feature Specs
    <!-- ARK:FEATURES:START -->
    | Feature | Scope | Promoted |
    |---------|-------|----------|
    <!-- ARK:FEATURES:END -->
```

[**Constraints**]

- C-1: `[**SPEC Path**]` body is a single non-empty line; leading/trailing whitespace trimmed; backtick-quoted form accepted.
- C-2: Path is split on `/`; each segment matches `^[a-z0-9][a-z0-9_-]*$` (same alphabet as `validate_slug`).
- C-3: The last segment of the parsed path MUST equal `task.toml.slug`; mismatch → `InvalidFeaturePath`.
- C-4: Absent block, empty body, or a segment failing C-2 → `FeaturePathMissing` / `InvalidFeaturePath` with a single quoted reason.
- C-5: `task commit` reads `[**SPEC Path**]` from the latest PRD (`PRD.md`) on every invocation; path changes across iterations are honored.
- C-6: `parse_spec_path` rejects paths containing `.`, `..`, or empty segments after the split.
- C-7: `spec_extract` writes to `layout.specs_feature_dir(&segments).join("SPEC.md")` and ensures the directory exists; CHANGELOG-on-overwrite semantics preserved.
- C-8: `spec_register` upserts rows from leaf to root: each intermediate `INDEX.md` is created from `FEATURE_SUBTREE_INDEX.md` template when missing; rows in the root `features/INDEX.md` are unchanged in shape for single-segment paths.
- C-9: Subtree INDEX rows point at the next path component: leaf rows render as `<segment>/SPEC.md`, branch rows as `<segment>/INDEX.md`; columns remain `Feature | Scope | Promoted`.
- C-10: Single-segment paths produce the pre-existing on-disk layout bit-for-bit (one row in `features/INDEX.md`, one `features/<slug>/SPEC.md`).
- C-11: `related_specs::extract` accepts both `specs/features/<...>/<slug>/SPEC.md` (full path) and bare backticked `<slug>` tokens; returns paths relative to `features/` without the `/SPEC.md` suffix.
- C-12: `gather::parse_features_index` recursively walks every `INDEX.md` under `specs/features/`; each leaf row produces one `SpecRow { path }` with `path` set to the segments joined by `/`.
- C-13: `SpecRow::path` is the JSON schema source of truth; `name` (last segment) is preserved for back-compat but redundant.
- C-14: Reserved names: no segment may be `INDEX.md` / `SPEC.md` / contain `.`; enforced by C-2 + C-6.
- C-15: Error messages quote the offending value (e.g. `` invalid SPEC path `xemu//csr`: empty segment ``).

---

## Runtime

[**Main Flow**] (deep-tier `task commit`)

1. `task_commit` validates phase, staging, etc. (existing logic).
2. Read PRD body, call `prd::parse_spec_path(prd_text, &toml.slug)`.
3. On `Ok(segments)`: build `SpecExtractOptions { feature_path: segments.clone(), .. }`; call `spec_extract`.
4. Build `SpecRegisterOptions { feature_path: segments, scope, .. }`; call `spec_register`. Walks leaf→root, seeding missing INDEXes, upserting rows.
5. Continue with the existing commit closure (toml save, git stage, commit).

[**Failure Flow**]

1. `[**SPEC Path**]` block missing → `Error::FeaturePathMissing { prd_path }`. No SPEC written, no git commit.
2. Invalid kebab-case, `..`, slug mismatch → `Error::InvalidFeaturePath { prd_path, value, reason }`. Same rollback.
3. `spec_extract` / `spec_register` failure mid-flight → falls through to existing scoped rollback (the SPEC and INDEX writes happen before `git commit`; failures abort cleanly because nothing is staged yet).

[**State Transitions**]

- PRD with valid `[**SPEC Path**]` → SPEC at `features/<path>/SPEC.md` + N INDEX upserts (one per path segment).
- PRD without block → commit refused; phase stays at `Verify`; user fills the block and retries.

---

## Implementation

[**Phase 1 — Path plumbing**]

1. Add `Error::FeaturePathMissing` and `Error::InvalidFeaturePath`.
2. Change `Layout::specs_feature_dir(&self, &str)` → `(&self, &[&str])`. Update one call site in `extract.rs`.
3. Add `commands/agent/task/prd.rs` with `parse_spec_path` + unit tests (valid 1/2/3-segment paths; slug mismatch; bad alphabet; empty body; missing block; `..` / `.` segments; trailing newlines; backtick-quoted form).

[**Phase 2 — Extract + Register**]

1. `SpecExtractOptions::feature_path: Vec<String>`; thread into `target_dir`. Existing tests pass single-segment `vec![slug]`.
2. `SpecRegisterOptions::feature_path: Vec<String>`; rewrite `upsert_index_row` → `upsert_index_rows_leaf_to_root`. Shared `sanitize_table_field` unchanged. New seed template `FEATURE_SUBTREE_INDEX.md` added to `templates.rs` via `include_str!`.
3. Update `spec_import` (brownfield path) to take `feature_path` symmetrically. Its existing single-segment behavior is preserved when called with `vec![feature]`.

[**Phase 3 — task_commit wiring**]

1. Read PRD inside `task_commit`; call `parse_spec_path`; thread the result into the existing `spec_extract` + `spec_register` calls.
2. Existing `task_commit` tests update to put a `[**SPEC Path**]` block in their seed PRDs.

[**Phase 4 — Context + parser surface**]

1. `SpecRow::path` field; populate from the parser walk.
2. `gather::parse_features_index` becomes a recursive walk: visit `features/INDEX.md`, follow `<segment>/INDEX.md` rows to subtree INDEXes, accumulate leaf rows.
3. `related_specs::extract` accepts nested paths and bare backticked slugs. Returns canonical relative paths (no `/SPEC.md` suffix; `xemu/csr`, `klib`).
4. Text-mode render shows the path; JSON adds `"path": "<...>"` to each features row.

[**Phase 5 — Templates + workflow doc**]

1. PRD template gains the `[**SPEC Path**]` block with placeholder example.
2. `templates/ark/templates/FEATURE_SUBTREE_INDEX.md` added.
3. `templates/ark/workflow.md` §6 (Specs) gains one paragraph on the recursive shape and the SPEC-Path requirement, mirroring §6's project-spec recursive-INDEX wording.

[**Phase 6 — Self-host demonstration**]

1. This task's own PRD already carries a single-segment SPEC Path (`detachable-feature-spec`). Commit produces `features/detachable-feature-spec/SPEC.md`, validating C-10 against the live binary.

---

## Trade-offs

- T-1: **`Vec<String>` vs typed `FeaturePath` newtype.** Plain Vec aligns with the rest of the agent layer (slugs are `String`); a newtype would centralize validation but adds a public type. Plan: keep `Vec<String>`, route validation through `parse_spec_path` + `Layout::specs_feature_dir`. If construction sites multiply later, refactoring to a newtype is cheap.
- T-2: **Reuse `validate_slug` alphabet for segments vs. relax to allow uppercase / underscores.** Reusing keeps the on-disk shape consistent with task slugs and feature-INDEX row identifiers (no case folding required for INDEX matching). Adv: one alphabet to remember. Disadv: a hypothetical `XemuCore` segment is rejected — but Ark already mandates kebab-case across the board.
- T-3: **Auto-create subtree INDEXes vs require user to seed.** Auto-create wins: the recursive shape is the new default and forcing users to pre-create files defeats the symmetry with `specs/project/`. The seed template carries the managed-block markers, so the next `task commit` at the same subtree finds the block and upserts cleanly.
- T-4: **Walk leaf→root vs root→leaf for INDEX upsert.** Ordering doesn't affect correctness (each level is independent); chose leaf→root so `indexes_touched` in the summary reflects the natural reading order ("wrote csr/SPEC.md, then xemu/INDEX.md, then features/INDEX.md").
- T-5: **`[**SPEC Path**]` body parsing strictness.** Accept either a single bare line or a single `` `code-quoted` `` token. Reject multi-line or multi-token bodies to keep the parser surface tiny and the failure mode unambiguous.

---

## Validation

[**Unit Tests**]

- V-UT-1 (G-2, C-1..C-6): `parse_spec_path` — happy paths (1/2/3 segments), block missing, empty body, bad alphabet, slug mismatch, `.`/`..`/empty segment.
- V-UT-2 (C-2): kebab-case alphabet test against `^[a-z0-9][a-z0-9_-]*$`.
- V-UT-3 (G-1, C-7): `spec_extract` writes to nested target; round-trip read equals input; CHANGELOG appended on overwrite at nested path.
- V-UT-4 (G-3, C-8, C-9): `spec_register` leaf→root walk seeds intermediate INDEX, upserts correct row shapes (leaf `<seg>/SPEC.md`, branch `<seg>/INDEX.md`).
- V-UT-5 (G-5, C-10): single-segment path produces byte-identical on-disk shape as the legacy flat layout (golden-file diff against a pre-recorded `features/<slug>/SPEC.md` + `features/INDEX.md`).
- V-UT-6 (G-4, C-11): `related_specs::extract` mixed input — nested path + bare slug + invalid segment + duplicate. Returns canonical relative paths, deduped, first-seen order.
- V-UT-7 (G-4, C-12, C-13): `gather::parse_features_index` recursive walk against a three-level fixture; resulting `SpecRow`s carry correct `path` fields.

[**Integration Tests**]

- V-IT-1 (G-1, G-3): end-to-end `task new`/`plan`/`execute`/`verify`/`commit` with `[**SPEC Path**]: foo/bar/baz`. Asserts: SPEC at `features/foo/bar/baz/SPEC.md`; INDEXes at three levels; root `features/INDEX.md` gains one row pointing at `foo/INDEX.md`.
- V-IT-2 (G-5): same flow with `[**SPEC Path**]: <slug>` (single segment) → flat layout reproduces exactly.
- V-IT-3 (G-3): two consecutive deep-tier tasks under the same subtree (`foo/a`, `foo/b`) — second task finds the seeded `foo/INDEX.md` and appends without duplicating the `foo/INDEX.md` row in root `features/INDEX.md`.

[**Failure / Robustness**]

- V-F-1 (C-4): `task commit` on deep tier with no `[**SPEC Path**]` errors `FeaturePathMissing`; no SPEC written; no git commit; phase unchanged.
- V-F-2 (C-3): `task commit` with slug mismatch errors `InvalidFeaturePath { reason: "last segment must equal task slug" }`.
- V-F-3 (C-6): `task commit` with `..` segment errors `InvalidFeaturePath { reason: "segment contains `..`" }`.

[**Edge Cases**]

- V-E-1: PRD with `[**SPEC Path**]` block followed by another bracketed section parses without consuming the next section's content.
- V-E-2: PRD with backtick-quoted path (`` `xemu/csr` ``) parses identically to bare path.
- V-E-3: Empty PRD related-specs section coexists with non-empty SPEC Path block (independent parsers).
- V-E-4: Pre-existing `features/INDEX.md` content above/below the managed block is preserved.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-3, V-UT-4, V-IT-1 |
| G-2 | V-UT-1, V-UT-2 |
| G-3 | V-UT-4, V-IT-1, V-IT-3 |
| G-4 | V-UT-6, V-UT-7 |
| G-5 | V-UT-5, V-IT-2 |
| C-1 | V-UT-1 |
| C-2 | V-UT-2 |
| C-3 | V-F-2 |
| C-4 | V-UT-1, V-F-1 |
| C-5 | V-IT-1 |
| C-6 | V-UT-1, V-F-3 |
| C-7 | V-UT-3 |
| C-8 | V-UT-4, V-IT-3 |
| C-9 | V-UT-4 |
| C-10 | V-UT-5, V-IT-2 |
| C-11 | V-UT-6 |
| C-12 | V-UT-7 |
| C-13 | V-UT-7 |
| C-14 | V-UT-1 |
| C-15 | V-F-2, V-F-3 |
