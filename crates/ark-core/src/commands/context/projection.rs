//! Project a [`Context`] down to a [`ProjectedContext`] per `--scope`/`--for`.
//!
//! Pure functions of `&Context` + [`Scope`]. No I/O.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::commands::context::model::{
    ArchiveState, CheckoutInfo, Context, CurrentTask, GitState, SpecRow, SpecsState, SubagentSet,
    TasksState,
};

/// Top-level scope selector. `Phase` carries the concrete phase filter.
/// Context projection scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Full session-start context.
    Session,
    /// Phase-specific context.
    Phase(PhaseFilter),
    /// Workspace record context (developer identity + active journal).
    ///
    /// Used by the `/ark:record` slash command's draft-render step to seed
    /// the agent's empty `### Summary` / `### Main Changes` placeholders
    /// with the right header information.
    Record,
}

/// Phase selector for scoped context projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PhaseFilter {
    /// Design-phase projection.
    Design,
    /// Plan-phase projection.
    Plan,
    /// Review-phase projection.
    Review,
    /// Execute-phase projection.
    Execute,
    /// Verify-phase projection.
    Verify,
    /// Commit-phase projection (slash command reads VERIFY.md + latest plan
    /// from disk via the returned paths; payload itself stays body-free per
    /// the `ark-context` SPEC's additive-only schema).
    Commit,
}

/// Serializable tag identifying the projection scope.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "scope", rename_all = "lowercase")]
pub enum ScopeTag {
    /// Session-start projection.
    Session,
    /// Phase-specific projection.
    Phase {
        /// Selected phase.
        phase: PhaseFilter,
    },
    /// Workspace record projection.
    Record,
}

/// Workspace record projection payload (additive on `ProjectedContext`).
///
/// Populated when `Scope::Record` is selected. Empty `Option`s communicate
/// "not set yet" without erroring; the slash command surface decides how to
/// behave (e.g., prompt for identity, default to a fresh journal).
#[derive(Debug, Clone, Default, Serialize)]
pub struct RecordProjection {
    /// Resolved developer name (None when `.ark/.developer` is absent and no
    /// `[workspace] developer` override is set).
    pub identity: Option<String>,
    /// Project-relative path of the active `journal-N.md` (None on a fresh
    /// install with no entries yet).
    pub active_journal_path: Option<String>,
    /// Configured rotation threshold (lines).
    pub journal_max_lines: usize,
    /// Total existing sessions across the developer's journals.
    pub session_count: u32,
    /// Current git branch (best-effort; None when git is unavailable).
    pub branch: Option<String>,
}

/// Context view after applying a scope projection.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectedContext {
    /// Context schema version.
    pub schema: u32,
    #[serde(flatten)]
    /// Serializable scope tag.
    pub scope: ScopeTag,
    /// Timestamp when the context snapshot was generated.
    pub generated_at: DateTime<Utc>,
    /// Project root used for gathering.
    pub project_root: PathBuf,
    /// Git repository state.
    pub git: GitState,
    /// Per-checkout location info (always populated; mirrors the gather
    /// pass's `CheckoutInfo`).
    pub checkout: CheckoutInfo,
    /// Active task state, when included by the projection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TasksState>,
    /// Current task state, when included by the projection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTask>,
    /// SPEC rows, when included by the projection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs: Option<SpecsState>,
    /// Archive rows, when included by the projection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveState>,
    /// Workspace record context, when `Scope::Record` or `Phase(Commit)`
    /// is selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<RecordProjection>,
    /// Installed subagent stems per platform. Populated on `Scope::Session`
    /// and on `Phase(Design / Plan / Review / Verify)`. Empty `Vec` on
    /// `Phase(Execute / Commit)` and `Scope::Record`.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub subagents: Vec<SubagentSet>,
    /// `Some(true)` when session-envelope wrapping had to drop fields to fit
    /// the host-side context cap (Claude Code documents 10K chars). Absent
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

/// Projects `ctx` per `scope`.
pub fn project(ctx: Context, scope: Scope) -> ProjectedContext {
    let Context {
        schema,
        generated_at,
        project_root,
        git,
        tasks,
        specs,
        archive,
        current_task,
        checkout,
        subagents,
    } = ctx;

    match scope {
        Scope::Session => ProjectedContext {
            schema,
            scope: ScopeTag::Session,
            generated_at,
            project_root,
            git,
            checkout,
            tasks: Some(tasks),
            current_task,
            specs: Some(specs),
            archive: Some(archive),
            record: None,
            // Per C-30: subagents populated on Session.
            subagents,
            truncated: None,
        },
        Scope::Phase(phase) => {
            let mut projected = ProjectedContext {
                schema,
                scope: ScopeTag::Phase { phase },
                generated_at,
                project_root,
                git,
                checkout,
                tasks: None,
                current_task,
                specs: None,
                archive: None,
                record: None,
                // Filled in by `apply_phase_filter` per C-30 placement matrix.
                subagents: Vec::new(),
                truncated: None,
            };
            apply_phase_filter(&mut projected, phase, specs, archive, subagents);
            projected
        }
        Scope::Record => ProjectedContext {
            schema,
            scope: ScopeTag::Record,
            generated_at,
            project_root,
            git,
            checkout,
            tasks: None,
            current_task: None,
            specs: None,
            archive: None,
            // Filled in by the `context()` entry point after projection;
            // the projector is pure (no I/O) and `Record` requires reading
            // the workspace tree.
            record: Some(RecordProjection::default()),
            // Per C-30: subagents are empty on Scope::Record.
            subagents: Vec::new(),
            truncated: None,
        },
    }
}

fn apply_phase_filter(
    out: &mut ProjectedContext,
    phase: PhaseFilter,
    specs: SpecsState,
    archive: ArchiveState,
    subagents: Vec<SubagentSet>,
) {
    let SpecsState {
        project,
        features,
        features_warnings,
        features_tree,
    } = specs;
    match phase {
        PhaseFilter::Design => {
            out.specs = Some(SpecsState {
                project,
                features,
                features_warnings,
                features_tree,
            });
            out.archive = Some(archive);
            out.subagents = subagents;
        }
        PhaseFilter::Plan | PhaseFilter::Review => {
            let related = out
                .current_task
                .as_ref()
                .map(|c| c.related_specs.as_slice())
                .unwrap_or(&[]);
            let filtered = filter_features_by_related(features, related);
            out.specs = Some(SpecsState {
                project,
                features: filtered,
                features_warnings,
                // Plan/Review filter to related; the tree is for orientation
                // and belongs to Session/Design only (C-30 placement matrix).
                features_tree: None,
            });
            out.subagents = subagents;
        }
        PhaseFilter::Verify => {
            // Verify gets project specs + subagents (the slash command's
            // "STOP and ask which verifier" prompt needs the installed
            // agent list). Features are not surfaced.
            out.specs = Some(SpecsState {
                project,
                features: Vec::new(),
                features_warnings: Vec::new(),
                features_tree: None,
            });
            out.subagents = subagents;
        }
        // Execute / Commit want project specs only. Commit is body-free:
        // the slash command reads VERIFY.md and the latest plan from the
        // path fields the projection already carries on `current_task`.
        // Commit additionally carries `record` (set below).
        PhaseFilter::Execute | PhaseFilter::Commit => {
            out.specs = Some(SpecsState {
                project,
                features: Vec::new(),
                features_warnings: Vec::new(),
                features_tree: None,
            });
            if matches!(phase, PhaseFilter::Commit) {
                // C-42: reuse the same `RecordProjection` shape that powers
                // `Scope::Record`. The `context()` entry point fills the
                // body after projection (pure-projector contract).
                out.record = Some(RecordProjection::default());
            }
        }
    }
}

/// Keeps feature specs whose paths match a related-spec entry.
///
/// A `SpecRow` `f` is kept iff any `r` in `related` satisfies
/// `normalize(r).ends_with(&normalize(f.path))`. Both sides strip leading
/// `./` and `.ark/`.
fn filter_features_by_related(features: Vec<SpecRow>, related: &[String]) -> Vec<SpecRow> {
    if related.is_empty() {
        return Vec::new();
    }
    let normalized_related: Vec<String> = related.iter().map(|r| normalize_path(r)).collect();
    features
        .into_iter()
        .filter(|f| {
            let f_path_str = f.path.to_string_lossy();
            let f_norm = normalize_path(&f_path_str);
            normalized_related.iter().any(|r| r.ends_with(&f_norm))
        })
        .collect()
}

fn normalize_path(s: &str) -> String {
    let t = s.trim();
    let t = t.strip_prefix("./").unwrap_or(t);
    let t = t.strip_prefix(".ark/").unwrap_or(t);
    t.to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::commands::context::model::{
        ArchiveState, Context, GitState, SCHEMA_VERSION, SpecRow, SpecsState, TasksState,
    };

    fn ctx_with(specs: SpecsState, current_task: Option<CurrentTask>) -> Context {
        Context {
            schema: SCHEMA_VERSION,
            generated_at: Utc::now(),
            project_root: PathBuf::from("/tmp/proj"),
            git: GitState::default(),
            tasks: TasksState::default(),
            specs,
            archive: ArchiveState::default(),
            current_task,
            checkout: CheckoutInfo::default(),
            subagents: Vec::new(),
        }
    }

    fn row(name: &str) -> SpecRow {
        SpecRow {
            name: name.to_string(),
            path: PathBuf::from(format!(".ark/specs/features/{name}/SPEC.md")),
            feature_path: vec![name.to_string()],
            scope: format!("scope of {name}"),
            promoted: None,
        }
    }

    #[test]
    fn session_scope_includes_all_sections() {
        let ctx = ctx_with(
            SpecsState {
                project: vec![row("p1")],
                features: vec![row("f1"), row("f2")],
                features_warnings: Vec::new(),
                features_tree: None,
            },
            None,
        );
        let pj = project(ctx, Scope::Session);
        assert!(pj.tasks.is_some());
        assert!(pj.specs.is_some());
        assert!(pj.archive.is_some());
    }

    #[test]
    fn design_phase_keeps_full_specs_and_archive() {
        let ctx = ctx_with(
            SpecsState {
                project: vec![row("p1")],
                features: vec![row("f1"), row("f2")],
                features_warnings: Vec::new(),
                features_tree: None,
            },
            None,
        );
        let pj = project(ctx, Scope::Phase(PhaseFilter::Design));
        assert!(pj.tasks.is_none());
        assert!(pj.archive.is_some());
        let s = pj.specs.unwrap();
        assert_eq!(s.features.len(), 2);
    }

    #[test]
    fn plan_phase_filters_features_to_related() {
        let related = vec!["specs/features/foo/SPEC.md".to_string()];
        let ct = CurrentTask {
            slug: "task".to_string(),
            summary: dummy_summary(),
            artifacts: Vec::new(),
            related_specs: related,
        };
        let ctx = ctx_with(
            SpecsState {
                project: vec![row("p1")],
                features: vec![row("foo"), row("bar"), row("baz")],
                features_warnings: Vec::new(),
                features_tree: None,
            },
            Some(ct),
        );
        let pj = project(ctx, Scope::Phase(PhaseFilter::Plan));
        let s = pj.specs.unwrap();
        assert_eq!(s.features.len(), 1);
        assert_eq!(s.features[0].name, "foo");
        assert!(pj.archive.is_none());
        assert!(pj.tasks.is_none());
    }

    #[test]
    fn execute_phase_yields_empty_features() {
        let ctx = ctx_with(
            SpecsState {
                project: vec![row("p1")],
                features: vec![row("f1")],
                features_warnings: Vec::new(),
                features_tree: None,
            },
            None,
        );
        let pj = project(ctx, Scope::Phase(PhaseFilter::Execute));
        let s = pj.specs.unwrap();
        assert!(s.features.is_empty());
        assert_eq!(s.project.len(), 1);
    }

    #[test]
    fn review_phase_filters_same_as_plan() {
        let related = vec!["specs/features/foo/SPEC.md".to_string()];
        let ct = CurrentTask {
            slug: "task".to_string(),
            summary: dummy_summary(),
            artifacts: Vec::new(),
            related_specs: related,
        };
        let ctx = ctx_with(
            SpecsState {
                project: vec![row("p1")],
                features: vec![row("foo"), row("bar")],
                features_warnings: Vec::new(),
                features_tree: None,
            },
            Some(ct),
        );
        let pj = project(ctx, Scope::Phase(PhaseFilter::Review));
        let s = pj.specs.unwrap();
        assert_eq!(s.features.len(), 1);
    }

    #[test]
    fn verify_phase_yields_empty_features() {
        let ctx = ctx_with(
            SpecsState {
                project: vec![row("p1")],
                features: vec![row("f1")],
                features_warnings: Vec::new(),
                features_tree: None,
            },
            None,
        );
        let pj = project(ctx, Scope::Phase(PhaseFilter::Verify));
        let s = pj.specs.unwrap();
        assert!(s.features.is_empty());
    }

    /// V-IT-9 / R-204: the commit projection is body-free per the
    /// `ark-context` SPEC's additive-only schema. Slash commands must read
    /// VERIFY.md and the latest plan from the artifact paths the projection
    /// already carries on `current_task`, not from any payload field.
    #[test]
    fn commit_phase_yields_paths_only_no_bodies() {
        let ctx = ctx_with(
            SpecsState {
                project: vec![row("p1")],
                features: vec![row("f1")],
                features_warnings: Vec::new(),
                features_tree: None,
            },
            None,
        );
        let pj = project(ctx, Scope::Phase(PhaseFilter::Commit));
        let s = pj.specs.as_ref().unwrap();
        // No feature bodies (empty list); project SPECs are paths-only by the
        // `SpecRow` shape itself.
        assert!(s.features.is_empty());
        assert_eq!(s.project.len(), 1);
        // Sanity: the projected JSON shape has no "verify_md_body" or
        // similar body-carrying field. Serializing and rescanning is the
        // robust check.
        let serialized = serde_json::to_string(&pj).unwrap();
        assert!(
            !serialized.contains("verify_md_body"),
            "commit projection must not carry verify body: {serialized}"
        );
        assert!(
            !serialized.contains("plan_body"),
            "commit projection must not carry plan body: {serialized}"
        );
    }

    #[test]
    fn empty_related_specs_yields_no_features_in_plan_phase() {
        let ct = CurrentTask {
            slug: "task".to_string(),
            summary: dummy_summary(),
            artifacts: Vec::new(),
            related_specs: Vec::new(),
        };
        let ctx = ctx_with(
            SpecsState {
                project: vec![row("p1")],
                features: vec![row("foo")],
                features_warnings: Vec::new(),
                features_tree: None,
            },
            Some(ct),
        );
        let pj = project(ctx, Scope::Phase(PhaseFilter::Plan));
        let s = pj.specs.unwrap();
        assert!(s.features.is_empty());
    }

    fn dummy_summary() -> crate::commands::context::model::TaskSummary {
        crate::commands::context::model::TaskSummary {
            slug: "task".to_string(),
            title: "title".to_string(),
            tier: crate::commands::agent::state::Tier::Deep,
            phase: crate::commands::agent::state::Phase::Plan,
            iteration: 0,
            path: PathBuf::from(".ark/tasks/task"),
            updated_at: Utc::now(),
        }
    }
}
