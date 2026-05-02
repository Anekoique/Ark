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
            let raw =
                serde_json::to_string_pretty(&projected).expect("ProjectedContext serializes");
            let body = if matches!(opts.scope, Scope::Session) {
                wrap_session_start_envelope(&raw)
            } else {
                raw
            };
            Ok(ContextSummary::Json(format!("{body}\n")))
        }
        Format::Text => Ok(ContextSummary::Text(Box::new(projected))),
    }
}

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
fn wrap_session_start_envelope(payload: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": payload,
        }
    }))
    .expect("envelope serializes")
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

        assert!(s.ends_with('\n'));
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
