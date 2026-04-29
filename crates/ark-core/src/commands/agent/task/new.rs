//! `ark agent task new` — scaffold a new task directory.
//!
//! Creates `.ark/tasks/<slug>/`, seeds `PRD.md` from the embedded template,
//! writes `task.toml` with phase=Design + iteration=0, and points
//! `.ark/tasks/.current` at the new slug. Refuses to overwrite an existing
//! task directory.
//!
//! When `opts.worktree.is_some()`, the worktree-first protocol kicks in
//! (worktree-support C-2): create a git worktree under `.ark/worktrees/`
//! first, then scaffold the task dir *inside the worktree*. The parent
//! checkout's `.ark/tasks/` is never modified by this path.

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
    },
    error::{Error, Result},
    io::{
        PathExt,
        git::{run_git, run_shell},
    },
    layout::Layout,
};

#[derive(Debug, Clone)]
pub struct TaskNewOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    /// When `Some`, run the worktree-first protocol (worktree-support G-3).
    pub worktree: Option<TaskNewWorktree>,
}

/// Per-invocation worktree settings for `task new --worktree`.
/// Validated by [`task_new`] (worktree-support C-4, C-5, C-6).
#[derive(Debug, Clone, Default)]
pub struct TaskNewWorktree {
    /// `--branch-type <type>` — one of `BRANCH_TYPES`. `None` → `cfg.branch_prefix`.
    pub branch_type: Option<String>,
    /// `--branch <full>` — overrides both `branch_type` and `cfg.branch_prefix`.
    pub branch_override: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskNewSummary {
    pub slug: String,
    pub tier: Tier,
    pub task_dir: PathBuf,
    pub worktree: Option<TaskNewWorktreeSummary>,
}

#[derive(Debug, Clone)]
pub struct TaskNewWorktreeSummary {
    pub branch: String,
    pub worktree_path: PathBuf,
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
        Ok(())
    }
}

pub fn task_new(opts: TaskNewOptions) -> Result<TaskNewSummary> {
    validate_slug(&opts.slug)?;
    validate_title(&opts.title)?;

    match opts.worktree.clone() {
        Some(wt) => task_new_with_worktree(opts, wt),
        None => task_new_no_worktree(opts),
    }
}

/// The original (pre-worktree-support) flow: scaffold the task dir on the
/// parent checkout. Used when `--worktree` is absent.
fn task_new_no_worktree(opts: TaskNewOptions) -> Result<TaskNewSummary> {
    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);

    if task_dir.exists() {
        return Err(Error::TaskAlreadyExists { slug: opts.slug });
    }

    task_dir.ensure_dir()?;
    copy_template("PRD", &task_dir.join("PRD.md"))?;
    build_task_toml(&opts).save(&task_dir)?;

    layout
        .tasks_current()
        .write_bytes(format!("{}\n", opts.slug).as_bytes())?;

    Ok(TaskNewSummary {
        slug: opts.slug,
        tier: opts.tier,
        task_dir,
        worktree: None,
    })
}

/// Worktree-first flow per worktree-support G-3 / C-2.
fn task_new_with_worktree(opts: TaskNewOptions, wt: TaskNewWorktree) -> Result<TaskNewSummary> {
    let layout = Layout::new(&opts.project_root);

    // C-13: reject when invoked from inside an existing `.ark/worktrees/<branch>/` tree.
    if is_under_worktrees(layout.root()) {
        return Err(Error::NestedWorktreeForbidden {
            current_root: layout.root().to_path_buf(),
        });
    }

    // F-8: refuse to silently shadow an existing parent task dir.
    let parent_task_dir = layout.task_dir(&opts.slug);
    if parent_task_dir.exists() {
        return Err(Error::TaskExistsOnParent {
            slug: opts.slug.clone(),
            path: parent_task_dir,
        });
    }

    let cfg = WorktreeConfig::load_or_default(&layout)?;

    // F-8 cross-worktree extension: also refuse if some existing worktree
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

    // C-5: reject malformed branch names early, before any side effects.
    let ref_check = run_git(&["check-ref-format", "--branch", &branch], layout.root())?;
    if !ref_check.is_success() {
        return Err(Error::InvalidBranchName {
            branch: branch.clone(),
            reason: ref_check.stderr.trim().to_string(),
        });
    }

    // F-4: refuse if the branch is checked out elsewhere.
    if let Some(where_at) = branch_in_use(layout.root(), &branch)? {
        return Err(Error::BranchInUse { branch, where_at });
    }

    // F-6: refuse if the worktree path already exists on disk.
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

    // C-21: store worktree_path as project-relative for snapshot portability.
    let relative_path = worktree_path
        .strip_prefix(project_root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| worktree_path.to_path_buf());

    let mut toml = build_task_toml(opts);
    toml.branch = Some(branch.to_string());
    toml.worktree_path = Some(relative_path);
    toml.base_branch = Some(base_branch.to_string());
    toml.save(&task_dir)?;

    wt_layout
        .tasks_current()
        .write_bytes(format!("{}\n", opts.slug).as_bytes())?;

    // F-2: hard-fail on missing copy source.
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

    // F-3: post_create hooks run sequentially; abort on first non-zero.
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
    })
}

fn build_task_toml(opts: &TaskNewOptions) -> TaskToml {
    let now = Utc::now();
    TaskToml {
        id: opts.slug.clone(),
        title: opts.title.clone(),
        tier: opts.tier,
        phase: Phase::Design,
        iteration: 0,
        max_iterations: match opts.tier {
            Tier::Deep => Some(3),
            _ => None,
        },
        created_at: now,
        updated_at: now,
        archived_at: None,
        branch: None,
        worktree_path: None,
        base_branch: None,
    }
}

/// True iff `root` resolves under any `.ark/worktrees/...` path. C-13 guard.
fn is_under_worktrees(root: &Path) -> bool {
    root.components()
        .map(|c| c.as_os_str())
        .collect::<Vec<_>>()
        .windows(2)
        .any(|w| w[0] == ".ark" && w[1] == "worktrees")
}

/// F-17: detached-HEAD fallback. Try `symbolic-ref --short HEAD` first; on
/// failure, fall back to `git rev-parse HEAD` and store the SHA verbatim.
fn resolve_base_branch(root: &Path) -> Result<String> {
    let sym = run_git(&["symbolic-ref", "--short", "HEAD"], root)?;
    if sym.is_success() {
        return Ok(sym.stdout.trim().to_string());
    }
    let rev = run_git(&["rev-parse", "HEAD"], root)?;
    if !rev.is_success() {
        // F-16: unborn HEAD — both fail.
        return Err(git_error(
            root,
            "could not resolve HEAD (unborn?)",
            &rev.stderr,
        ));
    }
    Ok(rev.stdout.trim().to_string())
}

/// Returns `Some(path)` if `branch` is currently checked out by another
/// worktree, else `None`. F-4 detection.
fn branch_in_use(root: &Path, branch: &str) -> Result<Option<PathBuf>> {
    Ok(parse_git_worktree_list(root)?
        .into_iter()
        .find(|e| e.branch.as_deref() == Some(branch))
        .map(|e| e.path))
}

/// Best-effort rollback per F-1. Errors during rollback are swallowed —
/// the original error is what the user sees.
fn rollback_worktree(root: &Path, worktree_path: &Path, branch: &str) {
    let Some(wt_str) = worktree_path.to_str() else {
        return;
    };
    let _ = run_git(&["worktree", "remove", "--force", wt_str], root);
    let _ = run_git(&["branch", "-D", branch], root);
}

/// Wrap a non-zero git exit as `Error::Io` carrying the trimmed stderr.
fn git_error(path: &Path, action: &str, stderr: &str) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(format!("{action} failed: {}", stderr.trim())),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// Helper: initialize a temp git repo with one commit so worktree-add
    /// works. Returns the repo root.
    fn init_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        run_git(&["init", "--quiet"], tmp.path()).unwrap();
        run_git(&["config", "user.email", "test@example.com"], tmp.path()).unwrap();
        run_git(&["config", "user.name", "Test"], tmp.path()).unwrap();
        run_git(&["config", "commit.gpgsign", "false"], tmp.path()).unwrap();
        // Default branch name varies; force `main` for predictability.
        run_git(&["checkout", "-b", "main"], tmp.path()).unwrap();
        tmp.path()
            .join("README.md")
            .write_bytes(b"# repo\n")
            .unwrap();
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(&["commit", "-m", "init", "--quiet"], tmp.path()).unwrap();
        tmp
    }

    #[test]
    fn creates_task_dir_prd_toml_and_current() {
        let tmp = tempfile::tempdir().unwrap();
        let summary = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "demo task".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();

        let task_dir = tmp.path().join(".ark/tasks/demo");
        assert!(task_dir.is_dir());
        assert!(task_dir.join("PRD.md").is_file());
        assert!(task_dir.join("task.toml").is_file());
        assert_eq!(
            tmp.path()
                .join(".ark/tasks/.current")
                .read_text()
                .unwrap()
                .trim(),
            "demo"
        );
        assert_eq!(summary.slug, "demo");
        assert_eq!(summary.tier, Tier::Standard);
        assert!(summary.worktree.is_none());

        let loaded = TaskToml::load(&task_dir).unwrap();
        assert_eq!(loaded.phase, Phase::Design);
        assert_eq!(loaded.iteration, 0);
    }

    #[test]
    fn errors_when_task_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskAlreadyExists { .. }));
    }

    #[test]
    fn rejects_path_traversal_slug() {
        let tmp = tempfile::tempdir().unwrap();
        for bad in ["../escape", "/abs", "a/b", "."] {
            let err = task_new(TaskNewOptions {
                project_root: tmp.path().to_path_buf(),
                slug: bad.into(),
                title: "t".into(),
                tier: Tier::Quick,
                worktree: None,
            })
            .unwrap_err();
            assert!(
                matches!(err, Error::InvalidTaskField { .. }),
                "expected reject for {bad:?}"
            );
        }
    }

    #[test]
    fn rejects_invalid_title() {
        let tmp = tempfile::tempdir().unwrap();
        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "ok".into(),
            title: "A | B".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidTaskField { .. }));
    }

    #[test]
    fn deep_tier_seeds_max_iterations() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep1".into(),
            title: "t".into(),
            tier: Tier::Deep,
            worktree: None,
        })
        .unwrap();
        let toml = TaskToml::load(&tmp.path().join(".ark/tasks/deep1")).unwrap();
        assert_eq!(toml.max_iterations, Some(3));
    }

    /// V-IT-1: task_new_worktree_happy_path.
    #[test]
    fn worktree_happy_path() {
        let tmp = init_repo();
        let summary = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "foo task".into(),
            tier: Tier::Deep,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap();

        let wt = tmp.path().join(".ark/worktrees/feat/foo");
        assert!(wt.is_dir(), "worktree dir should exist");
        assert!(
            !tmp.path().join(".ark/tasks/foo").exists(),
            "parent must NOT have the task dir (worktree-first)"
        );
        let wt_task_dir = wt.join(".ark/tasks/foo");
        assert!(wt_task_dir.is_dir(), "worktree's task dir should exist");
        assert!(wt_task_dir.join("PRD.md").is_file());
        assert!(wt_task_dir.join("task.toml").is_file());
        assert_eq!(
            wt.join(".ark/tasks/.current").read_text().unwrap().trim(),
            "foo"
        );

        let toml = TaskToml::load(&wt_task_dir).unwrap();
        assert_eq!(toml.branch.as_deref(), Some("feat/foo"));
        assert_eq!(toml.base_branch.as_deref(), Some("main"));
        // C-21: project-relative path stored.
        assert_eq!(
            toml.worktree_path.as_deref(),
            Some(Path::new(".ark/worktrees/feat/foo"))
        );

        assert_eq!(summary.slug, "foo");
        assert!(summary.worktree.is_some());
        let wt_sum = summary.worktree.unwrap();
        assert_eq!(wt_sum.branch, "feat/foo");
        assert_eq!(wt_sum.base_branch, "main");
    }

    /// V-IT-19: task_exists_on_parent_rejected.
    #[test]
    fn worktree_rejects_when_parent_task_dir_exists() {
        let tmp = init_repo();
        // Create the task without --worktree first.
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        // Now retry with --worktree → must reject.
        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskExistsOnParent { .. }));
    }

    /// V-IT-3: task_new_worktree_rejected_inside_worktree.
    /// Simulates: cwd is `<repo>/.ark/worktrees/feat/foo` — must reject.
    #[test]
    fn worktree_rejects_when_cwd_is_inside_worktree() {
        let tmp = init_repo();
        let inside = tmp.path().join(".ark/worktrees/feat/foo");
        inside.ensure_dir().unwrap();

        let err = task_new(TaskNewOptions {
            project_root: inside.clone(),
            slug: "bar".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap_err();
        assert!(matches!(err, Error::NestedWorktreeForbidden { .. }));
    }

    /// V-IT-7: task_new_worktree_copy_missing_source_hard_fails.
    #[test]
    fn worktree_copy_missing_source_hard_fails_and_rolls_back() {
        let tmp = init_repo();
        // Configure copy = [".env"] but don't create .env.
        tmp.path().join(".ark").ensure_dir().unwrap();
        tmp.path()
            .join(".ark/config.toml")
            .write_bytes(b"[worktree]\ncopy = [\".env\"]\n")
            .unwrap();

        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap_err();
        assert!(matches!(err, Error::WorktreeCopySourceMissing { .. }));
        // Rollback: worktree dir must be gone.
        assert!(
            !tmp.path().join(".ark/worktrees/feat/foo").exists(),
            "rollback should remove the worktree dir"
        );
    }

    /// V-IT-2: task_new_worktree_rollback_on_post_create.
    #[test]
    fn worktree_rollback_on_post_create_failure() {
        let tmp = init_repo();
        tmp.path().join(".ark").ensure_dir().unwrap();
        tmp.path()
            .join(".ark/config.toml")
            .write_bytes(b"[worktree]\npost_create = [\"false\"]\n")
            .unwrap();

        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap_err();
        assert!(matches!(err, Error::PostCreateHookFailed { .. }));
        assert!(!tmp.path().join(".ark/worktrees/feat/foo").exists());
    }

    /// V-IT-8: task_new_worktree_post_create_runs_in_worktree.
    #[test]
    fn worktree_post_create_runs_in_worktree() {
        let tmp = init_repo();
        tmp.path().join(".ark").ensure_dir().unwrap();
        tmp.path()
            .join(".ark/config.toml")
            .write_bytes(b"[worktree]\npost_create = [\"touch hello.txt\"]\n")
            .unwrap();

        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap();

        assert!(
            tmp.path()
                .join(".ark/worktrees/feat/foo/hello.txt")
                .exists()
        );
    }

    /// V-IT-17: task_new_worktree_invalid_branch_rejected.
    #[test]
    fn worktree_invalid_branch_rejected() {
        let tmp = init_repo();
        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree {
                branch_override: Some("..".into()),
                ..Default::default()
            }),
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidBranchName { .. }));
    }

    /// V-IT-18: task_new_worktree_dir_exists_rejected.
    #[test]
    fn worktree_dir_exists_rejected() {
        let tmp = init_repo();
        // Pre-create the worktree path as a normal dir.
        tmp.path()
            .join(".ark/worktrees/feat/foo")
            .ensure_dir()
            .unwrap();

        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap_err();
        assert!(matches!(err, Error::WorktreeDirExists { .. }));
    }

    /// V-IT-10: task_new_without_worktree_makes_no_worktree_changes.
    #[test]
    fn task_new_without_worktree_writes_nothing_under_worktrees_dir() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        let wt_root = tmp.path().join(".ark/worktrees");
        assert!(
            !wt_root.exists() || wt_root.read_dir().unwrap().next().is_none(),
            ".ark/worktrees should not exist or be empty"
        );
    }

    /// PR#8 fix: a second `task new --worktree --slug foo` with a different
    /// branch must reject — otherwise discovery (cleanup/list) is ambiguous.
    #[test]
    fn worktree_rejects_duplicate_slug_across_worktrees() {
        let tmp = init_repo();
        // First worktree: branch defaults to `feat/foo`.
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap();
        // Parent's task dir is empty (worktree-first), so the F-8 parent guard
        // doesn't fire. The new cross-worktree guard must.
        let err = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree {
                branch_override: Some("fix/foo-2".into()),
                branch_type: None,
            }),
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskExistsOnParent { .. }));
    }

    /// PR#8 fix: `[worktree].worktree_dir` in `.ark/config.toml` must control
    /// where worktrees land — previously it was loaded but ignored, so the
    /// layout default always won.
    #[test]
    fn worktree_honors_custom_worktree_dir_from_config() {
        let tmp = init_repo();
        tmp.path().join(".ark").ensure_dir().unwrap();
        tmp.path()
            .join(".ark/config.toml")
            .write_bytes(b"[worktree]\nworktree_dir = \".ark/wt\"\n")
            .unwrap();

        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap();

        assert!(
            tmp.path().join(".ark/wt/feat/foo").is_dir(),
            "worktree should land at the configured dir"
        );
        assert!(
            !tmp.path().join(".ark/worktrees").exists(),
            "default dir must not be created when overridden"
        );
    }

    /// Branch-type override: --branch-type fix → branch is `fix/<slug>`.
    #[test]
    fn worktree_branch_type_override() {
        let tmp = init_repo();
        let summary = task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree {
                branch_type: Some("fix".into()),
                branch_override: None,
            }),
        })
        .unwrap();
        let wt = summary.worktree.unwrap();
        assert_eq!(wt.branch, "fix/foo");
        assert!(tmp.path().join(".ark/worktrees/fix/foo").is_dir());
    }
}
