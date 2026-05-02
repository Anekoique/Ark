//! `ark agent task archive` (rename) — moves a committed task to the archive.
//!
//! After the workflow refactor, archive is **side-effect-free**: it performs
//! only directory rename + `task.toml` phase update + state-file cleanup.
//! SPEC promotion happens earlier, in `task_commit`,
//! and live in the task-closing commit. Bulk archive (`ark archive`,
//! top-level CLI) iterates this helper across every `phase = Committed`
//! task.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use chrono::Utc;

use crate::{
    commands::agent::state::{Phase, TaskToml, Tier, check_transition, validate_slug},
    error::{Error, Result},
    io::{PathExt, git::run_git, read_managed_block, update_managed_block, write_atomic},
    layout::{Layout, SESSIONS_MARKER},
    session::ppid::{Ppid, RealPpid},
    state::clear_focus_for_slug,
};

/// Options for moving a committed task to the archive directory.
#[derive(Debug, Clone)]
pub struct TaskArchiveMoveOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Slug of the task to archive.
    pub slug: String,
    /// `YYYY-MM` archive bucket. Caller derives this from the task's
    /// `committed_at` timestamp; passing `Utc::now()`-based values would
    /// place historical tasks in the wrong month.
    pub archive_month: String,
}

/// Summary of a single-task archive move.
#[derive(Debug, Clone)]
pub struct TaskArchiveMoveSummary {
    /// Slug of the archived task.
    pub slug: String,
    /// Workflow tier of the archived task.
    pub tier: Tier,
    /// Final on-disk path of the archived task directory.
    pub archive_path: PathBuf,
}

impl fmt::Display for TaskArchiveMoveSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "archived `{}` ({:?}) -> {}",
            self.slug,
            self.tier,
            self.archive_path.display()
        )
    }
}

/// Moves a committed task into `tasks/archive/<archive_month>/<slug>/`.
///
/// Performs no SPEC promotion — that happens at
/// `task_commit` time and are already part of the task-closing commit.
///
/// # Errors
///
/// Returns [`Error::TaskNotFound`] if the slug has no active task directory,
/// [`Error::IllegalPhaseTransition`] if the task is not in `Committed`, and
/// [`Error::TaskAlreadyExists`] if a same-slug archive entry already exists
/// in the destination month.
pub fn task_archive_move(opts: TaskArchiveMoveOptions) -> Result<TaskArchiveMoveSummary> {
    task_archive_move_with_ppid(opts, &RealPpid::new())
}

/// Test seam for [`task_archive_move`]: same flow with an injectable PPID provider.
pub(crate) fn task_archive_move_with_ppid(
    opts: TaskArchiveMoveOptions,
    ppid: &dyn Ppid,
) -> Result<TaskArchiveMoveSummary> {
    validate_slug(&opts.slug)?;

    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);
    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    let mut toml = TaskToml::load(&task_dir)?;
    check_transition(toml.tier, toml.phase, Phase::Archived)?;
    let tier = toml.tier;

    let archive_parent = layout.tasks_archive_dir().join(&opts.archive_month);
    archive_parent.ensure_dir()?;
    let archive_path = archive_parent.join(&opts.slug);
    if archive_path.exists() {
        return Err(Error::TaskAlreadyExists {
            slug: format!("archive/{}/{}", opts.archive_month, opts.slug),
        });
    }

    // Patch the workspace journal's `<PENDING:<slug>>` sentinel with the
    // resolved closing-commit short SHA, and re-render the Git Commits
    // table now that the closing commit's range is known. Idempotent on
    // re-archive (sentinel already gone). Quietly skips when the task has
    // no `journal_path` (legacy, --no-commit, identity-missing).
    if let Some(journal_rel) = toml.journal_path.as_deref() {
        patch_workspace_slot(
            &layout,
            journal_rel,
            &opts.slug,
            toml.start_head.as_deref(),
        )?;
    }

    // Clear state-file references *before* rename so the focused-session
    // probe still sees the live entry. If a later step fails, two-way
    // reconcile re-adds the slug from the surviving `tasks/<slug>/` dir
    // (the rename has not happened yet).
    clear_focus_for_slug(&layout, ppid, &opts.slug)?;

    task_dir.rename_to(&archive_path)?;

    let now = Utc::now();
    toml.phase = Phase::Archived;
    toml.archived_at = Some(now);
    toml.updated_at = now;
    toml.save(&archive_path)?;

    Ok(TaskArchiveMoveSummary {
        slug: opts.slug,
        tier,
        archive_path,
    })
}

/// Patches the journal's `<PENDING:<slug>>` sentinel with the resolved
/// closing-commit short SHA, re-renders the `### Git Commits` table over
/// the now-known `start_head..closing_sha` range, and patches the matching
/// personal-index row in lockstep.
///
/// The Git Commits table is empty at `task_commit` time because the
/// closing commit hasn't happened yet; archive is the first place both
/// ends of the range exist.
///
/// Idempotent: returns `Ok(())` when the sentinel is already absent
/// (re-archived task, or manual entry that never had a sentinel). Errors
/// with `SlotResolveNoMatch`, `SlotResolveAmbiguous`, or `JournalMissing`.
fn patch_workspace_slot(
    layout: &Layout,
    journal_rel: &str,
    slug: &str,
    start_head: Option<&str>,
) -> Result<()> {
    let journal_path = layout.root().join(journal_rel);
    if !journal_path.exists() {
        return Err(Error::JournalMissing { path: journal_path });
    }

    let sentinel = format!("<PENDING:{slug}>");
    let journal_text = journal_path.read_text()?;
    if !journal_text.contains(&sentinel) {
        return Ok(());
    }

    let (full_sha, short) = resolve_closing_sha(layout.root(), &journal_path, slug)?;
    let with_sha = journal_text.replace(&sentinel, &short);
    let final_text = match start_head {
        Some(start) => {
            let commits = collect_commits_in_range(layout.root(), start, &full_sha);
            replace_git_commits_table(&with_sha, slug, &commits)
        }
        None => with_sha,
    };
    write_atomic(&journal_path, final_text.as_bytes())?;

    if let Some(dev_dir) = journal_path.parent() {
        let personal_index = dev_dir.join("index.md");
        if personal_index.exists()
            && let Some(body) = read_managed_block(&personal_index, SESSIONS_MARKER)?
            && body.contains(&sentinel)
        {
            update_managed_block(
                &personal_index,
                SESSIONS_MARKER,
                &body.replace(&sentinel, &short),
            )?;
        }
    }
    Ok(())
}

/// Resolves the closing commit SHA via slug-anchored pickaxe.
///
/// Returns `(full_sha, short_sha)`. Runs `git log -S '**Slug**: <slug>'
/// --format=%H -- <journal>` (no `-n` cap) and classifies the match count:
/// 0 → `SlotResolveNoMatch`, >1 → `SlotResolveAmbiguous`, exactly 1 →
/// `git rev-parse --short=12` for the link-friendly form.
fn resolve_closing_sha(
    project_root: &Path,
    journal_path: &Path,
    slug: &str,
) -> Result<(String, String)> {
    let pickaxe = format!("-S**Slug**: {slug}");
    let journal_str = journal_path.to_string_lossy();
    let out = run_git(
        &["log", &pickaxe, "--format=%H", "--", &journal_str],
        project_root,
    )?;
    let no_match = || Error::SlotResolveNoMatch {
        slug: slug.to_string(),
        journal: journal_path.to_path_buf(),
    };
    if !out.is_success() {
        return Err(no_match());
    }
    let candidates: Vec<String> = out
        .stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    match candidates.as_slice() {
        [] => Err(no_match()),
        [full] => Ok((full.clone(), short_sha(project_root, full))),
        _ => Err(Error::SlotResolveAmbiguous {
            slug: slug.to_string(),
            candidates,
        }),
    }
}

/// Returns the 12-char short SHA for `full`, falling back to a hard truncate
/// when `git rev-parse` fails.
fn short_sha(project_root: &Path, full: &str) -> String {
    run_git(&["rev-parse", "--short=12", full], project_root)
        .ok()
        .filter(|out| out.is_success())
        .map(|out| out.stdout.trim().to_string())
        .unwrap_or_else(|| full.chars().take(12).collect())
}

/// Returns `(short_sha, subject)` rows for `start..end` in commit order
/// (newest first, matching `git log` default).
fn collect_commits_in_range(project_root: &Path, start: &str, end: &str) -> Vec<(String, String)> {
    let range = format!("{start}..{end}");
    let out = match run_git(&["log", &range, "--format=%h %s"], project_root) {
        Ok(o) if o.is_success() => o,
        _ => return Vec::new(),
    };
    out.stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let sha = parts.next()?.to_string();
            let subject = parts.next().unwrap_or("").to_string();
            Some((sha, subject))
        })
        .collect()
}

/// Rewrites the `### Git Commits` table for the entry whose `**Slug**:
/// <slug>` line we just patched. The block runs from the literal
/// `### Git Commits\n` heading to the next `## ` or `### ` heading (or
/// end-of-file).
///
/// If no such block exists in `text`, returns `text` unchanged — the
/// entry was a manual record (no Git Commits section) or the agent
/// stripped the heading.
fn replace_git_commits_table(text: &str, slug: &str, commits: &[(String, String)]) -> String {
    let slug_anchor = format!("**Slug**: {slug}");
    let Some(slug_pos) = text.find(&slug_anchor) else {
        return text.to_string();
    };
    let after_slug = &text[slug_pos..];
    let Some(rel_heading) = after_slug.find("### Git Commits") else {
        return text.to_string();
    };
    let heading_pos = slug_pos + rel_heading;
    let after_heading = &text[heading_pos..];
    // First line is the heading itself; scan for the next section break.
    let scan = match after_heading.find('\n') {
        Some(nl) => &after_heading[nl + 1..],
        None => "",
    };
    let next_section = scan
        .find("\n## ")
        .or_else(|| scan.find("\n### "))
        .map(|i| heading_pos + after_heading.find('\n').unwrap() + 1 + i + 1)
        .unwrap_or(text.len());

    let new_block = render_commits_block(commits);
    let mut out = String::with_capacity(text.len() + new_block.len());
    out.push_str(&text[..heading_pos]);
    out.push_str(&new_block);
    if next_section < text.len() {
        out.push_str(&text[next_section..]);
    }
    out
}

fn render_commits_block(commits: &[(String, String)]) -> String {
    let mut s = String::from("### Git Commits\n\n| Hash | Message |\n|------|---------|\n");
    if commits.is_empty() {
        s.push_str("| _(none)_ |   |\n");
    } else {
        for (sha, subject) in commits {
            s.push_str(&format!("| `{sha}` | {subject} |\n"));
        }
    }
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::task::{
        new::{TaskNewOptions, task_new},
        phase::{TaskPhaseOptions, task_execute, task_plan, task_verify},
    };

    /// Drives a fresh standard-tier task all the way to `Committed` so archive
    /// has something legal to move. The git plumbing isn't real here — we
    /// short-circuit by hand-saving `phase = Committed` for tests that only
    /// need the precondition met.
    fn standard_at_committed(tmp_path: &std::path::Path) {
        task_new(TaskNewOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_verify(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        // Force phase=Committed without going through task_commit (which would
        // require a real git repo + staged work). committed_at is set so
        // ark_archive can derive the month later if a test wants.
        let layout = Layout::new(tmp_path);
        let task_dir = layout.task_dir("demo");
        let mut toml = TaskToml::load(&task_dir).unwrap();
        toml.phase = Phase::Committed;
        let now = Utc::now();
        toml.committed_at = Some(now);
        toml.updated_at = now;
        toml.save(&task_dir).unwrap();
    }

    fn quick_at_committed(tmp_path: &std::path::Path) {
        task_new(TaskNewOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "qd".into(),
            title: "qd".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "qd".into(),
        })
        .unwrap();
        let layout = Layout::new(tmp_path);
        let task_dir = layout.task_dir("qd");
        let mut toml = TaskToml::load(&task_dir).unwrap();
        toml.phase = Phase::Committed;
        let now = Utc::now();
        toml.committed_at = Some(now);
        toml.updated_at = now;
        toml.save(&task_dir).unwrap();
    }

    #[test]
    fn standard_archive_moves_dir_and_clears_current() {
        let tmp = tempfile::tempdir().unwrap();
        standard_at_committed(tmp.path());

        let s = task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap();
        assert_eq!(s.tier, Tier::Standard);
        assert!(!tmp.path().join(".ark/tasks/demo").exists());
        assert!(s.archive_path.exists());
        assert!(s.archive_path.join("task.toml").exists());
        assert!(
            s.archive_path
                .to_string_lossy()
                .contains(".ark/tasks/archive/2026-05/demo"),
            "archive_month must select the directory: got {}",
            s.archive_path.display()
        );
        let state = crate::state::load_state(&Layout::new(tmp.path()), &RealPpid::new()).unwrap();
        assert!(!state.tasks.active.iter().any(|s| s == "demo"));
    }

    #[test]
    fn archive_twice_errors() {
        let tmp = tempfile::tempdir().unwrap();
        standard_at_committed(tmp.path());
        task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap();
        let err = task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::TaskNotFound { .. }));
    }

    /// Verifies that `Verify → Archived` is no longer a legal transition;
    /// archive must be preceded by `task_commit`.
    #[test]
    fn archive_from_verify_errors_after_refactor() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        task_verify(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        let err = task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::IllegalPhaseTransition { .. }));
    }

    /// Verifies that archive does not promote a SPEC.
    ///
    /// Deep-tier SPEC promotion happens at `task_commit` time. Any later
    /// archive run must leave `specs/features/INDEX.md` and existing SPEC
    /// files untouched.
    #[test]
    fn archive_writes_no_spec_files() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.specs_features_dir().ensure_dir().unwrap();
        layout
            .specs_features_index()
            .write_bytes(b"# Features\n")
            .unwrap();
        let pre_index = layout.specs_features_index().read_bytes().unwrap();

        // Use a quick-tier task — no deep SPEC to begin with.
        quick_at_committed(tmp.path());
        task_archive_move(TaskArchiveMoveOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            archive_month: "2026-05".into(),
        })
        .unwrap();

        let post_index = layout.specs_features_index().read_bytes().unwrap();
        assert_eq!(
            pre_index, post_index,
            "archive must not modify the features INDEX"
        );
        assert!(
            !layout.specs_feature_dir("qd").exists(),
            "archive must not create a SPEC dir for the slug"
        );
    }
}

#[cfg(test)]
mod slot_patch_tests {
    use super::*;
    use crate::io::{PathExt, git::run_git};

    /// Bootstraps a git repo with one commit that contains a workspace
    /// journal with a `<PENDING:<slug>>` sentinel and a `**Slug**: <slug>`
    /// anchor line. Returns the tempdir + journal relative path.
    fn repo_with_pending_journal(slug: &str) -> (tempfile::TempDir, String) {
        let tmp = tempfile::tempdir().unwrap();
        run_git(&["init", "--quiet"], tmp.path()).unwrap();
        run_git(&["config", "user.email", "test@example.com"], tmp.path()).unwrap();
        run_git(&["config", "user.name", "Test"], tmp.path()).unwrap();
        run_git(&["config", "commit.gpgsign", "false"], tmp.path()).unwrap();
        run_git(&["checkout", "-b", "main"], tmp.path()).unwrap();

        let rel = "ark/workspace/alice/journal-1.md".to_string();
        let abs = tmp.path().join(&rel);
        abs.parent().unwrap().ensure_dir().unwrap();
        let body =
            format!("## Session 1: t\n\n**Slug**: {slug}\n**Closing Commit**: <PENDING:{slug}>\n");
        abs.write_bytes(body.as_bytes()).unwrap();

        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(&["commit", "-m", "feat(t): add", "--quiet"], tmp.path()).unwrap();
        (tmp, rel)
    }

    #[test]
    fn resolve_closing_sha_returns_full_and_short_sha_for_unique_match() {
        let (tmp, rel) = repo_with_pending_journal("workspace");
        let layout = Layout::new(tmp.path());
        let journal = layout.root().join(&rel);
        let (full, short) = resolve_closing_sha(layout.root(), &journal, "workspace").unwrap();
        assert_eq!(full.len(), 40);
        assert!(full.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(short.len(), 12);
        assert!(full.starts_with(&short));
    }

    #[test]
    fn resolve_closing_sha_errors_on_missing_slug() {
        let (tmp, rel) = repo_with_pending_journal("workspace");
        let layout = Layout::new(tmp.path());
        let journal = layout.root().join(&rel);
        let err = resolve_closing_sha(layout.root(), &journal, "nonexistent").unwrap_err();
        assert!(matches!(err, Error::SlotResolveNoMatch { .. }));
    }

    #[test]
    fn patch_workspace_slot_replaces_sentinel_in_journal() {
        let (tmp, rel) = repo_with_pending_journal("workspace");
        let layout = Layout::new(tmp.path());
        patch_workspace_slot(&layout, &rel, "workspace", None).unwrap();

        let patched = std::fs::read_to_string(layout.root().join(&rel)).unwrap();
        assert!(!patched.contains("<PENDING:workspace>"));
        assert!(patched.contains("**Closing Commit**: "));
    }

    #[test]
    fn patch_workspace_slot_is_idempotent() {
        let (tmp, rel) = repo_with_pending_journal("workspace");
        let layout = Layout::new(tmp.path());
        patch_workspace_slot(&layout, &rel, "workspace", None).unwrap();
        // Second call: sentinel is gone — no-op, no error.
        patch_workspace_slot(&layout, &rel, "workspace", None).unwrap();
    }

    #[test]
    fn patch_workspace_slot_renders_git_commits_table_when_start_head_present() {
        // Build a repo with two intermediate commits between start_head
        // (the scaffold commit) and the final commit that introduces the
        // pending-sentinel journal. The table should list both intermediate
        // commits plus the closing commit.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        run_git(&["init", "--quiet"], root).unwrap();
        run_git(&["config", "user.email", "test@example.com"], root).unwrap();
        run_git(&["config", "user.name", "Test"], root).unwrap();
        run_git(&["config", "commit.gpgsign", "false"], root).unwrap();
        run_git(&["checkout", "-b", "main"], root).unwrap();

        // start_head: scaffold commit (table excludes this).
        root.join("scaffold").write_bytes(b"x").unwrap();
        run_git(&["add", "."], root).unwrap();
        run_git(&["commit", "-m", "scaffold", "--quiet"], root).unwrap();
        let start_head = run_git(&["rev-parse", "HEAD"], root).unwrap().stdout.trim().to_string();

        // Intermediate commits (table includes both).
        root.join("a.rs").write_bytes(b"a").unwrap();
        run_git(&["add", "."], root).unwrap();
        run_git(&["commit", "-m", "feat(x): step 1", "--quiet"], root).unwrap();
        root.join("b.rs").write_bytes(b"b").unwrap();
        run_git(&["add", "."], root).unwrap();
        run_git(&["commit", "-m", "feat(x): step 2", "--quiet"], root).unwrap();

        // Closing commit: the journal write itself, with sentinel + Git
        // Commits placeholder.
        let rel = ".ark/workspace/alice/journal-1.md".to_string();
        let abs = root.join(&rel);
        abs.parent().unwrap().ensure_dir().unwrap();
        let body = "## Session 1: x\n\n**Slug**: workspace\n\
            **Closing Commit**: <PENDING:workspace>\n\n\
            ### Git Commits\n\n| Hash | Message |\n|------|---------|\n| _(none)_ |   |\n";
        abs.write_bytes(body.as_bytes()).unwrap();
        run_git(&["add", "."], root).unwrap();
        run_git(&["commit", "-m", "close", "--quiet"], root).unwrap();

        let layout = Layout::new(root);
        patch_workspace_slot(&layout, &rel, "workspace", Some(&start_head)).unwrap();

        let patched = abs.read_text().unwrap();
        assert!(!patched.contains("<PENDING:workspace>"));
        assert!(patched.contains("feat(x): step 1"));
        assert!(patched.contains("feat(x): step 2"));
        assert!(patched.contains("close"));
        assert!(!patched.contains("_(none)_"));
        // Order: newest first (matches `git log` default).
        let step1 = patched.find("step 1").unwrap();
        let step2 = patched.find("step 2").unwrap();
        let close = patched.find("| close").unwrap();
        assert!(close < step2 && step2 < step1, "newest-first order broken");
    }

    #[test]
    fn patch_workspace_slot_errors_when_journal_missing() {
        let tmp = tempfile::tempdir().unwrap();
        run_git(&["init", "--quiet"], tmp.path()).unwrap();
        let layout = Layout::new(tmp.path());
        let err =
            patch_workspace_slot(&layout, ".ark/workspace/x/journal-1.md", "x", None).unwrap_err();
        assert!(matches!(err, Error::JournalMissing { .. }));
    }
}
