[**Goals**]

- G-1: `ark context` prints a JSON or text snapshot of git + tasks + specs + recent archive.
- G-2: `--scope {session|phase}` selects the snapshot's breadth; `--for <phase>` targets one phase.
- G-3: JSON payloads carry `"schema": 1`; the schema is additive-only.
- G-4: A `SessionStart` hook installed in `.claude/settings.json` invokes `ark context` automatically per session.
- G-5: Slash commands consume the projection; the `commit` projection is body-free (paths only).

[**Non-goals**]

- NG-1: No mutation; read-only command.
- NG-2: No file bodies inlined in the projection — paths and summaries only.
- NG-3: No caching layer; every invocation re-reads state.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                       (Context(ContextArgs) top-level)
└── ark-core/src/
    ├── error.rs                              (Error::GitSpawn)
    ├── layout.rs                             (claude_settings, specs_project_dir,
    │                                           specs_project_index, discover_from)
    ├── io/
    │   ├── fs.rs                             (update_settings_hook, remove_settings_hook,
    │   │                                      read_settings_hook, ARK_CONTEXT_HOOK_COMMAND,
    │   │                                      ark_session_start_hook_entry)
    │   └── git.rs                            (NEW; sole sanctioned `Command::new("git")` site)
    ├── state/snapshot.rs                     (hook_bodies + SnapshotHookBody, #[serde(default)])
    └── commands/context/
        ├── mod.rs                            (entry, ContextOptions, SessionStart envelope)
        ├── gather.rs                         (single-pass collection)
        ├── model.rs                          (Context + sub-structs, SCHEMA_VERSION, caps)
        ├── projection.rs                     (Scope, PhaseFilter, project())
        ├── render.rs                         (text-mode Display)
        └── related_specs.rs                  (PRD `[**Related Specs**]` parser)
```

Module coupling: `mod.rs → gather → model`; `mod.rs → projection → model`; `mod.rs → render → model`. `related_specs.rs` and `io/git.rs` are leaves used only by `gather.rs`.

Call graph for `ark context`:

```
context(opts)
  ├── (CLI) layout = TargetArgs::resolve_with_discovery       (C-21)
  ├── ctx = gather::gather_context(&layout)
  │     ├── run_git(["rev-parse","--abbrev-ref","HEAD"])      → branch
  │     ├── run_git(["status","--porcelain"])                 → dirty files (cap 20) + count
  │     ├── run_git(["log","--oneline","-n","5"])             → recent commits
  │     ├── list active tasks (skip "archive", ".current"; sort by updated_at desc)
  │     ├── list archive (5 most recent by archived_at desc)
  │     ├── parse specs/project/INDEX.md  (GFM table after `## Index`)
  │     ├── parse specs/features/INDEX.md (read_managed_block "ARK:FEATURES")
  │     └── if .current exists: load task.toml; list NN_PLAN/NN_REVIEW;
  │                              parse PRD `[**Related Specs**]`
  ├── projected = projection::project(ctx, opts.scope)
  └── if (scope=Session ∧ format=Json): wrap in SessionStart envelope (G-4)
      else: emit raw projection (or text via Display) with trailing newline
```

Call graph for `update_settings_hook`:

```
update_settings_hook(path, ark_entry) -> Result<bool>
  ├── read settings file → serde_json::Value (or {} if missing/empty)
  ├── navigate to "hooks"."SessionStart" (creating intermediates if absent)
  ├── find entry whose entry.hooks[*].command == ARK_CONTEXT_HOOK_COMMAND
  ├── replace if found, append if not
  ├── serialize back (pretty, 2-space, BTreeMap-ordered)
  └── write iff bytes differ
  → Ok(true) if a write happened, Ok(false) if idempotent no-op
```

[**Data Structure**]

```rust
// ark-core/src/commands/context/model.rs
pub const SCHEMA_VERSION: u32 = 1;
pub const DIRTY_FILES_CAP: usize = 20;
pub const RECENT_COMMITS_CAP: usize = 5;
pub const ARCHIVE_CAP: usize = 5;

pub struct Context {
    pub schema: u32,
    pub generated_at: DateTime<Utc>,
    pub project_root: PathBuf,
    pub git: GitState,
    pub tasks: TasksState,
    pub specs: SpecsState,
    pub archive: ArchiveState,
    pub current_task: Option<CurrentTask>,
}

pub struct GitState {
    pub branch: String,            // "unknown" if non-git or detached
    pub head_short: String,
    pub is_clean: bool,
    pub uncommitted_changes: u32,
    pub dirty_files: Vec<String>,  // capped at DIRTY_FILES_CAP
    pub recent_commits: Vec<GitCommit>,
}

pub struct GitCommit { pub hash: String, pub message: String }

pub struct TasksState  { pub active: Vec<TaskSummary> }   // sorted by updated_at desc
pub struct SpecsState  { pub project: Vec<SpecRow>, pub features: Vec<SpecRow> }
pub struct ArchiveState { pub recent: Vec<ArchivedTask> } // capped at ARCHIVE_CAP

pub struct TaskSummary {
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub iteration: u32,
    pub updated_at: DateTime<Utc>,
}

pub struct SpecRow { pub name: String, pub path: String, pub scope: String }

pub struct ArchivedTask {
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub archived_at: DateTime<Utc>,
    pub path: String,
}

pub struct CurrentTask {
    pub slug: String,
    pub summary: TaskSummary,
    pub artifacts: Vec<ArtifactSummary>,
    pub related_specs: Vec<String>,
}

pub struct ArtifactSummary { pub kind: ArtifactKind, pub path: String, pub lines: u32 }

#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ArtifactKind {
    Prd,
    Plan { iteration: u32 },
    Review { iteration: u32 },
    Verify,
    TaskToml,
}
impl ArtifactKind {
    pub fn iteration(&self) -> Option<u32>;   // C-19 helper
}

// ark-core/src/commands/context/projection.rs
pub enum Scope { Session, Phase(PhaseFilter) }

#[serde(rename_all = "lowercase")]
pub enum PhaseFilter { Design, Plan, Review, Execute, Verify }

#[serde(tag = "scope", rename_all = "lowercase")]
pub enum ScopeTag { Session, Phase { phase: PhaseFilter } }

pub struct ProjectedContext {
    pub schema: u32,
    #[serde(flatten)]
    pub scope: ScopeTag,
    pub generated_at: DateTime<Utc>,
    pub project_root: PathBuf,
    pub git: GitState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TasksState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs: Option<SpecsState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveState>,
}

// ark-core/src/io/fs.rs
pub const ARK_CONTEXT_HOOK_COMMAND: &str = "ark context --scope session --format json";
pub fn ark_session_start_hook_entry() -> serde_json::Value;
pub fn update_settings_hook(path: &Path, entry: serde_json::Value) -> Result<bool>;
pub fn remove_settings_hook(path: &Path, identity_value: &str) -> Result<bool>;
pub fn read_settings_hook(path: &Path, identity_value: &str) -> Result<Option<serde_json::Value>>;

// ark-core/src/io/git.rs
pub struct GitOutput { pub exit_code: i32, pub stdout: String, pub stderr: String }
pub fn run_git(args: &[&str], cwd: &Path) -> Result<GitOutput>;

// ark-core/src/layout.rs
impl Layout {
    pub fn claude_settings(&self) -> PathBuf;       // .claude/settings.json
    pub fn specs_project_dir(&self) -> PathBuf;     // .ark/specs/project/
    pub fn specs_project_index(&self) -> PathBuf;   // .ark/specs/project/INDEX.md
    pub fn discover_from(cwd: impl AsRef<Path>) -> Result<Self>;  // C-21
}

// ark-core/src/state/snapshot.rs
pub struct Snapshot {
    // ... existing fields ...
    #[serde(default)]
    pub hook_bodies: Vec<SnapshotHookBody>,
}
pub struct SnapshotHookBody {
    pub path: PathBuf,
    pub entry: serde_json::Value,
    pub identity_key: String,
}

// ark-core/src/error.rs (additions)
Error::GitSpawn { source: io::Error }
```

[**API Surface**]

```rust
// ark-core/src/commands/context/mod.rs
pub struct ContextOptions {
    pub layout: Layout,
    pub scope: Scope,
    pub format: Format,
}

pub enum Format { Json, Text }

pub struct ContextSummary {
    pub rendered: String,    // exactly one stdout write per invocation (C-7)
    pub schema: u32,
}

pub fn context(opts: ContextOptions) -> Result<ContextSummary>;

// CLI shape (ark-cli/src/main.rs)
#[derive(Subcommand)]
enum Command {
    Init(InitArgs),
    Load(LoadArgs),
    Unload(TargetArgs),
    Remove(TargetArgs),
    Upgrade(UpgradeArgs),
    Context(ContextArgs),                         // visible in ark --help
    #[command(hide = true)]
    Agent(AgentArgs),
}

#[derive(clap::Args)]
struct ContextArgs {
    #[command(flatten)]
    target: TargetArgs,
    #[arg(long, value_enum, default_value = "session")]
    scope: ScopeArg,
    #[arg(long = "for", value_enum)]
    r#for: Option<PhaseArg>,                      // required iff scope=phase; rejected otherwise
    #[arg(long, value_enum, default_value = "text")]
    format: FormatArg,
}
```

`TargetArgs` gains `resolve_with_discovery() -> Result<PathBuf>` for commands that require an existing project (`context`, `unload`, `remove`, `upgrade`, `load` without `--force`); the original `resolve()` is retained for `init` and `load --force`.

Library re-exports from `ark-core/src/lib.rs`: `ContextOptions`, `ContextSummary`, `Format` (as `ContextFormat`), `Scope` (as `ContextScope`), `PhaseFilter`, `context`, plus the model types (`Context`, `GitState`, `GitCommit`, `TaskSummary`, `ArtifactKind`, `ArtifactSummary`, `SpecRow`, `SpecsState`, `CurrentTask`, `ArchiveState`, `ArchivedTask`, `TasksState`, `ProjectedContext`, `ScopeTag`, `SCHEMA_VERSION`). Internal caps (`DIRTY_FILES_CAP`, `RECENT_COMMITS_CAP`, `ARCHIVE_CAP`) and `run_git` / `GitOutput` are NOT re-exported.

`io::fs` exports add `ARK_CONTEXT_HOOK_COMMAND`, `ark_session_start_hook_entry`, `update_settings_hook`, `remove_settings_hook`, `read_settings_hook`.

[**Constraints**]

- C-1: `ark context` is listed in `ark --help`; no `#[command(hide = true)]`.
- C-2: `ark context --help` mentions neither "hidden" nor "not covered by semver".
- C-3: JSON output's first field is `"schema": 1`.
- C-4: All filesystem access in `commands/context/` routes through `io::PathExt` / `io::fs`.
- C-5: All `.ark/`-relative path composition routes through `layout::Layout`.
- C-6: `Context` field order is the JSON schema source of truth; renames/removes bump `SCHEMA_VERSION`, adds are free.
- C-7: `ark context` emits exactly one stdout write per invocation.
- C-8: `gather_context` caps dirty_files at 20 and reads at most 5 commits from `git log`.
- C-9: Archive listing reads at most 5 most-recent subdirs, sorted by `archived_at` descending.
- C-10: Text mode is not machine-parseable and carries no schema version.
- C-11: `SessionStart` hook entry identity is `entry.hooks[*].command == ARK_CONTEXT_HOOK_COMMAND`.
- C-12: `update_settings_hook` is idempotent — running `ark init` twice produces a byte-identical `settings.json`.
- C-13: Empty state never errors; `ark context` returns valid empty vecs.
- C-14: Non-Ark directory → `Error::NotLoaded`.
- C-15: SessionStart hook timeout is 5000ms.
- C-16: Settings-hook helpers touch only the Ark-owned entry; sibling user hooks and unrelated top-level keys are preserved.
- C-17: `.claude/settings.json` is not hash-tracked; the Ark entry is re-applied on every `init` / `load` / `upgrade`.
- C-18: `Snapshot::hook_bodies` is `#[serde(default)]`; older `.ark.db` files (pre-`hook_bodies`) deserialize successfully.
- C-19: Artifact iteration files match `^(NN)_PLAN\.md$` and `^(NN)_REVIEW\.md$`, sorted ascending; "latest" = `max_by_key(iteration())`.
- C-20: Related-specs parser scans the line `[**Related Specs**]` until the next `[**...**]` or EOF, extracting `specs/features/<slug>/SPEC.md` tokens. Projection filter keeps a feature row iff some related-specs path ends with the row's normalized path.
- C-21: `Layout::discover_from(cwd)` walks ancestors for `.ark/` and is used by commands requiring an existing project (`context`, `unload`, `remove`, `upgrade`, `load` without `--force`); not used by `init` or `load --force`.
- C-22: `ark-core/src/io/git.rs` is the sole sanctioned `Command::new("git")` site; non-zero exit returns `Ok(GitOutput { exit_code, .. })`; spawn failure → `Error::GitSpawn`.
- C-23: JSON output is `<rendered>\n` with 2-space indent; for `--scope session --format json`, `<rendered>` is the SessionStart envelope.
- C-24: INDEX parsers: `specs/features/INDEX.md` via `read_managed_block("ARK:FEATURES")` (3 cols); `specs/project/INDEX.md` via first GFM table after `^##\s+Index\b`. Both skip header, separator, and `{...}`-wrapped placeholder rows.
- C-25: `DIRTY_FILES_CAP: usize = 20`.
- C-26: `std::process::Command::new` may be invoked only from `io/git.rs`; enforced by source-scan test `commands_no_bare_command_new`.
- C-27: Snapshot forward compat: new fields carry `#[serde(default)]`; `SCHEMA_VERSION` is not bumped on additive serde changes.
- C-28: Source-scan test `commands_no_bare_command_new` reads non-test files under `commands/` and asserts `Command::new` does not appear.
- C-29: `ark upgrade` twice in a row produces a byte-identical `.claude/settings.json`.

[**CHANGELOG**]

- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
