# Research: Claude Agent SDK streaming event model

- Query: enumerate every typed event the SDK emits during a session, the async-iterator consume pattern in Python and TS, turn vs. session boundaries, tool-use streaming, error/abort behavior, subagent attribution, and how the SDK stream compares to Claude Code's `--output-format stream-json`.
- Scope: external (primary sources: docs.claude.com / code.claude.com / SDK source on GitHub).
- Date: 2026-05-25
- SDK versions referenced:
  - Python `claude-agent-sdk` **0.2.87** (PyPI, released 2026-05-23).
  - TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.142** (npm).
  - Underlying Claude Code CLI: v2.1.x (tool rename "Task" → "Agent" landed at v2.1.63; `SDKRateLimitInfo` / `SDKRateLimitEvent` added at v2.1.45).
- Out of scope here (covered in sibling files): session lifecycle (02), hooks (04), subagent definitions and dispatch mechanics (06).

---

## 1. Event taxonomy

The SDK models the stream as a single `Message` (Python) / `SDKMessage` (TS) union. The Python union is the documented, narrower one; the TS union has the same core variants plus a long tail of CLI-only system events.

### 1.1 Python — `Message` union (verbatim)

From `src/claude_agent_sdk/types.py` (main branch, 2026-05) and the [Python reference page](https://code.claude.com/docs/en/agent-sdk/python):

```python
Message = (
    UserMessage
    | AssistantMessage
    | SystemMessage
    | ResultMessage
    | StreamEvent
    | RateLimitEvent
    # SystemMessage subclasses (the doc page also lists these as union members):
    | TaskStartedMessage
    | TaskProgressMessage
    | TaskNotificationMessage
)
```

The dataclasses (verbatim from `types.py`):

```python
@dataclass
class UserMessage:
    """User message."""
    content: str | list[ContentBlock]
    uuid: str | None = None
    parent_tool_use_id: str | None = None
    tool_use_result: dict[str, Any] | None = None

@dataclass
class AssistantMessage:
    """Assistant message with content blocks."""
    content: list[ContentBlock]
    model: str
    parent_tool_use_id: str | None = None
    error: AssistantMessageError | None = None
    usage: dict[str, Any] | None = None
    message_id: str | None = None
    stop_reason: str | None = None
    session_id: str | None = None
    uuid: str | None = None

@dataclass
class SystemMessage:
    """System message with metadata."""
    subtype: str
    data: dict[str, Any]

@dataclass
class ResultMessage:
    """Result message with cost and usage information."""
    subtype: str
    duration_ms: int
    duration_api_ms: int
    is_error: bool
    num_turns: int
    session_id: str
    stop_reason: str | None = None
    total_cost_usd: float | None = None
    usage: dict[str, Any] | None = None
    result: str | None = None
    structured_output: Any = None
    model_usage: dict[str, Any] | None = None
    permission_denials: list[Any] | None = None
    deferred_tool_use: DeferredToolUse | None = None
    errors: list[str] | None = None
    api_error_status: int | None = None
    uuid: str | None = None

@dataclass
class StreamEvent:
    """Stream event for partial message updates during streaming."""
    uuid: str
    session_id: str
    event: dict[str, Any]   # raw Anthropic API stream event
    parent_tool_use_id: str | None = None

@dataclass
class RateLimitEvent:
    """Rate limit event emitted when rate limit info changes."""
    rate_limit_info: RateLimitInfo
    uuid: str
    session_id: str
```

`AssistantMessageError` is a `Literal`:

```python
AssistantMessageError = Literal[
    "authentication_failed",
    "billing_error",
    "rate_limit",
    "invalid_request",
    "server_error",
    "max_output_tokens",
    "unknown",
]
```

`SystemMessage` subclasses documented on the reference page:

```python
@dataclass
class TaskStartedMessage(SystemMessage):
    task_id: str
    description: str
    uuid: str
    session_id: str
    tool_use_id: str | None = None
    task_type: str | None = None      # "local_bash" | "local_agent" | "remote_agent"

@dataclass
class TaskProgressMessage(SystemMessage):
    task_id: str
    description: str
    usage: TaskUsage
    uuid: str
    session_id: str
    tool_use_id: str | None = None
    last_tool_name: str | None = None

@dataclass
class TaskNotificationMessage(SystemMessage):
    task_id: str
    status: TaskNotificationStatus    # "completed" | "failed" | "stopped"
    output_file: str
    summary: str
    uuid: str
    session_id: str
    tool_use_id: str | None = None
    usage: TaskUsage | None = None
```

### 1.2 Python — `ContentBlock` union

Content blocks ride **inside** `AssistantMessage.content` and `UserMessage.content` (verbatim from `types.py`):

```python
@dataclass
class TextBlock:
    text: str

@dataclass
class ThinkingBlock:
    thinking: str
    signature: str

@dataclass
class ToolUseBlock:
    id: str
    name: str
    input: dict[str, Any]

@dataclass
class ToolResultBlock:
    tool_use_id: str
    content: str | list[dict[str, Any]] | None = None
    is_error: bool | None = None

@dataclass
class ServerToolUseBlock:
    """Server-side tool use (advisor, web_search, web_fetch, code_execution, ...)."""
    id: str
    name: ServerToolName
    input: dict[str, Any]

@dataclass
class ServerToolResultBlock:
    tool_use_id: str
    content: dict[str, Any]

ContentBlock = (
    TextBlock | ThinkingBlock | ToolUseBlock | ToolResultBlock
    | ServerToolUseBlock | ServerToolResultBlock
)

ServerToolName = Literal[
    "advisor", "web_search", "web_fetch", "code_execution",
    "bash_code_execution", "text_editor_code_execution",
    "tool_search_tool_regex", "tool_search_tool_bm25",
]
```

Server-tool blocks have no caller-returned result (the API executes them server-side); they appear in the stream for visibility only.

### 1.3 TypeScript — `SDKMessage` union

From the [TypeScript reference page](https://code.claude.com/docs/en/agent-sdk/typescript):

```typescript
type SDKMessage =
  | SDKAssistantMessage
  | SDKUserMessage
  | SDKUserMessageReplay
  | SDKResultMessage
  | SDKSystemMessage
  | SDKPartialAssistantMessage
  | SDKCompactBoundaryMessage
  | SDKStatusMessage
  | SDKLocalCommandOutputMessage
  | SDKHookStartedMessage
  | SDKHookProgressMessage
  | SDKHookResponseMessage
  | SDKPluginInstallMessage
  | SDKToolProgressMessage
  | SDKAuthStatusMessage
  | SDKTaskNotificationMessage
  | SDKTaskStartedMessage
  | SDKTaskProgressMessage
  | SDKTaskUpdatedMessage
  | SDKSessionStateChangedMessage
  | SDKNotificationMessage
  | SDKFilesPersistedEvent
  | SDKToolUseSummaryMessage
  | SDKMemoryRecallMessage
  | SDKRateLimitEvent
  | SDKElicitationCompleteMessage
  | SDKPermissionDeniedMessage
  | SDKPromptSuggestionMessage
  | SDKAPIRetryMessage
  | SDKMirrorErrorMessage;
```

The TS union is the **richer** of the two. The Python `Message` union exposes only the events the Python facade surfaces (assistant/user/system/result/stream/ratelimit/task-*); the TS union exposes every event the underlying CLI emits, including hook lifecycle (`SDKHookStartedMessage`, `SDKHookProgressMessage`, `SDKHookResponseMessage`), `SDKAPIRetryMessage`, `SDKPermissionDeniedMessage`, and so on. Both SDKs share the same wire data; the Python wrapper just hides the variants it has not yet typed.

Key member shapes (verbatim):

```typescript
type SDKAssistantMessage = {
  type: "assistant";
  uuid: UUID;
  session_id: string;
  message: BetaMessage;                 // Anthropic SDK type — full assistant Message
  parent_tool_use_id: string | null;
  error?: SDKAssistantMessageError;
};

type SDKUserMessage = {
  type: "user";
  uuid?: UUID;
  session_id?: string;
  message: MessageParam;                // Anthropic SDK MessageParam
  parent_tool_use_id: string | null;
  isSynthetic?: boolean;
  shouldQuery?: boolean;
  tool_use_result?: unknown;
  origin?: SDKMessageOrigin;
};

type SDKSystemMessage = {
  type: "system";
  subtype: "init";
  uuid: UUID;
  session_id: string;
  agents?: string[];
  apiKeySource: ApiKeySource;
  betas?: string[];
  claude_code_version: string;
  cwd: string;
  tools: string[];
  mcp_servers: { name: string; status: string }[];
  model: string;
  permissionMode: PermissionMode;
  slash_commands: string[];
  output_style: string;
  skills: string[];
  plugins: { name: string; path: string }[];
};

type SDKPartialAssistantMessage = {
  type: "stream_event";
  event: BetaRawMessageStreamEvent;     // Anthropic raw streaming event
  parent_tool_use_id: string | null;
  uuid: UUID;
  session_id: string;
};

type SDKCompactBoundaryMessage = {
  type: "system";
  subtype: "compact_boundary";
  uuid: UUID;
  session_id: string;
  compact_metadata: {
    trigger: "manual" | "auto";
    pre_tokens: number;
  };
};
```

`SDKResultMessage` is a **discriminated union over `subtype`**:

```typescript
// Success
type SDKResultMessage = {
  type: "result";
  subtype: "success";
  uuid: UUID;
  session_id: string;
  duration_ms: number;
  duration_api_ms: number;
  is_error: boolean;
  api_error_status?: number | null;
  num_turns: number;
  result: string;                       // final assistant text, concatenated
  stop_reason: string | null;
  ttft_ms?: number;
  total_cost_usd: number;
  usage: NonNullableUsage;
  modelUsage: { [modelName: string]: ModelUsage };
  permission_denials: SDKPermissionDenial[];
  structured_output?: unknown;
  deferred_tool_use?: { id: string; name: string; input: Record<string, unknown> };
  terminal_reason?: TerminalReason;
  fast_mode_state?: FastModeState;
  origin?: SDKMessageOrigin;
};

// Error
type SDKResultMessage = {
  type: "result";
  subtype:
    | "error_max_turns"
    | "error_during_execution"
    | "error_max_budget_usd"
    | "error_max_structured_output_retries";
  uuid: UUID;
  session_id: string;
  duration_ms: number;
  duration_api_ms: number;
  is_error: boolean;
  num_turns: number;
  stop_reason: string | null;
  total_cost_usd: number;
  usage: NonNullableUsage;
  modelUsage: { [modelName: string]: ModelUsage };
  permission_denials: SDKPermissionDenial[];
  errors: string[];                     // structured error strings (not exceptions)
  terminal_reason?: TerminalReason;
  fast_mode_state?: FastModeState;
  origin?: SDKMessageOrigin;
};
```

The Python `ResultMessage.subtype` is `str` (not a `Literal`) but the observed values match the TS literals — `"success"`, `"error_max_turns"`, `"error_during_execution"`, `"error_max_budget_usd"`, `"error_max_structured_output_retries"` (the [streaming output doc](https://code.claude.com/docs/en/agent-sdk/streaming-output) and [interrupt example](https://code.claude.com/docs/en/agent-sdk/python) only document `"success"` and `"error_during_execution"` explicitly; the other three come from the TS type definition).

### 1.4 Trigger / appearance matrix

| Event                              | Triggered by                                                       | When in session                                |
| ---------------------------------- | ------------------------------------------------------------------ | ---------------------------------------------- |
| `SDKSystemMessage` (`subtype:"init"`) | Session start — handshake with the CLI                          | First message, exactly once                    |
| `UserMessage` / `SDKUserMessage`   | Synthetic prompt replay or tool-result delivery into the model    | Before each model turn                         |
| `AssistantMessage` / `SDKAssistantMessage` | Model finishes a complete turn (all blocks collected)      | One per model turn                             |
| `StreamEvent` / `SDKPartialAssistantMessage` | Streaming enabled (`include_partial_messages`); per-token API event | Many per turn; absent when streaming off |
| `SDKCompactBoundaryMessage` (`subtype:"compact_boundary"`) | Auto/manual context compaction         | Mid-session, between turns                     |
| `SystemMessage` (other subtypes)   | CLI emits transport-level info (hooks, plugin install, status…) | Throughout                                     |
| `TaskStartedMessage`               | Background task (`background: true` agent) begins                 | When subagent is spawned in background mode    |
| `TaskProgressMessage`              | Background task progress tick                                     | Multiple, between start and notification       |
| `TaskNotificationMessage`          | Background task done (`completed`/`failed`/`stopped`)             | Once per task                                  |
| `RateLimitEvent` / `SDKRateLimitEvent` | Rate-limit bucket utilization changes                          | Sporadic; tied to API headers                  |
| `ResultMessage` / `SDKResultMessage` | Session reaches terminal state                                  | Last message, exactly once                     |

---

## 2. How to consume the stream

### 2.1 Minimal consumer — Python

`query()` is an async generator; iteration is **async-only**. There is no synchronous wrapper in the SDK. The standard pattern uses `asyncio.run` (or `anyio.run`):

```python
import asyncio
from claude_agent_sdk import query, AssistantMessage, TextBlock, ResultMessage

async def main():
    async for message in query(prompt="What is 2 + 2?"):
        if isinstance(message, AssistantMessage):
            for block in message.content:
                if isinstance(block, TextBlock):
                    print(block.text, end="")
        elif isinstance(message, ResultMessage):
            print(f"\ncost=${message.total_cost_usd:.4f} turns={message.num_turns}")

asyncio.run(main())
```

Dispatch is by `isinstance` — there is no `type` discriminator field on the Python dataclasses (the Python dataclasses do not carry a `type: Literal["..."]` field — the class identity *is* the discriminator).

`query()` signature ([Python reference](https://code.claude.com/docs/en/agent-sdk/python)):

```python
async def query(
    *,
    prompt: str | AsyncIterable[dict[str, Any]],
    options: ClaudeAgentOptions | None = None,
    transport: Transport | None = None,
) -> AsyncIterator[Message]
```

For long-lived sessions with input-stream and interrupt support, use `ClaudeSDKClient` instead:

```python
class ClaudeSDKClient:
    async def connect(self, prompt: str | AsyncIterable[dict] | None = None) -> None
    async def query(self, prompt: str | AsyncIterable[dict], session_id: str = "default") -> None
    async def receive_messages(self) -> AsyncIterator[Message]
    async def receive_response(self) -> AsyncIterator[Message]    # iterates until ResultMessage
    async def interrupt(self) -> None
    async def disconnect(self) -> None
```

`receive_response()` is the convenience helper for "drain to next ResultMessage."

### 2.2 Minimal consumer — TypeScript

`query()` returns an `AsyncGenerator<SDKMessage>` and the `Query` interface extends it with `interrupt()`. Dispatch is on the `type` (and `subtype`) discriminator:

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({ prompt: "What is 2 + 2?" })) {
  if (message.type === "assistant") {
    for (const block of message.message.content) {
      if (block.type === "text") {
        process.stdout.write(block.text);
      }
    }
  } else if (message.type === "result" && message.subtype === "success") {
    console.log(`\ncost=$${message.total_cost_usd} turns=${message.num_turns}`);
  }
}
```

**Sync alternative:** none. Both SDKs require an async runtime. The Python facade is built on `asyncio`/`anyio`; the TS facade is native async iteration. A "sync" caller must drive it with `asyncio.run()` / top-level `await` / equivalent.

**Shape divergence** to keep in mind:
- Python: `AssistantMessage.content: list[ContentBlock]` — direct list of typed dataclasses, blocks discriminated by Python class.
- TS: `SDKAssistantMessage.message: BetaMessage` — content is `message.message.content`, blocks discriminated by `block.type` string.

The subagent doc page explicitly calls out this divergence:

> The message structure differs between SDKs. In Python, content blocks are accessed directly via `message.content`. In TypeScript, `SDKAssistantMessage` wraps the Claude API message, so content is accessed via `message.message.content`.

---

## 3. Turn boundaries

There is **no dedicated "turn-end" event**. A turn is complete when an `AssistantMessage` (Python) / `SDKAssistantMessage` (TS) arrives — that single message carries the model's full set of blocks for that turn (`text`, `thinking`, `tool_use`, …). The order of blocks inside `content` reflects the order Claude produced them.

The [streaming-output doc](https://code.claude.com/docs/en/agent-sdk/streaming-output) describes the flow with partial messages enabled:

```text
StreamEvent (message_start)
StreamEvent (content_block_start) - text block
StreamEvent (content_block_delta) - text chunks...
StreamEvent (content_block_stop)
StreamEvent (content_block_start) - tool_use block
StreamEvent (content_block_delta) - tool input chunks...
StreamEvent (content_block_stop)
StreamEvent (message_delta)
StreamEvent (message_stop)
AssistantMessage - complete message with all content   ← turn boundary
... tool executes ...
... more streaming events for next turn ...
ResultMessage - final result                            ← session boundary
```

So a consumer detects "turn end" by **the arrival of a fully-typed `AssistantMessage`**. If `include_partial_messages` is on, the underlying API also surfaces `message_stop` as the last `StreamEvent` before the typed `AssistantMessage` arrives — but the SDK contract is "wait for `AssistantMessage`."

Subsequent `UserMessage` events that follow an `AssistantMessage` are tool-result envelopes the SDK feeds back into the next turn — they are not user input. The `tool_use_result` field on `UserMessage` carries the tool's payload.

---

## 4. Session completion

The single terminal event is `ResultMessage` (Python) / `SDKResultMessage` (TS), `type: "result"`. Exactly one fires per `query()` call (or per `receive_response()` iteration in `ClaudeSDKClient`).

What it carries (TS success variant; Python fields are a superset of the same names):

| Field                  | Meaning                                                                            |
| ---------------------- | ---------------------------------------------------------------------------------- |
| `subtype`              | `"success"` or one of the four error subtypes                                      |
| `session_id`           | The session this result belongs to (matches all preceding events)                  |
| `duration_ms`          | Wall-clock duration of the whole query                                             |
| `duration_api_ms`      | Time spent in API calls (excludes tool execution)                                  |
| `is_error`             | Boolean summary flag                                                               |
| `api_error_status`     | HTTP status code if the API errored                                                |
| `num_turns`            | Number of model turns consumed                                                     |
| `result`               | Final assistant text (string). Present on `subtype:"success"`.                     |
| `stop_reason`          | Last turn's stop reason (`end_turn`, `tool_use`, `max_tokens`, …)                  |
| `ttft_ms`              | Time to first token (optional)                                                     |
| `total_cost_usd`       | Cumulative cost across all turns and subagents in this query                       |
| `usage`                | Token totals — `NonNullableUsage` (input/output/cache_read/cache_creation)         |
| `modelUsage`           | Per-model breakdown `{ [model]: { costUSD, inputTokens, outputTokens, cacheRead… } }` |
| `permission_denials`   | List of `SDKPermissionDenial` from tools blocked by canUseTool/permissions         |
| `structured_output`    | Parsed JSON if structured-output mode is in use                                    |
| `deferred_tool_use`    | `{ id, name, input }` if the result is a deferred tool handoff                     |
| `errors`               | Error message strings (error variants only — `string[]`)                           |
| `terminal_reason`      | Why the session terminated (CLI-internal taxonomy)                                 |

Per the [cost-tracking doc](https://code.claude.com/docs/en/agent-sdk/cost-tracking), per-message live cost is **not** surfaced — `total_cost_usd` and `usage` are observable only at `ResultMessage`. Consumers that want intra-session totals must call `query()` multiple times and accumulate manually.

---

## 5. Tool-use events

Tool use shows up at two levels of granularity.

### 5.1 Block level (inside `AssistantMessage` / `UserMessage`)

- Model decides to call a tool → `ToolUseBlock` inside the next `AssistantMessage.content`.
  - Fields: `id` (the `tool_use_id` referenced by the matching result), `name` (tool name, e.g. `"Bash"`, `"Read"`, `"Agent"`), `input` (dict of arguments).
- Tool finishes → `ToolResultBlock` inside the **following** `UserMessage.content`.
  - Fields: `tool_use_id` (back-pointer to the `ToolUseBlock.id`), `content` (string or list of content parts), `is_error` (bool flag).
- For server-side tools (`web_search`, `web_fetch`, `advisor`, code-execution, tool-search): same pattern but with `ServerToolUseBlock` / `ServerToolResultBlock`. The host does not need to return a result — the API has already executed it.

The SDK feeds tool results back as **synthetic user messages**. From the type definition: `SDKUserMessage` has `isSynthetic?: boolean` and `tool_use_result?: unknown`. A consumer that wants only "real" user input filters out `isSynthetic === true`.

### 5.2 Streaming level (`StreamEvent` / `SDKPartialAssistantMessage`)

When `include_partial_messages` is on, the tool call also streams as a sequence of raw API events (the `event` field carries the [Anthropic streaming event](https://platform.claude.com/docs/en/build-with-claude/streaming#event-types)):

| Stream event type                                                      | Carries                                                              |
| ---------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `content_block_start` with `content_block.type === "tool_use"`         | `id`, `name` of the tool                                             |
| `content_block_delta` with `delta.type === "input_json_delta"`         | `partial_json` — concatenate to reconstruct `input`                  |
| `content_block_stop`                                                   | Tool block is fully transmitted; the typed `AssistantMessage` follows |

The official example from the streaming-output doc:

```python
if event_type == "content_block_start":
    content_block = event.get("content_block", {})
    if content_block.get("type") == "tool_use":
        current_tool = content_block.get("name")
        tool_input = ""
elif event_type == "content_block_delta":
    delta = event.get("delta", {})
    if delta.get("type") == "input_json_delta":
        tool_input += delta.get("partial_json", "")
elif event_type == "content_block_stop":
    ...  # tool_input now holds the complete JSON string
```

### 5.3 Pre- vs. post-execution

The **stream itself only signals "the model wants to call this tool"** (via `ToolUseBlock`) and **"here's the result coming back into the model"** (via `ToolResultBlock` inside a synthetic `UserMessage`). The actual *tool execution* is observable only via:

- The **hooks** API — `PreToolUse` and `PostToolUse` (file 04, out of scope here).
- TS-only event types: `SDKToolProgressMessage`, `SDKToolUseSummaryMessage`, `SDKLocalCommandOutputMessage`, `SDKPermissionDeniedMessage` — these surface execution-time signals from the CLI side. They are **not** in the Python `Message` union, but they ride the same wire stream and are accessible via the lower-level transport.

For background-tasked subagents (`AgentDefinition.background: true`), execution surfaces as `TaskStartedMessage` / `TaskProgressMessage` / `TaskNotificationMessage`, with the `task_type` discriminator `"local_bash" | "local_agent" | "remote_agent"`.

---

## 6. Streaming chunks within a turn

### 6.1 Default behavior

By default `include_partial_messages` is `false`. In that mode the SDK does **not** yield deltas — you receive only complete `AssistantMessage` envelopes (plus `SystemMessage`, `ResultMessage`, etc.). The model still streams from the API, but the SDK buffers until each turn is whole.

### 6.2 Streaming enabled

Set `include_partial_messages=True` (Python) / `includePartialMessages: true` (TS). Then `StreamEvent` (Python) / `SDKPartialAssistantMessage` with `type: "stream_event"` (TS) interleave with the complete-message events. The `event` field carries the **raw Anthropic API stream event** (`BetaRawMessageStreamEvent`), unwrapped.

Recognized event-type strings inside `event.type`:

| `event.type`            | Meaning                                                       |
| ----------------------- | ------------------------------------------------------------- |
| `message_start`         | Beginning of a new assistant message                          |
| `content_block_start`   | Beginning of a content block (text / thinking / tool_use)    |
| `content_block_delta`   | Incremental content — see `delta.type` below                  |
| `content_block_stop`    | End of a content block                                        |
| `message_delta`         | Mid-message updates: `stop_reason`, `usage`                  |
| `message_stop`          | End of the message                                            |

`delta.type` discriminants:

| `delta.type`           | Payload field                              |
| ---------------------- | ------------------------------------------ |
| `text_delta`           | `delta.text` (string chunk)                |
| `input_json_delta`     | `delta.partial_json` (string chunk of JSON)|
| `thinking_delta`       | `delta.thinking` (string chunk)            |
| `signature_delta`      | `delta.signature`                          |

### 6.3 Recognizing start / mid / end of a chunk run

For text:
- Start: `content_block_start` with `content_block.type === "text"`.
- Mid: `content_block_delta` with `delta.type === "text_delta"`, repeated.
- End: `content_block_stop` for that index.

The minimal text-streaming consumer (from the docs):

```python
async for message in query(prompt="…",
                           options=ClaudeAgentOptions(include_partial_messages=True)):
    if isinstance(message, StreamEvent):
        event = message.event
        if event.get("type") == "content_block_delta":
            delta = event.get("delta", {})
            if delta.get("type") == "text_delta":
                print(delta.get("text", ""), end="", flush=True)
```

### 6.4 Documented limitations

From the streaming-output doc:

- **Extended thinking incompatibility:** when `max_thinking_tokens` / `maxThinkingTokens` is set explicitly, `StreamEvent` messages are **not** emitted. Only complete typed messages arrive.
- **Structured output:** the parsed JSON appears only at `ResultMessage.structured_output`, not as deltas.

---

## 7. Errors and aborts

The SDK splits failures into **exceptions** (transport / spawn-time) and **typed messages** (runtime / model-side).

### 7.1 Exceptions (Python, from `src/claude_agent_sdk/_errors.py`)

```python
class ClaudeSDKError(Exception):
    """Base exception for all Claude SDK errors."""

class CLIConnectionError(ClaudeSDKError):
    """Raised when unable to connect to Claude Code."""

class CLINotFoundError(CLIConnectionError):
    """Raised when Claude Code is not found or not installed."""
    # __init__(self, message="...", cli_path: str | None = None)

class ProcessError(ClaudeSDKError):
    """Raised when the CLI process fails."""
    # attrs: exit_code: int | None, stderr: str | None

class CLIJSONDecodeError(ClaudeSDKError):
    """Raised when unable to decode JSON from CLI output."""
    # attrs: line: str, original_error: Exception

class MessageParseError(ClaudeSDKError):
    """Raised when unable to parse a message from CLI output."""
    # attrs: data: dict | None
```

These are thrown synchronously from the async iterator — caller wraps the `async for` in `try / except`:

```python
try:
    async for message in query(prompt="..."):
        ...
except CLINotFoundError:
    ...
except ProcessError as e:
    print(e.exit_code, e.stderr)
except CLIJSONDecodeError as e:
    print(e.line)
```

### 7.2 Typed in-stream errors

Mid-session model/API errors are **delivered as events**, not thrown:

- `AssistantMessage.error` carries an `AssistantMessageError` literal: `"authentication_failed" | "billing_error" | "rate_limit" | "invalid_request" | "server_error" | "max_output_tokens" | "unknown"`. (Note: [issue #505](https://github.com/anthropics/claude-agent-sdk-python/issues/505) and [#472](https://github.com/anthropics/claude-agent-sdk-python/issues/472) report this field is not always populated by the CLI as of recent releases — the error is sometimes surfaced as an assistant text block instead. Treat populated `error` as informative but not authoritative.)
- Permission denials accumulate in `ResultMessage.permission_denials: list[SDKPermissionDenial]`.
- Catastrophic outcomes set `ResultMessage.subtype` to one of `"error_max_turns" | "error_during_execution" | "error_max_budget_usd" | "error_max_structured_output_retries"` and populate `errors: list[str]`. `is_error` is `True`. The session still terminates "cleanly" with a single `ResultMessage`.
- The TS-only `SDKAPIRetryMessage` and `SDKMirrorErrorMessage` events surface lower-level transport hiccups.

### 7.3 Aborts / interrupts

- **TS:** `query()` returns a `Query` object that extends `AsyncGenerator` and exposes `interrupt(): Promise<void>`. There is also an external `AbortController` (`options.abortController`) — the QueryEngine wires `abortController.signal` down to every API call, every tool process, and every child subagent. Calling `interrupt()` (or `controller.abort()`) terminates the active turn; the session emits `ResultMessage` with `subtype: "error_during_execution"`.
- **Python:** `ClaudeSDKClient.interrupt()` is the equivalent. `query()` (the function form, no client) does not expose interrupt — cancellation is via `asyncio.Task.cancel()` on the outer task.

Known quirks (current as of 2026-05):
- [claude-agent-sdk-typescript#69](https://github.com/anthropics/claude-agent-sdk-typescript/issues/69): aborting right after `init` can corrupt the resume cursor.
- [claude-agent-sdk-typescript#120](https://github.com/anthropics/claude-agent-sdk-typescript/issues/120): no "soft interrupt that keeps the session open" exists in the current API.
- After `abort()`, the SDK closes stdin and waits ~2s for clean CLI shutdown before propagating the signal to spawn callbacks.

---

## 8. `parent_tool_use_id` and subagent attribution

A subagent runs in its own conversation context, but its events still flow through the **same** message stream as the parent. The discriminator is `parent_tool_use_id`.

### 8.1 The field

`parent_tool_use_id: string | null` is present on:

| Type                         | Notes                                                      |
| ---------------------------- | ---------------------------------------------------------- |
| `SDKAssistantMessage` (TS) / `AssistantMessage` (Python) | Set when the assistant turn is inside a subagent |
| `SDKUserMessage` / `UserMessage`                       | Set when the user turn (typically a synthetic tool result) is inside a subagent |
| `SDKPartialAssistantMessage` / `StreamEvent`           | Set when the streaming chunk is inside a subagent |
| `SDKUserMessageReplay`                                 | Same                                                       |

Subagent docs spell out the contract:

> For subagent messages, the `tool_use_id` of the spawning `Agent` tool call. `null` for main-session messages and older sessions.

### 8.2 Detecting subagent entry / exit

Subagent invocation is the `Agent` tool (renamed from `Task` at Claude Code v2.1.63; current SDK emits `"Agent"` in `tool_use` blocks but still uses `"Task"` in the `system:init` `tools` list and in `permission_denials[].tool_name`).

A consumer:
1. Watches for `ToolUseBlock` (or content-block-start) with `name in ("Agent", "Task")` — capture `id`. That's the subagent's *birth*.
2. For every subsequent message, if `parent_tool_use_id == <that id>`, it's *inside* the subagent.
3. The subagent's final output arrives as a `ToolResultBlock` whose `tool_use_id == <that id>` — this is what the parent model sees.

Verbatim from the subagent doc:

> Messages from within a subagent's context include a `parent_tool_use_id` field.
>
> The parent receives the subagent's final message verbatim as the Agent tool result, but may summarize it in its own response.

### 8.3 `forwardSubagentText`

Setting `forward_subagent_text: true` (Python) / `forwardSubagentText: true` (TS) makes the SDK forward the subagent's full transcript to the parent stream, so a consumer can render the nested conversation rather than only the `ToolResultBlock` final. Without it, only the final assistant message returns to the parent stream (and the subagent's intermediate `AssistantMessage` / `UserMessage` events are suppressed from the parent stream).

### 8.4 Caveats

- [claude-agent-sdk-python#272](https://github.com/anthropics/claude-agent-sdk-python/issues/272) tracks the gap that **hooks** do not get `parent_tool_use_id` context — observability in hook callbacks cannot currently attribute a `PreToolUse` to its enclosing subagent. Stream-level attribution works; hook-level does not (file 04 territory).
- Subagents **cannot spawn subagents** (the doc explicitly says: "Subagents cannot spawn their own subagents. Don't include `Agent` in a subagent's `tools` array."). So `parent_tool_use_id` is at most one level deep in current SDKs.

---

## 9. Comparison: SDK typed stream vs. Claude Code `--output-format stream-json`

The two streams **share the same on-the-wire format.** The SDK is a thin typed wrapper over the exact same JSONL the CLI emits with `claude -p --output-format stream-json`. Each line is one JSON object with a `type` discriminator (and, for system/result, a `subtype` discriminator).

### 9.1 Examples (verbatim shape)

CLI `--output-format stream-json` event shapes from public references:

```jsonc
// system init
{"type":"system","subtype":"init","session_id":"session_01","cwd":"/repo",
 "model":"sonnet","permissionMode":"auto","apiKeySource":"env",
 "tools":["Bash","Read","Write","WebSearch"],
 "mcp_servers":[{"name":"approvals","status":"connected"}]}

// assistant turn (whole)
{"type":"assistant","session_id":"session_01",
 "message":{"id":"msg_1","role":"assistant",
            "content":[{"type":"text","text":"Planning next steps."}], ...}}

// stream event (when --include-partial-messages)
{"type":"stream_event","session_id":"session_01",
 "event":{"type":"content_block_delta","index":0,
          "delta":{"type":"text_delta","text":"Hello"}}}

// result success
{"type":"result","subtype":"success","session_id":"session_01",
 "total_cost_usd":0.0123,"num_turns":2,"duration_ms":4500,
 "is_error":false, "usage":{...}, ...}
```

These line-shapes are **bit-identical** to the TS `SDKMessage` types. The TS SDK literally parses these lines into typed objects with `type` / `subtype` as discriminators.

### 9.2 Asymmetries

| Dimension                                         | SDK typed stream                                                                                       | CLI `stream-json`                                                                                 |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| Wire format                                       | (internal) — same JSONL, in-process                                                                    | JSONL on stdout                                                                                  |
| TS typing                                         | Full `SDKMessage` union covers every variant                                                            | Documentation is sparse ([claude-code#24612](https://github.com/anthropics/claude-code/issues/24612) is open to document them) |
| Python typing                                     | Narrower `Message` union — Python only types core variants (omits hook-progress, status, retry, etc.) | Same JSONL on the wire — Python users who want the un-typed events must read raw transport       |
| Discriminator                                     | `type` literal field; class identity in Python                                                          | `type` field, `subtype` for `system`/`result`                                                    |
| Partial messages                                  | Opt-in via `include_partial_messages` / `includePartialMessages`                                       | Opt-in via `--include-partial-messages`                                                          |
| Server-side / API-internal events                 | `ServerToolUseBlock` / `ServerToolResultBlock` typed                                                   | Same JSON shape — `{"type":"server_tool_use", ...}` blocks inside `message.content`              |
| Final result                                      | `ResultMessage` / `SDKResultMessage`                                                                   | `{"type":"result", ...}` final line                                                              |
| Errors during execution                           | Stored as `subtype:"error_during_execution"` in result + thrown exceptions for transport failures      | Same `subtype:"error_during_execution"` in result; transport failures are exit codes + stderr    |
| Rate-limit events                                 | `RateLimitEvent` / `SDKRateLimitEvent` (added v2.1.45)                                                  | Same line, `{"type":"rate_limit_event", ...}` — undocumented as of [claude-code#26392](https://github.com/anthropics/claude-code/issues/26392) |

### 9.3 Implication for Ark

The Ark project's PRD references hand-parsing of stream-json in `crates/ark-core/src/commands/agent/task/run/platform/claude.rs` (a path that does not yet exist on disk — the work is forward-looking). Two facts inform that work:

1. **The shape is stable.** A Rust parser that targets the `type` / `subtype` discriminator over JSONL handles both the CLI's stream-json output and (if Ark ever embedded the SDK directly) the underlying SDK event stream. There is no second wire format.
2. **The TS SDK union is the most complete reference.** When the CLI documentation is silent (it is, for many event types), the TS `SDKMessage` definitions are the canonical schema — every event the CLI emits is typed there because the TS SDK consumes the same lines.

Concrete fields that will matter to a Rust consumer:

- Discriminate on `type` first. For `"system"` and `"result"`, then discriminate on `subtype`.
- `parent_tool_use_id` (string | null) on `"assistant"`, `"user"`, `"stream_event"` — the only signal that an event belongs to a subagent vs. main.
- `session_id` on every event — primary key for multi-session multiplexing.
- `uuid` on every event — unique event identity, useful for de-dup if a stream resumes.
- For cost/turn accounting, only `{type:"result"}` events carry totals (`total_cost_usd`, `num_turns`, `duration_ms`, `usage`, `modelUsage`).

---

## External references

- [Agent SDK reference — Python (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/python) — primary Python reference; `Message` union, `ClaudeSDKClient`, `AgentDefinition`, `AssistantMessageError`, task-message types.
- [Agent SDK reference — TypeScript (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/typescript) — primary TS reference; full `SDKMessage` union, all result subtypes, partial-message types.
- [Stream responses in real-time (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/streaming-output) — `include_partial_messages`, message flow diagram, raw event-type table, text/tool-use streaming snippets, known limitations (extended thinking, structured output).
- [Streaming Input — Streaming vs. Single-mode (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode) — `query()` vs. `ClaudeSDKClient`, abort/interrupt support matrix.
- [Subagents in the SDK (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/subagents) — `parent_tool_use_id`, `forwardSubagentText`, `Agent` vs `Task` tool naming, recursion prohibition.
- [Track cost and usage (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/cost-tracking) — fields in `ResultMessage` for cost/usage; intra-session accumulation pattern.
- [`types.py` on main (anthropics/claude-agent-sdk-python)](https://raw.githubusercontent.com/anthropics/claude-agent-sdk-python/refs/heads/main/src/claude_agent_sdk/types.py) — canonical Python dataclasses (visited 2026-05-25).
- [`_errors.py` on main (anthropics/claude-agent-sdk-python)](https://raw.githubusercontent.com/anthropics/claude-agent-sdk-python/refs/heads/main/src/claude_agent_sdk/_errors.py) — exception hierarchy.
- [claude-code#24612](https://github.com/anthropics/claude-code/issues/24612) — open: full `stream-json` shape not yet documented.
- [claude-code#26392](https://github.com/anthropics/claude-code/issues/26392) — `SDKRateLimitInfo` / `SDKRateLimitEvent` are typed but undocumented.
- [claude-agent-sdk-python#505](https://github.com/anthropics/claude-agent-sdk-python/issues/505) — `AssistantMessage.error` not reliably populated.
- [claude-agent-sdk-python#472](https://github.com/anthropics/claude-agent-sdk-python/issues/472) — API errors arrive as assistant text rather than as exceptions.
- [claude-agent-sdk-python#272](https://github.com/anthropics/claude-agent-sdk-python/issues/272) — hooks lack `parent_tool_use_id` context.
- [claude-agent-sdk-typescript#69](https://github.com/anthropics/claude-agent-sdk-typescript/issues/69) — early-abort breaks resume.
- [claude-agent-sdk-typescript#120](https://github.com/anthropics/claude-agent-sdk-typescript/issues/120) — no in-session "soft interrupt."
- [Claude Code stream-json cheatsheet (takopi.dev)](https://takopi.dev/reference/runners/claude/stream-json-cheatsheet/) — third-party reference enumerating observed CLI event shapes; cited only for examples (cross-checked against SDK type definitions).
- [Anthropic streaming event types (platform.claude.com)](https://platform.claude.com/docs/en/build-with-claude/streaming) — canonical reference for raw `event.type` / `delta.type` enums inside `StreamEvent.event`.

---

## Caveats / Not found

- **`SystemMessage.subtype` values (Python):** the type is `str`; the documented subtype is `"init"` and `"compact_boundary"`, but the Python facade does not export a `Literal` enumerating them. Other CLI-emitted subtypes (status, hook lifecycle, plugin install, etc.) are visible only through the TS `SDKMessage` union or by reading the raw transport in Python.
- **`stop_reason` enum values (`AssistantMessage`, `ResultMessage`):** field is typed `str | None`; the SDK does not export an enum. Observed values in practice match the Anthropic Messages API: `end_turn`, `tool_use`, `max_tokens`, `stop_sequence`, `pause_turn`, `refusal`. The SDK reference does not document an authoritative list.
- **`RateLimitInfo` field schema:** referenced in `RateLimitEvent.rate_limit_info` but `SDKRateLimitInfo` is not exported in the public type surface ([claude-code#26392](https://github.com/anthropics/claude-code/issues/26392)). Inferred per-bucket fields (`utilization`, `resetsAt`) for buckets `five_hour | seven_day | seven_day_opus | seven_day_sonnet` come from issue discussion, not official docs.
- **`SDKStatusMessage`, `SDKLocalCommandOutputMessage`, `SDKHookStartedMessage`, `SDKHookProgressMessage`, `SDKHookResponseMessage`, `SDKPluginInstallMessage`, `SDKToolProgressMessage`, `SDKAuthStatusMessage`, `SDKSessionStateChangedMessage`, `SDKNotificationMessage`, `SDKFilesPersistedEvent`, `SDKToolUseSummaryMessage`, `SDKMemoryRecallMessage`, `SDKElicitationCompleteMessage`, `SDKPermissionDeniedMessage`, `SDKPromptSuggestionMessage`, `SDKAPIRetryMessage`, `SDKMirrorErrorMessage`:** present in the TS union but their full field shapes are not surfaced on the public TS reference page. They are visible in the TS SDK's `.d.ts` (under `sdk.d.ts` / generated types) but were not extractable directly via WebFetch on the GitHub blob (returned 404 — file path may have moved between releases).
- **Whether Python's `query()` ever yields events that have no Python dataclass:** the Python facade silently drops or stuffs into `SystemMessage.data` any wire event without a typed counterpart. This is not contractually documented; deduced from the asymmetry between the two unions.
- **Backpressure / flow-control between consumer and stream:** not documented. The Python `AsyncIterator` is consumer-paced; if the consumer is slow, the transport buffers. There is no documented "drop oldest" or watermark policy.
- **Order guarantee around subagent events when `forwardSubagentText` is on:** the docs say events from a subagent carry `parent_tool_use_id` but do not specify whether parent/child events are strictly interleaved by time or grouped per-subagent. Inferred to be interleaved-by-time but unverified.
- **Whether `ResultMessage.result` is populated for the error subtypes:** the TS type definition explicitly omits `result` from the error variants and only includes `errors: string[]`. The Python `ResultMessage` carries both `result` and `errors` as optional, so on an error subtype `result` may be `None` — verified by type, not by explicit doc statement.

