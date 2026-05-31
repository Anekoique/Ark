//! Builds a fresh [`Context`].
//!
//! Reads git, `.ark/tasks/`, and `.ark/specs/`. Pure I/O; no projection or
//! rendering.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::{
    commands::{
        agent::state::TaskToml,
        context::{
            checkout::detect_checkout,
            model::{
                ARCHIVE_CAP, ArchiveState, ArchivedTask, ArtifactKind, ArtifactSummary, Context,
                CurrentTask, DIRTY_FILES_CAP, GatherWarning, GitCommit, GitState,
                RECENT_COMMITS_CAP, SCHEMA_VERSION, SpecRow, SpecsState, TaskSummary, TasksState,
            },
            related_specs,
            spec_tree::build_features_tree,
            subagents::enumerate_subagents,
        },
    },
    error::Result,
    io::{PathExt, git, read_managed_block},
    layout::{FEATURES_MARKER, Layout},
    state::load_state,
};

/// Snapshot the project state into a [`Context`].
pub fn gather_context(layout: &Layout) -> Result<Context> {
    let project_root = layout.root().to_path_buf();
    let git = gather_git(&project_root)?;
    let tasks = gather_tasks(layout)?;
    let archive = gather_archive(layout)?;
    let specs = gather_specs(layout)?;
    let current_task = gather_current_task(layout, &tasks)?;
    let checkout = detect_checkout(layout, &git.branch);
    let subagents = enumerate_subagents(layout);
    Ok(Context {
        schema: SCHEMA_VERSION,
        generated_at: Utc::now(),
        project_root,
        git,
        tasks,
        specs,
        archive,
        current_task,
        checkout,
        subagents,
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
        if name == "archive" || name == ".current" {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let Ok(toml) = TaskToml::load(&path) else {
            // Corrupt/missing task.toml in a sub-directory should not crash the
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
    // Layout: archive/<YYYY-MM>/<tier>/<slug>/. Walk month dirs, then tier
    // dirs, then task dirs. Collect every task with task.toml, sort by
    // archived_at desc, take ARCHIVE_CAP.
    let mut all: Vec<ArchivedTask> = Vec::new();
    for month_entry in archive_root.list_dir()? {
        let month_entry = month_entry.map_err(|e| crate::error::Error::io(&archive_root, e))?;
        let month_path = month_entry.path();
        if !month_path.is_dir() {
            continue;
        }
        for tier_entry in month_path.list_dir()? {
            let tier_entry = tier_entry.map_err(|e| crate::error::Error::io(&month_path, e))?;
            let tier_path = tier_entry.path();
            if !tier_path.is_dir() {
                continue;
            }
            for task_entry in tier_path.list_dir()? {
                let task_entry = task_entry.map_err(|e| crate::error::Error::io(&tier_path, e))?;
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
    }
    all.sort_by_key(|a| std::cmp::Reverse(a.archived_at));
    all.truncate(ARCHIVE_CAP);
    Ok(ArchiveState { recent: all })
}

fn gather_specs(layout: &Layout) -> Result<SpecsState> {
    let project = parse_project_index(layout)?;
    let (features, features_warnings) = parse_features_index(layout)?;
    let features_tree = build_features_tree(&features);
    Ok(SpecsState {
        project,
        features,
        features_warnings,
        features_tree,
    })
}

/// Parses `.ark/specs/project/INDEX.md`.
///
/// Locates the `## Index` heading and returns the rows of the first GFM table.
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
                feature_path: Vec::new(),
                scope: cells.get(1).cloned().unwrap_or_default(),
                promoted: None,
            })
        })
        .collect())
}

/// Parses `.ark/specs/features/` recursively, descending into subtree
/// `INDEX.md` files via the managed block's row classification.
///
/// At each level, leaf rows (bare or `<seg>/SPEC.md` first cell) yield one
/// [`SpecRow`]; branch rows (`<seg>/INDEX.md` first cell) trigger a
/// recursive descent into the named subdirectory. Single-segment paths
/// produce the legacy flat shape bit-for-bit. Symlinks are not followed;
/// depth is bounded.
///
/// Returns `(rows, warnings)` — drift between the on-disk filesystem and
/// the INDEX rows is surfaced as [`GatherWarning`] values instead of being
/// silently dropped (per the iteration-02 design pivot).
fn parse_features_index(layout: &Layout) -> Result<(Vec<SpecRow>, Vec<GatherWarning>)> {
    let mut rows = Vec::new();
    let mut warnings = Vec::new();
    let mut visited_children: Vec<(Vec<String>, std::collections::HashSet<String>)> = Vec::new();
    walk_features_index(
        layout,
        &mut Vec::new(),
        &mut rows,
        &mut warnings,
        &mut visited_children,
        0,
    )?;
    detect_orphans(layout, &visited_children, &mut warnings)?;
    Ok((rows, warnings))
}

/// Walks every visited subtree's children and emits a warning for any
/// on-disk `SPEC.md` or subtree directory that the parent INDEX did not
/// row. Symlinks are not followed.
fn detect_orphans(
    layout: &Layout,
    visited: &[(Vec<String>, std::collections::HashSet<String>)],
    warnings: &mut Vec<GatherWarning>,
) -> Result<()> {
    for (prefix, visited_set) in visited {
        let mut dir = layout.specs_features_dir();
        for seg in prefix {
            dir.push(seg);
        }
        let entries = match dir.read_dir() {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "INDEX.md" || name.starts_with('.') {
                continue;
            }
            if visited_set.contains(&name) {
                continue;
            }
            // Not rowed in the parent INDEX — classify by what's on disk.
            let path = entry.path();
            if file_type.is_dir() {
                let leaf = path.join("SPEC.md");
                if leaf.exists() {
                    warnings.push(GatherWarning::OrphanLeaf {
                        path: project_relative(layout, &leaf),
                    });
                } else {
                    warnings.push(GatherWarning::OrphanSubtree {
                        path: project_relative(layout, &path),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Strips the project root from `path`, producing the same project-root-
/// relative `PathBuf` shape used elsewhere in [`SpecRow`].
fn project_relative(layout: &Layout, path: &Path) -> PathBuf {
    path.strip_prefix(layout.root())
        .map(PathBuf::from)
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Maximum recursion depth for the features INDEX walk; guards against
/// pathological / cyclic layouts. Eight levels is the documented bound.
const FEATURES_WALK_MAX_DEPTH: usize = 8;

/// Recursive walker for the features INDEX tree.
///
/// `prefix` carries the segments accumulated by parent recursive calls.
/// `out` is appended in row order; at each level, rows are emitted in the
/// order they appear in the parent `INDEX.md`'s managed block.
/// `warnings` collects drift signals (`MissingChild` for stale rows). The
/// child set the row walk visited at each level is recorded in `visited`
/// so the later orphan-detection pass can find on-disk entries the rows
/// did not name.
fn walk_features_index(
    layout: &Layout,
    prefix: &mut Vec<String>,
    out: &mut Vec<SpecRow>,
    warnings: &mut Vec<GatherWarning>,
    visited: &mut Vec<(Vec<String>, std::collections::HashSet<String>)>,
    depth: usize,
) -> Result<()> {
    if depth >= FEATURES_WALK_MAX_DEPTH {
        return Ok(());
    }
    let mut index_path = layout.specs_features_dir();
    for seg in prefix.iter() {
        index_path.push(seg);
    }
    index_path.push("INDEX.md");

    let Some(body) = read_managed_block(&index_path, FEATURES_MARKER)? else {
        return Ok(());
    };

    let mut local_visited: std::collections::HashSet<String> = std::collections::HashSet::new();

    for cells in gfm_table_rows(&body) {
        if cells.len() < 3 {
            continue;
        }
        let raw = cells[0].trim().trim_matches('`').trim();
        if raw.is_empty() {
            continue;
        }
        let scope = cells[1].clone();
        let promoted = Some(cells[2].clone());

        // Classify by target suffix; row text identifies leaf vs branch.
        // Accepted shapes: `<seg>`, `<seg>/SPEC.md` (leaf); `<seg>/INDEX.md` (branch).
        if let Some(seg) = raw.strip_suffix("/INDEX.md") {
            let seg_owned = seg.to_string();
            local_visited.insert(seg_owned.clone());
            // Verify the child INDEX exists on disk; if not, emit a warning
            // and skip the recursion (no rows from this row).
            let mut child_path = layout.specs_features_dir();
            for s in prefix.iter() {
                child_path.push(s);
            }
            child_path.push(&seg_owned);
            child_path.push("INDEX.md");
            if !child_path.exists() {
                warnings.push(GatherWarning::MissingChild {
                    row: raw.to_string(),
                    expected_path: project_relative(layout, &child_path),
                });
                continue;
            }
            prefix.push(seg_owned);
            walk_features_index(layout, prefix, out, warnings, visited, depth + 1)?;
            prefix.pop();
        } else {
            let seg = raw.strip_suffix("/SPEC.md").unwrap_or(raw).to_string();
            local_visited.insert(seg.clone());
            // Verify the leaf SPEC exists on disk; if not, emit a warning
            // and skip the row.
            let mut feature_path = prefix.clone();
            feature_path.push(seg.clone());
            let mut leaf_abs = layout.specs_features_dir();
            for s in &feature_path {
                leaf_abs.push(s);
            }
            leaf_abs.push("SPEC.md");
            if !leaf_abs.exists() {
                warnings.push(GatherWarning::MissingChild {
                    row: raw.to_string(),
                    expected_path: project_relative(layout, &leaf_abs),
                });
                continue;
            }
            let mut path = PathBuf::from(".ark/specs/features");
            for s in &feature_path {
                path.push(s);
            }
            path.push("SPEC.md");
            out.push(SpecRow {
                name: seg,
                path,
                feature_path,
                scope,
                promoted,
            });
        }
    }
    visited.push((prefix.clone(), local_visited));
    Ok(())
}

/// Locates the byte offset of the first index table line.
///
/// Returns `None` if either the `## Index` heading or a table line is absent.
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

/// Iterates data rows of a GFM-style pipe table.
///
/// Stops at the first non-pipe line and skips header rows (first cell
/// `Spec`/`Feature` or all-dashes separators), blank rows, and placeholder
/// rows whose cells are wrapped in `{...}`.
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

/// Returns `true` if every cell looks like a placeholder marker.
///
/// Backticks and surrounding whitespace are stripped before the check.
fn is_placeholder_row(cells: &[String]) -> bool {
    cells.iter().all(|c| {
        let stripped = c.trim().trim_matches('`').trim();
        stripped.starts_with('{') && stripped.ends_with('}')
    })
}

/// Strips surrounding backticks and trailing `/SPEC.md` from a cell value.
/// Returns `(name, path)` — the name is the slug-ish display value and path
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
    // Per-checkout focus is the only source of truth. A checkout that has
    // never had `task new` / `task resume` reports `None`.
    let state = load_state(layout)?;
    let Some(slug) = state.focus.clone() else {
        return Ok(None);
    };
    let task_dir = layout.task_dir(&slug);
    if !task_dir.is_dir() {
        return Ok(None);
    }
    let _ = TaskToml::load(&task_dir)?;
    let Some(summary) = tasks.active.iter().find(|t| t.slug == slug).cloned() else {
        return Ok(None);
    };

    let artifacts = list_artifacts(layout, &slug)?;
    let related = match task_dir.join("PRD.md").read_text_optional()? {
        Some(prd) => related_specs::extract(&prd),
        None => Vec::new(),
    };

    Ok(Some(CurrentTask {
        slug,
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
    if filename == "PLAN.md" {
        return Some(ArtifactKind::Plan { iteration: 0 });
    }
    if let Some(n) = parse_iteration_artifact(filename, "_PLAN.md") {
        return Some(ArtifactKind::Plan { iteration: n });
    }
    if let Some(n) = parse_iteration_artifact(filename, "_REVIEW.md") {
        return Some(ArtifactKind::Review { iteration: n });
    }
    None
}

/// Parses `^(\d{2})<suffix>$` filenames; e.g. `00_PLAN.md` → `Some(0)`.
fn parse_iteration_artifact(filename: &str, suffix: &str) -> Option<u32> {
    let nn = filename.strip_suffix(suffix)?;
    (nn.len() == 2).then(|| nn.parse::<u32>().ok())?
}

/// Returns a tuple sort key `(kind-bucket, iteration)` for artifact ordering.
///
/// PRD < Plan < Review < Verify < TaskToml; within Plan and Review, ascending
/// by iteration.
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

    /// Registers `slug` as this checkout's focus. Mirrors what `task new` does.
    fn register_focus(layout: &Layout, slug: &str) {
        use crate::state::state_mutate;
        state_mutate(layout, |state| {
            state.focus = Some(slug.to_string());
            Ok(())
        })
        .unwrap();
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
        register_focus(&layout, "feat-x");
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
    fn gather_current_returns_none_when_no_session_focus() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        let ctx = gather_context(&layout).unwrap();
        assert!(ctx.current_task.is_none());
    }

    #[test]
    fn gather_current_returns_none_when_focus_slug_dangling() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        // Plant a session whose focus does not match any task dir; reconcile
        // will drop it on load, yielding `None`.
        register_focus(&layout, "nope");
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
        // Materialize on-disk leaves so the INDEX-strict walker accepts them.
        let features_dir = layout.specs_features_dir();
        features_dir.join("foo").ensure_dir().unwrap();
        features_dir.join("foo/SPEC.md").write_bytes(b"x").unwrap();
        features_dir.join("bar-baz").ensure_dir().unwrap();
        features_dir
            .join("bar-baz/SPEC.md")
            .write_bytes(b"x")
            .unwrap();

        let ctx = gather_context(&layout).unwrap();
        assert_eq!(ctx.specs.features.len(), 2);
        assert_eq!(ctx.specs.features[0].name, "foo");
        assert_eq!(ctx.specs.features[1].name, "bar-baz");
        assert!(ctx.specs.features[0].promoted.is_some());
        // Single-segment feature_path for legacy flat leaves.
        assert_eq!(ctx.specs.features[0].feature_path, vec!["foo".to_string()]);
        assert_eq!(
            ctx.specs.features[0].path,
            std::path::PathBuf::from(".ark/specs/features/foo/SPEC.md"),
        );
        // No drift: every INDEX row matches an on-disk leaf, no orphans.
        assert!(ctx.specs.features_warnings.is_empty());
    }

    /// Verifies the recursive walk: root INDEX has a branch row pointing at
    /// `xemu/INDEX.md`; the subtree INDEX lists a leaf `csr/SPEC.md`. Both
    /// the nested `feature_path` and `path` come out correctly.
    #[test]
    fn gather_features_index_recurses_into_subtree() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        layout
            .specs_features_index()
            .write_bytes(
                b"<!-- ARK:FEATURES:START -->\n\
                  | Feature | Scope | Promoted |\n\
                  |---------|-------|----------|\n\
                  | `klib` | shared lib | 2026-05-01 |\n\
                  | `xemu/INDEX.md` | xemu subtree | 2026-05-10 |\n\
                  <!-- ARK:FEATURES:END -->\n",
            )
            .unwrap();
        let subtree = layout.specs_features_dir().join("xemu").join("INDEX.md");
        subtree.parent().unwrap().ensure_dir().unwrap();
        subtree
            .write_bytes(
                b"<!-- ARK:FEATURES:START -->\n\
                  | Feature | Scope | Promoted |\n\
                  |---------|-------|----------|\n\
                  | `csr` | CSR file | 2026-05-11 |\n\
                  <!-- ARK:FEATURES:END -->\n",
            )
            .unwrap();
        // Materialize on-disk leaves.
        let features_dir = layout.specs_features_dir();
        features_dir.join("klib").ensure_dir().unwrap();
        features_dir.join("klib/SPEC.md").write_bytes(b"x").unwrap();
        features_dir.join("xemu/csr").ensure_dir().unwrap();
        features_dir
            .join("xemu/csr/SPEC.md")
            .write_bytes(b"x")
            .unwrap();

        let ctx = gather_context(&layout).unwrap();
        // Order: klib leaf first, then csr (from xemu subtree).
        assert_eq!(ctx.specs.features.len(), 2);
        assert_eq!(ctx.specs.features[0].name, "klib");
        assert_eq!(ctx.specs.features[0].feature_path, vec!["klib".to_string()]);
        assert_eq!(ctx.specs.features[1].name, "csr");
        assert_eq!(
            ctx.specs.features[1].feature_path,
            vec!["xemu".to_string(), "csr".to_string()],
        );
        assert_eq!(
            ctx.specs.features[1].path,
            std::path::PathBuf::from(".ark/specs/features/xemu/csr/SPEC.md"),
        );
        assert_eq!(ctx.specs.features[1].scope, "CSR file");
        assert!(ctx.specs.features_warnings.is_empty());
    }

    /// An INDEX row whose target is missing on disk surfaces as
    /// `GatherWarning::MissingChild`; no `SpecRow` is emitted.
    #[test]
    fn gather_emits_missing_child_warning_for_stale_row() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        layout
            .specs_features_index()
            .write_bytes(
                b"<!-- ARK:FEATURES:START -->\n\
                  | Feature | Scope | Promoted |\n\
                  |---------|-------|----------|\n\
                  | `ghost` | row points at missing SPEC | 2026-05-01 |\n\
                  <!-- ARK:FEATURES:END -->\n",
            )
            .unwrap();
        let ctx = gather_context(&layout).unwrap();
        assert!(
            ctx.specs.features.is_empty(),
            "stale row must not yield a row"
        );
        assert_eq!(ctx.specs.features_warnings.len(), 1);
        match &ctx.specs.features_warnings[0] {
            crate::commands::context::model::GatherWarning::MissingChild { row, .. } => {
                assert_eq!(row, "ghost");
            }
            other => panic!("expected MissingChild, got {other:?}"),
        }
    }

    /// An on-disk `features/<seg>/SPEC.md` not rowed in the parent INDEX
    /// surfaces as `GatherWarning::OrphanLeaf`.
    #[test]
    fn gather_emits_orphan_leaf_warning_for_unrowed_spec() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        layout
            .specs_features_index()
            .write_bytes(
                b"<!-- ARK:FEATURES:START -->\n\
                  | Feature | Scope | Promoted |\n\
                  |---------|-------|----------|\n\
                  <!-- ARK:FEATURES:END -->\n",
            )
            .unwrap();
        let features_dir = layout.specs_features_dir();
        features_dir.join("orphan").ensure_dir().unwrap();
        features_dir
            .join("orphan/SPEC.md")
            .write_bytes(b"x")
            .unwrap();
        let ctx = gather_context(&layout).unwrap();
        assert!(ctx.specs.features.is_empty());
        assert_eq!(ctx.specs.features_warnings.len(), 1);
        match &ctx.specs.features_warnings[0] {
            crate::commands::context::model::GatherWarning::OrphanLeaf { path } => {
                assert!(path.ends_with(".ark/specs/features/orphan/SPEC.md"));
            }
            other => panic!("expected OrphanLeaf, got {other:?}"),
        }
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

    /// Verifies that placeholder rows are excluded from context output.
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

    /// Walks the `<month>/<tier>/<slug>` archive layout and lists archived
    /// tasks newest-first, with the tier segment reflected in each path.
    #[test]
    fn gather_archive_reads_tier_layout() {
        let tmp = arked_tempdir();
        let layout = Layout::new(tmp.path());
        // Both tasks are tier=deep → stored under archive/2026-04/deep/.
        let tier_dir = layout.tasks_archive_dir().join("2026-04").join("deep");
        tier_dir.ensure_dir().unwrap();
        for (slug, archived) in [("a", "2026-04-01T00:00:00Z"), ("b", "2026-04-22T00:00:00Z")] {
            let task_dir = tier_dir.join(slug);
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
        // Path reflects the tier segment.
        assert!(
            ctx.archive.recent[0]
                .path
                .to_string_lossy()
                .contains("archive/2026-04/deep/b"),
            "got {}",
            ctx.archive.recent[0].path.display()
        );
    }
}
