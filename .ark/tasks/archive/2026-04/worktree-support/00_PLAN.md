# `worktree-support` PLAN `00`

> Status: Draft
> Feature: `worktree-support`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Adds an opt-in `ark agent task worktree {create|cleanup|list}` subcommand group that binds an Ark task to a dedicated git worktree under `.ark/worktrees/<branch>/`, enabling multiple `/ark:design` runs to proceed in parallel without colliding on `.ark/tasks/.current`. Configuration lives in a new tracked file `.ark/worktree.toml` (singular — plural `workspace.toml` reserved for future per-developer journals). `task.toml` gains optional `branch` and `worktree_path` fields. Archive does not auto-cleanup; cleanup is an explicit verb. `ark context` already finds the right project from inside a worktree via `Layout::discover_from`; this task adds a regression test, no API change.

## Log `None in 00_PLAN`

---

## Spec

[**Goals**]

- **G-1:** New project-tracked file `.ark/worktree.toml` (TOML, parsed via the `toml` crate) carrying three keys, all optional with defaults: `worktree_dir = ".ark/worktrees"`, `copy = []`, `post_create = []`. Missing file is not an error — defaults apply. The file is created with documented defaults by `ark init` and snapshot-tracked like other `.ark/` content. Singular name (`worktree.toml`, not `workspace.toml`) reserves `workspace` for the per-developer journal feature.
- **G-2:** New hidden subcommand group under `ark agent task`: `task worktree {create|cleanup|list}`. The group is hidden from `ark --help` (inherited from `Agent`'s `hide = true`). Each subcommand prints a one-line `Display` summary in the `ark agent` style.
- **G-3:** `ark agent task worktree create [--slug <s>] [--branch-type <type>] [--branch <full>]` creates a git worktree at `<root>/.ark/worktrees/<branch>/`. The branch is `<type>/<slug>` where `<type>` defaults to `feat` (override per-invocation via `--branch-type` ∈ {`feat`,`fix`,`refactor`,`chore`,`ci`,`docs`}; project-wide default via `worktree.toml`'s `branch_prefix` key). `--branch <full>` overrides both, accepting any user-supplied ref name (validated via `git check-ref-format`). The base branch (current `HEAD`'s symbolic ref at create time) is recorded as `task.toml.base_branch`. `task.toml.branch` and `task.toml.worktree_path` are populated. After `git worktree add`, files listed under `worktree.toml.copy` are copied (path-by-path) and commands listed under `worktree.toml.post_create` are run sequentially in the worktree dir, aborting on first non-zero exit.
- **G-4:** `ark agent task worktree cleanup [--slug <s>] [--delete-branch] [--force]` removes the worktree dir via `git worktree remove`, optionally deletes the branch (`git branch -d` or `-D` with `--force`). If the worktree has uncommitted changes, the command refuses without `--force`. Idempotent: cleanup of a slug whose `worktree_path` is already absent succeeds with a no-op summary. After cleanup, `task.toml.worktree_path` is cleared (`branch` is preserved as historical metadata).
- **G-5:** `ark agent task worktree list` prints one line per active worktree-backed task in the form `<slug> <branch> <worktree_path>`, sorted by `task.toml.updated_at` descending. Tasks without `worktree_path` are skipped. Pre-archive only — archived tasks are not enumerated.
- **G-6:** `task.toml` gains four optional fields: `branch: Option<String>`, `worktree_path: Option<PathBuf>`, `base_branch: Option<String>`, all `#[serde(default, skip_serializing_if = "Option::is_none")]`. Pre-existing task.toml files (without these fields) load unchanged. `branch_prefix` lives in `worktree.toml`, not `task.toml`.
- **G-7:** `task new` does **NOT** auto-create a worktree. Users opt in either by running `task worktree create` after `task new`, or via the convenience flag `task new --worktree [--branch-type <t>] [--branch <full>]` which performs both atomically (rolling back the task dir if worktree creation fails). This decision is locked: `--worktree` is opt-in for every tier including deep.
- **G-8:** `task archive` does **NOT** remove the worktree. Archive moves the task dir to `tasks/archive/YYYY-MM/<slug>/` exactly as before; a worktree-bound task's branch and checkout survive intact. Cleanup is the user's explicit step. (Cross-worktree archive race is mooted because the archive lives on the worktree's branch — merging brings the archived dir into main.)
- **G-9:** `Layout::discover_from(cwd)` already walks ancestors looking for `.ark/`. From inside a worktree, the worktree's `.ark/` is the project root — found at the worktree dir itself, not the upstream checkout. `ark context` therefore resolves the worktree's `.current` automatically. This task adds a regression test (`gather_context_from_inside_worktree`) but introduces no API change to `ark context`.
- **G-10:** New entries in `.gitignore` (managed by `ark init`/`upgrade`): `.ark/worktrees/` (the worktree storage dir is tracked-by-convention but the *contents* are not — each worktree is itself a separate git working tree and must not be indexed by the parent). The line is appended idempotently via the existing `update_managed_block` helper.
- **G-11:** Docs updated: `.ark/workflow.md` gains a new `### Worktree (optional)` subsection under §6 Mechanics describing the three commands and the opt-in model. `templates/ark/workflow.md` is updated in lockstep. The shipped slash commands (`/ark:design`, `/ark:quick`) are **not** changed — they remain worktree-agnostic. Users opt in by running `ark agent task worktree create` themselves.

[**Non-goals**]

- **NG-1:** No per-developer workspace dir, no journals, no developer identity. Reserved for a follow-up `workspace-support` task.
- **NG-2:** No agent process supervision. `task worktree create` does not spawn `claude`/`codex`/etc; the user `cd`s in and runs their own agent.
- **NG-3:** No registry of running PIDs.
- **NG-4:** No automatic worktree creation. `--worktree` is opt-in for all tiers.
- **NG-5:** No automatic cleanup on archive.
- **NG-6:** No worktree rename or move post-creation. Re-create with a new branch if needed.
- **NG-7:** No monorepo / submodule init in worktrees. Trellis does this; we don't have monorepos in scope.
- **NG-8:** No structured-output JSON for `task worktree list`. One-line text per row, like other `agent` commands.
- **NG-9:** No PR-creation integration (`gh pr create`). Out of scope; the user runs `gh` themselves.
- **NG-10:** No cross-worktree task synchronization. Each worktree's `.ark/` is the branch's view; conflicts resolve at merge time like any other branch divergence.
- **NG-11:** No update to `/ark:design` / `/ark:quick` slash commands. They stay worktree-agnostic; worktree creation is a user-driven opt-in.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                       — adds `Worktree(WorktreeArgs)` under TaskCommand
└── ark-core/src/
    ├── lib.rs                                 — re-exports public worktree API
    ├── error.rs                               — new variants (see Data Structure)
    ├── layout.rs                              — adds worktrees_dir, worktree_dir(branch),
    │                                            worktree_config_file
    ├── io/
    │   └── git.rs                             — unchanged (existing run_git suffices)
    └── commands/
        └── agent/
            ├── mod.rs                         — pub use task::worktree::* (public types only)
            ├── state.rs                       — TaskToml: + branch, + worktree_path,
            │                                    + base_branch (all Option, #[serde(default)])
            ├── task/
            │   ├── mod.rs                     — pub mod worktree;
            │   │                                pub use worktree::*
            │   ├── new.rs                     — adds --worktree atomic path
            │   │                                (calls worktree::create on success)
            │   └── worktree/                  — NEW
            │       ├── mod.rs                 — public types + dispatch
            │       ├── config.rs              — WorktreeConfig (worktree.toml model)
            │       ├── create.rs              — worktree_create
            │       ├── cleanup.rs             — worktree_cleanup
            │       └── list.rs                — worktree_list
templates/
├── ark/
│   ├── worktree.toml                          — NEW: shipped default config
│   ├── workflow.md                            — adds §6 Worktree subsection
│   └── .gitignore                             — adds `.ark/worktrees/` line
└── (claude/codex/opencode unchanged)
```

**Module coupling.** `task::worktree::{create, cleanup, list} → config → state`. `worktree` is a sibling of `new`/`phase`/`promote`/`archive`; `task::new` imports `worktree::create` only for the `--worktree` convenience flag (one-way). The config module is leaf-level (no imports from siblings).

**Call graph for `ark agent task worktree create`:**

```
worktree::create::worktree_create(opts)
  ├── validate_slug(slug)
  ├── TaskToml::load(task_dir)                       → toml
  ├── if toml.worktree_path.is_some():               → Error::WorktreeAlreadyExists
  ├── WorktreeConfig::load_or_default(layout)        → cfg
  ├── resolve_branch(opts, &cfg, &toml)              → branch (e.g. "feat/foo")
  ├── git_check_ref_format(branch)                   → reject malformed branch
  ├── git_branch_in_use(branch, repo_root)           → Error::BranchInUse if checked out elsewhere
  ├── base_branch = run_git(["symbolic-ref","--short","HEAD"], root).stdout
  ├── worktree_path = layout.worktree_dir(&branch)   → .ark/worktrees/<branch>/
  ├── if worktree_path.exists():                     → Error::WorktreeExists
  ├── run_git(["worktree","add","-b",branch,worktree_path,base_branch], root)
  │     - on failure: surface stderr; do NOT update task.toml
  ├── for f in cfg.copy: copy_file(root.join(f), worktree_path.join(f))
  ├── for cmd in cfg.post_create: run_shell(cmd, cwd=worktree_path)
  │     - on first non-zero: roll back (git worktree remove --force) and error
  ├── toml.branch = Some(branch); toml.worktree_path = Some(worktree_path);
  │   toml.base_branch = Some(base_branch); toml.updated_at = now; toml.save(task_dir)
  └── return WorktreeCreateSummary { slug, branch, worktree_path, base_branch }
```

**Call graph for `ark agent task worktree cleanup`:**

```
worktree::cleanup::worktree_cleanup(opts)
  ├── validate_slug(slug)
  ├── TaskToml::load(task_dir)                       → toml
  ├── if toml.worktree_path.is_none():               → return no-op summary
  ├── if !opts.force:
  │     ├── run_git(["status","--porcelain"], wt)
  │     └── non-empty stdout                         → Error::WorktreeDirty
  ├── run_git(["worktree","remove",wt_path,…], root) → with --force iff opts.force
  ├── if opts.delete_branch:
  │     └── run_git(["branch", if force {"-D"} else {"-d"}, branch], root)
  ├── toml.worktree_path = None; toml.updated_at = now; toml.save(task_dir)
  └── return WorktreeCleanupSummary { slug, branch_deleted, worktree_path }
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
    /// Project-relative path; default ".ark/worktrees".
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,
    /// Project-relative paths to copy into each new worktree (e.g. `.env`).
    #[serde(default)]
    pub copy: Vec<String>,
    /// Shell commands run in the worktree dir after `git worktree add`.
    #[serde(default)]
    pub post_create: Vec<String>,
    /// Branch prefix when --branch-type / --branch absent. Default "feat".
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
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
pub struct WorktreeCreateOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub branch_type: Option<String>,    // e.g. "fix"; None → cfg.branch_prefix
    pub branch_override: Option<String>, // full branch name; wins over branch_type
}

#[derive(Debug, Clone)]
pub struct WorktreeCreateSummary {
    pub slug: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub base_branch: String,
}

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
pub struct WorktreeListSummary {
    pub rows: Vec<WorktreeRow>,
}

#[derive(Debug, Clone)]
pub struct WorktreeRow {
    pub slug: String,
    pub branch: String,
    pub worktree_path: PathBuf,
}

// All four summaries impl Display; Display body is the one-liner.

pub fn worktree_create(opts: WorktreeCreateOptions)   -> Result<WorktreeCreateSummary>;
pub fn worktree_cleanup(opts: WorktreeCleanupOptions) -> Result<WorktreeCleanupSummary>;
pub fn worktree_list(opts: WorktreeListOptions)       -> Result<WorktreeListSummary>;
```

```rust
// ark-core/src/error.rs (additions)

Error::WorktreeAlreadyExists { slug: String, path: PathBuf },
Error::WorktreeExists        { path: PathBuf },          // dir on disk before our git call
Error::WorktreeNotFound      { slug: String },
Error::WorktreeDirty         { path: PathBuf },
Error::BranchInUse           { branch: String, where_at: PathBuf },
Error::InvalidBranchName     { branch: String, reason: String },
Error::InvalidBranchType     { value: String },          // not in feat/fix/refactor/chore/ci/docs
Error::WorktreeConfigCorrupt { path: PathBuf, source: toml::de::Error },
Error::PostCreateHookFailed  { command: String, exit_code: i32 },
```

```rust
// ark-core/src/layout.rs (additions)

pub const WORKTREES_DIR: &str = ".ark/worktrees";
pub const WORKTREE_CONFIG_FILE: &str = ".ark/worktree.toml";

impl Layout {
    pub fn worktrees_dir(&self) -> PathBuf;            // .ark/worktrees/
    pub fn worktree_dir(&self, branch: &str) -> PathBuf; // .ark/worktrees/<branch>/
    pub fn worktree_config_file(&self) -> PathBuf;    // .ark/worktree.toml
}
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
    Worktree(WorktreeArgs),    // NEW
}

#[derive(clap::Args)]
struct WorktreeArgs {
    #[command(subcommand)]
    command: WorktreeCommand,
}

#[derive(Subcommand)]
enum WorktreeCommand {
    /// Create a git worktree bound to the task.
    Create(WorktreeCreateCliArgs),
    /// Remove the worktree dir; optionally delete the branch.
    Cleanup(WorktreeCleanupCliArgs),
    /// List active worktree-backed tasks.
    List(WorktreeListCliArgs),
}

#[derive(clap::Args)]
struct WorktreeCreateCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(long)] slug: Option<String>,
    #[arg(long = "branch-type")] branch_type: Option<String>,  // feat/fix/refactor/...
    #[arg(long = "branch")] branch: Option<String>,            // full override
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

`TaskNewCliArgs` gains optional `--worktree`, `--branch-type`, `--branch`. Library re-exports added: `WorktreeConfig`, the three option/summary pairs above, and the three `worktree_*` functions. `TaskNewOptions` gains a single `worktree: Option<TaskNewWorktree>` field; absent → existing behavior.

```rust
pub struct TaskNewWorktree {
    pub branch_type: Option<String>,
    pub branch_override: Option<String>,
}
```

[**Constraints**]

- **C-1:** `.ark/worktree.toml` is parsed with `toml::from_str` into `WorktreeConfig`. Missing file → `WorktreeConfig::default()`. Corrupt file → `Error::WorktreeConfigCorrupt { path, source }`. Source error chained for diagnostics. The shipped `templates/ark/worktree.toml` carries documented defaults (`worktree_dir = ".ark/worktrees"`, `branch_prefix = "feat"`, empty `copy` and `post_create`).
- **C-2:** `task.toml`'s three new fields (`branch`, `worktree_path`, `base_branch`) are `Option<_>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Pre-existing `task.toml` files in archived tasks must continue to deserialize (regression test required: `archived_task_toml_loads_without_worktree_fields`).
- **C-3:** `--branch-type` accepts only `{feat, fix, refactor, chore, ci, docs}`. Other values → `Error::InvalidBranchType`. The list lives as a constant `BRANCH_TYPES: &[&str]` in `task/worktree/create.rs`. Adding a type later is a one-line edit; not a SPEC change.
- **C-4:** `--branch <full>` bypasses type validation but is still passed through `git check-ref-format --branch`. Non-zero exit → `Error::InvalidBranchName { branch, reason }`. This is the existing `run_git` helper; no new git helper.
- **C-5:** Branch resolution precedence (highest to lowest): `--branch` > `--branch-type/<slug>` > `<cfg.branch_prefix>/<slug>`. The slug is appended verbatim — no auto-transformation (lowercasing, replacing `_` with `-`, etc.).
- **C-6:** `worktree_path` is always under `<root>/<cfg.worktree_dir>/<branch>/`, joined via `Layout::worktree_dir(&branch)`. `cfg.worktree_dir` is project-relative; absolute paths are rejected with `Error::InvalidConfigField { field: "worktree_dir", reason: "must be project-relative" }`.
- **C-7:** All git invocations route through `io::git::run_git`; no bare `Command::new`. Reuses the existing process-spawn locality enforcement test (`commands_no_bare_command_new`), extended to the new `worktree/` module path.
- **C-8:** `worktree create`'s rollback policy is best-effort: if `git worktree add` succeeds but a `copy` or `post_create` step fails, we run `git worktree remove --force` and return the underlying error. If rollback itself fails, we surface the rollback error chained under the original (via `anyhow::Context`).
- **C-9:** `worktree create` refuses to operate on a task whose `task.toml.phase == Archived`. Returns `Error::IllegalPhaseTransition { tier, from: Archived, to: <existing> }` — keeps the error variant set tight rather than introducing a new "archived task is read-only" error.
- **C-10:** `worktree create` is idempotent only in the sense of "succeeds once, fails on retry": a second invocation when `task.toml.worktree_path` is already populated returns `Error::WorktreeAlreadyExists`. Run `cleanup` first to retry. (This avoids ambiguity over whether to recreate, which would silently destroy a user's branch.)
- **C-11:** `worktree cleanup` with `task.toml.worktree_path == None` returns `Ok(WorktreeCleanupSummary { already_clean: true, .. })` instead of erroring. This makes scripted cleanup safe (e.g. running cleanup on every task in a sweep).
- **C-12:** `worktree cleanup` without `--force` runs `git status --porcelain` in the worktree dir; non-empty stdout → `Error::WorktreeDirty`. With `--force`, it skips the dirty check and uses `git worktree remove --force`.
- **C-13:** `worktree list` reads every directory under `.ark/tasks/` (skipping `archive` and `.current`), loads each `task.toml`, and filters to those with `worktree_path.is_some()`. Sorted by `updated_at` descending. One line per row.
- **C-14:** `task new --worktree` is atomic: if `worktree_create` fails, the task dir created by `task_new` is removed before returning the error. (Otherwise users would be left with a half-initialized task.)
- **C-15:** `Layout::discover_from(cwd)` already finds the worktree's `.ark/` correctly (it's the worktree dir itself, since the worktree is a full git checkout of the branch). No change needed; only a regression test that `gather_context` from inside a worktree returns the worktree's `.current`, not the upstream.
- **C-16:** `.gitignore` managed-block (existing, marker `ARK`) gains `.ark/worktrees/` entry, written at `init` and re-applied at `upgrade` like other managed content. The directory itself is created lazily by `worktree create`; nothing in `worktree.toml` references its presence.
- **C-17:** Output stability — every new command writes exactly one line via the summary's `Display` impl. No stdout writes in command bodies. Mirrors C-3 from `ark-agent-namespace`.
- **C-18:** All filesystem access in `task/worktree/` routes through `io::PathExt`; no bare `std::fs::*`. Mirrors `ark-agent-namespace` C-4.
- **C-19:** All `.ark/`-relative path composition routes through `Layout` helpers; no string concatenation. Mirrors `ark-agent-namespace` C-5.

---

## Runtime

[**Main Flow**]

1. User runs `ark agent task new --slug foo --tier deep` (existing path; no change).
2. User runs `ark agent task worktree create` from the project root (or `cd` into another worktree first).
3. CLI dispatches to `worktree_create`; loads `task.toml`, loads `worktree.toml`, resolves branch.
4. `git worktree add -b <branch> <path> <base_branch>` succeeds; `<path>` is `.ark/worktrees/<branch>/`.
5. `cfg.copy` files copied; `cfg.post_create` commands run sequentially.
6. `task.toml` updated with `branch`, `worktree_path`, `base_branch`. Saved.
7. CLI prints one-line summary; user `cd`s into `<path>` and runs `claude` / `codex` / etc.
8. Inside the worktree, `ark context` finds the local `.ark/` via `Layout::discover_from`; resolves `.current` to the task slug; the agent works on the bound branch.
9. After execution + verification + archive (existing flow inside the worktree), user runs `git push`, opens a PR, merges, then runs `ark agent task worktree cleanup --slug foo --delete-branch` to remove the worktree dir and prune the branch.

[**Failure Flow**]

1. Branch already checked out elsewhere → `Error::BranchInUse` (detected via `git worktree list --porcelain`). User can run `cleanup` on the prior worktree first.
2. Branch name fails `git check-ref-format --branch` → `Error::InvalidBranchName`.
3. Worktree dir already exists on disk → `Error::WorktreeExists`. (Distinct from `WorktreeAlreadyExists`, which is the task.toml level guard.)
4. `git worktree add` fails for any other reason (e.g. bare repo, ref ambiguity) → return error with stderr captured; `task.toml` untouched.
5. `cfg.copy` source missing → log warning, skip (do not abort). Source-not-found is a config error, not a runtime crash.
6. `cfg.post_create` command exits non-zero → `Error::PostCreateHookFailed { command, exit_code }`. Rollback per C-8.
7. `worktree.toml` parse error → `Error::WorktreeConfigCorrupt`.
8. Cleanup with dirty worktree, no `--force` → `Error::WorktreeDirty`. User commits/stashes or passes `--force`.
9. Cleanup with `--delete-branch` but branch is unmerged → `git branch -d` fails → return git's error verbatim. User passes `--force` to escalate to `-D`.

[**State Transitions**]

- `task.toml.worktree_path: None` → `Some(path)` on `worktree create` success.
- `task.toml.worktree_path: Some(path)` → `None` on `worktree cleanup` success (regardless of `--delete-branch`).
- `task.toml.branch: None` → `Some(name)` on `worktree create`; preserved across `cleanup` (branch may still exist on disk if `--delete-branch` not passed).
- `task.toml.base_branch: None` → `Some(name)` on `worktree create`; never cleared (historical metadata for PR targeting even after cleanup).
- Phase machine unaffected. Worktree creation/cleanup is orthogonal to `Design → Plan → … → Archived`.

---

## Implementation

[**Phase 1 — Layout, config, state extensions**]

1. Add `WORKTREES_DIR` and `WORKTREE_CONFIG_FILE` constants to `layout.rs`; add `worktrees_dir`, `worktree_dir(branch)`, `worktree_config_file` methods.
2. Extend `TaskToml` with three optional fields. Update `state.rs` tests: add `task_toml_loads_without_worktree_fields` to assert backward compat.
3. Add error variants to `error.rs` with `Display` impls matching existing style.
4. Create `commands/agent/task/worktree/` directory with `mod.rs` (option/summary types), `config.rs` (WorktreeConfig + load_or_default).
5. Wire re-exports in `lib.rs` and `commands/agent/mod.rs`.

Validation gate for Phase 1: `cargo test -p ark-core` passes; new types compile and serialize round-trip; backward-compat test green.

[**Phase 2 — Three subcommands**]

1. Implement `worktree::create::worktree_create` end-to-end. Unit tests via temp git repo (initialize, run, assert worktree dir exists, assert task.toml updated).
2. Implement `worktree::cleanup::worktree_cleanup`. Unit tests for: clean cleanup, dirty without `--force` (rejected), dirty with `--force` (succeeds), already-clean idempotence, `--delete-branch` happy + unmerged.
3. Implement `worktree::list::worktree_list`. Unit tests for: empty list, mixed worktreed/non-worktreed tasks, sort order.
4. Wire CLI subcommands in `ark-cli/src/main.rs`. Snapshot test: `ark agent task --help` includes `worktree` and `ark agent task worktree --help` includes the three subcommands.

Validation gate for Phase 2: `cargo test -p ark-core -p ark-cli` passes.

[**Phase 3 — Integration with `task new`, gitignore, docs, regression test**]

1. Add `--worktree`, `--branch-type`, `--branch` to `TaskNewCliArgs` and `TaskNewOptions`. Wire atomic rollback (delete task dir on worktree creation failure).
2. Update `templates/ark/.gitignore` (or create the managed `.gitignore` block if it doesn't exist) to include `.ark/worktrees/`. Update `init` to apply on fresh installs and `upgrade` to re-apply.
3. Ship `templates/ark/worktree.toml` with documented defaults; ensure `init` writes it and `upgrade` refreshes it like other ark templates.
4. Update `templates/ark/workflow.md` with a `### Worktree (optional)` block under §6 Mechanics; mirror to live `.ark/workflow.md`.
5. Add the `gather_context_from_inside_worktree` integration test to `commands/context/`: scaffold an Ark project, `git worktree add` a branch, set `.current` inside the worktree, assert `gather_context` returns the worktree-side slug.

Validation gate for Phase 3: `cargo test -p ark-core -p ark-cli`; manual end-to-end (create task with `--worktree`, cd in, run `ark context`, observe correct slug, cleanup, verify dir gone).

---

## Trade-offs

- **T-1: Worktree storage location.** Inside the repo (`.ark/worktrees/`) vs outside (`../ark-worktrees/`).
  - *Inside.* Adv.: ark-state grouped together; `ark unload` already reasons about its own dirs; one `.gitignore` line covers it; user's repo dir stays uncluttered. Disadv.: parent repo will see worktree dirs as ignored (need gitignore line); some IDEs index worktree contents twice.
  - *Outside.* Adv.: Trellis convention; zero risk of accidental indexing; conventional. Disadv.: leaks ark state to the repo's parent dir; harder to clean up if user moves the repo.
  - **Decision: inside (per user direction).** Cleaner mental model for ark-owned state.

- **T-2: `--worktree` opt-in vs opt-out by tier.**
  - *Always opt-in.* Adv.: predictable; `/ark:quick` stays cheap; user controls the disk cost. Disadv.: deep-tier tasks have to remember the flag.
  - *Auto-on for deep.* Adv.: matches the use case (deep = long-running, parallel-friendly). Disadv.: surprising for solo users; doubles disk silently.
  - **Decision: always opt-in (per user direction).**

- **T-3: Cleanup on archive.**
  - *Auto-cleanup.* Adv.: tidier; one less command to run. Disadv.: archive happens *before* merge typically; deleting checkouts pre-merge loses uncommitted work.
  - *Never auto.* Adv.: safe; explicit; matches user mental model ("archive is metadata, cleanup is filesystem"). Disadv.: requires running two commands.
  - **Decision: never auto (per user direction).**

- **T-4: Per-task vs project-wide config.**
  - *Per-task in `task.toml`.* Each task carries its own `copy` / `post_create`. Adv.: flexibility. Disadv.: redundant; hides the project convention.
  - *Project-wide in `worktree.toml`.* All worktrees inherit the same config. Adv.: one source of truth. Disadv.: one task can't override.
  - **Decision: project-wide.** Per-task overrides can ship later if needed.

- **T-5: Branch ref-format validation strategy.**
  - *Reject anything not in `{feat,fix,...}/<slug>`.* Adv.: enforces convention. Disadv.: brittle when conventions change; some users want `<initials>/<branch>`.
  - *Allow `--branch <full>` with `git check-ref-format` validation.* Adv.: convention-by-default, escape hatch when needed. Disadv.: more code paths to test.
  - **Decision: latter.** `--branch-type` is the curated path; `--branch` is the escape hatch.

- **T-6: `worktree.toml` as managed file vs user-editable.**
  - *Managed (hash-tracked, refresh on upgrade).* Adv.: keeps shipped defaults current. Disadv.: user customizations get clobbered.
  - *User-editable (created at init, never overwritten).* Adv.: user customizations survive upgrade. Disadv.: shipped defaults can drift.
  - **Decision: user-editable.** Mirror `.ark/workflow.md`'s treatment if it's user-editable, otherwise document explicitly. (Open: see Q-1.)

---

## Validation

[**Unit Tests**]

- **V-UT-1:** `WorktreeConfig::load_or_default` returns defaults when file missing.
- **V-UT-2:** `WorktreeConfig::load_or_default` errors `WorktreeConfigCorrupt` on invalid TOML.
- **V-UT-3:** `TaskToml` round-trips with all three new fields populated and with all three absent.
- **V-UT-4:** `archived_task_toml_loads_without_worktree_fields` — pre-existing TOML serialized without the new fields deserializes successfully.
- **V-UT-5:** `resolve_branch` precedence: `--branch` > `--branch-type/<slug>` > `<cfg.branch_prefix>/<slug>`.
- **V-UT-6:** `--branch-type` rejects values outside `{feat,fix,refactor,chore,ci,docs}` with `InvalidBranchType`.
- **V-UT-7:** `Layout::worktree_dir("feat/foo")` returns `<root>/.ark/worktrees/feat/foo/`.

[**Integration Tests**]

- **V-IT-1:** End-to-end create: scaffold a temp git repo with `ark init`, `task new --slug foo --tier deep`, `task worktree create`. Assert worktree dir exists, branch `feat/foo` exists, `task.toml` updated.
- **V-IT-2:** End-to-end cleanup: V-IT-1 then `worktree cleanup --slug foo --delete-branch`. Assert worktree dir gone, branch gone, `task.toml.worktree_path == None`, `task.toml.branch` preserved.
- **V-IT-3:** `task new --worktree` atomic happy path: assert task dir + worktree dir + task.toml all consistent.
- **V-IT-4:** `task new --worktree` rollback: simulate `worktree_create` failure (e.g. branch already exists elsewhere); assert task dir is removed.
- **V-IT-5:** `worktree list` round-trip: create two worktreed tasks + one non-worktreed; list returns exactly two rows in updated_at-desc order.
- **V-IT-6:** `gather_context_from_inside_worktree`: create a task in a worktree, set `.current` inside the worktree, run `gather_context` with `cwd = worktree_path`, assert returned slug matches.
- **V-IT-7:** `worktree.toml` `copy` propagation: create a worktree with `.env` listed in `copy`; assert `.env` exists in the worktree.
- **V-IT-8:** `worktree.toml` `post_create` execution: configure `post_create = ["touch hello.txt"]`; assert `hello.txt` exists in worktree.

[**Failure / Robustness Validation**]

- **V-F-1:** `worktree create` when `task.toml.worktree_path` already set → `WorktreeAlreadyExists`; task.toml unchanged.
- **V-F-2:** `worktree create` when target dir exists on disk → `WorktreeExists`.
- **V-F-3:** `worktree create` when branch in use elsewhere → `BranchInUse`.
- **V-F-4:** `worktree create` with malformed `--branch` → `InvalidBranchName`.
- **V-F-5:** `worktree create` with corrupt `worktree.toml` → `WorktreeConfigCorrupt`; no side effects.
- **V-F-6:** `worktree create` `post_create` failure: rollback removes the worktree; task.toml unchanged.
- **V-F-7:** `worktree cleanup` on dirty worktree without `--force` → `WorktreeDirty`; nothing changes.
- **V-F-8:** `worktree cleanup` already clean (worktree_path None) → `already_clean: true` summary, no error.
- **V-F-9:** `worktree create` on archived task → `IllegalPhaseTransition`.

[**Edge Case Validation**]

- **V-E-1:** Slug with hyphens, e.g. `worktree-support` → branch `feat/worktree-support`. No transformation.
- **V-E-2:** `worktree.toml` with `worktree_dir = "/abs/path"` (absolute) → `InvalidConfigField`.
- **V-E-3:** `cfg.copy` references a missing file → warning logged, file skipped (per Failure Flow #5), creation continues.
- **V-E-4:** `task new --worktree --branch feat/already-exists` (branch present on disk) → atomic rollback removes the freshly-created task dir.
- **V-E-5:** Two parallel `worktree create` invocations on different slugs in different shells: both succeed (worktrees are independent).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (worktree.toml) | V-UT-1, V-UT-2, V-IT-7, V-IT-8 |
| G-2 (subcommand group) | V-IT-1, V-IT-2, V-IT-5 (also CLI snapshot in Phase 2) |
| G-3 (create) | V-IT-1, V-IT-3, V-IT-7, V-IT-8, V-UT-5, V-UT-6 |
| G-4 (cleanup) | V-IT-2, V-F-7, V-F-8 |
| G-5 (list) | V-IT-5 |
| G-6 (task.toml fields) | V-UT-3, V-UT-4 |
| G-7 (--worktree opt-in) | V-IT-3, V-IT-4, V-E-4 |
| G-8 (no auto-cleanup) | V-IT-1 followed by `task archive` test (asserts worktree intact) |
| G-9 (context worktree-aware) | V-IT-6 |
| G-10 (.gitignore) | V-IT-1 (asserts `.gitignore` contains `.ark/worktrees/`) |
| G-11 (docs lockstep) | Manual review during Phase 3 |
| C-1 (config parsing) | V-UT-1, V-UT-2 |
| C-2 (backward compat) | V-UT-4 |
| C-3 (branch types) | V-UT-6 |
| C-4 (branch validation) | V-F-4 |
| C-5 (precedence) | V-UT-5 |
| C-6 (worktree_dir relative) | V-E-2 |
| C-7 (no bare Command::new) | Existing source-scan test extended |
| C-8 (rollback on failure) | V-F-6 |
| C-9 (archived rejection) | V-F-9 |
| C-10 (idempotence semantics) | V-F-1 |
| C-11 (cleanup idempotence) | V-F-8 |
| C-12 (dirty check) | V-F-7 |
| C-13 (list filtering/sort) | V-IT-5 |
| C-14 (task new atomic) | V-IT-4, V-E-4 |
| C-15 (discover_from worktree) | V-IT-6 |
| C-16 (.gitignore managed) | V-IT-1 |
| C-17 (output stability) | CLI snapshot tests |
| C-18 (PathExt only) | Source-scan or visual review during code-review |
| C-19 (Layout helpers) | Source-scan or visual review during code-review |

---

## Open Questions for the Reviewer

- **Q-1:** Should `worktree.toml` be hash-tracked (refresh on `ark upgrade`) or treated as user-editable like `workflow.md`? Trade-off T-6. Lean: user-editable, ship `templates/ark/worktree.toml` only as the init seed.
- **Q-2:** When `--branch <full>` is used, should we still record `task.toml.branch` exactly as given (including any prefix the user wrote), or extract the type and slug separately? Lean: record verbatim; we're not parsing user-supplied refs.
- **Q-3:** Should `worktree list` print anything when zero rows match, or stay silent? Lean: print `"no worktree-backed tasks"` to stderr-but-still-exit-0 — distinguishes "no matches" from "broken pipe".
