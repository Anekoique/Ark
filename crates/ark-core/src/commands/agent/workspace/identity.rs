//! Identity helpers — read/write `.ark/.developer`; name validation.
//!
//! Name pattern: `^[A-Za-z][A-Za-z0-9_-]{0,39}$`.
//! File shape: `name=<x>\ninitialized_at=<RFC3339>\n`. The reader is
//! line-oriented and tolerates blank lines plus unknown keys for forward
//! compatibility.

use chrono::{DateTime, Utc};

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Maximum length of a developer name.
const MAX_LEN: usize = 40;

/// Validates the developer name.
///
/// Rejects empty strings, names with a leading digit or dash, names
/// containing `/`, `\`, or `.`, names longer than 40 characters,
/// and non-ASCII input.
pub fn validate_developer_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidDeveloperName {
            name: name.to_string(),
            reason: "must not be empty",
        });
    }
    if name.len() > MAX_LEN {
        return Err(Error::InvalidDeveloperName {
            name: name.to_string(),
            reason: "must be at most 40 characters",
        });
    }
    let mut chars = name.chars();
    let first = chars.next().expect("non-empty checked above");
    if !first.is_ascii_alphabetic() {
        return Err(Error::InvalidDeveloperName {
            name: name.to_string(),
            reason: "must start with an ASCII letter",
        });
    }
    for c in std::iter::once(first).chain(chars) {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err(Error::InvalidDeveloperName {
                name: name.to_string(),
                reason: "must contain only ASCII letters, digits, `_`, or `-`",
            });
        }
    }
    Ok(())
}

/// Reads the developer name from `.ark/.developer`.
///
/// Returns `None` if the file is missing or has no `name=` line.
pub fn read_developer_name(layout: &Layout) -> Result<Option<String>> {
    let path = layout.developer_file();
    let Some(text) = path.read_text_optional()? else {
        return Ok(None);
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("name=") {
            let name = rest.trim();
            if !name.is_empty() {
                return Ok(Some(name.to_string()));
            }
        }
    }
    Ok(None)
}

/// Reads the developer name, or errors if the file is missing.
pub fn require_developer_name(layout: &Layout) -> Result<String> {
    match read_developer_name(layout)? {
        Some(name) => Ok(name),
        None => Err(Error::DeveloperNotInitialized {
            path: layout.developer_file(),
        }),
    }
}

/// Writes the canonical two-line `.developer` content.
pub fn write_developer_file(layout: &Layout, name: &str, now: DateTime<Utc>) -> Result<()> {
    let path = layout.developer_file();
    let body = format!("name={name}\ninitialized_at={}\n", now.to_rfc3339());
    path.write_bytes(body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that valid developer names are accepted.
    #[test]
    fn validate_accepts_basic_names() {
        for ok in ["A", "kleinhe", "dev_1", "a-b", "Z", "ab_cd-EF_12"] {
            validate_developer_name(ok).unwrap_or_else(|e| panic!("{ok} rejected: {e}"));
        }
    }

    #[test]
    fn validate_accepts_40_char_name() {
        let n = "a".repeat(40);
        validate_developer_name(&n).unwrap();
    }

    #[test]
    fn validate_rejects_empty() {
        assert!(matches!(
            validate_developer_name("").unwrap_err(),
            Error::InvalidDeveloperName { .. }
        ));
    }

    #[test]
    fn validate_rejects_too_long() {
        let n = "a".repeat(41);
        assert!(matches!(
            validate_developer_name(&n).unwrap_err(),
            Error::InvalidDeveloperName { .. }
        ));
    }

    #[test]
    fn validate_rejects_leading_digit() {
        assert!(matches!(
            validate_developer_name("1leading").unwrap_err(),
            Error::InvalidDeveloperName { .. }
        ));
        assert!(matches!(
            validate_developer_name("0").unwrap_err(),
            Error::InvalidDeveloperName { .. }
        ));
    }

    #[test]
    fn validate_rejects_leading_dash() {
        assert!(matches!(
            validate_developer_name("-leading").unwrap_err(),
            Error::InvalidDeveloperName { .. }
        ));
    }

    #[test]
    fn validate_rejects_path_separators_and_dots() {
        for bad in ["dev/path", "/abs", "dev\\back", "..", "a.b"] {
            assert!(
                matches!(
                    validate_developer_name(bad).unwrap_err(),
                    Error::InvalidDeveloperName { .. }
                ),
                "expected reject on {bad}"
            );
        }
    }

    #[test]
    fn validate_rejects_non_ascii() {
        for bad in ["é", "中", "café"] {
            assert!(
                matches!(
                    validate_developer_name(bad).unwrap_err(),
                    Error::InvalidDeveloperName { .. }
                ),
                "expected reject on {bad}"
            );
        }
    }

    /// Verifies that reading a missing file returns `None`.
    #[test]
    fn read_returns_none_for_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert!(read_developer_name(&layout).unwrap().is_none());
    }

    /// Verifies that a written developer name round-trips through a read.
    #[test]
    fn read_developer_name_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        write_developer_file(&layout, "kleinhe", Utc::now()).unwrap();
        assert_eq!(
            read_developer_name(&layout).unwrap().as_deref(),
            Some("kleinhe")
        );
    }

    /// Verifies that the reader tolerates blank lines and unknown keys.
    #[test]
    fn read_tolerates_blank_and_unknown_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .developer_file()
            .write_bytes(b"\nunknown=x\nname=alice\nfuture=42\n")
            .unwrap();
        assert_eq!(
            read_developer_name(&layout).unwrap().as_deref(),
            Some("alice")
        );
    }

    #[test]
    fn read_returns_none_for_malformed_file() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout
            .developer_file()
            .write_bytes(b"no name key here\n")
            .unwrap();
        assert!(read_developer_name(&layout).unwrap().is_none());
    }

    #[test]
    fn require_errors_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let err = require_developer_name(&layout).unwrap_err();
        assert!(matches!(err, Error::DeveloperNotInitialized { .. }));
    }
}
