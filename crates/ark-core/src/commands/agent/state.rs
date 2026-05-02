//! Task state: `task.toml` model, enums, and the legal-transition table.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    io::PathExt,
};

/// Workflow tier selected for an Ark task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Minimal lifecycle: design, execute, archive.
    Quick,
    /// Standard lifecycle: design, plan, execute, verify, archive.
    Standard,
    /// Deep lifecycle with review iterations and feature SPEC promotion.
    Deep,
}

/// Lifecycle phase recorded in a task's `task.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// Initial task definition phase.
    Design,
    /// Planning phase.
    Plan,
    /// Deep-tier review phase.
    Review,
    /// Implementation phase.
    Execute,
    /// Verification phase.
    Verify,
    /// Closure phase: work + journal + task.toml + (deep) SPEC committed atomically.
    Committed,
    /// Terminal archived phase.
    Archived,
}

/// Derived from [`Phase`]. Not persisted; computed on demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Task has not reached its terminal phase.
    InProgress,
    /// Task has reached its terminal phase.
    Completed,
}

/// Serialized model stored in each task's `task.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToml {
    /// Task slug.
    pub id: String,
    /// Human-readable task title.
    pub title: String,
    /// Workflow tier.
    pub tier: Tier,
    /// Current lifecycle phase.
    pub phase: Phase,
    /// Current deep-tier plan/review iteration.
    pub iteration: u32,
    /// Maximum deep-tier review iterations.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_iterations: Option<u32>,
    /// Timestamp when the task was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp when the task state last changed.
    pub updated_at: DateTime<Utc>,
    /// Timestamp when the task was archived.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archived_at: Option<DateTime<Utc>>,

    /// Timestamp when `task_commit` flipped the task to `Committed`.
    ///
    /// Drives the YYYY-MM bucket selected by `ark archive`. `None` until the
    /// task has been committed; `None` for pre-refactor tasks.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub committed_at: Option<DateTime<Utc>>,

    /// Stores the worktree branch ref set by `task new --worktree`.
    ///
    /// Stored verbatim.
    /// Persists across the task lifecycle.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub branch: Option<String>,

    /// Stores the project-relative worktree path.
    ///
    /// Resolved against `Layout::root()` at read time. Forward-slash
    /// separators on disk; `PathBuf` deserializes both styles transparently.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub worktree_path: Option<std::path::PathBuf>,

    /// Stores the branch ref or SHA captured at worktree-create time.
    ///
    /// May be a SHA when invoked from detached HEAD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_branch: Option<String>,

    /// HEAD SHA captured at `task new` time, used as the start of the journal
    /// entry's commit-range table.
    ///
    /// `None` on unborn HEAD or for pre-refactor tasks.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_head: Option<String>,
}

impl TaskToml {
    /// Returns the derived [`Status`], computed from the current [`Phase`].
    pub fn status(&self) -> Status {
        if self.phase == Phase::Archived {
            Status::Completed
        } else {
            Status::InProgress
        }
    }

    /// Loads `task.toml` from a task directory (expects `<task_dir>/task.toml`).
    pub fn load(task_dir: &Path) -> Result<Self> {
        let path = task_dir.join("task.toml");
        let text = path.read_text()?;
        toml::from_str(&text).map_err(|source| Error::TaskTomlCorrupt { path, source })
    }

    /// Saves to `<task_dir>/task.toml`, overwriting.
    pub fn save(&self, task_dir: &Path) -> Result<()> {
        let path = task_dir.join("task.toml");
        let text = toml::to_string_pretty(self).expect("TaskToml serializes");
        path.write_bytes(text.as_bytes())
    }
}

/// `true` if `(tier, from, to)` is a legal phase transition.
///
/// The table encodes the state machines documented in `.ark/workflow.md` §4:
///
/// - Quick:    Design → Execute → Committed → Archived
/// - Standard: Design → Plan → Execute → Verify → Committed → Archived
/// - Deep:     Design → Plan ⇄ Review → Execute → Verify → Committed → Archived
///
/// `Review → Plan` is the "iterate" transition (deep tier only). `Archived`
/// is reachable only from `Committed`; the legacy direct
/// `Verify → Archived` / `Execute → Archived` transitions were removed by
/// the workflow refactor.
pub fn can_transition(tier: Tier, from: Phase, to: Phase) -> bool {
    use Phase::*;
    use Tier::*;
    match (tier, from, to) {
        // Quick
        (Quick, Design, Execute) => true,
        (Quick, Execute, Committed) => true,
        (Quick, Committed, Archived) => true,
        // Standard
        (Standard, Design, Plan) => true,
        (Standard, Plan, Execute) => true,
        (Standard, Execute, Verify) => true,
        (Standard, Verify, Committed) => true,
        (Standard, Committed, Archived) => true,
        // Deep
        (Deep, Design, Plan) => true,
        (Deep, Plan, Review) => true,
        (Deep, Review, Plan) => true, // iterate
        (Deep, Review, Execute) => true,
        (Deep, Execute, Verify) => true,
        (Deep, Verify, Committed) => true,
        (Deep, Committed, Archived) => true,
        _ => false,
    }
}

/// Returns an error if the phase transition is illegal for the given tier.
pub fn check_transition(tier: Tier, from: Phase, to: Phase) -> Result<()> {
    if can_transition(tier, from, to) {
        Ok(())
    } else {
        Err(Error::IllegalPhaseTransition { tier, from, to })
    }
}

/// Rejects slugs that would escape `.ark/tasks/`.
///
/// Also rejects values that would be unsafe as a file-system component. Called
/// at every `ark agent` entry point that joins a user-supplied slug into a path.
///
/// Rules: non-empty; no path separators (`/`, `\`); no `..` / `.`; no absolute
/// root; no leading/trailing whitespace; ASCII printable non-whitespace only.
pub fn validate_slug(slug: &str) -> Result<()> {
    let invalid = |reason: &'static str| Error::InvalidTaskField {
        field: "slug".into(),
        reason,
    };
    if slug.is_empty() {
        return Err(invalid("empty"));
    }
    if slug.trim() != slug {
        return Err(invalid("leading or trailing whitespace"));
    }
    if slug == "." || slug == ".." {
        return Err(invalid("reserved name"));
    }
    for ch in slug.chars() {
        match ch {
            '/' | '\\' => return Err(invalid("contains path separator")),
            c if c.is_ascii_control() => return Err(invalid("contains control character")),
            c if c.is_whitespace() => return Err(invalid("contains whitespace")),
            c if !c.is_ascii() => return Err(invalid("non-ASCII character")),
            _ => {}
        }
    }
    Ok(())
}

/// Rejects task titles that cannot round-trip through `spec_register`.
///
/// The feature-scope column forbids `|` and newlines. This keeps deep-tier
/// archive from failing on titles that were accepted at creation time.
pub fn validate_title(title: &str) -> Result<()> {
    let invalid = |reason: &'static str| Error::InvalidTaskField {
        field: "title".into(),
        reason,
    };
    if title.trim().is_empty() {
        return Err(invalid("empty"));
    }
    if title.contains('|') {
        return Err(invalid("contains `|`"));
    }
    if title.contains('\n') || title.contains('\r') {
        return Err(invalid("contains newline"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> TaskToml {
        TaskToml {
            id: "demo".into(),
            title: "demo task".into(),
            tier: Tier::Standard,
            phase: Phase::Design,
            iteration: 0,
            max_iterations: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
            committed_at: None,
            branch: None,
            worktree_path: None,
            base_branch: None,
            start_head: None,
        }
    }

    #[test]
    fn status_is_derived_from_phase() {
        let mut t = sample();
        assert_eq!(t.status(), Status::InProgress);
        t.phase = Phase::Archived;
        assert_eq!(t.status(), Status::Completed);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let t = sample();
        t.save(tmp.path()).unwrap();
        let loaded = TaskToml::load(tmp.path()).unwrap();
        assert_eq!(loaded.id, t.id);
        assert_eq!(loaded.tier, t.tier);
        assert_eq!(loaded.phase, t.phase);
    }

    #[test]
    fn load_errors_on_corrupt_toml() {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path()
            .join("task.toml")
            .write_bytes(b"not = valid = toml")
            .unwrap();
        let err = TaskToml::load(tmp.path()).unwrap_err();
        assert!(matches!(err, Error::TaskTomlCorrupt { .. }));
    }

    #[test]
    fn can_transition_quick() {
        assert!(can_transition(Tier::Quick, Phase::Design, Phase::Execute));
        assert!(can_transition(
            Tier::Quick,
            Phase::Execute,
            Phase::Committed
        ));
        assert!(can_transition(
            Tier::Quick,
            Phase::Committed,
            Phase::Archived
        ));
        assert!(!can_transition(
            Tier::Quick,
            Phase::Execute,
            Phase::Archived
        ));
        assert!(!can_transition(Tier::Quick, Phase::Design, Phase::Plan));
        assert!(!can_transition(Tier::Quick, Phase::Execute, Phase::Verify));
    }

    #[test]
    fn can_transition_standard() {
        assert!(can_transition(Tier::Standard, Phase::Design, Phase::Plan));
        assert!(can_transition(Tier::Standard, Phase::Plan, Phase::Execute));
        assert!(can_transition(
            Tier::Standard,
            Phase::Execute,
            Phase::Verify
        ));
        assert!(can_transition(
            Tier::Standard,
            Phase::Verify,
            Phase::Committed
        ));
        assert!(can_transition(
            Tier::Standard,
            Phase::Committed,
            Phase::Archived
        ));
        assert!(!can_transition(
            Tier::Standard,
            Phase::Verify,
            Phase::Archived
        ));
        assert!(!can_transition(Tier::Standard, Phase::Plan, Phase::Review));
        assert!(!can_transition(
            Tier::Standard,
            Phase::Design,
            Phase::Execute
        ));
    }

    #[test]
    fn can_transition_deep() {
        assert!(can_transition(Tier::Deep, Phase::Design, Phase::Plan));
        assert!(can_transition(Tier::Deep, Phase::Plan, Phase::Review));
        assert!(can_transition(Tier::Deep, Phase::Review, Phase::Plan));
        assert!(can_transition(Tier::Deep, Phase::Review, Phase::Execute));
        assert!(can_transition(Tier::Deep, Phase::Execute, Phase::Verify));
        assert!(can_transition(Tier::Deep, Phase::Verify, Phase::Committed));
        assert!(can_transition(
            Tier::Deep,
            Phase::Committed,
            Phase::Archived
        ));
        assert!(!can_transition(Tier::Deep, Phase::Verify, Phase::Archived));
        assert!(!can_transition(Tier::Deep, Phase::Plan, Phase::Execute));
        assert!(!can_transition(Tier::Deep, Phase::Design, Phase::Review));
    }

    /// Verifies that `Archived` is reachable only from `Committed`, across every tier.
    #[test]
    fn archived_only_reachable_from_committed() {
        for tier in [Tier::Quick, Tier::Standard, Tier::Deep] {
            for from in [
                Phase::Design,
                Phase::Plan,
                Phase::Review,
                Phase::Execute,
                Phase::Verify,
            ] {
                assert!(
                    !can_transition(tier, from, Phase::Archived),
                    "{tier:?} {from:?} → Archived should be illegal"
                );
            }
            assert!(
                can_transition(tier, Phase::Committed, Phase::Archived),
                "{tier:?} Committed → Archived should be legal"
            );
        }
    }

    #[test]
    fn archived_is_terminal() {
        for tier in [Tier::Quick, Tier::Standard, Tier::Deep] {
            for to in [
                Phase::Design,
                Phase::Plan,
                Phase::Review,
                Phase::Execute,
                Phase::Verify,
                Phase::Committed,
                Phase::Archived,
            ] {
                assert!(
                    !can_transition(tier, Phase::Archived, to),
                    "archived should be terminal for {tier:?} → {to:?}"
                );
            }
        }
    }

    #[test]
    fn validate_slug_accepts_ordinary() {
        for slug in ["ok", "task-1", "a_b_c", "feat-42"] {
            assert!(validate_slug(slug).is_ok(), "{slug}");
        }
    }

    #[test]
    fn validate_slug_rejects_traversal_and_separators() {
        for bad in [
            "",
            ".",
            "..",
            "../escape",
            "/abs",
            "a/b",
            "a\\b",
            "has space",
            "\ttab",
            "bad\n",
            "a/b/c",
            " leading",
            "trailing ",
            "emoji😀",
        ] {
            assert!(
                matches!(validate_slug(bad), Err(Error::InvalidTaskField { .. })),
                "expected reject for {bad:?}"
            );
        }
    }

    #[test]
    fn validate_title_accepts_ordinary() {
        for t in ["demo", "Add feature X", "fix: handle edge case"] {
            assert!(validate_title(t).is_ok(), "{t}");
        }
    }

    #[test]
    fn validate_title_rejects_pipe_and_newlines() {
        for bad in ["", "   ", "A | B", "line1\nline2", "carriage\rreturn"] {
            assert!(
                matches!(validate_title(bad), Err(Error::InvalidTaskField { .. })),
                "expected reject for {bad:?}"
            );
        }
    }

    /// Verifies that a pre-existing `task.toml` without the optional fields still loads.
    #[test]
    fn task_toml_loads_without_optional_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let legacy = r#"
id = "legacy"
title = "legacy task"
tier = "deep"
phase = "design"
iteration = 0
created_at = "2026-01-01T00:00:00Z"
updated_at = "2026-01-01T00:00:00Z"
"#;
        tmp.path()
            .join("task.toml")
            .write_bytes(legacy.as_bytes())
            .unwrap();
        let loaded = TaskToml::load(tmp.path()).unwrap();
        assert!(loaded.branch.is_none());
        assert!(loaded.worktree_path.is_none());
        assert!(loaded.base_branch.is_none());
        assert!(loaded.start_head.is_none());
        assert!(loaded.committed_at.is_none());
    }

    #[test]
    fn task_toml_round_trips_with_optional_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let mut t = sample();
        t.branch = Some("feat/foo".into());
        t.worktree_path = Some(".ark/worktrees/feat/foo".into());
        t.base_branch = Some("main".into());
        t.start_head = Some("abc123".into());
        t.committed_at = Some(Utc::now());
        t.save(tmp.path()).unwrap();
        let loaded = TaskToml::load(tmp.path()).unwrap();
        assert_eq!(loaded.branch.as_deref(), Some("feat/foo"));
        assert_eq!(
            loaded.worktree_path.as_deref(),
            Some(std::path::Path::new(".ark/worktrees/feat/foo"))
        );
        assert_eq!(loaded.base_branch.as_deref(), Some("main"));
        assert_eq!(loaded.start_head.as_deref(), Some("abc123"));
        assert!(loaded.committed_at.is_some());
    }

    #[test]
    fn check_transition_returns_named_error() {
        let err = check_transition(Tier::Quick, Phase::Design, Phase::Plan).unwrap_err();
        assert!(matches!(
            err,
            Error::IllegalPhaseTransition {
                tier: Tier::Quick,
                from: Phase::Design,
                to: Phase::Plan,
            }
        ));
    }
}
