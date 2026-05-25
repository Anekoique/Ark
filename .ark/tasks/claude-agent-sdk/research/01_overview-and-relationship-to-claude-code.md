# Research: Claude Agent SDK — overview and relationship to Claude Code

- Query: what the Claude Agent SDK is, its scope, and how it relates to Claude Code (corpus entry topic)
- Scope: external
- Date: 2026-05-25
- Doc snapshot: `code.claude.com` Agent SDK docs fetched 2026-05-25
- SDK versions pinned: Python `claude-agent-sdk` **0.2.87** (PyPI, released 2026-05-23) and TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.150** (npm, current as of 2026-05-23)

## 1. What the SDK is

The Claude Agent SDK is Anthropic's official library for running Claude as an
**autonomous agent loop** inside a host process, programmable from Python or
TypeScript. The headline framing on the overview page:

> "The Agent SDK gives you the same tools, agent loop, and context management
> that power Claude Code, programmable in Python and TypeScript."
> — [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview)

The abstraction it raises over the raw Anthropic Messages API is the **tool-use
loop itself**. With the lower-level [Anthropic Client SDK](https://platform.claude.com/docs/en/api/client-sdks),
the caller writes the `while response.stop_reason == "tool_use"` loop, executes
each tool the model requests, and re-injects results. The Agent SDK's
own framing of the difference is explicit on the overview page:

> "With the Client SDK, you implement a tool loop. With the Agent SDK, Claude
> handles it."

The SDK ships a built-in tool inventory (Read / Write / Edit / Bash / Glob /
Grep / WebSearch / WebFetch / Monitor / AskUserQuestion — see the "Built-in
tools" tab on the overview page), runs the multi-turn agent loop internally,
streams typed events back to the caller, and exposes intercept points (hooks,
permission callbacks, subagent definitions, MCP servers) for everything the
caller does want to control. It is therefore best read as **"Claude Code as a
library"** rather than as a thin wrapper over `messages.create`.

The SDK is not a generic "agent framework." It is bound to Claude as the
underlying model, to the Claude Code CLI binary as the execution host (see §3),
and to Anthropic-controlled model endpoints (see §4).

## 2. Languages, packages, versions, repos

| Lang       | Package                            | Latest version (this doc) | Released   | Repo                                                                             | Lic / terms                                                                                      |
| ---------- | ---------------------------------- | ------------------------- | ---------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Python     | `claude-agent-sdk`                 | `0.2.87`                  | 2026-05-23 | [anthropics/claude-agent-sdk-python](https://github.com/anthropics/claude-agent-sdk-python) | MIT (per PyPI page metadata)                                                                     |
| TypeScript | `@anthropic-ai/claude-agent-sdk`   | `0.3.150`                 | 2026-05-23 | [anthropics/claude-agent-sdk-typescript](https://github.com/anthropics/claude-agent-sdk-typescript) | Anthropic [Commercial Terms of Service](https://www.anthropic.com/legal/commercial-terms)        |

Runtime requirements:

- Python: `Python >=3.10` (PyPI metadata).
- TypeScript: Node.js 18+ (per GitHub repo README extract).

Install commands the docs give verbatim:

```bash
pip install claude-agent-sdk
npm install @anthropic-ai/claude-agent-sdk
```

Both packages **bundle the native Claude Code CLI binary** as a dependency, so
no separate Claude Code install is required. The overview page states this
directly for the TS package ("The TypeScript SDK bundles a native Claude Code
binary for your platform as an optional dependency, so you don't need to
install Claude Code separately"). The Python README says the equivalent: "The
Claude Code CLI is automatically bundled with the package — no separate
installation required! The SDK will use the bundled CLI by default."

Rename history (relevant for older code search hits and skill-pack docs):

- Old package names — Python: `claude-code-sdk`, TS: `@anthropic-ai/claude-code`.
- New package names — Python: `claude-agent-sdk`, TS: `@anthropic-ai/claude-agent-sdk`.
- Old options class name (Python): `ClaudeCodeOptions` → new: `ClaudeAgentOptions`.
- The rename also changed default behavior: "The SDK no longer uses Claude
  Code's system prompt by default." Callers that want the Claude Code system
  prompt must pass `systemPrompt: { type: "preset", preset: "claude_code" }`.
- Source: [Migrate to Claude Agent SDK](https://code.claude.com/docs/en/agent-sdk/migration-guide).
  The migration guide does not state a precise rename date.

The TS package has had one significant API churn since the rename: an
experimental **V2 session API** (`unstable_v2_createSession`,
`unstable_v2_resumeSession`, `unstable_v2_prompt`, `SDKSession`,
`SDKSessionOptions`) shipped in the 0.2.x series and was **removed in
0.3.142**. Today's V1 `query()` API is the only supported surface. Quoting the
deprecation banner: "TypeScript Agent SDK 0.3.142 removes
`unstable_v2_createSession`, `unstable_v2_resumeSession`, `unstable_v2_prompt`,
and the `SDKSession` and `SDKSessionOptions` types." Source:
[TypeScript SDK V2 session API (removed)](https://code.claude.com/docs/en/agent-sdk/typescript-v2-preview).
Topic 02 (`sessions.md`) covers the live session model in detail.

## 3. Relationship to Claude Code

The SDK is the **library form of Claude Code**, sharing the binary, agent
loop, tool inventory, hook system, and subagent model. The overview docs page
states this directly twice — in the lead paragraph ("the same tools, agent
loop, and context management that power Claude Code") and again in the "Agent
SDK vs Claude Code CLI" tab ("Same capabilities, different interface").

The shared substrate, per the overview page's "Capabilities" section and the
migration guide:

| Concept                              | SDK surface (TS / Python)                               | CLI surface                              |
| ------------------------------------ | -------------------------------------------------------- | ---------------------------------------- |
| Built-in tools (Read, Bash, …)       | `allowedTools` / `allowed_tools`                        | same names, same semantics               |
| Hook events                          | `PreToolUse`, `PostToolUse`, `Stop`, `SessionStart`, `SessionEnd`, `UserPromptSubmit`, "and more" | same event names                         |
| Subagents                            | `agents: { name: AgentDefinition }`                     | filesystem `.claude/agents/*.md`         |
| MCP servers                          | `mcpServers` / `mcp_servers`                            | `.mcp.json`                              |
| Permission modes                     | `permission_mode` literal: `"default" \| "acceptEdits" \| "plan" \| "dontAsk" \| "bypassPermissions"` (Python; TS adds `"auto"`) | same names                               |
| Project context                      | `CLAUDE.md`, `.claude/CLAUDE.md`                        | same files                               |
| Skills                               | `.claude/skills/*/SKILL.md`                             | same files                               |
| Slash commands                       | `.claude/commands/*.md` (loaded when `settingSources` includes filesystem) | same files                               |

What translates 1:1 between CLI and SDK (overview page wording: "Workflows
translate directly between them"):

- The Read / Write / Edit / Bash / Glob / Grep / WebSearch / WebFetch / Monitor /
  AskUserQuestion tool set.
- Hook event names and matchers (`Edit|Write`, etc.).
- Subagent definitions written as filesystem `.claude/agents/*.md` cards work
  in both surfaces, since the SDK loads them via `settingSources`.
- Skills (`.claude/skills/*/SKILL.md`) and project memory (`CLAUDE.md`).
- MCP server config; the SDK accepts both a programmatic `mcpServers` map *and*
  the on-disk `.mcp.json` the CLI uses.

What is **CLI-only** (per the migration guide's framing: "The Claude Code
docs now focus on the CLI tool and automation features"):

- The interactive REPL and TUI itself.
- `~/.claude/settings.json` *as a discovery mechanism* — the SDK can load it
  via `settingSources: ["user"]` but does so opt-in; the CLI loads it always.
- The `/resume`, `/rewind`, etc. slash commands as a UI surface (the SDK
  exposes equivalent operations programmatically — e.g. `query.rewindFiles()`
  on the TS `Query` interface).
- The CLI subscription-plan auth flow (claude.ai login). The overview page
  states explicitly: "Unless previously approved, Anthropic does not allow
  third party developers to offer claude.ai login or rate limits for their
  products, including agents built on the Claude Agent SDK."

What is **SDK-only** (per the migration guide's "Agent SDK navigation menu"
inventory and the overview's "Capabilities" tabs):

- Programmatic in-process custom tools via the `tool` decorator /
  `createSdkMcpServer()` (no shelling out — the tool function runs in the
  caller's process).
- Programmatic `AgentDefinition` registration (the CLI only takes filesystem
  agent cards).
- Programmatic `HookMatcher` / `HookCallback` registration with arbitrary
  caller code in the callback body.
- `CanUseTool` permission callbacks — caller-side allow/deny logic invoked
  per tool call.
- `query.interrupt()`, `query.setPermissionMode()`, `query.setModel()`,
  `query.applyFlagSettings()`, `query.rewindFiles()` on the TS `Query`
  object (mid-session mutation from caller code).
- The session-management helpers `listSessions()` / `getSessionMessages()` /
  `getSessionInfo()` / `renameSession()` / `tagSession()` (these read the same
  on-disk JSONL the CLI writes, but expose it as typed library calls).
- `resolveSettings()` (TS) — read effective Claude Code settings without
  spawning the CLI.
- Structured-output mode and OpenTelemetry observability flags.

The takeaway: the SDK and CLI share **state and substrate**; they differ on
**interface and embedding**. Anything the CLI does as a slash command, an
end-user keybinding, or an interactive prompt, the SDK exposes as a function
call, a callback, or an event in the message stream.

## 4. Provider lock-in

The SDK is **Claude-only** — it does not abstract over model providers. It
abstracts over the *transport* by which Claude is reached. The overview page's
"Set your API key" step lists every supported route:

> "* **Amazon Bedrock**: set `CLAUDE_CODE_USE_BEDROCK=1` environment variable
>   and configure AWS credentials
> * **Claude Platform on AWS**: set `CLAUDE_CODE_USE_ANTHROPIC_AWS=1` and
>   `ANTHROPIC_AWS_WORKSPACE_ID`, then configure AWS credentials
> * **Google Vertex AI**: set `CLAUDE_CODE_USE_VERTEX=1` environment variable
>   and configure Google Cloud credentials
> * **Microsoft Azure**: set `CLAUDE_CODE_USE_FOUNDRY=1` environment variable
>   and configure Azure credentials"

The default path is direct Anthropic API via `ANTHROPIC_API_KEY`. None of
these routes serve non-Claude models: Bedrock, Vertex, and Azure Foundry are
just hosting surfaces for Anthropic-built Claude weights, and "Claude Platform
on AWS" is Anthropic's own AWS-region offering. The model selection knob
(`model: "claude-opus-4-7"` in examples) takes Claude family identifiers only.

There is no documented adapter for OpenAI, Google Gemini, Llama, or any other
model family. If a substrate needs cross-model support, that abstraction must
live above the Agent SDK, not inside it. The SDK's `Transport` ABC (Python) /
custom transport interfaces (TS) are extension points for **how messages reach
the Claude Code subprocess**, not for swapping in a different model API.

The "claude.ai login" path used by the interactive CLI is *not* available to
third-party SDK integrators, per the overview page (see §3).

## 5. Top-level entry points

The names below are the public surface of each SDK as documented on the
TypeScript and Python reference pages. One-liners only; depth lives in later
topic files.

### Python (`claude_agent_sdk`)

Functions:

- `query(*, prompt, options=None, transport=None) -> AsyncIterator[Message]`
  — primary one-shot entry; spawns a fresh session and streams typed messages.
- `tool(name, description, input_schema, annotations=None) -> decorator`
  — decorator that registers a Python coroutine as an MCP tool.
- `create_sdk_mcp_server(name, version="1.0.0", tools=None) -> McpSdkServerConfig`
  — packages decorated tools into an **in-process** MCP server config.
- `list_sessions(directory=None, limit=None, include_worktrees=True) -> list[SDKSessionInfo]`
  — enumerate past sessions from the on-disk store (synchronous).
- `get_session_messages(session_id, directory=None, limit=None, offset=0) -> list[SessionMessage]`
  — read transcript of a past session.
- `get_session_info(session_id, directory=None) -> SDKSessionInfo | None`
  — single-session metadata lookup.
- `rename_session(session_id, title, directory=None) -> None`
  — append a custom-title entry.
- `tag_session(session_id, tag, directory=None) -> None`
  — set or clear (`None`) the session's tag.

Classes (selected — full list in [Python SDK reference](https://code.claude.com/docs/en/agent-sdk/python)):

- `ClaudeSDKClient` — bidirectional client for multi-turn interactive
  sessions, with mid-session interrupts and custom tools.
- `ClaudeAgentOptions` — config dataclass (tools, permissions, MCP servers,
  model, system prompts, hooks, etc.).
- `AgentDefinition` — programmatic subagent definition.
- `HookMatcher` — hook registration (event + matcher pattern + callback list).
- `SdkMcpTool` — return type of the `tool` decorator.
- `Transport` — abstract base class for custom transport implementations.
- Message types: `UserMessage`, `AssistantMessage`, `SystemMessage`,
  `ResultMessage`, `StreamEvent`, `RateLimitEvent`, `TaskStartedMessage`,
  `TaskProgressMessage`, `TaskNotificationMessage`.
- Permission types: `ToolPermissionContext`, `PermissionResultAllow`,
  `PermissionResultDeny`, `PermissionUpdate`.

Type aliases:

- `Message = UserMessage | AssistantMessage | SystemMessage | ResultMessage | StreamEvent | RateLimitEvent`
- `PermissionMode = "default" | "acceptEdits" | "plan" | "dontAsk" | "bypassPermissions"`
- `EffortLevel = "low" | "medium" | "high" | "xhigh" | "max"`
- `CanUseTool = Callable[[str, dict, ToolPermissionContext], Awaitable[PermissionResult]]`
- `McpServerConfig = McpStdioServerConfig | McpSSEServerConfig | McpHttpServerConfig | McpSdkServerConfig`

### TypeScript (`@anthropic-ai/claude-agent-sdk`)

Functions:

- `query({ prompt, options }) -> Query` — primary entry; returns an
  `AsyncGenerator<SDKMessage>` that *also* exposes session-control methods
  (`interrupt()`, `setPermissionMode()`, `setModel()`, `rewindFiles()`,
  `applyFlagSettings()`).
- `startup() -> WarmQuery` — pre-warm the CLI subprocess; subsequent
  `warmQuery.query(prompt)` skips the init handshake.
- `tool(...)` — type-safe MCP tool definition for SDK MCP servers.
- `createSdkMcpServer(...)` — in-process MCP server.
- `listSessions(...)`, `getSessionMessages(...)`, `getSessionInfo(...)`,
  `renameSession(...)`, `tagSession(...)` — session store accessors (mirror
  the Python set).
- `resolveSettings(directory) -> ResolvedSettings` — read effective Claude
  Code settings for a directory without spawning the CLI.

Interfaces / types (selected — full list in
[TypeScript SDK reference](https://code.claude.com/docs/en/agent-sdk/typescript)):

- `Query` — the iterable returned by `query()`; extends
  `AsyncGenerator<SDKMessage>` with the mutation methods listed above.
- `WarmQuery` — `AsyncDisposable` returned by `startup()`.
- `Options` — config object passed via `query({ options })`; described in
  the reference as having "~50 configurable properties."
- `AgentDefinition` — programmatic subagent (description, prompt, tools,
  disallowedTools, model, mcpServers, skills).
- `HookCallback` — caller-side hook function type.
- `CanUseTool` — caller-side per-tool permission function.
- `SDKMessage` — discriminated union of every typed event the stream emits;
  see topic 03 for the full list (assistant / user / result / system /
  partial / compact-boundary / plugin / permission-denied / task / tool /
  memory / prompt-suggestion / auth / rate-limit / retry / notification /
  status / local-command / hook / files-persisted / tool-use-summary /
  elicitation / mirror-error).
- `PermissionMode = "default" | "acceptEdits" | "bypassPermissions" | "plan" | "dontAsk" | "auto"` — note the TS literal adds `"auto"` over Python.
- `McpServerConfig` — stdio / SSE / HTTP / SDK / claude.ai-proxy variants.
- `SettingSource = "user" | "project" | "local"` — `settingSources` option
  values.

### Minimal hello-world (overview page, verbatim — clarifies the entry-point shape)

Python:

```python
import asyncio
from claude_agent_sdk import query, ClaudeAgentOptions


async def main():
    async for message in query(
        prompt="Find and fix the bug in auth.py",
        options=ClaudeAgentOptions(allowed_tools=["Read", "Edit", "Bash"]),
    ):
        print(message)


asyncio.run(main())
```

TypeScript:

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Find and fix the bug in auth.ts",
  options: { allowedTools: ["Read", "Edit", "Bash"] }
})) {
  console.log(message);
}
```

The async-iterator shape is the central abstraction: every event Claude
emits — initialization, assistant chunks, tool calls, tool results, the final
result message, telemetry events — arrives as one yielded message in the
caller's loop. Topic 03 (`streaming-events.md`) inventories what those
messages carry.

## 6. What this file is NOT covering

Pointers to the per-topic files where deeper detail lives:

- **02 `sessions.md`** — fresh / resume / fork patterns, the on-disk JSONL
  session store, `listSessions()` and friends, `settingSources` loading
  (`CLAUDE.md`, `AGENTS.md`, skills, commands), session-ID propagation.
- **03 `streaming-events.md`** — every `SDKMessage` / `Message` variant, how
  to detect turn-complete vs. session-complete (`ResultMessage` /
  `SDKResultMessage`), partial-message streaming.
- **04 `hooks.md`** — exact hook signatures, return-shape semantics
  (deny / approve / mutate), the data each hook receives.
- **05 `tools-and-permissions.md`** — built-in tool inventory, `allowedTools`
  / `disallowedTools`, `CanUseTool` callback signature, all `PermissionMode`
  values including `"plan"` and `"bypassPermissions"`.
- **06 `subagents.md`** — `AgentDefinition` vs filesystem agent cards, how
  results come back (parent_tool_use_id thread on messages), recursion limits.
- **07 `mcp-integration.md`** — `mcpServers` transports (stdio / SSE / HTTP /
  in-process SDK / claude.ai proxy), `createSdkMcpServer` and `tool` for
  publishing in-process tools.
- **08 `cost-and-budget.md`** — `ResultMessage` cost fields, rate-limit
  events, budget-enforcement patterns.
- **09–14** — concurrency, persistence/memory, skills/AGENTS.md, extended
  thinking + model config, telemetry, gaps.
- **99 `SYNTHESIS.md`** — cross-corpus reading for ArkOS stage 1.

## Caveats / Not found

- The doc snapshot **does not** state a precise calendar date for the
  `claude-code-sdk → claude-agent-sdk` rename. Inferring it from the npm
  history and the TS V2 removal note suggests "before Agent SDK 0.2.x," but
  this is not directly cited.
- The **Python SDK does not currently document `startup()`** the way the TS
  SDK does, nor does it document a `WarmQuery`. If a pre-warm pattern exists
  in Python it is undocumented on the official reference page snapshot read
  here. Topic 02 should re-check.
- The full list of hook events on the overview page is documented as
  `PreToolUse`, `PostToolUse`, `Stop`, `SessionStart`, `SessionEnd`,
  `UserPromptSubmit`, **"and more"** — i.e. the overview page does not
  enumerate the complete set. Topic 04 will enumerate from the hooks
  reference page.
- The exact TS license is "Anthropic Commercial Terms of Service"
  (overview page) rather than an OSI license. The Python repo's PyPI page
  declares `MIT`. This is a real asymmetry worth flagging for any downstream
  redistribution — but the Python repo's own `LICENSE` file was not directly
  inspected in this pass and the asymmetry should be re-verified before being
  acted on.
- "Managed Agents" is a separate Anthropic product (a hosted REST agent
  runtime). The overview page describes it as a sibling to the Agent SDK, not
  a successor. It is **out of scope** for this corpus; the corpus targets the
  in-process library form.
- This file deliberately did **not** read the per-topic reference pages
  (hooks, MCP, subagents) in depth — those belong to topics 04, 06, 07.

## Primary sources

- [Agent SDK overview (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/overview) — primary citation for §1, §3, §4, §5 examples.
- [Migrate to Claude Agent SDK](https://code.claude.com/docs/en/agent-sdk/migration-guide) — rename, default-behavior changes, CLI-vs-SDK boundary.
- [Python SDK reference](https://code.claude.com/docs/en/agent-sdk/python) — Python public API listing.
- [TypeScript SDK reference](https://code.claude.com/docs/en/agent-sdk/typescript) — TS public API listing.
- [TypeScript SDK V2 session API (removed)](https://code.claude.com/docs/en/agent-sdk/typescript-v2-preview) — V2 deprecation and 0.3.142 removal.
- [PyPI: claude-agent-sdk 0.2.87](https://pypi.org/project/claude-agent-sdk/) — Python package metadata and release date.
- [npm: @anthropic-ai/claude-agent-sdk](https://www.npmjs.com/package/@anthropic-ai/claude-agent-sdk) — TS package version (0.3.150).
- [GitHub: anthropics/claude-agent-sdk-python](https://github.com/anthropics/claude-agent-sdk-python) — Python repo README, hello-world, bundled-CLI claim.
- [GitHub: anthropics/claude-agent-sdk-typescript](https://github.com/anthropics/claude-agent-sdk-typescript) — TS repo README, Node 18+ requirement.
