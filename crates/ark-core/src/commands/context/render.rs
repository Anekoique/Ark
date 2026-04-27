//! Render a [`ProjectedContext`] as human-readable text.
//!
//! Section names are locked per ark-context G-12: `## GIT STATUS`,
//! `## CURRENT TASK`, `## ACTIVE TASKS`, `## SPECS`, `## ARCHIVE`. Sections
//! absent from the projection are omitted entirely. Text mode carries no
//! schema version (per C-10).

use std::fmt;

use crate::commands::context::{
    model::{ArchiveState, ArtifactKind, CurrentTask, GitState, SpecsState, TasksState},
    projection::{PhaseFilter, ProjectedContext, ScopeTag},
};

pub struct TextSummary<'a>(pub &'a ProjectedContext);

impl fmt::Display for TextSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.0;

        // Header line declaring the scope so a human reader can orient.
        match p.scope {
            ScopeTag::Session => writeln!(f, "ark context (scope=session)")?,
            ScopeTag::Phase { phase } => {
                writeln!(f, "ark context (scope=phase, for={})", phase_label(phase))?
            }
        }
        writeln!(f, "project: {}", p.project_root.display())?;
        writeln!(f)?;

        write_git(f, &p.git)?;

        if let Some(ct) = &p.current_task {
            write_current_task(f, ct)?;
        }

        if let Some(tasks) = &p.tasks {
            write_active_tasks(f, tasks)?;
        }

        if let Some(specs) = &p.specs {
            write_specs(f, specs)?;
        }

        if let Some(archive) = &p.archive {
            write_archive(f, archive)?;
        }

        Ok(())
    }
}

fn phase_label(p: PhaseFilter) -> &'static str {
    match p {
        PhaseFilter::Design => "design",
        PhaseFilter::Plan => "plan",
        PhaseFilter::Review => "review",
        PhaseFilter::Execute => "execute",
        PhaseFilter::Verify => "verify",
    }
}

fn write_git(f: &mut fmt::Formatter<'_>, g: &GitState) -> fmt::Result {
    writeln!(f, "## GIT STATUS")?;
    writeln!(f, "branch: {}", g.branch)?;
    if !g.head_short.is_empty() {
        writeln!(f, "head: {}", g.head_short)?;
    }
    if g.is_clean {
        writeln!(f, "working directory: clean")?;
    } else {
        writeln!(f, "working directory: {} change(s)", g.uncommitted_changes)?;
        for file in &g.dirty_files {
            writeln!(f, "  {file}")?;
        }
    }
    if !g.recent_commits.is_empty() {
        writeln!(f)?;
        writeln!(f, "recent commits:")?;
        for c in &g.recent_commits {
            writeln!(f, "  {} {}", c.hash, c.message)?;
        }
    }
    writeln!(f)?;
    Ok(())
}

fn write_current_task(f: &mut fmt::Formatter<'_>, ct: &CurrentTask) -> fmt::Result {
    writeln!(f, "## CURRENT TASK")?;
    writeln!(f, "slug: {}", ct.slug)?;
    writeln!(f, "title: {}", ct.summary.title)?;
    writeln!(f, "tier: {:?}", ct.summary.tier)?;
    writeln!(f, "phase: {:?}", ct.summary.phase)?;
    writeln!(f, "iteration: {}", ct.summary.iteration)?;
    writeln!(f, "path: {}", ct.summary.path.display())?;
    if !ct.artifacts.is_empty() {
        writeln!(f, "artifacts:")?;
        for a in &ct.artifacts {
            let kind = artifact_label(&a.kind);
            writeln!(f, "  [{kind}] {} ({} lines)", a.path.display(), a.lines)?;
        }
    }
    if !ct.related_specs.is_empty() {
        writeln!(f, "related specs:")?;
        for s in &ct.related_specs {
            writeln!(f, "  {s}")?;
        }
    }
    writeln!(f)?;
    Ok(())
}

fn artifact_label(k: &ArtifactKind) -> String {
    match k {
        ArtifactKind::Prd => "PRD".to_string(),
        ArtifactKind::Plan { iteration } => format!("PLAN {iteration:02}"),
        ArtifactKind::Review { iteration } => format!("REVIEW {iteration:02}"),
        ArtifactKind::Verify => "VERIFY".to_string(),
        ArtifactKind::TaskToml => "task.toml".to_string(),
    }
}

fn write_active_tasks(f: &mut fmt::Formatter<'_>, tasks: &TasksState) -> fmt::Result {
    writeln!(f, "## ACTIVE TASKS")?;
    if tasks.active.is_empty() {
        writeln!(f, "(none)")?;
    } else {
        for t in &tasks.active {
            writeln!(
                f,
                "  {} [{:?} {:?} iter={}] {}",
                t.slug, t.tier, t.phase, t.iteration, t.title
            )?;
        }
    }
    writeln!(f)?;
    Ok(())
}

fn write_specs(f: &mut fmt::Formatter<'_>, specs: &SpecsState) -> fmt::Result {
    writeln!(f, "## SPECS")?;
    if specs.project.is_empty() && specs.features.is_empty() {
        writeln!(f, "(no specs)")?;
        writeln!(f)?;
        return Ok(());
    }
    if !specs.project.is_empty() {
        writeln!(f, "project:")?;
        for r in &specs.project {
            writeln!(f, "  {} — {}", r.name, r.scope)?;
        }
    }
    if !specs.features.is_empty() {
        writeln!(f, "features:")?;
        for r in &specs.features {
            writeln!(f, "  {} — {}", r.name, r.scope)?;
        }
    }
    writeln!(f)?;
    Ok(())
}

fn write_archive(f: &mut fmt::Formatter<'_>, archive: &ArchiveState) -> fmt::Result {
    writeln!(f, "## ARCHIVE")?;
    if archive.recent.is_empty() {
        writeln!(f, "(none)")?;
    } else {
        for a in &archive.recent {
            writeln!(
                f,
                "  {} ({:?}) archived {}",
                a.slug,
                a.tier,
                a.archived_at.format("%Y-%m-%d")
            )?;
        }
    }
    writeln!(f)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use super::*;
    use crate::commands::context::{
        model::*,
        projection::{PhaseFilter, ScopeTag},
    };

    fn empty_projection(scope: ScopeTag) -> ProjectedContext {
        ProjectedContext {
            schema: SCHEMA_VERSION,
            scope,
            generated_at: Utc::now(),
            project_root: PathBuf::from("/tmp/proj"),
            git: GitState::default(),
            tasks: Some(TasksState::default()),
            current_task: None,
            specs: Some(SpecsState::default()),
            archive: Some(ArchiveState::default()),
        }
    }

    #[test]
    fn session_text_contains_locked_section_names() {
        let p = empty_projection(ScopeTag::Session);
        let out = format!("{}", TextSummary(&p));
        assert!(out.contains("## GIT STATUS"), "missing GIT STATUS\n{out}");
        assert!(out.contains("## ACTIVE TASKS"));
        assert!(out.contains("## SPECS"));
        assert!(out.contains("## ARCHIVE"));
    }

    #[test]
    fn text_does_not_contain_schema_version() {
        // C-10 / R-006: text mode carries no schema version.
        let p = empty_projection(ScopeTag::Session);
        let out = format!("{}", TextSummary(&p));
        assert!(!out.contains("schema=1"));
        assert!(!out.contains("\"schema\""));
    }

    #[test]
    fn phase_text_omits_absent_sections() {
        let mut p = empty_projection(ScopeTag::Phase {
            phase: PhaseFilter::Plan,
        });
        p.tasks = None;
        p.archive = None;
        let out = format!("{}", TextSummary(&p));
        assert!(!out.contains("## ACTIVE TASKS"));
        assert!(!out.contains("## ARCHIVE"));
        assert!(out.contains("## SPECS"));
    }
}
