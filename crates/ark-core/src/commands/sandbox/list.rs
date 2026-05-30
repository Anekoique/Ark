//! `ark sandbox list` — enumerate running Ark sandboxes.

use crate::{
    commands::sandbox::{
        SandboxListOptions, SandboxListSummary, config::SandboxConfig, engine::select_engine,
    },
    error::Result,
    layout::Layout,
};

/// Lists running Ark sandboxes for this checkout's configured engine.
///
/// Rows are filtered by the `ark.sandbox.slug` label, so non-Ark containers
/// never appear; empty when none are running.
pub fn sandbox_list(opts: SandboxListOptions) -> Result<SandboxListSummary> {
    let layout = Layout::new(&opts.project_root);
    let cfg = SandboxConfig::load_or_default(&layout)?;
    let engine = select_engine(&cfg)?;
    engine.is_available()?;
    let rows = engine.list()?;
    Ok(SandboxListSummary { rows })
}
