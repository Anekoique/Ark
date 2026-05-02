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

/// Lists empty directories created during init.
pub const EMPTY_DIRS: &[&str] = &[".ark/tasks", ".ark/tasks/archive"];

/// Subdirectories and files under `.ark/` that the `agent` namespace manipulates.
pub const TASKS_DIR: &str = ".ark/tasks";
/// Directory for archived tasks under `.ark/`.
pub const TASKS_ARCHIVE_DIR: &str = ".ark/tasks/archive";
/// File that stores the current active task slug.
pub const TASKS_CURRENT_FILE: &str = ".ark/tasks/.current";
/// Directory for promoted feature specs.
pub const SPECS_FEATURES_DIR: &str = ".ark/specs/features";
/// Feature-spec index file path.
pub const SPECS_FEATURES_INDEX_FILE: &str = ".ark/specs/features/INDEX.md";
/// Directory for project-level specs.
pub const SPECS_PROJECT_DIR: &str = ".ark/specs/project";
/// Project-spec index file path.
pub const SPECS_PROJECT_INDEX_FILE: &str = ".ark/specs/project/INDEX.md";
/// Directory for task artifact templates.
pub const ARK_TEMPLATES_DIR: &str = ".ark/templates";

/// Root for git worktrees bound to Ark tasks (`<project>/.ark/worktrees/`).
///
/// Created lazily on first `task new --worktree`. Excluded from
/// `unload`'s snapshot capture.
pub const WORKTREES_DIR: &str = ".ark/worktrees";

/// User-editable consolidated config (`<project>/.ark/config.toml`).
///
/// Sectioned TOML: `[worktree]`, etc. Created by `init` from the embedded
/// template; `upgrade` does NOT overwrite.
pub const CONFIG_FILE: &str = ".ark/config.toml";

/// Fully Ark-owned gitignore at `<project>/.ark/.gitignore`.
///
/// Lists `worktrees/` so the parent checkout's index does not pick up
/// per-task worktree directories. Shipped as a regular template under
/// `.ark/`; no managed-block is needed since users do not co-author it.
pub const ARK_GITIGNORE_FILE: &str = ".ark/.gitignore";

/// Per-checkout state index (`<project>/.ark/.state.toml`).
///
/// Carries the active-task slug set and a per-session focus map. Treated
/// as an index over `task.toml` truth: reconciled on every read.
/// Gitignored. Skipped by `unload` in both walk sites.
pub const STATE_FILE: &str = ".ark/.state.toml";

/// Lock file co-located with the state file (`<project>/.ark/.state.toml.lock`).
///
/// A zero-byte sentinel that exclusive `File::try_lock` operates on. Created
/// on first `state_mutate`. Lock is OS-released on `File` drop, so a crashed
/// process never holds it past exit.
pub const STATE_LOCK_FILE: &str = ".ark/.state.toml.lock";

/// Filename prefix for in-flight state-file writes (`<project>/.ark/.state.toml.tmp.*`).
///
/// `state_mutate` writes a `.state.toml.tmp.<pid>` next to the canonical file
/// then atomically renames it into place. Crash-orphans are unlinked on the
/// next mutation under the lock.
pub const STATE_TMP_PREFIX: &str = ".state.toml.tmp.";

/// Host-side Claude Code settings file.
pub const CLAUDE_SETTINGS_FILE: &str = ".claude/settings.json";

/// Root directory for Codex integration (relative to project root).
pub const CODEX_DIR: &str = ".codex";

/// `<project>/.codex/skills/` — where Codex skill folders are extracted.
pub const CODEX_SKILLS_DIR: &str = ".codex/skills";

/// Codex hook configuration file.
pub const CODEX_HOOKS_FILE: &str = ".codex/hooks.json";

/// Project-scoped Codex defaults file.
pub const CODEX_CONFIG_FILE: &str = ".codex/config.toml";

/// Project-root file carrying the Codex-side managed block.
pub const AGENTS_MD: &str = "AGENTS.md";

/// Root directory for OpenCode integration (relative to project root).
pub const OPENCODE_DIR: &str = ".opencode";

/// OpenCode slash-command extraction directory.
pub const OPENCODE_COMMANDS_DIR: &str = ".opencode/commands";

/// Bun-loaded OpenCode context plugin path.
pub const OPENCODE_PLUGIN_FILE: &str = ".opencode/plugins/ark-context.ts";

/// Marker used for the feature-spec roster in `specs/features/INDEX.md`.
pub const FEATURES_MARKER: &str = "ARK:FEATURES";

/// Body written into the managed `CLAUDE.md` block.
pub const MANAGED_BLOCK_BODY: &str = "\
Ark is installed in this project. Use `/ark:quick` or `/ark:design` to start tasks.

See `.ark/workflow.md` for the full workflow.

@.ark/specs/INDEX.md";

/// Rooted view of an Ark-managed project.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Absolute or caller-provided project root.
    pub root: PathBuf,
}

impl Layout {
    /// Creates a layout rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the project root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the root directory for Ark state.
    pub fn ark_dir(&self) -> PathBuf {
        self.root.join(ARK_DIR)
    }

    /// Returns the root directory for Claude Code integration.
    pub fn claude_dir(&self) -> PathBuf {
        self.root.join(CLAUDE_DIR)
    }

    /// Returns the directory containing Ark's Claude Code slash commands.
    pub fn claude_commands_ark_dir(&self) -> PathBuf {
        self.root.join(CLAUDE_COMMANDS_ARK_DIR)
    }

    /// Returns the project-root `CLAUDE.md` path.
    pub fn claude_md(&self) -> PathBuf {
        self.root.join(CLAUDE_MD)
    }

    /// Managed-block marker name used in `CLAUDE.md` (e.g. `"ARK"`).
    pub fn managed_marker(&self) -> &'static str {
        MANAGED_MARKER
    }

    /// Resolves a project-relative path to an absolute path under `root`.
    pub fn resolve(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    /// Resolves a project-relative path after validating it is safe.
    ///
    /// Rejects absolute paths, root/prefix components, and any `..` traversal.
    ///
    /// Use for paths sourced from untrusted input (e.g. `.ark.db`
    /// snapshots).
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsafeSnapshotPath`] if `relative` is absolute,
    /// empty, contains a drive/UNC prefix, or contains a `..` component.
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

    /// Returns the root directory for active Ark tasks.
    pub fn tasks_dir(&self) -> PathBuf {
        self.root.join(TASKS_DIR)
    }

    /// Returns the directory for archived Ark tasks.
    pub fn tasks_archive_dir(&self) -> PathBuf {
        self.root.join(TASKS_ARCHIVE_DIR)
    }

    /// Returns the file that records the current Ark task slug.
    pub fn tasks_current(&self) -> PathBuf {
        self.root.join(TASKS_CURRENT_FILE)
    }

    /// Returns the active task directory for `slug`.
    pub fn task_dir(&self, slug: &str) -> PathBuf {
        self.tasks_dir().join(slug)
    }

    /// Returns the root directory for feature specs.
    pub fn specs_features_dir(&self) -> PathBuf {
        self.root.join(SPECS_FEATURES_DIR)
    }

    /// Returns the feature-spec directory for `feature`.
    pub fn specs_feature_dir(&self, feature: &str) -> PathBuf {
        self.specs_features_dir().join(feature)
    }

    /// Returns the feature-spec index path.
    pub fn specs_features_index(&self) -> PathBuf {
        self.root.join(SPECS_FEATURES_INDEX_FILE)
    }

    /// Returns the root directory for project specs.
    pub fn specs_project_dir(&self) -> PathBuf {
        self.root.join(SPECS_PROJECT_DIR)
    }

    /// Returns the project-spec index path.
    pub fn specs_project_index(&self) -> PathBuf {
        self.root.join(SPECS_PROJECT_INDEX_FILE)
    }

    /// Returns the Claude Code settings path.
    pub fn claude_settings(&self) -> PathBuf {
        self.root.join(CLAUDE_SETTINGS_FILE)
    }

    /// Returns the root directory for Ark task worktrees.
    pub fn worktrees_dir(&self) -> PathBuf {
        self.root.join(WORKTREES_DIR)
    }

    /// Returns the worktree directory for `branch`.
    ///
    /// Branch names may contain `/`, so this joins the branch onto
    /// [`Self::worktrees_dir`].
    pub fn worktree_dir(&self, branch: &str) -> PathBuf {
        self.worktrees_dir().join(branch)
    }

    /// Returns the project-level Ark config path.
    pub fn config_file(&self) -> PathBuf {
        self.root.join(CONFIG_FILE)
    }

    /// Returns the Ark-owned `.ark/.gitignore` path.
    pub fn ark_gitignore(&self) -> PathBuf {
        self.root.join(ARK_GITIGNORE_FILE)
    }

    /// Returns the per-checkout state file path.
    pub fn state_file(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    /// Returns the lock file path co-located with the state file.
    pub fn state_lock_file(&self) -> PathBuf {
        self.root.join(STATE_LOCK_FILE)
    }

    /// Returns the root directory for Codex integration.
    pub fn codex_dir(&self) -> PathBuf {
        self.root.join(CODEX_DIR)
    }

    /// Returns the directory containing extracted Codex skills.
    pub fn codex_skills_dir(&self) -> PathBuf {
        self.root.join(CODEX_SKILLS_DIR)
    }

    /// Returns the Codex hook configuration path.
    pub fn codex_hooks_file(&self) -> PathBuf {
        self.root.join(CODEX_HOOKS_FILE)
    }

    /// Returns the project-scoped Codex defaults path.
    pub fn codex_config_file(&self) -> PathBuf {
        self.root.join(CODEX_CONFIG_FILE)
    }

    /// Returns the project-root `AGENTS.md` path.
    pub fn agents_md(&self) -> PathBuf {
        self.root.join(AGENTS_MD)
    }

    /// Returns the root directory for OpenCode integration.
    pub fn opencode_dir(&self) -> PathBuf {
        self.root.join(OPENCODE_DIR)
    }

    /// Returns the OpenCode command extraction directory.
    pub fn opencode_commands_dir(&self) -> PathBuf {
        self.root.join(OPENCODE_COMMANDS_DIR)
    }

    /// Returns the OpenCode context plugin path.
    pub fn opencode_plugin_file(&self) -> PathBuf {
        self.root.join(OPENCODE_PLUGIN_FILE)
    }

    /// Returns the directory containing Ark task templates.
    pub fn ark_templates_dir(&self) -> PathBuf {
        self.root.join(ARK_TEMPLATES_DIR)
    }

    /// Discovers the nearest ancestor containing `.ark/`.
    ///
    /// Commands that operate on an *existing* Ark project (`context`,
    /// `unload`, `remove`, `upgrade`, `load` without `--force`) use this.
    /// Commands that scaffold a project (`init`, `load --force`) must use
    /// the explicit-target path instead.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotLoaded`] if no ancestor of `cwd` contains an
    /// `.ark/` directory.
    pub fn discover_from(cwd: impl AsRef<Path>) -> Result<Self> {
        let cwd = cwd.as_ref();
        for ancestor in cwd.ancestors() {
            if ancestor.join(ARK_DIR).is_dir() {
                return Ok(Self::new(ancestor.to_path_buf()));
            }
        }
        Err(Error::NotLoaded {
            path: cwd.to_path_buf(),
        })
    }

    /// Returns directories whose full contents round-trip through snapshots.
    ///
    /// `.codex/` and `.opencode/` join the set whether or not those platforms
    /// are installed in this project — `walk_files` on a missing directory
    /// yields an empty vec, so the un-installed cases are silently no-ops.
    pub fn owned_dirs(&self) -> [PathBuf; 4] {
        [
            self.ark_dir(),
            self.claude_commands_ark_dir(),
            self.codex_dir(),
            self.opencode_dir(),
        ]
    }

    /// Returns empty parent directories to prune after removing Ark content.
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
    use crate::io::PathExt;

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

    #[test]
    fn discover_from_finds_project_at_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join(".ark").join("tasks").ensure_dir().unwrap();
        let layout = Layout::discover_from(tmp.path()).unwrap();
        assert_eq!(layout.root(), tmp.path());
    }

    #[test]
    fn discover_from_walks_up_to_arked_ancestor() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join(".ark").join("tasks").ensure_dir().unwrap();
        let nested = tmp.path().join("a").join("b").join("c");
        nested.ensure_dir().unwrap();

        let layout = Layout::discover_from(&nested).unwrap();
        assert_eq!(layout.root(), tmp.path());
    }

    #[test]
    fn discover_from_errors_when_no_ancestor_is_arked() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("not").join("arked");
        nested.ensure_dir().unwrap();

        let err = Layout::discover_from(&nested).unwrap_err();
        assert!(matches!(err, Error::NotLoaded { .. }));
    }
}
