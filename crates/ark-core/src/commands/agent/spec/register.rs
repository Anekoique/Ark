//! `ark agent spec register` — upsert a row in `specs/features/INDEX.md`'s
//! `ARK:FEATURES` managed block.
//!
//! `feature` and `scope` are trimmed, then rejected if empty or containing
//! `|` (markdown table separator) or any newline character.

use std::{fmt, path::PathBuf};

use chrono::NaiveDate;

use crate::{
    error::{Error, Result},
    io::{PathExt, read_managed_block, update_managed_block},
    layout::{FEATURES_MARKER, Layout},
};

#[derive(Debug, Clone)]
pub struct SpecRegisterOptions {
    pub project_root: PathBuf,
    pub feature: String,
    pub scope: String,
    pub from_task: String,
    pub date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct SpecRegisterSummary {
    pub feature: String,
    pub was_update: bool,
}

impl fmt::Display for SpecRegisterSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let verb = if self.was_update {
            "updated"
        } else {
            "registered"
        };
        write!(f, "{verb} feature `{}` in features INDEX", self.feature)
    }
}

pub fn spec_register(opts: SpecRegisterOptions) -> Result<SpecRegisterSummary> {
    let feature = sanitize_field("feature", &opts.feature)?;
    let scope = sanitize_field("scope", &opts.scope)?;
    let from_task = sanitize_field("from_task", &opts.from_task)?;

    let layout = Layout::new(&opts.project_root);
    let index_path = layout.specs_features_index();
    if let Some(parent) = index_path.parent() {
        parent.ensure_dir()?;
    }

    let existing_body = read_managed_block(&index_path, FEATURES_MARKER)?.unwrap_or_default();
    let (new_body, was_update) =
        upsert_row(&existing_body, &feature, &scope, &from_task, opts.date);
    update_managed_block(&index_path, FEATURES_MARKER, &new_body)?;

    Ok(SpecRegisterSummary {
        feature: opts.feature,
        was_update,
    })
}

/// Trim `raw` and reject empty / pipe / newline — all of which would corrupt
/// the markdown table row.
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
    Err(Error::InvalidSpecField {
        field: name.to_string(),
        reason,
    })
}

const HEADER: [&str; 2] = [
    "| Feature | Scope | Promoted |",
    "|---------|-------|----------|",
];

/// Upsert a markdown table row for `feature` in `body`. Preserves any existing
/// rows for other features. Returns `(new_body, was_update)`.
fn upsert_row(
    body: &str,
    feature: &str,
    scope: &str,
    from_task: &str,
    date: NaiveDate,
) -> (String, bool) {
    let new_row = format!(
        "| `{feature}` | {scope} | {} from task `{from_task}` |",
        date.format("%Y-%m-%d")
    );
    let row_prefix = format!("| `{feature}` |");

    let trimmed = body.trim();
    let has_header = trimmed
        .lines()
        .next()
        .is_some_and(|l| l.trim_start().starts_with("| Feature"));

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
    use crate::io::PathExt;

    fn project_with_empty_index(tmp: &std::path::Path) {
        let dir = tmp.join(".ark/specs/features");
        dir.ensure_dir().unwrap();
        dir.join("INDEX.md")
            .write_bytes(
                b"# Feature Specs\n\n<!-- ARK:FEATURES:START -->\n<!-- ARK:FEATURES:END -->\n",
            )
            .unwrap();
    }

    #[test]
    fn registers_fresh_row() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());

        let s = spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature: "oauth".into(),
            scope: "OAuth integration".into(),
            from_task: "oauth".into(),
            date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        })
        .unwrap();
        assert!(!s.was_update);

        let idx = tmp
            .path()
            .join(".ark/specs/features/INDEX.md")
            .read_text()
            .unwrap();
        assert!(idx.contains("| `oauth` | OAuth integration | 2026-04-24 from task `oauth` |"));
    }

    #[test]
    fn updates_existing_row_by_feature() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        let opts = |scope: &str| SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature: "oauth".into(),
            scope: scope.into(),
            from_task: "oauth".into(),
            date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        };
        spec_register(opts("initial")).unwrap();
        let s = spec_register(opts("revised")).unwrap();
        assert!(s.was_update);
        let idx = tmp
            .path()
            .join(".ark/specs/features/INDEX.md")
            .read_text()
            .unwrap();
        assert!(idx.contains("revised"));
        assert!(!idx.contains("initial"));
    }

    #[test]
    fn appends_when_different_feature() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        for (f, sc) in [("oauth", "A"), ("billing", "B")] {
            spec_register(SpecRegisterOptions {
                project_root: tmp.path().to_path_buf(),
                feature: f.into(),
                scope: sc.into(),
                from_task: f.into(),
                date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
            })
            .unwrap();
        }
        let idx = tmp
            .path()
            .join(".ark/specs/features/INDEX.md")
            .read_text()
            .unwrap();
        assert!(idx.contains("oauth"));
        assert!(idx.contains("billing"));
    }

    #[test]
    fn rejects_pipe_in_feature() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        let err = spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature: "oa|uth".into(),
            scope: "ok".into(),
            from_task: "oauth".into(),
            date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidSpecField { .. }));
    }

    #[test]
    fn rejects_newline_in_scope() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        let err = spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature: "oauth".into(),
            scope: "line1\nline2".into(),
            from_task: "oauth".into(),
            date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidSpecField { .. }));
    }

    #[test]
    fn rejects_empty_feature() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        let err = spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature: "   ".into(),
            scope: "ok".into(),
            from_task: "oauth".into(),
            date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidSpecField { .. }));
    }

    #[test]
    fn errors_on_corrupt_managed_block() {
        let tmp = tempfile::tempdir().unwrap();
        let idx = tmp.path().join(".ark/specs/features/INDEX.md");
        idx.parent().unwrap().ensure_dir().unwrap();
        idx.write_bytes(b"# features\n\n<!-- ARK:FEATURES:START -->\nno-end-here\n")
            .unwrap();

        let err = spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature: "oauth".into(),
            scope: "ok".into(),
            from_task: "oauth".into(),
            date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::ManagedBlockCorrupt { .. }));
    }
}
