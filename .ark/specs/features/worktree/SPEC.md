[**Goals**]

- G-1: `task new --worktree` creates a git worktree at `.ark/worktrees/<branch>/` and scaffolds the task dir inside it.
- G-2: `task worktree cleanup` removes a worktree dir post-merge; `task worktree list` enumerates active worktree-backed tasks.
- G-3: `[worktree]` section of `.ark/config.toml` configures `worktree_dir`, `branch_prefix`, `copy`, `post_create`.
- G-4: `--worktree` is opt-in for every tier; `task new` without it is unchanged.
- G-5: Each worktree's `.ark/` is independent — `ark context` and `ark agent task ...` invoked inside a worktree see its own state.

[**Non-goals**]

- NG-1: No automatic worktree creation; `--worktree` is opt-in.
- NG-2: No automatic cleanup on archive; cleanup is a separate user step.
- NG-3: No PR-creation integration (`gh pr create`).

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                       (TaskNewCliArgs gains --worktree,
│                                               --branch-type, --branch;
│                                               adds WorktreeCommand{Cleanup, List})
└── ark-core/src/
    ├── lib.rs                                 (re-exports public worktree API)
    ├── error.rs                               (new variants — see Data Structure)
    ├── layout.rs                              (worktrees_dir, worktree_dir(branch),
    │                                            worktree_config_file;
    │                                            WORKTREES_DIR, WORKTREE_CONFIG_FILE consts)
    ├── io/
    │   ├── fs.rs                              (walk_files_excluding(root, skip_under))
    │   └── git.rs                             (unchanged; run_git suffices)
    └── commands/
        ├── unload.rs                          (uses walk_files_excluding skipping worktrees_dir())
        ├── upgrade.rs                         (same)
        └── agent/
            ├── mod.rs                         (pub use task::worktree::*)
            ├── state.rs                       (TaskToml: + branch, + worktree_path,
            │                                    + base_branch — all Option, #[serde(default)])
            ├── task/
            │   ├── mod.rs                     (pub mod worktree)
            │   ├── new.rs                     (gains worktree-first path)
            │   └── worktree/                  (NEW)
            │       ├── mod.rs                 (public types + dispatch)
            │       ├── config.rs              (WorktreeConfig — worktree.toml model)
            │       ├── cleanup.rs             (worktree_cleanup)
            │       ├── list.rs                (worktree_list)
            │       └── discovery.rs           (git worktree list parsing helper)
templates/
├── ark/
│   ├── worktree.toml                          (NEW; shipped default config)
│   ├── workflow.md                            (adds Worktree subsection)
│   └── .gitignore (managed block)             (adds `.ark/worktrees/` line)
```

Module coupling: `task::new` imports `task::worktree::{config, discovery}` (one-way). `task::worktree::{cleanup, list}` import `discovery` and `config`. `discovery` is a leaf — only `git` and `task.toml` knowledge.

Call graph for `task new --worktree`:

```
task::new::task_new(opts)
  ├── if opts.worktree.is_some():
  │     ├── validate_slug, validate_title
  │     ├── reject_if_under_worktrees(cwd, layout)        → Error::NestedWorktreeForbidden
  │     ├── if layout.task_dir(slug).exists():            → Error::TaskExistsOnParent
  │     ├── cfg = WorktreeConfig::load_or_default(layout)
  │     ├── branch = resolve_branch(opts.worktree, &cfg, slug)
  │     ├── git_check_ref_format(branch)                  → Error::InvalidBranchName
  │     ├── base_branch = run_git(["symbolic-ref","--short","HEAD"], root)
  │     │     fall back to `git rev-parse HEAD` 40-char SHA on detached HEAD
  │     ├── reject_if_branch_in_use(branch, root)         → Error::BranchInUse
  │     ├── worktree_path = layout.worktree_dir(&branch)
  │     ├── if worktree_path.exists():                    → Error::WorktreeDirExists
  │     ├── run_git(["worktree","add","-b",branch,wt,base], root)
  │     │     ── rollback boundary: any failure from here runs cleanup_worktree(wt, branch)
  │     ├── scaffold_task_dir_in_worktree(wt, slug, title, tier, branch, base_branch)
  │     │     creates <wt>/.ark/tasks/<slug>/, copies PRD template, writes task.toml
  │     ├── for f in cfg.copy:
  │     │     if !root.join(f).exists():                  → Error::WorktreeCopySourceMissing
  │     │     copy(root.join(f), wt.join(f))
  │     ├── for cmd in cfg.post_create:
  │     │     run_shell(cmd, cwd=wt)                      → Error::PostCreateHookFailed
  │     └── return TaskNewSummary (extended Display: branch + worktree_path on --worktree)
  └── else: existing flow unchanged
```

`cleanup_worktree(wt, branch)` runs `git worktree remove --force <wt>` and `git branch -D <branch>` best-effort. If rollback itself fails, the original error is returned with the rollback error chained.

Call graph for `task worktree cleanup`:

```
worktree::cleanup::worktree_cleanup(opts)
  ├── (slug, wt) = discovery::find_worktree_for_slug(slug, layout)
  │     walks `git worktree list --porcelain`, reads each <wt>/.ark/tasks/.current
  │     returns (slug, wt) pair, or None
  ├── if wt is None:                                      → Error::WorktreeNotFound
  ├── toml = TaskToml::load(<wt>/.ark/tasks/<slug>)
  ├── if !opts.force:
  │     status = run_git(["status","--porcelain"], wt)
  │     status non-empty                                  → Error::WorktreeDirty
  ├── run_git(["worktree","remove", maybe_force, wt], root)
  ├── if opts.delete_branch:
  │     run_git(["branch", if force {"-D"} else {"-d"}, branch], root)
  ├── prune_empty_parents(wt.parent(), up_to: layout.worktrees_dir())
  └── return WorktreeCleanupSummary { slug, branch, branch_deleted, worktree_path: Some(wt), already_clean: false }
```

Call graph for `task worktree list`:

```
worktree::list::worktree_list(opts)
  ├── worktrees = parse_git_worktree_list(root)
  ├── for wt in worktrees:
  │     if !wt.is_under(layout.worktrees_dir()): skip
  │     current = read_text(wt/.ark/tasks/.current).ok()?
  │     toml    = TaskToml::load(wt/.ark/tasks/current).ok()?
  │     rows.push(WorktreeRow { slug, branch, worktree_path, updated_at })
  ├── sort rows by toml.updated_at desc
  └── return WorktreeListSummary { rows }     // Display: empty when rows empty
```

[**Data Structure**]

```rust
// ark-core/src/commands/agent/state.rs (TaskToml additions)
pub struct TaskToml {
    // ...existing fields unchanged...
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub worktree_path: Option<PathBuf>,    // project-relative — C-21
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_branch: Option<String>,
}

// ark-core/src/commands/agent/task/worktree/config.rs
#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeConfig {
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,    // project-relative; absolute → error
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,   // default "feat"
    #[serde(default)]
    pub copy: Vec<String>,       // project-relative paths
    #[serde(default)]
    pub post_create: Vec<String>, // shell commands
}

impl WorktreeConfig {
    pub fn load_or_default(layout: &Layout) -> Result<Self>;
}

fn default_worktree_dir() -> String { ".ark/worktrees".into() }
fn default_branch_prefix() -> String { "feat".into() }

// ark-core/src/commands/agent/task/worktree/mod.rs (public types)
#[derive(Debug, Clone)]
pub struct WorktreeCleanupOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub delete_branch: bool,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct WorktreeCleanupSummary {
    pub slug: String,
    pub branch: Option<String>,
    pub branch_deleted: bool,
    pub worktree_path: Option<PathBuf>,
    pub already_clean: bool,
}

#[derive(Debug, Clone)]
pub struct WorktreeListOptions { pub project_root: PathBuf }

#[derive(Debug, Clone)]
pub struct WorktreeListSummary { pub rows: Vec<WorktreeRow> }

#[derive(Debug, Clone)]
pub struct WorktreeRow {
    pub slug: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub updated_at: DateTime<Utc>,
}
// All summaries impl Display.

// ark-core/src/commands/agent/task/new.rs (TaskNewOptions additions)
pub struct TaskNewOptions {
    // ...existing fields unchanged...
    pub worktree: Option<TaskNewWorktree>,
}

#[derive(Debug, Clone)]
pub struct TaskNewWorktree {
    pub branch_type: Option<String>,    // None → cfg.branch_prefix
    pub branch_override: Option<String>, // wins over branch_type if Some
}

pub struct TaskNewSummary {
    // existing fields...
    pub worktree: Option<TaskNewWorktreeSummary>,
}

pub struct TaskNewWorktreeSummary {
    pub branch: String,
    pub worktree_path: PathBuf,
    pub base_branch: String,
}

// ark-core/src/error.rs (additions)
Error::WorktreeDirExists         { path: PathBuf },
Error::WorktreeNotFound          { slug: String },
Error::WorktreeDirty             { path: PathBuf },
Error::BranchInUse               { branch: String, where_at: PathBuf },
Error::InvalidBranchName         { branch: String, reason: String },
Error::InvalidBranchType         { value: String },
Error::WorktreeConfigCorrupt     { path: PathBuf, source: toml::de::Error },
Error::PostCreateHookFailed      { command: String, exit_code: i32 },
Error::WorktreeCopySourceMissing { path: PathBuf },
Error::TaskExistsOnParent        { slug: String, path: PathBuf },
Error::NestedWorktreeForbidden   { current_root: PathBuf },
Error::InvalidConfigField        { field: &'static str, reason: &'static str },

// ark-core/src/layout.rs (additions)
pub const WORKTREES_DIR:        &str = ".ark/worktrees";
pub const WORKTREE_CONFIG_FILE: &str = ".ark/worktree.toml";

impl Layout {
    pub fn worktrees_dir(&self)        -> PathBuf;
    pub fn worktree_dir(&self, branch: &str) -> PathBuf;
    pub fn worktree_config_file(&self) -> PathBuf;
}

// ark-core/src/io/fs.rs (extension)
/// Recursively enumerate files under `root`, pruning any subtree whose path
/// starts with one of the `skip_under` prefixes. Existing `walk_files` is
/// the zero-skip case (or becomes a thin wrapper).
pub fn walk_files_excluding(
    root: impl AsRef<Path>,
    skip_under: &[impl AsRef<Path>],
) -> Result<Vec<PathBuf>>;
```

[**API Surface**]

```rust
pub fn worktree_cleanup(opts: WorktreeCleanupOptions) -> Result<WorktreeCleanupSummary>;
pub fn worktree_list(opts: WorktreeListOptions)       -> Result<WorktreeListSummary>;

pub fn walk_files_excluding(
    root: impl AsRef<Path>,
    skip_under: &[impl AsRef<Path>],
) -> Result<Vec<PathBuf>>;

// Library re-exports added: WorktreeConfig, the two option/summary pairs,
// WorktreeRow, worktree_cleanup, worktree_list, walk_files_excluding.
// TaskNewOptions extended.

// CLI shape (ark-cli/src/main.rs)
#[derive(Subcommand)]
enum TaskCommand {
    New(TaskNewCliArgs),
    Plan(TaskSlugArgs),
    Review(TaskSlugArgs),
    Execute(TaskSlugArgs),
    Verify(TaskSlugArgs),
    Archive(TaskSlugArgs),
    Promote(TaskPromoteCliArgs),
    Worktree(WorktreeArgs),       // Cleanup + List only
}

#[derive(clap::Args)]
struct TaskNewCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(long)] slug: String,
    #[arg(long)] title: String,
    #[arg(long, value_parser = parse_tier)] tier: Tier,
    #[arg(long)] worktree: bool,
    #[arg(long = "branch-type", conflicts_with = "branch", requires = "worktree")]
    branch_type: Option<String>,
    #[arg(long, conflicts_with = "branch_type", requires = "worktree")]
    branch: Option<String>,
}

#[derive(Subcommand)]
enum WorktreeCommand {
    /// Remove the worktree dir; optionally delete the branch.
    Cleanup(WorktreeCleanupCliArgs),
    /// List active worktree-backed tasks.
    List(WorktreeListCliArgs),
}

#[derive(clap::Args)]
struct WorktreeCleanupCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(long = "delete-branch")] delete_branch: bool,
    #[arg(long)] force: bool,
}
```

[**Constraints**]

- C-1: @test-binding: load_or_default_errors_on_corrupt_toml
`.ark/config.toml` `[worktree]` section is parsed via `toml`; missing file → defaults; corrupt → `Error::WorktreeConfigCorrupt { path, source }` with source error chained.
- C-2: @test-binding: cleanup_happy_path_with_delete_branch
Worktree-first protocol: validate → check parent collision → load config → resolve branch → check ref format → check base branch → check branch-in-use → check worktree path → `git worktree add` → scaffold task dir inside worktree → copy files → run `post_create`. Steps after `git worktree add` are inside a rollback boundary that runs `git worktree remove --force <wt>` and `git branch -D <branch>` on any failure.
- C-3: @test-binding: missing_worktree_section_returns_defaults
`task.toml`'s three new fields are `Option<_>` with `#[serde(default)]`; pre-existing files deserialize unchanged.
- C-4: @test-binding: resolve_branch_rejects_unknown_branch_type
`--branch-type` accepts `{feat, fix, refactor, chore, ci, docs}`; other values → `Error::InvalidBranchType`. The list lives as `BRANCH_TYPES: &[&str; 6]` in `task/new.rs`.
- C-5: @judgment
`--branch <full>` bypasses type validation but goes through `git check-ref-format --branch <name>`; non-zero → `Error::InvalidBranchName`.
- C-6: @test-binding: resolve_branch_precedence
Branch resolution: `--branch` if present, else `<--branch-type>/<slug>`, else `<cfg.branch_prefix>/<slug>`. Slug appended verbatim — no auto-transformation.
- C-7: @test-binding: unload_excludes_worktree_contents
Both `unload.rs` walk callsites (`owned_dirs` snapshot capture loop AND `capture_orphan_hook_entries`) use `walk_files_excluding(root, &[layout.worktrees_dir()])`. Lexical match on absolute paths; no symlink canonicalization.
- C-8: @judgment
`worktree_path` lives under `<root>/<cfg.worktree_dir>/<branch>/`, joined via `Layout::worktree_dir(&branch)`. `cfg.worktree_dir` is project-relative; absolute paths → `Error::InvalidConfigField { field: "worktree_dir", reason: "must be project-relative" }`.
- C-9: @judgment
`worktree.toml` is created by `ark init` from `templates/ark/worktree.toml`; `ark upgrade` does NOT overwrite it. `ark unload`/`load` capture/restore it like other `.ark/` content.
- C-10: @source-scan: Command::new @ crates/ark-core/src/commands/agent/task/worktree/**/*.rs
All git invocations route through `io::git::run_git`. `Command::new` may NOT appear under `commands/agent/task/worktree/` (extends the existing source-scan test).
- C-11: @judgment
All filesystem access in `task/worktree/` and the `--worktree` path of `task/new.rs` routes through `io::PathExt`. All `.ark/`-relative paths route through `Layout`.
- C-12: @test-binding: cleanup_happy_path_with_delete_branch
`task.toml.branch` stores the resolved branch verbatim; no parsing into `branch_type` + `slug` parts.
- C-13: @judgment
`task new --worktree` rejects when `cwd` resolves under any `*/.ark/worktrees/*/`. Detection: `Layout::discover_from(cwd)?.root()` matched against `*/.ark/worktrees/*/`.
- C-14: @test-binding: worktree_list_is_empty_when_no_worktrees_exist
`worktree list` prints `<row>\n` per row; zero rows → empty stdout, exit 0. No stderr noise.
- C-15: @test-binding: cleanup_happy_path_with_delete_branch
After `git worktree remove`, cleanup walks parent dirs upward (not crossing `worktrees_dir()`) and removes empty ones.
- C-16: @judgment
`templates/ark/.gitignore` (managed-block, marker `ARK`) carries the `.ark/worktrees/` line; re-applied on `init` and `upgrade`.
- C-17: @judgment
Every command writes a single `Display` summary; no ad-hoc stdout writes in command bodies.
- C-18: @test-binding: archive_leaves_worktree_intact
`task archive` MUST NOT remove the worktree dir, delete the branch, or modify `task.toml.worktree_path`.
- C-19: @test-binding: task_new_without_worktree_writes_nothing_under_worktrees_dir
`task new` without `--worktree` writes nothing under `<root>/<cfg.worktree_dir>/`.
- C-20: @test-binding: worktree_list_reports_active_tasks
`worktree cleanup` discovery parses `git worktree list --porcelain` and reads each candidate's `.ark/tasks/.current`. Worktrees with missing/unreadable `.current` or `task.toml` are silently skipped (non-Ark third-party worktrees may live under `worktrees_dir()`).
- C-21: @judgment
`task.toml.worktree_path` is project-relative with forward-slash separators; consumers join against `layout.root()` for absolute use. Keeps `.ark.db` snapshots portable across machine moves.
- C-22: @judgment
`task new --worktree` mirrors developer identity from the parent's `.ark/.developer` into the new worktree. On `MissingIdentity` and TTY, prompts via `identity_prompt` (max 3 attempts) then writes both. Failure rolls back.

[**CHANGELOG**]

- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
