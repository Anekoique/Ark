# `improve-ark-context` PLAN `01`

> Status: Draft
> Feature: `ark-context`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

Additive growth of `ark context`'s projection surface so a single `--scope phase --for <phase>` call carries the workflow state introduced by features shipped after `ark-context` was first promoted: per-checkout `checkout` (root_kind + branch + focus_slug) on every projection; `specs.features_tree` on session + design; `subagents` (every installed agent stem per detected platform, not just Ark's three) on session + design / plan / review / verify; `record: Some(RecordProjection)` on commit scope. `SCHEMA_VERSION` stays at 1 (additive serde fields). Slash commands and other downstream consumers are NOT updated in this task — the new fields are additive and any consumer that wants them can read them. The reviewer/verifier-pick workflow continues to talk about the three reserved Ark canonical stems per `subagent-support` SPEC; it does not branch on arbitrary installed agents. Research tier reuses the design projection — `--for research` is not added, honoring `ark-research` NG-4. The existing SPEC at `specs/features/ark-context/SPEC.md` is updated in place with a `[**CHANGELOG**]` entry on commit; no new SPEC is promoted.

Iteration 01 fixes two HIGH and three MEDIUM findings from `00_REVIEW.md`: (1) Codex agent layout in C-41/V-UT-9 was the slash-command layout, not the agent layout — agents are flat `.toml` files under `.codex/agents/`; (2) `SubagentSet.platform` now reuses `Platform::cli_flag` ("claude" / "codex" / "opencode") instead of an undocumented hand-rolled tag; (3) envelope cap ceiling now references the in-code constant via new `C-45`; (4) `detect_checkout` validation split into "Main-by-main-checkout" and "Main-by-non-git-fallback" cases; (5) commit-scope `record` semantics clarified — `Some(RecordProjection::default())` means "no journal entries yet", documented in C-42. LOW findings (R-006 architecture-map wording, R-007 silent-skip filter constraint) addressed too.

## Log

[**Added**]

- New `C-44`: `SubagentSet.platform` derivation rule mapped to `Platform::cli_flag`.
- New `C-45`: SessionStart envelope cap codified by reference to the in-code constant in `commands/context/mod.rs` (`ADDITIONAL_CONTEXT_CAP`).
- New `V-UT-1a` / `V-UT-1b` split for `detect_checkout` worktree fixtures.
- Trade-off `T-7`: documents the "no journal yet" vs "skipped" disambiguation for commit-scope `record` (R-005 resolution).

[**Changed**]

- `C-38` rewritten — `SubagentSet.platform` reuses `Platform::cli_flag` ("claude" / "codex" / "opencode") rather than a hand-defined enumeration that did not match `Platform::id`.
- `C-41` rewritten — Codex stem derivation is `.toml` filename minus extension (matches `.codex/agents/<name>.toml`), not subdirectory + `SKILL.md`. Added the silent-skip clause for non-matching extensions per R-007.
- `V-UT-9` rewritten — asserts `.codex/agents/ark-reviewer.toml` → stem `ark-reviewer`.
- `V-F-3` rewritten — references the in-code envelope cap constant (`ADDITIONAL_CONTEXT_CAP`) rather than the prose "9,500-byte" number.
- `V-UT-1` split into `V-UT-1a` (Main-because-main-checkout, git repo present) and `V-UT-1b` (Worktree-because-`show-toplevel`-differs).
- `V-IT-1` clarified — fixture is `git init`'d as a real repo, so Main is the "main checkout" case rather than the "non-git fallback" case.
- Architecture-map line for `platforms.rs` annotated as `read-only consumer` (R-006).
- `C-42` extended with the "`session_count == 0` means no journal entries yet, not record-gather skip" disambiguation.

[**Removed**]

- None. Every prior `G-*` / `NG-*` / `C-1..C-43` from 00_PLAN is preserved verbatim.

[**Unresolved**]

- None — every CRITICAL / HIGH / MEDIUM / LOW finding from `00_REVIEW.md` resolved (see Response Matrix).

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 (HIGH) | Accepted | C-41 rewritten: Codex = filename minus `.toml` (flat `.codex/agents/<name>.toml`). V-UT-9 rewritten to match. The skills/`SKILL.md` layout cited in 00_PLAN was for Codex slash commands; agents use the flat `.toml` layout per `subagent-support` SPEC and on-disk `templates/codex/agents/`. |
| Review | R-002 (HIGH) | Accepted | C-38 rewritten: `SubagentSet.platform` reuses `Platform::cli_flag` ("claude" / "codex" / "opencode"). Added C-44 documenting the mapping rule explicitly. `Platform::id` ("claude-code" for Claude) is not used because `cli_flag` already provides the short normalized tag that downstream consumers expect. |
| Review | R-003 (MEDIUM) | Accepted | Added C-45 codifying the envelope cap by reference to the in-code constant `ADDITIONAL_CONTEXT_CAP`. V-F-3 rewritten to assert against that constant rather than the prose 9,500-byte number. |
| Review | R-004 (MEDIUM) | Accepted | Split V-UT-1 into V-UT-1a (Main-because-this-is-a-git-main-checkout) and V-UT-1b (Worktree-because-`show-toplevel`-differs). V-IT-1 fixture now explicitly initialized as a real git repo so Main is the main-checkout case, not the non-git fallback. V-UT-2 (Main-because-non-git) stands. |
| Review | R-005 (MEDIUM) | Accepted | C-42 extended with the disambiguation. Trade-off T-7 added explaining that commit-scope `record` is always `Some(_)` when scope is selected — `session_count == 0` means "no journal entries yet for this dev/branch", not "gather skipped"; slash commands branch on individual field values, not on the option. Chose option (a) from R-005 (`Some(_)` always) over option (b) (`None` when identity unresolvable) because the symmetry with `Scope::Record` is more valuable than an Option that consumers would re-check. |
| Review | R-006 (LOW) | Accepted | Architecture-map line for `platforms.rs` now reads `(read-only consumer; Platform::agents_dest_dir is what subagent enumeration scans)`. |
| Review | R-007 (LOW) | Accepted | C-41 extended with the silent-skip clause for non-matching extensions; V-E-5 already exists and now traces cleanly to the constraint. |
| Review | TR-1..TR-6 (advice) | All adopted | Reviewer recommended Adopt on all six trade-offs; no changes to T-1..T-6. T-7 added as the R-005 resolution. |

---

## Spec

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
- C-20: Related-specs parser unchanged from `detachable-feature-spec` C-11 / C-11a.
- C-21: `Layout::discover_from(cwd)` walks ancestors for `.ark/` and is used by commands requiring an existing project.
- C-22: `ark-core/src/io/git.rs` is the sole sanctioned `Command::new("git")` site; non-zero exit returns `Ok(GitOutput { exit_code, .. })`; spawn failure → `Error::GitSpawn`.
- C-23: JSON output is `<rendered>\n` with 2-space indent; for `--scope session --format json`, `<rendered>` is the SessionStart envelope.
- C-24: INDEX parsers: `specs/features/INDEX.md` via `read_managed_block("ARK:FEATURES")` (3 cols); `specs/project/INDEX.md` via first GFM table after `^##\s+Index\b`. Both skip header, separator, and `{...}`-wrapped placeholder rows.
- C-25: `DIRTY_FILES_CAP: usize = 20`.
- C-26: `std::process::Command::new` may be invoked only from `io/git.rs`; enforced by source-scan test `commands_no_bare_command_new`.
- C-27: Snapshot forward compat: new fields carry `#[serde(default)]`; `SCHEMA_VERSION` is not bumped on additive serde changes.
- C-28: Source-scan test `commands_no_bare_command_new` reads non-test files under `commands/` and asserts `Command::new` does not appear.
- C-29: `ark upgrade` twice in a row produces a byte-identical `.claude/settings.json`.
- C-30: `ProjectedContext.checkout` is populated on every projection (Session / Phase / Record); `ProjectedContext.subagents` is populated on Session and Phase(Design / Plan / Review / Verify) — empty Vec elsewhere; `ProjectedContext.specs.features_tree` is populated on Session and Phase(Design) only — `None` elsewhere; `ProjectedContext.record` is populated on Scope::Record and Phase(Commit).
- C-31: `CheckoutInfo.root_kind = Worktree` iff `git rev-parse --show-toplevel` from `layout.root()` differs from the project root resolved by walking `git rev-parse --git-common-dir`'s parent. Detection failure (non-git, spawn error) defaults to `root_kind = Main`.
- C-32: `CheckoutInfo.branch` is the same string as `GitState.branch`; no second `git` invocation.
- C-33: `CheckoutInfo.focus_slug` reads `state.focus` via `load_state(&layout)`; `None` when no focus is bound or when `.state.toml` is absent.
- C-34: `SpecNode::Branch.children` is sorted by `segment` ascending; `SpecNode::Leaf` siblings sort identically. Build order is deterministic across runs.
- C-35: `SpecNode` is `Some` iff `gather_context` produced a non-empty `features` Vec **and** the projection selected it (per C-30); `None` otherwise.
- C-36: `build_features_tree` recursion is bounded by `FEATURES_TREE_MAX_DEPTH = 8`, mirroring `gather`'s walker.
- C-37: `enumerate_subagents` scans only platform agent directories whose paths exist under `layout.root()`; absent directories produce no `SubagentSet` row (not an empty `stems` row).
- C-38: `SubagentSet.platform` is `Platform::cli_flag` verbatim — one of `"claude"`, `"codex"`, `"opencode"` — chosen because the `cli_flag` field is the existing normalized short tag (whereas `Platform::id` for Claude is `"claude-code"`, which downstream slash commands would otherwise have to special-case).
- C-39: `SubagentSet.stems` lists every agent stem found, **not** filtered to Ark canonicals; user-installed agents appear alongside `ark-researcher` / `ark-reviewer` / `ark-verifier`. Stems are sorted ascending.
- C-40: `enumerate_subagents` does not follow symlinks; entries whose `file_type` is symlink are skipped.
- C-41: Stem derivation per platform: Claude / OpenCode = filename with `.md` stripped (flat `.claude/agents/<name>.md`, `.opencode/agents/<name>.md`); Codex = filename with `.toml` stripped (flat `.codex/agents/<name>.toml`). Files whose extension does not match the platform's expected extension are skipped silently. The Codex *slash-command* layout (`.codex/skills/<name>/SKILL.md`) is out of scope for this scan.
- C-42: Commit-phase projection's `record` field is populated by reusing the same record-gather helper that powers `Scope::Record`. No duplication of journal-scan logic. The returned `RecordProjection` is always `Some(_)` when the scope is selected; `session_count == 0` means "no journal entries yet for this developer / branch", not "gather skipped" — slash commands branch on individual field values (`identity.is_none()` vs `session_count == 0`), not on the option.
- C-43: SessionStart envelope cap behavior unchanged: drop `archive` first, then truncate `tasks.active` to 5; new additive fields are NOT dropped (they are small and load-bearing for the agent's first message).
- C-44: `SubagentSet.platform` is derived by reading `Platform::cli_flag` directly — `enumerate_subagents` iterates `PLATFORMS` and emits each `cli_flag` as the platform tag. No hand-rolled mapping table; if `Platform::cli_flag` changes, `SubagentSet.platform` follows automatically.
- C-45: The SessionStart envelope's byte cap is sourced from the in-code constant `ADDITIONAL_CONTEXT_CAP` in `commands/context/mod.rs` (currently 9,500 bytes — documented as "Claude Code's 10K-character cap with envelope headroom"). The cap is unchanged by this task; new fields fit comfortably for typical projects (≤10 features, ≤6 stems per platform) per V-F-3.
- C-46: Text-mode `## SPECS` renders both project and feature rows in tree shape — indented branch lines for each directory / feature-path segment, leaves rendered as `<name> — <scope>` with one indent level per segment. There is no separate `## FEATURES TREE` text heading; the JSON `specs.features_tree` field remains the machine-readable nested view, while text mode collapses both surfaces into one `## SPECS` tree to avoid redundancy when features are flat single-segment leaves.

---

## Runtime

[**Main Flow**]

1. CLI parses `--scope` / `--for` / `--format`; constructs `ContextOptions`.
2. `context()` resolves `Layout`, errors `NotLoaded` if `.ark/` missing (existing C-14).
3. `gather_context()` produces a `Context` (single I/O pass — git, tasks, archive, specs walk, current-task focus + related-specs parse, **new**: checkout detect, features tree build, subagents enumerate).
4. `project()` reduces to `ProjectedContext` per `Scope` — new fields populated per C-30.
5. Format dispatch: JSON via `serde_json::to_string_pretty` (Session wraps in SessionStart envelope with cap-driven truncation per C-43); Text via `TextSummary` Display.
6. Single stdout write (C-7).

[**Failure Flow**]

1. `git` spawn failure → `Error::GitSpawn` (C-22). `git` non-zero exit returns `GitOutput { exit_code, .. }`; callers in `gather` interpret per shape (existing soft-fail for branch resolution preserved).
2. `detect_checkout` failure (non-git checkout, missing parent) defaults `root_kind = Main` (C-31). No error surfaced.
3. `enumerate_subagents` failure on a single platform directory (permission denied, broken symlink) skips that platform; other platforms still enumerate.
4. `build_features_tree` on an empty `features` Vec returns `None` (C-35).
5. Settings-hook write failures surface as `Error::Io { path: claude_settings, source }` (existing).

[**State Transitions**]

- `Context` always populated → `ProjectedContext` selects which Optional fields survive per scope, per C-30. No new lifecycle state.

---

## Implementation

[**Phase 1 — Core types + gather**]

1. Add `CheckoutInfo`, `CheckoutRootKind`, `SpecNode`, `SubagentSet`, `FEATURES_TREE_MAX_DEPTH` to `commands/context/model.rs`. Extend `Context` and `SpecsState` with the new fields.
2. Write `commands/context/checkout.rs::detect_checkout(&Layout) -> CheckoutInfo` — `git rev-parse --show-toplevel` + `--git-common-dir` comparison; reads `state.focus`. Unit tests via tempdir + real `git init` for the Main-because-main-checkout case and `git worktree add` for the Worktree case; non-git tempdir for the Main-because-non-git fallback.
3. Write `commands/context/spec_tree.rs::build_features_tree(&[SpecRow]) -> Option<SpecNode>` — pure grouping by `feature_path` segments; sorted child Vec. Unit tests cover flat-only, single-subtree, mixed, empty, max-depth.
4. Write `commands/context/subagents.rs::enumerate_subagents(&Layout) -> Vec<SubagentSet>` — iterates each `Platform`, stats each `agents_dest_dir`, derives stems per C-41 (filename minus the per-platform expected extension; silent-skip otherwise). Emits `Platform::cli_flag` as the `platform` tag per C-44. Unit tests cover each platform layout + Ark-canonical mixed with user agents.
5. Wire all three into `gather::gather_context`. Add to `mod.rs` `pub use`.

[**Phase 2 — Projection + render**]

1. Extend `commands/context/projection.rs::ProjectedContext` with `checkout` (always) and `subagents` (per C-30). Move `SpecsState`'s `features_tree` population through `project()` per C-30 (Session + Design carry it; other scopes set `None`).
2. Refactor the `Scope::Record` helper out of `mod.rs` into a small free function reachable from `projection.rs` so the Phase(Commit) arm can call it without re-implementing journal scan logic (C-42). Keep the helper's I/O scope identical.
3. Update `commands/context/render.rs::TextSummary` — add `## CHECKOUT` (always), `## SUBAGENTS` (when non-empty), and ensure commit scope renders `## RECORD` when populated. Restructure `## SPECS` to render both project and feature rows in tree shape (indented by directory / feature path); no separate `## FEATURES TREE` heading — the JSON `specs.features_tree` field is the machine-readable surface, text mode's `## SPECS` is the human-readable one (per C-46).

[**Phase 3 — SPEC update + verify gate**]

1. Confirm `## Spec` (this section) matches the final shape after any review iteration; `task commit` will overwrite `specs/features/ark-context/SPEC.md` with this body verbatim and append a CHANGELOG entry per `detachable-feature-spec` C-7.
2. Run smoke test from `CLAUDE.md` (build → load → unload → load → remove).
3. `cargo fmt --all`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`.

---

## Trade-offs

- T-1: **Additive vs schema-bump.** Picked additive (kept `SCHEMA_VERSION = 1`). Adv: zero breakage for downstream consumers that don't yet read new fields; satisfies the `ark-context` SPEC's stated additive-only contract; lets the projection contract land independently from any consumer update. Disadv: `ProjectedContext` grows a few `#[serde(skip_serializing_if = ...)]`-gated fields, slightly more shape variation per scope. Mitigated by C-30 which names exactly when each field is populated.

- T-2: **`features_tree` placement — session+design only vs every scope.** Picked session+design. Adv: smallest payload growth where it matters most (orientation + DESIGN navigation); plan/review already filter features down to related, so a tree there is mostly redundant; commit/execute/verify don't navigate features. Disadv: consumers see a shape that varies by scope. Mitigated by C-35 (deterministic `Option<SpecNode>` rule).

- T-3: **Subagent detection — manifest vs filesystem scan.** Picked filesystem scan. Adv: catches user-added agents (e.g., a project-local `code-reviewer` agent under `.claude/agents/`); robust to manifest drift; the slash command's "which reviewer?" prompt sees the truth. Disadv: per-platform stem derivation (C-41); one extra `read_dir` per detected platform. Mitigated by C-37 (skip absent dirs), C-40 (skip symlinks), and the silent-skip-on-extension-mismatch clause of C-41.

- T-4: **Commit-scope `record` field — reuse `RecordProjection` vs inline minimal fields.** Picked reuse. Adv: one schema across `Scope::Record` and `Scope::Phase(Commit)` — byte-identical for downstream consumers when they show up. Disadv: commit-scope payload grows by ~5 fields no consumer currently reads. Negligible (RecordProjection is ~80 bytes JSON) and keeps the contract clean for the first consumer that arrives.

- T-5: **No `--for research`.** Honored `ark-research` SPEC NG-4. Adv: research-tier slash command keeps using the design projection — no surface area churn for a tier that already lives without PLAN/REVIEW/VERIFY. Disadv: a future request to differentiate (e.g., suppress `archive` for research scope) would need a SPEC amendment first.

- T-6: **In-place SPEC update vs new SPEC.** This task modifies `specs/features/ark-context/SPEC.md` in place; no new feature directory is promoted. `task commit` already handles this via the CHANGELOG-on-overwrite path per `detachable-feature-spec` C-7. Adv: history of the SPEC stays linear at a single location. Disadv: requires the PLAN's `## Spec` to be a self-contained superset of the prior SPEC's body (which it is — every prior `G-*` / `C-*` is preserved or explicitly retired with rationale).

- T-7: **Commit-scope `record` always `Some(_)` vs `None` when identity unresolvable.** Picked always `Some(_)` (symmetric with `Scope::Record`). Adv: slash commands never need to re-check the option separately on commit vs record scope; the two scopes are byte-for-byte interchangeable for the `record` field (V-IT-3 asserts this). Disadv: a slash command that wants the "no developer registered" signal must branch on `record.identity.is_none()` rather than on `record.is_none()`. C-42's disambiguation clause documents the rule; the cost is one extra field-check on the consumer side, which is cheaper than the alternative — a non-symmetric Option that consumers would have to convert into the same field check anyway.

---

## Validation

[**Unit Tests**]

- V-UT-1a: `detect_checkout` returns `Main` when `Layout::root` points at a real git repo's main checkout (a tempdir initialized via `git init` where `git rev-parse --show-toplevel` equals `git rev-parse --git-common-dir`'s parent).
- V-UT-1b: `detect_checkout` returns `Worktree` when `Layout::root` points at a real git worktree (created via `git worktree add` from the V-UT-1a tempdir) and `git rev-parse --show-toplevel` differs from `--git-common-dir`'s parent.
- V-UT-2: `detect_checkout` defaults to `Main` when `git` is unavailable (non-git tempdir, no `.git/` directory).
- V-UT-3: `detect_checkout` populates `focus_slug` from a written `.state.toml` and `None` when absent.
- V-UT-4: `build_features_tree` over a flat `[ark-context, worktree]` Vec produces two leaf nodes.
- V-UT-5: `build_features_tree` over a `[klib, xemu/csr, xemu/io/mmio]` Vec produces one leaf (`klib`) and one branch (`xemu`) whose children are a leaf (`csr`) and a sub-branch (`xemu/io` with leaf `mmio`).
- V-UT-6: `build_features_tree` on empty Vec returns `None`; ordering of input doesn't change tree shape (deterministic sort).
- V-UT-7: `enumerate_subagents` on a tempdir with `.claude/agents/{ark-reviewer.md, code-reviewer.md}` returns one `SubagentSet { platform: "claude", stems: ["ark-reviewer", "code-reviewer"] }` (sorted, user agent included, `cli_flag` tag).
- V-UT-8: `enumerate_subagents` skips a platform dir that doesn't exist (no row emitted, not an empty stems row).
- V-UT-9: `enumerate_subagents` derives Codex stems from `.toml` filenames — `.codex/agents/ark-reviewer.toml` → stem `ark-reviewer`, emitted as `SubagentSet { platform: "codex", stems: [...] }`. Files with non-`.toml` extensions in the same dir are skipped silently.
- V-UT-10: `enumerate_subagents` skips symlinked entries.
- V-UT-11: `projection::project` sets `checkout` on Session, Phase(Design), Phase(Plan), Phase(Review), Phase(Execute), Phase(Verify), Phase(Commit), Record.
- V-UT-12: `projection::project` sets `features_tree = Some(_)` on Session + Design only (given a non-empty input); `None` on the other six scopes.
- V-UT-13: `projection::project` sets `subagents` non-empty on Session and Phase(Design / Plan / Review / Verify); empty Vec on Phase(Execute / Commit) and Record.
- V-UT-14: `projection::project` sets `record = Some(_)` on Phase(Commit) and Record; `None` elsewhere.

[**Integration Tests**]

- V-IT-1: `ark context --scope session --format json` in a `git init`'d tempdir with seeded `.claude/agents/` emits an envelope whose stringified `additionalContext` contains `"checkout": { "root_kind": "main", ...}` (main-checkout case, not non-git fallback) and a non-empty `"subagents"` array.
- V-IT-2: `ark context --scope phase --for design --format json` in a tempdir with a recursive features tree emits `"specs": { ..., "features_tree": { ... } }` with the expected nested shape.
- V-IT-3: `ark context --scope phase --for commit --format json` emits `"record": { "identity": ..., "active_journal_path": ..., "session_count": ... }` when a journal exists; emits `"record": { "identity": null, "active_journal_path": null, "session_count": 0, ... }` when none — asserting byte-for-byte parity with `--scope record` output on the same fixture (C-42 reuse contract).
- V-IT-4: `ark context --scope phase --for plan --format json` includes `subagents` but not `features_tree`.
- V-IT-5: `ark context --scope phase --for execute --format json` includes `checkout` but neither `features_tree` nor `subagents`.
- V-IT-6: `ark context` text mode includes `## CHECKOUT`, `## SUBAGENTS` (when populated), `## RECORD` (when populated) sub-sections in addition to the existing locked sections. `## SPECS` renders project and feature rows in tree shape per C-46; there is no separate `## FEATURES TREE` heading.
- V-IT-7: `commands_no_bare_command_new` source-scan test continues to pass after adding `checkout.rs` / `subagents.rs` / `spec_tree.rs` (C-26 / C-28).
- V-IT-8: `ark upgrade` round-trip with the new templates produces a byte-identical `.claude/settings.json` (C-29) and writes the updated slash command bodies.

[**Failure / Robustness**]

- V-F-1: `detect_checkout` in a fully non-git tempdir returns `Main`, never panics.
- V-F-2: `enumerate_subagents` on a platform dir with a broken symlink continues, emitting the rest of the stems.
- V-F-3: With the existing `ADDITIONAL_CONTEXT_CAP` constant in `commands/context/mod.rs` (C-45), an envelope with `checkout` + `subagents` populated for a fixture of ≤10 features and ≤6 stems per platform stays under the cap; `archive` is dropped first per C-43. Asserted by computing serialized envelope size and comparing against `ADDITIONAL_CONTEXT_CAP`.
- V-F-4: An older client deserializing a context payload with new fields it doesn't know about does not error (serde additive-fields contract; covered by a round-trip test).

[**Edge Cases**]

- V-E-1: Empty `features` Vec → `features_tree = None` on every scope.
- V-E-2: A project with zero installed agents (no `.claude/agents/`, no `.codex/agents/`, no `.opencode/agents/`) → `subagents = []` on every scope.
- V-E-3: A worktree whose `state.focus` slug doesn't match any active task → `focus_slug = Some("orphan-slug")` (truthful to disk; reconciliation is a separate task per `task-concurrency-control` G-2).
- V-E-4: Recursive `features_tree` at exactly `FEATURES_TREE_MAX_DEPTH = 8`; deeper subtrees are flattened or dropped per C-36 (mirrors `gather` walker bound).
- V-E-5: A platform whose `agents_dest_dir` contains a file with no matching extension (Claude / OpenCode: not `.md`; Codex: not `.toml`) is skipped silently without erroring (matches C-41 silent-skip clause).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-1, V-IT-2 (JSON snapshot still present) |
| G-2 | V-IT-1 (session), V-IT-4 (phase=plan), V-IT-3 (record reuse on commit) |
| G-3 | V-IT-1, V-IT-2 (schema=1 preserved); V-F-4 (additive forward-compat) |
| G-4 | V-IT-1 (envelope shape unchanged); existing `context_session_json_wraps_in_session_start_envelope` regression covers |
| G-5 | V-IT-3 (commit projection carries `record` paths, no bodies) |
| NG-4 | absence of any `--for research` arg in `ContextArgs` + projection match arm; no test fixture invokes that arg |
| C-7 | existing `ContextSummary` Display contract; one write per invocation |
| C-30 | V-UT-11, V-UT-12, V-UT-13, V-UT-14 |
| C-31 | V-UT-1a, V-UT-1b, V-UT-2, V-F-1 |
| C-32 | code review of `detect_checkout` (branch reads `GitState`, not a fresh `run_git`) |
| C-33 | V-UT-3 |
| C-34 | V-UT-5, V-UT-6 |
| C-35 | V-UT-12, V-E-1 |
| C-36 | V-E-4 |
| C-37 | V-UT-8, V-E-2 |
| C-38 | V-UT-7, V-UT-9 (each platform variant emitted with its `cli_flag` tag) |
| C-39 | V-UT-7 (code-reviewer alongside ark-reviewer) |
| C-40 | V-UT-10, V-F-2 |
| C-41 | V-UT-7 (Claude `.md` stem), V-UT-9 (Codex `.toml` stem + silent-skip on extension mismatch), V-E-5 (silent-skip edge case) |
| C-42 | V-IT-3 (commit scope's `record` matches `--scope record` shape byte-for-byte; documents "session_count == 0 means no entries yet") |
| C-43 | V-F-3 |
| C-44 | V-UT-7 + V-UT-9 (each row's `platform` field is the `Platform::cli_flag` of the underlying variant — `"claude"` / `"codex"` / `"opencode"`) |
| C-45 | V-F-3 (asserts the envelope-size budget by reference to `ADDITIONAL_CONTEXT_CAP`) |
| C-46 | V-IT-6 + render unit test `populated_sections_render_specs_as_tree` (project rows with `rust/` subdir indent; features with `xemu/csr` subdir indent) |
| C-1 through C-29 | unchanged from prior SPEC; existing regression tests apply |
