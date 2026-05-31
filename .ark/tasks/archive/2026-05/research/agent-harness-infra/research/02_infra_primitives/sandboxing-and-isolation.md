# Sandboxing and Isolation

## What the primitive means

"Isolation" in a coding-agent harness is the answer to: *when the agent runs a
tool — edit, shell, network — what stops it from corrupting another agent's
work, breaking the host, or exfiltrating secrets?* It is not one mechanism but
a **stack** with four useful layers:

| Layer | Granularity | Mechanism | Failure it prevents |
| ----- | ----------- | --------- | ------------------- |
| Working directory | Filesystem path | `git worktree`, sibling clones | Two agents editing the same file. |
| Process / kernel | Linux namespaces | Docker, Podman, containerd, Bubblewrap | Shell escape, port collision, system-package install on host. |
| OS / hypervisor | Kernel | Firecracker microVMs (E2B), QEMU, gVisor (Modal) | Kernel-CVE lateral movement; untrusted-code execution. |
| Capability tokens | Per call | MCP roots, Claude's allow/deny rules | Tool access to off-limits files / URLs. |

Most harnesses pick *one* layer and live with the gaps. The interesting
designs *compose* them: worktree + container (Dagger's Container Use, Cursor
2.0 sandboxed terminals), worktree + microVM (E2B + Devin-class platforms).

## How leading harnesses implement it

### Claude Code (Anthropic)

- **Working dir.** Built-in `--worktree` flag (Claude Code Docs,
  `code.claude.com/docs/en/worktrees`); subagents declare `isolation: worktree`
  and the parent transparently spawns each in its own branch.
- **Process.** "Bubblewrap on Linux, Seatbelt on macOS, off by default"
  (Augment / Shayon writeup). The sandbox knob is `--dangerously-skip-permissions`
  for the *opposite* extreme; the default state is "ask the user before
  destructive ops."
- **Capability.** PreToolUse hooks return `{decision: allow|deny|ask|defer}`
  — finer-grained than coarse sandbox flags (Anthropic's hooks reference,
  `code.claude.com/docs/en/hooks`).

### OpenAI Codex CLI

- **Process.** Landlock + seccomp on Linux; "the only major agent with
  sandboxing **enabled** by default" (Shayon / Augment writeups). Workspaces
  are read-write; system paths are read-only.
- **Working dir.** No native worktree integration; users layer Container Use
  or shell scripts on top.
- **Capability.** Hooks (`developers.openai.com/codex/hooks`) gate
  PreToolUse / PostToolUse / Stop similarly to Claude.

### OpenHands (formerly OpenDevin)

- **Process.** Docker sandbox is the default and recommended option
  (`docs.openhands.dev/openhands/usage/sandboxes/docker`); the agent server
  itself runs *inside* a container.
- **Session persistence.** `keep_runtime_alive = 1` keeps the container across
  OpenHands restarts (OpenHands Issue #6382 documented the regression where
  rejoining recreated the container and lost changes — fixed in main).
- **Capability.** Tool access flows through MCP servers; microagents
  (`.openhands/microagents/`) gate by *trigger* (always / keyword / manual).

### Cursor 2.0 (October 2025)

- **Working dir.** Each of up to 8 concurrent agents gets a sandboxed
  workspace via either git worktrees or remote machines (Cursor changelog,
  `cursor.com/changelog/2-0`).
- **Process.** "Sandboxed terminals … with read/write access to the workspace
  but no network by default" — `cursor.com/blog/2-0`.
- **Capability.** Background / cloud agents add per-agent runtime envelopes,
  not just file isolation.

### Container Use (Dagger, 2025)

- **Composite design.** Each agent gets *both* a git worktree *and* a Docker
  container automatically (`github.com/dagger/container-use`); explicitly
  attacks the worktree-only failure mode where two agents collide on port
  3000.
- Quote: "Docker containers provide runtime isolation through Linux namespaces
  and cgroups that worktrees cannot match … Docker's namespace isolation
  prevents port conflicts in this class."

### E2B (Firecracker microVMs)

- Each sandbox is a **Firecracker microVM** with its own Linux kernel; cold
  start ~150 ms via pre-warmed snapshot pool (`addozhang.medium.com/ai-agent-code-execution-sandboxes-...`).
- "Two sandboxes share no kernel code paths whatsoever, fundamentally
  eliminating the possibility of kernel vulnerabilities propagating
  laterally" (E2B vs Modal writeup, `northflank.com/blog/e2b-vs-modal`).
- Scale: "E2B alone went from 40,000 sandbox sessions per month in March 2024
  to roughly 15 million per month by March 2025."

### Modal (gVisor)

- gVisor-based isolation rather than full microVM; sandbox is one product
  among inference / training / batch. Lower isolation strength than E2B's
  microVMs but tighter integration with broader compute.

### MCP roots (capability layer, transport-agnostic)

- "Roots are URIs (typically file:// paths) the client exposes to a server
  to define the allowable filesystem boundaries on the client machine"
  (`modelcontextprotocol.io/specification/2025-06-18/client/roots`).
- Capability negotiation at session open: client advertises
  `"capabilities": {"roots": {"listChanged": true}}`; server is forbidden from
  touching anything not in the declared list.
- Servers can `roots/list` and subscribe to changes; clients can revoke at
  any time. Permission lives on the *client* side — the inverse of POSIX
  capability checks.

## Tradeoff matrix

| Mechanism | File isolation | Net / port isolation | Kernel isolation | Cold start | Footprint |
| --------- | -------------- | -------------------- | ---------------- | ---------- | --------- |
| `git worktree` | Per-branch | None (shared host) | None | ms (`git worktree add`) | <1 MB |
| tmpfs scratch | Per agent | None | None | µs (mount) | RAM-bound |
| Docker / Podman | Per container | Per container | Shared host kernel | 0.5–2 s | 50–500 MB image |
| Bubblewrap / chroot | Per agent | None by default | Shared kernel | <100 ms | Tiny |
| gVisor (Modal) | Per sandbox | Per sandbox | User-mode kernel | ~500 ms | Moderate |
| Firecracker microVM (E2B) | Per VM | Per VM | Independent kernel | ~150 ms (pre-warm) | 5–50 MB / VM |
| MCP roots | Per server session | (orthogonal) | (orthogonal) | 0 ms | Header bytes |

Reading: **worktrees give you 90% of the win for 5% of the cost** when the
agent is trusted not to fork-bomb or open a listener; the moment either of
those matters, you have to compose with at least a container.

## What Ark does today

Ark's isolation surface is exactly **worktrees + capability hints**:

- **Worktrees** are first-class (`crates/ark-core/src/commands/agent/task/worktree/`).
  Layout fields:
  - `WORKTREES_DIR = ".ark/worktrees"` (`crates/ark-core/src/layout.rs:56`).
  - `Layout::worktree_dir(branch)` (`layout.rs:332`).
  - `Layout::slug_from_worktree_root` lexically decodes branch → slug
    (`layout.rs:352`).
- **Deep tier MUST use `--worktree`** — workflow contract in
  `.ark/workflow.md:295`. Standard / quick tiers may; research tier does not
  participate.
- **Per-checkout state** carries the per-worktree focus
  (`crates/ark-core/src/state/checkout/`): each worktree owns its own
  `.state.toml`, so two terminals literally cannot share a focus slug. This
  closes the "two agents on the same task slot" race that bare worktrees
  leave open.
- **No process or kernel isolation.** Ark inherits whatever the host platform
  (Claude Code, Codex, OpenCode) chooses — Bubblewrap / Seatbelt /
  Landlock-seccomp / Docker. Ark does not run agent processes itself, so it
  has no opinions to enforce here.
- **No capability tokens.** Ark does not introspect or gate individual tool
  calls; that lives in the platform layer (Claude's PreToolUse, Codex's
  hooks).

Compare to Container Use's worktree+container composite: Ark stops at the
worktree boundary because Ark is a *workflow* layer, not a *runtime*. The
runtime is whatever embedding harness ships.

## Directions for Ark

1. **Worktree post-create container hook.** `.ark/config.toml` already
   reserves `[worktree].post_create` (workflow.md:297). Land a worked example
   that boots a Docker / `devcontainer.json` per worktree, so users who *want*
   Container Use semantics can opt in without a separate tool. Code site:
   `crates/ark-core/src/commands/agent/task/worktree/`.
2. **Capability-aware `ark agent` surface.** Today `ark agent task new` etc.
   has no notion of "this caller is trusted." Add an optional capability
   token (env var or argv) that constrains which subverbs may run; useful
   when Ark is invoked from inside a sandboxed shell that wants Ark to do
   only `task verify`, not `task discard --force`. Code site:
   `crates/ark-core/src/commands/agent/state.rs` (transition table).
3. **MCP roots emitter.** When Ark is later exposed as MCP (see
   `mcp-and-tool-registries.md`), the natural `roots` it should advertise are
   `file://<root>/.ark/tasks/<active-slug>/` and the worktree dir — the
   *workflow* boundary, not the project boundary. This narrows blast radius
   without sacrificing usefulness.
4. **Per-worktree `.gitignore` for ephemeral scratch.** `.ark/.gitignore`
   (`layout.rs:65`) is fully Ark-owned; consider extending it with a
   `worktrees/*/scratch/` rule and exposing a `Layout::worktree_scratch_dir`
   for agents to dump intermediate artifacts that should never be staged.
   Cheaper than tmpfs and aligns with the install-snapshot story.
5. **Document the "worktree is not a sandbox" line.** Workflow.md currently
   treats worktrees as the isolation answer; a short prose note that
   worktrees give file isolation only — not network, port, or kernel — would
   prevent users from over-trusting them. Useful when a future SPEC adds a
   container hook so the boundary is explicit.

## Caveats / Not found

- I did not find a *built-in* containerization story in any of Aider, Cline,
  Continue. They appear to leave it to the user / the host editor.
- Cursor 2.0's exact sandbox runtime (Firecracker? gVisor? container?) is
  not disclosed publicly beyond "sandboxed terminals."
- The Devin platform's isolation primitives are not documented at the
  primitive level outside Cognition's marketing pages.

## Sources

- [Claude Code worktrees](https://code.claude.com/docs/en/worktrees)
- [OpenAI Codex CLI features](https://developers.openai.com/codex/cli/features)
- [OpenHands Docker sandbox](https://docs.openhands.dev/openhands/usage/sandboxes/docker)
- [Cursor 2.0 blog](https://cursor.com/blog/2-0)
- [Container Use (Dagger)](https://github.com/dagger/container-use)
- [E2B vs Modal](https://northflank.com/blog/e2b-vs-modal)
- [AI agent sandboxes: containers to microVMs](https://addozhang.medium.com/ai-agent-code-execution-sandboxes-isolation-from-containers-to-microvms-e80848effea5)
- [Let's discuss sandbox isolation (Shayon)](https://www.shayon.dev/post/2026/52/lets-discuss-sandbox-isolation/)
- [MCP Roots specification](https://modelcontextprotocol.io/specification/2025-06-18/client/roots)
- [Augment: What is an agent execution sandbox?](https://www.augmentcode.com/guides/agent-execution-sandbox)
