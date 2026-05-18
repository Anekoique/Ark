# `detachable-feature-spec` PLAN `01`

> Status: Revised
> Feature: `detachable-feature-spec`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

Iteration 00 was Approved with Revisions (0 CRITICAL, 4 HIGH, 4 MEDIUM, 1 LOW). The design direction stands — required PRD `[**SPEC Path**]` block, recursive `features/` tree, leaf-or-root INDEX upsert, single-segment back-compat — but the contract was loose in four ways: (R-001) `SpecRow.path` got silently repurposed instead of additively extended; (R-002) the bare-slug back-compat parser had no disambiguator; (R-003) `Layout::specs_feature_dir` actually has four call sites; (R-004) the recursive walk had no specified source of truth, no cycle bound, no depth bound. This iteration adds a new `feature_path: Vec<String>` field to `SpecRow` (preserves the existing `path: PathBuf` for byte-identical JSON contract), constrains the bare-slug parser to bullet-leading backticked tokens, enumerates every call site in Phase 1, and specifies a filesystem-authoritative walk with a depth-8 bound and symlink rejection. Goal/Constraint shape fixed per R-005; `[**CHANGELOG**]` added per R-006; Error-Display + JSON-shape validation added per R-007; `spec_import` brought into `[**API Surface**]` and Constraints per R-008; reserved-name wording tightened per R-009. Trade-off advice TR-1 (validation-gate pinning) and TR-3 (rollback snapshot per level) are incorporated as Constraints.

## Log

[**Added**]

- `SpecRow::feature_path: Vec<String>` as a *new* field alongside the existing `path: PathBuf`. The JSON contract becomes additive — pre-existing consumers reading `path` get the same `.ark/specs/features/<...>/SPEC.md` value they always got. This addresses R-001 without bumping `SCHEMA_VERSION`.
- Constraint C-2a: `Layout::specs_feature_dir(&[&str])` revalidates every segment against the kebab-case alphabet; manual callers cannot smuggle malformed segments to disk (TR-1 pin).
- Constraint C-8a: every intermediate INDEX path is snapshotted in `task_commit`'s `RollbackGuard` before mutation; mid-walk failures restore all touched INDEXes (TR-3 atomicity).
- Constraint C-11a: `related_specs::extract` accepts a *bare backticked slug* token only when it is the first non-whitespace token on a bullet line (`^\s*-\s*\` `slug` \``); inline prose tokens are ignored (R-002).
- Constraint C-12a: the recursive `parse_features_index` walker is *filesystem-authoritative* — it enumerates subdirectories of `features/`, reading each child's `INDEX.md` or `SPEC.md` directly. Rows in any INDEX are not consulted for traversal; INDEXes are written-only artifacts maintained by `spec_register`. Symlinks are not followed; max recursion depth is 8 (R-004).
- `[**CHANGELOG**]` section to the `## Spec` block with one seed entry (R-006).
- V-UT-8: asserts `ark context --format json` carries `specs.features[*].feature_path` matching the relative form (R-007.1).
- V-UT-9: asserts both new Error variants' Display strings conform to project-spec `rust/ERRORS.md` E-9 + quote the offending value (R-007.2).
- V-E-5: text-mode render shows the nested path (R-007 text-mode contract).
- V-UT-10: `spec_import` accepts a nested `--feature <path>` (R-008).
- V-UT-11: prose-token rejection in `related_specs::extract` (R-002 closure).
- V-F-4: mid-walk INDEX failure leaves no orphan files post-rollback (TR-3 closure).
- Validation entry V-UT-12 covers the grep-assertion that no `specs_feature_dir(&str)` invocation survives the refactor (R-003 closure).

[**Changed**]

- G-2, G-3, G-5 rewritten as verb-led capability statements; their invariant content moved into `[**Constraints**]` (already present as C-1..C-10) where it belongs (R-005).
- Phase 1 step 2 enumerates four production call sites and one test site for the `specs_feature_dir` signature change (R-003).
- `## Spec` `[**API Surface**]` now lists `spec_import` with its widened option shape (R-008).
- C-14 tightened to reject case-insensitive `index` / `spec` segments in addition to the alphabet-based rejection of `.md` suffixes (R-009).

[**Removed**]

- The implicit promise that `parse_features_index` is row-driven — replaced by an explicit filesystem-authoritative model. The previous wording in C-12 is superseded by C-12 + C-12a (R-004).
- The understated Phase 1 step 2 wording "Update one call site in `extract.rs`" — superseded by the enumerated list (R-003).

[**Unresolved**]

- None — every CRITICAL/HIGH/MEDIUM/LOW finding is accepted with a concrete change. TR-2 / TR-4 endorsed as-is.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Added `feature_path: Vec<String>` as a new field on `SpecRow` (option (a) from the recommendation). Existing `path: PathBuf` semantics preserved. No `SCHEMA_VERSION` bump needed; addition is permitted by `ark-context` C-6. |
| Review | R-002 | Accepted | C-11 narrowed: bare backticked slug is accepted only when it is the first token on a bullet line. C-11a + V-UT-11 enforce. Inline prose like `` `task commit` `` is rejected. |
| Review | R-003 | Accepted | Phase 1 step 2 enumerates `extract.rs:96`, `import.rs:78`, `commit.rs:175`, `archive.rs:488`, and `commit.rs:830` (test). V-UT-12 adds a grep-assert against the old signature. |
| Review | R-004 | Accepted | C-12 + C-12a specify filesystem-authoritative walk, no symlink following, max depth 8. V-E-6 covers the symlink-cycle edge case. |
| Review | R-005 | Accepted | G-2, G-3, G-5 rewritten as capabilities; their declarative content lives in `[**Constraints**]`. |
| Review | R-006 | Accepted | `[**CHANGELOG**]` section added with one seed entry. |
| Review | R-007 | Accepted | V-UT-8 (JSON shape), V-UT-9 (Error Display), V-E-5 (text-mode render) added. |
| Review | R-008 | Accepted | `spec_import` added to `[**API Surface**]` with widened option shape; new Constraint C-16; V-UT-10 added. |
| Review | R-009 | Accepted | C-14 tightened to also reject case-insensitive `index` / `spec`. |
| Review | TR-1 | Accepted | C-2a pins the validation gate in `Layout::specs_feature_dir`. |
| Review | TR-2 | Accepted | C-8b added: subtree-INDEX seed carries `ARK:FEATURES` markers byte-identical to the root INDEX. |
| Review | TR-3 | Accepted | C-8a added: every intermediate INDEX is snapshotted in `RollbackGuard` before mutation. V-F-4 added. |
| Review | TR-4 | Accepted | C-1 already requires single-token body; V-UT-1 covers "two backticked paths" rejection. |

---

## Spec

[**Goals**]

- G-1: Feature SPECs may live at arbitrary depth under `.ark/specs/features/`.
- G-2: Deep-tier `task commit` extracts SPECs into the declared subtree of the recursive `features/` tree.
- G-3: Nested feature paths surface in `ark context` and PRD-related-specs parsing.
- G-4: Single-segment feature paths preserve the pre-existing flat layout bit-for-bit.

[**Non-goals**]

- NG-1: No auto-migration of existing flat SPECs; single-segment paths remain valid leaves at the root.
- NG-2: No recursive layout for `.ark/tasks/`; the tasks tree stays flat.
- NG-3: No `ark agent spec move` helper; reorganization is hand-edit until a follow-up task adds one.

[**Architecture**]

```
crates/ark-core/src/
├── error.rs                          (+) FeaturePathMissing, InvalidFeaturePath
├── layout.rs                         (*) specs_feature_dir: (&self, &[&str]) -> PathBuf
│                                          + revalidates segments (C-2a)
├── commands/agent/
│   ├── spec/
│   │   ├── extract.rs                (*) opts.feature_path: Vec<String>;
│   │   │                                 target = layout.specs_feature_dir(&segments)
│   │   ├── register.rs               (*) opts.feature_path: Vec<String>;
│   │   │                                 upsert_index_rows_leaf_to_root with seed template
│   │   ├── import.rs                 (*) accepts segments; CLI --feature widened
│   │   └── mod.rs                    (re-exports unchanged shape)
│   └── task/
│       ├── commit.rs                 (*) reads PRD, calls parse_spec_path(),
│       │                                 threads segments into spec_extract + spec_register;
│       │                                 RollbackGuard snapshots every intermediate INDEX
│       └── prd.rs                    (+) NEW leaf: SPEC-path parser
│                                          fn parse_spec_path(prd: &str, slug: &str)
│                                              -> Result<Vec<String>>
└── commands/context/
    ├── related_specs.rs              (*) accepts bullet-led nested + bare backticked paths
    ├── gather.rs                     (*) parse_features_index walks filesystem (C-12 + C-12a)
    └── model.rs                      (*) SpecRow gains feature_path: Vec<String>
                                            (path: PathBuf preserved)

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
  │     ├── rollback.snapshot_each(intermediate_index_paths(&layout, &segments))
  │     ├── spec_extract(SpecExtractOptions { feature_path: segments.clone(), .. })
  │     │     └── target = layout.specs_feature_dir(&segments).join("SPEC.md")
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
  └── existing git commit closure
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
    /// segment against the kebab-case alphabet; panics on a malformed
    /// segment (parse_spec_path is the only legal constructor).
    pub fn specs_feature_dir(&self, segments: &[&str]) -> PathBuf;
}

// ark-core/src/commands/context/model.rs
pub struct SpecRow {
    pub name: String,            // last path segment (leaf slug)
    pub path: PathBuf,           // existing: project-root-relative SPEC.md path
    pub feature_path: Vec<String>,   // NEW: features/-relative directory segments
    pub scope: String,
}

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

// ark-core/src/commands/context/related_specs.rs
/// Returns paths relative to `features/` (no `/SPEC.md` suffix). Recognizes:
///   - canonical: `specs/features/<...>/<slug>/SPEC.md` anywhere in the section
///   - bare backticked slug: `^\s*-\s*` `slug` `` (bullet-leading only)
/// Inline backticked tokens in prose are ignored.
pub fn extract(prd_text: &str) -> Vec<String>;
```

CLI surface change (semver-stable surface): `ark agent spec import --feature <p>` accepts `/`-separated values; single-segment values continue to work. The `ark agent` namespace is hidden and non-semver-stable; the practical surface is the slash commands' invocations.

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
- C-2a: `Layout::specs_feature_dir(&[&str])` revalidates every segment against C-2 and panics on a malformed segment. `parse_spec_path` is the only legal constructor of a validated segment vec.
- C-3: The last segment of the parsed path MUST equal `task.toml.slug`; mismatch → `InvalidFeaturePath { reason: "last segment must equal task slug" }`.
- C-4: Absent block, empty body, or a segment failing C-2 → `FeaturePathMissing` / `InvalidFeaturePath` with a single quoted reason.
- C-5: `task commit` reads `[**SPEC Path**]` from the latest `PRD.md` on every invocation; path changes across iterations are honored.
- C-6: `parse_spec_path` rejects paths containing `.`, `..`, or empty segments after the split.
- C-7: `spec_extract` writes to `Layout::specs_feature_dir(&segments).join("SPEC.md")` and ensures the directory exists; CHANGELOG-on-overwrite semantics preserved.
- C-8: `spec_register` upserts rows from leaf to root: each intermediate `INDEX.md` is created from `FEATURE_SUBTREE_INDEX.md` template when missing.
- C-8a: `task_commit`'s `RollbackGuard` snapshots every intermediate INDEX path produced by `intermediate_index_paths(&layout, &segments)` *before* mutation; mid-walk failures restore all touched INDEXes.
- C-8b: `FEATURE_SUBTREE_INDEX.md` carries `<!-- ARK:FEATURES:START -->` / `<!-- ARK:FEATURES:END -->` markers byte-identical to the root `features/INDEX.md`'s managed-block delimiters; the existing `read_managed_block` / `update_managed_block` helpers apply unchanged.
- C-9: Subtree INDEX rows point at the next path component: leaf rows render as `<segment>/SPEC.md`, branch rows as `<segment>/INDEX.md`; columns remain `Feature | Scope | Promoted`.
- C-10: Single-segment paths produce the pre-existing on-disk layout bit-for-bit (one row in `features/INDEX.md`, one `features/<slug>/SPEC.md`); no intermediate INDEX created.
- C-11: `related_specs::extract` recognizes (a) canonical `specs/features/<...>/<slug>/SPEC.md` substrings anywhere inside the section, and (b) bare backticked slug tokens only when they are the bullet-leading element of a list line.
- C-11a: Bullet-leading pattern is `^\s*-\s*` `<seg>(/<seg>)*` `` where each `<seg>` matches C-2; inline backticked tokens in prose are not matched.
- C-12: `gather::parse_features_index` recursively walks the `features/` directory; each leaf `SPEC.md` produces one `SpecRow` whose `feature_path` is the segments joined by `/`.
- C-12a: The walk is filesystem-authoritative — directories are enumerated via `read_dir`; INDEX-row content is ignored for traversal. Symlinks are not followed. Maximum recursion depth is 8.
- C-13: `SpecRow.feature_path` is the canonical relative-segments form; `SpecRow.path` retains its existing project-root-relative `PathBuf` value. JSON output adds `feature_path` additively, preserving `path`.
- C-14: No segment may be `.` / `..`, case-insensitive `index`, or case-insensitive `spec`; segments containing `.` are already rejected by C-2.
- C-15: Error messages quote the offending value verbatim (e.g. `` invalid SPEC path `xemu//csr`: empty segment ``).
- C-16: `ark agent spec import --feature <p>` accepts the same `/`-separated form as the deep-tier `[**SPEC Path**]` block; existing single-segment values continue to work.

[**CHANGELOG**]

- 2026-05-18 `detachable-feature-spec` iteration 01: initial promotion. Establishes recursive `features/` tree, required PRD `[**SPEC Path**]` block, leaf-to-root INDEX upsert with rollback snapshots.

---

## Runtime

[**Main Flow**] (deep-tier `task commit`)

1. `task_commit` validates phase, staging, etc. (existing logic).
2. Read `PRD.md`, call `prd::parse_spec_path(prd_text, &toml.slug)`.
3. On `Ok(segments)`: compute `intermediate_index_paths(&layout, &segments)` and register them in `RollbackGuard` (C-8a).
4. Build `SpecExtractOptions { feature_path: segments.clone(), .. }`; call `spec_extract`.
5. Build `SpecRegisterOptions { feature_path: segments, scope, .. }`; call `spec_register` — walks leaf→root, seeding missing INDEXes, upserting rows.
6. Continue with the existing commit closure (toml save, git stage, commit).

[**Failure Flow**]

1. `[**SPEC Path**]` block missing → `Error::FeaturePathMissing { prd_path }`. No SPEC written, no INDEX touched, no git commit.
2. Invalid kebab-case, `..`, slug mismatch, reserved segment → `Error::InvalidFeaturePath { prd_path, value, reason }`. Same rollback.
3. `spec_extract` failure → `RollbackGuard` restores any pre-snapshot of `SPEC.md`. INDEXes untouched (they come later).
4. `spec_register` failure mid-walk → `RollbackGuard` restores every INDEX snapshotted in step 3. No orphan files survive.

[**State Transitions**]

- PRD with valid `[**SPEC Path**]` → SPEC at `features/<path>/SPEC.md` + N INDEX upserts (one per path segment).
- PRD without block → commit refused; phase stays at `Verify`; user fills the block and retries.

---

## Implementation

[**Phase 1 — Path plumbing**]

1. Add `Error::FeaturePathMissing { prd_path }` and `Error::InvalidFeaturePath { prd_path, value, reason }`. Display strings per `rust/ERRORS.md` E-9 (lowercase, no trailing punctuation, no `error:` prefix).
2. Change `Layout::specs_feature_dir(&self, &str) -> PathBuf` to `(&self, &[&str]) -> PathBuf` and add segment revalidation (panics on malformed). Update **four production call sites and one test**:
   - `crates/ark-core/src/commands/agent/spec/extract.rs:96`
   - `crates/ark-core/src/commands/agent/spec/import.rs:78`
   - `crates/ark-core/src/commands/agent/task/commit.rs:175`
   - `crates/ark-core/src/commands/agent/task/archive.rs:488`
   - `crates/ark-core/src/commands/agent/task/commit.rs:830` (test)
   Confirm with `grep -rn 'specs_feature_dir(' crates/` after the refactor; V-UT-12 enforces.
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
3. Extend `spec_import`: `SpecImportOptions::feature_path: Vec<String>`; CLI flag parses `/`-separated input. Existing single-segment behavior preserved when called with `vec![feature]`.

[**Phase 3 — task_commit wiring**]

1. Read PRD inside `task_commit`; call `parse_spec_path`; thread the result into the existing `spec_extract` + `spec_register` calls.
2. Compute `intermediate_index_paths(&layout, &segments) -> Vec<PathBuf>` returning paths from leaf-parent to root-parent. Register each in `RollbackGuard` before the first INDEX mutation.
3. Existing `task_commit` tests update to put a `[**SPEC Path**]` block in their seed PRDs.

[**Phase 4 — Context + parser surface**]

1. `SpecRow.feature_path: Vec<String>` field added; `path: PathBuf` preserved (R-001 closure).
2. `gather::parse_features_index` reshaped to a recursive `read_dir` walk: visit `features/`, for each subdir descend; at each level, if `SPEC.md` exists it's a leaf, accumulate; if any subdir exists, descend (bounded by max depth 8, no symlinks). The root `features/INDEX.md`'s managed-block is read but only to populate the `scope` / `promoted` columns for matching leaves; existence and tree shape come from the filesystem.
3. `related_specs::extract` accepts canonical-path tokens anywhere in the section, plus bullet-leading bare backticked slug tokens (C-11 + C-11a). Returns canonical relative paths.
4. Text-mode render shows the nested path (`xemu/csr`) rather than the leaf segment alone; JSON adds `"feature_path": [...]` to each features row.

[**Phase 5 — Templates + workflow doc**]

1. PRD template gains the `[**SPEC Path**]` block with placeholder example.
2. `templates/ark/templates/FEATURE_SUBTREE_INDEX.md` added.
3. `templates/ark/workflow.md` §6 (Specs) gains one paragraph on the recursive shape and the SPEC-Path requirement, mirroring §6's project-spec recursive-INDEX wording.

[**Phase 6 — Self-host demonstration**]

1. This task's own PRD carries a single-segment SPEC Path (`detachable-feature-spec`). Commit produces `features/detachable-feature-spec/SPEC.md`, validating C-10 against the live binary.

---

## Trade-offs

- T-1: **`Vec<String>` vs typed `FeaturePath` newtype.** Vec aligns with the rest of the agent layer; validation is pinned by `Layout::specs_feature_dir` revalidating segments (C-2a). If construction sites multiply later, refactoring to a newtype is cheap. Reviewer endorsed with the pin (TR-1).
- T-2: **Reuse `validate_slug` alphabet for segments vs. relax.** Reusing keeps the on-disk shape consistent. Adv: one alphabet across slugs / segments / row identifiers. Disadv: hypothetical mixed-case segments rejected — out of scope.
- T-3: **Auto-create subtree INDEXes vs require user to seed.** Auto-create wins on symmetry with `specs/project/`. Seed template carries the managed-block markers byte-identical to the root (C-8b), so the next upsert is idempotent. Reviewer endorsed (TR-2).
- T-4: **Walk order leaf→root vs root→leaf for INDEX upsert.** Atomicity comes from snapshotting *every* intermediate INDEX in `RollbackGuard` before mutation (C-8a); order is then equivalent on rollback. Leaf→root retained so `indexes_touched` in the summary reflects the natural reading order ("wrote csr/SPEC.md, then xemu/INDEX.md, then features/INDEX.md"). Reviewer's atomicity concern (TR-3) is the snapshot, not the order.
- T-5: **`[**SPEC Path**]` body parsing strictness.** Strict: single token, bare or backticked. Reject multi-line / multi-token bodies. Reviewer endorsed (TR-4).
- T-6: **Filesystem-authoritative vs row-driven walk in `parse_features_index`.** Filesystem-authoritative wins: it produces a deterministic surface regardless of stale or hand-edited INDEX rows. Stale rows that point at missing children are silently dropped; missing rows for present directories still surface the leaf. INDEXes are write-only artifacts maintained by `spec_register`. Adv: no two-source-of-truth bug class. Disadv: agents reading the INDEX directly may see rows whose subtrees don't exist; mitigated by `gather` being the only consumer in the projection.

---

## Validation

[**Unit Tests**]

- V-UT-1 (G-2, C-1..C-6): `parse_spec_path` — happy paths (1/2/3 segments), block missing, empty body, multi-line body, multi-token body (two backticked paths), bad alphabet, slug mismatch, `.`/`..`/empty segment, reserved `index`/`spec`/`INDEX` segments, trailing newlines, backtick-quoted form.
- V-UT-2 (C-2, C-2a): kebab-case alphabet test against `^[a-z0-9][a-z0-9_-]*$`; `Layout::specs_feature_dir` panics on a malformed segment.
- V-UT-3 (G-1, C-7): `spec_extract` writes to nested target; round-trip read equals input; CHANGELOG appended on overwrite at nested path.
- V-UT-4 (G-2, C-8, C-9): `spec_register` leaf→root walk seeds intermediate INDEX, upserts correct row shapes (leaf `<seg>/SPEC.md`, branch `<seg>/INDEX.md`).
- V-UT-5 (G-4, C-10): single-segment path produces byte-identical on-disk shape as the legacy flat layout (golden-file diff against a pre-recorded `features/<slug>/SPEC.md` + `features/INDEX.md`).
- V-UT-6 (G-3, C-11, C-11a): `related_specs::extract` mixed input — canonical path + bullet-leading bare slug + inline prose backtick + nested bullet-leading + invalid segment + duplicate. Returns canonical relative paths, deduped, first-seen order.
- V-UT-7 (G-3, C-12, C-12a, C-13): `gather::parse_features_index` recursive walk against a three-level fixture; resulting `SpecRow`s carry correct `feature_path` and preserved `path`.
- V-UT-8 (R-007.1): `ark context --format json` carries `specs.features[*].feature_path` matching the relative form; `specs.features[*].path` is byte-identical to the pre-change form.
- V-UT-9 (R-007.2): `Display` for `Error::FeaturePathMissing` and `Error::InvalidFeaturePath` conforms to `rust/ERRORS.md` E-9 and quotes the offending value (`FeaturePathMissing` quotes `prd_path`; `InvalidFeaturePath` quotes `value`).
- V-UT-10 (R-008, C-16): `spec_import` accepts a nested `--feature <p>` and produces the same on-disk shape as a deep-tier commit at the same path.
- V-UT-11 (R-002, C-11, C-11a): `related_specs::extract` rejects inline backticked tokens like `` `task commit` `` and `` `feature_path` `` while accepting bullet-leading `` `klib` `` / `` `xemu/csr` ``.
- V-UT-12 (R-003): grep-assertion in a `#[test]` that no `specs_feature_dir(&str)` invocation survives the refactor (uses `assert_source_clean` parallel pattern; sees only `(&[`).

[**Integration Tests**]

- V-IT-1 (G-1, G-2): end-to-end `task new`/`plan`/`execute`/`verify`/`commit` with `[**SPEC Path**]: foo/bar/baz`. Asserts: SPEC at `features/foo/bar/baz/SPEC.md`; INDEXes at three levels; root `features/INDEX.md` gains one row pointing at `foo/INDEX.md`.
- V-IT-2 (G-4): same flow with `[**SPEC Path**]: <slug>` (single segment) → flat layout reproduces exactly.
- V-IT-3 (G-2): two consecutive deep-tier tasks under the same subtree (`foo/a`, `foo/b`) — second task finds the seeded `foo/INDEX.md` and appends without duplicating the `foo/INDEX.md` row in root `features/INDEX.md`.

[**Failure / Robustness**]

- V-F-1 (C-4): `task commit` on deep tier with no `[**SPEC Path**]` errors `FeaturePathMissing`; no SPEC written; no INDEX touched; no git commit; phase unchanged.
- V-F-2 (C-3, C-15): `task commit` with slug mismatch errors `InvalidFeaturePath { reason: "last segment must equal task slug" }`; Display string quotes the offending value.
- V-F-3 (C-6, C-15): `task commit` with `..` segment errors `InvalidFeaturePath { reason: "segment contains `..`" }`; Display string quotes the offending value.
- V-F-4 (C-8a, TR-3): inject a failure in the second INDEX upsert of a three-level walk; assert that the leaf `SPEC.md` and every snapshotted INDEX are restored to pre-mutation state; no orphan files survive.

[**Edge Cases**]

- V-E-1: PRD with `[**SPEC Path**]` block followed by another bracketed section parses without consuming the next section's content.
- V-E-2: PRD with backtick-quoted path (`` `xemu/csr` ``) parses identically to bare path.
- V-E-3: Empty PRD related-specs section coexists with non-empty SPEC Path block (independent parsers).
- V-E-4: Pre-existing `features/INDEX.md` content above/below the managed block is preserved.
- V-E-5 (R-007 text-mode): `ark context --format text` renders nested-path features rows showing `xemu/csr` rather than the leaf alone.
- V-E-6 (C-12a, R-004): a symlink under `features/` is not followed; depth-8+ trees are surfaced up to depth 8 with a warning logged for deeper subtrees.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-3, V-UT-4, V-IT-1 |
| G-2 | V-UT-1, V-UT-2, V-UT-4, V-IT-1, V-IT-3 |
| G-3 | V-UT-6, V-UT-7, V-UT-8, V-UT-11, V-E-5 |
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
| C-10 | V-UT-5, V-IT-2 |
| C-11 | V-UT-6, V-UT-11 |
| C-11a | V-UT-11 |
| C-12 | V-UT-7 |
| C-12a | V-UT-7, V-E-6 |
| C-13 | V-UT-7, V-UT-8 |
| C-14 | V-UT-1 |
| C-15 | V-UT-9, V-F-2, V-F-3 |
| C-16 | V-UT-10 |
