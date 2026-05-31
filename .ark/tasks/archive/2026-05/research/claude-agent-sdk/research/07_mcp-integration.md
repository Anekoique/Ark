# Research: Claude Agent SDK — MCP integration

- Query: configuring MCP servers via `mcpServers`/`mcp_servers` and `.mcp.json`; transports (stdio / HTTP / SSE); what the agent consumes (tools vs resources vs prompts); the `mcp__<server>__<tool>` namespace; in-process `create_sdk_mcp_server`/`createSdkMcpServer` + `tool()`; the key question — can the SDK *publish* a standalone MCP server other agents dial into; tool naming/collision vs `allowedTools`/`disallowedTools`; lifecycle; error handling.
- Scope: mixed (primary docs `code.claude.com` + Python SDK source `claude-agent-sdk-python` @ 0.2.87)
- Date: 2026-05-25
- Doc snapshot: `code.claude.com/docs/en/agent-sdk/{mcp,custom-tools}` fetched 2026-05-25.
- SDK versions pinned: Python `claude-agent-sdk` **0.2.87** (PyPI; confirmed latest available 2026-05-25 — no newer release), TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.150** (npm; confirmed latest 2026-05-25 — no newer release). Source read against the Python repo checked out at tag-equivalent `version = "0.2.87"` in `pyproject.toml`.
- Standalone MCP libs cross-checked (for §5): PyPI `mcp` latest **1.27.1** / `fastmcp` **3.3.1**; npm `@modelcontextprotocol/sdk` **1.29.0**. The Agent SDK pins `mcp>=1.23.0` as a hard dependency (`pyproject.toml:31`).

Why this matters (one line): RFC 001 names MCP as the intended portable substrate surface for ArkOS. This file establishes, definitively, that the Agent SDK is an MCP **client** (consumes servers) and an **in-process tool host** (`create_sdk_mcp_server`), but is **not** an MCP **publisher** — exposing ArkOS primitives to *external* agents requires a separate standalone MCP server (§5).

Boundaries with neighbor files:
- **Topic 01** (`01_overview-and-relationship-to-claude-code.md`) named `create_sdk_mcp_server`/`createSdkMcpServer`, the `McpServerConfig` union, and listed MCP under SDK↔CLI shared substrate. This file is the API-level expansion; it does not re-derive the package/version facts (cited there).
- **Topic 05** (`05_tools-and-permissions.md` §6) covered the *tool-definition basics* (`@tool`/`tool()` four-part shape, handler return dict, `readOnlyHint`, the Python `structuredContent` caveat, `mcp__server__tool` naming, availability-vs-permission). This file references those, does not duplicate them. Where §4 below shows a snippet it is the minimal wiring snippet only.

---

## 1. Consuming external MCP servers

### 1.1 The option name and value shape — verbatim

The config goes on `ClaudeAgentOptions` (Python) / `Options` (TypeScript):

- **Python:** `mcp_servers` (snake_case).
- **TypeScript:** `mcpServers` (camelCase).

The Python field signature, verbatim from SDK source (`src/claude_agent_sdk/types.py:1615`):

```python
mcp_servers: dict[str, McpServerConfig] | str | Path = field(default_factory=dict)
```

So the value is one of three shapes:
1. **A dict** mapping a caller-chosen *server name* → a per-server config (the common case).
2. **A `str`** — a raw JSON string of `{"mcpServers": {...}}` (passed through to the CLI's `--mcp-config`).
3. **A `Path`** — a path to a `.mcp.json`-shaped file.

The per-server config is the union (`types.py:635`):

```python
McpServerConfig = (
    McpStdioServerConfig | McpSSEServerConfig | McpHttpServerConfig | McpSdkServerConfig
)
```

i.e. four variants: **stdio**, **SSE**, **HTTP**, and **sdk** (in-process). Field shapes per variant are in §2 (external transports) and §4 (the `sdk` variant).

The dict **key** is load-bearing: it becomes the `{server_name}` segment in every tool's fully-qualified name `mcp__{server_name}__{tool_name}` (§3, §6). The caller chooses it freely; it need not match the server's own self-reported name.

### 1.2 Minimal snippet — wiring an external stdio MCP server

From the MCP doc page ("Add an MCP server → In code"), trimmed:

Python:

```python
from claude_agent_sdk import query, ClaudeAgentOptions, ResultMessage

options = ClaudeAgentOptions(
    mcp_servers={
        "filesystem": {
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/projects"],
        }
    },
    allowed_tools=["mcp__filesystem__*"],   # without this Claude SEES the tools but cannot call them (§6)
)

async for message in query(prompt="List files in my project", options=options):
    if isinstance(message, ResultMessage) and message.subtype == "success":
        print(message.result)
```

TypeScript:

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

const options = {
  mcpServers: {
    filesystem: {
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/projects"],
    },
  },
  allowedTools: ["mcp__filesystem__*"],
};
```

### 1.3 `.mcp.json` — the on-disk form the CLI also uses

A `.mcp.json` file at the project root carries the **same** `{"mcpServers": {...}}` structure. Verbatim from the MCP doc page ("From a config file"):

> Create a `.mcp.json` file at your project root. The file is picked up when the `project` setting source is enabled, which it is for default `query()` options. If you set `settingSources` explicitly, include `"project"` for this file to load.

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/projects"]
    }
  }
}
```

**Load condition (load-bearing for ArkOS settingSources discipline — see topic 02):** `.mcp.json` is read **only when the `"project"` setting source is active**. With *default* `query()` options it is active; but if a substrate sets `settingSources` explicitly (e.g. `settingSources: []` to harden), it must include `"project"` or the file is silently ignored. In-code `mcpServers` does **not** depend on `settingSources` — it is always honored. So a substrate that wants deterministic, code-controlled MCP wiring should prefer the in-code `mcpServers` map over `.mcp.json`.

**`.mcp.json`-specific env expansion:** in `.mcp.json` the `${VAR}` syntax expands environment variables at runtime (e.g. `"GITHUB_TOKEN": "${GITHUB_TOKEN}"`). The in-code form has no `${}` magic — you interpolate in your own language (`os.environ[...]` / `process.env...`).

**`.mcp.json` transport-alias quirk:** in `.mcp.json` (and other JSON config files) `"streamable-http"` is accepted as an alias for `"http"`. The programmatic `mcpServers` option accepts **only `"http"`** (§2.2). Verbatim:

> For the streamable HTTP transport, use `"type": "http"` instead. In `.mcp.json` and other JSON config files, `"streamable-http"` is accepted as an alias for `"http"`. The programmatic `mcpServers` option accepts only `"http"`.

### 1.4 How the config reaches the agent (mechanism)

The Python transport serializes the `mcpServers` dict to JSON and passes it to the bundled Claude Code CLI subprocess via `--mcp-config` (`src/claude_agent_sdk/_internal/transport/subprocess_cli.py:307-332`). For `type == "sdk"` servers it strips the in-process `instance` field before serialization (the instance can't be JSON-encoded — it lives in the SDK process; see §4). String/Path forms are passed straight through as `--mcp-config <value>`. So **external** MCP servers are actually connected by the **CLI subprocess**, not the SDK library — the SDK is a passthrough for their config. (This matters for §7 lifecycle: the connection lives with the CLI subprocess, i.e. the session.)

A related option exists: `strict_mcp_config` / `strictMcpConfig` adds `--strict-mcp-config` to the CLI invocation (`subprocess_cli.py:340-341`), which restricts the CLI to only the MCP servers passed via the SDK (ignoring user/global `.mcp.json` discovery). Useful for a substrate that wants a hermetic, fully-declared server set.

---

## 2. Transports the SDK supports as a CLIENT

The SDK (via the CLI) supports **three external transports plus one in-process kind**:

| `type` literal      | Variant class (Python)    | Transport                        | Identifying fields |
| :------------------ | :------------------------ | :------------------------------- | :----------------- |
| `"stdio"` (optional)| `McpStdioServerConfig`    | local subprocess via stdin/stdout| `command` (req), `args`, `env` |
| `"sse"`             | `McpSSEServerConfig`      | Server-Sent Events over HTTP     | `url` (req), `headers` |
| `"http"`            | `McpHttpServerConfig`     | streamable HTTP                  | `url` (req), `headers` |
| `"sdk"`             | `McpSdkServerConfig`      | **in-process** (no network, no subprocess) — §4 | `name` (req), `instance` (req) |

Verbatim type definitions (`src/claude_agent_sdk/types.py:602-637`):

```python
class McpStdioServerConfig(TypedDict):
    """MCP stdio server configuration."""
    type: NotRequired[Literal["stdio"]]   # Optional for backwards compatibility
    command: str
    args: NotRequired[list[str]]
    env: NotRequired[dict[str, str]]

class McpSSEServerConfig(TypedDict):
    """MCP SSE server configuration."""
    type: Literal["sse"]
    url: str
    headers: NotRequired[dict[str, str]]

class McpHttpServerConfig(TypedDict):
    """MCP HTTP server configuration."""
    type: Literal["http"]
    url: str
    headers: NotRequired[dict[str, str]]

class McpSdkServerConfig(TypedDict):
    """SDK MCP server configuration."""
    type: Literal["sdk"]
    name: str
    instance: "McpServer"
```

### 2.1 stdio — local subprocess

For servers you run on the same machine. `type` is **optional** (absent ⇒ treated as stdio, "Optional for backwards compatibility"). The presence of `command` is the discriminator the docs use: "If the docs give you a **command to run** … use stdio."

```python
"github": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-github"],
    "env": {"GITHUB_TOKEN": os.environ["GITHUB_TOKEN"]},
}
```

### 2.2 HTTP (streamable HTTP) and SSE — remote

For cloud-hosted servers and remote APIs. Both carry a `url` and optional `headers`. The doc rule of thumb: "If the docs give you a **URL**, use HTTP or SSE."

- **`"type": "http"`** — the streamable HTTP transport (the current MCP remote transport). This is the variant the quickstart example uses (`url: "https://code.claude.com/docs/mcp"`).
- **`"type": "sse"`** — the older Server-Sent Events transport. Example url shape `https://api.example.com/mcp/sse`.

Programmatic config accepts **only `"http"`**, not `"streamable-http"` (the latter is a `.mcp.json`-only alias — §1.3).

### 2.3 Auth for HTTP/SSE — headers only; OAuth is caller's job

The SDK's only first-class auth mechanism for remote servers is **HTTP headers**. Verbatim ("HTTP headers for remote servers"):

> For HTTP and SSE servers, pass authentication headers directly in the server configuration:

```python
"secure-api": {
    "type": "http",
    "url": "https://api.example.com/mcp",
    "headers": {"Authorization": f"Bearer {os.environ['API_TOKEN']}"},
}
```

**OAuth is explicitly NOT handled by the SDK.** Verbatim ("OAuth2 authentication"):

> The [MCP specification supports OAuth 2.1] for authorization. The SDK doesn't handle OAuth flows automatically, but you can pass access tokens via headers after completing the OAuth flow in your application.

i.e. the substrate runs the OAuth dance itself, then injects the resulting bearer token as an `Authorization` header. There is no token-refresh or discovery support in the SDK.

**For stdio servers, auth is via `env`** (credentials passed as environment variables to the subprocess): `"env": {"GITHUB_TOKEN": ...}`. Verbatim: "Pass credentials through environment variables in the server configuration."

There is also a `"needs-auth"` connection status (§7, §8) the SDK surfaces when a remote server reports it requires authorization — but the SDK does not act on it; it just reports it.

---

## 3. What the agent consumes — TOOLS ONLY (not resources, not prompts)

This is a precise and load-bearing finding. MCP defines three server-side primitives — **tools**, **resources**, and **prompts**. **The Agent SDK surfaces only TOOLS to the model.**

**Evidence (docs):** every MCP doc-page example, the quickstart, the troubleshooting section, and the permission model are framed entirely around *tools* (`allowedTools`, `mcp__server__tool`, "what tools an MCP server provides"). The `system`/`init` message exposes `mcp_servers` connection status and tool inventory (`McpToolInfo`, §7) — there is **no** `mcp_resources` or `mcp_prompts` surface documented anywhere on the MCP or custom-tools pages.

**Evidence (source) — strongest for the in-process bridge:** the SDK's JSON-RPC bridge for in-process servers (`_internal/query.py:585-680`) hard-codes handling of exactly three MCP methods: `initialize`, `tools/list`, and `tools/call`. There is **no** `resources/list`, `resources/read`, `prompts/list`, or `prompts/get`. The `initialize` response advertises only the `tools` capability (`"capabilities": {"tools": {}}`, `query.py:592-594`). So even an in-process server that *could* expose resources/prompts would have those ignored.

For *external* servers the connection is the CLI's, so the bridge limitation above is specific to in-process servers — but the consumption surface the SDK exposes to the caller (the `allowedTools` namespace, the `init` message's tool inventory) is uniformly tool-shaped, and no doc surface exists for binding an external server's resources or prompts into the session. **Conclusion: treat MCP-as-consumed-by-the-SDK as "tools only."**

**Resources still appear — but only as tool *output*, not as MCP resources.** A tool handler may *return* a `resource` content block (`{type: "resource", resource: {uri, text|blob, mimeType}}`) — see topic 05 §6 and custom-tools "Return images and resources". Crucially the doc clarifies the URI is a *label*, not something the SDK fetches: "The URI `file:///tmp/report.md` is a label that Claude can reference later; **the SDK does not read from that path**." That is a tool-result shape, not MCP resource subscription/listing. There is no `@server:protocol://resource` mention mechanism documented for the SDK.

### 3.1 Tool namespace — `mcp__<server>__<tool>` (verified)

Verbatim ("Tool naming convention"):

> MCP tools follow the naming pattern `mcp__<server-name>__<tool-name>`. For example, a GitHub server named `"github"` with a `list_issues` tool becomes `mcp__github__list_issues`.

And custom-tools ("Tool name format"):

> * Pattern: `mcp__{server_name}__{tool_name}`
> * Example: A tool named `get_temperature` in server `weather` becomes `mcp__weather__get_temperature`

**Verified.** The `{server_name}` segment is the **caller's dict key** in `mcpServers` (§1.1), not the server's self-reported `serverInfo.name`. Double underscore (`__`) is the delimiter on both sides.

### 3.2 Discovering what tools a server exposes

At session start the SDK emits a `system` message with subtype `init` carrying the connected servers and their tools. Verbatim:

```typescript
if (message.type === "system" && message.subtype === "init") {
  console.log("Available MCP tools:", message.mcp_servers);
}
```

Python reads the same via `message.data.get("mcp_servers")` on a `SystemMessage` with `subtype == "init"`. The per-server entries carry `name`, `status`, and (when connected) tool info (`McpToolInfo` = `{name, description?, annotations?}`, `types.py`).

---

## 4. In-process SDK MCP servers — `create_sdk_mcp_server` / `createSdkMcpServer`

### 4.1 What it is — literally an `mcp.server.Server`, run in-process

The custom-tools page is explicit: "The server runs in-process inside your application, not as a separate process." The signature, verbatim from source (`src/claude_agent_sdk/__init__.py:307-309`):

```python
def create_sdk_mcp_server(
    name: str, version: str = "1.0.0", tools: list[SdkMcpTool[Any]] | None = None
) -> McpSdkServerConfig:
```

It returns an `McpSdkServerConfig` TypedDict — `{type: "sdk", name, instance}` — where `instance` is a real **`mcp.server.Server`** object from the standalone MCP SDK. Source proof (`__init__.py:379-391`):

```python
from mcp.server import Server
from mcp.types import (AudioContent, CallToolResult, EmbeddedResource,
                       ImageContent, ResourceLink, TextContent, Tool)
...
server = Server(name, version=version)
```

So the answer to "is it literally an MCP server, or MCP-shaped sugar?" is: **it is a genuine `mcp` SDK `Server` object** (the Agent SDK depends on `mcp>=1.23.0`), with `list_tools` and `call_tool` handlers registered against it (`__init__.py:445-452`). It is *not* a from-scratch shim — the tool schemas and result conversion go through real `mcp.types`. **But** (this is the §5 crux) it is **never bound to a transport that listens for connections.** It is invoked purely in-memory.

### 4.2 How it is invoked — in-memory JSON-RPC over the SDK↔CLI control channel

When the model calls an in-process tool, the CLI sends an `mcp_message` **control request** back over the existing stdio pipe to the SDK process; the SDK routes it to the right server object and replies. Source (`_internal/query.py:454-466`, `548-600`):

```python
elif subtype == "mcp_message":
    server_name = request_data.get("server_name")
    mcp_message = request_data.get("message")
    ...
    mcp_response = await self._handle_sdk_mcp_request(server_name, mcp_message)
```

`_handle_sdk_mcp_request` is documented in-source as a manual bridge (`query.py:551-555`):

> This acts as a bridge between JSONRPC messages from the CLI and the in-process MCP server. Ideally the MCP SDK would provide a method to handle raw JSONRPC, but for now we route manually.

It handles exactly `initialize`, `tools/list`, `tools/call` (§3). There is **no socket, no port, no stdio child** for the in-process server. The `instance` field is stripped before the config is serialized to the CLI (`subprocess_cli.py:312-317`) precisely because the instance lives only in the SDK process and is reached via this control-channel callback — the CLI only learns the server's *name* and `type: "sdk"`.

### 4.3 Minimal snippet (one tool, wired)

This duplicates topic 05 §6 only as the minimal anchor required by this file's brief; tool-definition depth lives there.

Python:

```python
from typing import Any
from claude_agent_sdk import tool, create_sdk_mcp_server, query, ClaudeAgentOptions

@tool("get_temperature", "Get the current temperature at a location",
      {"latitude": float, "longitude": float})
async def get_temperature(args: dict[str, Any]) -> dict[str, Any]:
    return {"content": [{"type": "text", "text": "Temperature: 64F"}]}

weather_server = create_sdk_mcp_server(name="weather", version="1.0.0",
                                       tools=[get_temperature])

options = ClaudeAgentOptions(
    mcp_servers={"weather": weather_server},          # key "weather" -> namespace segment
    allowed_tools=["mcp__weather__get_temperature"],  # pre-approve
)
```

TypeScript:

```typescript
import { tool, createSdkMcpServer, query } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";

const getTemperature = tool(
  "get_temperature", "Get the current temperature at a location",
  { latitude: z.number(), longitude: z.number() },
  async (args) => ({ content: [{ type: "text", text: "Temperature: 64F" }] }),
);

const weatherServer = createSdkMcpServer({
  name: "weather", version: "1.0.0", tools: [getTemperature],
});
// options: { mcpServers: { weather: weatherServer },
//            allowedTools: ["mcp__weather__get_temperature"] }
```

### 4.4 Performance / isolation vs an external subprocess MCP server

What the SDK claims, verbatim (`create_sdk_mcp_server` docstring, `__init__.py:311-317`):

> Unlike external MCP servers that run as separate processes, SDK MCP servers run directly in your application's process. This provides:
> - Better performance (no IPC overhead)
> - Simpler deployment (single process)
> - Easier debugging (same process)
> - Direct access to your application's state

Trade-offs for ArkOS (inferred from the mechanism, flagged as inference):
- **No process isolation.** An in-process tool handler shares the substrate's address space, GIL/event loop, and crash domain. A handler that segfaults or hard-blocks the event loop takes the SDK process with it — unlike an external stdio/HTTP server, which fails in its own process and surfaces as a connection error (§8). (Inferred from "runs directly in your application's process"; the docs do not state a crash-isolation guarantee either way.)
- **Direct state access** is the upside the docstring sells: the handler closes over substrate objects (DB handles, in-memory state) with no serialization boundary. The custom-tools "Server with application state access" example shows exactly this (`store.items.append(...)`).
- **Note the "no IPC overhead" claim is partial.** There *is* a control-channel round-trip between the CLI subprocess and the SDK process for each `mcp_message` (§4.2) — the tool runs in the SDK process but the *model* runs through the CLI subprocess, so a tool call still crosses the SDK↔CLI stdio boundary. The saving vs an external MCP server is the *second* hop (CLI→external-server subprocess) and the server's own startup, not a fully in-RAM model loop.

### 4.5 Python/TS divergence on in-process servers

- **`structuredContent`:** TS in-process tools can return it; the Python `@tool` decorator forwards only `content` and `is_error` (topic 05 §6). Verbatim doc `<Note>`: "To return `structuredContent` from Python, run a standalone MCP server instead of an in-process SDK server." (This is itself a pointer toward §5's standalone-server path.)
- **Bridge implementation note (Python only):** the Python bridge is manual because "the Python MCP SDK lacks the Transport abstraction that TypeScript has" (`query.py:579-584`). TS uses `server.connect(transport)` with a custom transport; Python uses `server.run(read_stream, write_stream)` and so the SDK routes `initialize`/`tools/list`/`tools/call` by hand. Functional surface is the same (tools only); the internal plumbing differs.

---

## 5. THE KEY QUESTION — can the SDK publish a standalone MCP server other agents connect to?

**No. Definitively no, and confirmed from source — not inferred.**

`create_sdk_mcp_server` / `createSdkMcpServer` produces an **in-process tool host consumed only by THIS SDK process's own agent.** It is never bound to a listening transport, so no external client can connect to it.

### 5.1 What "definitively" rests on

1. **It returns a config, not a running endpoint.** `create_sdk_mcp_server(...) -> McpSdkServerConfig` = `{type: "sdk", name, instance}`. There is no host, no port, no `listen()`, no `bind()`, no URL anywhere in the return value or the function body (`__init__.py:307-...`).
2. **The `instance` is reachable only via the in-memory control-channel callback.** The server object is invoked exclusively through `_handle_sdk_mcp_request` in response to `mcp_message` control requests from the *bundled CLI subprocess that this SDK spawned* (`query.py:454-466`, `548-600`). It is never `server.run(...)` over a real stdio/socket transport (the source comment at `query.py:579-584` explicitly notes the Python MCP `Server.run` path is *not* used; methods are routed manually).
3. **The instance is stripped before crossing any boundary.** `subprocess_cli.py:312-317` removes the `instance` field, passing the CLI only `{type, name}`. Nothing outside the SDK process ever gets a handle to the server.
4. **No publish/serve/expose API exists** in the public surface. The Agent SDK's exports (`__init__.py:653` region: `tool`, `create_sdk_mcp_server`, `query`, `ClaudeSDKClient`, session helpers) contain no server-publishing entry point. There is no `serve()`, `run_server()`, `mount()`, or transport-binding function.

This matches the consumption model: the SDK is an MCP **client** (it dials external stdio/HTTP/SSE servers, §1–2) and an in-process **tool provider for its own agent** (§4). It is not an MCP **server publisher** for foreign clients.

### 5.2 Consequence for ArkOS

**Exposing ArkOS substrate primitives as an MCP server that arbitrary external agents (not this SDK's own agent) dial into requires a SEPARATE MCP server implementation.** The Agent SDK does not do this. The libraries to use instead — all independent of the Agent SDK:

| Language   | Library                                  | Latest (2026-05-25) | Notes |
| :--------- | :--------------------------------------- | :------------------ | :---- |
| Python     | `mcp` (the official MCP SDK; `mcp.server.Server` / its `FastMCP` API) | `1.27.1` | This is the *same* package the Agent SDK already depends on (`mcp>=1.23.0`) — but used directly with a real transport (`stdio_server()`, streamable-HTTP, or SSE) you get a publishable server. |
| Python     | `fastmcp` (the standalone FastMCP project, now v2/v3) | `3.3.1` | Higher-level decorator API + first-class HTTP/SSE serving, auth, deployment. The ergonomic choice for a network-exposed substrate server. |
| TypeScript | `@modelcontextprotocol/sdk` (official)   | `1.29.0` | `McpServer` + `StdioServerTransport` / `StreamableHTTPServerTransport`. |

The shape of the ArkOS decision: an ArkOS substrate server that publishes primitives (task lifecycle, spec access, journals — analogous to today's `ark agent` CLI namespace) to *external* agent runtimes would be written against `mcp` / `fastmcp` / `@modelcontextprotocol/sdk` and bound to a transport. Agents built on the Claude Agent SDK would then **consume** it via `mcpServers` (§1) over stdio or HTTP. The Agent SDK sits on the *client* side of that boundary; it never sits on the *server* side for foreign clients.

### 5.3 What is confirmed vs inferred

- **Confirmed from source:** `create_sdk_mcp_server` returns a config wrapping an in-memory `mcp.server.Server`; the server is invoked only via the SDK↔CLI in-process control channel; the `instance` never leaves the SDK process; there is no listening transport, port, or publish API. (Points 1–4 above all cite specific source lines.)
- **Confirmed from docs:** the custom-tools page frames `createSdkMcpServer` as "runs in-process inside your application, not as a separate process"; the MCP page's only server-*authoring* pointer is to the standalone MCP SDK ("Build your own MCP server that runs in-process with your SDK application" — note: *in-process with your SDK application*, i.e. still consumed by your own agent, not published).
- **Inferred (low risk):** that the standalone `mcp`/`fastmcp`/`@modelcontextprotocol/sdk` libraries are the correct alternative for publishing. This is the canonical MCP server path per modelcontextprotocol.io and is not Agent-SDK-specific; the Agent SDK's own dependency on `mcp` corroborates it. The *specific ArkOS architecture* (which primitives to expose, which transport) is a design decision out of scope here.

---

## 6. Tool naming / collision and `allowedTools`/`disallowedTools` (cross-ref topic 05)

**Namespacing prevents collision with built-ins.** Built-in tools are bare identifiers (`Read`, `Bash`, `Edit` — topic 05 §1). MCP tools are always `mcp__{server}__{tool}` (§3.1). The `mcp__` prefix means an MCP tool can never shadow a built-in, and two servers can each expose a `query` tool without collision (`mcp__db__query` vs `mcp__postgres__query`). Collision *within* one server name is the caller's responsibility (don't reuse a server key for two different servers).

**MCP tools require explicit permission — they are NOT auto-available.** Verbatim (MCP page "Allow MCP tools"):

> MCP tools require explicit permission before Claude can use them. Without permission, Claude will see that tools are available but won't be able to call them.

**Allow specific MCP tools** via `allowedTools`/`allowed_tools` with three granularities (verbatim):

```typescript
allowedTools: [
  "mcp__github__*",          // All tools from the github server
  "mcp__db__query",          // Only the query tool from db server
  "mcp__slack__send_message" // Only send_message from slack server
]
```

Wildcards (`*`) allow a whole server. This is the SDK's preferred MCP-grant mechanism — verbatim `<Note>`:

> **Prefer `allowedTools` over permission modes for MCP access.** `permissionMode: "acceptEdits"` does not auto-approve MCP tools (only file edits and filesystem Bash commands). `permissionMode: "bypassPermissions"` does auto-approve MCP tools but also disables all other safety prompts, which is broader than necessary. A wildcard in `allowedTools` grants exactly the MCP server you want and nothing more.

→ Load-bearing for ArkOS: `acceptEdits` does **not** auto-approve MCP tools; only `allowedTools` (preferred) or `bypassPermissions` (too broad) do. To grant an MCP server without loosening anything else, use `mcp__<server>__*` in `allowedTools`.

**Deny specific MCP tools** via `disallowedTools`/`disallowed_tools` using the same `mcp__server__tool` / `mcp__server__*` forms. Per topic 05 §5: scoped MCP rules are `mcp__puppeteer` (any tool from server), `mcp__puppeteer__*` (wildcard, same effect), `mcp__puppeteer__puppeteer_navigate` (one tool). Deny rules outrank allow rules and even `bypassPermissions` (topic 05 §9 precedence). So a substrate can pin an MCP server to a tool allowlist (allow `mcp__x__safe_*`) and hard-deny dangerous tools (deny `mcp__x__delete_*`) and the deny wins.

The full precedence order (blocking hook > deny rule > permission mode > allow rule > `canUseTool`) is owned by topic 05 §9 and applies identically to MCP tool names.

---

## 7. Lifecycle — when MCP servers start/stop relative to a session

**External servers (stdio/HTTP/SSE) are connected by the CLI subprocess at session init and live for the session's duration.**

- **Start:** at the beginning of a `query()` (or a `ClaudeSDKClient` connection), the SDK passes server configs to the CLI via `--mcp-config` (§1.4); the CLI connects each server during initialization. The SDK then emits the `system`/`init` message carrying each server's connection status (§3.2, §8). For stdio servers this means a child process is **spawned per session** (the CLI launches `command args`). They are not shared across separate `query()` calls — each fresh `query()` produces a fresh CLI subprocess and therefore fresh stdio children.
- **Connection timeout:** verbatim ("Connection timeouts"): "The MCP SDK has a default timeout of 60 seconds for server connections. If your server takes longer to start, the connection will fail." Mitigations the doc lists: a lighter server, **pre-warming the server before starting your agent**, or checking server logs.
- **Stop / cleanup:** server lifetime is bound to the CLI subprocess / session. When the session ends (the `query()` async iterator is exhausted, or the `ClaudeSDKClient` context manager exits), the CLI subprocess terminates and its child MCP processes / connections go with it. The docs do not document an explicit per-server `close()` for external servers; cleanup is implicit via subprocess teardown. **In-process (`sdk`) servers** have no separate lifecycle — the `create_sdk_mcp_server` docstring states "Server lifecycle is managed automatically by the SDK" (`__init__.py:373`); the server object simply lives as long as the SDK process holds it, and is reached on-demand via the control channel (§4.2).
- **Reuse:** to keep a server connection alive across multiple turns, use the persistent `ClaudeSDKClient` (streaming mode) rather than repeated one-shot `query()` calls. The client exposes mid-session MCP management (below).

**Mid-session MCP control (Python `ClaudeSDKClient`, streaming mode only):**

| Method | Effect | Source |
| :----- | :----- | :----- |
| `get_mcp_status()` | Returns `{mcpServers: [McpServerStatus...]}` with per-server `name`/`status`/`serverInfo`. | `client.py` / `query.py:725` |
| `reconnect_mcp_server(name)` | Retry a `failed`/disconnected server; raises on failure. | `client.py:402-422` |
| `toggle_mcp_server(name, enabled)` | Disable (disconnect + remove its tools) / enable (reconnect + restore tools) a server mid-session. | `client.py:424-448` |

These exist only in **streaming mode** (a connected `ClaudeSDKClient`), not in one-shot `query()`. For ArkOS this is the surface to keep a long-lived agent with dynamically-managed MCP servers (e.g. attach a server only for one phase, then `toggle_mcp_server(..., enabled=False)`).

---

## 8. Error handling — unreachable server / failing tool / mid-session crash

### 8.1 Server fails to connect → reported in `system`/`init`, not raised

The SDK does **not** raise on a server that fails to connect; it reports per-server status in the `init` message. The connection-status enum, verbatim (`types.py`):

```python
McpServerConnectionStatus = Literal["connected", "failed", "needs-auth", "pending", "disabled"]
```

`McpServerStatus = {name, status, serverInfo?}`. Detection pattern (verbatim, "Error handling"):

```python
async for message in query(prompt="Process data", options=options):
    if isinstance(message, SystemMessage) and message.subtype == "init":
        failed_servers = [s for s in message.data.get("mcp_servers", [])
                          if s.get("status") != "connected"]
        if failed_servers:
            print(f"Failed to connect: {failed_servers}")
```

A server with `status == "failed"` still lets the session proceed — its tools are simply absent. So **a substrate must inspect the `init` message** to fail-fast if a required server didn't connect; otherwise the agent runs tool-less and may improvise. Documented common causes (verbatim "Server shows 'failed' status"): missing env vars, server not installed (`npx` package missing / Node not in PATH), invalid connection string, network issues for remote servers. `"needs-auth"` indicates the remote server requires authorization the caller hasn't supplied (§2.3).

### 8.2 A tool call fails

For **in-process** custom tools, error semantics are owned by topic 05 §6 / custom-tools "Handle errors": an **uncaught exception ends the whole `query()` call**; returning `isError: true`/`"is_error": True` keeps the loop alive and lets Claude react. For **external** servers, a tool that errors returns an MCP error result back through the CLI; the model sees it as a failed tool result and can retry or route around it — the session is not torn down by a single failed external tool call.

### 8.3 Server crashes mid-session

If a connected external server dies mid-session, its status transitions away from `connected`. In streaming mode the substrate can poll `get_mcp_status()` and call `reconnect_mcp_server(name)` to recover (§7). There is no documented automatic-reconnect; recovery is caller-driven. In one-shot `query()` mode there is no recovery handle — a mid-session server death simply removes those tools for the remainder of the run.

### 8.4 Session-level error result

Beyond per-server status, an execution-level failure surfaces as a `result` message with `subtype == "error_during_execution"` (verbatim TS/Python error-handling snippet). A substrate should branch on `ResultMessage.subtype` (`"success"` vs error subtypes) regardless of MCP specifics. (`ResultMessage` subtypes belong to topic 03.)

---

## Decision table — MCP facts that shape ArkOS

| Question | Answer | Where |
| :------- | :----- | :---- |
| Option to consume servers | `mcp_servers` (Py) / `mcpServers` (TS) on options; or `.mcp.json` when `project` source on | §1 |
| Value shape | `dict[str, McpServerConfig] \| str \| Path`; key = namespace segment | §1.1 |
| Transports (as client) | stdio, http (streamable), sse, + in-process `sdk` | §2 |
| Remote auth | headers only; OAuth is caller's job (token via header) | §2.3 |
| Resources / prompts consumed? | **No — tools only** (bridge handles only `initialize`/`tools/list`/`tools/call`) | §3 |
| Tool namespace | `mcp__<server>__<tool>`, `__` delimiter, server = caller's dict key | §3.1, §6 |
| In-process server is a real MCP server? | Yes — wraps `mcp.server.Server`, but never bound to a listening transport | §4.1 |
| Can SDK **publish** a server for foreign agents? | **NO.** In-process only, consumed by this SDK's own agent. Use standalone `mcp`/`fastmcp`/`@modelcontextprotocol/sdk` to publish. | §5 |
| Allow/deny specific MCP tools | `allowedTools`/`disallowedTools` with `mcp__s__t` / `mcp__s__*`; prefer allowlist over modes; `acceptEdits` does NOT grant MCP | §6 |
| Lifecycle | external = per-session CLI subprocess connection; stdio spawns child per session; 60s connect timeout; mid-session control via `ClaudeSDKClient` | §7 |
| Errors | connection failures reported in `init` (status enum), not raised; in-process uncaught throw kills `query()`; `reconnect_mcp_server`/`toggle_mcp_server` for recovery | §8 |

---

## Caveats / Not found

- **Resources & prompts: confirmed not surfaced** for in-process servers (source: the bridge handles only three tool methods and advertises only the `tools` capability). For *external* servers, no doc surface for binding their resources/prompts into the session was found — treat as "tools only" but flagged: a byte-exact check of the CLI's external-server handling (the CLI is a bundled binary, not in this Python repo) was not performed. If a future ArkOS need depends on MCP resources/prompts, verify empirically against the CLI.
- **In-process crash isolation** is inferred from "runs directly in your application's process," not from an explicit doc guarantee. The docs neither promise nor deny that a crashing handler is isolated; the mechanism implies it is not.
- **"No IPC overhead"** is the docstring's claim; in practice an in-process tool call still crosses the SDK↔CLI stdio control channel once (§4.4). The claim is true relative to an *external* server (saves the second hop + startup), not literally zero-IPC.
- **TS source not read this pass.** All source citations are the Python repo at 0.2.87. The TS SDK (0.3.150) is taken from the docs (which show TS+Python side by side) and topic 01's TS API listing. The TS in-process bridge uses `server.connect(transport)` (per the Python source's own comparison comment) rather than manual routing, but the consumed surface (tools only) and the publish answer (§5: no) are the same — the doc framing ("runs in-process … not as a separate process") is language-agnostic.
- **Streamable-http vs http alias:** programmatic config takes only `"http"`; `"streamable-http"` is a `.mcp.json`-only alias. Don't pass `"streamable-http"` in the in-code `mcpServers` map.
- **`McpClaudeAIProxyServerConfig` (`type: "claudeai-proxy"`)** exists in `types.py` as an **output-only** status type (servers proxied through Claude.ai). It is not a config a third-party SDK integrator constructs; consistent with topic 01's note that claude.ai login is not available to third-party integrators. Not covered as a consumable transport.
- **Version pins hold:** Python 0.2.87 and TS 0.3.150 are both the latest published as of 2026-05-25 (verified via PyPI/npm). No newer release to flag.

## Primary sources

- [Connect to external tools with MCP (Agent SDK)](https://code.claude.com/docs/en/agent-sdk/mcp) — `mcpServers`/`mcp_servers`, `.mcp.json`, transports (stdio/http/sse), `streamable-http` alias, auth (env + headers + OAuth-is-yours), `mcp__server__tool` naming, `allowedTools` grant + `acceptEdits` caveat, `init`-message discovery + status check, error handling, 60s timeout, troubleshooting.
- [Give Claude custom tools (Agent SDK)](https://code.claude.com/docs/en/agent-sdk/custom-tools) — `tool()`/`@tool`, `createSdkMcpServer`/`create_sdk_mcp_server` "runs in-process … not as a separate process", tool-name format, error semantics, resource/image blocks (URI is a label, SDK doesn't read it), Python `structuredContent` → use standalone server.
- SDK source `anthropics/claude-agent-sdk-python` @ 0.2.87 (read locally):
  - `src/claude_agent_sdk/types.py:602-637` — `McpStdioServerConfig`/`McpSSEServerConfig`/`McpHttpServerConfig`/`McpSdkServerConfig` + `McpServerConfig` union; `:1615` `mcp_servers` field; `McpServerConnectionStatus`/`McpServerStatus`/`McpToolInfo`/`McpServerInfo` (status types); `McpClaudeAIProxyServerConfig` (output-only).
  - `src/claude_agent_sdk/__init__.py:166-232` (`tool`/`SdkMcpTool`), `:307-...` (`create_sdk_mcp_server` signature + docstring + `from mcp.server import Server`, `server.list_tools()`/`server.call_tool()` registration).
  - `src/claude_agent_sdk/_internal/transport/subprocess_cli.py:307-341` — `--mcp-config` serialization, `instance`-stripping for `type:"sdk"`, `--strict-mcp-config`.
  - `src/claude_agent_sdk/_internal/query.py:454-466`, `548-680` — `mcp_message` control-request routing, `_handle_sdk_mcp_request` manual bridge (only `initialize`/`tools/list`/`tools/call`; advertises only `tools` capability).
  - `src/claude_agent_sdk/client.py:402-448` — `reconnect_mcp_server`, `toggle_mcp_server`, `get_mcp_status` usage.
  - `pyproject.toml:31` — `mcp>=1.23.0` dependency.
- Standalone MCP server libraries (for §5, version-checked 2026-05-25): [PyPI `mcp` 1.27.1](https://pypi.org/project/mcp/), [PyPI `fastmcp` 3.3.1](https://pypi.org/project/fastmcp/), [npm `@modelcontextprotocol/sdk` 1.29.0](https://www.npmjs.com/package/@modelcontextprotocol/sdk), [Model Context Protocol](https://modelcontextprotocol.io).
- Neighbor corpus (cross-ref, not re-derived): `01_overview-and-relationship-to-claude-code.md` (§5 `McpServerConfig` union, `create_sdk_mcp_server`/`createSdkMcpServer` one-liners), `05_tools-and-permissions.md` (§5 MCP scoped rules, §6 in-process `@tool` basics + Python `structuredContent` caveat, §9 precedence).
