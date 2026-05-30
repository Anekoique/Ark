//! Sanctioned `docker` subprocess spawns.
//!
//! The Docker-specific sibling of [`crate::io::git`]: the only place in the
//! crate that calls `Command::new("docker")`. The `ark sandbox` engine routes
//! every container operation through here so the no-`Command::new`-in-commands
//! invariant (asserted by the `commands_no_bare_command_new` test) holds.
//!
//! Three entry points: [`run_docker`] captures output for parsing,
//! [`exec_interactive`] inherits stdio so an in-box agent CLI keeps its TTY
//! (mirroring [`crate::io::git::run_shell`]), and [`docker_info_ok`] probes
//! the daemon. Spawn failure (binary missing, permissions) is a hard
//! [`Error::DockerSpawn`]; a completed non-zero exit is returned for the
//! caller to interpret.

use std::{path::Path, process::Command};

use crate::error::{Error, Result};

/// Captured output of a `docker` invocation.
#[derive(Debug, Clone)]
pub struct DockerOutput {
    /// Process exit code, or `-1` when the process terminated without one.
    pub exit_code: i32,
    /// Captured standard output.
    pub stdout: String,
    /// Captured for diagnostic logging — surfaced by callers when a non-zero
    /// exit is tolerated (e.g. volume-in-use during `sandbox rm`) so the
    /// underlying daemon message reaches the user.
    pub stderr: String,
}

impl DockerOutput {
    /// Reports whether the command exited with status code zero.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Runs `docker <args...>` with `cwd` as the working directory, capturing output.
///
/// Returns [`Ok`] for any completed run including non-zero exits; spawn
/// failure yields [`Error::DockerSpawn`]. `op` names the subcommand for the
/// spawn-error message (e.g. `"run"`, `"pull"`).
pub fn run_docker(op: &'static str, args: &[&str], cwd: &Path) -> Result<DockerOutput> {
    let output = Command::new("docker")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|source| Error::DockerSpawn { op, source })?;
    Ok(DockerOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Runs `docker <args...>` with inherited stdio, returning the exit code.
///
/// Used for `docker exec -it` so the in-box shell or agent CLI keeps a live
/// TTY. Spawn failure yields [`Error::DockerSpawn`].
pub fn exec_interactive(args: &[&str], cwd: &Path) -> Result<i32> {
    let status = Command::new("docker")
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|source| Error::DockerSpawn { op: "exec", source })?;
    Ok(status.code().unwrap_or(-1))
}

/// Reports whether `docker info` exits zero — i.e. the daemon is reachable.
///
/// Stronger than a bare spawn probe: a present `docker` binary with a dead
/// daemon returns non-zero, which this treats as unavailable.
pub fn docker_info_ok() -> bool {
    Command::new("docker")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `run_docker` against a missing/odd subcommand never panics; either the
    /// daemon answers (non-zero) or the binary is absent (`DockerSpawn`).
    #[test]
    fn run_docker_unknown_flag_does_not_panic() {
        let tmp = tempfile::tempdir().unwrap();
        match run_docker("test", &["--not-a-real-flag"], tmp.path()) {
            Ok(out) => assert!(!out.is_success()),
            Err(Error::DockerSpawn { op, .. }) => assert_eq!(op, "test"),
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }

    /// The probe degrades to `false` rather than erroring when docker is
    /// unavailable; on a docker-equipped host it simply reports daemon state.
    #[test]
    fn probes_never_panic() {
        let _ = docker_info_ok();
    }
}
