
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

- C-1: @test-binding: multi_token_body_errors
`[**SPEC Path**]` body is a single non-empty line; one token (bare path or backticked path); leading/trailing whitespace trimmed. Multi-line or multi-token bodies → `InvalidFeaturePath { reason: "body must be a single token" }`.
- C-2: @test-binding: uppercase_segment_rejected
Path is split on `/`; each segment matches `^[a-z0-9][a-z0-9_-]*$`.
- C-2a: @test-binding: uppercase_segment_rejected
`Layout::specs_feature_dir(&[&str]) -> Result<PathBuf>` revalidates every segment against C-2 and returns `Error::InvalidFeaturePath` on a malformed segment; aligns with `Layout::resolve_safe` precedent (project SPEC `rust/ERRORS.md` E-1/E-10).
- C-3: @test-binding: rejects_slug_mismatch_when_no_existing_spec
The last segment of the parsed path MUST equal `task.toml.slug` (new-feature case) **or** the path must point at an existing on-disk feature SPEC at `Layout::specs_feature_dir(&segments)?.join("SPEC.md")` (in-place-update case, where the task slug describes the change rather than the feature). Mismatch with no existing SPEC → `InvalidFeaturePath { reason: "last segment must equal task slug or name an existing feature SPEC" }`.
- C-4: @test-binding: empty_or_placeholder_body_errors
Absent block, empty body, or a segment failing C-2 → `FeaturePathMissing` / `InvalidFeaturePath` with a single quoted reason.
- C-5: @judgment
`task commit` reads `[**SPEC Path**]` from the latest `PRD.md` on every invocation; path changes across iterations are honored.
- C-6: @test-binding: dot_and_dotdot_segments_rejected
`parse_spec_path` rejects paths containing `.`, `..`, or empty segments after the split.
- C-7: @test-binding: nested_register_writes_branch_discriminator_row
`spec_extract` writes to `Layout::specs_feature_dir(&segments)?.join("SPEC.md")` and ensures the directory exists; CHANGELOG-on-overwrite semantics preserved.
- C-8: @test-binding: nested_register_writes_branch_discriminator_row
`spec_register` upserts rows from leaf to root: each intermediate `INDEX.md` is created from `FEATURE_SUBTREE_INDEX.md` template when missing.
- C-8a: @test-binding: intermediate_index_paths_leaf_to_root
`task_commit`'s `RollbackGuard.features_indexes: Vec<FeaturesIndexSnapshot>` is populated by `snapshot_features_indexes(&intermediate_index_paths(&layout, &segments)?)` *before* any INDEX mutation; mid-walk failures restore the captured snapshots in reverse insertion order.
- C-8b: @judgment
`FEATURE_SUBTREE_INDEX.md` carries `<!-- ARK:FEATURES:START -->` / `<!-- ARK:FEATURES:END -->` markers byte-identical to the root `features/INDEX.md`'s managed-block delimiters; the existing `read_managed_block` / `update_managed_block` helpers apply unchanged.
- C-9: @test-binding: nested_register_writes_branch_discriminator_row
Subtree INDEX rows point at the next path component: leaf rows render as `<segment>/SPEC.md`, branch rows as `<segment>/INDEX.md`; columns remain `Feature | Scope | Promoted`.
- C-9a: @test-binding: gather_features_index_recurses_into_subtree
`SpecRow.scope` and `SpecRow.promoted` for a leaf at `features/<a>/<b>/.../<z>/SPEC.md` are populated from the row in `features/<a>/<b>/.../<y>/INDEX.md` whose first cell normalizes to `z` or `z/SPEC.md`. The root `features/INDEX.md`'s rows describe their own children (single-segment leaves and immediate subtree branches), not any deeper leaf.
- C-10: @test-binding: single_segment_register_preserves_flat_layout
Single-segment paths produce the pre-existing on-disk layout bit-for-bit (one row in `features/INDEX.md`, one `features/<slug>/SPEC.md`); no intermediate INDEX created.
- C-11: @test-binding: extracts_mixed_flat_and_nested
`related_specs::extract` recognizes (a) canonical `specs/features/<...>/<slug>/SPEC.md` substrings anywhere inside the section, and (b) bare backticked path tokens only when they are the bullet-leading element of a list line.
- C-11a: @test-binding: extracts_three_segment_nested_path
Bullet-leading pattern is `^\s*[-*+]\s*` `<seg>(/<seg>)*` `` where each `<seg>` matches C-2; all three GFM bullet markers (`-`, `*`, `+`) are accepted; inline backticked tokens in prose are not matched.
- C-12: @test-binding: gather_features_index_recurses_into_subtree
`gather::parse_features_index` walks `features/` INDEX-strict: traversal follows rows declared in each `INDEX.md`'s managed block; subdirectories not rowed are not descended into.
- C-12a: @judgment
Walk is bounded by max recursion depth 8; symlinks are not followed.
- C-12b: @test-binding: gather_emits_missing_child_warning_for_stale_row
Drift surfaces as warnings, not silent dropping. Row pointing at missing child file → `GatherWarning::MissingChild`; on-disk leaf at `features/<...>/<seg>/SPEC.md` with no parent INDEX row → `GatherWarning::OrphanLeaf`; on-disk subtree with no parent INDEX row → `GatherWarning::OrphanSubtree`. INDEX-registered SPECs preserve their pre-change `path` field byte-identically.
- C-13: @test-binding: gather_features_index_parses_managed_block
`SpecRow.feature_path` is the canonical relative-segments form; `SpecRow.path` retains its existing project-root-relative `PathBuf` value. JSON output adds `feature_path` additively, preserving `path`.
- C-14: @test-binding: reserved_segments_rejected
No segment may be `.` / `..`, case-insensitive `index`, or case-insensitive `spec`; segments containing `.` are already rejected by C-2.
- C-15: @test-binding: slug_mismatch_errors
Error messages quote the offending value verbatim (e.g. `` invalid SPEC path `xemu//csr`: empty segment ``).
- C-16: @test-binding: imports_writes_spec_and_index_row
`ark agent spec import --feature <p>` accepts the same `/`-separated form as the deep-tier `[**SPEC Path**]` block; existing single-segment values continue to work.
- C-16a: @test-binding: register_then_import_preserves_existing_row
`spec_import` calls the same `upsert_index_rows_leaf_to_root` as `spec_register`; brownfield imports produce the same INDEX shape as deep-tier promotions at the same path.

[**CHANGELOG**]

- 2026-05-18 `detachable-feature-spec` iteration 02: initial promotion. Establishes recursive `features/` tree, required PRD `[**SPEC Path**]` block, INDEX-strict walk with drift warnings, leaf-to-root INDEX upsert with rollback snapshots, fallible `Layout::specs_feature_dir`.
- 2026-05-24 `improve-ark-context`: C-3 relaxed to accept the in-place-update case where the parsed path points at an existing on-disk feature SPEC even if its last segment does not equal the task slug. Enables tasks whose slug describes the change (e.g. `improve-ark-context`) to update an existing feature SPEC (`ark-context`) without fragmenting the SPEC across two directories. `parse_spec_path` gains a `&Layout` parameter; the canonical "new feature, slug names it" case is unchanged.

---
