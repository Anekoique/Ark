//! Parser for the PRD's `[**Related Specs**]` section.
//!
//! Locates and parses the PRD related-specs section.
//!
//! The parser scans forward from `[**Related Specs**]` until the next
//! bracketed section header or EOF. Inside that range, it extracts
//! `specs/features/[a-z0-9_-]+/SPEC.md` tokens and dedupes them while
//! preserving first-seen order.

const SECTION_HEADER: &str = "[**Related Specs**]";
const PATH_PREFIX: &str = "specs/features/";
const PATH_SUFFIX: &str = "/SPEC.md";

/// Extracts related feature-spec paths from PRD text.
///
/// Returns an empty vec if the section is missing, empty, or malformed.
pub fn extract(prd_text: &str) -> Vec<String> {
    let Some(section) = locate_section(prd_text) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for token in scan_paths(section) {
        if !out.iter().any(|t| t == &token) {
            out.push(token);
        }
    }
    out
}

/// Returns the body of the `[**Related Specs**]` section.
///
/// The slice starts after the header line and ends before the next
/// `[**...**]` header line, or at EOF. Inline mentions inside prose do not
/// anchor the section.
fn locate_section(text: &str) -> Option<&str> {
    let mut header_end: Option<usize> = None;
    let mut idx = 0usize;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with(SECTION_HEADER) {
            header_end = Some(idx + line.len());
            break;
        }
        idx += line.len();
    }
    let body_start = header_end?;
    let body = &text[body_start..];

    let mut sub = 0usize;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if is_section_header_line(trimmed) {
            return Some(&body[..sub]);
        }
        sub += line.len();
    }
    Some(body)
}

fn is_section_header_line(line: &str) -> bool {
    // Match `[**...**]` at start of (trimmed) line.
    if !line.starts_with("[**") {
        return false;
    }
    line.contains("**]")
}

/// Scans for `specs/features/<seg>(/<seg>)*<slug>/SPEC.md` substrings.
///
/// Each segment is case-sensitive `[a-z0-9][a-z0-9_-]*`; `/` is the segment
/// separator. The match consumes everything up to and including the trailing
/// `/SPEC.md` suffix.
fn scan_paths(slice: &str) -> Vec<String> {
    let mut found = Vec::new();
    let bytes = slice.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // find next occurrence of "specs/features/" starting at i
        let Some(rel) = slice[i..].find(PATH_PREFIX) else {
            break;
        };
        let start = i + rel;
        let after_prefix = start + PATH_PREFIX.len();
        // Consume `<seg>(/<seg>)*` up to where `/SPEC.md` begins. Each segment
        // must start with a slug byte and contain only slug bytes.
        let mut j = after_prefix;
        loop {
            // segment must start with a slug byte
            if j >= bytes.len() || !is_slug_byte(bytes[j]) {
                break;
            }
            // consume the segment body
            while j < bytes.len() && is_slug_byte(bytes[j]) {
                j += 1;
            }
            // /SPEC.md ends the path
            if slice[j..].starts_with(PATH_SUFFIX) {
                break;
            }
            // otherwise, accept a `/` separator and continue with another segment
            if j < bytes.len() && bytes[j] == b'/' {
                j += 1;
                continue;
            }
            // anything else terminates without a match
            break;
        }
        if j > after_prefix && slice[j..].starts_with(PATH_SUFFIX) {
            let end = j + PATH_SUFFIX.len();
            found.push(slice[start..end].to_string());
            i = end;
        } else {
            // not a match, advance past this prefix occurrence to avoid
            // infinite loops
            i = start + PATH_PREFIX.len();
        }
    }
    found
}

fn is_slug_byte(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_two_valid_paths_within_section() {
        // Fixture: two valid bullets + one stray-outside + one
        // malformed-slug (uppercase) within section.
        let prd = "\
# foo PRD

[**What**]
something

[**Related Specs**]

- `specs/features/foo/SPEC.md` — interaction one
- `specs/features/bar-baz/SPEC.md` — interaction two
- `specs/features/InvalidSlug/SPEC.md` — should be ignored

[**Outcome**]

(stray reference outside the section: specs/features/zzz/SPEC.md)
";
        let got = extract(prd);
        assert_eq!(
            got,
            vec![
                "specs/features/foo/SPEC.md".to_string(),
                "specs/features/bar-baz/SPEC.md".to_string(),
            ]
        );
    }

    #[test]
    fn returns_empty_when_section_missing() {
        let prd = "no related specs section here\n";
        assert!(extract(prd).is_empty());
    }

    #[test]
    fn returns_empty_when_section_empty() {
        let prd = "[**Related Specs**]\n\n[**Outcome**]\nfoo\n";
        assert!(extract(prd).is_empty());
    }

    #[test]
    fn dedupes_repeated_paths() {
        let prd = "\
[**Related Specs**]
- `specs/features/foo/SPEC.md` — first
- `specs/features/foo/SPEC.md` — duplicate
[**Outcome**]
done
";
        assert_eq!(extract(prd), vec!["specs/features/foo/SPEC.md".to_string()]);
    }

    #[test]
    fn handles_section_at_end_of_file() {
        let prd = "[**Related Specs**]\n- `specs/features/foo/SPEC.md`\n";
        assert_eq!(extract(prd), vec!["specs/features/foo/SPEC.md".to_string()]);
    }

    #[test]
    fn rejects_uppercase_slug() {
        let prd = "[**Related Specs**]\n- `specs/features/Foo/SPEC.md`\n";
        assert!(extract(prd).is_empty());
    }

    #[test]
    fn ignores_inline_mention_of_section_header() {
        let prd = "\
some prose mentioning [**Related Specs**] inline
- `specs/features/should-not-match/SPEC.md`

[**Related Specs**]
- `specs/features/real/SPEC.md`
";
        assert_eq!(
            extract(prd),
            vec!["specs/features/real/SPEC.md".to_string()]
        );
    }

    /// Nested PRD paths (`specs/features/xemu/csr/SPEC.md`) parse end-to-end.
    /// Single-segment paths continue to work.
    #[test]
    fn extracts_nested_path() {
        let prd = "[**Related Specs**]\n- `specs/features/xemu/csr/SPEC.md` — note\n";
        assert_eq!(
            extract(prd),
            vec!["specs/features/xemu/csr/SPEC.md".to_string()]
        );
    }

    /// Three-segment nested path parses; multi-segment with explicit
    /// separators between each segment.
    #[test]
    fn extracts_three_segment_nested_path() {
        let prd = "[**Related Specs**]\n- `specs/features/core/runtime/scheduler/SPEC.md`\n";
        assert_eq!(
            extract(prd),
            vec!["specs/features/core/runtime/scheduler/SPEC.md".to_string()]
        );
    }

    /// Mixed flat + nested paths in the same section both surface, in
    /// first-seen order, deduped.
    #[test]
    fn extracts_mixed_flat_and_nested() {
        let prd = "\
[**Related Specs**]
- `specs/features/klib/SPEC.md` — flat
- `specs/features/xemu/csr/SPEC.md` — nested
- `specs/features/klib/SPEC.md` — dup
";
        assert_eq!(
            extract(prd),
            vec![
                "specs/features/klib/SPEC.md".to_string(),
                "specs/features/xemu/csr/SPEC.md".to_string(),
            ],
        );
    }

    /// Path that ends with `/` without `SPEC.md` is rejected.
    #[test]
    fn rejects_path_without_spec_suffix() {
        let prd = "[**Related Specs**]\n- `specs/features/xemu/`\n";
        assert!(extract(prd).is_empty());
    }

    /// Path with an uppercase segment is rejected (kebab-case alphabet).
    #[test]
    fn rejects_uppercase_in_nested_path() {
        let prd = "[**Related Specs**]\n- `specs/features/Xemu/csr/SPEC.md`\n";
        assert!(extract(prd).is_empty());
    }
}
