//! `ark sandbox create` — start a container confining a task's worktree.

use crate::{
    commands::sandbox::{
        SandboxCreateOptions, SandboxCreateSummary,
        config::SandboxConfig,
        engine::{RemoveOpts, select_engine},
        resolve::{build_spec, resolve_task},
    },
    error::{Error, Result},
    layout::Layout,
};

/// Creates a sandbox for the focused (or named) task.
///
/// Probes the backend, resolves the worktree, and (unless one already exists)
/// pulls the image and starts the container. `--recreate` removes a pre-existing
/// sandbox first.
pub fn sandbox_create(opts: SandboxCreateOptions) -> Result<SandboxCreateSummary> {
    let layout = Layout::new(&opts.project_root);
    let mut cfg = SandboxConfig::load_or_default(&layout)?;
    // CLI `--share-host-config` forces sharing on for this `create`; a
    // config `true` is left as-is so the flag never flips it off.
    if opts.share_host_config {
        cfg.share_host_config = true;
    }
    cfg.validate()?;
    let engine = select_engine(&cfg)?;
    engine.is_available()?;

    let task = resolve_task(&layout, opts.slug)?;
    let spec = build_spec(&task, &cfg, engine.as_ref())?;

    if engine.sandbox_exists(&spec.names)? {
        if opts.recreate {
            let handle = engine.resolve_handle(&task.slug, &task.branch)?;
            engine.remove(&handle, &RemoveOpts { keep_volume: true })?;
        } else {
            return Err(Error::SandboxExists {
                slug: task.slug.clone(),
                container: spec.names.container.clone(),
            });
        }
    }

    let handle = engine.create(&spec)?;
    Ok(SandboxCreateSummary {
        slug: handle.slug,
        branch: handle.branch,
        engine: engine.id().to_string(),
        container: handle.container,
        volume: handle.volume,
        image: cfg.image,
    })
}
