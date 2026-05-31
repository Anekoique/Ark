# `ark-upgrade` PLAN `01`

> Status: Draft
> Feature: `ark-upgrade`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Iteration 01 resolves the three blocking findings and nine non-blocking findings from `00_REVIEW.md`, and amends the PRD to match the PLAN's manifest-embedded hash design. The architecture is unchanged: extend `Manifest` with a `hashes` map, classify each desired template against on-disk content + recorded hash, resolve conflicts through an injectable `Prompter`. Key revisions: (a) the blanket `.ark/specs/` protected-path filter is dropped — upgrade now only acts on the union of `manifest.files` and the embedded template set, with `.ark/.installed.json` as the single file-level exemption; (b) `Classification::Unchanged` gains an internal `refresh_hash: bool` discriminator so the "user reverted to template" and "pre-hash install" cases can update the stored hash without a content rewrite; (c) `load.rs` is left untouched — snapshot round-trip already preserves hashes because the manifest is itself snapshotted; (d) CLI flags migrate to a clap `ArgGroup`; (e) `Prompter` drops the `Send` bound.

## Log

[**Added**]

- `Classification::Unchanged { refresh_hash: bool }` — internal discriminator routing the "revert-to-template" and "pre-hash-install-with-matching-content" cases into a hash-only write.
- `PlannedAction::RefreshHashOnly` — applied variant that writes to the manifest but not to disk.
- `V-UT-15` — asserts hash is refreshed when content matches desired but recorded hash is stale.
- `V-UT-16` — asserts `init` populates `manifest.hashes` for every file in `manifest.files`.
- `V-IT-12` — `managed_block_body_refreshed` happy-path coverage for G-10.
- `V-IT-13` — `managed_block_reapplied_when_manifest_lacks_entry`.
- `V-IT-14` — `specs_index_md_round_trips_through_upgrade` — explicitly verifies the shipped INDEX.md templates are upgradable.
- `V-F-8` — `partial_write_then_rerun_classifies_written_files_as_unchanged`; documents expected recovery.
- `NG-8` — no CRLF normalization.
- `C-15` — `Prompter` is dyn-compatible; no generic methods or `Self: Sized` bounds.
- `C-16` — upgrade is not safe against concurrent file modification by other processes.

[**Changed**]

- C-3 rewritten to drop the blanket `.ark/specs/` prefix filter. New wording: "Upgrade only acts on paths in `manifest.files ∪ desired_templates`; `.ark/.installed.json` is the sole file-level exemption." User-authored paths are out of scope automatically because they appear in neither set.
- G-9 rewritten to match C-3.
- `Prompter` trait signature loses the `: Send` bound.
- `UpgradeArgs` migrates from pairwise `conflicts_with_all` to a single `ArgGroup { id = "policy", multiple = false }` with `#[arg(long, group = "policy")]` on each policy variant.
- T-2 / Phase 1.1: removed `is-terminal` from the dependency add list — `std::io::IsTerminal` is in std since Rust 1.70, and the crate targets edition 2024. Only `sha2 = "0.10"` and `semver = "1"` are added to `ark-core`.
- Phase 1.5 replaced: `load.rs` is no longer modified. The snapshot captures `.ark/.installed.json` verbatim; restore writes the manifest byte-for-byte, so `manifest.hashes` survives automatically. V-IT-11 is retained, reworded to verify the preserved-by-snapshot behavior directly.
- `Classification::Unchanged` is now `Unchanged { refresh_hash: bool }` (internal only; `UpgradeSummary.unchanged` counter semantics unchanged).
- Runtime Main Flow step 10 sequence: manifest write now happens BEFORE deletions (per R-004). Files we added/updated have their hashes persisted even if a later delete fails.
- PRD `[**Outcome**]` section rewritten: removes `.ark/.hashes.json` and `.ark/.version` sidecar claims; states hashes and version live inside the existing manifest.

[**Removed**]

- Phase 1.5's "compute hashes at restore time" (unnecessary; snapshot preserves hashes for free — R-007).
- `is-terminal` crate dependency from Phase 1.1 (TR-2 adopted cleanly).
- `Send` bound on `Prompter` (R-003).
- Blanket `.ark/specs/` prefix filter (R-001).

[**Unresolved**]

- None. All blocking and non-blocking findings have explicit resolutions in the Response Matrix.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Adopted reviewer's recommendation option 3. C-3 and G-9 rewritten; protected set is `manifest.files ∪ desired_templates` minus `.ark/.installed.json`. Drops the `.ark/specs/` prefix filter. Added V-IT-14 covering shipped INDEX.md round-trip. |
| Review | R-002 | Accepted | PRD amended in-place: removed sidecar-file claims; now explicitly describes manifest-embedded hashes and version. |
| Review | R-003 | Accepted | `Prompter` loses `: Send`. Added C-15 naming the dyn-compatibility invariant. |
| Review | R-004 | Accepted | Failure Flow step 4 rewritten with the recovery story. Main Flow reordered so manifest write happens before deletions. Added V-F-8 covering partial-failure rerun behavior. |
| Review | R-005 | Accepted | `Classification::Unchanged { refresh_hash: bool }` added with corresponding `PlannedAction::RefreshHashOnly`. Added V-UT-15. |
| Review | R-006 | Accepted | `UpgradeArgs` switches to `ArgGroup { multiple = false }`. V-UT-14 covers the rejection case. |
| Review | R-007 | Accepted | Phase 1.5 dropped. V-IT-11 reworded to verify snapshot-preserved hash behavior (no `load.rs` changes). |
| Review | R-008 | Accepted | Added V-IT-12 and V-IT-13. G-10 Acceptance Mapping updated to cite them. |
| Review | R-009 | Rejected | Moot under R-001 option 3 (no prefix filter exists). The "filter" is an equality check on paths that `manifest.files` and `include_dir!` walking already produce in a normalized form. No separator/case code needed. |
| Review | R-010 | Accepted | Folds into R-005 fix — `AmbiguousNoHash`-with-content-match becomes `Unchanged{refresh_hash=true}` before action resolution. V-IT-15 (`hash_backfill_after_same_content`) wires it up. |
| Review | R-011 | Accepted | Added C-16. Concurrent-editor safety is documented as out of scope; no lock/retry. |
| Review | R-012 | Accepted | Added V-UT-16: `init_populates_manifest_hashes`. |
| Review | TR-1 | Accepted | Option A confirmed (manifest extension). PRD amendment resolves the reviewer's caveat. |
| Review | TR-2 | Accepted | `std::io::IsTerminal` only; no new crate dep. Phase 1.1 dep list trimmed. |
| Review | TR-3 | Accepted | Per-file prompt. No change. |
| Review | TR-4 | Accepted | `.ark/.installed.json` exempt. No change. |
| Review | TR-5 | Accepted | Folded into R-005 fix. Public counter semantics unchanged; internal action type discriminates. |
| Review | TR-6 | Accepted | Same-version upgrade remains a full pass. V-E-1 keeps the lock. |
| Review | TR-7 | Accepted | Added NG-8. Ship without normalization; revisit on user reports. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec `Core specification`

[**Goals**]

- G-1: `ark upgrade` is a top-level, visible subcommand (not hidden behind `ark agent`). It refreshes embedded-template content in an already-initialized project and is safe to run repeatedly.
- G-2: User-modified files are detected by SHA-256 content hashing. `Manifest` records the hash of every file Ark writes; upgrade compares current on-disk content to the recorded hash. Match = user hasn't touched it; mismatch = user modified it.
- G-3: When a template file's content changes between versions AND the user has not modified it locally, upgrade silently rewrites it and records the new hash.
- G-4: When a template file changes AND the user has modified it, upgrade prompts (overwrite / skip / write `.new`). Non-TTY environments and the three flag overrides resolve conflicts non-interactively.
- G-5: Files the user has modified but which did not change in the template set are left alone and reported as "preserved".
- G-6: Files that were in the old template set but no longer appear in the new set are deleted IF their current hash matches the stored hash (unmodified). User-modified disappeared templates are listed as "orphaned" and left for the user.
- G-7: A fresh `init` records hashes at write time. `upgrade` on a pre-hash-tracking project (`Manifest.hashes` empty) backfills hashes by comparing current content to the embedded template — exact match = record the hash, mismatch = treat as user-modified.
- G-8: Upgrade refuses with `Error::NotLoaded` when `.ark/.installed.json` is missing, and with `Error::DowngradeRefused` when CLI version < project version (unless `--allow-downgrade`).
- G-9: The only file-level protection is `.ark/.installed.json`, which upgrade never touches directly. The broader "don't clobber user content" invariant is satisfied by upgrade acting only on `manifest.files ∪ desired_templates`. User-authored paths (`.ark/tasks/**`, `.ark/specs/features/<slug>/**`, `.ark/specs/project/<name>/**`) are automatically safe because they are in neither set.
- G-10: The `CLAUDE.md` managed block is re-applied with the latest body via `update_managed_block` on every upgrade. The managed block is not hash-tracked; its contents are governed by markers.
- G-11: Upgrade prints a `Display` one-line header plus a structured multi-line summary: `{N} file(s): {A} added · {U} updated · {S} unchanged · {M} modified-preserved · {O} overwritten · {K} skipped · {C} .new-copied · {D} deleted · {R} orphaned`, followed by the version transition `{prev} -> {curr}`.

- NG-1: No migration manifest system (no structural renames/deletes across versions). Deferred.
- NG-2: No network I/O.
- NG-3: No backup directory.
- NG-4: No recursive directory rename logic.
- NG-5: No config file controlling which files to skip. Deferred.
- NG-6: No `.version` sidecar file. The manifest already carries `version`.
- NG-7: No automatic invocation — upgrade is a user-run command.
- NG-8: No CRLF/LF normalization before hashing. Documented; workaround is `git config core.autocrlf false`.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                     — adds `Upgrade(UpgradeArgs)` top-level
└── ark-core/src/
    ├── lib.rs                               — re-exports upgrade public API
    ├── error.rs                             — adds Error::DowngradeRefused
    ├── io/path_ext.rs                       — adds `hash_sha256()` + free `hash_bytes`
    ├── state/manifest.rs                    — adds `hashes: BTreeMap<PathBuf,String>`
    │                                          + `record_file_with_hash`, `hash_for`,
    │                                          `clear_hash`, `drop_file`
    ├── commands/
    │   ├── init.rs                          — records hashes when writing files
    │   └── upgrade.rs                       — the new command
    └── templates.rs                          — unchanged (just re-consumed)
```

**Call graph for `upgrade` (order revised per R-004):**

```
upgrade(opts, prompter)
  ├── Manifest::read → Error::NotLoaded if missing
  ├── check_version (semver cmp) → Error::DowngradeRefused if project > cli and !allow_downgrade
  ├── collect_desired_templates()              → Vec<(PathBuf, &'static [u8])>
  ├── plan_actions()                            → Vec<PlannedAction>
  │     per desired file:  classify → Add | Unchanged{refresh} | AutoUpdate | UserModified | AmbiguousNoHash
  │     per manifest file not in desired:       classify_removal → SafeRemove | Orphaned
  │     resolve(UserModified | AmbiguousNoHash-with-content-mismatch) via policy or prompter
  ├── apply_writes()                            (Adds, AutoUpdates, Overwrites, CreateNews, RefreshHashOnly)
  │     — mutates manifest in-memory
  ├── update_managed_block(CLAUDE.md, "ARK", MANAGED_BLOCK_BODY)
  │     — records block in manifest if newly inserted
  ├── manifest.version = CARGO_PKG_VERSION; manifest.installed_at = now; manifest.write()
  │     ^ durable BEFORE deletions
  ├── apply_deletions()                         (SafeRemove unlinks; Orphaned leaves file, drops entry)
  ├── manifest.write() again if deletions changed the manifest
  └── UpgradeSummary
```

**Module coupling.** `commands/upgrade.rs` imports from `crate::{layout, io, state::Manifest, templates}` — same shape as `commands/init.rs`. No agent-namespace coupling.

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
    pub fn drop_file(&mut self, path: &Path);   // removes from both `files` and `hashes`
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
pub enum ConflictPolicy {
    Interactive,
    Force,
    Skip,
    CreateNew,
}

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

enum PlannedAction {
    Write { relative: PathBuf, contents: &'static [u8], kind: WriteKind },
    RefreshHashOnly { relative: PathBuf, contents: Vec<u8> },     // on-disk bytes
    CreateNew { relative: PathBuf, contents: &'static [u8] },     // writes <path>.new
    Delete { relative: PathBuf },
    DropManifestEntry { relative: PathBuf },                      // orphaned
    Preserve { relative: PathBuf },                               // counter only
}

enum WriteKind { Add, AutoUpdate, Overwrite }
```

New error variant:

```rust
// ark-core/src/error.rs
Error::DowngradeRefused { project_version: String, cli_version: String },
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

CLI (in `ark-cli/src/main.rs`):

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

Stdio prompter lives in the binary crate:

```rust
struct StdioPrompter;   // uses std::io::IsTerminal; non-TTY → Skip
```

[**Constraints**]

- C-1: Hashes are SHA-256, hex-encoded lowercase. Stored under `manifest.hashes` using project-relative `PathBuf` keys matching `manifest.files` entries exactly.
- C-2: Every `init` write routes through a helper that records file AND hash. No write path can update `files` without also updating `hashes`.
- C-3: Upgrade only acts on paths in `manifest.files ∪ desired_templates`. `.ark/.installed.json` is the sole file-level exemption. No prefix filter is applied; user-authored paths under `.ark/tasks/**`, `.ark/specs/features/<slug>/**`, and `.ark/specs/project/<name>/**` are automatically untouched because they are in neither set.
- C-4: Upgrade refuses with `Error::NotLoaded` when the manifest file is absent.
- C-5: Upgrade refuses with `Error::DowngradeRefused` when `semver::Version::parse(&manifest.version) > semver::Version::parse(CARGO_PKG_VERSION)`, unless `opts.allow_downgrade` is true. If `manifest.version` fails to parse, treat as unknown and proceed.
- C-6: Version comparison uses `semver::Version`. Same-version upgrades are a legal full pass.
- C-7: In `ConflictPolicy::Interactive`, the CLI binary constructs a `StdioPrompter` that reads one line from stdin per conflict. Non-TTY stdin → `ConflictChoice::Skip` without reading, with a single stderr note emitted at upgrade start. Library callers pass their own `Prompter`.
- C-8: The `CLAUDE.md` managed block is re-applied on every upgrade via `update_managed_block` with `MANAGED_BLOCK_BODY`. Not hash-tracked.
- C-9: `.new` files are NOT recorded in the manifest and NOT hashed.
- C-10: When a template file has been removed between versions, upgrade deletes the on-disk file only if `manifest.hash_for(path) == Some(current_sha256)`. Otherwise the file is left in place (orphaned). Either way, the file's entry is dropped from `manifest.files` and `manifest.hashes`.
- C-11: `AmbiguousNoHash` (no recorded hash + on-disk content differs from desired) is treated as `UserModified` for conflict resolution. The case "no recorded hash + on-disk matches desired" is `Classification::Unchanged { refresh_hash: true }`.
- C-12: All filesystem access in `commands/upgrade.rs` routes through `io::PathExt` / `io::fs` helpers. No bare `std::fs::*`.
- C-13: All path composition in `upgrade.rs` routes through `layout::Layout`. No hand-joined `.ark/...` string literals.
- C-14: `UpgradeSummary::Display` output is deterministic: counters always printed in the fixed order above, version transition always printed as a second line, even when all counts are zero.
- C-15: `Prompter` is dyn-compatible. Do not add generic methods or `Self: Sized` bounds; the library passes `&mut dyn Prompter`.
- C-16: Upgrade is not safe against concurrent file modification by other processes (editors, other Ark invocations). No locking is attempted; users close editors before running upgrade.

---

## Runtime `runtime logic`

[**Main Flow**]

1. `main.rs` parses `UpgradeArgs`, builds `UpgradeOptions` (`ConflictPolicy` derived from the `ArgGroup`; default `Interactive`), constructs `StdioPrompter`, calls `ark_core::upgrade(opts, &mut prompter)`.
2. `upgrade` reads `Manifest` (errors `NotLoaded` if absent).
3. Parse `manifest.version` and `CARGO_PKG_VERSION` as `semver::Version`; if project > cli and `!allow_downgrade` → `DowngradeRefused`.
4. Enumerate desired templates by walking `ARK_TEMPLATES` and `CLAUDE_TEMPLATES` (same as `init`), yielding `Vec<(PathBuf, &'static [u8])>`.
5. Build the operating set: `manifest.files ∪ desired` minus `.ark/.installed.json`.
6. For each desired file, classify; for each manifest file not in desired, classify removal.
7. Resolve conflicts (`UserModified` + `AmbiguousNoHash-with-content-mismatch`) via `ConflictPolicy` or `prompter.prompt()` → `PlannedAction`.
8. `apply_writes()`: execute `Add`, `AutoUpdate`, `Overwrite`, `CreateNew`, `RefreshHashOnly` in order. Each on-disk write goes through `PathExt::write_bytes`; each `record_file_with_hash` mutates the in-memory manifest.
9. `update_managed_block(CLAUDE.md, "ARK", MANAGED_BLOCK_BODY)`. If freshly inserted, `manifest.record_block(...)`.
10. `manifest.version = CARGO_PKG_VERSION; manifest.installed_at = Utc::now(); manifest.write(layout.root())`. **Write happens BEFORE deletions so fresh hashes are durable even if a delete fails.**
11. `apply_deletions()`: `SafeRemove` unlinks the file and `manifest.drop_file(path)`. `Orphaned` leaves the file and `manifest.drop_file(path)`.
12. If any deletions ran, `manifest.write()` once more.
13. Return `UpgradeSummary`. `main.rs` prints `Display`.

[**Failure Flow**]

1. `.ark/.installed.json` missing → `Error::NotLoaded { path }`. No files touched.
2. Manifest present but JSON-corrupt → existing `Error::ManifestCorrupt` propagates.
3. Project version > CLI version without `--allow-downgrade` → `Error::DowngradeRefused`. No files touched.
4. **Mid-write failure (step 8):** returns `Error::Io`. On-disk state: files written before the failure remain in their new state; the manifest file on disk still reflects the pre-upgrade version (step 10 hasn't run). Recovery on next upgrade: files whose on-disk bytes now match the embedded template are classified `Unchanged { refresh_hash: true }` because the match-desired branch takes precedence regardless of stale recorded hash. The next upgrade writes the new hash, counts them as `unchanged`, and proceeds — no spurious prompts.
5. **Mid-delete failure (step 11):** the manifest has already been written with the new version and hashes (step 10). Files successfully deleted before the failure are gone; subsequent files remain on disk with their manifest entries intact (the mutation for failed/subsequent entries hasn't happened). Recovery: next upgrade re-classifies residual files as `SafeRemove` / `Orphaned` again and retries.
6. `update_managed_block` returns `ManagedBlockCorrupt` → propagate; file writes before this step are correct for the new version already and are a valid partial update. Next upgrade picks up from there.
7. `Prompter::prompt` returns `Err` → propagate; behavior identical to (4).
8. `manifest.version` fails to parse as semver (e.g., "dev", "0.1-dirty") → treat as unknown; skip downgrade check; proceed as if same version. `UpgradeSummary.version_from` carries the original string.

[**State Transitions**]

Per-file state machine at upgrade time:

```
Desired (file in embedded template set):
  absent                                                → Add
  present, bytes == desired, recorded == sha(current)   → Unchanged{refresh_hash=false}
  present, bytes == desired, recorded != sha(current)   → Unchanged{refresh_hash=true}   // revert-to-template
  present, bytes == desired, recorded is None           → Unchanged{refresh_hash=true}   // pre-hash install
  present, bytes != desired, recorded == sha(current)   → AutoUpdate
  present, bytes != desired, recorded != sha(current)   → UserModified                    → resolve
  present, bytes != desired, recorded is None           → AmbiguousNoHash                 → resolve

Not-desired (file in manifest.files only):
  present, recorded == sha(current)                     → SafeRemove
  present, recorded != sha(current) or None             → Orphaned
  absent                                                → (silent) drop manifest entry
```

`Preserve` (counter `modified_preserved`) is the result of resolving `UserModified` via `Skip` — no manifest mutation.

---

## Implementation `split task into phases`

[**Phase 1 — Manifest + hash plumbing**]

1.1 Add deps to `ark-core/Cargo.toml`:
- `sha2 = "0.10"`
- `semver = "1"`

  Nothing added to `ark-cli/Cargo.toml`.

1.2 Extend `Manifest` (`ark-core/src/state/manifest.rs`):
- Add `hashes: BTreeMap<PathBuf, String>` with `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`.
- `record_file_with_hash(&mut self, path, contents)` — also calls `record_file`.
- `hash_for(&self, path) -> Option<&str>`.
- `clear_hash(&mut self, path)` — removes from `hashes` only.
- `drop_file(&mut self, path)` — removes from `files` AND `hashes`.
- Keep `record_file` (no behavioral change).

1.3 Add `PathExt::hash_sha256(&self) -> Result<Option<String>>` and free `hash_bytes(contents: &[u8]) -> String` in `io/path_ext.rs`. Both hex-lowercase.

1.4 Update `init.rs`'s `extract()` helper to call `manifest.record_file_with_hash(relative, entry.contents)` instead of `record_file`. This is the only `init` write path.

1.5 *(removed per R-007)* `load.rs` is not modified. Snapshot restore writes the manifest verbatim, which naturally preserves `manifest.hashes`. Verified by V-IT-11.

1.6 Unit tests: V-UT-1, V-UT-2, V-UT-3, V-UT-4, V-UT-5, V-UT-16.

[**Phase 2 — Upgrade command core**]

2.1 Create `ark-core/src/commands/upgrade.rs`:
- `UpgradeOptions`, `ConflictPolicy`, `ConflictChoice`, `Prompter` (no `Send`).
- `UpgradeSummary` + `Display` (fixed order per C-14).
- `upgrade(opts, &mut dyn Prompter) -> Result<UpgradeSummary>`.
- Internal enums `Classification`, `RemovalClassification`, `PlannedAction`, `WriteKind`.
- Helpers: `collect_desired`, `plan_actions`, `classify`, `classify_removal`, `resolve_conflict`, `apply_writes`, `apply_deletions`.
- `is_exempted(path: &Path) -> bool` — single-file equality against `MANIFEST_RELATIVE_PATH`. No prefix logic.

2.2 Error variant `Error::DowngradeRefused { project_version, cli_version }` in `error.rs`.

2.3 Plan-then-apply pattern: `plan_actions()` produces `Vec<PlannedAction>` with no side effects except reading files to hash. `apply_writes` / `apply_deletions` execute actions and mutate the manifest.

2.4 Export from `lib.rs` (re-export list above).

2.5 Unit tests: V-UT-6..V-UT-15.

[**Phase 3 — CLI + interactive prompter + integration tests**]

3.1 In `ark-cli/src/main.rs`:
- Add `Upgrade(UpgradeArgs)` variant with `ArgGroup` policy.
- Derive `ConflictPolicy` from flags (default `Interactive`).
- Implement `StdioPrompter` in the binary crate. Uses `std::io::IsTerminal`; non-TTY → `Skip` with a stderr note emitted at upgrade start.

3.2 `StdioPrompter::prompt` parses: `o`/`O`/`y`/`Y` → Overwrite; `s`/`S`/`n`/`N`/"" → Skip; `c`/`C` → CreateNew; anything else → Skip with a stderr note.

3.3 Integration tests in `crates/ark-cli/tests/upgrade.rs`:
- V-IT-1: `fresh_install_then_upgrade_is_noop`.
- V-IT-2: `template_change_with_unmodified_file_auto_updates`.
- V-IT-3: `user_modified_force_overwrites`.
- V-IT-4: `user_modified_skip_preserves`.
- V-IT-5: `user_modified_create_new_writes_sidecar`.
- V-IT-6: `removed_template_unmodified_is_deleted` (inject a fake manifest entry).
- V-IT-7: `removed_template_modified_is_orphaned`.
- V-IT-8: `ambiguous_no_hash_prompt_path` (stub prompter returns Skip).
- V-IT-9: `user_authored_task_file_untouched`.
- V-IT-10: `cli_help_lists_upgrade` + `upgrade --help` shows four flags.
- V-IT-11: `hashes_survive_unload_load_roundtrip`.
- V-IT-12: `managed_block_body_refreshed`.
- V-IT-13: `managed_block_reapplied_when_manifest_lacks_entry`.
- V-IT-14: `specs_index_md_round_trips_through_upgrade`.
- V-IT-15: `hash_backfill_after_same_content`.

---

## Trade-offs `ask reviewer for advice`

- T-1: Manifest extension (A) vs sidecar file (B). **Adopted A** per TR-1.
- T-2: TTY detection. **Adopted `std::io::IsTerminal`** per TR-2 — no new dep.
- T-3: Prompt UX. **Adopted per-file prompts** per TR-3.
- T-4: `.installed.json` exempt. **Adopted** per TR-4.
- T-5: `Unchanged` internals. **Adopted split** per TR-5 + R-005.
- T-6: Same-version upgrade runs a full pass per TR-6.
- T-7: CRLF normalization — **ship without** per TR-7. Added NG-8.
- T-8 (noted): `ConflictPolicy::Interactive` is the only variant that reaches `Prompter::prompt`; others short-circuit. Covered implicitly by V-IT-3..V-IT-5 (non-interactive paths) and V-IT-8 (interactive stub). No new reviewer question.

---

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `Manifest::record_file_with_hash` populates both `files` and `hashes`; idempotent under repeat calls.
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
- V-UT-12: `is_exempted` returns true for `.ark/.installed.json`; false for `.ark/workflow.md`, `.claude/commands/ark/quick.md`, and `.ark/specs/INDEX.md`.
- V-UT-13: `UpgradeSummary::Display` prints all counters in fixed order with a version transition line, even when all counts are zero.
- V-UT-14: `ConflictPolicy` flag parsing: single-flag → policy; two policy flags together → clap rejects with an "cannot be used with" error.
- V-UT-15 *(new)*: `classify` returns `Unchanged{refresh_hash=true}` when on-disk == desired but recorded hash is stale OR missing; after apply, manifest has `hash == sha256(desired)`.
- V-UT-16 *(new)*: `init_populates_manifest_hashes` — after `init`, every entry in `manifest.files` has a matching entry in `manifest.hashes`, and each value equals `hash_bytes(file_contents)`.

[**Integration Tests**]

V-IT-1..V-IT-15 as listed in Phase 3.3.

[**Failure / Robustness Validation**]

- V-F-1: `missing_manifest_errors` — `Error::NotLoaded`.
- V-F-2: `downgrade_refused_without_flag` — `Error::DowngradeRefused`.
- V-F-3: `downgrade_allowed_with_flag` succeeds.
- V-F-4: `corrupt_manifest_errors` — existing `ManifestCorrupt` propagates.
- V-F-5: `write_failure_leaves_manifest_untouched` — simulate a non-writable template dest; verify the on-disk manifest's `version` is NOT bumped.
- V-F-6: `managed_block_corrupt_surfaced` — orphan `<!-- ARK:START -->` in CLAUDE.md; `ManagedBlockCorrupt`.
- V-F-7: `non_semver_project_version_parses_as_unknown` — `manifest.version = "dev"`; upgrade proceeds.
- V-F-8 *(new)*: `partial_write_then_rerun_classifies_written_files_as_unchanged` — simulate a write failure mid-pass; verify the next upgrade does NOT prompt for the successfully-written files.

[**Edge Case Validation**]

- V-E-1: `same_version_upgrade_is_noop`.
- V-E-2: `empty_project_root_fails` — no `.ark/` at all → `NotLoaded`.
- V-E-3: `user_edit_reverts_to_new_template_is_unchanged` — bytes match desired with stale recorded hash → `Unchanged{refresh_hash=true}`; counter `unchanged` + 1; manifest hash refreshed.
- V-E-4: `non_tty_default_prompter_skips` — stub prompter + non-interactive default preserves modified files.
- V-E-5: `new_files_do_not_get_hashed` — `--create-new`, rerun upgrade, `.new` file is NOT tracked.
- V-E-6: `empty_template_set_manifest_roundtrips` — with an all-removed template set (fixture), upgrade writes a manifest with empty `files` and empty `hashes`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-10, V-UT-14 |
| G-2 | V-UT-1, V-UT-4, V-UT-5 |
| G-3 | V-IT-2, V-UT-8 |
| G-4 | V-IT-3, V-IT-4, V-IT-5 |
| G-5 | V-E-3, V-IT-4 |
| G-6 | V-IT-6, V-IT-7, V-UT-11 |
| G-7 | V-IT-15, V-F-7, V-UT-15 |
| G-8 | V-F-1, V-F-2, V-F-3 |
| G-9 | V-IT-9, V-UT-12, V-IT-14 |
| G-10 | V-IT-12, V-IT-13 |
| G-11 | V-UT-13 |
| C-1 | V-UT-4 |
| C-2 | V-UT-1, V-UT-16 |
| C-3 | V-IT-9, V-IT-14, V-UT-12 |
| C-4 | V-F-1 |
| C-5 | V-F-2, V-F-3 |
| C-6 | V-E-1, V-F-7 |
| C-7 | V-E-4 |
| C-8 | V-IT-12, V-IT-13 |
| C-9 | V-E-5 |
| C-10 | V-IT-6, V-IT-7 |
| C-11 | V-IT-8, V-UT-15 |
| C-12 | review-only (grep `std::fs::` in upgrade.rs during VERIFY) |
| C-13 | review-only (grep `.ark/` literals in upgrade.rs during VERIFY) |
| C-14 | V-UT-13 |
| C-15 | review-only (compiler rejects violation) |
| C-16 | documented; ack in VERIFY |
