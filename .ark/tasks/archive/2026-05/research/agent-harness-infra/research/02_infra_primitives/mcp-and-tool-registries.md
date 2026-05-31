# MCP and Tool Registries

## What the primitive means

A coding agent without tools is a chat window. The "tool registry" is *how
the agent discovers what it can do* — every harness has one, but the shape
varies enormously: bespoke JSON schemas, plugin folders, OpenAPI specs,
language-binding manifests. Since late 2024 the field has been converging on
**Model Context Protocol (MCP)** as the cross-harness standard. This file
covers MCP in depth, surveys the non-MCP registries still in use, and asks
where Ark fits.

### LSP parallel (why MCP looks the way it does)

MCP is explicitly modelled on the Language Server Protocol pattern: one
generic *client* (the IDE / harness) talks JSON-RPC to many *servers* that
each vend a small surface (lint, completion, references). MCP keeps the
JSON-RPC pipe and replaces "language features" with "tools, resources,
prompts." The lifecycle (`initialize` → `initialized` → request/response /
notification) is LSP-shaped.

### ChatGPT plugins as the failed precursor

OpenAI's ChatGPT Plugins (2023) tried the same goal — a portable way to
expose tools to an LLM — but tied it to OpenAPI specs and a hosted plugin
store. They were deprecated in favour of GPTs (April 2024) and then Function
Calling. MCP succeeds where Plugins failed because it (a) is transport-
agnostic, (b) covers prompts and resources, not just functions, and (c) was
launched as an open standard with reference SDKs in TypeScript and Python.

## MCP architecture in depth

### Three primitives

Every MCP server exposes some combination of:

| Primitive | Verb | Purpose | Example |
| --------- | ---- | ------- | ------- |
| **Tool** | `tools/call` | Executable side-effectful action | `filesystem.write_file`, `git.commit` |
| **Resource** | `resources/read` | Read-only data the model may include in context | `file:///README.md`, `db://users/42` |
| **Prompt** | `prompts/get` | Reusable templated message | `summarize-pr`, `explain-error` |

Tools are the load-bearing one; resources are documents-as-context (an
intentional alternative to RAG); prompts are slash-command-shaped.

### Transports

Three transports defined; two recommended in 2026 (per
`modelcontextprotocol.io/specification/2025-06-18/basic/transports` and
`blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/`):

| Transport | When | Mechanism |
| --------- | ---- | --------- |
| **stdio** | Local server, subprocess of client | Newline-delimited JSON-RPC 2.0 on the server's stdin/stdout; no embedded newlines. |
| **Streamable HTTP** | Remote, modern | Single endpoint accepting POST / GET; server picks `application/json` (single response) or `text/event-stream` (SSE-style streaming). Replaces standalone SSE. |
| **HTTP+SSE (legacy)** | Pre-2025 servers | Persistent SSE for server → client; POST for client → server. Deprecated 2025-03-26. |

**stdio is the default for editor-embedded servers**: the client launches the
server as a subprocess and the protocol speaks through the pipes. This is
what Claude Desktop uses for "local" MCP servers.

### Lifecycle and capability negotiation

```
client → server   initialize(protocolVersion, capabilities, clientInfo)
server → client   { protocolVersion, capabilities, serverInfo, instructions? }
client → server   notifications/initialized
                  --- now meaningful requests are allowed ---
client → server   tools/list           → { tools: [...] }
client → server   tools/call           → { content: [...] }
server → client   notifications/tools/list_changed
```

Before the `initialized` notification, only `ping` and server log
notifications are permitted; **everything else is forbidden**. The
`capabilities` blob lets each side advertise opt-in features (e.g.
`{"roots": {"listChanged": true}}`).

### Roots (filesystem boundary)

`roots` are URIs the *client* exposes to the *server* declaring "you may
operate inside these directories" — see
`sandboxing-and-isolation.md` for the security framing. This is
client-asserted capability, not server-requested permission.

### Authorization

MCP authorization spec
(`modelcontextprotocol.io/specification/draft/basic/authorization`) is a
**subset of OAuth 2.1** with PKCE mandatory:

- Servers advertise required scopes in `WWW-Authenticate` headers.
- Clients must use PKCE (RFC 7636).
- Discovery via RFC 8414 (Authorization Server Metadata).
- **Static-client-ID proxies must obtain user consent for each dynamically
  registered client** — closes a delegation hole inherent in shared MCP
  gateways.

The shape parallels OAuth 2.1 + Resource Indicators (RFC 8707) with MCP-
specific guidance about how the agent stages consent.

## Tool registry models in non-MCP harnesses

### Claude Code (multi-modal registry)

A *plugin* is a directory containing any of: slash commands, subagents,
skills, hooks, MCP servers. Installed via `/plugin marketplace add` and
`/plugin install` (Claude Code plugins reference,
`code.claude.com/docs/en/plugin-marketplaces`). The marketplace is a JSON
catalog; the official one is `anthropics/claude-plugins-official` on GitHub.

So Claude Code's "tool registry" is actually **five** registries layered:

- Built-in tools (`Write`, `Edit`, `Bash`, …) — fixed in the binary.
- Slash commands in `.claude/commands/` — flat markdown files.
- Subagents in `.claude/agents/` — markdown with frontmatter (`tools:`, `model:`).
- Skills in `.claude/skills/` — directory + SKILL.md + scripts.
- MCP servers — declared in `.claude/settings.json`'s `mcpServers` block.

### Codex CLI

Codex consumes MCP directly (no separate plugin shape). Tool surface is
fixed in the binary plus user-configured MCP servers in `~/.codex/config.toml`.
Skills (`.codex/skills/`) are project-scoped capability folders, similar
shape to Claude's skills.

### OpenHands

Tool registry = **microagents** + MCP. `.openhands/microagents/` ships
markdown files keyed by `trigger: always | keyword | manual`; an "always"
microagent is roughly a global system prompt fragment, a "keyword" one is a
JIT-loaded specialisation.

### Aider

Tools are baked into the binary (edit, commit, undo, lint, tests).
Extension is via "prompts" — bring-your-own conventions in chat. No external
plugin loader.

### Cline

Tool surface is fixed (the agent loop), but extensible via MCP. The
Cline-Memory-Bank example is a community MCP server purpose-built to give
Cline persistent project context across sessions.

### Continue.dev

Tools = built-in modes (Chat, Edit, Agent) + MCP servers declared in
`config.yaml`. Rules live under `.continue/rules/` and feed system prompts.

### Cursor

Cursor consumes MCP servers; bespoke tool format is internal.

### Goose (Block)

Goose was an early adopter — its toolkit / extension system is **entirely
MCP**. "It connects to 3,000+ tools via MCP" (Block writeup). This is the
clearest case in the survey of a harness whose tool registry == MCP client.

## Where Ark sits

Ark today is a **workflow** layer with no tool registry. It exposes:

- One CLI surface: `ark agent {task,spec} …`, hidden (`#[command(hide=true)]`),
  documented as **not semver-stable**.
- Per-platform extracts: slash commands for Claude (`templates/claude/`),
  skills for Codex, commands for OpenCode — see `platforms.rs:283`
  (`PLATFORMS` registry).
- A `SessionStart` hook that injects `ark context` JSON into each session
  (`crates/ark-core/src/io/fs/hook.rs:15`,
  `ARK_CONTEXT_HOOK_COMMAND = "ark context --scope session --format json"`).

Ark consumes **no** MCP servers. It exposes **no** MCP interface. It does
not register tools with any harness directly — it injects markdown
instructions and a hook command.

This is the right minimum for Phase 0. The interesting question is whether
the *next* phase exposes Ark as an MCP server.

## How Ark could expose itself as an MCP server

The natural shape: a thin MCP server fronting `ark agent` plus `ark context`.

### Sketch of resources

```
resource: ark://task/active                  → active task summary as JSON
resource: ark://task/<slug>/prd              → PRD body
resource: ark://task/<slug>/plan/latest      → latest PLAN body
resource: ark://task/<slug>/verify           → VERIFY checklist
resource: ark://specs/project/<name>         → project SPEC body
resource: ark://specs/features/<path>        → feature SPEC body
resource: ark://workflow                     → .ark/workflow.md
```

### Sketch of tools

```
tool: ark_task_new        { slug, title, tier, worktree? }
tool: ark_task_plan       (advance from design/plan)
tool: ark_task_review
tool: ark_task_execute
tool: ark_task_verify
tool: ark_task_commit     { message }
tool: ark_task_archive
tool: ark_task_resume     { slug }
tool: ark_context         { scope, for?, format }
```

### Sketch of prompts

```
prompt: design-prd        — fill-in template for the design phase
prompt: plan-template     — standard / deep plan skeleton
prompt: review-checklist  — deep tier reviewer prompt
prompt: verify-checklist  — VERIFY seeded sections
```

### Tradeoffs of exposing MCP

| Pro | Con |
| --- | --- |
| Reach: any MCP-capable harness gets Ark, not just Claude / Codex / OpenCode | New stability surface — current `ark agent` is explicitly *not* semver |
| `tools/list` lets agents *discover* the workflow, removing template-as-context overhead | MCP server is a long-running process — Ark is currently invocation-as-a-Service |
| Subagent prompts (researcher / reviewer / verifier) become MCP prompts | Some tools require focus state; MCP semantics for that are still informal |
| Authorization story is reusable across platforms | OAuth dance is over-engineered for a single-user local tool |

The strongest argument for MCP is **discoverability** — today a fresh Claude
session reads `CLAUDE.md` and `workflow.md` to learn what Ark is. An MCP
server makes Ark callable without that bootstrap.

The strongest argument against is **stability**. `ark agent`'s explicit
not-semver contract relies on the binary and templates moving in lockstep.
An MCP surface would expose Ark to clients that *don't* ship with the
matching templates.

## Authorization model for an Ark MCP server

For a *local* server (stdio transport), full OAuth is overkill: the
subprocess IS the user. The realistic stance:

- stdio transport: trust = parent process; no token; rely on filesystem
  ACL on `.ark/`.
- Streamable HTTP / remote: full OAuth 2.1 + PKCE; scopes shaped by
  command verb (`task.write`, `task.read`, `spec.write`).

The Claude Code hooks model — return `{decision: allow|deny|ask|defer}` —
is conceptually elegant but not portable to MCP, which gates at the
authorization-server layer, not per call.

## Directions for Ark

1. **MCP server prototype.** A `crates/ark-mcp` binary that vends the
   resource / tool / prompt sketch above over stdio. Implementing this as
   a thin wrapper over `ark-core`'s existing `commands::agent::*` routes
   keeps the canonical CLI in charge — no logic divergence. Code site:
   new crate; reuse `crates/ark-core/src/commands/agent/task/mod.rs`.
2. **Expose `ark context` as an MCP resource.** Even before full MCP, vend
   `ark context --scope session --format json` as `ark://context/session`
   plus `ark://context/phase/<phase>`. The current `SessionStart` hook
   already produces this JSON — promoting it to MCP is a serialization
   shift. Code site: `crates/ark-core/src/commands/context/`.
3. **Adopt MCP roots to declare the workflow boundary.** When a host
   harness consumes Ark via MCP, Ark should declare its own roots:
   `file://<root>/.ark/`, `file://<root>/.ark/worktrees/<branch>/`. This
   formally inverts the current trust: instead of "we live anywhere the
   host lets us," Ark says "these are the directories I claim." Pairs
   with Direction 3 in `sandboxing-and-isolation.md`.
4. **Plugin-format compat.** Anthropic's plugin spec is a directory of
   `commands / agents / skills / hooks / mcpServers`. Ark already extracts
   `commands/ark/`, `agents/`, and injects a hook. Adding a top-level
   `plugin.json` manifest at the root of `templates/claude/` would make
   Ark installable via `/plugin install` — a richer onboarding path than
   `ark init`. Code site: `templates/claude/`, `crates/ark-core/src/templates.rs`.
5. **MCP-style capability negotiation for `ark agent`.** When `ark agent`
   gains a stability tier, expose a `capabilities` subcommand
   (`ark agent capabilities --format json`) listing the verbs available in
   *this* binary. Slash commands and MCP clients consume it; protects
   against template / binary version skew. Code site:
   `crates/ark-core/src/commands/agent/mod.rs`.

## Caveats / Not found

- I did not find a primary-source document for "MCP traffic over WebSocket"
  — the spec lists stdio and Streamable HTTP only, despite community
  references to WebSocket transports.
- The exact list of capabilities in the 2025-06-18 spec evolves; the
  draft authorization spec is the moving piece — pin to a version when
  implementing.
- No public benchmark of MCP server CPU / memory overhead for editor-
  embedded servers; "stdio is cheap" is folklore-grade, not measured.

## Sources

- [MCP Specification — Architecture](https://modelcontextprotocol.io/docs/learn/architecture)
- [MCP Specification — Transports (2025-06-18)](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
- [MCP Specification — Roots](https://modelcontextprotocol.io/specification/2025-06-18/client/roots)
- [MCP Authorization Draft](https://modelcontextprotocol.io/specification/draft/basic/authorization)
- [Why MCP Deprecated SSE for Streamable HTTP](https://blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/)
- [MCP JSON-RPC Reference](https://portkey.ai/blog/mcp-message-types-complete-json-rpc-reference-guide/)
- [MCP Lifecycle Explained](https://medium.com/@ashishpandey2062/mcp-lifecycle-explained-client-server-workflow-c366fd45328b)
- [Claude Code plugin marketplaces](https://code.claude.com/docs/en/plugin-marketplaces)
- [Anthropic official plugin marketplace](https://github.com/anthropics/claude-plugins-official)
- [OpenAI Codex config — MCP servers](https://github.com/openai/codex/blob/main/docs/config.md)
- [OpenHands microagents overview](https://docs.openhands.dev/openhands/usage/microagents/microagents-overview)
- [Continue.dev customization](https://docs.continue.dev/customize/overview)
- [Goose (Block) — open framework](https://block.xyz/inside/block-open-source-introduces-codename-goose)
- [Stytch — MCP authentication guide](https://stytch.com/blog/MCP-authentication-and-authorization-guide/)
