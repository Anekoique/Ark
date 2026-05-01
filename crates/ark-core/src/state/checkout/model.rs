//! `.ark/.state.toml` model: identity, active task slugs, per-session focus.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Per-checkout state index.
///
/// Truth lives in `.ark/tasks/<slug>/task.toml`; this struct is reconciled
/// against that truth on every read by `state::checkout::reconcile`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateFile {
    /// Developer identity, when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,

    /// Active (non-archived) task slugs.
    #[serde(default)]
    pub tasks: Tasks,

    /// Session focus map keyed by UUID.
    ///
    /// `BTreeMap` for stable serialization order across writes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sessions: BTreeMap<String, Session>,
}

/// Per-checkout developer identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Developer name. Validated by `workspace::identity::validate_developer_name`.
    pub name: String,
    /// Timestamp the identity was first written.
    pub initialized_at: DateTime<Utc>,
}

/// Active task slugs.
///
/// Stored as `Vec` (TOML's only sequence type) and deduped on save.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tasks {
    /// Slugs of tasks created but not yet archived.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<String>,
}

/// Per-session focus pointer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Slug this session is currently driving.
    pub focus: String,
    /// Parent process id at session registration; the liveness-probe key.
    pub pid: u32,
}
