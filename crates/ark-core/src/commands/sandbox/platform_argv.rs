//! `enter --agent`: which CLI to launch in the box, and with which yolo flag.
//!
//! Only platforms with a verified non-interactive ("yolo") flag are
//! supported: Claude (`--dangerously-skip-permissions`) and Codex (`--yolo`).
//! OpenCode has no established yolo flag, so `--agent` rejects it rather than
//! guess. When several platforms are installed, the first in [`PLATFORMS`]
//! order wins unless `--platform <id>` overrides.

use crate::{
    error::{Error, Result},
    layout::Layout,
    platforms::{self, Platform},
    state::manifest::Manifest,
};

/// Returns the launch argv for `platform`'s yolo mode.
///
/// `Err(AgentYoloUnsupported)` for a platform with no defined yolo flag
/// (currently OpenCode).
pub fn yolo_argv(platform: &Platform) -> Result<Vec<String>> {
    let argv = match platform.cli_flag {
        "claude" => vec!["claude", "--dangerously-skip-permissions"],
        "codex" => vec!["codex", "--yolo"],
        _ => {
            return Err(Error::AgentYoloUnsupported {
                platform: platform.id.to_string(),
            });
        }
    };
    Ok(argv.into_iter().map(String::from).collect())
}

/// Selects which installed platform `enter --agent` launches.
///
/// With `override_id`, selects that platform if installed. Otherwise the
/// first installed platform in [`PLATFORMS`] order. `Err(NoAgentPlatform)`
/// when none is installed (or the override is not installed).
pub fn select_platform(layout: &Layout, override_id: Option<&str>) -> Result<&'static Platform> {
    let manifest = Manifest::read(layout.root())?.unwrap_or_default();
    let mut installed = platforms::installed(&manifest);
    let chosen = match override_id {
        Some(id) => installed.find(|p| p.id == id || p.cli_flag == id),
        None => installed.next(),
    };
    chosen.ok_or_else(|| Error::NoAgentPlatform {
        project_root: layout.root().to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platforms::{CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM};

    /// Maps claude/codex to their yolo flags and rejects opencode.
    #[test]
    fn yolo_argv_per_platform() {
        assert_eq!(
            yolo_argv(&CLAUDE_PLATFORM).unwrap(),
            vec!["claude", "--dangerously-skip-permissions"]
        );
        assert_eq!(yolo_argv(&CODEX_PLATFORM).unwrap(), vec!["codex", "--yolo"]);
        assert!(matches!(
            yolo_argv(&OPENCODE_PLATFORM),
            Err(Error::AgentYoloUnsupported { .. })
        ));
    }

    /// Errors with `NoAgentPlatform` when no platform is installed.
    #[test]
    fn select_none_installed_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        assert!(matches!(
            select_platform(&layout, None),
            Err(Error::NoAgentPlatform { .. })
        ));
    }

    /// Picks first-in-`PLATFORMS` order (claude) by default, and honors a
    /// `--platform codex` override, when both are installed.
    #[test]
    fn select_first_in_order_and_override() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let mut manifest = Manifest::new();
        // is_installed checks manifest.files for a path under the platform's
        // dest_dir; seed one file per platform.
        manifest.record_file(".claude/commands/ark/quick.md");
        manifest.record_file(".codex/skills/quick/SKILL.md");
        manifest.write(layout.root()).unwrap();

        assert_eq!(select_platform(&layout, None).unwrap().id, "claude-code");
        assert_eq!(select_platform(&layout, Some("codex")).unwrap().id, "codex");
    }
}
