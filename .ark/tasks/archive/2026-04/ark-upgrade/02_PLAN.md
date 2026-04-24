# `ark-upgrade` PLAN `02`

> Status: Approved for Implementation
> Feature: `ark-upgrade`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Iteration 02 closes `01_REVIEW`'s single HIGH finding (R-013) and absorbs six non-blocking polish items. No architecture changes. The substantive additions are (a) a path-safety invariant that mirrors `load.rs`'s trust posture — every path read from `manifest.files` is normalized through `layout.resolve_safe` before any read/write/delete; (b) a constraint pinning `collect_desired_templates` to produce project-relative keys matching `init.rs::extract`'s `dest_root.join(entry.relative_path).strip_prefix(project_root)` shape; (c) explicit documentation that `Classification::Unchanged { refresh_hash: false }` emits no `PlannedAction` (direct counter bump), distinguishing it cleanly from `Preserve`; (d) deterministic `plan_actions` output via `(bucket, relative_path)` sort; (e) a failure-mode entry for the step-14 manifest rewrite; (f) extended V-UT-14 asserting `--allow-downgrade` is orthogonal to the policy group; (g) V-F-6 re-mapped under G-10 / C-8 to close an orphan citation.

## Log

[**Added**]

- `C-17` — every path read from `manifest.files` is normalized via `layout.resolve_safe` before any read/write/delete. Entries that fail validation surface `Error::UnsafeManifestPath` with no filesystem activity attempted.
- `C-18` — `collect_desired_templates` produces project-relative `PathBuf` keys matching the shape `init.rs::extract` stores in `manifest.files` (`.ark/<tree-rel>` for `ARK_TEMPLATES`, `.claude/<tree-rel>` for `CLAUDE_TEMPLATES`).
- `C-19` — `plan_actions` output is sorted by `(action_bucket, relative_path)` before `apply_writes` / `apply_deletions`.
- `Error::UnsafeManifestPath { path: PathBuf, reason: &'static str }` — new error variant.
- `V-F-9` — `manifest_entry_outside_project_root_is_rejected`: inject `../escape.md` into `.installed.json`; upgrade returns `UnsafeManifestPath`; no filesystem activity outside project root.
- `V-UT-17` — `desired_template_keys_match_init_manifest_entries`: after `init`, `sorted(collect_desired_templates().keys()) == sorted(manifest.files)`.
- `V-UT-18` — `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`: mechanical source-level enforcement of C-12 and C-13 via `include_str!("upgrade.rs")`. Absorbs `02_REVIEW` R-022.
- Failure Flow entry 9 — step-14 manifest write failure: on-disk manifest still references now-deleted files; next upgrade classifies them as "not-desired, absent → `DropManifestEntry`" cleanly.
- C-14 sentence documenting `installed_at` as "time of last successful init or upgrade" (not "first install time"). Absorbs `02_REVIEW` R-020.

[**Changed**]

- V-UT-14 extended: asserts `--force --allow-downgrade` (and peers) parse successfully; any two policy flags together are rejected by clap.
- Acceptance Mapping: G-10 row now lists `V-IT-12, V-IT-13, V-F-6`; C-8 row now lists `V-IT-12, V-IT-13, V-F-6`.
- `Classification::Unchanged { refresh_hash: false }` handling spelled out: "classifier bumps `summary.unchanged` directly; no `PlannedAction` emitted". `Preserve` handling: "emits `PlannedAction::Preserve`; `apply_writes` bumps `summary.modified_preserved`".
- PlannedAction enum comment expanded to spell out the two counter-only cases and which variant each maps to.
- Main Flow renumbered: step 3 is now `validate_manifest_paths` (was 4), step 4 is `check_version` (was 3). Path safety runs before semantic checks — absorbs `02_REVIEW` R-021.
- Acceptance Mapping: C-12 and C-13 rows changed from "review-only" to `V-UT-18`. Absorbs `02_REVIEW` R-022.

[**Removed**]

Nothing removed.

[**Unresolved**]

None. Every `01_REVIEW` finding has an explicit resolution below.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-013 | Accepted | Added `C-17` (resolve_safe on every `manifest.files` entry) and `Error::UnsafeManifestPath`. Added V-F-9. Mirrors `load.rs:85-93`'s trust-boundary pattern. Applies the same guard to `collect_desired_templates` output symmetrically (the walk produces safe paths today, but the plan states the invariant explicitly — cheap hardening). |
| Review | R-014 | Accepted | Added `C-18` pinning the project-relative key shape. Call graph updated to `collect_desired_templates() -> Vec<(PathBuf /* project-relative, `.ark/<...>` \| `.claude/<...>` */, &'static [u8])>`. Added V-UT-17 as a parity test. |
| Review | R-015 | Accepted | Data Structure enum comment expanded. "Runtime / State Transitions" annotated to say which cases emit a `PlannedAction` and which bump counters directly. `Unchanged{refresh_hash=false}` → no action, bump `summary.unchanged` in `plan_actions`. `Preserve` → explicit variant; `apply_writes` bumps `summary.modified_preserved`. |
| Review | R-016 | Accepted | Added `C-19` requiring `plan_actions` to sort its output by `(bucket, relative_path)`. V-F-8 extended to verify two consecutive partial-state reruns produce byte-identical manifest + disk state. |
| Review | R-017 | Accepted (option b) | Kept the two-write design (per TR-8); added Failure Flow entry 9 documenting the step-14 failure path. |
| Review | R-018 | Accepted | V-UT-14 extended; see [**Changed**]. |
| Review | R-019 | Accepted | Acceptance Mapping updated; see [**Changed**]. |
| Review | TR-8 | Accepted | Two-write design retained. Documented as per R-017 option (b). |
| Review | TR-9 | Accepted | `resolve_safe`-equivalent adopted. Chose a distinct `Error::UnsafeManifestPath` variant (rather than reusing `UnsafeSnapshotPath`) so the error chain makes clear which trust boundary triggered — snapshot vs. manifest. |
| Review(02) | R-020 | Accepted | Option (a) — documented `installed_at` semantics in C-14 as "time of last successful init or upgrade". Simpler than gating the refresh on PlannedAction count and matches the two-write pattern. |
| Review(02) | R-021 | Accepted | Swapped Main Flow steps 3 ↔ 4 so `validate_manifest_paths` runs before `check_version`. Failure Flow entries renumbered correspondingly. No test impact. |
| Review(02) | R-022 | Accepted | Added V-UT-18 (`include_str!("upgrade.rs")` source-grep); C-12 and C-13 Acceptance Mapping rows now cite V-UT-18 instead of VERIFY-phase greps. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec `Core specification`

[**Goals**]

- G-1: `ark upgrade` is a top-level, visible subcommand. Safe to run repeatedly.
- G-2: User-modified files are detected by SHA-256 content hashing. `Manifest` records the hash of every file Ark writes; upgrade compares current on-disk content to the recorded hash.
- G-3: When a template file's content changes AND the user has not modified it, upgrade rewrites it silently and records the new hash.
- G-4: When a template file changes AND the user has modified it, upgrade prompts (overwrite / skip / write `.new`). Non-TTY and flag overrides resolve non-interactively.
- G-5: Files the user has modified but which did not change in the template set are left alone (`modified_preserved` counter).
- G-6: Files in the old template set that no longer exist in the new set are deleted if the recorded hash matches current sha256; otherwise left in place and the manifest entry is dropped.
- G-7: Fresh `init` records hashes at write time. `upgrade` on a pre-hash-tracking project backfills hashes.
- G-8: Upgrade refuses with `Error::NotLoaded` (missing manifest), `Error::DowngradeRefused` (cli < project, no `--allow-downgrade`), or `Error::UnsafeManifestPath` (manifest entry escapes project root or contains unsafe components).
- G-9: The only file-level protection is `.ark/.installed.json`. The broader "don't clobber user content" invariant falls out of upgrade acting only on `manifest.files ∪ desired_templates`. User-authored paths (`.ark/tasks/**`, `.ark/specs/features/<slug>/**`, `.ark/specs/project/<name>/**`) are in neither set.
- G-10: The `CLAUDE.md` managed block is re-applied on every upgrade via `update_managed_block`.
- G-11: Upgrade prints a `Display` summary: `{N} file(s): {A} added · {U} updated · {S} unchanged · {M} modified-preserved · {O} overwritten · {K} skipped · {C} .new-copied · {D} deleted · {R} orphaned` + version transition line. Counters always printed in fixed order, even when zero.

- NG-1: No migration manifest system. Deferred.
- NG-2: No network I/O.
- NG-3: No backup directory.
- NG-4: No recursive directory rename logic.
- NG-5: No config-driven skip list. Deferred.
- NG-6: No `.version` sidecar file.
- NG-7: No automatic invocation.
- NG-8: No CRLF/LF normalization before hashing. Documented; workaround is `git config core.autocrlf false`.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                     — adds `Upgrade(UpgradeArgs)` top-level
└── ark-core/src/
    ├── lib.rs                               — re-exports upgrade public API
    ├── error.rs                             — adds Error::DowngradeRefused + Error::UnsafeManifestPath
    ├── io/path_ext.rs                       — adds `hash_sha256()` + free `hash_bytes`
    ├── state/manifest.rs                    — adds `hashes: BTreeMap<PathBuf,String>`
    │                                          + `record_file_with_hash`, `hash_for`,
    │                                          `clear_hash`, `drop_file`
    ├── commands/
    │   ├── init.rs                          — records hashes when writing files
    │   └── upgrade.rs                       — the new command
    └── templates.rs                          — unchanged
```

**Call graph for `upgrade`:**

```
upgrade(opts, prompter)
  ├── Manifest::read → Error::NotLoaded if missing
  ├── validate_manifest_paths(&manifest.files) → Error::UnsafeManifestPath on violation  (C-17; runs BEFORE version check)
  ├── check_version (semver cmp) → Error::DowngradeRefused if project > cli and !allow_downgrade
  ├── collect_desired_templates()              → Vec<(PathBuf /* project-relative */, &'static [u8])>  (C-18)
  ├── plan_actions()                            → Vec<PlannedAction>  (sorted per C-19)
  │     per desired file:   classify → Add | Unchanged{refresh} | AutoUpdate | UserModified | AmbiguousNoHash
  │     per manifest file not in desired:       classify_removal → SafeRemove | Orphaned
  │     resolve(UserModified | AmbiguousNoHash-with-content-mismatch) via policy or prompter
  │     Unchanged{refresh_hash=false}: bump summary.unchanged inline, no PlannedAction emitted
  ├── apply_writes()                            (Write, CreateNew, RefreshHashOnly, Preserve)
  │     — mutates manifest in-memory
  ├── update_managed_block(CLAUDE.md, "ARK", MANAGED_BLOCK_BODY)
  ├── manifest.version = CARGO_PKG_VERSION; manifest.installed_at = now; manifest.write()
  │     ^ durable BEFORE deletions
  ├── apply_deletions()                         (Delete, DropManifestEntry)
  ├── manifest.write() again if deletions mutated the manifest
  └── UpgradeSummary
```

[**Data Structure**]

```rust
// ark-core/src/state/manifest.rs  (extension)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub installed_at: DateTime<Utc>,
    pub files: Vec<PathBuf>,
    pub managed_blocks: Vec<ManagedBlock>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hashes: BTreeMap<PathBuf, String>,
}

impl Manifest {
    pub fn record_file_with_hash(&mut self, path: impl Into<PathBuf>, contents: &[u8]);
    pub fn hash_for(&self, path: &Path) -> Option<&str>;
    pub fn clear_hash(&mut self, path: &Path);
    pub fn drop_file(&mut self, path: &Path);
}
```

```rust
// ark-core/src/commands/upgrade.rs

#[derive(Debug, Clone)]
pub struct UpgradeOptions {
    pub project_root: PathBuf,
    pub conflict_policy: ConflictPolicy,
    pub allow_downgrade: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy { Interactive, Force, Skip, CreateNew }

pub trait Prompter {
    fn prompt(&mut self, relative_path: &Path) -> Result<ConflictChoice>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice { Overwrite, Skip, CreateNew }

#[derive(Debug, Default, Clone)]
pub struct UpgradeSummary {
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub modified_preserved: usize,
    pub overwritten: usize,
    pub skipped: usize,
    pub created_new: usize,
    pub deleted: usize,
    pub orphaned: usize,
    pub version_from: String,
    pub version_to: String,
}

pub fn upgrade(opts: UpgradeOptions, prompter: &mut dyn Prompter) -> Result<UpgradeSummary>;
```

```rust
// internal to upgrade.rs

enum Classification {
    Add,
    Unchanged { refresh_hash: bool },
    AutoUpdate,
    UserModified,
    AmbiguousNoHash,
}

enum RemovalClassification { SafeRemove, Orphaned }

// PlannedAction variants correspond 1:1 to on-disk or manifest side-effects
// performed in `apply_writes` / `apply_deletions`. Counter-only cases are
// handled inline by `plan_actions` without emitting a PlannedAction:
//   - Unchanged { refresh_hash: false } → `summary.unchanged += 1` inline
// `Preserve` IS a variant because it is the resolved form of UserModified→Skip
// and must be distinguishable in the sorted apply pass so summary accounting
// remains consistent under partial failures.
enum PlannedAction {
    Write { relative: PathBuf, contents: &'static [u8], kind: WriteKind },
    RefreshHashOnly { relative: PathBuf, contents: Vec<u8> },
    CreateNew { relative: PathBuf, contents: &'static [u8] },
    Preserve { relative: PathBuf },
    Delete { relative: PathBuf },
    DropManifestEntry { relative: PathBuf },
}

enum WriteKind { Add, AutoUpdate, Overwrite }

// Bucket order for the C-19 sort:
//   Write{Add} < Write{AutoUpdate} < Write{Overwrite}
//     < CreateNew < RefreshHashOnly < Preserve
//     < Delete < DropManifestEntry
// Within a bucket, `relative_path` (lexicographic) is the secondary key.
```

New error variants:

```rust
// ark-core/src/error.rs
Error::DowngradeRefused    { project_version: String, cli_version: String },
Error::UnsafeManifestPath  { path: PathBuf, reason: &'static str },
```

`PathExt` additions:

```rust
// ark-core/src/io/path_ext.rs
fn hash_sha256(&self) -> Result<Option<String>>;   // hex lowercase; None if file missing
pub fn hash_bytes(contents: &[u8]) -> String;       // free fn, hex-lowercase
```

[**API Surface**]

Library re-exports from `ark-core/src/lib.rs`:

```rust
pub use commands::{
    InitOptions, InitSummary, LoadOptions, LoadSummary, RemoveOptions, RemoveSummary,
    UnloadOptions, UnloadSummary,
    UpgradeOptions, UpgradeSummary, ConflictPolicy, ConflictChoice, Prompter,
    agent::{...},
    init, load, remove, unload, upgrade,
};
```

CLI (`ark-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum Command {
    Init(InitArgs),
    Load(LoadArgs),
    Unload(UnloadArgs),
    Remove(RemoveArgs),
    Upgrade(UpgradeArgs),
    #[command(hide = true)]
    Agent(AgentArgs),
}

#[derive(Args)]
#[group(id = "policy", multiple = false)]
struct UpgradeArgs {
    /// Overwrite user-modified files without prompting.
    #[arg(long, group = "policy")]
    force: bool,
    /// Preserve user-modified files without prompting.
    #[arg(long, group = "policy")]
    skip_modified: bool,
    /// Write updated template as `<path>.new` without prompting.
    #[arg(long, group = "policy")]
    create_new: bool,
    /// Allow proceeding when CLI version < project version.
    #[arg(long)]
    allow_downgrade: bool,
}
```

(`--allow-downgrade` has NO `group` attribute; orthogonal to the policy group.)

Stdio prompter in the binary crate:

```rust
struct StdioPrompter;   // uses std::io::IsTerminal; non-TTY → Skip
```

[**Constraints**]

- C-1: Hashes are SHA-256, hex-encoded lowercase. Stored under `manifest.hashes` using project-relative `PathBuf` keys matching `manifest.files` entries exactly.
- C-2: Every `init` write routes through a helper that records file AND hash.
- C-3: Upgrade only acts on paths in `manifest.files ∪ desired_templates`. `.ark/.installed.json` is the sole file-level exemption.
- C-4: Upgrade refuses with `Error::NotLoaded` when the manifest file is absent.
- C-5: Upgrade refuses with `Error::DowngradeRefused` when `semver::Version::parse(&manifest.version) > semver::Version::parse(CARGO_PKG_VERSION)`, unless `opts.allow_downgrade`. Unparseable `manifest.version` → treat as unknown and proceed.
- C-6: Version comparison uses `semver::Version`. Same-version upgrades run a full pass.
- C-7: `ConflictPolicy::Interactive` + non-TTY stdin → `ConflictChoice::Skip` without reading, with a single stderr note at upgrade start.
- C-8: The `CLAUDE.md` managed block is re-applied on every upgrade via `update_managed_block` with `MANAGED_BLOCK_BODY`. Not hash-tracked.
- C-9: `.new` files are NOT recorded in the manifest and NOT hashed.
- C-10: When a template has been removed between versions, upgrade deletes the on-disk file iff `manifest.hash_for(path) == Some(current_sha256)`. Otherwise the file is left in place (orphaned). Either way, the entry is dropped from `manifest.files` AND `manifest.hashes`.
- C-11: `AmbiguousNoHash` (no recorded hash + on-disk differs from desired) is treated as `UserModified`. "No recorded hash + on-disk matches desired" is `Classification::Unchanged { refresh_hash: true }`.
- C-12: All filesystem access in `commands/upgrade.rs` routes through `io::PathExt` / `io::fs` helpers.
- C-13: All path composition in `upgrade.rs` routes through `layout::Layout`.
- C-14: `UpgradeSummary::Display` output is deterministic. `Manifest.installed_at` is refreshed to `Utc::now()` on every successful upgrade (step 12), including zero-delta runs — the field is defined as "time of last successful `init` or `upgrade`", not "time of first install". Tests that serialize the manifest to a golden fixture must either mask or override this field.
- C-15: `Prompter` is dyn-compatible. No generic methods; no `Self: Sized` bounds.
- C-16: Upgrade is not safe against concurrent file modification.
- C-17 *(new)*: Every path read from `manifest.files` is normalized via `layout.resolve_safe` before any read/write/delete. Entries that fail validation surface `Error::UnsafeManifestPath { path, reason }` and halt the upgrade before any filesystem mutation. Validation runs immediately after `Manifest::read`, before `plan_actions`. The same `resolve_safe` invariant holds for `collect_desired_templates` output (compiled-in, but asserted symmetrically for consistency).
- C-18 *(new)*: `collect_desired_templates` yields project-relative `PathBuf` keys shaped identically to `init.rs::extract`'s output:
  - `ARK_TEMPLATES`: `.ark/<tree-relative-path>`.
  - `CLAUDE_TEMPLATES`: `.claude/<tree-relative-path>`.

  Implementation: use `dest_root.join(entry.relative_path).strip_prefix(project_root)` where `dest_root` is `layout.ark_dir()` or `layout.claude_dir()` — same idiom as `init.rs::extract`.
- C-19 *(new)*: `plan_actions` returns `Vec<PlannedAction>` sorted by `(bucket, relative_path)`. Bucket order: `Write{Add}`, `Write{AutoUpdate}`, `Write{Overwrite}`, `CreateNew`, `RefreshHashOnly`, `Preserve`, `Delete`, `DropManifestEntry`. Two consecutive partial-recovery runs from the same state produce byte-identical manifests and filesystem layouts.

---

## Runtime `runtime logic`

[**Main Flow**]

1. `main.rs` parses `UpgradeArgs`, builds `UpgradeOptions` (`ConflictPolicy` from the `ArgGroup`; default `Interactive`), constructs `StdioPrompter`, calls `ark_core::upgrade(opts, &mut prompter)`.
2. `upgrade` reads `Manifest` (errors `NotLoaded` if absent; `ManifestCorrupt` if malformed).
3. `validate_manifest_paths(&manifest.files)` — for every entry, `layout.resolve_safe(path)`. First violation → `UnsafeManifestPath { path, reason }`. No filesystem activity. **Safety check runs before any semantic check on the manifest contents.**
4. Parse `manifest.version` and `CARGO_PKG_VERSION` as `semver::Version`. If project > cli and `!allow_downgrade` → `DowngradeRefused`.
5. `collect_desired_templates` walks `ARK_TEMPLATES` and `CLAUDE_TEMPLATES` with the project-relative mapping per C-18. Each key is re-checked with `resolve_safe` for symmetry.
6. Build operating set: `manifest.files ∪ desired` minus `.ark/.installed.json`.
7. Classify desired files; classify removals for manifest files not in desired.
8. Resolve conflicts (`UserModified` + `AmbiguousNoHash-with-content-mismatch`) via `ConflictPolicy` or `prompter.prompt()`. `Unchanged{refresh_hash=false}` cases inline-bump `summary.unchanged` during planning.
9. `plan_actions` returns the `Vec<PlannedAction>` sorted by (bucket, relative_path) per C-19.
10. `apply_writes()`: execute `Write`, `CreateNew`, `RefreshHashOnly`, `Preserve` in sorted order. Each on-disk write via `PathExt::write_bytes`; each hash update mutates the in-memory manifest.
11. `update_managed_block(CLAUDE.md, "ARK", MANAGED_BLOCK_BODY)`. If freshly inserted, `manifest.record_block(...)`.
12. `manifest.version = CARGO_PKG_VERSION; manifest.installed_at = Utc::now(); manifest.write(layout.root())`. **Happens BEFORE deletions.**
13. `apply_deletions()`: `Delete` unlinks + `manifest.drop_file(path)`. `DropManifestEntry` leaves the file + `manifest.drop_file(path)`.
14. If any deletions mutated the manifest, `manifest.write()` once more.
15. Return `UpgradeSummary`. `main.rs` prints `Display`.

[**Failure Flow**]

1. `.ark/.installed.json` missing → `Error::NotLoaded { path }`. No files touched.
2. Manifest JSON-corrupt → `Error::ManifestCorrupt` propagates.
3. **Step 3 path validation fails:** `Error::UnsafeManifestPath { path, reason }`. No filesystem activity. User hand-repairs `.installed.json` before retry.
4. Project > CLI without `--allow-downgrade` → `Error::DowngradeRefused`. No files touched. (Path validation has already passed by the time we reach this step.)
5. **Mid-write failure (step 10):** `Error::Io`. Files written before the failure remain in their new state; on-disk manifest still reflects pre-upgrade version (step 12 hasn't run). Recovery on next upgrade: files whose on-disk bytes now match the embedded template classify as `Unchanged { refresh_hash: true }` and silently refresh the hash — no spurious prompts.
6. **Mid-delete failure (step 13):** the step-12 manifest write already persisted the new version + fresh hashes. Files successfully deleted are gone; subsequent files remain on disk with their manifest entries intact. Recovery: next upgrade re-classifies residuals as `SafeRemove` / `Orphaned`.
7. `update_managed_block` returns `ManagedBlockCorrupt` → propagate; writes before this step are correct for the new version — a valid partial upgrade.
8. `Prompter::prompt` returns `Err` → propagate; behavior identical to (5).
9. **Step 14 manifest write failure:** on-disk manifest still references now-deleted files. Next upgrade re-classifies those entries: file absent, they fall through to "not-desired, absent → `DropManifestEntry`" and are cleaned up. No user-visible state corruption.
10. `manifest.version` fails to parse as semver → treat as unknown; skip downgrade check; proceed as if same version.

[**State Transitions**]

Per-file state machine at upgrade time:

```
Desired (file in embedded template set):
  absent                                                 → Add                                → Write{Add}
  present, bytes == desired, recorded == sha(current)    → Unchanged{refresh_hash=false}      → (counter only, no PlannedAction)
  present, bytes == desired, recorded != sha(current)    → Unchanged{refresh_hash=true}       → RefreshHashOnly   // revert-to-template
  present, bytes == desired, recorded is None            → Unchanged{refresh_hash=true}       → RefreshHashOnly   // pre-hash install
  present, bytes != desired, recorded == sha(current)    → AutoUpdate                         → Write{AutoUpdate}
  present, bytes != desired, recorded != sha(current)    → UserModified                       → resolve → Write{Overwrite} | Preserve | CreateNew
  present, bytes != desired, recorded is None            → AmbiguousNoHash                    → resolve (same as above)

Not-desired (file in manifest.files only):
  present, recorded == sha(current)                      → SafeRemove                         → Delete
  present, recorded != sha(current) or None              → Orphaned                           → DropManifestEntry
  absent                                                 → (silent) manifest entry drop       → DropManifestEntry
```

---

## Implementation `split task into phases`

[**Phase 1 — Manifest + hash plumbing**]

1.1 Add deps to `ark-core/Cargo.toml`:
- `sha2 = "0.10"`
- `semver = "1"`

1.2 Extend `Manifest`:
- `hashes: BTreeMap<PathBuf, String>` with `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`.
- `record_file_with_hash`, `hash_for`, `clear_hash`, `drop_file`.

1.3 `PathExt::hash_sha256(&self)` + free `hash_bytes(contents: &[u8])` in `io/path_ext.rs`.

1.4 `init.rs::extract()` routes through `record_file_with_hash`.

1.5 Unit tests: V-UT-1..V-UT-5, V-UT-16.

[**Phase 2 — Upgrade command core**]

2.1 Create `ark-core/src/commands/upgrade.rs`:
- `UpgradeOptions`, `ConflictPolicy`, `ConflictChoice`, `Prompter`.
- `UpgradeSummary` + `Display`.
- `upgrade(opts, &mut dyn Prompter) -> Result<UpgradeSummary>`.
- Internal enums `Classification`, `RemovalClassification`, `PlannedAction`, `WriteKind`.
- Helpers: `collect_desired_templates`, `validate_manifest_paths`, `plan_actions`, `classify`, `classify_removal`, `resolve_conflict`, `apply_writes`, `apply_deletions`, `is_exempted`.

2.2 Error variants in `error.rs`:
- `Error::DowngradeRefused { project_version, cli_version }`.
- `Error::UnsafeManifestPath { path, reason }`.

2.3 Plan-then-apply pattern. `plan_actions` sorts per C-19 before returning.

2.4 Export from `lib.rs`.

2.5 Unit tests: V-UT-6..V-UT-15, V-UT-17, V-UT-18.

[**Phase 3 — CLI + interactive prompter + integration tests**]

3.1 `ark-cli/src/main.rs`:
- `Upgrade(UpgradeArgs)` variant with `ArgGroup` policy; `--allow-downgrade` outside the group.
- Derive `ConflictPolicy` from flags (default `Interactive`).
- `StdioPrompter` in the binary.

3.2 `StdioPrompter::prompt` parses: `o/O/y/Y` → Overwrite; `s/S/n/N/""` → Skip; `c/C` → CreateNew; else → Skip with stderr note.

3.3 Integration tests in `crates/ark-cli/tests/upgrade.rs`:
- V-IT-1: `fresh_install_then_upgrade_is_noop`.
- V-IT-2: `template_change_with_unmodified_file_auto_updates`.
- V-IT-3: `user_modified_force_overwrites`.
- V-IT-4: `user_modified_skip_preserves`.
- V-IT-5: `user_modified_create_new_writes_sidecar`.
- V-IT-6: `removed_template_unmodified_is_deleted`.
- V-IT-7: `removed_template_modified_is_orphaned`.
- V-IT-8: `ambiguous_no_hash_prompt_path`.
- V-IT-9: `user_authored_task_file_untouched`.
- V-IT-10: `cli_help_lists_upgrade` + `upgrade --help` shows all four flags.
- V-IT-11: `hashes_survive_unload_load_roundtrip`.
- V-IT-12: `managed_block_body_refreshed`.
- V-IT-13: `managed_block_reapplied_when_manifest_lacks_entry`.
- V-IT-14: `specs_index_md_round_trips_through_upgrade`.
- V-IT-15: `hash_backfill_after_same_content`.

---

## Trade-offs `ask reviewer for advice`

- T-1: Manifest extension vs sidecar — **A** per TR-1.
- T-2: TTY detection — **`std::io::IsTerminal`** per TR-2.
- T-3: Prompt UX — **per-file** per TR-3.
- T-4: `.installed.json` exempt per TR-4.
- T-5: `Unchanged { refresh_hash: bool }` split per TR-5 + R-005.
- T-6: Same-version upgrade is a full pass per TR-6.
- T-7: No CRLF normalization per TR-7 (NG-8).
- T-8: Two-write manifest (steps 12 + 14) per TR-8. R-017 adds step-14 failure-mode entry.
- T-9: `resolve_safe` on manifest paths per TR-9. Distinct `Error::UnsafeManifestPath` for error-chain clarity.

---

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `Manifest::record_file_with_hash` populates both `files` and `hashes`; idempotent.
- V-UT-2: `Manifest::clear_hash` removes from `hashes` only; `drop_file` removes from both.
- V-UT-3: `Manifest::hash_for` returns `None` for unknown paths.
- V-UT-4: `hash_bytes(b"hello")` equals the known SHA-256 hex.
- V-UT-5: `PathExt::hash_sha256` returns `Ok(None)` for a missing file.
- V-UT-6: `classify` returns `Add` for missing files.
- V-UT-7: `classify` returns `Unchanged{refresh_hash=false}` when on-disk == desired AND recorded hash matches.
- V-UT-8: `classify` returns `AutoUpdate` when recorded hash matches AND content differs.
- V-UT-9: `classify` returns `UserModified` when recorded hash mismatches AND content differs.
- V-UT-10: `classify` returns `AmbiguousNoHash` when no recorded hash AND content differs.
- V-UT-11: `classify_removal` returns `SafeRemove` when hash matches, `Orphaned` otherwise.
- V-UT-12: `is_exempted` returns true for `.ark/.installed.json`; false for `.ark/workflow.md`, `.claude/commands/ark/quick.md`, `.ark/specs/INDEX.md`.
- V-UT-13: `UpgradeSummary::Display` prints counters in fixed order with a version transition line, all zeros included.
- V-UT-14 *(extended)*: `ConflictPolicy` flag parsing:
  - Single policy flag → corresponding policy.
  - No policy flag → `Interactive`.
  - Any two policy flags together (`--force --skip-modified`, `--force --create-new`, `--skip-modified --create-new`) → clap rejects with "cannot be used with".
  - `--force --allow-downgrade`, `--skip-modified --allow-downgrade`, `--create-new --allow-downgrade` → parse successfully.
  - `--allow-downgrade` alone → `conflict_policy = Interactive, allow_downgrade = true`.
- V-UT-15: `classify` returns `Unchanged{refresh_hash=true}` when on-disk == desired but recorded hash is stale OR missing; after apply, manifest hash matches `sha256(desired)`.
- V-UT-16: `init_populates_manifest_hashes` — after `init`, every `manifest.files` entry has a matching `manifest.hashes` entry; each value equals `hash_bytes(file_contents)`.
- V-UT-17 *(new)*: `desired_template_keys_match_init_manifest_entries` — after `init`, `sorted(manifest.files) == sorted(collect_desired_templates().map(|(p,_)| p))`.
- V-UT-18 *(new)*: `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` — read `upgrade.rs` via `include_str!`; assert that neither `std::fs::` nor the literal `".ark/"` appears outside `//` line comments or `#[cfg(test)]` blocks. Locks in C-12 and C-13 mechanically instead of relying on VERIFY-phase grep.

[**Integration Tests**]

V-IT-1..V-IT-15 as listed in Phase 3.3.

[**Failure / Robustness Validation**]

- V-F-1: `missing_manifest_errors` — `Error::NotLoaded`.
- V-F-2: `downgrade_refused_without_flag` — `Error::DowngradeRefused`.
- V-F-3: `downgrade_allowed_with_flag` succeeds.
- V-F-4: `corrupt_manifest_errors` — `ManifestCorrupt` propagates.
- V-F-5: `write_failure_leaves_manifest_untouched` — simulate non-writable template dest; on-disk manifest `version` NOT bumped.
- V-F-6: `managed_block_corrupt_surfaced` — orphan `<!-- ARK:START -->`; `ManagedBlockCorrupt`.
- V-F-7: `non_semver_project_version_parses_as_unknown` — `version = "dev"` proceeds.
- V-F-8 *(extended per R-016)*: `partial_write_then_rerun_classifies_written_files_as_unchanged` — simulate mid-write failure; second run does NOT prompt for written files. Two consecutive runs from the same partial state produce byte-identical manifest + disk state.
- V-F-9 *(new)*: `manifest_entry_outside_project_root_is_rejected` — inject `../escape.md` into `.installed.json.files`; upgrade returns `UnsafeManifestPath { path, reason }`; no filesystem activity outside project root (verified via `tempdir()` + post-run directory scan).

[**Edge Case Validation**]

- V-E-1: `same_version_upgrade_is_noop`.
- V-E-2: `empty_project_root_fails` — no `.ark/` → `NotLoaded`.
- V-E-3: `user_edit_reverts_to_new_template_is_unchanged` — bytes match desired + stale recorded hash → `Unchanged{refresh_hash=true}`; `unchanged` counter + 1; manifest hash refreshed.
- V-E-4: `non_tty_default_prompter_skips` — stub prompter + default policy preserves modified files without reading stdin.
- V-E-5: `new_files_do_not_get_hashed` — `--create-new`, rerun upgrade, `.new` file NOT tracked.
- V-E-6: `empty_template_set_manifest_roundtrips`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1  | V-IT-10, V-UT-14 |
| G-2  | V-UT-1, V-UT-4, V-UT-5 |
| G-3  | V-IT-2, V-UT-8 |
| G-4  | V-IT-3, V-IT-4, V-IT-5 |
| G-5  | V-E-3, V-IT-4 |
| G-6  | V-IT-6, V-IT-7, V-UT-11 |
| G-7  | V-IT-15, V-F-7, V-UT-15 |
| G-8  | V-F-1, V-F-2, V-F-3, V-F-9 |
| G-9  | V-IT-9, V-UT-12, V-IT-14 |
| G-10 | V-IT-12, V-IT-13, V-F-6 |
| G-11 | V-UT-13 |
| C-1  | V-UT-4 |
| C-2  | V-UT-1, V-UT-16 |
| C-3  | V-IT-9, V-IT-14, V-UT-12 |
| C-4  | V-F-1 |
| C-5  | V-F-2, V-F-3 |
| C-6  | V-E-1, V-F-7 |
| C-7  | V-E-4 |
| C-8  | V-IT-12, V-IT-13, V-F-6 |
| C-9  | V-E-5 |
| C-10 | V-IT-6, V-IT-7 |
| C-11 | V-IT-8, V-UT-15 |
| C-12 | V-UT-18 |
| C-13 | V-UT-18 |
| C-14 | V-UT-13 |
| C-15 | review-only (compiler rejects violation) |
| C-16 | documented; ack in VERIFY |
| C-17 | V-F-9 |
| C-18 | V-UT-17 |
| C-19 | V-F-8 (determinism half) |
