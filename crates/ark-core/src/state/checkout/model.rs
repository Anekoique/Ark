//! `.ark/.state.toml` model: active task slugs and per-checkout focus.

use serde::{Deserialize, Serialize};

/// Per-checkout state index.
///
/// Truth lives in `.ark/tasks/<slug>/task.toml`; this struct is reconciled
/// against that truth on every read by `state::checkout::reconcile`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateFile {
    /// Active (non-archived) task slugs.
    #[serde(default)]
    pub tasks: Tasks,

    /// Slug this checkout is currently driving. `None` when no task is bound.
    ///
    /// Set by `task new` and `task resume`; cleared by `task archive` and
    /// `task discard` when the cleared slug matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
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
