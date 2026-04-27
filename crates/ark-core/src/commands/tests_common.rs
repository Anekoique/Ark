//! Shared test helpers for the per-command source-scan invariants
//! (codex-support C-18). Each command-file test calls
//! [`assert_source_clean`] with `include_str!("<file>.rs")`.
//!
//! The scan asserts that command bodies route filesystem and path access
//! through `Layout` / `PathExt` / `io::fs` rather than hand-joining
//! literals or calling `std::fs::*` directly. The sanctioned exceptions
//! (`upgrade.rs`'s test-only `std::fs` calls and the `<.../>"` substrings
//! that appear in test fixtures) are skipped because the scan walks code
//! lines only — `#[cfg(test)]` bodies, comments, and string-content lines
//! that begin with `//` are excluded.

/// Line-by-line scan: assert the production half of `source` contains no
/// bare `std::fs::*` calls, and no path-composition that hand-joins one of
/// the canonical Ark/Claude/Codex prefixes (those go through `Layout`).
///
/// User-facing labels like `RemoveSummary`'s `".ark/"` display string are
/// allowed — only patterns that compose paths trigger the assert. The
/// composition patterns we look for are `.join("<prefix>"`,
/// `Path::new("<prefix>"`, and `PathBuf::from("<prefix>"`.
pub fn assert_source_clean(source: &str) {
    let path_prefixes = [".ark/", ".claude/", ".codex/"];
    let composition_patterns = [".join(\"", "Path::new(\"", "PathBuf::from(\""];
    let mut in_tests = false;
    for (idx, line) in source.lines().enumerate() {
        if line.contains("#[cfg(test)]") {
            in_tests = true;
        }
        if in_tests {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("//!") {
            continue;
        }
        let code = trimmed.split("//").next().unwrap_or(trimmed);
        assert!(
            !code.contains("std::fs::"),
            "line {} contains bare `std::fs::`: {line}",
            idx + 1
        );
        for pattern in &composition_patterns {
            for prefix in &path_prefixes {
                let needle = format!("{pattern}{prefix}");
                assert!(
                    !code.contains(&needle),
                    "line {} hand-composes a path with `{pattern}{prefix}…` — route through \
                     `Layout` instead: {line}",
                    idx + 1
                );
            }
        }
    }
}
