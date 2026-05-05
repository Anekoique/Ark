
[**Goals**]

- **G-1:** Project-tracked `[worktree]` section of `.ark/config.toml` (TOML, parsed via the `toml` crate) carrying four keys, all optional with defaults: `worktree_dir = ".ark/worktrees"`, `branch_prefix = "feat"`, `copy = []`, `post_create = ["git submodule update --init --recursive"]`. Missing file or section is not an error — defaults apply. Created with documented defaults by `ark init`, never overwritten by `ark upgrade` (per C-9). The submodule-init default is a no-op in repos without `.gitmodules` (`git submodule update` exits 0); users disable it explicitly with `post_create = []`.

- **G-2:** Worktree creation is bound to `task new`, not exposed as a separate subcommand. The CLI flag `task new --slug <s> --tier <t> --worktree [--branch-type <t>] [--branch <full>]` is the *sole* path that creates a worktree. The flag is opt-in for every tier; `task new` without `--worktree` is unchanged. Two `task worktree` subcommands exist for post-creation lifecycle: `cleanup` and `list`.

- **G-3:** When `--worktree` is given, the create flow runs in this exact order:
  1. Validate slug, title, branch-type/branch combination, and that `cwd` is not under any `*/.ark/worktrees/*/` path (C-13).
  2. Reject if `<root>/.ark/tasks/<slug>/` already exists on the parent → `Error::TaskExistsOnParent`.
  3. Load `worktree.toml` (or defaults).
  4. Resolve branch name: `--branch <full>` if given, else `<--branch-type or cfg.branch_prefix>/<slug>`.
  5. Resolve `base_branch`: try `git symbolic-ref --short HEAD` in the project root; on failure (detached HEAD), fall back to `git rev-parse HEAD` and store the 40-char SHA verbatim. Unborn HEAD (no commits) errors via the underlying `run_git` Io error and aborts (F-16).
  6. Run `git check-ref-format --branch <branch>`; reject on non-zero with `Error::InvalidBranchName`.
  7. Run `git worktree list --porcelain`; reject if `<branch>` already checked out anywhere with `Error::BranchInUse { branch, where_at }`.
  8. Compute `worktree_path = <root>/<cfg.worktree_dir>/<branch>/`. If the path exists on disk, error `Error::WorktreeDirExists { path }`.
  9. `git worktree add -b <branch> <worktree_path> <base_branch>`. Failure → return error; nothing else mutated.
  10. **Inside the worktree**, scaffold the task dir: create `<worktree>/.ark/tasks/<slug>/`, copy `PRD.md` template, write `task.toml` with `phase = "design"`, `iteration = 0`, `tier`, `branch = Some(<branch>)`, `worktree_path = Some(<worktree_path_relative>)` (project-relative; see C-21), `base_branch = Some(<base_branch>)`.
  11. Update `<worktree>/.ark/tasks/.current` to point at `<slug>`.
  12. For each path in `cfg.copy`: copy `<root>/<path>` into `<worktree>/<path>`. Source-missing → hard fail with `Error::WorktreeCopySourceMissing { path }`; rollback per Failure Modes.
  13. For each command in `cfg.post_create`: run shell command in `<worktree>` cwd, sequential, abort on first non-zero with `Error::PostCreateHookFailed`; rollback per Failure Modes.
  14. Print one-line `TaskNewSummary` (extended `Display` to include the branch + worktree path when `--worktree` was used).

  The parent checkout's `.ark/tasks/`, `.ark/tasks/.current`, and working tree are **never modified** by `--worktree`.

- **G-4:** `task worktree cleanup [--slug <s>] [--delete-branch] [--force]` removes the worktree dir and optionally deletes the branch. The cleanup sequence:
  1. Discover via `git worktree list --porcelain`, reading each candidate worktree's `.ark/tasks/.current` to find the slug match (C-20).
  2. If discovery returns nothing → `Error::WorktreeNotFound { slug }`. (Acts as success-on-second-cleanup per F-15.)
  3. Load `task.toml` from the discovered worktree.
  4. If not `--force`, run `git status --porcelain` in the worktree dir. Non-empty stdout → `Error::WorktreeDirty { path }`.
  5. Run `git worktree remove [--force] <path>`.
  6. If `--delete-branch`, run `git branch -d` (or `-D` with `--force`) for `task.toml.branch`.
  7. Prune empty parent directories under `worktrees_dir()` (C-15).
  8. Return `WorktreeCleanupSummary { slug, branch, branch_deleted, worktree_path: Some(wt), already_clean: false }`.

  Cleanup runs from any checkout that can see the parent repo's git state — typically the parent. There is no surviving `task.toml` to update post-cleanup (the file lived in the now-removed worktree).

- **G-5:** `task worktree list` enumerates active worktree-backed tasks by running `git worktree list --porcelain` from the project root, then for each worktree path that is *under* `<root>/<cfg.worktree_dir>/`, reading its `.ark/tasks/.current`, then `task.toml`. Output: one line `<slug> <branch> <worktree_path>` per row, sorted by `task.toml.updated_at` descending. Zero rows → empty stdout, exit 0 (C-14). Pre-archive only (archived tasks live under `tasks/archive/`, not `tasks/`, so they're naturally excluded). Worktrees whose `.ark/tasks/.current` or `task.toml` is missing/unreadable are silently skipped — this is the documented behavior for non-Ark third-party worktrees that may live under `worktrees_dir()` (R-108 lock-in).

- **G-6:** `TaskToml` gains three optional fields: `branch: Option<String>`, `worktree_path: Option<PathBuf>`, `base_branch: Option<String>`. All `#[serde(default, skip_serializing_if = "Option::is_none")]`. Pre-existing `task.toml` files (without these fields) deserialize unchanged.

- **G-7:** `task new --worktree` is opt-in for every tier including deep. Without `--worktree`, behavior is identical to pre-existing `task new`.

- **G-8:** `task archive` does not modify the worktree dir or branch. The archive moves `.ark/tasks/<slug>/` → `.ark/tasks/archive/YYYY-MM/<slug>/` *within whatever checkout it runs in* — typically the worktree, since that's where the task lives. The branch and worktree dir on disk remain intact. Cleanup is the user's separate explicit step.

- **G-9:** `Layout::discover_from(cwd)` correctly resolves the worktree as the project root when invoked from inside one — because the worktree has its own `.ark/` (created by step 10 of G-3 inside the worktree). `ark context`, `ark agent task plan`, and other commands invoked inside the worktree see the worktree's own `.current` and `task.toml` — exactly the parallelism the feature enables.

- **G-10:** `walk_files` gains a sibling helper `walk_files_excluding(root, skip_under: &[impl AsRef<Path>])` that prunes any subtree whose path starts with one of the skip prefixes. **Both** filesystem-walking callsites in `unload.rs` (the `owned_dirs` capture loop and `capture_orphan_hook_entries`) pass `&[layout.worktrees_dir()]` so they do not descend into worktree storage. `upgrade::extract` does NOT need this change: per R-101 verification, it walks the embedded `ARK_TEMPLATES` (`include_dir!`) tree, not the user's filesystem. The original `walk_files(root)` is preserved as a thin wrapper that calls `walk_files_excluding(root, &[] as &[PathBuf])`; existing call sites are unaffected.

- **G-11:** `templates/ark/.gitignore` (managed-block, marker `ARK`) gains `.ark/worktrees/` line. Idempotent via `update_managed_block`. The line prevents *the parent's* git index from picking up worktree dirs (each worktree is a separate working tree in git's eyes; the parent's index would otherwise see them as untracked dirs).

- **G-12:** `templates/ark/workflow.md` and the in-repo `.ark/workflow.md` gain a `### Worktree (optional)` subsection under §6 Mechanics describing `task new --worktree`, `task worktree cleanup`, `task worktree list`, and noting that `worktree.toml` is user-editable.

- **G-13 (identity sync):** `task new --worktree` mirrors developer identity from the parent's `.ark/.developer` into the new worktree. Step runs after `register_focus` and before the `cfg.copy` loop, inside the existing rollback boundary (C-2). Sequence: (1) `identity_resolve(parent)` — on `Ok(id)`, `identity_write(worktree, &id)`; (2) on `Err(MissingIdentity)` and `std::io::stdin().is_terminal() == true`, prompt via `identity_prompt` (writer = stderr, max 3 attempts), then `identity_write(parent, &id)` followed by `identity_write(worktree, &id)`; (3) on `Err(MissingIdentity)` and non-TTY, return `Error::MissingIdentity` unchanged so agents surface the existing remediation message ("run `ark init --developer <name>` ..."); (4) any other error from `identity_resolve` / `identity_write` propagates unchanged. Failure triggers `rollback_worktree`. No new error variants; reuses `Error::MissingIdentity` and `Error::DeveloperWriteFailed`.

[**Non-goals**]

- **NG-1:** No per-developer state outside the worktree's own `.ark/`.
- **NG-2:** No agent process supervision. The user `cd`s into the worktree and runs their own AI agent.
- **NG-3:** No registry of running PIDs.
- **NG-4:** No automatic worktree creation. `--worktree` is opt-in for all tiers.
- **NG-5:** No automatic cleanup on archive.
- **NG-6:** No worktree rename or move post-creation. Re-create with a new branch if needed.
- **NG-7:** No automatic submodule logic in Ark code. Submodule init in new worktrees is achieved via the documented default `post_create` shell command (G-1); the worktree code itself remains submodule-agnostic and just runs the configured shell. Users who want to opt out set `post_create = []` in `.ark/config.toml`.
- **NG-8:** No structured-output JSON for `task worktree list`. One line per row, like other `agent` commands.
- **NG-9:** No PR-creation integration (`gh pr create`). Out of scope.
- **NG-10:** No standalone `task worktree create`. Worktree creation only via `task new --worktree`. Migration of pre-existing parent-only task dirs into a worktree is unsupported (see `Error::TaskExistsOnParent`).
- **NG-11:** No update to `/ark:design` / `/ark:quick` slash commands. They stay worktree-agnostic; users opt in by passing `--worktree` to the underlying `ark agent task new`.
- **NG-12:** No cross-worktree task synchronization. Each worktree's `.ark/` is the branch's view; conflicts resolve at merge time.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                       — extends TaskNewCliArgs with --worktree,
│                                               --branch-type, --branch; adds
│                                               WorktreeCommand{Cleanup,List}
└── ark-core/src/
    ├── lib.rs                                 — re-exports public worktree API
    ├── error.rs                               — new variants (see Data Structure)
    ├── layout.rs                              — adds worktrees_dir, worktree_dir(branch),
    │                                            worktree_config_file
    ├── io/
    │   ├── fs.rs                              — adds walk_files_excluding(root, skip_under)
    │   └── git.rs                             — unchanged (run_git suffices)
    └── commands/
        ├── unload.rs                          — uses walk_files_excluding skipping worktrees_dir()
        ├── upgrade.rs                         — same
        └── agent/
            ├── mod.rs                         — pub use task::worktree::*
            ├── state.rs                       — TaskToml: + branch, + worktree_path,
            │                                    + base_branch (Option, #[serde(default)])
            ├── task/
            │   ├── mod.rs                     — pub mod worktree
            │   ├── new.rs                     — gains worktree-first path
            │   └── worktree/                  — NEW
            │       ├── mod.rs                 — public types + dispatch
            │       ├── config.rs              — WorktreeConfig (worktree.toml model)
            │       ├── cleanup.rs             — worktree_cleanup
            │       ├── list.rs                — worktree_list
            │       └── discovery.rs           — git worktree list parsing helper
templates/
├── ark/
│   ├── worktree.toml                          — NEW: shipped default config
│   ├── workflow.md                            — adds §6 Worktree subsection
│   └── .gitignore (managed block)             — adds `.ark/worktrees/` line
```

**Module coupling.** `task::new` imports `task::worktree::config` and `task::worktree::discovery` (one-way: `task::new` → `task::worktree`). `task::worktree::{cleanup, list}` import `discovery` and `config`. `discovery` is leaf; only `git` and `task.toml` knowledge.

**Call graph for `task new --worktree`:**

```
task::new::task_new(opts)
  ├── if opts.worktree.is_some():
  │     ├── validate_slug, validate_title
  │     ├── reject_if_under_worktrees(cwd, layout)             → Error::NestedWorktreeForbidden
  │     ├── if layout.task_dir(slug).exists():                 → Error::TaskExistsOnParent
  │     ├── cfg = WorktreeConfig::load_or_default(layout)
  │     ├── branch = resolve_branch(opts.worktree, &cfg, slug)
  │     ├── git_check_ref_format(branch)                       → Error::InvalidBranchName
  │     ├── base_branch = run_git(["symbolic-ref","--short","HEAD"], root)
  │     ├── reject_if_branch_in_use(branch, root)              → Error::BranchInUse
  │     ├── worktree_path = layout.worktree_dir(&branch)
  │     ├── if worktree_path.exists():                         → Error::WorktreeDirExists
  │     ├── run_git(["worktree","add","-b",branch,wt,base], root)
  │     │     - rollback boundary: from here on, failure runs cleanup_worktree(wt, branch)
  │     ├── scaffold_task_dir_in_worktree(wt, slug, title, tier, branch, base_branch)
  │     ├── for f in cfg.copy:
  │     │     ├── if !root.join(f).exists():                   → Error::WorktreeCopySourceMissing
  │     │     └── copy(root.join(f), wt.join(f))
  │     ├── for cmd in cfg.post_create:
  │     │     └── run_shell(cmd, cwd=wt)                       → Error::PostCreateHookFailed
  │     └── return TaskNewSummary (extended Display).
  └── else: existing flow unchanged
```

`cleanup_worktree(wt, branch)` runs `git worktree remove --force <wt>` and `git branch -D <branch>` best-effort. If rollback itself fails, the original error is returned with the rollback error chained via `anyhow::Context`.

**Call graph for `task worktree cleanup`:**

```
worktree::cleanup::worktree_cleanup(opts)
  ├── validate_slug(slug)
  ├── (slug, wt) = discovery::find_worktree_for_slug(slug, layout)
  │     → walks `git worktree list --porcelain`, reads each <wt>/.ark/tasks/.current
  │     → returns (slug, wt) pair, or None
  ├── if wt is None:                                           → Error::WorktreeNotFound (F-15)
  ├── toml = TaskToml::load(<wt>/.ark/tasks/<slug>)
  ├── if !opts.force:
  │     ├── status = run_git(["status","--porcelain"], wt)
  │     └── status.stdout non-empty                            → Error::WorktreeDirty
  ├── run_git(["worktree","remove", maybe_force, wt], root)
  ├── if opts.delete_branch:
  │     └── run_git(["branch", if force {"-D"} else {"-d"}, branch], root)
  ├── prune_empty_parents(wt.parent(), up_to: layout.worktrees_dir())
  └── return WorktreeCleanupSummary { slug, branch, branch_deleted, worktree_path: Some(wt), already_clean: false }
```

**Call graph for `task worktree list`:**

```
worktree::list::worktree_list(opts)
  ├── worktrees = parse_git_worktree_list(root)
  ├── rows = []
  ├── for wt in worktrees:
  │     ├── if !wt.is_under(layout.worktrees_dir()): skip
  │     ├── current = read_text(wt/.ark/tasks/.current).ok()?
  │     ├── toml = TaskToml::load(wt/.ark/tasks/current).ok()?
  │     └── rows.push(WorktreeRow { slug, branch, worktree_path, updated_at })
  ├── sort rows by toml.updated_at desc
  └── return WorktreeListSummary { rows }    // Display: "" when rows empty
```

[**Data Structure**]

```rust
// ark-core/src/commands/agent/state.rs (TaskToml additions)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToml {
    // ...existing fields unchanged...
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub worktree_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_branch: Option<String>,
}
```

```rust
// ark-core/src/commands/agent/task/worktree/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeConfig {
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,    // project-relative; abs path → error
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
```

```rust
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

pub fn worktree_cleanup(opts: WorktreeCleanupOptions) -> Result<WorktreeCleanupSummary>;
pub fn worktree_list(opts: WorktreeListOptions)       -> Result<WorktreeListSummary>;
```

```rust
// ark-core/src/commands/agent/task/new.rs (TaskNewOptions additions)

#[derive(Debug, Clone)]
pub struct TaskNewOptions {
    // ...existing fields unchanged...
    pub worktree: Option<TaskNewWorktree>,
}

#[derive(Debug, Clone)]
pub struct TaskNewWorktree {
    pub branch_type: Option<String>,    // None → cfg.branch_prefix
    pub branch_override: Option<String>, // wins over branch_type if Some
}

#[derive(Debug, Clone)]
pub struct TaskNewSummary {
    // existing fields...
    pub worktree: Option<TaskNewWorktreeSummary>,
}

#[derive(Debug, Clone)]
pub struct TaskNewWorktreeSummary {
    pub branch: String,
    pub worktree_path: PathBuf,
    pub base_branch: String,
}
```

```rust
// ark-core/src/error.rs (additions; renamed per R-005)

Error::WorktreeDirExists        { path: PathBuf },                // on-disk level
Error::WorktreeNotFound         { slug: String },
Error::WorktreeDirty            { path: PathBuf },
Error::BranchInUse              { branch: String, where_at: PathBuf },
Error::InvalidBranchName        { branch: String, reason: String },
Error::InvalidBranchType        { value: String },
Error::WorktreeConfigCorrupt    { path: PathBuf, source: toml::de::Error },
Error::PostCreateHookFailed     { command: String, exit_code: i32 },
Error::WorktreeCopySourceMissing{ path: PathBuf },
Error::TaskExistsOnParent       { slug: String, path: PathBuf },
Error::NestedWorktreeForbidden  { current_root: PathBuf },
Error::BaseBranchLacksArk       { base_branch: String },           // F-18 (optional probe)
Error::InvalidConfigField       { field: &'static str, reason: &'static str },
```

```rust
// ark-core/src/layout.rs (additions)

pub const WORKTREES_DIR: &str = ".ark/worktrees";
pub const WORKTREE_CONFIG_FILE: &str = ".ark/worktree.toml";

impl Layout {
    pub fn worktrees_dir(&self) -> PathBuf;
    pub fn worktree_dir(&self, branch: &str) -> PathBuf;
    pub fn worktree_config_file(&self) -> PathBuf;
}
```

```rust
// ark-core/src/io/fs.rs (extension)

/// Recursively enumerate files under `root`, pruning any subtree whose path
/// starts with one of the `skip_under` prefixes. The existing `walk_files`
/// is the zero-skip case (or becomes a thin wrapper).
pub fn walk_files_excluding(
    root: impl AsRef<Path>,
    skip_under: &[impl AsRef<Path>],
) -> Result<Vec<PathBuf>>;
```

[**API Surface**]

CLI shape (in `ark-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum TaskCommand {
    New(TaskNewCliArgs),
    Plan(TaskSlugArgs),
    Review(TaskSlugArgs),
    Execute(TaskSlugArgs),
    Verify(TaskSlugArgs),
    Archive(TaskSlugArgs),
    Promote(TaskPromoteCliArgs),
    Worktree(WorktreeArgs),       // only Cleanup + List
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

#[derive(clap::Args)]
struct WorktreeArgs {
    #[command(subcommand)] command: WorktreeCommand,
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
    #[arg(long)] slug: Option<String>,
    #[arg(long = "delete-branch")] delete_branch: bool,
    #[arg(long)] force: bool,
}

#[derive(clap::Args)]
struct WorktreeListCliArgs {
    #[command(flatten)] target: TargetArgs,
}
```

Library re-exports added: `WorktreeConfig`, the two option/summary pairs (`Cleanup*`, `List*`, `Row`), and `worktree_cleanup`, `worktree_list`, `walk_files_excluding`. `TaskNewOptions` extended.

[**Constraints**]

- **C-1:** `.ark/worktree.toml` is parsed with `toml::from_str` into `WorktreeConfig`. Missing file → `WorktreeConfig::default()`. Corrupt file → `Error::WorktreeConfigCorrupt { path, source }` with source error chained.
- **C-2 (worktree-first protocol):** When `task new --worktree` runs, the order of operations is fixed: validate inputs → check parent for collision → load config → resolve branch → check ref format → check base_branch → check branch-in-use → check worktree path → `git worktree add` → scaffold task dir *inside the worktree* → copy files → run post_create hooks → return summary. The parent's `.ark/tasks/`, `.ark/tasks/.current`, and working tree are **never modified**. Steps after `git worktree add` are inside a rollback boundary: any failure runs `git worktree remove --force <wt>` and `git branch -D <branch>` before returning the error.
- **C-3:** `task.toml`'s three new fields (`branch`, `worktree_path`, `base_branch`) are `Option<_>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Pre-existing `task.toml` files (in archived tasks or non-worktreed tasks) deserialize successfully. Regression test: `task_toml_loads_without_worktree_fields`.
- **C-4 (branch types):** `--branch-type` accepts only `{feat, fix, refactor, chore, ci, docs}`. Other values → `Error::InvalidBranchType { value }`. The list lives as `BRANCH_TYPES: &[&str; 6]` in `task/new.rs`. Adding a type later is a one-line edit; not a SPEC change.
- **C-5 (branch validation):** `--branch <full>` bypasses type validation but is passed through `git check-ref-format --branch <name>`. Non-zero → `Error::InvalidBranchName { branch, reason }`. Reuses existing `run_git` helper.
- **C-6 (branch precedence):** Final branch resolution: `--branch` if present, else `<--branch-type>/<slug>`, else `<cfg.branch_prefix>/<slug>`. The slug is appended verbatim — no auto-transformation.
- **C-7 (walk_files skip):** Both filesystem-walking callsites in `unload.rs` MUST NOT recurse into `<root>/<cfg.worktree_dir>/`: the primary `owned_dirs` snapshot capture loop AND the `capture_orphan_hook_entries` Stage B walk. Implementation: `walk_files_excluding(root, &[layout.worktrees_dir()])`. `walk_files_excluding` performs lexical `Path::starts_with` match against the caller-supplied prefixes; both `root` and `skip_under` entries MUST be absolute paths (callers in `unload.rs` get this for free via `Layout` helpers, which carry the project root). No symlink canonicalization is performed — a `cfg.worktree_dir` that resolves through a symlink is not supported. `upgrade::extract` does not call `walk_files` (it walks the embedded `ARK_TEMPLATES` tree only); no change required there. Regression test: `unload_excludes_worktree_contents`.
- **C-8 (worktree_path location):** `worktree_path` is always under `<root>/<cfg.worktree_dir>/<branch>/`, joined via `Layout::worktree_dir(&branch)`. `cfg.worktree_dir` is project-relative; absolute paths in config → `Error::InvalidConfigField { field: "worktree_dir", reason: "must be project-relative" }`.
- **C-9 (worktree.toml lifecycle):** `worktree.toml` is created by `ark init` from `templates/ark/worktree.toml`. `ark upgrade` does NOT overwrite it. The file lives under `.ark/`, so `ark unload`/`load` capture/restore it like other `.ark/` content (modulo the `worktrees_dir()` skip in C-7 — `worktree.toml` itself is NOT under `worktrees_dir()`). Regression test: `upgrade_does_not_overwrite_worktree_toml`.
- **C-10 (process spawn locality):** All git invocations route through `io::git::run_git`. `Command::new` may NOT appear under `commands/agent/task/worktree/` (extends the existing source-scan test).
- **C-11 (path/io discipline):** All filesystem access in `task/worktree/` and the `--worktree` path of `task/new.rs` routes through `io::PathExt`. All `.ark/`-relative paths route through `Layout` helpers.
- **C-12 (branch verbatim):** `task.toml.branch` stores the resolved branch verbatim (whether computed from `--branch-type` or supplied via `--branch`). No parsing into `branch_type` + `slug` parts. Downstream consumers (PR creation later) read the full ref.
- **C-13 (no nested worktree):** `task new --worktree` rejects when `cwd` resolves under any `*/.ark/worktrees/*/`. Detection: check `Layout::discover_from(cwd)?.root()` against the `*/.ark/worktrees/*/` pattern. Error: `Error::NestedWorktreeForbidden { current_root }`.
- **C-14 (list silent on zero rows):** `worktree list` prints `<row>\n` for each row; zero rows → empty stdout, exit 0. No stderr noise.
- **C-15 (cleanup parent pruning):** After `git worktree remove`, cleanup walks parent dirs upward (not crossing `worktrees_dir()`) and removes empty ones. Mirrors `Layout::prunable_empty_parents` style.
- **C-16 (.gitignore):** `templates/ark/.gitignore` (managed-block, marker `ARK`) gains `.ark/worktrees/` line at `init` and re-applied at `upgrade` like other managed content. The directory is created lazily by the first `task new --worktree`.
- **C-17 (output stability):** Every command writes a single `Display` summary; no ad-hoc stdout writes in command bodies. Mirrors `ark-agent-namespace` C-3.
- **C-18 (archive doesn't touch worktree):** `task archive` MUST NOT remove the worktree dir, delete the branch, or modify `task.toml.worktree_path`. Regression test: `archive_inside_worktree_leaves_worktree_intact`.
- **C-19 (`task new` without `--worktree` baseline):** When `--worktree` is not passed, `task_new` writes nothing under `<root>/<cfg.worktree_dir>/`. Regression test: `task_new_without_worktree_makes_no_worktree_changes`.
- **C-20 (cleanup discovery):** `worktree cleanup` finds the worktree by parsing `git worktree list --porcelain` and reading each candidate's `.ark/tasks/.current`. If no worktree's `.current` matches the slug, `Error::WorktreeNotFound { slug }`. Worktrees whose `.current` or `task.toml` is missing/unreadable are silently skipped — this is the documented behavior for non-Ark third-party worktrees living under `worktrees_dir()` (R-108 disambiguation).
- **C-21 (path representation):** `task.toml.worktree_path` stores the worktree path as **project-relative** (e.g. `.ark/worktrees/feat/foo`), normalized to forward-slash separators on disk. Consumers needing an absolute path resolve it against `layout.root()`. This keeps `.ark.db` snapshots portable across machine moves and repo relocations. `WorktreeRow.worktree_path` and `worktree list` output are computed by joining `layout.root()` with the relative value at format time, but stored as relative in `task.toml`. Regression test: `task_new_worktree_stores_project_relative_path`.
