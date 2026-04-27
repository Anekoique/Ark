//! Data model for `ark context`. Serialized to JSON via `serde`; the same
//! values render as text in [`super::render`].
//!
//! [`Context`] is the unprojected snapshot. [`super::projection`] derives a
//! [`super::projection::ProjectedContext`] from it per `--scope` / `--for`.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::commands::agent::state::{Phase, Tier};

/// JSON schema version. Bump when removing or renaming fields. Additive
/// changes do not bump.
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum number of dirty files reported in `git.dirty_files`. Total count
/// is reported separately in `git.uncommitted_changes`. Per ark-context C-25.
pub const DIRTY_FILES_CAP: usize = 20;

/// Maximum number of recent commits in `git.recent_commits`.
pub const RECENT_COMMITS_CAP: usize = 5;

/// Maximum number of archive entries in `archive.recent`.
pub const ARCHIVE_CAP: usize = 5;

/// Full unprojected snapshot. The projection layer reduces this per scope.
#[derive(Debug, Clone, Serialize)]
pub struct Context {
    pub schema: u32,
    pub generated_at: DateTime<Utc>,
    pub project_root: PathBuf,
    pub git: GitState,
    pub tasks: TasksState,
    pub specs: SpecsState,
    pub archive: ArchiveState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTask>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitState {
    pub branch: String,
    pub head_short: String,
    pub is_clean: bool,
    pub uncommitted_changes: u32,
    pub dirty_files: Vec<String>,
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

#[derive(Debug, Clone, Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TasksState {
    pub active: Vec<TaskSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSummary {
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub iteration: u32,
    pub path: PathBuf,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentTask {
    pub slug: String,
    pub summary: TaskSummary,
    pub artifacts: Vec<ArtifactSummary>,
    pub related_specs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactSummary {
    #[serde(flatten)]
    pub kind: ArtifactKind,
    pub path: PathBuf,
    pub lines: u32,
}

/// `kind`-tagged artifact discriminator; `Plan` and `Review` carry an
/// iteration so projections can pick the latest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ArtifactKind {
    Prd,
    Plan { iteration: u32 },
    Review { iteration: u32 },
    Verify,
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

#[derive(Debug, Clone, Default, Serialize)]
pub struct SpecsState {
    pub project: Vec<SpecRow>,
    pub features: Vec<SpecRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecRow {
    pub name: String,
    pub path: PathBuf,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promoted: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ArchiveState {
    pub recent: Vec<ArchivedTask>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchivedTask {
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub archived_at: DateTime<Utc>,
    pub path: PathBuf,
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
