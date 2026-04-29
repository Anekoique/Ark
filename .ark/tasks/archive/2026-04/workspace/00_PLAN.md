# `workspace` PLAN `00`

> Status: Draft
> Feature: `workspace`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`
> - PRD: `PRD.md`
> - Related specs: `specs/features/worktree-support/SPEC.md`, `specs/features/ark-agent-namespace/SPEC.md`, `specs/features/ark-context/SPEC.md`

---

## Summary

Add per-developer session journals to Ark. New `commands/agent/workspace/` module providing `ark agent workspace init` (bootstrap identity) and `ark agent workspace record` (append a session entry). `task archive` gains an internal call to `workspace::record::record_task` that auto-appends a `kind = task` entry whenever `.ark/.developer` is set. A new `/ark:record` slash command (and Codex/OpenCode peers) calls `ark agent workspace record` for `kind = manual` entries — work that wasn't task-shaped. Identity is opt-in: missing `.developer` → auto-record no-ops with stderr note; record CLI errors with `Error::DeveloperNotInitialized`. From inside a worktree, both auto-record and manual record write to the **parent**'s `.ark/workspace/<dev>/`, since the journal is the developer's repo-global lifetime log, not the branch's. Optional `.ark/workspace.toml` carries tunables (`journal_max_lines`, `auto_record_on_archive`).

## Log `None in 00_PLAN`

[**Added**]
- Initial plan.

[**Changed**]
- N/A.

[**Removed**]
- N/A.

[**Unresolved**]
- T-1: should `/ark:record` slash command shell out to `ark agent workspace record` per phase, or be a one-shot?
- T-2: `workspace.toml` placement — `.ark/workspace.toml` (sibling of `worktree.toml`) vs `.ark/workspace/.config.toml` (inside the workspace tree)?
- T-3: parent-root resolution from inside a worktree — `git rev-parse --git-common-dir` vs walking up via `task.toml.worktree_path`?

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec `Core specification`

[**Goals**]

- **G-1:** New `commands/agent/workspace/` module with public API: `workspace_init(WorkspaceInitOptions) -> Result<WorkspaceInitSummary>` and `workspace_record(WorkspaceRecordOptions) -> Result<WorkspaceRecordSummary>`. Both follow the existing `ark agent` patterns: write to disk, return one-line `Display` summary, no `println!` in command bodies, all FS via `io::PathExt`, all `.ark/`-relative paths via `Layout` helpers.

- **G-2:** Identity is opt-in. `ark agent workspace init --name <x>` is the **sole** path that creates `.ark/.developer` and the `.ark/workspace/<x>/` tree. `ark init` does NOT prompt for identity and does NOT create `.developer` or `.ark/workspace/`. Missing `.developer` → identity-required commands return `Error::DeveloperNotInitialized { path }`; auto-record on `task archive` becomes a no-op (returns `Ok(WorkspaceRecorded::SkippedNoIdentity)` and emits one-line stderr "no developer set; skipping workspace record").

- **G-3:** `workspace_init` sequence (creates):
  1. Validate `name` per C-3 (non-empty, ASCII alphanumeric + `_-`, ≤40 chars). Reject otherwise → `Error::InvalidDeveloperName`.
  2. If `.ark/.developer` exists with a `name=` line → `Error::DeveloperAlreadyInitialized { name }` (re-init must remove file first).
  3. Write `.ark/.developer` as `name=<x>\ninitialized_at=<RFC3339>\n`.
  4. Ensure `.ark/workspace/<x>/` exists.
  5. Seed `<dev>/index.md` from embedded template (managed-block markers `ARK:WORKSPACE_STATUS` and `ARK:WORKSPACE_SESSIONS` already present).
  6. Seed `<dev>/journal-1.md` with the dated heading template.
  7. Return `WorkspaceInitSummary { name, dev_dir, created: bool }`.

  Idempotent: if both files exist with current name, no overwrite (just return `created: false`).

- **G-4:** `workspace_record` sequence (appends):
  1. Resolve **parent layout** via `resolve_parent_layout(&layout)` (G-9) — auto-record from inside a worktree writes to parent's workspace.
  2. Read `.developer` (parent's). Missing → `Error::DeveloperNotInitialized` for the manual path; **caller-controlled skip** for the task-archive path (see G-7).
  3. Load `WorkspaceConfig::load_or_default(&parent_layout)`.
  4. Determine active journal file: enumerate `<dev>/journal-N.md`, pick the highest-numbered. If its line count ≥ `cfg.journal_max_lines`, rotate: next file is `journal-{N+1}.md`. Hard cap `cfg.journal_max_lines == 0` → `Error::InvalidConfigField`. Hard cap on `N` (e.g. 9999) → `Error::JournalRotationLimit` (defensive; never expected to trigger).
  5. Render the session entry (G-5) and append (`fs::OpenOptions::new().append(true)` via `PathExt`).
  6. Re-render `<dev>/index.md`'s two managed blocks via `update_managed_block`:
     - `ARK:WORKSPACE_STATUS` body: bullet list with active file + total sessions + last-active date.
     - `ARK:WORKSPACE_SESSIONS` body: GFM table `# | Date | Title | Kind | Slug | Branch | Commits` sorted by date desc. Source of truth for the table is the journal-file headings; `workspace_record` re-scans all `<dev>/journal-*.md` and rebuilds the table.
  7. Return `WorkspaceRecordSummary { dev, journal_path, journal_index, session_number, rotated: bool }`.

- **G-5:** Session entry markdown shape (Trellis-style trimmed). Each entry begins with a unique anchor heading the index parser keys on:

  ```markdown
  ## Session {N}: {Title}

  **Date**: {YYYY-MM-DD}
  **Kind**: {task|manual}
  **Slug**: {slug | -}
  **Branch**: `{branch}`

  ### Summary
  {body — one or more paragraphs}

  ### Commits
  | Hash | Message |
  |------|---------|
  | `{short}` | {message} |

  ### Next Steps
  - {item}
  - {item}
  ```

  Fields are written in this exact order. `## Session {N}` is the anchor pattern (`^## Session (\d+):` regex) used to count sessions and parse for the index re-render.

- **G-6:** Commit collection. For `kind = task`: read `git log <base_branch>..HEAD --oneline` from the **task's worktree path** (or `archive_path` if archived) — captures the branch's lineage. For `kind = manual`: read `git log -n 5 --oneline` from cwd (last 5 commits on whichever branch the user is on). Capped at 20 entries either way. If `git` fails (non-git, no commits) → empty commits table, no error.

- **G-7:** `task::archive::task_archive` integration. After `spec_extract`+`spec_register` on deep tier (existing pattern, lines 96–111 of `task/archive.rs`), insert:

  ```rust
  let workspace_recorded = workspace::record::record_task(RecordTaskOptions {
      project_root: opts.project_root.clone(),
      slug: opts.slug.clone(),
      title: toml.title.clone(),
      tier,
      branch: toml.branch.clone(),
      base_branch: toml.base_branch.clone(),
      worktree_path: toml.worktree_path.clone(),
      archive_path: archive_path.clone(),
      archived_at: now,
  })?;
  ```

  `record_task` is a **thin wrapper** over `workspace_record`: gathers task-derived inputs, then calls into the shared journal writer. Wraps `Error::DeveloperNotInitialized` by returning `RecordTaskOutcome::SkippedNoIdentity` rather than propagating (auto-record is best-effort). All other errors propagate. **No-rollback policy**, mirroring `spec_extract` (the user can hand-run `/ark:record` if it failed). `TaskArchiveSummary` gains `workspace_recorded: WorkspaceRecorded` enum (`Recorded { journal_path, session_number } | SkippedNoIdentity | SkippedDisabled`).

- **G-8:** `WorkspaceConfig` mirrors `WorktreeConfig`:

  ```rust
  pub struct WorkspaceConfig {
      pub journal_max_lines: u32,        // default 2000
      pub auto_record_on_archive: bool,  // default true
  }
  ```

  `auto_record_on_archive = false` → `record_task` returns `SkippedDisabled`. Loaded via `WorkspaceConfig::load_or_default(&parent_layout)`. Corrupt → `Error::WorkspaceConfigCorrupt { path, source }` with source chained.

- **G-9:** Parent-root resolution. New helper `workspace::parent::resolve_parent_layout(layout: &Layout) -> Result<Layout>`:
  1. If `<root>/.git` is a directory (regular checkout) → return `layout.clone()`.
  2. If `<root>/.git` is a file (worktree pointer) → run `git rev-parse --git-common-dir` from `<root>`. The output is e.g. `/abs/path/.git`. Parent of that dir is the parent root. Return `Layout::new(parent_root)`.
  3. If `<root>` has no `.git` at all → return `layout.clone()` (operate locally; same as a regular non-worktree repo).

  Single git call, no walking; `task.toml.worktree_path` is NOT used (the worktree might be reached via a path that doesn't go through `worktrees_dir()`). Errors only on `Error::GitSpawn`; non-zero exit reports `Error::ParentRootResolution { reason }`.

- **G-10:** New CLI subcommand group:

  ```
  ark agent workspace init   --name <x>
  ark agent workspace record [--title "<t>"] [--summary "<s>"] [--next "<n>"]
  ```

  Each takes `TargetArgs` (existing pattern); `record` resolves layout via `TargetArgs::resolve_with_discovery`. Both subcommands' outputs go through `Display` summary types; no ad-hoc `println!`.

- **G-11:** `templates/ark/.gitignore` adds a second line `.developer`. Currently a flat 1-line file (`worktrees/`); becomes a flat 2-line file. **No managed block** — fully Ark-owned (matches the existing comment in `templates.rs` precedent). `ark upgrade` re-applies the canonical content unconditionally per the upgrade SPEC's `.gitignore` policy.

- **G-12:** `templates/ark/workspace.toml` ships a commented config with default values surfaced as comments (mirrors `templates/ark/worktree.toml`). Created by `ark init`; **NEVER overwritten** by `ark upgrade` (mirrors worktree-support C-9).

- **G-13:** `.ark/workspace/<dev>/index.md` template (NEW: `templates/ark/_workspace_index.md` — leading underscore so it's not auto-copied as a top-level file by `ark init`; explicitly fetched by `workspace_init`):

  ```markdown
  # Workspace Index — {{name}}

  > Per-developer session journal. Updated automatically by `ark agent workspace {init|record}` and `ark agent task archive`.

  <!-- ARK:WORKSPACE_STATUS:START -->
  - **Active File**: `journal-1.md`
  - **Total Sessions**: 0
  - **Last Active**: -
  <!-- ARK:WORKSPACE_STATUS:END -->

  ## Sessions

  <!-- ARK:WORKSPACE_SESSIONS:START -->
  | # | Date | Title | Kind | Slug | Branch | Commits |
  |---|------|-------|------|------|--------|---------|
  <!-- ARK:WORKSPACE_SESSIONS:END -->
  ```

  `{{name}}` is a single literal token replaced at scaffold time (not a templating engine). Marker syntax matches `update_managed_block`'s existing convention.

- **G-14:** `.ark/workspace/<dev>/journal-1.md` template:

  ```markdown
  # Journal — {{name}} (Part 1)

  > AI development session journal. Started: {{date}}

  ---

  ```

  `{{date}}` = ISO date. `journal-{N+1}.md` (created on rotation) uses the same template with `(Part {{N+1}})`.

- **G-15:** `/ark:record [<title>]` slash command. The shipped slash file `templates/claude/commands/ark/record.md` (and Codex skill `ark-record/`, OpenCode `record.md`) wraps `ark agent workspace record`. Body of the slash command tells the agent:
  1. Pull `ark context --scope session --format json`.
  2. If `<title>` provided: invoke `ark agent workspace record --title "<title>" --summary "<gen>" --next "<gen>"` where summary/next are agent-generated from the conversation context.
  3. If `<title>` absent: agent first summarizes the most recent topic from conversation context into `<title>` + `<summary>` + `<next>`, then invokes the CLI.

  The CLI is the source of truth; the slash command is a recipe for content generation.

- **G-16:** Workflow doc additions. `.ark/workflow.md` (and `templates/ark/workflow.md`) gain a `### Workspace (optional)` subsection under §6 Mechanics — terse, table-row style per the doc-edit-scope convention. AGENTS.md command table gains one short row per existing terseness. `templates/claude/commands/ark/record.md` is the new slash command file. Slash commands `quick.md`, `design.md`, `archive.md` are NOT modified — they don't archive (they tell the user to run `/ark:archive`), and `archive.md` already invokes `ark agent task archive` whose updated implementation handles workspace internally.

[**Non-goals**]

- **NG-1:** No team / multi-developer aggregation. Each `<dev>/` is private to its developer; no rollup view, no `ark agent workspace list-developers`. Cross-developer journals are a future task.
- **NG-2:** No PR / Slack / GitHub publishing. Sessions are local markdown.
- **NG-3:** No edit/delete history of recorded sessions. Append-only. To "fix" an entry, the developer hand-edits the markdown.
- **NG-4:** No structured-output JSON for journal entries. Trellis-style markdown only. (Stable schema concerns belong in `ark context`.)
- **NG-5:** No `ark unload`/`load` schema changes. `.ark/workspace/` is plain content; existing capture logic handles it. `.ark/.developer` is the lone file-level exclusion (G-19 / C-7).
- **NG-6:** No automatic `task archive` rollback when `record_task` fails. Mirrors `spec_extract` policy: archive succeeded; user runs `/ark:record` manually to recover the journal entry.
- **NG-7:** No identity migration / rename. Re-init requires deleting `.ark/.developer` first.
- **NG-8:** No `ark context` scope changes for the workspace MVP. A future task may surface developer name + active journal in `--scope session`; out of scope here. (Decision deferred — see T-3.)
- **NG-9:** No worktree management of `.ark/workspace/`. Each worktree's view of `.ark/workspace/` mirrors whatever was on the branch at branch-create time; auto-record always writes to parent regardless. Nothing in this feature creates branches, merges, or syncs workspace state across branches.
- **NG-10:** No `.ark/workspace/` shipped in `templates/ark/`. The dir is created lazily by `workspace init`; no empty-dir scaffold.
- **NG-11:** No CLI for `workspace deinit` / "remove developer". Single-command surface: `init` and `record`. To remove identity: hand-delete `.ark/.developer`.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                              — adds Workspace(WorkspaceCliArgs)
│                                                     under AgentCommand; two subcommands
│                                                     Init, Record
└── ark-core/src/
    ├── lib.rs                                       — re-exports public workspace API
    ├── error.rs                                     — adds 5 variants (see Data Structure)
    ├── layout.rs                                    — adds workspace_dir,
    │                                                  workspace_developer_dir,
    │                                                  workspace_index, workspace_journal,
    │                                                  workspace_config_file, developer_file
    ├── commands/
    │   ├── unload.rs                                — extends file-level skip to include
    │   │                                              layout.developer_file()
    │   └── agent/
    │       ├── mod.rs                               — pub mod workspace;
    │       │                                          re-exports for siblings
    │       ├── state.rs                             — unchanged
    │       ├── task/
    │       │   └── archive.rs                       — calls super::workspace::record::record_task
    │       │                                          after spec promotion;
    │       │                                          TaskArchiveSummary gains workspace_recorded
    │       └── workspace/                           — NEW
    │           ├── mod.rs                           — public types + dispatch
    │           ├── config.rs                        — WorkspaceConfig (workspace.toml model)
    │           ├── identity.rs                      — read/write .developer; name validation
    │           ├── parent.rs                        — resolve_parent_layout via run_git
    │           ├── journal.rs                       — JournalEntry render; rotation;
    │           │                                      header parser for re-render
    │           ├── index.rs                         — render_status_block, render_sessions_block
    │           ├── init.rs                          — workspace_init
    │           └── record.rs                        — workspace_record + record_task wrapper
templates/
├── ark/
│   ├── .gitignore                                   — adds `.developer` line
│   ├── workspace.toml                               — NEW: shipped default config
│   ├── _workspace_index.md                          — NEW: per-dev index.md template
│   ├── _workspace_journal.md                        — NEW: per-dev journal-1.md template
│   └── workflow.md                                  — adds §6 Workspace subsection
└── claude/commands/ark/record.md                    — NEW
templates/codex/skills/ark-record/                   — NEW (mirrors claude record.md)
templates/opencode/commands/ark/record.md            — NEW
```

**Module coupling.** `task::archive` imports `super::workspace::record::record_task` (one-way: `task → workspace`). `workspace::{init, record}` import `workspace::{config, identity, journal, index, parent}`. Within workspace: `init → identity, journal, index`; `record → identity, parent, config, journal, index`. `parent` calls `io::git::run_git`; `journal` and `index` are leaves (markdown rendering only).

**Call graph for `ark agent workspace init`:**

```
workspace::init::workspace_init(opts)
  ├── identity::validate_developer_name(&opts.name)         → Error::InvalidDeveloperName
  ├── if layout.developer_file().exists():
  │     ├── existing = identity::read_developer_name(&layout)
  │     └── return Error::DeveloperAlreadyInitialized { name: existing }
  ├── identity::write_developer_file(&layout, &opts.name, now)
  ├── dev_dir = layout.workspace_developer_dir(&opts.name)
  ├── dev_dir.ensure_dir()
  ├── index_path = layout.workspace_index(&opts.name)
  ├── if !index_path.exists():
  │     └── index::seed_index(&index_path, &opts.name)        // managed-block markers shipped
  ├── journal_path = layout.workspace_journal(&opts.name, 1)
  ├── if !journal_path.exists():
  │     └── journal::seed_journal(&journal_path, &opts.name, 1, today)
  └── return WorkspaceInitSummary { name, dev_dir, created: <bool> }
```

**Call graph for `ark agent workspace record` (manual):**

```
workspace::record::workspace_record(opts)
  ├── parent_layout = parent::resolve_parent_layout(&layout)?
  ├── name = identity::read_developer_name(&parent_layout)?
  │           → if missing: Error::DeveloperNotInitialized
  ├── cfg = WorkspaceConfig::load_or_default(&parent_layout)?
  ├── (active_journal, active_n) = journal::find_active(&parent_layout, &name)?
  ├── needs_rotate = journal::line_count(&active_journal)? >= cfg.journal_max_lines
  ├── (target_journal, target_n) = if needs_rotate {
  │       let n = active_n + 1;
  │       (parent_layout.workspace_journal(&name, n), n)
  │   } else { (active_journal, active_n) };
  ├── if needs_rotate: journal::seed_journal(&target_journal, &name, target_n, today)
  ├── session_n = journal::scan_session_count(&parent_layout, &name)? + 1
  ├── commits = io::git::run_git(["log", "-n", "20", "--oneline"], &layout.root())   // cwd, not parent
  ├── entry = journal::render_entry(JournalEntry { kind: Manual, session_n, ... })
  ├── target_journal.append_text(&entry)?
  ├── index::rerender(&parent_layout, &name)?                  // re-scans all journals
  └── return WorkspaceRecordSummary { dev, journal_path, journal_index, session_number, rotated }
```

**Call graph for `task archive` → workspace auto-record:**

```
task::archive::task_archive(opts)
  ├── ... existing flow up to spec_extract+spec_register on deep tier
  ├── outcome = workspace::record::record_task(RecordTaskOptions {
  │       project_root, slug, title, tier, branch, base_branch,
  │       worktree_path, archive_path, archived_at,
  │   })
  │     ├── parent_layout = parent::resolve_parent_layout(&layout)?
  │     ├── if !parent_layout.developer_file().exists():
  │     │     ├── eprintln!("no developer set; skipping workspace record")
  │     │     └── return Ok(WorkspaceRecorded::SkippedNoIdentity)
  │     ├── cfg = WorkspaceConfig::load_or_default(&parent_layout)?
  │     ├── if !cfg.auto_record_on_archive:
  │     │     └── return Ok(WorkspaceRecorded::SkippedDisabled)
  │     ├── commits = run_git(["log", "{base}..HEAD", "--oneline"], task_cwd)
  │     │       where task_cwd = worktree_path.unwrap_or(archive_path)
  │     │       fallback if base_branch is None: run_git(["log","-n","20","--oneline"], task_cwd)
  │     ├── entry = journal::render_entry(JournalEntry { kind: Task { slug }, ... })
  │     ├── ... same append + index re-render path as workspace_record
  │     └── return Ok(WorkspaceRecorded::Recorded { journal_path, session_number })
  ├── summary.workspace_recorded = outcome
  └── return summary
```

**`record_task` is NOT in `task::archive`.** It lives in `workspace::record` to keep the dependency direction one-way. `task::archive` calls into `workspace`; `workspace` knows nothing about tasks beyond the option struct it receives.

[**Data Structure**]

```rust
// ark-core/src/commands/agent/workspace/config.rs
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_journal_max_lines")]
    pub journal_max_lines: u32,
    #[serde(default = "default_auto_record_on_archive")]
    pub auto_record_on_archive: bool,
}

impl WorkspaceConfig {
    pub fn load_or_default(layout: &Layout) -> Result<Self>;
}

impl Default for WorkspaceConfig {
    fn default() -> Self { Self { journal_max_lines: 2000, auto_record_on_archive: true } }
}

fn default_journal_max_lines() -> u32 { 2000 }
fn default_auto_record_on_archive() -> bool { true }
```

```rust
// ark-core/src/commands/agent/workspace/mod.rs (public types)

pub mod config;

#[derive(Debug, Clone)]
pub struct WorkspaceInitOptions {
    pub project_root: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct WorkspaceInitSummary {
    pub name: String,
    pub dev_dir: PathBuf,
    pub created: bool,    // false if idempotent re-init
}

#[derive(Debug, Clone)]
pub struct WorkspaceRecordOptions {
    pub project_root: PathBuf,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub next: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceRecordSummary {
    pub dev: String,
    pub journal_path: PathBuf,
    pub journal_index: u32,
    pub session_number: u32,
    pub rotated: bool,
}

#[derive(Debug, Clone)]
pub enum WorkspaceRecorded {
    Recorded { journal_path: PathBuf, session_number: u32 },
    SkippedNoIdentity,
    SkippedDisabled,
}

// task -> workspace bridge:
#[derive(Debug, Clone)]
pub struct RecordTaskOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub branch: Option<String>,
    pub base_branch: Option<String>,
    pub worktree_path: Option<PathBuf>,
    pub archive_path: PathBuf,
    pub archived_at: DateTime<Utc>,
}

// All summaries impl Display (one-line).

pub fn workspace_init(opts: WorkspaceInitOptions)     -> Result<WorkspaceInitSummary>;
pub fn workspace_record(opts: WorkspaceRecordOptions) -> Result<WorkspaceRecordSummary>;
pub fn record_task(opts: RecordTaskOptions)            -> Result<WorkspaceRecorded>;
```

```rust
// ark-core/src/commands/agent/workspace/journal.rs

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub session_number: u32,
    pub title: String,
    pub date: NaiveDate,
    pub kind: JournalKind,
    pub branch: Option<String>,
    pub summary: String,
    pub commits: Vec<JournalCommit>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum JournalKind {
    Task { slug: String },
    Manual,
}

#[derive(Debug, Clone)]
pub struct JournalCommit {
    pub short: String,
    pub message: String,
}

pub fn render_entry(entry: &JournalEntry) -> String;
pub fn parse_entries(text: &str) -> Vec<ParsedEntry>;     // for index re-render
pub fn find_active(layout: &Layout, dev: &str) -> Result<(PathBuf, u32)>;
pub fn line_count(path: &Path) -> Result<u32>;
pub fn scan_session_count(layout: &Layout, dev: &str) -> Result<u32>;
pub fn seed_journal(path: &Path, dev: &str, n: u32, date: NaiveDate) -> Result<()>;
```

```rust
// ark-core/src/commands/agent/workspace/identity.rs

pub fn validate_developer_name(name: &str) -> Result<()>;
pub fn read_developer_name(layout: &Layout) -> Result<Option<String>>;   // None if missing
pub fn require_developer_name(layout: &Layout) -> Result<String>;        // errors if missing
pub fn write_developer_file(layout: &Layout, name: &str, now: DateTime<Utc>) -> Result<()>;
```

```rust
// ark-core/src/commands/agent/workspace/parent.rs

pub fn resolve_parent_layout(layout: &Layout) -> Result<Layout>;
```

```rust
// ark-core/src/commands/agent/workspace/index.rs

pub fn seed_index(path: &Path, dev: &str) -> Result<()>;
pub fn rerender(layout: &Layout, dev: &str) -> Result<()>;
fn render_status_block(active_file: &str, total: u32, last_active: NaiveDate) -> String;
fn render_sessions_block(entries: &[ParsedEntry]) -> String;
```

```rust
// ark-core/src/error.rs (additions)

Error::DeveloperNotInitialized { path: PathBuf },
Error::DeveloperAlreadyInitialized { name: String },
Error::WorkspaceConfigCorrupt { path: PathBuf, source: toml::de::Error },
Error::JournalRotationLimit { dev: String, max: u32 },
Error::InvalidDeveloperName { name: String, reason: &'static str },
Error::ParentRootResolution { reason: String },
```

```rust
// ark-core/src/layout.rs (additions)

pub const WORKSPACE_DIR: &str = ".ark/workspace";
pub const WORKSPACE_CONFIG_FILE: &str = ".ark/workspace.toml";
pub const DEVELOPER_FILE: &str = ".ark/.developer";

impl Layout {
    pub fn workspace_dir(&self) -> PathBuf;
    pub fn workspace_developer_dir(&self, dev: &str) -> PathBuf;
    pub fn workspace_index(&self, dev: &str) -> PathBuf;
    pub fn workspace_journal(&self, dev: &str, n: u32) -> PathBuf;
    pub fn workspace_config_file(&self) -> PathBuf;
    pub fn developer_file(&self) -> PathBuf;
}
```

```rust
// ark-core/src/commands/agent/task/archive.rs (additions to TaskArchiveSummary)

pub struct TaskArchiveSummary {
    // existing fields...
    pub workspace_recorded: WorkspaceRecorded,
}
```

[**API Surface**]

CLI shape (in `ark-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum AgentCommand {
    Task(TaskArgs),
    Spec(SpecArgs),
    Workspace(WorkspaceCliArgs),    // NEW
}

#[derive(clap::Args)]
struct WorkspaceCliArgs {
    #[command(subcommand)]
    command: WorkspaceSubcommand,
}

#[derive(Subcommand)]
enum WorkspaceSubcommand {
    /// Initialize developer identity. Opt-in; not run by `ark init`.
    Init(WorkspaceInitCliArgs),
    /// Append a manual session entry to the developer's active journal.
    Record(WorkspaceRecordCliArgs),
}

#[derive(clap::Args)]
struct WorkspaceInitCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(long)] name: String,
}

#[derive(clap::Args)]
struct WorkspaceRecordCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(long)] title: Option<String>,
    #[arg(long)] summary: Option<String>,
    #[arg(long)] next: Option<String>,
}
```

Library re-exports added to `ark-core/src/lib.rs`:

```rust
agent::{
    // ...existing...
    workspace::{
        WorkspaceConfig,
        WorkspaceInitOptions, WorkspaceInitSummary,
        WorkspaceRecordOptions, WorkspaceRecordSummary,
        WorkspaceRecorded,
        RecordTaskOptions,
        workspace_init, workspace_record, record_task,
    },
}
```

Internal helpers (`identity::*`, `journal::*`, `index::*`, `parent::*`) are NOT re-exported.

[**Constraints**]

- **C-1 (process spawn locality):** All git invocations route through `io::git::run_git`. `Command::new` MUST NOT appear under `commands/agent/workspace/`. Extends the existing source-scan test.
- **C-2 (path/io discipline):** All filesystem access in `commands/agent/workspace/` routes through `io::PathExt` (no bare `std::fs::*`). All `.ark/`-relative paths route through `Layout` helpers (no string concatenation).
- **C-3 (developer name validation):** Name MUST match `^[A-Za-z0-9_-]{1,40}$`. Empty, whitespace-only, leading dash, contains `/` or `\\`, > 40 chars → `Error::InvalidDeveloperName { name, reason }`. Filesystem hostility: name is also the *directory name* under `.ark/workspace/<name>/`.
- **C-4 (`.developer` file format):** Two key=value lines, `\n`-separated, UTF-8: `name=<x>\ninitialized_at=<RFC3339>\n`. Reader is line-oriented; ignores blanks and unknown keys for forward compat. Trailing newline required.
- **C-5 (idempotent init):** Running `workspace init --name <x>` twice with the same name on a fully-scaffolded workspace is a no-op (`created: false`). With a *different* name → `Error::DeveloperAlreadyInitialized` (no automatic rename).
- **C-6 (parent-write invariant):** `workspace_record` and `record_task` MUST write to `parent_layout.workspace_dir()`, never to `layout.workspace_dir()` when those differ. Regression test: `record_from_worktree_writes_to_parent_workspace`.
- **C-7 (snapshot exclusion):** `commands/unload.rs`'s file-capture loop MUST skip `layout.developer_file()`. The file is gitignored and per-machine — capturing it would leak identity on `unload`/`load` between machines or developers. Implementation: file-level path equality skip, additive to the existing `walk_files_excluding(skip = [worktrees_dir])` directory skip. `.ark/workspace/<dev>/` is captured normally (it's the journal content; meant to round-trip).
- **C-8 (managed-block re-render):** `index.md` is regenerated from journal contents on every `workspace_record` and `record_task` via `update_managed_block` for `ARK:WORKSPACE_STATUS` and `ARK:WORKSPACE_SESSIONS`. The journal files are the source of truth; `index.md`'s blocks are derived. Hand-edits *outside* the managed blocks are preserved.
- **C-9 (journal rotation):** Rotate when `line_count(active) >= cfg.journal_max_lines` AT THE START of `workspace_record`. The new entry then writes to `journal-{N+1}.md`. Cap `cfg.journal_max_lines >= 100` (very small caps make rotation pathological); below that → `Error::InvalidConfigField { field: "journal_max_lines", reason: "must be >= 100" }`.
- **C-10 (workspace.toml lifecycle):** Created by `ark init` from `templates/ark/workspace.toml`. `ark upgrade` does NOT overwrite it (mirrors worktree-support C-9). Lives under `.ark/`, captured by `unload`/`load` like other `.ark/` content.
- **C-11 (auto-record best-effort):** `record_task` MUST NOT propagate `Error::DeveloperNotInitialized` — returns `WorkspaceRecorded::SkippedNoIdentity` and emits one-line stderr "no developer set; skipping workspace record". All other errors (FS, git, config corrupt) propagate up to `task_archive`. Per NG-6, `task_archive` does NOT roll back the archive on workspace failure (the rename is already committed; the user runs `/ark:record` to recover).
- **C-12 (output stability):** Every workspace command writes a single `Display` summary; no ad-hoc stdout writes in command bodies. Mirrors `ark-agent-namespace` C-3.
- **C-13 (commit collection cap):** `git log` invocations cap at 20 entries via `-n 20`. For `record_task`, range is `<base_branch>..HEAD` (uses `task.toml.base_branch`); fallback to `-n 20` when `base_branch` is None. For manual `workspace_record`, always `-n 20` since there's no implied base. Output silently truncated; no header about truncation.
- **C-14 (parent resolution single git call):** `resolve_parent_layout` invokes `run_git` AT MOST ONCE per call. Cached per-process: NOT a constraint — each `task_archive` / `workspace_record` is a fresh CLI invocation, so caching is moot.
- **C-15 (CLI hidden under agent):** `ark agent workspace` follows the same hidden semantics as the rest of `ark agent`. `ark --help` does NOT list `agent` (existing C-1 from agent-namespace); `ark agent workspace --help` is reachable and includes "Not covered by semver".
- **C-16 (one-way coupling):** `task::archive` imports `super::workspace::record::record_task`. `workspace::*` modules MUST NOT import `super::task` or `super::spec`. Dependency direction enforced by code inspection at review time; no source-scan test (would be over-engineered for this scope).
- **C-17 (gitignore template lifecycle):** `templates/ark/.gitignore` is fully Ark-owned (no managed block). Adding `.developer` is a one-line content change. `ark upgrade` reapplies the canonical content — user edits to this file are reverted (matches existing precedent).
- **C-18 (slash command parity):** `templates/claude/commands/ark/record.md`, `templates/codex/skills/ark-record/`, and `templates/opencode/commands/ark/record.md` MUST stay in lockstep on every change to the slash command body. Tests compare byte-equality of the shared paragraphs (mirrors existing parity tests for `ark:design` / `ark:quick` / `ark:archive`).

---

## Runtime `runtime logic`

[**Main Flow — `workspace init`**]
1. CLI parses `--name <x>` and `TargetArgs`.
2. `validate_developer_name(&x)`.
3. Read `.ark/.developer`; if present → `Error::DeveloperAlreadyInitialized`.
4. Write `.ark/.developer` (`name=<x>\ninitialized_at=<now>\n`).
5. `mkdir -p .ark/workspace/<x>/`.
6. Seed `index.md` from `_workspace_index.md` template with `{{name}}` substitution.
7. Seed `journal-1.md` from `_workspace_journal.md` template with `{{name}}` + `{{date}}`.
8. Print one-line summary `developer "<x>" initialized at .ark/workspace/<x>`.

[**Main Flow — `workspace record` (manual)**]
1. CLI parses `[--title <t>] [--summary <s>] [--next <n>]` and `TargetArgs`.
2. `resolve_parent_layout(&layout)` → parent or self.
3. `read_developer_name(&parent_layout)` → required, else `Error::DeveloperNotInitialized`.
4. `WorkspaceConfig::load_or_default(&parent_layout)`.
5. `find_active(&parent_layout, &dev)` → `(active_path, active_n)`.
6. Check `line_count(active_path) >= cfg.journal_max_lines` → maybe rotate.
7. `scan_session_count(&parent_layout, &dev)` → assigns next session number.
8. Branch from `run_git(["symbolic-ref","--short","HEAD"], cwd)` (best-effort; "unknown" on failure).
9. Commits from `run_git(["log","-n","20","--oneline"], cwd)`.
10. Build `JournalEntry` (`title`, `summary`, `next_steps` from CLI args; agent provides them in slash-command path).
11. Append rendered entry to `target_journal`.
12. `index::rerender(&parent_layout, &dev)` rebuilds both managed blocks.
13. Print one-line summary `recorded session <N> to <journal_path>`.

[**Main Flow — `task archive` auto-record**]
1. `task_archive` reaches the post-rename phase (existing flow).
2. After `spec_extract`+`spec_register` (deep tier only) succeed.
3. Call `record_task(opts)` with the task-derived inputs.
4. `record_task` resolves parent layout, checks identity, returns `SkippedNoIdentity` if missing.
5. Otherwise renders a `kind = task` entry with commits from `<base_branch>..HEAD` (or fallback) run from `worktree_path` (active task) or `archive_path` (post-rename).
6. Appends to journal, re-renders index — same as manual flow.
7. Returns `WorkspaceRecorded` enum back into `TaskArchiveSummary`.

[**Failure Flow**]
- `Error::DeveloperNotInitialized` (path) — manual `workspace record` errors out cleanly; no partial writes. Auto-record converts to `SkippedNoIdentity` + stderr line, archive succeeds.
- `Error::DeveloperAlreadyInitialized` (name) — `workspace init` aborts before any disk write.
- `Error::WorkspaceConfigCorrupt` (path, source) — both `workspace_record` and `record_task` propagate. `task_archive` propagates the same — corrupt config breaks auto-record (rare; user fixes `workspace.toml`).
- `Error::InvalidDeveloperName` (name, reason) — pre-flight, no writes.
- `Error::JournalRotationLimit` (dev, max) — defensive; only reachable at >9999 journals (~20M lines). If hit, user must hand-rename or archive.
- `Error::ParentRootResolution` (reason) — `git rev-parse --git-common-dir` failed inside a presumed worktree. Auto-record propagates; manual `workspace record` propagates. Operator sees the underlying git stderr.
- `Error::Io` — wraps any FS error from `PathExt`; chained via thiserror source.
- `update_managed_block` failure (orphan marker) → `Error::ManagedBlockCorrupt` (existing). User must hand-fix `index.md`.

[**State Transitions**]
- `.developer` absent → identity-uninitialized; auto-record skips silently. Manual record errors.
- `.developer` present, `<dev>/` missing → init failure mid-flight from a previous run; `workspace init` (with same name) idempotent-completes the scaffold.
- `<dev>/journal-N.md` exists, `<dev>/journal-{N+1}.md` does not, current at line cap → next record creates `journal-{N+1}.md`. The old file is closed for new writes.
- `index.md` managed block missing or corrupt → on next record, `update_managed_block` writes the marker (idempotent insert); orphan marker → `Error::ManagedBlockCorrupt` (no auto-recovery).

---

## Implementation `split task into phases`

[**Phase 1 — core module + identity**]
- Add `Layout` constants/methods (`workspace_dir`, `workspace_developer_dir`, `workspace_index`, `workspace_journal`, `workspace_config_file`, `developer_file`).
- Add error variants in `error.rs`.
- Create `commands/agent/workspace/{mod.rs, identity.rs, parent.rs, config.rs}` with stubs returning sensible defaults. `identity.rs` complete: read/write/validate.
- Wire up `lib.rs` re-exports for the public types declared so far.
- Tests: `validate_developer_name` (rejects empty / >40 / `/` / `\` / leading dash; accepts ASCII alnum + `_-`), `read_developer_name_round_trip`, `WorkspaceConfig::load_or_default` (missing file → defaults; corrupt → error).

[**Phase 2 — journal + index rendering**]
- `journal.rs`: `JournalEntry`, `render_entry`, `parse_entries`, `find_active`, `line_count`, `scan_session_count`, `seed_journal`.
- `index.rs`: `seed_index`, `rerender`, internal `render_*_block` helpers.
- Templates: add `templates/ark/_workspace_index.md` and `_workspace_journal.md` to embedded tree.
- Tests: `render_entry_golden` (snapshot of a known entry), `parse_entries_round_trip` (render → parse → equals input subset), `rerender_idempotent` (re-rendering twice is byte-stable), `find_active_picks_highest_N`, `seed_index_includes_markers`.

[**Phase 3 — `workspace init` + `workspace record` CLI**]
- `init.rs`: full implementation per the call graph.
- `record.rs`: full `workspace_record`. `record_task` is a separate fn with its own option struct.
- CLI wiring in `ark-cli/src/main.rs`: add `Workspace(WorkspaceCliArgs)` to `AgentCommand`; dispatch + render.
- Tests: `workspace_init_creates_files`, `workspace_init_idempotent`, `workspace_init_already_initialized`, `workspace_record_appends_to_journal`, `workspace_record_rotates_at_cap`, `workspace_record_rerenders_index`, `workspace_record_no_identity_errors`.

[**Phase 4 — `task archive` integration + parent resolution**]
- `parent.rs`: `resolve_parent_layout` via `run_git`.
- Wire `record_task` into `task::archive::task_archive` after spec promotion. Add `workspace_recorded` to `TaskArchiveSummary`; update `Display`.
- Tests: `archive_records_to_workspace_when_identity_set`, `archive_skips_when_no_identity_with_stderr`, `archive_skips_when_auto_record_disabled`, `archive_from_worktree_writes_to_parent` (golden test using a real worktree fixture), `parent_resolution_in_regular_repo_returns_self`, `parent_resolution_in_worktree_returns_parent`.

[**Phase 5 — `templates/`, `.gitignore`, slash command, snapshot exclusion**]
- `templates/ark/.gitignore`: append `.developer`.
- `templates/ark/workspace.toml`: ship commented defaults.
- `templates/claude/commands/ark/record.md`: new slash command body.
- Codex skill + OpenCode command parity (`templates/codex/skills/ark-record/`, `templates/opencode/commands/ark/record.md`).
- `commands/unload.rs`: file-level skip for `layout.developer_file()`.
- Workflow doc: §6 Workspace subsection in `templates/ark/workflow.md` and `.ark/workflow.md`. AGENTS.md: one row.
- Tests: `unload_skips_developer_file`, `gitignore_includes_developer_after_init`, `upgrade_does_not_overwrite_workspace_toml`, slash-command parity tests.

---

## Trade-offs `ask reviewer for advice`

- **T-1: Slash command shape.** Option A: `/ark:record` is a one-shot — agent writes `--title`, `--summary`, `--next` to single CLI call, no follow-up. Option B: `/ark:record` is a multi-step recipe — agent first generates a draft, asks the user to confirm, then invokes the CLI. Option A is faster and matches `/ark:archive`'s stop-and-go style; Option B is safer for accidental records but adds friction. I lean **A** — recording is append-only and append-only, mistakes are recoverable by hand-editing the journal markdown. Reviewer: confirm or push back.

- **T-2: `workspace.toml` placement.** Option A: `.ark/workspace.toml` (sibling of `worktree.toml`). Option B: `.ark/workspace/.config.toml` (inside the workspace tree). A keeps all top-level Ark config in one place; B groups per-feature data with the feature dir. The PRD chose A for symmetry with `worktree.toml`. **Sticking with A** unless reviewer wants B.

- **T-3: Parent-root resolution method.** Option A: `git rev-parse --git-common-dir` then strip `/.git` (single git call, robust to non-default worktree paths). Option B: read `task.toml.worktree_path` and walk up until past `worktrees_dir()`. A is general (works even if the user moved the worktree dir or `worktrees_dir()` is misconfigured); B is git-free. **Going with A** because workspace already needs git for commit collection — the dependency is unavoidable.

- **T-4: Auto-record toggle plumbing.** `auto_record_on_archive` could live in `worktree.toml` instead of `workspace.toml` — but that conflates worktree concerns with workspace concerns. Keeping it in `workspace.toml`.

- **T-5: Index re-render strategy.** Option A: every `record` call re-scans all journals and rebuilds the table from scratch (current plan). Option B: in-place delta — find the latest table row, append a new row. A is robust to hand-edits and rotations; B is faster on huge journals but fragile. Going with A — `O(total_lines_across_all_journals)` is cheap until journals reach ~50K lines per developer.

- **T-6: `record_task` location.** Option A: in `task::archive` (sibling of spec_extract). Option B: in `workspace::record` (current plan, called by `task_archive`). A is closer to the call site; B preserves one-way coupling (`task → workspace`, never the reverse). Going with **B**. `record_task`'s logic is workspace-side bookkeeping; only its inputs come from task state.

---

## Validation `test design`

[**Unit Tests**]
- **V-UT-1 (G-3, C-3):** `validate_developer_name` accepts `kleinhe`, `dev_1`, `a-b`, `A`, 40-char strings; rejects empty, whitespace, `..`, `dev/path`, `/abs`, `dev\\back`, `-leading`, 41-char strings, non-ASCII (`é`, `中`).
- **V-UT-2 (G-3):** `read_developer_name` returns `None` for missing file, `Some(name)` for valid file, `None` for malformed file (no `name=` key). No errors on absent.
- **V-UT-3 (G-8, C-10):** `WorkspaceConfig::load_or_default` returns `Default` for missing file, parsed config for valid TOML, `Error::WorkspaceConfigCorrupt` for malformed TOML.
- **V-UT-4 (G-5):** `render_entry` produces byte-identical output to a golden fixture for a known `JournalEntry`. Anchor heading regex matches `^## Session (\d+):`.
- **V-UT-5 (G-5, C-8):** `parse_entries` round-trips: render → parse → render produces the same bytes.
- **V-UT-6 (G-4):** `find_active` picks the highest-numbered `journal-N.md` (numeric, not lexical: `journal-10` > `journal-9`).
- **V-UT-7 (G-4, C-9):** `line_count` returns exact line count; rotation logic triggers when count ≥ cap.
- **V-UT-8 (G-9):** `resolve_parent_layout` returns `layout.clone()` when `<root>/.git` is a directory; returns parent layout when `.git` is a file (mocked via fixture).

[**Integration Tests**]
- **V-IT-1 (G-3):** `workspace_init` creates `.developer`, `<dev>/index.md`, `<dev>/journal-1.md` with expected content.
- **V-IT-2 (C-5):** Re-running `workspace_init` with same name → `created: false`, no overwrite. Different name → `Error::DeveloperAlreadyInitialized`.
- **V-IT-3 (G-4, G-5):** `workspace_record` appends a manual entry; `index.md` `ARK:WORKSPACE_SESSIONS` block contains the new row.
- **V-IT-4 (C-9):** Setting `journal_max_lines = 100` then writing entries that push past the cap → next record creates `journal-2.md`; `Active File` in status block updates.
- **V-IT-5 (G-7, C-6):** `task_archive` of a deep-tier task records a session entry. Archived task fixture: branch `feat/foo`, slug `foo`, base `main`, two commits — verify the entry's commits table has both.
- **V-IT-6 (C-6):** `task_archive` invoked from inside a worktree writes the entry to **parent**'s `.ark/workspace/<dev>/`, not the worktree's. Fixture: parent repo + worktree + identity in parent.
- **V-IT-7 (C-7):** `ark unload` followed by `ark load` round-trips `.ark/workspace/<dev>/` content but does NOT capture `.ark/.developer` (file remains untouched on disk through the cycle, snapshot's manifest excludes it).

[**Failure / Robustness Validation**]
- **V-F-1 (G-2, C-11):** `task_archive` with no `.developer` set → archive succeeds, summary's `workspace_recorded == SkippedNoIdentity`, stderr contains "no developer set; skipping workspace record".
- **V-F-2 (NG-6, C-11):** `task_archive` with corrupt `workspace.toml` → archive directory rename succeeds, then `record_task` returns `Error::WorkspaceConfigCorrupt`. Task is in archive state on disk; the user can fix the config and run `/ark:record`.
- **V-F-3 (G-9):** `resolve_parent_layout` with `git rev-parse --git-common-dir` returning non-zero → `Error::ParentRootResolution { reason }`.
- **V-F-4 (G-2):** `workspace_record` (manual) with no `.developer` → `Error::DeveloperNotInitialized`. No partial writes to journal or index.
- **V-F-5 (C-8):** `update_managed_block` orphan marker (manually corrupted `index.md`) → `Error::ManagedBlockCorrupt`. No content lost from journals (journals are independent).

[**Edge Case Validation**]
- **V-E-1 (C-9):** `journal_max_lines = 99` → `Error::InvalidConfigField { field: "journal_max_lines", reason: "must be >= 100" }`.
- **V-E-2 (C-3):** Developer name `con` on Windows-reserved-name list — left to OS to reject at directory creation; we don't pre-validate (cross-platform OS-specific blacklists are out of scope).
- **V-E-3 (G-6):** `record_task` with `base_branch == None` (task scaffolded without `--worktree`) → fallback to `git log -n 20 --oneline` from `archive_path`. Verify entry has commits.
- **V-E-4 (G-6):** Task archived inside a non-git directory (extreme corner) → empty commits table, no error, entry still written.
- **V-E-5 (G-4):** Two rapid records (e.g., parallel `task archive` calls in different worktrees against same parent) — second call sees first's session in the count. Race window is small; we don't lock — last-writer's index re-render wins. Mitigated by always re-scanning journals (T-5 Option A).
- **V-E-6 (G-15):** `/ark:record` invoked with empty conversation context (fresh session, no work yet) — agent must produce *some* title. Slash-command body instructs the agent to handle this by asking the user for a title rather than fabricating one.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-UT-2, V-UT-3, V-UT-4 |
| G-2 | V-F-1, V-F-4 |
| G-3 | V-UT-1, V-UT-2, V-IT-1, V-IT-2 |
| G-4 | V-UT-6, V-UT-7, V-IT-3, V-IT-4, V-E-5 |
| G-5 | V-UT-4, V-UT-5, V-IT-3 |
| G-6 | V-IT-5, V-E-3, V-E-4 |
| G-7 | V-IT-5, V-F-1, V-F-2 |
| G-8 | V-UT-3, V-IT-4 |
| G-9 | V-UT-8, V-IT-6, V-F-3 |
| G-10 | V-IT-1, V-IT-3 |
| G-11 | V-IT-7 (gitignore line via templates/upgrade) |
| G-12 | V-IT-7 (workspace.toml preserved by upgrade) |
| G-13 | V-UT-4, V-IT-1 |
| G-14 | V-IT-1 |
| G-15 | V-E-6 |
| G-16 | (doc-only; verified by review, not test) |
| C-1  | (source-scan test extension; existing C-26 from agent-namespace) |
| C-2  | (source-scan test extension; existing C-4 from agent-namespace) |
| C-3  | V-UT-1 |
| C-4  | V-UT-2 |
| C-5  | V-IT-2 |
| C-6  | V-IT-6 |
| C-7  | V-IT-7 |
| C-8  | V-IT-3, V-IT-4, V-F-5 |
| C-9  | V-UT-7, V-IT-4, V-E-1 |
| C-10 | V-IT-7 |
| C-11 | V-F-1, V-F-2 |
| C-12 | (existing pattern; covered by lib API surface tests) |
| C-13 | V-IT-5 (range form), V-E-3 (fallback) |
| C-14 | (covered by V-UT-8 — single git call observable) |
| C-15 | (existing — `ark agent` hidden from `ark --help`) |
| C-16 | (review-time inspection; no test) |
| C-17 | V-IT-7 (gitignore upgrade) |
| C-18 | (slash-command parity test extension) |
