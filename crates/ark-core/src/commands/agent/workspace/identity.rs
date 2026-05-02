//! Developer identity stored in `.ark/.developer`.
//!
//! `Identity` wraps a non-empty developer name. `identity_resolve` reads
//! `.ark/.developer` and errors with [`Error::MissingIdentity`] when the
//! file is absent. The prompt path retries on blank input + missing env.

use std::{io::Write, path::Path};

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Validated developer name.
///
/// Construction enforces non-empty and trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    name: String,
}

impl Identity {
    /// Validates `name`, trimming whitespace; rejects empty results.
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let trimmed = name.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(Error::InvalidConfigField {
                field: "developer",
                reason: "must not be empty",
            });
        }
        Ok(Self { name: trimmed })
    }

    /// Returns the developer name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Options for [`identity_resolve`].
#[derive(Debug, Clone)]
pub struct ResolveOptions<'a> {
    /// Project root (used to locate `.ark/.developer`).
    pub project_root: &'a Path,
}

impl<'a> ResolveOptions<'a> {
    /// Creates options rooted at `project_root`.
    pub fn new(project_root: &'a Path) -> Self {
        Self { project_root }
    }
}

/// Resolves the active developer identity from `.ark/.developer`.
///
/// Errors with [`Error::MissingIdentity`] when the file is absent or empty.
/// Whitespace is trimmed; the first non-blank line wins.
pub fn identity_resolve(opts: ResolveOptions<'_>) -> Result<Identity> {
    let layout = Layout::new(opts.project_root);

    if let Some(text) = layout.developer_file().read_text_optional()?
        && let Some(name) = text.lines().find(|l| !l.trim().is_empty())
    {
        return Identity::new(name);
    }

    Err(Error::MissingIdentity)
}

/// Writes `identity.name()` to `.ark/.developer`, creating parent dirs.
///
/// Overwrites if the file exists; the caller controls when to call.
pub fn identity_write(project_root: &Path, identity: &Identity) -> Result<()> {
    let layout = Layout::new(project_root);
    let path = layout.developer_file();
    layout.ark_dir().ensure_dir()?;

    let mut content = identity.name().to_string();
    content.push('\n');
    std::fs::write(&path, content.as_bytes())
        .map_err(|source| Error::DeveloperWriteFailed { path, source })
}

/// Prompts the user (interactively) for a developer name.
///
/// Reads from `reader`, writes prompts to `writer`, retries up to
/// `max_attempts` on blank input. Trims whitespace. Suggests an env-derived
/// default (`USER`/`USERNAME`) on the first prompt only.
///
/// Designed for testability: the caller passes the streams and attempt cap.
pub fn identity_prompt<R: std::io::BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    max_attempts: u8,
) -> Result<Identity> {
    let env_default = std::env::var("USER")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("USERNAME")
                .ok()
                .filter(|s| !s.trim().is_empty())
        });

    for attempt in 0..max_attempts {
        if attempt == 0 {
            if let Some(env) = env_default.as_deref() {
                writeln!(writer, "Developer name [{env}]:").ok();
            } else {
                writeln!(writer, "Developer name:").ok();
            }
        } else {
            writeln!(writer, "Name cannot be blank. Please try again:").ok();
        }
        writer.flush().ok();

        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            continue;
        }

        let trimmed = line.trim();
        let candidate = if trimmed.is_empty() && attempt == 0 {
            env_default.as_deref().unwrap_or("")
        } else {
            trimmed
        };

        if !candidate.is_empty() {
            return Identity::new(candidate);
        }
    }

    Err(Error::MissingIdentity)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construction trims and rejects empty names.
    #[test]
    fn identity_new_rejects_blank() {
        assert!(Identity::new("alice").is_ok());
        assert!(Identity::new("  bob  ").is_ok());
        assert_eq!(Identity::new("  carol  ").unwrap().name(), "carol");
        assert!(matches!(
            Identity::new("").unwrap_err(),
            Error::InvalidConfigField { .. }
        ));
        assert!(matches!(
            Identity::new("   ").unwrap_err(),
            Error::InvalidConfigField { .. }
        ));
    }

    /// `.ark/.developer` is the single source of truth.
    #[test]
    fn identity_resolve_reads_developer_file() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();

        let err = identity_resolve(ResolveOptions::new(tmp.path())).expect_err("expected error");
        assert!(matches!(err, Error::MissingIdentity));

        identity_write(tmp.path(), &Identity::new("alice").unwrap()).unwrap();
        let id = identity_resolve(ResolveOptions::new(tmp.path())).unwrap();
        assert_eq!(id.name(), "alice");
    }

    /// Round-trip: write then resolve.
    #[test]
    fn identity_write_then_resolve_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        identity_write(tmp.path(), &Identity::new("alice").unwrap()).unwrap();
        let id = identity_resolve(ResolveOptions::new(tmp.path())).unwrap();
        assert_eq!(id.name(), "alice");
    }

    /// Prompt accepts non-blank on first try.
    #[test]
    fn identity_prompt_accepts_first_try() {
        let mut reader = std::io::Cursor::new(b"alice\n".to_vec());
        let mut writer = Vec::new();
        let id = identity_prompt(&mut reader, &mut writer, 3).unwrap();
        assert_eq!(id.name(), "alice");
    }

    /// Prompt retries on blank, accepts on second try.
    #[test]
    fn identity_prompt_retries_on_blank() {
        // First prompt: blank → reprompt (env_default is unset in test, so blank
        // does NOT fall back to env); second prompt: "bob".
        unsafe {
            std::env::remove_var("USER");
            std::env::remove_var("USERNAME");
        }
        let mut reader = std::io::Cursor::new(b"\nbob\n".to_vec());
        let mut writer = Vec::new();
        let id = identity_prompt(&mut reader, &mut writer, 3).unwrap();
        assert_eq!(id.name(), "bob");
        let prompt = String::from_utf8(writer).unwrap();
        assert!(prompt.contains("blank"));
    }

    /// Prompt errors after max_attempts blank inputs.
    #[test]
    fn identity_prompt_errors_after_max_attempts() {
        unsafe {
            std::env::remove_var("USER");
            std::env::remove_var("USERNAME");
        }
        let mut reader = std::io::Cursor::new(b"\n\n\n".to_vec());
        let mut writer = Vec::new();
        let err = identity_prompt(&mut reader, &mut writer, 3).unwrap_err();
        assert!(matches!(err, Error::MissingIdentity));
    }
}
