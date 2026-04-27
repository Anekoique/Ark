[**Goals**]

- **G-1:** `ark context` is a top-level, visible, semver-covered subcommand. Appears in `ark --help`. Stable JSON schema, separate from the hidden `ark agent` namespace.
- **G-2:** Two flags control the output: `--scope {session|phase}` (default `session`) and `--for {design|plan|review|execute|verify}` (required iff `--scope=phase`; rejected otherwise). Clap rejects mismatched combinations with a clear message.
- **G-3:** `--format {json|text}` (default `text`) selects output shape. Both modes derive from the same in-memory `Context` struct; JSON via `serde_json::to_string_pretty`, text via a `Display`-impl summary.
- **G-4:** JSON output (raw projection bodies) carries `"schema": 1` as the first field. The schema is **additive-only** going forward; field rename or removal requires bumping `SCHEMA_VERSION`.
- **G-5:** Payload contains paths and summaries only — never file bodies. Artifacts appear as `{kind, iteration?, path, lines}`; specs as index-row data; archives as `{slug, title, tier, archived_at, path}`.
- **G-6:** Session projection (`--scope session`) returns: git state, active-tasks list (flat), project specs index, feature specs index, recent-archive (last 5), current task (if any). Text-mode section names: `## GIT STATUS`, `## CURRENT TASK`, `## ACTIVE TASKS`, `## SPECS`, `## ARCHIVE`. Sections absent from a projection are omitted entirely.
- **G-7:** Phase projections (`--scope phase --for <phase>`):
  - `design`: full `specs` (project + features unfiltered) + `archive`. No `tasks`.
  - `plan` / `review`: `specs.project` unchanged, `specs.features` filtered via the C-20 predicate. No `archive`. No `tasks`.
  - `execute` / `verify`: `specs.project` only; `specs.features = []`. No `archive`. No `tasks`.
- **G-8:** A `SessionStart` hook is installed into `.claude/settings.json` via `update_settings_hook` at `ark init` / `ark load` / `ark upgrade`. The file is **not** in the embedded template tree and **not** hash-tracked. The helper is idempotent and preserves sibling hooks (other `hooks.*` keys) plus unrelated top-level keys.
- **G-9:** When `--scope session --format json` is selected, the inner JSON is wrapped in Claude Code's SessionStart hook envelope (`{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: <stringified projection>}}`). Every other `(scope, format)` combination emits raw output. The envelope shape is fixed: `additionalContext` is a string (the hook contract requires it); the inner string still contains `"schema": 1` as its first field.
- **G-10:** The three shipped slash commands (`templates/claude/commands/ark/{quick,design,archive}.md`) reference `ark context --scope phase --for <phase> --format json` at their phase entry points. The design slash command threads the recipe through every phase (DESIGN, PLAN, REVIEW, EXECUTE, VERIFY).
- **G-11:** End-to-end round-trip:
  1. `ark init` → `update_settings_hook` writes the canonical Ark entry.
  2. `ark unload` → `Snapshot::hook_bodies` captures the Ark entry; the live entry is surgically removed (sibling user hooks left in place).
  3. `ark load` → re-applies the entry via `update_settings_hook`; older snapshots that pre-date `hook_bodies` deserialize successfully (defaulting to empty) and the canonical entry is still re-applied.
  4. `ark remove` → removes the Ark entry only; sibling user hooks survive.
  5. `ark upgrade` → re-applies the canonical entry unconditionally; user customizations to the entry itself are reverted (matches `CLAUDE.md` managed-block precedent).
- **G-12:** `Layout::discover_from(cwd)` walks ancestors looking for `.ark/`; commands that require an existing project (`context`, `unload`, `remove`, `upgrade`, `load` without `--force`) use it. `init` and `load --force` continue using the explicit target — they scaffold a project, not locate one.

Non-goals:

- **NG-1:** No mutation. `ark context` is read-only; no `--write`, no state transitions, no file creation.
- **NG-2:** No multi-developer / assignee / journal concepts.
- **NG-3:** No monorepo / sub-repo aggregation.
- **NG-4:** No deep git history beyond the last 5 one-line commits. `git log` is still the tool for that.
- **NG-5:** No cross-task search (`ark search` is reserved for a future phase).
- **NG-6:** No file bodies inlined in JSON. Callers `Read` files they need.
- **NG-7:** No caching layer. Every invocation re-reads state.
- **NG-8:** No `--scope task` / `--scope feature` for per-entity drill-down (the flag shape leaves room).
- **NG-9:** No hook rendering for `UserPromptSubmit`, `PreToolUse`, etc. Only `SessionStart`.
- **NG-10:** No `.codex/` / `.cursor/` rendering. Claude-only.
- **NG-11:** No structured parsing of PRD's `[**Related Specs**]` beyond C-20's regex.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                        — adds Context(ContextArgs) top-level
└── ark-core/src/
    ├── lib.rs                                  — re-exports public context API + Layout
    ├── error.rs                                — adds Error::GitSpawn
    ├── layout.rs                               — adds claude_settings(), specs_project_dir(),
    │                                             specs_project_index(), discover_from()
    ├── io/
    │   ├── path_ext.rs                         — unchanged
    │   ├── fs.rs                               — adds update_settings_hook,
    │   │                                         remove_settings_hook,
    │   │                                         read_settings_hook,
    │   │                                         ARK_CONTEXT_HOOK_COMMAND,
    │   │                                         ark_session_start_hook_entry()
    │   └── git.rs                              — NEW (C-22) — only sanctioned
    │                                             Command::new("git") site
    ├── state/
    │   ├── manifest.rs                         — unchanged
    │   └── snapshot.rs                         — adds hook_bodies + SnapshotHookBody
    │                                             with #[serde(default)] (C-27)
    ├── commands/
    │   ├── init.rs                             — calls update_settings_hook
    │   ├── load.rs                             — restores snapshot.hook_bodies +
    │   │                                         re-applies the canonical entry
    │   ├── unload.rs                           — captures Ark hook into snapshot.hook_bodies
    │   │                                         and surgically removes from settings.json
    │   ├── remove.rs                           — calls remove_settings_hook
    │   ├── upgrade.rs                          — calls update_settings_hook unconditionally
    │   └── context/
    │       ├── mod.rs                          — context() entry, ContextOptions/Summary,
    │       │                                     SessionStart envelope wrapper
    │       ├── gather.rs                       — single-pass collection (git + tasks +
    │       │                                     specs); GFM table iterator
    │       ├── model.rs                        — Context + sub-structs, Serialize,
    │       │                                     SCHEMA_VERSION, DIRTY_FILES_CAP,
    │       │                                     RECENT_COMMITS_CAP, ARCHIVE_CAP
    │       ├── projection.rs                   — Scope, PhaseFilter, project()
    │       ├── render.rs                       — text-mode Display
    │       └── related_specs.rs                — PRD [**Related Specs**] parser (C-20)
└── templates/
    ├── ark/                                    — workflow.md updated to inline
    │                                             ark context calls per phase
    └── claude/
        └── commands/ark/                       — three .md files updated per G-10
```

**Module coupling.** `mod.rs → gather → model`; `mod.rs → projection → model`; `mod.rs → render → model`. `related_specs.rs` is a leaf used only by `gather.rs`. `io/git.rs` is a leaf used only by `gather.rs`.

**Call graph for `ark context`:**

```
context(opts)
  ├── (CLI layer) layout = TargetArgs::resolve_with_discovery (C-21)
  ├── ctx = gather::gather_context(&layout)
  │     ├── io::git::run_git(["rev-parse","--abbrev-ref","HEAD"], root) → branch
  │     ├── io::git::run_git(["status","--porcelain"], root)            → dirty files (cap 20) + count
  │     ├── io::git::run_git(["log","--oneline","-n","5"], root)        → commits
  │     ├── list active tasks (skip "archive", ".current"; sort by updated_at desc)
  │     ├── list archive (5 most recent by archived_at desc)
  │     ├── parse specs/project/INDEX.md (C-24, GFM table after `## Index`)
  │     ├── parse specs/features/INDEX.md via read_managed_block("ARK:FEATURES")
  │     └── if .current exists: load task.toml, list NN_PLAN/NN_REVIEW (C-19),
  │         parse PRD [**Related Specs**] (C-20)
  ├── projected = projection::project(ctx, opts.scope)
  └── if (scope=Session, format=Json): wrap in SessionStart envelope (G-9)
      else: emit raw projection (or text) with trailing newline
```

**Call graph for `update_settings_hook`:**

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

pub struct TasksState  { pub active: Vec<TaskSummary> }    // sorted by updated_at desc
pub struct SpecsState  { pub project: Vec<SpecRow>, pub features: Vec<SpecRow> }
pub struct ArchiveState { pub recent: Vec<ArchivedTask> } // capped at ARCHIVE_CAP

pub struct CurrentTask {
    pub slug: String,
    pub summary: TaskSummary,
    pub artifacts: Vec<ArtifactSummary>,
    pub related_specs: Vec<String>,
}

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
```

```rust
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

pub fn project(ctx: Context, scope: Scope) -> ProjectedContext;
```

```rust
// ark-core/src/state/snapshot.rs
pub struct Snapshot {
    pub version: String,
    pub ark_version: String,
    pub created_at: DateTime<Utc>,
    pub files: Vec<SnapshotFile>,
    pub managed_blocks: Vec<SnapshotBlock>,
    #[serde(default)]                                // C-27 forward-compat
    pub hook_bodies: Vec<SnapshotHookBody>,
}

pub struct SnapshotHookBody {
    pub path: PathBuf,                  // .claude/settings.json
    pub json_pointer: String,           // "/hooks/SessionStart" — reserved for portability
    pub identity_key: String,           // "command" — reserved for portability
    pub identity_value: String,         // ARK_CONTEXT_HOOK_COMMAND
    pub entry: serde_json::Value,       // the Claude Code-shaped hook wrapper
}
```

```rust
// ark-core/src/error.rs
Error::GitSpawn { source: std::io::Error }
```

```rust
// ark-core/src/io/fs.rs
pub const ARK_CONTEXT_HOOK_COMMAND: &str = "ark context --scope session --format json";

pub fn ark_session_start_hook_entry() -> serde_json::Value {
    // {"matcher": "", "hooks": [{"type":"command","command":<ARK_CONTEXT_HOOK_COMMAND>,"timeout":5000}]}
}

pub fn update_settings_hook(path: &Path, entry: serde_json::Value) -> Result<bool>;
pub fn remove_settings_hook(path: &Path, identity_value: &str) -> Result<bool>;
pub fn read_settings_hook(path: &Path, identity_value: &str) -> Result<Option<serde_json::Value>>;
```

```rust
// ark-core/src/io/git.rs
pub struct GitOutput { pub exit_code: i32, pub stdout: String, pub stderr: String }
pub fn run_git(args: &[&str], cwd: &Path) -> Result<GitOutput>;
```

```rust
// ark-core/src/layout.rs
impl Layout {
    pub fn claude_settings(&self) -> PathBuf;       // .claude/settings.json
    pub fn specs_project_dir(&self) -> PathBuf;     // .ark/specs/project/
    pub fn specs_project_index(&self) -> PathBuf;   // .ark/specs/project/INDEX.md
    pub fn discover_from(cwd: impl AsRef<Path>) -> Result<Self>;  // C-21
}
```

[**API Surface**]

CLI shape (in `ark-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum Command {
    Init(InitArgs),
    Load(LoadArgs),
    Unload(TargetArgs),
    Remove(TargetArgs),
    Upgrade(UpgradeArgs),
    Context(ContextArgs),                   // NEW — visible in ark --help
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
    r#for: Option<PhaseArg>,
    #[arg(long, value_enum, default_value = "text")]
    format: FormatArg,
}
```

Library re-exports (from `ark-core/src/lib.rs`): `ContextOptions`, `ContextSummary`, `Format` (as `ContextFormat`), `Scope` (as `ContextScope`), `PhaseFilter`, `context`, plus the model types (`Context`, `GitState`, `TaskSummary`, `ArtifactKind`, `ArtifactSummary`, `SpecRow`, `SpecsState`, `CurrentTask`, `ArchiveState`, `ArchivedTask`, `GitCommit`, `TasksState`, `ProjectedContext`, `ScopeTag`, `SCHEMA_VERSION`) for downstream library consumers. Internal-only constants (`DIRTY_FILES_CAP`, `RECENT_COMMITS_CAP`, `ARCHIVE_CAP`) are NOT re-exported. `run_git`/`GitOutput` are NOT re-exported (internal to gather).

`io::fs` exports add `ARK_CONTEXT_HOOK_COMMAND`, `ark_session_start_hook_entry`, `update_settings_hook`, `remove_settings_hook`, `read_settings_hook`.

`TargetArgs` (CLI) gains `resolve_with_discovery() -> anyhow::Result<PathBuf>` for commands that require an existing project (uses `Layout::discover_from`); `resolve()` retains the original explicit-target semantics for `init` and `load --force`.

[**Constraints**]

- **C-1:** `ark context` is listed in `ark --help` (no `#[command(hide = true)]`).
- **C-2:** `ark context --help` mentions neither "hidden" nor "not covered by semver" — it's a stable public command.
- **C-3:** JSON output's first field is `"schema": 1` (in raw payloads; for `--scope session --format json`, the schema field appears inside `hookSpecificOutput.additionalContext` once that string is parsed). Asserted by byte-level / json-parse tests.
- **C-4:** All filesystem access in `commands/context/` routes through `io::PathExt` / `io::fs` helpers. No bare `std::fs::*`.
- **C-5:** All `.ark/`-relative path composition routes through `layout::Layout` helpers.
- **C-6:** `Context` struct field order and names are the source of truth for JSON schema. Renames / removals require bumping `SCHEMA_VERSION`; additions are free.
- **C-7:** `ark context` emits **exactly one** stdout write per invocation: JSON via a single pre-rendered string + trailing newline, text via a single `Display` write + trailing newline. No interspersed debug prints.
- **C-8:** `gather_context` reads at most `RECENT_COMMITS_CAP` (5) commits from `git log` and caps `dirty_files` at `DIRTY_FILES_CAP` (20). Larger sets are truncated silently; `uncommitted_changes` always reflects the true total.
- **C-9:** Archive listing reads at most `ARCHIVE_CAP` (5) most-recent subdirectories under `.ark/tasks/archive/`, sorted by `archived_at` descending.
- **C-10:** Text mode format is **not** machine-parseable and carries no schema version. Users who need stability parse JSON.
- **C-11:** The `SessionStart` hook entry's identity is `entry.hooks[*].command == ARK_CONTEXT_HOOK_COMMAND`. The detector tolerates a flat-shape entry (`entry.command`) for forward-compat with snapshots written before the matcher-wrapper was introduced. Tests reference the constant, not the literal.
- **C-12:** `update_settings_hook` is idempotent: running `ark init` twice produces a byte-identical `settings.json`.
- **C-13:** `ark context` never exits non-zero on empty state (no `.current`, no active tasks, empty specs). Empty state returns a valid `Context` with empty vecs and `current_task: None`.
- **C-14:** On a non-Ark directory, `ark context` exits with `Error::NotLoaded`.
- **C-15:** Hook timeout is 5000ms (Claude Code-side).
- **C-16:** Settings-hook helpers must NOT rewrite `settings.json` whole-cloth: only the Ark-owned `SessionStart` entry is touched. Sibling hook entries and unrelated top-level keys are preserved (modulo BTreeMap reordering during `serde_json::to_string_pretty`).
- **C-17:** `.claude/settings.json` is **not hash-tracked.** The Ark-owned `SessionStart` entry is re-applied on every successful `init`, `load`, and `upgrade`, mirroring the `CLAUDE.md` managed-block treatment (upgrade SPEC C-8). The file is not in `manifest.files` and has no entry in `manifest.hashes`.
- **C-18:** `Snapshot` carries a `hook_bodies: Vec<SnapshotHookBody>` slot. `unload` captures the Ark entry; `load` re-applies via `update_settings_hook`. The `unload` path is **surgical** (calls `remove_settings_hook`, not a whole-file delete), so user-authored sibling entries persist on disk between `unload` and `load` even though `Snapshot::hook_bodies` only carries Ark's entry. (V-001 in the task's VERIFY documents the divergence between this and an earlier, more pessimistic plan wording.)
- **C-19 (Artifact iteration rule):** `gather_context` emits all files matching `^(\d{2})_PLAN\.md$` and `^(\d{2})_REVIEW\.md$` in the current task dir, sorted ascending by parsed `NN`. `ArtifactKind::iteration()` returns `Some(n)` for `Plan{n}` / `Review{n}`, `None` for others. Projections that need "latest" call `artifacts.iter().filter(...).max_by_key(|a| a.kind.iteration())`.
- **C-20 (Related-specs parser + projection filter):** PRD section parser:
  > Locate the line starting with `[**Related Specs**]`. Scan forward until the next line matching `^\[\*\*.*\*\*\]` or EOF. Extract every token matching `specs/features/[a-z0-9_-]+/SPEC\.md` (case-sensitive). Dedupe preserving first-seen order. Empty / missing section → empty vec, no error.
  Projection filter: a `SpecRow` `f` is kept iff any `r ∈ related_specs` satisfies `normalize(r).ends_with(&normalize(f.path))`, where `normalize` strips leading `./` and `.ark/`. Empty `related_specs` → empty features list.
- **C-21 (cwd discovery, narrowed per R-102):** `Layout::discover_from(cwd) -> Result<Self>` walks ancestors of `cwd` until `.ark/` is found, else returns `Error::NotLoaded`. Used by **commands that require an existing `.ark/`** — `context`, `unload`, `remove`, `upgrade`, and `load` *without* `--force`. **NOT** used by `init` or `load --force` (they scaffold a project, not locate one). `--dir` always wins over discovery.
- **C-22 (Git helper):** `ark-core/src/io/git.rs` exposes `run_git(args, cwd) -> Result<GitOutput>`. Non-zero exit returns `Ok(GitOutput { exit_code, .. })` (callers soft-fail on non-git dirs). Spawn failure returns `Error::GitSpawn { source }`.
- **C-23 (JSON output shape lock):** JSON-mode output is exactly `<rendered>\n`. Indent is 2 spaces. Field order follows `Serialize` derive order. For `--scope session --format json`, `<rendered>` is the SessionStart envelope; otherwise it is the raw projection.
- **C-24 (Specs-INDEX parser grammar):**
  - `specs/features/INDEX.md`: parsed via `read_managed_block(path, "ARK:FEATURES")` → 3-column GFM table (`Feature | Scope | Promoted`).
  - `specs/project/INDEX.md`: locate first line matching `^##\s+Index\b`, then the first GFM table.
  - Both grammars share `gfm_table_rows`, which iterates `|`-prefixed lines, splits cells, skips header (`Spec`/`Feature`), separator (`---`), and **placeholder rows** whose every cell is wrapped in `{...}` (filters out the shipped INDEX templates' `{e.g. rust/SPEC.md}` example row).
- **C-25 (Dirty files cap):** Named constant `DIRTY_FILES_CAP: usize = 20`.
- **C-26 (Process-spawn locality):** `std::process::Command::new` may be invoked **only** from `io/git.rs`. Enforced by the source-scan test `commands_no_bare_command_new` in `commands/context/mod.rs::tests`.
- **C-27 (Snapshot forward compatibility):** New fields added to `Snapshot` carry `#[serde(default)]`. `Snapshot::hook_bodies` defaults to an empty vec when absent. Older `.ark.db` files (pre-`hook_bodies`) deserialize successfully. `SCHEMA_VERSION` (the snapshot version) is **not** bumped — additive at the serde level.
- **C-28 (Process-spawn enforcement test):** A source-scan test (`commands_no_bare_command_new`) reads every non-test file under `commands/` via `include_str!` and asserts the literal `Command::new` does not appear. Mirrors `upgrade.rs`'s `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` pattern.
- **C-29 (Settings-hook upgrade idempotence):** Running `ark upgrade` twice in a row on a project initialized via `ark init` produces a byte-identical `.claude/settings.json`. Asserted by `upgrade_settings_hook_idempotent`.

[**Runtime**]

Main flow (5 steps): CLI parses → discover layout → gather → project → emit (with envelope wrapping iff session/json).

Failure flow: `.ark/` missing → `Error::NotLoaded`; `.current` dangling → `current_task: None`; `task.toml` corrupt → `Error::TaskTomlCorrupt`; specs INDEX malformed → empty list (silent in JSON, stderr warning in text); `git` missing from PATH → `Error::GitSpawn`; non-git directory → soft fail (`branch: "unknown"`); `update_settings_hook` write failure → `Error::Io`.

State transitions: `ark context` is stateless. The settings-hook surface has four states based on settings.json contents: (a) Ark entry present and canonical → stable; (b) Ark entry present but tampered → next `update_settings_hook` reverts to canonical; (c) Ark entry absent (file present) → next call inserts; (d) file absent → next call creates with only the Ark entry. `ark remove` deletes the Ark entry leaving the (possibly empty) `SessionStart` array in place.
