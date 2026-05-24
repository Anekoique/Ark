//! Parser for the PRD's `[**SPEC Path**]` block.
//!
//! Deep-tier `task commit` reads the latest PRD and calls
//! [`parse_spec_path`] to determine where in `.ark/specs/features/` the
//! extracted SPEC will land. The block is required on deep tier; a missing
//! or malformed body errors before any commit-time mutation.

use std::path::Path;

use crate::{
    error::{Error, Result},
    layout::{Layout, validate_feature_path_segment},
};

const SECTION_HEADER: &str = "[**SPEC Path**]";

/// Parses the PRD's `[**SPEC Path**]` body into validated kebab-case segments.
///
/// The body must be a single non-empty line carrying one token — bare or
/// backtick-quoted. Examples: `xemu/csr`, `` `klib` ``, `core/runtime/scheduler`.
/// The last segment must equal `slug` (new feature SPEC) **or** the parsed
/// path must point at an existing on-disk feature SPEC (in-place update by a
/// task whose slug names the change, not the feature).
///
/// # Errors
///
/// - [`Error::FeaturePathMissing`] — block absent or empty.
/// - [`Error::InvalidFeaturePath`] — multi-token body, bad alphabet, slug
///   mismatch with no existing SPEC, reserved segment, or `.` / `..` segment.
///   `value` carries the offending substring verbatim.
pub fn parse_spec_path(
    prd_text: &str,
    slug: &str,
    prd_path: &Path,
    layout: &Layout,
) -> Result<Vec<String>> {
    let body = locate_section(prd_text).ok_or_else(|| Error::FeaturePathMissing {
        source_path: prd_path.to_path_buf(),
    })?;

    // Distinguish "empty body" (missing) from "multi-token body" (invalid).
    let token = match extract_single_token(body) {
        SingleToken::Found(t) => t,
        SingleToken::Empty => {
            return Err(Error::FeaturePathMissing {
                source_path: prd_path.to_path_buf(),
            });
        }
        SingleToken::MultiToken(raw) => {
            return Err(Error::InvalidFeaturePath {
                source_path: prd_path.to_path_buf(),
                value: raw,
                reason: "body must be a single token",
            });
        }
    };

    let bare = strip_backticks(token).ok_or_else(|| Error::InvalidFeaturePath {
        source_path: prd_path.to_path_buf(),
        value: token.to_string(),
        reason: "body must be a single bare or backticked token",
    })?;

    let segments: Vec<&str> = bare.split('/').collect();
    for seg in &segments {
        validate_feature_path_segment(seg, prd_path)?;
    }

    // Accept either (a) the canonical "new feature, slug names it" case, or
    // (b) the in-place-update case where the path points at an existing
    // on-disk feature SPEC and the task slug describes the change rather than
    // the feature itself (e.g. `improve-ark-context` updates `ark-context`).
    if segments.last() != Some(&slug) {
        let owned_segments: Vec<String> = segments.iter().map(|s| s.to_string()).collect();
        let segment_refs: Vec<&str> = owned_segments.iter().map(String::as_str).collect();
        let existing = layout
            .specs_feature_dir(&segment_refs, prd_path)
            .map(|dir| dir.join("SPEC.md").is_file())
            .unwrap_or(false);
        if !existing {
            return Err(Error::InvalidFeaturePath {
                source_path: prd_path.to_path_buf(),
                value: bare.to_string(),
                reason: "last segment must equal task slug or name an existing feature SPEC",
            });
        }
    }

    Ok(segments.into_iter().map(str::to_string).collect())
}

/// Returns the body text after the `[**SPEC Path**]` header line up to the
/// next bracketed section header or EOF. Inline mentions inside prose do not
/// anchor the section.
fn locate_section(text: &str) -> Option<&str> {
    let mut header_end: Option<usize> = None;
    let mut idx = 0usize;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with(SECTION_HEADER) {
            header_end = Some(idx + line.len());
            break;
        }
        idx += line.len();
    }
    let body_start = header_end?;
    let body = &text[body_start..];

    let mut sub = 0usize;
    for line in body.split_inclusive('\n') {
        if is_section_header_line(line.trim_start()) {
            return Some(&body[..sub]);
        }
        sub += line.len();
    }
    Some(body)
}

fn is_section_header_line(line: &str) -> bool {
    line.starts_with("[**") && line.contains("**]")
}

/// Result classes for the single-token parse: missing-vs-invalid is
/// distinguished so the caller can pick `FeaturePathMissing` (empty body)
/// vs `InvalidFeaturePath { reason: "body must be a single token" }`
/// (multi-token body) per C-1.
enum SingleToken<'a> {
    Found(&'a str),
    Empty,
    MultiToken(String),
}

/// Returns the single non-blank, non-placeholder token in the section body,
/// or a typed signal for empty / multi-token bodies.
///
/// Placeholders are `<...>`-bracketed prompts that ship with the template.
/// They are filtered out so an unedited PRD raises `FeaturePathMissing`,
/// not a fake `InvalidFeaturePath`.
fn extract_single_token(body: &str) -> SingleToken<'_> {
    let mut found: Option<&str> = None;
    for raw_line in body.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('<') {
            continue;
        }
        if let Some(prev) = found {
            return SingleToken::MultiToken(format!("{prev} {line}"));
        }
        let mut parts = line.split_whitespace();
        let first = match parts.next() {
            Some(t) => t,
            None => continue,
        };
        if parts.next().is_some() {
            return SingleToken::MultiToken(line.to_string());
        }
        found = Some(first);
    }
    match found {
        Some(t) => SingleToken::Found(t),
        None => SingleToken::Empty,
    }
}

fn strip_backticks(token: &str) -> Option<&str> {
    if let Some(inner) = token.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
        if inner.is_empty() {
            return None;
        }
        return Some(inner);
    }
    if token.contains('`') {
        return None;
    }
    Some(token)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn prd() -> PathBuf {
        PathBuf::from("/p/.ark/tasks/x/PRD.md")
    }

    /// Stub layout pointing at an unused tempdir; tests that exercise the
    /// in-place-update fallback create their own layout with the SPEC on disk.
    fn stub_layout() -> Layout {
        Layout::new(std::env::temp_dir().join("ark-prd-stub-layout"))
    }

    /// Single-segment bare path parses and round-trips.
    #[test]
    fn parses_single_segment() {
        let body = "[**What**]\nfoo\n\n[**SPEC Path**]\n\nklib\n\n[**Outcome**]\nx\n";
        let got = parse_spec_path(body, "klib", &prd(), &stub_layout()).unwrap();
        assert_eq!(got, vec!["klib".to_string()]);
    }

    /// Two- and three-segment bare paths parse.
    #[test]
    fn parses_nested_segments() {
        let body = "[**SPEC Path**]\n\nxemu/csr\n";
        assert_eq!(
            parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap(),
            vec!["xemu".to_string(), "csr".to_string()],
        );
        let body = "[**SPEC Path**]\n\ncore/runtime/scheduler\n";
        assert_eq!(
            parse_spec_path(body, "scheduler", &prd(), &stub_layout()).unwrap(),
            vec![
                "core".to_string(),
                "runtime".to_string(),
                "scheduler".to_string(),
            ],
        );
    }

    /// Backtick-quoted form parses identically to bare.
    #[test]
    fn parses_backticked_form() {
        let body = "[**SPEC Path**]\n\n`xemu/csr`\n";
        assert_eq!(
            parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap(),
            vec!["xemu".to_string(), "csr".to_string()],
        );
    }

    /// Missing block → FeaturePathMissing.
    #[test]
    fn missing_block_errors() {
        let body = "[**What**]\nfoo\n\n[**Why**]\nbar\n";
        let err = parse_spec_path(body, "x", &prd(), &stub_layout()).unwrap_err();
        assert!(matches!(err, Error::FeaturePathMissing { .. }));
    }

    /// Empty body (only the placeholder template prompt) → FeaturePathMissing.
    #[test]
    fn empty_or_placeholder_body_errors() {
        let body = "[**SPEC Path**]\n\n\n[**Outcome**]\nx\n";
        let err = parse_spec_path(body, "x", &prd(), &stub_layout()).unwrap_err();
        assert!(matches!(err, Error::FeaturePathMissing { .. }));

        let body = "[**SPEC Path**]\n\n<path relative to specs/features/...>\n";
        let err = parse_spec_path(body, "x", &prd(), &stub_layout()).unwrap_err();
        assert!(matches!(err, Error::FeaturePathMissing { .. }));
    }

    /// Multi-line body (two tokens on separate lines) → InvalidFeaturePath
    /// with reason "body must be a single token" (V-003 closure).
    #[test]
    fn multi_line_body_errors() {
        let body = "[**SPEC Path**]\n\nxemu/csr\nklib\n";
        let err = parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap_err();
        let Error::InvalidFeaturePath { reason, .. } = err else {
            panic!("expected InvalidFeaturePath, got {err:?}");
        };
        assert!(reason.contains("single token"), "reason: {reason}");
    }

    /// Multi-token body on one line → InvalidFeaturePath with reason
    /// "body must be a single token" (V-003 closure).
    #[test]
    fn multi_token_body_errors() {
        let body = "[**SPEC Path**]\n\n`xemu/csr` `klib`\n";
        let err = parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap_err();
        let Error::InvalidFeaturePath { reason, .. } = err else {
            panic!("expected InvalidFeaturePath, got {err:?}");
        };
        assert!(reason.contains("single token"), "reason: {reason}");
    }

    /// Slug mismatch is reported with the bad value quoted.
    #[test]
    fn slug_mismatch_errors() {
        let body = "[**SPEC Path**]\n\nxemu/csr\n";
        let err = parse_spec_path(body, "foo", &prd(), &stub_layout()).unwrap_err();
        let Error::InvalidFeaturePath { value, reason, .. } = err else {
            panic!("wrong variant");
        };
        assert_eq!(value, "xemu/csr");
        assert!(reason.contains("last segment"));
    }

    /// Uppercase segment is rejected by the alphabet rule.
    #[test]
    fn uppercase_segment_rejected() {
        let body = "[**SPEC Path**]\n\nXemu/csr\n";
        let err = parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap_err();
        assert!(matches!(err, Error::InvalidFeaturePath { .. }));
    }

    /// Leading hyphen rejected.
    #[test]
    fn leading_hyphen_segment_rejected() {
        let body = "[**SPEC Path**]\n\n-bad/csr\n";
        let err = parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap_err();
        assert!(matches!(err, Error::InvalidFeaturePath { .. }));
    }

    /// Reserved segments `index` / `spec` (any case) are rejected.
    #[test]
    fn reserved_segments_rejected() {
        for body in [
            "[**SPEC Path**]\n\nindex/csr\n",
            "[**SPEC Path**]\n\nINDEX/csr\n",
            "[**SPEC Path**]\n\nspec/csr\n",
            "[**SPEC Path**]\n\nSPEC/csr\n",
        ] {
            let err = parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap_err();
            assert!(
                matches!(err, Error::InvalidFeaturePath { .. }),
                "body: {body:?}"
            );
        }
    }

    /// `.` and `..` segments are rejected.
    #[test]
    fn dot_and_dotdot_segments_rejected() {
        for body in [
            "[**SPEC Path**]\n\n./csr\n",
            "[**SPEC Path**]\n\n../csr\n",
            "[**SPEC Path**]\n\nxemu/./csr\n",
            "[**SPEC Path**]\n\nxemu/../csr\n",
        ] {
            let err = parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap_err();
            assert!(
                matches!(err, Error::InvalidFeaturePath { .. }),
                "body: {body:?}"
            );
        }
    }

    /// Empty segment from a double slash is rejected.
    #[test]
    fn double_slash_rejected() {
        let body = "[**SPEC Path**]\n\nxemu//csr\n";
        let err = parse_spec_path(body, "csr", &prd(), &stub_layout()).unwrap_err();
        assert!(matches!(err, Error::InvalidFeaturePath { .. }));
    }

    /// Section parser does not consume the next bracketed block.
    #[test]
    fn does_not_consume_next_section() {
        let body = "[**SPEC Path**]\n\nklib\n\n[**Outcome**]\n\nfoo bar baz\nmore words\n";
        assert_eq!(
            parse_spec_path(body, "klib", &prd(), &stub_layout()).unwrap(),
            vec!["klib".to_string()],
        );
    }

    /// Inline mention of the header in prose does not anchor parsing.
    #[test]
    fn inline_header_mention_ignored() {
        let body = "\
some prose mentioning [**SPEC Path**] inline
should-not-match

[**SPEC Path**]

real/slug
";
        assert_eq!(
            parse_spec_path(body, "slug", &prd(), &stub_layout()).unwrap(),
            vec!["real".to_string(), "slug".to_string()],
        );
    }

    /// Trailing whitespace is tolerated.
    #[test]
    fn tolerates_trailing_whitespace() {
        let body = "[**SPEC Path**]\n\n  klib  \n";
        assert_eq!(
            parse_spec_path(body, "klib", &prd(), &stub_layout()).unwrap(),
            vec!["klib".to_string()],
        );
    }

    /// A path whose last segment differs from the slug is accepted when an
    /// on-disk feature SPEC already exists at the path (in-place update by a
    /// task whose slug describes the change rather than the feature itself).
    #[test]
    fn accepts_in_place_update_when_feature_spec_exists() {
        use crate::io::PathExt;
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        // Plant `.ark/specs/features/ark-context/SPEC.md` on disk.
        let dir = layout.specs_features_dir().join("ark-context");
        dir.ensure_dir().unwrap();
        dir.join("SPEC.md").write_bytes(b"existing").unwrap();

        let body = "[**SPEC Path**]\n\nark-context\n";
        let got = parse_spec_path(body, "improve-ark-context", &prd(), &layout).unwrap();
        assert_eq!(got, vec!["ark-context".to_string()]);
    }

    /// Slug-mismatched paths whose target is not on disk still fail, and the
    /// error message reflects the relaxed contract.
    #[test]
    fn rejects_slug_mismatch_when_no_existing_spec() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let body = "[**SPEC Path**]\n\nghost\n";
        let err = parse_spec_path(body, "improve-ark-context", &prd(), &layout).unwrap_err();
        match err {
            Error::InvalidFeaturePath { reason, .. } => {
                assert!(
                    reason.contains("name an existing feature SPEC"),
                    "reason: {reason}",
                );
            }
            other => panic!("expected InvalidFeaturePath, got {other:?}"),
        }
    }
}
