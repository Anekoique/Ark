//! The `SandboxEngine` backend abstraction.
//!
//! v1 implements exactly one backend ([`super::engines::docker::DockerEngine`]),
//! but every command goes through this trait so the Docker name never leaks
//! into a command signature and a future OS-native (Seatbelt/bwrap) backend
//! is purely additive. The trait is deliberately narrower than agent-infra's
//! adapter surface: Ark does not manage VM resources or build images, so there
//! is no `startVm`/`syncResources`/`defaultResources`.

use std::path::PathBuf;

use crate::{
    commands::sandbox::{config::SandboxConfig, gitmounts::GitMounts, naming::SandboxNames},
    error::{Error, Result},
};

/// One row of `ark sandbox list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxRow {
    /// Task slug (from the container's `ark.sandbox.slug` label).
    pub slug: String,
    /// Branch (from the container's `ark.sandbox.branch` label).
    pub branch: String,
    /// Engine-reported status string (e.g. `"Up 3 minutes"`).
    pub status: String,
}

/// OS-agnostic intent passed to [`SandboxEngine::create`].
///
/// Host→guest path translation is the engine's responsibility, not the
/// caller's — so this carries host paths and the engine maps them.
#[derive(Debug, Clone)]
pub struct SandboxSpec {
    /// Host worktree directory, mounted rw at `/workspace`.
    pub workspace: PathBuf,
    /// Git directories to mount for in-box git.
    pub git: GitMounts,
    /// Whether to mount git at all.
    pub mount_git: bool,
    /// Branch the box commits to.
    pub branch: String,
    /// Already-resolved host env name→value pairs to forward.
    pub env_passthrough: Vec<(String, String)>,
    /// Named volume backing the agent config dir.
    pub config_volume: String,
    /// `"<uid>:<gid>"` on a rootful Unix daemon; `None` on rootless / non-Unix.
    pub user: Option<String>,
    /// Host config paths (dirs + sidecar session files) that should bind-mount
    /// read-write into the box (e.g. `~/.claude`, `~/.claude.json`, `~/.codex`,
    /// `~/.codex.toml`). When non-empty, these replace the named volume so the
    /// in-box CLIs see and can refresh the host's login and settings. Empty
    /// when `share_host_config = false` or none of the paths exist.
    pub host_config_mounts: Vec<HostConfigMount>,
    /// Container / volume / label names.
    pub names: SandboxNames,
}

/// One host→guest read-write config bind mount (dir or sidecar file).
#[derive(Debug, Clone)]
pub struct HostConfigMount {
    /// Absolute host path that exists (e.g. `/Users/x/.claude`,
    /// `/Users/x/.claude.json`).
    pub host: PathBuf,
    /// Absolute guest path the entry mounts at (e.g. `/home/ark-sandbox/.claude`).
    pub guest: String,
}

/// A handle to a live sandbox, returned by create/resolve and consumed by
/// enter/remove.
#[derive(Debug, Clone)]
pub struct SandboxHandle {
    /// Container name.
    pub container: String,
    /// Config volume name.
    pub volume: String,
    /// Task slug.
    pub slug: String,
    /// Branch.
    pub branch: String,
}

/// Options for [`SandboxEngine::remove`].
#[derive(Debug, Clone)]
pub struct RemoveOpts {
    /// Preserve the config volume (the default; the login token lives there).
    pub keep_volume: bool,
}

/// Outcome of [`SandboxEngine::remove`].
#[derive(Debug, Clone)]
pub struct RemoveOutcome {
    /// Whether a container was actually removed (false if already absent).
    pub container_removed: bool,
    /// Whether the volume was removed (false when kept or already absent).
    pub volume_removed: bool,
}

/// A sandbox backend: create / enter / remove / list a confined workspace.
pub trait SandboxEngine {
    /// Stable id, stored in config and used for backend selection.
    fn id(&self) -> &'static str;

    /// Cheap precondition probe (binary present + backend reachable).
    fn is_available(&self) -> Result<()>;

    /// `"<uid>:<gid>"` to run the container as, or `None` to use the image user.
    fn host_user(&self) -> Option<String>;

    /// Reports whether a sandbox already exists for `names`.
    fn sandbox_exists(&self, names: &SandboxNames) -> Result<bool>;

    /// Starts a confined workspace for `spec`; returns its handle.
    fn create(&self, spec: &SandboxSpec) -> Result<SandboxHandle>;

    /// Resolves the live handle for a slug/branch, or errors if absent.
    fn resolve_handle(&self, slug: &str, branch: &str) -> Result<SandboxHandle>;

    /// Resolves the live handle from the slug alone, via the backend's own
    /// labels — without needing the worktree on disk.
    ///
    /// Lets `enter`/`rm` reach a container after its worktree has been cleaned
    /// up, so teardown never depends on `ark cleanup` ordering.
    fn resolve_handle_by_slug(&self, slug: &str) -> Result<SandboxHandle>;

    /// Runs `argv` inside the box, TTY-attached; returns the child exit code.
    fn enter(&self, handle: &SandboxHandle, argv: &[&str]) -> Result<i32>;

    /// Tears down the sandbox (idempotent).
    fn remove(&self, handle: &SandboxHandle, opts: &RemoveOpts) -> Result<RemoveOutcome>;

    /// Enumerates live Ark sandboxes for this backend.
    fn list(&self) -> Result<Vec<SandboxRow>>;

    /// Warms any per-backend startup cost so the first `create` is fast.
    ///
    /// Returns a short free-text description of what was done, surfaced in
    /// the `Display` summary. The default impl is a no-op for backends that
    /// have no meaningful warmup (e.g. OS-native sandboxes with no image).
    fn warmup(&self) -> Result<String> {
        Ok(format!("nothing to warm for backend `{}`", self.id()))
    }
}

/// Selects the engine named by `cfg.engine`.
///
/// Only `"docker"` is implemented in v1; any other value → `UnknownSandboxEngine`.
pub fn select_engine(cfg: &SandboxConfig) -> Result<Box<dyn SandboxEngine>> {
    match cfg.engine.as_str() {
        "docker" => Ok(Box::new(super::engines::docker::DockerEngine::new(
            cfg.image.clone(),
        ))),
        other => Err(Error::UnknownSandboxEngine {
            value: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rejects a non-docker engine by name.
    #[test]
    fn select_unknown_engine_errors() {
        let cfg = SandboxConfig {
            engine: "podman".into(),
            image: "x:1".into(),
            mount_git: true,
            env_passthrough: vec![],
            share_host_config: false,
        };
        assert!(matches!(
            select_engine(&cfg),
            Err(Error::UnknownSandboxEngine { .. })
        ));
    }

    /// The docker engine is selectable and reports its id.
    #[test]
    fn select_docker_engine() {
        let cfg = SandboxConfig {
            engine: "docker".into(),
            image: "x:1".into(),
            mount_git: true,
            env_passthrough: vec![],
            share_host_config: false,
        };
        assert_eq!(select_engine(&cfg).unwrap().id(), "docker");
    }

    /// Reports a backend-named no-op message for an engine that doesn't
    /// override `warmup`, so a future native backend gets the no-op for free.
    #[test]
    fn warmup_default_is_a_named_noop() {
        struct StubEngine;
        impl SandboxEngine for StubEngine {
            fn id(&self) -> &'static str {
                "native"
            }
            fn is_available(&self) -> Result<()> {
                Ok(())
            }
            fn host_user(&self) -> Option<String> {
                None
            }
            fn sandbox_exists(&self, _: &SandboxNames) -> Result<bool> {
                unreachable!()
            }
            fn create(&self, _: &SandboxSpec) -> Result<SandboxHandle> {
                unreachable!()
            }
            fn resolve_handle(&self, _: &str, _: &str) -> Result<SandboxHandle> {
                unreachable!()
            }
            fn resolve_handle_by_slug(&self, _: &str) -> Result<SandboxHandle> {
                unreachable!()
            }
            fn enter(&self, _: &SandboxHandle, _: &[&str]) -> Result<i32> {
                unreachable!()
            }
            fn remove(&self, _: &SandboxHandle, _: &RemoveOpts) -> Result<RemoveOutcome> {
                unreachable!()
            }
            fn list(&self) -> Result<Vec<SandboxRow>> {
                unreachable!()
            }
        }
        let msg = StubEngine.warmup().unwrap();
        assert!(msg.contains("nothing to warm"));
        assert!(msg.contains("`native`"));
    }
}
