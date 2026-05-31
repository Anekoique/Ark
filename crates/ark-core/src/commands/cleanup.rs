//! `ark cleanup` — list (and on `--apply`, remove) worktrees of closed tasks
//! or worktrees whose backing branch is gone.
//!
//! Stable top-level CLI verb, peer to `ark archive`. Dry-run by default;
//! `--apply` is required to mutate. The mutation path delegates to
//! [`worktree_cleanup`] so per-row dirty checks, branch deletion, and empty
//! parent pruning all reuse the existing worktree-feature contract.

use std::{
    collections::HashSet,
    fmt,
    path::{Path, PathBuf},
};

use crate::{
    commands::agent::{
        state::{Phase, TaskToml},
        task::worktree::{
            WorktreeCleanupOptions, WorktreeCleanupSummary, WorktreeConfig,
            discovery::{is_under, parse_git_worktree_list},
            worktree_cleanup,
        },
    },
    error::Result,
    io::{PathExt, git::run_git},
    layout::Layout,
    state::load_state,
};

/// Reason a worktree is eligible for removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupReason {
    /// Backing task's `task.toml` reports `phase = Committed`.
    Committed,
    /// Slug exists under `tasks/archive/<YYYY-MM>/<tier>/<slug>/` on the parent.
    Archived,
    /// Backing branch is missing from `git branch --list`.
    BranchGone,
}

impl fmt::Display for CleanupReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Committed => write!(f, "committed"),
            Self::Archived => write!(f, "archived"),
            Self::BranchGone => write!(f, "branch-gone"),
        }
    }
}

/// One prunable worktree row.
///
/// A worktree may carry several active slugs (e.g. multiple tasks created
/// and committed inside the same worktree); they are grouped here so that
/// the user sees one row per *worktree*, not one row per slug.
#[derive(Debug, Clone)]
pub struct CleanupRow {
    /// Slug used to drive `worktree_cleanup` — the slug whose
    /// `task.toml.branch` matches the worktree's branch when one such slug
    /// exists, otherwise the first sorted member of [`slugs`].
    ///
    /// [`slugs`]: Self::slugs
    pub primary_slug: String,
    /// All task slugs bound to the worktree, sorted ascending.
    pub slugs: Vec<String>,
    /// Branch checked out by the worktree, when known.
    pub branch: Option<String>,
    /// Path to the worktree checkout.
    pub worktree_path: PathBuf,
    /// Reason this row was selected.
    pub reason: CleanupReason,
}

/// Options for `ark cleanup`.
#[derive(Debug, Clone)]
pub struct CleanupOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Restrict to a single slug.
    pub slug: Option<String>,
    /// Remove the listed worktrees instead of just printing them.
    pub apply: bool,
    /// Also delete the backing branch (only meaningful with `apply`).
    pub delete_branch: bool,
    /// Force removal of dirty worktrees and force-delete unmerged branches.
    pub force: bool,
}

impl CleanupOptions {
    /// Creates dry-run options for `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            slug: None,
            apply: false,
            delete_branch: false,
            force: false,
        }
    }

    /// Restricts the run to one slug.
    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Switches from dry-run to mutating mode.
    pub fn with_apply(mut self, apply: bool) -> Self {
        self.apply = apply;
        self
    }

    /// Sets `--delete-branch`.
    pub fn with_delete_branch(mut self, delete_branch: bool) -> Self {
        self.delete_branch = delete_branch;
        self
    }

    /// Sets `--force`.
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

/// Outcome of `ark cleanup`.
#[derive(Debug, Clone, Default)]
pub struct CleanupSummary {
    /// `true` when the run was a dry-run.
    pub dry_run: bool,
    /// Rows surfaced by enumeration. Populated in both modes.
    pub planned: Vec<CleanupRow>,
    /// Per-row removal outcomes; populated only on `apply`.
    pub successes: Vec<WorktreeCleanupSummary>,
    /// Per-row removal failures; populated only on `apply`.
    pub failures: Vec<(String, String)>,
}

impl fmt::Display for CleanupSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dry_run {
            if self.planned.is_empty() {
                return write!(f, "ark cleanup: nothing to prune");
            }
            writeln!(
                f,
                "ark cleanup --dry-run: {} candidate(s)",
                self.planned.len()
            )?;
            for row in &self.planned {
                let branch = row.branch.as_deref().unwrap_or("(unknown)");
                writeln!(
                    f,
                    "  {} {} [{}] {}",
                    row.slugs.join(","),
                    branch,
                    row.reason,
                    row.worktree_path.display()
                )?;
            }
            return Ok(());
        }

        if self.planned.is_empty() {
            return write!(f, "ark cleanup: nothing to prune");
        }
        writeln!(
            f,
            "ark cleanup: {} removed, {} failed",
            self.successes.len(),
            self.failures.len()
        )?;
        for s in &self.successes {
            writeln!(f, "  {s}")?;
        }
        for (slug, err) in &self.failures {
            writeln!(f, "  fail {slug}: {err}")?;
        }
        Ok(())
    }
}

/// Enumerates prunable worktrees and, on `apply`, removes them.
///
/// Per-row failures during `apply` are collected in `summary.failures`; the
/// loop never aborts. Enumeration errors propagate before any mutation.
///
/// # Errors
///
/// - [`Error::Io`] from filesystem walks of `tasks/archive/`.
/// - [`Error::GitSpawn`] / [`Error::Io`] from `git worktree list` / `git branch`.
/// - [`Error::WorktreeConfigCorrupt`] when `.ark/config.toml`'s `[worktree]`
///   section fails to parse.
pub fn cleanup(opts: CleanupOptions) -> Result<CleanupSummary> {
    let layout = Layout::new(&opts.project_root);
    let cfg = WorktreeConfig::load_or_default(&layout)?;
    let worktrees_dir = cfg.resolve_worktrees_dir(&layout);

    let archived = enumerate_archived(&layout)?;
    let local_branches = enumerate_local_branches(layout.root())?;

    let planned = enumerate_candidates(
        &layout,
        &worktrees_dir,
        &archived,
        &local_branches,
        opts.slug.as_deref(),
    )?;

    if !opts.apply {
        return Ok(CleanupSummary {
            dry_run: true,
            planned,
            ..Default::default()
        });
    }

    let mut summary = CleanupSummary {
        dry_run: false,
        planned: planned.clone(),
        ..Default::default()
    };
    for row in planned {
        let result = worktree_cleanup(WorktreeCleanupOptions {
            project_root: opts.project_root.clone(),
            slug: row.primary_slug.clone(),
            delete_branch: opts.delete_branch,
            force: opts.force,
        });
        match result {
            Ok(s) => summary.successes.push(s),
            Err(e) => summary.failures.push((row.primary_slug, e.to_string())),
        }
    }
    Ok(summary)
}

/// Walks `.ark/tasks/archive/<YYYY-MM>/<tier>/<slug>/` and returns the slug set.
///
/// Reads directory names only — no `task.toml` parse needed for membership.
fn enumerate_archived(layout: &Layout) -> Result<HashSet<String>> {
    let archive_dir = layout.tasks_archive_dir();
    if !archive_dir.is_dir() {
        return Ok(HashSet::new());
    }
    let mut out = HashSet::new();
    for month_entry in archive_dir.list_dir()? {
        let month_path = month_entry
            .map_err(|e| crate::error::Error::io(&archive_dir, e))?
            .path();
        if !month_path.is_dir() {
            continue;
        }
        for tier_entry in month_path.list_dir()? {
            let tier_path = tier_entry
                .map_err(|e| crate::error::Error::io(&month_path, e))?
                .path();
            if !tier_path.is_dir() {
                continue;
            }
            for slug_entry in tier_path.list_dir()? {
                let slug_path = slug_entry
                    .map_err(|e| crate::error::Error::io(&tier_path, e))?
                    .path();
                if !slug_path.is_dir() {
                    continue;
                }
                if let Some(name) = slug_path.file_name().and_then(|n| n.to_str()) {
                    out.insert(name.to_string());
                }
            }
        }
    }
    Ok(out)
}

/// Returns the local branch set via `git branch --list --format=%(refname:short)`.
///
/// On non-zero git exit (e.g. non-git working tree) returns an empty set —
/// branch-gone classification then never fires, matching the worktree feature's
/// "soft-fail when not a git repo" behaviour from `parse_git_worktree_list`.
fn enumerate_local_branches(repo_root: &Path) -> Result<HashSet<String>> {
    let out = run_git(
        &["branch", "--list", "--format=%(refname:short)"],
        repo_root,
    )?;
    if !out.is_success() {
        return Ok(HashSet::new());
    }
    Ok(out
        .stdout
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect())
}

/// Builds the prunable-row list, **one row per worktree**.
///
/// Reads the *worktree's own* `task.toml` for each slug — worktree-backed
/// tasks live entirely inside their worktree, so the parent's `tasks/` is
/// empty for them. A single worktree may carry several active slugs
/// (multiple tasks created and committed inside the same checkout); they
/// are grouped into one [`CleanupRow`] so the user removes one worktree,
/// not N.
///
/// Reason is picked by priority across the worktree's prunable slugs:
/// `Archived` > `Committed` > `BranchGone`. Primary slug is the slug whose
/// `task.toml.branch` matches the worktree's branch when one such slug
/// exists, else the first sorted member of the row's slug set.
fn enumerate_candidates(
    layout: &Layout,
    worktrees_dir: &Path,
    archived: &HashSet<String>,
    local_branches: &HashSet<String>,
    slug_filter: Option<&str>,
) -> Result<Vec<CleanupRow>> {
    let mut out = Vec::new();
    for entry in parse_git_worktree_list(layout.root())? {
        if !is_under(&entry.path, worktrees_dir) {
            continue;
        }
        // Worktrees with unreadable Ark state are silently skipped (worktree
        // feature C-20: third-party worktrees may live under worktrees_dir).
        let wt_layout = Layout::new(&entry.path);
        let Ok(state) = load_state(&wt_layout) else {
            continue;
        };

        let mut prunable: Vec<(String, CleanupReason, Option<String>)> = Vec::new();
        for slug in state.tasks.active {
            if slug_filter.is_some_and(|f| f != slug) {
                continue;
            }
            let task_dir = wt_layout.task_dir(&slug);
            let Ok(toml) = TaskToml::load(&task_dir) else {
                continue;
            };
            let slug_branch = toml.branch.clone();
            let lookup_branch = slug_branch.clone().or(entry.branch.clone());
            let reason = classify(
                &slug,
                toml.phase,
                lookup_branch.as_deref(),
                archived,
                local_branches,
            );
            if let Some(reason) = reason {
                prunable.push((slug, reason, slug_branch));
            }
        }
        if prunable.is_empty() {
            continue;
        }

        prunable.sort_by(|a, b| a.0.cmp(&b.0));
        let row_reason = prunable
            .iter()
            .map(|(_, r, _)| *r)
            .min_by_key(|r| reason_priority(*r))
            .expect("non-empty");
        let row_branch = prunable
            .iter()
            .find_map(|(_, _, b)| b.clone())
            .or(entry.branch.clone());
        let primary_slug = prunable
            .iter()
            .find(|(_, _, b)| b.as_deref() == row_branch.as_deref())
            .map(|(s, ..)| s.clone())
            .unwrap_or_else(|| prunable[0].0.clone());
        let slugs = prunable.into_iter().map(|(s, ..)| s).collect();

        out.push(CleanupRow {
            primary_slug,
            slugs,
            branch: row_branch,
            worktree_path: entry.path.clone(),
            reason: row_reason,
        });
    }
    Ok(out)
}

/// Lower number = higher priority. Mirrors [`classify`]'s ordering.
fn reason_priority(reason: CleanupReason) -> u8 {
    match reason {
        CleanupReason::Archived => 0,
        CleanupReason::Committed => 1,
        CleanupReason::BranchGone => 2,
    }
}

/// Picks a [`CleanupReason`] for one slug. Returns `None` if the worktree is
/// healthy (active task on a live branch).
///
/// Priority: `Archived` > `Committed` > `BranchGone`.
fn classify(
    slug: &str,
    phase: Phase,
    branch: Option<&str>,
    archived: &HashSet<String>,
    local_branches: &HashSet<String>,
) -> Option<CleanupReason> {
    if archived.contains(slug) {
        return Some(CleanupReason::Archived);
    }
    if phase == Phase::Committed {
        return Some(CleanupReason::Committed);
    }
    if let Some(b) = branch
        && !local_branches.contains(b)
    {
        return Some(CleanupReason::BranchGone);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::agent::{
            state::Tier,
            task::new::{TaskNewOptions, TaskNewWorktree, task_new},
            workspace::{Identity, identity_write},
        },
        io::{PathExt, git::run_git},
    };

    fn init_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        run_git(&["init", "--quiet"], tmp.path()).unwrap();
        run_git(&["config", "user.email", "test@example.com"], tmp.path()).unwrap();
        run_git(&["config", "user.name", "Test"], tmp.path()).unwrap();
        run_git(&["config", "commit.gpgsign", "false"], tmp.path()).unwrap();
        run_git(&["checkout", "-b", "main"], tmp.path()).unwrap();
        tmp.path()
            .join("README.md")
            .write_bytes(b"# repo\n")
            .unwrap();
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(&["commit", "-m", "init", "--quiet"], tmp.path()).unwrap();
        identity_write(tmp.path(), &Identity::new("dev").unwrap()).unwrap();
        tmp
    }

    fn create_worktree_task(root: &Path, slug: &str) {
        task_new(TaskNewOptions {
            project_root: root.to_path_buf(),
            slug: slug.into(),
            title: format!("{slug} task"),
            tier: Tier::Standard,
            worktree: Some(TaskNewWorktree::default()),
        })
        .unwrap();
    }

    /// Drives a worktree-backed slug to `phase = Committed` by mutating its
    /// own `task.toml` directly. The phase-transition machinery requires
    /// passing through Plan → Execute → Verify → Committed, which is too
    /// heavy for a unit test of cleanup classification.
    fn force_committed_in_worktree(root: &Path, slug: &str) {
        let wt = root.join(".ark/worktrees/feat").join(slug);
        let task_dir = wt.join(".ark/tasks").join(slug);
        let mut toml = TaskToml::load(&task_dir).unwrap();
        toml.phase = Phase::Committed;
        toml.committed_at = Some(chrono::Utc::now());
        toml.save(&task_dir).unwrap();
    }

    /// Commits the scaffolded worktree contents so cleanup without `--force`
    /// passes the dirty check downstream.
    fn commit_in_worktree(root: &Path, slug: &str) {
        let wt = root.join(".ark/worktrees/feat").join(slug);
        run_git(&["add", "."], &wt).unwrap();
        run_git(&["commit", "-m", "scaffold", "--quiet"], &wt).unwrap();
    }

    /// Active worktree task → no row.
    #[test]
    fn classify_active_returns_none() {
        let archived = HashSet::new();
        let branches: HashSet<String> = ["feat/foo".into()].into_iter().collect();
        assert_eq!(
            classify("foo", Phase::Plan, Some("feat/foo"), &archived, &branches),
            None
        );
    }

    /// Committed phase → Committed reason.
    #[test]
    fn classify_committed() {
        let archived = HashSet::new();
        let branches: HashSet<String> = ["feat/foo".into()].into_iter().collect();
        assert_eq!(
            classify(
                "foo",
                Phase::Committed,
                Some("feat/foo"),
                &archived,
                &branches
            ),
            Some(CleanupReason::Committed)
        );
    }

    /// Archived precedence wins over a Committed phase.
    #[test]
    fn classify_archived_over_committed() {
        let archived: HashSet<String> = ["foo".into()].into_iter().collect();
        let branches: HashSet<String> = ["feat/foo".into()].into_iter().collect();
        assert_eq!(
            classify(
                "foo",
                Phase::Committed,
                Some("feat/foo"),
                &archived,
                &branches
            ),
            Some(CleanupReason::Archived)
        );
    }

    /// Missing branch → BranchGone (when not committed/archived).
    #[test]
    fn classify_branch_gone() {
        let archived = HashSet::new();
        let branches: HashSet<String> = HashSet::new();
        assert_eq!(
            classify("foo", Phase::Plan, Some("feat/foo"), &archived, &branches),
            Some(CleanupReason::BranchGone)
        );
    }

    /// No branch field with active phase → not prunable.
    #[test]
    fn classify_no_branch_active_returns_none() {
        let archived = HashSet::new();
        let branches: HashSet<String> = HashSet::new();
        assert_eq!(
            classify("foo", Phase::Plan, None, &archived, &branches),
            None
        );
    }

    /// Empty repo: nothing to prune.
    #[test]
    fn cleanup_dry_run_empty_repo_is_nothing_to_prune() {
        let tmp = init_repo();
        let summary = cleanup(CleanupOptions::new(tmp.path())).unwrap();
        assert!(summary.dry_run);
        assert!(summary.planned.is_empty());
        assert_eq!(summary.to_string(), "ark cleanup: nothing to prune");
    }

    /// Active worktree task is not surfaced.
    #[test]
    fn cleanup_dry_run_skips_active_task() {
        let tmp = init_repo();
        create_worktree_task(tmp.path(), "alpha");

        let summary = cleanup(CleanupOptions::new(tmp.path())).unwrap();
        assert!(summary.dry_run);
        assert!(
            summary.planned.is_empty(),
            "active task surfaced: {:?}",
            summary.planned
        );
    }

    /// Committed worktree task surfaces with reason `Committed`.
    #[test]
    fn cleanup_dry_run_surfaces_committed() {
        let tmp = init_repo();
        create_worktree_task(tmp.path(), "alpha");
        force_committed_in_worktree(tmp.path(), "alpha");

        let summary = cleanup(CleanupOptions::new(tmp.path())).unwrap();
        let rows: Vec<_> = summary
            .planned
            .iter()
            .map(|r| (r.primary_slug.clone(), r.reason))
            .collect();
        assert!(
            rows.iter()
                .any(|(s, r)| s == "alpha" && *r == CleanupReason::Committed),
            "expected committed alpha; got {rows:?}"
        );
    }

    /// `--slug <s>` filters out other prunable rows.
    #[test]
    fn cleanup_slug_filter_narrows() {
        let tmp = init_repo();
        create_worktree_task(tmp.path(), "alpha");
        create_worktree_task(tmp.path(), "beta");
        force_committed_in_worktree(tmp.path(), "alpha");
        force_committed_in_worktree(tmp.path(), "beta");

        let summary = cleanup(CleanupOptions::new(tmp.path()).with_slug("alpha")).unwrap();
        let primaries: Vec<&str> = summary
            .planned
            .iter()
            .map(|r| r.primary_slug.as_str())
            .collect();
        assert_eq!(primaries, vec!["alpha"]);
        assert_eq!(summary.planned[0].slugs, vec!["alpha".to_string()]);
    }

    /// One row per worktree even when several active slugs share it.
    ///
    /// Reproduces the bug observed in the wild after multiple committed-but-
    /// not-yet-archived tasks were authored inside the same deep-tier
    /// worktree: the worktree's own `state.tasks.active` lists every slug,
    /// each `task.toml` reports `phase = Committed`, but only one worktree
    /// dir exists. The summary must reflect that — one row, with all slugs
    /// grouped, and `--apply` invokes `worktree_cleanup` exactly once.
    #[test]
    fn cleanup_groups_multiple_slugs_under_one_worktree_path() {
        let tmp = init_repo();
        // First task creates the worktree at .ark/worktrees/feat/alpha.
        create_worktree_task(tmp.path(), "alpha");
        force_committed_in_worktree(tmp.path(), "alpha");

        // Author two extra tasks *inside* the alpha worktree. The CLI does
        // this implicitly when the user runs `task new` from inside the
        // worktree on later iterations of the same branch.
        let alpha_wt = tmp.path().join(".ark/worktrees/feat/alpha");
        for slug in ["bravo", "charlie"] {
            task_new(TaskNewOptions {
                project_root: alpha_wt.clone(),
                slug: slug.into(),
                title: format!("{slug} task"),
                tier: Tier::Standard,
                worktree: None,
            })
            .unwrap();
            // Drive each follow-up task to Committed in alpha's task dir.
            let task_dir = alpha_wt.join(".ark/tasks").join(slug);
            let mut t = TaskToml::load(&task_dir).unwrap();
            t.phase = Phase::Committed;
            t.committed_at = Some(chrono::Utc::now());
            t.save(&task_dir).unwrap();
        }

        let summary = cleanup(CleanupOptions::new(tmp.path())).unwrap();
        assert_eq!(
            summary.planned.len(),
            1,
            "expected one row per worktree; got {:?}",
            summary.planned
        );
        let row = &summary.planned[0];
        assert_eq!(
            row.slugs,
            vec![
                "alpha".to_string(),
                "bravo".to_string(),
                "charlie".to_string()
            ]
        );
        // Primary is the slug whose `task.toml.branch` matches the worktree
        // branch — only `alpha` has `branch = "feat/alpha"`.
        assert_eq!(row.primary_slug, "alpha");
        assert_eq!(row.reason, CleanupReason::Committed);
        assert_eq!(row.branch.as_deref(), Some("feat/alpha"));
    }

    /// `--apply` removes a Committed worktree dir; rerun shows nothing.
    #[test]
    fn cleanup_apply_removes_committed_worktree() {
        let tmp = init_repo();
        create_worktree_task(tmp.path(), "alpha");
        force_committed_in_worktree(tmp.path(), "alpha");
        commit_in_worktree(tmp.path(), "alpha");

        let wt = tmp.path().join(".ark/worktrees/feat/alpha");
        assert!(wt.is_dir());

        let summary = cleanup(
            CleanupOptions::new(tmp.path())
                .with_apply(true)
                .with_force(true),
        )
        .unwrap();
        assert!(!summary.dry_run);
        assert_eq!(summary.successes.len(), 1);
        assert!(summary.failures.is_empty(), "{:?}", summary.failures);
        assert!(!wt.exists());

        // Second run: nothing to prune.
        let again = cleanup(CleanupOptions::new(tmp.path())).unwrap();
        assert!(again.planned.is_empty());
    }

    /// `--apply --delete-branch --force` deletes the branch as well.
    #[test]
    fn cleanup_apply_delete_branch() {
        let tmp = init_repo();
        create_worktree_task(tmp.path(), "alpha");
        force_committed_in_worktree(tmp.path(), "alpha");
        commit_in_worktree(tmp.path(), "alpha");

        cleanup(
            CleanupOptions::new(tmp.path())
                .with_apply(true)
                .with_delete_branch(true)
                .with_force(true),
        )
        .unwrap();

        let branches = run_git(&["branch", "--list", "feat/alpha"], tmp.path())
            .unwrap()
            .stdout;
        assert!(branches.trim().is_empty(), "branches: {branches:?}");
    }

    /// Per-row failure on `--apply` is collected without aborting.
    ///
    /// Drives one worktree dirty (the scaffolded but uncommitted state) so
    /// `worktree_cleanup` returns `WorktreeDirty`; the loop continues on the
    /// other slug.
    #[test]
    fn cleanup_apply_collects_per_row_failure() {
        let tmp = init_repo();

        // dirty: scaffolded files plus the `task.toml` mutation are
        // uncommitted, so cleanup without `--force` trips WorktreeDirty.
        create_worktree_task(tmp.path(), "dirty");
        force_committed_in_worktree(tmp.path(), "dirty");

        // clean: commit AFTER mutating `task.toml` to phase=Committed so the
        // worktree is clean when cleanup runs.
        create_worktree_task(tmp.path(), "clean");
        force_committed_in_worktree(tmp.path(), "clean");
        commit_in_worktree(tmp.path(), "clean");

        let summary = cleanup(CleanupOptions::new(tmp.path()).with_apply(true)).unwrap();
        assert_eq!(summary.successes.len(), 1, "{:?}", summary.successes);
        assert_eq!(summary.failures.len(), 1, "{:?}", summary.failures);
        assert_eq!(summary.failures[0].0, "dirty");
    }

    /// Walks the `<month>/<tier>/<slug>` archive layout and returns each slug
    /// regardless of which tier bucket it lives in.
    #[test]
    fn enumerate_archived_reads_tier_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let deep = layout
            .tasks_archive_dir()
            .join("2026-05")
            .join("deep")
            .join("alpha");
        deep.ensure_dir().unwrap();
        let quick = layout
            .tasks_archive_dir()
            .join("2026-04")
            .join("quick")
            .join("beta");
        quick.ensure_dir().unwrap();

        let set = enumerate_archived(&layout).unwrap();
        assert!(set.contains("alpha"), "deep-tier slug missing: {set:?}");
        assert!(set.contains("beta"), "quick-tier slug missing: {set:?}");
        assert_eq!(set.len(), 2);
    }

    /// Source-scan invariant.
    #[test]
    fn cleanup_source_no_bare_std_fs_or_dot_path_literals() {
        crate::commands::tests_common::assert_source_clean(include_str!("cleanup.rs"));
    }
}
