//! Build a fresh [`Context`] by reading git, `.ark/tasks/`, and
//! `.ark/specs/`. Pure I/O — no projection, no rendering.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::{
    commands::{
        agent::state::TaskToml,
        context::{
            model::{
                ARCHIVE_CAP, ArchiveState, ArchivedTask, ArtifactKind, ArtifactSummary, Context,
                CurrentTask, DIRTY_FILES_CAP, GitCommit, GitState, RECENT_COMMITS_CAP,
                SCHEMA_VERSION, SpecRow, SpecsState, TaskSummary, TasksState,
            },
            related_specs,
        },
    },
    error::Result,
    io::{PathExt, git, read_managed_block},
    layout::{FEATURES_MARKER, Layout},
};

/// Snapshot the project state into a [`Context`].
pub fn gather_context(layout: &Layout) -> Result<Context> {
    let project_root = layout.root().to_path_buf();
    let git = gather_git(&project_root)?;
    let tasks = gather_tasks(layout)?;
    let archive = gather_archive(layout)?;
    let specs = gather_specs(layout)?;
    let current_task = gather_current_task(layout, &tasks)?;
    Ok(Context {
        schema: SCHEMA_VERSION,
        generated_at: Utc::now(),
        project_root,
        git,
        tasks,
        specs,
        archive,
        current_task,
    })
}

fn gather_git(project_root: &Path) -> Result<GitState> {
    let branch = git::run_git(&["rev-parse", "--abbrev-ref", "HEAD"], project_root)?;
    if !branch.is_success() {
        // Non-git directory (or git missing rev info). Soft-fail.
        return Ok(GitState::default());
    }
    let head_short = git::run_git(&["rev-parse", "--short", "HEAD"], project_root)
        .map(|o| {
            if o.is_success() {
                o.stdout.trim().to_string()
            } else {
                String::new()
            }
        })
        .unwrap_or_default();

    let status = git::run_git(&["status", "--porcelain"], project_root)?;
    let dirty_lines: Vec<&str> = status.stdout.lines().filter(|l| !l.is_empty()).collect();
    let total = u32::try_from(dirty_lines.len()).unwrap_or(u32::MAX);
    let dirty_files: Vec<String> = dirty_lines
        .iter()
        .take(DIRTY_FILES_CAP)
        .map(|l| l.get(3..).unwrap_or(*l).to_string())
        .collect();

    let log = git::run_git(
        &["log", "--oneline", "-n", &RECENT_COMMITS_CAP.to_string()],
        project_root,
    )?;
    let recent_commits = if log.is_success() {
        log.stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| match line.split_once(' ') {
                Some((hash, message)) => GitCommit {
                    hash: hash.to_string(),
                    message: message.to_string(),
                },
                None => GitCommit {
                    hash: line.to_string(),
                    message: String::new(),
                },
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(GitState {
        branch: branch.stdout.trim().to_string(),
        head_short,
        is_clean: total == 0,
        uncommitted_changes: total,
        dirty_files,
        recent_commits,
    })
}

fn gather_tasks(layout: &Layout) -> Result<TasksState> {
    let tasks_dir = layout.tasks_dir();
    if !tasks_dir.exists() {
        return Ok(TasksState::default());
    }
    let mut active: Vec<TaskSummary> = Vec::new();
    for entry in tasks_dir.list_dir()? {
        let entry = entry.map_err(|e| crate::error::Error::io(&tasks_dir, e))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Skip the archive subdirectory and the `.current` pointer file.
        if name == "archive" || name == ".current" {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let Ok(toml) = TaskToml::load(&path) else {
            // Corrupt/missing task.toml in a sub-directory shouldn't crash the
            // whole context; skip the offender. (`.current` will catch this
            // separately if it points here.)
            continue;
        };
        let relative = path
            .strip_prefix(layout.root())
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.clone());
        active.push(TaskSummary {
            slug: name.to_string(),
            title: toml.title.clone(),
            tier: toml.tier,
            phase: toml.phase,
            iteration: toml.iteration,
            path: relative,
            updated_at: toml.updated_at,
        });
    }
    active.sort_by_key(|t| std::cmp::Reverse(t.updated_at));
    Ok(TasksState { active })
}

fn gather_archive(layout: &Layout) -> Result<ArchiveState> {
    let archive_root = layout.tasks_archive_dir();
    if !archive_root.exists() {
        return Ok(ArchiveState::default());
    }
    // Layout: archive/YYYY-MM/<slug>/. Walk one level down to find month dirs,
    // then one more level to find tasks. Collect every task with task.toml,
    // sort by archived_at desc, take ARCHIVE_CAP.
    let mut all: Vec<ArchivedTask> = Vec::new();
    for month_entry in archive_root.list_dir()? {
        let month_entry = month_entry.map_err(|e| crate::error::Error::io(&archive_root, e))?;
        let month_path = month_entry.path();
        if !month_path.is_dir() {
            continue;
        }
        for task_entry in month_path.list_dir()? {
            let task_entry = task_entry.map_err(|e| crate::error::Error::io(&month_path, e))?;
            let task_path = task_entry.path();
            if !task_path.is_dir() {
                continue;
            }
            let Some(slug) = task_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(toml) = TaskToml::load(&task_path) else {
                continue;
            };
            let archived_at: DateTime<Utc> = toml.archived_at.unwrap_or(toml.updated_at);
            let relative = task_path
                .strip_prefix(layout.root())
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| task_path.clone());
            all.push(ArchivedTask {
                slug: slug.to_string(),
                title: toml.title.clone(),
                tier: toml.tier,
                archived_at,
                path: relative,
            });
        }
    }
    all.sort_by_key(|a| std::cmp::Reverse(a.archived_at));
    all.truncate(ARCHIVE_CAP);
    Ok(ArchiveState { recent: all })
}

fn gather_specs(layout: &Layout) -> Result<SpecsState> {
    let project = parse_project_index(layout)?;
    let features = parse_features_index(layout)?;
    Ok(SpecsState { project, features })
}

/// `.ark/specs/project/INDEX.md` parser per ark-context C-24:
/// locate `## Index` heading, then the first GFM table.
fn parse_project_index(layout: &Layout) -> Result<Vec<SpecRow>> {
    let path = layout.specs_project_index();
    let Some(text) = path.read_text_optional()? else {
        return Ok(Vec::new());
    };
    let Some(table_start) = find_index_table(&text) else {
        return Ok(Vec::new());
    };
    Ok(gfm_table_rows(&text[table_start..])
        .filter_map(|cells| {
            let (name, spec_path) = normalize_spec_cell(cells.first()?);
            (!name.is_empty()).then(|| SpecRow {
                name,
                path: PathBuf::from(spec_path),
                scope: cells.get(1).cloned().unwrap_or_default(),
                promoted: None,
            })
        })
        .collect())
}

/// `.ark/specs/features/INDEX.md` parser per ark-context C-24: parse the
/// `ARK:FEATURES` managed block as a 3-column GFM table.
fn parse_features_index(layout: &Layout) -> Result<Vec<SpecRow>> {
    let Some(body) = read_managed_block(layout.specs_features_index(), FEATURES_MARKER)? else {
        return Ok(Vec::new());
    };
    Ok(gfm_table_rows(&body)
        .filter_map(|cells| {
            if cells.len() < 3 {
                return None;
            }
            let (name, _) = normalize_spec_cell(&cells[0]);
            (!name.is_empty()).then(|| SpecRow {
                name: name.clone(),
                path: PathBuf::from(format!(".ark/specs/features/{name}/SPEC.md")),
                scope: cells[1].clone(),
                promoted: Some(cells[2].clone()),
            })
        })
        .collect())
}

/// Locate the byte offset of the first GFM table line after a `## Index`
/// heading. Returns `None` if either the heading or a table line is absent.
fn find_index_table(text: &str) -> Option<usize> {
    let mut idx = 0usize;
    let mut found_index_header = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_start();
        if trimmed.starts_with("## Index") {
            found_index_header = true;
        } else if found_index_header && trimmed.starts_with('|') {
            return Some(idx);
        }
        idx += line.len();
    }
    None
}

/// Iterate data rows of a GFM-style pipe table. Stops at the first non-pipe
/// line; skips header rows (first cell `Spec`/`Feature` or all-dashes
/// separators), blank rows, and placeholder rows whose cells are wrapped in
/// `{...}` (the shipped INDEX templates use `{e.g. rust/SPEC.md}` as a
/// fill-me-in marker).
fn gfm_table_rows(text: &str) -> impl Iterator<Item = Vec<String>> + '_ {
    text.lines()
        .map_while(|line| line.trim_start().starts_with('|').then_some(line))
        .filter_map(|line| {
            let cells: Vec<String> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect();
            let first = cells.first()?;
            let is_header_or_separator = first.is_empty()
                || first.eq_ignore_ascii_case("Spec")
                || first.eq_ignore_ascii_case("Feature")
                || first.chars().all(|c| c == '-');
            (!is_header_or_separator && !is_placeholder_row(&cells)).then_some(cells)
        })
}

/// `true` if every cell in the row looks like a `{e.g. …}` placeholder, i.e.
/// the unedited template marker. Backticks and surrounding whitespace are
/// stripped before the check.
fn is_placeholder_row(cells: &[String]) -> bool {
    cells.iter().all(|c| {
        let stripped = c.trim().trim_matches('`').trim();
        stripped.starts_with('{') && stripped.ends_with('}')
    })
}

/// Strip surrounding backticks and trailing `/SPEC.md` from a cell value.
/// Returns `(name, path)` — the name is the slug-ish display value, path
/// is the original cell text suitable for `PathBuf`.
fn normalize_spec_cell(raw: &str) -> (String, String) {
    let trimmed = raw.trim().trim_matches('`').trim();
    let name = trimmed
        .strip_suffix("/SPEC.md")
        .unwrap_or(trimmed)
        .rsplit('/')
        .next()
        .unwrap_or(trimmed)
        .to_string();
    (name, trimmed.to_string())
}

fn gather_current_task(layout: &Layout, tasks: &TasksState) -> Result<Option<CurrentTask>> {
    let pointer = layout.tasks_current();
    let Some(text) = pointer.read_text_optional()? else {
        return Ok(None);
    };
    let slug = text.trim();
    if slug.is_empty() {
        return Ok(None);
    }
    let task_dir = layout.task_dir(slug);
    if !task_dir.is_dir() {
        return Ok(None);
    }
    // task.toml corruption is propagated as Error::TaskTomlCorrupt.
    let _ = TaskToml::load(&task_dir)?;
    let summary = match tasks.active.iter().find(|t| t.slug == slug) {
        Some(s) => s.clone(),
        None => return Ok(None),
    };

    let artifacts = list_artifacts(layout, slug)?;
    let related = match task_dir.join("PRD.md").read_text_optional()? {
        Some(prd) => related_specs::extract(&prd),
        None => Vec::new(),
    };

    Ok(Some(CurrentTask {
        slug: slug.to_string(),
        summary,
        artifacts,
        related_specs: related,
    }))
}

fn list_artifacts(layout: &Layout, slug: &str) -> Result<Vec<ArtifactSummary>> {
    let task_dir = layout.task_dir(slug);
    let mut out: Vec<ArtifactSummary> = Vec::new();
    for entry in task_dir.list_dir()? {
        let entry = entry.map_err(|e| crate::error::Error::io(&task_dir, e))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let kind = match classify_artifact(name) {
            Some(k) => k,
            None => continue,
        };
        let lines = match path.read_text_optional()? {
            Some(text) => u32::try_from(text.lines().count()).unwrap_or(u32::MAX),
            None => 0,
        };
        let relative = path
            .strip_prefix(layout.root())
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.clone());
        out.push(ArtifactSummary {
            kind,
            path: relative,
            lines,
        });
    }
    out.sort_by_key(|a| artifact_sort_key(a.kind));
    Ok(out)
}

fn classify_artifact(filename: &str) -> Option<ArtifactKind> {
    if filename == "PRD.md" {
        return Some(ArtifactKind::Prd);
    }
    if filename == "VERIFY.md" {
        return Some(ArtifactKind::Verify);
    }
    if filename == "task.toml" {
        return Some(ArtifactKind::TaskToml);
    }
    if let Some(n) = parse_iteration_artifact(filename, "_PLAN.md") {
        return Some(ArtifactKind::Plan { iteration: n });
    }
    if let Some(n) = parse_iteration_artifact(filename, "_REVIEW.md") {
        return Some(ArtifactKind::Review { iteration: n });
    }
    None
}

/// Parse `^(\d{2})<suffix>$` filenames; e.g. `00_PLAN.md` → `Some(0)`.
fn parse_iteration_artifact(filename: &str, suffix: &str) -> Option<u32> {
    let nn = filename.strip_suffix(suffix)?;
    (nn.len() == 2).then(|| nn.parse::<u32>().ok())?
}

/// Tuple sort key: (kind-bucket, iteration). PRD < Plan < Review < Verify
/// < TaskToml; within Plan / Review, ascending iteration.
fn artifact_sort_key(kind: ArtifactKind) -> (u8, u32) {
    match kind {
        ArtifactKind::Prd => (0, 0),
        ArtifactKind::Plan { iteration } => (1, iteration),
        ArtifactKind::Review { iteration } => (2, iteration),
        ArtifactKind::Verify => (3, 0),
        ArtifactKind::TaskToml => (4, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::state::{Phase, Tier};

    fn arked_tempdir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join(".ark/tasks").ensure_dir().unwrap();
        tmp.path().join(".ark/tasks/archive").ensure_dir().unwrap();
        tmp.path().join(".ark/specs/project").ensure_dir().unwrap();
        tmp.path().join(".ark/specs/features").ensure_dir().unwrap();
        tmp
    }

    #[test]
    fn gather_on_empty_ark_returns_empty_state() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        let ctx = gather_context(&layout).unwrap();
        assert_eq!(ctx.schema, SCHEMA_VERSION);
        assert!(ctx.tasks.active.is_empty());
        assert!(ctx.archive.recent.is_empty());
        assert!(ctx.specs.project.is_empty());
        assert!(ctx.specs.features.is_empty());
        assert!(ctx.current_task.is_none());
    }

    #[test]
    fn gather_in_non_git_dir_yields_unknown_branch() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        let ctx = gather_context(&layout).unwrap();
        assert_eq!(ctx.git.branch, "unknown");
        assert!(ctx.git.is_clean);
        assert!(ctx.git.dirty_files.is_empty());
    }

    fn write_task(layout: &Layout, slug: &str, toml_body: &str) {
        let dir = layout.task_dir(slug);
        dir.ensure_dir().unwrap();
        dir.join("task.toml")
            .write_bytes(toml_body.as_bytes())
            .unwrap();
        dir.join("PRD.md").write_bytes(b"# stub PRD\n").unwrap();
    }

    fn deep_task_toml(slug: &str, phase: &str, iteration: u32) -> String {
        format!(
            "id = \"{slug}\"\ntitle = \"Test {slug}\"\ntier = \"deep\"\nphase = \
             \"{phase}\"\niteration = {iteration}\ncreated_at = \
             \"2026-04-24T00:00:00Z\"\nupdated_at = \"2026-04-24T00:00:00Z\"\n"
        )
    }

    #[test]
    fn gather_active_tasks_lists_seeded_task() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        write_task(&layout, "feat-x", &deep_task_toml("feat-x", "plan", 0));

        let ctx = gather_context(&layout).unwrap();
        assert_eq!(ctx.tasks.active.len(), 1);
        let t = &ctx.tasks.active[0];
        assert_eq!(t.slug, "feat-x");
        assert_eq!(t.tier, Tier::Deep);
        assert_eq!(t.phase, Phase::Plan);
        assert_eq!(t.iteration, 0);
    }

    #[test]
    fn gather_current_task_pointer_resolves() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        write_task(&layout, "feat-x", &deep_task_toml("feat-x", "plan", 0));
        layout.tasks_current().write_bytes(b"feat-x\n").unwrap();
        // Add NN_PLAN, NN_REVIEW, VERIFY artifacts.
        let task_dir = layout.task_dir("feat-x");
        task_dir
            .join("00_PLAN.md")
            .write_bytes(b"plan body\nline2\n")
            .unwrap();
        task_dir
            .join("00_REVIEW.md")
            .write_bytes(b"review body\n")
            .unwrap();

        let ctx = gather_context(&layout).unwrap();
        let current = ctx.current_task.expect("current task present");
        assert_eq!(current.slug, "feat-x");
        let kinds: Vec<ArtifactKind> = current.artifacts.iter().map(|a| a.kind).collect();
        assert!(kinds.contains(&ArtifactKind::Prd));
        assert!(kinds.contains(&ArtifactKind::Plan { iteration: 0 }));
        assert!(kinds.contains(&ArtifactKind::Review { iteration: 0 }));
        assert!(kinds.contains(&ArtifactKind::TaskToml));
    }

    #[test]
    fn gather_current_returns_none_when_pointer_missing() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        let ctx = gather_context(&layout).unwrap();
        assert!(ctx.current_task.is_none());
    }

    #[test]
    fn gather_current_returns_none_when_pointer_dangling() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        layout.tasks_current().write_bytes(b"nope\n").unwrap();
        let ctx = gather_context(&layout).unwrap();
        assert!(ctx.current_task.is_none());
    }

    #[test]
    fn gather_features_index_parses_managed_block() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        layout
            .specs_features_index()
            .write_bytes(
                b"<!-- ARK:FEATURES:START -->\n\
                  | Feature | Scope | Promoted |\n\
                  |---------|-------|----------|\n\
                  | `foo` | something foo does | 2026-04-24 |\n\
                  | `bar-baz` | bar baz scope | 2026-04-25 |\n\
                  <!-- ARK:FEATURES:END -->\n",
            )
            .unwrap();
        let ctx = gather_context(&layout).unwrap();
        assert_eq!(ctx.specs.features.len(), 2);
        assert_eq!(ctx.specs.features[0].name, "foo");
        assert_eq!(ctx.specs.features[1].name, "bar-baz");
        assert!(ctx.specs.features[0].promoted.is_some());
    }

    #[test]
    fn gather_project_index_parses_user_authored_table() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        layout
            .specs_project_index()
            .write_bytes(
                b"# Project Specs\n\n\
                  ## Index\n\n\
                  | Spec | Scope |\n\
                  |------|-------|\n\
                  | `rust/SPEC.md` | language style |\n\
                  | `tests/SPEC.md` | testing conventions |\n",
            )
            .unwrap();
        let ctx = gather_context(&layout).unwrap();
        assert_eq!(ctx.specs.project.len(), 2);
        assert_eq!(ctx.specs.project[0].name, "rust");
        assert_eq!(ctx.specs.project[1].scope, "testing conventions");
    }

    /// The shipped `templates/ark/specs/project/INDEX.md` ships with a
    /// `{e.g. rust/SPEC.md}` placeholder row. Hosts that haven't filled in
    /// any specs must not see that placeholder leak into context output.
    #[test]
    fn gather_project_index_skips_placeholder_template_row() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        layout
            .specs_project_index()
            .write_bytes(
                b"# Project Specs\n\n\
                  ## Index\n\n\
                  | Spec | Scope |\n\
                  |------|-------|\n\
                  | `{e.g. rust/SPEC.md}` | `{e.g. language style}` |\n",
            )
            .unwrap();
        let ctx = gather_context(&layout).unwrap();
        assert!(
            ctx.specs.project.is_empty(),
            "placeholder rows must not appear in specs.project; got: {:?}",
            ctx.specs.project,
        );
    }

    #[test]
    fn gather_archive_lists_recent_first() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        let month = layout.tasks_archive_dir().join("2026-04");
        month.ensure_dir().unwrap();
        for (slug, archived) in [("a", "2026-04-01T00:00:00Z"), ("b", "2026-04-22T00:00:00Z")] {
            let task_dir = month.join(slug);
            task_dir.ensure_dir().unwrap();
            task_dir
                .join("task.toml")
                .write_bytes(
                    format!(
                        "id = \"{slug}\"\ntitle = \"archived {slug}\"\ntier = \"deep\"\nphase = \
                         \"archived\"\niteration = 0\ncreated_at = \
                         \"2026-04-01T00:00:00Z\"\nupdated_at = \"{archived}\"\narchived_at = \
                         \"{archived}\"\n"
                    )
                    .as_bytes(),
                )
                .unwrap();
        }
        let ctx = gather_context(&layout).unwrap();
        assert_eq!(ctx.archive.recent.len(), 2);
        assert_eq!(ctx.archive.recent[0].slug, "b");
        assert_eq!(ctx.archive.recent[1].slug, "a");
    }
}
