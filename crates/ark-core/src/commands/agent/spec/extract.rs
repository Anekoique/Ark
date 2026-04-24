//! `ark agent spec extract` — pull the final PLAN's `## Spec` section into
//! `specs/features/<slug>/SPEC.md`.
//!
//! Deep-tier only. Resolves the final PLAN by picking the highest-NN
//! `NN_PLAN.md` in the task dir (overridable). On fresh write, emits the
//! extracted body; on overwrite, emits the new body followed by a dated
//! CHANGELOG entry noting the replacement.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use chrono::Utc;

use crate::{
    commands::agent::state::{TaskToml, Tier, validate_slug},
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

#[derive(Debug, Clone)]
pub struct SpecExtractOptions {
    pub project_root: PathBuf,
    pub slug: String,
    /// Optional explicit plan path; defaults to highest-NN `NN_PLAN.md`.
    pub plan_override: Option<PathBuf>,
    /// Optional task directory override. Used by `task archive` which operates
    /// on the archived task path (`tasks/archive/YYYY-MM/<slug>/`) rather than
    /// the active path (`tasks/<slug>/`). Defaults to the active path.
    pub task_dir_override: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SpecExtractSummary {
    pub slug: String,
    pub target_path: PathBuf,
    pub was_update: bool,
}

impl fmt::Display for SpecExtractSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let verb = if self.was_update { "updated" } else { "wrote" };
        write!(
            f,
            "{verb} SPEC for `{}` at {}",
            self.slug,
            self.target_path.display()
        )
    }
}

pub fn spec_extract(opts: SpecExtractOptions) -> Result<SpecExtractSummary> {
    validate_slug(&opts.slug)?;

    let layout = Layout::new(&opts.project_root);
    let task_dir = opts
        .task_dir_override
        .clone()
        .unwrap_or_else(|| layout.task_dir(&opts.slug));

    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    let toml = TaskToml::load(&task_dir)?;
    if toml.tier != Tier::Deep {
        return Err(Error::WrongTier {
            expected: Tier::Deep,
            actual: toml.tier,
        });
    }

    let plan_path = match opts.plan_override {
        Some(p) => p,
        None => find_final_plan(&task_dir)?,
    };
    let plan_text = plan_path.read_text()?;
    let extracted = extract_spec_section(&plan_text).ok_or_else(|| Error::SpecSectionMissing {
        plan_path: plan_path.clone(),
    })?;

    let target_dir = layout.specs_feature_dir(&opts.slug);
    target_dir.ensure_dir()?;
    let target_path = target_dir.join("SPEC.md");
    let was_update = target_path.exists();

    let iteration = plan_iteration_nn(&plan_path).unwrap_or(toml.iteration);
    let mut content = extracted.trim_end().to_string();
    content.push('\n');
    if was_update {
        let today = Utc::now().format("%Y-%m-%d");
        content.push_str(&format!(
            "\n[**CHANGELOG**]\n\n- {today}: replaced from {nn:02}_PLAN.md (prior body preserved \
             in git history)\n",
            nn = iteration,
        ));
    }
    target_path.write_bytes(content.as_bytes())?;

    Ok(SpecExtractSummary {
        slug: opts.slug,
        target_path,
        was_update,
    })
}

/// Locate the highest-NN `NN_PLAN.md` in a task directory.
fn find_final_plan(task_dir: &Path) -> Result<PathBuf> {
    let mut best: Option<(u32, PathBuf)> = None;
    for entry in task_dir.list_dir()? {
        let entry = entry.map_err(|e| Error::io(task_dir, e))?;
        let Some(nn) = parse_nn_plan(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        if best.as_ref().is_none_or(|(best_nn, _)| nn > *best_nn) {
            best = Some((nn, entry.path()));
        }
    }
    best.map(|(_, p)| p).ok_or_else(|| Error::NoPlanFound {
        task_dir: task_dir.to_path_buf(),
    })
}

/// Parse `NN` out of a filename like `"03_PLAN.md"`.
fn parse_nn_plan(name: &str) -> Option<u32> {
    let stripped = name.strip_suffix("_PLAN.md")?;
    (stripped.len() == 2).then_some(())?;
    stripped.parse().ok()
}

fn plan_iteration_nn(plan_path: &Path) -> Option<u32> {
    parse_nn_plan(&plan_path.file_name()?.to_string_lossy())
}

/// Extract the `## Spec` section body. Returns `None` if no start line matches.
fn extract_spec_section(text: &str) -> Option<String> {
    let mut lines = text.lines().skip_while(|l| !is_spec_start(l));
    lines.next()?; // drop the `## Spec` header itself
    Some(
        lines
            .take_while(|l| !is_section_boundary(l))
            .fold(String::new(), |mut acc, l| {
                acc.push_str(l);
                acc.push('\n');
                acc
            }),
    )
}

/// Match `## Spec` or `## Spec ...`, rejecting `## Speculation`.
fn is_spec_start(line: &str) -> bool {
    let bytes = line.as_bytes();
    line.starts_with("## Spec") && (bytes.len() == 7 || bytes[7] == b' ')
}

/// Any H2 header or bare `##` line; `###` subheadings do not match.
fn is_section_boundary(line: &str) -> bool {
    line.starts_with("## ") || line == "##"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::{
        state::Tier,
        task::{
            new::{TaskNewOptions, task_new},
            phase::{TaskPhaseOptions, task_plan},
        },
    };

    fn setup_deep_with_plan_body(tmp_path: &std::path::Path, plan_body: &str) {
        task_new(TaskNewOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Deep,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp_path.to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();
        let plan_path = tmp_path.join(".ark/tasks/demo/00_PLAN.md");
        plan_path.write_bytes(plan_body.as_bytes()).unwrap();
    }

    #[test]
    fn extracts_plain_header() {
        let body = "# Title\n## Spec\n\ngoals and stuff\n\n## Runtime\nnot spec\n";
        let out = extract_spec_section(body).unwrap();
        assert!(out.contains("goals and stuff"));
        assert!(!out.contains("not spec"));
    }

    #[test]
    fn extracts_with_inline_code_suffix() {
        let body = "## Spec `{Core specification}`\n\nG-1: foo\n\n## Runtime\n";
        let out = extract_spec_section(body).unwrap();
        assert!(out.contains("G-1: foo"));
    }

    #[test]
    fn does_not_terminate_on_subheading() {
        let body = "## Spec\n\nintro\n\n### Subheading\n\ndetail\n\n## Runtime\n";
        let out = extract_spec_section(body).unwrap();
        assert!(out.contains("### Subheading"));
        assert!(out.contains("detail"));
        assert!(!out.contains("Runtime"));
    }

    #[test]
    fn rejects_wrong_prefix() {
        let body = "## Speculation\n\nnot this\n\n## Runtime\n";
        assert!(extract_spec_section(body).is_none());
    }

    #[test]
    fn spec_extract_writes_fresh_spec() {
        let tmp = tempfile::tempdir().unwrap();
        setup_deep_with_plan_body(
            tmp.path(),
            "# title\n## Spec\n\n[**Goals**]\n- G-1: ship it\n\n## Runtime\n",
        );

        let s = spec_extract(SpecExtractOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            plan_override: None,
            task_dir_override: None,
        })
        .unwrap();
        assert!(!s.was_update);
        let spec = s.target_path.read_text().unwrap();
        assert!(spec.contains("G-1: ship it"));
        assert!(!spec.contains("CHANGELOG"));
    }

    #[test]
    fn spec_extract_appends_changelog_on_update() {
        let tmp = tempfile::tempdir().unwrap();
        setup_deep_with_plan_body(
            tmp.path(),
            "## Spec\n\n[**Goals**]\n- G-1: v1\n\n## Runtime\n",
        );
        spec_extract(SpecExtractOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            plan_override: None,
            task_dir_override: None,
        })
        .unwrap();

        tmp.path()
            .join(".ark/tasks/demo/00_PLAN.md")
            .write_bytes(b"## Spec\n\n[**Goals**]\n- G-1: v2\n\n## Runtime\n")
            .unwrap();

        let s = spec_extract(SpecExtractOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            plan_override: None,
            task_dir_override: None,
        })
        .unwrap();
        assert!(s.was_update);
        let spec = s.target_path.read_text().unwrap();
        assert!(spec.contains("G-1: v2"));
        assert!(!spec.contains("G-1: v1"));
        assert!(spec.contains("CHANGELOG"));
    }

    #[test]
    fn spec_extract_wrong_tier_errors() {
        let tmp = tempfile::tempdir().unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "t".into(),
            tier: Tier::Standard,
        })
        .unwrap();
        let err = spec_extract(SpecExtractOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            plan_override: None,
            task_dir_override: None,
        })
        .unwrap_err();
        assert!(matches!(
            err,
            Error::WrongTier {
                expected: Tier::Deep,
                actual: Tier::Standard
            }
        ));
    }

    #[test]
    fn spec_extract_missing_section_errors() {
        let tmp = tempfile::tempdir().unwrap();
        setup_deep_with_plan_body(tmp.path(), "# no spec section\n## Runtime\nstuff\n");
        let err = spec_extract(SpecExtractOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            plan_override: None,
            task_dir_override: None,
        })
        .unwrap_err();
        assert!(matches!(err, Error::SpecSectionMissing { .. }));
    }
}
