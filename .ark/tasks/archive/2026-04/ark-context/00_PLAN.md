# `ark-context` PLAN `00`

> Status: Draft
> Feature: `ark-context`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Introduce `ark context`, a top-level read-only subcommand that emits a versioned JSON (or text) snapshot of git + `.ark/` workflow state. Two orthogonal flags control the payload: `--scope {session|phase}` selects which **projection** runs, and `--for {design|plan|review|execute|verify}` further trims the `phase` projection to what a given workflow step needs. Session-scoped output is auto-invoked via a new `SessionStart` hook rendered into `.claude/settings.json` at `ark init` time. Phase-scoped output is invoked explicitly inside each shipped slash command's prompt.

Implementation lives in a new module `ark-core/src/commands/context/` split into a **data-gathering engine** (one pass over git + tasks + specs, produces a full `Context` struct) and a set of **projections** (methods that render subsets). JSON is the source of truth (`schema = 1`); text rendering wraps it. All filesystem access routes through `io::PathExt` and `layout::Layout` per existing code conventions.

## Log `{None in 00_PLAN}`

[**Added**]
*None — 00_PLAN.*

[**Changed**]
*None — 00_PLAN.*

[**Removed**]
*None — 00_PLAN.*

[**Unresolved**]
*None — 00_PLAN.*

[**Response Matrix**]

*None — 00_PLAN has no prior review to respond to.*

---

## Spec `{Core specification}`

[**Goals**]

- **G-1:** `ark context` is a **top-level**, visible, semver-covered subcommand. Appears in `ark --help`. Separate stability tier from the hidden `ark agent` namespace.
- **G-2:** Two flags control the output: `--scope {session|phase}` (default `session`) and `--for {design|plan|review|execute|verify}` (required iff `--scope=phase`). Clap rejects `--for` without `--scope=phase` and rejects `--scope=phase` without `--for`.
- **G-3:** `--format {json|text}` (default `text`) selects output shape. Both modes derive from the same in-memory `Context` struct: JSON via `serde_json::to_writer_pretty`, text via a `Display`-returning summary type.
- **G-4:** JSON output carries `"schema": 1` as the first field. Schema is **additive-only** going forward; field removal or rename requires bumping `schema`.
- **G-5:** Payload contains **paths and summaries only** — no file bodies. Artifacts appear as `{path, exists, lines}` entries; specs appear as index-row data (name, scope); task artifacts appear as `{path, kind, lines}`.
- **G-6:** Session projection (`--scope session`) returns: git state, active-tasks list (flat), project specs index, feature specs index, recent-archive summary (last 5 archived tasks, name + tier + archived_at).
- **G-7:** Phase projections (`--scope phase --for <phase>`) return: git state, current task (slug, tier, iteration, phase, artifacts list), plus phase-specific slices:
  - `design`: project specs index + feature specs index + recent archive.
  - `plan`: current task's PRD path + related feature specs (parsed from PRD's `[**Related Specs**]` section) + project specs index.
  - `review`: current task's latest `NN_PLAN.md` path + related feature specs + project specs index.
  - `execute`: current task's latest PLAN path + git dirty files + project specs index.
  - `verify`: current task's latest PLAN path + PRD path + VERIFY.md path (if exists) + git state.
- **G-8:** A `SessionStart` hook is rendered into `templates/claude/settings.json` (managed block) that runs `ark context --scope session --format json` on session start. Hook is non-blocking: a non-zero exit prints to stderr but does not halt Claude Code's session.
- **G-9:** `ark init` (and `ark load`'s scaffold path) writes `.claude/settings.json` with the managed hook block as a tracked template file, picked up by the existing `ark upgrade` hash-tracking machinery.
- **G-10:** All five shipped slash commands (`templates/claude/commands/ark/quick.md`, `design.md`, `archive.md`; plus any that reference raw `git status` / `ls .ark/tasks` invocations) are updated to call `ark context --scope phase --for <phase>` at their entry point in place of ad-hoc shell recipes.
- **G-11:** End-to-end round-trip: `ark init` → edit a task → `ark unload` → `ark load` → `ark remove` preserves the hook and the `ark context` command works throughout. Integration test in `commands/context/mod.rs::tests` + an end-to-end test in `commands/load.rs::tests` covers the managed-block round-trip for `settings.json`.
- **G-12:** `ark context --format text` (both scopes) produces human-readable output suitable for ad-hoc debugging, following the text layout established by Trellis's `get_context.py` where applicable: `## GIT STATUS` / `## CURRENT TASK` / `## ACTIVE TASKS` / `## SPECS` headings with blank-line separators.

Non-goals:

- **NG-1:** No mutation. `ark context` is read-only; no `--write`, no state transitions, no file creation.
- **NG-2:** No multi-developer / assignee / journal concepts. Out of scope.
- **NG-3:** No monorepo / sub-repo aggregation. Ark projects are single-repo.
- **NG-4:** No git-log output beyond the last 5 commit one-liners. Deeper history is `git log`'s job.
- **NG-5:** No search across tasks + specs + memory. That's Phase 3 roadmap (`ark search`).
- **NG-6:** No file bodies inlined in JSON. Callers read files they need via `Read`.
- **NG-7:** No caching layer. Every invocation re-reads state. Simpler and always-correct.
- **NG-8:** No `--scope task` for per-task drill-down in this iteration. Reserved for follow-up; the `--scope` flag is designed to accept it without renaming modes.
- **NG-9:** No hook rendering for `UserPromptSubmit`, `PreToolUse`, or other events in this task. Only `SessionStart`.
- **NG-10:** No `.codex/` / `.cursor/` rendering. Claude-only (matches Phase 0 scope).
- **NG-11:** No structured parsing of PRD's `[**Related Specs**]` beyond a line-by-line "extract `specs/features/<name>/SPEC.md` paths". If the section is malformed, the field reports an empty list without erroring.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                        — adds `Context(ContextArgs)` top-level subcommand
└── ark-core/src/
    ├── lib.rs                                  — re-exports context public API
    ├── error.rs                                — adds Error::ContextProjectionMismatch
    ├── layout.rs                               — adds claude_settings(); specs_project_dir();
    │                                             specs_project_index(); specs_features_index() exists
    ├── io/path_ext.rs                          — unchanged (existing read_text / read_text_optional suffice)
    ├── templates.rs                            — unchanged (adding claude/settings.json to the
    │                                             CLAUDE_TEMPLATES tree picks up automatically via include_dir!)
    ├── commands/
    │   ├── mod.rs                              — `pub mod context;` + re-exports
    │   ├── init.rs                             — no logic change; gains coverage of the new
    │   │                                         templates/claude/settings.json file automatically
    │   ├── load.rs                             — no logic change (same reason)
    │   ├── upgrade.rs                          — no logic change (hash-tracking handles it)
    │   └── context/
    │       ├── mod.rs                          — `context()` entry; options; re-exports; Display
    │       ├── gather.rs                       — one-pass data collection → `Context` struct
    │       ├── model.rs                        — `Context` + all sub-structs, Serialize, schema const
    │       ├── projection.rs                   — enum Scope { Session, Phase(Phase) }; fn project()
    │       └── render.rs                       — Display-impls for text mode
└── templates/
    ├── ark/                                    — unchanged
    └── claude/
        ├── commands/ark/                       — updated slash commands (design.md, quick.md, archive.md)
        └── settings.json                       — NEW: managed block with SessionStart hook
```

**Module coupling.** One-way dependency: `mod.rs` → `gather` → `model`; `mod.rs` → `projection` → `model`; `mod.rs` → `render` → `model`. `gather` is the only module that touches the filesystem. `projection` and `render` are pure functions of `&Context` + `Scope`.

**Call graph for `ark context`:**

```
context(opts)
  ├── gather::gather_context(project_root)      → Context (full, unprojected)
  │     ├── read git (branch, porcelain, log -5)
  │     ├── walk .ark/tasks/ (active) and .ark/tasks/archive/ (most-recent 5)
  │     ├── read .ark/specs/project/INDEX.md    (parse table rows from managed block or body)
  │     ├── read .ark/specs/features/INDEX.md   (parse table rows from managed block)
  │     ├── read .ark/tasks/.current            (optional; sets current_task slug)
  │     └── for current task: load task.toml, list artifact files
  ├── projection::project(&context, opts.scope) → ProjectedContext (subset view)
  └── match opts.format:
        Format::Json → serde_json::to_writer_pretty(stdout, &projected)
        Format::Text → println!("{}", render::TextSummary(&projected))
```

Internal invariant: `gather_context` is **infallible for a well-formed `.ark/`** — missing `.current` or empty archive are normal cases, not errors. The only hard failures are `.ark/` not initialized (`Error::NotLoaded`) or a corrupt `task.toml` (`Error::TaskTomlCorrupt`, reused from `ark-agent-namespace`).

[**Data Structure**]

```rust
// ark-core/src/commands/context/model.rs

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
pub struct Context {
    pub schema: u32,                         // always SCHEMA_VERSION
    pub generated_at: DateTime<Utc>,
    pub project_root: PathBuf,
    pub git: GitState,
    pub tasks: TasksState,
    pub specs: SpecsState,
    pub archive: ArchiveState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTask>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitState {
    pub branch: String,                      // "unknown" if detached or no branch
    pub head_short: String,                  // 7-char short sha; "" if no HEAD
    pub is_clean: bool,
    pub uncommitted_changes: u32,
    pub dirty_files: Vec<String>,            // relative paths, capped at 20
    pub recent_commits: Vec<GitCommit>,      // last 5
}

#[derive(Debug, Clone, Serialize)]
pub struct GitCommit { pub hash: String, pub message: String }

#[derive(Debug, Clone, Serialize)]
pub struct TasksState {
    pub active: Vec<TaskSummary>,            // flat; sorted by updated_at desc
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSummary {
    pub slug: String,
    pub title: String,
    pub tier: Tier,                          // reused from commands::agent
    pub phase: Phase,                        // reused from commands::agent
    pub iteration: u32,
    pub path: PathBuf,                       // project-relative
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentTask {
    pub slug: String,
    pub summary: TaskSummary,
    pub artifacts: Vec<ArtifactSummary>,
    pub related_specs: Vec<String>,          // paths parsed from PRD's [**Related Specs**]
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactSummary {
    pub kind: ArtifactKind,                  // Prd | Plan { iteration } | Review { iteration } | Verify | TaskToml
    pub path: PathBuf,                       // project-relative
    pub lines: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ArtifactKind {
    Prd,
    Plan { iteration: u32 },
    Review { iteration: u32 },
    Verify,
    TaskToml,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecsState {
    pub project: Vec<SpecRow>,
    pub features: Vec<SpecRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecRow {
    pub name: String,
    pub path: PathBuf,                       // project-relative
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promoted: Option<String>,            // raw column text from features index
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveState {
    pub recent: Vec<ArchivedTask>,           // last 5, most-recent first
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchivedTask {
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub archived_at: DateTime<Utc>,
    pub path: PathBuf,                       // project-relative
}
```

```rust
// ark-core/src/commands/context/projection.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Session,
    Phase(PhaseFilter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PhaseFilter { Design, Plan, Review, Execute, Verify }

/// Projected shape: same fields, optional-ified or trimmed. Serialized with
/// `#[serde(skip_serializing_if = "Option::is_none")]` throughout so the JSON
/// output reflects exactly what the projection chose to include.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectedContext {
    pub schema: u32,
    pub scope: ScopeTag,                          // {"session"} or {"phase": "design"|…}
    pub generated_at: DateTime<Utc>,
    pub project_root: PathBuf,
    pub git: GitState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TasksState>,                // session only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTask>,        // phase only (session includes it too if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs: Option<SpecsState>,                // trimmed per phase
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveState>,            // session + phase=design only
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "scope", rename_all = "lowercase")]
pub enum ScopeTag {
    Session,
    Phase { phase: PhaseFilter },
}

pub fn project(ctx: Context, scope: Scope) -> ProjectedContext;
```

```rust
// ark-core/src/commands/context/mod.rs

#[derive(Debug, Clone)]
pub struct ContextOptions {
    pub project_root: PathBuf,
    pub scope: Scope,
    pub format: Format,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format { Json, Text }

pub fn context(opts: ContextOptions) -> Result<ContextSummary>;

/// Implements Display for text mode; carries the JSON bytes for json mode.
/// The CLI binary's `render(...)` helper prints `{summary}` which in JSON
/// mode prints the pre-serialized bytes and in text mode prints the rendered
/// human text. Same one-shot-print contract as every other command.
pub struct ContextSummary { /* opaque */ }
```

Error additions:

```rust
// ark-core/src/error.rs
Error::ContextProjectionMismatch { scope: &'static str, reason: &'static str },
// Raised only in tests / invariant guards; e.g. attempting to project with
// `Scope::Phase(_)` when `current_task` is None — in production that case
// yields an empty `current_task` field, not an error. Guard is there to
// catch programmer mistakes in future projections.
```

(No new `Error::NotLoaded` — reuse the existing variant already raised by manifest reads when `.ark/` is missing. `context` checks for `.ark/` existence via `Layout::ark_dir().exists()` and returns `Error::NotLoaded` if absent, matching the `upgrade` pattern.)

[**API Surface**]

Library re-exports from `ark-core/src/lib.rs`:

```rust
pub use commands::{
    context::{
        context, Context, ContextOptions, ContextSummary, Format,
        PhaseFilter, ProjectedContext, Scope, SCHEMA_VERSION,
    },
    // … existing re-exports
};
```

CLI shape (in `ark-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum Command {
    Init(InitArgs),
    Load(LoadArgs),
    Unload(TargetArgs),
    Remove(TargetArgs),
    Upgrade(UpgradeArgs),
    /// Print a structured snapshot of git + .ark/ workflow state.
    Context(ContextArgs),                    // NEW — visible in ark --help
    #[command(hide = true)]
    Agent(AgentArgs),
}

#[derive(clap::Args)]
struct ContextArgs {
    #[command(flatten)]
    target: TargetArgs,

    /// Which projection to run.
    #[arg(long, value_enum, default_value = "session")]
    scope: ScopeArg,

    /// Phase to filter by (required when --scope=phase).
    #[arg(long, value_enum, requires_if("phase", "scope"))]
    r#for: Option<PhaseArg>,

    /// Output format.
    #[arg(long, value_enum, default_value = "text")]
    format: FormatArg,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum ScopeArg { Session, Phase }

#[derive(Copy, Clone, clap::ValueEnum)]
enum PhaseArg { Design, Plan, Review, Execute, Verify }

#[derive(Copy, Clone, clap::ValueEnum)]
enum FormatArg { Json, Text }
```

Argument validation: `--scope=phase` without `--for` → clap error "`--for <PHASE>` is required when `--scope=phase`". `--scope=session` with `--for` → clap error "`--for` is only valid with `--scope=phase`". Both checked in an `ArgGroup`-style relation or a post-parse `validator` closure in the dispatch arm.

Templates additions:

- `templates/claude/settings.json` — new file, contains:
  ```json
  {
    "hooks": {
      "SessionStart": [
        {
          "type": "command",
          "command": "ark context --scope session --format json",
          "timeout": 5000
        }
      ]
    }
  }
  ```
  Wrapped in the existing Ark managed block (`<!-- ARK:START --> ... <!-- ARK:END -->` pattern from `io::fs::update_managed_block`) **inside the file** so user-added hooks coexist with Ark's. **Design note:** JSON doesn't support comments, so the managed-block mechanism used for `CLAUDE.md` doesn't apply directly. Instead: when `init` / `upgrade` runs, if `.claude/settings.json` doesn't exist, write it fresh; if it exists, read it, merge the `hooks.SessionStart` entry via JSON-object merge (matching on `"command" = "ark context --scope session --format json"` as the identity key), write back. Unchanged in any other key. This avoids managed-block markers in JSON entirely. The merge logic lives in a small new helper `io::fs::merge_json_managed` taking the settings path, a pointer (`/hooks/SessionStart`), and the identity key.

Slash command updates (`templates/claude/commands/ark/{quick,design,archive}.md`): prepend a "## Context" section with the verbatim shell recipe `ark context --scope phase --for <phase> --format json`, and remove any explicit `git status` / `ls .ark/tasks` recipes that are now redundant. **Keep** references to `ark agent task new`, `ark agent task plan`, etc. — `ark context` doesn't replace the mutation commands, only the orientation calls.

[**Constraints**]

- **C-1:** `ark context` is listed in `ark --help` (no `#[command(hide = true)]`). Verified by a CLI snapshot test.
- **C-2:** `ark context --help` mentions neither "hidden" nor "not covered by semver" — it's a stable public command.
- **C-3:** JSON output's first field is `"schema": 1`. Text output's first line contains `schema=1` in a header comment. Both asserted in unit tests.
- **C-4:** All filesystem access in `commands/context/` routes through `io::PathExt` / `io::fs` helpers. No bare `std::fs::*`.
- **C-5:** All `.ark/`-relative path composition routes through `layout::Layout` helpers. New helpers `claude_settings()`, `specs_project_dir()`, `specs_project_index()`, `tasks_archive_dir()` added to `Layout` as needed.
- **C-6:** `Context` struct field order and names are the source of truth for JSON schema. Reviewer must approve any additions; renames / removals are blocked until schema version is bumped.
- **C-7:** `ark context` emits **exactly one** stdout write in each mode: JSON mode → one `serde_json::to_writer_pretty` call; text mode → one `println!("{}", summary)`. No interspersed debug prints, no progress indicators. stderr reserved for errors only.
- **C-8:** `gather_context` reads at most **5** commits from `git log` and caps `dirty_files` at **20** entries. Larger sets are truncated silently; `uncommitted_changes` count always reflects the true total.
- **C-9:** Archive listing reads at most the **5** most-recently-modified subdirectories under `.ark/tasks/archive/`. Older archives are not scanned. Sort order: by `task.toml.archived_at` descending.
- **C-10:** Text mode format is **not** machine-parseable and carries no schema version. Users who need stability parse JSON.
- **C-11:** The `SessionStart` hook entry's identity key is `"command": "ark context --scope session --format json"`. `init` / `upgrade` merging uses this exact string. If a user changes the command, the hook is treated as user-modified and preserved per `ConflictPolicy`.
- **C-12:** `merge_json_managed` is idempotent: running `ark init` twice produces byte-identical `settings.json`.
- **C-13:** `ark context` never exits with a non-zero code on empty state (no `.current`, no active tasks, empty specs). Empty state returns a valid `Context` with empty vecs and `current_task: None`.
- **C-14:** On a non-Ark directory (`.ark/` missing), `ark context` exits with `Error::NotLoaded`. Same error name as `upgrade`.
- **C-15:** Hook timeout is 5000ms. The hook command is expected to complete well under this (<100ms on a typical repo); timeout exists only as a safety bound.
- **C-16:** Managed-block merge must NOT rewrite the full `settings.json`: only the `hooks.SessionStart` array is touched, and only the single Ark-owned entry within it. All other JSON keys and array entries are preserved byte-for-byte where possible (reserialize only the sub-document that changed).

## Runtime `{runtime logic}`

[**Main Flow**]

1. User / hook runs `ark context [--scope …] [--for …] [--format …]` from within an Ark project directory.
2. CLI parses args, validates `--scope`/`--for` relationship, builds `ContextOptions`, dispatches to `commands::context::context`.
3. `context` calls `gather_context(project_root)` → full `Context` struct.
4. `context` calls `project(context, scope)` → `ProjectedContext`.
5. Depending on `format`:
   - `Json`: serialize `ProjectedContext` with `serde_json::to_writer_pretty` to a `Vec<u8>`, wrap in `ContextSummary::Json(bytes)`.
   - `Text`: wrap in `ContextSummary::Text(ProjectedContext)`.
6. Return `ContextSummary`. CLI binary prints it via `render(summary)` — same pattern as every other command.
7. `SessionStart` hook case: Claude Code's hook runner captures stdout and injects it into the session context; exit code 0 = success, non-zero = warning logged but session continues.

[**Failure Flow**]

1. `.ark/` missing → `Error::NotLoaded { path }` at step 3. CLI prints `error: ark not loaded in <path>` and exits 1.
2. `.ark/tasks/.current` points to a nonexistent slug → `current_task = None`, not an error (the `.current` pointer is a hint, not a hard invariant).
3. `task.toml` of current task is corrupt → `Error::TaskTomlCorrupt { path, source }` propagated. Same variant used by `ark agent task`.
4. Specs INDEX has malformed managed block → parse what we can, log warning to stderr (text mode only), JSON mode silently returns empty specs list. Rationale: `ark context` being noisy on stderr pollutes hook output.
5. `git` command missing from `PATH` → `run_git` returns a helpful error; `context` treats this as a hard failure (`Error::Io` wrapping the spawn error). The hook will log stderr but session continues.
6. `git status --porcelain` on a non-git directory → git exits non-zero; `gather_context` treats this as a soft failure and returns `GitState { branch: "unknown", is_clean: true, … }` with empty vecs. Rationale: `.ark/` doesn't require git. Tests verify this path.
7. JSON serialization error (should be impossible for a well-formed `Context`) → `Error::Io` wrapping `serde_json::Error`.
8. `SessionStart` hook timeout (>5s) → Claude Code kills the process, logs timeout. `ark context` should complete in ≪100ms on the largest reasonable project (500 task dirs).

[**State Transitions**]

`ark context` is **stateless**. No on-disk state changes, no in-process state machine. Only state observed is:

- `.current` exists → `current_task: Some(_)` (subject to `task.toml` existing and parsing).
- `.current` absent → `current_task: None`.
- `.ark/tasks/<slug>/NN_PLAN.md` files → latest-iteration PLAN picked by max(NN) parsed from filename.

## Implementation `{split task into phases}`

[**Phase 1 — Core data model & gather engine**]

1. Add `ark-core/src/commands/context/{mod,model,gather,projection,render}.rs` skeleton files.
2. Define `Context`, `GitState`, `TasksState`, `TaskSummary`, `CurrentTask`, `ArtifactSummary`, `ArtifactKind`, `SpecsState`, `SpecRow`, `ArchiveState`, `ArchivedTask` in `model.rs` with `Serialize` derives. `SCHEMA_VERSION` const.
3. Implement `gather_context(project_root)` in `gather.rs`:
   - git: reuse `std::process::Command::new("git")`; 3 calls (branch, porcelain, log -5). Soft-fail on non-git dir.
   - active tasks: walk `.ark/tasks/` (skip `archive`, `.current`), load each `task.toml` via existing `TaskToml::load`, collect summaries.
   - archive: walk `.ark/tasks/archive/YYYY-MM/`, pick 5 most-recently-modified, load `task.toml` from each.
   - specs: parse managed-block rows from `specs/project/INDEX.md` and `specs/features/INDEX.md` (markdown table inside `<!-- ARK:... -->` markers or whole body for project index which is user-authored).
   - current task: read `.ark/tasks/.current`, assemble `CurrentTask` with artifact listing (glob `*.md` and `task.toml`, classify by filename pattern: `PRD.md`, `NN_PLAN.md`, `NN_REVIEW.md`, `VERIFY.md`).
   - related specs: parse PRD's `[**Related Specs**]` section for `specs/features/<name>/SPEC.md` path entries (regex or line-by-line).
4. Add `Layout::claude_settings()` returning `.claude/settings.json`.
5. Unit tests in each module: minimum `gather` happy-path fixture + empty-state fixture.

[**Phase 2 — Projection, rendering, CLI**]

1. Implement `projection::project(ctx, scope)` producing `ProjectedContext`. Per-phase field nulling per G-7.
2. Implement `render.rs` — `impl Display for TextSummary<'a>`. Text layout mirrors Trellis's `get_context.py` headings: `## GIT STATUS`, `## CURRENT TASK`, `## ACTIVE TASKS`, `## SPECS`, `## ARCHIVE` (emit only sections present in the projection).
3. Implement `commands/context/mod.rs::context()` entry:
   - `Layout::ark_dir().exists()` check → `Error::NotLoaded` if missing.
   - Call `gather_context` → `project` → wrap in `ContextSummary`.
4. Wire `lib.rs` re-exports.
5. Wire CLI: add `Context(ContextArgs)` variant, `ScopeArg`/`PhaseArg`/`FormatArg` enums, dispatch arm, `validate_scope_for(...)` helper.
6. Unit tests: one test per `(scope, phase)` combination asserting the set of fields present / absent in the projection; CLI arg-relation test (`--for` without `--scope=phase` fails).

[**Phase 3 — Template + hook integration**]

1. Add `templates/claude/settings.json` with the `SessionStart` hook entry (no managed-block markers inside JSON).
2. Implement `io::fs::merge_json_managed(path, pointer, identity_key, value)` helper:
   - Reads existing JSON (or defaults to `{}`).
   - Navigates to `pointer` (e.g. `/hooks/SessionStart`), ensuring intermediate objects exist.
   - In the array, finds entry matching `identity_key` (e.g. `"command" == "ark context …"`). Replaces or inserts.
   - Writes pretty-printed JSON back.
3. Wire `init.rs` to call `merge_json_managed` for `.claude/settings.json` in addition to writing embedded templates. Hash-track the post-merge contents so `upgrade` handles changes correctly.
4. Wire `upgrade.rs` — no new logic needed if `settings.json` is in the template tree and hash-tracked. Verify via round-trip test.
5. Update `templates/claude/commands/ark/{quick,design,archive}.md` to add "## Context" section at the top with `ark context --scope phase --for <phase> --format json` recipe. Remove redundant `git status` / `ls` recipes if present.
6. Update `.ark/workflow.md` mechanics table §7 to add a row for `ark context` — **read-only**, so it's not listed under "structural mutation" but under a new row: `| Session / phase orientation | ark context [--scope … ] [--for …] |` placed just above the `ark agent` rows.
7. End-to-end round-trip test in `commands/load.rs::tests`: init → modify settings.json's non-Ark key → unload → load → assert non-Ark key preserved AND Ark's `SessionStart` hook present.

[**Phase 4 — Docs and cleanup**]

1. Update `AGENTS.md` §Repository Layout and `ark-core` module map to include `commands/context/`.
2. Update `README.md` user-facing command list to include `ark context` with a short description.
3. Update `docs/ROADMAP.md` — mark `ark context` as shipped under Phase 1; note Phase 2 hook rendering is partially started.
4. Run `cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`. All must pass.
5. Run the End-to-End smoke from `AGENTS.md:83` and extend it with an `ark context` invocation.

## Trade-offs `{ask reviewer for advice}`

- **T-1: `--scope` + `--for` as two orthogonal flags vs a single `--mode` enum.**
  - **Chosen:** two flags.
  - *Advantages:* future `--scope task` / `--scope feature` additions don't force a flat enum explosion (`--mode session|design|plan|review|execute|verify|task|feature…`). Clean semantic split: "which projection" vs "which phase within a projection".
  - *Disadvantages:* two flags mean two places to document and a clap arg-relation to enforce. Slightly more verbose CLI. User confusion risk: "do I always need `--for`?" (answer: only with `--scope=phase`).
  - *Rejected alternative (single `--mode`):* simpler, Trellis-style (`--mode default|record|packages`). Rejected because we expect `--scope` to grow and want the flag-shape to absorb growth without renaming.

- **T-2: JSON schema versioning from v1 vs wait-until-breaking-change.**
  - **Chosen:** version from day one (`"schema": 1`).
  - *Advantages:* downstream parsers can gate on `schema` from the start. "Is this the version I expect?" is a 1-line check. Adding the field later is itself a breaking change (old JSON has no `schema`, new does) — better to establish the invariant upfront.
  - *Disadvantages:* adds a field users will mostly ignore. One more thing to remember to bump.

- **T-3: Merge `settings.json` via a JSON-pointer helper vs maintain a full template file.**
  - **Chosen:** JSON-merge helper, no managed-block markers inside JSON.
  - *Advantages:* coexists with user-added hooks (user's `PreToolUse` / `UserPromptSubmit` entries survive). Idempotent. No fragile JSON-with-comments gymnastics.
  - *Disadvantages:* new helper to maintain (`merge_json_managed`). Hash-tracking the post-merge output means a user who changes unrelated keys triggers the "user-modified" path on upgrade — acceptable, and the existing conflict policy (`--force` / `--skip-modified` / `--create-new`) handles it.
  - *Rejected alternative (full template overwrite):* simpler but stomps any user-added hooks. Unacceptable.
  - *Rejected alternative (managed-block markers in JSON):* JSON doesn't support comments. Would require parsing the file, finding `"_ark_marker_start"` / `"_ark_marker_end"` string keys or similar hack. Ugly.

- **T-4: Emit `dirty_files` list (capped at 20) vs only the count.**
  - **Chosen:** emit both the list (capped) and the total count.
  - *Advantages:* execute-phase callers want the specific dirty files to know what they're about to commit. Session bootstrap uses the count.
  - *Disadvantages:* payload grows by up to 20 strings. Acceptable (~1KB).
  - *Rejected alternative (count only):* forces every caller that needs filenames to shell out to `git status` anyway, defeating the purpose.

- **T-5: Text mode rendering style — Trellis-inspired sections vs compact one-liner summary.**
  - **Chosen:** multi-section Trellis-inspired layout for text mode.
  - *Advantages:* human-readable, familiar to anyone who's seen Trellis. Easy to scan.
  - *Disadvantages:* ~30 lines of output for a "simple" query. Users who want compact output use `--format json | jq`.
  - *Rejected alternative (one-liner):* too terse for humans; JSON exists for machines.

- **T-6: Parse `[**Related Specs**]` section from PRD vs require the caller to supply spec list.**
  - **Chosen:** parse it out of PRD.
  - *Advantages:* self-contained; `ark context` produces a complete picture without the caller needing to pre-process.
  - *Disadvantages:* couples `context` to PRD template format. A template change breaks the parser silently (empty related_specs). Tests must cover both shapes (existing `{path} — {note}` bullets and the empty-section placeholder).
  - *Accepted risk:* PRD template is shipped by Ark and versioned with the CLI. A template change is an intentional act with a test sweep.

## Validation `{test design}`

[**Unit Tests**]

- **V-UT-1:** `gather::gather_context` on a fresh `ark init` tempdir returns `Context` with empty active tasks, empty specs, `current_task: None`, `git.branch != "unknown"` if tempdir is a git repo.
- **V-UT-2:** `gather::gather_context` on a non-git tempdir returns `GitState { branch: "unknown", is_clean: true, uncommitted_changes: 0, dirty_files: [], recent_commits: [] }` and succeeds overall.
- **V-UT-3:** `gather::gather_context` with a seeded active task (via `task_new` + dir edits) reports it in `tasks.active` with correct `tier`, `phase`, `iteration`.
- **V-UT-4:** `gather::gather_context` with `.current` pointing at a deep-tier task mid-plan-review returns `current_task.artifacts` including `PRD`, `Plan { iteration: 0 }`, `Review { iteration: 0 }` with correct line counts.
- **V-UT-5:** `gather::gather_context` with a malformed `.current` (points at nonexistent slug) returns `current_task: None` and does not error.
- **V-UT-6:** `gather::gather_context` with corrupt `task.toml` in the current task returns `Error::TaskTomlCorrupt`.
- **V-UT-7:** `projection::project(ctx, Scope::Session)` returns a `ProjectedContext` with `tasks: Some(_)`, `specs: Some(_)`, `archive: Some(_)`; `current_task` is `Some` iff `ctx.current_task.is_some()`.
- **V-UT-8:** `projection::project(ctx, Scope::Phase(PhaseFilter::Design))` returns `current_task: Some|None`, `specs: Some(_)` (full), `archive: Some(_)`, but `tasks: None`.
- **V-UT-9:** `projection::project(ctx, Scope::Phase(PhaseFilter::Plan))` returns `current_task: Some|None`, `specs: Some(_)` (features filtered to `current_task.related_specs` ∪ project specs), `archive: None`, `tasks: None`.
- **V-UT-10:** `projection::project(ctx, Scope::Phase(PhaseFilter::Review))` same as Plan.
- **V-UT-11:** `projection::project(ctx, Scope::Phase(PhaseFilter::Execute))` returns `current_task`, `specs.project`, `git` (including `dirty_files`), but `specs.features: []`, `archive: None`, `tasks: None`.
- **V-UT-12:** `projection::project(ctx, Scope::Phase(PhaseFilter::Verify))` returns `current_task` with all artifacts (PRD + latest PLAN + VERIFY if exists), `specs.project`, `git`, no features, no archive.
- **V-UT-13:** `render::TextSummary` on a projected context produces output matching a golden-file fixture per scope/phase combination.
- **V-UT-14:** JSON serialization of each `ProjectedContext` variant starts with `"schema": 1` (byte-level assertion).
- **V-UT-15:** `io::fs::merge_json_managed` on an empty file writes `{"hooks":{"SessionStart":[<entry>]}}`.
- **V-UT-16:** `io::fs::merge_json_managed` on an existing file preserves unrelated top-level keys.
- **V-UT-17:** `io::fs::merge_json_managed` called twice produces byte-identical output (idempotence).
- **V-UT-18:** `io::fs::merge_json_managed` updates an existing Ark-owned entry (matched by identity key) in place rather than appending.
- **V-UT-19:** PRD `[**Related Specs**]` parser extracts `specs/features/<name>/SPEC.md` paths from bullet lines; malformed / missing section yields empty vec without error.

[**Integration Tests**]

- **V-IT-1:** Full pipeline: `ark init` → `ark agent task new --tier deep` → `ark agent task plan` → invoke `context({scope: Session, format: Json})` → JSON parses, contains the new task in `tasks.active`, `current_task.summary.phase = Plan`.
- **V-IT-2:** CLI integration: `ark context --scope session --format json` on a fresh `ark init` tempdir prints valid JSON to stdout with exit code 0.
- **V-IT-3:** CLI integration: `ark context --scope phase --for design` (text mode) prints human-readable sections.
- **V-IT-4:** CLI integration: `ark context` (no flags, defaults `--scope=session --format=text`) prints text for a fresh tempdir.
- **V-IT-5:** CLI arg-relation: `ark context --scope=phase` (no `--for`) exits 2 with clap error mentioning "required".
- **V-IT-6:** CLI arg-relation: `ark context --scope=session --for=design` exits 2 with clap error mentioning "only valid with".
- **V-IT-7:** Template round-trip: `ark init` → `.claude/settings.json` contains the `SessionStart` Ark hook entry. Parseable as JSON. Matches fixture.
- **V-IT-8:** Template round-trip: edit `.claude/settings.json` to add a user `PreToolUse` hook → run `ark upgrade` → user hook preserved, Ark hook still present.
- **V-IT-9:** Template round-trip: `ark init` → `ark unload` → `ark load` → `.claude/settings.json` identical to post-init state (both Ark hook and any user edits captured in the snapshot preserved).
- **V-IT-10:** `ark context` on a non-Ark directory (no `.ark/`) exits 1 with `Error::NotLoaded`.
- **V-IT-11:** `ark --help` output contains "context" as a command. `ark agent --help` does not list `context`.

[**Failure / Robustness Validation**]

- **V-F-1:** Validate behavior when `git` binary is missing from `PATH`: `gather_context` surfaces `Error::Io` wrapping the spawn error; `ark context` exits non-zero. Documented; no special handling.
- **V-F-2:** Validate behavior when `.ark/tasks/.current` contains a trailing newline or leading whitespace: parsed correctly (trimmed).
- **V-F-3:** Validate behavior when `specs/features/INDEX.md` managed block is malformed (missing END marker): rows parse as empty, stderr warning in text mode only, JSON mode silent.
- **V-F-4:** Validate behavior when two `NN_PLAN.md` files claim the same iteration number (e.g. `00_PLAN.md` and `00_PLAN.md.bak` — unlikely but): `.bak` excluded by regex; only canonical `^\d{2}_PLAN\.md$` matches.
- **V-F-5:** Validate behavior when `.ark/tasks/archive/` doesn't exist (fresh project): `archive.recent: []`, no error.
- **V-F-6:** Validate behavior when `SessionStart` hook itself errors (simulated by pointing it at a broken path): Claude Code logs but session continues. Not an `ark context` failure — verified only by documentation / manual test.

[**Edge Case Validation**]

- **V-E-1:** Empty project: `ark init` in a brand-new git repo, then `ark context`. All vecs empty, exit 0.
- **V-E-2:** Very large project: 500 active tasks. Benchmark `ark context --scope session` completes in <500ms on a dev laptop. Not a regression gate; documented in PLAN.
- **V-E-3:** Unicode task titles and slugs: JSON output is valid UTF-8; text output renders correctly.
- **V-E-4:** Concurrent `ark init` + `ark context` (race): undefined behavior, documented. Not expected in practice (single-user tool).
- **V-E-5:** `ark context` with cwd deep inside `.ark/tasks/<slug>/`: `TargetArgs::resolve` climbs to an absolute cwd, but `Layout::new(root)` treats `root` as the project root — **bug risk**: we need to ensure `--dir` flag or project-root detection walks up to find `.ark/`. Follow-up: reuse whatever detection existing commands use. V-E-5 test: `ark context` run from `.ark/tasks/<slug>/` works if `--dir` is passed; without `--dir`, it errors with `NotLoaded` (consistent with other commands).
- **V-E-6:** `--format json` output piped through `jq .schema` returns `1`.
- **V-E-7:** Schema-forward-compat test: a future schema=2 reader that expects `schema: 2` refuses to parse schema=1 output. Documented; we don't need to test the future reader, but the gating pattern is specified.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (top-level, visible) | V-IT-11 |
| G-2 (`--scope` + `--for` flags) | V-IT-5, V-IT-6, V-UT-7 through V-UT-12 |
| G-3 (`--format`) | V-IT-2, V-IT-3, V-IT-4, V-UT-13, V-UT-14 |
| G-4 (schema version) | V-UT-14, V-E-6 |
| G-5 (paths + summaries only) | V-UT-4 (asserts artifacts are path+lines, no body) |
| G-6 (session projection content) | V-UT-7, V-IT-1 |
| G-7 (phase projections content) | V-UT-8 through V-UT-12 |
| G-8 (SessionStart hook rendered) | V-IT-7 |
| G-9 (settings.json in template tree) | V-IT-7, V-IT-9 |
| G-10 (slash commands updated) | Manual review + V-IT-7 (template content) |
| G-11 (round-trip preserves hook) | V-IT-8, V-IT-9 |
| G-12 (text mode layout) | V-UT-13 |
| C-1 (visible in `--help`) | V-IT-11 |
| C-2 (`--help` text) | V-IT-11 (extended) |
| C-3 (schema=1 first field) | V-UT-14 |
| C-4 (PathExt only) | Enforced by manual code review + clippy; implicit in V-UT-*/V-IT-* passing without `std::fs` |
| C-5 (Layout only) | Same as C-4 |
| C-6 (schema field lock) | Policy, not a test — reviewer gate |
| C-7 (single stdout write) | V-IT-2 captures stdout; asserts single write / no interspersed output |
| C-8 (5 commits, 20 dirty files cap) | V-E-2, plus a dedicated test with >5 commits and >20 dirty files |
| C-9 (5 archive entries cap) | Test: seed 10 archived tasks; assert `archive.recent.len() == 5` |
| C-10 (text not versioned) | V-UT-13 asserts text has no `schema` field |
| C-11 (identity key) | V-UT-18 |
| C-12 (merge idempotent) | V-UT-17 |
| C-13 (empty state returns valid Context) | V-UT-1, V-E-1 |
| C-14 (NotLoaded on non-Ark) | V-IT-10 |
| C-15 (hook timeout) | Template fixture assertion in V-IT-7 |
| C-16 (preserve unrelated JSON) | V-UT-16, V-IT-8 |
