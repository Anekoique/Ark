//! `ark context` — print a structured snapshot of git + `.ark/` workflow
//! state. Read-only; no mutation.
//!
//! This is the top-level public command, paired with [`ark agent`] which
//! handles workflow mutation.

pub mod gather;
pub mod model;
pub mod projection;
pub mod related_specs;
pub mod render;

use std::{fmt, path::PathBuf};

pub use gather::gather_context;
pub use model::{
    ArchiveState, ArchivedTask, ArtifactKind, ArtifactSummary, Context, CurrentTask, GitCommit,
    GitState, SCHEMA_VERSION, SpecRow, SpecsState, TaskSummary, TasksState,
};
pub use projection::{PhaseFilter, ProjectedContext, Scope, ScopeTag, project};
use render::TextSummary;

use crate::{
    error::{Error, Result},
    layout::Layout,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Text,
}

#[derive(Debug, Clone)]
pub struct ContextOptions {
    pub project_root: PathBuf,
    pub scope: Scope,
    pub format: Format,
}

impl ContextOptions {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            scope: Scope::Session,
            format: Format::Text,
        }
    }

    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }
}

/// Implements [`fmt::Display`]; the CLI calls `render(summary)` once.
///
/// JSON mode pre-serializes to a `String` (with trailing newline per C-23)
/// so `Display` is a single byte-write. Text mode formats on demand.
/// `Text` boxes the projection to keep the enum's stack size small.
#[derive(Debug)]
pub enum ContextSummary {
    Json(String),
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
/// combination returns raw output. See ark-context C-23.
pub fn context(opts: ContextOptions) -> Result<ContextSummary> {
    let layout = Layout::new(&opts.project_root);
    let ark_dir = layout.ark_dir();
    if !ark_dir.try_exists().map_err(|e| Error::io(&ark_dir, e))? {
        return Err(Error::NotLoaded {
            path: opts.project_root,
        });
    }
    let ctx = gather_context(&layout)?;
    let projected = project(ctx, opts.scope);
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

/// Wrap a JSON payload in Claude Code's SessionStart envelope. The payload
/// is embedded as a stringified value of `additionalContext` because the
/// hook contract requires that field to be a string.
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

    /// C-26 / R-103 enforcement: no `Command::new` call sites under
    /// `commands/`. The git helper lives in `io/git.rs`, which is not under
    /// `commands/` — so a literal scan over `commands/**/*.rs` should find
    /// no occurrences (excluding tests).
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
            ("commands/upgrade.rs", include_str!("../upgrade.rs")),
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
                "non-test code in {name} contains Command::new — use io::git::run_git instead per \
                 ark-context C-26"
            );
        }
    }
}
