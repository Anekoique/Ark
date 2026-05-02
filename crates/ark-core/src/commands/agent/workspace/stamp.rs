//! Stamp the agent's just-written `## Session N` block with auto-fields.
//!
//! Workflow: the agent appends a Markdown block to the active journal
//! ending with a `## Session N: <title>` heading + `### Summary` + `###
//! Main Changes` table. This module locates that block (the **last** `##
//! Session` heading in the file) and inserts the auto-populated fields
//! between the heading and the first `###` subheading.
//!
//! Auto-fields rendered:
//!
//! - Task mode: `**Date**`, `**Slug**`, `**Branch**`, `**Base Branch**`,
//!   `**Start Head**`, `**Closing Commit**: <PENDING:<slug>>`, plus a
//!   `### Git Commits` table inserted after `### Main Changes`.
//! - Manual mode: `**Date**`, `**Slug**: -`, `**Branch**`. No closing-commit
//!   sentinel, no git-commits table.

use std::path::Path;

use chrono::NaiveDate;

use crate::{
    error::{Error, Result},
    io::PathExt,
};

/// Auto-populated header fields for a task-driven entry.
#[derive(Debug, Clone)]
pub struct TaskStampFields<'a> {
    /// Local date for the entry.
    pub date: NaiveDate,
    /// Task slug. Used in `**Slug**` and the closing-commit sentinel.
    pub slug: &'a str,
    /// Active task branch.
    pub branch: &'a str,
    /// Base branch the task forked from.
    pub base_branch: &'a str,
    /// Short SHA of `task.toml.start_head`.
    pub start_head_short: &'a str,
    /// `(short_sha, subject)` rows from `git log <start_head>..HEAD`.
    pub commits_in_range: &'a [(String, String)],
}

/// Auto-populated header fields for a manual entry.
#[derive(Debug, Clone)]
pub struct ManualStampFields<'a> {
    /// Local date for the entry.
    pub date: NaiveDate,
    /// Current branch (best-effort; "-" when git is unavailable).
    pub branch: &'a str,
}

/// Outcome of stamping a journal file.
///
/// Returned by [`stamp_task`] / [`stamp_manual`] so the caller can record
/// (a) the bytes the stamp inserted, for transactional rollback, and (b)
/// the exact byte position where the heading was found.
#[derive(Debug, Clone)]
pub struct StampOutcome {
    /// Bytes the stamp inserted into the file. Together with `original_len`,
    /// these let a transaction roll back by truncating the file back to
    /// `original_len` and re-appending the agent's pre-stamp bytes.
    pub inserted: String,
    /// Byte length of the file before stamping. The agent's pre-stamp
    /// content is `original[..original_len]`; the stamp re-emits the
    /// post-heading remainder via the `inserted` slice.
    pub original_len: u64,
}

/// Stamps a task-driven entry in `journal_path`.
///
/// Errors with [`Error::EntryFileMalformed`] when the journal does not end
/// with a `## Session N: <title>` block (the agent failed to append one
/// before invoking the CLI).
pub fn stamp_task(journal_path: &Path, fields: &TaskStampFields<'_>) -> Result<StampOutcome> {
    let original = journal_path.read_text()?;
    let original_len = original.len() as u64;

    let split = locate_last_session_block(&original)?;
    let heading = &original[split.heading_start..split.heading_end];
    let body = &original[split.heading_end..];

    let auto_fields = render_task_auto_fields(fields);
    let commits_block = render_git_commits_block(fields.commits_in_range);
    let stamped_body = insert_after_main_changes(body, &commits_block);

    let mut out = String::with_capacity(original.len() + auto_fields.len() + commits_block.len());
    out.push_str(&original[..split.heading_start]);
    out.push_str(heading);
    out.push_str(&auto_fields);
    out.push_str(&stamped_body);

    journal_path.write_bytes(out.as_bytes())?;

    Ok(StampOutcome {
        inserted: out[split.heading_start..].to_string(),
        original_len,
    })
}

/// Stamps a manual entry in `journal_path`.
///
/// Errors when the journal does not end with a `## Session N: <title>` block.
pub fn stamp_manual(journal_path: &Path, fields: &ManualStampFields<'_>) -> Result<StampOutcome> {
    let original = journal_path.read_text()?;
    let original_len = original.len() as u64;

    let split = locate_last_session_block(&original)?;
    let heading = &original[split.heading_start..split.heading_end];
    let body = &original[split.heading_end..];

    let auto_fields = render_manual_auto_fields(fields);

    let mut out = String::with_capacity(original.len() + auto_fields.len());
    out.push_str(&original[..split.heading_start]);
    out.push_str(heading);
    out.push_str(&auto_fields);
    out.push_str(body);

    journal_path.write_bytes(out.as_bytes())?;

    Ok(StampOutcome {
        inserted: out[split.heading_start..].to_string(),
        original_len,
    })
}

/// Returns the title of the last `## Session N: <title>` heading in `text`.
///
/// Used by callers to populate the per-developer index row without
/// re-parsing the heading themselves.
pub fn extract_last_session_title(text: &str) -> Result<String> {
    let split = locate_last_session_block(text)?;
    let heading = &text[split.heading_start..split.heading_end];
    let line = heading.lines().next().unwrap_or("");
    let after = line
        .trim_start()
        .strip_prefix("## Session")
        .and_then(|rest| rest.find(':').map(|i| rest[i + 1..].trim().to_string()));
    after.ok_or(Error::EntryFileMalformed {
        reason: "could not parse `## Session N: <title>` heading",
    })
}

struct SessionSplit {
    heading_start: usize,
    heading_end: usize,
}

/// Locates the last `## Session ...` heading in `text`.
///
/// `heading_start` is the byte index of the line's first character;
/// `heading_end` is the byte index *after* the trailing newline.
fn locate_last_session_block(text: &str) -> Result<SessionSplit> {
    let mut last: Option<(usize, usize)> = None;
    let mut cursor = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("## Session") {
            last = Some((cursor, cursor + line.len()));
        }
        cursor += line.len();
    }
    let (start, end) = last.ok_or(Error::EntryFileMalformed {
        reason: "journal does not end with a `## Session N: <title>` heading",
    })?;
    Ok(SessionSplit {
        heading_start: start,
        heading_end: end,
    })
}

fn render_task_auto_fields(fields: &TaskStampFields<'_>) -> String {
    format!(
        "\n**Date**: {date}\n**Slug**: {slug}\n**Branch**: `{branch}`\n**Base Branch**: \
         `{base}`\n**Start Head**: `{head}`\n**Closing Commit**: <PENDING:{slug}>\n",
        date = fields.date.format("%Y-%m-%d"),
        slug = fields.slug,
        branch = fields.branch,
        base = fields.base_branch,
        head = fields.start_head_short,
    )
}

fn render_manual_auto_fields(fields: &ManualStampFields<'_>) -> String {
    format!(
        "\n**Date**: {date}\n**Slug**: -\n**Branch**: `{branch}`\n",
        date = fields.date.format("%Y-%m-%d"),
        branch = fields.branch,
    )
}

fn render_git_commits_block(commits: &[(String, String)]) -> String {
    let mut s = String::from("\n### Git Commits\n\n");
    if commits.is_empty() {
        s.push_str("| Hash | Message |\n|------|---------|\n| _(none)_ |   |\n");
    } else {
        s.push_str("| Hash | Message |\n|------|---------|\n");
        for (sha, subject) in commits {
            s.push_str(&format!("| `{sha}` | {subject} |\n"));
        }
    }
    s
}

/// Inserts `commits_block` after the agent's `### Main Changes` table.
///
/// Walks lines from the heading forward, tracking the running byte offset.
/// The insertion point is the byte just after the last `|`-prefixed line.
/// If no Main Changes heading or table is found, appends to the body.
fn insert_after_main_changes(body: &str, commits_block: &str) -> String {
    let Some(start) = body.find("### Main Changes") else {
        return format!("{body}{commits_block}");
    };

    let mut cursor = start;
    let mut after_table: Option<usize> = None;
    for line in body[start..].split_inclusive('\n') {
        if line.trim_start().starts_with('|') {
            after_table = Some(cursor + line.len());
        }
        cursor += line.len();
    }

    let Some(end) = after_table else {
        return format!("{body}{commits_block}");
    };

    let mut s = String::with_capacity(body.len() + commits_block.len());
    s.push_str(&body[..end]);
    s.push_str(commits_block);
    s.push_str(&body[end..]);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simulates what the agent appends: heading + Summary + Main Changes
    /// table. No auto-fields yet — those are the CLI's job.
    fn agent_journal(title: &str) -> String {
        format!(
            "## Session 1: {title}\n\n### Summary\n\nDid the thing.\n\n### Main Changes\n\n| Area \
             | Description |\n|------|-------------|\n| identity | bootstrap |\n\n",
        )
    }

    fn task_fields<'a>(slug: &'a str, commits: &'a [(String, String)]) -> TaskStampFields<'a> {
        TaskStampFields {
            date: NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            slug,
            branch: "feat/workspace",
            base_branch: "main",
            start_head_short: "73d46ba10f39",
            commits_in_range: commits,
        }
    }

    #[test]
    fn stamp_task_inserts_all_auto_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("journal-1.md");
        std::fs::write(&journal, agent_journal("Add workspace")).unwrap();

        let commits = vec![("abc1234".into(), "init".into())];
        let fields = task_fields("workspace", &commits);
        stamp_task(&journal, &fields).unwrap();

        let out = std::fs::read_to_string(&journal).unwrap();
        assert!(out.contains("**Date**: 2026-05-02"));
        assert!(out.contains("**Slug**: workspace"));
        assert!(out.contains("**Branch**: `feat/workspace`"));
        assert!(out.contains("**Base Branch**: `main`"));
        assert!(out.contains("**Start Head**: `73d46ba10f39`"));
        assert!(out.contains("**Closing Commit**: <PENDING:workspace>"));
        assert!(out.contains("### Git Commits"));
        assert!(out.contains("| `abc1234` | init |"));
        // Auto-fields appear between heading and Summary.
        let date_pos = out.find("**Date**:").unwrap();
        let summary_pos = out.find("### Summary").unwrap();
        assert!(date_pos < summary_pos);
        // Git Commits appears AFTER Main Changes.
        let main_pos = out.find("### Main Changes").unwrap();
        let commits_pos = out.find("### Git Commits").unwrap();
        assert!(main_pos < commits_pos);
    }

    #[test]
    fn stamp_manual_omits_task_only_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("journal-1.md");
        std::fs::write(&journal, agent_journal("Note")).unwrap();

        let fields = ManualStampFields {
            date: NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            branch: "main",
        };
        stamp_manual(&journal, &fields).unwrap();

        let out = std::fs::read_to_string(&journal).unwrap();
        assert!(out.contains("**Slug**: -"));
        assert!(!out.contains("**Closing Commit**:"));
        assert!(!out.contains("**Start Head**:"));
        assert!(!out.contains("**Base Branch**:"));
        assert!(!out.contains("### Git Commits"));
    }

    #[test]
    fn stamp_errors_when_no_session_heading() {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("journal-1.md");
        std::fs::write(&journal, "# Some other content\n\nno session heading\n").unwrap();
        let commits: Vec<(String, String)> = Vec::new();
        let fields = task_fields("workspace", &commits);
        let err = stamp_task(&journal, &fields).unwrap_err();
        assert!(matches!(err, Error::EntryFileMalformed { .. }));
    }

    #[test]
    fn stamp_targets_last_session_when_multiple_present() {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("journal-1.md");
        let prior = "## Session 1: Earlier\n\n**Date**: 2026-01-01\n**Slug**: -\n**Branch**: \
                     `main`\n\n### Summary\n\nold\n\n### Main Changes\n\n| Area | Description \
                     |\n|------|-------------|\n| x | y |\n\n";
        let new = "## Session 2: Newer\n\n### Summary\n\nfresh\n\n### Main Changes\n\n| Area | \
                   Description |\n|------|-------------|\n| a | b |\n\n";
        std::fs::write(&journal, format!("{prior}{new}")).unwrap();

        let commits: Vec<(String, String)> = Vec::new();
        let fields = task_fields("workspace", &commits);
        stamp_task(&journal, &fields).unwrap();

        let out = std::fs::read_to_string(&journal).unwrap();
        // Earlier session unchanged, newer session has auto-fields.
        let s1 = out.find("## Session 1:").unwrap();
        let s2 = out.find("## Session 2:").unwrap();
        // Slug auto-field MUST appear after Session 2, not before it.
        let slug_pos = out.find("**Slug**: workspace").expect("slug stamped");
        assert!(slug_pos > s2);
        assert!(slug_pos > s1);
        // Earlier session's `**Slug**: -` is preserved.
        assert!(out.contains("**Slug**: -"));
    }

    #[test]
    fn extract_last_session_title_returns_title() {
        let text = "## Session 1: Older\n\n## Session 2: Add workspace support\n\n### \
                    Summary\n\nx\n\n### Main Changes\n\n| a | b |\n";
        let title = extract_last_session_title(text).unwrap();
        assert_eq!(title, "Add workspace support");
    }
}
