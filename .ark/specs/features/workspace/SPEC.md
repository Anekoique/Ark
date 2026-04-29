
[**Goals**]

- **G-1:** New `commands/agent/workspace/` module with public API: `workspace_init(WorkspaceInitOptions) -> Result<WorkspaceInitSummary>` and `workspace_record(WorkspaceRecordOptions) -> Result<WorkspaceRecordSummary>`. Both follow the existing `ark agent` patterns: write to disk, return one-line `Display` summary, no `println!` in command bodies, all FS via `io::PathExt`, all `.ark/`-relative paths via `Layout` helpers. A third public function `record_task(RecordTaskOptions) -> Result<WorkspaceRecorded>` is the bridge called by `task::archive`; it shares the journal-write path with `workspace_record` but is invoked internally rather than from the CLI.

- **G-2:** Identity has two bootstrap paths. **(a)** Default: `ark init` prompts for a developer name (parallel to the platform prompts; see G-18). **(b)** Manual: `ark agent workspace init --name <x>` for already-installed projects, idempotent re-init, and scripted setup. Missing `.developer` → identity-required commands return `Error::DeveloperNotInitialized { path }`; auto-record on `task archive` becomes a no-op (returns `WorkspaceRecorded::SkippedNoIdentity` and emits one-line stderr "no developer set; skipping workspace record"). Auto-record may also be globally disabled via `[workspace].auto_record_on_archive = false` in `.ark/config.toml` (G-8) — in that case it returns `WorkspaceRecorded::SkippedDisabled`. **There is no per-archive `--no-record` CLI flag.** The two skip conditions above are the only paths that disable auto-record.

- **G-3:** `workspace_init` sequence (creates):
  1. Validate `name` per C-3 (leading letter, then ASCII alphanumeric + `_-`, 1..=40 chars). Reject otherwise → `Error::InvalidDeveloperName`.
  2. If `.ark/.developer` exists with a `name=` line and the existing name ≠ `<x>` → `Error::DeveloperAlreadyInitialized { name }` (re-init must remove file first). If existing name == `<x>`, fall through (idempotent re-scaffold).
  3. Write `.ark/.developer` as `name=<x>\ninitialized_at=<RFC3339>\n`.
  4. Ensure `.ark/workspace/<x>/` exists.
  5. Seed `<dev>/index.md` from the embedded `_workspace_index.md` template (managed-block markers `ARK:WORKSPACE_STATUS` and `ARK:WORKSPACE_SESSIONS` already present in the template; bodies start empty).
  6. Seed `<dev>/journal-1.md` from the embedded `_workspace_journal.md` template.
  7. Return `WorkspaceInitSummary { name, dev_dir, created: bool }`.

  Idempotent: if `.ark/.developer` exists with the **same name** AND both files exist with their seeded content, return `created: false` without modification.

- **G-4:** `workspace_record` sequence (appends). Operates on the **current checkout** — from inside a git worktree, the journal lands in the worktree's `.ark/workspace/<dev>/`, not the parent's. The session entry rides along with the task commit on the same branch.
  1. Read `.developer` from the current layout. Missing → `Error::DeveloperNotInitialized` for the manual path. (Caller-controlled skip for the task-archive path is handled in `record_task`, not here — see G-7.)
  2. Load `WorkspaceConfig::load_or_default(&layout)`.
  3. Determine active journal file: enumerate `<dev>/journal-N.md`, pick the highest-numbered (numeric, not lexical). If its line count ≥ `cfg.journal_max_lines`, rotate: next file is `journal-{N+1}.md`. `cfg.journal_max_lines < 100` → `Error::InvalidConfigField`. `N+1 > 9999` → `Error::JournalRotationLimit`.
  4. Render the session entry per G-5 and append via `target_journal.append_text(&entry)`. The `append_text` primitive is declared on `PathExt` (G-17) with semantics: opens the file with `create(true).append(true)`, writes the bytes, and closes — atomic at the syscall level for the single write call. No prior file content is modified.
  5. Re-render `<dev>/index.md`'s two managed blocks via `update_managed_block`:
     - `ARK:WORKSPACE_STATUS` body: bullet list with active file name + total sessions + last-active date.
     - `ARK:WORKSPACE_SESSIONS` body: GFM table `# | Date | Title | Kind | Slug | Branch | Commits` sorted by session number desc. The journal files are the source of truth; `index.md`'s blocks are derived by re-scanning all `<dev>/journal-*.md` files (C-21 caps the scan).
  6. Return `WorkspaceRecordSummary { dev, journal_path, journal_index, session_number, rotated: bool }`.

- **G-5:** Session entry markdown shape (Trellis-style trimmed). Each entry begins with the unique anchor heading `## Session {N}: {Title}` and follows this exact field order:

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
  ```

  Two consecutive entries are separated by a single blank line. The anchor regex used by the parser (C-19) is `^## Session (\d+):\s+(.+?)\s*$`.

- **G-6:** Commit collection. **Task-cwd resolution** for `record_task`:
  1. If `worktree_path.is_some()` AND that path exists on disk AND is a directory → use it.
  2. Else if `archive_path` exists on disk AND is a directory → use it.
  3. Else → use the project root (`layout.root()`).

  **Git invocation** chosen by available metadata:
  - If `base_branch.is_some()` → `git log <base_branch>..HEAD --oneline -n 20` from the resolved cwd.
  - Else (no `--worktree` was used; pre-existing tasks) → `git log -n 20 --oneline` from the resolved cwd.

  For manual `workspace_record`: always `git log -n 20 --oneline` from the **caller's cwd** — captures whichever branch the user is on.

  Non-zero git exit (non-git directory, no commits) → empty commits table, no error. Output is parsed via `parse_oneline` (C-13).

- **G-7:** `task::archive::task_archive` integration. The new call site is **after** the `.current` cleanup block AND **before** the `Ok(summary)` return, **regardless of tier**. Quick / standard / deep all auto-record. The insertion is a single statement:

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

  `record_task` is a thin wrapper over the shared journal-write path: it gathers task-derived inputs, then invokes the same append + index-re-render logic as `workspace_record`. It MUST internally swallow `Error::DeveloperNotInitialized` (returning `WorkspaceRecorded::SkippedNoIdentity` instead) so that absent identity does not abort archive. All other errors propagate. **No-rollback policy**: if `record_task` returns an error, the archive directory rename is already committed and the user runs `/ark:record` to reconcile. `TaskArchiveSummary` gains `workspace_recorded: WorkspaceRecorded`.

- **G-8:** `WorkspaceConfig` mirrors `WorktreeConfig`:

  ```rust
  pub struct WorkspaceConfig {
      pub journal_max_lines: u32,        // default 2000
      pub auto_record_on_archive: bool,  // default true
  }
  ```

  `auto_record_on_archive = false` → `record_task` returns `SkippedDisabled` without reading identity or invoking git. `journal_max_lines < 100` → `Error::InvalidConfigField { field: "journal_max_lines", reason: "must be >= 100" }`. Loaded via `WorkspaceConfig::load_or_default(&layout)`. Corrupt → `Error::WorkspaceConfigCorrupt { path, source }` with source chained.

- **G-9:** *(removed: parent-root resolution. The journal is per-checkout; identity and content live wherever you're working. From inside a git worktree, the journal lands in the worktree's own `.ark/workspace/<dev>/`.)*

- **G-10:** New CLI subcommand group:

  ```
  ark agent workspace init   --name <x>
  ark agent workspace record [--title "<t>"] [--summary "<s>"] [--next "<n>"]
  ```

  Both subcommands flatten `TargetArgs` and resolve layout via `TargetArgs::resolve_with_discovery`. Both outputs go through `Display` summary types; no ad-hoc `println!`.

- **G-11:** `templates/ark/.gitignore` adds `.developer` as a second line. Currently a flat 1-line file (`worktrees/`); becomes a flat 2-line file. **No managed block** — fully Ark-owned. `ark upgrade` re-applies the canonical content unconditionally per the existing upgrade `.gitignore` policy.

- **G-12:** `templates/ark/config.toml` ships a commented sectioned config (`[worktree]` + `[workspace]`) with default values surfaced as comments. Created by `ark init`; **NEVER overwritten** by `ark upgrade`.

- **G-13:** `templates/ark/_workspace_index.md` is the per-developer `index.md` template (the leading underscore prevents top-level auto-copy by `ark init`):

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

  `{{name}}` is a single literal token replaced at scaffold time by string substitution (no templating engine). Marker syntax matches `update_managed_block`'s existing convention.

- **G-14:** `templates/ark/_workspace_journal.md` is the per-developer `journal-N.md` template:

  ```markdown
  # Journal — {{name}} (Part {{n}})

  > AI development session journal. Started: {{date}}

  ---

  ```

  `{{name}}`, `{{n}}`, `{{date}}` are literal-token substitutions. Trailing blank line is intentional (separates header from first appended entry).

- **G-15:** `/ark:record [<title>]` slash command. Shipped at `templates/claude/commands/ark/record.md`, with peers `templates/codex/skills/ark-record/SKILL.md` and `templates/opencode/commands/ark/record.md`. The slash command's body instructs the agent to:
  1. Pull `ark context --scope session --format json`.
  2. If `<title>` provided: invoke `ark agent workspace record --title "<title>" --summary "<gen>" --next "<gen>"` where summary/next are agent-generated from the conversation context.
  3. If `<title>` absent: agent first summarizes the most recent topic from conversation context into `<title>` + `<summary>` + `<next>`, then invokes the CLI. If conversation context has no work yet (fresh session), the agent asks the user for a title rather than fabricating one.

  The CLI is the source of truth; the slash command is a recipe for content generation.

- **G-16:** Workflow doc additions. `.ark/workflow.md` (and `templates/ark/workflow.md`) gain a `### Workspace (optional)` subsection under §6 Mechanics — terse, table-row style. AGENTS.md command table gains one short row matching the existing terseness of utility-command rows. Slash commands `quick.md`, `design.md`, `archive.md` are NOT modified.

- **G-17:** New `PathExt::append_text(&self, contents: &str) -> Result<()>` method on `crates/ark-core/src/io/path_ext.rs`. Opens the file with `OpenOptions::new().create(true).append(true)`, writes `contents.as_bytes()` via `write_all`, and drops the handle. The single `write_all` call is atomic at the OS level for buffer sizes below the pipe-write atomicity limit (PIPE_BUF on POSIX); journal entries are well below this threshold (~1KB typical). Errors map to `Error::Io` via `PathExt`'s existing wrapping pattern. Used internally by `commands/agent/workspace/record.rs`; available to all of `ark-core`. Re-exported alongside the existing `PathExt` re-export.

- **G-18:** `ark init` developer-identity bootstrap. The init flow gains a fourth interactive prompt parallel to the existing `claude / codex / opencode` prompts: `set up workspace identity?`. On `Y`, the user is asked for a name (default suggestion = `whoami` output if available; otherwise no default). Validation reuses `identity::validate_developer_name` per C-3. On valid input, the init flow calls `workspace_init` *after* the platform-template extraction completes — so `.ark/config.toml` exists by the time identity is bootstrapped. Two new CLI flags mirror the platform pattern:
  - `--developer <name>` — explicit non-interactive bootstrap (validated; runs `workspace_init` once at the end of init).
  - `--no-developer` — opt out of the prompt; `.ark/.developer` and `.ark/workspace/` are NOT created.

  Non-interactive init (no TTY) requires one of `--developer <name>` or `--no-developer`; running `ark init` in a script without either flag now errors with the same UX as the existing platform prompts (`init requires at least one of …`). When the user already has identity from a prior init (idempotent rerun), the prompt offers to keep the existing name (default = `Y`) or accept a new one (which errors with `DeveloperAlreadyInitialized` since rename is NG-7). The `InitSummary` gains a `developer: Option<String>` field summarizing the outcome.

- **NG-1:** No team / multi-developer aggregation.
- **NG-2:** No PR / Slack / GitHub publishing. Sessions are local markdown.
- **NG-3:** No edit/delete history of recorded sessions. Append-only.
- **NG-4:** No structured-output JSON for journal entries. Trellis-style markdown only.
- **NG-5:** No `ark unload`/`load` schema changes. `.ark/workspace/` is plain content; existing capture logic handles it. `.ark/.developer` is the lone file-level exclusion (C-7).
- **NG-6:** No automatic `task archive` rollback when `record_task` fails. Archive succeeded; user runs `/ark:record` to reconcile.
- **NG-7:** No identity migration / rename. Re-init requires deleting `.ark/.developer` first.
- **NG-8:** No `ark context` scope changes for the workspace MVP.
- **NG-9:** No worktree management of `.ark/workspace/`. The journal is committed branch content — git merges across branches like any other file.
- **NG-10:** No `.ark/workspace/` shipped in `templates/ark/`. Created lazily by `workspace init`.
- **NG-11:** No CLI for `workspace deinit`. Hand-delete `.ark/.developer` to remove identity.
- **NG-12:** No per-archive `--no-record` CLI flag. Auto-record is gated by missing `.developer` (skip silently with stderr note) or `auto_record_on_archive = false`.
- **NG-13:** No exotic-markdown parsing in the entry-boundary predicate (C-19). The parser handles 3+ backtick fences and 3+ tilde fences with length-aware close matching, but it does NOT handle: (a) HTML `<pre>` blocks; (b) raw `<!-- ... -->` HTML comments containing `## Session N:` lines; (c) lines that begin with `## Session N:` indented to 4 or more leading spaces (CommonMark indented-code-block territory). Authors who paste markdown-with-headings into summaries should verify the index re-render after recording. Misparsed entries are silently skipped from the index table; the journal markdown is not modified.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                              — adds Workspace(WorkspaceCliArgs)
│                                                     under AgentCommand; two subcommands
│                                                     Init, Record
└── ark-core/src/
    ├── lib.rs                                       — re-exports public workspace API
    ├── error.rs                                     — adds 6 variants (see Data Structure)
    ├── io/path_ext.rs                               — adds PathExt::append_text (G-17)
    ├── layout.rs                                    — adds workspace_dir,
    │                                                  workspace_developer_dir,
    │                                                  workspace_index, workspace_journal,
    │                                                  config_file, developer_file;
    │                                                  ensures Layout: Clone
    ├── commands/
    │   ├── unload.rs                                — extends file-level skip in BOTH
    │   │                                              walk sites (Stage A unload loop AND
    │   │                                              Stage B capture_orphan_hook_entries)
    │   │                                              to include layout.developer_file()
    │   └── agent/
    │       ├── mod.rs                               — pub mod workspace;
    │       ├── state.rs                             — unchanged
    │       ├── task/
    │       │   └── archive.rs                       — calls super::workspace::record::record_task
    │       │                                          after .current cleanup, before Ok return,
    │       │                                          regardless of tier;
    │       │                                          TaskArchiveSummary gains workspace_recorded
    │       └── workspace/                           — NEW
    │           ├── mod.rs                           — public types + dispatch
    │           ├── config.rs                        — WorkspaceConfig
    │           ├── identity.rs                      — read/write .developer; name validation
    │           ├── journal.rs                       — JournalEntry render; rotation;
    │           │                                      ParsedEntry; entry-boundary parser (C-19);
    │           │                                      pub(crate) parse_oneline
    │           ├── index.rs                         — render_status_block, render_sessions_block
    │           ├── init.rs                          — workspace_init
    │           └── record.rs                        — workspace_record + record_task wrapper
templates/
├── ark/
│   ├── .gitignore                                   — adds `.developer` line
│   ├── config.toml                                  — sectioned [worktree]+[workspace] config
│   ├── _workspace_index.md                          — NEW: per-dev index.md template
│   ├── _workspace_journal.md                        — NEW: per-dev journal-1.md template
│   └── workflow.md                                  — adds §6 Workspace subsection
├── claude/commands/ark/record.md                    — NEW
├── codex/skills/ark-record/SKILL.md                 — NEW
└── opencode/commands/ark/record.md                  — NEW
```

**Module coupling.** `task::archive` imports `super::workspace::record::record_task` (one-way: `task → workspace`). `workspace::{init, record}` import `workspace::{config, identity, journal, index}`. Within workspace: `init → identity, journal, index`; `record → identity, config, journal, index`. `journal` and `index` are leaves (markdown rendering only). `workspace::*` MUST NOT import `super::task` or `super::spec`.

**Call graph for `ark agent workspace init`:**

```
workspace::init::workspace_init(opts)
  ├── identity::validate_developer_name(&opts.name)         → Error::InvalidDeveloperName
  ├── if layout.developer_file().exists():
  │     ├── existing = identity::read_developer_name(&layout)?
  │     └── if existing != Some(opts.name):
  │           return Error::DeveloperAlreadyInitialized { name: existing.unwrap_or_default() }
  │     // else: fall through to idempotent re-scaffold (created = false)
  ├── identity::write_developer_file(&layout, &opts.name, now)
  ├── dev_dir = layout.workspace_developer_dir(&opts.name)
  ├── dev_dir.ensure_dir()
  ├── index_path = layout.workspace_index(&opts.name)
  ├── if !index_path.exists():
  │     └── index::seed_index(&index_path, &opts.name)
  │            // seeded with markers + empty body; first record populates
  │            // managed-block bodies via update_managed_block.
  ├── journal_path = layout.workspace_journal(&opts.name, 1)
  ├── if !journal_path.exists():
  │     └── journal::seed_journal(&journal_path, &opts.name, 1, today)
  └── return WorkspaceInitSummary { name, dev_dir, created: <bool> }
```

**Call graph for `ark agent workspace record` (manual):**

```
workspace::record::workspace_record(opts)
  ├── name = identity::require_developer_name(&layout)?
  │           → if missing: Error::DeveloperNotInitialized
  ├── cfg = WorkspaceConfig::load_or_default(&layout)?
  ├── (active_journal, active_n) = journal::find_active(&layout, &name)?
  ├── needs_rotate = journal::line_count(&active_journal)? >= cfg.journal_max_lines
  ├── (target_journal, target_n) = if needs_rotate {
  │       let n = active_n + 1;
  │       if n > 9999 { return Error::JournalRotationLimit { dev: name, max: 9999 } }
  │       (layout.workspace_journal(&name, n), n)
  │   } else { (active_journal, active_n) };
  ├── if needs_rotate: journal::seed_journal(&target_journal, &name, target_n, today)
  ├── session_n = journal::scan_session_count(&layout, &name)? + 1
  ├── branch = run_git(["symbolic-ref", "--short", "HEAD"], cwd).ok()
  │             .filter(|o| o.exit_code == 0)
  │             .map(|o| o.stdout.trim().to_string())   // None → "unknown" rendered
  ├── commits = run_git(["log", "-n", "20", "--oneline"], cwd)
  │             .map(|o| journal::parse_oneline(&o.stdout))
  │             .unwrap_or_default()                    // non-zero → empty
  ├── entry = journal::render_entry(JournalEntry { kind: Manual, session_number: session_n, ... })
  ├── target_journal.append_text(&entry)?              // PathExt::append_text per G-17
  ├── index::rerender(&layout, &name)?                  // re-scans journals, capped per C-21
  └── return WorkspaceRecordSummary { dev, journal_path, journal_index, session_number, rotated }
```

**Call graph for `task archive` → workspace auto-record:**

```
task::archive::task_archive(opts)
  ├── ... existing flow (validate, load TaskToml, check_transition, reserve archive path,
  │     rename to archive, save TOML with phase = Archived, deep-tier spec promotion)
  ├── ... clear .ark/tasks/.current if it pointed at slug
  │
  │     // archive_path is computed earlier in task_archive's frame and is the
  │     // post-rename location of the task dir. record_task receives a clone
  │     // through RecordTaskOptions.archive_path; G-6's fallback chain may
  │     // use it as a git cwd (it exists on disk after the rename).
  │
  ├── outcome = workspace::record::record_task(RecordTaskOptions {
  │       project_root: opts.project_root, slug, title, tier,
  │       branch, base_branch, worktree_path, archive_path, archived_at: now,
  │   })
  │     ├── cfg = WorkspaceConfig::load_or_default(&layout)?
  │     ├── if !cfg.auto_record_on_archive:
  │     │     └── return Ok(WorkspaceRecorded::SkippedDisabled)
  │     ├── name = match identity::read_developer_name(&layout)? {
  │     │       Some(n) => n,
  │     │       None => {
  │     │           eprintln!("no developer set; skipping workspace record");
  │     │           return Ok(WorkspaceRecorded::SkippedNoIdentity);
  │     │       }
  │     │   };
  │     ├── task_cwd = resolve_task_cwd(&opts, &layout)
  │     │      // 1. worktree_path if Some + exists + is_dir
  │     │      // 2. else archive_path if exists + is_dir
  │     │      // 3. else layout.root()
  │     ├── commits = match base_branch {
  │     │       Some(base) => run_git(["log", &format!("{base}..HEAD"), "--oneline", "-n", "20"], task_cwd),
  │     │       None       => run_git(["log", "-n", "20", "--oneline"], task_cwd),
  │     │   }.map(|o| journal::parse_oneline(&o.stdout)).unwrap_or_default()
  │     ├── entry = journal::render_entry(JournalEntry { kind: Task { slug }, ... })
  │     ├── target_journal.append_text(&entry)?
  │     ├── index::rerender(&layout, &name)?
  │     └── return Ok(WorkspaceRecorded::Recorded { journal_path, session_number })
  ├── summary.workspace_recorded = outcome
  └── return Ok(summary)
```

[**Data Structure**]

```rust
// ark-core/src/io/path_ext.rs (NEW method on PathExt; G-17)

pub trait PathExt {
    // ...existing methods unchanged...

    /// Appends UTF-8 text to the file, creating it if absent.
    /// Equivalent to `OpenOptions::new().create(true).append(true).open(self)?
    ///                .write_all(contents.as_bytes())`.
    /// Single `write_all` call; atomic for journal-sized writes.
    /// Errors map to `Error::Io` via the existing wrapping pattern.
    fn append_text(&self, contents: &str) -> Result<()>;
}
```

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
    pub created: bool,
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
    pub branch: Option<String>,    // None → renders as "unknown"
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

/// Output of `parse_entries` — one row per `## Session N: Title` heading found.
/// Used by `index::rerender` to rebuild the sessions table without re-reading
/// the full entry body.
#[derive(Debug, Clone)]
pub struct ParsedEntry {
    pub session_number: u32,
    pub title: String,
    pub date: NaiveDate,
    pub kind_label: String,        // "task" or "manual"
    pub slug: Option<String>,      // None for manual
    pub branch: String,            // "unknown" if missing
    pub commits_count: u32,
}

pub fn render_entry(entry: &JournalEntry) -> String;
pub fn parse_entries(text: &str) -> Vec<ParsedEntry>;
pub fn find_active(layout: &Layout, dev: &str) -> Result<(PathBuf, u32)>;
pub fn line_count(path: &Path) -> Result<u32>;
pub fn scan_session_count(layout: &Layout, dev: &str) -> Result<u32>;
pub fn seed_journal(path: &Path, dev: &str, n: u32, date: NaiveDate) -> Result<()>;

/// Parse `git log --oneline` output into commit rows. Each non-empty line
/// splits at the first ASCII space: prefix is `short`, suffix (trimmed) is
/// `message`. Empty lines are skipped. Caller is responsible for capping
/// `git log` output (`-n 20`); `parse_oneline` does not impose its own cap.
pub(crate) fn parse_oneline(stdout: &str) -> Vec<JournalCommit>;
```

```rust
// ark-core/src/commands/agent/workspace/identity.rs

pub fn validate_developer_name(name: &str) -> Result<()>;
pub fn read_developer_name(layout: &Layout) -> Result<Option<String>>;
pub fn require_developer_name(layout: &Layout) -> Result<String>;
pub fn write_developer_file(layout: &Layout, name: &str, now: DateTime<Utc>) -> Result<()>;
```

```rust
// ark-core/src/commands/agent/workspace/index.rs

pub fn seed_index(path: &Path, dev: &str) -> Result<()>;
pub fn rerender(layout: &Layout, dev: &str) -> Result<()>;
fn render_status_block(active_file: &str, total: u32, last_active: NaiveDate) -> String;
fn render_sessions_block(entries: &[ParsedEntry]) -> String;

const INDEX_RERENDER_JOURNAL_CAP: usize = 100;
const INDEX_RERENDER_ENTRIES_PER_JOURNAL_CAP: usize = 100;
```

```rust
// ark-core/src/error.rs (additions)

Error::DeveloperNotInitialized { path: PathBuf },
Error::DeveloperAlreadyInitialized { name: String },
Error::WorkspaceConfigCorrupt { path: PathBuf, source: toml::de::Error },
Error::JournalRotationLimit { dev: String, max: u32 },
Error::InvalidDeveloperName { name: String, reason: &'static str },
```

```rust
// ark-core/src/layout.rs (additions)

pub const WORKSPACE_DIR: &str = ".ark/workspace";
pub const CONFIG_FILE: &str = ".ark/config.toml";
pub const DEVELOPER_FILE: &str = ".ark/.developer";

#[derive(Debug, Clone)]
pub struct Layout { /* ... */ }

impl Layout {
    pub fn workspace_dir(&self) -> PathBuf;
    pub fn workspace_developer_dir(&self, dev: &str) -> PathBuf;
    pub fn workspace_index(&self, dev: &str) -> PathBuf;
    pub fn workspace_journal(&self, dev: &str, n: u32) -> PathBuf;
    pub fn config_file(&self) -> PathBuf;
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

`PathExt::append_text` is added via the existing `PathExt` re-export (no new public name in `lib.rs`). Internal helpers (`identity::*`, `journal::*` except `parse_oneline` which is `pub(crate)`, `index::*`) are NOT re-exported.

[**Constraints**]

- **C-1 (process spawn locality):** All git invocations route through `io::git::run_git`. `Command::new` MUST NOT appear under `commands/agent/workspace/`. Extends the existing source-scan test.
- **C-2 (path/io discipline):** All filesystem access in `commands/agent/workspace/` routes through `io::PathExt` (no bare `std::fs::*`, no bare `std::fs::OpenOptions`). All `.ark/`-relative paths route through `Layout` helpers (no string concatenation). The append primitive is `PathExt::append_text` (G-17); raw `OpenOptions::append` is forbidden in this module.
- **C-3 (developer name validation):** Name MUST match `^[A-Za-z][A-Za-z0-9_-]{0,39}$` — leading ASCII letter, then up to 39 more characters from `[A-Za-z0-9_-]`. Empty, whitespace-only, leading digit, leading dash, contains `/` or `\\`, contains `.`, > 40 chars, non-ASCII → `Error::InvalidDeveloperName { name, reason }`. Filesystem hostility: name is also the *directory name* under `.ark/workspace/<name>/`. Cross-platform OS-reserved names are NOT pre-validated.
- **C-4 (`.developer` file format):** Two key=value lines, `\n`-separated, UTF-8: `name=<x>\ninitialized_at=<RFC3339>\n`. Reader is line-oriented; ignores blank lines and unknown keys for forward compat. Trailing newline required.
- **C-5 (idempotent init):** Running `workspace init --name <x>` twice with the same name on a fully-scaffolded workspace is a no-op (`created: false`). With a different name → `Error::DeveloperAlreadyInitialized` (no automatic rename).
- **C-6 (per-checkout journal):** `workspace_record` and `record_task` write to the **current checkout's** `.ark/workspace/<dev>/`. From inside a git worktree, the journal lands in the worktree's tree, not the parent's. The session entry is committed branch content and rides along with the task commit. Different branches may carry independent histories that git merges like any other file.
- **C-7 (snapshot exclusion):** `commands/unload.rs` MUST skip `layout.developer_file()` in **both** filesystem walks: the Stage A `unload` capture loop AND the Stage B `capture_orphan_hook_entries` walk. Implementation: append `layout.developer_file()` to each call's existing `skip_under` slice. The existing `walk_files_excluding` matches via `Path::starts_with`; a single file path is a valid prefix, so no API change is needed. The file is gitignored and per-machine — capturing it would leak identity. `.ark/workspace/<dev>/` is captured normally.
- **C-8 (managed-block re-render):** `index.md` is regenerated from journal contents on every `workspace_record` and `record_task` via `update_managed_block` for `ARK:WORKSPACE_STATUS` and `ARK:WORKSPACE_SESSIONS`. Hand-edits *outside* the managed blocks are preserved.
- **C-9 (journal rotation):** Rotate when `line_count(active) >= cfg.journal_max_lines` AT THE START of a record call. `cfg.journal_max_lines < 100` → `Error::InvalidConfigField`. `N+1 > 9999` → `Error::JournalRotationLimit`.
- **C-10 (config.toml lifecycle):** Created by `ark init` from `templates/ark/config.toml`. `ark upgrade` does NOT overwrite it. Sectioned: `[worktree]` and `[workspace]` sub-tables; missing section → that feature's defaults.
- **C-11 (auto-record best-effort):** `record_task` MUST NOT propagate `Error::DeveloperNotInitialized` — returns `WorkspaceRecorded::SkippedNoIdentity`. `auto_record_on_archive = false` → returns `WorkspaceRecorded::SkippedDisabled` without reading identity or invoking git. All other errors propagate. `task_archive` does NOT roll back the archive on workspace failure.
- **C-12 (output stability):** Every workspace command writes a single `Display` summary; no ad-hoc stdout writes. The single `eprintln!` in `record_task`'s skip-no-identity path is the documented exception (stderr, not stdout).
- **C-13 (commit collection cap):** `git log` invocations cap at 20 entries via `-n 20`. For `record_task`, range is `<base_branch>..HEAD` when `base_branch` is `Some`; fallback to `-n 20 --oneline` when `None`. For manual `workspace_record`, always `-n 20 --oneline`. Output silently truncated; no truncation header. Parsed via `journal::parse_oneline` (a `pub(crate)` helper documented in Data Structure): splits each non-empty line at the first ASCII space, prefix is the short hash and suffix (trimmed) is the message.
- **C-14 (CLI hidden under agent):** `ark agent workspace` follows the same hidden semantics as the rest of `ark agent`. `ark --help` does NOT list `agent`; `ark agent workspace --help` is reachable and includes "Not covered by semver".
- **C-15 (one-way coupling):** `task::archive` imports `super::workspace::record::record_task`. `workspace::*` modules MUST NOT import `super::task` or `super::spec`. Dependency direction enforced by code review.
- **C-16 (gitignore template lifecycle):** `templates/ark/.gitignore` is fully Ark-owned (no managed block). `ark upgrade` reapplies the canonical content.
- **C-17 (slash command parity):** `templates/claude/commands/ark/record.md`, `templates/codex/skills/ark-record/SKILL.md`, and `templates/opencode/commands/ark/record.md` MUST stay in lockstep on every change to the slash command body.
- **C-18 (auto-record placement, tier-agnostic):** `task::archive::task_archive` MUST invoke `record_task` exactly once per call, **after** the `.current` cleanup block AND **before** the `Ok(summary)` return, **regardless of tier** (quick / standard / deep). The deep-tier `spec_extract` + `spec_register` block runs earlier in `task_archive` and is unaffected by this change. `record_task` receives `archive_path` via `RecordTaskOptions` — it is a clone of the same value `task_archive` computed earlier in its frame, after the rename, and points to the now-archived task dir on disk. Regression tests: `quick_tier_archive_records_session`, `standard_tier_archive_records_session`, `deep_tier_archive_records_session`.
- **C-19 (entry-boundary parser predicate):** `parse_entries(text: &str) -> Vec<ParsedEntry>` operates on the concatenated text of one journal file. Boundary rules:
  - **Entry start:** a line matching `^## Session (\d+):\s+(.+?)\s*$` outside of a fenced code block.
  - **Entry end:** the *next* line matching the start regex outside of a fenced code block, OR EOF.
  - **Fenced code blocks:** the parser recognizes both backtick fences and tilde fences. The opening-fence regex is `^([` + "`" + `~]{3,})(\w+)?\s*$` (line-start, 3+ of either character, optional language tag). The matching close fence MUST use the **same character type** AND be **at least as long** as the opening fence. Lines inside an active fenced block are ignored for boundary detection. A fence opened but never closed at EOF is treated as still-open (everything after the open fence is excluded from boundary detection — defensive against malformed pastes).
  - **Indented code blocks (CommonMark 4-space rule) are NOT treated as fenced** by this parser. A `## Session N:` line with 4 leading spaces is technically code in CommonMark, but the parser still matches it as an entry start. This is a deliberate simplification (NG-13).
  - **Within an entry body**, the parser extracts: `**Date**: YYYY-MM-DD` (first match), `**Kind**: (task|manual)`, `**Slug**: <slug>` (or `-`), `**Branch**: \`<branch>\``, and the `### Commits` table row count (lines under `### Commits` matching `^\| \``).
  - Malformed entries (missing required fields) are silently skipped during re-render with a single stderr warning per re-render call; the journal file itself is not modified.
- **C-20:** *(removed: parent-root resolution. Journal writes target the current `Layout` directly; no parent traversal.)*

- **C-21 (index re-render hard ceiling):** `index::rerender(&layout, &dev)` reads at most **100 journal files** (`journal-1.md` … `journal-100.md`) and parses at most **100 entries per file**. Beyond the cap, additional entries are silently truncated from the rendered sessions table; the journal files themselves remain canonical. The cap is a constant in `index.rs`: `INDEX_RERENDER_JOURNAL_CAP: usize = 100`, `INDEX_RERENDER_ENTRIES_PER_JOURNAL_CAP: usize = 100`.

---
