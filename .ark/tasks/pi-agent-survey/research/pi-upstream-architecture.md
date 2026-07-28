# Research: Pi upstream architecture and project health

- Query: Survey the intended Pi agent harness upstream, verify its identity and current health, and map its packages, runtime loop, providers, tools, sessions, context/compaction, events, interfaces, extension system, security boundary, licensing, strengths, limitations, and open questions for a personal harness.
- Scope: external
- Date: 2026-07-22
- Inspection pin: `earendil-works/pi@dd6bea41efa8caa7a10fe5a6401676dc5699f83f` (commit timestamp 2026-07-21T18:40:11+02:00)

## Findings

Labels below distinguish repository facts from conclusions drawn from those facts:

- **Verified** — directly supported by the pinned checkout or a primary upstream page.
- **Inference** — source-derived interpretation, not an upstream guarantee.

### Identity and inspection boundary

- **Verified:** The project requested as `badlogic/pi-mono` is the Pi Agent Harness. The old GitHub URL redirects to `https://github.com/earendil-works/pi`, and `git ls-remote https://github.com/badlogic/pi-mono.git HEAD` resolved to the same SHA as the read-only local checkout: `dd6bea41efa8caa7a10fe5a6401676dc5699f83f`.
- **Verified:** The canonical repository and npm scope changed over time. Release `0.74.0` recorded the move to `earendil-works/pi-mono` and `@earendil-works/*`; the current canonical repository is `earendil-works/pi` (`reference/pi/packages/coding-agent/CHANGELOG.md:937-947`). Some current docs still contain `pi-mono` links, so repository-name strings alone are not reliable identity checks.
- **Verified:** The product package is `@earendil-works/pi-coding-agent`, described upstream as a minimal terminal coding harness (`reference/pi/packages/coding-agent/README.md:15-19`).
- **Verified:** `@mariozechner/pi` is a different npm package: a GPU-pod/vLLM deployment manager. It is not the coding-agent package surveyed here. Historical coding-agent packages used names such as `@mariozechner/pi-coding-agent`; the unqualified scoped package `@mariozechner/pi` must not be substituted for it.
- **Verified:** Raspberry Pi, Inflection Pi, `agentic-pi`, and other packages/products named “Pi” are outside this corpus. This report concerns only the repository whose README identifies it as “Pi Agent Harness” (`reference/pi/README.md:13-35`).

### Snapshot summary

| Surface | Verified state at the pin |
| --- | --- |
| Canonical upstream | `earendil-works/pi`; the requested `badlogic/pi-mono` URL redirects there |
| Source pin | `dd6bea41efa8caa7a10fe5a6401676dc5699f83f`; clean local `main`, matching remote HEAD |
| Latest release | `v0.81.1`, released 2026-07-21; tag `20be4b18d4c57487f8993d2762bace129f0cf7c6` |
| Runtime baseline | TypeScript monorepo; every inspected package declares Node `>=22.19.0`; standalone binaries are built with Bun |
| License | MIT in the repository and every inspected package |
| Primary shipped coding path | `pi-coding-agent` → `AgentSession` → low-level `Agent`/`agentLoop` → `pi-ai` provider |
| Persistence in the shipped CLI | Version-3 append-oriented JSONL session trees under `~/.pi/agent/sessions/` |
| Newer runtime track | `AgentHarness` plus abstract session storage, JSONL/in-memory implementations, and a separate Node SQLite backend |
| Trust model | Full local-user authority; project trust gates some project resources but is not a sandbox |
| Headless/embedding | Print, JSONL events, stdin/stdout JSONL RPC, in-process SDK; experimental process-supervisor server |

### Package and layer map

All six inspected package manifests are version `0.81.1`, MIT, and require Node `>=22.19.0`.

| Package | Role | Maturity/evidence |
| --- | --- | --- |
| `@earendil-works/pi-ai` | Provider, model, auth, streaming, token/cost, and cross-provider message layer | Public foundation; `reference/pi/packages/ai/package.json` and `packages/ai/src/models.ts:66-187` |
| `@earendil-works/pi-agent-core` | Low-level agent loop and stateful `Agent`; also exports the newer `AgentHarness` and session abstractions | Public foundation, but its harness migration remains incomplete; `packages/agent/src/index.ts:1-49` and `packages/agent/docs/agent-harness.md:256-339` |
| `@earendil-works/pi-coding-agent` | CLI, built-in coding tools, prompt/resource loading, JSONL session manager, compaction, extensions, TUI modes, SDK, and RPC | Main end-user harness; `packages/coding-agent/README.md:15-19` |
| `@earendil-works/pi-tui` | Component TUI with three-strategy differential rendering and synchronized output | Reusable UI library; `packages/tui/README.md:1-14,591-615` |
| `@earendil-works/pi-server` | IPC supervisor that owns multiple coding-agent RPC child processes; optional Radius integration | Explicitly experimental and removable without notice; `packages/server/README.md:1-5` |
| `@earendil-works/pi-storage-sqlite-node` | `node:sqlite` adapter plus SQLite session repository, migrations, and materialized views for agent-core sessions | Added in `0.81.0`; `packages/storage/sqlite-node/CHANGELOG.md:5-11` and `packages/storage/sqlite-node/README.md:1-5` |

The root README’s “All Packages” table lists only AI, agent core, coding agent, and TUI (`reference/pi/README.md:17-35`). The server and SQLite packages exist in the pinned tree but are omitted there. Their omission plus the server’s warning should prevent treating every workspace package as equally established.

```text
interactive terminal                 in-process application
        |                                      |
        v                                      v
pi-coding-agent modes  <----->  AgentSession / AgentSessionRuntime SDK
        |                         (same session abstraction for all modes)
        +---- JSON events
        +---- stdin/stdout RPC <----- RpcClient / experimental pi-server
        |                                      |
        v                                      v
low-level pi-agent-core Agent  <--------- one RPC child per server instance
        |
        v
pi-ai Models collection -> provider-owned auth/catalog/stream implementation

Parallel migration track (not the coding-agent substrate at this pin):
AgentHarness -> SessionStorage -> in-memory | JSONL | separate SQLite backend
```

- **Verified:** `AgentSession` says it is shared by interactive, print, and RPC modes and owns persistence, model/thinking state, compaction, bash, and branching (`packages/coding-agent/src/core/agent-session.ts:1-13`).
- **Verified:** `createAgentSession()` still constructs `new Agent(...)`, and the coding-agent source imports `Agent`, not `AgentHarness` (`packages/coding-agent/src/core/sdk.ts:1-18,294-320`).
- **Verified:** `AgentHarness` documentation calls itself the orchestration layer above the loop, but also calls its session facade planned, auto-compaction unimplemented, model registry planned, lifecycle work in progress, and generic hooks designed but unimplemented (`packages/agent/docs/agent-harness.md:1-22,188-228,256-339`).
- **Inference:** `AgentHarness` is an active replacement/generalization track, not the production substrate of the current coding-agent CLI. Evaluations must keep its promises and storage model separate from current `AgentSession` behavior.

### Agent loop and state ownership

- **Verified:** The low-level loop receives prompts, an `AgentContext`, configuration, an abort signal, and an injected streaming function. It emits events through an async sink (`packages/agent/src/agent-loop.ts:31-53,95-117`).
- **Verified:** A run has an outer follow-up loop and an inner tool/steering loop. Each turn streams an assistant response, validates and executes tool calls, appends tool results, emits `turn_end`, optionally updates context/model/thinking state, polls steering, and finally polls follow-ups (`packages/agent/src/agent-loop.ts:155-275`).
- **Verified:** Context transformation and conversion to provider-compatible messages occur immediately before each model call (`packages/agent/src/agent-loop.ts:281-312`). This is the principal context-policy seam.
- **Verified:** Tool calls default to parallel execution unless the loop or any selected tool requires sequential execution (`packages/agent/src/agent-loop.ts:411-425`; `packages/agent/src/agent.ts:207-230`). Argument preparation/schema validation and the block-capable `beforeToolCall` hook occur before execution (`packages/agent/src/agent-loop.ts:600-663`).
- **Verified:** If an assistant response ends because of length, every included tool call is failed instead of executing possibly truncated arguments (`packages/agent/src/agent-loop.ts:202-216`).
- **Verified:** `Agent` owns the live transcript, model, prompt, tools, streaming state, two queues, and lifecycle listeners. Listener promises are awaited in registration order and remain part of run settlement (`packages/agent/src/agent.ts:165-245,529-575`).
- **Verified:** The low-level `Agent` has no persistence responsibility. Current persistence belongs to coding-agent `AgentSession`; the newer `AgentHarness` owns a separate storage-backed `Session` abstraction.

### Providers, models, and authentication

- **Verified:** A `Provider` owns identity/base metadata, authentication, model listing/refresh/filtering, and streaming. A `Models` collection owns provider registration, auth resolution, lookup, refresh, and delegation (`packages/ai/src/models.ts:66-187`).
- **Verified:** Built-in provider factories at the pin cover Amazon Bedrock, Ant Ling, Anthropic, Azure OpenAI Responses, Cerebras, Cloudflare AI Gateway and Workers AI, DeepSeek, Fireworks, GitHub Copilot, Google and Vertex, Groq, Hugging Face, Kimi Coding, MiniMax (global/China), Mistral, Moonshot (global/China), NVIDIA, OpenAI, OpenAI Codex, OpenCode and OpenCode Go, OpenRouter, Qwen Token Plan (global/China), Radius, Together, Vercel AI Gateway, xAI, Xiaomi and three regional token plans, ZAI, and ZAI Coding China (`packages/ai/src/providers/all.ts:5-43,84-135`).
- **Verified:** Provider catalogs may be static or dynamically refreshed and cached; refresh errors are returned per provider without rejecting the entire collection (`packages/ai/src/models.ts:91-111,131-153`).
- **Verified:** Current coding-agent auth resolution order is runtime override, stored `auth.json`, environment variables, then custom fallback. An SDK host may inject its own credential store and model paths (`packages/coding-agent/docs/sdk.md:433-470`).
- **Verified:** Extensions can now register complete providers, including auth, model refresh/filtering, and streaming; this was featured in release `0.81.0`.
- **Inference:** Broad provider coverage is a strength, but the compatibility surface is intrinsically high-churn: provider payloads, model metadata, auth flows, and retry semantics produce a large share of changelog fixes. A personal harness embedding Pi should pin versions and test the exact providers it relies on.

### Tools and execution environment

- **Verified:** The complete built-in tool vocabulary is `read`, `bash`, `edit`, `write`, `grep`, `find`, and `ls`. The default coding set is only `read`, `bash`, `edit`, and `write`; a read-only bundle uses `read`, `grep`, `find`, and `ls` (`packages/coding-agent/src/core/tools/index.ts:81-94,138-183`).
- **Verified:** Tools are replaceable and extensible through both the SDK and TypeScript extensions. A host can disable all built-ins, disable just default built-ins, or allow/exclude named tools (`packages/coding-agent/docs/sdk.md:491-509`).
- **Verified:** File paths may be absolute or `~`-expanded and are not confined to the working directory by the path resolver (`packages/coding-agent/src/core/tools/path-utils.ts:40-50`).
- **Verified:** Tool output truncation defaults to 2,000 lines or 50 KiB (`packages/coding-agent/src/core/tools/truncate.ts:1-13`). Same-file mutations are serialized while distinct files may run concurrently (`packages/coding-agent/src/core/tools/file-mutation-queue.ts:28-60`).
- **Verified:** The default system prompt is assembled from the enabled tools, concise generic guidelines, project context files, skills metadata, and current working directory (`packages/coding-agent/src/core/system-prompt.ts:79-159`).
- **Verified:** Pi intentionally does not ship first-class MCP, sub-agents, or plan mode. Upstream expects extensions/packages or external processes to supply those policies (`packages/coding-agent/README.md:15-19,495-501`).

### Sessions, context, branching, and compaction

There are two related but currently distinct persistence stacks.

#### Shipped coding-agent stack

- **Verified:** Sessions auto-save as JSONL files beneath `~/.pi/agent/sessions/`, partitioned by working directory; `--no-session` is ephemeral (`packages/coding-agent/docs/sessions.md:3-20`).
- **Verified:** Current format version is 3. Entries form a tree through `id`/`parentId`, and versions 1/2 are migrated on load (`packages/coding-agent/docs/session-format.md:1-27`; `packages/coding-agent/src/core/session-manager.ts:30`).
- **Verified:** `/tree` moves the active leaf within one file; `/fork` and `/clone` create files. Branch summaries can retain information from an abandoned path (`packages/coding-agent/docs/sessions.md:69-139`).
- **Verified:** `custom` entries persist extension state but do not enter model context; `custom_message` entries do (`packages/coding-agent/docs/session-format.md:261-282`).
- **Verified:** Context reconstruction walks the leaf-to-root path, honors the newest compaction checkpoint, then projects entries into model messages (`packages/coding-agent/docs/session-format.md:304-340`).
- **Verified:** Automatic compaction triggers when estimated context tokens exceed `contextWindow - reserveTokens`; defaults are a 16,384-token reserve and a 20,000-token recent tail. It never cuts at a tool result (`packages/coding-agent/docs/compaction.md:27-45,79-117,381-400`).
- **Verified:** Compaction is lossy for active model context but not destructive to the underlying JSONL history. Summary serialization truncates each tool result to 2,000 characters (`packages/coding-agent/docs/compaction.md:255-269`).
- **Verified:** The implementation uses synchronous append/write filesystem operations for the session file (`packages/coding-agent/src/core/session-manager.ts:984-1040`).

#### Agent-core harness stack

- **Verified:** Agent core exports storage-backed `Session`, in-memory and JSONL repositories/storage, compaction primitives, skills, prompt templates, and system-prompt utilities (`packages/agent/src/index.ts:6-43`).
- **Verified:** Its session context derives branch-scoped model, thinking level, and active tools, and treats `retainedTail` compactions as self-contained checkpoints (`packages/agent/src/harness/session/session.ts:39-95,150-228`).
- **Verified:** The separate SQLite package adapts Node’s synchronous `node:sqlite` API and re-exports SQLite session storage/repository support (`packages/storage/sqlite-node/src/index.ts:1-3,48-97`).
- **Verified:** Upstream explicitly defines the realistic target as “semi-durable”: the append-only session is durable, while the host recreates tools, providers, extensions, resources, and prompt callbacks. Provider streams cannot resume; recovery restarts from a durable boundary (`packages/agent/docs/durable-harness.md:9-24,120-152`).
- **Inference:** JSONL is a transparent, portable personal-session format; SQLite is better suited to indexed/multi-session consumers. Neither makes live tool calls or provider streams crash-resumable, and the detailed recovery journal remains a design/spike rather than a shipped guarantee (`packages/agent/docs/durable-harness.md:194-212`).

### Events and observability

- **Verified:** Low-level events cover agent, turn, message-stream, and tool-execution lifecycles (`packages/agent/src/types.ts:405-437`).
- **Verified:** Coding-agent adds queue, persistence, model/thinking, compaction, automatic retry, summarization retry, and final-settlement events (`packages/coding-agent/src/core/agent-session.ts:138-179`).
- **Verified:** Extension hooks additionally span project trust, startup/resource discovery, input, pre-agent prompt mutation, context mutation, provider headers/request/response, tool blocking/result mutation, session switching, compaction, and tree navigation (`packages/coding-agent/docs/extensions.md:273-348,648-709`).
- **Verified:** `agent_end` does not necessarily mean quiescence because retry, compaction, or follow-up work may continue; `agent_settled` is the integration-level idle event (`packages/coding-agent/docs/extensions.md:558-571`).
- **Verified:** In parallel tool mode, updates may interleave and completion events follow completion order, while final tool-result messages are emitted in assistant source order (`packages/coding-agent/docs/extensions.md:624-646`).
- **Inference:** The event surface is sufficient for rich observability and policy injection, but consumers must choose the correct semantic layer (`AgentEvent`, `AgentSessionEvent`, or extension events) and must not equate end-of-run with settled state.

### User interfaces, SDK, RPC, and server

- **Verified:** CLI dispatch selects interactive, text print, JSON event stream, or RPC based on flags and TTY state (`packages/coding-agent/src/main.ts:100-114`; `packages/coding-agent/docs/usage.md:166-180`).
- **Verified:** JSON mode is a one-way JSONL event stream. RPC is bidirectional JSONL over stdin/stdout with strict LF framing, correlated commands/responses, asynchronous events, steering/follow-up, abort, session/model/compaction/tree controls, and extension-UI messages (`packages/coding-agent/docs/json.md:1-27`; `packages/coding-agent/docs/rpc.md:1-37,39-130`).
- **Verified:** For Node/TypeScript, upstream recommends the in-process `AgentSession` API rather than a child process. `AgentSessionRuntime` is the replacement layer for new/resume/fork/import and is also used by built-in modes (`packages/coding-agent/docs/sdk.md:44-121`).
- **Verified:** `DefaultResourceLoader` is injectable, so an embedding host can replace standard extension, skill, prompt, theme, context, and system-prompt discovery (`packages/coding-agent/docs/sdk.md:44-64,330-365`).
- **Verified:** The experimental server exposes spawn/list/status/stop and request/stream bridges, and each instance spawns a coding-agent RPC subprocess (`packages/server/src/handler.ts:49-160`; `packages/server/src/rpc-process.ts:37-60`). It listens through a local IPC socket and can optionally register with Radius (`packages/server/src/serve.ts:9-37`).
- **Inference:** The SDK is the cleanest in-process composition surface; RPC provides process isolation and language neutrality; the server should not yet be treated as a stable daemon contract.

### Extensions, skills, prompts, and context files

- **Verified:** Extensions are TypeScript modules loaded with `jiti`; they can register tools, commands, shortcuts, flags, UI components, renderers, providers, and lifecycle hooks, and persist custom session entries (`packages/coding-agent/docs/extensions.md:3-29,109-181`).
- **Verified:** Global and project extensions can be discovered automatically and reloaded. Project-local extensions load only after project trust; global/CLI extensions participate in the trust decision itself (`packages/coding-agent/src/core/resource-loader.ts:330-353`).
- **Verified:** Skills follow the Agent Skills standard leniently. Pi puts only skill metadata in the system prompt and relies on `read` or `/skill:name` to load full instructions on demand (`packages/coding-agent/docs/skills.md:3-7,20-41,64-82`).
- **Verified:** Prompt templates are Markdown slash-command expansions with positional/default/slice arguments. Project templates require trust; normal `prompts/` discovery is non-recursive (`packages/coding-agent/docs/prompt-templates.md:3-17,65-96`).
- **Verified:** `DefaultResourceLoader` loads one `AGENTS.md` or `CLAUDE.md` from the global agent directory and each ancestor directory from cwd to filesystem root (`packages/coding-agent/src/core/resource-loader.ts:67-119`). This differs from project skill discovery, which stops at a Git root when present.
- **Inference:** Pi’s customization system is unusually direct for a personal harness, but TypeScript extensions are executable plugins rather than declarative capabilities. Extension review, version pinning, and provenance are part of the trusted computing base.

### Security and trust boundary

- **Verified:** Pi runs inside the launching user’s OS security boundary and deliberately has no built-in sandbox or permission system. Upstream directs users to containers, VMs, micro-VMs, or policy sandboxes for stronger boundaries (`reference/pi/SECURITY.md:6-22,48-68`; `packages/coding-agent/docs/security.md:31-53`).
- **Verified:** Built-in tools can read/write/edit and run arbitrary shell commands with process permissions. Extensions also execute arbitrary code with those permissions (`packages/coding-agent/docs/security.md:31-37`; `packages/coding-agent/docs/extensions.md:109-152`).
- **Verified:** Project trust gates `.pi` settings, packages, extensions, skills, prompts, themes, and system-prompt files. It does not gate `AGENTS.md`/`CLAUDE.md`; those load unless context loading is disabled (`packages/coding-agent/docs/security.md:5-29`).
- **Verified:** In non-interactive modes, an unresolved `ask` decision fails closed for trust-gated resources. Explicit CLI overrides, saved ancestor decisions, or global defaults can change that behavior (`packages/coding-agent/src/core/project-trust.ts:46-95`).
- **Verified:** The security policy explicitly treats prompt injection, untrusted repositories, installed extensions/skills, public internet exposure, and sandbox absence as outside Pi’s vulnerability boundary (`reference/pi/SECURITY.md:48-68`).
- **Inference:** “Project trusted” means permission to load project configuration/code, not permission for an individual tool operation. A personal unattended harness needs an external sandbox and/or a blocking tool hook; project trust alone is not an adequate execution policy.

### License, governance, releases, and maintenance health

- **Verified:** The repository license is MIT, copyright Mario Zechner 2025 (`reference/pi/LICENSE:1-20`), and each inspected package manifest declares MIT.
- **Verified:** Earendil RFC 0015 (2026-03-30) says Earendil acquired Pi and Mario Zechner joined the company. It commits the Pi core to remain MIT, while allowing future adjacent/value-added features to be Fair Source or proprietary.
- **Verified:** GitHub marked `v0.81.1` latest on 2026-07-21. It added deterministic checksummed source archives and retry lifecycle behavior for compaction/branch summarization. The inspected HEAD is one post-release “Unreleased” commit later.
- **Verified:** On 2026-07-22, GitHub showed approximately 74.7k stars, 9.2k forks, 51 open issues, 15–16 open pull requests, 249 GitHub releases, and 5,046 commits. The local history contained 305 `v*` tags; tag count and GitHub release count are different metrics.
- **Verified:** The local 30-day window beginning 2026-06-22 contained 373 commits from 31 distinct author names. The latest twelve commits included work by Mario Zechner, Armin Ronacher, David Brailovsky, Christian Klotz, and Cristina Poncela Cubeiro.
- **Verified:** Contribution policy says the core should remain minimal and extensible. New contributors’ issues and PRs are auto-closed by default, maintainers review the buffer daily, and `npm run check` plus `./test.sh` are required (`reference/pi/CONTRIBUTING.md:5-17,21-34,56-71`).
- **Inference:** Release cadence and recent multi-author activity indicate active maintenance. However, issue count is not a clean backlog/health measure because the contribution gate deliberately suppresses new-contributor tracker entries. Rapid `0.x` releases and active runtime migrations also imply meaningful API churn.
- **Inference:** Current core licensing is favorable for a personal fork/harness. The stewardship/acquisition change is a governance dependency, and future commercial adjacent features should not be assumed MIT merely because this repository is.

### Strengths for a personal harness

- **Inference:** Clear layers: provider plumbing, low-level loop, coding policy/session layer, TUI, and multiple integration surfaces can be adopted independently.
- **Inference:** The low-level loop is small enough to reason about yet exposes steering/follow-up queues, parallel/sequential tools, pre/post tool hooks, per-call context transforms, abort, and awaited events.
- **Inference:** Provider breadth, provider-owned authentication, custom provider registration, and runtime-injected credential/model stores reduce vendor lock-in.
- **Inference:** Transparent JSONL tree sessions, branching, compaction checkpoints, extension entries, and an optional storage abstraction are strong foundations for inspectable personal history.
- **Inference:** The same `AgentSession` powers interactive, print, JSON, RPC, and SDK modes, reducing behavioral drift among interfaces.
- **Inference:** TypeScript extensions, progressive-disclosure skills, prompt templates, context files, custom UI, and provider hooks allow substantial customization without maintaining a fork.
- **Inference:** The project documents its security non-goals plainly and provides supply-chain hardening practices and reproducible/checksummed release sources.

### Limitations and adoption risks

- **Inference:** Default execution is deliberately high-trust: unrestricted file paths, shell, network inherited through tools/processes, credentials, prompt injection, and arbitrary extension code. Safe unattended use requires external containment and explicit policy.
- **Inference:** There are two session/orchestration generations. Current coding-agent durability should not be inferred from `AgentHarness` design documents, and code built directly on the newer harness may track unfinished APIs.
- **Inference:** Current JSONL persistence preserves completed entries, not an in-flight provider stream or arbitrary tool side effects. Crash recovery cannot safely replay non-idempotent tools without application policy.
- **Inference:** The server is explicitly unstable, and direct RPC consumers own process lifecycle, strict framing, trust configuration, and extension-UI request handling.
- **Inference:** The built-in feature set intentionally omits MCP, sub-agents, and plan mode. Extensions make these possible, but that moves compatibility, security, and UX responsibility to the adopter.
- **Inference:** Node `>=22.19` and TypeScript/Bun-oriented packaging may be a portability constraint for older systems or non-JavaScript embedding hosts; RPC is the escape hatch.
- **Inference:** Fast provider/model churn and a pre-1.0 version line make pinning, changelog review, and provider-specific regression tests important.
- **Inference:** Session summaries necessarily lose detail in active context, and summarization input truncates large tool results. The complete JSONL remains available, but the model will not see all of it after compaction.

### Unanswered questions for a personal-harness decision

1. Which API is the intended long-lived embed contract after `AgentHarness` becomes migration-ready: current `AgentSession`, `AgentHarness`, or a compatibility facade?
2. What migration tooling will exist between coding-agent v3 JSONL sessions and agent-core `SessionStorage`/SQLite sessions, including custom extension entries?
3. What exact crash-consistency guarantees do JSONL and SQLite storage provide for partial writes, durable queues, pending extension writes, and leaf updates?
4. Will the planned harness hook/event mechanism reach feature parity with coding-agent extensions, and which event names/results will remain compatible?
5. What is the server’s intended authentication, authorization, socket-permission, remote Radius, multi-user, and API-stability model?
6. How are `auth.json`, OAuth refresh tokens, session logs, and provider payload metadata protected at rest on each supported OS?
7. Which telemetry, update-check, provider-attribution, and catalog-refresh requests occur by default in each mode, and how should an offline/private harness disable them comprehensively?
8. What are the compatibility guarantees for provider/model identifiers and for session replay after switching providers or upgrading model metadata?
9. What extension/package signing, provenance, lockfile, and revocation mechanisms—if any—are planned beyond trusted-source review and pinned dependencies?
10. What benchmarked limits exist for very large JSONL trees, compaction frequency, SQLite concurrency, parallel tools, and long-running unattended sessions?

### Files (internal)

The paths below are inside the read-only upstream mirror at the pinned commit; they are not Ark implementation files.

| Path | Description |
| ---- | ----------- |
| `reference/pi/README.md` | Canonical project identity, package overview, security posture, build/release hardening, license |
| `reference/pi/SECURITY.md` | Upstream security boundary and explicit out-of-scope risks |
| `reference/pi/CONTRIBUTING.md` | Minimal-core philosophy, contribution gate, required checks |
| `reference/pi/LICENSE` | MIT license text |
| `reference/pi/packages/ai/src/models.ts` | Provider and Models collection contracts |
| `reference/pi/packages/ai/src/providers/all.ts` | Built-in provider registry at the pin |
| `reference/pi/packages/agent/src/agent-loop.ts` | Low-level LLM/tool/queue loop |
| `reference/pi/packages/agent/src/agent.ts` | Stateful Agent wrapper and awaited event reduction |
| `reference/pi/packages/agent/src/types.ts` | Agent context and lifecycle event types |
| `reference/pi/packages/agent/docs/agent-harness.md` | New orchestration lifecycle, implemented/planned status |
| `reference/pi/packages/agent/docs/durable-harness.md` | Semi-durability boundaries and recovery design |
| `reference/pi/packages/agent/src/harness/session/session.ts` | Storage-backed branch/context projection |
| `reference/pi/packages/coding-agent/src/core/sdk.ts` | Construction of current AgentSession/Agent runtime |
| `reference/pi/packages/coding-agent/src/core/agent-session.ts` | Shared mode-neutral coding session and extended events |
| `reference/pi/packages/coding-agent/src/core/session-manager.ts` | Current v3 JSONL session tree persistence |
| `reference/pi/packages/coding-agent/src/core/resource-loader.ts` | Extension/skill/prompt/theme/context discovery and trust bootstrap |
| `reference/pi/packages/coding-agent/src/core/tools/index.ts` | Built-in tool registry and default/read-only bundles |
| `reference/pi/packages/coding-agent/docs/extensions.md` | Executable extension API and lifecycle hooks |
| `reference/pi/packages/coding-agent/docs/skills.md` | Skill discovery and progressive disclosure |
| `reference/pi/packages/coding-agent/docs/prompt-templates.md` | Prompt-template discovery and expansion |
| `reference/pi/packages/coding-agent/docs/session-format.md` | JSONL tree schema and context reconstruction |
| `reference/pi/packages/coding-agent/docs/compaction.md` | Trigger, cut-point, summary, and configuration semantics |
| `reference/pi/packages/coding-agent/docs/sdk.md` | In-process embedding surface |
| `reference/pi/packages/coding-agent/docs/rpc.md` | Headless JSONL protocol |
| `reference/pi/packages/server/src/rpc-process.ts` | Per-instance coding-agent subprocess bridge |
| `reference/pi/packages/server/src/handler.ts` | Supervisor IPC/RPC operations |
| `reference/pi/packages/storage/sqlite-node/src/index.ts` | Node SQLite adapter/export surface |

### Code patterns

The following short excerpts carry core architectural rules.

`reference/pi/packages/agent/src/agent-loop.ts:169-174` — nested continuation semantics:

```ts
// Outer loop: continues when queued follow-up messages arrive after agent would stop
while (true) {
  // Inner loop: process tool calls and steering messages
```

`reference/pi/packages/agent/src/agent-loop.ts:288-300` — context policy is applied at the LLM boundary:

```ts
let messages = context.messages;
messages = await config.transformContext(messages, signal);
const llmMessages = await config.convertToLlm(messages);
```

`reference/pi/packages/agent/src/agent-loop.ts:619-640` — policy hooks can block tools:

```ts
if (config.beforeToolCall) {
  const beforeResult = await config.beforeToolCall(...);
  if (beforeResult?.block) return createErrorToolResult(...);
}
```

`reference/pi/packages/coding-agent/src/core/agent-session.ts:2-13` — one session layer backs all shipped modes:

```ts
 * This class is shared between all run modes (interactive, print, rpc).
 * Modes use this class and add their own I/O layer on top.
```

`reference/pi/packages/coding-agent/src/core/sdk.ts:294-302` — current coding-agent constructs the low-level Agent:

```ts
agent = new Agent({
  initialState: { systemPrompt: "", model, thinkingLevel, tools: [] },
  convertToLlm: convertToLlmWithBlockImages,
```

`reference/pi/packages/agent/docs/agent-harness.md:224-228` — the newer harness lacks current coding-agent auto-compaction parity:

```text
Compaction and tree navigation are structural session mutations.
Auto-compaction and retry decision points are not implemented in AgentHarness yet.
```

`reference/pi/packages/agent/docs/durable-harness.md:19-24` — durability is explicitly bounded:

```text
The practical target is a semi-durable harness:
- session is the durable append-only state tree
- recovery restarts from durable boundaries, not from an in-flight provider stream
```

`reference/pi/packages/coding-agent/docs/security.md:31-35` — isolation is external:

```text
Pi does not include a built-in sandbox.
Real isolation needs to come from the operating system or a virtualization/container boundary.
```

`reference/pi/packages/coding-agent/src/core/project-trust.ts:77-95` — unresolved headless trust fails closed:

```ts
if (!options.projectTrustContext.hasUI) {
  return false;
}
```

### External references

- [Canonical Pi repository](https://github.com/earendil-works/pi) — primary upstream; live activity/release metrics are a 2026-07-22 snapshot.
- [Pinned source tree](https://github.com/earendil-works/pi/tree/dd6bea41efa8caa7a10fe5a6401676dc5699f83f) — immutable source basis for all `reference/pi/...` citations.
- [Historical `badlogic/pi-mono` URL](https://github.com/badlogic/pi-mono) — redirects to the canonical repository and identifies the requested project lineage.
- [Release v0.81.1](https://github.com/earendil-works/pi/releases/tag/v0.81.1) — latest release at inspection time; source archives and summarization-retry events.
- [Earendil RFC 0015 — Pi Licensing](https://rfc.earendil.com/0015/) — primary acquisition/governance statement and MIT-core commitment.
- [`@earendil-works/pi-coding-agent` on npm](https://www.npmjs.com/package/@earendil-works/pi-coding-agent) — current product package identity; npm’s indexed version may lag GitHub releases.
- [`@mariozechner/pi` on npm](https://www.npmjs.com/package/@mariozechner/pi) — unrelated GPU-pod/vLLM manager used only to disambiguate names.
- [Agent Skills specification](https://agentskills.io/specification) — external format Pi implements leniently.

## Caveats / Not found

- The upstream checkout was treated as strictly read-only. This was a static architecture and history review; no Pi binary, provider request, extension, session migration, crash-recovery scenario, server, or benchmark was executed.
- Remote HEAD matched the local pin when checked, but repository metrics, npm metadata, issues, pull requests, and “latest” release status are volatile after 2026-07-22.
- No stable public contract was found that commits `AgentSession`, `AgentHarness`, the experimental server, or extension hook shapes to long-term compatibility. The `0.x` version line and explicit harness TODOs are the available evidence.
- No completed automatic migration path was found between coding-agent’s current `SessionManager` JSONL representation and agent-core’s storage-backed session/SQLite representation.
- No implemented mid-stream provider recovery or universally safe unfinished-tool replay was found; upstream design notes explicitly state those limits.
- No built-in per-operation permission UI or sandbox was found. Extension blocking hooks and documented external containment are mechanisms, not equivalent built-in guarantees.
- No cryptographic signing/provenance mechanism for third-party Pi extensions, skills, or packages was found in the inspected documentation. The repository itself documents dependency pinning, lockfile checks, checksummed source archives, and trusted-source review.
- The exact current publication status of the server and SQLite packages was not independently verified against npm; their package manifests and changelogs exist at the pin, and the server labels itself experimental.
- GitHub displayed 249 releases while the local repository had 305 `v*` tags. This report does not infer a cause for that difference.
- Some source documentation still links to `earendil-works/pi-mono`. Those links are treated as documentation drift, not evidence of a second active codebase.
