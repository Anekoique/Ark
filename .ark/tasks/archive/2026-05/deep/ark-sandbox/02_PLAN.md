# `ark-sandbox` PLAN `02`

> Status: Draft
> Feature: `ark-sandbox`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`

---

## Summary

`ark sandbox` confines a task's worktree inside a Docker container so an unsupervised yolo agent can only touch `/workspace` while still committing to its own branch. Four user-facing verbs — `create`, `enter`, `rm`, `list` — mirror `ark cleanup`'s CLI shape (`TargetArgs` + optional `--slug`), resolving a missing slug through the concurrency SPEC's `state.focus` contract via a new `ark-core` resolver. The backend is abstracted behind a tiny `SandboxEngine` trait; v1 ships only `DockerEngine`, selected via `[sandbox] engine = "docker"`. A new `io/docker.rs`, sibling to `io/git.rs`, is the sole sanctioned `Command::new("docker")` site. Git dirs are derived from `git rev-parse --git-common-dir` (never `root.join(".git")`, because a worktree's `.git` is a file): the shared `.git` common dir — which contains the worktree's nested gitdir, the object store, and all refs — is mounted **read-write** so in-box `git commit` works, with the full widened blast radius documented honestly. The box has open outbound network in v1. `[sandbox]` follows the `[upgrade]` config precedent (`deny_unknown_fields` on the inner section). The published image must tolerate an arbitrary uid so `--user` writes are host-owned. Sandbox writes nothing under `.ark/`.

## Log `02_PLAN`

[**Added**]

- Inlined `SandboxCreateOptions`/`SandboxRmOptions`/`SandboxListOptions` so the `## Spec` is self-contained (R-001).
- C-24: the published image must tolerate an arbitrary uid (writable `$HOME`/config dir) (R-003).
- A note on V-IT-4 that the real-commit leg is docker-host-gated (R-005).

[**Changed**]

- `GitMounts` reduced to a single `common_dir` (the worktree gitdir is nested inside it, so the separate mount was redundant); C-8 + Architecture simplified (R-002).
- TR-8 restated to name the real blast radius: all branch refs, object store, config/hooks, every sibling worktree's per-worktree gitdir (R-002).
- C-22 qualified for the rootless-Docker case (container-root already maps to host user) (R-003).
- C-8/C-18 trailing rationale trimmed (now solely in TR-8 / the hash definition); G-1 tightened under 80 chars (R-004).

[**Removed**]

- The redundant `worktree_gitdir` field/mount and TR-8's understated "sibling working state stays out of the mount set" clause (R-002).

[**Unresolved**]

- None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 (HIGH) | Accepted | Inlined the three missing option structs (`SandboxCreateOptions { project_root, slug, recreate }`, `SandboxRmOptions { project_root, slug, keep_volume }`, `SandboxListOptions { project_root }`) into Data Structure; deleted the "as 00_PLAN" phrase. The `## Spec` now restates every public type in full. |
| Review | R-002 (MED) | Accepted | `GitMounts` is now just `common_dir` (the worktree gitdir lives under it, so the second mount was redundant). C-8 mounts the common dir rw and rewrites the gitdir. TR-8 now states the true surface: shared object store, ALL `refs/heads/*` + `packed-refs`, `.git/config`/`hooks`, and every sibling worktree's per-worktree gitdir; only sibling working *trees* stay out. |
| Review | R-003 (MED) | Accepted | New C-24: the published image must run as / tolerate an arbitrary uid with a writable config dir. C-22 qualified: on rootless Docker, container-root already maps to the host user, so `--user` is applied only on rootful daemons (detected via `docker info` SecurityOptions); rootless omits it. V-UT-8 updated. |
| Review | R-004 (LOW) | Accepted | Trimmed `so …` rationale from C-8 and C-18; tightened G-1 to ≤80 chars. C-14 split into default vs. release-coupling. C-23 read as a single sentence (a control-procedure is correctly a Constraint). |
| Review | R-005 (LOW) | Accepted | V-IT-4 + its Acceptance Mapping row note the in-box `git commit` leg is docker-host-gated (skipped on docker-less CI; derivation always runs). |

---

## Spec

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

## Runtime

[**Main Flow**]

1. User runs `task new --slug X --worktree`, then `ark sandbox create` from the worktree (or `-C` / `--slug X` from the parent).
2. `create` loads config, selects the engine, probes `docker info`, resolves the focus/named slug to a worktree, derives the common dir via `rev-parse`, then `engine.create`: pulls the image, runs a detached container with path-translated rw mounts (`/workspace`, `.git` common dir) + `--user` (rootful only) + env + labels, and rewrites the in-box gitdir.
3. User runs `ark sandbox enter` → bash at `/workspace`; runs `claude /login` once (token persists in the volume), then `claude --dangerously-skip-permissions` — or `enter --agent`.
4. Work happens confined to `/workspace`; the agent reads history AND commits to its branch in-box (git resolves through the rw common-dir mount). The box has open network so the agent CLI reaches its API.
5. User exits, then `ark sandbox rm` (volume kept) and later `ark cleanup` for the worktree.

[**Failure Flow**]

1. `[sandbox] engine` not `"docker"` → `Error::UnknownSandboxEngine` before any side effect.
2. `docker info` fails → `Error::SandboxBackendUnavailable`.
3. No worktree for slug → `Error::WorktreeNotFound`; no focus → `Error::NoFocus`.
4. `docker pull` non-zero → `Error::ImagePullFailed` (names the expected tag); no container created.
5. `docker run` non-zero → `Error::ContainerStartFailed`; best-effort `rm -f` of the half-started container.
6. `enter --agent` with no installed platform → `Error::NoAgentPlatform`; opencode selected → `Error::AgentYoloUnsupported`.

[**State Transitions**]

- (no container) → running when `create` succeeds.
- running → (no container) when `rm` succeeds; volume persists unless dropped.
- running → running on repeated `enter` (idempotent).
- `create` on an existing container → `Error::SandboxExists` unless `--recreate` (which `rm -f`s first).

---

## Implementation

[**Phase 1**] — `io/docker.rs` + error variants. Add `run_docker`, `exec_interactive`, `docker_info_ok`, rootful detection; add the `Error` variants with E-9 `#[error]` templates; extend `commands_no_bare_command_new` SOURCES. Unit tests: docker-absent path, arg construction.

[**Phase 2**] — `commands/sandbox/{engine,config,naming,gitmounts,resolve,platform_argv}.rs` + `engines/docker.rs` + `state/checkout::resolve_focus_slug`. The trait + `SandboxSpec`/`GitMounts`; `select_engine` (only `DockerEngine`); config load/validate (mirror `upgrade/strategy.rs`, inner `deny_unknown_fields`); name derivation + hash8; `derive_git_mounts` via `rev-parse`; `resolve_focus_slug`; yolo-argv map + selection rule; `DockerEngine` with path translation, `build_run_args` (incl. conditional `--user`, rw common-dir mount), `rewrite_gitdir`. Unit tests per Validation.

[**Phase 3**] — `commands/sandbox/{create,enter,rm,list}.rs` + `mod.rs` + lib re-exports + CLI `Sandbox` dispatch + `templates/ark/config.toml` `[sandbox]` block + `sandbox/Dockerfile` (arbitrary-uid-tolerant) + CI publish job (records the release→tag invariant, C-15). Wire `Display`. Integration tests: dispatch, docker-absent path, in-box `git commit` against a throwaway local repo (docker-host-gated).

---

## Trade-offs

- TR-1: **Top-level `ark sandbox`** (not hidden `ark agent sandbox`) — it is a user-facing command like `ark cleanup`, not workflow-structural machinery.
- TR-2: **Pull published image vs build** — pull is faster/smaller at the cost of a per-release CI publish job + an offline failure mode (C-13/C-15).
- TR-3: **Persistent volume + env-var vs credential reconciliation** — the volume persists a one-time in-box login cross-platform with no keychain code; reconciliation is out of scope (NG-2).
- TR-4: **Open network in v1** — matches the reference project (agent-infra ships zero network confinement); the stricter egress-allowlist-proxy model is a separate v2. v1's cage is filesystem + process.
- TR-5: **`SandboxEngine` trait now** — ~one struct of cost, de-Dockers the signatures, makes a v2 native backend additive.
- TR-6: **Keep `io/docker.rs` Docker-specific** (like `io/git.rs`); the abstraction lives in `commands/sandbox/engine.rs`. Template dir `docker/` → `sandbox/`.
- TR-7: **Yolo argv via a local map** (not a `Platform.yolo_flag` field) — one consumer; `Platform` is `#[non_exhaustive]` so a field stays additive later.
- TR-8: **Commit-in-box (rw `.git` common dir) vs read-only history.** Maintainer authorized commit on the branch, so the `.git` common dir mounts rw. The honest blast radius: the box can write the shared object store, ALL branch refs (`refs/heads/*` + `packed-refs`), `.git/config` and `hooks/`, and every sibling worktree's per-worktree gitdir (each nested under the common dir). Only the sibling *working trees* (separate directories) stay outside the mount set. A compromised yolo agent could thus rewrite any ref or corrupt a sibling's HEAD; this is accepted as the cost of a full in-box workflow on the maintainer's authorization. A narrower mount (per-worktree gitdir + objects only) is a possible v2 hardening but is not attempted in v1 because git's object/ref layout makes a partial rw mount fragile.
- TR-9: **Conditional `--user` + uid-tolerant image vs unconditional `--user`.** Applying `--user <hostuid>` only on rootful daemons avoids the rootless case (where container-root already maps to the host user, so an explicit sub-uid would mis-own writes); pairing it with an arbitrary-uid-tolerant image (C-24) keeps the volume-backed config dir writable. The two halves are one contract: the flag and the image's uid assumptions travel together.

---

## Validation

[**Unit Tests**]

- V-UT-1: `SandboxConfig::load_or_default` — missing file, missing section, full round-trip, rejection of an unknown key *inside* `[sandbox]` (inner `deny_unknown_fields`); a foreign `[worktree]` section present does NOT error.
- V-UT-2: `SandboxConfig::validate` rejects an empty `image`; `select_engine` rejects `engine` ≠ `"docker"` → `UnknownSandboxEngine`.
- V-UT-3: `naming::derive` → `ark-sandbox-feat-x-<hash8>`, stable across calls, and `feat/x` vs `feat-x` get DISTINCT names (collision-resistance, C-19).
- V-UT-4: `build_run_args` forwards only host-set `env_passthrough` vars (unset → omitted) and applies no `--network` flag (C-16).
- V-UT-5: host→guest path translation maps a Windows host path to the engine POSIX view; no-op for a POSIX path.
- V-UT-6: `run_docker` in a docker-absent env yields `Error::DockerSpawn`, never a panic; `docker_info_ok` returns false.
- V-UT-7: `build_run_args` includes `-v <guest-workspace>:/workspace` rw, the common-dir rw mount when `mount_git`, and the `rewrite_gitdir` exec argv targets the correct guest `gitdir:` path (C-7/C-8).
- V-UT-8: `build_run_args` includes `--user <uid>:<gid>` when `spec.user` is `Some` (rootful) and omits it when `None` (rootless / non-Unix) (C-22).
- V-UT-9: `platform_argv::yolo_argv` returns the right flag for claude/codex and `AgentYoloUnsupported` for opencode; `select_platform` picks first-in-order and honors `--platform` (C-18/C-25).
- V-UT-10: `resolve_focus_slug` returns the focus when slug is `None`, the given slug when `Some`, and `Error::NoFocus` when neither (C-5).

[**Integration Tests**]

- V-IT-1: `commands_no_bare_command_new` passes with the sandbox sources in SOURCES.
- V-IT-2: CLI parses `ark sandbox {create,enter,rm,list}` with their flags (clap assertion, no docker).
- V-IT-3: Every verb returns `Error::SandboxBackendUnavailable` (not a panic) when docker is absent. **Docker-host-gated** on a docker-equipped CI; the engine-level `is_available_is_well_typed` test asserts the well-typed error/Ok dichotomy unconditionally.
- V-IT-4: Against a throwaway local git repo + worktree, `derive_git_mounts` returns the real `--git-common-dir`, and the rw common-dir mount is sufficient for an in-box `git commit`. The real-commit leg is **docker-host-gated** (skipped on docker-less CI); the rev-parse derivation always runs.

[**Failure / Robustness**]

- V-F-1: `DockerEngine.create` rolls back (best-effort `rm -f`) when `docker run` fails after a successful pull. **Docker-host-gated** (daemon-only path; not run on docker-less CI).
- V-F-2: `remove` of an absent container reports `container_removed: false`, exit 0 (C-21). **Docker-host-gated** (daemon-only path).
- V-F-3: `remove` volume-in-use warns and still reports container removal; does not fail (C-10). **Docker-host-gated** (daemon-only path).

[**Edge Cases**]

- V-E-1: `--slug` absent with no focus → `Error::NoFocus` (C-5).
- V-E-2: `create` twice without `--recreate` → `Error::SandboxExists`; with `--recreate` → replaces, volume preserved. **Docker-host-gated** (daemon-only path).
- V-E-3: `list` with zero Ark containers → empty stdout, exit 0. Row parsing/shape covered docker-less by `parse_ps_row_reads_slug_label`; the live empty-set leg is docker-host-gated.
- V-E-4: `enter --agent` with no installed platform → `Error::NoAgentPlatform`; with only opencode → `Error::AgentYoloUnsupported`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-7 (/workspace rw mount), V-F-1 |
| G-2 | V-IT-2, V-UT-9, V-E-4 |
| G-3 | V-F-2, V-F-3, V-E-2 |
| G-4 | V-E-3, V-IT-2 |
| G-5 | V-UT-7 (volume mount in args), V-E-2 (recreate preserves volume) |
| C-1 | V-IT-1 |
| C-2 | V-IT-2, V-IT-3 |
| C-3 | V-IT-3, V-UT-6 |
| C-4 | V-IT-1, V-E-3 |
| C-5 | V-UT-10, V-E-1 |
| C-6 | V-IT-4 (read-only derive), V-F-1 |
| C-7 | V-IT-4, V-UT-7 |
| C-8 | V-UT-7, V-IT-4 (commit leg docker-host-gated) |
| C-9 | V-UT-5 |
| C-10 | V-F-3, V-E-2 |
| C-11 | V-UT-1 |
| C-12 | V-UT-2 |
| C-13 | V-F-1 (pull then run) |
| C-14 | V-UT-2 (version-pinned default present) |
| C-15 | V-F-1 (pull path); CI publish step (Implementation Phase 3) |
| C-16 | V-UT-4 |
| C-17 | V-E-3 (label filter) |
| C-18 | V-UT-9, V-E-4 |
| C-19 | V-UT-3 |
| C-20 | V-IT-2 |
| C-21 | V-F-2 |
| C-22 | V-UT-8 |
| C-23 | V-IT-1 |
| C-24 | V-UT-8 (uid-independent volume write target) |
| C-25 | V-UT-9 |
