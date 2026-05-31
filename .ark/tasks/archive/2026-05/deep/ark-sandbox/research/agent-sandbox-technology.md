# Research: agent sandbox technology (cross-platform, non-Docker-only)

- Query: Survey agent-sandbox / code-execution isolation; decide viable backends + whether Ark should abstract over an engine trait instead of hardcoding Docker; produce a v1 recommendation for `ark sandbox` across macOS/Windows/Linux.
- Scope: mixed (internal reference read + external web verification)
- Date: 2026-05-27

> **Confidence tags** used below: **[verified]** = checked against a primary/vendor source this session; **[reported]** = from secondary write-ups (blogs/benchmarks) and should be re-checked before being load-bearing in a SPEC; **[inferred]** = my synthesis, not a cited fact.

---

## Context anchors (internal)

What the current draft assumes, so the recommendation can be diffed against it:

| Source | Line(s) | What it says |
| ------ | ------- | ------------ |
| `.ark/tasks/ark-sandbox/00_PLAN.md` | 57, NG-3 | "No multi-engine abstraction (colima/orbstack/wsl2); `docker` on PATH is required." |
| `00_PLAN.md` | 71-73 | `io/docker.rs` is "the only `Command::new("docker")`"; sole sanctioned site. |
| `00_PLAN.md` | 88-91 | New `docker/Dockerfile` at repo root; CI builds + pushes `ghcr.io/anekoique/ark-sandbox:<ver>`; `create` pulls, never builds. |
| `00_PLAN.md` | 110-116, C-7/C-8 | Mounts: worktree rw `/workspace`; parent `.git` ro `/repo/.git` with gitdir rewrite; named volume for agent config dir; `-e ANTHROPIC_API_KEY` passthrough. |
| `00_PLAN.md` | 339, TR-3 | Persistent volume + env-var fast path replaces agent-infra's ~1,100-LOC credential reconciliation (NG-2). |
| `PRD.md` | 11 | Goal: turn the platform "seatbelt" into a kernel-enforced "cage". |

The reference engine abstraction Ark is asked to evaluate against:

| Reference file | Role |
| -------------- | ---- |
| `reference/agent-infra/lib/sandbox/engine.ts` | `detectEngine()` + `PLATFORM_DEFAULTS` (linux→native, darwin→colima, win32→wsl2); `ensureDocker()`; `validateSandboxEngine()` rejects engines not in the per-OS support set. |
| `reference/agent-infra/lib/sandbox/engines/index.ts` | `SandboxAdapter` type — the trait. Methods: `ensure`, `startVm?`, `stopVm?`, `syncResources`, `defaultResources`; fields `id/displayName/supportedPlatforms/dockerContext/managed/canApplyResources`. `ADAPTERS` registry + `getAdapter()`. |
| `engines/colima.ts` | darwin, managed VM, sets `DOCKER_CONTEXT=colima`; brew-installs colima+docker; `colima start --vm-type=vz --mount-type=virtiofs` on arm64. |
| `engines/orbstack.ts` | darwin, managed, `DOCKER_CONTEXT=orbstack`; hot CPU/mem resize via `orb config set`. |
| `engines/docker-desktop.ts` | darwin+linux+win32, **not** managed, `DOCKER_CONTEXT=desktop-linux`; only checks `docker info`. |
| `engines/native.ts` | linux+win32, not managed; detects rootless docker via `DOCKER_HOST=unix:///run/user/...` or `SecurityOptions` containing `rootless`; rich error guidance. |
| `engines/wsl2.ts` | win32 only, managed; probes `wsl.exe --status` then `wsl.exe -- docker info`. |
| `engines/wsl2-paths.ts` | `toEnginePath()` / `volumeArg()` — rewrites Windows host paths to the engine's POSIX view for `-v` mounts. **This is the single most important cross-platform detail the Docker-only PLAN omits.** |
| `credentials.ts` | ~630 LOC: keychain extract (macOS `security`), blob validation, multi-endpoint reconcile (host↔N mount files), redaction. The NG-2 that the PLAN deliberately drops. |
| `commands/create.ts` | The full create path: `detectEngine` → `ensureDocker` → build/pull → worktree add → tool-state seeding → `docker run -d` with `volumeArg()`/`toEnginePath()`-rewritten mounts. Note: agent-infra **builds** images; the PLAN **pulls**. |

Key takeaway from the reference: the 5-engine abstraction is **not about kernel isolation differences** — all five end up driving the same `docker` CLI against an OCI daemon. The abstraction exists to answer one question per OS: **"how do I get a working Docker daemon + correct host→guest path translation?"** That is the real cross-platform problem, and the Docker-only PLAN currently hand-waves it (assumes `docker` on PATH with Unix-style mounts).

---

## A. Landscape of isolation technologies

### A.1 Container runtimes (OCI; share host kernel via namespaces+cgroups)

| Tech | Isolates | mac | win | linux | Startup | Daemon/root | Maturity | Notes |
| ---- | -------- | --- | --- | ----- | ------- | ----------- | -------- | ----- |
| **Docker (Engine/Desktop)** | fs/net/proc (ns) | via Desktop VM | via Desktop+WSL2 VM | native | ~0.1–1s warm **[inferred]** | daemon; rootless mode exists | very mature | The de-facto common denominator. On mac/win it is a Linux VM under the hood. Bind-mount path syntax differs per host. |
| **Podman** | fs/net/proc (ns) | `podman machine` VM (uses Virtualization.framework or QEMU) **[verified]** | `podman machine` on WSL2 **[verified]** | native, rootless+daemonless by default **[verified]** | similar to docker | **no central daemon**; rootless by default | mature | Docker-CLI-compatible; on mac/win still needs a `podman machine` Linux VM. Listens on a Docker-API socket so docker-targeting code mostly works. macOS can't bind ports 80/443. **[reported]** |
| **containerd / nerdctl** | fs/net/proc (ns) | needs a VM | needs a VM | native daemon | low | daemon (containerd) | mature (k8s backbone) | Lower-level; `nerdctl` is the docker-like CLI. No real win/mac convenience story for a CLI. Skip for v1. |
| **Apple `container`** | fs/net/proc, **one microVM per container** via Virtualization.framework **[verified]** | macOS 26 (best); macOS 15 limited **[verified]** | no | no (Linux-guest only, runs *on* mac) | "<1s boot" **[reported]** | `container system start` apiserver (a daemon); no root **[verified]** | new (WWDC 2025, evolving) | **Bind mounts to host dirs are the worst case / limited; networking limited esp. on macOS 15** **[verified]** — disqualifying for a worktree-bind-mount design today. OCI-image compatible. Watch, don't adopt. |
| **Colima** | (drives docker) | darwin | no | no | VM boot once | manages a Lima VM | mature | A *daemon provider* for Docker on mac, not a separate isolation tech. agent-infra's mac default. |
| **OrbStack** | (drives docker) | darwin | no | no | fast VM | manages VM (proprietary) | mature, popular | Faster/lighter Docker+Linux VM on mac; hot resource resize. Not FOSS. |
| **Rancher Desktop** | (drives docker/containerd) | darwin | win | linux | VM boot | manages VM (moby or containerd) | mature | Another Docker-daemon provider; cross-platform. Equivalent role to Colima/OrbStack/Docker Desktop. |

**Interpretation:** Colima, OrbStack, Rancher Desktop, Docker Desktop, WSL2 are all *daemon providers* for the same `docker`/OCI surface — exactly what agent-infra's "engine" abstracts. They are not distinct isolation models. Podman is the one genuinely different runtime (daemonless/rootless) but presents the same CLI. Apple `container` is a distinct microVM model but is not usable for bind-mounted worktrees yet.

### A.2 MicroVM / VM-based (hardware-virt boundary; strongest isolation)

| Tech | Isolates | mac | win | linux | Startup | Daemon/root | Maturity | Notes |
| ---- | -------- | --- | --- | ----- | ------- | ----------- | -------- | ----- |
| **Firecracker** | full VM (KVM) | no | no | linux+KVM only | ~120–200ms cold **[reported]** | needs KVM; root-ish | mature (AWS Lambda, E2B) | No mac/win host support (needs KVM). Overkill for a local single-user CLI. |
| **gVisor (`runsc`)** | user-space kernel intercepting ~68 syscalls **[reported]** | no | no | linux | ~50–100ms **[reported]** | runs as OCI runtime; needs setup | mature (GKE, Modal) | 18% median syscall overhead **[reported]**. Linux-only. Strong fs/net via the intercept layer. Not a local-CLI fit. |
| **Kata Containers** | full VM per pod | no | no | linux+KVM | ~150–480ms **[reported]** | k8s/OCI runtime | mature | Heavyweight; cluster-oriented. Not for a laptop CLI. |
| **Cloud Hypervisor** | full VM (KVM) | no | no | linux+KVM | low | KVM | mature | Same KVM constraint. |
| **Apple Virtualization.framework** | full VM | darwin (Apple Silicon) | no | no | <1s **[reported]** | no daemon; user-level | mature | The substrate under Apple `container`, Podman-on-mac, Lima/Colima. Not directly a sandbox you'd shell to. |
| **WSL2** | Linux VM on Windows | no | win | no | VM boot | no root, but Windows feature | mature | The Windows Linux substrate; hosts Docker/Podman on Windows. agent-infra's win default. |
| **Lima** | Linux VM on mac | darwin | no | no | VM boot | user-level | mature | Generic Linux-VM provider on mac; Colima is Lima+docker-preset. |

**Interpretation:** All true microVMs require KVM (Linux) or Apple/Windows hypervisor and target servers/clusters. For a single-user laptop CLP they are the wrong weight class. The VM-based items that matter to Ark are the *substrates* (Virtualization.framework, WSL2, Lima) because they are how a Docker/Podman daemon comes to exist on mac/win — i.e. they live *inside* the engine-detection problem, not as user-facing backends.

### A.3 OS-native sandboxing (no daemon, no container, confine a process tree)

This is the row the Docker-only PLAN ignores and the one that most changes the cross-platform calculus.

| Tech | Isolates | OS | Daemon/root | Network isolation | Maturity | Notes |
| ---- | -------- | -- | ----------- | ----------------- | -------- | ----- |
| **macOS Seatbelt** (`sandbox-exec` / SBPL profile) | fs read/write, network | macOS | no daemon, no root **[verified]** | yes, deny-by-default in profile **[verified]** | mature but Apple-"deprecated" (still ships, widely used) **[reported]** | Used by **Codex CLI** and **Claude Code** on mac. Profile is Scheme-like SBPL; deny network by default, allow specific write paths. **[verified]** |
| **Linux Landlock** (LSM) | filesystem (per-path rw), + ABI4 TCP bind/connect, ABI6 unix-socket/signal scoping | Linux 5.13+ (fs); net ABI4=6.7, ABI5=6.10, ABI6=6.12 **[verified]** | **unprivileged**, no root, no daemon **[verified]** | partial: TCP port allow/deny only; **"network sandboxing still incomplete as of ABI v8; augment with seccomp"** **[verified]** | maturing fast | Used by Codex CLI (Landlock = fs) + seccomp (net). UDP/raw not covered by Landlock → seccomp needed to fully block egress. |
| **Linux seccomp-bpf** | syscall allow/deny (incl. `connect`/`bind`/`socket` for net) | Linux | unprivileged (no_new_privs) | yes, by blocking net syscalls **[verified]** | mature | Codex blocks net via seccomp; Anthropic's runtime uses seccomp BPF for unix-socket control. **[verified]** |
| **bubblewrap (`bwrap`)** | mount/pid/net/user namespaces; bind-mount a dir as the only writable root | Linux | unprivileged (uses user namespaces) **[verified]** | yes (`--unshare-net` or proxy) | mature (Flatpak) | **Claude Code's Linux backend** + **Anthropic sandbox-runtime** + Codex (bwrap+seccomp default). The pragmatic Linux primitive for "confine to one dir, no net." |
| **nsjail / firejail** | namespaces+seccomp wrappers | Linux | varies | yes | mature | Heavier-config alternatives to bwrap; not needed if bwrap chosen. |
| **Windows AppContainer** | fs/registry (deny-by-default, grant specific paths), low integrity | Windows | no root; programmatic Win32 API | capability-based (no broad net deny knob like Seatbelt) **[reported]** | mature (since Win8; Chromium/IE EPM) **[verified]** | Confining a process to a dir is possible but requires `DeriveCapabilitySidsFromName` + profile + token/job setup via Win32 — **non-trivial, no CLI equivalent of `sandbox-exec`/`bwrap`.** **[verified]** |
| **Windows Job Objects** | process resource/lifetime limits | Windows | no root | no | mature | Limits/kills process trees; **not a filesystem confinement** tool. Complements AppContainer. |
| **Windows Sandbox** | full disposable VM (Hyper-V) | Windows Pro/Ent | Windows feature; admin to enable | yes | mature | Throwaway desktop VM; not scriptable per-worktree bind-mount cleanly. Heavyweight. |

**Interpretation:** On mac + Linux there is a clean, **no-daemon, no-root** process-confinement story (Seatbelt; bwrap/Landlock+seccomp) — and it's exactly what Codex CLI and Claude Code already ship. Windows has **no equivalent lightweight, CLI-scriptable, network-denying file confinement**: AppContainer is capability-based and code-heavy, Windows Sandbox is a full VM. This asymmetry is the crux of the cross-platform decision (Section C).

### A.4 Agent/remote sandboxes (informative only — not local backends)

| Tech | Model | Why it's only informative |
| ---- | ----- | ------------------------- |
| **E2B** | Firecracker microVMs, hosted | Remote SaaS; self-host is enterprise. Not a local CLI confinement. |
| **Daytona** | Docker containers, sub-90ms create, hosted **[reported]** | Remote; confirms "containers are fine if startup is optimized." |
| **Modal sandboxes** | gVisor, hosted **[reported]** | Remote; validates gVisor for untrusted code at scale. |
| **microsandbox** | libkrun microVMs, **local-first/self-hosted**, ~<200ms **[reported]** | The closest to "local microVM"; v0.1.0 May 2025, libkrun = Linux/KVM + macOS HVF. Too young + Linux/mac-only to bet a CLI on, but the model (secrets never leave host) matches Ark's goal. Watch. |
| **Cloudflare/Deno isolates, WASM** | V8/Wasm sandboxes | For running *code snippets*, not a full agent CLI + git + arbitrary tools. Out of scope. |

**Takeaway for a local CLI:** the agent ecosystem's consensus is "untrusted agent code → isolated sandbox + outbound network allowlist." The hosted players use microVMs/gVisor because they're multi-tenant; a single-user laptop tool does not need that tenancy boundary and should pick the lightest thing that gives fs+net confinement on the host OS.

---

## B. Prior art & papers (sandboxing autonomous coding agents)

### B.1 The two agents Ark integrates already ship OS-native sandboxes

- **OpenAI Codex CLI** **[verified]** — mac: `sandbox-exec` + dynamically generated **Seatbelt/SBPL** profile, network denied by default, `.git`/`.codex`/`.agents` kept read-only even in workspace-write. Linux: a **separate helper** (`codex-rs/linux-sandbox/`, a Rust crate in the codex workspace) applying `PR_SET_NO_NEW_PRIVS` + **seccomp** network filter; **bubblewrap is the default**, Landlock is the "legacy fallback" for fs. Three modes: **ReadOnly / WorkspaceWrite / FullAccess**; net modes `Isolated / ProxyOnly / FullAccess`. This is the exact "process-level confinement" model — no Docker.
  - Sources: developers.openai.com/codex/agent-approvals-security; deepwiki.com/openai/codex/5.6-sandboxing-implementation; simonwillison.net/2025/Nov/9/codex-sandbox-investigation/.
- **Claude Code sandboxing** (shipped Oct/Nov 2025) **[verified]** — Linux **bubblewrap** + macOS **Seatbelt**; **two boundaries: filesystem (rw cwd only, deny outside) and network (egress only via a host-side proxy enforcing a domain allowlist; `socat`/unix-socket relay)**. Anthropic states *both* boundaries are required. **No Windows support mentioned.** Reported internal result: 84% fewer permission prompts. Threat model called out explicitly: **prompt-injected Claude modifying sensitive files, stealing SSH keys, phoning home, downloading malware.**
  - Sources: anthropic.com/engineering/claude-code-sandboxing; code.claude.com/docs/en/sandboxing.
- **`anthropic-experimental/sandbox-runtime`** ("`srt`") **[verified]** — a reusable, **no-container** sandbox library/CLI: macOS Seatbelt + Linux bubblewrap; **fs deny-then-allow for reads, allow-only for writes; network blocked by default with an allowlist enforced by host-side HTTP+SOCKS5 proxies**; seccomp-BPF for unix-socket control on Linux. **Written in TypeScript** (not Rust), exports `SandboxManager`. **Windows not yet supported.** This is the single best blueprint for a non-Docker Ark backend — and confirms the limits (no Windows).

### B.2 Threat-model / defense literature (2023–2026)

- **OWASP LLM01:2025 Prompt Injection** — ranked #1 risk; indirect injection (poisoned web pages, PDFs, **code comments in cloned repos**, transcripts) is the real coding-agent vector. **[verified]** Source: genai.owasp.org/llmrisk/llm01-prompt-injection/.
- **Real CVEs** **[reported]**: EchoLeak (CVE-2025-32711, M365 Copilot exfil via crafted email); GitHub Copilot CVE-2025-53773 (injection in public-repo comments → settings change → arbitrary code exec). These are concrete "prompt injection → code exec / exfil" instances that motivate kernel/OS confinement + egress allowlists.
- **"Design Patterns for Securing LLM Agents against Prompt Injections"** (arXiv 2506.08837, 2025) **[reported]** — pattern catalog; relevant pattern: confine tool execution + restrict egress.
- **"Prompt Injection Attacks on Agentic Coding Assistants"** (arXiv 2601.17548, 2026) **[reported]** — systematic vuln analysis of skills/tools/MCP ecosystems.
- **Industry guidance consensus** **[verified, multiple sources]**: run all tool execution in an isolated sandbox; **never run the agent as root**; enforce an **outbound network allowlist** so a compromised tool can't exfiltrate to arbitrary hosts; OpenAI (Dec 2025) concedes prompt injection is "unlikely to ever be fully solved" → defense-in-depth, not prevention.
- **Pierce Freeman, "A deep dive on agent sandboxes"** **[reported]** — argues Seatbelt (mac) + Landlock/seccomp (Linux) are the practical primitives; notes Seatbelt's "deprecated"-but-ubiquitous status and policy-complexity foot-guns; emphasizes protecting `.git` and home dotfiles from agent writes (mirrors Codex keeping `.git` ro).

**Cross-cutting finding for Ark:** the published threat model treats **network egress** as co-equal with filesystem confinement (exfiltration / phone-home). The current PLAN's container gives fs confinement and process confinement but **says nothing about network** — by default a Docker container has full outbound internet, so a yolo agent in Ark's box can still exfiltrate. Every serious agent sandbox (Claude Code, Codex, sandbox-runtime) adds an **egress allowlist**. This is a gap REVIEW should weigh (Section D risks), independent of the engine question.

---

## C. Cross-platform strategy

### C.1 Does any single backend cover all three OSes acceptably?

| Backend | mac | win | linux | One-binary CLI shell-out? | Verdict |
| ------- | --- | --- | ----- | ------------------------- | ------- |
| Docker/OCI via `docker` CLI | yes (Desktop/Colima/OrbStack VM) | yes (Desktop+WSL2) | yes (native) | yes (`docker` on PATH) | **Best common denominator** but requires a daemon/VM the user must install; path-mount syntax differs per host. |
| Podman | yes (machine) | yes (machine/WSL2) | yes (rootless, daemonless) | yes (`podman`, docker-API-compatible) | Strong **rootless** story on Linux; same VM caveat on mac/win as Docker. |
| OS-native (Seatbelt/bwrap/AppContainer) | yes (Seatbelt) | **weak** (AppContainer code-heavy; no `sandbox-exec` analog) | yes (bwrap/Landlock+seccomp) | partially (mac/linux yes; win is a Win32-API project) | **No-daemon, no-root, instant**, but **Windows is the hole**. |
| Apple `container` | macOS 26 only, **bind-mount limited** | no | no | yes | Not viable for bind-mounted worktrees today. |
| MicroVMs (Firecracker/gVisor/Kata) | no | no | linux only | n/a | Server/cluster tech; wrong weight class for a laptop CLI. |

**Conclusion:** No single backend is both lightweight and covers all three OSes. The realistic options are a spectrum:

1. **Container path (Docker/Podman):** uniform model across all three OSes, but every non-Linux host needs a Linux VM (Docker Desktop / Podman machine / WSL2 / Colima / OrbStack). This is the daemon-provider problem agent-infra's 5-engine abstraction solves. The PLAN's "docker on PATH required" is the **degenerate single-engine case** of that abstraction (= agent-infra's `docker-desktop`/`native` adapters), and works today on all three OSes *if the user has set up a Docker daemon*. What "breaks" vs. the assumption: (a) bind-mount path translation on Windows hosts (`C:\...` → `/mnt/c/...` or the engine's view) — the PLAN's `-v <wt>:/workspace` will not work verbatim on a Windows host path; agent-infra's `wsl2-paths.ts::toEnginePath/volumeArg` exists precisely for this; (b) the parent-`.git` ro mount + gitdir rewrite assumes a Unix path layout; (c) rootless-Docker uid/gid (agent-infra's `resolveBuildUid` returns 0/0 for rootless) — file ownership of worktree writes.

2. **OS-native path (Seatbelt/bwrap):** no daemon, instant, exactly Codex/Claude Code's model, and the agent **inherits the host `~/.claude` directly** (no volume, no login-in-box) — but **Windows has no clean equivalent**, so a Windows story still needs a container/WSL2 fallback.

### C.2 Is Docker/Podman the pragmatic common denominator? What breaks?

**Yes for "works on all three OSes," with these breakages to design around** (all **[verified]**/**[inferred]** as marked):
- **Windows host path → mount translation** [verified gap]: needs the equivalent of `volumeArg()`/`toEnginePath()`. The PLAN has none. This is the biggest concrete cross-platform bug in the current draft.
- **Daemon presence is not guaranteed** [verified]: `docker info` can fail even when `docker` is on PATH (daemon down, rootless mis-set, Desktop not started). agent-infra's `native.ts` shows the spread of failure guidance needed. The PLAN's `is_available()` (spawn probe) is weaker than `docker info`.
- **Rootless uid/gid ownership** [reported]: worktree files written inside a rootless container land as uid 0 unless handled.
- **No network confinement by default** [verified, design]: a plain `docker run` has open egress; doesn't meet the published agent threat model without `--network none` or a proxy.

### C.3 OS-native as a no-daemon alternative — limits

OS-native confinement (the Codex/Claude Code model) **can** confine a process tree to the worktree dir with **no container and no daemon** [verified]:
- **mac:** `sandbox-exec -p <SBPL>` — deny default, allow writes under the worktree, deny network. No root.
- **linux:** `bwrap` bind-mounting the worktree as the only writable path + `--unshare-net` (or proxy) + optional seccomp/Landlock. Unprivileged.
- **Limits** [verified]: (1) **Windows has no lightweight equivalent** — AppContainer is a Win32-API project, Windows Sandbox is a full VM. (2) **Network isolation is partial/awkward:** Landlock only does TCP bind/connect port allow/deny (incomplete through ABI v8 → needs seccomp); a true egress *allowlist* needs a host proxy (what Claude Code/sandbox-runtime do). bwrap `--unshare-net` is all-or-nothing. (3) **Reliability/policy complexity:** Seatbelt SBPL is fiddly and Apple-"deprecated"; profile mistakes silently over- or under-restrict. (4) **Credential exposure:** because the process inherits the host home, it inherits host `~/.claude`/`~/.codex` **directly** — convenient (no login-in-box) but means the agent can read host credentials unless the profile explicitly denies those paths (Codex denies `.codex`, etc.).

### C.4 What an engine-trait abstraction needs to look like (small but not Docker-locked)

The lesson from agent-infra is to **not** model "5 engines" up front. Ark's actual axis of variation is narrower than agent-infra's (Ark doesn't manage VM CPU/mem, doesn't build images, drops credential reconciliation). The minimal trait abstracts **the backend that creates/enters/removes a confined workspace**, with Docker as the first (only v1) implementor and an OS-native implementor as the obvious second.

Proposed Rust trait (name + 3–4 methods), see Section D for the recommended exact shape.

---

## D. Concrete recommendation for Ark v1

### D.1 `SandboxEngine` trait shape

Keep the trait tiny and verb-aligned with the four user commands. Do **not** import agent-infra's `ensure/startVm/stopVm/syncResources/defaultResources` surface — that exists to manage Docker daemons + VM resources, which Ark explicitly does not do (no `[sandbox.vm]`, pull-not-build).

```rust
// ark-core/src/commands/sandbox/engine.rs  (the abstraction)
pub trait SandboxEngine {
    /// Stable id used in config + labels, e.g. "docker", "native".
    fn id(&self) -> &'static str;

    /// Cheap precondition probe (binary present + backend reachable).
    /// Docker impl: `docker info`. Native impl: kernel/OS capability check.
    fn is_available(&self) -> Result<()>;            // Err = Error::SandboxBackendUnavailable

    /// Start a confined workspace for a resolved worktree; returns a handle/id.
    fn create(&self, spec: &SandboxSpec) -> Result<SandboxHandle>;

    /// Run argv (shell or yolo agent CLI) inside the confined workspace, TTY-attached.
    fn enter(&self, h: &SandboxHandle, argv: &[&str]) -> Result<i32>;

    /// Tear down the confined workspace (idempotent).
    fn remove(&self, h: &SandboxHandle, opts: &RemoveOpts) -> Result<RemoveOutcome>;

    /// Enumerate live Ark sandboxes for this backend.
    fn list(&self) -> Result<Vec<SandboxRow>>;
}
```

- `SandboxSpec` carries the OS-agnostic intent: `worktree_path`, `repo_git_dir`, `mount_git: bool`, `env_passthrough: Vec<(String,String)>` (already-resolved host values), `config_dirs` (agent config dir paths), `labels`. **Crucially, path-to-guest translation lives inside each engine impl**, not in `resolve.rs` — that's where the Windows `volumeArg`/`toEnginePath` logic belongs for the Docker impl, and where "no translation needed" is a no-op for a native impl.
- This is essentially the PLAN's existing `sandbox_create/enter/rm/list` free functions re-expressed as trait methods, plus an `id()`/`is_available()`. **v1 cost of introducing the trait now ≈ near-zero** (one `DockerEngine` struct), and it removes the Docker name from every signature.

### D.2 Which backend(s) v1 ships; cross-platform matrix

**Recommendation: v1 ships exactly one backend — Docker/OCI (`DockerEngine`) — but behind the `SandboxEngine` trait and named generically, with a `[sandbox] engine = "docker"` config key whose only accepted value today is `"docker"` (so the second backend is additive, not a breaking change).** This honors the maintainer's "not Docker-locked" intent while keeping v1 scope at roughly the current PLAN. **[inferred recommendation]**

| OS | v1 (`docker`) | Daemon provider the user supplies | Deferred backend |
| -- | ------------- | --------------------------------- | ---------------- |
| Linux | works (native daemon; rootless ok) | Docker Engine / Podman (docker-compat socket) | `native` (bwrap+seccomp), no-daemon |
| macOS | works | Docker Desktop / Colima / OrbStack | `native` (Seatbelt), no-daemon |
| Windows | works **only with mount-path translation** | Docker Desktop + WSL2 | (none clean; container stays the Windows story) |

**Must-add to v1 even in the Docker-only impl** (these are correctness, not new backends):
1. **Host→guest mount-path translation** for Windows hosts (port `wsl2-paths.ts` logic). Without it, `ark sandbox` is silently macOS/Linux-only despite the cross-platform ask.
2. **`docker info`-strength availability check** (not just spawn success), with per-OS guidance like `native.ts`.
3. Decide network posture (see D.5 risks) — at minimum document that the box has open egress, or add `--network none` as a default with an opt-out.

**Defer:** a `native` (Seatbelt/bwrap) no-daemon backend as **v2**. It is the highest-value follow-up because it removes the Docker dependency entirely on mac+Linux and matches Codex/Claude Code — but it is a meaningful chunk of new code (SBPL generation, bwrap arg construction, seccomp, proxy for egress) and has no Windows answer, so it should not block v1.

### D.3 Renames (`io/docker.rs` and the `docker/` template dir)

**Yes, rename — the maintainer's instinct is correct and cheap to honor now.**
- `io/docker.rs` → keep a thin **`io/docker.rs`** as the *Docker-specific* `Command::new("docker")` site (analogous to `io/git.rs` being git-specific), **but** put the backend-agnostic surface in `commands/sandbox/engine.rs` (the trait) with `commands/sandbox/engines/docker.rs` as the impl that calls `io::docker`. Rationale: `io/` is "the only sanctioned `Command::new(X)` site for tool X" (matches `io/git.rs`); a future `native` backend calls `sandbox-exec`/`bwrap`, which would be `io/sandbox_native.rs` or inline. So: **don't rename `io/docker.rs` to `io/sandbox.rs`** — that would conflate the engine abstraction (a `commands/sandbox/` concern) with the raw-process-spawn layer (an `io/` concern). Keep `io/docker.rs` honest about what it spawns. **[inferred]**
- **`docker/` template dir → `sandbox/`**: agreed, rename. The dir holds the image build source; naming it `sandbox/` keeps it backend-neutral (a future `native` backend needs no image, and a Podman variant could share the same Dockerfile). Low-risk doc/template rename; update the PLAN's Architecture block (`00_PLAN.md` lines 88-91) and any CI publish path.

Net module shape suggestion:
```
io/docker.rs                         # raw `docker` spawn (Docker-only site, like io/git.rs)
commands/sandbox/
  engine.rs                          # SandboxEngine trait + SandboxSpec/Handle + select_engine()
  engines/docker.rs                  # impl SandboxEngine for DockerEngine (calls io::docker, owns path-translation)
  config.rs                          # [sandbox] incl. `engine = "docker"`
  naming.rs resolve.rs create.rs enter.rs rm.rs list.rs   # thin: delegate to the selected engine
sandbox/Dockerfile                   # (renamed from docker/) image build source
```

### D.4 Credential implications per backend

| Backend | Credential mechanism | Implication |
| ------- | -------------------- | ----------- |
| **Docker (v1)** | Named volume holds the in-box `~/.claude` etc.; one-time `claude /login` in box persists across recreate (PLAN G-5/TR-3). Optional `-e ANTHROPIC_API_KEY`. | Clean separation: host creds never enter the box unless the user logs in inside. This is the **whole reason the volume trick exists** and it is **container-specific**. |
| **OS-native (deferred)** | Process inherits the **host home directly** → sees host `~/.claude`/`~/.codex` with **no volume, no login-in-box** [verified model: this is how Codex/Claude Code run]. | **Convenience win** (zero credential setup) but **a confinement consideration**: the agent can read (and the SBPL/bwrap profile must decide whether to allow) host credential files. Codex explicitly keeps `.codex` read-only. A native Ark backend would inherit creds for free but must choose whether to expose/deny `~/.claude`. **No volume-persistence code is needed at all** for this backend. |

**Recommendation:** keep the volume model for the Docker backend exactly as the PLAN has it. Document that the deferred native backend trades the volume for direct host-home inheritance (simpler, but the sandbox profile, not a volume boundary, becomes the credential boundary). This asymmetry is worth a one-line note in the SPEC so v2 doesn't reintroduce volume code where it's unneeded.

### D.5 Risks / unknowns for REVIEW to scrutinize

1. **Network egress is unaddressed [HIGH].** Every cited agent sandbox (Claude Code, Codex, sandbox-runtime) treats egress allowlisting as co-equal with fs confinement because the documented threat is exfiltration/phone-home. A vanilla Docker container has open internet. The PRD's "cage" claim (PRD line 11) is **only a filesystem/process cage, not a network cage** as drafted. REVIEW should decide: default `--network none`? a proxy allowlist (big scope)? or explicitly scope network confinement OUT with a documented warning? **[verified threat model; design gap]**
2. **Windows mount-path translation missing [HIGH].** As drafted, `-v <wt>:/workspace` will not work for a Windows host path. Either implement translation (port `wsl2-paths.ts`) or scope Windows as "via WSL2 only, run `ark` inside the WSL2 distro" and document it. **[verified gap]**
3. **`is_available()` too weak [MEDIUM].** Spawn-success ≠ daemon-reachable. Should be `docker info`. **[verified vs native.ts]**
4. **Engine-trait now vs later [MEDIUM, trade-off].** Introducing the trait in v1 costs ~one struct and de-Dockers the signatures (cheap insurance); *not* introducing it risks a churny rename when the native backend lands. Recommend introducing the trait shell in v1 (D.1) even though only `DockerEngine` is implemented. REVIEW to confirm the cost is acceptable vs. the PLAN's NG-3 ("no multi-engine abstraction").
5. **Apple `container` temptation [LOW].** Do not adopt for v1 — bind-mount + networking limits make it unfit for a worktree-mount design today. Note as "watch." **[verified]**
6. **Rootless-Docker file ownership [MEDIUM].** Worktree writes from inside a rootless container may land as uid 0; the PLAN doesn't address uid/gid mapping (agent-infra's `resolveBuildUid` does). **[reported]**
7. **Branch-name → container/volume identity collisions [LOW].** PLAN C-15 sanitizes slashes; confirm two branches that sanitize to the same string can't collide (e.g. `feat/x` vs `feat-x`). **[inferred]**
8. **`native` backend's incomplete network story [LOW, future].** If/when the native backend lands, Landlock alone can't fully block egress (incomplete through ABI v8); it needs seccomp + a proxy — i.e. the native backend is *more* code than it first appears. Flag so v2 scoping is honest. **[verified]**

---

## Caveats / Not found

- **Latency numbers** for gVisor/Firecracker/Kata and "Docker warm start" are **[reported]** from benchmark blogs, not primary measurement on Ark's target hardware; treat as order-of-magnitude only.
- **Apple `container` bind-mount status** is evolving (macOS 26 improved networking); the "bind mounts are the worst case" finding is current-as-of sources read this session and should be re-verified before any future adoption.
- **Codex Landlock ABI version / exact seccomp filter list** — not pinned down to a number this session; deepwiki notes Landlock is a "legacy fallback" behind bwrap and that rules live in `apply_permission_profile_to_current_thread`, but the precise ABI/ruleset was not extracted. If v2 ports this, read `codex-rs/linux-sandbox/` source directly.
- **Windows AppContainer as a CLI-scriptable confinement** — I found the Win32 API path (`DeriveCapabilitySidsFromName` + profile/token/job) but **no off-the-shelf `sandbox-exec`/`bwrap`-equivalent CLI**. If a no-daemon Windows backend is ever wanted, this needs deeper investigation (or accept Windows = container-only).
- **Podman as a drop-in for the Docker backend** — it's docker-API-compatible and could likely be driven by the same `io/docker.rs` via `DOCKER_HOST`/`podman` symlink, but I did **not** verify that Ark's specific arg vector (labels, `-v` ro `.git`, named volume) works unmodified under rootless Podman. Worth a spike if Podman support is desired.
- No academic paper specifically benchmarking *local single-user* agent confinement was found; the literature is about hosted/multi-tenant isolation and prompt-injection defense patterns. The local-CLI design space is currently defined by tool implementations (Codex, Claude Code, sandbox-runtime), not papers.

## Citations

- Codex CLI security/sandbox: https://developers.openai.com/codex/agent-approvals-security ; https://deepwiki.com/openai/codex/5.6-sandboxing-implementation ; https://simonwillison.net/2025/Nov/9/codex-sandbox-investigation/
- Claude Code sandboxing: https://www.anthropic.com/engineering/claude-code-sandboxing ; https://code.claude.com/docs/en/sandboxing
- Anthropic sandbox-runtime (no-container, Seatbelt+bwrap+proxy): https://github.com/anthropic-experimental/sandbox-runtime
- Apple container / Containerization: https://github.com/apple/container ; https://github.com/apple/containerization ; https://thenewstack.io/apple-containers-on-macos-a-technical-comparison-with-docker/
- Podman macOS/Windows + rootless: https://podman.io/docs/installation ; https://oneuptime.com/blog/post/2026-02-02-podman-machine-macos-windows/view
- microVM comparison (gVisor/Firecracker/Kata latency+overhead): https://northflank.com/blog/kata-containers-vs-firecracker-vs-gvisor ; https://johal.in/benchmark-gvisor-10-vs-kata-containers-30-vs/
- Landlock ABI/network maturity: https://docs.kernel.org/userspace-api/landlock.html ; https://man7.org/linux/man-pages/man7/landlock.7.html ; https://landlock.io/rust-landlock/landlock/enum.ABI.html
- Windows AppContainer / Job Objects: https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation ; https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer
- Agent sandbox landscape (E2B/Daytona/Modal/microsandbox): https://github.com/restyler/awesome-sandbox ; https://northflank.com/blog/daytona-vs-e2b-ai-code-execution-sandboxes ; https://pixeljets.com/blog/ai-sandboxes-daytona-vs-microsandbox/ ; https://modal.com/blog/top-code-agent-sandbox-products
- Threat model / prompt injection: https://genai.owasp.org/llmrisk/llm01-prompt-injection/ ; https://arxiv.org/pdf/2506.08837 ; https://arxiv.org/html/2601.17548v1 ; https://pierce.dev/notes/a-deep-dive-on-agent-sandboxes
- Pragmatic deep dive: https://pierce.dev/notes/a-deep-dive-on-agent-sandboxes
