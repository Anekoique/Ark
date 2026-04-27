//! Ark-flavored file writes, walkers, and managed-block editing.
//!
//! Low-level filesystem primitives live on [`PathExt`]. This module adds:
//!
//! - [`write_file`] — content-aware writes that distinguish new / unchanged
//!   / overwritten / skipped outcomes.
//! - [`update_managed_block`] / [`remove_managed_block`] / [`read_managed_block`]
//!   — operations on `<!-- NAME:START -->...<!-- NAME:END -->` blocks
//!   embedded in text files like `CLAUDE.md`.
//! - [`walk_files`] — recursive enumeration of files under a directory.

use std::path::{Path, PathBuf};

use crate::{
    error::{Error, Result},
    io::path_ext::PathExt,
};

/// How to handle an existing file whose contents differ from what we'd write.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    /// Leave the existing file untouched.
    #[default]
    Skip,
    /// Overwrite.
    Force,
}

/// Outcome of a single write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Created,
    Unchanged,
    Overwritten,
    Skipped,
}

impl WriteOutcome {
    pub fn wrote(self) -> bool {
        matches!(self, Self::Created | Self::Overwritten)
    }
}

/// Write `contents` to `path`, obeying [`WriteMode`] on conflicts.
///
/// Skips silently when the file already contains byte-identical content.
pub fn write_file(
    path: impl AsRef<Path>,
    contents: &[u8],
    mode: WriteMode,
) -> Result<WriteOutcome> {
    let path = path.as_ref();
    let outcome = match (path.read_optional()?, mode) {
        (None, _) => WriteOutcome::Created,
        (Some(existing), _) if existing == contents => WriteOutcome::Unchanged,
        (Some(_), WriteMode::Skip) => WriteOutcome::Skipped,
        (Some(_), WriteMode::Force) => WriteOutcome::Overwritten,
    };
    if outcome.wrote() {
        path.write_bytes(contents)?;
    }
    Ok(outcome)
}

/// Read the body between `<!-- {marker}:START -->` and `<!-- {marker}:END -->`
/// in `path`, if both delimiters exist. Returns `Ok(None)` if the file or the
/// markers are missing.
pub fn read_managed_block(path: impl AsRef<Path>, marker: &str) -> Result<Option<String>> {
    Ok(path
        .as_ref()
        .read_text_optional()?
        .and_then(|text| Marker::new(marker).extract_body(&text)))
}

/// Replace the body between `<!-- {marker}:START -->` and `<!-- {marker}:END -->`
/// in `text` with `body`. Returns `None` if the marker pair is not present.
pub fn splice_managed_block(text: &str, marker: &str, body: &str) -> Option<String> {
    Marker::new(marker).replace_in(text, body)
}

/// Return every `ARK:*` marker name whose START+END pair appears in `text`, in
/// document order, de-duplicated. Used by upgrade to discover which regions of
/// a desired template should be reconciled against on-disk content.
pub fn scan_managed_markers(text: &str) -> Vec<String> {
    const PREFIX: &str = "<!-- ARK:";
    const END: &str = ":END -->";
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(PREFIX) {
        let tail = &rest[pos + PREFIX.len()..];
        let Some(colon) = tail.find(":START -->") else {
            break;
        };
        let name = &tail[..colon];
        let full = format!("ARK:{name}");
        // Only accept pairs: confirm a matching END appears later in the file.
        if text.contains(&format!("<!-- {full}{END}")) && !names.contains(&full) {
            names.push(full);
        }
        rest = &tail[colon..];
    }
    names
}

/// Insert or replace a delimited managed block in a text file. Creates the
/// file if it doesn't exist. Returns `true` once written.
///
/// Errors with [`Error::ManagedBlockCorrupt`] if the file contains a START
/// marker without a matching END — appending a fresh block in that case would
/// silently duplicate the marker and yield garbled state on subsequent reads.
pub fn update_managed_block(path: impl AsRef<Path>, marker: &str, body: &str) -> Result<bool> {
    let path = path.as_ref();
    let m = Marker::new(marker);
    let block = m.render(body);
    let new_contents = match path.read_text_optional()? {
        None => block,
        Some(text) => match m.replace_in(&text, body) {
            Some(replaced) => replaced,
            None if text.contains(&m.start()) => {
                return Err(Error::ManagedBlockCorrupt {
                    path: path.to_path_buf(),
                    marker: marker.to_string(),
                });
            }
            None => append_block(&text, &block),
        },
    };
    path.write_bytes(new_contents.as_bytes())?;
    Ok(true)
}

/// Splice every `ARK:*` managed-block body from the file at `path` into
/// `template`, returning the merged bytes. If `path` doesn't exist, returns
/// the template bytes unchanged. If `template` carries no managed blocks,
/// returns the template bytes unchanged.
///
/// This lets `init` and `upgrade` write embedded templates without clobbering
/// managed-block content that other commands (e.g. `spec register`) wrote
/// into the live file. The block bodies in the embedded template act as
/// fallbacks; the on-disk body always wins when present.
pub fn merge_managed_blocks(path: impl AsRef<Path>, template: &[u8]) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let Ok(text) = std::str::from_utf8(template) else {
        return Ok(template.to_vec());
    };
    let markers = scan_managed_markers(text);
    if markers.is_empty() {
        return Ok(template.to_vec());
    }
    let Some(on_disk) = path.read_text_optional()? else {
        return Ok(template.to_vec());
    };
    let mut spliced = text.to_string();
    for marker in &markers {
        if let Some(body) = extract_block_body_for_splice(&on_disk, marker)
            && let Some(new_text) = splice_managed_block(&spliced, marker, body)
        {
            spliced = new_text;
        }
    }
    Ok(spliced.into_bytes())
}

/// Extract the body between START/END tags, trimming exactly one leading and
/// one trailing `\n`. Round-trip-safe with `splice_managed_block` /
/// `Marker::render` (which write `\n{body}\n`); does NOT collapse interior
/// blank lines, so a body like "row\n\n" survives a splice round-trip
/// byte-identically. Distinct from [`Marker::extract_body`], which trims
/// *all* leading/trailing newlines and is appropriate when the caller only
/// wants a clean string for human consumption.
fn extract_block_body_for_splice<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let m = Marker::new(marker);
    let span = m.locate(text)?;
    let body = &text[span.body];
    let body = body.strip_prefix('\n').unwrap_or(body);
    let body = body.strip_suffix('\n').unwrap_or(body);
    Some(body)
}

/// Remove a managed block from a text file if present. If the resulting file
/// would be effectively empty, deletes it so no Ark-orphaned file lingers.
/// Returns `true` if the block was present and removed.
pub fn remove_managed_block(path: impl AsRef<Path>, marker: &str) -> Result<bool> {
    let path = path.as_ref();
    let Some(stripped) = path
        .read_text_optional()?
        .and_then(|text| Marker::new(marker).strip_from(&text))
    else {
        return Ok(false);
    };
    if stripped.trim().is_empty() {
        path.remove_if_exists()?;
    } else {
        path.write_bytes(stripped.as_bytes())?;
    }
    Ok(true)
}

// === Hook-file helpers — Ark hook entry surgery (ark-context C-17, codex-support C-19/C-22/C-25) ===

/// Canonical command string identifying the Ark-owned `SessionStart` hook
/// entry within a platform's hook file (`.claude/settings.json`,
/// `.codex/hooks.json`). Used as the identity value for upserts via
/// [`update_hook_file`] and removals via [`remove_hook_file`].
pub const ARK_CONTEXT_HOOK_COMMAND: &str = "ark context --scope session --format json";

/// Specification for a JSON-array hook region in a config file. Carried by
/// `Platform::hook_file` so the platform-iteration plumbing in
/// `init`/`upgrade`/`load`/`unload`/`remove` can drive each platform's
/// hook surface from one descriptor.
#[derive(Debug, Clone, Copy)]
pub struct HookFileSpec {
    /// Project-relative path to the JSON file (e.g. `.claude/settings.json`).
    pub path: &'static str,
    /// Array key under root `hooks` carrying the Ark entry. Both shipping
    /// platforms use `"SessionStart"`; future platforms with the same JSON
    /// shape pass a different key.
    pub hooks_array_key: &'static str,
    /// Field name used to identify Ark's entry within the array. Both
    /// shipping platforms use `"command"`.
    pub identity_key: &'static str,
    /// Value of `identity_key` Ark uses to find its own entry.
    pub identity_value: &'static str,
    /// Builds the canonical Ark entry. Called by `init` / `load` / `upgrade`.
    pub entry_builder: fn() -> serde_json::Value,
}

/// Build the canonical Ark Claude Code `SessionStart` hook entry.
///
/// Schema follows Claude Code's hooks contract: each `SessionStart` array
/// entry is a `{matcher, hooks: [...]}` wrapper. The empty matcher matches
/// every session-start event. The inner `hooks[0].command` is the identity
/// key Ark uses to detect (and replace) its own entry across runs.
///
/// Note: `timeout` is in **milliseconds** (Claude Code's hook schema). 5000
/// is the existing canonical value (per ark-context C-15). Codex's hook
/// schema uses seconds, not milliseconds — see [`ark_codex_hook_entry`].
pub fn ark_session_start_hook_entry() -> serde_json::Value {
    serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": ARK_CONTEXT_HOOK_COMMAND,
                "timeout": 5000,
            }
        ],
    })
}

/// Build the canonical Ark Codex `SessionStart` hook entry.
///
/// Schema follows Codex's hooks contract (parallel to Claude's). Note:
/// `timeout` is in **seconds**, not milliseconds — Codex's hook schema
/// (`developers.openai.com/codex/hooks`) defaults to 600 seconds when
/// omitted. 30 seconds gives `ark context` more than enough budget.
/// Per codex-support C-25.
pub fn ark_codex_hook_entry() -> serde_json::Value {
    serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": ARK_CONTEXT_HOOK_COMMAND,
                "timeout": 30,
            }
        ],
    })
}

/// Insert or replace the Ark-owned hook entry in a platform hook file.
/// Idempotent: callable on every `init` / `load` / `upgrade` without
/// surprise. Preserves unrelated keys and sibling entries in the array.
///
/// `hooks_array_key` selects the array under root `hooks` (e.g.
/// `"SessionStart"`). `identity_key` selects the field within an entry
/// that identifies Ark's own (e.g. `"command"`). Identity is derived from
/// the inner `entry.hooks[*][identity_key]` (Claude/Codex hook wrapper
/// shape `{matcher, hooks: [...]}`).
///
/// Per codex-support C-19: `hooks_array_key` must match `[A-Za-z0-9_-]+`.
/// Both shipping platforms pass `"SessionStart"`.
///
/// Per ark-context C-17 / codex-support C-11: the file is *not* hash-
/// tracked. Re-applied unconditionally on every init/load/upgrade.
///
/// Returns `Ok(true)` if a write happened, `Ok(false)` if the on-disk JSON
/// already encoded the canonical entry byte-identically (idempotence skip).
pub fn update_hook_file(
    path: impl AsRef<Path>,
    entry: serde_json::Value,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<bool> {
    validate_hooks_array_key(hooks_array_key)?;
    let path = path.as_ref();
    let mut root = read_settings_or_empty(path)?;
    upsert_hook_entry(&mut root, entry, hooks_array_key, identity_key)?;
    let serialized = render_settings_json(&root);
    let on_disk = path.read_optional()?;
    if on_disk.as_deref() == Some(serialized.as_bytes()) {
        return Ok(false);
    }
    path.write_bytes(serialized.as_bytes())?;
    Ok(true)
}

/// Remove the Ark-owned hook entry by identity value. Returns `Ok(true)`
/// if an entry was removed, `Ok(false)` if absent. The hook array is left
/// in place even if it becomes empty so users can re-add siblings without
/// re-init.
///
/// `identity_value` is matched against `entry.hooks[*][identity_key]`.
pub fn remove_hook_file(
    path: impl AsRef<Path>,
    identity_value: &str,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<bool> {
    validate_hooks_array_key(hooks_array_key)?;
    let path = path.as_ref();
    let Some(mut root) = read_settings_json(path)? else {
        return Ok(false);
    };
    let Some(array) = navigate_hook_array(&mut root, hooks_array_key) else {
        return Ok(false);
    };
    let before = array.len();
    array.retain(|e| !entry_carries_command(e, identity_value, identity_key));
    if array.len() == before {
        return Ok(false);
    }
    path.write_bytes(render_settings_json(&root).as_bytes())?;
    Ok(true)
}

/// Read the Ark-owned hook entry as a snapshot-ready JSON value, if present.
/// Returns `None` if the file is missing or contains no Ark entry.
///
/// `identity_value` is matched against `entry.hooks[*][identity_key]`.
pub fn read_hook_file(
    path: impl AsRef<Path>,
    identity_value: &str,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<Option<serde_json::Value>> {
    validate_hooks_array_key(hooks_array_key)?;
    let path = path.as_ref();
    let Some(mut root) = read_settings_json(path)? else {
        return Ok(None);
    };
    let Some(array) = navigate_hook_array(&mut root, hooks_array_key) else {
        return Ok(None);
    };
    Ok(array
        .iter()
        .find(|e| entry_carries_command(e, identity_value, identity_key))
        .cloned())
}

// --- Deprecated thin wrappers (codex-support C-23). Removed at 0.3.0. ---

/// Deprecated alias for [`update_hook_file`] with `hooks_array_key =
/// "SessionStart"` and `identity_key = "command"`. Removed at 0.3.0.
#[deprecated(since = "0.2.0", note = "use update_hook_file")]
pub fn update_settings_hook(path: impl AsRef<Path>, entry: serde_json::Value) -> Result<bool> {
    update_hook_file(path, entry, "SessionStart", "command")
}

/// Deprecated alias for [`remove_hook_file`] with `hooks_array_key =
/// "SessionStart"` and `identity_key = "command"`. Removed at 0.3.0.
#[deprecated(since = "0.2.0", note = "use remove_hook_file")]
pub fn remove_settings_hook(path: impl AsRef<Path>, identity_value: &str) -> Result<bool> {
    remove_hook_file(path, identity_value, "SessionStart", "command")
}

/// Deprecated alias for [`read_hook_file`] with `hooks_array_key =
/// "SessionStart"` and `identity_key = "command"`. Removed at 0.3.0.
#[deprecated(since = "0.2.0", note = "use read_hook_file")]
pub fn read_settings_hook(
    path: impl AsRef<Path>,
    identity_value: &str,
) -> Result<Option<serde_json::Value>> {
    read_hook_file(path, identity_value, "SessionStart", "command")
}

fn validate_hooks_array_key(key: &str) -> Result<()> {
    if !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Ok(())
    } else {
        Err(Error::io(
            std::path::PathBuf::from("<hook-file>"),
            std::io::Error::other("invalid hooks array key"),
        ))
    }
}

/// `true` if `entry` is a Claude/Codex hook wrapper whose inner `hooks[*]`
/// array contains a step with `[identity_key] == identity_value`. Tolerates
/// the older flat shape (`entry[identity_key]`) for forward-compat with
/// snapshots captured before the wrapper was introduced.
pub(crate) fn entry_carries_command(
    entry: &serde_json::Value,
    identity_value: &str,
    identity_key: &str,
) -> bool {
    let Some(obj) = entry.as_object() else {
        return false;
    };
    if let Some(inner) = obj.get("hooks").and_then(|v| v.as_array()) {
        return inner.iter().any(|step| {
            step.as_object()
                .and_then(|m| m.get(identity_key))
                .and_then(|v| v.as_str())
                == Some(identity_value)
        });
    }
    obj.get(identity_key).and_then(|v| v.as_str()) == Some(identity_value)
}

fn read_settings_or_empty(path: &Path) -> Result<serde_json::Value> {
    Ok(read_settings_json(path)?.unwrap_or_else(|| serde_json::json!({})))
}

/// Parse `.claude/settings.json` if it exists. Returns `None` for a missing
/// or empty file, `Some(value)` for a successful parse, and `Err` for malformed
/// JSON.
fn read_settings_json(path: &Path) -> Result<Option<serde_json::Value>> {
    let Some(text) = path.read_text_optional()? else {
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| Error::io(path, std::io::Error::other(e)))
}

fn upsert_hook_entry(
    root: &mut serde_json::Value,
    entry: serde_json::Value,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<()> {
    let identity = identity_value_of(&entry, identity_key).ok_or_else(|| {
        Error::io(
            std::path::PathBuf::from("<hook-file>"),
            std::io::Error::other(format!(
                "hook entry missing inner `hooks[*].{identity_key}` (or top-level \
                 `{identity_key}`)"
            )),
        )
    })?;

    // Ensure root is an object.
    if !root.is_object() {
        *root = serde_json::json!({});
    }
    let root_obj = root.as_object_mut().expect("root is object");

    // Ensure root.hooks is an object.
    let hooks = root_obj
        .entry("hooks".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !hooks.is_object() {
        *hooks = serde_json::json!({});
    }
    let hooks_obj = hooks.as_object_mut().expect("hooks is object");

    // Ensure hooks.<hooks_array_key> is an array.
    let session = hooks_obj
        .entry(hooks_array_key.to_string())
        .or_insert_with(|| serde_json::json!([]));
    if !session.is_array() {
        *session = serde_json::json!([]);
    }
    let array = session.as_array_mut().expect("hooks array");

    // Replace existing entry with the same identity, else append.
    if let Some(existing) = array
        .iter_mut()
        .find(|e| entry_carries_command(e, &identity, identity_key))
    {
        *existing = entry;
    } else {
        array.push(entry);
    }
    Ok(())
}

/// Extract the inner-step identity string from a hook-wrapper entry.
/// Falls back to the top-level `identity_key` field for the older flat shape.
fn identity_value_of(entry: &serde_json::Value, identity_key: &str) -> Option<String> {
    let obj = entry.as_object()?;
    if let Some(inner) = obj.get("hooks").and_then(|v| v.as_array())
        && let Some(cmd) = inner.iter().find_map(|step| {
            step.as_object()
                .and_then(|m| m.get(identity_key))
                .and_then(|v| v.as_str())
        })
    {
        return Some(cmd.to_string());
    }
    obj.get(identity_key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn navigate_hook_array<'a>(
    root: &'a mut serde_json::Value,
    hooks_array_key: &str,
) -> Option<&'a mut Vec<serde_json::Value>> {
    root.as_object_mut()?
        .get_mut("hooks")?
        .as_object_mut()?
        .get_mut(hooks_array_key)?
        .as_array_mut()
}

/// Pretty-print with a trailing newline. `serde_json` defaults to BTreeMap-
/// ordered objects, which gives stable byte-identical output across runs
/// (per ark-context C-29).
fn render_settings_json(root: &serde_json::Value) -> String {
    let mut s = serde_json::to_string_pretty(root).expect("settings json serializes");
    s.push('\n');
    s
}

/// Yield every file under `root` recursively, in an unspecified order.
///
/// Directories are skipped; only regular files are reported. Returns an empty
/// vector if `root` doesn't exist.
pub fn walk_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let root = root.as_ref();
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))? {
            let path = entry.map_err(|e| Error::io(&dir, e))?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn append_block(text: &str, block: &str) -> String {
    let sep = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("{text}{sep}\n{block}")
}

// --- Internal: managed-block delimiter helpers ---

/// `<!-- NAME:START -->` / `<!-- NAME:END -->` delimiter pair. Internal helper
/// for the managed-block functions above.
#[derive(Debug, Clone, Copy)]
struct Marker<'a> {
    name: &'a str,
}

impl<'a> Marker<'a> {
    const fn new(name: &'a str) -> Self {
        Self { name }
    }

    fn start(&self) -> String {
        format!("<!-- {}:START -->", self.name)
    }

    fn end(&self) -> String {
        format!("<!-- {}:END -->", self.name)
    }

    fn render(&self, body: &str) -> String {
        format!("{}\n{}\n{}\n", self.start(), body, self.end())
    }

    fn extract_body(&self, text: &str) -> Option<String> {
        let span = self.locate(text)?;
        Some(text[span.body].trim_matches('\n').to_string())
    }

    fn replace_in(&self, text: &str, body: &str) -> Option<String> {
        let span = self.locate(text)?;
        Some(format!(
            "{prefix}{block}\n{suffix}",
            prefix = &text[..span.start],
            block = self.render(body).trim_end_matches('\n'),
            suffix = &text[span.end..],
        ))
    }

    fn strip_from(&self, text: &str) -> Option<String> {
        let span = self.locate(text)?;
        let before = text[..span.start].trim_end_matches('\n');
        let after = text[span.end..].trim_start_matches('\n');
        Some(match (before.is_empty(), after.is_empty()) {
            (true, true) => String::new(),
            (true, false) => format!("{after}\n"),
            (false, true) => format!("{before}\n"),
            (false, false) => format!("{before}\n{after}"),
        })
    }

    fn locate(&self, text: &str) -> Option<MarkerSpan> {
        let start = text.find(&self.start())?;
        let rel_end = text[start..].find(&self.end())? + start;
        let body_start = start + self.start().len();
        let line_end = text[rel_end..]
            .find('\n')
            .map_or(text.len(), |i| rel_end + i + 1);
        Some(MarkerSpan {
            start,
            end: line_end,
            body: body_start..rel_end,
        })
    }
}

struct MarkerSpan {
    start: usize,
    end: usize,
    body: std::ops::Range<usize>,
}

#[cfg(test)]
#[allow(deprecated)] // exercises the deprecated SessionStart-aliased helpers (codex-support C-23)
mod tests {
    use super::*;

    #[test]
    fn write_file_creates_new() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("new.txt");
        assert_eq!(
            write_file(&target, b"hi", WriteMode::Skip).unwrap(),
            WriteOutcome::Created
        );
    }

    #[test]
    fn write_file_is_unchanged_on_identical() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"same").unwrap();
        assert_eq!(
            write_file(tmp.path(), b"same", WriteMode::Force).unwrap(),
            WriteOutcome::Unchanged
        );
    }

    #[test]
    fn write_file_skip_mode_preserves() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"old").unwrap();
        assert_eq!(
            write_file(tmp.path(), b"new", WriteMode::Skip).unwrap(),
            WriteOutcome::Skipped
        );
        assert_eq!(std::fs::read(tmp.path()).unwrap(), b"old");
    }

    #[test]
    fn write_file_force_overwrites() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"old").unwrap();
        assert_eq!(
            write_file(tmp.path(), b"new", WriteMode::Force).unwrap(),
            WriteOutcome::Overwritten
        );
    }

    #[test]
    fn managed_block_insert_and_replace() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "hello\n").unwrap();
        update_managed_block(tmp.path(), "ARK", "first").unwrap();
        let t1 = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(t1.contains("first"));

        update_managed_block(tmp.path(), "ARK", "second").unwrap();
        let t2 = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(t2.contains("second"));
        assert!(!t2.contains("first"));
    }

    #[test]
    fn managed_block_remove_deletes_file_when_only_block() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "<!-- ARK:START -->\nbody\n<!-- ARK:END -->\n").unwrap();
        assert!(remove_managed_block(tmp.path(), "ARK").unwrap());
        assert!(!tmp.path().exists());
    }

    #[test]
    fn update_managed_block_errors_on_orphan_start() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "<!-- ARK:START -->\nbody\nno-end-here\n").unwrap();
        let err = update_managed_block(tmp.path(), "ARK", "new body").unwrap_err();
        assert!(matches!(err, Error::ManagedBlockCorrupt { .. }));
    }

    #[test]
    fn read_managed_block_returns_body_or_none() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "before\n<!-- ARK:START -->\nfoo\nbar\n<!-- ARK:END -->\nafter\n",
        )
        .unwrap();
        assert_eq!(
            read_managed_block(tmp.path(), "ARK").unwrap().unwrap(),
            "foo\nbar"
        );

        std::fs::write(tmp.path(), "no markers here\n").unwrap();
        assert!(read_managed_block(tmp.path(), "ARK").unwrap().is_none());
    }

    #[test]
    fn walk_files_collects_recursively() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join("a.txt").write_bytes(b"").unwrap();
        tmp.path().join("sub/b.txt").write_bytes(b"").unwrap();
        let files = walk_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn walk_files_returns_empty_for_missing_root() {
        let tmp = tempfile::tempdir().unwrap();
        let files = walk_files(tmp.path().join("nope")).unwrap();
        assert!(files.is_empty());
    }

    /// Canonical Claude-Code-shaped Ark entry for testing. Mirrors what
    /// `commands::context::ark_session_start_hook_entry()` produces.
    fn ark_entry() -> serde_json::Value {
        serde_json::json!({
            "matcher": "",
            "hooks": [
                {
                    "type": "command",
                    "command": ARK_CONTEXT_HOOK_COMMAND,
                    "timeout": 5000,
                }
            ],
        })
    }

    #[test]
    fn update_settings_hook_creates_file_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        assert!(update_settings_hook(&path, ark_entry()).unwrap());
        let s = path.read_text().unwrap();
        assert!(s.contains(ARK_CONTEXT_HOOK_COMMAND));
        assert!(s.contains("SessionStart"));
    }

    #[test]
    fn update_settings_hook_is_idempotent_on_repeat() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        update_settings_hook(&path, ark_entry()).unwrap();
        let first = path.read_bytes().unwrap();
        let wrote_again = update_settings_hook(&path, ark_entry()).unwrap();
        assert!(!wrote_again, "second call should be a no-op");
        let second = path.read_bytes().unwrap();
        assert_eq!(first, second, "byte-identical after second update");
    }

    #[test]
    fn update_settings_hook_preserves_unrelated_pretooluse_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        let user_settings = serde_json::json!({
            "hooks": {
                "PreToolUse": [{"type": "command", "command": "user-hook"}],
            }
        });
        path.write_bytes(
            serde_json::to_string_pretty(&user_settings)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();

        update_settings_hook(&path, ark_entry()).unwrap();

        let after: serde_json::Value = serde_json::from_str(&path.read_text().unwrap()).unwrap();
        assert_eq!(
            after["hooks"]["PreToolUse"][0]["command"],
            serde_json::Value::String("user-hook".to_string()),
            "user PreToolUse must survive"
        );
        assert_eq!(
            after["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            serde_json::Value::String(ARK_CONTEXT_HOOK_COMMAND.to_string()),
        );
    }

    #[test]
    fn update_settings_hook_overwrites_user_modified_ark_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        // User stuck timeout: 99999 onto the Ark entry (wrapped shape).
        let tampered = serde_json::json!({
            "hooks": {
                "SessionStart": [{
                    "matcher": "",
                    "hooks": [{
                        "type": "command",
                        "command": ARK_CONTEXT_HOOK_COMMAND,
                        "timeout": 99999,
                    }]
                }]
            }
        });
        path.write_bytes(serde_json::to_string_pretty(&tampered).unwrap().as_bytes())
            .unwrap();

        update_settings_hook(&path, ark_entry()).unwrap();

        let after: serde_json::Value = serde_json::from_str(&path.read_text().unwrap()).unwrap();
        // Whole entry replaced — timeout should be 5000, not 99999.
        assert_eq!(
            after["hooks"]["SessionStart"][0]["hooks"][0]["timeout"],
            serde_json::Value::from(5000)
        );
    }

    #[test]
    fn remove_settings_hook_removes_only_ark_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        let mixed = serde_json::json!({
            "hooks": {
                "SessionStart": [
                    {
                        "matcher": "",
                        "hooks": [{"type": "command", "command": ARK_CONTEXT_HOOK_COMMAND}],
                    },
                    {
                        "matcher": "",
                        "hooks": [{"type": "command", "command": "user-extra"}],
                    },
                ]
            }
        });
        path.write_bytes(serde_json::to_string_pretty(&mixed).unwrap().as_bytes())
            .unwrap();

        let removed = remove_settings_hook(&path, ARK_CONTEXT_HOOK_COMMAND).unwrap();
        assert!(removed);

        let after: serde_json::Value = serde_json::from_str(&path.read_text().unwrap()).unwrap();
        let arr = after["hooks"]["SessionStart"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["hooks"][0]["command"], "user-extra");
    }

    #[test]
    fn remove_settings_hook_returns_false_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        assert!(!remove_settings_hook(&path, ARK_CONTEXT_HOOK_COMMAND).unwrap());
    }

    #[test]
    fn read_settings_hook_returns_entry_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        update_settings_hook(&path, ark_entry()).unwrap();
        let entry = read_settings_hook(&path, ARK_CONTEXT_HOOK_COMMAND)
            .unwrap()
            .unwrap();
        // Returned value is the matcher-wrapper; the command lives one level
        // deeper at entry.hooks[0].command (Claude Code's hook schema).
        assert_eq!(entry["hooks"][0]["command"], ARK_CONTEXT_HOOK_COMMAND);
    }

    /// Forward-compat: the identity matcher tolerates a flat-shape entry
    /// (no `matcher`/`hooks` wrapper) so older snapshots whose `hook_bodies`
    /// captured the pre-wrapper form can still be detected and replaced.
    #[test]
    fn entry_carries_command_tolerates_legacy_flat_shape() {
        let legacy = serde_json::json!({
            "type": "command",
            "command": ARK_CONTEXT_HOOK_COMMAND,
        });
        assert!(entry_carries_command(
            &legacy,
            ARK_CONTEXT_HOOK_COMMAND,
            "command"
        ));
        assert_eq!(
            identity_value_of(&legacy, "command").as_deref(),
            Some(ARK_CONTEXT_HOOK_COMMAND),
        );
    }

    #[test]
    fn read_settings_hook_returns_none_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        assert!(
            read_settings_hook(&path, ARK_CONTEXT_HOOK_COMMAND)
                .unwrap()
                .is_none()
        );
    }

    /// V-UT-4 (codex-support G-6, C-4): `update_hook_file` round-trips with
    /// explicit `(hooks_array_key, identity_key)` arguments.
    #[test]
    fn update_hook_file_round_trips_with_explicit_key() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hooks.json");
        assert!(update_hook_file(&path, ark_entry(), "SessionStart", "command").unwrap());
        let written = read_hook_file(&path, ARK_CONTEXT_HOOK_COMMAND, "SessionStart", "command")
            .unwrap()
            .expect("entry present");
        assert_eq!(written["hooks"][0]["command"], ARK_CONTEXT_HOOK_COMMAND);
    }

    /// codex-support C-19: `update_hook_file` rejects an empty / out-of-charset
    /// `hooks_array_key`.
    #[test]
    fn update_hook_file_rejects_invalid_array_key() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hooks.json");
        let err = update_hook_file(&path, ark_entry(), "", "command").unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
        let err = update_hook_file(&path, ark_entry(), "Has Spaces", "command").unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    /// codex-support C-23: deprecated alias delegates to the new helper. The
    /// pre-existing `update_settings_hook_*` tests above already exercise the
    /// alias path; this test pins the alias-to-new equivalence explicitly.
    #[test]
    fn deprecated_alias_delegates_to_update_hook_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path_alias = tmp.path().join("via_alias.json");
        let path_direct = tmp.path().join("via_direct.json");
        update_settings_hook(&path_alias, ark_entry()).unwrap();
        update_hook_file(&path_direct, ark_entry(), "SessionStart", "command").unwrap();
        assert_eq!(
            std::fs::read(&path_alias).unwrap(),
            std::fs::read(&path_direct).unwrap(),
        );
    }
}
