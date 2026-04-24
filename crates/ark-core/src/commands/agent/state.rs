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
