//! Project a [`Context`] down to a [`ProjectedContext`] per `--scope`/`--for`.
//!
//! Pure functions of `&Context` + [`Scope`]. No I/O.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::commands::context::model::{
    ArchiveState, Context, CurrentTask, GitState, SpecRow, SpecsState, TasksState,
};

/// Top-level scope selector. `Phase` carries the concrete phase filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Session,
    Phase(PhaseFilter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PhaseFilter {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "scope", rename_all = "lowercase")]
pub enum ScopeTag {
    Session,
    Phase { phase: PhaseFilter },
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectedContext {
    pub schema: u32,
    #[serde(flatten)]
    pub scope: ScopeTag,
    pub generated_at: DateTime<Utc>,
    pub project_root: PathBuf,
    pub git: GitState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TasksState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<CurrentTask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs: Option<SpecsState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveState>,
}

/// Project `ctx` per `scope`. See ark-context plan G-6 / G-7.
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
    } = ctx;

    match scope {
        Scope::Session => ProjectedContext {
            schema,
            scope: ScopeTag::Session,
            generated_at,
            project_root,
            git,
            tasks: Some(tasks),
            current_task,
            specs: Some(specs),
            archive: Some(archive),
        },
        Scope::Phase(phase) => {
            let mut projected = ProjectedContext {
                schema,
                scope: ScopeTag::Phase { phase },
                generated_at,
                project_root,
                git,
                tasks: None,
                current_task,
                specs: None,
                archive: None,
            };
            apply_phase_filter(&mut projected, phase, specs, archive);
            projected
        }
    }
}

fn apply_phase_filter(
    out: &mut ProjectedContext,
    phase: PhaseFilter,
    specs: SpecsState,
    archive: ArchiveState,
) {
    let SpecsState { project, features } = specs;
    match phase {
        PhaseFilter::Design => {
            out.specs = Some(SpecsState { project, features });
            out.archive = Some(archive);
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
            });
        }
        // Execute / Verify both want project specs only (no features).
        // Diverge by adding a separate arm if behavior ever needs to split.
        PhaseFilter::Execute | PhaseFilter::Verify => {
            out.specs = Some(SpecsState {
                project,
                features: Vec::new(),
            });
        }
    }
}

/// Per ark-context C-20 second half: a `SpecRow` `f` is kept iff any
/// `r ∈ related` satisfies `normalize(r).ends_with(&normalize(f.path))`.
/// Both sides normalized: leading `./` and leading `.ark/` stripped.
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
        }
    }

    fn row(name: &str) -> SpecRow {
        SpecRow {
            name: name.to_string(),
            path: PathBuf::from(format!(".ark/specs/features/{name}/SPEC.md")),
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
            },
            None,
        );
        let pj = project(ctx, Scope::Phase(PhaseFilter::Verify));
        let s = pj.specs.unwrap();
        assert!(s.features.is_empty());
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
