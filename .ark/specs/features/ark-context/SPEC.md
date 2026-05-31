
> This section is the durable design record. On deep-tier commit, it is copied **verbatim** into `specs/features/ark-context/SPEC.md` (overwriting the existing body); a `[**CHANGELOG**]` entry is appended.

[**Goals**]

- G-1: `ark context` prints a JSON or text snapshot of git + tasks + specs + recent archive.
- G-2: `--scope {session|phase|record}` selects breadth; `--for <phase>` targets one phase.
- G-3: JSON payloads carry `"schema": 1`; the schema is additive-only.
- G-4: A `SessionStart` hook installed in `.claude/settings.json` invokes `ark context` automatically per session.
- G-5: Slash commands consume the projection; the `commit` projection is body-free (paths only).

[**Non-goals**]

- NG-1: No mutation; read-only command.
- NG-2: No file bodies inlined in the projection — paths and summaries only.
- NG-3: No caching layer; every invocation re-reads state.
- NG-4: No `--for research` phase projection; the existing `design` projection serves research tasks (mirrors `ark-research` SPEC NG-4).

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                       (Context(ContextArgs) top-level)
└── ark-core/src/
    ├── error.rs                              (Error::GitSpawn)
    ├── layout.rs                             (claude_settings, specs_project_dir,
    │                                           specs_project_index, discover_from)
    ├── platforms.rs                          (read-only consumer; Platform::agents_dest_dir
    │                                           is what subagent enumeration scans;
    │                                           Platform::cli_flag is what SubagentSet.platform
    │                                           reuses verbatim)
    ├── io/
    │   ├── fs.rs                             (update_settings_hook, remove_settings_hook,
    │   │                                      read_settings_hook, ARK_CONTEXT_HOOK_COMMAND,
    │   │                                      ark_session_start_hook_entry)
    │   └── git.rs                            (sole sanctioned `Command::new("git")` site)
    ├── state/snapshot.rs                     (hook_bodies + SnapshotHookBody, #[serde(default)])
    └── commands/context/
        ├── mod.rs                            (entry, ContextOptions, SessionStart envelope;
        │                                      hosts ADDITIONAL_CONTEXT_CAP)
        ├── gather.rs                         (single-pass collection; adds checkout,
        │                                      features_tree, subagents enumeration)
        ├── model.rs                          (Context + sub-structs, SCHEMA_VERSION, caps;
        │                                      adds CheckoutInfo, SpecNode, SubagentSet)
        ├── projection.rs                     (Scope, PhaseFilter, project(); per-phase
        │                                      placement of new fields)
        ├── render.rs                         (text-mode Display; new sub-sections)
        ├── related_specs.rs                  (PRD `[**Related Specs**]` parser; unchanged)
        ├── checkout.rs                       (NEW; worktree detection)
        ├── spec_tree.rs                      (NEW; flat Vec<SpecRow> → tree builder)
        └── subagents.rs                      (NEW; per-platform agent stem scan)
```

Module coupling: `mod.rs → gather → model`; `mod.rs → projection → model`; `mod.rs → render → model`. `related_specs.rs` and `io/git.rs` remain leaves used only by `gather.rs`. New leaves `checkout.rs`, `subagents.rs`, `spec_tree.rs` are also gather-only.

Call graph for `ark context` (additions in **bold**):

```
context(opts)
  ├── layout = Layout::new (CLI resolves with discovery)
  ├── ctx = gather::gather_context(&layout)
  │     ├── run_git → branch / status / log                          (existing)
  │     ├── list active tasks                                        (existing)
  │     ├── list archive (5 most recent)                             (existing)
  │     ├── parse specs/project/INDEX.md                             (existing)
  │     ├── parse specs/features/INDEX.md (recursive walk)           (existing)
  │     ├── if focus exists: current_task + related_specs            (existing)
  │     ├── **checkout::detect_checkout(&layout, branch)** → CheckoutInfo (new)
  │     ├── **spec_tree::build_features_tree(&features)** → SpecNode (new)
  │     └── **subagents::enumerate_subagents(&layout)** → Vec<...>   (new)
  ├── projected = projection::project(ctx, opts.scope)
  └── format-dispatch + SessionStart envelope wrapping               (existing)
```

`detect_checkout` reads `git rev-parse --show-toplevel` and `git rev-parse --git-common-dir`, classifies `root_kind = Worktree` when the toplevel differs from the common-dir's parent and `root_kind = Main` otherwise. The `branch` arg is the already-resolved `GitState::branch` value threaded in by the caller (no second `git rev-parse --abbrev-ref` call, per C-32). `focus_slug` reads `state.focus` via the existing `load_state(&layout)`.

`build_features_tree` is a pure function over the already-collected `Vec<SpecRow>`: it groups by `feature_path[..n]` prefixes, building a `SpecNode` tree whose leaves carry row metadata and whose branches carry the segment name. Returns `None` when `features` is empty.

`enumerate_subagents` walks each `Platform`'s `agents_dest_dir` if present under `layout.root()`. For each, it lists immediate children, filters to files whose extension matches the platform's expected extension (`.md` for Claude / OpenCode; `.toml` for Codex), and emits stems. It does **not** filter to Ark-canonical stems — user-installed agents appear too. Symlinks are not followed.

[**Data Structure**]

```rust
// ark-core/src/commands/context/model.rs

pub const SCHEMA_VERSION: u32 = 1;
pub const DIRTY_FILES_CAP: usize = 20;
pub const RECENT_COMMITS_CAP: usize = 5;
pub const ARCHIVE_CAP: usize = 5;
pub const FEATURES_TREE_MAX_DEPTH: usize = 8;     // mirrors gather walker bound

pub struct Context {
    pub schema: u32,
    pub generated_at: DateTime<Utc>,
    pub project_root: PathBuf,
    pub git: GitState,
    pub tasks: TasksState,
    pub specs: SpecsState,
    pub archive: ArchiveState,
    pub current_task: Option<CurrentTask>,
    pub checkout: CheckoutInfo,                   // NEW; always populated
    pub subagents: Vec<SubagentSet>,              // NEW; empty Vec if no platforms detected
}

pub struct GitState {
    pub branch: String,
    pub head_short: String,
    pub is_clean: bool,
    pub uncommitted_changes: u32,
    pub dirty_files: Vec<String>,
    pub recent_commits: Vec<GitCommit>,
}

pub struct GitCommit { pub hash: String, pub message: String }

pub struct TasksState  { pub active: Vec<TaskSummary> }

pub struct SpecsState {
    pub project: Vec<SpecRow>,
    pub features: Vec<SpecRow>,
    pub features_warnings: Vec<GatherWarning>,    // existing
    pub features_tree: Option<SpecNode>,          // NEW; Some only when projected on
                                                  //      Session / Design and non-empty
}

pub struct ArchiveState { pub recent: Vec<ArchivedTask> }

pub struct TaskSummary {
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub iteration: u32,
    pub path: PathBuf,
    pub updated_at: DateTime<Utc>,
}

pub struct SpecRow {
    pub name: String,
    pub path: PathBuf,
    pub feature_path: Vec<String>,
    pub scope: String,
    pub promoted: Option<String>,
}

#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GatherWarning {
    MissingChild  { row: String, expected_path: PathBuf },
    OrphanLeaf    { path: PathBuf },
    OrphanSubtree { path: PathBuf },
}

pub struct ArchivedTask {
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub archived_at: DateTime<Utc>,
    pub path: PathBuf,
}

pub struct CurrentTask {
    pub slug: String,
    pub summary: TaskSummary,
    pub artifacts: Vec<ArtifactSummary>,
    pub related_specs: Vec<String>,
}

pub struct ArtifactSummary { pub kind: ArtifactKind, pub path: PathBuf, pub lines: u32 }

#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ArtifactKind {
    Prd,
    Plan { iteration: u32 },
    Review { iteration: u32 },
    Verify,
    TaskToml,
}
impl ArtifactKind {
    pub fn iteration(&self) -> Option<u32>;
}

// NEW types.

/// Per-checkout location info. `root_kind = Worktree` when this checkout's
/// project root differs from the parent checkout's project root.
pub struct CheckoutInfo {
    pub root_kind: CheckoutRootKind,              // Main | Worktree
    pub branch: String,                           // mirrors GitState::branch
    pub focus_slug: Option<String>,               // from state.toml [focus]
}

#[serde(rename_all = "lowercase")]
pub enum CheckoutRootKind { Main, Worktree }

/// One node in the feature SPECs tree, projected from flat SpecRows by
/// grouping their `feature_path` segments.
#[serde(untagged)]
pub enum SpecNode {
    Branch {
        segment: String,                          // path component at this depth
        children: Vec<SpecNode>,                  // sorted by `segment` asc
    },
    Leaf {
        segment: String,                          // last `feature_path` segment
        name: String,                             // mirrors SpecRow.name
        path: PathBuf,                            // mirrors SpecRow.path
        scope: String,
        promoted: Option<String>,
    },
}

/// Installed-agent enumeration for one platform on this checkout's disk.
pub struct SubagentSet {
    pub platform: String,                         // verbatim `Platform::cli_flag`
                                                  // ("claude" | "codex" | "opencode")
    pub stems: Vec<String>,                       // sorted ascending; may include user agents
}

// ark-core/src/commands/context/projection.rs

pub enum Scope { Session, Phase(PhaseFilter), Record }

#[serde(rename_all = "lowercase")]
pub enum PhaseFilter { Design, Plan, Review, Execute, Verify, Commit }

#[serde(tag = "scope", rename_all = "lowercase")]
pub enum ScopeTag { Session, Phase { phase: PhaseFilter }, Record }

pub struct ProjectedContext {
    pub schema: u32,
    #[serde(flatten)]
    pub scope: ScopeTag,
    pub generated_at: DateTime<Utc>,
    pub project_root: PathBuf,
    pub git: GitState,
    pub checkout: CheckoutInfo,                   // NEW; always serialized
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TasksState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs: Option<SpecsState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<RecordProjection>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub subagents: Vec<SubagentSet>,              // NEW; populated per C-30
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

pub struct RecordProjection {                     // existing; unchanged shape
    pub identity: Option<String>,
    pub active_journal_path: Option<String>,
    pub journal_max_lines: usize,
    pub session_count: u32,
    pub branch: Option<String>,
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
    pub fn claude_settings(&self) -> PathBuf;
    pub fn specs_project_dir(&self) -> PathBuf;
    pub fn specs_project_index(&self) -> PathBuf;
    pub fn discover_from(cwd: impl AsRef<Path>) -> Result<Self>;
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

// ark-core/src/error.rs (additions — unchanged from prior SPEC)
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
    pub rendered: String,
    pub schema: u32,
}

pub fn context(opts: ContextOptions) -> Result<ContextSummary>;

// Existing private const that C-45 elevates to a referenced contract:
const ADDITIONAL_CONTEXT_CAP: usize = 9_500;

// CLI shape (ark-cli/src/main.rs) — unchanged

#[derive(Subcommand)]
enum Command {
    Init(InitArgs),
    Load(LoadArgs),
    Unload(TargetArgs),
    Remove(TargetArgs),
    Upgrade(UpgradeArgs),
    Context(ContextArgs),
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

// NEW pure-function leaves used by gather:

pub fn detect_checkout(layout: &Layout, branch: &str) -> CheckoutInfo;      // checkout.rs
pub fn build_features_tree(rows: &[SpecRow]) -> Option<SpecNode>;           // spec_tree.rs
pub fn enumerate_subagents(layout: &Layout) -> Vec<SubagentSet>;            // subagents.rs
```

`TargetArgs` continues to expose `resolve_with_discovery()` for commands that require an existing project.

Library re-exports from `ark-core/src/lib.rs` add: `CheckoutInfo`, `CheckoutRootKind`, `SpecNode`, `SubagentSet` to the existing context-module exports.

[**Constraints**]

- C-1: @judgment
`ark context` is listed in `ark --help`; no `#[command(hide = true)]`.
- C-2: @judgment
`ark context --help` mentions neither "hidden" nor "not covered by semver".
- C-3: @test-binding: context_session_json_wraps_in_session_start_envelope
JSON output's first field is `"schema": 1`.
- C-4: @source-scan: std::fs:: @ crates/ark-core/src/commands/context/**/*.rs
All filesystem access in `commands/context/` routes through `io::PathExt` / `io::fs`.
- C-5: @judgment
All `.ark/`-relative path composition routes through `layout::Layout`.
- C-6: @judgment
`Context` field order is the JSON schema source of truth; renames/removes bump `SCHEMA_VERSION`, adds are free.
- C-7: @judgment
`ark context` emits exactly one stdout write per invocation.
- C-8: @judgment
`gather_context` caps dirty_files at 20 and reads at most 5 commits from `git log`.
- C-9: @judgment
Archive listing reads at most 5 most-recent subdirs, sorted by `archived_at` descending.
- C-10: @judgment
Text mode is not machine-parseable and carries no schema version.
- C-11: @judgment
`SessionStart` hook entry identity is `entry.hooks[*].command == ARK_CONTEXT_HOOK_COMMAND`.
- C-12: @test-binding: update_settings_hook_is_idempotent_on_repeat
`update_settings_hook` is idempotent — running `ark init` twice produces a byte-identical `settings.json`.
- C-13: @test-binding: gather_on_empty_ark_returns_empty_state
Empty state never errors; `ark context` returns valid empty vecs.
- C-14: @test-binding: context_errors_on_non_ark_dir
Non-Ark directory → `Error::NotLoaded`.
- C-15: @judgment
SessionStart hook timeout is 5000ms.
- C-16: @test-binding: update_settings_hook_preserves_unrelated_pretooluse_entries
Settings-hook helpers touch only the Ark-owned entry; sibling user hooks and unrelated top-level keys are preserved.
- C-17: @judgment
`.claude/settings.json` is not hash-tracked; the Ark entry is re-applied on every `init` / `load` / `upgrade`.
- C-18: @judgment
`Snapshot::hook_bodies` is `#[serde(default)]`; older `.ark.db` files (pre-`hook_bodies`) deserialize successfully.
- C-19: @test-binding: artifact_kind_iteration_returns_some_for_plan_and_review
Artifact iteration files match `^(NN)_PLAN\.md$` and `^(NN)_REVIEW\.md$`, sorted ascending; "latest" = `max_by_key(iteration())`.
- C-20: @judgment
Related-specs parser unchanged from `detachable-feature-spec` C-11 / C-11a.
- C-21: @judgment
`Layout::discover_from(cwd)` walks ancestors for `.ark/` and is used by commands requiring an existing project.
- C-22: @source-scan: Command::new @ crates/ark-core/src/commands/**/*.rs
`ark-core/src/io/git.rs` is the sole sanctioned `Command::new("git")` site; non-zero exit returns `Ok(GitOutput { exit_code, .. })`; spawn failure → `Error::GitSpawn`.
- C-23: @judgment
JSON output is `<rendered>\n` with 2-space indent; for `--scope session --format json`, `<rendered>` is the SessionStart envelope.
- C-24: @test-binding: gather_features_index_parses_managed_block
INDEX parsers: `specs/features/INDEX.md` via `read_managed_block("ARK:FEATURES")` (3 cols); `specs/project/INDEX.md` via first GFM table after `^##\s+Index\b`. Both skip header, separator, and `{...}`-wrapped placeholder rows.
- C-25: @judgment
`DIRTY_FILES_CAP: usize = 20`.
- C-26: @test-binding: commands_no_bare_command_new
`std::process::Command::new` may be invoked only from `io/git.rs`; enforced by source-scan test `commands_no_bare_command_new`.
- C-27: @judgment
Snapshot forward compat: new fields carry `#[serde(default)]`; `SCHEMA_VERSION` is not bumped on additive serde changes.
- C-28: @test-binding: commands_no_bare_command_new
Source-scan test `commands_no_bare_command_new` reads non-test files under `commands/` and asserts `Command::new` does not appear.
- C-29: @judgment
`ark upgrade` twice in a row produces a byte-identical `.claude/settings.json`.
- C-30: @judgment
`ProjectedContext.checkout` is populated on every projection (Session / Phase / Record); `ProjectedContext.subagents` is populated on Session and Phase(Design / Plan / Review / Verify) — empty Vec elsewhere; `ProjectedContext.specs.features_tree` is populated on Session and Phase(Design) only — `None` elsewhere; `ProjectedContext.record` is populated on Scope::Record and Phase(Commit).
- C-31: @test-binding: returns_main_when_layout_is_main_checkout
`CheckoutInfo.root_kind = Worktree` iff `git rev-parse --show-toplevel` from `layout.root()` differs from the project root resolved by walking `git rev-parse --git-common-dir`'s parent. Detection failure (non-git, spawn error) defaults to `root_kind = Main`.
- C-32: @judgment
`CheckoutInfo.branch` is the same string as `GitState.branch`; no second `git` invocation.
- C-33: @test-binding: focus_slug_reads_state_toml_when_bound
`CheckoutInfo.focus_slug` reads `state.focus` via `load_state(&layout)`; `None` when no focus is bound or when `.state.toml` is absent.
- C-34: @test-binding: mixed_input_groups_by_subtree
`SpecNode::Branch.children` is sorted by `segment` ascending; `SpecNode::Leaf` siblings sort identically. Build order is deterministic across runs.
- C-35: @test-binding: empty_input_returns_none
`SpecNode` is `Some` iff `gather_context` produced a non-empty `features` Vec **and** the projection selected it (per C-30); `None` otherwise.
- C-36: @test-binding: rows_beyond_max_depth_are_dropped
`build_features_tree` recursion is bounded by `FEATURES_TREE_MAX_DEPTH = 8`, mirroring `gather`'s walker.
- C-37: @test-binding: skips_platform_dir_that_does_not_exist
`enumerate_subagents` scans only platform agent directories whose paths exist under `layout.root()`; absent directories produce no `SubagentSet` row (not an empty `stems` row).
- C-38: @test-binding: enumerates_claude_stems_with_user_agents
`SubagentSet.platform` is `Platform::cli_flag` verbatim — one of `"claude"`, `"codex"`, `"opencode"` — chosen because the `cli_flag` field is the existing normalized short tag (whereas `Platform::id` for Claude is `"claude-code"`, which downstream slash commands would otherwise have to special-case).
- C-39: @test-binding: enumerates_claude_stems_with_user_agents
`SubagentSet.stems` lists every agent stem found, **not** filtered to Ark canonicals; user-installed agents appear alongside `ark-researcher` / `ark-reviewer` / `ark-verifier`. Stems are sorted ascending.
- C-40: @test-binding: skips_symlinked_entries
`enumerate_subagents` does not follow symlinks; entries whose `file_type` is symlink are skipped.
- C-41: @test-binding: enumerates_codex_stems_from_toml_files
Stem derivation per platform: Claude / OpenCode = filename with `.md` stripped (flat `.claude/agents/<name>.md`, `.opencode/agents/<name>.md`); Codex = filename with `.toml` stripped (flat `.codex/agents/<name>.toml`). Files whose extension does not match the platform's expected extension are skipped silently. The Codex *slash-command* layout (`.codex/skills/<name>/SKILL.md`) is out of scope for this scan.
- C-42: @test-binding: commit_phase_yields_paths_only_no_bodies
Commit-phase projection's `record` field is populated by reusing the same record-gather helper that powers `Scope::Record`. No duplication of journal-scan logic. The returned `RecordProjection` is always `Some(_)` when the scope is selected; `session_count == 0` means "no journal entries yet for this developer / branch", not "gather skipped" — slash commands branch on individual field values (`identity.is_none()` vs `session_count == 0`), not on the option.
- C-43: @judgment
SessionStart envelope cap behavior unchanged: drop `archive` first, then truncate `tasks.active` to 5; new additive fields are NOT dropped (they are small and load-bearing for the agent's first message).
- C-44: @judgment
`SubagentSet.platform` is derived by reading `Platform::cli_flag` directly — `enumerate_subagents` iterates `PLATFORMS` and emits each `cli_flag` as the platform tag. No hand-rolled mapping table; if `Platform::cli_flag` changes, `SubagentSet.platform` follows automatically.
- C-45: @judgment
The SessionStart envelope's byte cap is sourced from the in-code constant `ADDITIONAL_CONTEXT_CAP` in `commands/context/mod.rs` (currently 9,500 bytes — documented as "Claude Code's 10K-character cap with envelope headroom"). The cap is unchanged by this task; new fields fit comfortably for typical projects (≤10 features, ≤6 stems per platform) per V-F-3.
- C-46: @test-binding: populated_sections_render_specs_as_tree
Text-mode `## SPECS` renders both project and feature rows in tree shape — indented branch lines for each directory / feature-path segment, leaves rendered as `<name> — <scope>` with one indent level per segment. There is no separate `## FEATURES TREE` text heading; the JSON `specs.features_tree` field remains the machine-readable nested view, while text mode collapses both surfaces into one `## SPECS` tree to avoid redundancy when features are flat single-segment leaves.

---

[**CHANGELOG**]

- 2026-05-24: replaced from 01_PLAN.md (prior body preserved in git history)
