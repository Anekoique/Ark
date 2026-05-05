//! End-to-end integration tests for the `ark agent` namespace.
//!
//! Exercises the post-refactor task lifecycle across quick/standard/deep tiers
//! in a tempdir. The closure path (`task_commit`) requires a real git repo with
//! staged work; rather than spinning up git for these tests, the lifecycle
//! tests hand-flip phase to `Committed` (via a small helper) before invoking
//! `task_archive_move`. End-to-end commit tests live alongside `task_commit`
//! itself in `crates/ark-core/src/commands/agent/task/commit.rs`.

use ark_core::{
    InitOptions, PathExt, Phase, TaskArchiveMoveOptions, TaskNewOptions, TaskPhaseOptions,
    TaskToml, Tier, init, task_archive_move, task_execute, task_new, task_plan, task_review,
    task_verify,
};
use chrono::Utc;

fn init_ark(tmp: &std::path::Path) {
    init(InitOptions::new(tmp)).unwrap();
}

/// Hand-flips a task's `phase` to `Committed` so `task_archive_move` has a
/// legal precondition. Sets `committed_at` to now so callers that derive an
/// archive month from it get a valid value. This is *not* the real closure
/// path — the integration tests below exercise the side-effect-free archive
/// helper, not the full `task_commit` protocol.
fn force_committed(tmp: &std::path::Path, slug: &str) {
    let layout = ark_core::Layout::new(tmp);
    let task_dir = layout.task_dir(slug);
    let mut toml = TaskToml::load(&task_dir).unwrap();
    toml.phase = Phase::Committed;
    let now = Utc::now();
    toml.committed_at = Some(now);
    toml.updated_at = now;
    toml.save(&task_dir).unwrap();
}

#[test]
fn standard_tier_archive_after_commit() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());

    let opts = |slug: &str| TaskPhaseOptions {
        project_root: tmp.path().to_path_buf(),
        slug: slug.into(),
    };

    task_new(TaskNewOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "std1".into(),
        title: "standard demo".into(),
        tier: Tier::Standard,
        worktree: None,
    })
    .unwrap();
    task_plan(opts("std1")).unwrap();
    task_execute(opts("std1")).unwrap();
    task_verify(opts("std1")).unwrap();
    force_committed(tmp.path(), "std1");

    let s = task_archive_move(TaskArchiveMoveOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "std1".into(),
        archive_month: "2026-05".into(),
    })
    .unwrap();
    assert!(s.archive_path.exists());
    assert!(s.archive_path.join("PRD.md").exists());
    // Standard tier seeds the unprefixed `PLAN.md`.
    assert!(s.archive_path.join("PLAN.md").exists());
    assert!(s.archive_path.join("VERIFY.md").exists());
    assert!(!tmp.path().join(".ark/tasks/std1").exists());

    // Active set in `.state.toml` no longer contains the archived slug.
    let state = ark_core::load_state(
        &ark_core::Layout::new(tmp.path()),
        &ark_core::RealPpid::new(),
    )
    .unwrap();
    assert!(!state.tasks.active.iter().any(|s| s == "std1"));
    // Side-effect-free archive: no SPEC promotion happens here.
    assert!(!tmp.path().join(".ark/specs/features/std1").exists());

    let toml = TaskToml::load(&s.archive_path).unwrap();
    assert_eq!(toml.phase, Phase::Archived);
    assert!(toml.archived_at.is_some());
}

#[test]
fn deep_tier_archive_after_commit_does_not_promote_spec() {
    // Post-refactor: SPEC promotion happens at `task_commit` time, not at
    // archive time. This test drives the archive-move helper directly with
    // a `Committed` task and asserts no SPEC files appear afterwards.
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());

    let opts = |slug: &str| TaskPhaseOptions {
        project_root: tmp.path().to_path_buf(),
        slug: slug.into(),
    };

    task_new(TaskNewOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "deep1".into(),
        title: "deep demo".into(),
        tier: Tier::Deep,
        worktree: None,
    })
    .unwrap();
    task_plan(opts("deep1")).unwrap();
    tmp.path()
        .join(".ark/tasks/deep1/00_PLAN.md")
        .write_bytes(b"# plan 00\n## Spec\n\n[**Goals**]\n- G-1: v1\n\n## Runtime\nrt\n")
        .unwrap();
    task_review(opts("deep1")).unwrap();
    task_execute(opts("deep1")).unwrap();
    task_verify(opts("deep1")).unwrap();
    force_committed(tmp.path(), "deep1");

    let pre_spec_dir = tmp.path().join(".ark/specs/features/deep1");
    assert!(!pre_spec_dir.exists(), "no SPEC dir before archive");

    let s = task_archive_move(TaskArchiveMoveOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "deep1".into(),
        archive_month: "2026-05".into(),
    })
    .unwrap();

    assert!(s.archive_path.join("00_PLAN.md").exists());
    assert!(s.archive_path.join("00_REVIEW.md").exists());
    // Archive must not promote a SPEC: that's task_commit's job.
    assert!(
        !pre_spec_dir.exists(),
        "task_archive_move must not promote a SPEC; got {:?}",
        pre_spec_dir
    );
}

#[test]
fn quick_tier_archive_after_commit() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());

    task_new(TaskNewOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "q1".into(),
        title: "quick demo".into(),
        tier: Tier::Quick,
        worktree: None,
    })
    .unwrap();
    task_execute(TaskPhaseOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "q1".into(),
    })
    .unwrap();
    force_committed(tmp.path(), "q1");

    let s = task_archive_move(TaskArchiveMoveOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "q1".into(),
        archive_month: "2026-05".into(),
    })
    .unwrap();
    assert_eq!(s.tier, Tier::Quick);
    assert!(s.archive_path.exists());
    assert!(s.archive_path.join("PRD.md").exists());
    assert!(!s.archive_path.join("00_PLAN.md").exists());
    assert!(!s.archive_path.join("VERIFY.md").exists());
}

#[test]
fn embedded_slash_commands_do_not_contain_raw_recipes() {
    use ark_core::templates::CLAUDE_TEMPLATES;

    let files = ["commands/ark/quick.md", "commands/ark/design.md"];
    for rel in files {
        let file = CLAUDE_TEMPLATES
            .get_file(rel)
            .unwrap_or_else(|| panic!("expected embedded {rel}"));
        let body = std::str::from_utf8(file.contents()).unwrap();

        for forbidden in [
            "mkdir -p \".ark/tasks/",
            "cp .ark/templates/",
            "echo \"$SLUG\" > .ark/tasks/.current",
            "mv \".ark/tasks/",
        ] {
            assert!(
                !body.contains(forbidden),
                "{rel} should not contain raw recipe `{forbidden}`"
            );
        }

        assert!(
            body.contains("ark agent task"),
            "{rel} should reference `ark agent task` commands"
        );
    }
}
