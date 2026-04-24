//! CLI-level tests for `ark upgrade` — help output and flag parsing.

use std::process::Command;

fn ark_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_ark"))
}

fn run(args: &[&str]) -> (String, bool) {
    let out = Command::new(ark_bin())
        .args(args)
        .output()
        .expect("failed to run ark");
    let mut s = String::from_utf8(out.stdout).expect("stdout is utf-8");
    s.push_str(&String::from_utf8(out.stderr).expect("stderr is utf-8"));
    (s, out.status.success())
}

#[test]
fn top_level_help_lists_upgrade() {
    let (out, _) = run(&["--help"]);
    assert!(
        out.contains("upgrade"),
        "`ark --help` must list `upgrade`; got:\n{out}"
    );
}

#[test]
fn upgrade_help_lists_four_flags() {
    let (out, _) = run(&["upgrade", "--help"]);
    for flag in [
        "--force",
        "--skip-modified",
        "--create-new",
        "--allow-downgrade",
    ] {
        assert!(
            out.contains(flag),
            "`ark upgrade --help` must list `{flag}`; got:\n{out}"
        );
    }
}

#[test]
fn upgrade_rejects_two_policy_flags() {
    let (out, ok) = run(&["upgrade", "--force", "--skip-modified"]);
    assert!(!ok, "two policy flags must fail; got:\n{out}");
    assert!(
        out.contains("cannot be used with"),
        "expected clap conflict error; got:\n{out}"
    );
}

#[test]
fn upgrade_rejects_force_plus_create_new() {
    let (_, ok) = run(&["upgrade", "--force", "--create-new"]);
    assert!(!ok);
}

#[test]
fn upgrade_rejects_skip_plus_create_new() {
    let (_, ok) = run(&["upgrade", "--skip-modified", "--create-new"]);
    assert!(!ok);
}

/// `--allow-downgrade` is orthogonal to the policy group — pairing with any
/// single policy flag must parse successfully. We can't fully test runtime
/// (no initialized project in tempdir, so we expect NotLoaded), but parsing
/// succeeding means the runtime error is the `NotLoaded` one rather than clap's
/// group-conflict error.
#[test]
fn allow_downgrade_orthogonal_to_policy() {
    let tmp = tempfile::tempdir().unwrap();
    for policy_flag in ["--force", "--skip-modified", "--create-new"] {
        let out = Command::new(ark_bin())
            .args([
                "upgrade",
                "-C",
                tmp.path().to_str().unwrap(),
                policy_flag,
                "--allow-downgrade",
            ])
            .output()
            .expect("failed to run ark");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            !stderr.contains("cannot be used with"),
            "flag `{policy_flag}` with --allow-downgrade must not be rejected by clap; \
             got:\n{stderr}"
        );
        assert!(
            stderr.contains("no ark installation found"),
            "expected NotLoaded runtime error; got:\n{stderr}"
        );
    }
}

#[test]
fn upgrade_allow_downgrade_alone_parses() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(ark_bin())
        .args([
            "upgrade",
            "-C",
            tmp.path().to_str().unwrap(),
            "--allow-downgrade",
        ])
        .output()
        .expect("failed to run ark");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stderr.contains("cannot be used with"));
    assert!(stderr.contains("no ark installation found"));
}
