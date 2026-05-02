//! `ark archive` — bulk-archive every committed task into its month bucket.
//!
//! Manager-only (visible) top-level CLI. Iterates `.ark/tasks/<slug>/` for
//! every entry with `phase = Committed && committed_at = Some(...)` and
//! invokes [`task_archive_move`] per slug, deriving the `YYYY-MM` bucket
//! from the task's own `committed_at` timestamp. **No SPEC promotion. No
//! journal recording.** Both already happened at `task_commit` time.

use std::{fmt, path::PathBuf};

use crate::{
    commands::agent::{
        state::{Phase, TaskToml},
        task::archive::{TaskArchiveMoveOptions, TaskArchiveMoveSummary, task_archive_move},
    },
    error::{Error, Result},
    layout::Layout,
};

/// Options for `ark archive`.
#[derive(Debug, Clone)]
pub struct ArchiveOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Restrict archival to tasks whose `committed_at` falls in `YYYY-MM`.
    pub month: Option<String>,
    /// List candidates without performing any move.
    pub dry_run: bool,
}

/// Outcome of a bulk-archive run.
#[derive(Debug, Clone, Default)]
pub struct ArchiveSummary {
    /// Successfully archived tasks.
    pub successes: Vec<TaskArchiveMoveSummary>,
    /// Per-slug failures, with the original error preserved.
    pub failures: Vec<(String, String)>,
    /// Slugs reported by `--dry-run` (`(slug, archive_path_string)`).
    pub planned: Vec<(String, String)>,
    /// True iff this run was `--dry-run`.
    pub dry_run: bool,
}

impl fmt::Display for ArchiveSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dry_run {
            writeln!(
                f,
                "ark archive --dry-run: {} candidate(s)",
                self.planned.len()
            )?;
            for (slug, path) in &self.planned {
                writeln!(f, "  {slug} -> {path}")?;
            }
            return Ok(());
        }
        writeln!(
            f,
            "ark archive: {} archived, {} failed",
            self.successes.len(),
            self.failures.len(),
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

/// Bulk-archives every `phase = Committed` task whose month matches.
///
/// Errors during enumeration propagate; per-slug move errors are collected
/// in `summary.failures` so a single bad task does not block the rest.
pub fn ark_archive(opts: ArchiveOptions) -> Result<ArchiveSummary> {
    let layout = Layout::new(&opts.project_root);
    let candidates = enumerate_committed(&layout)?;

    let mut summary = ArchiveSummary {
        dry_run: opts.dry_run,
        ..ArchiveSummary::default()
    };

    for (slug, archive_month) in candidates {
        if let Some(filter) = &opts.month
            && &archive_month != filter
        {
            continue;
        }
        if opts.dry_run {
            let dest = layout.tasks_archive_dir().join(&archive_month).join(&slug);
            summary
                .planned
                .push((slug, dest.to_string_lossy().into_owned()));
            continue;
        }
        match task_archive_move(TaskArchiveMoveOptions {
            project_root: opts.project_root.clone(),
            slug: slug.clone(),
            archive_month,
        }) {
            Ok(s) => summary.successes.push(s),
            Err(e) => summary.failures.push((slug, e.to_string())),
        }
    }

    Ok(summary)
}

/// Walks `.ark/tasks/` for `phase = Committed` tasks with a `committed_at`
/// month, returning `(slug, "YYYY-MM")` pairs.
///
/// Tasks with `phase = Committed && committed_at = None` are pushed onto the
/// caller-visible failure list via a dedicated [`Error::CommittedAtMissing`]
/// surfaced by the next mutation; here we silently skip them so the
/// enumeration itself is total. Non-`Committed` phases are skipped without
/// comment.
fn enumerate_committed(layout: &Layout) -> Result<Vec<(String, String)>> {
    let tasks_dir = layout.tasks_dir();
    if !tasks_dir.is_dir() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(&tasks_dir).map_err(|e| Error::io(&tasks_dir, e))?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Skip the archive subdirectory itself.
        if path.file_name().map(|n| n == "archive").unwrap_or(false) {
            continue;
        }
        let toml_path = path.join("task.toml");
        if !toml_path.exists() {
            continue;
        }
        let toml = TaskToml::load(&path)?;
        if toml.phase != Phase::Committed {
            continue;
        }
        let Some(committed_at) = toml.committed_at else {
            // Inconsistent state; skip silently here. The CLI surfaces
            // CommittedAtMissing via `task_archive_move` when the user
            // names the slug explicitly; bulk archive doesn't pretend to
            // archive an unknown month.
            eprintln!(
                "warn: task `{}` has phase=Committed but committed_at is missing; skipping in ark \
                 archive",
                toml.id
            );
            continue;
        };
        out.push((toml.id, committed_at.format("%Y-%m").to_string()));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::{
        commands::agent::{
            state::Tier,
            task::{
                new::{TaskNewOptions, task_new},
                phase::{TaskPhaseOptions, task_execute},
            },
        },
        io::PathExt,
    };

    /// Hand-flips a task to `Committed` with the given commit timestamp.
    fn force_committed_at(tmp: &std::path::Path, slug: &str, when: chrono::DateTime<Utc>) {
        let layout = Layout::new(tmp);
        let task_dir = layout.task_dir(slug);
        let mut toml = TaskToml::load(&task_dir).unwrap();
        toml.phase = Phase::Committed;
        toml.committed_at = Some(when);
        toml.updated_at = when;
        toml.save(&task_dir).unwrap();
    }

    fn quick_task(tmp: &std::path::Path, slug: &str) {
        task_new(TaskNewOptions {
            project_root: tmp.to_path_buf(),
            slug: slug.into(),
            title: slug.into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.to_path_buf(),
            slug: slug.into(),
        })
        .unwrap();
    }

    /// Verifies that bulk archive picks up only `Committed` tasks.
    #[test]
    fn ark_archive_skips_non_committed_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        quick_task(tmp.path(), "ready");
        quick_task(tmp.path(), "not_ready");
        // Only `ready` reaches Committed.
        force_committed_at(
            tmp.path(),
            "ready",
            Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap(),
        );

        let summary = ark_archive(ArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            month: None,
            dry_run: false,
        })
        .unwrap();
        assert_eq!(summary.successes.len(), 1);
        assert_eq!(summary.successes[0].slug, "ready");
        assert!(summary.failures.is_empty());

        // not_ready stays put.
        assert!(tmp.path().join(".ark/tasks/not_ready").exists());
        // ready moved to 2026-05.
        assert!(tmp.path().join(".ark/tasks/archive/2026-05/ready").exists());
    }

    /// Verifies that `archive_month` is derived from `committed_at`, not the
    /// archive run's wall clock.
    #[test]
    fn ark_archive_uses_committed_at_month_not_now() {
        let tmp = tempfile::tempdir().unwrap();
        quick_task(tmp.path(), "old");
        // committed_at is in March 2026 even if today is May.
        force_committed_at(
            tmp.path(),
            "old",
            Utc.with_ymd_and_hms(2026, 3, 15, 12, 0, 0).unwrap(),
        );

        let summary = ark_archive(ArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            month: None,
            dry_run: false,
        })
        .unwrap();
        assert_eq!(summary.successes.len(), 1);
        let archive_path = &summary.successes[0].archive_path;
        assert!(
            archive_path
                .to_string_lossy()
                .contains(".ark/tasks/archive/2026-03/old"),
            "archive_path must use committed_at's YYYY-MM (2026-03), got {}",
            archive_path.display()
        );
    }

    /// Verifies that `--month` filters out tasks committed in other months.
    #[test]
    fn ark_archive_filters_by_month() {
        let tmp = tempfile::tempdir().unwrap();
        quick_task(tmp.path(), "march");
        quick_task(tmp.path(), "may");
        force_committed_at(
            tmp.path(),
            "march",
            Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap(),
        );
        force_committed_at(
            tmp.path(),
            "may",
            Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
        );

        let summary = ark_archive(ArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            month: Some("2026-03".into()),
            dry_run: false,
        })
        .unwrap();
        assert_eq!(summary.successes.len(), 1);
        assert_eq!(summary.successes[0].slug, "march");
        // May's task is still active.
        assert!(tmp.path().join(".ark/tasks/may").exists());
    }

    /// Verifies that `--dry-run` does not move any directory.
    #[test]
    fn ark_archive_dry_run_does_not_move() {
        let tmp = tempfile::tempdir().unwrap();
        quick_task(tmp.path(), "demo");
        force_committed_at(
            tmp.path(),
            "demo",
            Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
        );

        let summary = ark_archive(ArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            month: None,
            dry_run: true,
        })
        .unwrap();
        assert!(summary.dry_run);
        assert_eq!(summary.planned.len(), 1);
        assert!(summary.successes.is_empty());
        assert!(tmp.path().join(".ark/tasks/demo").exists());
        assert!(!tmp.path().join(".ark/tasks/archive/2026-05/demo").exists());
    }

    /// Verifies that `ark archive` does not write to any workspace journal.
    ///
    /// Journal recording is `task_commit`'s responsibility (and that already
    /// happened in the closing commit). Bulk archive must not duplicate it.
    #[test]
    fn ark_archive_writes_no_journal_entry() {
        use crate::commands::agent::workspace::init::{WorkspaceInitOptions, workspace_init};
        let tmp = tempfile::tempdir().unwrap();
        workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        let layout = Layout::new(tmp.path());
        let journal_path = layout.workspace_journal("alice", 1);
        let pre_bytes = journal_path.read_bytes().unwrap();
        quick_task(tmp.path(), "demo");
        force_committed_at(
            tmp.path(),
            "demo",
            Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
        );

        ark_archive(ArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            month: None,
            dry_run: false,
        })
        .unwrap();

        let post_bytes = journal_path.read_bytes().unwrap();
        assert_eq!(
            pre_bytes, post_bytes,
            "ark archive must not touch the journal"
        );
    }

    /// Verifies idempotency: re-running over the same set is a no-op.
    #[test]
    fn ark_archive_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        quick_task(tmp.path(), "demo");
        force_committed_at(
            tmp.path(),
            "demo",
            Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
        );

        let first = ark_archive(ArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            month: None,
            dry_run: false,
        })
        .unwrap();
        assert_eq!(first.successes.len(), 1);

        // Second run finds nothing committed (the dir moved into archive/),
        // so successes and failures are both empty.
        let second = ark_archive(ArchiveOptions {
            project_root: tmp.path().to_path_buf(),
            month: None,
            dry_run: false,
        })
        .unwrap();
        assert!(second.successes.is_empty());
        assert!(second.failures.is_empty());
    }
}
