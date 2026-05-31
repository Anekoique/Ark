# `ark-upgrade` PLAN `00`

> Status: Draft
> Feature: `ark-upgrade`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Add `ark upgrade` — a top-level subcommand that re-applies Ark's embedded templates to an already-initialized project, using SHA-256 content hashes recorded in the existing `.ark/.installed.json` manifest to distinguish user-modified files from template-refreshed files. Modified files produce an interactive prompt (overwrite / skip / write `.new`), with `--force` / `--skip-modified` / `--create-new` flags for non-interactive operation. Downgrades are refused unless `--allow-downgrade`. Protected user paths (`.ark/tasks/`, `.ark/specs/`, the manifest itself) are never touched. Migrations (structural renames/deletes across versions) are deferred.

## Log `None in 00_PLAN`

---

## Spec `Core specification`

[**Goals**]

- G-1: `ark upgrade` is a top-level, visible subcommand (not hidden behind `ark agent`). It refreshes embedded-template content in an already-initialized project and is safe to run repeatedly.
- G-2: User-modified files are detected by SHA-256 hashing. The `Manifest` records the hash of every file Ark writes (at `init`, at `upgrade`, or at `load`-restored state); upgrade compares the current on-disk content to the recorded hash. Match = user hasn't touched it; mismatch = user modified it.
- G-3: When a template file's content changes between versions AND the user has not modified it locally, upgrade silently rewrites it and records the new hash.
- G-4: When a template file changes AND the user has modified it, upgrade prompts (overwrite / skip / write `.new`). Non-TTY environments and the three flag overrides resolve the conflict non-interactively.
- G-5: Files the user has modified but which did not change in the template set are left alone and reported as "preserved".
- G-6: Files that were in the old template set but no longer appear in the new set (removed templates) are deleted IF their current hash matches the stored hash (unmodified). User-modified disappeared templates are listed as "orphaned" and left for the user.
- G-7: A fresh `init` (on a project with no `.ark/`) records hashes at write time. `upgrade` on a pre-hash-tracking project (`Manifest.hashes` empty) is safe: it backfills hashes by comparing current content to the embedded template — exact match = record the hash and refresh, mismatch = treat as user-modified.
- G-8: Upgrade refuses with `Error::NotLoaded` when `.ark/.installed.json` is missing, and with `Error::DowngradeRefused` when CLI version < project version (unless `--allow-downgrade`).
- G-9: Protected paths — `.ark/tasks/`, `.ark/specs/`, and `.ark/.installed.json` itself — are never candidates for upgrade, regardless of whether the template set contains entries under them.
- G-10: The `CLAUDE.md` managed block is re-applied with the latest body via `update_managed_block` on every upgrade. The managed block is not hash-tracked; its contents are governed by markers, not byte-identity.
- G-11: Upgrade prints a `Display` one-line header plus a structured multi-line summary: `{N} file(s): {A} added · {U} updated · {S} unchanged · {M} modified-preserved · {O} overwritten · {K} skipped · {C} .new-copied · {D} deleted · {R} orphaned`, followed by the version transition `{prev} -> {curr}`.

- NG-1: No migration manifest system (no structural renames/deletes across versions). Deferred; a later task introduces it when the first rename lands.
- NG-2: No network I/O. Does not check npm/crates.io for newer CLI versions — this is purely a local refresh of templates against whatever CLI is installed.
- NG-3: No backup directory. The manifest stores hashes so the user's modifications are detectable, and the user has git; creating timestamped backup directories adds clutter without adding recoverability.
- NG-4: No recursive directory rename logic. The template set is a flat list of files; the only "directory operations" are creating parents on write.
- NG-5: No config file controlling which files to skip (`update.skip` in Trellis). Deferred — if users need per-file pinning, a follow-up task adds it.
- NG-6: No `.version` sidecar file. The manifest already carries `version`; the header Display just reads `CARGO_PKG_VERSION` and the manifest version.
- NG-7: No automatic invocation — upgrade is a user-run command, never triggered by other Ark subcommands.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                     — adds `Upgrade(UpgradeArgs)` top-level
└── ark-core/src/
    ├── lib.rs                               — re-exports upgrade public API
    ├── error.rs                             — new variants (see Data Structure)
    ├── io/path_ext.rs                       — adds `hash_sha256()` helper
    ├── state/manifest.rs                    — adds `hashes: BTreeMap<PathBuf,String>`
    │                                          + `record_file_with_hash`, `hash_for`,
    │                                          `clear_hash`
    ├── commands/
    │   ├── init.rs                          — records hashes when writing files
    │   └── upgrade.rs                       — the new command
    └── templates.rs                          — unchanged (just re-consumed)
```

**Call graph for `upgrade`:**

```
upgrade(opts)
  ├── Manifest::read → Error::NotLoaded if missing
  ├── check_version (cmp prev_version to CARGO_PKG_VERSION) → Error::DowngradeRefused if lower
  ├── collect_desired_templates()              → Vec<DesiredFile> (relative_path, bytes)
  ├── for each desired file: classify()        → Classification enum
  │     - Add (file missing on disk)
  │     - Unchanged (on-disk matches desired)
  │     - AutoUpdate (hash matches recorded, content differs)
  │     - UserModified (hash mismatches recorded)
  │     - AmbiguousNoHash (no recorded hash, content differs — pre-hash install)
  ├── for each recorded-but-not-desired file: classify_removal()
  │     - SafeRemove (hash matches recorded)
  │     - Orphaned (hash mismatches — user modified)
  ├── resolve_conflicts (UserModified + AmbiguousNoHash → Action)
  │     - interactive prompt OR --force/--skip-modified/--create-new
  ├── apply (Add → write+hash, AutoUpdate → write+hash,
  │          Overwrite → write+hash, Skip → noop,
  │          CreateNew → write to <path>.new (no hash),
  │          SafeRemove → unlink+clear_hash, Orphaned → leave+clear_hash)
  ├── update_managed_block on CLAUDE.md        (always — marker-based, not hash)
  ├── manifest.version = CARGO_PKG_VERSION; manifest.write
  └── UpgradeSummary { counts, version_from, version_to }
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
    /// SHA-256 hex of each recorded file's contents at last-write time.
    /// Keyed by the same project-relative path stored in `files`.
    /// `BTreeMap` for stable JSON key order across serializations.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hashes: BTreeMap<PathBuf, String>,
}

impl Manifest {
    pub fn record_file_with_hash(&mut self, path: impl Into<PathBuf>, contents: &[u8]);
    pub fn hash_for(&self, path: &Path) -> Option<&str>;
    pub fn clear_hash(&mut self, path: &Path);
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
    /// Ask the user per-file (via injected Prompter).
    Interactive,
    /// --force: always overwrite modified files.
    Force,
    /// --skip-modified: always preserve modified files.
    Skip,
    /// --create-new: always write the new version as `<path>.new`.
    CreateNew,
}

pub trait Prompter: Send {
    fn prompt(&mut self, relative_path: &Path) -> Result<ConflictChoice>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice { Overwrite, Skip, CreateNew }

#[derive(Debug, Default, Clone, Copy)]
pub struct UpgradeSummary {
    pub added: usize,
    pub updated: usize,        // auto-updated (unmodified template refresh)
    pub unchanged: usize,
    pub modified_preserved: usize,  // user-modified but template unchanged
    pub overwritten: usize,    // user-modified, prompt → overwrite
    pub skipped: usize,        // user-modified, prompt → skip
    pub created_new: usize,    // user-modified, prompt → .new copy
    pub deleted: usize,        // template removed, hash matched → unlinked
    pub orphaned: usize,       // template removed, user modified → left in place
    pub version_from: String,
    pub version_to: String,
}

pub fn upgrade(opts: UpgradeOptions, prompter: &mut dyn Prompter) -> Result<UpgradeSummary>;
```

```rust
// internal to upgrade.rs

enum Classification {
    Add,
    Unchanged,
    AutoUpdate,
    UserModified,
    AmbiguousNoHash,
}

enum RemovalClassification { SafeRemove, Orphaned }
```

New error variants:

```rust
// ark-core/src/error.rs
Error::DowngradeRefused { project_version: String, cli_version: String },
```

(`NotLoaded` already exists — reused.)

`PathExt` addition:

```rust
// ark-core/src/io/path_ext.rs
fn hash_sha256(&self) -> Result<Option<String>>;   // hex; None if file missing
```

*(Consideration: compute hashes on raw `&[u8]` too, for in-memory template content. Implemented as a free fn `hash_bytes(&[u8]) -> String` in `path_ext.rs` alongside the trait method.)*

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
    Upgrade(UpgradeArgs),     // NEW, visible
    #[command(hide = true)]
    Agent(AgentArgs),
}

#[derive(Args)]
struct UpgradeArgs {
    /// Overwrite user-modified files without prompting.
    #[arg(long, conflicts_with_all = ["skip_modified", "create_new"])]
    force: bool,
    /// Preserve user-modified files without prompting.
    #[arg(long, conflicts_with_all = ["force", "create_new"])]
    skip_modified: bool,
    /// Write updated template as `<path>.new` without prompting.
    #[arg(long, conflicts_with_all = ["force", "skip_modified"])]
    create_new: bool,
    /// Allow proceeding when CLI version < project version.
    #[arg(long)]
    allow_downgrade: bool,
}
```

Top-level prompt implementation in `ark-cli`:

```rust
struct StdioPrompter;   // reads y/n/c from stdin; non-TTY → defaults to Skip
```

[**Constraints**]

- C-1: Hashes are SHA-256, hex-encoded lowercase. Stored under `manifest.hashes` using project-relative `PathBuf` keys that match `manifest.files` entries exactly.
- C-2: Every `init` write routes through a helper that both writes the file AND records the file + hash. No write path can update `files` without also updating `hashes`.
- C-3: `upgrade` NEVER reads, writes, deletes, or hashes files under `.ark/tasks/`, `.ark/specs/`, or `.ark/.installed.json`. This is enforced by a path filter applied to both the desired-templates list AND the manifest's `files` list at upgrade entry (so even if a stale manifest entry exists, it's ignored). The filter matches a fixed allow-list of top-level dirs derived from the layout constants.
- C-4: `upgrade` refuses with `Error::NotLoaded` when the manifest file is absent.
- C-5: `upgrade` refuses with `Error::DowngradeRefused` when `semver::Version::parse(&manifest.version) > semver::Version::parse(CARGO_PKG_VERSION)`, unless `opts.allow_downgrade` is true. (If `manifest.version` fails to parse, treat as unknown and proceed — back-compat for pre-semver manifests.)
- C-6: The version comparison uses `semver::Version` (added as a dep). Same-version upgrades are legal and a no-op for hash-matching files.
- C-7: In `ConflictPolicy::Interactive`, the CLI binary constructs a `StdioPrompter` whose `prompt` reads one line from stdin per conflict. On non-TTY (`atty::is(Stream::Stdin) == false`) the prompter returns `ConflictChoice::Skip` without reading, and emits a stderr note. Library callers pass their own `Prompter`.
- C-8: The `CLAUDE.md` managed block is re-applied on every upgrade via `update_managed_block` with `MANAGED_BLOCK_BODY`. Its contents are not hash-tracked.
- C-9: `.new` files do NOT go into the manifest and do NOT get a hash entry — they're user-visible scratch for review, not Ark-managed state.
- C-10: When a template file has been removed between versions, upgrade only deletes it if `manifest.hash_for(path) == Some(current_sha256)`. Otherwise the file is left in place, the hash is cleared, and the file is reported as `orphaned`. The file's entry is dropped from `manifest.files` in both cases.
- C-11: `AmbiguousNoHash` (no recorded hash + on-disk differs from new template) is treated as `UserModified` for conflict resolution. This is the migration path for projects initialized before `hashes` existed.
- C-12: All filesystem access in `commands/upgrade.rs` routes through `io::PathExt` / `io::fs` helpers. No bare `std::fs::*`.
- C-13: All path composition routes through `layout::Layout`. No hand-joined `.ark/...` strings inside `upgrade.rs`.
- C-14: `UpgradeSummary::Display` output is deterministic: counts always printed in the fixed order above, version transition always printed as a second line, even when counts are zero.

---

## Runtime `runtime logic`

[**Main Flow**]

1. `main.rs` parses `UpgradeArgs`, builds `UpgradeOptions` (with `ConflictPolicy` derived from flags; default `Interactive`), constructs `StdioPrompter`, calls `ark_core::upgrade(opts, &mut prompter)`.
2. `upgrade` reads `Manifest` (errors `NotLoaded` if absent).
3. `upgrade` parses `manifest.version` and `CARGO_PKG_VERSION` as `semver::Version`; if project > cli and `!allow_downgrade` → `DowngradeRefused`.
4. `upgrade` enumerates desired templates by walking the same `ARK_TEMPLATES` and `CLAUDE_TEMPLATES` that `init` uses, producing `Vec<(PathBuf /*relative*/, &'static [u8])>`.
5. `upgrade` applies the protected-path filter to the desired list AND the manifest's recorded `files` list.
6. For each desired file, classify (Add / Unchanged / AutoUpdate / UserModified / AmbiguousNoHash). For each manifest file not in desired, classify (SafeRemove / Orphaned).
7. For UserModified + AmbiguousNoHash:
   - If `conflict_policy != Interactive`, resolve directly from the policy.
   - Else call `prompter.prompt(relative_path)` → `ConflictChoice`.
8. Apply all actions in order: Adds → AutoUpdates → (Overwrites/Skips/CreateNews) → SafeRemoves → Orphaned-clears. Each write goes through `PathExt::write_bytes`; each hash update goes through `Manifest::record_file_with_hash`.
9. `update_managed_block(CLAUDE.md, "ARK", MANAGED_BLOCK_BODY)`. If the marker was absent and now present, `manifest.record_block(...)`.
10. `manifest.version = CARGO_PKG_VERSION; manifest.installed_at = Utc::now(); manifest.write(layout.root())`.
11. Return `UpgradeSummary`. `main.rs` prints `Display`.

[**Failure Flow**]

1. `.ark/.installed.json` missing → `Error::NotLoaded { path }`. Stdout unchanged; exit 1.
2. Manifest present but JSON-corrupt → existing `Error::ManifestCorrupt` propagates.
3. Project version > CLI version and no `--allow-downgrade` → `Error::DowngradeRefused`. No files touched.
4. Mid-upgrade write fails (e.g., permission denied on one template) → returns `Error::Io`. Files written up to that point are on disk, manifest is NOT yet rewritten (step 10 hasn't run). Next `ark upgrade` retries; already-applied changes are reported as `Unchanged` (hash now matches since content matches embedded template).
5. `update_managed_block` returns `ManagedBlockCorrupt` → propagate; manifest unchanged.
6. `Prompter::prompt` returns `Err` → propagate; partial state behavior identical to (4).

[**State Transitions**]

Per-file state machine at upgrade time:

```
(desired, on-disk, recorded-hash)
  (Y, absent, -)              → Add           → write + record hash
  (Y, present, match-recorded):
        match-desired          → Unchanged    → noop
        ≠ desired              → AutoUpdate   → write + update hash
  (Y, present, mismatch-recorded OR none):
        match-desired          → ModifiedPreserved  (user edit happens to equal new template — rare, treat as Unchanged + refresh hash)
        ≠ desired              → UserModified → ConflictChoice branch
  (N-in-desired, present, match-recorded)     → SafeRemove  → unlink + drop manifest entry
  (N-in-desired, present, mismatch)           → Orphaned    → leave + drop manifest entry
  (N-in-desired, absent, anything)            → already gone → drop manifest entry
```

*(The "(present, match-desired, mismatch-recorded)" case — user edited then reverted — is subsumed as a harmless Unchanged write once we refresh the hash. Counted in `unchanged`, not a separate bucket.)*

---

## Implementation `split task into phases`

[**Phase 1 — Manifest + hash plumbing**]

1.1 Add `sha2 = "0.10"` and `semver = "1"` to `ark-core/Cargo.toml`. `is-terminal = "0.4"` to `ark-cli/Cargo.toml` (for TTY detection; `atty` is unmaintained).

1.2 Extend `Manifest`:
- Add `hashes: BTreeMap<PathBuf, String>` field with `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`.
- Add `record_file_with_hash(path, contents)` — writes path to `files` AND hex(sha256(contents)) to `hashes`.
- Add `hash_for(path) -> Option<&str>`.
- Add `clear_hash(path)` — removes from both `files` and `hashes`.
- Keep existing `record_file` for callers that don't have the content at hand (used by `load`'s snapshot restore path — see 1.4).

1.3 Add `PathExt::hash_sha256(&self) -> Result<Option<String>>` and free `hash_bytes(&[u8]) -> String` in `io/path_ext.rs`. Both hex-lowercase.

1.4 Update `init.rs`'s `extract()` helper to call `manifest.record_file_with_hash(relative, entry.contents)` instead of `record_file`. No other caller change needed — `init` is the only place that writes templates today.

1.5 Audit `load.rs` (snapshot restore). The snapshot contents are byte-copies of what was previously on disk. If we can compute hashes at restore time against the restored bytes, hashes survive a load. Otherwise hashes are left absent for restored files and `upgrade` treats them as `AmbiguousNoHash`. **Decision: compute at restore**, since snapshot restore reads each file's bytes anyway.

1.6 Unit tests for `Manifest` hash round-trip + `hash_sha256`.

[**Phase 2 — Upgrade command core**]

2.1 Create `ark-core/src/commands/upgrade.rs` with:
- `UpgradeOptions`, `ConflictPolicy`, `ConflictChoice`, `Prompter` trait.
- `UpgradeSummary` + `Display`.
- `upgrade(opts, &mut dyn Prompter) -> Result<UpgradeSummary>`.
- Internal helpers: `collect_desired`, `filter_protected`, `classify`, `classify_removal`, `resolve`, `apply_action`, `is_protected_path`.
- `Error::DowngradeRefused` variant added to `error.rs`.

2.2 Implement classification + resolution logic purely in-memory (no I/O side effects except reading files to hash). Produce a `Vec<PlannedAction>` first, then apply in a single pass. This makes unit testing easier (classification can be tested without writing anything) and ensures partial-failure is well-defined (actions before the failing write complete, subsequent ones don't).

2.3 Export from `lib.rs`.

2.4 Unit tests for each branch of `classify` and `classify_removal`; unit tests for the ordering of actions; failing-write test that verifies manifest is NOT rewritten.

[**Phase 3 — CLI + interactive prompter + integration tests**]

3.1 In `ark-cli/src/main.rs`: add `Upgrade(UpgradeArgs)` variant, derive `ConflictPolicy` from the (mutually exclusive) flags. Implement `StdioPrompter` in the binary crate (not in the library — library code must not read stdin).

3.2 `StdioPrompter::prompt` reads a single line, parses `y`/`Y` → Overwrite, `n`/`N`/""→ Skip, `c`/`C` → CreateNew, anything else → Skip with a stderr note. TTY check via `is_terminal::IsTerminal`; non-TTY → Skip (no stdin read) with a single stderr note at upgrade start.

3.3 Integration tests in `crates/ark-cli/tests/upgrade.rs`:
- `fresh_install_then_upgrade_is_noop` — init → upgrade with the same binary → all `unchanged`.
- `template_change_with_unmodified_file` — simulate by flipping a byte in the manifest's stored hash, run upgrade → verifies the file is rewritten (AutoUpdate path).
- `template_change_with_user_modified_file_force` — modify a template, run `upgrade --force` → overwritten.
- `template_change_with_user_modified_file_skip` — modify, run `--skip-modified` → preserved.
- `template_change_with_user_modified_file_create_new` — `--create-new` → `.new` file written.
- `removed_template_unmodified_is_deleted` — drop an entry from `Manifest.files`... wait, we can't simulate the reverse (we can only test with what the embedded templates actually contain). Alternative: inject a fake manifest entry pointing at a file whose content matches its recorded hash but is NOT in the embedded templates → should be deleted.
- `removed_template_modified_is_orphaned` — same but with mismatched hash → file remains, manifest entry dropped.
- `downgrade_refused` — manually bump `manifest.version` to "99.0.0", run upgrade → error.
- `downgrade_allowed_with_flag` — same, `--allow-downgrade` → succeeds.
- `missing_manifest_refuses` — `rm .ark/.installed.json`, run upgrade → `NotLoaded`.
- `protected_paths_untouched` — create a file under `.ark/tasks/` with arbitrary content, run upgrade → file unchanged, no manifest tampering.
- `ambiguous_no_hash_treated_as_modified` — init, then manually delete the `hashes` field from the manifest JSON, modify a file, run `upgrade --skip-modified` → file preserved.
- `hash_backfill_after_same_content` — init, delete hashes, run `upgrade --force` → every file hashed fresh in the new manifest.
- `cli_help_lists_upgrade` — `ark --help` contains `upgrade`.

---

## Trade-offs `ask reviewer for advice`

- T-1: **Where to store hashes — extend `Manifest` vs. separate `.ark/.hashes.json`.**
  - Option A (chosen): Extend the existing manifest. Adv: one file to load/save; `unload`/`remove` already handle it; no new protected path. Disadv: manifest grows; mixes two concerns (what-we-installed vs. hashes). ~30 entries today, fine.
  - Option B: Separate `.ark/.hashes.json`. Adv: cleaner separation. Disadv: two files to keep in sync; `unload`/`remove` need extra handling; if one file gets out of sync with the other, behavior is surprising. **Pick A.**

- T-2: **`atty` vs. `is-terminal` vs. `std::io::IsTerminal` (stable since 1.70).**
  - `std::io::IsTerminal` is in std, no new dep. Ark's MSRV is edition 2024 (rust 1.85+), which includes it. **Pick stdlib.** (`Cargo.toml` doesn't need `is-terminal`.)

- T-3: **Prompt UX — single letter per file vs. "apply to all" batch prompts like Trellis.**
  - Option A: Per-file prompt. Simpler. Fine for Ark's small file set (~15 templates + 3 slash commands).
  - Option B: Batch with "apply to all" options. More complex; only wins when there are many conflicts. Ark will rarely see >3 conflicts in one upgrade. **Pick A.** Users who want batch can use `--force`/`--skip-modified`/`--create-new`.

- T-4: **Include `.installed.json` itself in the managed set, or specially exempt it.**
  - Exempting it is simpler — it's Ark's own state, not a template. Not listing it in `manifest.files` today already. **Keep exempt.**

- T-5: **Do we need the "UserModified happens to match new template" bucket (ModifiedPreserved → Unchanged)?**
  - Rare (user reverts their edit right before upgrade). Makes classification cleaner to handle uniformly: on (present, match-desired, anything) → Unchanged + refresh hash. **Merge into Unchanged, not a separate counter.**

- T-6: **Should `upgrade` on the SAME version be a no-op or still re-apply?**
  - Re-apply is cheap (hash match = noop anyway) and provides a "repair" mechanism if someone deleted a template file. **Same-version upgrade behaves identically — refresh what's missing, leave everything else alone.**

- T-7: **Newline normalization.**
  - Hashes are byte-exact. On Windows, editors may rewrite `\n` → `\r\n`, flipping every hash. Ark is agent-targeted, agents write `\n`. **Document that Windows users who open templates in editors with CRLF conversion will see spurious "modified" detections; accept for now.** (`git config core.autocrlf false` is the workaround.)

---

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `Manifest::record_file_with_hash` populates both `files` and `hashes`; identical calls are idempotent.
- V-UT-2: `Manifest::clear_hash` removes from both maps.
- V-UT-3: `Manifest::hash_for` returns `None` for unknown paths.
- V-UT-4: `hash_bytes(b"hello")` equals the known SHA-256 hex for that input.
- V-UT-5: `PathExt::hash_sha256` returns `Ok(None)` for a missing file.
- V-UT-6: `classify` returns `Add` for missing files.
- V-UT-7: `classify` returns `Unchanged` when on-disk == desired (regardless of hash presence).
- V-UT-8: `classify` returns `AutoUpdate` when hash matches recorded AND content differs.
- V-UT-9: `classify` returns `UserModified` when hash mismatches recorded AND content differs.
- V-UT-10: `classify` returns `AmbiguousNoHash` when no recorded hash AND content differs.
- V-UT-11: `classify_removal` returns `SafeRemove` when hash matches, `Orphaned` otherwise.
- V-UT-12: `is_protected_path` filters `.ark/tasks/...`, `.ark/specs/...`, `.ark/.installed.json`, but not `.ark/workflow.md` or `.claude/commands/ark/quick.md`.
- V-UT-13: `UpgradeSummary::Display` prints all counters in fixed order with a version transition line.
- V-UT-14: `ConflictPolicy` + flag parsing: `--force` ⇒ `Force`, `--skip-modified` ⇒ `Skip`, etc.; mutually-exclusive flags rejected by clap.

[**Integration Tests**]

- V-IT-1: `fresh_install_then_upgrade_is_noop`.
- V-IT-2: `template_change_with_unmodified_file` — tamper with manifest hash to simulate a changed template.
- V-IT-3: `user_modified_file_force_overwrites`.
- V-IT-4: `user_modified_file_skip_preserves`.
- V-IT-5: `user_modified_file_create_new_writes_sidecar`.
- V-IT-6: `removed_template_unmodified_is_deleted` — fake manifest entry.
- V-IT-7: `removed_template_modified_is_orphaned`.
- V-IT-8: `ambiguous_no_hash_prompt_path` — inject `StubPrompter` that returns `Skip`; verify the file survives.
- V-IT-9: `protected_tasks_dir_untouched` — create `.ark/tasks/foo/task.toml` (not Ark-managed), upgrade, file unchanged.
- V-IT-10: `cli_help_lists_upgrade` and `upgrade --help` shows all three flags + `--allow-downgrade`.
- V-IT-11: `hashes_survive_unload_load_roundtrip` — init, unload, load, verify `manifest.hashes` matches.

[**Failure / Robustness Validation**]

- V-F-1: `missing_manifest_errors` — `Error::NotLoaded`.
- V-F-2: `downgrade_refused_without_flag` — `Error::DowngradeRefused`.
- V-F-3: `downgrade_allowed_with_flag` succeeds.
- V-F-4: `corrupt_manifest_errors` — existing `ManifestCorrupt`.
- V-F-5: `write_failure_leaves_manifest_untouched` — make one template dest read-only, run upgrade, verify manifest's version is NOT bumped.
- V-F-6: `managed_block_corrupt_surfaced` — place orphan `<!-- ARK:START -->` in CLAUDE.md, run upgrade → `ManagedBlockCorrupt`.
- V-F-7: `non_semver_project_version_parses_as_unknown` — manifest.version = "dev", upgrade proceeds (no downgrade check).

[**Edge Case Validation**]

- V-E-1: `same_version_upgrade_is_noop`.
- V-E-2: `empty_project_root_fails` — no `.ark/` at all → `NotLoaded`.
- V-E-3: `user_edit_reverts_to_new_template_is_unchanged` — on-disk happens to equal new desired, even with stale hash → counts as `unchanged`, hash refreshed.
- V-E-4: `non_tty_default_prompter_skips` — `StubPrompter` wrapping a closed stdin → all user-modified files preserved.
- V-E-5: `.new_files_do_not_get_hashed` — `--create-new`, re-run upgrade → `.new` file is not tracked.
- V-E-6: `removing_last_template_entry_does_not_corrupt_manifest` — empty-ish manifest still round-trips.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-10 (help lists upgrade), V-UT-14 (flag parsing) |
| G-2 | V-UT-1, V-UT-4, V-UT-5 |
| G-3 | V-IT-2 (auto-update), V-UT-8 |
| G-4 | V-IT-3, V-IT-4, V-IT-5 |
| G-5 | V-E-3 |
| G-6 | V-IT-6, V-IT-7, V-UT-11 |
| G-7 | V-IT-8, V-F-7 |
| G-8 | V-F-1, V-F-2, V-F-3 |
| G-9 | V-IT-9, V-UT-12 |
| G-10 | V-F-6 (corrupt case), manual observation in V-IT-1 (block re-applied) |
| G-11 | V-UT-13 |
| C-1 | V-UT-4 |
| C-2 | V-UT-1 (record_file_with_hash) + V-IT-1 (init-then-upgrade finds hashes) |
| C-3 | V-IT-9, V-UT-12 |
| C-4 | V-F-1 |
| C-5 | V-F-2, V-F-3 |
| C-6 | V-E-1, V-F-7 |
| C-7 | V-E-4 |
| C-8 | V-F-6 + V-IT-1 (block re-applied) |
| C-9 | V-E-5 |
| C-10 | V-IT-6, V-IT-7 |
| C-11 | V-IT-8 |
| C-12 | review-only (grep for `std::fs::` in upgrade.rs) |
| C-13 | review-only (grep for `.ark/` literals in upgrade.rs) |
| C-14 | V-UT-13 |
