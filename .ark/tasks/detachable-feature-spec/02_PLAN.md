# `detachable-feature-spec` PLAN `02`

> Status: Revised
> Feature: `detachable-feature-spec`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`

---

## Summary

Iteration 01 was Approved with Revisions (0 CRITICAL, 3 HIGH, 3 MEDIUM, 2 LOW + 3 trade-off advice). The design is the same — required PRD `[**SPEC Path**]` block, recursive `features/` tree, leaf-to-root INDEX upsert, single-segment back-compat — but four substantive design decisions were under-specified or wrong:

1. **Walk model**: switch from filesystem-authoritative to INDEX-strict + warnings (R-010, TR-5). INDEXes are the source of truth; on-disk leaves missing from their parent INDEX are dropped with a warning, INDEX rows pointing at missing children are dropped with a warning. Both surface through a new `warnings` field on the gather summary.
2. **`Layout::specs_feature_dir` fallibility**: switch from panic to `Result<PathBuf>` (R-013, TR-6). Matches the existing `resolve_safe` precedent; honors `rust/ERRORS.md` E-1/E-10.
3. **Scope/promoted provenance** (R-012, R-017): for a nested leaf `features/<a>/<b>/<c>/SPEC.md`, `SpecRow.scope` and `SpecRow.promoted` come from the row in `features/<a>/<b>/INDEX.md` whose first cell normalizes to `c` or `c/SPEC.md`. Spelled out as C-9a.
4. **`spec_import` walk** (R-014): brownfield imports share the leaf-to-root walk; a nested `spec_import --feature foo/csr` produces the same three-level INDEX shape as a deep-tier promotion.

Plus mechanical tightenings: `RollbackGuard` shape now in `## Spec` `[**Data Structure**]` (R-011); bullet pattern accepts `-`/`*`/`+` (R-015); G-4 rewritten as capability (R-016); V-UT-12 names its test (TR-7); Phase 4 step 2 wording fixed (R-017).

## Log

[**Added**]

- C-9a: scope/promoted provenance pinned to "row in the parent subtree INDEX whose first cell normalizes to the child name."
- C-12b: INDEX-strict walk model with bidirectional drift warnings.
- C-16a: `spec_import` shares `upsert_index_rows_leaf_to_root` with `spec_register`.
- `RollbackGuard::features_indexes: Vec<FeaturesIndexSnapshot>` field shape shown in `## Spec` `[**Data Structure**]` (R-011 closure).
- `GatherWarning` type and `warnings: Vec<GatherWarning>` on the projection (R-010, TR-5 closure).
- V-UT-13: orphan leaf — INDEX missing the row — is dropped with a warning surfaced through the projection.
- V-UT-14: scope/promoted provenance for a three-level fixture; nested leaf's scope comes from the parent subtree INDEX, not the root.
- V-UT-15: `spec_import` with nested `--feature` produces the same INDEX shape as `spec_register`.
- V-UT-16: GFM bullet markers `*` and `+` both accepted by C-11a.

[**Changed**]

- `Layout::specs_feature_dir(&self, &[&str])` returns `Result<PathBuf>` (was `PathBuf` w/ panic). C-2a rewritten as fallible boundary check.
- C-11a's bullet pattern is `^\s*[-*+]\s*` rather than `^\s*-\s*`; matches all three GFM bullet markers.
- G-4 rewritten: "Existing flat-namespace SPECs and tasks continue to work without migration." (R-016 closure)
- Phase 4 step 2 wording fixed: nested leaves take scope/promoted from the *parent subtree* INDEX, not the root (R-017 closure).
- V-UT-12 names the test: `specs_feature_dir_no_single_str_invocations` lives in `crates/ark-core/src/commands/agent/spec/mod.rs` (TR-7 closure).

[**Removed**]

- The filesystem-authoritative walk model — superseded by INDEX-strict + warnings (R-010, TR-5).
- The panic-on-malformed claim in C-2a — superseded by the Result-returning shape (R-013, TR-6).
- T-6's prior framing — replaced by a new T-6 covering the strict-vs-tolerant tradeoff explicitly.

[**Unresolved**]

- None — every R-010..R-017 finding accepted with a concrete change; TR-5/TR-6/TR-7 all closed.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-010 | Accepted | Adopt INDEX-strict walk (TR-5 lean). C-12b spells out the drift model: rows pointing at missing children dropped with warning; on-disk leaves not rowed dropped with warning. V-UT-13 covers the latter case; V-UT-7 (existing) extends to assert the former. V-UT-8 wording stays — strict walk preserves byte-identical `path` for INDEX-registered SPECs by construction. |
| Review | R-011 | Accepted | `RollbackGuard::features_indexes: Vec<FeaturesIndexSnapshot>` shown in `[**Data Structure**]`; restore loop iterates reverse insertion order. |
| Review | R-012 | Accepted | C-9a pins provenance. V-UT-14 covers a three-level fixture asserting which INDEX populates `SpecRow.scope` for the deepest leaf. |
| Review | R-013 | Accepted | `Layout::specs_feature_dir(&[&str]) -> Result<PathBuf>` (option (a) from R-013). Aligns with `resolve_safe`; no E-1/E-10 carve-out needed. |
| Review | R-014 | Accepted | C-16a: `spec_import` calls `upsert_index_rows_leaf_to_root`. V-UT-15 covers the nested-import INDEX shape. |
| Review | R-015 | Accepted | C-11a bullet pattern broadened to `[-*+]`. V-UT-16 covers `*` and `+` cases. |
| Review | R-016 | Accepted | G-4 rewritten as capability. C-10 already carries the byte-for-byte invariant. |
| Review | R-017 | Accepted | Phase 4 step 2 rewritten — subtree INDEXes populate nested-leaf scope/promoted; cross-references C-9a. |
| Review | TR-5 | Accepted | INDEX-strict adopted. See R-010. |
| Review | TR-6 | Accepted | `Result` return adopted. See R-013. |
| Review | TR-7 | Accepted | V-UT-12 names the test and its file. |

---

## Spec

[**Goals**]

- G-1: Feature SPECs may live at arbitrary depth under `.ark/specs/features/`.
- G-2: Deep-tier `task commit` extracts SPECs into the declared subtree of the recursive `features/` tree.
- G-3: Nested feature paths surface in `ark context` and PRD-related-specs parsing.
- G-4: Existing flat-namespace SPECs and tasks continue to work without migration.

[**Non-goals**]

- NG-1: No auto-migration of existing flat SPECs; single-segment paths remain valid leaves at the root.
- NG-2: No recursive layout for `.ark/tasks/`; the tasks tree stays flat.
- NG-3: No `ark agent spec move` helper; reorganization is hand-edit until a follow-up task adds one.

[**Architecture**]

```
crates/ark-core/src/
├── error.rs                          (+) FeaturePathMissing, InvalidFeaturePath
├── layout.rs                         (*) specs_feature_dir: (&self, &[&str]) -> Result<PathBuf>
│                                          revalidates segments; aligns with resolve_safe (C-2a)
├── commands/agent/
│   ├── spec/
│   │   ├── extract.rs                (*) opts.feature_path: Vec<String>;
│   │   │                                 target = layout.specs_feature_dir(&segments)?
│   │   ├── register.rs               (*) opts.feature_path: Vec<String>;
│   │   │                                 upsert_index_rows_leaf_to_root with seed template
│   │   ├── import.rs                 (*) opts.feature_path: Vec<String>; CLI --feature widened;
│   │   │                                 shares upsert_index_rows_leaf_to_root (C-16a)
│   │   ├── mod.rs                    (*) hosts source-scan test
│   │   │                                 specs_feature_dir_no_single_str_invocations
│   │   │                                 (cf. C-28 commands_no_bare_command_new in ark-context)
│   │   └── (peer module re-exports unchanged shape)
│   └── task/
│       ├── commit.rs                 (*) reads PRD, calls parse_spec_path(),
│       │                                 threads segments into spec_extract + spec_register;
│       │                                 RollbackGuard.features_indexes: Vec<FeaturesIndexSnapshot>
│       └── prd.rs                    (+) NEW leaf: SPEC-path parser
│                                          fn parse_spec_path(prd: &str, slug: &str)
│                                              -> Result<Vec<String>>
└── commands/context/
    ├── related_specs.rs              (*) accepts bullet-led [-*+] nested + bare backticked paths
    ├── gather.rs                     (*) parse_features_index: INDEX-strict walk + drift warnings
    └── model.rs                      (*) SpecRow gains feature_path: Vec<String>
                                            (path: PathBuf preserved);
                                            GatherWarning + warnings: Vec<GatherWarning>

templates/ark/templates/
├── PRD.md                            (*) adds [**SPEC Path**] block placeholder
└── FEATURE_SUBTREE_INDEX.md          (+) subtree INDEX seed, byte-identical
                                            ARK:FEATURES markers (C-8b)
```

Call graph for the closure path:

```
task_commit(opts)
  ├── ... existing precondition checks ...
  ├── prd::parse_spec_path(prd_text, &toml.slug)?      → segments: Vec<String>
  ├── if tier == Deep:
  │     ├── rollback.snapshot_each(intermediate_index_paths(&layout, &segments)?)
  │     │                                              → snapshots populate
  │     │                                                RollbackGuard.features_indexes
  │     ├── spec_extract(SpecExtractOptions { feature_path: segments.clone(), .. })
  │     │     └── target = layout.specs_feature_dir(&segments)?.join("SPEC.md")
  │     └── spec_register(SpecRegisterOptions { feature_path: segments, scope, .. })
  │           └── upsert_index_rows_leaf_to_root(layout, &segments, scope, from_task, date)
  │                 ├── for i in (1..=segments.len()).rev():
  │                 │     parent = features/.join(&segments[..i-1])
  │                 │     child  = segments[i-1]
  │                 │     ensure parent/INDEX.md exists (seed from
  │                 │       FEATURE_SUBTREE_INDEX.md if missing); single-segment paths
  │                 │       use the existing root features/INDEX.md as-is
  │                 │     upsert row pointing at child (leaf = SPEC.md, else INDEX.md)
  │                 └── (root level: features/INDEX.md is the existing managed block)
  └── existing git commit closure (rollback restores features_indexes in reverse on failure)
```

Walk model (`gather::parse_features_index`):

```
walk(layout) -> (Vec<SpecRow>, Vec<GatherWarning>)
  ├── start at features/INDEX.md, read managed block rows
  ├── for each row:
  │     classify by first cell:
  │       `<seg>/SPEC.md` → leaf  ── stat features/<seg>/SPEC.md
  │                                  exists  → emit SpecRow { feature_path: [seg], ... }
  │                                  missing → emit warning MissingChild { row, expected_path }
  │       `<seg>/INDEX.md` → branch ── stat features/<seg>/INDEX.md
  │                                  exists  → recurse into features/<seg>/
  │                                              with feature_path prefix [seg],
  │                                              bounded by max depth 8
  │                                  missing → emit warning MissingChild { ... }
  ├── after row pass: for each subdir under features/ not visited via a row:
  │     if features/<orphan>/SPEC.md exists
  │         → emit warning OrphanLeaf { path, suggestion }
  │     if features/<orphan>/INDEX.md exists
  │         → emit warning OrphanSubtree { path }
  └── return (rows, warnings)
```

[**Data Structure**]

```rust
// ark-core/src/commands/agent/spec/extract.rs
pub struct SpecExtractOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub feature_path: Vec<String>,     // segments relative to features/, last == slug
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
    pub feature_path: Vec<String>,     // segments relative to features/
    pub scope: String,
    pub from_task: String,
    pub date: NaiveDate,
}

pub struct SpecRegisterSummary {
    pub feature_path: Vec<String>,
    pub indexes_touched: Vec<PathBuf>,  // ordered leaf→root for tests + logs
    pub was_update: bool,               // true iff the leaf row was replaced
}

// ark-core/src/commands/agent/spec/import.rs
pub struct SpecImportOptions {
    pub project_root: PathBuf,
    pub feature_path: Vec<String>,     // accepts /-separated form via CLI parse
    // ... existing fields ...
}

// ark-core/src/commands/agent/task/prd.rs (new leaf module)
/// Parses the PRD's `[**SPEC Path**]` block into validated kebab-case segments
/// relative to `features/`. Last segment must equal `slug`.
pub fn parse_spec_path(prd: &str, slug: &str) -> Result<Vec<String>>;

// ark-core/src/layout.rs
impl Layout {
    /// Resolves a feature SPEC directory from segments. Revalidates each
    /// segment against the kebab-case alphabet; returns
    /// Error::InvalidFeaturePath on a malformed segment.
    pub fn specs_feature_dir(&self, segments: &[&str]) -> Result<PathBuf>;
}

// ark-core/src/commands/agent/task/commit.rs
pub struct RollbackGuard {
    // ... existing fields ...
    pub features_indexes: Vec<FeaturesIndexSnapshot>,
}

pub struct FeaturesIndexSnapshot {
    pub path: PathBuf,
    pub pre_bytes: Option<Vec<u8>>,    // None iff INDEX.md did not exist pre-mutation
}

impl RollbackGuard {
    /// Append snapshots in order; restore iterates in reverse insertion order.
    pub fn snapshot_features_indexes(&mut self, paths: &[PathBuf]) -> Result<()>;
}

// ark-core/src/commands/context/model.rs
pub struct SpecRow {
    pub name: String,                  // last path segment (leaf slug)
    pub path: PathBuf,                 // existing: project-root-relative SPEC.md path
    pub feature_path: Vec<String>,     // NEW: features/-relative directory segments
    pub scope: String,                 // populated per C-9a from parent subtree INDEX
    pub promoted: Option<String>,      // existing promoted column
}

#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GatherWarning {
    MissingChild { row: String, expected_path: PathBuf },
    OrphanLeaf { path: PathBuf, suggestion: String },
    OrphanSubtree { path: PathBuf },
}

// ProjectedContext gains:
//   #[serde(skip_serializing_if = "Vec::is_empty")]
//   pub warnings: Vec<GatherWarning>,

// ark-core/src/error.rs (additions)
Error::FeaturePathMissing  { prd_path: PathBuf },
Error::InvalidFeaturePath  { prd_path: PathBuf, value: String, reason: &'static str },
```

`Error` Display per `rust/ERRORS.md`:
- `FeaturePathMissing`: `` PRD at `<prd_path>` has no `[**SPEC Path**]` block ``
- `InvalidFeaturePath`: `` invalid SPEC path `<value>`: <reason> ``

[**API Surface**]

```rust
// ark-core/src/commands/agent/spec/
pub fn spec_extract(opts: SpecExtractOptions) -> Result<SpecExtractSummary>;
pub fn spec_register(opts: SpecRegisterOptions) -> Result<SpecRegisterSummary>;
pub fn spec_import(opts: SpecImportOptions) -> Result<SpecImportSummary>;

// ark-core/src/commands/agent/task/prd.rs (new)
pub fn parse_spec_path(prd: &str, slug: &str) -> Result<Vec<String>>;

// ark-core/src/commands/agent/task/commit.rs
pub fn intermediate_index_paths(layout: &Layout, segments: &[String]) -> Result<Vec<PathBuf>>;

// ark-core/src/commands/context/related_specs.rs
/// Returns paths relative to `features/` (no `/SPEC.md` suffix). Recognizes:
///   - canonical: `specs/features/<...>/<slug>/SPEC.md` anywhere in the section
///   - bare backticked path on a bullet line: `^\s*[-*+]\s*` `<seg>(/<seg>)*` ``
/// Inline backticked tokens in prose are ignored. GFM bullet markers
/// `-`, `*`, `+` all match.
pub fn extract(prd_text: &str) -> Vec<String>;

// ark-core/src/commands/context/gather.rs
/// Walk returns rows and drift warnings together.
pub fn parse_features_index(layout: &Layout) -> Result<(Vec<SpecRow>, Vec<GatherWarning>)>;
```

CLI surface change: `ark agent spec import --feature <p>` accepts `/`-separated values; single-segment values continue to work. The `ark agent` namespace is hidden and non-semver-stable.

Template additions:

```
templates/ark/templates/PRD.md
    + [**SPEC Path**]
    +
    + <path relative to specs/features/, slash-separated, ending in the task slug.
    +  Examples: `xemu/csr`, `klib`, `core/runtime/scheduler`. Required for deep tier.>

templates/ark/templates/FEATURE_SUBTREE_INDEX.md   (new file)
    # `<subtree>` Feature Specs
    <!-- ARK:FEATURES:START -->
    | Feature | Scope | Promoted |
    |---------|-------|----------|
    <!-- ARK:FEATURES:END -->
```

[**Constraints**]

- C-1: `[**SPEC Path**]` body is a single non-empty line; one token (bare path or backticked path); leading/trailing whitespace trimmed. Multi-line or multi-token bodies → `InvalidFeaturePath { reason: "body must be a single token" }`.
- C-2: Path is split on `/`; each segment matches `^[a-z0-9][a-z0-9_-]*$`.
- C-2a: `Layout::specs_feature_dir(&[&str]) -> Result<PathBuf>` revalidates every segment against C-2 and returns `Error::InvalidFeaturePath` on a malformed segment; aligns with `Layout::resolve_safe` precedent (project SPEC `rust/ERRORS.md` E-1/E-10).
- C-3: The last segment of the parsed path MUST equal `task.toml.slug`; mismatch → `InvalidFeaturePath { reason: "last segment must equal task slug" }`.
- C-4: Absent block, empty body, or a segment failing C-2 → `FeaturePathMissing` / `InvalidFeaturePath` with a single quoted reason.
- C-5: `task commit` reads `[**SPEC Path**]` from the latest `PRD.md` on every invocation; path changes across iterations are honored.
- C-6: `parse_spec_path` rejects paths containing `.`, `..`, or empty segments after the split.
- C-7: `spec_extract` writes to `Layout::specs_feature_dir(&segments)?.join("SPEC.md")` and ensures the directory exists; CHANGELOG-on-overwrite semantics preserved.
- C-8: `spec_register` upserts rows from leaf to root: each intermediate `INDEX.md` is created from `FEATURE_SUBTREE_INDEX.md` template when missing.
- C-8a: `task_commit`'s `RollbackGuard.features_indexes: Vec<FeaturesIndexSnapshot>` is populated by `snapshot_features_indexes(&intermediate_index_paths(&layout, &segments)?)` *before* any INDEX mutation; mid-walk failures restore the captured snapshots in reverse insertion order.
- C-8b: `FEATURE_SUBTREE_INDEX.md` carries `<!-- ARK:FEATURES:START -->` / `<!-- ARK:FEATURES:END -->` markers byte-identical to the root `features/INDEX.md`'s managed-block delimiters; the existing `read_managed_block` / `update_managed_block` helpers apply unchanged.
- C-9: Subtree INDEX rows point at the next path component: leaf rows render as `<segment>/SPEC.md`, branch rows as `<segment>/INDEX.md`; columns remain `Feature | Scope | Promoted`.
- C-9a: `SpecRow.scope` and `SpecRow.promoted` for a leaf at `features/<a>/<b>/.../<z>/SPEC.md` are populated from the row in `features/<a>/<b>/.../<y>/INDEX.md` whose first cell normalizes to `z` or `z/SPEC.md`. The root `features/INDEX.md`'s rows describe their own children (single-segment leaves and immediate subtree branches), not any deeper leaf.
- C-10: Single-segment paths produce the pre-existing on-disk layout bit-for-bit (one row in `features/INDEX.md`, one `features/<slug>/SPEC.md`); no intermediate INDEX created.
- C-11: `related_specs::extract` recognizes (a) canonical `specs/features/<...>/<slug>/SPEC.md` substrings anywhere inside the section, and (b) bare backticked path tokens only when they are the bullet-leading element of a list line.
- C-11a: Bullet-leading pattern is `^\s*[-*+]\s*` `<seg>(/<seg>)*` `` where each `<seg>` matches C-2; all three GFM bullet markers (`-`, `*`, `+`) are accepted; inline backticked tokens in prose are not matched.
- C-12: `gather::parse_features_index` walks `features/` INDEX-strict: traversal follows rows declared in each `INDEX.md`'s managed block; subdirectories not rowed are not descended into.
- C-12a: Walk is bounded by max recursion depth 8; symlinks are not followed.
- C-12b: Drift surfaces as warnings, not silent dropping. Row pointing at missing child file → `GatherWarning::MissingChild`; on-disk leaf at `features/<...>/<seg>/SPEC.md` with no parent INDEX row → `GatherWarning::OrphanLeaf`; on-disk subtree with no parent INDEX row → `GatherWarning::OrphanSubtree`. INDEX-registered SPECs preserve their pre-change `path` field byte-identically.
- C-13: `SpecRow.feature_path` is the canonical relative-segments form; `SpecRow.path` retains its existing project-root-relative `PathBuf` value. JSON output adds `feature_path` additively, preserving `path`.
- C-14: No segment may be `.` / `..`, case-insensitive `index`, or case-insensitive `spec`; segments containing `.` are already rejected by C-2.
- C-15: Error messages quote the offending value verbatim (e.g. `` invalid SPEC path `xemu//csr`: empty segment ``).
- C-16: `ark agent spec import --feature <p>` accepts the same `/`-separated form as the deep-tier `[**SPEC Path**]` block; existing single-segment values continue to work.
- C-16a: `spec_import` calls the same `upsert_index_rows_leaf_to_root` as `spec_register`; brownfield imports produce the same INDEX shape as deep-tier promotions at the same path.

[**CHANGELOG**]

- 2026-05-18 `detachable-feature-spec` iteration 02: initial promotion. Establishes recursive `features/` tree, required PRD `[**SPEC Path**]` block, INDEX-strict walk with drift warnings, leaf-to-root INDEX upsert with rollback snapshots, fallible `Layout::specs_feature_dir`.

---

## Runtime

[**Main Flow**] (deep-tier `task commit`)

1. `task_commit` validates phase, staging, etc. (existing logic).
2. Read `PRD.md`, call `prd::parse_spec_path(prd_text, &toml.slug)`.
3. On `Ok(segments)`: compute `intermediate_index_paths(&layout, &segments)?` and call `RollbackGuard::snapshot_features_indexes` on the result (C-8a).
4. Build `SpecExtractOptions { feature_path: segments.clone(), .. }`; call `spec_extract` (which goes through `Layout::specs_feature_dir(&segments)?`).
5. Build `SpecRegisterOptions { feature_path: segments, scope, .. }`; call `spec_register` — walks leaf→root, seeding missing INDEXes, upserting rows.
6. Continue with the existing commit closure (toml save, git stage, commit).

[**Failure Flow**]

1. `[**SPEC Path**]` block missing → `Error::FeaturePathMissing { prd_path }`. No SPEC written, no INDEX touched, no git commit.
2. Invalid kebab-case, `..`, slug mismatch, reserved segment → `Error::InvalidFeaturePath { prd_path, value, reason }`. Same rollback.
3. `Layout::specs_feature_dir` rejects a smuggled segment → same `Error::InvalidFeaturePath`; no SPEC written.
4. `spec_extract` write failure → `RollbackGuard` restores any pre-snapshot of `SPEC.md`. INDEXes untouched (they come later).
5. `spec_register` failure mid-walk → `RollbackGuard.features_indexes` restored in reverse insertion order; pre-existing INDEXes return to original bytes; INDEXes that did not exist pre-mutation are unlinked.

[**State Transitions**]

- PRD with valid `[**SPEC Path**]` → SPEC at `features/<path>/SPEC.md` + N INDEX upserts (one per path segment, leaf→root).
- PRD without block → commit refused; phase stays at `Verify`; user fills the block and retries.

---

## Implementation

[**Phase 1 — Path plumbing**]

1. Add `Error::FeaturePathMissing { prd_path }` and `Error::InvalidFeaturePath { prd_path, value, reason }`. Display strings per `rust/ERRORS.md` E-9.
2. Change `Layout::specs_feature_dir(&self, &str) -> PathBuf` to `(&self, &[&str]) -> Result<PathBuf>`. Implementation revalidates each segment against C-2 and returns `Error::InvalidFeaturePath` on failure (the `prd_path` field is filled by callers using `Layout::specs_features_index()` as a sentinel for "not from a PRD"). Update **four production call sites and one test**:
   - `crates/ark-core/src/commands/agent/spec/extract.rs:96`
   - `crates/ark-core/src/commands/agent/spec/import.rs:78`
   - `crates/ark-core/src/commands/agent/task/commit.rs:175`
   - `crates/ark-core/src/commands/agent/task/archive.rs:488`
   - `crates/ark-core/src/commands/agent/task/commit.rs:830` (test)
   Add `?` propagation at each call site; existing single-segment call sites pass `&[slug.as_str()]`. V-UT-12 enforces post-refactor cleanliness via source-scan.
3. Add `commands/agent/task/prd.rs` with `parse_spec_path` + unit tests covering:
   - happy paths: 1/2/3 segments (`klib`, `xemu/csr`, `core/runtime/scheduler`)
   - block missing, empty body, multi-line body, multi-token body
   - slug mismatch, bad alphabet (uppercase, leading `-`)
   - reserved segments (`.`, `..`, `index`, `INDEX`, `spec`)
   - backtick-quoted form parses identically to bare
   - section-boundary handling (does not consume next `[**...**]` block)

[**Phase 2 — Extract + Register**]

1. `SpecExtractOptions::feature_path: Vec<String>`; thread into `target_dir` via `Layout::specs_feature_dir`. Existing tests pass single-segment `vec![slug]`.
2. `SpecRegisterOptions::feature_path: Vec<String>`; rewrite `upsert_index_row` → `upsert_index_rows_leaf_to_root` taking segments. Shared `sanitize_table_field` unchanged. Add `FEATURE_SUBTREE_INDEX.md` to `templates.rs` via `include_str!`; seed body carries byte-identical `ARK:FEATURES` markers (C-8b).
3. Extend `spec_import`: `SpecImportOptions::feature_path: Vec<String>`; CLI flag parses `/`-separated input; switch from `upsert_index_row` to `upsert_index_rows_leaf_to_root` (C-16a). Existing single-segment behavior preserved when called with `vec![feature]`.

[**Phase 3 — task_commit wiring**]

1. Read PRD inside `task_commit`; call `parse_spec_path`; thread the result into the existing `spec_extract` + `spec_register` calls.
2. Extend `RollbackGuard`: add `features_indexes: Vec<FeaturesIndexSnapshot>` field; add `snapshot_features_indexes(&mut self, paths: &[PathBuf]) -> Result<()>` method; extend the restore loop to iterate `features_indexes.iter().rev()` and restore each (re-write `pre_bytes` if `Some`; unlink the file if `None`).
3. Add `intermediate_index_paths(layout: &Layout, segments: &[String]) -> Result<Vec<PathBuf>>` returning paths from leaf-parent to root-parent. Call before any INDEX mutation.
4. Existing `task_commit` tests update to put a `[**SPEC Path**]` block in their seed PRDs.

[**Phase 4 — Context + parser surface**]

1. `SpecRow.feature_path: Vec<String>` field added; `path: PathBuf` preserved.
2. `gather::parse_features_index` reshaped to INDEX-strict recursive walk: visit `features/INDEX.md`'s managed block, classify each row as leaf or branch by first-cell suffix, stat the referenced file, recurse into branch directories (bounded by depth 8, no symlinks). Emit `GatherWarning::MissingChild` for rows whose target is absent; after the row walk, list `features/` directory entries not visited and emit `OrphanLeaf` / `OrphanSubtree` warnings for unrowed `SPEC.md` / `INDEX.md` finds. At each subtree level, the parent `INDEX.md`'s managed-block populates `SpecRow.scope` and `SpecRow.promoted` for the leaf rows it directly references (C-9a).
3. `related_specs::extract` accepts canonical-path tokens anywhere in the section, plus bullet-leading bare backticked path tokens (C-11 + C-11a) where the bullet marker is one of `-` / `*` / `+`. Returns canonical relative paths.
4. Text-mode render shows the nested path (`xemu/csr`); JSON adds `"feature_path": [...]` to each features row; `warnings` array on the projection (omitted when empty).

[**Phase 5 — Templates + workflow doc**]

1. PRD template gains the `[**SPEC Path**]` block with placeholder example.
2. `templates/ark/templates/FEATURE_SUBTREE_INDEX.md` added.
3. `templates/ark/workflow.md` §6 (Specs) gains one paragraph on the recursive shape, the SPEC-Path requirement, and the INDEX-strict drift warnings.

[**Phase 6 — Self-host demonstration**]

1. This task's own PRD carries a single-segment SPEC Path (`detachable-feature-spec`). Commit produces `features/detachable-feature-spec/SPEC.md`, validating C-10 against the live binary.

---

## Trade-offs

- T-1: **`Vec<String>` vs typed `FeaturePath` newtype.** Vec aligns with the rest of the agent layer; validation is pinned by `Layout::specs_feature_dir` revalidating segments (C-2a). The fallible signature change makes the gate the type system itself: `parse_spec_path` and `Layout::specs_feature_dir` are the two `Result`-returning constructors of validated segments. Reviewer endorsed (TR-1, TR-6).
- T-2: **Reuse `validate_slug` alphabet for segments vs. relax.** Reusing keeps the on-disk shape consistent.
- T-3: **Auto-create subtree INDEXes vs require user to seed.** Auto-create wins on symmetry with `specs/project/`. Seed template carries the managed-block markers byte-identical to the root (C-8b), so the next upsert is idempotent. Reviewer endorsed (TR-2).
- T-4: **Walk order leaf→root vs root→leaf for INDEX upsert.** Atomicity comes from snapshotting every intermediate INDEX in `RollbackGuard.features_indexes` before mutation (C-8a); order is equivalent on rollback. Leaf→root retained so `indexes_touched` in the summary reflects natural reading order. Reviewer's atomicity concern (TR-3) is the snapshot, not the order.
- T-5: **`[**SPEC Path**]` body parsing strictness.** Strict: single token, bare or backticked. Reject multi-line / multi-token bodies. Reviewer endorsed (TR-4).
- T-6: **INDEX-strict walk + drift warnings vs filesystem-authoritative walk.** INDEX-strict wins for SPEC integrity: INDEXes are the source of truth; drift surfaces explicitly through `GatherWarning::OrphanLeaf` / `OrphanSubtree` / `MissingChild` instead of silently leaking orphan SPECs into `ark context`. Disadv: hand-creating a `SPEC.md` without registration shows the file in `git status` but not in `ark context` — agents must register via `ark agent spec import` to surface it. The warnings channel ensures discoverability without changing the JSON contract for INDEX-registered SPECs (reviewer TR-5 lean).

---

## Validation

[**Unit Tests**]

- V-UT-1 (G-2, C-1..C-6): `parse_spec_path` — happy paths (1/2/3 segments), block missing, empty body, multi-line body, multi-token body (two backticked paths), bad alphabet, slug mismatch, `.`/`..`/empty segment, reserved `index`/`spec`/`INDEX` segments, trailing newlines, backtick-quoted form.
- V-UT-2 (C-2, C-2a): kebab-case alphabet test against `^[a-z0-9][a-z0-9_-]*$`; `Layout::specs_feature_dir` returns `Error::InvalidFeaturePath` on a malformed segment (not panic).
- V-UT-3 (G-1, C-7): `spec_extract` writes to nested target; round-trip read equals input; CHANGELOG appended on overwrite at nested path.
- V-UT-4 (G-2, C-8, C-9): `spec_register` leaf→root walk seeds intermediate INDEX, upserts correct row shapes (leaf `<seg>/SPEC.md`, branch `<seg>/INDEX.md`).
- V-UT-5 (G-4, C-10): single-segment path produces byte-identical on-disk shape as the legacy flat layout (golden-file diff against a pre-recorded `features/<slug>/SPEC.md` + `features/INDEX.md`).
- V-UT-6 (G-3, C-11, C-11a): `related_specs::extract` mixed input — canonical path + bullet-leading bare slug + inline prose backtick + nested bullet-leading + invalid segment + duplicate. Returns canonical relative paths, deduped, first-seen order.
- V-UT-7 (G-3, C-12, C-12a, C-13): `gather::parse_features_index` INDEX-strict walk against a three-level fixture; resulting `SpecRow`s carry correct `feature_path` and preserved `path`; row pointing at missing child emits `MissingChild` warning.
- V-UT-8 (C-12b, C-13): `ark context --format json` carries `specs.features[*].feature_path` matching the relative form; `specs.features[*].path` is byte-identical to the pre-change form for SPECs registered in the relevant INDEX (the INDEX-strict walk preserves this by construction).
- V-UT-9 (C-15): `Display` for `Error::FeaturePathMissing` and `Error::InvalidFeaturePath` conforms to `rust/ERRORS.md` E-9 and quotes the offending value.
- V-UT-10 (C-16): `spec_import` accepts a nested `--feature <p>`.
- V-UT-11 (C-11, C-11a): `related_specs::extract` rejects inline backticked tokens like `` `task commit` `` and `` `feature_path` `` while accepting bullet-leading `` `klib` `` / `` `xemu/csr` ``.
- V-UT-12 (R-003 closure): source-scan test `specs_feature_dir_no_single_str_invocations` in `crates/ark-core/src/commands/agent/spec/mod.rs` asserts no `specs_feature_dir(&str)` invocation survives the refactor (parallels the `commands_no_bare_command_new` test from `ark-context` C-28).
- V-UT-13 (C-12b, TR-5): on-disk `features/orphan/SPEC.md` with no row in `features/INDEX.md` is dropped from `SpecRow` output; `GatherWarning::OrphanLeaf` is emitted carrying the path.
- V-UT-14 (C-9a): three-level fixture (`features/foo/INDEX.md` rowing `csr/SPEC.md` with scope `S1`; `features/foo/csr/SPEC.md` present). Assert the resulting `SpecRow.scope == "S1"`; assert the row in the *root* `features/INDEX.md` rowing `foo/INDEX.md` with scope `S0` does NOT bleed into the leaf's scope.
- V-UT-15 (C-16a): `spec_import --feature foo/csr --scope "..."` produces `features/foo/csr/SPEC.md`, `features/foo/INDEX.md` (seeded), and a row in the root `features/INDEX.md` pointing at `foo/INDEX.md` — same shape as the deep-tier promotion path.
- V-UT-16 (C-11a): `related_specs::extract` accepts both `* \`klib\`` and `+ \`xemu/csr\`` bullet markers identically to `-`.

[**Integration Tests**]

- V-IT-1 (G-1, G-2): end-to-end `task new`/`plan`/`execute`/`verify`/`commit` with `[**SPEC Path**]: foo/bar/baz`. Asserts: SPEC at `features/foo/bar/baz/SPEC.md`; INDEXes at three levels; root `features/INDEX.md` gains one row pointing at `foo/INDEX.md`.
- V-IT-2 (G-4): same flow with `[**SPEC Path**]: <slug>` (single segment) → flat layout reproduces exactly.
- V-IT-3 (G-2): two consecutive deep-tier tasks under the same subtree (`foo/a`, `foo/b`) — second task finds the seeded `foo/INDEX.md` and appends without duplicating the `foo/INDEX.md` row in root `features/INDEX.md`.

[**Failure / Robustness**]

- V-F-1 (C-4): `task commit` on deep tier with no `[**SPEC Path**]` errors `FeaturePathMissing`; no SPEC written; no INDEX touched; no git commit; phase unchanged.
- V-F-2 (C-3, C-15): `task commit` with slug mismatch errors `InvalidFeaturePath { reason: "last segment must equal task slug" }`; Display string quotes the offending value.
- V-F-3 (C-6, C-15): `task commit` with `..` segment errors `InvalidFeaturePath { reason: "segment contains `..`" }`; Display string quotes the offending value.
- V-F-4 (C-8a, TR-3): inject a failure in the second INDEX upsert of a three-level walk; assert that the leaf `SPEC.md` and every snapshotted INDEX in `features_indexes` are restored to pre-mutation state in reverse order; no orphan files survive.

[**Edge Cases**]

- V-E-1: PRD with `[**SPEC Path**]` block followed by another bracketed section parses without consuming the next section's content.
- V-E-2: PRD with backtick-quoted path (`` `xemu/csr` ``) parses identically to bare path.
- V-E-3: Empty PRD related-specs section coexists with non-empty SPEC Path block (independent parsers).
- V-E-4: Pre-existing `features/INDEX.md` content above/below the managed block is preserved.
- V-E-5 (C-12b text-mode): `ark context --format text` renders nested-path features rows showing `xemu/csr` rather than the leaf alone.
- V-E-6 (C-12a): a symlink under `features/` is not followed; depth-8+ trees are surfaced up to depth 8 with a warning logged for deeper subtrees.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-3, V-UT-4, V-IT-1 |
| G-2 | V-UT-1, V-UT-2, V-UT-4, V-IT-1, V-IT-3 |
| G-3 | V-UT-6, V-UT-7, V-UT-8, V-UT-11, V-UT-16, V-E-5 |
| G-4 | V-UT-5, V-IT-2 |
| C-1 | V-UT-1 |
| C-2 | V-UT-2 |
| C-2a | V-UT-2 |
| C-3 | V-F-2 |
| C-4 | V-UT-1, V-F-1 |
| C-5 | V-IT-1 |
| C-6 | V-UT-1, V-F-3 |
| C-7 | V-UT-3 |
| C-8 | V-UT-4, V-IT-3 |
| C-8a | V-F-4 |
| C-8b | V-UT-4 |
| C-9 | V-UT-4 |
| C-9a | V-UT-14 |
| C-10 | V-UT-5, V-IT-2 |
| C-11 | V-UT-6, V-UT-11, V-UT-16 |
| C-11a | V-UT-11, V-UT-16 |
| C-12 | V-UT-7 |
| C-12a | V-UT-7, V-E-6 |
| C-12b | V-UT-7, V-UT-13 |
| C-13 | V-UT-7, V-UT-8 |
| C-14 | V-UT-1 |
| C-15 | V-UT-9, V-F-2, V-F-3 |
| C-16 | V-UT-10 |
| C-16a | V-UT-15 |
