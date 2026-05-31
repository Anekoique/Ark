//! Data model for `ark context`.
//!
//! Serialized to JSON via `serde`; the same values render as text in
//! [`crate::commands::context::render`].
//!
//! [`Context`] is the unprojected snapshot.
//! [`crate::commands::context::projection`] derives a
//! [`crate::commands::context::projection::ProjectedContext`] from it per
//! `--scope` / `--for`.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::commands::agent::state::{Phase, Tier};

/// JSON schema version.
///
/// Bump when removing or renaming fields. Additive changes do not bump.
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum number of dirty files reported in `git.dirty_files`.
///
/// Total count is reported separately in `git.uncommitted_changes`.
pub const DIRTY_FILES_CAP: usize = 20;

/// Maximum number of recent commits in `git.recent_commits`.
pub const RECENT_COMMITS_CAP: usize = 5;

/// Maximum number of archive entries in `archive.recent`.
pub const ARCHIVE_CAP: usize = 5;

/// Maximum recursion depth for `build_features_tree`.
///
/// Mirrors the bound on `gather::parse_features_index` so the projected
/// tree never reaches a level the gather walker refused to descend into.
pub const FEATURES_TREE_MAX_DEPTH: usize = 8;

/// Full unprojected snapshot. The projection layer reduces this per scope.
#[derive(Debug, Clone, Serialize)]
pub struct Context {
    /// Context schema version.
    pub schema: u32,
    /// Timestamp when the context snapshot was generated.
    pub generated_at: DateTime<Utc>,
    /// Project root used for gathering.
    pub project_root: PathBuf,
    /// Git repository state.
    pub git: GitState,
    /// Active task state.
    pub tasks: TasksState,
    /// Project and feature SPEC state.
    pub specs: SpecsState,
    /// Recent archived task state.
    pub archive: ArchiveState,
    /// Current task details, when `.ark/tasks/.current` resolves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTask>,
    /// Per-checkout location info (main vs worktree, branch, focus slug).
    pub checkout: CheckoutInfo,
    /// Installed subagents grouped by platform. Empty `Vec` when no platform
    /// agent directory exists under `layout.root()`.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub subagents: Vec<SubagentSet>,
}

/// Git state included in context output.
#[derive(Debug, Clone, Serialize)]
pub struct GitState {
    /// Current branch name or `unknown`.
    pub branch: String,
    /// Short HEAD hash.
    pub head_short: String,
    /// Reports whether there are no uncommitted changes.
    pub is_clean: bool,
    /// Count of uncommitted status entries.
    pub uncommitted_changes: u32,
    /// Dirty file paths, capped by [`DIRTY_FILES_CAP`].
    pub dirty_files: Vec<String>,
    /// Recent commit summaries.
    pub recent_commits: Vec<GitCommit>,
}

impl Default for GitState {
    fn default() -> Self {
        Self {
            branch: "unknown".to_string(),
            head_short: String::new(),
            is_clean: true,
            uncommitted_changes: 0,
            dirty_files: Vec::new(),
            recent_commits: Vec::new(),
        }
    }
}

/// One recent git commit.
#[derive(Debug, Clone, Serialize)]
pub struct GitCommit {
    /// Short commit hash.
    pub hash: String,
    /// Commit subject.
    pub message: String,
}

/// Active task collection.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TasksState {
    /// Active task summaries.
    pub active: Vec<TaskSummary>,
}

/// Summary of one active task.
#[derive(Debug, Clone, Serialize)]
pub struct TaskSummary {
    /// Task slug.
    pub slug: String,
    /// Task title.
    pub title: String,
    /// Workflow tier.
    pub tier: Tier,
    /// Current lifecycle phase.
    pub phase: Phase,
    /// Path to the task directory.
    pub path: PathBuf,
    /// Timestamp when the task was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Expanded details for the current task.
#[derive(Debug, Clone, Serialize)]
pub struct CurrentTask {
    /// Current task slug.
    pub slug: String,
    /// Current task summary.
    pub summary: TaskSummary,
    /// Known artifact files for the current task.
    pub artifacts: Vec<ArtifactSummary>,
    /// Related feature SPEC paths parsed from the plan.
    pub related_specs: Vec<String>,
}

/// Summary of one task artifact file.
#[derive(Debug, Clone, Serialize)]
pub struct ArtifactSummary {
    #[serde(flatten)]
    /// Artifact kind and iteration metadata.
    pub kind: ArtifactKind,
    /// Artifact file path.
    pub path: PathBuf,
    /// Number of lines in the artifact.
    pub lines: u32,
}

/// Discriminates artifact kind.
///
/// `Plan` and `Review` carry an iteration so projections can pick the latest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ArtifactKind {
    /// Product requirements document.
    Prd,
    /// Plan artifact for one iteration.
    Plan {
        /// Plan iteration number.
        iteration: u32,
    },
    /// Review artifact for one iteration.
    Review {
        /// Review iteration number.
        iteration: u32,
    },
    /// Verification artifact.
    Verify,
    /// Task metadata artifact.
    TaskToml,
}

impl ArtifactKind {
    /// Iteration number for plan/review artifacts; `None` for everything else.
    pub fn iteration(&self) -> Option<u32> {
        match self {
            Self::Plan { iteration } | Self::Review { iteration } => Some(*iteration),
            _ => None,
        }
    }
}

/// Project and feature SPEC rows.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SpecsState {
    /// Project SPEC rows.
    pub project: Vec<SpecRow>,
    /// Feature SPEC rows.
    pub features: Vec<SpecRow>,
    /// Drift warnings emitted by the recursive features INDEX walk. Empty
    /// in healthy projects; populated when on-disk SPECs and INDEX rows
    /// disagree.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub features_warnings: Vec<GatherWarning>,
    /// Nested feature-spec tree derived from `features`. `Some` only when
    /// the projection selects it (Session and Phase(Design)) and the input
    /// `features` Vec is non-empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features_tree: Option<SpecNode>,
}

/// One row from a SPEC index.
#[derive(Debug, Clone, Serialize)]
pub struct SpecRow {
    /// SPEC name (last path segment / leaf slug).
    pub name: String,
    /// SPEC file path, project-root-relative (e.g.
    /// `.ark/specs/features/<...>/SPEC.md`). Preserved unchanged across the
    /// recursive-tree refactor for back-compat.
    pub path: PathBuf,
    /// Segments relative to `.ark/specs/features/`, e.g. `["xemu", "csr"]`.
    /// Empty for project SPEC rows. Single-segment for legacy flat layout.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub feature_path: Vec<String>,
    /// Scope text from the index row.
    pub scope: String,
    /// Promotion provenance for feature SPEC rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promoted: Option<String>,
}

/// Drift warning emitted by the recursive features INDEX walk.
///
/// `gather::parse_features_index` is INDEX-strict (rows declare traversal),
/// so any mismatch between rows and the filesystem surfaces as a warning
/// instead of being silently dropped. Consumers can ignore the field
/// (it serializes only when non-empty) or treat warnings as advisory
/// signals for index repair.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GatherWarning {
    /// An INDEX row points at a child file that does not exist on disk.
    MissingChild {
        /// The row's first cell verbatim.
        row: String,
        /// The expected on-disk path of the missing child.
        expected_path: PathBuf,
    },
    /// An on-disk leaf `SPEC.md` is not rowed in any parent INDEX.
    OrphanLeaf {
        /// On-disk path of the orphan leaf.
        path: PathBuf,
    },
    /// An on-disk subtree directory is not rowed in its parent INDEX.
    OrphanSubtree {
        /// On-disk path of the orphan subtree.
        path: PathBuf,
    },
}

/// Recent archive state.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ArchiveState {
    /// Recent archived tasks.
    pub recent: Vec<ArchivedTask>,
}

/// Summary of one archived task.
#[derive(Debug, Clone, Serialize)]
pub struct ArchivedTask {
    /// Task slug.
    pub slug: String,
    /// Task title.
    pub title: String,
    /// Workflow tier at archive time.
    pub tier: Tier,
    /// Timestamp when the task was archived.
    pub archived_at: DateTime<Utc>,
    /// Path to the archived task directory.
    pub path: PathBuf,
}

/// Per-checkout location info.
///
/// `root_kind` distinguishes the main checkout from a worktree; `branch`
/// mirrors `GitState::branch` (no extra git call); `focus_slug` reads the
/// per-checkout `.state.toml` `[focus]` slug.
#[derive(Debug, Clone, Serialize)]
pub struct CheckoutInfo {
    /// Whether this is the main checkout or a git worktree.
    pub root_kind: CheckoutRootKind,
    /// Current branch (same string as [`GitState::branch`]).
    pub branch: String,
    /// Slug bound to this checkout's `.state.toml` `[focus]`, when set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_slug: Option<String>,
}

impl Default for CheckoutInfo {
    fn default() -> Self {
        Self {
            root_kind: CheckoutRootKind::Main,
            branch: "unknown".to_string(),
            focus_slug: None,
        }
    }
}

/// Distinguishes the main git checkout from a worktree checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutRootKind {
    /// Main git checkout (`git rev-parse --show-toplevel` equals
    /// `git rev-parse --git-common-dir`'s parent).
    Main,
    /// Git worktree (toplevel differs from common-dir's parent).
    Worktree,
}

/// One node in the feature-SPECs tree projected from flat [`SpecRow`]s.
///
/// Branches carry a `segment` (path component at this depth) and sorted
/// children; leaves carry the per-row metadata mirrored from `SpecRow`.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SpecNode {
    /// Non-leaf node grouping children under a `segment` name.
    Branch {
        /// Path component at this depth.
        segment: String,
        /// Sorted children (branches and leaves intermixed).
        children: Vec<SpecNode>,
    },
    /// Leaf node carrying one SPEC's metadata.
    Leaf {
        /// Last `feature_path` segment for this leaf.
        segment: String,
        /// Mirrors [`SpecRow::name`].
        name: String,
        /// Mirrors [`SpecRow::path`].
        path: PathBuf,
        /// Mirrors [`SpecRow::scope`].
        scope: String,
        /// Mirrors [`SpecRow::promoted`].
        #[serde(skip_serializing_if = "Option::is_none")]
        promoted: Option<String>,
    },
}

/// Installed agents for one platform under this checkout's disk layout.
///
/// `platform` carries the platform's `cli_flag` verbatim (`claude`, `codex`,
/// `opencode`). `stems` lists every agent file's stem, sorted ascending,
/// including user-installed agents alongside Ark canonicals.
#[derive(Debug, Clone, Serialize)]
pub struct SubagentSet {
    /// Platform tag (verbatim `Platform::cli_flag`).
    pub platform: String,
    /// Sorted ascending list of agent stems found in the platform's
    /// `agents_dest_dir`.
    pub stems: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_kind_iteration_returns_some_for_plan_and_review() {
        assert_eq!(ArtifactKind::Plan { iteration: 3 }.iteration(), Some(3));
        assert_eq!(ArtifactKind::Review { iteration: 0 }.iteration(), Some(0));
    }

    #[test]
    fn artifact_kind_iteration_returns_none_for_others() {
        assert_eq!(ArtifactKind::Prd.iteration(), None);
        assert_eq!(ArtifactKind::Verify.iteration(), None);
        assert_eq!(ArtifactKind::TaskToml.iteration(), None);
    }
}
