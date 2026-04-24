//! End-to-end integration tests for the `ark agent` namespace.
//!
//! Exercises the full task lifecycle across quick/standard/deep tiers in a
//! tempdir, verifying filesystem state at each phase transition.

use ark_core::{
    InitOptions, PathExt, Phase, TaskArchiveOptions, TaskNewOptions, TaskPhaseOptions, TaskToml,
    Tier, init, task_archive, task_execute, task_new, task_plan, task_review, task_verify,
};

fn init_ark(tmp: &std::path::Path) {
    init(InitOptions::new(tmp)).unwrap();
}

#[test]
fn standard_tier_round_trip() {
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
    })
    .unwrap();
    task_plan(opts("std1")).unwrap();
    task_execute(opts("std1")).unwrap();
    task_verify(opts("std1")).unwrap();

    let s = task_archive(TaskArchiveOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "std1".into(),
    })
    .unwrap();
    assert!(!s.deep_spec_promoted);
    assert!(s.archive_path.exists());
    assert!(s.archive_path.join("PRD.md").exists());
    assert!(s.archive_path.join("00_PLAN.md").exists());
    assert!(s.archive_path.join("VERIFY.md").exists());
    assert!(!tmp.path().join(".ark/tasks/std1").exists());
    assert!(!tmp.path().join(".ark/tasks/.current").exists());
    assert!(!tmp.path().join(".ark/specs/features/std1").exists());

    let toml = TaskToml::load(&s.archive_path).unwrap();
    assert_eq!(toml.phase, Phase::Archived);
    assert!(toml.archived_at.is_some());
}

#[test]
fn deep_tier_round_trip() {
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
    })
    .unwrap();
    task_plan(opts("deep1")).unwrap();

    // Seed the final plan so spec_extract has content.
    tmp.path()
        .join(".ark/tasks/deep1/00_PLAN.md")
        .write_bytes(b"# plan 00\n## Spec\n\n[**Goals**]\n- G-1: v1\n\n## Runtime\nrt\n")
        .unwrap();

    task_review(opts("deep1")).unwrap();
    task_execute(opts("deep1")).unwrap();
    task_verify(opts("deep1")).unwrap();

    let s = task_archive(TaskArchiveOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "deep1".into(),
    })
    .unwrap();
    assert!(s.deep_spec_promoted);

    assert!(s.archive_path.join("00_PLAN.md").exists());
    assert!(s.archive_path.join("00_REVIEW.md").exists());

    let spec_path = tmp.path().join(".ark/specs/features/deep1/SPEC.md");
    assert!(spec_path.exists());
    let spec = spec_path.read_text_optional().unwrap().unwrap();
    assert!(spec.contains("G-1: v1"));

    let idx = tmp
        .path()
        .join(".ark/specs/features/INDEX.md")
        .read_text_optional()
        .unwrap()
        .unwrap();
    assert!(idx.contains("| `deep1` |"));
}

#[test]
fn quick_tier_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());

    task_new(TaskNewOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "q1".into(),
        title: "quick demo".into(),
        tier: Tier::Quick,
    })
    .unwrap();
    task_execute(TaskPhaseOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "q1".into(),
    })
    .unwrap();

    let s = task_archive(TaskArchiveOptions {
        project_root: tmp.path().to_path_buf(),
        slug: "q1".into(),
    })
    .unwrap();
    assert_eq!(s.tier, Tier::Quick);
    assert!(!s.deep_spec_promoted);
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
