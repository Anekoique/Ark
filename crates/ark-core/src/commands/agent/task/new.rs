//! `ark agent task new` — scaffold a new task directory.
//!
//! Creates `.ark/tasks/<slug>/`, seeds `PRD.md` from the embedded template,
//! writes `task.toml` with phase=Design + iteration=0, then registers the
//! slug in `.ark/.state.toml` as both an active task and the focus of this
//! checkout. Refuses to overwrite an existing task directory.
//!
//! When `opts.worktree.is_some()`, creation uses the worktree-first protocol.
//!
//! Ark creates a git worktree under `.ark/worktrees/` first, then scaffolds the
//! task dir *inside the worktree*. The parent checkout's state file is never
//! modified by this path.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use chrono::Utc;

use crate::{
    commands::agent::{
        state::{Phase, TaskToml, Tier, validate_slug, validate_title},
        task::worktree::{
            WorktreeConfig,
            discovery::{find_worktree_for_slug, parse_git_worktree_list},
            resolve_branch,
        },
        template::copy_template,
        workspace::{
            ResolveOptions as IdentityResolveOptions, identity::identity_prompt, identity_resolve,
            identity_write,
        },
    },
    error::{Error, Result},
    io::{
        PathExt,
        git::{run_git, run_shell},
    },
    layout::Layout,
    state::state_mutate,
};

/// Options for creating a new Ark task.
#[derive(Debug, Clone)]
pub struct TaskNewOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// New task slug.
    pub slug: String,
    /// Human-readable task title.
    pub title: String,
    /// Workflow tier for the task.
    pub tier: Tier,
    /// When `Some`, run the worktree-first protocol.
    pub worktree: Option<TaskNewWorktree>,
}

/// Stores per-invocation worktree settings for `task new --worktree`.
///
/// Validated by [`task_new`].
#[derive(Debug, Clone, Default)]
pub struct TaskNewWorktree {
    /// `--branch-type <type>` — one of `BRANCH_TYPES`. `None` → `cfg.branch_prefix`.
    pub branch_type: Option<String>,
    /// `--branch <full>` — overrides both `branch_type` and `cfg.branch_prefix`.
    pub branch_override: Option<String>,
}

/// Summary of a new task creation.
#[derive(Debug, Clone)]
pub struct TaskNewSummary {
    /// Created task slug.
    pub slug: String,
    /// Workflow tier for the created task.
    pub tier: Tier,
    /// Path to the created task directory.
    pub task_dir: PathBuf,
    /// Worktree details when the task was created with `--worktree`.
    pub worktree: Option<TaskNewWorktreeSummary>,
    /// Slug whose focus was overwritten by this `task new`, if any.
    ///
    /// Populated when the checkout had `state.focus = Some(other)` *before*
    /// this call and `other != self.slug`. The CLI surfaces this as a
    /// stderr-rendered warning suggesting `--worktree` for parallel work.
    /// `None` when the checkout had no prior focus, or when the prior focus
    /// already matched the new slug.
    pub overwrote_focus: Option<String>,
}

/// Summary of worktree state created for a new task.
#[derive(Debug, Clone)]
pub struct TaskNewWorktreeSummary {
    /// Branch checked out in the worktree.
    pub branch: String,
    /// Path to the created worktree checkout.
    pub worktree_path: PathBuf,
    /// Base branch or detached HEAD used to create the worktree.
    pub base_branch: String,
}

impl fmt::Display for TaskNewSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "created {:?} task `{}` at {}",
            self.tier,
            self.slug,
            self.task_dir.display()
        )?;
        if let Some(wt) = &self.worktree {
            write!(
                f,
                " (worktree: branch `{}` at {} from base `{}`)",
                wt.branch,
                wt.worktree_path.display(),
                wt.base_branch
            )?;
        }
        if let Some(prev) = &self.overwrote_focus {
            write!(
                f,
                "\nwarn: focus rebound from `{prev}` to `{}`. Run `ark agent task resume --slug \
                 {prev}` to restore, or pass `--worktree` to keep both bound in parallel.",
                self.slug
            )?;
        }
        Ok(())
    }
}

/// Creates a new task directory and optional task worktree.
pub fn task_new(opts: TaskNewOptions) -> Result<TaskNewSummary> {
    validate_slug(&opts.slug)?;
    validate_title(&opts.title)?;

    match opts.worktree.clone() {
        Some(wt) => task_new_with_worktree(opts, wt),
        None => task_new_no_worktree(opts),
    }
}

/// Scaffolds the task directory on the parent checkout, without worktree creation.
fn task_new_no_worktree(opts: TaskNewOptions) -> Result<TaskNewSummary> {
    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);

    if task_dir.exists() {
        return Err(Error::TaskAlreadyExists { slug: opts.slug });
    }

    task_dir.ensure_dir()?;
    copy_template("PRD", &task_dir.join("PRD.md"))?;
    build_task_toml(&opts).save(&task_dir)?;

    let overwrote_focus = register_focus(&layout, &opts.slug)?;

    Ok(TaskNewSummary {
        slug: opts.slug,
        tier: opts.tier,
        task_dir,
        worktree: None,
        overwrote_focus,
    })
}

/// Runs the worktree-first creation flow.
fn task_new_with_worktree(opts: TaskNewOptions, wt: TaskNewWorktree) -> Result<TaskNewSummary> {
    let layout = Layout::new(&opts.project_root);

    // Reject when invoked from inside an existing `.ark/worktrees/<branch>/` tree.
    if is_under_worktrees(layout.root()) {
        return Err(Error::NestedWorktreeForbidden {
            current_root: layout.root().to_path_buf(),
        });
    }

    // Refuse to silently shadow an existing parent task dir.
    let parent_task_dir = layout.task_dir(&opts.slug);
    if parent_task_dir.exists() {
        return Err(Error::TaskExistsOnParent {
            slug: opts.slug.clone(),
            path: parent_task_dir,
        });
    }

    let cfg = WorktreeConfig::load_or_default(&layout)?;

    // Cross-worktree extension: also refuse if some existing worktree
    // already owns the slug. Without this guard, a second `task new
    // --worktree --slug foo --branch other/foo` would succeed and leave
    // discovery ambiguous (find_worktree_for_slug returns the first match).
    let worktrees_dir = cfg.resolve_worktrees_dir(&layout);
    if let Some((existing, _)) = find_worktree_for_slug(layout.root(), &worktrees_dir, &opts.slug)?
    {
        return Err(Error::TaskExistsOnParent {
            slug: opts.slug.clone(),
            path: existing,
        });
    }

    let branch = resolve_branch(
        wt.branch_override.as_deref(),
        wt.branch_type.as_deref(),
        &cfg.branch_prefix,
        &opts.slug,
    )?;
    let base_branch = resolve_base_branch(layout.root())?;

    // Reject malformed branch names early, before any side effects.
    let ref_check = run_git(&["check-ref-format", "--branch", &branch], layout.root())?;
    if !ref_check.is_success() {
        return Err(Error::InvalidBranchName {
            branch: branch.clone(),
            reason: ref_check.stderr.trim().to_string(),
        });
    }

    // Refuse if the branch is checked out elsewhere.
    if let Some(where_at) = branch_in_use(layout.root(), &branch)? {
        return Err(Error::BranchInUse { branch, where_at });
    }

    // Refuse if the worktree path already exists on disk.
    let worktree_path = cfg.resolve_worktree_dir(&layout, &branch);
    if worktree_path.exists() {
        return Err(Error::WorktreeDirExists {
            path: worktree_path,
        });
    }
    if let Some(parent) = worktree_path.parent() {
        parent.ensure_dir()?;
    }

    // `git worktree add` is the rollback boundary: any failure past here
    // runs `rollback_worktree` before returning the original error.
    let wt_str = worktree_path
        .to_str()
        .expect("worktree paths are utf-8 by construction");
    let add_out = run_git(
        &["worktree", "add", "-b", &branch, wt_str, &base_branch],
        layout.root(),
    )?;
    if !add_out.is_success() {
        return Err(git_error(
            &worktree_path,
            "git worktree add",
            &add_out.stderr,
        ));
    }

    scaffold_inside_worktree(&opts, &cfg, &worktree_path, &branch, &base_branch)
        .inspect_err(|_| rollback_worktree(layout.root(), &worktree_path, &branch))
}

fn scaffold_inside_worktree(
    opts: &TaskNewOptions,
    cfg: &WorktreeConfig,
    worktree_path: &Path,
    branch: &str,
    base_branch: &str,
) -> Result<TaskNewSummary> {
    let project_root = &opts.project_root;
    let wt_layout = Layout::new(worktree_path);
    let task_dir = wt_layout.task_dir(&opts.slug);
    task_dir.ensure_dir()?;
    copy_template("PRD", &task_dir.join("PRD.md"))?;

    // Store worktree_path as project-relative for snapshot portability.
    let relative_path = worktree_path
        .strip_prefix(project_root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| worktree_path.to_path_buf());

    let mut toml = build_task_toml(opts);
    toml.branch = Some(branch.to_string());
    toml.worktree_path = Some(relative_path);
    toml.base_branch = Some(base_branch.to_string());
    toml.save(&task_dir)?;

    let overwrote_focus = register_focus(&wt_layout, &opts.slug)?;

    // Mirror parent identity into the worktree (prompts on TTY when missing).
    sync_identity(project_root, worktree_path)?;

    // Hard-fail on missing copy source so partial worktrees never escape creation.
    for rel in &cfg.copy {
        let src = project_root.join(rel);
        if !src.exists() {
            return Err(Error::WorktreeCopySourceMissing { path: src });
        }
        let dst = worktree_path.join(rel);
        if let Some(parent) = dst.parent() {
            parent.ensure_dir()?;
        }
        dst.write_bytes(&src.read_bytes()?)?;
    }

    // Run post-creation hooks sequentially; abort on the first non-zero exit.
    for cmd in &cfg.post_create {
        let exit_code = run_shell(cmd, worktree_path)?;
        if exit_code != 0 {
            return Err(Error::PostCreateHookFailed {
                command: cmd.clone(),
                exit_code,
            });
        }
    }

    Ok(TaskNewSummary {
        slug: opts.slug.clone(),
        tier: opts.tier,
        task_dir,
        worktree: Some(TaskNewWorktreeSummary {
            branch: branch.to_string(),
            worktree_path: worktree_path.to_path_buf(),
            base_branch: base_branch.to_string(),
        }),
        overwrote_focus,
    })
}

/// Mirrors developer identity from `parent` into `worktree`.
///
/// Reads `<parent>/.ark/.developer`. On `MissingIdentity`, prompts the user
/// when stdin is a TTY, writes the answer to the parent (so subsequent
/// worktrees skip the prompt), then mirrors into the worktree. Non-TTY +
/// missing identity returns `Error::MissingIdentity` unchanged so the agent
/// surfaces the existing remediation message.
fn sync_identity(parent: &Path, worktree: &Path) -> Result<()> {
    use std::io::{BufReader, IsTerminal, stderr, stdin};
    const MAX_PROMPT_ATTEMPTS: u8 = 3;

    match identity_resolve(IdentityResolveOptions::new(parent)) {
        Ok(id) => identity_write(worktree, &id),
        Err(Error::MissingIdentity) if stdin().is_terminal() => {
            let stdin_handle = stdin();
            let mut reader = BufReader::new(stdin_handle.lock());
            let stderr_handle = stderr();
            let mut writer = stderr_handle.lock();
            let id = identity_prompt(&mut reader, &mut writer, MAX_PROMPT_ATTEMPTS)?;
            identity_write(parent, &id)?;
            identity_write(worktree, &id)
        }
        Err(e) => Err(e),
    }
}

fn build_task_toml(opts: &TaskNewOptions) -> TaskToml {
    let now = Utc::now();
    TaskToml {
        id: opts.slug.clone(),
        title: opts.title.clone(),
        tier: opts.tier,
        // Research tier starts in its dedicated working phase; every other
        // tier starts in Design.
        phase: match opts.tier {
            Tier::Research => Phase::Research,
            _ => Phase::Design,
        },
        iteration: 0,
        max_iterations: match opts.tier {
            Tier::Deep => Some(3),
            _ => None,
        },
        created_at: now,
        updated_at: now,
        archived_at: None,
        committed_at: None,
        branch: None,
        worktree_path: None,
        base_branch: None,
        start_head: capture_start_head(&opts.project_root),
        journal_path: None,
    }
}

/// Captures the parent checkout's HEAD SHA for the journal commit-range start.
///
/// Returns `None` when `git rev-parse HEAD` fails — typically on an unborn
/// HEAD in a freshly-initialized repository. Tasks created in this state
/// degrade gracefully: their later `task_commit` falls back to `git log -n
/// 20` for the journal commits-in-range table.
fn capture_start_head(project_root: &Path) -> Option<String> {
    run_git(&["rev-parse", "HEAD"], project_root)
        .ok()
        .filter(|out| out.is_success())
        .map(|out| out.stdout.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Returns `true` if `root` resolves under any `.ark/worktrees/` path.
fn is_under_worktrees(root: &Path) -> bool {
    root.components()
        .map(|c| c.as_os_str())
        .collect::<Vec<_>>()
        .windows(2)
        .any(|w| w[0] == ".ark" && w[1] == "worktrees")
}

/// Resolves the current branch name, falling back to the commit SHA on a detached HEAD.
///
/// Tries `symbolic-ref --short HEAD` first; on failure, falls back to `git rev-parse HEAD`.
fn resolve_base_branch(root: &Path) -> Result<String> {
    let sym = run_git(&["symbolic-ref", "--short", "HEAD"], root)?;
    if sym.is_success() {
        return Ok(sym.stdout.trim().to_string());
    }
    let rev = run_git(&["rev-parse", "HEAD"], root)?;
    if !rev.is_success() {
        // Both symbolic-ref and rev-parse fail on an unborn HEAD.
        return Err(git_error(
            root,
            "could not resolve HEAD (unborn?)",
            &rev.stderr,
        ));
    }
    Ok(rev.stdout.trim().to_string())
}

/// Returns the path of another worktree currently using `branch`.
fn branch_in_use(root: &Path, branch: &str) -> Result<Option<PathBuf>> {
    Ok(parse_git_worktree_list(root)?
        .into_iter()
        .find(|e| e.branch.as_deref() == Some(branch))
        .map(|e| e.path))
}

/// Performs best-effort worktree rollback.
///
/// Errors during rollback are swallowed; the original error is what the user
/// sees.
fn rollback_worktree(root: &Path, worktree_path: &Path, branch: &str) {
    let Some(wt_str) = worktree_path.to_str() else {
        return;
    };
    let _ = run_git(&["worktree", "remove", "--force", wt_str], root);
    let _ = run_git(&["branch", "-D", branch], root);
}

/// Wraps a non-zero git exit as [`Error::Io`] carrying the trimmed stderr.
fn git_error(path: &Path, action: &str, stderr: &str) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(format!("{action} failed: {}", stderr.trim())),
    }
}

/// Records `slug` as an active task and this checkout's focus.
///
/// Returns the previously-bound focus (if any) when it differed from `slug`.
/// Callers surface that as a "focus rebound" warning in their summary.
fn register_focus(layout: &Layout, slug: &str) -> Result<Option<String>> {
    let mut overwrote: Option<String> = None;
    state_mutate(layout, |state| {
        let prev = state.focus.clone();
        if !state.tasks.active.iter().any(|s| s == slug) {
            state.tasks.active.push(slug.to_string());
        }
        state.focus = Some(slug.to_string());
        overwrote = match prev {
            Some(p) if p != slug => Some(p),
            _ => None,
        };
        Ok(())
    })?;
    Ok(overwrote)
}

#[cfg(test)]
#[path = "new_tests.rs"]
mod tests;
