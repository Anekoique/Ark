# `ark-sandbox` PLAN `00`

> Status: Draft
> Feature: `ark-sandbox`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none

---

## Summary

`ark sandbox` confines a task's worktree inside a sandbox so an unsupervised yolo agent can only touch `/workspace`. Four user-facing verbs — `create`, `enter`, `rm`, `list` — mirror `ark cleanup`'s CLI shape (`TargetArgs` + optional `--slug` resolving against `state.focus`). The backend is abstracted behind a tiny `SandboxEngine` trait so the feature is not Docker-locked; v1 ships exactly one implementor, `DockerEngine`, selected via `[sandbox] engine = "docker"` (the only accepted value today), leaving an OS-native (Seatbelt/bwrap) backend as an additive v2. A new `io/docker.rs`, sibling to `io/git.rs`, is the sole sanctioned `Command::new("docker")` site; every Docker op routes through it. The container pulls a prebuilt published image, bind-mounts the worktree rw at `/workspace` and the parent `.git` ro (with the worktree gitdir rewritten so in-box git resolves history) — translating host paths to the engine's guest view so Windows hosts work — and persists the agent config dir in a named volume so a one-time in-box login survives recreate. The box has **open outbound network** in v1 (matching the reference project); the cage is filesystem + process, not network. `[sandbox]` in `.ark/config.toml` follows the `[worktree]`/`[upgrade]` pattern verbatim. No worktree creation, no credential reconciliation — sandbox is strictly downstream of `task new --worktree`.

## Log `None in 00_PLAN`

[**Added**]

- N/A (first iteration)

[**Changed**]

- N/A

[**Removed**]

- N/A

[**Unresolved**]

- TR-1 (top-level `ark sandbox` vs hidden `ark agent sandbox`) — resolved here as top-level; flagged for reviewer confirmation.
- TR-5 (`SandboxEngine` trait shape) — adopted from research §D.1; reviewer to confirm the trait surface is minimal-yet-sufficient.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| — | — | — | First iteration; no prior review. Research findings folded in directly (see Trade-offs TR-2..TR-6). |

---

## Spec

[**Goals**]

- G-1: `ark sandbox create` starts a sandbox confining a task's worktree at `/workspace`.
- G-2: `ark sandbox enter` opens a shell in the box, or the agent CLI with `--agent`.
- G-3: `ark sandbox rm` tears down the sandbox, preserving the named volume by default.
- G-4: `ark sandbox list` enumerates running Ark sandboxes, one row each.
- G-5: A persistent named volume lets a one-time in-box login survive container recreate.

[**Non-goals**]

- NG-1: No worktree creation or teardown; sandbox reuses `task new --worktree` and leaves cleanup to `ark cleanup`.
- NG-2: No credential reconciliation, keychain extraction, GPG/SSH/dotfiles bind, or host-config sync beyond the named volume + `ANTHROPIC_API_KEY` pass-through.
- NG-3: No network egress confinement in v1; the container has open outbound internet, so the cage is filesystem + process, not network (a v2 egress-allowlist proxy is the follow-up).
- NG-4: One backend (`docker`) ships in v1; the `SandboxEngine` trait exists but only `DockerEngine` is implemented (`native` Seatbelt/bwrap deferred to v2).

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                  (Command::Sandbox(SandboxArgs); SandboxCommand
│                                          {Create, Enter, Rm, List}; dispatch mirrors Cleanup)
└── ark-core/src/
    ├── lib.rs                            (re-exports public sandbox API)
    ├── error.rs                          (new variants — see Data Structure)
    ├── io/
    │   └── docker.rs                     (NEW — Docker-specific spawn site, like io/git.rs:
    │                                       run_docker(args, cwd) -> DockerOutput;
    │                                       exec_interactive(args, cwd) -> i32 via .status();
    │                                       docker_info_ok(); the only Command::new("docker"))
    └── commands/
        └── sandbox/                      (NEW)
            ├── mod.rs                     (public types + dispatch; re-exports)
            ├── engine.rs                  (SandboxEngine trait + SandboxSpec/SandboxHandle/
            │                               RemoveOpts/RemoveOutcome; select_engine(&cfg))
            ├── engines/
            │   └── docker.rs              (impl SandboxEngine for DockerEngine; owns host→guest
            │                               path translation; calls io::docker; the only impl in v1)
            ├── config.rs                  (SandboxConfig — [sandbox] incl. `engine`, `image`,
            │                               `mount_git`, `env_passthrough`)
            ├── naming.rs                  (container/volume name + label derivation from branch)
            ├── resolve.rs                 (slug → (worktree_path, TaskToml) via worktree discovery;
            │                               builds the OS-agnostic SandboxSpec)
            ├── create.rs                  (sandbox_create — select engine, engine.create)
            ├── enter.rs                   (sandbox_enter — engine.enter; shell | yolo agent argv)
            ├── rm.rs                       (sandbox_rm — engine.remove)
            └── list.rs                     (sandbox_list — engine.list)
templates/
└── ark/
    └── config.toml                       (gains a commented [sandbox] section default)
sandbox/                                   (NEW — renamed intent of the old `docker/`)
└── Dockerfile                            (build source for the published image; CI builds + pushes
                                            ghcr.io/anekoique/ark-sandbox:<ver>; NOT include_dir!-
                                            embedded — create pulls, never builds)
```

Module coupling (one-way): `create`/`enter`/`rm`/`list` → `engine::select_engine` + `resolve` + `config` + `naming`. `engines/docker` → `io::docker` (+ owns path translation). `resolve` → `worktree::discovery::find_worktree_for_slug` + `WorktreeConfig` (read-only reuse). `naming` is a leaf (pure string derivation). No sandbox module writes under `.ark/`.

Call graph for `ark sandbox create`:

```
sandbox_create(opts)
  ├── cfg = SandboxConfig::load_or_default(layout); cfg.validate()  → Error::SandboxConfigInvalid
  ├── engine = engine::select_engine(&cfg)                          → Error::UnknownSandboxEngine
  ├── engine.is_available()                                         → Error::SandboxBackendUnavailable
  │     (DockerEngine: io::docker::docker_info_ok())
  ├── (wt, toml) = resolve::resolve_task(root, slug)                → Error::WorktreeNotFound / NoFocus
  │     find_worktree_for_slug(root, worktrees_dir, slug)
  ├── names = naming::derive(&slug, &toml.branch)                   (container + volume + labels)
  ├── if engine reports existing sandbox for names:                 → Error::SandboxExists (unless --recreate)
  ├── spec = resolve::build_spec(&wt, root, &toml, &names, &cfg)
  │     worktree_path, repo_git_dir, mount_git, env_passthrough (resolved host values),
  │     config_dir_mounts (named volume), labels — all OS-agnostic; engine translates paths
  └── engine.create(&spec)                                          → Error::ContainerStartFailed / ImagePullFailed
        DockerEngine.create:
          ├── run_docker(["pull", &cfg.image])                      → ImagePullFailed (non-zero)
          ├── args = build_run_args(&spec)  (volumeArg/toEnginePath per host OS; labels; -e; -w; -d)
          ├── run_docker(args)                                      → ContainerStartFailed (best-effort rm -f on fail)
          └── rewrite_gitdir(container)     (exec: point /workspace/.git at <repo>/.git/worktrees/<b>)
  └── return SandboxCreateSummary { slug, branch, engine, container, volume, image }
```

Call graph for `ark sandbox enter`:

```
sandbox_enter(opts)
  ├── cfg + engine = select; engine.is_available()
  ├── handle = engine.resolve_handle(slug)                          → Error::SandboxNotFound
  ├── argv = if opts.agent {
  │             platform_yolo_argv(detect_platform(layout))         → Error::NoAgentPlatform
  │           } else { ["bash"] }
  └── engine.enter(&handle, argv)                                   (DockerEngine: exec_interactive(["exec","-it",..]))
                                                                     (returns child exit code)
```

Call graph for `ark sandbox rm`:

```
sandbox_rm(opts)
  ├── cfg + engine = select; engine.is_available()
  ├── handle = engine.resolve_handle(slug)                          → Error::SandboxNotFound
  └── engine.remove(&handle, &RemoveOpts { keep_volume })
        DockerEngine.remove:
          ├── run_docker(["rm","-f",&container])                    (idempotent; absent → removed:false)
          └── if !keep_volume: run_docker(["volume","rm",&volume])  (in-use → warn, not fail)
  └── return SandboxRmSummary { slug, container_removed, volume_removed }
```

Call graph for `ark sandbox list`:

```
sandbox_list(opts)
  ├── cfg + engine = select; engine.is_available()
  └── engine.list()
        DockerEngine.list:
          ├── run_docker(["ps","--filter","label=ark.sandbox.slug",
          │               "--format","{{.Names}}\t{{.Status}}\t{{.Label \"ark.sandbox.branch\"}}"])
          └── parse rows; sort by slug
  └── return SandboxListSummary { rows }                            (Display: empty stdout when none)
```

[**Data Structure**]

```rust
// ark-core/src/io/docker.rs
#[derive(Debug, Clone)]
pub struct DockerOutput {
    pub exit_code: i32,    // -1 when the process terminated without a code
    pub stdout: String,
    pub stderr: String,    // captured for diagnostics
}

// ark-core/src/commands/sandbox/engine.rs
pub trait SandboxEngine {
    fn id(&self) -> &'static str;                 // "docker"; stored in config + labels
    fn is_available(&self) -> Result<()>;         // Err = Error::SandboxBackendUnavailable
    fn create(&self, spec: &SandboxSpec) -> Result<SandboxHandle>;
    fn resolve_handle(&self, slug: &str) -> Result<SandboxHandle>;
    fn enter(&self, h: &SandboxHandle, argv: &[&str]) -> Result<i32>;
    fn remove(&self, h: &SandboxHandle, opts: &RemoveOpts) -> Result<RemoveOutcome>;
    fn list(&self) -> Result<Vec<SandboxRow>>;
}

#[derive(Debug, Clone)]
pub struct SandboxSpec {
    pub worktree_path: PathBuf,             // host path; engine translates to guest
    pub repo_git_dir: PathBuf,              // host <repo>/.git; ro mount when mount_git
    pub mount_git: bool,
    pub branch: String,
    pub env_passthrough: Vec<(String, String)>,  // already-resolved host name→value pairs
    pub config_volume: String,             // named volume for the agent config dir
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
    pub engine: String,                 // default "docker"; only "docker" accepted in v1
    pub image: String,                  // default "ghcr.io/anekoique/ark-sandbox:<crate-ver>"
    pub mount_git: bool,                // default true
    pub env_passthrough: Vec<String>,   // default ["ANTHROPIC_API_KEY"]; host-set vars to -e
}
// Private upgrade-style RawConfig { sandbox: Option<SandboxSection> } with
// #[serde(deny_unknown_fields)]; missing file/section → defaults.

// ark-core/src/commands/sandbox/naming.rs
#[derive(Debug, Clone)]
pub struct SandboxNames {
    pub container: String,   // "ark-sandbox-<sanitized-branch>"
    pub volume: String,      // "ark-sandbox-<sanitized-branch>-cfg"
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
pub struct SandboxEnterOptions { pub project_root: PathBuf, pub slug: Option<String>, pub agent: bool }
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

// ark-core/src/error.rs (additions)
Error::DockerSpawn          { source: std::io::Error },     // spawn failure (mirrors GitSpawn)
Error::SandboxBackendUnavailable { engine: String },        // backend not reachable (docker info fails)
Error::UnknownSandboxEngine { value: String },              // [sandbox] engine not "docker"
Error::SandboxConfigCorrupt { path: PathBuf, source: toml::de::Error },
Error::SandboxConfigInvalid { reason: &'static str },
Error::SandboxExists        { slug: String, container: String },
Error::SandboxNotFound      { slug: String },
Error::ImagePullFailed      { image: String, exit_code: i32 },
Error::ContainerStartFailed { container: String, exit_code: i32 },
Error::NoAgentPlatform      { project_root: PathBuf },       // --agent but no platform installed
```

[**API Surface**]

```rust
// io/docker.rs
pub fn docker_info_ok() -> bool;                                   // `docker info` exits 0
pub fn run_docker(args: &[&str], cwd: &Path) -> Result<DockerOutput>;
pub fn exec_interactive(args: &[&str], cwd: &Path) -> Result<i32>;

// commands/sandbox/engine.rs
pub fn select_engine(cfg: &SandboxConfig) -> Result<Box<dyn SandboxEngine>>;

// commands/sandbox/
pub fn sandbox_create(opts: SandboxCreateOptions) -> Result<SandboxCreateSummary>;
pub fn sandbox_enter(opts: SandboxEnterOptions)   -> Result<SandboxEnterSummary>;
pub fn sandbox_rm(opts: SandboxRmOptions)         -> Result<SandboxRmSummary>;
pub fn sandbox_list(opts: SandboxListOptions)     -> Result<SandboxListSummary>;

// config.rs
impl SandboxConfig {
    pub fn load_or_default(layout: &Layout) -> Result<Self>;
    pub fn validate(&self) -> Result<()>;
}

// CLI shape (ark-cli/src/main.rs) — mirrors CleanupArgs
#[derive(clap::Args)]
struct SandboxArgs { #[command(subcommand)] command: SandboxCommand }

#[derive(Subcommand)]
enum SandboxCommand {
    Create(SandboxCreateCliArgs),  // --slug?, --recreate
    Enter(SandboxEnterCliArgs),    // --slug?, --agent
    Rm(SandboxRmCliArgs),          // --slug?, --keep-volume
    List(SandboxListCliArgs),      // (target only)
}
// Each *CliArgs flattens TargetArgs (-C DIR) and resolves via resolve_with_discovery().
```

[**Constraints**]

- C-1: All `docker` invocations route through `io::docker`; `Command::new` may NOT appear under `commands/sandbox/` (extends the `commands_no_bare_command_new` SOURCES list).
- C-2: `io::docker::exec_interactive` uses `Command::status()` with inherited stdio so the agent CLI's TTY works; `run_docker` uses `.output()` and captures.
- C-3: `SandboxEngine::is_available` for `DockerEngine` checks `docker info` (not spawn success); every verb calls it after engine selection and before other work.
- C-4: Sandbox modules never write under `.ark/`; all sandbox state is engine-side (Docker container/volume/label) state, discovered live (`docker ps`).
- C-5: `--slug` is optional on every verb; absent → resolve `state.focus` (→ `Error::NoFocus`), mirroring `ark cleanup` and the concurrency SPEC.
- C-6: The worktree is resolved read-only via `find_worktree_for_slug`; sandbox neither creates nor removes worktrees (worktree SPEC NG-1/NG-2 preserved).
- C-7: `create` mounts the worktree rw at `/workspace` and (when `cfg.mount_git`) the parent `.git` ro, rewriting `/workspace/.git` to point at the in-guest `<repo>/.git/worktrees/<branch>`.
- C-8: Host→guest path translation lives inside `DockerEngine` (not `resolve.rs`); Windows host paths are translated to the engine's POSIX view for every `-v` mount.
- C-9: A named volume `ark-sandbox-<branch>-cfg` mounts the agent config dir; `rm` preserves it unless `--keep-volume` is absent, and a volume-in-use error warns rather than fails.
- C-10: Only host env vars named in `cfg.env_passthrough` AND actually set are forwarded via `-e`; an unset var is silently skipped, never forwarded empty.
- C-11: `[sandbox]` parses via a private `RawConfig` with `#[serde(deny_unknown_fields)]`; missing → defaults, corrupt → `Error::SandboxConfigCorrupt`.
- C-12: `cfg.engine` defaults to `"docker"`; any other value → `Error::UnknownSandboxEngine` (the trait exists for v2, but only `DockerEngine` is registered).
- C-13: `create` does `docker pull` of `cfg.image`; it never builds. `sandbox/Dockerfile` is the CI build source, not embedded via `include_dir!`.
- C-14: `cfg.image` default is pinned to the crate version (`ghcr.io/anekoique/ark-sandbox:<CARGO_PKG_VERSION>`) so binary and image version together.
- C-15: The container has open outbound network in v1; no `--network` confinement or egress proxy is applied (documented limitation, NG-3).
- C-16: Containers carry labels `ark.sandbox.slug` and `ark.sandbox.branch`; `list` filters on the slug label so non-Ark containers never appear.
- C-17: `enter` without `--agent` runs `bash`; with `--agent` it maps the installed platform's `cli_flag` to its yolo argv, erroring `NoAgentPlatform` when none is installed.
- C-18: Container/volume names derive from the sanitized branch (non-`[A-Za-z0-9_.-]` → `-`); the derivation is injective enough that distinct branches cannot collide on a name.
- C-19: Every verb writes a single `Display` summary; no ad-hoc stdout writes in command bodies (project convention).
- C-20: `remove` is idempotent: removing an already-absent container reports `container_removed: false` without error.
- C-21: `ark unload` / `load` / `upgrade` / snapshot ignore sandbox state entirely; no `.ark/`-disk footprint means nothing to capture.

---

## Runtime

[**Main Flow**]

1. User runs `task new --slug X --worktree`, then `ark sandbox create` from the worktree (or `-C` / `--slug X` from the parent).
2. `create` loads config, selects the engine, probes `docker info`, resolves the worktree, then `engine.create`: pulls the image, runs a detached container with path-translated mounts + env pass-through + labels, and rewrites the in-box gitdir.
3. User runs `ark sandbox enter` → bash at `/workspace`; runs `claude /login` once (token persists in the volume), then `claude --dangerously-skip-permissions` — or `enter --agent` to launch it directly.
4. Work happens confined to `/workspace` (filesystem + process); the box has open network, so the agent CLI reaches its API. Git reads history through the ro `.git` mount and commits to the worktree's index.
5. User exits, commits from host or in-box, then `ark sandbox rm` (volume kept) and later `ark cleanup` for the worktree.

[**Failure Flow**]

1. `[sandbox] engine` not `"docker"` → `Error::UnknownSandboxEngine` before any side effect.
2. `docker info` fails (absent / daemon down / rootless mis-set) → `Error::SandboxBackendUnavailable`.
3. No worktree for slug → `Error::WorktreeNotFound`; no focus → `Error::NoFocus`.
4. `docker pull` non-zero (offline / private registry / bad tag) → `Error::ImagePullFailed`; no container created.
5. `docker run` non-zero → `Error::ContainerStartFailed`; `DockerEngine.create` attempts a best-effort `rm -f` of the half-started container before returning.
6. `enter --agent` with no installed platform → `Error::NoAgentPlatform`.

[**State Transitions**]

- (no container) → running when `create` succeeds.
- running → (no container) when `rm` succeeds; volume persists unless dropped.
- running → running on repeated `enter` (idempotent; exec into the live box).
- `create` on an existing container → `Error::SandboxExists` unless `--recreate` (which `rm -f`s first).

---

## Implementation

[**Phase 1**] — `io/docker.rs` + error variants. Add `run_docker`, `exec_interactive`, `docker_info_ok`; add the `Error` variants; extend `commands_no_bare_command_new` SOURCES with the new sandbox sources. Unit tests against the docker-absent path and arg construction.

[**Phase 2**] — `commands/sandbox/{engine,config,naming,resolve}.rs` + `engines/docker.rs`. The `SandboxEngine` trait + `SandboxSpec`/`SandboxHandle`; `select_engine` (registers only `DockerEngine`); `SandboxConfig` load/validate (mirror upgrade's `strategy.rs`); `SandboxNames` derivation + sanitization; `resolve_task` + `build_spec`; `DockerEngine` with host→guest path translation + `build_run_args`. Unit tests for config round-trip, name sanitization, path translation (Windows vs POSIX host), the run-arg vector (env skipped when unset, git mount toggled), and unknown-engine rejection.

[**Phase 3**] — `commands/sandbox/{create,enter,rm,list}.rs` + `mod.rs` + lib re-exports + CLI `Sandbox` dispatch + `templates/ark/config.toml` `[sandbox]` block + `sandbox/Dockerfile` + a CI publish job. Wire `Display` summaries. Integration test the dispatch + docker-absent path end-to-end.

---

## Trade-offs

- TR-1: **Top-level `ark sandbox` vs hidden `ark agent sandbox`.** Chose top-level (semver-covered) because it is a user-facing command like `ark cleanup`/`ark archive`, not a workflow-structural mutation the slash commands drive. The `ark agent` namespace is reserved for internal phase/SPEC machinery. Reviewer to confirm.
- TR-2: **Pull published image vs embed Dockerfile + build.** Pull is faster first-run and keeps the binary small, at the cost of a per-release CI publish job and an offline failure mode (C-13/ImagePullFailed). User chose pull.
- TR-3: **Persistent volume + env-var fast path vs full credential reconciliation.** The volume lets a one-time in-box login persist cross-platform with zero keychain code; the env var serves API-key users in one flag. agent-infra's ~1,100-LOC reconciliation (keychain extract, N-container token sync) is out of scope (NG-2). Note: a deferred `native` backend would inherit the host home directly and need no volume at all (research §D.4).
- TR-4: **Open network in v1 vs egress confinement.** The reference project (agent-infra) ships open container egress and invests in *more* host access, not less — its `docker run` has no `--network`/proxy/firewall at all. v1 matches that: open egress, documented as NG-3. The stricter "egress allowlist proxy" model (Claude Code, Anthropic sandbox-runtime) defends a *hostile* agent and is a large, separate v2 (host proxy + allowlist + per-OS plumbing). v1's cage is filesystem + process; that meets the stated "confine blast radius on disk" goal.
- TR-5: **`SandboxEngine` trait now vs Docker-only free functions.** Introducing the trait in v1 costs ~one `DockerEngine` struct and removes the Docker name from every command signature, so a v2 `native` (Seatbelt/bwrap) backend is additive rather than a churny rename. The trait surface is deliberately narrower than agent-infra's (`ensure/startVm/syncResources/defaultResources` are daemon/VM-management Ark does not do). Adopted from research §D.1; reviewer to confirm the cost is acceptable vs. a strict minimal-v1 reading.
- TR-6: **Keep `io/docker.rs` Docker-specific vs rename to `io/sandbox.rs`.** Kept Docker-specific (like `io/git.rs` is git-specific) so the raw-spawn layer stays honest about what it spawns; the backend abstraction lives in `commands/sandbox/engine.rs`, not `io/`. The in-repo template dir is renamed `docker/` → `sandbox/` (backend-neutral; a native backend needs no image). Per maintainer request + research §D.3.
- TR-7: **Yolo argv via a local `cli_flag`→command map vs adding a `yolo_flag` field to `Platform`.** A local map in `enter.rs` keeps the platform registry unchanged for a single consumer; a registry field would be cleaner if more code needed it later. Leaning local map; reviewer may prefer the registry field.

---

## Validation

[**Unit Tests**]

- V-UT-1: `SandboxConfig::load_or_default` — missing file, missing section, full section round-trip, and `deny_unknown_fields` rejection (mirrors upgrade strategy tests).
- V-UT-2: `SandboxConfig::validate` rejects an empty `image`; `select_engine` rejects `engine` ≠ `"docker"` with `Error::UnknownSandboxEngine`.
- V-UT-3: `naming::derive` sanitizes `feat/x` → `ark-sandbox-feat-x` (+ volume suffix), stable across calls, and distinct branches never collide on a name.
- V-UT-4: `build_run_args` includes the git mount iff `cfg.mount_git`, forwards only host-set env vars from `env_passthrough` (unset → omitted), and applies no `--network` flag (open egress, C-15).
- V-UT-5: host→guest path translation maps a Windows host path to the engine POSIX view and is a no-op for an already-POSIX path.
- V-UT-6: `run_docker` in a docker-absent env yields `Error::DockerSpawn`, never a panic; `docker_info_ok` returns false.

[**Integration Tests**]

- V-IT-1: `commands_no_bare_command_new` still passes with the sandbox sources added to SOURCES (no `Command::new` under `commands/sandbox/`).
- V-IT-2: CLI parses `ark sandbox {create,enter,rm,list}` with their flags (clap assertion, no docker needed).
- V-IT-3: Every verb returns `Error::SandboxBackendUnavailable` (not a panic) when docker is absent — full dispatch path.

[**Failure / Robustness**]

- V-F-1: `DockerEngine.create` rolls back (best-effort `rm -f`) when `docker run` fails after a successful pull.
- V-F-2: `remove` of an absent container reports `container_removed: false`, exit 0 (idempotent, C-20).
- V-F-3: `remove` volume-in-use warns and still reports container removal; does not fail the command (C-9).

[**Edge Cases**]

- V-E-1: `--slug` absent with no focus → `Error::NoFocus` (concurrency SPEC parity).
- V-E-2: `create` twice without `--recreate` → `Error::SandboxExists`; with `--recreate` → replaces, volume preserved.
- V-E-3: `list` with zero Ark containers → empty stdout, exit 0.
- V-E-4: `enter --agent` with no installed platform → `Error::NoAgentPlatform`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-2, V-UT-4, V-F-1 |
| G-2 | V-IT-2, V-E-4 |
| G-3 | V-F-2, V-F-3, V-E-2 |
| G-4 | V-E-3, V-IT-2 |
| G-5 | V-UT-4 (volume in run args), V-E-2 (recreate preserves volume) |
| C-1 | V-IT-1 |
| C-2 | V-IT-2 (exec path parses), V-IT-3 |
| C-3 | V-IT-3, V-UT-6 |
| C-4 | V-IT-1 (no `.ark/` writes), V-E-3 |
| C-5 | V-E-1 |
| C-6 | V-UT-4 (read-only resolve), V-F-1 |
| C-7 | V-UT-4 |
| C-8 | V-UT-5 |
| C-9 | V-F-3, V-E-2 |
| C-10 | V-UT-4 |
| C-11 | V-UT-1 |
| C-12 | V-UT-2 |
| C-13 | V-F-1 (pull then run), TR-2 |
| C-14 | V-UT-2 (image present + version-pinned default) |
| C-15 | V-UT-4 (no `--network` flag) |
| C-16 | V-E-3 (label filter) |
| C-17 | V-E-4 |
| C-18 | V-UT-3 |
| C-19 | V-IT-2 |
| C-20 | V-F-2 |
| C-21 | V-IT-1 |
