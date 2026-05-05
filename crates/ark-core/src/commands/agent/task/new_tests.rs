use std::path::Path;

use super::*;
use crate::commands::agent::workspace::{Identity, identity_write};

/// Initializes a temp git repo with one commit and a parent developer identity.
///
/// The initial commit lets `git worktree add` run. The pre-seeded
/// `.ark/.developer` lets `task new --worktree` skip its prompt branch in
/// non-TTY test processes. Returns the repo root.
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
    identity_write(tmp.path(), &Identity::new("test-dev").unwrap()).unwrap();
    tmp
}

/// Variant of [`init_repo`] without seeded developer identity.
///
/// Use to exercise the missing-identity branch of `task new --worktree`.
fn init_repo_without_identity() -> tempfile::TempDir {
    let tmp = init_repo();
    let dev = tmp.path().join(".ark/.developer");
    if dev.exists() {
        std::fs::remove_file(&dev).unwrap();
    }
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
    let layout = Layout::new(tmp.path());
    let state = crate::state::load_state(&layout, &crate::session::ppid::RealPpid::new()).unwrap();
    assert!(state.tasks.active.iter().any(|s| s == "demo"));
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

/// Verifies that `task new --worktree` scaffolds inside the worktree.
///
/// The parent checkout is untouched, and `task.toml` stores `branch`,
/// `base_branch`, and a project-relative `worktree_path`.
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
    let wt_layout = Layout::new(&wt);
    let wt_state =
        crate::state::load_state(&wt_layout, &crate::session::ppid::RealPpid::new()).unwrap();
    assert!(wt_state.tasks.active.iter().any(|s| s == "foo"));

    let toml = TaskToml::load(&wt_task_dir).unwrap();
    assert_eq!(toml.branch.as_deref(), Some("feat/foo"));
    assert_eq!(toml.base_branch.as_deref(), Some("main"));
    // Project-relative path stored.
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

/// Verifies same-slug parent task rejection for worktree tasks.
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

/// Verifies nested worktree rejection.
///
/// The current root resolves under `.ark/worktrees/`.
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

/// Verifies missing copy-source rollback.
///
/// A missing `[worktree].copy` source aborts with
/// `WorktreeCopySourceMissing` and rolls back the worktree dir.
#[test]
fn worktree_copy_missing_source_hard_fails_and_rolls_back() {
    let tmp = init_repo();
    // Configure copy = [".env"] but do not create .env.
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

/// Verifies failing post-create rollback.
///
/// A failing `[worktree].post_create` command rolls back the worktree dir
/// and returns `PostCreateHookFailed`.
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

/// Verifies that post-create commands run in the worktree.
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

/// Verifies invalid branch rejection before side effects.
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

/// Verifies existing worktree path rejection.
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

/// Verifies that non-worktree tasks leave `.ark/worktrees/` empty.
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

/// Verifies duplicate-slug rejection across worktrees.
///
/// A second `task new --worktree --slug foo` with a different branch is
/// rejected to keep worktree discovery unambiguous.
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
    // The parent's task dir is empty under worktree-first, so the parent-slug guard
    // does not fire; the cross-worktree guard must catch this.
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

/// Verifies that `[worktree].worktree_dir` controls worktree placement.
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

/// Verifies that `--branch-type fix` produces the branch `fix/<slug>`.
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

// ---- V-IT-1 / V-IT-2 / V-IT-3 / V-E-1: identity-sync + submodule defaults ----

/// V-IT-1: parent identity is mirrored into the new worktree.
#[test]
fn worktree_creation_mirrors_parent_identity() {
    let tmp = init_repo(); // seeds .ark/.developer = "test-dev"
    // Override post_create to keep the test fast (skip submodule init).
    tmp.path().join(".ark").ensure_dir().unwrap();
    tmp.path()
        .join(".ark/config.toml")
        .write_bytes(b"[worktree]\npost_create = []\n")
        .unwrap();

    task_new(TaskNewOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "foo".into(),
        title: "t".into(),
        tier: Tier::Standard,
        worktree: Some(TaskNewWorktree::default()),
    })
    .unwrap();

    let wt_dev = tmp.path().join(".ark/worktrees/feat/foo/.ark/.developer");
    assert!(wt_dev.is_file(), "worktree must have .ark/.developer");
    let content = std::fs::read_to_string(&wt_dev).unwrap();
    assert_eq!(content.trim(), "test-dev");
}

/// V-IT-2: missing parent identity in non-TTY context returns MissingIdentity
/// and rolls back the worktree dir.
#[test]
fn worktree_creation_fails_on_missing_identity_when_non_tty() {
    let tmp = init_repo_without_identity();
    // cargo test runs with stdin redirected — IsTerminal returns false.
    tmp.path().join(".ark").ensure_dir().unwrap();
    tmp.path()
        .join(".ark/config.toml")
        .write_bytes(b"[worktree]\npost_create = []\n")
        .unwrap();

    let err = task_new(TaskNewOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "foo".into(),
        title: "t".into(),
        tier: Tier::Standard,
        worktree: Some(TaskNewWorktree::default()),
    })
    .unwrap_err();
    assert!(matches!(err, Error::MissingIdentity));
    assert!(
        !tmp.path().join(".ark/worktrees/feat/foo").exists(),
        "rollback should remove the worktree dir"
    );
}

/// V-IT-3: default `post_create` (submodule init) is a safe no-op in repos
/// without `.gitmodules`.
#[test]
fn worktree_post_create_default_runs_submodule_init() {
    let tmp = init_repo();
    // No config.toml override — exercise the embedded default post_create.
    task_new(TaskNewOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "foo".into(),
        title: "t".into(),
        tier: Tier::Standard,
        worktree: Some(TaskNewWorktree::default()),
    })
    .unwrap();

    assert!(tmp.path().join(".ark/worktrees/feat/foo").is_dir());
}

/// V-E-1: explicit `post_create = []` overrides the default.
#[test]
fn worktree_creation_succeeds_when_user_overrides_post_create_to_empty() {
    let tmp = init_repo();
    tmp.path().join(".ark").ensure_dir().unwrap();
    tmp.path()
        .join(".ark/config.toml")
        .write_bytes(b"[worktree]\npost_create = []\n")
        .unwrap();

    task_new(TaskNewOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "foo".into(),
        title: "t".into(),
        tier: Tier::Standard,
        worktree: Some(TaskNewWorktree::default()),
    })
    .unwrap();

    assert!(tmp.path().join(".ark/worktrees/feat/foo").is_dir());
}
