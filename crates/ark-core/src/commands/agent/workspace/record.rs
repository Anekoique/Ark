//! `workspace_record` — appends auto-fields to the agent's pre-written
//! journal entry, then updates the personal + top-level indices.
//!
//! Unlike the earlier `--entry-file` design, the agent writes its block
//! (heading + Summary + Main Changes) **directly into** the active journal
//! file before invoking the CLI. The CLI then:
//!
//! 1. Locates the file from the resolved identity.
//! 2. Stamps the auto-fields (Date, Slug, Branch, Base Branch, Start Head,
//!    Closing Commit sentinel for tasks; Date, Slug=`-`, Branch for manual)
//!    after the last `## Session N: <title>` heading via [`stamp_task`] /
//!    [`stamp_manual`].
//! 3. Upserts the personal-index Session History row.
//! 4. Refreshes the top-level Active Developers row.
//!
//! All four steps are transactional via [`RecordTransaction`]; on internal
//! failure the primitive rolls itself back and returns `Err`. On success it
//! returns `RecordSummary` whose adoptable [`RecordSnapshot`] outer
//! transactions (`RollbackGuard`) can adopt for outer rollback.

use std::path::{Path, PathBuf};

use chrono::Local;

use crate::{
    commands::agent::workspace::{
        developer::{DeveloperRegisterOptions, developer_register},
        identity::{ResolveOptions, identity_resolve},
        stamp::{
            ManualStampFields, TaskStampFields, extract_last_session_title, stamp_manual,
            stamp_task,
        },
        transaction::{RecordSnapshot, RecordTransaction},
    },
    error::{Error, Result},
    io::{PathExt, read_managed_block, update_managed_block},
    layout::{Layout, SESSIONS_MARKER},
};

/// Personal-index static preface (per-developer `index.md`).
const PERSONAL_PREFACE: &str = "# Sessions\n\n> Personal session history.\n\n## History\n\n";
const PERSONAL_SUFFIX: &str =
    "\n\n---\n\nHand-edits outside the `ARK:SESSIONS` markers are preserved.\n";

const PERSONAL_HEADER: [&str; 2] = [
    "| # | Date | Title | Slug | Branch | Closing Commit | Journal |",
    "|---|------|-------|------|--------|----------------|---------|",
];

/// Selects task vs manual rendering for [`workspace_record`].
pub enum RecordMode<'a> {
    /// Task-driven entry. Agent has already appended the entry block to the
    /// active journal; CLI stamps the auto-fields.
    Task {
        /// Task slug (sentinel anchor).
        slug: &'a str,
        /// Active task branch.
        branch: &'a str,
        /// Base branch the task forked from.
        base_branch: &'a str,
        /// Short SHA of `task.toml.start_head`.
        start_head_short: &'a str,
        /// Commits in `<start_head>..HEAD` for the rendered table.
        commits_in_range: &'a [(String, String)],
    },
    /// Manual entry (no slug, no Closing-Commit sentinel, no Git Commits table).
    Manual,
}

/// Options for [`workspace_record`].
pub struct RecordOptions<'a> {
    project_root: &'a Path,
    mode: RecordMode<'a>,
}

impl<'a> RecordOptions<'a> {
    /// Constructs options.
    pub fn new(project_root: &'a Path, mode: RecordMode<'a>) -> Self {
        Self { project_root, mode }
    }
}

/// Outcome of a successful [`workspace_record`].
///
/// Pass `into_snapshot()` to outer transactions for adoption.
#[derive(Debug)]
pub struct RecordSummary {
    journal_path: PathBuf,
    journal_path_relative: String,
    session_number: u32,
    snapshot: RecordSnapshot,
}

impl RecordSummary {
    /// Returns the journal file path.
    pub fn journal_path(&self) -> &Path {
        &self.journal_path
    }

    /// Returns the project-relative journal path (used by
    /// `task.toml.journal_path`).
    pub fn journal_path_relative(&self) -> &str {
        &self.journal_path_relative
    }

    /// Returns the entry's session number.
    pub fn session_number(&self) -> u32 {
        self.session_number
    }

    /// Yields the adoptable snapshot (consumes self).
    pub fn into_snapshot(self) -> RecordSnapshot {
        self.snapshot
    }
}

/// Stamps auto-fields on the agent's pre-written journal entry, then
/// updates the personal and top-level indices.
///
/// Errors:
/// - [`Error::MissingIdentity`] — `.ark/.developer` absent.
/// - [`Error::EntryFileMalformed`] — agent did not append a `## Session N:
///   <title>` heading, or the journal file does not exist.
pub fn workspace_record(opts: RecordOptions<'_>) -> Result<RecordSummary> {
    let layout = Layout::new(opts.project_root);
    let identity = identity_resolve(ResolveOptions::new(opts.project_root))?;

    let dev_dir = layout.workspace_developer_dir(identity.name());
    dev_dir.ensure_dir()?;

    let (journal_filename, session_number) = select_active_journal(&dev_dir)?;
    let journal_path = dev_dir.join(&journal_filename);
    let personal_index_path = layout.workspace_developer_index(identity.name());
    let top_level_index_path = layout.workspace_index();

    if !journal_path.exists() {
        return Err(Error::EntryFileMalformed {
            reason: "active journal does not exist; the agent must append the entry to the \
                     journal before invoking this command",
        });
    }

    // Snapshots taken BEFORE any mutation. Mutations live inside `apply`;
    // any `?`-return there triggers `tx.rollback()`. `commit()` runs only
    // when every step succeeds.
    let mut tx = RecordTransaction::begin(
        journal_path.clone(),
        personal_index_path.clone(),
        top_level_index_path.clone(),
    )?;

    let date = Local::now().date_naive();
    let mut apply = || -> Result<()> {
        match &opts.mode {
            RecordMode::Task {
                slug,
                branch,
                base_branch,
                start_head_short,
                commits_in_range,
            } => {
                stamp_task(
                    &journal_path,
                    &TaskStampFields {
                        date,
                        slug,
                        branch,
                        base_branch,
                        start_head_short,
                        commits_in_range,
                    },
                )?;
            }
            RecordMode::Manual => {
                let branch = current_branch(opts.project_root).unwrap_or_else(|| "-".to_string());
                stamp_manual(
                    &journal_path,
                    &ManualStampFields {
                        date,
                        branch: &branch,
                    },
                )?;
            }
        }
        capture_stamped_suffix(&mut tx);
        tx.mark_journal_appended();

        let title = extract_last_session_title(&journal_path.read_text()?)?;
        update_personal_index(
            &personal_index_path,
            session_number,
            date,
            &title,
            &opts.mode,
            &journal_filename,
        )?;
        tx.mark_personal_updated();

        developer_register(DeveloperRegisterOptions {
            project_root: opts.project_root.to_path_buf(),
            name: identity.name().to_string(),
            active_journal: journal_filename.clone(),
            date,
            session_count: session_number,
        })?;
        tx.mark_top_level_updated();
        Ok(())
    };

    if let Err(e) = apply() {
        let _ = tx.rollback();
        return Err(e);
    }

    let snapshot = tx.commit();
    Ok(RecordSummary {
        journal_path: journal_path.clone(),
        journal_path_relative: relativize(opts.project_root, &journal_path),
        session_number,
        snapshot,
    })
}

/// Captures the byte range the stamp added to the journal so the
/// transaction's suffix-check rollback can detect concurrent appends.
///
/// At `begin()` the journal was at length `L0` (the agent's pre-stamp
/// content). After `stamp_*`, the journal is at `L1 > L0`. The
/// `L1 - L0` trailing bytes are what rollback truncates and what
/// suffix-check compares against.
fn capture_stamped_suffix(tx: &mut RecordTransaction) {
    let Ok(metadata) = std::fs::metadata(tx.journal_path()) else {
        return;
    };
    let new_len = metadata.len();
    let original_len = tx.journal_byte_length_before();
    if new_len <= original_len {
        return;
    }
    let trailing_len = (new_len - original_len) as usize;
    let Ok(bytes) = std::fs::read(tx.journal_path()) else {
        return;
    };
    let start = bytes.len().saturating_sub(trailing_len);
    tx.record_appended_bytes(bytes[start..].to_vec());
}

/// Returns `(filename, session_number)` for the journal containing the
/// agent's pre-written entry.
///
/// The agent appends to the highest-numbered `journal-N.md` and creates a
/// new file when rotation is needed; this function returns whichever file
/// already holds the entry, plus the next session number.
fn select_active_journal(dev_dir: &Path) -> Result<(String, u32)> {
    let mut journals: Vec<u32> = std::fs::read_dir(dev_dir)
        .map_err(|source| Error::Io {
            path: dev_dir.to_path_buf(),
            source,
        })?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_prefix("journal-")
                .and_then(|rest| rest.strip_suffix(".md"))
                .and_then(|n| n.parse::<u32>().ok())
        })
        .collect();
    journals.sort_unstable_by(|a, b| b.cmp(a));

    let max_n = *journals.first().ok_or(Error::EntryFileMalformed {
        reason: "no journal-N.md files in the developer's workspace; the agent must create \
                 journal-1.md and append the entry before invoking this command",
    })?;
    let session_number = scan_session_count(dev_dir)?;
    Ok((format!("journal-{max_n}.md"), session_number))
}

/// Counts existing `## Session N:` headings across all journals in `dev_dir`.
fn scan_session_count(dev_dir: &Path) -> Result<u32> {
    let mut count: u32 = 0;
    let entries = match std::fs::read_dir(dev_dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(source) => {
            return Err(Error::Io {
                path: dev_dir.to_path_buf(),
                source,
            });
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("journal-") || !name.ends_with(".md") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(entry.path()) {
            count += text
                .lines()
                .filter(|l| l.trim_start().starts_with("## Session "))
                .count() as u32;
        }
    }
    Ok(count)
}

/// Upserts a row in the per-developer Session History table.
fn update_personal_index(
    path: &Path,
    session_number: u32,
    date: chrono::NaiveDate,
    title: &str,
    mode: &RecordMode<'_>,
    journal_filename: &str,
) -> Result<()> {
    if !path.exists() {
        let scaffold = format!(
            "{}<!-- {SESSIONS_MARKER}:START -->\n<!-- {SESSIONS_MARKER}:END -->{}",
            PERSONAL_PREFACE, PERSONAL_SUFFIX
        );
        path.write_bytes(scaffold.as_bytes())?;
    }

    let existing = read_managed_block(path, SESSIONS_MARKER)?.unwrap_or_default();
    let new_body = upsert_personal_row(
        &existing,
        session_number,
        date,
        title,
        mode,
        journal_filename,
    );
    update_managed_block(path, SESSIONS_MARKER, &new_body)?;
    Ok(())
}

fn upsert_personal_row(
    body: &str,
    session_number: u32,
    date: chrono::NaiveDate,
    title: &str,
    mode: &RecordMode<'_>,
    journal_filename: &str,
) -> String {
    let (slug, closing) = match mode {
        RecordMode::Task { slug, .. } => (slug.to_string(), format!("<PENDING:{slug}>")),
        RecordMode::Manual => ("-".to_string(), "-".to_string()),
    };
    let branch = match mode {
        RecordMode::Task { branch, .. } => branch.to_string(),
        RecordMode::Manual => "-".to_string(),
    };
    let new_row = format!(
        "| {session_number} | {date_str} | {title_safe} | `{slug}` | `{branch}` | {closing} | \
         `{journal_filename}` |",
        date_str = date.format("%Y-%m-%d"),
        title_safe = sanitize_cell(title),
    );

    let trimmed = body.trim();
    let has_header = trimmed
        .lines()
        .next()
        .is_some_and(|l| l.trim_start().starts_with("| #"));

    let (header_lines, data_lines): (Vec<&str>, Vec<&str>) = if has_header {
        let mut it = trimmed.lines();
        let h1 = it.next().unwrap_or("");
        let h2 = it.next().unwrap_or("");
        (vec![h1, h2], it.filter(|l| !l.trim().is_empty()).collect())
    } else {
        (
            PERSONAL_HEADER.to_vec(),
            trimmed.lines().filter(|l| !l.trim().is_empty()).collect(),
        )
    };

    let mut out: Vec<String> = header_lines.into_iter().map(String::from).collect();
    out.extend(data_lines.into_iter().map(String::from));
    out.push(new_row);
    let mut text = out.join("\n");
    text.push('\n');
    text
}

fn sanitize_cell(raw: &str) -> String {
    raw.replace('|', "/").replace(['\r', '\n'], " ")
}

fn current_branch(project_root: &Path) -> Option<String> {
    let out = crate::io::git::run_git(&["rev-parse", "--abbrev-ref", "HEAD"], project_root).ok()?;
    if !out.is_success() {
        return None;
    }
    Some(out.stdout.trim().to_string())
}

fn relativize(project_root: &Path, absolute: &Path) -> String {
    absolute
        .strip_prefix(project_root)
        .map(|p| p.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/"))
        .unwrap_or_else(|_| absolute.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::workspace::identity::{Identity, identity_write};

    fn setup_with_identity_and_journal(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        identity_write(tmp.path(), &Identity::new(name).unwrap()).unwrap();
        let layout = Layout::new(tmp.path());
        let dev_dir = layout.workspace_developer_dir(name);
        dev_dir.ensure_dir().unwrap();
        let journal = dev_dir.join("journal-1.md");
        // Pre-write the agent's portion: heading + Summary + Main Changes table.
        std::fs::write(
            &journal,
            "## Session 1: First entry\n\n### Summary\n\nDid the thing.\n\n### Main Changes\n\n| \
             Area | Description |\n|------|-------------|\n| identity | bootstrap |\n",
        )
        .unwrap();
        (tmp, journal)
    }

    #[test]
    fn record_task_stamps_auto_fields_and_updates_indices() {
        let (tmp, journal) = setup_with_identity_and_journal("alice");
        let commits: Vec<(String, String)> = vec![("abc1234".into(), "init".into())];
        let mode = RecordMode::Task {
            slug: "workspace",
            branch: "feat/workspace",
            base_branch: "main",
            start_head_short: "73d46ba",
            commits_in_range: &commits,
        };
        let summary = workspace_record(RecordOptions::new(tmp.path(), mode)).unwrap();
        assert_eq!(summary.session_number(), 1);

        let body = std::fs::read_to_string(&journal).unwrap();
        assert!(body.contains("**Slug**: workspace"));
        assert!(body.contains("**Closing Commit**: <PENDING:workspace>"));
        assert!(body.contains("### Git Commits"));

        let layout = Layout::new(tmp.path());
        let personal = std::fs::read_to_string(layout.workspace_developer_index("alice")).unwrap();
        assert!(personal.contains("<PENDING:workspace>"));

        let top = std::fs::read_to_string(layout.workspace_index()).unwrap();
        assert!(top.contains("| `alice` |"));
    }

    #[test]
    fn record_manual_uses_dash_slug_and_no_sentinel() {
        let (tmp, journal) = setup_with_identity_and_journal("alice");
        let summary = workspace_record(RecordOptions::new(tmp.path(), RecordMode::Manual)).unwrap();
        assert_eq!(summary.session_number(), 1);

        let body = std::fs::read_to_string(&journal).unwrap();
        assert!(body.contains("**Slug**: -"));
        assert!(!body.contains("**Closing Commit**:"));

        let layout = Layout::new(tmp.path());
        let personal = std::fs::read_to_string(layout.workspace_developer_index("alice")).unwrap();
        assert!(!personal.contains("<PENDING:"));
    }

    #[test]
    fn record_errors_when_identity_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let err = workspace_record(RecordOptions::new(tmp.path(), RecordMode::Manual)).unwrap_err();
        assert!(matches!(err, Error::MissingIdentity));
    }

    #[test]
    fn record_errors_when_journal_missing_or_no_heading() {
        let tmp = tempfile::tempdir().unwrap();
        identity_write(tmp.path(), &Identity::new("alice").unwrap()).unwrap();
        // No journal at all.
        let err = workspace_record(RecordOptions::new(tmp.path(), RecordMode::Manual)).unwrap_err();
        assert!(matches!(err, Error::EntryFileMalformed { .. }));

        // Journal exists but has no `## Session` heading.
        let layout = Layout::new(tmp.path());
        let dev_dir = layout.workspace_developer_dir("alice");
        dev_dir.ensure_dir().unwrap();
        std::fs::write(dev_dir.join("journal-1.md"), "no headings here\n").unwrap();
        let err = workspace_record(RecordOptions::new(tmp.path(), RecordMode::Manual)).unwrap_err();
        assert!(matches!(err, Error::EntryFileMalformed { .. }));
    }
}
