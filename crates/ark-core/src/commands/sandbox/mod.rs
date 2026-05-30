//! `ark sandbox` — run a task's worktree inside a confined container.
//!
//! Opt-in, per task. `create` starts a container with the worktree bind-mounted
//! at `/workspace` and the git common dir mounted rw so the in-box agent can
//! commit to its branch; `enter` opens a shell or the agent CLI; `rm` tears it
//! down; `list` enumerates running boxes. The backend is abstracted behind
//! [`engine::SandboxEngine`]; v1 ships only the Docker engine.
//!
//! The cage is filesystem + process, not network: v1 applies no egress
//! confinement (the container has open outbound internet).

use std::{fmt, path::PathBuf};

/// `[sandbox]` config loading.
pub mod config;
/// The `SandboxEngine` backend abstraction.
pub mod engine;
/// Concrete backends (Docker in v1).
pub mod engines;
/// Git common-dir derivation for mounts.
pub mod gitmounts;
/// Container/volume/label name derivation.
pub mod naming;
/// `enter --agent` platform selection + yolo argv.
pub mod platform_argv;
/// Slug → worktree → `SandboxSpec` resolution.
pub mod resolve;

mod create;
mod enter;
mod list;
mod rm;
mod warmup;

pub use config::SandboxConfig;
pub use create::sandbox_create;
pub use engine::{
    RemoveOpts, RemoveOutcome, SandboxEngine, SandboxHandle, SandboxRow, SandboxSpec, select_engine,
};
pub use enter::sandbox_enter;
pub use list::sandbox_list;
pub use naming::SandboxNames;
pub use rm::sandbox_rm;
pub use warmup::sandbox_warmup;

/// Options for [`sandbox_create`].
#[derive(Debug, Clone)]
pub struct SandboxCreateOptions {
    /// Project root (parent checkout or a worktree).
    pub project_root: PathBuf,
    /// Task slug; `None` resolves the checkout focus.
    pub slug: Option<String>,
    /// Replace an existing sandbox instead of erroring.
    pub recreate: bool,
    /// Force-enable host-config sharing for this `create`, overriding
    /// `[sandbox] share_host_config` only when the config value is `false`.
    /// A `true` config value is honored regardless.
    pub share_host_config: bool,
}

/// Summary of [`sandbox_create`].
#[derive(Debug, Clone)]
pub struct SandboxCreateSummary {
    /// Task slug.
    pub slug: String,
    /// Branch the box is on.
    pub branch: String,
    /// Engine id (`"docker"`).
    pub engine: String,
    /// Container name.
    pub container: String,
    /// Config volume name.
    pub volume: String,
    /// Image that was pulled and run.
    pub image: String,
}

/// Options for [`sandbox_enter`].
#[derive(Debug, Clone)]
pub struct SandboxEnterOptions {
    /// Project root.
    pub project_root: PathBuf,
    /// Task slug; `None` resolves the checkout focus.
    pub slug: Option<String>,
    /// Open a bash shell instead of the agent CLI. Default `false` so the
    /// common case — running the yolo agent in its confined box — is one
    /// command. With no platform installed, the agent path falls back to a
    /// shell anyway (with a stderr warning), so this flag is the explicit
    /// shell opt-in.
    pub shell: bool,
    /// Platform id/flag to launch (overrides first-installed).
    pub platform: Option<String>,
}

/// Summary of [`sandbox_enter`].
#[derive(Debug, Clone)]
pub struct SandboxEnterSummary {
    /// Task slug.
    pub slug: String,
    /// Exit code of the in-box process.
    pub exit_code: i32,
}

/// Options for [`sandbox_rm`].
#[derive(Debug, Clone)]
pub struct SandboxRmOptions {
    /// Project root.
    pub project_root: PathBuf,
    /// Task slug; `None` resolves the checkout focus.
    pub slug: Option<String>,
    /// Keep the config volume. Default is true at the CLI surface (the
    /// `--drop-volume` flag is opt-in) so a routine `ark sandbox rm` never
    /// wipes the persisted login token.
    pub keep_volume: bool,
}

/// Summary of [`sandbox_rm`].
#[derive(Debug, Clone)]
pub struct SandboxRmSummary {
    /// Task slug.
    pub slug: String,
    /// Whether a container was removed.
    pub container_removed: bool,
    /// Whether the volume was removed.
    pub volume_removed: bool,
}

/// Options for [`sandbox_list`].
#[derive(Debug, Clone)]
pub struct SandboxListOptions {
    /// Project root.
    pub project_root: PathBuf,
}

/// Summary of [`sandbox_list`].
#[derive(Debug, Clone)]
pub struct SandboxListSummary {
    /// One row per running Ark sandbox.
    pub rows: Vec<SandboxRow>,
}

/// Options for [`sandbox_warmup`].
#[derive(Debug, Clone)]
pub struct SandboxWarmupOptions {
    /// Project root.
    pub project_root: PathBuf,
}

/// Summary of [`sandbox_warmup`].
#[derive(Debug, Clone)]
pub struct SandboxWarmupSummary {
    /// Engine id that did (or skipped) the warmup.
    pub engine: String,
    /// Free-text description of what was done.
    pub detail: String,
}

impl fmt::Display for SandboxCreateSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "created sandbox `{}` ({}) for `{}` on `{}` from {}",
            self.container, self.engine, self.slug, self.branch, self.image
        )
    }
}

impl fmt::Display for SandboxEnterSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sandbox `{}` exited with code {}",
            self.slug, self.exit_code
        )
    }
}

impl fmt::Display for SandboxRmSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let container = if self.container_removed {
            "removed"
        } else {
            "absent"
        };
        let volume = if self.volume_removed {
            "removed"
        } else {
            "kept"
        };
        write!(
            f,
            "sandbox `{}`: container {container}, volume {volume}",
            self.slug
        )
    }
}

impl fmt::Display for SandboxListSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.rows {
            writeln!(f, "{}\t{}\t{}", row.slug, row.branch, row.status)?;
        }
        Ok(())
    }
}

impl fmt::Display for SandboxWarmupSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.engine, self.detail)
    }
}
