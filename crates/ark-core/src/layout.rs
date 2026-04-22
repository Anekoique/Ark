//! How an Ark-managed project is named and laid out on disk.
//!
//! [`Layout`] is a rooted view: it pairs a project root with the well-known
//! paths and names Ark reserves, so callers never join path fragments by hand.

use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// Root directory for Ark state (relative to project root).
pub const ARK_DIR: &str = ".ark";

/// Root directory for Claude Code integration (relative to project root).
pub const CLAUDE_DIR: &str = ".claude";

/// Subdirectory under `.claude/` where Ark's slash commands live.
pub const CLAUDE_COMMANDS_ARK_DIR: &str = ".claude/commands/ark";

/// Project-root file carrying the shared `CLAUDE.md` managed block.
pub const CLAUDE_MD: &str = "CLAUDE.md";

/// Marker name used for the managed block in `CLAUDE.md`.
pub const MANAGED_MARKER: &str = "ARK";

/// Directories under `.ark/` that must exist after init even though no
/// template files populate them. Users and the workflow fill these later.
pub const EMPTY_DIRS: &[&str] = &[".ark/tasks", ".ark/tasks/archive"];

/// Body written into the managed `CLAUDE.md` block.
pub const MANAGED_BLOCK_BODY: &str = "\
Ark is installed in this project. Use `/ark:quick` or `/ark:design` to start tasks.

See `.ark/workflow.md` for the full workflow.

@.ark/specs/INDEX.md";

/// Rooted view of an Ark-managed project.
#[derive(Debug, Clone)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/.ark/`
    pub fn ark_dir(&self) -> PathBuf {
        self.root.join(ARK_DIR)
    }

    /// `<root>/.claude/`
    pub fn claude_dir(&self) -> PathBuf {
        self.root.join(CLAUDE_DIR)
    }

    /// `<root>/.claude/commands/ark/`
    pub fn claude_commands_ark_dir(&self) -> PathBuf {
        self.root.join(CLAUDE_COMMANDS_ARK_DIR)
    }

    /// `<root>/CLAUDE.md`
    pub fn claude_md(&self) -> PathBuf {
        self.root.join(CLAUDE_MD)
    }

    /// Managed-block marker name used in `CLAUDE.md` (e.g. `"ARK"`).
    pub fn managed_marker(&self) -> &'static str {
        MANAGED_MARKER
    }

    /// Resolve a project-relative path to an absolute path under `root`.
    pub fn resolve(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    /// Resolve a project-relative path, rejecting absolute paths, root/prefix
    /// components, and any `..` traversal. Use for paths sourced from
    /// untrusted input (e.g. `.ark.db` snapshots).
    pub fn resolve_safe(&self, relative: impl AsRef<Path>) -> Result<PathBuf> {
        let relative = relative.as_ref();
        let reason = classify_unsafe(relative);
        if let Some(reason) = reason {
            return Err(Error::UnsafeSnapshotPath {
                path: relative.to_path_buf(),
                reason,
            });
        }
        Ok(self.root.join(relative))
    }

    /// Directories whose full contents are captured by `unload` and restored by
    /// `load`. User edits and additions under these survive a round-trip.
    pub fn owned_dirs(&self) -> [PathBuf; 2] {
        [self.ark_dir(), self.claude_commands_ark_dir()]
    }

    /// Parent directories we opportunistically prune after removing ark content,
    /// in deepest-first order.
    pub fn prunable_empty_parents(&self) -> [PathBuf; 2] {
        [
            self.root.join(".claude/commands"),
            self.root.join(".claude"),
        ]
    }
}

fn classify_unsafe(path: &Path) -> Option<&'static str> {
    if path.as_os_str().is_empty() {
        return Some("empty path");
    }
    if path.is_absolute() {
        return Some("absolute path");
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Some("contains `..` traversal"),
            Component::RootDir => return Some("contains root component"),
            Component::Prefix(_) => return Some("contains drive/UNC prefix"),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::new("/project")
    }

    #[test]
    fn resolve_safe_accepts_relative_paths() {
        let l = layout();
        assert_eq!(
            l.resolve_safe(".ark/workflow.md").unwrap(),
            PathBuf::from("/project/.ark/workflow.md"),
        );
        assert_eq!(
            l.resolve_safe("CLAUDE.md").unwrap(),
            PathBuf::from("/project/CLAUDE.md"),
        );
    }

    #[test]
    fn resolve_safe_rejects_absolute() {
        let err = layout().resolve_safe("/etc/passwd").unwrap_err();
        assert!(matches!(err, Error::UnsafeSnapshotPath { .. }));
    }

    #[test]
    fn resolve_safe_rejects_parent_traversal() {
        let err = layout().resolve_safe("../secrets").unwrap_err();
        assert!(matches!(err, Error::UnsafeSnapshotPath { .. }));

        let err = layout().resolve_safe(".ark/../../outside").unwrap_err();
        assert!(matches!(err, Error::UnsafeSnapshotPath { .. }));
    }

    #[test]
    fn resolve_safe_rejects_empty() {
        let err = layout().resolve_safe("").unwrap_err();
        assert!(matches!(err, Error::UnsafeSnapshotPath { .. }));
    }
}
