//! Identity helpers — name validation plus read/write of identity in
//! `.ark/.state.toml`.
//!
//! Name pattern: `^[A-Za-z][A-Za-z0-9_-]{0,39}$`. The on-disk format moved
//! from the legacy `.ark/.developer` (line-oriented `name=<x>`) to the
//! `[identity]` section of `.state.toml`. Migration is transparent: a pure
//! [`read_developer_name`] tolerates either layout, and [`write_developer_file`]
//! always writes the new format and cleans up the legacy file as a side effect
//! of `state_mutate`.

use chrono::{DateTime, Utc};

use crate::{
    error::{Error, Result},
    layout::Layout,
    session::ppid::RealPpid,
    state::{Identity, load_state, state_mutate},
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

/// Reads the developer name from `.ark/.state.toml`'s `[identity]` section.
///
/// Returns `None` if no identity has been recorded. The state-file load path
/// transparently synthesizes from the legacy `.ark/.developer` when the
/// state file is absent (see `state::checkout::migrate`).
pub fn read_developer_name(layout: &Layout) -> Result<Option<String>> {
    Ok(load_state(layout, &RealPpid::new())?
        .identity
        .map(|i| i.name))
}

/// Reads the developer name, or errors if no identity is set.
pub fn require_developer_name(layout: &Layout) -> Result<String> {
    match read_developer_name(layout)? {
        Some(name) => Ok(name),
        None => Err(Error::DeveloperNotInitialized {
            path: layout.state_file(),
        }),
    }
}

/// Writes developer identity into `.ark/.state.toml`'s `[identity]` section.
///
/// # Errors
///
/// Returns [`Error::DeveloperAlreadyInitialized`] when an identity is already
/// recorded under a different name. Re-writing the same name is idempotent:
/// the original `initialized_at` is preserved so "first initialized" semantics
/// survive repeat calls.
pub fn write_developer_file(layout: &Layout, name: &str, now: DateTime<Utc>) -> Result<()> {
    state_mutate(layout, &RealPpid::new(), |state| {
        match &state.identity {
            Some(existing) if existing.name != name => {
                return Err(Error::DeveloperAlreadyInitialized {
                    name: existing.name.clone(),
                });
            }
            Some(_) => return Ok(()), // idempotent re-write; preserve initialized_at.
            None => {}
        }
        state.identity = Some(Identity {
            name: name.to_string(),
            initialized_at: now,
        });
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

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
