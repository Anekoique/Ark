//! `ark sandbox rm` — stop and remove a sandbox container.

use crate::{
    commands::sandbox::{
        SandboxRmOptions, SandboxRmSummary,
        config::SandboxConfig,
        engine::{RemoveOpts, SandboxEngine, select_engine},
        resolve::resolve_handle_for,
    },
    error::Result,
    layout::Layout,
};

/// Removes the sandbox for the focused (or named) task.
///
/// Preserves the config volume by default so the persisted login token
/// survives a normal teardown; `--drop-volume` is the opt-in wipe. An in-use
/// volume warns (reported as `volume_removed: false`) rather than failing.
/// Works even after the worktree is cleaned up (resolves by slug label).
pub fn sandbox_rm(opts: SandboxRmOptions) -> Result<SandboxRmSummary> {
    let layout = Layout::new(&opts.project_root);
    let cfg = SandboxConfig::load_or_default(&layout)?;
    let engine = select_engine(&cfg)?;
    engine.is_available()?;

    let handle = resolve_handle_for(&layout, opts.slug, engine.as_ref() as &dyn SandboxEngine)?;
    let outcome = engine.remove(
        &handle,
        &RemoveOpts {
            keep_volume: opts.keep_volume,
        },
    )?;

    Ok(SandboxRmSummary {
        slug: handle.slug,
        container_removed: outcome.container_removed,
        volume_removed: outcome.volume_removed,
    })
}
