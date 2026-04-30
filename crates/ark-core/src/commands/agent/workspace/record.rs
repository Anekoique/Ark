//! `ark agent workspace record` (manual) + `record_task` (task→workspace bridge).
//!
//! Both entrypoints share the same journal-write path and write to the
//! **current checkout's** `.ark/workspace/<dev>/`. The session entry rides
//! along with the task commit on the same branch.

use std::{fmt, path::PathBuf};

use chrono::{DateTime, Utc};

use super::{
    config::WorkspaceConfig,
    identity::{read_developer_name, require_developer_name},
    index,
    journal::{
        self, JournalCommit, JournalEntry, JournalKind, MAX_JOURNAL_INDEX, find_active, line_count,
        render_entry, scan_session_count, seed_journal,
    },
};
use crate::{
    commands::agent::state::Tier,
    error::{Error, Result},
    io::{PathExt, git::run_git},
    layout::Layout,
};

/// Options for manually recording a workspace session.
#[derive(Debug, Clone)]
pub struct WorkspaceRecordOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Optional session title.
    pub title: Option<String>,
    /// Optional session summary body.
    pub summary: Option<String>,
    /// Optional newline-delimited next-step list.
    pub next: Option<String>,
}

/// Summary of a manual workspace record operation.
#[derive(Debug, Clone)]
pub struct WorkspaceRecordSummary {
    /// Developer identity that received the journal entry.
    pub dev: String,
    /// Path to the journal file that was written.
    pub journal_path: PathBuf,
    /// Journal file index.
    pub journal_index: u32,
    /// Session number assigned to the entry.
    pub session_number: u32,
    /// Reports whether a new journal file was opened.
    pub rotated: bool,
}

impl fmt::Display for WorkspaceRecordSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "recorded session {} for `{}` to {}",
            self.session_number,
            self.dev,
            self.journal_path.display()
        )
    }
}

/// Outcome of `record_task` invoked from `task::archive::task_archive`.
#[derive(Debug, Clone)]
pub enum WorkspaceRecorded {
    /// Session was recorded into a workspace journal.
    Recorded {
        /// Path to the journal file that was written.
        journal_path: PathBuf,
        /// Session number assigned to the entry.
        session_number: u32,
    },
    /// Identity was absent, so task archive skipped workspace recording.
    SkippedNoIdentity,
    /// Workspace config disabled archive auto-recording.
    SkippedDisabled,
}

impl fmt::Display for WorkspaceRecorded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recorded {
                journal_path,
                session_number,
            } => write!(
                f,
                "session {} recorded to {}",
                session_number,
                journal_path.display()
            ),
            Self::SkippedNoIdentity => write!(f, "skipped (no developer set)"),
            Self::SkippedDisabled => {
                write!(f, "skipped (auto_record_on_archive = false)")
            }
        }
    }
}

/// Options for recording an archived task into a workspace journal.
#[derive(Debug, Clone)]
pub struct RecordTaskOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Archived task slug.
    pub slug: String,
    /// Archived task title.
    pub title: String,
    /// Archived task workflow tier.
    pub tier: Tier,
    /// Branch associated with the archived task.
    pub branch: Option<String>,
    /// Base branch captured when the task worktree was created.
    pub base_branch: Option<String>,
    /// Task worktree path, when the task used a worktree.
    pub worktree_path: Option<PathBuf>,
    /// Path where the task was archived.
    pub archive_path: PathBuf,
    /// Timestamp when the task was archived.
    pub archived_at: DateTime<Utc>,
}

// -- Manual record ---------------------------------------------------------

/// Records a manual workspace session.
pub fn workspace_record(opts: WorkspaceRecordOptions) -> Result<WorkspaceRecordSummary> {
    let layout = Layout::new(&opts.project_root);
    let dev = require_developer_name(&layout)?;
    let cfg = WorkspaceConfig::load_or_default(&layout)?;

    let title = opts.title.unwrap_or_else(|| "(untitled)".to_string());
    let summary = opts.summary.unwrap_or_default();
    let next_steps = parse_next_steps(opts.next.as_deref().unwrap_or(""));

    let cwd = opts.project_root.as_path();
    let branch = collect_current_branch(cwd);
    let commits = collect_commits_recent(cwd);

    write_journal_and_index(
        &layout,
        &dev,
        &cfg,
        title,
        Utc::now(),
        JournalKind::Manual,
        branch,
        summary,
        commits,
        next_steps,
    )
}

/// Parses a `\n`-separated list of next-step bullets.
///
/// Each line may begin with a single leading bullet character (`-` or `*`)
/// followed by whitespace; only that one bullet is stripped (so `"-- literal"`
/// keeps its second dash). Empty lines after stripping are dropped.
fn parse_next_steps(input: &str) -> Vec<String> {
    input
        .lines()
        .map(strip_bullet)
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

fn strip_bullet(line: &str) -> &str {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix('-')
        .or_else(|| trimmed.strip_prefix('*'))
        .map(str::trim_start)
        .unwrap_or(trimmed)
}

// -- task → workspace bridge ----------------------------------------------

/// Records an archived task into the developer workspace journal.
pub fn record_task(opts: RecordTaskOptions) -> Result<WorkspaceRecorded> {
    let layout = Layout::new(&opts.project_root);
    let cfg = WorkspaceConfig::load_or_default(&layout)?;

    if !cfg.auto_record_on_archive {
        return Ok(WorkspaceRecorded::SkippedDisabled);
    }

    let dev = match read_developer_name(&layout)? {
        Some(name) => name,
        None => {
            eprintln!("no developer set; skipping workspace record");
            return Ok(WorkspaceRecorded::SkippedNoIdentity);
        }
    };

    let task_cwd = resolve_task_cwd(&opts, &layout);
    let commits = collect_commits_for_task(&task_cwd, opts.base_branch.as_deref());

    let WorkspaceRecordSummary {
        journal_path,
        session_number,
        ..
    } = write_journal_and_index(
        &layout,
        &dev,
        &cfg,
        opts.title.clone(),
        opts.archived_at,
        JournalKind::Task {
            slug: opts.slug.clone(),
        },
        opts.branch.clone(),
        format!(
            "Archived `{}` ({}). See {} for the task artifacts.",
            opts.slug,
            tier_label(opts.tier),
            opts.archive_path.display()
        ),
        commits,
        Vec::new(),
    )?;

    Ok(WorkspaceRecorded::Recorded {
        journal_path,
        session_number,
    })
}

/// Returns the lowercase tier label matching the serde representation.
fn tier_label(tier: Tier) -> &'static str {
    match tier {
        Tier::Quick => "quick",
        Tier::Standard => "standard",
        Tier::Deep => "deep",
    }
}

/// Returns the working directory for git lookups.
///
/// The fallback chain is the live worktree path if it exists, then the archived
/// task dir, then the project root.
fn resolve_task_cwd(opts: &RecordTaskOptions, layout: &Layout) -> PathBuf {
    if let Some(wt) = &opts.worktree_path
        && wt.is_dir()
    {
        return wt.clone();
    }
    if opts.archive_path.is_dir() {
        return opts.archive_path.clone();
    }
    layout.root().to_path_buf()
}

/// `git log` cap shared across collection sites.
const COMMIT_LOG_CAP: &str = "20";

/// Runs `git log` for workspace journal collection.
///
/// Uses the given range argument, or the bare `-n N` form when `range` is
/// `None`. Returns an empty list on non-zero exit or spawn failure.
fn collect_commits(cwd: &std::path::Path, range: Option<&str>) -> Vec<JournalCommit> {
    let mut args: Vec<&str> = vec!["log"];
    if let Some(r) = range {
        args.push(r);
    }
    args.extend(["--oneline", "-n", COMMIT_LOG_CAP]);
    run_git(&args, cwd)
        .ok()
        .filter(|o| o.is_success())
        .map(|o| journal::parse_oneline(&o.stdout))
        .unwrap_or_default()
}

fn collect_commits_for_task(
    cwd: &std::path::Path,
    base_branch: Option<&str>,
) -> Vec<JournalCommit> {
    let range = base_branch.map(|b| format!("{b}..HEAD"));
    collect_commits(cwd, range.as_deref())
}

fn collect_commits_recent(cwd: &std::path::Path) -> Vec<JournalCommit> {
    collect_commits(cwd, None)
}

fn collect_current_branch(cwd: &std::path::Path) -> Option<String> {
    run_git(&["symbolic-ref", "--short", "HEAD"], cwd)
        .ok()
        .filter(|o| o.is_success())
        .map(|o| o.stdout.trim().to_string())
        .filter(|s| !s.is_empty())
}

// -- Shared write path -----------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn write_journal_and_index(
    layout: &Layout,
    dev: &str,
    cfg: &WorkspaceConfig,
    title: String,
    when: DateTime<Utc>,
    kind: JournalKind,
    branch: Option<String>,
    summary: String,
    commits: Vec<JournalCommit>,
    next_steps: Vec<String>,
) -> Result<WorkspaceRecordSummary> {
    let dev_dir = layout.workspace_developer_dir(dev);
    dev_dir.ensure_dir()?;

    let (active_path, active_n) = find_active(layout, dev)?;
    let needs_rotate = active_path.exists() && line_count(&active_path)? >= cfg.journal_max_lines;

    let (target_path, target_n) = if needs_rotate {
        let n = active_n + 1;
        if n > MAX_JOURNAL_INDEX {
            return Err(Error::JournalRotationLimit {
                dev: dev.to_string(),
                max: MAX_JOURNAL_INDEX,
            });
        }
        (layout.workspace_journal(dev, n), n)
    } else {
        (active_path.clone(), active_n)
    };

    if needs_rotate || !target_path.exists() {
        seed_journal(&target_path, dev, target_n, when.date_naive())?;
    }

    let highest_session = scan_session_count(layout, dev)?;
    let session_number = highest_session + 1;

    let entry = JournalEntry {
        session_number,
        title,
        date: when.date_naive(),
        kind,
        branch,
        summary,
        commits,
        next_steps,
    };
    let rendered = render_entry(&entry);
    target_path.append_text(&rendered)?;

    // Re-render index. Seed if missing (e.g. user rm'd it).
    let index_path = layout.workspace_index(dev);
    if !index_path.exists() {
        super::index::seed_index(&index_path, dev)?;
    }
    index::rerender(layout, dev)?;

    Ok(WorkspaceRecordSummary {
        dev: dev.to_string(),
        journal_path: target_path,
        journal_index: target_n,
        session_number,
        rotated: needs_rotate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::workspace::init::{WorkspaceInitOptions, workspace_init};

    fn init_dev(tmp: &std::path::Path, name: &str) {
        workspace_init(WorkspaceInitOptions {
            project_root: tmp.to_path_buf(),
            name: name.to_string(),
        })
        .unwrap();
    }

    /// Verifies that a manual record without `.developer` returns an error.
    #[test]
    fn workspace_record_no_identity_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = workspace_record(WorkspaceRecordOptions {
            project_root: tmp.path().to_path_buf(),
            title: Some("x".into()),
            summary: None,
            next: None,
        })
        .unwrap_err();
        assert!(matches!(err, Error::DeveloperNotInitialized { .. }));
        assert!(!tmp.path().join(".ark/workspace").exists());
    }

    /// Verifies that `workspace_record` appends and re-renders the index.
    #[test]
    fn workspace_record_appends_to_journal() {
        let tmp = tempfile::tempdir().unwrap();
        init_dev(tmp.path(), "alice");
        let s = workspace_record(WorkspaceRecordOptions {
            project_root: tmp.path().to_path_buf(),
            title: Some("hello".into()),
            summary: Some("did stuff".into()),
            next: Some("- next1\n- next2".into()),
        })
        .unwrap();
        assert_eq!(s.dev, "alice");
        assert_eq!(s.session_number, 1);
        assert!(!s.rotated);

        let layout = Layout::new(tmp.path());
        let journal = layout.workspace_journal("alice", 1).read_text().unwrap();
        assert!(journal.contains("## Session 1: hello"));
        assert!(journal.contains("did stuff"));
        assert!(journal.contains("- next1"));

        let idx = layout.workspace_index("alice").read_text().unwrap();
        assert!(idx.contains("**Total Sessions**: 1"));
        assert!(idx.contains("| 1 | "));
        assert!(idx.contains(" | hello | manual | - | "));
    }

    /// Verifies that the journal rotates when the active file reaches the line cap.
    #[test]
    fn workspace_record_rotates_at_cap() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        // Set a tight cap and pre-fill journal-1.md to exceed it.
        layout
            .config_file()
            .write_bytes(b"[workspace]\njournal_max_lines = 100\nauto_record_on_archive = true\n")
            .unwrap();
        init_dev(tmp.path(), "alice");
        // Append filler lines so line_count >= 100.
        let filler = "x\n".repeat(120);
        layout
            .workspace_journal("alice", 1)
            .append_text(&filler)
            .unwrap();

        let s = workspace_record(WorkspaceRecordOptions {
            project_root: tmp.path().to_path_buf(),
            title: Some("after rotate".into()),
            summary: None,
            next: None,
        })
        .unwrap();
        assert!(s.rotated);
        assert_eq!(s.journal_index, 2);
        assert!(layout.workspace_journal("alice", 2).exists());
        let idx = layout.workspace_index("alice").read_text().unwrap();
        assert!(idx.contains("**Active File**: `journal-2.md`"));
    }

    /// Verifies that `record_task` skips cleanly when identity is missing.
    #[test]
    fn record_task_skips_when_no_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join(".ark/tasks/archive/2026-04/demo");
        archive.ensure_dir().unwrap();
        let outcome = record_task(RecordTaskOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Quick,
            branch: None,
            base_branch: None,
            worktree_path: None,
            archive_path: archive,
            archived_at: Utc::now(),
        })
        .unwrap();
        assert!(matches!(outcome, WorkspaceRecorded::SkippedNoIdentity));
        assert!(!tmp.path().join(".ark/workspace").exists());
    }

    /// Verifies that `record_task` returns `SkippedDisabled` when auto-record is disabled.
    #[test]
    fn record_task_skips_when_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        init_dev(tmp.path(), "alice");
        let layout = Layout::new(tmp.path());
        layout
            .config_file()
            .write_bytes(b"[workspace]\nauto_record_on_archive = false\n")
            .unwrap();
        let archive = tmp.path().join(".ark/tasks/archive/2026-04/demo");
        archive.ensure_dir().unwrap();
        let outcome = record_task(RecordTaskOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
            branch: None,
            base_branch: None,
            worktree_path: None,
            archive_path: archive,
            archived_at: Utc::now(),
        })
        .unwrap();
        assert!(matches!(outcome, WorkspaceRecorded::SkippedDisabled));
    }

    /// Verifies that `record_task` writes a task-kind journal entry on the happy path.
    #[test]
    fn record_task_writes_entry() {
        let tmp = tempfile::tempdir().unwrap();
        init_dev(tmp.path(), "alice");
        let archive = tmp.path().join(".ark/tasks/archive/2026-04/demo");
        archive.ensure_dir().unwrap();
        let outcome = record_task(RecordTaskOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "demo task".into(),
            tier: Tier::Standard,
            branch: Some("feat/demo".into()),
            base_branch: None,
            worktree_path: None,
            archive_path: archive,
            archived_at: Utc::now(),
        })
        .unwrap();
        assert!(matches!(outcome, WorkspaceRecorded::Recorded { .. }));

        let layout = Layout::new(tmp.path());
        let journal = layout.workspace_journal("alice", 1).read_text().unwrap();
        assert!(journal.contains("## Session 1:"));
        assert!(journal.contains("**Kind**: task"));
        assert!(journal.contains("**Slug**: demo"));
        assert!(journal.contains("**Branch**: `feat/demo`"));
        let idx = layout.workspace_index("alice").read_text().unwrap();
        assert!(idx.contains("| 1 | "));
        assert!(idx.contains(" | task | demo | "));
    }

    /// Verifies that a missing `worktree_path` falls back to `archive_path`.
    #[test]
    fn record_task_falls_back_when_worktree_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        init_dev(tmp.path(), "alice");
        let archive = tmp.path().join(".ark/tasks/archive/2026-04/demo");
        archive.ensure_dir().unwrap();
        let outcome = record_task(RecordTaskOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Quick,
            branch: None,
            base_branch: None,
            worktree_path: Some(tmp.path().join("nonexistent_wt")),
            archive_path: archive,
            archived_at: Utc::now(),
        })
        .unwrap();
        assert!(matches!(outcome, WorkspaceRecorded::Recorded { .. }));
    }
}
