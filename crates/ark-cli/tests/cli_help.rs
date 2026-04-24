//! Integration tests for CLI help output — verify the `ark agent` namespace
//! contract (hidden from top-level help, still discoverable via `ark agent --help`).

use std::process::Command;

fn ark_bin() -> std::path::PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo when running integration tests.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_ark"))
}

fn run(args: &[&str]) -> String {
    let out = Command::new(ark_bin())
        .args(args)
        .output()
        .expect("failed to run ark");
    let mut s = String::from_utf8(out.stdout).expect("stdout is utf-8");
    s.push_str(&String::from_utf8(out.stderr).expect("stderr is utf-8"));
    s
}

#[test]
fn top_level_help_does_not_mention_agent() {
    let out = run(&["--help"]);
    assert!(
        !out.contains("  agent"),
        "`ark --help` must NOT list `agent`; got:\n{out}"
    );
}

#[test]
fn agent_help_includes_stability_banner() {
    let out = run(&["agent", "--help"]);
    assert!(
        out.contains("Not covered by semver"),
        "`ark agent --help` must contain the stability banner; got:\n{out}"
    );
}

#[test]
fn agent_help_lists_children() {
    let out = run(&["agent", "--help"]);
    for child in ["task", "spec"] {
        assert!(
            out.contains(child),
            "`ark agent --help` must list `{child}`; got:\n{out}"
        );
    }
}
