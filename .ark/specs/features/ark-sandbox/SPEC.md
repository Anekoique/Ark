
[**Goals**]

- G-1: `ark sandbox create` confines a task's worktree in a container at `/workspace`.
- G-2: `ark sandbox enter` opens a shell in the box, or the agent CLI with `--agent`.
- G-3: `ark sandbox rm` tears down the sandbox, preserving the named volume by default.
- G-4: `ark sandbox list` enumerates running Ark sandboxes, one row each.
- G-5: `ark sandbox` persists a one-time in-box login across container recreate.

[**Non-goals**]

- NG-1: No worktree creation or teardown; sandbox reuses `task new --worktree` and leaves cleanup to `ark cleanup`.
- NG-2: No credential reconciliation, keychain extraction, GPG/SSH/dotfiles bind, or host-config sync beyond the named volume + env pass-through.
- NG-3: No network egress confinement in v1; the container has open outbound internet, so the cage is filesystem + process, not network.
- NG-4: One backend (`docker`) ships in v1; the `SandboxEngine` trait exists but only `DockerEngine` is implemented.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                  (Command::Sandbox(SandboxArgs); SandboxCommand
│                                          {Create, Enter, Rm, List}; dispatch mirrors Cleanup)
└── ark-core/src/
    ├── lib.rs                            (re-exports public sandbox API + resolve_focus_slug)
    ├── error.rs                          (new variants — see Data Structure)
    ├── state/checkout/                   (resolve_focus_slug added beside load_state)
    ├── io/
    │   ├── git.rs                        (unchanged; run_git derives git dirs)
    │   └── docker.rs                     (NEW — Docker-specific spawn site, like io/git.rs:
    │                                       run_docker(args, cwd) -> DockerOutput;
    │                                       exec_interactive(args, cwd) -> i32 via .status();
    │                                       docker_info_ok(); rootful detection; the only
    │                                       Command::new("docker"))
    └── commands/
        └── sandbox/                      (NEW)
            ├── mod.rs                     (public types + dispatch; re-exports)
            ├── engine.rs                  (SandboxEngine trait + SandboxSpec/SandboxHandle/
            │                               GitMounts/RemoveOpts/RemoveOutcome; select_engine)
            ├── engines/
            │   └── docker.rs              (impl SandboxEngine for DockerEngine; host→guest path
            │                               translation; build_run_args; rewrite_gitdir; io::docker)
            ├── config.rs                  (SandboxConfig — [sandbox]: engine, image,
            │                               mount_git, env_passthrough)
            ├── naming.rs                  (container/volume/label derivation: sanitized branch + hash8)
            ├── gitmounts.rs               (derive_git_mounts: rev-parse --git-common-dir)
            ├── resolve.rs                 (resolve_focus_slug → find_worktree_for_slug → SandboxSpec)
            ├── platform_argv.rs           (yolo-argv map + multi-platform selection rule)
            ├── create.rs   enter.rs   rm.rs   list.rs
templates/
└── ark/
    └── config.toml                       (gains a commented [sandbox] section default)
sandbox/                                   (renamed from the old `docker/` intent)
└── Dockerfile                            (CI build source for ghcr.io/anekoique/ark-sandbox:<ver>;
                                            runs as an arbitrary-uid-tolerant user; NOT
                                            include_dir!-embedded — create pulls, never builds)
```

Module coupling (one-way): `create`/`enter`/`rm`/`list` → `engine::select_engine` + `resolve` + `config` + `naming` + `platform_argv`. `resolve` → `resolve_focus_slug` + `worktree::discovery::find_worktree_for_slug` + `gitmounts::derive_git_mounts` (read-only). `engines/docker` → `io::docker`. `naming` + `gitmounts` are leaves. No sandbox module writes under `.ark/`.

Call graph for `ark sandbox create`:

```
sandbox_create(opts)
  ├── cfg = SandboxConfig::load_or_default(layout); cfg.validate()  → SandboxConfigInvalid
  ├── engine = select_engine(&cfg)                                  → UnknownSandboxEngine
  ├── engine.is_available()                                         → SandboxBackendUnavailable
  │     (DockerEngine: io::docker::docker_info_ok())
  ├── slug = resolve_focus_slug(layout, opts.slug)                  → NoFocus
  ├── (wt, toml) = find_worktree_for_slug(root, worktrees_dir, &slug) → WorktreeNotFound
  ├── git = derive_git_mounts(&wt)        (rev-parse --path-format=absolute --git-common-dir)
  ├── names = naming::derive(&slug, &toml.branch)                   (container+volume+label, hash8)
  ├── if engine.sandbox_exists(&names):                             → SandboxExists (unless --recreate)
  ├── spec = SandboxSpec { workspace: wt, git, mount_git: cfg.mount_git, env_passthrough (resolved),
  │                        config_volume: names.volume, user: engine.host_user(), names }
  └── engine.create(&spec)
        DockerEngine.create:
          ├── run_docker(["pull", &cfg.image])                      → ImagePullFailed
          ├── args = build_run_args(&spec)  (path-translated -v mounts; --user when rootful; labels; -e; -w; -d)
          ├── run_docker(args)                                      → ContainerStartFailed (best-effort rm -f)
          └── rewrite_gitdir(container, &spec.git)  (exec: write /workspace/.git → guest gitdir path)
  └── return SandboxCreateSummary { slug, branch, engine, container, volume, image }
```

`derive_git_mounts(wt)` returns `GitMounts { common_dir }` (absolute host path of `<repo>/.git`). The worktree's own gitdir (`<repo>/.git/worktrees/<branch>/`, holding HEAD/index/logs) is nested inside `common_dir`, so a single rw mount of `common_dir` covers HEAD, index, the object store, and `refs/heads/<branch>` — everything an in-box commit writes. `rewrite_gitdir` rewrites the in-box `/workspace/.git` file's `gitdir:` line to the guest path so git resolves.

Call graphs for `enter` / `rm` / `list`:

```
sandbox_enter(opts):  select+available → handle = engine.resolve_handle(slug-or-focus, branch) → SandboxNotFound
  → argv = if opts.agent { platform_argv::yolo_argv(select_platform(layout, opts.platform)) } else { ["bash"] }
  → engine.enter(&handle, argv)   (exec_interactive(["exec","-it",..]))

sandbox_rm(opts):     select+available → handle → engine.remove(&handle, RemoveOpts{keep_volume})
  → run_docker(["rm","-f",container]) (idempotent) → if !keep_volume: run_docker(["volume","rm",volume]) (in-use → warn)

sandbox_list(opts):   select+available → engine.list()
  → run_docker(["ps","--filter","label=ark.sandbox.slug","--format",..]) → sort by slug
```

[**Data Structure**]

```rust
// ark-core/src/io/docker.rs
#[derive(Debug, Clone)]
pub struct DockerOutput { pub exit_code: i32, pub stdout: String, pub stderr: String }

// ark-core/src/commands/sandbox/engine.rs
pub trait SandboxEngine {
    fn id(&self) -> &'static str;
    fn is_available(&self) -> Result<()>;                 // Err = SandboxBackendUnavailable
    fn host_user(&self) -> Option<String>;                // Some("uid:gid") on rootful Unix; None otherwise
    fn sandbox_exists(&self, names: &SandboxNames) -> Result<bool>;
    fn create(&self, spec: &SandboxSpec) -> Result<SandboxHandle>;
    fn resolve_handle(&self, slug: &str, branch: &str) -> Result<SandboxHandle>;
    fn enter(&self, h: &SandboxHandle, argv: &[&str]) -> Result<i32>;
    fn remove(&self, h: &SandboxHandle, opts: &RemoveOpts) -> Result<RemoveOutcome>;
    fn list(&self) -> Result<Vec<SandboxRow>>;
}

#[derive(Debug, Clone)]
pub struct GitMounts {
    pub common_dir: PathBuf,   // <repo>/.git — rw; nests the worktree gitdir, objects, all refs
}

#[derive(Debug, Clone)]
pub struct SandboxSpec {
    pub workspace: PathBuf,                       // host worktree → /workspace rw
    pub git: GitMounts,
    pub mount_git: bool,
    pub branch: String,
    pub env_passthrough: Vec<(String, String)>,   // resolved host name→value
    pub config_volume: String,                    // named volume for agent config dir
    pub user: Option<String>,                     // "<uid>:<gid>" on rootful Unix; None otherwise
    pub names: SandboxNames,
}

#[derive(Debug, Clone)]
pub struct SandboxHandle { pub container: String, pub volume: String, pub slug: String, pub branch: String }
#[derive(Debug, Clone)]
pub struct RemoveOpts { pub keep_volume: bool }
#[derive(Debug, Clone)]
pub struct RemoveOutcome { pub container_removed: bool, pub volume_removed: bool }

// ark-core/src/commands/sandbox/config.rs
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub engine: String,                 // default "docker"
    pub image: String,                  // default "ghcr.io/anekoique/ark-sandbox:<crate-ver>"
    pub mount_git: bool,                // default true
    pub env_passthrough: Vec<String>,   // default ["ANTHROPIC_API_KEY"]
}
// Outer RawConfig { sandbox: Option<SandboxSection> } — NO deny_unknown_fields (shares config.toml
// with [worktree]/[workspace]/[upgrade]). Inner #[serde(deny_unknown_fields)] struct SandboxSection.

// ark-core/src/commands/sandbox/naming.rs
#[derive(Debug, Clone)]
pub struct SandboxNames {
    pub container: String,   // "ark-sandbox-<sanitized-branch>-<hash8>"
    pub volume: String,      // "<container>-cfg"
    pub slug: String,
    pub branch: String,
}

// ark-core/src/commands/sandbox/mod.rs (options + summaries; all summaries impl Display)
#[derive(Debug, Clone)]
pub struct SandboxCreateOptions { pub project_root: PathBuf, pub slug: Option<String>, pub recreate: bool }
#[derive(Debug, Clone)]
pub struct SandboxCreateSummary {
    pub slug: String, pub branch: String, pub engine: String,
    pub container: String, pub volume: String, pub image: String,
}

#[derive(Debug, Clone)]
pub struct SandboxEnterOptions {
    pub project_root: PathBuf, pub slug: Option<String>, pub agent: bool, pub platform: Option<String>,
}
#[derive(Debug, Clone)]
pub struct SandboxEnterSummary { pub slug: String, pub exit_code: i32 }

#[derive(Debug, Clone)]
pub struct SandboxRmOptions { pub project_root: PathBuf, pub slug: Option<String>, pub keep_volume: bool }
#[derive(Debug, Clone)]
pub struct SandboxRmSummary { pub slug: String, pub container_removed: bool, pub volume_removed: bool }

#[derive(Debug, Clone)]
pub struct SandboxListOptions { pub project_root: PathBuf }
#[derive(Debug, Clone)]
pub struct SandboxListSummary { pub rows: Vec<SandboxRow> }
#[derive(Debug, Clone)]
pub struct SandboxRow { pub slug: String, pub branch: String, pub status: String }

// ark-core/src/state/checkout/  (new public fn)
pub fn resolve_focus_slug(layout: &Layout, slug: Option<String>) -> Result<String>;

// ark-core/src/error.rs (additions; #[error] templates E-9 compliant, lowercase, no trailing punct)
#[error("docker {op} failed to spawn: {source}")]
Error::DockerSpawn { op: &'static str, #[source] source: std::io::Error },
#[error("sandbox backend `{engine}` is unavailable")]
Error::SandboxBackendUnavailable { engine: String },
#[error("unknown sandbox engine `{value}`")]
Error::UnknownSandboxEngine { value: String },
#[error("sandbox config `{path}` is corrupt: {source}")]
Error::SandboxConfigCorrupt { path: PathBuf, #[source] source: toml::de::Error },
#[error("invalid sandbox config: {reason}")]
Error::SandboxConfigInvalid { reason: &'static str },
#[error("sandbox for `{slug}` already exists ({container})")]
Error::SandboxExists { slug: String, container: String },
#[error("no sandbox found for `{slug}`")]
Error::SandboxNotFound { slug: String },
#[error("failed to pull image `{image}` (exit {exit_code})")]
Error::ImagePullFailed { image: String, exit_code: i32 },
#[error("failed to start container `{container}` (exit {exit_code})")]
Error::ContainerStartFailed { container: String, exit_code: i32 },
#[error("no agent platform installed for `--agent`")]
Error::NoAgentPlatform { project_root: PathBuf },
#[error("platform `{platform}` has no supported yolo mode for `--agent`")]
Error::AgentYoloUnsupported { platform: String },
```

[**API Surface**]

```rust
// io/docker.rs
pub fn docker_info_ok() -> bool;
pub fn run_docker(args: &[&str], cwd: &Path) -> Result<DockerOutput>;
pub fn exec_interactive(args: &[&str], cwd: &Path) -> Result<i32>;

// state/checkout/
pub fn resolve_focus_slug(layout: &Layout, slug: Option<String>) -> Result<String>;

// commands/sandbox/
pub fn select_engine(cfg: &SandboxConfig) -> Result<Box<dyn SandboxEngine>>;
pub fn sandbox_create(opts: SandboxCreateOptions) -> Result<SandboxCreateSummary>;
pub fn sandbox_enter(opts: SandboxEnterOptions)   -> Result<SandboxEnterSummary>;
pub fn sandbox_rm(opts: SandboxRmOptions)         -> Result<SandboxRmSummary>;
pub fn sandbox_list(opts: SandboxListOptions)     -> Result<SandboxListSummary>;

impl SandboxConfig {
    pub fn load_or_default(layout: &Layout) -> Result<Self>;
    pub fn validate(&self) -> Result<()>;
}

// CLI (ark-cli/src/main.rs) — mirrors CleanupArgs; each *CliArgs flattens TargetArgs.
#[derive(Subcommand)]
enum SandboxCommand {
    Create(SandboxCreateCliArgs),  // --slug?, --recreate
    Enter(SandboxEnterCliArgs),    // --slug?, --agent, --platform?
    Rm(SandboxRmCliArgs),          // --slug?, --keep-volume
    List(SandboxListCliArgs),
}
```

[**Constraints**]

- C-1: All `docker` invocations route through `io::docker`; `Command::new` may NOT appear under `commands/sandbox/` (extends the `commands_no_bare_command_new` SOURCES list).
- C-2: `io::docker::exec_interactive` uses `Command::status()` with inherited stdio; `run_docker` uses `.output()` and captures.
- C-3: `DockerEngine::is_available` checks `docker info`; every verb calls it after engine selection and before other work.
- C-4: Sandbox modules never write under `.ark/`; all sandbox state is engine-side (container/volume/label), discovered live via `docker ps`.
- C-5: `--slug` absent resolves to `state.focus` via `resolve_focus_slug`, raising `Error::NoFocus { project_root, candidates }` when unset, per task-concurrency-control SPEC C-23.
- C-6: The worktree and its git dirs are resolved read-only; sandbox neither creates nor removes worktrees.
- C-6a: Worktree resolution is local-first — when invoked from inside the worktree (the slug's local `task.toml` carries a `worktree_path`), the current root is the worktree; only otherwise is `git worktree list` walked from the parent checkout.
- C-7: The git common dir is derived via `git rev-parse --path-format=absolute --git-common-dir`, never `root.join(".git")`.
- C-8: When `mount_git`, `create` mounts the common dir read-write and rewrites `/workspace/.git`'s `gitdir:` line to the guest path.
- C-9: Host→guest path translation lives inside `DockerEngine`; Windows host paths are translated to the engine POSIX view for every `-v` mount.
- C-10: A named volume `<container>-cfg` mounts the agent config dir; `rm` preserves it unless `--keep-volume` is absent, and a volume-in-use error warns rather than fails.
- C-11: `[sandbox]` parses via an outer `RawConfig` with no `deny_unknown_fields` and an inner `SandboxSection` carrying it; missing → defaults, corrupt → `Error::SandboxConfigCorrupt`.
- C-12: `cfg.engine` defaults to `"docker"`; any other value → `Error::UnknownSandboxEngine`.
- C-13: `create` does `docker pull` of `cfg.image`; it never builds. `sandbox/Dockerfile` is the CI build source, not embedded via `include_dir!`.
- C-14: `cfg.image` defaults to `ghcr.io/anekoique/ark-sandbox:<CARGO_PKG_VERSION>`; `ImagePullFailed` names the expected tag.
- C-15: A released crate version must ship its matching published image tag (CI publish invariant).
- C-16: The container has open outbound network in v1; no `--network` confinement or egress proxy is applied.
- C-17: Containers carry labels `ark.sandbox.slug` and `ark.sandbox.branch`; `list` filters on the slug label.
- C-18: `enter --agent` maps `claude → --dangerously-skip-permissions` and `codex → --yolo`; opencode → `Error::AgentYoloUnsupported`.
- C-19: Container/volume names are `ark-sandbox-<sanitized-branch>-<hash8>`, `hash8` being the first 8 hex of SHA-256 of the exact branch.
- C-20: Every verb writes a single `Display` summary; no ad-hoc stdout writes in command bodies.
- C-21: `remove` is idempotent: an already-absent container reports `container_removed: false` without error.
- C-22: On a rootful Unix daemon `create` passes `--user <uid>:<gid>`; on rootless Docker (container-root already maps to the host user) and non-Unix hosts the flag is omitted.
- C-23: `ark unload` / `load` / `upgrade` / snapshot ignore sandbox state; no `.ark/`-disk footprint means nothing to capture.
- C-24: The published image runs as a user tolerating an arbitrary `--user` uid/gid with a writable config dir.
- C-25: `--agent` selects the first installed platform in `PLATFORMS` order, overridable by `--platform <id>`; none installed → `Error::NoAgentPlatform`.

---
