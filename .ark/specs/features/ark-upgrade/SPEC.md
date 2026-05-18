[**Goals**]

- G-1: `ark upgrade` is a top-level visible subcommand, safe to run repeatedly.
- G-2: User-modified files are detected by SHA-256 hash comparison against the manifest.
- G-3: Unmodified template files update silently; modified files prompt (overwrite / skip / write `.new`).
- G-4: Files removed between versions are deleted only when their on-disk hash matches the manifest; otherwise left as orphans.
- G-5: Managed blocks (`CLAUDE.md`, `AGENTS.md`) and SessionStart hook entries are re-applied unconditionally; not hash-tracked.

[**Non-goals**]

- NG-1: No network I/O; no migration manifest system.
- NG-2: No backup directory; rollback is not promised.
- NG-3: No CRLF/LF normalization before hashing.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs            (Upgrade(UpgradeArgs) top-level)
└── ark-core/src/
    ├── error.rs                   (DowngradeRefused, UnsafeManifestPath)
    ├── io/path_ext.rs             (hash_sha256 method + free hash_bytes fn)
    ├── state/manifest.rs          (hashes: BTreeMap<PathBuf, String>;
    │                                record_file_with_hash, hash_for, clear_hash, drop_file)
    ├── commands/init.rs           (records hashes when writing files)
    └── commands/upgrade.rs        (the new command)
```

Call graph for `upgrade`:

```
upgrade(opts, prompter)
  ├── Manifest::read                                       → Error::NotLoaded if missing
  ├── validate_manifest_paths(&manifest.files)             → Error::UnsafeManifestPath  (C-15; before version check)
  ├── check_version (semver cmp)                           → Error::DowngradeRefused if project > cli ∧ !allow_downgrade
  ├── collect_desired_templates()                          → Vec<(PathBuf, &'static [u8])>
  ├── plan_actions()                                       → Vec<PlannedAction> sorted by (bucket, path)
  │     per desired:    classify → Add | Unchanged{refresh_hash} | AutoUpdate | UserModified | AmbiguousNoHash
  │     per orphan:     classify_removal → SafeRemove | Orphaned
  │     resolve UserModified | AmbiguousNoHash via policy or prompter
  ├── apply_writes()                                       (Write, CreateNew, RefreshHashOnly, Preserve)
  │     mutates manifest in-memory
  ├── update_managed_block(CLAUDE.md, "ARK", MANAGED_BLOCK_BODY)
  ├── update_settings_hook(.claude/settings.json, ark_session_start_hook_entry())
  ├── manifest.version = CARGO_PKG_VERSION
  ├── manifest.write()                                     ← durable BEFORE deletions
  ├── apply_deletions()                                    (Delete, DropManifestEntry)
  ├── manifest.write() again iff deletions mutated the manifest
  └── UpgradeSummary
```

[**Data Structure**]

```rust
// ark-core/src/state/manifest.rs (extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub files: Vec<PathBuf>,
    pub managed_blocks: Vec<ManagedBlock>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hashes: BTreeMap<PathBuf, String>,
}

impl Manifest {
    pub fn record_file_with_hash(&mut self, path: impl Into<PathBuf>, contents: &[u8]);
    pub fn hash_for(&self, path: &Path) -> Option<&str>;
    pub fn clear_hash(&mut self, path: &Path);
    pub fn drop_file(&mut self, path: &Path);   // removes from both files and hashes
}

// ark-core/src/commands/upgrade.rs
#[derive(Debug, Clone)]
pub struct UpgradeOptions {
    pub project_root: PathBuf,
    pub conflict_policy: ConflictPolicy,
    pub allow_downgrade: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy { Interactive, Force, Skip, CreateNew }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice { Overwrite, Skip, CreateNew }

pub trait Prompter {
    fn prompt(&mut self, relative_path: &Path) -> Result<ConflictChoice>;
}

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
    RefreshHashOnly { relative: PathBuf, contents: Vec<u8> },
    CreateNew { relative: PathBuf, contents: &'static [u8] },
    Preserve { relative: PathBuf },
    Delete { relative: PathBuf },
    DropManifestEntry { relative: PathBuf },
}
enum WriteKind { Add, AutoUpdate, Overwrite }

// Bucket order for the C-16 sort:
//   Write{Add} < Write{AutoUpdate} < Write{Overwrite}
//     < CreateNew < RefreshHashOnly < Preserve
//     < Delete < DropManifestEntry
// Within a bucket, `relative_path` (lex) is the secondary key.

// ark-core/src/error.rs (additions)
Error::DowngradeRefused    { project_version: String, cli_version: String },
Error::UnsafeManifestPath  { path: PathBuf, reason: &'static str },

// ark-core/src/io/path_ext.rs (additions)
trait PathExt {
    fn hash_sha256(&self) -> Result<Option<String>>;        // hex lowercase; None if file missing
}
pub fn hash_bytes(contents: &[u8]) -> String;               // free fn, hex lowercase
```

[**API Surface**]

```rust
// Library re-exports from ark-core/src/lib.rs
pub use commands::{
    InitOptions, InitSummary,
    LoadOptions, LoadSummary,
    RemoveOptions, RemoveSummary,
    UnloadOptions, UnloadSummary,
    UpgradeOptions, UpgradeSummary, ConflictPolicy, ConflictChoice, Prompter,
    init, load, remove, unload, upgrade,
};

pub fn upgrade(opts: UpgradeOptions, prompter: &mut dyn Prompter) -> Result<UpgradeSummary>;

// CLI (ark-cli/src/main.rs)
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
    #[arg(long, group = "policy")] force: bool,
    /// Preserve user-modified files without prompting.
    #[arg(long, group = "policy")] skip_modified: bool,
    /// Write updated template as `<path>.new` without prompting.
    #[arg(long, group = "policy")] create_new: bool,
    /// Allow proceeding when CLI version < project version.
    /// Orthogonal to the policy group — no `group` attribute.
    #[arg(long)] allow_downgrade: bool,
}

// Stdio prompter in the binary crate
struct StdioPrompter;   // uses std::io::IsTerminal; non-TTY → ConflictChoice::Skip
```

`UpgradeSummary::Display` output is deterministic and prints all counters in fixed order even when zero:

```
{N} file(s): {A} added · {U} updated · {S} unchanged · {M} modified-preserved · {O} overwritten · {K} skipped · {C} .new-copied · {D} deleted · {R} orphaned
{from} → {to}
```

[**Constraints**]

- C-1: Hashes are SHA-256 hex-lowercase; keys in `manifest.hashes` mirror `manifest.files` entries exactly.
- C-2: Every `init` write records the file path AND its hash via a single helper.
- C-3: Upgrade only acts on `manifest.files ∪ desired_templates`; `.ark/.installed.json` is the sole file-level exemption.
- C-4: Missing manifest → `Error::NotLoaded`.
- C-5: `manifest.version > CARGO_PKG_VERSION` → `Error::DowngradeRefused` unless `opts.allow_downgrade`. Unparseable version proceeds.
- C-6: Version comparison uses `semver::Version`; same-version upgrades run a full pass.
- C-7: `Interactive` policy on non-TTY downgrades to `Skip` with a stderr note.
- C-8: `CLAUDE.md` / `AGENTS.md` managed blocks and SessionStart hook entries are re-applied on every `init` / `load` / `upgrade`; not hash-tracked. Sibling user content is preserved.
- C-9: `.new` files are never recorded in the manifest and never hashed.
- C-10: Removed templates are deleted iff `manifest.hash_for(path) == on_disk_sha256`; otherwise orphaned. Either way, the manifest entry is dropped.
- C-11: AmbiguousNoHash with on-disk-differs-from-desired is treated as `UserModified`.
- C-12: All filesystem access in `upgrade.rs` routes through `io::PathExt` / `io::fs`; all path composition routes through `layout::Layout`.
- C-13: `Prompter` is dyn-compatible (no generics, no `Self: Sized`).
- C-14: Upgrade is not safe against concurrent file modification.
- C-15: Every `manifest.files` path is normalized via `layout.resolve_safe` before any I/O; failures surface `Error::UnsafeManifestPath` and halt before mutation.
- C-16: `plan_actions` returns actions sorted by `(bucket, relative_path)` for deterministic execution: `Write{Add}`, `Write{AutoUpdate}`, `Write{Overwrite}`, `CreateNew`, `RefreshHashOnly`, `Preserve`, `Delete`, `DropManifestEntry`.

[**CHANGELOG**]

- 2026-04-25 `ark-context`: C-8 extended to cover `.claude/settings.json` `SessionStart` hook entry alongside `CLAUDE.md` managed block.
- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
