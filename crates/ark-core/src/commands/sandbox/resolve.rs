//! Turning command options into a resolved [`SandboxSpec`].
//!
//! Resolves the focus/named slug to its worktree (read-only, via the worktree
//! feature's discovery helper), derives the git common dir, picks up the host
//! values of the configured env-passthrough vars, and assembles the spec the
//! engine consumes. Creates and removes nothing.

use std::path::PathBuf;

use crate::{
    commands::{
        agent::{
            state::TaskToml,
            task::worktree::{config::WorktreeConfig, discovery::find_worktree_for_slug},
        },
        sandbox::{
            config::SandboxConfig,
            engine::{HostConfigMount, SandboxEngine, SandboxHandle, SandboxSpec},
            gitmounts::derive_git_mounts,
            naming,
        },
    },
    error::{Error, Result},
    layout::Layout,
    state::checkout::resolve_focus_slug,
};

/// A resolved worktree target: its path plus the branch its task is on.
pub struct ResolvedTask {
    /// Absolute worktree path.
    pub worktree: PathBuf,
    /// Branch the worktree (and its sandbox) is on.
    pub branch: String,
    /// Task slug.
    pub slug: String,
}

/// Resolves `slug` (or the checkout focus) to its worktree + branch.
///
/// `Err(NoFocus)` when no slug and no focus; `Err(WorktreeNotFound)` when the
/// slug names no worktree-backed task.
pub fn resolve_task(layout: &Layout, slug: Option<String>) -> Result<ResolvedTask> {
    let slug = resolve_focus_slug(layout, slug)?;
    resolve_task_for_slug(layout, &slug)
}

/// Resolves an already-determined slug to its worktree + branch (read-only).
///
/// Two resolution modes. **Local**: when invoked from inside the worktree
/// itself (the slug's `task.toml` lives in this checkout and carries a
/// `worktree_path`), the current root *is* the worktree — resolve directly,
/// without walking any parent. **Parent**: otherwise (run from the main
/// checkout, typically with `--slug`), walk `git worktree list` for the slug.
fn resolve_task_for_slug(layout: &Layout, slug: &str) -> Result<ResolvedTask> {
    if let Some(task) = resolve_local_worktree(layout, slug) {
        return Ok(task);
    }
    let wt_cfg = WorktreeConfig::load_or_default(layout)?;
    let worktrees_dir = wt_cfg.resolve_worktrees_dir(layout);
    let found = find_worktree_for_slug(layout.root(), &worktrees_dir, slug)?;
    let (worktree, toml) = found.ok_or_else(|| Error::WorktreeNotFound {
        slug: slug.to_string(),
    })?;
    let branch = toml.branch.ok_or_else(|| Error::WorktreeNotFound {
        slug: slug.to_string(),
    })?;
    Ok(ResolvedTask {
        worktree,
        branch,
        slug: slug.to_string(),
    })
}

/// Resolves the slug against the current checkout when it *is* the worktree.
///
/// Returns `Some` only when the slug's `task.toml` exists locally and carries
/// both a `branch` and a `worktree_path` — i.e. this checkout is a worktree
/// running its own task. The worktree path is the current root, since a
/// worktree never nests its own `worktree_path` under itself.
fn resolve_local_worktree(layout: &Layout, slug: &str) -> Option<ResolvedTask> {
    let task_dir = layout.task_dir(slug);
    let toml = TaskToml::load(&task_dir).ok()?;
    let branch = toml.branch?;
    toml.worktree_path?;
    Some(ResolvedTask {
        worktree: layout.root().to_path_buf(),
        branch,
        slug: slug.to_string(),
    })
}

/// Resolves a live sandbox handle for the focus/named slug, for `enter`/`rm`.
///
/// Tries the worktree path first (so the branch comes from `task.toml`); if the
/// worktree has been cleaned up (`WorktreeNotFound`), falls back to resolving
/// the container by its slug label, so teardown never depends on whether
/// `ark cleanup` ran first.
pub fn resolve_handle_for(
    layout: &Layout,
    slug: Option<String>,
    engine: &dyn SandboxEngine,
) -> Result<SandboxHandle> {
    let slug = resolve_focus_slug(layout, slug)?;
    match resolve_task_for_slug(layout, &slug) {
        Ok(task) => engine.resolve_handle(&task.slug, &task.branch),
        Err(Error::WorktreeNotFound { .. }) => engine.resolve_handle_by_slug(&slug),
        Err(e) => Err(e),
    }
}

/// Builds the [`SandboxSpec`] for a resolved task under `cfg`.
///
/// `engine` supplies the `--user` decision (rootful vs rootless). Only host
/// env vars in `cfg.env_passthrough` that are actually set are carried.
pub fn build_spec(
    task: &ResolvedTask,
    cfg: &SandboxConfig,
    engine: &dyn SandboxEngine,
) -> Result<SandboxSpec> {
    let git = derive_git_mounts(&task.worktree)?;
    let names = naming::derive(&task.slug, &task.branch);
    let env_passthrough = cfg
        .env_passthrough
        .iter()
        .filter_map(|name| std::env::var(name).ok().map(|v| (name.clone(), v)))
        .collect();
    let host_config_mounts = if cfg.share_host_config {
        resolve_host_config_mounts()
    } else {
        Vec::new()
    };
    Ok(SandboxSpec {
        workspace: task.worktree.clone(),
        git,
        mount_git: cfg.mount_git,
        branch: task.branch.clone(),
        env_passthrough,
        config_volume: names.volume.clone(),
        user: engine.host_user(),
        host_config_mounts,
        names,
    })
}

/// Resolves the host config paths the in-box agent CLIs and git read.
///
/// Each agent stores two things in the host's home: a config *directory*
/// (`~/.claude`, `~/.codex`) holding settings, MCP config, and caches, and a
/// sidecar *file* (`~/.claude.json`, `~/.codex.toml`) holding session/auth
/// state. Both must be mounted for `claude` / `codex` inside the box to see
/// the host's login. `~/.gitconfig` (and the XDG variant `~/.config/git`)
/// carries the host's git identity (`user.name`, `user.email`) so in-box
/// commits don't fail with "tell me who you are". Returns one entry per path
/// that exists on the host; missing paths are skipped silently.
fn resolve_host_config_mounts() -> Vec<HostConfigMount> {
    const PAIRS: &[(&str, &str)] = &[
        (".claude", "/home/ark-sandbox/.claude"),
        (".claude.json", "/home/ark-sandbox/.claude.json"),
        (".codex", "/home/ark-sandbox/.codex"),
        (".codex.toml", "/home/ark-sandbox/.codex.toml"),
        (".gitconfig", "/home/ark-sandbox/.gitconfig"),
        (".config/git", "/home/ark-sandbox/.config/git"),
    ];
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return Vec::new();
    };
    PAIRS
        .iter()
        .filter_map(|(host_sub, guest)| {
            let host = home.join(host_sub);
            // is_dir OR is_file: the dir mounts cover settings/state, the
            // file mounts (e.g. .claude.json, .gitconfig) carry session and
            // identity. All exist iff the user has actually used the
            // respective tool on the host before.
            host.exists().then(|| HostConfigMount {
                host,
                guest: guest.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sandbox::{
        engine::{RemoveOpts, RemoveOutcome, SandboxRow},
        naming::SandboxNames,
    };

    /// Stub engine recording which handle-resolution path was taken.
    struct StubEngine;

    impl SandboxEngine for StubEngine {
        fn id(&self) -> &'static str {
            "stub"
        }
        fn is_available(&self) -> Result<()> {
            Ok(())
        }
        fn host_user(&self) -> Option<String> {
            None
        }
        fn sandbox_exists(&self, _: &SandboxNames) -> Result<bool> {
            Ok(false)
        }
        fn create(&self, _: &SandboxSpec) -> Result<SandboxHandle> {
            unreachable!()
        }
        fn resolve_handle(&self, slug: &str, _: &str) -> Result<SandboxHandle> {
            panic!("resolve_handle should not run when the worktree is absent: {slug}")
        }
        fn resolve_handle_by_slug(&self, slug: &str) -> Result<SandboxHandle> {
            Ok(SandboxHandle {
                container: format!("c-{slug}"),
                volume: format!("c-{slug}-cfg"),
                slug: slug.to_string(),
                branch: "recovered".to_string(),
            })
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

    /// Resolves directly to the current root when invoked from inside the
    /// worktree (the slug's `task.toml` is local and carries a worktree_path),
    /// without walking any parent checkout.
    #[test]
    fn resolve_local_worktree_uses_current_root() {
        use crate::{
            commands::agent::state::{Phase, Tier},
            io::PathExt,
        };

        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let dir = layout.task_dir("inbox");
        dir.ensure_dir().unwrap();
        let now = chrono::Utc::now();
        TaskToml {
            id: "inbox".into(),
            title: "inbox".into(),
            tier: Tier::Deep,
            phase: Phase::Execute,
            created_at: now,
            updated_at: now,
            archived_at: None,
            committed_at: None,
            branch: Some("feat/inbox".into()),
            worktree_path: Some(".ark/worktrees/feat/inbox".into()),
            base_branch: Some("main".into()),
            start_head: None,
            journal_path: None,
        }
        .save(&dir)
        .unwrap();

        let task = super::resolve_task(&layout, Some("inbox".into())).unwrap();
        assert_eq!(task.worktree, layout.root());
        assert_eq!(task.branch, "feat/inbox");
        assert_eq!(task.slug, "inbox");
    }

    /// Falls back to the by-slug label path when no worktree is on disk,
    /// instead of failing with `WorktreeNotFound`.
    #[test]
    fn resolve_handle_for_falls_back_when_worktree_gone() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let handle = resolve_handle_for(
            &layout,
            Some("orphan".into()),
            &StubEngine as &dyn SandboxEngine,
        )
        .unwrap();
        assert_eq!(handle.slug, "orphan");
        assert_eq!(handle.container, "c-orphan");
        assert_eq!(handle.branch, "recovered");
    }
}
