//! `ark sandbox warmup` — warm any per-backend startup cost.

use crate::{
    commands::sandbox::{
        SandboxWarmupOptions, SandboxWarmupSummary, config::SandboxConfig, engine::select_engine,
    },
    error::Result,
    layout::Layout,
};

/// Warms the configured engine ahead of `create`.
///
/// Delegates to [`crate::commands::sandbox::engine::SandboxEngine::warmup`] —
/// for the Docker backend this is `docker pull <image>` so the first `create`
/// is not blocked on the registry fetch. Backends with no meaningful warmup
/// return a descriptive no-op message.
pub fn sandbox_warmup(opts: SandboxWarmupOptions) -> Result<SandboxWarmupSummary> {
    let layout = Layout::new(&opts.project_root);
    let cfg = SandboxConfig::load_or_default(&layout)?;
    cfg.validate()?;
    let engine = select_engine(&cfg)?;
    engine.is_available()?;
    let detail = engine.warmup()?;
    Ok(SandboxWarmupSummary {
        engine: engine.id().to_string(),
        detail,
    })
}
