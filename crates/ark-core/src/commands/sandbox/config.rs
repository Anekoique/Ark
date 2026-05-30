//! `[sandbox]` section of `.ark/config.toml`.
//!
//! Declares the backend engine, the published image, whether to mount git,
//! and which host env vars to forward into the box. Owns its own raw-config
//! struct so a malformed `[sandbox]` section surfaces as
//! [`Error::SandboxConfigCorrupt`] rather than another feature's error.
//! Missing file or missing section → defaults (no behavior change).
//!
//! Mirrors `upgrade/strategy.rs`: `#[serde(deny_unknown_fields)]` sits on the
//! *inner* [`SandboxSection`], never the outer [`RawConfig`] — the outer
//! struct shares `config.toml` with `[worktree]`/`[workspace]`/`[upgrade]`,
//! so denying unknown fields there would reject every sibling section.

use serde::Deserialize;

use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Default engine id. The only value v1 implements.
const DEFAULT_ENGINE: &str = "docker";
/// Image registry/repo; the tag is the crate version, appended at load time.
const IMAGE_REPO: &str = "ghcr.io/anekoique/ark-sandbox";

/// Raw shape of `.ark/config.toml` as far as the sandbox feature cares.
///
/// No `deny_unknown_fields` here — the file carries other features' sections.
#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    sandbox: Option<SandboxSection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SandboxSection {
    #[serde(default = "default_engine")]
    engine: String,
    #[serde(default = "default_image")]
    image: String,
    #[serde(default = "default_true")]
    mount_git: bool,
    #[serde(default = "default_env_passthrough")]
    env_passthrough: Vec<String>,
    #[serde(default)]
    share_host_config: bool,
}

impl Default for SandboxSection {
    fn default() -> Self {
        Self {
            engine: default_engine(),
            image: default_image(),
            mount_git: default_true(),
            env_passthrough: default_env_passthrough(),
            share_host_config: false,
        }
    }
}

fn default_engine() -> String {
    DEFAULT_ENGINE.to_string()
}

fn default_image() -> String {
    format!("{IMAGE_REPO}:{}", env!("CARGO_PKG_VERSION"))
}

fn default_true() -> bool {
    true
}

fn default_env_passthrough() -> Vec<String> {
    // `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` power API-key auth without an
    // in-box `claude` or `codex` login; the `*_PROXY` vars carry the host's
    // outbound proxy configuration so users behind a corporate/local proxy
    // get working network in-box without manual setup. Upper-case is the
    // formal spelling and what most documentation shows; users who only set
    // the lower-case variant on their host can add it explicitly.
    [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Resolved `[sandbox]` configuration.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Backend engine id; only `"docker"` is implemented in v1.
    pub engine: String,
    /// Published image reference (`ghcr.io/anekoique/ark-sandbox:<ver>`).
    pub image: String,
    /// Whether to mount the git common dir for in-box git.
    pub mount_git: bool,
    /// Host env var names to forward (only those actually set are passed).
    pub env_passthrough: Vec<String>,
    /// Bind-mount the host's `~/.claude{,.json}` and `~/.codex{,.toml}` into
    /// the box read-write so the in-box CLIs inherit and can refresh the
    /// host's login and settings. Opt-in — defaults to false (isolated,
    /// per-task volume). Read-write is required for the in-box CLI to
    /// refresh its session against the same files the host CLI uses; the
    /// documented cost is that an in-box agent can write to host config.
    pub share_host_config: bool,
}

impl SandboxConfig {
    /// Loads `[sandbox]` from `.ark/config.toml`.
    ///
    /// Returns defaults when the file or the section is absent; corrupt TOML
    /// surfaces as [`Error::SandboxConfigCorrupt`].
    pub fn load_or_default(layout: &Layout) -> Result<Self> {
        let path = layout.config_file();
        let section = match path.read_text_optional()? {
            Some(text) => {
                let raw: RawConfig = toml::from_str(&text)
                    .map_err(|source| Error::SandboxConfigCorrupt { path, source })?;
                raw.sandbox.unwrap_or_default()
            }
            None => SandboxSection::default(),
        };
        Ok(Self {
            engine: section.engine,
            image: section.image,
            mount_git: section.mount_git,
            env_passthrough: section.env_passthrough,
            share_host_config: section.share_host_config,
        })
    }

    /// Validates the resolved config.
    ///
    /// Rejects an empty image reference; engine validity is enforced at
    /// engine selection so the error names the offending value.
    pub fn validate(&self) -> Result<()> {
        if self.image.trim().is_empty() {
            return Err(Error::SandboxConfigInvalid {
                reason: "image must not be empty",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_with_config(text: &str) -> (tempfile::TempDir, Layout) {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        layout.ark_dir().ensure_dir().unwrap();
        layout.config_file().write_bytes(text.as_bytes()).unwrap();
        (tmp, layout)
    }

    /// Loads defaults when the config file is absent.
    #[test]
    fn defaults_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let cfg = SandboxConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.engine, "docker");
        assert!(cfg.mount_git);
        assert!(cfg.env_passthrough.iter().any(|s| s == "ANTHROPIC_API_KEY"));
        assert!(cfg.env_passthrough.iter().any(|s| s == "OPENAI_API_KEY"));
        assert!(cfg.env_passthrough.iter().any(|s| s == "HTTPS_PROXY"));
        assert!(
            !cfg.env_passthrough.iter().any(|s| s == "https_proxy"),
            "uppercase only; users add lowercase explicitly if needed"
        );
        assert!(!cfg.share_host_config, "isolation is the default");
        assert!(cfg.image.starts_with("ghcr.io/anekoique/ark-sandbox:"));
    }

    /// Tolerates a foreign `[worktree]` section without erroring (the outer
    /// struct has no `deny_unknown_fields`); the sandbox section still defaults.
    #[test]
    fn foreign_section_does_not_error() {
        let (_t, layout) = layout_with_config("[worktree]\nbranch_prefix = \"feat\"\n");
        let cfg = SandboxConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.engine, "docker");
    }

    /// Round-trips a fully specified `[sandbox]` section.
    #[test]
    fn round_trips_full_section() {
        let (_t, layout) = layout_with_config(
            "[sandbox]\nengine = \"docker\"\nimage = \"x:1\"\nmount_git = false\nenv_passthrough \
             = [\"A\", \"B\"]\n",
        );
        let cfg = SandboxConfig::load_or_default(&layout).unwrap();
        assert_eq!(cfg.image, "x:1");
        assert!(!cfg.mount_git);
        assert_eq!(cfg.env_passthrough, vec!["A".to_string(), "B".to_string()]);
    }

    /// Rejects an unknown key inside `[sandbox]` (inner `deny_unknown_fields`).
    #[test]
    fn unknown_inner_key_rejected() {
        let (_t, layout) = layout_with_config("[sandbox]\nengin = \"docker\"\n");
        let err = SandboxConfig::load_or_default(&layout).unwrap_err();
        assert!(
            matches!(err, Error::SandboxConfigCorrupt { .. }),
            "got {err:?}"
        );
    }

    /// Rejects an empty image reference in validate.
    #[test]
    fn validate_rejects_empty_image() {
        let cfg = SandboxConfig {
            engine: "docker".into(),
            image: "  ".into(),
            mount_git: true,
            env_passthrough: vec![],
            share_host_config: false,
        };
        assert!(matches!(
            cfg.validate(),
            Err(Error::SandboxConfigInvalid { .. })
        ));
    }
}
