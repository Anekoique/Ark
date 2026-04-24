//! End-to-end integration tests for `ark upgrade`.
//!
//! Exercises the library surface (not the binary) in a tempdir so tests don't
//! depend on spawning the compiled CLI. The binary's CLI-facing tests live in
//! `cli_help.rs` / `cli_upgrade.rs`.

use std::path::{Path, PathBuf};

use ark_core::{
    ConflictChoice, ConflictPolicy, InitOptions, LoadOptions, Prompter, SpecRegisterOptions,
    UnloadOptions, UpgradeOptions, hash_bytes, init, load, spec_register, unload, upgrade,
};
use chrono::NaiveDate;

fn init_ark(tmp: &Path) {
    init(InitOptions::new(tmp)).unwrap();
}

fn manifest_path(tmp: &Path) -> PathBuf {
    tmp.join(".ark/.installed.json")
}

fn read_manifest(tmp: &Path) -> serde_json::Value {
    let raw = std::fs::read_to_string(manifest_path(tmp)).unwrap();
    serde_json::from_str(&raw).unwrap()
}

/// Read `.installed.json`, apply `edit`, write it back.
fn modify_manifest(tmp: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    let path = manifest_path(tmp);
    let mut m: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    edit(&mut m);
    std::fs::write(&path, serde_json::to_string_pretty(&m).unwrap()).unwrap();
}

struct PanicPrompter;
impl Prompter for PanicPrompter {
    fn prompt(&mut self, _: &Path) -> ark_core::Result<ConflictChoice> {
        panic!("prompter must not be invoked for this test");
    }
}

struct StubPrompter(ConflictChoice);
impl Prompter for StubPrompter {
    fn prompt(&mut self, _: &Path) -> ark_core::Result<ConflictChoice> {
        Ok(self.0)
    }
}

#[test]
fn fresh_install_then_upgrade_is_noop() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    assert_eq!(summary.added, 0);
    assert_eq!(summary.updated, 0);
    assert_eq!(summary.overwritten, 0);
    assert_eq!(summary.created_new, 0);
    assert_eq!(summary.deleted, 0);
    assert_eq!(summary.orphaned, 0);
    assert!(summary.unchanged > 0);
}

#[test]
fn template_change_with_unmodified_file_auto_updates() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    // To trigger AutoUpdate: on-disk content must NOT match the embedded
    // template, but the recorded hash must match on-disk. Write a known body
    // and pin the manifest's hash to sha(that body); the embedded template
    // then classifies as "changed since recording, user hasn't touched it".
    let target = tmp.path().join(".ark/workflow.md");
    std::fs::write(&target, b"prior template content").unwrap();
    modify_manifest(tmp.path(), |m| {
        m["hashes"].as_object_mut().unwrap().insert(
            ".ark/workflow.md".into(),
            hash_bytes(b"prior template content").into(),
        );
    });

    let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    assert_eq!(summary.updated, 1);
    assert_ne!(
        std::fs::read_to_string(&target).unwrap(),
        "prior template content"
    );
}

#[test]
fn user_modified_force_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let target = tmp.path().join(".ark/workflow.md");
    std::fs::write(&target, b"user edit").unwrap();
    let summary = upgrade(
        UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Force),
        &mut PanicPrompter,
    )
    .unwrap();
    assert_eq!(summary.overwritten, 1);
    assert_ne!(std::fs::read_to_string(&target).unwrap(), "user edit");
}

#[test]
fn user_modified_skip_preserves() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let target = tmp.path().join(".ark/workflow.md");
    std::fs::write(&target, b"user edit").unwrap();
    let summary = upgrade(
        UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Skip),
        &mut PanicPrompter,
    )
    .unwrap();
    assert_eq!(summary.modified_preserved, 1);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
}

#[test]
fn user_modified_create_new_writes_sidecar() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let target = tmp.path().join(".ark/workflow.md");
    std::fs::write(&target, b"user edit").unwrap();
    let summary = upgrade(
        UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::CreateNew),
        &mut PanicPrompter,
    )
    .unwrap();
    assert_eq!(summary.created_new, 1);
    assert!(tmp.path().join(".ark/workflow.md.new").exists());
    let manifest = read_manifest(tmp.path());
    assert!(
        !manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some(".ark/workflow.md.new"))
    );
}

#[test]
fn ambiguous_no_hash_prompt_path_skip_preserves() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let target = tmp.path().join(".ark/workflow.md");
    std::fs::write(&target, b"user edit").unwrap();
    // Clear the hashes field on the manifest to simulate a pre-hash install.
    modify_manifest(tmp.path(), |m| {
        m.as_object_mut().unwrap().remove("hashes");
    });

    let summary = upgrade(
        UpgradeOptions::new(tmp.path()),
        &mut StubPrompter(ConflictChoice::Skip),
    )
    .unwrap();
    assert_eq!(summary.modified_preserved, 1);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "user edit");
}

#[test]
fn user_authored_task_file_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let user_dir = tmp.path().join(".ark/tasks/my-task");
    std::fs::create_dir_all(&user_dir).unwrap();
    let user_file = user_dir.join("task.toml");
    std::fs::write(&user_file, "title = \"mine\"\n").unwrap();

    upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    assert!(user_file.exists());
    assert_eq!(
        std::fs::read_to_string(&user_file).unwrap(),
        "title = \"mine\"\n"
    );
}

#[test]
fn removed_template_unmodified_is_deleted() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    // Simulate a removed-in-current-version template: add a fake file + matching
    // manifest entry. Upgrade classifies it as SafeRemove.
    let ghost = tmp.path().join(".ark/ghost.md");
    std::fs::write(&ghost, b"ghost content").unwrap();
    modify_manifest(tmp.path(), |m| {
        m["files"]
            .as_array_mut()
            .unwrap()
            .push(".ark/ghost.md".into());
        m["hashes"]
            .as_object_mut()
            .unwrap()
            .insert(".ark/ghost.md".into(), hash_bytes(b"ghost content").into());
    });

    let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    assert_eq!(summary.deleted, 1);
    assert!(!ghost.exists());
}

#[test]
fn removed_template_modified_is_orphaned() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let ghost = tmp.path().join(".ark/ghost.md");
    std::fs::write(&ghost, b"user edited ghost").unwrap();
    modify_manifest(tmp.path(), |m| {
        m["files"]
            .as_array_mut()
            .unwrap()
            .push(".ark/ghost.md".into());
        m["hashes"]
            .as_object_mut()
            .unwrap()
            .insert(".ark/ghost.md".into(), hash_bytes(b"original ghost").into());
    });

    let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    assert_eq!(summary.orphaned, 1);
    assert!(ghost.exists());
    assert_eq!(
        std::fs::read_to_string(&ghost).unwrap(),
        "user edited ghost"
    );
}

#[test]
fn hashes_survive_unload_load_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let before = read_manifest(tmp.path())["hashes"].clone();
    unload(UnloadOptions::new(tmp.path())).unwrap();
    load(LoadOptions::new(tmp.path())).unwrap();
    let after = read_manifest(tmp.path())["hashes"].clone();
    assert_eq!(before, after);
}

#[test]
fn managed_block_body_refreshed() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let claude_md = tmp.path().join("CLAUDE.md");
    let original = std::fs::read_to_string(&claude_md).unwrap();
    // Tamper with the CLAUDE.md block body between markers.
    let tampered = original.replace("Ark is installed", "ARK SOMETHING");
    std::fs::write(&claude_md, &tampered).unwrap();

    upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    let after = std::fs::read_to_string(&claude_md).unwrap();
    assert!(after.contains("Ark is installed"));
    assert!(!after.contains("ARK SOMETHING"));
}

#[test]
fn managed_block_reapplied_when_manifest_lacks_entry() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    modify_manifest(tmp.path(), |m| {
        m["managed_blocks"] = serde_json::Value::Array(Vec::new());
    });

    upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    let claude_md = tmp.path().join("CLAUDE.md");
    let after = std::fs::read_to_string(&claude_md).unwrap();
    assert!(after.contains("<!-- ARK:START -->"));
    assert!(after.contains("<!-- ARK:END -->"));
}

#[test]
fn specs_index_md_round_trips_through_upgrade() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    // Set a stale hash for specs/INDEX.md so it classifies as AutoUpdate.
    let target = tmp.path().join(".ark/specs/INDEX.md");
    std::fs::write(&target, b"prior content").unwrap();
    modify_manifest(tmp.path(), |m| {
        m["hashes"].as_object_mut().unwrap().insert(
            ".ark/specs/INDEX.md".into(),
            hash_bytes(b"prior content").into(),
        );
    });

    let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    assert_eq!(summary.updated, 1);
    assert_ne!(std::fs::read_to_string(&target).unwrap(), "prior content");
}

#[test]
fn hash_backfill_after_same_content() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    modify_manifest(tmp.path(), |m| {
        m["hashes"] = serde_json::json!({});
    });

    upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    let after = read_manifest(tmp.path());
    let file_count = after["files"].as_array().unwrap().len();
    let hash_count = after["hashes"].as_object().unwrap().len();
    assert_eq!(file_count, hash_count);
}

#[test]
fn missing_manifest_errors_not_loaded() {
    let tmp = tempfile::tempdir().unwrap();
    let err = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap_err();
    assert!(matches!(err, ark_core::Error::NotLoaded { .. }));
}

#[test]
fn downgrade_refused_without_flag() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    modify_manifest(tmp.path(), |m| {
        m["version"] = "99.0.0".into();
    });

    let err = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap_err();
    assert!(matches!(err, ark_core::Error::DowngradeRefused { .. }));
}

#[test]
fn downgrade_allowed_with_flag() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    modify_manifest(tmp.path(), |m| {
        m["version"] = "99.0.0".into();
    });

    let summary = upgrade(
        UpgradeOptions::new(tmp.path()).with_allow_downgrade(true),
        &mut PanicPrompter,
    )
    .unwrap();
    assert_eq!(summary.version_from, "99.0.0");
}

#[test]
fn spec_register_then_upgrade_is_noop() {
    // Regression: `spec register` mutates the ARK:FEATURES managed block in
    // .ark/specs/features/INDEX.md, so the file's bytes diverge from the
    // shipped template. Before the reconcile fix, upgrade prompted the user
    // to overwrite and Overwrite wiped the registered features.
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    spec_register(SpecRegisterOptions {
        project_root: tmp.path().to_path_buf(),
        feature: "demo".into(),
        scope: "demo scope".into(),
        from_task: "demo-task".into(),
        date: NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
    })
    .unwrap();
    let index = tmp.path().join(".ark/specs/features/INDEX.md");
    let before = std::fs::read_to_string(&index).unwrap();
    assert!(
        before.contains("| `demo` |"),
        "spec register should have added the row"
    );

    let summary = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap();
    assert_eq!(summary.overwritten, 0);
    assert_eq!(summary.modified_preserved, 0);
    assert_eq!(summary.created_new, 0);

    let after = std::fs::read_to_string(&index).unwrap();
    assert_eq!(
        before, after,
        "upgrade must not touch managed-block content"
    );
}

#[test]
fn user_edit_outside_managed_block_still_prompts() {
    // Reconcile only neutralizes managed-block divergence. If the user edits
    // prose *outside* the block, upgrade must still classify as UserModified.
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    let index = tmp.path().join(".ark/specs/features/INDEX.md");
    let original = std::fs::read_to_string(&index).unwrap();
    // Append a user note outside the managed block.
    std::fs::write(&index, format!("{original}\n\n## My personal note\n")).unwrap();

    let summary = upgrade(
        UpgradeOptions::new(tmp.path()).with_policy(ConflictPolicy::Skip),
        &mut PanicPrompter,
    )
    .unwrap();
    assert_eq!(summary.modified_preserved, 1);
    assert!(
        std::fs::read_to_string(&index)
            .unwrap()
            .contains("My personal note")
    );
}

#[test]
fn manifest_entry_outside_project_root_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    init_ark(tmp.path());
    modify_manifest(tmp.path(), |m| {
        m["files"]
            .as_array_mut()
            .unwrap()
            .push("../escape.md".into());
    });

    let err = upgrade(UpgradeOptions::new(tmp.path()), &mut PanicPrompter).unwrap_err();
    assert!(matches!(err, ark_core::Error::UnsafeManifestPath { .. }));
    // Post-run scan: parent directory must not gain an `escape.md`.
    let parent = tmp
        .path()
        .parent()
        .map(|p| p.join("escape.md"))
        .unwrap_or_else(|| PathBuf::from("escape.md"));
    assert!(!parent.exists());
}
