//! The Docker backend for `ark sandbox`.
//!
//! Drives the `docker` CLI through [`crate::io::docker`]. Owns host→guest path
//! translation (so Windows host paths mount correctly) and the gitdir rewrite
//! that lets the in-box `/workspace/.git` file resolve to the mounted common
//! dir. The image is pulled, never built (the in-repo `sandbox/Dockerfile` is
//! the CI build source).

use std::path::Path;

use crate::{
    commands::sandbox::{
        engine::{
            RemoveOpts, RemoveOutcome, SandboxEngine, SandboxHandle, SandboxRow, SandboxSpec,
        },
        naming::{LABEL_BRANCH, LABEL_SLUG, SandboxNames},
    },
    error::{Error, Result},
    io::docker,
};

/// Guest path the worktree mounts at.
const GUEST_WORKSPACE: &str = "/workspace";
/// Guest path the per-task config volume mounts at — the in-image user's full
/// $HOME. The volume covers everything under `~`: `.claude/` and `.claude.json`,
/// `.codex/` and `.codex.toml`, the agent caches, the shell history, etc. So a
/// one-time `claude /login` inside the box survives `--recreate` and the
/// non-destructive default `ark sandbox rm`. Docker seeds an empty named volume
/// from the image's content at this path on first mount, so the baked-in
/// dotfiles (`.bashrc`, the prompt) carry over too.
const GUEST_CONFIG_DIR: &str = "/home/ark-sandbox";
/// `docker ps --format` template emitting `slug<TAB>branch<TAB>status`, the
/// slug and branch read from the labels `create` sets on every Ark container.
const PS_FORMAT: &str =
    "{{.Label \"ark.sandbox.slug\"}}\t{{.Label \"ark.sandbox.branch\"}}\t{{.Status}}";

/// Docker-backed sandbox engine.
#[derive(Debug, Clone)]
pub struct DockerEngine {
    image: String,
}

impl DockerEngine {
    /// Builds an engine that pulls and runs `image`.
    pub fn new(image: String) -> Self {
        Self { image }
    }
}

impl SandboxEngine for DockerEngine {
    fn id(&self) -> &'static str {
        "docker"
    }

    fn is_available(&self) -> Result<()> {
        if docker::docker_info_ok() {
            Ok(())
        } else {
            Err(Error::SandboxBackendUnavailable {
                engine: "docker".to_string(),
            })
        }
    }

    fn host_user(&self) -> Option<String> {
        // The published image bakes in a fixed `ark-sandbox` user (uid 2000)
        // with its own /etc/passwd entry, so the container runs as that user
        // by default and bash / git / claude all see a real name. Overriding
        // with `--user <host-uid>` would skip the entry and reintroduce
        // "I have no name!" behavior. Users on a custom image who want
        // host-uid ownership of /workspace writes can pass the flag manually
        // via a future `--user` CLI option; v1 ships with the baked-in user.
        None
    }

    fn sandbox_exists(&self, names: &SandboxNames) -> Result<bool> {
        let out = docker::run_docker(
            "ps",
            &[
                "ps",
                "-a",
                "--filter",
                &format!("name=^{}$", names.container),
                "--format",
                "{{.Names}}",
            ],
            Path::new("."),
        )?;
        Ok(out.stdout.lines().any(|l| l.trim() == names.container))
    }

    fn create(&self, spec: &SandboxSpec) -> Result<SandboxHandle> {
        let pull = docker::run_docker("pull", &["pull", &self.image], &spec.workspace)?;
        if !pull.is_success() {
            return Err(Error::ImagePullFailed {
                image: self.image.clone(),
                exit_code: pull.exit_code,
            });
        }

        let args = build_run_args(spec, &self.image);
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let run = docker::run_docker("run", &argv, &spec.workspace)?;
        if !run.is_success() {
            // Best-effort rollback of a half-started container.
            let _ = docker::run_docker("rm", &["rm", "-f", &spec.names.container], &spec.workspace);
            return Err(Error::ContainerStartFailed {
                container: spec.names.container.clone(),
                exit_code: run.exit_code,
            });
        }

        if spec.mount_git {
            rewrite_gitdir(
                &spec.names.container,
                &spec.workspace,
                &spec.git.common_dir,
                &spec.workspace,
            )?;
        }

        Ok(SandboxHandle {
            container: spec.names.container.clone(),
            volume: spec.names.volume.clone(),
            slug: spec.names.slug.clone(),
            branch: spec.names.branch.clone(),
        })
    }

    fn resolve_handle(&self, slug: &str, branch: &str) -> Result<SandboxHandle> {
        let names = crate::commands::sandbox::naming::derive(slug, branch);
        if self.sandbox_exists(&names)? {
            Ok(SandboxHandle {
                container: names.container,
                volume: names.volume,
                slug: names.slug,
                branch: names.branch,
            })
        } else {
            Err(Error::SandboxNotFound {
                slug: slug.to_string(),
            })
        }
    }

    fn resolve_handle_by_slug(&self, slug: &str) -> Result<SandboxHandle> {
        // Look the container up by its slug label, so teardown works after the
        // worktree is gone. Format: `name<TAB>branch-label`.
        let out = docker::run_docker(
            "ps",
            &[
                "ps",
                "-a",
                "--filter",
                &format!("label={LABEL_SLUG}={slug}"),
                "--format",
                "{{.Names}}\t{{.Label \"ark.sandbox.branch\"}}",
            ],
            Path::new("."),
        )?;
        let line = out
            .stdout
            .lines()
            .next()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .ok_or_else(|| Error::SandboxNotFound {
                slug: slug.to_string(),
            })?;
        let mut parts = line.splitn(2, '\t');
        let container = parts.next().unwrap_or("").trim().to_string();
        let branch = parts.next().unwrap_or("").trim().to_string();
        Ok(SandboxHandle {
            volume: format!("{container}-cfg"),
            container,
            slug: slug.to_string(),
            branch,
        })
    }

    fn enter(&self, handle: &SandboxHandle, argv: &[&str]) -> Result<i32> {
        let mut args = vec!["exec", "-it", &handle.container];
        args.extend_from_slice(argv);
        docker::exec_interactive(&args, Path::new("."))
    }

    fn remove(&self, handle: &SandboxHandle, opts: &RemoveOpts) -> Result<RemoveOutcome> {
        let rm = docker::run_docker("rm", &["rm", "-f", &handle.container], Path::new("."))?;
        let container_removed = rm.is_success() && !rm.stdout.trim().is_empty();

        let mut volume_removed = false;
        if !opts.keep_volume {
            let vrm =
                docker::run_docker("volume", &["volume", "rm", &handle.volume], Path::new("."))?;
            volume_removed = vrm.is_success();
            if !volume_removed {
                // Per SPEC C-10, a volume-in-use error warns rather than
                // fails. Distinguish that from the `--keep-volume` intentional
                // skip — without this, both surface as `volume_removed: false`
                // with no signal that the wipe was attempted and refused.
                let detail = vrm.stderr.trim();
                if detail.is_empty() {
                    eprintln!(
                        "ark sandbox: volume `{}` could not be removed (likely still in use)",
                        handle.volume
                    );
                } else {
                    eprintln!(
                        "ark sandbox: volume `{}` could not be removed: {detail}",
                        handle.volume
                    );
                }
            }
        }

        Ok(RemoveOutcome {
            container_removed,
            volume_removed,
        })
    }

    fn list(&self) -> Result<Vec<SandboxRow>> {
        let out = docker::run_docker(
            "ps",
            &[
                "ps",
                "--filter",
                &format!("label={LABEL_SLUG}"),
                "--format",
                PS_FORMAT,
            ],
            Path::new("."),
        )?;
        let mut rows: Vec<SandboxRow> = out.stdout.lines().filter_map(parse_ps_row).collect();
        rows.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(rows)
    }

    fn warmup(&self) -> Result<String> {
        let out = docker::run_docker("pull", &["pull", &self.image], Path::new("."))?;
        if !out.is_success() {
            return Err(Error::ImagePullFailed {
                image: self.image.clone(),
                exit_code: out.exit_code,
            });
        }
        Ok(format!("pulled {}", self.image))
    }
}

/// Parses one `docker ps` row into a [`SandboxRow`].
///
/// The line is `slug<TAB>branch<TAB>status`, slug and branch taken from the
/// `ark.sandbox.{slug,branch}` labels (both set at create time). A row with an
/// empty slug label is skipped. The branch falls back to the slug only if its
/// label is somehow absent.
fn parse_ps_row(line: &str) -> Option<SandboxRow> {
    let mut parts = line.splitn(3, '\t');
    let slug = parts.next()?.trim();
    let branch = parts.next().unwrap_or("").trim();
    let status = parts.next().unwrap_or("").trim();
    if slug.is_empty() {
        return None;
    }
    let branch = if branch.is_empty() { slug } else { branch };
    Some(SandboxRow {
        slug: slug.to_string(),
        branch: branch.to_string(),
        status: status.to_string(),
    })
}

/// Builds the full `docker run` argument vector for `spec`.
///
/// Pure over its inputs so it is unit-testable without a daemon. Forwards only
/// env vars present in `spec.env_passthrough` (already resolved to values),
/// applies `--user` only when `spec.user` is `Some`, and mounts the git common
/// dir rw only when `spec.mount_git`. No `--network` flag — open egress (v1).
pub fn build_run_args(spec: &SandboxSpec, image: &str) -> Vec<String> {
    let mut a: Vec<String> = vec!["run".into(), "-d".into()];

    a.push("--name".into());
    a.push(spec.names.container.clone());

    a.push("--label".into());
    a.push(format!("{LABEL_SLUG}={}", spec.names.slug));
    a.push("--label".into());
    a.push(format!("{LABEL_BRANCH}={}", spec.branch));

    if let Some(user) = &spec.user {
        a.push("--user".into());
        a.push(user.clone());
    }

    // Worktree → /workspace (rw).
    a.push("-v".into());
    a.push(volume_arg(&spec.workspace, GUEST_WORKSPACE, ""));

    // Git common dir (rw) — nests the worktree gitdir, objects, all refs.
    if spec.mount_git {
        let guest = to_guest_path(&spec.git.common_dir);
        a.push("-v".into());
        a.push(volume_arg(&spec.git.common_dir, &guest, ""));
    }

    // Config: either bind-mount the host's config dirs and sidecar files
    // read-write (opt-in via `share_host_config`) so the in-box CLIs inherit
    // the host's login AND can refresh their session against the same files
    // the host CLI uses, or mount a persistent named volume so a one-time
    // in-box login survives recreate. The two are mutually exclusive at the
    // same guest path. Rw is required because tokens need to refresh; the
    // documented cost is that an in-box agent can write to host config.
    if spec.host_config_mounts.is_empty() {
        a.push("-v".into());
        a.push(format!("{}:{GUEST_CONFIG_DIR}", spec.config_volume));
    } else {
        for m in &spec.host_config_mounts {
            a.push("-v".into());
            a.push(volume_arg(&m.host, &m.guest, ""));
        }
    }

    // Make `host.docker.internal` resolve to the host gateway on Linux too
    // (Docker Desktop / Colima already provide it on macOS+Windows). The
    // alias is what a forwarded proxy var points at when the host's proxy
    // listens on 127.0.0.1.
    a.push("--add-host".into());
    a.push("host.docker.internal:host-gateway".into());

    // Forward only the named host vars (values already resolved + present).
    // Proxy vars get their host-loopback addresses rewritten to
    // `host.docker.internal` so they resolve from inside the container.
    for (name, value) in &spec.env_passthrough {
        a.push("-e".into());
        let value = if is_proxy_var(name) {
            rewrite_loopback_for_guest(value)
        } else {
            value.clone()
        };
        a.push(format!("{name}={value}"));
    }

    a.push("-w".into());
    a.push(GUEST_WORKSPACE.into());
    a.push(image.to_string());
    a.push("sleep".into());
    a.push("infinity".into());
    a
}

/// Rewrites the in-box `/workspace/.git` file so its `gitdir:` line points at
/// the mounted common dir's per-worktree path.
///
/// The per-worktree subdir name is derived host-side from the host worktree's
/// `.git` file (a one-line `gitdir: <abs-path>` pointer) and the guest path
/// is built from it. No in-box `git` binary is required — only `sh` and
/// `tee` / shell redirection, which any base image carries.
fn rewrite_gitdir(
    container: &str,
    host_worktree: &Path,
    common_dir: &Path,
    cwd: &Path,
) -> Result<()> {
    let Some(name) = read_worktree_gitdir_name(host_worktree) else {
        // The host worktree's `.git` is missing or unparseable. Leave the
        // in-box `.git` alone; the container starts, in-box git is broken
        // but the rest of the workflow still works.
        return Ok(());
    };
    let guest_common = to_guest_path(common_dir);
    let new_line = format!("gitdir: {guest_common}/worktrees/{name}");
    // Use `printf` + redirection (POSIX, in every shell-bearing image) to
    // overwrite the in-box `.git` pointer in one exec. The `if [ -f ]` guard
    // makes the no-`.git`-file case a no-op rather than an error.
    let script = format!(
        "[ -f {GUEST_WORKSPACE}/.git ] && printf '%s\\n' '{new_line}' > {GUEST_WORKSPACE}/.git || \
         true"
    );
    let _ = docker::run_docker("exec", &["exec", container, "sh", "-c", &script], cwd)?;
    Ok(())
}

/// Reads the per-worktree gitdir basename from the host worktree's `.git` file.
///
/// A linked worktree's `.git` is a one-line file `gitdir: <abs path to
/// `<repo>/.git/worktrees/<name>`>`. Returns `<name>` (the last path segment),
/// or `None` when the file is missing, has no `gitdir:` line, or the path
/// has no basename.
fn read_worktree_gitdir_name(worktree: &Path) -> Option<String> {
    let text = std::fs::read_to_string(worktree.join(".git")).ok()?;
    let path = text
        .lines()
        .find_map(|l| l.strip_prefix("gitdir:").map(str::trim))?;
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
}

/// Builds a `-v` value `host:guest[:mode]`, translating the host path to the
/// engine's guest view (e.g. Windows `C:\x` → `/c/x`).
fn volume_arg(host: &Path, guest: &str, mode: &str) -> String {
    let host = to_engine_host_path(host);
    if mode.is_empty() {
        format!("{host}:{guest}")
    } else {
        format!("{host}:{guest}:{mode}")
    }
}

/// Translates a host path to the form the docker daemon accepts in a `-v` mount.
///
/// On Unix this is the path unchanged. On Windows, `C:\Users\x` → `/c/Users/x`
/// (the Docker Desktop / WSL2 convention).
fn to_engine_host_path(host: &Path) -> String {
    let s = host.to_string_lossy().into_owned();
    translate_windows_path(&s)
}

/// Maps a host path used as a *guest* mount target into a POSIX path.
///
/// Guest paths are always POSIX; a Windows host `.git` path is normalized so
/// the in-guest mount target is valid.
fn to_guest_path(host: &Path) -> String {
    let s = host.to_string_lossy().into_owned();
    translate_windows_path(&s)
}

/// Reports whether `name` is one of the standard outbound-proxy env vars,
/// upper- or lower-case.
fn is_proxy_var(name: &str) -> bool {
    matches!(
        name,
        "HTTP_PROXY"
            | "HTTPS_PROXY"
            | "ALL_PROXY"
            | "NO_PROXY"
            | "http_proxy"
            | "https_proxy"
            | "all_proxy"
            | "no_proxy"
    )
}

/// Rewrites host-loopback addresses in a proxy URL to `host.docker.internal`
/// so the container can reach a proxy the host runs on `127.0.0.1`/`localhost`.
///
/// Non-loopback values are returned unchanged. The rewrite is naive string
/// replacement — sufficient because proxy values are URLs whose host segment
/// is the only place these literals legitimately appear.
fn rewrite_loopback_for_guest(value: &str) -> String {
    value
        .replace("://127.0.0.1", "://host.docker.internal")
        .replace("://localhost", "://host.docker.internal")
}

/// Converts a `C:\a\b` style path to `/c/a/b`; leaves POSIX paths untouched.
fn translate_windows_path(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        let rest = s[2..].replace('\\', "/");
        format!("/{drive}{rest}")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::commands::sandbox::{engine::HostConfigMount, gitmounts::GitMounts, naming};

    fn spec(mount_git: bool, user: Option<String>, env: Vec<(String, String)>) -> SandboxSpec {
        SandboxSpec {
            workspace: PathBuf::from("/home/u/wt"),
            git: GitMounts {
                common_dir: PathBuf::from("/home/u/repo/.git"),
            },
            mount_git,
            branch: "feat/x".into(),
            env_passthrough: env,
            config_volume: "vol-cfg".into(),
            user,
            host_config_mounts: Vec::new(),
            names: naming::derive("task", "feat/x"),
        }
    }

    /// Mounts the worktree at `/workspace`, the git common dir, and the config
    /// volume when `mount_git` is set.
    #[test]
    fn run_args_mounts() {
        let args = build_run_args(&spec(true, None, vec![]), "img:1");
        assert!(args.iter().any(|a| a == "/home/u/wt:/workspace"));
        assert!(
            args.iter()
                .any(|a| a == "/home/u/repo/.git:/home/u/repo/.git")
        );
        assert!(
            args.iter().any(|a| a == "vol-cfg:/home/ark-sandbox"),
            "named volume mounts at $HOME so both .claude and .claude.json persist across recreate"
        );
        assert!(args.iter().any(|a| a == "-w") && args.iter().any(|a| a == "/workspace"));
    }

    /// Replaces the named config volume with read-write host config binds
    /// when `share_host_config` is set, covering both the dir and the sidecar
    /// session file so the in-box CLI can read and refresh the host login.
    #[test]
    fn run_args_share_host_config_swaps_volume_for_rw_binds() {
        let mut s = spec(true, None, vec![]);
        s.host_config_mounts = vec![
            HostConfigMount {
                host: PathBuf::from("/host/.claude"),
                guest: "/home/ark-sandbox/.claude".into(),
            },
            HostConfigMount {
                host: PathBuf::from("/host/.claude.json"),
                guest: "/home/ark-sandbox/.claude.json".into(),
            },
        ];
        let args = build_run_args(&s, "img:1");
        assert!(
            args.iter()
                .any(|a| a == "/host/.claude:/home/ark-sandbox/.claude"),
            "dir mount, rw (no :ro suffix)"
        );
        assert!(
            args.iter()
                .any(|a| a == "/host/.claude.json:/home/ark-sandbox/.claude.json"),
            "sidecar file mount carries the host session"
        );
        // The named volume must NOT also mount the in-image $HOME — host
        // binds would shadow only their target paths anyway, but mounting the
        // volume on top of them would re-introduce the isolated empty state.
        assert!(
            !args.iter().any(|a| a == "vol-cfg:/home/ark-sandbox"),
            "named volume must be suppressed when host_config_mounts is non-empty"
        );
    }

    /// Omits the git mount when `mount_git` is false.
    #[test]
    fn run_args_no_git_mount() {
        let args = build_run_args(&spec(false, None, vec![]), "img:1");
        assert!(
            !args.iter().any(|a| a.contains("/.git:")),
            "git mount must be absent"
        );
    }

    /// Forwards only set env vars and applies no `--network` flag (open egress).
    #[test]
    fn run_args_env_and_no_network() {
        let args = build_run_args(
            &spec(
                true,
                None,
                vec![("ANTHROPIC_API_KEY".into(), "sk-1".into())],
            ),
            "img:1",
        );
        assert!(args.iter().any(|a| a == "ANTHROPIC_API_KEY=sk-1"));
        assert!(
            !args.iter().any(|a| a == "--network"),
            "v1 applies no --network flag"
        );
    }

    /// Includes `--user` when the spec carries one and omits it otherwise.
    #[test]
    fn run_args_user_flag() {
        let with = build_run_args(&spec(true, Some("1000:1000".into()), vec![]), "img:1");
        assert!(with.iter().any(|a| a == "--user") && with.iter().any(|a| a == "1000:1000"));

        let without = build_run_args(&spec(true, None, vec![]), "img:1");
        assert!(!without.iter().any(|a| a == "--user"));
    }

    /// Rewrites host-loopback addresses in proxy URLs to `host.docker.internal`
    /// and leaves non-loopback values and non-proxy vars alone.
    #[test]
    fn proxy_loopback_rewrite() {
        assert_eq!(
            rewrite_loopback_for_guest("http://127.0.0.1:7897"),
            "http://host.docker.internal:7897"
        );
        assert_eq!(
            rewrite_loopback_for_guest("http://localhost:8080"),
            "http://host.docker.internal:8080"
        );
        // Non-loopback proxies pass through.
        assert_eq!(
            rewrite_loopback_for_guest("http://proxy.example.com:3128"),
            "http://proxy.example.com:3128"
        );
        assert!(is_proxy_var("HTTPS_PROXY"));
        assert!(is_proxy_var("http_proxy"));
        assert!(!is_proxy_var("ANTHROPIC_API_KEY"));
    }

    /// Forwards proxy vars with loopback rewritten and adds the
    /// `host.docker.internal` host alias for Linux engines.
    #[test]
    fn run_args_proxy_forwarding_and_host_alias() {
        let args = build_run_args(
            &spec(
                true,
                None,
                vec![
                    ("HTTPS_PROXY".into(), "http://127.0.0.1:7897".into()),
                    ("MY_BUILD_HOST".into(), "127.0.0.1:5000".into()),
                ],
            ),
            "img:1",
        );
        assert!(
            args.iter()
                .any(|a| a == "HTTPS_PROXY=http://host.docker.internal:7897"),
            "proxy var must be rewritten"
        );
        assert!(
            args.iter().any(|a| a == "MY_BUILD_HOST=127.0.0.1:5000"),
            "non-proxy var must not be rewritten"
        );
        assert!(args.iter().any(|a| a == "--add-host"));
        assert!(
            args.iter()
                .any(|a| a == "host.docker.internal:host-gateway")
        );
    }

    /// Translates a Windows host path to a POSIX engine path; POSIX is a no-op.
    #[test]
    fn windows_path_translation() {
        assert_eq!(translate_windows_path("C:\\Users\\x\\wt"), "/c/Users/x/wt");
        assert_eq!(translate_windows_path("/home/u/wt"), "/home/u/wt");
    }

    /// Reads the slug from the slug label, distinct from the branch, and drops
    /// a row whose slug label is blank.
    #[test]
    fn parse_ps_row_reads_slug_label() {
        let row = parse_ps_row("ark-sandbox\tfeat/ark-sandbox\tUp 3 minutes").unwrap();
        assert_eq!(row.slug, "ark-sandbox");
        assert_eq!(row.branch, "feat/ark-sandbox");
        assert_ne!(row.slug, row.branch);
        assert_eq!(row.status, "Up 3 minutes");

        // A row with no slug label (e.g. a stray match) is dropped.
        assert!(parse_ps_row("\t\tUp 1 second").is_none());
    }

    /// Reads the per-worktree gitdir basename host-side, with no dependency
    /// on an in-box `git` binary; returns `None` cleanly when absent.
    #[test]
    fn read_worktree_gitdir_name_parses_dot_git_file() {
        let tmp = tempfile::tempdir().unwrap();
        let wt = tmp.path();
        std::fs::write(
            wt.join(".git"),
            "gitdir: /Users/x/Agent/Ark/.git/worktrees/ark-sandbox\n",
        )
        .unwrap();
        assert_eq!(
            read_worktree_gitdir_name(wt).as_deref(),
            Some("ark-sandbox")
        );

        let empty = tempfile::tempdir().unwrap();
        assert_eq!(read_worktree_gitdir_name(empty.path()), None);
    }

    /// Returns `Ok` when the daemon is reachable and exactly
    /// `SandboxBackendUnavailable` otherwise, never panicking.
    #[test]
    fn is_available_is_well_typed() {
        let engine = DockerEngine::new("img:1".into());
        match engine.is_available() {
            Ok(()) => {}
            Err(Error::SandboxBackendUnavailable { engine }) => assert_eq!(engine, "docker"),
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }
}
