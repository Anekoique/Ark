//! Registers a feature SPEC in the feature index.
//!
//! Upserts a row in `specs/features/INDEX.md`'s `ARK:FEATURES` managed block.
//!
//! `feature` and `scope` are trimmed, then rejected if empty or containing
//! `|` (markdown table separator) or any newline character.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;

use crate::{
    error::{Error, Result},
    io::{PathExt, read_managed_block, update_managed_block},
    layout::{FEATURES_MARKER, Layout},
};

/// Options for registering a feature SPEC in the feature index.
#[derive(Debug, Clone)]
pub struct SpecRegisterOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Feature path segments relative to `.ark/specs/features/`. Single-segment
    /// `["slug"]` reproduces the legacy flat layout bit-for-bit.
    pub feature_path: Vec<String>,
    /// Short scope description rendered in the index table.
    pub scope: String,
    /// Task slug that promoted the feature SPEC.
    pub from_task: String,
    /// Promotion date rendered in the index table.
    pub date: NaiveDate,
}

/// Summary of a feature SPEC registration.
#[derive(Debug, Clone)]
pub struct SpecRegisterSummary {
    /// Feature path segments that were registered.
    pub feature_path: Vec<String>,
    /// Reports whether the leaf row was replaced.
    pub was_update: bool,
    /// INDEX files touched, ordered leaf-to-root.
    pub indexes_touched: Vec<PathBuf>,
}

impl fmt::Display for SpecRegisterSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let verb = if self.was_update {
            "updated"
        } else {
            "registered"
        };
        write!(
            f,
            "{verb} feature `{}` in features INDEX (touched {} index file(s))",
            self.feature_path.join("/"),
            self.indexes_touched.len(),
        )
    }
}

/// Registers or updates one feature row in the recursive features INDEX tree.
///
/// Walks leaf-to-root, upserting a row at each level: leaf row points at the
/// SPEC file, branch rows point at the child INDEX. Missing intermediate
/// INDEX files are seeded from `FEATURE_SUBTREE_INDEX.md`.
pub fn spec_register(opts: SpecRegisterOptions) -> Result<SpecRegisterSummary> {
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
    let from_task = sanitize_table_field("from_task", &opts.from_task)?;

    let (was_update, indexes_touched) = upsert_index_rows_leaf_to_root(
        &opts.project_root,
        &segments,
        &scope,
        &from_task,
        opts.date,
    )?;

    Ok(SpecRegisterSummary {
        feature_path: opts.feature_path,
        was_update,
        indexes_touched,
    })
}

/// Upserts rows from leaf to root in the recursive features INDEX tree.
///
/// Shared between `spec_register` (deep-tier promotion) and `spec_import`
/// (brownfield extraction). Inputs are assumed pre-sanitized.
///
/// For segments `[a, b, c]`:
/// - `features/a/b/INDEX.md`: row `c/SPEC.md` (leaf)
/// - `features/a/INDEX.md`:   row `b/INDEX.md` (branch)
/// - `features/INDEX.md`:     row `a/INDEX.md` (branch)
///
/// Returns `(was_update_at_leaf, indexes_touched_leaf_to_root)`. Each missing
/// intermediate INDEX is seeded from `FEATURE_SUBTREE_INDEX.md` before its
/// row is upserted.
pub(crate) fn upsert_index_rows_leaf_to_root(
    project_root: &Path,
    segments: &[String],
    scope: &str,
    from_task: &str,
    date: NaiveDate,
) -> Result<(bool, Vec<PathBuf>)> {
    let layout = Layout::new(project_root);
    let mut indexes_touched = Vec::with_capacity(segments.len());
    let mut leaf_was_update = false;

    for i in (0..segments.len()).rev() {
        let mut parent_dir = layout.specs_features_dir();
        for seg in &segments[..i] {
            parent_dir.push(seg);
        }
        let index_path = parent_dir.join("INDEX.md");
        let child = &segments[i];
        let is_leaf = i == segments.len() - 1;

        ensure_subtree_index(&index_path)?;

        // Leaf rows render with the bare segment so single-segment paths
        // preserve the legacy flat on-disk shape bit-for-bit; branch rows
        // carry the `<seg>/INDEX.md` suffix so the gather walker's
        // `strip_suffix("/INDEX.md")` discriminator descends into the
        // subtree.
        let row_feature = if is_leaf {
            child.clone()
        } else {
            format!("{child}/INDEX.md")
        };

        let existing_body = read_managed_block(&index_path, FEATURES_MARKER)?.unwrap_or_default();
        let (new_body, was_update) =
            upsert_row_with_target(&existing_body, &row_feature, "", scope, from_task, date);
        update_managed_block(&index_path, FEATURES_MARKER, &new_body)?;

        if is_leaf {
            leaf_was_update = was_update;
        }
        indexes_touched.push(index_path);
    }

    Ok((leaf_was_update, indexes_touched))
}

/// Seeds an `INDEX.md` from the embedded subtree template if missing.
///
/// The root `features/INDEX.md` is shipped by `ark init`; only subtree
/// `INDEX.md` files need lazy creation here. Existing files are left alone
/// (the managed-block helper handles row upsert).
fn ensure_subtree_index(index_path: &Path) -> Result<()> {
    if let Some(parent) = index_path.parent() {
        parent.ensure_dir()?;
    }
    if !index_path.exists() {
        index_path.write_bytes(crate::templates::FEATURE_SUBTREE_INDEX_MD.as_bytes())?;
    }
    Ok(())
}

/// Sanitizes a markdown table field.
///
/// Rejects empty, pipe, and newline values because all would corrupt the
/// markdown table row. Shared with `spec_import` so both verbs apply the
/// same boundary validation.
pub(crate) fn sanitize_table_field(name: &'static str, raw: &str) -> Result<String> {
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

/// Upserts a markdown table row for `feature` in `body`. The `target` field
/// is rendered into the row body (currently the SPEC scope column carries
/// the textual scope, but readers can derive the on-disk target from the
/// feature name + the INDEX's own location).
///
/// Preserves rows for other features. Returns `(new_body, was_update)`.
fn upsert_row_with_target(
    body: &str,
    feature: &str,
    _target: &str,
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
            feature_path: vec!["oauth".to_string()],
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
            feature_path: vec!["oauth".to_string()],
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
                feature_path: vec![f.to_string()],
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
            feature_path: vec!["oa|uth".to_string()],
            scope: "ok".into(),
            from_task: "oauth".into(),
            date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidSpecField { .. }));
    }

    /// Nested register produces a branch row whose first cell carries the
    /// `<seg>/INDEX.md` discriminator, and a leaf row at the subtree INDEX
    /// with a bare-slug first cell. Verifies the row text that the gather
    /// walker keys off.
    #[test]
    fn nested_register_writes_branch_discriminator_row() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());

        spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature_path: vec!["xemu".to_string(), "csr".to_string()],
            scope: "CSR file".into(),
            from_task: "csr".into(),
            date: NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
        })
        .unwrap();

        // Root features INDEX gains a branch row `xemu/INDEX.md`.
        let root_idx = tmp
            .path()
            .join(".ark/specs/features/INDEX.md")
            .read_text()
            .unwrap();
        assert!(
            root_idx.contains("| `xemu/INDEX.md` |"),
            "root INDEX must carry branch discriminator; got:\n{root_idx}",
        );

        // Subtree INDEX exists and has a leaf row (bare slug).
        let subtree_idx = tmp
            .path()
            .join(".ark/specs/features/xemu/INDEX.md")
            .read_text()
            .unwrap();
        assert!(
            subtree_idx.contains("| `csr` |"),
            "subtree INDEX must carry bare-slug leaf row; got:\n{subtree_idx}",
        );
        assert!(
            !subtree_idx.contains("| `csr/SPEC.md` |"),
            "leaf rows must be bare-slug so single-segment paths preserve the legacy flat layout",
        );
    }

    /// Single-segment register preserves the legacy flat layout bit-for-bit
    /// (no branch discriminator, no subtree INDEX).
    #[test]
    fn single_segment_register_preserves_flat_layout() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());

        spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature_path: vec!["klib".to_string()],
            scope: "shared lib".into(),
            from_task: "klib".into(),
            date: NaiveDate::from_ymd_opt(2026, 5, 18).unwrap(),
        })
        .unwrap();

        let idx = tmp
            .path()
            .join(".ark/specs/features/INDEX.md")
            .read_text()
            .unwrap();
        assert!(idx.contains("| `klib` |"), "got:\n{idx}");
        assert!(
            !idx.contains("INDEX.md` |"),
            "single-segment must not produce a branch discriminator",
        );
    }

    #[test]
    fn rejects_newline_in_scope() {
        let tmp = tempfile::tempdir().unwrap();
        project_with_empty_index(tmp.path());
        let err = spec_register(SpecRegisterOptions {
            project_root: tmp.path().to_path_buf(),
            feature_path: vec!["oauth".to_string()],
            scope: "line1\nline2".to_string(),
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
            feature_path: vec!["   ".to_string()],
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
            feature_path: vec!["oauth".to_string()],
            scope: "ok".into(),
            from_task: "oauth".into(),
            date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::ManagedBlockCorrupt { .. }));
    }
}
