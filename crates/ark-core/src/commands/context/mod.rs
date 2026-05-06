//! `ark context` prints a structured snapshot of workflow state.
//!
//! The command reads git and `.ark/` state without mutation.
//!
//! This is the top-level public command, paired with [`ark agent`] which
//! handles workflow mutation.

/// Gathers raw context state from disk and git.
pub mod gather;
/// Serializable context model types.
pub mod model;
/// Projects full context into scope-specific views.
pub mod projection;
/// Parses related feature SPEC references from task plans.
pub mod related_specs;
/// Renders context projections as text.
pub mod render;

use std::{fmt, path::PathBuf};

pub use gather::gather_context;
pub use model::{
    ArchiveState, ArchivedTask, ArtifactKind, ArtifactSummary, Context, CurrentTask, GitCommit,
    GitState, SCHEMA_VERSION, SpecRow, SpecsState, TaskSummary, TasksState,
};
pub use projection::{PhaseFilter, ProjectedContext, RecordProjection, Scope, ScopeTag, project};
use render::TextSummary;

use crate::{
    error::{Error, Result},
    layout::Layout,
};

/// Output format for `ark context`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// JSON output.
    Json,
    /// Human-readable text output.
    Text,
}

/// Options for producing a context snapshot.
#[derive(Debug, Clone)]
pub struct ContextOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Projection scope.
    pub scope: Scope,
    /// Output format.
    pub format: Format,
}

impl ContextOptions {
    /// Creates context options for `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            scope: Scope::Session,
            format: Format::Text,
        }
    }

    /// Sets the projection scope.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Sets the output format.
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }
}

/// Implements [`fmt::Display`]; the CLI calls `render(summary)` once.
///
/// JSON mode pre-serializes to a `String` (with trailing newline) so
/// `Display` is a single byte-write. Text mode formats on demand.
/// `Text` boxes the projection to keep the enum's stack size small.
#[derive(Debug)]
pub enum ContextSummary {
    /// Pre-rendered JSON output.
    Json(String),
    /// Text-rendered projected context.
    Text(Box<ProjectedContext>),
}

impl fmt::Display for ContextSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(bytes) => f.write_str(bytes),
            Self::Text(p) => write!(f, "{}", TextSummary(p)),
        }
    }
}

/// Entry point. Reads project state, projects per scope, returns a renderer.
///
/// `--scope session --format json` is wrapped in Claude Code's
/// `SessionStart` hook envelope (`{hookSpecificOutput: {hookEventName,
/// additionalContext}}`) so the SessionStart hook's stdout is recognized
/// and injected as additional context. Every other `(scope, format)`
/// combination returns raw output.
pub fn context(opts: ContextOptions) -> Result<ContextSummary> {
    let layout = Layout::new(&opts.project_root);
    let ark_dir = layout.ark_dir();
    if !ark_dir.try_exists().map_err(|e| Error::io(&ark_dir, e))? {
        return Err(Error::NotLoaded {
            path: opts.project_root,
        });
    }
    let ctx = gather_context(&layout)?;
    let mut projected = project(ctx, opts.scope);
    if matches!(opts.scope, Scope::Record) {
        projected.record = Some(gather_record_projection(&layout));
    }
    match opts.format {
        Format::Json => {
            let body = if matches!(opts.scope, Scope::Session) {
                let summary = summary_one_line(&projected);
                let inner = stringify_under_cap(&mut projected, ADDITIONAL_CONTEXT_CAP);
                wrap_session_start_envelope(&inner, &summary)
            } else {
                serde_json::to_string_pretty(&projected).expect("ProjectedContext serializes")
            };
            Ok(ContextSummary::Json(format!("{body}\n")))
        }
        Format::Text => Ok(ContextSummary::Text(Box::new(projected))),
    }
}

/// Maximum byte length of the stringified `additionalContext` payload.
///
/// Claude Code documents a 10,000-character cap on injected context;
/// staying under 9,500 leaves headroom for the envelope's outer keys and
/// avoids host-side mid-payload truncation.
const ADDITIONAL_CONTEXT_CAP: usize = 9_500;

/// Gathers the workspace record projection (developer + active journal +
/// config + branch).
///
/// All fields are best-effort: missing identity / no entries / git
/// unavailable produce `None` rather than erroring, so the slash command's
/// renderer can still produce a useful draft.
fn gather_record_projection(layout: &Layout) -> RecordProjection {
    use crate::commands::agent::workspace::{
        WorkspaceConfig, identity::ResolveOptions, identity_resolve,
    };

    let cfg = WorkspaceConfig::load_or_default(layout).unwrap_or_default();
    let identity = identity_resolve(ResolveOptions::new(layout.root()))
        .ok()
        .map(|id| id.name().to_string());

    let (active_journal_path, session_count) = match identity.as_deref() {
        Some(name) => {
            let dev_dir = layout.workspace_developer_dir(name);
            scan_developer_dir(&dev_dir, layout.root())
        }
        None => (None, 0),
    };

    let branch = crate::io::git::run_git(&["rev-parse", "--abbrev-ref", "HEAD"], layout.root())
        .ok()
        .filter(|out| out.is_success())
        .map(|out| out.stdout.trim().to_string());

    RecordProjection {
        identity,
        active_journal_path,
        journal_max_lines: cfg.journal_max_lines(),
        session_count,
        branch,
    }
}

/// Returns `(Option<journal-relpath>, session_count)` for `dev_dir`.
fn scan_developer_dir(
    dev_dir: &std::path::Path,
    project_root: &std::path::Path,
) -> (Option<String>, u32) {
    let entries = match std::fs::read_dir(dev_dir) {
        Ok(it) => it,
        Err(_) => return (None, 0),
    };
    let mut max_n: Option<(u32, std::path::PathBuf)> = None;
    let mut count: u32 = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_prefix("journal-") else {
            continue;
        };
        let Some(stem) = rest.strip_suffix(".md") else {
            continue;
        };
        let Ok(n) = stem.parse::<u32>() else {
            continue;
        };
        let path = entry.path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            count += text
                .lines()
                .filter(|l| l.trim_start().starts_with("## Session "))
                .count() as u32;
        }
        match &max_n {
            Some((m, _)) if n <= *m => {}
            _ => max_n = Some((n, path)),
        }
    }
    let active = max_n.map(|(_, p)| {
        p.strip_prefix(project_root)
            .map(|r| r.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/"))
            .unwrap_or_else(|_| p.to_string_lossy().into_owned())
    });
    (active, count)
}

/// Wraps a JSON payload in Claude Code's SessionStart envelope.
///
/// The payload is embedded as a stringified value of `additionalContext`
/// because the hook contract requires that field to be a string.
/// `suppressOutput: true` keeps the raw stdout out of transcript mode on
/// hosts that honor it; `systemMessage` carries a single user-visible line
/// summarizing what was loaded. Both fields are documented siblings of
/// `hookSpecificOutput` in Claude Code's hook contract and are accepted by
/// Codex CLI's matching shape.
fn wrap_session_start_envelope(payload: &str, system_message: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": payload,
        },
        "suppressOutput": true,
        "systemMessage": system_message,
    }))
    .expect("envelope serializes")
}

/// Renders a `Serialize`-derived enum (e.g. `Tier`, `Phase`) as its
/// lowercase string form by routing through `serde_json::to_value`.
///
/// Falls back to `?` when serde_json yields a non-string Value, which is
/// not expected for the unit-variant enums in `model.rs` but keeps the
/// summary line printable rather than panicking on a future refactor.
fn json_str<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        _ => "?".to_string(),
    }
}

/// Builds a one-line user-visible summary of `projected`.
///
/// Mirrors the most actionable bits of the session projection: branch,
/// current task slug + tier + phase, active-task count, and feature-spec
/// count. Omits noisy fields (paths, timestamps) — this line is what the
/// host renders to the user, not what the model consumes.
fn summary_one_line(projected: &ProjectedContext) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(4);
    parts.push(format!("branch={}", projected.git.branch));
    if let Some(current) = &projected.current_task {
        parts.push(format!(
            "current={} ({}/{})",
            current.summary.slug,
            json_str(&current.summary.tier),
            json_str(&current.summary.phase),
        ));
    }
    if let Some(tasks) = &projected.tasks {
        parts.push(format!("{} active", tasks.active.len()));
    }
    if let Some(specs) = &projected.specs {
        parts.push(format!("{} specs", specs.features.len()));
    }
    format!("Ark: {}", parts.join(" \u{b7} "))
}

/// Serializes `projected` as JSON, dropping low-value sections in priority
/// order until the result fits within `cap` bytes.
///
/// Drop order: `archive` first (least actionable for an open session), then
/// `tasks.active` past the first 5 entries (newest survive). When a drop
/// happens, sets `truncated: Some(true)` so the model knows the snapshot is
/// partial. Returns the serialized string regardless — callers always emit
/// something.
fn stringify_under_cap(projected: &mut ProjectedContext, cap: usize) -> String {
    let initial = serde_json::to_string_pretty(projected).expect("ProjectedContext serializes");
    if initial.len() <= cap {
        return initial;
    }

    projected.archive = None;
    projected.truncated = Some(true);

    let after_archive =
        serde_json::to_string_pretty(projected).expect("ProjectedContext serializes");
    if after_archive.len() <= cap {
        return after_archive;
    }

    if let Some(tasks) = projected.tasks.as_mut() {
        tasks.active.truncate(5);
    }

    serde_json::to_string_pretty(projected).expect("ProjectedContext serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PathExt;

    fn arked_tempdir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        tmp.path().join(".ark/tasks").ensure_dir().unwrap();
        tmp.path().join(".ark/tasks/archive").ensure_dir().unwrap();
        tmp.path().join(".ark/specs/project").ensure_dir().unwrap();
        tmp.path().join(".ark/specs/features").ensure_dir().unwrap();
        tmp
    }

    #[test]
    fn context_errors_on_non_ark_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let opts = ContextOptions::new(tmp.path()).with_format(Format::Json);
        let err = context(opts).unwrap_err();
        assert!(matches!(err, Error::NotLoaded { .. }));
    }

    #[test]
    fn context_session_json_wraps_in_session_start_envelope() {
        let tmp = arked_tempdir();
        let opts = ContextOptions::new(tmp.path()).with_format(Format::Json);
        let summary = context(opts).unwrap();
        let s = format!("{summary}");

        // Outer envelope: hookSpecificOutput → hookEventName + additionalContext.
        let outer: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(outer["hookSpecificOutput"]["hookEventName"], "SessionStart");

        // Inner additionalContext is a stringified ProjectedContext.
        let inner = outer["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .expect("additionalContext is a string");
        let parsed: serde_json::Value = serde_json::from_str(inner).unwrap();
        assert_eq!(parsed["schema"], 1);
        assert_eq!(parsed["scope"], "session");

        // Sibling fields keep raw stdout out of transcript mode and surface
        // a single user-visible line summarizing the load.
        assert_eq!(outer["suppressOutput"], serde_json::Value::Bool(true));
        let msg = outer["systemMessage"]
            .as_str()
            .expect("systemMessage is a string");
        assert!(msg.starts_with("Ark: "), "msg={msg}");
        assert!(
            !msg.contains('\n'),
            "systemMessage must be single-line: {msg}"
        );
        assert!(msg.contains("branch="), "msg={msg}");

        // Empty-state default does not trigger truncation.
        assert!(parsed.get("truncated").is_none(), "parsed={parsed}");

        assert!(s.ends_with('\n'));
    }

    /// `summary_one_line` includes the current-task slug/tier/phase when
    /// present. Covers the path that the empty-state envelope test does
    /// not exercise (the tempdir has no focused task).
    #[test]
    fn summary_one_line_with_current_task_includes_slug_tier_phase() {
        use std::path::PathBuf;

        use chrono::Utc;

        use crate::commands::{
            agent::state::{Phase, Tier},
            context::model::{CurrentTask, GitState, TaskSummary, TasksState},
        };

        let summary = TaskSummary {
            slug: "demo".to_string(),
            title: "demo".to_string(),
            tier: Tier::Quick,
            phase: Phase::Execute,
            iteration: 0,
            path: PathBuf::from(".ark/tasks/demo"),
            updated_at: Utc::now(),
        };
        let projected = ProjectedContext {
            schema: 1,
            scope: ScopeTag::Session,
            generated_at: Utc::now(),
            project_root: PathBuf::from("/tmp/proj"),
            git: GitState {
                branch: "main".to_string(),
                ..GitState::default()
            },
            tasks: Some(TasksState::default()),
            current_task: Some(CurrentTask {
                slug: "demo".to_string(),
                summary,
                artifacts: vec![],
                related_specs: vec![],
            }),
            specs: None,
            archive: None,
            record: None,
            truncated: None,
        };
        let line = summary_one_line(&projected);
        assert!(line.starts_with("Ark: "), "{line}");
        assert!(line.contains("branch=main"), "{line}");
        assert!(line.contains("current=demo (quick/execute)"), "{line}");
        assert!(!line.contains('\n'), "{line}");
    }

    /// When the inner payload exceeds [`ADDITIONAL_CONTEXT_CAP`], the
    /// envelope still emits valid JSON, the inner stringified projection
    /// fits under the cap, and `truncated: true` is set so downstream
    /// readers know they're seeing a partial snapshot.
    #[test]
    fn context_session_json_trims_oversized_payload() {
        let tmp = arked_tempdir();
        // Seed an archive directory full of synthetic entries to push the
        // raw payload over the cap. ARCHIVE_CAP is 5, so 5 directories with
        // long titles in their task.toml is enough.
        let archive_root = tmp.path().join(".ark/tasks/archive/2026-05");
        archive_root.ensure_dir().unwrap();
        for i in 0..5 {
            let dir = archive_root.join(format!("synthetic-task-{i:02}"));
            dir.ensure_dir().unwrap();
            let title = "x".repeat(2_500); // 5 * 2_500 bytes ≈ pushes past 9,500
            let task_toml = format!(
                r#"id = "synthetic-task-{i:02}"
title = "{title}"
tier = "quick"
phase = "archived"
iteration = 0
created_at = "2026-05-01T00:00:00Z"
updated_at = "2026-05-01T00:00:00Z"
archived_at = "2026-05-01T00:00:00Z"
"#
            );
            std::fs::write(dir.join("task.toml"), task_toml).unwrap();
        }

        let opts = ContextOptions::new(tmp.path()).with_format(Format::Json);
        let summary = context(opts).unwrap();
        let s = format!("{summary}");

        let outer: serde_json::Value = serde_json::from_str(&s).unwrap();
        let inner = outer["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap();
        assert!(
            inner.len() <= ADDITIONAL_CONTEXT_CAP,
            "inner payload exceeds cap: {} > {}",
            inner.len(),
            ADDITIONAL_CONTEXT_CAP
        );

        let parsed: serde_json::Value = serde_json::from_str(inner).unwrap();
        assert_eq!(parsed["truncated"], true);
        // Archive is the first thing we drop.
        assert!(parsed.get("archive").is_none(), "archive should be dropped");
    }

    #[test]
    fn context_phase_json_emits_raw_projection_without_envelope() {
        let tmp = arked_tempdir();
        let opts = ContextOptions::new(tmp.path())
            .with_scope(Scope::Phase(PhaseFilter::Design))
            .with_format(Format::Json);
        let summary = context(opts).unwrap();
        let s = format!("{summary}");

        // Phase JSON is consumed by slash commands that parse it inline; it
        // is NOT wrapped in the SessionStart hook envelope.
        assert!(!s.contains("hookSpecificOutput"), "got:\n{s}");
        assert!(s.contains("\"schema\": 1"));
        assert!(s.contains("\"scope\": \"phase\""));
        assert!(s.contains("\"phase\": \"design\""));
        assert!(s.ends_with('\n'));
    }

    /// Verifies that `commands/` contains no `Command::new` call sites.
    ///
    /// The git helper lives in `io/git.rs`, which is not under `commands/`.
    #[test]
    fn commands_no_bare_command_new() {
        // Concatenate every commands/*.rs source via include_str! at compile
        // time. This mirrors `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`
        // in `commands/upgrade.rs`.
        const SOURCES: &[(&str, &str)] = &[
            ("commands/init.rs", include_str!("../init.rs")),
            ("commands/load.rs", include_str!("../load.rs")),
            ("commands/unload.rs", include_str!("../unload.rs")),
            ("commands/remove.rs", include_str!("../remove.rs")),
            ("commands/upgrade/mod.rs", include_str!("../upgrade/mod.rs")),
            (
                "commands/upgrade/plan.rs",
                include_str!("../upgrade/plan.rs"),
            ),
            ("commands/mod.rs", include_str!("../mod.rs")),
            ("commands/agent/mod.rs", include_str!("../agent/mod.rs")),
            ("commands/agent/state.rs", include_str!("../agent/state.rs")),
            (
                "commands/agent/template.rs",
                include_str!("../agent/template.rs"),
            ),
            (
                "commands/agent/task/mod.rs",
                include_str!("../agent/task/mod.rs"),
            ),
            (
                "commands/agent/task/new.rs",
                include_str!("../agent/task/new.rs"),
            ),
            (
                "commands/agent/task/phase.rs",
                include_str!("../agent/task/phase.rs"),
            ),
            (
                "commands/agent/task/promote.rs",
                include_str!("../agent/task/promote.rs"),
            ),
            (
                "commands/agent/task/archive.rs",
                include_str!("../agent/task/archive.rs"),
            ),
            (
                "commands/agent/spec/mod.rs",
                include_str!("../agent/spec/mod.rs"),
            ),
            (
                "commands/agent/spec/extract.rs",
                include_str!("../agent/spec/extract.rs"),
            ),
            (
                "commands/agent/spec/register.rs",
                include_str!("../agent/spec/register.rs"),
            ),
            ("commands/context/mod.rs", include_str!("./mod.rs")),
            ("commands/context/gather.rs", include_str!("./gather.rs")),
            ("commands/context/model.rs", include_str!("./model.rs")),
            (
                "commands/context/projection.rs",
                include_str!("./projection.rs"),
            ),
            ("commands/context/render.rs", include_str!("./render.rs")),
            (
                "commands/context/related_specs.rs",
                include_str!("./related_specs.rs"),
            ),
        ];
        for (name, source) in SOURCES {
            // Strip everything after `#[cfg(test)]` heuristically — same
            // technique used in upgrade.rs's analog test. Tests are allowed
            // to call Command::new (e.g. for setting up git fixtures).
            let live = match source.find("#[cfg(test)]") {
                Some(idx) => &source[..idx],
                None => source,
            };
            assert!(
                !live.contains("Command::new"),
                "non-test code in {name} contains Command::new — use io::git::run_git instead",
            );
        }
    }
}
