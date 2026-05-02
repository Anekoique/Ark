//! `<dev>/index.md` re-render via managed blocks.
//!
//! The index owns two managed blocks.
//!
//! - `ARK:WORKSPACE_STATUS` — active file + total sessions + last-active date.
//! - `ARK:WORKSPACE_SESSIONS` — GFM table of every session, desc by session number.
//!
//! Re-render scope is capped: 100 journal files × 100 entries each.

use std::path::Path;

use chrono::NaiveDate;

use super::journal::{ParsedEntry, parse_entries, parse_journal_index};
use crate::{
    error::{Error, Result},
    io::{PathExt, fs::update_managed_block},
    layout::Layout,
};

/// Hard ceiling on journal files scanned per `rerender`.
pub(crate) const INDEX_RERENDER_JOURNAL_CAP: usize = 100;

/// Hard ceiling on entries parsed per journal file.
pub(crate) const INDEX_RERENDER_ENTRIES_PER_JOURNAL_CAP: usize = 100;

/// Marker for the workspace status managed block.
const STATUS_MARKER: &str = "ARK:WORKSPACE_STATUS";

/// Marker for the workspace sessions managed block.
const SESSIONS_MARKER: &str = "ARK:WORKSPACE_SESSIONS";

/// Default body shipped in the seeded `index.md` for the status block.
const SEED_STATUS_BODY: &str = "\
- **Active File**: `journal-1.md`
- **Total Sessions**: 0
- **Last Active**: -";

/// Default body shipped in the seeded `index.md` for the sessions table.
const SEED_SESSIONS_BODY: &str = "\
| # | Date | Title | Kind | Slug | Branch | Commits |
|---|------|-------|------|------|--------|---------|";

/// Seeds `<dev>/index.md` with managed-block markers and empty bodies.
///
/// The first record call populates the bodies via `update_managed_block`.
pub fn seed_index(path: &Path, dev: &str) -> Result<()> {
    let body = format!(
        "# Workspace Index — {dev}\n\n> Per-developer session journal. Updated automatically by \
         `ark agent workspace {{init|record}}` and `ark agent task archive`.\n\n<!-- \
         {STATUS_MARKER}:START -->\n{SEED_STATUS_BODY}\n<!-- {STATUS_MARKER}:END -->\n\n## \
         Sessions\n\n<!-- {SESSIONS_MARKER}:START -->\n{SEED_SESSIONS_BODY}\n<!-- \
         {SESSIONS_MARKER}:END -->\n"
    );
    path.write_bytes(body.as_bytes())
}

/// Re-renders both managed blocks in `<dev>/index.md` from the journal files.
pub fn rerender(layout: &Layout, dev: &str) -> Result<()> {
    let dir = layout.workspace_developer_dir(dev);
    if !dir.exists() {
        return Err(Error::DeveloperNotInitialized {
            path: layout.developer_file(),
        });
    }

    // Collect journal files with their parsed numeric index, sorted DESCENDING
    // so the journal cap takes the *newest* N journals. Scanning ascending
    // and capping would silently drop recent sessions on long-lived
    // workspaces.
    let mut journals: Vec<(u32, std::path::PathBuf)> = Vec::new();
    for entry in dir.list_dir()? {
        let entry = entry.map_err(|e| Error::io(&dir, e))?;
        if let Some(n) = parse_journal_index(&entry.file_name().to_string_lossy()) {
            journals.push((n, entry.path()));
        }
    }
    journals.sort_by_key(|(n, _)| std::cmp::Reverse(*n));

    // Build the entry list (capped) and track the latest entry date.
    let mut all_entries: Vec<ParsedEntry> = Vec::new();
    let mut last_date: Option<NaiveDate> = None;
    for (_, path) in journals.iter().take(INDEX_RERENDER_JOURNAL_CAP) {
        let text = path.read_text()?;
        for entry in parse_entries(&text)
            .into_iter()
            .take(INDEX_RERENDER_ENTRIES_PER_JOURNAL_CAP)
        {
            if last_date.map(|d| entry.date > d).unwrap_or(true) {
                last_date = Some(entry.date);
            }
            all_entries.push(entry);
        }
    }

    // Active file = highest-numbered journal-N.md (first after desc sort).
    let active_file = journals
        .first()
        .map(|(n, _)| format!("journal-{n}.md"))
        .unwrap_or_else(|| "journal-1.md".to_string());

    let total = all_entries.len() as u32;

    let status_body = render_status_block(&active_file, total, last_date);
    let sessions_body = render_sessions_block(&mut all_entries);

    let path = layout.workspace_index(dev);
    update_managed_block(&path, STATUS_MARKER, &status_body)?;
    update_managed_block(&path, SESSIONS_MARKER, &sessions_body)?;
    Ok(())
}

fn render_status_block(active_file: &str, total: u32, last_active: Option<NaiveDate>) -> String {
    let last = match last_active {
        Some(d) => d.to_string(),
        None => "-".to_string(),
    };
    format!(
        "- **Active File**: `{active_file}`\n- **Total Sessions**: {total}\n- **Last Active**: \
         {last}"
    )
}

fn render_sessions_block(entries: &mut [ParsedEntry]) -> String {
    entries.sort_by_key(|e| std::cmp::Reverse(e.session_number));
    let mut out = String::with_capacity(64 + entries.len() * 80);
    out.push_str("| # | Date | Title | Kind | Slug | Branch | Commits |\n");
    out.push_str("|---|------|-------|------|------|--------|---------|");
    for e in entries.iter() {
        let slug = e.slug.as_deref().unwrap_or("-");
        let title = e.title.replace('|', "\\|");
        out.push_str(&format!(
            "\n| {} | {} | {} | {} | {} | `{}` | {} |",
            e.session_number, e.date, title, e.kind_label, slug, e.branch, e.commits_count,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::workspace::journal::{JournalEntry, JournalKind, render_entry};

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// Verifies that `seed_index` produces both managed-block markers.
    #[test]
    fn seed_index_includes_markers() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("index.md");
        seed_index(&path, "alice").unwrap();
        let text = path.read_text().unwrap();
        assert!(text.contains(&format!("<!-- {STATUS_MARKER}:START -->")));
        assert!(text.contains(&format!("<!-- {STATUS_MARKER}:END -->")));
        assert!(text.contains(&format!("<!-- {SESSIONS_MARKER}:START -->")));
        assert!(text.contains(&format!("<!-- {SESSIONS_MARKER}:END -->")));
        assert!(text.contains("# Workspace Index — alice"));
    }

    /// Verifies that rerender on a fresh dev dir is idempotent.
    #[test]
    fn rerender_idempotent_on_empty_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let dev = "alice";
        let dev_dir = layout.workspace_developer_dir(dev);
        dev_dir.ensure_dir().unwrap();
        seed_index(&layout.workspace_index(dev), dev).unwrap();

        rerender(&layout, dev).unwrap();
        let first = layout.workspace_index(dev).read_bytes().unwrap();
        rerender(&layout, dev).unwrap();
        let second = layout.workspace_index(dev).read_bytes().unwrap();
        assert_eq!(first, second);
    }

    /// Verifies that rerender surfaces one entry in the sessions table.
    #[test]
    fn rerender_reflects_one_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let dev = "alice";
        let dev_dir = layout.workspace_developer_dir(dev);
        dev_dir.ensure_dir().unwrap();
        seed_index(&layout.workspace_index(dev), dev).unwrap();

        // Seed journal-1.md with a header + one rendered entry.
        let entry = JournalEntry {
            session_number: 1,
            title: "first".into(),
            date: date("2026-04-29"),
            kind: JournalKind::Manual,
            branch: Some("main".into()),
            start_head: None,
            base_branch: None,
            summary: "did stuff".into(),
            commits: Vec::new(),
            next_steps: Vec::new(),
        };
        let body = format!(
            "# Journal — alice (Part 1)\n\n> j\n\n---\n{}",
            render_entry(&entry)
        );
        layout
            .workspace_journal(dev, 1)
            .write_bytes(body.as_bytes())
            .unwrap();

        rerender(&layout, dev).unwrap();
        let text = layout.workspace_index(dev).read_text().unwrap();
        assert!(text.contains("**Total Sessions**: 1"));
        assert!(text.contains("**Last Active**: 2026-04-29"));
        assert!(text.contains("| 1 | 2026-04-29 | first | manual | - | `main` | 0 |"));
    }
}
