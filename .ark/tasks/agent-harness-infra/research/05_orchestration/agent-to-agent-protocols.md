# Agent-to-Agent Protocols (A2A, ACP, MCP-as-A2A)

The emerging protocol layer for *agents talking to other agents*. As of 2026 this is the most-contested layer below MCP — multiple proposals, none yet dominant, but real adoption signals.

## Why a separate protocol from MCP

MCP is *agent → tool*: the agent (or its host) calls discrete functions on a server. The protocol is asymmetric: client = agent, server = function provider. Agents do not naturally call other agents as MCP servers — an MCP server is a passive function, not an autonomous peer.

A2A protocols target *agent ↔ agent*: peer interactions with their own context, autonomy, and goals. Different semantics:

- **MCP:** "call this function with these args"
- **A2A:** "ask this agent to do this; it will reason about it and respond"

The line is sometimes blurry — an MCP server *can* wrap an agent. But the protocols are designed for different shapes of interaction.

## Google's A2A (Agent2Agent)

Announced April 2025; donated to Linux Foundation in June 2025. Now a community project (`a2aproject.github.io`).

### Mechanics

- **Transport:** JSON-RPC over HTTP + Server-Sent Events.
- **Discovery:** `/.well-known/agent.json` at the agent's host. Capabilities advertised; client decides what to invoke.
- **Tasks:** A2A operates on *tasks* (unit of work). Client creates a task; agent processes; client polls or streams updates.
- **Identity:** Each agent has an ID and an optional public key for verification.
- **State:** Tasks have explicit lifecycle (created → working → completed / failed).

### Adoption

- Google's own agents (Gemini, Imagen agents).
- Several enterprise integrations (Salesforce, ServiceNow announced support).
- Open-source agent frameworks adding A2A clients/servers.

### Strengths

- HTTP transport is universal; firewalls / load balancers / observability all "just work".
- Well-known endpoint pattern is RFC-blessed; discovery is standard.
- Task lifecycle maps cleanly to background async work.

### Weaknesses

- HTTP server requirement is heavy for CLI tools (Ark would need to host one).
- Authentication / authorisation patterns are still maturing.
- Adoption outside Google's orbit is slower than MCP's was.

## Zed/JetBrains' ACP (Agent Client Protocol)

Shipped 2025; co-promoted by Zed and JetBrains. Sometimes positioned as "LSP for agents".

### Mechanics

- **Transport:** JSON-RPC over stdio (mirroring LSP).
- **Direction:** Editor (client) talks to agent (server) over a sub-process pipe.
- **Surface:** Edit actions, file actions, prompt actions. Editor renders; agent reasons.
- **Stateful:** Persistent connection; server holds session state.

### Adoption

- Zed (Agent Panel uses ACP).
- JetBrains (AI Assistant exposes ACP).
- Kiro (kiro.dev).
- OpenCode (sst/opencode supports ACP).
- Anthropic's Claude Code does NOT speak ACP (uses its own native protocol).

### Strengths

- stdio transport is cheap; no HTTP server required.
- Maps to LSP's mature pattern; editor vendors know how to integrate.
- Vendor-neutral by design.

### Weaknesses

- Editor↔agent is one shape; doesn't cover agent↔agent.
- LSP's "client-server" framing is awkward when the "server" is autonomous.
- Adoption is concentrated in the Zed/JetBrains axis; Cursor/Windsurf/Claude Code stay native.

## MCP as A2A

Less a protocol than a pattern. An agent wraps itself in an MCP server, exposing its capabilities as MCP tools. Other agents call those tools.

### Mechanics

- The wrapped agent runs as an MCP server (stdio or HTTP).
- Its capabilities are tool definitions in the MCP schema.
- Calling agent invokes them as normal MCP tool calls.

### Adoption

- Cursor exposes a (proprietary) capability allowing custom MCP servers to wrap agent-like behaviour.
- Goose recipes can wrap agents as MCP tools.
- OpenAI Codex can self-expose as MCP server.

### Strengths

- Reuses MCP transport / SDK / observability.
- Single protocol simplifies the integration surface.

### Weaknesses

- MCP's tool-call shape doesn't model long-running async work well.
- No native concept of "task in progress" with intermediate status.
- The agent-as-tool framing hides autonomy.

## How they relate

| Protocol | Surface | Maturity (2026) | Best fit |
| -------- | ------- | --------------- | -------- |
| **MCP** | Agent ↔ tool | High; dominant | Tool calls, resource fetching |
| **A2A** | Agent ↔ agent | Medium; growing | Async cross-org agent invocation |
| **ACP** | Editor ↔ agent | Medium; concentrated in Zed/JetBrains | IDE-driven agent invocation |
| **MCP-as-A2A** | Agent ↔ agent via MCP tools | Working pattern, no formal blessing | Same-host or simple cases |

The likely 2026–2027 outcome is *both A2A and ACP survive*, addressing different needs (A2A for cross-org / cloud agents; ACP for editor integration), with MCP-as-A2A as a same-host pragmatic substitute.

## What this means for Ark

Ark is a CLI tool, not an agent in the protocol sense. The harness sits between the user and the host agent (Claude Code, Codex, OpenCode). Where do protocols fit?

### Could Ark be an MCP server?

Yes, and probably should. Concrete: `ark-mcp` crate exposes:
- **Resources:** task list, current task PRD, current PLAN, feature SPEC listings.
- **Tools:** `ark agent task new`, `task plan`, `task verify`, `task commit`, etc.
- **Prompts:** Templated prompts for each phase.

Hosts that speak MCP (Cursor, Continue, Zed, Claude Code via custom MCP, etc.) could then drive Ark without needing per-host templates. Reduces the per-platform templating cost in `platforms.rs`.

This is the highest-leverage protocol move for Ark.

### Could Ark speak ACP?

Possible. ACP would let Ark be the "agent" half of an editor-agent pair — Zed or JetBrains would invoke Ark, Ark would orchestrate the workflow and (via its host agent dependency) call out to a coding agent for actual code work.

This is interesting but indirect. Ark today depends on a coding agent being present (Claude Code, Codex, etc.); making Ark itself ACP-speakable would require either:
- Ark embeds an LLM client (architecturally bigger move).
- Ark exposes its workflow steps as ACP actions and lets the editor orchestrate the model call.

The second option is more in keeping with Ark's "harness layer, not agent" positioning.

### Could Ark speak A2A?

In principle yes — Ark hosts a `/.well-known/agent.json`, lists its workflow primitives as capabilities, accepts task creation over JSON-RPC + SSE. But this is a bigger commitment than MCP because Ark would need an HTTP server.

For now, the right call is probably "MCP first, A2A later if demand".

## Where Ark's protocol thinking should go

The dominant primitives in 2026 are:
- **MCP** for tool exposure → Ark should host an MCP server.
- **AGENTS.md** for portable always-loaded memory → Ark already writes this on Codex/OpenCode; should on Claude too.
- **SKILL.md** for portable behaviour packs → Ark's per-platform skills should converge to a SKILL.md format that exports to each platform's directory.

ACP and A2A are options Ark should track but not commit to in 2026.

## Trade-offs of going protocol-first

| Pro | Con |
| --- | --- |
| Reduces per-platform templating cost (one MCP surface vs. four template trees) | Doesn't replace templates entirely — slash-command UX still expected by users |
| Opens Ark to platforms it doesn't template for today (Cursor, Continue, Zed) | Requires keeping MCP surface stable as Ark evolves |
| Encourages clean separation between "what Ark does" and "how the host surfaces it" | MCP schema design is a real cost (capability list, tool docs, error model) |
| Aligns with the industry direction | Splits Ark's effort between templates AND protocol — risk of doing both half-well |

The decisive factor: Ark's `ark agent` namespace is already a *typed* CLI surface — every verb has a clean Rust signature in `crates/ark-core/src/`. Exposing those as MCP tools is largely a translation layer, not new design.

## Directions for Ark

1. **Build `ark-mcp` as a new crate exposing the `ark agent` namespace.** Resources for tasks/specs/context; tools for each task-namespace verb; prompts seeded from the existing slash-command templates. The single highest-leverage protocol move.

2. **Always write `AGENTS.md`.** Cross-platform convergence is real. Per-platform managed-block patches already exist; broadening Claude's install to write AGENTS.md in addition to CLAUDE.md is cheap.

3. **Audit the slash-command vs. skill divergence.** Claude Code's 2026 docs favour skills. Ark ships both. A unified source-of-truth that emits to both formats (or migrates to skills) avoids the maintenance tax.

4. **Track ACP adoption, don't ship yet.** Zed/JetBrains-only is too narrow today. If Cursor or Windsurf adopts ACP, the calculus changes.

5. **Track A2A as RFC-grade, not ship-grade.** The HTTP-server commitment is too heavy for a local CLI in 2026. Worth a `docs/rfcs/00X-a2a.md` to think through; not worth a shipping commitment.
