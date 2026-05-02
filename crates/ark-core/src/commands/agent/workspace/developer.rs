//! Developer registrar — top-level Active Developers index.
//!
//! Maintains `<!-- ARK:DEVELOPERS:START/END -->` block in
//! `.ark/workspace/index.md`. Rows: `Developer | Last Active | Sessions |
//! Active Journal`. Hand-edits outside the markers are preserved across
//! `register` / `touch` calls (the underlying `update_managed_block` helper
//! splices bodies in place).
//!
//! `developer_register` upserts a fresh row; `developer_touch` rewrites the
//! row's mutable cells (Last Active / Sessions / Active Journal) without
//! changing the developer's position in the table.

use std::{fmt, path::PathBuf};

use chrono::NaiveDate;

use crate::{
    error::{Error, Result},
    io::{PathExt, read_managed_block, update_managed_block},
    layout::{DEVELOPERS_MARKER, Layout},
};

/// Default top-level workspace index body shipped with `ark init`.
///
/// Static prose surrounds the managed `ARK:DEVELOPERS` block. Hand-edits to
/// the prose survive `register` / `touch`.
const TOP_INDEX_PREFACE: &str = "# Workspace Index

> Records of all AI Agent work across all developers in this project.

## Layout

```text
.ark/workspace/
|-- index.md              # this file
+-- <developer>/          # per-developer subtree
    |-- index.md          # personal session-history index
    +-- journal-N.md      # sequential journal files
```

## Active Developers

";

const TOP_INDEX_SUFFIX: &str = "

---

Hand-edits **outside** the `ARK:DEVELOPERS` markers are preserved by
`ark agent workspace developer register` and `developer touch`.
";

const HEADER: [&str; 2] = [
    "| Developer | Last Active | Sessions | Active Journal |",
    "|-----------|-------------|----------|----------------|",
];

/// Options for registering or upserting a developer.
#[derive(Debug, Clone)]
pub struct DeveloperRegisterOptions {
    /// Project root.
    pub project_root: PathBuf,
    /// Developer name (already validated by `Identity::new`).
    pub name: String,
    /// Active journal filename (e.g. `journal-1.md`), relative to the
    /// developer's own directory.
    pub active_journal: String,
    /// Last-active date for the new row.
    pub date: NaiveDate,
    /// Session count to record (0 for fresh registration).
    pub session_count: u32,
}

/// Summary of a developer registration.
#[derive(Debug, Clone)]
pub struct DeveloperRegisterSummary {
    /// Developer name.
    pub name: String,
    /// True when an existing row was replaced.
    pub was_update: bool,
}

impl fmt::Display for DeveloperRegisterSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let verb = if self.was_update {
            "updated"
        } else {
            "registered"
        };
        write!(f, "{verb} developer `{}` in workspace index", self.name)
    }
}

/// Options for touching (refreshing) an existing developer row.
#[derive(Debug, Clone)]
pub struct DeveloperTouchOptions {
    /// Project root.
    pub project_root: PathBuf,
    /// Developer name to touch.
    pub name: String,
    /// New last-active date.
    pub date: NaiveDate,
    /// New session count.
    pub session_count: u32,
    /// New active-journal filename.
    pub active_journal: String,
}

/// Registers a developer in the top-level Active Developers table.
///
/// Idempotent: re-running with the same `name` updates the row in place.
/// Scaffolds `.ark/workspace/index.md` from the embedded preface if absent.
pub fn developer_register(opts: DeveloperRegisterOptions) -> Result<DeveloperRegisterSummary> {
    let name = sanitize_field("name", &opts.name)?;
    let active_journal = sanitize_field("active_journal", &opts.active_journal)?;

    let layout = Layout::new(&opts.project_root);
    let index_path = layout.workspace_index();
    if let Some(parent) = index_path.parent() {
        parent.ensure_dir()?;
    }

    // Scaffold the preface if the file is fresh (no markers yet).
    if !index_path.exists() {
        let scaffold = format!(
            "{}<!-- {DEVELOPERS_MARKER}:START -->\n<!-- {DEVELOPERS_MARKER}:END -->\n{}",
            TOP_INDEX_PREFACE, TOP_INDEX_SUFFIX
        );
        index_path.write_bytes(scaffold.as_bytes())?;
    }

    let existing_body = read_managed_block(&index_path, DEVELOPERS_MARKER)?.unwrap_or_default();
    let (new_body, was_update) = upsert_row(
        &existing_body,
        &name,
        opts.date,
        opts.session_count,
        &active_journal,
    );
    update_managed_block(&index_path, DEVELOPERS_MARKER, &new_body)?;

    Ok(DeveloperRegisterSummary {
        name: opts.name,
        was_update,
    })
}

/// Updates an existing developer row's mutable cells.
///
/// If no row exists for `name`, falls through to `developer_register` (a
/// touch on a missing row creates it).
pub fn developer_touch(opts: DeveloperTouchOptions) -> Result<DeveloperRegisterSummary> {
    developer_register(DeveloperRegisterOptions {
        project_root: opts.project_root,
        name: opts.name,
        active_journal: opts.active_journal,
        date: opts.date,
        session_count: opts.session_count,
    })
}

/// Sanitizes a markdown table field; rejects empty / pipe / newline.
fn sanitize_field(name: &'static str, raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    let reason = if trimmed.is_empty() {
        "must not be empty"
    } else if raw.contains('|') {
        "must not contain `|`"
    } else if raw.contains('\n') || raw.contains('\r') {
        "must not contain newlines"
    } else {
        return Ok(trimmed.to_string());
    };
    Err(Error::InvalidConfigField {
        field: name,
        reason,
    })
}

/// Upserts a developer row inside the managed block body.
///
/// Preserves rows for other developers. Returns `(new_body, was_update)`.
fn upsert_row(
    body: &str,
    name: &str,
    date: NaiveDate,
    session_count: u32,
    active_journal: &str,
) -> (String, bool) {
    let new_row = format!(
        "| `{name}` | {} | {session_count} | `{active_journal}` |",
        date.format("%Y-%m-%d")
    );
    let row_prefix = format!("| `{name}` |");

    let trimmed = body.trim();
    let has_header = trimmed
        .lines()
        .next()
        .is_some_and(|l| l.trim_start().starts_with("| Developer"));

    let (header_lines, data_lines): (Vec<&str>, Vec<&str>) = if has_header {
        let mut it = trimmed.lines();
        let h1 = it.next().unwrap_or("");
        let h2 = it.next().unwrap_or("");
        (vec![h1, h2], it.filter(|l| !l.trim().is_empty()).collect())
    } else {
        (
            HEADER.to_vec(),
            trimmed.lines().filter(|l| !l.trim().is_empty()).collect(),
        )
    };

    let mut replaced = false;
    let body_rows = data_lines.into_iter().map(|line| {
        if line.starts_with(&row_prefix) {
            replaced = true;
            new_row.clone()
        } else {
            line.to_string()
        }
    });

    let mut out: Vec<String> = header_lines.into_iter().map(String::from).collect();
    out.extend(body_rows);
    if !replaced {
        out.push(new_row);
    }
    let mut text = out.join("\n");
    text.push('\n');
    (text, replaced)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(tmp: &std::path::Path, name: &str) -> DeveloperRegisterOptions {
        DeveloperRegisterOptions {
            project_root: tmp.to_path_buf(),
            name: name.into(),
            active_journal: "journal-1.md".into(),
            date: NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            session_count: 1,
        }
    }

    #[test]
    fn register_creates_index_and_row() {
        let tmp = tempfile::tempdir().unwrap();
        let s = developer_register(opts(tmp.path(), "alice")).unwrap();
        assert!(!s.was_update);

        let idx = Layout::new(tmp.path())
            .workspace_index()
            .read_text()
            .unwrap();
        assert!(idx.contains("# Workspace Index"));
        assert!(idx.contains("<!-- ARK:DEVELOPERS:START -->"));
        assert!(idx.contains("<!-- ARK:DEVELOPERS:END -->"));
        assert!(idx.contains("| `alice` | 2026-05-02 | 1 | `journal-1.md` |"));
    }

    #[test]
    fn register_is_idempotent_for_same_developer() {
        let tmp = tempfile::tempdir().unwrap();
        developer_register(opts(tmp.path(), "alice")).unwrap();
        let s = developer_register(opts(tmp.path(), "alice")).unwrap();
        assert!(s.was_update);
        let idx = Layout::new(tmp.path())
            .workspace_index()
            .read_text()
            .unwrap();
        // Single row in the table, not duplicated.
        let count = idx.matches("| `alice` |").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn register_appends_distinct_developers() {
        let tmp = tempfile::tempdir().unwrap();
        developer_register(opts(tmp.path(), "alice")).unwrap();
        developer_register(opts(tmp.path(), "bob")).unwrap();
        let idx = Layout::new(tmp.path())
            .workspace_index()
            .read_text()
            .unwrap();
        assert!(idx.contains("| `alice` |"));
        assert!(idx.contains("| `bob` |"));
    }

    #[test]
    fn touch_updates_mutable_cells() {
        let tmp = tempfile::tempdir().unwrap();
        developer_register(opts(tmp.path(), "alice")).unwrap();
        let s = developer_touch(DeveloperTouchOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
            date: NaiveDate::from_ymd_opt(2026, 5, 3).unwrap(),
            session_count: 5,
            active_journal: "journal-2.md".into(),
        })
        .unwrap();
        assert!(s.was_update);
        let idx = Layout::new(tmp.path())
            .workspace_index()
            .read_text()
            .unwrap();
        assert!(idx.contains("| `alice` | 2026-05-03 | 5 | `journal-2.md` |"));
        assert!(!idx.contains("journal-1.md"));
    }

    #[test]
    fn register_preserves_hand_edits_outside_markers() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.workspace_dir().ensure_dir().unwrap();
        layout
            .workspace_index()
            .write_bytes(
                b"# Custom Heading\n\nUSER PROSE\n\n<!-- ARK:DEVELOPERS:START -->\n<!-- ARK:DEVELOPERS:END -->\n\nMORE USER PROSE\n",
            )
            .unwrap();

        developer_register(opts(tmp.path(), "alice")).unwrap();
        let idx = layout.workspace_index().read_text().unwrap();
        assert!(idx.contains("# Custom Heading"));
        assert!(idx.contains("USER PROSE"));
        assert!(idx.contains("MORE USER PROSE"));
        assert!(idx.contains("| `alice` |"));
    }

    #[test]
    fn rejects_pipe_in_name() {
        let tmp = tempfile::tempdir().unwrap();
        let mut o = opts(tmp.path(), "al|ice");
        o.name = "al|ice".into();
        let err = developer_register(o).unwrap_err();
        assert!(matches!(err, Error::InvalidConfigField { .. }));
    }
}
