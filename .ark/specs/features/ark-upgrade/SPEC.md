
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
