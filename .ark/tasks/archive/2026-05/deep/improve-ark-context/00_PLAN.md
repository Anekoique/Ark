# `improve-ark-context` PLAN `00`

> Status: Draft
> Feature: `ark-context`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none

---

## Summary

Additive growth of `ark context`'s projection surface so a single `--scope phase --for <phase>` call carries the workflow state introduced by features shipped after `ark-context` was first promoted: per-checkout `checkout` (root_kind + branch + focus_slug) on every projection; `specs.features_tree` on session + design; `subagents` (every installed agent stem per detected platform, not just Ark's three) on session + design / plan / review / verify; `record: Some(RecordProjection)` on commit scope. `SCHEMA_VERSION` stays at 1 (additive serde fields). Slash commands `/ark:design`, `/ark:commit`, `/ark:research` across Claude / Codex / OpenCode are updated to consume the new fields. Research tier reuses the design projection — `--for research` is not added, honoring `ark-research` NG-4. The existing SPEC at `specs/features/ark-context/SPEC.md` is updated in place with a `[**CHANGELOG**]` entry on commit; no new SPEC is promoted.

## Log `None in 00_PLAN`

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
    ├── platforms.rs                          (Platform::agents_dest_dir is the input
    │                                           subagent enumeration reads from)
    ├── io/
    │   ├── fs.rs                             (update_settings_hook, remove_settings_hook,
    │   │                                      read_settings_hook, ARK_CONTEXT_HOOK_COMMAND,
    │   │                                      ark_session_start_hook_entry)
    │   └── git.rs                            (sole sanctioned `Command::new("git")` site)
    ├── state/snapshot.rs                     (hook_bodies + SnapshotHookBody, #[serde(default)])
    └── commands/context/
        ├── mod.rs                            (entry, ContextOptions, SessionStart envelope)
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
  │     ├── **checkout::detect_checkout(&layout)** → CheckoutInfo    (new)
  │     ├── **spec_tree::build_features_tree(&features)** → SpecNode (new)
  │     └── **subagents::enumerate_subagents(&layout)** → Vec<...>   (new)
  ├── projected = projection::project(ctx, opts.scope)
  └── format-dispatch + SessionStart envelope wrapping               (existing)
```

`detect_checkout` reads `git rev-parse --show-toplevel` and `git rev-parse --git-common-dir`, classifies `root_kind = Worktree` when the toplevel differs from the common-dir's parent and `root_kind = Main` otherwise. `branch` reuses `GitState::branch` (no extra `run_git` call). `focus_slug` reads `state.focus` via the existing `load_state(&layout)`.

`build_features_tree` is a pure function over the already-collected `Vec<SpecRow>`: it groups by `feature_path[..n]` prefixes, building a `SpecNode` tree whose leaves carry row metadata and whose branches carry the segment name. Returns `None` when `features` is empty.

`enumerate_subagents` walks each `Platform`'s `agents_dest_dir` if present under `layout.root()`. For each, it lists immediate children and extracts stems per platform layout. It does **not** filter to Ark-canonical stems — user-installed agents appear too. Symlinks are not followed.

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
    pub platform: String,                         // "claude" | "codex" | "opencode"
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

pub fn detect_checkout(layout: &Layout) -> CheckoutInfo;                    // checkout.rs
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
- C-38: `SubagentSet.platform` is one of `"claude"`, `"codex"`, `"opencode"` — lowercase serde tag matches `templates::Platform`'s existing display form.
- C-39: `SubagentSet.stems` lists every agent stem found, **not** filtered to Ark canonicals; user-installed agents appear alongside `ark-researcher` / `ark-reviewer` / `ark-verifier`. Stems are sorted ascending.
- C-40: `enumerate_subagents` does not follow symlinks; entries whose `file_type` is symlink are skipped.
- C-41: Stem derivation per platform: Claude / OpenCode = filename with trailing `.md` stripped; Codex = subdirectory name (stem of the directory containing `SKILL.md`). Matches `subagent-support` SPEC's per-platform install layout.
- C-42: Commit-phase projection's `record` field is populated by reusing the same record-gather helper that powers `Scope::Record`. No duplication of journal-scan logic.
- C-43: SessionStart envelope cap behavior unchanged: drop `archive` first, then truncate `tasks.active` to 5; new additive fields are NOT dropped (they are small and load-bearing for the agent's first message).

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
2. Write `commands/context/checkout.rs::detect_checkout(&Layout) -> CheckoutInfo` — `git rev-parse --show-toplevel` + `--git-common-dir` comparison; reads `state.focus`. Unit tests via tempdir + injected git (or skip when git missing).
3. Write `commands/context/spec_tree.rs::build_features_tree(&[SpecRow]) -> Option<SpecNode>` — pure grouping by `feature_path` segments; sorted child Vec. Unit tests cover flat-only, single-subtree, mixed, empty, max-depth.
4. Write `commands/context/subagents.rs::enumerate_subagents(&Layout) -> Vec<SubagentSet>` — iterates each `Platform`, stats each `agents_dest_dir`, derives stems per C-41. Unit tests cover each platform layout + Ark-canonical mixed with user agents.
5. Wire all three into `gather::gather_context`. Add to `mod.rs` `pub use`.

[**Phase 2 — Projection + render**]

1. Extend `commands/context/projection.rs::ProjectedContext` with `checkout` (always) and `subagents` (per C-30). Move `SpecsState`'s `features_tree` population through `project()` per C-30 (Session + Design carry it; other scopes set `None`).
2. Refactor the `Scope::Record` helper out of `mod.rs` into a small free function reachable from `projection.rs` so the Phase(Commit) arm can call it without re-implementing journal scan logic (C-42). Keep the helper's I/O scope identical.
3. Update `commands/context/render.rs::TextSummary` — add `## CHECKOUT` (always), `## FEATURES TREE` (when present), `## SUBAGENTS` (when non-empty), and ensure commit scope renders `## RECORD` when populated.

[**Phase 3 — Slash commands**]

1. Update `templates/claude/commands/ark/design.md` — reference `checkout` and `subagents` in the `[USER]` "STOP and ask the user which reviewer" prompt so only installed agents are offered.
2. Update `templates/claude/commands/ark/commit.md` — read `record.identity` / `record.active_journal_path` / `record.session_count` from commit scope instead of separately calling `--scope record`.
3. Update `templates/claude/commands/ark/research.md` — reference `checkout.focus_slug` in the staging-step preflight.
4. Mirror the three edits to `templates/opencode/commands/ark/*.md` (byte-identical bodies modulo frontmatter per existing convention).
5. Mirror the three edits to `templates/codex/skills/ark-*/SKILL.md` (Codex substitution map: `/ark:<name>` → `ark-<name>`).
6. Update `.ark/workflow.md`'s `## CLI surfaces` block — mention the new fields in the `ark context` paragraph; keep additions ≤6 lines.

[**Phase 4 — SPEC update + verify gate**]

1. Confirm `## Spec` (this section) matches the final shape after any review iteration; `task commit` will overwrite `specs/features/ark-context/SPEC.md` with this body verbatim and append a CHANGELOG entry per `detachable-feature-spec` C-7.
2. Run smoke test from `CLAUDE.md` (build → load → unload → load → remove).
3. `cargo fmt --all`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`.

---

## Trade-offs

- T-1: **Additive vs schema-bump.** Picked additive (kept `SCHEMA_VERSION = 1`). Adv: zero breakage for in-flight slash commands that don't yet read new fields; can ship incrementally per template platform; satisfies the `ark-context` SPEC's stated additive-only contract. Disadv: `ProjectedContext` grows a few `#[serde(skip_serializing_if = ...)]`-gated fields, slightly more shape variation per scope. Mitigated by C-30 which names exactly when each field is populated.

- T-2: **`features_tree` placement — session+design only vs every scope.** Picked session+design. Adv: smallest payload growth where it matters most (orientation + DESIGN navigation); plan/review already filter features down to related, so a tree there is mostly redundant; commit/execute/verify don't navigate features. Disadv: consumers see a shape that varies by scope. Mitigated by C-35 (deterministic `Option<SpecNode>` rule).

- T-3: **Subagent detection — manifest vs filesystem scan.** Picked filesystem scan. Adv: catches user-added agents (e.g., a project-local `code-reviewer` agent under `.claude/agents/`); robust to manifest drift; the slash command's "which reviewer?" prompt sees the truth. Disadv: per-platform stem derivation (C-41); one extra `read_dir` per detected platform. Mitigated by C-37 (skip absent dirs) and C-40 (skip symlinks).

- T-4: **Commit-scope `record` field — reuse `RecordProjection` vs inline minimal fields.** Picked reuse. Adv: one schema, two consumers; `/ark:commit` and `/ark:record` share rendering logic. Disadv: commit-scope payload grows by ~5 fields the slash command may not all read. Negligible (RecordProjection is ~80 bytes JSON).

- T-5: **No `--for research`.** Honored `ark-research` SPEC NG-4. Adv: research-tier slash command keeps using the design projection — no surface area churn for a tier that already lives without PLAN/REVIEW/VERIFY. Disadv: a future request to differentiate (e.g., suppress `archive` for research scope) would need a SPEC amendment first.

- T-6: **In-place SPEC update vs new SPEC.** This task modifies `specs/features/ark-context/SPEC.md` in place; no new feature directory is promoted. `task commit` already handles this via the CHANGELOG-on-overwrite path per `detachable-feature-spec` C-7. Adv: history of the SPEC stays linear at a single location. Disadv: requires the PLAN's `## Spec` to be a self-contained superset of the prior SPEC's body (which it is — every prior `G-*` / `C-*` is preserved or explicitly retired with rationale).

---

## Validation

[**Unit Tests**]

- V-UT-1: `detect_checkout` returns `Main` in a freshly-init'd tempdir + `Worktree` when invoked from inside `.ark/worktrees/<branch>/`.
- V-UT-2: `detect_checkout` defaults to `Main` when `git` is unavailable (non-git tempdir).
- V-UT-3: `detect_checkout` populates `focus_slug` from a written `.state.toml` and `None` when absent.
- V-UT-4: `build_features_tree` over a flat `[ark-context, worktree]` Vec produces two leaf nodes.
- V-UT-5: `build_features_tree` over a `[klib, xemu/csr, xemu/io/mmio]` Vec produces one leaf (`klib`) and one branch (`xemu`) whose children are a leaf (`csr`) and a sub-branch (`xemu/io` with leaf `mmio`).
- V-UT-6: `build_features_tree` on empty Vec returns `None`; ordering of input doesn't change tree shape (deterministic sort).
- V-UT-7: `enumerate_subagents` on a tempdir with `.claude/agents/{ark-reviewer.md, code-reviewer.md}` returns one `SubagentSet { platform: "claude", stems: ["ark-reviewer", "code-reviewer"] }` (sorted, user agent included).
- V-UT-8: `enumerate_subagents` skips a platform dir that doesn't exist (no row emitted, not an empty stems row).
- V-UT-9: `enumerate_subagents` derives Codex stems from subdirectory names (`.codex/skills/ark-reviewer/SKILL.md` → stem `ark-reviewer`).
- V-UT-10: `enumerate_subagents` skips symlinked entries.
- V-UT-11: `projection::project` sets `checkout` on Session, Phase(Design), Phase(Plan), Phase(Review), Phase(Execute), Phase(Verify), Phase(Commit), Record.
- V-UT-12: `projection::project` sets `features_tree = Some(_)` on Session + Design only (given a non-empty input); `None` on the other six scopes.
- V-UT-13: `projection::project` sets `subagents` non-empty on Session and Phase(Design / Plan / Review / Verify); empty Vec on Phase(Execute / Commit) and Record.
- V-UT-14: `projection::project` sets `record = Some(_)` on Phase(Commit) and Record; `None` elsewhere.

[**Integration Tests**]

- V-IT-1: `ark context --scope session --format json` in a tempdir with seeded `.claude/agents/` emits an envelope whose stringified `additionalContext` contains `"checkout": { "root_kind": "main", ...}` and a non-empty `"subagents"` array.
- V-IT-2: `ark context --scope phase --for design --format json` in a tempdir with a recursive features tree emits `"specs": { ..., "features_tree": { ... } }` with the expected nested shape.
- V-IT-3: `ark context --scope phase --for commit --format json` emits `"record": { "identity": ..., "active_journal_path": ..., "session_count": ... }` when a journal exists; emits `"record": { "identity": null, "active_journal_path": null, "session_count": 0, ... }` when none.
- V-IT-4: `ark context --scope phase --for plan --format json` includes `subagents` but not `features_tree`.
- V-IT-5: `ark context --scope phase --for execute --format json` includes `checkout` but neither `features_tree` nor `subagents`.
- V-IT-6: `ark context` text mode includes `## CHECKOUT`, `## SUBAGENTS` (when populated), `## FEATURES TREE` (when present), `## RECORD` (when populated) sub-sections in addition to the existing locked sections.
- V-IT-7: `commands_no_bare_command_new` source-scan test continues to pass after adding `checkout.rs` / `subagents.rs` / `spec_tree.rs` (C-26 / C-28).
- V-IT-8: `ark upgrade` round-trip with the new templates produces a byte-identical `.claude/settings.json` (C-29) and writes the updated slash command bodies.

[**Failure / Robustness**]

- V-F-1: `detect_checkout` in a fully non-git tempdir returns `Main`, never panics.
- V-F-2: `enumerate_subagents` on a platform dir with a broken symlink continues, emitting the rest of the stems.
- V-F-3: SessionStart envelope cap drops `archive` first; with `checkout` + `subagents` populated, the cap math still fits the documented 9,500-byte ceiling for a typical project (≤10 features, ≤6 stems per platform).
- V-F-4: An older client deserializing a context payload with new fields it doesn't know about does not error (serde additive-fields contract; covered by a round-trip test).

[**Edge Cases**]

- V-E-1: Empty `features` Vec → `features_tree = None` on every scope.
- V-E-2: A project with zero installed agents (no `.claude/agents/`, no `.codex/skills/`, etc.) → `subagents = []` on every scope.
- V-E-3: A worktree whose `state.focus` slug doesn't match any active task → `focus_slug = Some("orphan-slug")` (truthful to disk; reconciliation is a separate task per `task-concurrency-control` G-2).
- V-E-4: Recursive `features_tree` at exactly `FEATURES_TREE_MAX_DEPTH = 8`; deeper subtrees are flattened or dropped per C-36 (mirrors `gather` walker bound).
- V-E-5: A platform whose `agents_dest_dir` contains a file with no recognized stem suffix (Claude: not `.md`) is skipped without erroring.

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
| C-31 | V-UT-1, V-UT-2, V-F-1 |
| C-32 | code review of `detect_checkout` (branch reads `GitState`, not a fresh `run_git`) |
| C-33 | V-UT-3 |
| C-34 | V-UT-5, V-UT-6 |
| C-35 | V-UT-12, V-E-1 |
| C-36 | V-E-4 |
| C-37 | V-UT-8, V-E-2 |
| C-38 | V-UT-7, V-UT-9 (each platform variant emitted with documented tag) |
| C-39 | V-UT-7 (code-reviewer alongside ark-reviewer) |
| C-40 | V-UT-10, V-F-2 |
| C-41 | V-UT-7 (Claude `.md` stem), V-UT-9 (Codex dir stem) |
| C-42 | V-IT-3 (commit scope's `record` matches `--scope record` shape byte-for-byte) |
| C-43 | V-F-3 |
| C-1 through C-29 | unchanged from prior SPEC; existing regression tests apply |
