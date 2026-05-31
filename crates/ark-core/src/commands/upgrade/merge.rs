//! diff3 3-way merge for `[upgrade] merged` files.
//!
//! Wraps [`diffy::merge_bytes`] so the rest of upgrade speaks in `Vec<u8>`
//! and a single [`MergeOutcome`] rather than `diffy`'s `Result`-as-status.
//! Operates on raw bytes so non-UTF-8 template content round-trips losslessly.
//! A conflict is rendered with `diffy`'s default `ConflictStyle::Merge`,
//! producing Git-style `<<<<<<<` / `=======` / `>>>>>>>` markers.

/// Result of a 3-way merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Sides merged without conflict; carries the merged bytes.
    Clean(Vec<u8>),
    /// Sides conflicted; carries the marker-laden bytes to write.
    Conflict(Vec<u8>),
}

/// Performs a 3-way merge of `ours` and `theirs` against their common `base`.
///
/// `base` is the bytes Ark last wrote (the diff3 ancestor), `ours` the user's
/// current file, `theirs` the new embedded template. A clean merge returns
/// [`MergeOutcome::Clean`]; overlapping edits return [`MergeOutcome::Conflict`]
/// with Git-style markers.
pub fn three_way_merge(base: &[u8], ours: &[u8], theirs: &[u8]) -> MergeOutcome {
    match diffy::merge_bytes(base, ours, theirs) {
        Ok(merged) => MergeOutcome::Clean(merged),
        Err(conflicted) => MergeOutcome::Conflict(conflicted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Disjoint edits merge without conflict.
    #[test]
    fn disjoint_edits_merge_clean() {
        let base = b"line1\nline2\nline3\n";
        let ours = b"line1 EDITED\nline2\nline3\n";
        let theirs = b"line1\nline2\nline3 EDITED\n";
        let MergeOutcome::Clean(merged) = three_way_merge(base, ours, theirs) else {
            panic!("expected clean merge");
        };
        let text = String::from_utf8(merged).unwrap();
        assert!(text.contains("line1 EDITED"));
        assert!(text.contains("line3 EDITED"));
        assert!(!text.contains("<<<<<<<"));
    }

    /// Overlapping edits produce Git-style markers.
    #[test]
    fn overlapping_edits_conflict_with_git_markers() {
        let base = b"shared line\n";
        let ours = b"our version\n";
        let theirs = b"their version\n";
        let MergeOutcome::Conflict(bytes) = three_way_merge(base, ours, theirs) else {
            panic!("expected conflict");
        };
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("<<<<<<<"), "missing start marker: {text}");
        assert!(text.contains("======="), "missing separator: {text}");
        assert!(text.contains(">>>>>>>"), "missing end marker: {text}");
        assert!(text.contains("our version"));
        assert!(text.contains("their version"));
    }

    /// Non-UTF-8 bytes round-trip through a clean merge.
    #[test]
    fn non_utf8_round_trips() {
        // A disjoint edit that keeps the non-UTF-8 byte sequence intact.
        let base = b"a\n\xff\xfe\nc\n";
        let ours = b"A\n\xff\xfe\nc\n";
        let theirs = b"a\n\xff\xfe\nC\n";
        let MergeOutcome::Clean(merged) = three_way_merge(base, ours, theirs) else {
            panic!("expected clean merge");
        };
        // The non-UTF-8 line survives verbatim.
        assert!(
            merged.windows(2).any(|w| w == [0xff, 0xfe]),
            "non-utf8 bytes dropped"
        );
    }
}
