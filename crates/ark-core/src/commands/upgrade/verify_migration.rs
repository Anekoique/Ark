//! In-flight `VERIFY.md` migration to the post-refactor living-checklist shape.
//!
//! Detection heuristic: a `VERIFY.md` with a top-level `## Verdict` heading is
//! the legacy verdict-driven body. Migration regenerates the file using the
//! new template (seeded from project SPEC INDEX, the PRD's Related Specs and
//! Outcome, and the latest plan's Goals), preserving any prior `### V-NNN`
//! findings verbatim under the new `## Findings` section.
//!
//! Per-task errors are logged to stderr and skipped; the upgrade flow does not
//! abort on a single bad task.

use crate::{
    commands::agent::{
        state::{Phase, TaskToml},
        task::verify_seed::{
            SeedInputs, read_plan_goals, read_prd_constraints, read_project_specs,
            read_related_specs, render_seeded_verify,
        },
    },
    error::Result,
    io::PathExt,
    layout::Layout,
    templates::ARK_TEMPLATES,
};

/// Walks `.ark/tasks/<slug>/` and migrates every legacy `VERIFY.md`.
///
/// Returns the number of tasks whose `VERIFY.md` was rewritten. Tasks whose
/// VERIFY is already in the new shape (no `## Verdict` heading near the top)
/// are skipped silently — re-running the upgrade is idempotent.
pub fn migrate_in_flight_verify_files(layout: &Layout) -> usize {
    let tasks_dir = layout.tasks_dir();
    let Ok(entries) = std::fs::read_dir(&tasks_dir) else {
        return 0;
    };
    let mut migrated = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().map(|n| n == "archive").unwrap_or(false) {
            continue;
        }
        match try_migrate_task(layout, &path) {
            Ok(true) => migrated += 1,
            Ok(false) => {}
            Err(e) => {
                eprintln!(
                    "ark upgrade: failed to migrate VERIFY.md at {:?}: {e}",
                    path
                );
            }
        }
    }
    migrated
}

/// Returns `Ok(true)` when the task's VERIFY.md was rewritten; `Ok(false)`
/// when nothing needed to change.
fn try_migrate_task(layout: &Layout, task_dir: &std::path::Path) -> Result<bool> {
    let toml_path = task_dir.join("task.toml");
    if !toml_path.exists() {
        return Ok(false);
    }
    let toml = TaskToml::load(task_dir)?;
    if !matches!(toml.phase, Phase::Verify | Phase::Committed) {
        return Ok(false);
    }
    let verify_path = task_dir.join("VERIFY.md");
    let Some(legacy) = verify_path.read_text_optional()? else {
        return Ok(false);
    };
    if !is_legacy_verdict_shape(&legacy) {
        return Ok(false);
    }

    let template_bytes = ARK_TEMPLATES
        .get_file("templates/VERIFY.md")
        .map(|f| f.contents())
        .unwrap_or_default();
    let template = std::str::from_utf8(template_bytes).unwrap_or_default();

    let prd_path = task_dir.join("PRD.md");
    let inputs = SeedInputs {
        project_specs: read_project_specs(&layout.specs_project_index())?,
        related_specs: read_related_specs(&prd_path)?,
        prd_constraints: read_prd_constraints(&prd_path)?,
        plan_goals: latest_plan_goals(task_dir)?,
    };
    let mut rendered = render_seeded_verify(template, &inputs, &verify_path)?;

    // Preserve prior `### V-NNN` findings verbatim. Lift the block by name;
    // append it to the migrated body's `## Findings` section.
    let preserved_findings = extract_findings_block(&legacy);
    if !preserved_findings.trim().is_empty() {
        rendered = inject_legacy_findings(&rendered, &preserved_findings);
    }
    eprintln!("ark upgrade: dropped legacy verdict from `{}`", toml.id);
    verify_path.write_bytes(rendered.as_bytes())?;
    Ok(true)
}

/// Heuristic: `## Verdict` near the top of the file marks the legacy shape.
///
/// Limited to the first 40 lines so the cost is bounded and we don't match an
/// inadvertent `## Verdict` written into a Notes section by an implementer.
fn is_legacy_verdict_shape(text: &str) -> bool {
    text.lines()
        .take(40)
        .any(|l| l.trim_start().starts_with("## Verdict"))
}

/// Returns the latest plan's Goals as bullet labels (mirrors the helper in
/// `task::phase`; duplicated here to avoid pulling phase.rs into upgrade's
/// dependency graph).
///
/// Prefers `PLAN.md` (standard tier); falls back to highest `NN_PLAN.md`.
fn latest_plan_goals(task_dir: &std::path::Path) -> Result<Vec<String>> {
    let plain = task_dir.join("PLAN.md");
    if plain.is_file() {
        return read_plan_goals(&plain);
    }
    let mut highest: Option<(u32, std::path::PathBuf)> = None;
    let Ok(entries) = std::fs::read_dir(task_dir) else {
        return Ok(Vec::new());
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(stem) = name.strip_suffix("_PLAN.md") else {
            continue;
        };
        let Ok(n) = stem.parse::<u32>() else { continue };
        if highest.as_ref().is_none_or(|(prev, _)| n > *prev) {
            highest = Some((n, entry.path()));
        }
    }
    match highest {
        Some((_, path)) => read_plan_goals(&path),
        None => Ok(Vec::new()),
    }
}

/// Lifts the `## Findings` block from a legacy VERIFY.md.
///
/// Returns the body of the section excluding the heading itself; empty
/// string when no findings exist (legacy verdict-only document).
fn extract_findings_block(text: &str) -> String {
    let mut in_section = false;
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("## Findings") {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            // Stop at the next H2.
            break;
        }
        if in_section {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Appends `findings` text after the new template's `## Findings` heading,
/// in front of the seeded V-NNN scaffolding so the user sees their prior
/// findings first when scanning the document.
fn inject_legacy_findings(body: &str, findings: &str) -> String {
    let needle = "## Findings\n";
    let Some(idx) = body.find(needle) else {
        return body.to_string();
    };
    let insertion_point = idx + needle.len();
    let mut out = String::with_capacity(body.len() + findings.len() + 64);
    out.push_str(&body[..insertion_point]);
    out.push_str(
        "\n> Migrated from a pre-refactor `VERIFY.md`. The legacy verdict heading was dropped; \
         prior findings preserved below verbatim.\n\n",
    );
    out.push_str(findings.trim_end());
    out.push_str("\n\n");
    out.push_str(&body[insertion_point..]);
    out
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::{
        commands::agent::{
            state::Tier,
            task::new::{TaskNewOptions, task_new},
        },
        init::{InitOptions, init},
    };

    fn legacy_verify_body() -> &'static str {
        "# `demo` VERIFY 00\n\n> Status: Open\n\n## Verdict\n\n- Decision: Approved\n- Blocking \
         Issues: 0\n\n## Findings\n\n### V-001 `prior finding`\n\n- Severity: HIGH\n- Problem: old \
         issue\n\n## Follow-ups\n- (none)\n"
    }

    /// Verifies that a legacy-shaped VERIFY.md is rewritten in place.
    #[test]
    fn migrates_legacy_verdict_verify_md() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "demo".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        // Force phase=Verify and place a legacy VERIFY.md on disk.
        let layout = Layout::new(tmp.path());
        let task_dir = layout.task_dir("demo");
        let mut toml = TaskToml::load(&task_dir).unwrap();
        toml.phase = Phase::Verify;
        toml.updated_at = Utc::now();
        toml.save(&task_dir).unwrap();
        task_dir
            .join("VERIFY.md")
            .write_bytes(legacy_verify_body().as_bytes())
            .unwrap();

        let migrated = migrate_in_flight_verify_files(&layout);
        assert_eq!(migrated, 1);

        let body = task_dir.join("VERIFY.md").read_text().unwrap();
        assert!(
            !body.contains("## Verdict"),
            "legacy verdict heading must be dropped: {body}"
        );
        assert!(
            body.contains("V-001"),
            "prior finding must be preserved: {body}"
        );
        assert!(
            body.contains("## Project Spec Compliance"),
            "new shape must be present: {body}"
        );
    }

    /// Verifies idempotency: re-running on an already-migrated file is a no-op.
    #[test]
    fn migration_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "demo".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        let layout = Layout::new(tmp.path());
        let task_dir = layout.task_dir("demo");
        let mut toml = TaskToml::load(&task_dir).unwrap();
        toml.phase = Phase::Verify;
        toml.save(&task_dir).unwrap();
        task_dir
            .join("VERIFY.md")
            .write_bytes(legacy_verify_body().as_bytes())
            .unwrap();

        assert_eq!(migrate_in_flight_verify_files(&layout), 1);
        // Second run should find nothing legacy left.
        assert_eq!(migrate_in_flight_verify_files(&layout), 0);
    }

    /// Verifies that tasks not in `Verify` or `Committed` phase are skipped.
    #[test]
    fn skips_tasks_not_in_verify_or_committed() {
        let tmp = tempfile::tempdir().unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "early".into(),
            title: "early".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        // Phase remains Design. Drop a legacy VERIFY.md anyway.
        let layout = Layout::new(tmp.path());
        let task_dir = layout.task_dir("early");
        task_dir
            .join("VERIFY.md")
            .write_bytes(legacy_verify_body().as_bytes())
            .unwrap();

        assert_eq!(migrate_in_flight_verify_files(&layout), 0);
        // File untouched.
        let body = task_dir.join("VERIFY.md").read_text().unwrap();
        assert!(body.contains("## Verdict"));
    }
}
