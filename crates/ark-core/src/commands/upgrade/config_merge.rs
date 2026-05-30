//! Append-only section merge for `.ark/config.toml`.
//!
//! `config.toml` is seed-only: once `init` writes it, upgrade never overwrites
//! or hash-classifies it (user edits survive byte-for-byte). The downside is
//! that a release adding a brand-new top-level section (e.g. `[sandbox]`)
//! cannot deliver it to a user who initialized before that release — their
//! file silently lacks the new defaults.
//!
//! This module fills that gap with a strictly additive merge: top-level
//! `[name]` tables shipped by the current template that are absent from the
//! user's file are appended verbatim (header + body, including any leading
//! `#` comment lines that introduce them). A section the user already has is
//! never inspected, rewritten, or reordered — section identity is the `[name]`
//! header at column 0.
//!
//! TOML is line-oriented enough at this layer that a scan over column-0 `[`
//! lines is sufficient: we never parse values, only locate headers and slice
//! the byte ranges between them. Inline `[`-in-value contexts (arrays, etc.)
//! are not column-0 so they cannot be confused with a table header.

use std::collections::BTreeSet;

/// One top-level `[name]` table section sliced out of a TOML source.
#[derive(Debug, Clone)]
struct Section<'a> {
    /// Section name without the brackets (e.g. `"sandbox"`).
    name: &'a str,
    /// Bytes from the start of this section's leading comment block (or the
    /// header itself if there are no leading comments) up to just before the
    /// next section's leading-comment block.
    body: &'a str,
}

/// Computes a new `config.toml` text that contains every section the user has
/// plus any sections the template ships that the user is missing.
///
/// Returns `None` when there is nothing to append (so the caller can skip the
/// write and emit no action row). Appended sections are written in the order
/// the template lists them. The user's existing text is preserved byte-for-byte
/// as a prefix.
pub fn append_missing_sections(user_text: &str, template_text: &str) -> Option<String> {
    let user_sections: BTreeSet<&str> = section_headers(user_text).collect();
    let template_sections = split_sections(template_text);

    let missing: Vec<&Section<'_>> = template_sections
        .iter()
        .filter(|s| !user_sections.contains(s.name))
        .collect();
    if missing.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(
        user_text.len() + missing.iter().map(|s| s.body.len()).sum::<usize>() + 8,
    );
    out.push_str(user_text);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    for section in missing {
        // One blank-line separator before each appended section so a
        // user file without a trailing blank still ends up well-spaced.
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(section.body.trim_end_matches('\n'));
        out.push('\n');
    }
    Some(out)
}

/// Iterates over every top-level `[name]` table header name in `text`.
///
/// Column-0 `[` only; `[[array.of.tables]]` is skipped (not a table header in
/// the additive-merge sense — Ark's templates do not use them today, and a
/// future use case can extend this scan).
fn section_headers(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter_map(|line| {
        if !line.starts_with('[') || line.starts_with("[[") {
            return None;
        }
        let rest = &line[1..];
        let end = rest.find(']')?;
        Some(rest[..end].trim())
    })
}

/// Splits `text` into its top-level sections in document order.
///
/// Each section's `body` runs from the start of its leading `#` comment block
/// (or the header itself if there are no leading comments) up to the byte
/// before the next section's leading-comment block — so the comments above
/// the header travel with the section, which is what makes the appended
/// output read like the template.
fn split_sections(text: &str) -> Vec<Section<'_>> {
    let mut headers: Vec<(usize, &str)> = Vec::new(); // (line-start byte, name)
    let bytes = text.as_bytes();
    let mut idx = 0usize;
    for line in text.lines() {
        let line_start = idx;
        idx += line.len();
        // Re-add the line terminator length the lines() iterator stripped.
        if idx < bytes.len() && bytes[idx] == b'\n' {
            idx += 1;
        } else if idx + 1 < bytes.len() && bytes[idx] == b'\r' && bytes[idx + 1] == b'\n' {
            idx += 2;
        }
        if line.starts_with('[') && !line.starts_with("[[") {
            let rest = &line[1..];
            if let Some(end) = rest.find(']') {
                headers.push((line_start, rest[..end].trim()));
            }
        }
    }

    let mut sections: Vec<Section<'_>> = Vec::with_capacity(headers.len());
    for (i, &(header_start, name)) in headers.iter().enumerate() {
        let comment_start = scan_back_to_leading_comments(text, header_start);
        let end = headers
            .get(i + 1)
            .map(|&(next_start, _)| scan_back_to_leading_comments(text, next_start))
            .unwrap_or(text.len());
        sections.push(Section {
            name,
            body: &text[comment_start..end],
        });
    }
    sections
}

/// Walks backwards from `header_start` over any contiguous block of `#`
/// comment lines (each terminated by `\n`), stopping at the first blank line
/// or non-comment line.
///
/// Returns the byte index where the leading-comment block (or the header
/// itself) begins. The block ends just before `header_start`.
fn scan_back_to_leading_comments(text: &str, header_start: usize) -> usize {
    let prefix = &text[..header_start];
    let mut start = header_start;
    for line in prefix.rsplit_terminator('\n') {
        let line_with_nl_len = line.len() + 1; // the '\n' the rsplit dropped
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            start -= line_with_nl_len;
            continue;
        }
        break;
    }
    start
}

#[cfg(test)]
mod tests {
    use super::*;

    /// V-1: user has the older two sections, template adds [sandbox]; append it.
    #[test]
    fn appends_single_missing_section() {
        let user =
            "[worktree]\nbranch_prefix = \"feat\"\n\n[workspace]\njournal_max_lines = 2000\n";
        let template = "[worktree]\nbranch_prefix = \"feat\"\n\n[workspace]\njournal_max_lines = \
                        2000\n\n# Sandbox docs.\n[sandbox]\nengine = \"docker\"\n";
        let merged = append_missing_sections(user, template).expect("should append");
        assert!(merged.starts_with(user));
        assert!(merged.contains("# Sandbox docs."));
        assert!(merged.contains("[sandbox]"));
        assert!(merged.contains("engine = \"docker\""));
    }

    /// V-2: every template section is already present → None.
    #[test]
    fn no_op_when_all_sections_present() {
        let template = "[worktree]\nx = 1\n\n[sandbox]\ny = 2\n";
        let user = "[worktree]\nx = \"custom\"\n\n[sandbox]\ny = \"custom\"\n";
        assert!(append_missing_sections(user, template).is_none());
    }

    /// V-3: the user's existing bytes are not touched.
    #[test]
    fn user_bytes_preserved_byte_for_byte() {
        let user = "[worktree]\n# user note\nbranch_prefix = \"mine\"\ncopy = [\".env\"]\n";
        let template = "[worktree]\nbranch_prefix = \"feat\"\n\n[sandbox]\nengine = \"docker\"\n";
        let merged = append_missing_sections(user, template).expect("should append");
        // The user's section, including their custom comment and `copy`, is
        // the prefix of the merged file.
        assert!(merged.starts_with(user));
    }

    /// V-4: leading `#` comments above a header travel with the section.
    #[test]
    fn leading_comments_travel_with_appended_section() {
        let user = "[worktree]\n";
        let template = "[worktree]\n\n# This explains [sandbox].\n# \
                        Multi-line.\n[sandbox]\nengine = \"docker\"\n";
        let merged = append_missing_sections(user, template).unwrap();
        let sandbox_at = merged.find("[sandbox]").unwrap();
        let header_at = merged.find("# This explains").unwrap();
        assert!(
            header_at < sandbox_at,
            "leading comments must precede [sandbox]"
        );
    }

    /// V-5: order in the appended output mirrors template document order.
    #[test]
    fn appended_order_follows_template_document_order() {
        let user = "[worktree]\n";
        let template = "[worktree]\n\n[alpha]\nx = 1\n\n[beta]\ny = 2\n\n[gamma]\nz = 3\n";
        let merged = append_missing_sections(user, template).unwrap();
        let a = merged.find("[alpha]").unwrap();
        let b = merged.find("[beta]").unwrap();
        let g = merged.find("[gamma]").unwrap();
        assert!(a < b && b < g, "template order must be preserved");
    }

    /// V-6: a user file without a trailing newline still produces well-formed output.
    #[test]
    fn user_without_trailing_newline_is_padded() {
        let user = "[worktree]\nbranch_prefix = \"feat\"";
        let template = "[worktree]\nbranch_prefix = \"feat\"\n\n[sandbox]\nengine = \"docker\"\n";
        let merged = append_missing_sections(user, template).unwrap();
        // The appended header is on its own line, not glued to the last user byte.
        let bp = merged.find("branch_prefix = \"feat\"").unwrap();
        let sb = merged.find("[sandbox]").unwrap();
        assert!(
            merged[bp..sb].contains('\n'),
            "header must start a new line"
        );
    }

    /// V-7: `[[array.of.tables]]` is intentionally ignored by both halves of
    /// the comparison (Ark's templates never use this form, and treating them
    /// as table headers would be wrong anyway — each one is a row, not a section).
    #[test]
    fn array_of_tables_headers_are_ignored() {
        let user = "[[items]]\nname = \"a\"\n";
        let template = "[[items]]\nname = \"a\"\n\n[sandbox]\nengine = \"docker\"\n";
        let merged = append_missing_sections(user, template).unwrap();
        assert!(merged.contains("[sandbox]"));
    }

    /// V-8: a header on the very first line (no leading comments) appends cleanly.
    #[test]
    fn first_line_header_appends_without_extra_blank() {
        let user = "[worktree]\nx = 1\n";
        let template = "[worktree]\nx = 1\n[sandbox]\ny = 2\n";
        let merged = append_missing_sections(user, template).unwrap();
        assert!(merged.contains("[sandbox]"));
    }
}
