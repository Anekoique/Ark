//! Imports a feature SPEC authored from an existing codebase.
//!
//! The brownfield counterpart to [`super::extract::spec_extract`]: instead of
//! pulling a `## Spec` section out of a deep-tier PLAN, the SPEC body is
//! authored externally (typically by `/ark:extract-spec`) and handed to this
//! verb as a file. Writes `specs/features/<slug>/SPEC.md`, stamps a
//! provenance entry into `[**CHANGELOG**]`, and upserts the index row through
//! the same managed-block helper `spec_register` uses.

use std::{fmt, path::PathBuf};

use chrono::NaiveDate;

use crate::{
    commands::agent::spec::register::{sanitize_table_field, upsert_index_rows_leaf_to_root},
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Sentinel `from_task` value recorded in the features INDEX for SPECs that
/// did not originate from a deep-tier task.
pub const EXTRACTED_SENTINEL: &str = "extracted";

/// Options for importing a feature SPEC authored from a codebase.
#[derive(Debug, Clone)]
pub struct SpecImportOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Feature path segments relative to `.ark/specs/features/`; last segment
    /// doubles as the SPEC directory name. Single-segment input reproduces
    /// the legacy flat layout bit-for-bit.
    pub feature_path: Vec<String>,
    /// Short scope rendered into the features index row.
    pub scope: String,
    /// Path to the authored SPEC body (raw, without provenance entry).
    pub from_file: PathBuf,
    /// Short SHA recorded in the provenance CHANGELOG entry.
    pub from_commit: String,
    /// Date used for the CHANGELOG entry and the index row.
    pub date: NaiveDate,
}

/// Summary of a feature SPEC import.
#[derive(Debug, Clone)]
pub struct SpecImportSummary {
    /// Feature slug that was imported.
    pub feature: String,
    /// Path where the SPEC body was written.
    pub spec_path: PathBuf,
    /// Path of the features index updated by the import.
    pub index_path: PathBuf,
}

impl fmt::Display for SpecImportSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "imported feature `{}` at {}",
            self.feature,
            self.spec_path.display()
        )
    }
}

/// Imports an authored SPEC body into `specs/features/<slug>/SPEC.md`.
///
/// # Errors
///
/// Returns [`Error::InvalidSpecField`] when `feature`, `scope`, or
/// `from_commit` fail boundary validation; [`Error::SpecAlreadyExists`] when
/// the target SPEC path already exists; [`Error::Io`] when the source body
/// cannot be read or the target cannot be written.
pub fn spec_import(opts: SpecImportOptions) -> Result<SpecImportSummary> {
    if opts.feature_path.is_empty() {
        return Err(Error::InvalidSpecField {
            field: "feature_path".to_string(),
            reason: "must not be empty",
        });
    }
    let segments: Vec<String> = opts
        .feature_path
        .iter()
        .map(|s| sanitize_table_field("feature_path", s))
        .collect::<Result<_>>()?;
    let scope = sanitize_table_field("scope", &opts.scope)?;
    let from_commit = sanitize_table_field("from_commit", &opts.from_commit)?;

    let layout = Layout::new(&opts.project_root);
    let seg_refs: Vec<&str> = segments.iter().map(String::as_str).collect();
    // Use the features INDEX path as the `source_path` sentinel; import is
    // not PRD-anchored.
    let target_dir = layout.specs_feature_dir(&seg_refs, layout.specs_features_index())?;
    let spec_path = target_dir.join("SPEC.md");
    let feature_name = segments.last().cloned().unwrap_or_default();
    if spec_path.exists() {
        return Err(Error::SpecAlreadyExists {
            feature: feature_name.clone(),
            path: spec_path,
        });
    }

    let body = opts.from_file.read_text()?;
    let composed = compose_with_changelog(&body, &from_commit, opts.date);

    target_dir.ensure_dir()?;
    spec_path.write_bytes(composed.as_bytes())?;

    let _ = upsert_index_rows_leaf_to_root(
        &opts.project_root,
        &segments,
        &scope,
        EXTRACTED_SENTINEL,
        opts.date,
    )?;

    Ok(SpecImportSummary {
        feature: feature_name,
        spec_path,
        index_path: layout.specs_features_index(),
    })
}

/// Composes the SPEC body with a provenance CHANGELOG entry.
///
/// If `body` already has a `[**CHANGELOG**]` section, the entry is inserted
/// at the top of that section. Otherwise the section is appended.
fn compose_with_changelog(body: &str, short_sha: &str, date: NaiveDate) -> String {
    let entry = format!(
        "- `{date}` `extracted`: initial extraction from codebase at `{short_sha}`.",
        date = date.format("%Y-%m-%d"),
    );

    if let Some(section_pos) = find_changelog_section(body) {
        return splice_changelog_entry(body, section_pos, &entry);
    }

    let mut out = body.trim_end().to_string();
    out.push_str("\n\n[**CHANGELOG**]\n\n");
    out.push_str(&entry);
    out.push('\n');
    out
}

/// Locates the start of the `[**CHANGELOG**]` section header in `body`.
///
/// Returns the byte offset of the line containing the header, or `None` when
/// no such section exists.
fn find_changelog_section(body: &str) -> Option<usize> {
    let needle = "[**CHANGELOG**]";
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        if line.trim() == needle {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Inserts `entry` immediately after the `[**CHANGELOG**]` header.
///
/// Inserts a single blank line above the entry when the existing section
/// body is non-empty so the new entry stays readable.
fn splice_changelog_entry(body: &str, header_offset: usize, entry: &str) -> String {
    let header_line_end = body[header_offset..]
        .find('\n')
        .map(|i| header_offset + i + 1)
        .unwrap_or(body.len());
    let (head, tail) = body.split_at(header_line_end);

    let mut out = String::with_capacity(body.len() + entry.len() + 4);
    out.push_str(head);
    out.push('\n');
    out.push_str(entry);
    out.push('\n');
    if !tail.trim_start().is_empty() {
        out.push('\n');
        out.push_str(tail.trim_start_matches('\n'));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 5, 8).unwrap()
    }

    #[test]
    fn appends_changelog_section_when_absent() {
        let body = "[**Goals**]\n\n- G-1: ship\n";
        let composed = compose_with_changelog(body, "abc1234", date());
        assert!(composed.contains("[**CHANGELOG**]"));
        assert!(composed.contains(
            "- `2026-05-08` `extracted`: initial extraction from codebase at `abc1234`."
        ));
        assert!(composed.contains("G-1: ship"));
        let header_pos = composed.find("[**CHANGELOG**]").unwrap();
        let goals_pos = composed.find("[**Goals**]").unwrap();
        assert!(goals_pos < header_pos, "goals must precede changelog");
    }

    #[test]
    fn inserts_entry_at_top_when_section_present() {
        let body = "[**Goals**]\n\n- G-1: ship\n\n[**CHANGELOG**]\n\n- `2026-04-01` `prior`: \
                    earlier note.\n";
        let composed = compose_with_changelog(body, "abc1234", date());
        let entry_pos = composed.find("initial extraction").unwrap();
        let prior_pos = composed.find("earlier note").unwrap();
        assert!(
            entry_pos < prior_pos,
            "new entry must appear above prior entries"
        );
        assert!(composed.contains("earlier note"));
    }

    #[test]
    fn sanitize_rejects_pipe_and_empty() {
        assert!(matches!(
            sanitize_table_field("feature", ""),
            Err(Error::InvalidSpecField { .. })
        ));
        assert!(matches!(
            sanitize_table_field("feature", "a|b"),
            Err(Error::InvalidSpecField { .. })
        ));
        assert!(matches!(
            sanitize_table_field("scope", "line\nbreak"),
            Err(Error::InvalidSpecField { .. })
        ));
    }

    fn project_with_empty_index(tmp: &std::path::Path) {
        let dir = tmp.join(".ark/specs/features");
        dir.ensure_dir().unwrap();
        dir.join("INDEX.md")
            .write_bytes(
                b"# Feature Specs\n\n<!-- ARK:FEATURES:START -->\n<!-- ARK:FEATURES:END -->\n",
            )
            .unwrap();
    }

    fn write_body(tmp: &std::path::Path, body: &str) -> PathBuf {
        let p = tmp.join("body.md");
        p.write_bytes(body.as_bytes()).unwrap();
        p
    }

    #[test]
    fn imports_writes_spec_and_index_row() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        let body = write_body(tmp.path(), "[**Goals**]\n\n- G-1: ship\n");

        let summary = spec_import(SpecImportOptions {
            project_root: tmp.path().to_path_buf(),
            feature_path: vec!["copy-on-write".to_string()],
            scope: "memory subsystem COW".into(),
            from_file: body,
            from_commit: "abc1234".into(),
            date: date(),
        })
        .unwrap();

        let spec = summary.spec_path.read_text().unwrap();
        assert!(spec.contains("G-1: ship"));
        assert!(spec.contains("[**CHANGELOG**]"));
        assert!(spec.contains("`abc1234`"));

        let idx = summary.index_path.read_text().unwrap();
        assert!(idx.contains("| `copy-on-write` |"));
        assert!(idx.contains("from task `extracted` |"));
    }

    #[test]
    fn import_refuses_when_spec_exists() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        let feature_dir = tmp.path().join(".ark/specs/features/cow");
        feature_dir.ensure_dir().unwrap();
        feature_dir
            .join("SPEC.md")
            .write_bytes(b"existing\n")
            .unwrap();
        let body = write_body(tmp.path(), "[**Goals**]\n\n- G-1: x\n");

        let err = spec_import(SpecImportOptions {
            project_root: tmp.path().to_path_buf(),
            feature_path: vec!["cow".to_string()],
            scope: "x".into(),
            from_file: body,
            from_commit: "abc".into(),
            date: date(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::SpecAlreadyExists { .. }));
    }

    #[test]
    fn import_rejects_empty_scope() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        let body = write_body(tmp.path(), "x\n");

        let err = spec_import(SpecImportOptions {
            project_root: tmp.path().to_path_buf(),
            feature_path: vec!["cow".to_string()],
            scope: "   ".into(),
            from_file: body,
            from_commit: "abc".into(),
            date: date(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidSpecField { .. }));
    }

    #[test]
    fn import_errors_on_missing_body_file() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());

        let err = spec_import(SpecImportOptions {
            project_root: tmp.path().to_path_buf(),
            feature_path: vec!["cow".to_string()],
            scope: "x".into(),
            from_file: tmp.path().join("does-not-exist.md"),
            from_commit: "abc".into(),
            date: date(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    #[test]
    fn summary_display_renders_one_line() {
        let s = SpecImportSummary {
            feature: "cow".to_string(),
            spec_path: PathBuf::from("/x/.ark/specs/features/cow/SPEC.md"),
            index_path: PathBuf::from("/x/.ark/specs/features/INDEX.md"),
        };
        let line = format!("{s}");
        assert_eq!(
            line,
            "imported feature `cow` at /x/.ark/specs/features/cow/SPEC.md"
        );
        assert!(!line.contains('\n'));
    }

    #[test]
    fn register_then_import_preserves_existing_row() {
        // Sister-test of `register.rs::appends_when_different_feature` —
        // proves the shared `upsert_index_row` is parity-preserving.
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());

        crate::commands::agent::spec::spec_register(
            crate::commands::agent::spec::SpecRegisterOptions {
                project_root: tmp.path().to_path_buf(),
                feature_path: vec!["billing".to_string()],
                scope: "billing svc".into(),
                from_task: "billing".into(),
                date: date(),
            },
        )
        .unwrap();

        let body = write_body(tmp.path(), "[**Goals**]\n\n- G-1: x\n");
        spec_import(SpecImportOptions {
            project_root: tmp.path().to_path_buf(),
            feature_path: vec!["cow".to_string()],
            scope: "memory cow".into(),
            from_file: body,
            from_commit: "abc1234".into(),
            date: date(),
        })
        .unwrap();

        let idx = tmp
            .path()
            .join(".ark/specs/features/INDEX.md")
            .read_text()
            .unwrap();
        assert!(idx.contains("| `billing` |"));
        assert!(idx.contains("from task `billing` |"));
        assert!(idx.contains("| `cow` |"));
        assert!(idx.contains("from task `extracted` |"));
    }
}
