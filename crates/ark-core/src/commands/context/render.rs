//! Render a [`ProjectedContext`] as human-readable text.
//!
//! Section names are locked: `## GIT STATUS`, `## CHECKOUT`, `## CURRENT
//! TASK`, `## ACTIVE TASKS`, `## SPECS`, `## SUBAGENTS`, `## ARCHIVE`,
//! `## RECORD`. Sections absent from the projection are omitted entirely.
//! Text mode carries no schema version. The `## SPECS` section renders both
//! project and feature rows in tree shape (indented by directory / feature
//! path); JSON consumers get the flat list plus the nested
//! `specs.features_tree` field.

use std::fmt;

use crate::commands::context::{
    model::{
        ArchiveState, ArtifactKind, CheckoutInfo, CheckoutRootKind, CurrentTask, GitState, SpecRow,
        SpecsState, SubagentSet, TasksState,
    },
    projection::{PhaseFilter, ProjectedContext, ScopeTag},
};

/// Display wrapper for text-mode context rendering.
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
            ScopeTag::Record => writeln!(f, "ark context (scope=record)")?,
        }
        writeln!(f, "project: {}", p.project_root.display())?;
        writeln!(f)?;

        write_git(f, &p.git)?;
        write_checkout(f, &p.checkout)?;

        if let Some(ct) = &p.current_task {
            write_current_task(f, ct)?;
        }

        if let Some(tasks) = &p.tasks {
            write_active_tasks(f, tasks)?;
        }

        if let Some(specs) = &p.specs {
            write_specs(f, specs)?;
        }

        if !p.subagents.is_empty() {
            write_subagents(f, &p.subagents)?;
        }

        if let Some(archive) = &p.archive {
            write_archive(f, archive)?;
        }

        if let Some(record) = &p.record {
            write_record(f, record)?;
        }

        Ok(())
    }
}

fn write_checkout(f: &mut fmt::Formatter<'_>, c: &CheckoutInfo) -> fmt::Result {
    writeln!(f, "## CHECKOUT")?;
    let kind = match c.root_kind {
        CheckoutRootKind::Main => "main",
        CheckoutRootKind::Worktree => "worktree",
    };
    writeln!(f, "kind: {kind}")?;
    writeln!(f, "branch: {}", c.branch)?;
    writeln!(
        f,
        "focus: {}",
        c.focus_slug.as_deref().unwrap_or("<unbound>")
    )?;
    writeln!(f)?;
    Ok(())
}

fn write_subagents(f: &mut fmt::Formatter<'_>, sets: &[SubagentSet]) -> fmt::Result {
    writeln!(f, "## SUBAGENTS")?;
    for set in sets {
        writeln!(f, "{}:", set.platform)?;
        if set.stems.is_empty() {
            writeln!(f, "  (none)")?;
        } else {
            for stem in &set.stems {
                writeln!(f, "  {stem}")?;
            }
        }
    }
    writeln!(f)?;
    Ok(())
}

fn write_record(
    f: &mut fmt::Formatter<'_>,
    r: &crate::commands::context::projection::RecordProjection,
) -> fmt::Result {
    writeln!(f, "## RECORD")?;
    writeln!(
        f,
        "identity: {}",
        r.identity.as_deref().unwrap_or("<unset>")
    )?;
    writeln!(
        f,
        "active journal: {}",
        r.active_journal_path.as_deref().unwrap_or("<none>")
    )?;
    writeln!(f, "sessions: {}", r.session_count)?;
    writeln!(f, "journal_max_lines: {}", r.journal_max_lines)?;
    writeln!(f, "branch: {}", r.branch.as_deref().unwrap_or("<unknown>"))?;
    Ok(())
}

fn phase_label(p: PhaseFilter) -> &'static str {
    match p {
        PhaseFilter::Design => "design",
        PhaseFilter::Plan => "plan",
        PhaseFilter::Review => "review",
        PhaseFilter::Execute => "execute",
        PhaseFilter::Verify => "verify",
        PhaseFilter::Commit => "commit",
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
    // The flat `PLAN.md` / `REVIEW.md` classify as iteration 0 and render
    // bare; legacy `NN_PLAN.md` archives (iteration ≥ 1) keep the number.
    match k {
        ArtifactKind::Prd => "PRD".to_string(),
        ArtifactKind::Plan { iteration: 0 } => "PLAN".to_string(),
        ArtifactKind::Plan { iteration } => format!("PLAN {iteration:02}"),
        ArtifactKind::Review { iteration: 0 } => "REVIEW".to_string(),
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
            writeln!(f, "  {} [{:?} {:?}] {}", t.slug, t.tier, t.phase, t.title)?;
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
        write_spec_rows_tree(f, &specs.project, "project")?;
    }
    if !specs.features.is_empty() {
        writeln!(f, "features:")?;
        write_spec_rows_tree(f, &specs.features, "features")?;
    }
    writeln!(f)?;
    Ok(())
}

/// Renders `rows` as an indented tree, grouped by directory prefix.
///
/// Each row's path is split on `/` after stripping the leading
/// `.ark/specs/<group>/` prefix. For features the trailing `SPEC.md`
/// filename is dropped so the meaningful leaf is the last directory
/// (e.g. `xemu/csr/SPEC.md` → leaf `csr` under branch `xemu/`); project
/// rows keep the filename as the leaf (e.g. `rust/COMMENTS.md` → leaf
/// `COMMENTS.md` under branch `rust/`). Intermediate segments become
/// indented branch lines (`<seg>/`); leaves render as
/// `<segment> — <scope>`.
fn write_spec_rows_tree(f: &mut fmt::Formatter<'_>, rows: &[SpecRow], group: &str) -> fmt::Result {
    let prefix = format!(".ark/specs/{group}/");
    let mut last_path: Vec<String> = Vec::new();
    for row in rows {
        let path_str = row.path.to_string_lossy();
        let relative = path_str.strip_prefix(&prefix).unwrap_or(&path_str);
        let mut segments: Vec<&str> = relative.split('/').collect();
        // Features always live at `<...>/SPEC.md`; treat the trailing
        // SPEC.md as a marker, not as the leaf label.
        if group == "features"
            && segments.last().map(|s| s.eq_ignore_ascii_case("SPEC.md")) == Some(true)
            && segments.len() >= 2
        {
            segments.pop();
        }
        let (leaf, dirs) = match segments.split_last() {
            Some((last, head)) => (*last, head.to_vec()),
            None => {
                writeln!(f, "  {} — {}", row.name, row.scope)?;
                continue;
            }
        };
        // Emit branch headers for each new directory segment relative to
        // the previously-rendered row's directories. Shared prefixes don't
        // re-print.
        let mut common = 0;
        while common < dirs.len()
            && common < last_path.len()
            && dirs[common] == last_path[common].as_str()
        {
            common += 1;
        }
        for (depth, seg) in dirs.iter().enumerate().skip(common) {
            let indent = "  ".repeat(depth + 1);
            writeln!(f, "{indent}{seg}/")?;
        }
        let leaf_indent = "  ".repeat(dirs.len() + 1);
        writeln!(f, "{leaf_indent}{leaf} — {}", row.scope)?;
        last_path = dirs.into_iter().map(str::to_string).collect();
    }
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
            checkout: CheckoutInfo::default(),
            tasks: Some(TasksState::default()),
            current_task: None,
            specs: Some(SpecsState::default()),
            archive: Some(ArchiveState::default()),
            record: None,
            subagents: Vec::new(),
            truncated: None,
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
        // Text mode carries no schema version.
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

    /// `## CHECKOUT` always renders; `## SUBAGENTS` renders only when its
    /// backing field is non-empty. There is no separate `## FEATURES TREE`
    /// section — features render in tree shape under `## SPECS` itself.
    #[test]
    fn checkout_always_renders_subagents_gated_no_features_tree_section() {
        let p = empty_projection(ScopeTag::Session);
        let out = format!("{}", TextSummary(&p));
        assert!(out.contains("## CHECKOUT"), "missing CHECKOUT in:\n{out}");
        assert!(!out.contains("## SUBAGENTS"));
        assert!(!out.contains("## FEATURES TREE"), "stale section: {out}");
    }

    /// `## SUBAGENTS` renders when populated; `## SPECS` renders project
    /// and feature rows in tree shape (indented by directory / feature
    /// path), with deeper paths sharing common prefixes.
    #[test]
    fn populated_sections_render_specs_as_tree() {
        use std::path::PathBuf;
        let mut p = empty_projection(ScopeTag::Session);
        p.subagents = vec![SubagentSet {
            platform: "claude".to_string(),
            stems: vec!["ark-reviewer".to_string()],
        }];
        if let Some(specs) = p.specs.as_mut() {
            specs.project = vec![
                SpecRow {
                    name: "LAYOUT".to_string(),
                    path: PathBuf::from(".ark/specs/project/LAYOUT.md"),
                    feature_path: Vec::new(),
                    scope: "Convention SPEC layout".to_string(),
                    promoted: None,
                },
                SpecRow {
                    name: "COMMENTS".to_string(),
                    path: PathBuf::from(".ark/specs/project/rust/COMMENTS.md"),
                    feature_path: Vec::new(),
                    scope: "Rust comment conventions".to_string(),
                    promoted: None,
                },
                SpecRow {
                    name: "STYLE".to_string(),
                    path: PathBuf::from(".ark/specs/project/rust/STYLE.md"),
                    feature_path: Vec::new(),
                    scope: "Rust style conventions".to_string(),
                    promoted: None,
                },
            ];
            specs.features = vec![
                SpecRow {
                    name: "ark-context".to_string(),
                    path: PathBuf::from(".ark/specs/features/ark-context/SPEC.md"),
                    feature_path: vec!["ark-context".to_string()],
                    scope: "Add ark context command".to_string(),
                    promoted: None,
                },
                SpecRow {
                    name: "csr".to_string(),
                    path: PathBuf::from(".ark/specs/features/xemu/csr/SPEC.md"),
                    feature_path: vec!["xemu".to_string(), "csr".to_string()],
                    scope: "xemu csr".to_string(),
                    promoted: None,
                },
            ];
        }
        let out = format!("{}", TextSummary(&p));

        assert!(out.contains("## SUBAGENTS"));
        assert!(out.contains("claude:"));
        assert!(out.contains("ark-reviewer"));

        // Project SPECs render the path-derived leaf (filename incl. .md)
        // under `rust/` branch for nested rows.
        assert!(out.contains("project:"));
        assert!(out.contains("  LAYOUT.md — Convention SPEC layout"));
        assert!(out.contains("  rust/"));
        assert!(out.contains("    COMMENTS.md — Rust comment conventions"));
        assert!(out.contains("    STYLE.md — Rust style conventions"));

        // Feature SPECs render the path-derived leaf (last directory; the
        // trailing SPEC.md filename is dropped per the features-tree rule).
        assert!(out.contains("features:"));
        assert!(out.contains("  ark-context — Add ark context command"));
        assert!(out.contains("  xemu/"));
        assert!(out.contains("    csr — xemu csr"));
    }
}
