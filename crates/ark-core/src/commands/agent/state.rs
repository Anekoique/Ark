//! Task state: `task.toml` model, enums, and the legal-transition table.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    io::PathExt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Quick,
    Standard,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
    Archived,
}

/// Derived from [`Phase`]. Not persisted; computed on demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToml {
    pub id: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub iteration: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_iterations: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archived_at: Option<DateTime<Utc>>,
}

impl TaskToml {
    /// Derived status; not persisted.
    pub fn status(&self) -> Status {
        if self.phase == Phase::Archived {
            Status::Completed
        } else {
            Status::InProgress
        }
    }

    /// Load `task.toml` from a task directory (expects `<task_dir>/task.toml`).
    pub fn load(task_dir: &Path) -> Result<Self> {
        let path = task_dir.join("task.toml");
        let text = path.read_text()?;
        toml::from_str(&text).map_err(|source| Error::TaskTomlCorrupt { path, source })
    }

    /// Save to `<task_dir>/task.toml`, overwriting.
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
/// - Quick:    Design → Execute → Archived
/// - Standard: Design → Plan → Execute → Verify → Archived
/// - Deep:     Design → Plan ⇄ Review → Execute → Verify → Archived
///
/// `Review → Plan` is the "iterate" transition (deep tier only).
pub fn can_transition(tier: Tier, from: Phase, to: Phase) -> bool {
    use Phase::*;
    use Tier::*;
    match (tier, from, to) {
        // Quick
        (Quick, Design, Execute) => true,
        (Quick, Execute, Archived) => true,
        // Standard
        (Standard, Design, Plan) => true,
        (Standard, Plan, Execute) => true,
        (Standard, Execute, Verify) => true,
        (Standard, Verify, Archived) => true,
        // Deep
        (Deep, Design, Plan) => true,
        (Deep, Plan, Review) => true,
        (Deep, Review, Plan) => true, // iterate
        (Deep, Review, Execute) => true,
        (Deep, Execute, Verify) => true,
        (Deep, Verify, Archived) => true,
        _ => false,
    }
}

/// Wrap [`can_transition`] as a `Result`.
pub fn check_transition(tier: Tier, from: Phase, to: Phase) -> Result<()> {
    if can_transition(tier, from, to) {
        Ok(())
    } else {
        Err(Error::IllegalPhaseTransition { tier, from, to })
    }
}

/// Reject slugs that would escape `.ark/tasks/` or be unsafe as a file-system
/// component. Called at every `ark agent` entry point that joins a user-supplied
/// slug into a path.
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

/// Reject task titles that can't round-trip through `spec_register` as the
/// feature-scope column (which forbids `|` and newlines). Keeps deep-tier
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
        assert!(can_transition(Tier::Quick, Phase::Execute, Phase::Archived));
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
        assert!(can_transition(Tier::Deep, Phase::Verify, Phase::Archived));
        assert!(!can_transition(Tier::Deep, Phase::Plan, Phase::Execute));
        assert!(!can_transition(Tier::Deep, Phase::Design, Phase::Review));
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
