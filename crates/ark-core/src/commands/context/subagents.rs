//! Enumerates installed agent stems per platform.
//!
//! Walks each [`Platform`]'s `agents_dest_dir` under [`Layout::root`].
//! Returns one [`SubagentSet`] per platform whose directory exists. The
//! `platform` field is the platform's [`Platform::cli_flag`] verbatim
//! (per C-44). Stems are derived per [`platform_extension`] and sorted
//! ascending. Symlinks and non-matching extensions are skipped silently.

use std::path::Path;

use crate::{
    commands::context::model::SubagentSet,
    layout::Layout,
    platforms::{PLATFORMS, Platform},
};

/// Returns every installed agent stem grouped by platform.
///
/// One [`SubagentSet`] per platform whose `agents_dest_dir` exists under
/// [`Layout::root`]. Empty `Vec` when no platform directory is present.
/// Stems are sorted ascending; user-installed agents appear alongside
/// Ark canonicals.
pub fn enumerate_subagents(layout: &Layout) -> Vec<SubagentSet> {
    let root = layout.root();
    let mut out = Vec::new();
    for platform in PLATFORMS.iter().copied() {
        let Some(dir_rel) = platform.agents_dest_dir else {
            continue;
        };
        let dir = root.join(dir_rel);
        if !dir.exists() {
            continue;
        }
        let ext = platform_extension(platform);
        let stems = read_stems(&dir, ext);
        out.push(SubagentSet {
            platform: platform.cli_flag.to_string(),
            stems,
        });
    }
    out
}

/// Returns the per-platform file extension that distinguishes an agent
/// file (per C-41).
fn platform_extension(platform: &Platform) -> &'static str {
    match platform.id {
        "codex" => "toml",
        _ => "md",
    }
}

/// Reads agent stems from `dir`, filtering to files whose extension
/// matches `ext`. Symlinks and entries failing `file_type()` are skipped.
fn read_stems(dir: &Path, ext: &str) -> Vec<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };
    let mut stems: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if file_type.is_symlink() || !file_type.is_file() {
                return None;
            }
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some(ext) {
                return None;
            }
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
        })
        .collect();
    stems.sort();
    stems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

    /// Claude stems include user agents alongside Ark canonicals,
    /// sorted, emitted with the `claude` `cli_flag` tag.
    #[test]
    fn enumerates_claude_stems_with_user_agents() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".claude/agents");
        dir.ensure_dir().unwrap();
        std::fs::write(dir.join("ark-reviewer.md"), b"x").unwrap();
        std::fs::write(dir.join("code-reviewer.md"), b"x").unwrap();
        let layout = Layout::new(tmp.path());
        let out = enumerate_subagents(&layout);
        assert_eq!(out.len(), 1, "expected one platform row, got {out:?}");
        assert_eq!(out[0].platform, "claude");
        assert_eq!(out[0].stems, vec!["ark-reviewer", "code-reviewer"]);
    }

    /// Platform dir absent → no row emitted (not an empty-stems
    /// row).
    #[test]
    fn skips_platform_dir_that_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let out = enumerate_subagents(&layout);
        assert!(out.is_empty(), "expected no rows, got {out:?}");
    }

    /// Codex stems derive from `.toml` filenames; non-matching
    /// extensions are skipped.
    #[test]
    fn enumerates_codex_stems_from_toml_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".codex/agents");
        dir.ensure_dir().unwrap();
        std::fs::write(dir.join("ark-reviewer.toml"), b"x").unwrap();
        std::fs::write(dir.join("custom-helper.toml"), b"x").unwrap();
        // A stray `.md` file is silently skipped.
        std::fs::write(dir.join("readme.md"), b"not an agent").unwrap();
        let layout = Layout::new(tmp.path());
        let out = enumerate_subagents(&layout);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].platform, "codex");
        assert_eq!(out[0].stems, vec!["ark-reviewer", "custom-helper"]);
    }

    /// Symlinked entries are skipped.
    #[cfg(unix)]
    #[test]
    fn skips_symlinked_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".claude/agents");
        dir.ensure_dir().unwrap();
        std::fs::write(dir.join("real.md"), b"x").unwrap();
        std::os::unix::fs::symlink(dir.join("real.md"), dir.join("link.md")).unwrap();
        let layout = Layout::new(tmp.path());
        let out = enumerate_subagents(&layout);
        assert_eq!(out.len(), 1);
        // Only `real` survives — the symlink is filtered.
        assert_eq!(out[0].stems, vec!["real"]);
    }

    /// Zero installed agents anywhere → empty `Vec`.
    #[test]
    fn returns_empty_when_no_platform_dirs_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert!(enumerate_subagents(&layout).is_empty());
    }

    /// Multiple platforms each emit their own row with their `cli_flag`.
    #[test]
    fn emits_one_row_per_present_platform() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join(".claude/agents").ensure_dir().unwrap();
        std::fs::write(tmp.path().join(".claude/agents/a.md"), b"x").unwrap();
        tmp.path().join(".opencode/agents").ensure_dir().unwrap();
        std::fs::write(tmp.path().join(".opencode/agents/b.md"), b"x").unwrap();
        let layout = Layout::new(tmp.path());
        let out = enumerate_subagents(&layout);
        let tags: Vec<&str> = out.iter().map(|s| s.platform.as_str()).collect();
        // PLATFORMS order is claude, codex, opencode; codex dir absent.
        assert_eq!(tags, vec!["claude", "opencode"]);
    }
}
