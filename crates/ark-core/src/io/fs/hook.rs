//! Hook-file helpers — Ark hook entry surgery for `settings.json` and Codex
//! `hooks.json`.

use std::path::Path;

use crate::{
    error::{Error, Result},
    io::path_ext::PathExt,
};

/// Canonical command string identifying the Ark-owned hook entry.
///
/// Used as the identity value for upserts via [`update_hook_file`] and removals
/// via [`remove_hook_file`].
pub const ARK_CONTEXT_HOOK_COMMAND: &str = "ark context --scope session --format json";

/// Codex currently parses `suppressOutput` from hook stdout but does not hide
/// that stdout in the UI, so Ark's Codex hook must be silent.
pub const CODEX_CONTEXT_HOOK_COMMAND: &str = "ark context --scope session --format json >/dev/null";

/// Describes a JSON-array hook region in a config file.
///
/// Carried by `Platform::hook_file` so platform-iteration plumbing can drive
/// each platform's hook surface from one descriptor.
#[derive(Debug, Clone, Copy)]
pub struct HookFileSpec {
    /// Project-relative path to the JSON file (e.g. `.claude/settings.json`).
    pub path: &'static str,
    /// Stores the array key under root `hooks` carrying the Ark entry.
    pub hooks_array_key: &'static str,
    /// Stores the field name used to identify Ark's entry.
    pub identity_key: &'static str,
    /// Value of `identity_key` Ark uses to find its own entry.
    pub identity_value: &'static str,
    /// Builds the canonical Ark entry. Called by `init` / `load` / `upgrade`.
    pub entry_builder: fn() -> serde_json::Value,
}

/// Builds the canonical Ark Claude Code `SessionStart` hook entry.
///
/// Schema follows Claude Code's hooks contract: each `SessionStart` array
/// entry is a `{matcher, hooks: [...]}` wrapper. The empty matcher matches
/// every session-start event. The inner `hooks[0].command` is the identity
/// key Ark uses to detect (and replace) its own entry across runs.
///
/// Note: `timeout` is in **milliseconds** (Claude Code's hook schema). 5000
/// is the existing canonical value. Codex's hook schema uses seconds, not
/// milliseconds — see [`ark_codex_hook_entry`].
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

/// Builds the canonical Ark Codex `SessionStart` hook entry.
///
/// Schema follows Codex's hooks contract (parallel to Claude's). Note:
/// `timeout` is in **seconds**, not milliseconds — Codex's hook schema
/// (`developers.openai.com/codex/hooks`) defaults to 600 seconds when
/// omitted. 30 seconds gives `ark context` more than enough budget. The
/// command redirects stdout because Codex does not yet implement
/// `suppressOutput` for hook stdout.
pub fn ark_codex_hook_entry() -> serde_json::Value {
    serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": CODEX_CONTEXT_HOOK_COMMAND,
                "timeout": 30,
            }
        ],
    })
}

/// Inserts or replaces the Ark-owned hook entry in a platform hook file.
/// Idempotent: callable on every `init` / `load` / `upgrade` without
/// surprise. Preserves unrelated keys and sibling entries in the array.
///
/// `hooks_array_key` selects the array under root `hooks` (e.g.
/// `"SessionStart"`). `identity_key` selects the field within an entry
/// that identifies Ark's own (e.g. `"command"`). Identity is derived from
/// the inner `entry.hooks[*][identity_key]` (Claude/Codex hook wrapper
/// shape `{matcher, hooks: [...]}`).
///
/// `hooks_array_key` must match `[A-Za-z0-9_-]+`. Both shipping platforms
/// pass `"SessionStart"`.
///
/// The file is *not* hash-tracked. Re-applied unconditionally on every
/// init/load/upgrade.
///
/// Returns `Ok(true)` if a write happened, `Ok(false)` if the on-disk JSON
/// already encoded the canonical entry byte-identically (idempotence skip).
pub fn update_hook_file(
    path: impl AsRef<Path>,
    entry: serde_json::Value,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<bool> {
    let identity = identity_value_of(&entry, identity_key).ok_or_else(|| {
        Error::io(
            std::path::PathBuf::from("<hook-file>"),
            std::io::Error::other(format!(
                "hook entry missing inner `hooks[*].{identity_key}` (or top-level \
                 `{identity_key}`)"
            )),
        )
    })?;
    update_hook_file_with_identity(path, entry, &identity, hooks_array_key, identity_key)
}

/// Inserts or replaces an entry using an explicit stable identity value.
///
/// This is used when a platform's executable command changes while the
/// Ark-owned hook identity must still migrate older installed entries.
pub fn update_hook_file_with_identity(
    path: impl AsRef<Path>,
    entry: serde_json::Value,
    identity_value: &str,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<bool> {
    validate_hooks_array_key(hooks_array_key)?;
    let path = path.as_ref();
    let mut root = read_settings_or_empty(path)?;
    upsert_hook_entry(
        &mut root,
        entry,
        identity_value,
        hooks_array_key,
        identity_key,
    )?;
    let serialized = render_settings_json(&root);
    let on_disk = path.read_optional()?;
    if on_disk.as_deref() == Some(serialized.as_bytes()) {
        return Ok(false);
    }
    path.write_bytes(serialized.as_bytes())?;
    Ok(true)
}

/// Removes the Ark-owned hook entry by identity value.
///
/// Returns `Ok(true)` if an entry was removed, `Ok(false)` if absent. The hook
/// array is left in place even if it becomes empty.
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

/// Reads the Ark-owned hook entry as a snapshot-ready JSON value, if present.
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

// --- Deprecated thin wrappers. Removed at 0.3.0. ---

/// Deprecated alias updates the Claude Code `SessionStart` hook.
///
/// Uses [`update_hook_file`] with `hooks_array_key = "SessionStart"` and
/// `identity_key = "command"`. Removed at 0.3.0.
#[deprecated(since = "0.2.0", note = "use update_hook_file")]
pub fn update_settings_hook(path: impl AsRef<Path>, entry: serde_json::Value) -> Result<bool> {
    update_hook_file(path, entry, "SessionStart", "command")
}

/// Deprecated alias removes the Claude Code `SessionStart` hook.
///
/// Uses [`remove_hook_file`] with `hooks_array_key = "SessionStart"` and
/// `identity_key = "command"`. Removed at 0.3.0.
#[deprecated(since = "0.2.0", note = "use remove_hook_file")]
pub fn remove_settings_hook(path: impl AsRef<Path>, identity_value: &str) -> Result<bool> {
    remove_hook_file(path, identity_value, "SessionStart", "command")
}

/// Deprecated alias reads the Claude Code `SessionStart` hook.
///
/// Uses [`read_hook_file`] with `hooks_array_key = "SessionStart"` and
/// `identity_key = "command"`. Removed at 0.3.0.
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

/// Returns `true` if `entry` carries the given hook identity.
///
/// Tolerates the older flat shape for forward compatibility with snapshots
/// captured before the wrapper was introduced.
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
                .is_some_and(|actual| command_matches_identity(actual, identity_value))
        });
    }
    obj.get(identity_key)
        .and_then(|v| v.as_str())
        .is_some_and(|actual| command_matches_identity(actual, identity_value))
}

fn command_matches_identity(actual: &str, identity_value: &str) -> bool {
    actual == identity_value
        || actual
            .strip_prefix(identity_value)
            .is_some_and(|suffix| suffix.starts_with(char::is_whitespace))
}

fn read_settings_or_empty(path: &Path) -> Result<serde_json::Value> {
    Ok(read_settings_json(path)?.unwrap_or_else(|| serde_json::json!({})))
}

/// Parses a hook JSON file if it exists.
///
/// Returns `None` for a missing or empty file, `Some(value)` for a successful
/// parse, and `Err` for malformed JSON.
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
    identity: &str,
    hooks_array_key: &str,
    identity_key: &str,
) -> Result<()> {
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
        .find(|e| entry_carries_command(e, identity, identity_key))
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

/// Pretty-prints JSON with a trailing newline.
fn render_settings_json(root: &serde_json::Value) -> String {
    let mut s = serde_json::to_string_pretty(root).expect("settings json serializes");
    s.push('\n');
    s
}

#[cfg(test)]
#[allow(deprecated)] // exercises the deprecated SessionStart-aliased helpers
mod tests {
    use super::*;

    /// Returns a canonical Claude-Code-shaped Ark entry for testing.
    ///
    /// Mirrors what `commands::context::ark_session_start_hook_entry()`
    /// produces.
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

    /// Verifies that the identity matcher tolerates a flat-shape entry.
    ///
    /// Older snapshots whose `hook_bodies` captured the pre-wrapper form can
    /// still be detected and replaced.
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

    /// Verifies that `update_hook_file` round-trips with explicit keys.
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

    /// Verifies that `update_hook_file` rejects invalid array keys.
    #[test]
    fn update_hook_file_rejects_invalid_array_key() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hooks.json");
        let err = update_hook_file(&path, ark_entry(), "", "command").unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
        let err = update_hook_file(&path, ark_entry(), "Has Spaces", "command").unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    /// Verifies that the deprecated alias delegates to the new helper.
    ///
    /// The pre-existing `update_settings_hook_*` tests above already exercise
    /// the alias path; this test pins the alias-to-new equivalence explicitly.
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
