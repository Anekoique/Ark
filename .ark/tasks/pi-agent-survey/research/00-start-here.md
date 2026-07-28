# Pi for Ark and ArkOS — start here

Research snapshot: 2026-07-22.

Upstream identity: [`earendil-works/pi`](https://github.com/earendil-works/pi),
formerly `badlogic/pi-mono`. The historical URL currently redirects to the
canonical repository. The user's clean, read-only `reference/pi` checkout is
at `dd6bea41efa8caa7a10fe5a6401676dc5699f83f`, one bookkeeping commit after
the latest release,
[`v0.81.1`](https://github.com/earendil-works/pi/releases/tag/v0.81.1),
published 2026-07-21 from `20be4b1`.

## Executive answer

**Pi is useful to both projects, but in different ways.**

- For **Ark**, Pi is a credible future host and an excellent test case for
  Ark's move toward portable `AGENTS.md` + Agent Skills. It is **not** a
  registry-only fifth-platform addition: full Ark behavior would also require
  a Pi extension/package for context injection and Ark's scoped subagents.
  Recommendation: **prototype, do not ship first-class support yet**.
- For **ArkOS**, Pi is a strong open, programmatic coding-runtime candidate.
  Its provider layer, agent loop, session/compaction machinery, typed events,
  SDK, and JSONL RPC are directly relevant below an ArkOS adapter.
  Recommendation: **prototype a pinned runtime adapter now**, but keep Pi
  behind an ArkOS-owned interface.
- Pi is **not Ark, not ArkOS, and not a substitute for either**. It supplies a
  model/tool/session runtime and a runnable coding harness. It does not supply
  Ark's human-gated task lifecycle or ArkOS's task tree, SPEC service,
  grounding policy, recursion discipline, or canonical workflow event log.

The most useful near-term composition is therefore:

```text
                 workflow policy and durable artifacts
        ┌──────────────────────┴──────────────────────┐
        │                                             │
  Ark (human-gated)                          ArkOS (agent-facing)
        │ skills + extension                        │ adapter API
        └───────────────────┬─────────────────────────┘
                            │
                    Pi coding-agent / SDK / RPC
                            │
                       pi-agent-core
                            │
                          pi-ai
                            │
             model providers + OS-confined tools
```

This preserves the layer boundary in
[`docs/rfcs/001-arkos.md`](../../../../docs/rfcs/001-arkos.md): Pi can be a
runtime *hosted by* ArkOS, while ArkOS remains the workflow substrate.

## Decision register

| Candidate action | Call | Reason |
|---|---|---|
| Use Pi as a personal runtime beneath Ark in a disposable environment | **Prototype now** | Native `AGENTS.md`, Agent Skills, broad providers, and transparent sessions make the experiment cheap. |
| Add Pi immediately to Ark's shipping `PLATFORMS` registry | **Wait** | Full parity needs more than templates: stable command UX, context injection, subagent dispatch, project trust, upgrade/unload behavior, and tests. |
| Use Pi as the forcing test for Ark's skill-first/canonical-template direction | **Adopt as a design test** | Pi supports `AGENTS.md` and `SKILL.md`, but Ark lifecycle entry must remain an explicitly invoked command rather than a model-selected skill. |
| Copy Pi's agent loop, provider code, or session state into `ark-core` | **Reject** | Ark deliberately sits above host runtimes; this would collapse layers and add a Node/TypeScript runtime to a Rust workflow CLI. |
| Add Pi as an ArkOS stage-1 hosted runtime | **Prototype now** | The CLI, SDK, and RPC give ArkOS three adapter choices without making Pi the substrate API. |
| Embed `pi-agent-core` / `pi-ai` directly in ArkOS | **Watch / conditional prototype** | Attractive if ArkOS chooses Node/TypeScript; otherwise RPC is a cleaner language boundary. Pre-1.0 API churn remains material. |
| Build ArkOS on `pi-server` or `pi-storage-sqlite-node` now | **Reject now; watch** | Storage is new and the server explicitly says it is experimental and may change or disappear. |
| Reuse Pi's session tree as ArkOS's task tree | **Reject** | Conversation branching and workflow decomposition have different identity, consistency, and recovery semantics. |
| Reuse Pi's raw event stream as ArkOS's canonical event log | **Reject as-is** | Normalize it into an ArkOS-owned, versioned event schema with task/session correlation. |
| Treat Pi's subagent example as ArkOS recursion discipline | **Reject** | It is an optional process-spawning extension and an orchestration example, not a substrate policy or grounded halting rule. |

## What Pi actually contributes

The current repository is a layered toolkit, not a single terminal script:

| Package / surface | Verified upstream responsibility | Relevance here |
|---|---|---|
| `@earendil-works/pi-ai` | Unified provider/model API and credential/model discovery | Avoids writing a multi-provider compatibility layer for a runtime prototype. |
| `@earendil-works/pi-agent-core` | Agent loop, tool calls, state, transport, attachments | A small native runtime kernel beneath an adapter. |
| `@earendil-works/pi-coding-agent` | Runnable CLI plus `AgentSession`, `AgentSessionRuntime`, tools, resource loading, session management | Candidate Ark host and ArkOS runtime. |
| `@earendil-works/pi-tui` | Differential terminal UI | Useful for a personal CLI, not an ArkOS substrate dependency. |
| `@earendil-works/pi-storage-sqlite-node` | SQLite session repository/storage and materialized views | Interesting prior art; too new to become ArkOS storage. |
| `@earendil-works/pi-server` | Experimental server around the coding agent | Watch only; upstream disclaims stability. |
| SDK | In-process creation and control of sessions, models, tools, resources, and events | Best fit for a Node/TypeScript prototype. |
| RPC mode | Strict stdin/stdout JSONL commands, correlated responses, and streamed events | Best language-neutral boundary for a Rust or mixed-language ArkOS. |
| JSON event mode | Structured headless run output | Useful for experiments and trace capture, less capable than RPC. |
| Extensions | TypeScript lifecycle hooks, tools, commands, gates, UI, providers, and compaction customization | Enough to build a Pi-specific Ark adapter without patching Pi. |
| Skills / prompts / packages | Agent Skills, slash-expandable prompt templates, and distributable resource bundles | Strong alignment with Ark's portable-behavior direction. |

The detailed source and maintenance map is in
[`pi-upstream-architecture.md`](pi-upstream-architecture.md).

## Why the Ark fit is real but not trivial

### What already fits

These are verified Pi capabilities that already match Ark's direction:

1. Pi loads `AGENTS.md` while walking from the working directory to the
   filesystem root. Ark already owns a managed block in `AGENTS.md` for several
   hosts.
2. Pi implements the Agent Skills convention and can explicitly load existing
   Codex or Claude skill directories through `.pi/settings.json`, which fits
   portable non-lifecycle guidance.
3. Pi prompt templates support `$ARGUMENTS`, so most Ark command prose is
   mechanically portable.
4. A project-local Pi package can bundle extensions, skills, prompts, and
   themes, giving Ark one coherent integration artifact.
5. Pi's `before_agent_start` event can inject the output of `ark context`, and
   `tool_call` can implement approval/path gates. This is analogous to Ark's
   current host-specific context hooks/plugins.

### What remains hard

1. Pi prompt discovery is non-recursive by default and command names derive
   from filenames. A Pi extension can register a colon-bearing command name,
   so `/ark:<verb>` is structurally possible, but the exact namespace and
   argument contract still need an executable compatibility test.
2. Pi advertises discovered skills to the model, so a skill cannot be Ark's
   primary lifecycle entry without weakening Ark's explicit-invocation rule.
   The adapter's `ark:<verb>` command must remain the activation boundary.
3. Pi does not ship a built-in declarative subagent runtime matching Ark's
   contract. The official repository has an **example extension** that
   discovers Markdown agent profiles and spawns child `pi` processes. Ark
   would need to own, pin, test, and safely configure that bridge (or a smaller
   equivalent).
4. Project-local `.pi` resources require project trust. Headless modes cannot
   ask; the adapter must make approval policy explicit instead of silently
   enabling repository-controlled TypeScript.
5. Pi extensions run arbitrary code with the user's permissions. Pi's trust
   prompt is an input-loading gate, not process isolation.
6. Ark's [`Platform`](../../../../crates/ark-core/src/platforms.rs) abstraction
   can carry templates, whole files, managed blocks, hook descriptors, and
   agent trees, but Pi parity spans all of those at once. This is a larger
   maintenance commitment than the recent CodeAgent CLI registry entry.
7. Pi's context loader walks through every ancestor to the filesystem root.
   From an Ark worktree nested under the primary checkout, it therefore finds
   both the worktree and primary-checkout `AGENTS.md` files. An adapter must
   suppress native context discovery or explicitly de-duplicate those inputs.

Consequently, Pi should be the validation target for Ark's existing
skill-first and canonical-template ambitions, not a reason to duplicate a
fifth complete workflow family first. See
[`ark-integration-fit.md`](ark-integration-fit.md) for the full compatibility
matrix and implementation boundary.

## Why the ArkOS fit is stronger

Pi directly addresses the part the ArkOS RFC intentionally leaves to hosted
agent runtimes:

- model/provider normalization and authentication;
- the model/tool loop and streaming;
- process-embeddable and subprocess-controllable sessions;
- context usage, compaction, branching, retries, and abort;
- typed lifecycle/tool/message events; and
- local session persistence with a documented JSONL format.

Those surfaces let ArkOS concentrate on what Pi does not provide:

| ArkOS service | Pi input | Boundary ArkOS must retain |
|---|---|---|
| Lifecycle and task tree | Start/stop/resume/fork session operations | Task identity and legal workflow transitions remain ArkOS-owned. |
| Context service | Resource loader, `before_agent_start`, `context`, compaction hooks | ArkOS selects and versions task/SPEC/memory projections; Pi only transports them. |
| Memory | Session JSONL and optional SQLite session storage | Conversation history is evidence, not canonical semantic/procedural memory. |
| SPEC storage | Skills/context files can expose SPECs | SPEC authority, indexing, drift rules, and promotion remain ArkOS-owned. |
| Event log | SDK/RPC lifecycle, message, and tool events | Adapter normalizes to a stable ArkOS schema and adds task/workload correlation. |
| Grounding hooks | Tool results and `agent_settled` events | Evaluator independence and acceptance policy remain outside Pi. |
| Recursion discipline | Optional subagent process example | Decomposition, conflict isolation, budgets, halting, and reconciliation remain ArkOS services. |
| Portability surface | SDK/RPC and Agent Skills | ArkOS should still expose its own MCP/canonical API; Pi is one runtime adapter. |
| Isolation | Tool override hooks and external containerization guidance | ArkOS/host must enforce the OS boundary and credential/network policy. |

The important architectural rule is one-way dependency: an ArkOS Pi adapter
may depend on a pinned Pi contract; ArkOS workflow semantics must not depend on
Pi session-file details or extension conventions. The fuller analysis is in
[`arkos-runtime-fit.md`](arkos-runtime-fit.md).

## Risks that change the adoption call

### API and release stability

Pi is active and widely used: on the snapshot date GitHub showed roughly
75,000 stars, about 5,000 commits, and 249 releases. That is strong maintenance
evidence, but it is not a stability guarantee. Pi is at `0.81.x`; its own
release rules permit breaking API changes in minor releases. The repository
and npm scope also moved from Mario Zechner's historical names to Earendil's in
2026. Any adapter must pin a tested version and absorb change behind its own
contract.

The source also contains two runtime generations. The shipping coding agent
still uses `AgentSession` over the low-level `Agent`; the newer storage-backed
`AgentHarness` documentation marks important parity work such as automatic
compaction and parts of its session/hook surface as incomplete. This is another
reason to prototype against the released coding-agent RPC boundary rather than
designing ArkOS around the newer harness types today.

### Security and supply chain

Pi explicitly has no built-in filesystem/process/network/credential sandbox.
Extensions are arbitrary TypeScript, skills may direct arbitrary actions, and
project packages may install dependencies after trust. This is compatible
with Pi's local-tool philosophy, but it means:

- run the prototype inside [`ark sandbox`](../../../specs/features/ark-sandbox/SPEC.md)
  or an equivalent OS boundary;
- mount only the task worktree and the minimum credentials;
- pin Pi and every integration package;
- disable unattended package updates; and
- treat project trust as provenance approval, never as confinement.

### Governance and licensing

The repository and current packages are MIT. Earendil's public
[`RFC 0015`](https://rfc.earendil.com/0015/) commits the Pi core to remaining
MIT, while explicitly reserving the possibility of Fair Source or proprietary
adjacent/value-added services. Core reuse is license-compatible with Ark;
ArkOS should avoid making future hosted/server extras load-bearing.

### Runtime footprint

The current npm packages require Node `>=22.19.0`; standalone binaries are also
published. Direct SDK embedding therefore makes the most sense for a
Node/TypeScript ArkOS. RPC to a pinned standalone binary is the cleaner initial
boundary for a Rust or mixed-language implementation.

## Recommended experiment

Do one disposable, pinned adapter spike. It is a follow-up implementation task,
not part of this research task.

### Shape

1. Pin Pi `v0.81.1` (or a newer explicitly reviewed version when the spike
   begins) and record the binary/package checksum.
2. Run Pi inside an Ark sandbox/worktree with short-lived credentials and no
   host config mount.
3. Build one wholly owned `.pi/extensions/ark/` adapter subtree containing:
   - Ark command bodies generated from one canonical source and registered as
     exact `ark:<verb>` extension commands;
   - a small extension that selects the current checkout's instructions and
     injects `ark context` once per relevant session/phase;
   - the three Ark agent profiles; and
   - a pinned, narrowly scoped subagent bridge.
   This layout is auto-discovered by Pi while letting Ark remove its own
   subtree without touching sibling user prompts, extensions, or settings.
4. Exercise one quick task and one research or standard task. All structural
   mutations must still go through `ark agent`; the Pi extension must not own
   Ark lifecycle state.
5. In parallel, drive the same Pi version through RPC and normalize prompt,
   turn, tool, retry, compaction, abort, and settled events into a tiny
   ArkOS-shaped trace carrying both `task_id` and `runtime_session_id`.

### Acceptance criteria

- Explicit user invocation remains the only way an Ark workflow starts.
- `AGENTS.md`, every required Ark skill, and phase context are discoverable
  exactly once without copying stale prose into the system prompt, including
  from a nested Ark worktree.
- `ark-researcher`, `ark-reviewer`, and `ark-verifier` run with their intended
  read/write restrictions, isolated contexts, bounded concurrency, and useful
  failure propagation.
- A task can resume after restarting Pi without deriving Ark phase from chat
  history.
- `ark unload` / `load` round-trips every Ark-owned Pi artifact while
  preserving unrelated user Pi configuration.
- The sandbox prevents access outside the mounted worktree and minimum
  credential surface.
- The ArkOS trace can be replayed and correlated without parsing Pi's TUI or
  treating Pi's session tree as the task tree.
- Upgrading one Pi minor version either passes the adapter contract suite or
  fails at the adapter boundary with an actionable incompatibility.

### Stop / reject criteria

Stop first-class adoption if the proof requires patching Pi internals, silently
approving repository code, weakening Ark's human gates, making Pi's example
subagent implementation a workflow authority, or exposing Pi-specific types as
ArkOS's public substrate API. Also stop if safe unload/upgrade ownership cannot
be expressed without overwriting user `.pi` state.

## What to watch

Revisit the calls when any of these change:

- Ark lands a canonical skill/template emitter;
- Pi publishes a supported subagent contract rather than an example;
- Pi publishes a documented stable API tier (its current release policy has no
  major releases);
- Pi's server stabilizes and the SQLite storage package earns a supported
  compatibility contract;
- Pi adopts a first-class MCP client/server surface relevant to ArkOS; or
- an adapter contract test survives several Pi minor upgrades.

## Corpus map

- [`pi-upstream-architecture.md`](pi-upstream-architecture.md) — upstream
  identity, packages, runtime mechanics, extension model, security, licensing,
  and maintenance evidence.
- [`ark-integration-fit.md`](ark-integration-fit.md) — current Ark platform
  contract, compatibility matrix, integration cost, and Ark-specific calls.
- [`arkos-runtime-fit.md`](arkos-runtime-fit.md) — layer-correct ArkOS mapping,
  adapter options, event/session boundaries, risks, and de-risking spike.

## Primary evidence

- [Pi repository and package map](https://github.com/earendil-works/pi)
- local source snapshot: clean `reference/pi` checkout at `dd6bea41`
- [Pi documentation index](https://pi.dev/docs/latest)
- [SDK](https://pi.dev/docs/latest/sdk)
- [RPC mode](https://pi.dev/docs/latest/rpc)
- [extensions and lifecycle events](https://pi.dev/docs/latest/extensions)
- [Agent Skills](https://pi.dev/docs/latest/skills)
- [prompt templates](https://pi.dev/docs/latest/prompt-templates)
- [session format](https://pi.dev/docs/latest/session-format)
- [security and project trust](https://pi.dev/docs/latest/security)
- [Pi packages](https://pi.dev/docs/latest/packages)
- [official subagent example](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/subagent)
- [v0.81.1 release](https://github.com/earendil-works/pi/releases/tag/v0.81.1)
- [Earendil RFC 0015 — Pi licensing](https://rfc.earendil.com/0015/)

## Evidence labels

Statements describing Pi APIs, paths, versions, license, and documented
security boundaries are verified against the primary sources above at the
snapshot date. Calls such as **Prototype**, **Wait**, and **Reject** are this
corpus's architectural recommendations derived by comparing those facts with
the current Ark code/specs and ArkOS RFC; they are not upstream claims.
