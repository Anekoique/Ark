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

/// Subdirectory under `.claude/` where Ark's subagent definitions live.
///
/// Sits outside [`CLAUDE_COMMANDS_ARK_DIR`] (Claude's commands/ subtree) so it
/// must be tracked separately by [`Layout::owned_dirs`] via the platform's
/// `extra_dirs` field; otherwise `unload`/`load`/`remove` would not see it.
pub const CLAUDE_AGENTS_DIR: &str = ".claude/agents";

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

/// Sidecar holding the last-Ark-written bytes for `[upgrade] merged` paths
/// (`<project>/.ark/.upgrade-base/`).
///
/// Each merged path is mirrored under this root so a future `ark upgrade`
/// has the diff3 base (the manifest stores only a hash, not the bytes).
/// Gitignored and skipped by `unload` — local-only state, like `.state.toml`.
pub const UPGRADE_BASE_DIR: &str = ".ark/.upgrade-base";

/// Pre-write backup captured by `ark upgrade` (`<project>/.ark/.upgrade-backup/`).
///
/// Holds a copy of every file (and the manifest) an upgrade is about to
/// mutate or delete, so a failed upgrade can roll back and a regretted one
/// can `--restore`. Gitignored and skipped by `unload`; one backup is
/// retained per non-dry-run upgrade, replacing any prior one.
pub const UPGRADE_BACKUP_DIR: &str = ".ark/.upgrade-backup";

/// Root directory for per-developer workspace journals (`<project>/.ark/workspace/`).
///
/// Holds the top-level Active Developers index and one subdirectory per
/// developer (`<dev>/index.md`, `<dev>/journal-N.md`). Created lazily on
/// first `workspace_record` write.
pub const WORKSPACE_DIR: &str = ".ark/workspace";

/// Top-level Active Developers index file (`<project>/.ark/workspace/index.md`).
///
/// Auto-maintained by `developer_register` / `developer_touch` between
/// `<!-- ARK:DEVELOPERS:START -->` / `<!-- ARK:DEVELOPERS:END -->` markers.
pub const WORKSPACE_INDEX_FILE: &str = ".ark/workspace/index.md";

/// Identity file (`<project>/.ark/.developer`).
///
/// Single-line plain text containing the active developer's name. Gitignored.
/// Read by `identity_resolve` at every `workspace_record` invocation; written
/// by `ark init --developer <name>` (or the interactive prompt).
pub const DEVELOPER_FILE: &str = ".ark/.developer";

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

/// `<project>/.codex/agents/` — where Codex subagent TOML files are extracted.
///
/// Lives under [`CODEX_DIR`] so it inherits the platform's `removal_root`;
/// no separate `extra_dirs` entry is needed.
pub const CODEX_AGENTS_DIR: &str = ".codex/agents";

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

/// `<project>/.opencode/agents/` — where OpenCode subagent markdown files are extracted.
///
/// Lives under [`OPENCODE_DIR`] so it inherits the platform's `removal_root`;
/// no separate `extra_dirs` entry is needed.
pub const OPENCODE_AGENTS_DIR: &str = ".opencode/agents";

/// Bun-loaded OpenCode context plugin path.
pub const OPENCODE_PLUGIN_FILE: &str = ".opencode/plugins/ark-context.ts";

/// Marker used for the feature-spec roster in `specs/features/INDEX.md`.
pub const FEATURES_MARKER: &str = "ARK:FEATURES";

/// Marker used for the Active Developers table in `.ark/workspace/index.md`.
pub const DEVELOPERS_MARKER: &str = "ARK:DEVELOPERS";

/// Marker used for the Session History table in `.ark/workspace/<dev>/index.md`.
pub const SESSIONS_MARKER: &str = "ARK:SESSIONS";

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

    /// Returns the directory containing Ark's Claude Code subagent files.
    pub fn claude_agents_dir(&self) -> PathBuf {
        self.root.join(CLAUDE_AGENTS_DIR)
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

    /// Returns the tier-grouped archive index file (`tasks/archive/INDEX.md`).
    pub fn tasks_archive_index(&self) -> PathBuf {
        self.tasks_archive_dir().join("INDEX.md")
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

    /// Returns the feature-spec directory for a `/`-joined segments path.
    ///
    /// Revalidates every segment against the kebab-case alphabet
    /// (`^[a-z0-9][a-z0-9_-]*$`); rejects `.`, `..`, empty segments, and
    /// case-insensitive reserved names `index` / `spec`. `parse_spec_path`
    /// and this method are the two `Result`-returning constructors of a
    /// validated feature path (cf. `Layout::resolve_safe` precedent).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidFeaturePath`] if any segment fails
    /// validation. `source_path` is set to the caller-provided origin so
    /// the error carries provenance.
    pub fn specs_feature_dir(
        &self,
        segments: &[&str],
        source_path: impl Into<PathBuf>,
    ) -> Result<PathBuf> {
        let source_path = source_path.into();
        if segments.is_empty() {
            return Err(Error::InvalidFeaturePath {
                source_path,
                value: String::new(),
                reason: "empty SPEC path",
            });
        }
        let mut out = self.specs_features_dir();
        for seg in segments {
            validate_feature_path_segment(seg, &source_path)?;
            out.push(seg);
        }
        Ok(out)
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

    /// Returns the trailing path component when the root is a worktree.
    ///
    /// Matches `<parent>/.ark/worktrees/<branch-type>/<slug>` lexically; no
    /// filesystem access, no canonicalization. Symlinked roots and case-
    /// insensitive volumes that produce a non-canonical [`Self::root`] are
    /// out of scope: if the trailing components do not literally match the
    /// expected pattern, returns `None`.
    ///
    /// The `<branch-type>` slot is not validated against any allowlist —
    /// the type may have been added since the worktree was created. Callers
    /// gate further by checking [`Self::task_dir`] existence and active-set
    /// membership.
    ///
    /// Returns `None` when the root has fewer than four trailing components,
    /// when any fixed-name component does not match, or when the slug
    /// component fails UTF-8 conversion.
    pub fn slug_from_worktree_root(&self) -> Option<String> {
        use std::path::Component;
        let mut tail: Vec<&std::ffi::OsStr> = self
            .root
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s),
                _ => None,
            })
            .collect();
        let slug = tail.pop()?;
        let _branch_type = tail.pop()?;
        let worktrees = tail.pop()?;
        let ark = tail.pop()?;
        if ark != ARK_DIR || worktrees != "worktrees" {
            return None;
        }
        slug.to_str().map(str::to_owned)
    }

    /// Returns the project-level Ark config path.
    pub fn config_file(&self) -> PathBuf {
        self.root.join(CONFIG_FILE)
    }

    /// Returns the upgrade merge-base sidecar root (`.ark/.upgrade-base`).
    pub fn upgrade_base_dir(&self) -> PathBuf {
        self.root.join(UPGRADE_BASE_DIR)
    }

    /// Returns the upgrade backup root (`.ark/.upgrade-backup`).
    pub fn upgrade_backup_dir(&self) -> PathBuf {
        self.root.join(UPGRADE_BACKUP_DIR)
    }

    /// Returns the root directory for per-developer workspace journals.
    pub fn workspace_dir(&self) -> PathBuf {
        self.root.join(WORKSPACE_DIR)
    }

    /// Returns the top-level Active Developers index path.
    pub fn workspace_index(&self) -> PathBuf {
        self.root.join(WORKSPACE_INDEX_FILE)
    }

    /// Returns the per-developer workspace directory for `name`.
    pub fn workspace_developer_dir(&self, name: &str) -> PathBuf {
        self.workspace_dir().join(name)
    }

    /// Returns the per-developer personal index path.
    pub fn workspace_developer_index(&self, name: &str) -> PathBuf {
        self.workspace_developer_dir(name).join("index.md")
    }

    /// Returns the developer identity file path (`.ark/.developer`).
    pub fn developer_file(&self) -> PathBuf {
        self.root.join(DEVELOPER_FILE)
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

    /// Returns the directory containing Ark's Codex subagent files.
    pub fn codex_agents_dir(&self) -> PathBuf {
        self.root.join(CODEX_AGENTS_DIR)
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

    /// Returns the directory containing Ark's OpenCode subagent files.
    pub fn opencode_agents_dir(&self) -> PathBuf {
        self.root.join(OPENCODE_AGENTS_DIR)
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
    /// Derived from the platform registry: each [`crate::platforms::Platform`]
    /// contributes its `removal_root` plus every entry in `extra_dirs`. The
    /// `.ark/` root is always included. `walk_files` on a missing directory
    /// yields an empty vec, so un-installed platforms are silent no-ops.
    ///
    /// Claude's `removal_root` is `.claude/commands/ark/` (narrow, to coexist
    /// with user-authored Claude artifacts), so its agent directory at
    /// `.claude/agents/` is contributed via `extra_dirs`. Codex and OpenCode
    /// own their entire platform root; `extra_dirs` is empty for them.
    pub fn owned_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![self.ark_dir()];
        for p in crate::platforms::PLATFORMS {
            dirs.push(self.resolve(p.removal_root));
            for extra in p.extra_dirs {
                dirs.push(self.resolve(extra));
            }
        }
        dirs
    }

    /// Returns empty parent directories to prune after removing Ark content.
    pub fn prunable_empty_parents(&self) -> [PathBuf; 2] {
        [
            self.root.join(".claude/commands"),
            self.root.join(".claude"),
        ]
    }
}

/// Validates one feature-path segment against the kebab-case alphabet plus
/// reserved-name rules. Shared by `Layout::specs_feature_dir` and
/// `parse_spec_path` so the two constructors stay consistent.
pub(crate) fn validate_feature_path_segment(seg: &str, source_path: &Path) -> Result<()> {
    let invalid = |reason: &'static str| -> Error {
        Error::InvalidFeaturePath {
            source_path: source_path.to_path_buf(),
            value: seg.to_string(),
            reason,
        }
    };
    if seg.is_empty() {
        return Err(invalid("empty segment"));
    }
    if seg == "." {
        return Err(invalid("segment contains `.`"));
    }
    if seg == ".." {
        return Err(invalid("segment contains `..`"));
    }
    let lower = seg.to_ascii_lowercase();
    if lower == "index" || lower == "spec" {
        return Err(invalid("reserved segment name"));
    }
    let mut bytes = seg.bytes();
    let Some(first) = bytes.next() else {
        // Already rejected at the `is_empty()` guard above; defensive fallback.
        return Err(invalid("empty segment"));
    };
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return Err(invalid("segment must start with lowercase letter or digit"));
    }
    for b in bytes {
        if !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_') {
            return Err(invalid(
                "segment must be kebab-case (`^[a-z0-9][a-z0-9_-]*$`)",
            ));
        }
    }
    Ok(())
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

    fn fake_prd() -> PathBuf {
        PathBuf::from("/project/.ark/tasks/x/PRD.md")
    }

    /// Verifies happy-path joining of 1/2/3-segment feature paths.
    #[test]
    fn specs_feature_dir_joins_segments() {
        let l = layout();
        assert_eq!(
            l.specs_feature_dir(&["klib"], fake_prd()).unwrap(),
            PathBuf::from("/project/.ark/specs/features/klib"),
        );
        assert_eq!(
            l.specs_feature_dir(&["xemu", "csr"], fake_prd()).unwrap(),
            PathBuf::from("/project/.ark/specs/features/xemu/csr"),
        );
        assert_eq!(
            l.specs_feature_dir(&["core", "runtime", "scheduler"], fake_prd())
                .unwrap(),
            PathBuf::from("/project/.ark/specs/features/core/runtime/scheduler"),
        );
    }

    /// Empty segments slice is rejected (`InvalidFeaturePath`).
    #[test]
    fn specs_feature_dir_rejects_empty_segments() {
        let err = layout().specs_feature_dir(&[], fake_prd()).unwrap_err();
        assert!(matches!(err, Error::InvalidFeaturePath { .. }));
    }

    /// Reserved segment names and bad alphabets are rejected.
    #[test]
    fn specs_feature_dir_rejects_reserved_and_bad_alphabet() {
        for bad in ["INDEX", "index", "SPEC", "spec", ".", ".."] {
            let err = layout().specs_feature_dir(&[bad], fake_prd()).unwrap_err();
            assert!(
                matches!(err, Error::InvalidFeaturePath { .. }),
                "case `{bad}`"
            );
        }
        for bad in ["Xemu", "-csr", "x csr", "xemu/csr", "xemu.csr", ""] {
            let err = layout().specs_feature_dir(&[bad], fake_prd()).unwrap_err();
            assert!(
                matches!(err, Error::InvalidFeaturePath { .. }),
                "case `{bad}`"
            );
        }
    }

    /// Validation runs per segment; only the offending one is reported.
    #[test]
    fn specs_feature_dir_reports_first_bad_segment() {
        let err = layout()
            .specs_feature_dir(&["xemu", "Bad"], fake_prd())
            .unwrap_err();
        let Error::InvalidFeaturePath { value, .. } = err else {
            panic!("wrong variant");
        };
        assert_eq!(value, "Bad");
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

    #[test]
    fn slug_from_worktree_root_returns_some_for_canonical_path() {
        let l = Layout::new("/project/.ark/worktrees/feat/foo");
        assert_eq!(l.slug_from_worktree_root().as_deref(), Some("foo"));
    }

    #[test]
    fn slug_from_worktree_root_returns_none_for_parent_root() {
        let l = Layout::new("/project");
        assert_eq!(l.slug_from_worktree_root(), None);
    }

    #[test]
    fn slug_from_worktree_root_returns_none_for_short_path() {
        let l = Layout::new("/.ark/worktrees");
        assert_eq!(l.slug_from_worktree_root(), None);
    }

    #[test]
    fn slug_from_worktree_root_returns_none_when_marker_dirs_dont_match() {
        let l = Layout::new("/project/foo/bar/feat/baz");
        assert_eq!(l.slug_from_worktree_root(), None);
    }

    #[cfg(unix)]
    #[test]
    fn slug_from_worktree_root_returns_none_for_non_utf8_slug() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let bad = OsString::from_vec(vec![0xFF, 0xFE]);
        let mut p = PathBuf::from("/project/.ark/worktrees/feat");
        p.push(&bad);
        let l = Layout::new(p);
        assert_eq!(l.slug_from_worktree_root(), None);
    }
}
