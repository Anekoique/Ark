//! Journal entry rendering, parsing, rotation discovery, and `git log`
//! oneline parsing.
//!
//! Entry shape per workspace G-5; boundary parser per workspace C-19. The
//! journal files are the source of truth for the index sessions table; the
//! parser is the inverse of the renderer.

use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Maximum journal index. `N+1 > 9999` triggers `JournalRotationLimit`.
pub const MAX_JOURNAL_INDEX: u32 = 9999;

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub session_number: u32,
    pub title: String,
    pub date: NaiveDate,
    pub kind: JournalKind,
    /// `None` renders as `unknown`.
    pub branch: Option<String>,
    pub summary: String,
    pub commits: Vec<JournalCommit>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum JournalKind {
    Task { slug: String },
    Manual,
}

#[derive(Debug, Clone)]
pub struct JournalCommit {
    pub short: String,
    pub message: String,
}

/// One row's worth of metadata extracted from a `## Session N: Title` block.
#[derive(Debug, Clone)]
pub struct ParsedEntry {
    pub session_number: u32,
    pub title: String,
    pub date: NaiveDate,
    pub kind_label: String,
    pub slug: Option<String>,
    pub branch: String,
    pub commits_count: u32,
}

// -- Render -----------------------------------------------------------------

/// Render a session entry per G-5. Output begins with a leading blank line
/// (separator from the previous entry or from the journal preamble).
pub fn render_entry(entry: &JournalEntry) -> String {
    let kind_label = match &entry.kind {
        JournalKind::Task { .. } => "task",
        JournalKind::Manual => "manual",
    };
    let slug = match &entry.kind {
        JournalKind::Task { slug } => slug.as_str(),
        JournalKind::Manual => "-",
    };
    let branch = entry.branch.as_deref().unwrap_or("unknown");

    let mut out = String::with_capacity(512);
    out.push('\n');
    out.push_str(&format!(
        "## Session {}: {}\n\n",
        entry.session_number, entry.title
    ));
    out.push_str(&format!("**Date**: {}\n", entry.date));
    out.push_str(&format!("**Kind**: {kind_label}\n"));
    out.push_str(&format!("**Slug**: {slug}\n"));
    out.push_str(&format!("**Branch**: `{branch}`\n\n"));

    out.push_str("### Summary\n");
    if entry.summary.is_empty() {
        out.push_str("(no summary provided)\n");
    } else {
        out.push_str(entry.summary.trim_end());
        out.push('\n');
    }

    out.push_str("\n### Commits\n\n");
    out.push_str("| Hash | Message |\n");
    out.push_str("|------|---------|\n");
    if entry.commits.is_empty() {
        out.push_str("| - | (no commits) |\n");
    } else {
        for c in &entry.commits {
            // Pipe and newline are reserved in GFM table cells; replace with
            // safe substitutes to keep the table parseable on read-back.
            let msg = c.message.replace('|', "\\|").replace('\n', " ");
            out.push_str(&format!("| `{}` | {} |\n", c.short, msg));
        }
    }

    out.push_str("\n### Next Steps\n");
    if entry.next_steps.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for step in &entry.next_steps {
            out.push_str(&format!("- {}\n", step.trim_end()));
        }
    }

    out
}

// -- Parse ------------------------------------------------------------------

/// Parse all `## Session N: Title` entries in `text`. Skips malformed
/// entries silently (writes a single stderr warning per call). Boundary
/// rules per C-19: respects fenced code blocks (3+ backticks or 3+ tildes,
/// length-aware close-fence matching).
pub fn parse_entries(text: &str) -> Vec<ParsedEntry> {
    let lines: Vec<&str> = text.lines().collect();
    let starts = scan_entry_starts(&lines);

    let mut out = Vec::with_capacity(starts.len());
    let mut warned = false;
    for w in 0..starts.len() {
        let begin = starts[w];
        let end = starts.get(w + 1).copied().unwrap_or(lines.len());
        match parse_one_entry(&lines, begin, end) {
            Some(p) => out.push(p),
            None => {
                if !warned {
                    eprintln!("workspace: skipping malformed journal entry");
                    warned = true;
                }
            }
        }
    }
    out
}

/// Scan and return the indices of all valid entry-start lines, skipping
/// lines inside an active fenced code block.
fn scan_entry_starts(lines: &[&str]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut fence: Option<FenceState> = None;
    for (i, line) in lines.iter().enumerate() {
        if let Some(state) = &fence {
            if is_close_fence(line, state) {
                fence = None;
            }
            continue;
        }
        if let Some(state) = open_fence(line) {
            fence = Some(state);
            continue;
        }
        if matches_session_header(line).is_some() {
            starts.push(i);
        }
    }
    starts
}

#[derive(Debug, Clone, Copy)]
struct FenceState {
    char: char,
    len: usize,
}

/// Returns `Some(state)` if `line` opens a fenced code block per C-19:
/// 3+ identical fence chars (`` ` `` or `~`) at the start, optionally
/// followed by a language tag. The tag is not validated — anything past the
/// fence run is accepted.
fn open_fence(line: &str) -> Option<FenceState> {
    let trimmed = line.trim_end_matches([' ', '\t']);
    let bytes = trimmed.as_bytes();
    let first = *bytes.first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let len = bytes.iter().take_while(|&&b| b == first).count();
    if len < 3 {
        return None;
    }
    Some(FenceState {
        char: first as char,
        len,
    })
}

/// True if `line` is a closing fence for the given open state: every byte is
/// `state.char`, and the run is at least as long as the opening fence.
/// Trailing whitespace is tolerated; nothing else may follow the fence chars.
fn is_close_fence(line: &str, state: &FenceState) -> bool {
    let trimmed = line.trim_end_matches([' ', '\t']);
    let bytes = trimmed.as_bytes();
    let target = state.char as u8;
    bytes.len() >= state.len && bytes.iter().all(|&b| b == target)
}

/// Match `^## Session (\d+):\s+(.+?)\s*$`. Returns `(session_number, title)`.
fn matches_session_header(line: &str) -> Option<(u32, String)> {
    let rest = line.strip_prefix("## Session ")?;
    let colon = rest.find(':')?;
    let n_str = &rest[..colon];
    let n: u32 = n_str.parse().ok()?;
    let title = rest[colon + 1..].trim();
    if title.is_empty() {
        return None;
    }
    Some((n, title.to_string()))
}

fn parse_one_entry(lines: &[&str], begin: usize, end: usize) -> Option<ParsedEntry> {
    let (session_number, title) = matches_session_header(lines[begin])?;
    let body = &lines[begin + 1..end];

    let date_str = body
        .iter()
        .find_map(|l| l.strip_prefix("**Date**: ").map(str::trim))
        .unwrap_or("");
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;

    let kind_label = body
        .iter()
        .find_map(|l| l.strip_prefix("**Kind**: ").map(str::trim))
        .unwrap_or("")
        .to_string();
    if kind_label != "task" && kind_label != "manual" {
        return None;
    }

    let slug_raw = body
        .iter()
        .find_map(|l| l.strip_prefix("**Slug**: ").map(str::trim))
        .unwrap_or("-");
    let slug = if slug_raw == "-" || slug_raw.is_empty() {
        None
    } else {
        Some(slug_raw.to_string())
    };

    let branch = body
        .iter()
        .find_map(|l| l.strip_prefix("**Branch**: ").map(str::trim))
        .map(|raw| raw.trim_matches('`').to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Count rows under the `### Commits` table — lines starting with `| ` and
    // containing a backtick (the hash cell is `` `<short>` ``). Stop counting
    // when we hit the next `### ` heading or a blank+heading boundary. The
    // header and separator rows don't have backticks, so the predicate
    // naturally skips them. Sentinel rows like `| - | (no commits) |` also
    // skip (no backtick); commits_count = 0 in that case.
    let mut in_commits = false;
    let mut commits_count = 0u32;
    for line in body {
        if line.trim() == "### Commits" {
            in_commits = true;
            continue;
        }
        if line.starts_with("### ") && line.trim() != "### Commits" {
            in_commits = false;
            continue;
        }
        if in_commits && line.starts_with("| `") {
            commits_count += 1;
        }
    }

    Some(ParsedEntry {
        session_number,
        title,
        date,
        kind_label,
        slug,
        branch,
        commits_count,
    })
}

// -- File helpers -----------------------------------------------------------

/// Find the highest-numbered `journal-N.md` in the developer dir. Numeric
/// sort, not lexical (`journal-10` > `journal-9`). Returns `(journal-1.md, 1)`
/// when the dev dir is missing or holds no `journal-*.md` files.
pub fn find_active(layout: &Layout, dev: &str) -> Result<(PathBuf, u32)> {
    let dir = layout.workspace_developer_dir(dev);
    if !dir.exists() {
        return Ok((layout.workspace_journal(dev, 1), 1));
    }

    let mut highest: Option<(PathBuf, u32)> = None;
    for entry in dir.list_dir()? {
        let entry = entry.map_err(|e| Error::io(&dir, e))?;
        let Some(n) = parse_journal_index(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        let current = highest.as_ref().map_or(0, |(_, h)| *h);
        if n > current {
            highest = Some((entry.path(), n));
        }
    }

    Ok(highest.unwrap_or_else(|| (layout.workspace_journal(dev, 1), 1)))
}

pub(super) fn parse_journal_index(filename: &str) -> Option<u32> {
    let stem = filename.strip_suffix(".md")?;
    let n_str = stem.strip_prefix("journal-")?;
    n_str.parse().ok()
}

/// Count newline-separated lines in the file. Missing file → 0.
pub fn line_count(path: &Path) -> Result<u32> {
    let Some(text) = path.read_text_optional()? else {
        return Ok(0);
    };
    Ok(text.lines().count() as u32)
}

/// Highest session number across all `journal-*.md` files in the developer
/// dir, scanning from the newest journal backward up to C-21's journal cap.
/// Returns `0` when the dev dir is missing or holds no parseable entries.
///
/// Scans newest-first so long-lived workspaces (>100 journals) still see
/// their latest session number — counting from the oldest journals would
/// stop at session 100×N and cause new entries to reuse numbers.
pub fn scan_session_count(layout: &Layout, dev: &str) -> Result<u32> {
    let dir = layout.workspace_developer_dir(dev);
    if !dir.exists() {
        return Ok(0);
    }

    let mut paths: Vec<(u32, PathBuf)> = Vec::new();
    for entry in dir.list_dir()? {
        let entry = entry.map_err(|e| Error::io(&dir, e))?;
        if let Some(n) = parse_journal_index(&entry.file_name().to_string_lossy()) {
            paths.push((n, entry.path()));
        }
    }
    paths.sort_by_key(|(n, _)| std::cmp::Reverse(*n));

    let mut highest = 0u32;
    for (_, path) in paths.iter().take(super::index::INDEX_RERENDER_JOURNAL_CAP) {
        let text = path.read_text()?;
        for entry in parse_entries(&text)
            .into_iter()
            .take(super::index::INDEX_RERENDER_ENTRIES_PER_JOURNAL_CAP)
        {
            if entry.session_number > highest {
                highest = entry.session_number;
            }
        }
    }
    Ok(highest)
}

/// Seed `journal-N.md` from the embedded template.
pub fn seed_journal(path: &Path, dev: &str, n: u32, date: NaiveDate) -> Result<()> {
    let body = format!(
        "# Journal — {dev} (Part {n})\n\n> AI development session journal. Started: \
         {date}\n\n---\n\n"
    );
    path.write_bytes(body.as_bytes())
}

// -- `git log --oneline` parser --------------------------------------------

/// Parse `git log --oneline` output. Each non-empty line splits at the first
/// ASCII space: prefix is `short`, suffix (trimmed) is `message`. Empty
/// lines are skipped. Caller caps via `-n 20`.
pub(crate) fn parse_oneline(stdout: &str) -> Vec<JournalCommit> {
    stdout
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            match line.split_once(' ') {
                Some((short, msg)) => Some(JournalCommit {
                    short: short.to_string(),
                    message: msg.trim().to_string(),
                }),
                None => Some(JournalCommit {
                    short: line.to_string(),
                    message: String::new(),
                }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// V-UT-4: golden render.
    #[test]
    fn render_entry_golden_task() {
        let entry = JournalEntry {
            session_number: 1,
            title: "First session".to_string(),
            date: date("2026-04-29"),
            kind: JournalKind::Task {
                slug: "demo".to_string(),
            },
            branch: Some("feat/demo".to_string()),
            summary: "Did the thing.".to_string(),
            commits: vec![JournalCommit {
                short: "abc1234".to_string(),
                message: "feat: thing".to_string(),
            }],
            next_steps: vec!["next step".to_string()],
        };
        let rendered = render_entry(&entry);
        let expected = "\n## Session 1: First session\n\n**Date**: 2026-04-29\n**Kind**: \
                        task\n**Slug**: demo\n**Branch**: `feat/demo`\n\n### Summary\nDid the \
                        thing.\n\n### Commits\n\n| Hash | Message |\n|------|---------|\n| \
                        `abc1234` | feat: thing |\n\n### Next Steps\n- next step\n";
        assert_eq!(rendered, expected);
    }

    /// V-UT-4: manual render uses `-` slug and "manual" label.
    #[test]
    fn render_entry_manual_uses_dash_slug() {
        let entry = JournalEntry {
            session_number: 5,
            title: "Manual one".to_string(),
            date: date("2026-04-29"),
            kind: JournalKind::Manual,
            branch: None,
            summary: String::new(),
            commits: Vec::new(),
            next_steps: Vec::new(),
        };
        let r = render_entry(&entry);
        assert!(r.contains("**Kind**: manual"));
        assert!(r.contains("**Slug**: -"));
        assert!(r.contains("**Branch**: `unknown`"));
        assert!(r.contains("(no summary provided)"));
        assert!(r.contains("| - | (no commits) |"));
        assert!(r.contains("- (none)"));
    }

    /// V-UT-5: round-trip render → parse.
    #[test]
    fn parse_entries_round_trips() {
        let e1 = JournalEntry {
            session_number: 1,
            title: "first".into(),
            date: date("2026-04-01"),
            kind: JournalKind::Task { slug: "a".into() },
            branch: Some("feat/a".into()),
            summary: "s1".into(),
            commits: vec![JournalCommit {
                short: "abc1234".into(),
                message: "m1".into(),
            }],
            next_steps: vec!["n1".into()],
        };
        let e2 = JournalEntry {
            session_number: 2,
            title: "second".into(),
            date: date("2026-04-02"),
            kind: JournalKind::Manual,
            branch: None,
            summary: "s2".into(),
            commits: Vec::new(),
            next_steps: Vec::new(),
        };
        let text = format!("# header\n\n{}{}", render_entry(&e1), render_entry(&e2));
        let parsed = parse_entries(&text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].session_number, 1);
        assert_eq!(parsed[0].title, "first");
        assert_eq!(parsed[0].kind_label, "task");
        assert_eq!(parsed[0].slug.as_deref(), Some("a"));
        assert_eq!(parsed[0].branch, "feat/a");
        assert_eq!(parsed[0].commits_count, 1);
        assert_eq!(parsed[1].session_number, 2);
        assert_eq!(parsed[1].kind_label, "manual");
        assert!(parsed[1].slug.is_none());
        assert_eq!(parsed[1].branch, "unknown");
        assert_eq!(parsed[1].commits_count, 0);
    }

    /// V-UT-5 / C-19: backtick fences hide trick `## Session` headers.
    #[test]
    fn parse_entries_respects_backtick_fences() {
        let text = "\n## Session 1: real\n\n**Date**: 2026-04-01\n**Kind**: manual\n**Slug**: \
                    -\n**Branch**: `b`\n\n### Summary\n```\n## Session 999: trick\n```\n\n### \
                    Commits\n| Hash | Message |\n|------|---------|\n| - | (no commits) |\n\n### \
                    Next Steps\n- (none)\n";
        let parsed = parse_entries(text);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].session_number, 1);
    }

    /// V-E-8 / C-19: tilde fences hide trick headers.
    #[test]
    fn parse_entries_respects_tilde_fences() {
        let text = "\n## Session 1: real\n\n**Date**: 2026-04-01\n**Kind**: manual\n**Slug**: \
                    -\n**Branch**: `b`\n\n### Summary\n~~~\n## Session 999: trick\n~~~\n\n### \
                    Commits\n| - | (no commits) |\n\n### Next Steps\n- (none)\n";
        let parsed = parse_entries(text);
        assert_eq!(parsed.len(), 1, "tilde fence should hide the trick header");
    }

    /// C-19: 4-backtick block containing a 3-backtick line stays one fence.
    #[test]
    fn parse_entries_length_aware_close_fence() {
        let text = "\n## Session 1: real\n\n**Date**: 2026-04-01\n**Kind**: manual\n**Slug**: \
                    -\n**Branch**: `b`\n\n### Summary\n````\n## Session 999: trick\n```\nstill \
                    inside\n````\n\n### Commits\n| - | (no commits) |\n\n### Next Steps\n- \
                    (none)\n";
        let parsed = parse_entries(text);
        assert_eq!(parsed.len(), 1);
    }

    /// V-UT-6: find_active picks numerically highest.
    #[test]
    fn find_active_picks_highest_n() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let dev = "alice";
        let dir = layout.workspace_developer_dir(dev);
        dir.ensure_dir().unwrap();
        for n in [1u32, 2, 9, 10] {
            layout.workspace_journal(dev, n).write_bytes(b"x").unwrap();
        }
        let (_, n) = find_active(&layout, dev).unwrap();
        assert_eq!(n, 10, "10 must beat 9 lexically");
    }

    /// V-UT-6: returns (journal-1, 1) when no journal files exist yet.
    #[test]
    fn find_active_returns_one_when_dev_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let (path, n) = find_active(&layout, "newbie").unwrap();
        assert_eq!(n, 1);
        assert_eq!(path, layout.workspace_journal("newbie", 1));
    }

    /// V-UT-7: line_count exact.
    #[test]
    fn line_count_returns_exact() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("j.md");
        p.write_bytes(b"a\nb\nc\n").unwrap();
        assert_eq!(line_count(&p).unwrap(), 3);
    }

    /// V-UT-12 / R-002: parse_oneline strips and skips blanks.
    #[test]
    fn parse_oneline_strips_and_skips_blanks() {
        let input = "abc1234 first\n\nfedcba9 second\n";
        let got = parse_oneline(input);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].short, "abc1234");
        assert_eq!(got[0].message, "first");
        assert_eq!(got[1].short, "fedcba9");
        assert_eq!(got[1].message, "second");
    }

    #[test]
    fn parse_oneline_handles_hash_only_line() {
        let got = parse_oneline("abc1234\n");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].short, "abc1234");
        assert_eq!(got[0].message, "");
    }

    /// Pipe in commit message gets escaped on render.
    #[test]
    fn render_escapes_pipe_in_commit_message() {
        let entry = JournalEntry {
            session_number: 1,
            title: "x".into(),
            date: date("2026-04-29"),
            kind: JournalKind::Manual,
            branch: None,
            summary: "s".into(),
            commits: vec![JournalCommit {
                short: "abc".into(),
                message: "feat: a | b".into(),
            }],
            next_steps: Vec::new(),
        };
        let r = render_entry(&entry);
        assert!(r.contains("`abc` | feat: a \\| b"));
    }
}
