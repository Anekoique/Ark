//! `ark sandbox enter` — launch the agent CLI (or a shell) in the box.

use crate::{
    commands::sandbox::{
        SandboxEnterOptions, SandboxEnterSummary,
        config::SandboxConfig,
        engine::{SandboxEngine, select_engine},
        platform_argv::{select_platform, yolo_argv},
        resolve::resolve_handle_for,
    },
    error::{Error, Result},
    layout::Layout,
};

/// Argv that opens a bash shell inside the box.
fn shell_argv() -> Vec<String> {
    vec!["bash".to_string()]
}

/// Enters the sandbox for the focused (or named) task.
///
/// Defaults to launching the platform's yolo CLI — the sandbox exists to
/// confine an agent, so an agent in the box is the common path. `opts.shell`
/// is the explicit bash escape. When no platform is installed (or it has no
/// yolo argv), the agent path falls back to a bash shell with a stderr
/// warning, so a fresh user can always enter their box. Returns the in-box
/// process exit code. Works even after the worktree is cleaned up (resolves
/// by slug label).
pub fn sandbox_enter(opts: SandboxEnterOptions) -> Result<SandboxEnterSummary> {
    let layout = Layout::new(&opts.project_root);
    let cfg = SandboxConfig::load_or_default(&layout)?;
    let engine = select_engine(&cfg)?;
    engine.is_available()?;

    let handle = resolve_handle_for(&layout, opts.slug, engine.as_ref() as &dyn SandboxEngine)?;
    let owned_argv = resolve_enter_argv(&layout, opts.shell, opts.platform.as_deref());
    let argv: Vec<&str> = owned_argv.iter().map(String::as_str).collect();
    let exit_code = engine.enter(&handle, &argv)?;

    Ok(SandboxEnterSummary {
        slug: handle.slug,
        exit_code,
    })
}

/// Resolves the argv `enter` runs in the box.
///
/// `shell = true` short-circuits to bash. Otherwise tries the agent path and
/// falls back to bash with a stderr warning when no platform is installed or
/// the chosen platform has no yolo argv.
fn resolve_enter_argv(layout: &Layout, shell: bool, platform: Option<&str>) -> Vec<String> {
    if shell {
        return shell_argv();
    }
    match select_platform(layout, platform).and_then(yolo_argv) {
        Ok(argv) => argv,
        Err(e @ (Error::NoAgentPlatform { .. } | Error::AgentYoloUnsupported { .. })) => {
            eprintln!("ark sandbox: {e}; opening a shell instead (pass --shell to silence)");
            shell_argv()
        }
        // Other errors are infrastructure-level (e.g. a corrupt manifest).
        // Still fall back to a shell so the user can enter the box at all, but
        // print the error first so the underlying problem isn't invisible.
        Err(e) => {
            eprintln!("ark sandbox: platform resolution failed ({e}); opening a shell instead");
            shell_argv()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Layout;

    /// `shell = true` short-circuits to bash without touching the platform
    /// registry.
    #[test]
    fn shell_flag_returns_bash_unconditionally() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert_eq!(resolve_enter_argv(&layout, true, None), shell_argv());
    }

    /// Falls back to bash when no platform is installed, instead of erroring.
    #[test]
    fn no_platform_falls_back_to_shell() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert_eq!(resolve_enter_argv(&layout, false, None), shell_argv());
    }
}
