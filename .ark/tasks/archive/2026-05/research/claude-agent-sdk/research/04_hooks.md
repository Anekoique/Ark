# Research: Claude Agent SDK hook system

- Query: enumerate every hook event the SDK supports; exact registration API (`HookMatcher` / `HookCallback` / `ClaudeAgentOptions.hooks`); the `PreToolUse` allow/deny/mutate return convention; what `PostToolUse` sees and can return; the per-callback input payload (incl. `parent_tool_use_id` / subagent-attribution status, cross-ref issue #272 from topic 03); `Stop` / `SessionEnd` capabilities; how the separate `CanUseTool` permission callback differs from a `PreToolUse` hook and their ordering; async hooks and I/O; failure behavior.
- Scope: external (primary: docs.claude.com / code.claude.com hooks + permissions + user-input pages; SDK source `claude-agent-sdk-python/src/claude_agent_sdk/types.py` on `main`).
- Date: 2026-05-25
- SDK versions referenced (pinned per topic 01):
  - Python `claude-agent-sdk` **0.2.87** (PyPI, released 2026-05-23).
  - TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.150** (npm, current as of 2026-05-23). (Topic 03 cited 0.3.142 for its event-union extract; 0.3.150 is the head referenced by topic 01. No hook-API change is documented between these.)
  - Doc snapshot: `code.claude.com` hooks / permissions / user-input pages fetched 2026-05-25.
- Distinction from topic 03: topic 03 covers the **event STREAM** — `Message` / `SDKMessage` envelopes the caller *observes* by iterating `query()`. This file covers **HOOKS** — callbacks the SDK *invokes synchronously inside the agent loop* that can **deny, mutate, or inject** before/after a tool runs or at lifecycle points. Events are observe-only and pull-based (the consumer iterates). Hooks are intervene-capable and push-based (the SDK calls you and waits for a decision). The two are wired separately: events arrive in the `async for`; hooks arrive as calls to functions registered in `options.hooks`.

---

## 0. Why this matters for the reader (ArkOS stage-1)

A workflow substrate needs three distinct intervention surfaces, and the SDK splits them across two mechanisms that are easy to conflate:

| Substrate need | Right surface | Why |
| -------------- | ------------- | --- |
| Out-of-scope-write guard (block writes outside the task dir) | **`PreToolUse` hook** returning `permissionDecision: "deny"` | Runs first in the permission chain, deterministic (your regex/path logic), no user prompt, applies to subagents too. `canUseTool` is the wrong layer — it is the *last* step and is skipped entirely in `dontAsk` mode. |
| Budget cap / hard abort | **`PreToolUse` hook** (deny when over budget) or top-level `continue_: false` / `PermissionResultDeny(interrupt=True)` | Hooks see every tool call before it spends; denying or interrupting is the deterministic gate. Note: live per-call cost is **not** in the hook payload (topic 03 §4: cost only at `ResultMessage`), so the substrate must track spend itself and consult its own tally inside the hook. |
| Grounding-signal feedback (e.g. "tests still failing", "spec says X") | **`PostToolUse` hook** returning `hookSpecificOutput.additionalContext` (append) or `updatedToolOutput` (replace) | `PostToolUse` is the only hook that sees the tool *result* and can splice text the model reads on its next turn, without faking a user message. |
| Interactive human approval | **`canUseTool`** callback | Purpose-built for "pause and ask a human"; can stay pending indefinitely. Not for deterministic policy. |

Keep the split clear: **`PreToolUse` is the policy/automation gate; `canUseTool` is the human-in-the-loop gate.** Both can coexist; the hook runs first (§7).

---

## 1. Hook taxonomy

### 1.1 The full set (verbatim event names)

The hooks reference page publishes a Python-vs-TypeScript availability matrix. Reproduced verbatim (column "What triggers it" / "Example use case" condensed):

| Hook Event           | Python | TypeScript | Fires when… | Can do |
| -------------------- | ------ | ---------- | ----------- | ------ |
| `PreToolUse`         | Yes    | Yes        | A tool call is requested, before execution | **Block or modify** the call (allow / deny / ask / defer, `updatedInput`) |
| `PostToolUse`        | Yes    | Yes        | A tool returned a result | Append/replace result context (`additionalContext`, `updatedToolOutput`); cannot un-run the tool |
| `PostToolUseFailure` | Yes    | Yes        | A tool execution **failed** | Observe/handle the error (`error`, `is_interrupt`) |
| `PostToolBatch`      | **No** | Yes        | A full batch of tool calls resolves, once per batch before the next model call | Inject conventions once for the whole batch (TS-only) |
| `UserPromptSubmit`   | Yes    | Yes        | The user prompt is submitted | Inject context into the prompt; can block |
| `Stop`               | Yes    | Yes        | The agent is about to stop (end of turn / idle) | Save state; **can block the stop** (`decision: "block"`) — see §6 |
| `SubagentStart`      | Yes    | Yes        | A subagent initializes | Observe spawn (`agent_id`, `agent_type`) |
| `SubagentStop`       | Yes    | Yes        | A subagent completes | Observe completion / aggregate (`agent_id`, `agent_transcript_path`) |
| `PreCompact`         | Yes    | Yes        | A conversation compaction is requested | Archive transcript before summarizing (`trigger`, `custom_instructions`) |
| `PermissionRequest`  | Yes    | Yes        | A permission dialog *would* be shown | Custom permission handling / external notify |
| `Notification`       | Yes    | Yes        | An agent status message fires | Forward to Slack/PagerDuty; cannot modify behavior |
| `SessionStart`       | **No** | Yes        | Session initialization | Init logging/telemetry |
| `SessionEnd`         | **No** | Yes        | Session termination | Clean up resources |
| `Setup`              | **No** | Yes        | Session setup/maintenance | Run init tasks (TS-only) |
| `TeammateIdle`       | **No** | Yes        | A teammate becomes idle | Reassign / notify (TS-only) |
| `TaskCompleted`      | **No** | Yes        | A background task completes | Aggregate parallel-task results (TS-only) |
| `ConfigChange`       | **No** | Yes        | A config file changes | Reload settings dynamically (TS-only) |
| `WorktreeCreate`     | **No** | Yes        | A git worktree is created | Track isolated workspaces (TS-only) |
| `WorktreeRemove`     | **No** | Yes        | A git worktree is removed | Clean up workspace resources (TS-only) |

Source: [Intercept and control agent behavior with hooks — "Available hooks"](https://code.claude.com/docs/en/agent-sdk/hooks#available-hooks).

### 1.2 The canonical Python `HookEvent` literal (verbatim from `types.py`)

The Python SDK's accepted event keys are exactly this `Literal` — anything not in it is rejected at registration:

```python
HookEvent = (
    Literal["PreToolUse"]
    | Literal["PostToolUse"]
    | Literal["PostToolUseFailure"]
    | Literal["UserPromptSubmit"]
    | Literal["Stop"]
    | Literal["SubagentStop"]
    | Literal["PreCompact"]
    | Literal["Notification"]
    | Literal["SubagentStart"]
    | Literal["PermissionRequest"]
)
```

Source: [`types.py` on `main`](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/types.py).

**Cross-checking the prompt's expected set:**

- `PreToolUse`, `PostToolUse`, `Stop`, `SubagentStop`, `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `Notification`, `PreCompact` — **all exist** as named events. `SubagentStop` exists (Python + TS).
- **`SessionStart` / `SessionEnd` are NOT registrable as Python SDK callback hooks.** They are omitted from the Python `HookEvent` literal above. The docs are explicit:
  > "`SessionStart` and `SessionEnd` can be registered as SDK callback hooks in TypeScript, but are not available in the Python SDK (`HookEvent` omits them). In Python, they are only available as [shell command hooks](https://code.claude.com/docs/en/hooks#hook-events) defined in settings files (for example, `.claude/settings.json`)."
  To use them from Python, load them from settings via `setting_sources=["project"]`, or use the first message of `client.receive_response()` as the start trigger. (Source: hooks page, "Session hooks not available in Python".)
- Beyond the prompt's list, the SDK adds: `PostToolUseFailure` (both), `SubagentStart` (both), `PermissionRequest` (both), and a long TS-only tail (`PostToolBatch`, `Setup`, `TeammateIdle`, `TaskCompleted`, `ConfigChange`, `WorktreeCreate`, `WorktreeRemove`).

**Asymmetry to flag:** the docs matrix lists `PostToolUseFailure` as **Python = Yes**, and the Python `HookEvent` literal includes `"PostToolUseFailure"` — consistent. But the matrix lists `SessionStart`/`SessionEnd` as **Python = No**, and the literal omits them — also consistent. Trust the `types.py` literal as authoritative for Python registrability.

---

## 2. Registration

### 2.1 The wiring types (Python, verbatim from `types.py`)

```python
@dataclass
class HookMatcher:
    matcher: str | None = None              # regex tested against the event's filter field (tool name for tool hooks)
    hooks: list[HookCallback] = field(default_factory=list)
    timeout: float | None = None            # per-matcher timeout, seconds (default 60 per docs)

HookCallback = Callable[
    [HookInput, str | None, HookContext],   # (input_data, tool_use_id, context)
    Awaitable[HookJSONOutput],              # MUST be awaitable — async only (see §8)
]

class HookContext(TypedDict):
    signal: Any | None                      # TS: AbortSignal; Python: "reserved for future use"
```

`HookCallback` is a three-argument coroutine: `(input_data, tool_use_id, context)`. The middle arg `tool_use_id: str | None` correlates `PreToolUse` ↔ `PostToolUse` for the same tool call.

### 2.2 Attaching to `ClaudeAgentOptions` (Python, verbatim)

```python
# field on ClaudeAgentOptions:
hooks: dict[HookEvent, list[HookMatcher]] | None = None
```

So the shape is: event name → list of `HookMatcher`, each carrying a `matcher` regex and a list of callbacks. (TS mirror: `hooks?: Partial<Record<HookEvent, HookCallbackMatcher[]>>` on `Options`, per the [TypeScript reference](https://code.claude.com/docs/en/agent-sdk/typescript).)

### 2.3 Minimal `PreToolUse` registration — Python

From the hooks page (verbatim), a deny-on-`.env`-write guard registered with a `"Write|Edit"` matcher:

```python
import asyncio
from claude_agent_sdk import (
    AssistantMessage, ClaudeSDKClient, ClaudeAgentOptions, HookMatcher, ResultMessage,
)

async def protect_env_files(input_data, tool_use_id, context):
    file_path = input_data["tool_input"].get("file_path", "")
    file_name = file_path.split("/")[-1]
    if file_name == ".env":
        return {
            "hookSpecificOutput": {
                "hookEventName": input_data["hook_event_name"],
                "permissionDecision": "deny",
                "permissionDecisionReason": "Cannot modify .env files",
            }
        }
    return {}   # empty object → allow unchanged

async def main():
    options = ClaudeAgentOptions(
        hooks={"PreToolUse": [HookMatcher(matcher="Write|Edit", hooks=[protect_env_files])]}
    )
    async with ClaudeSDKClient(options=options) as client:
        await client.query("Update the database configuration")
        async for message in client.receive_response():
            if isinstance(message, (AssistantMessage, ResultMessage)):
                print(message)

asyncio.run(main())
```

### 2.4 TypeScript equivalent (verbatim)

```typescript
import { query, HookCallback, PreToolUseHookInput } from "@anthropic-ai/claude-agent-sdk";

const protectEnvFiles: HookCallback = async (input, toolUseID, { signal }) => {
  const preInput = input as PreToolUseHookInput;
  const toolInput = preInput.tool_input as Record<string, unknown>;
  const filePath = toolInput?.file_path as string;
  const fileName = filePath?.split("/").pop();
  if (fileName === ".env") {
    return {
      hookSpecificOutput: {
        hookEventName: preInput.hook_event_name,
        permissionDecision: "deny",
        permissionDecisionReason: "Cannot modify .env files",
      },
    };
  }
  return {};
};

for await (const message of query({
  prompt: "Update the database configuration",
  options: {
    hooks: { PreToolUse: [{ matcher: "Write|Edit", hooks: [protectEnvFiles] }] },
  },
})) {
  if (message.type === "assistant" || message.type === "result") console.log(message);
}
```

TS divergences from Python:
- TS matcher entries are plain objects `{ matcher, hooks, timeout? }` (the `HookCallbackMatcher` type) — no `HookMatcher` class. Python uses the `HookMatcher` dataclass.
- TS output keys are camelCase throughout. Python output keys are **mixed**: the `hookSpecificOutput` payload uses camelCase (`hookEventName`, `permissionDecision`, `updatedInput`), but the top-level keys use Python-keyword-safe names (`continue_`, `async_`) — see §3 and §8.
- TS `context` carries a live `signal: AbortSignal`; Python's `HookContext.signal` is "reserved for future use" (no working cancellation signal in Python today).

### 2.5 Matcher semantics

- `matcher` is a **regex string**, tested against the event's filter field. For tool hooks that field is the **tool name** (`"Bash"`, `"Write"`, `"Edit"`, `"Glob"`, `"Grep"`, `"WebFetch"`, `"Agent"`, …). MCP tools match `mcp__<server>__<action>` (so `matcher="^mcp__"` catches all MCP tools).
- A `HookMatcher` with `matcher=None` (Python) / omitted (TS) runs for **every** event of that type.
- **Matchers filter by tool name only, never by arguments.** To gate on a file path, inspect `tool_input["file_path"]` inside the callback. (Hooks page, "Matcher not filtering as expected".)
- Multiple matchers under one event → **all matching hooks run in parallel**; "For permission decisions, the most restrictive result wins: a single `deny` blocks the tool call regardless of what the other hooks return." Completion order is non-deterministic; write each hook to be independent.

---

## 3. `PreToolUse` return convention (allow / deny / mutate)

`PreToolUse` is the one hook that can change whether and how a tool runs. The decision lives in `hookSpecificOutput`, a `PreToolUseHookSpecificOutput` (verbatim from `types.py`):

```python
class PreToolUseHookSpecificOutput(TypedDict):
    hookEventName: Literal["PreToolUse"]
    permissionDecision: NotRequired[Literal["allow", "deny", "ask", "defer"]]
    permissionDecisionReason: NotRequired[str]
    updatedInput: NotRequired[dict[str, Any]]
    additionalContext: NotRequired[str]
```

The four `permissionDecision` values:

| Value     | Effect |
| --------- | ------ |
| `"allow"` | Auto-approve this tool call (skips the user prompt / `canUseTool`). Required when you supply `updatedInput`. |
| `"deny"`  | Block the call. `permissionDecisionReason` is fed to the model so it doesn't retry blindly. |
| `"ask"`   | Force a permission prompt (falls through to `canUseTool` / the user). |
| `"defer"` | **Ends the query** so it can be resumed later from the persisted session (used to release the process while a human decides). `updatedInput` is ignored under `defer`. |

Precedence when several hooks/rules apply (docs `<Note>`): **deny > defer > ask > allow.** Any `deny` wins.

### (a) Allow

```python
return {}   # empty object → allow with no change
# or explicitly:
return {"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow"}}
```

### (b) Deny with a reason (+ optional user-facing message)

Verbatim deny-`/etc`-writes example from the hooks page:

```python
async def block_etc_writes(input_data, tool_use_id, context):
    file_path = input_data["tool_input"].get("file_path", "")
    if file_path.startswith("/etc"):
        return {
            "systemMessage": "Remember: system directories like /etc are protected.",  # top-level → shown to USER
            "hookSpecificOutput": {
                "hookEventName": input_data["hook_event_name"],
                "permissionDecision": "deny",
                "permissionDecisionReason": "Writing to /etc is not allowed",        # → shown to MODEL
            },
        }
    return {}
```

`permissionDecisionReason` goes to the **model**; `systemMessage` goes to the **user** (and by default is *not* surfaced in the stream — see §3.1).

### (c) Mutate the tool input before execution

Yes — `PreToolUse` **can rewrite the args**, via `updatedInput`, but only when paired with `permissionDecision: "allow"` (or `"ask"`). Verbatim sandbox-redirect example:

```python
async def redirect_to_sandbox(input_data, tool_use_id, context):
    if input_data["hook_event_name"] != "PreToolUse":
        return {}
    if input_data["tool_name"] == "Write":
        original_path = input_data["tool_input"].get("file_path", "")
        return {
            "hookSpecificOutput": {
                "hookEventName": input_data["hook_event_name"],
                "permissionDecision": "allow",
                "updatedInput": {
                    **input_data["tool_input"],
                    "file_path": f"/sandbox{original_path}",
                },
            }
        }
    return {}
```

Documented constraints on `updatedInput` (hooks page `<Note>` blocks):
- Must be inside `hookSpecificOutput`, not at the top level.
- Requires `permissionDecision: "allow"` (auto-approve the rewrite) or `"ask"` (show it). With `"defer"`, `updatedInput` is ignored.
- "Always return a **new object** rather than mutating the original `tool_input`." (Aligns with Ark's immutability rule.)

So `PreToolUse` is strictly more powerful than allow/deny: it is allow / deny / ask / defer **plus arbitrary arg rewriting**.

### 3.1 `PreToolUse` can also inject context

`PreToolUseHookSpecificOutput` includes `additionalContext: NotRequired[str]` — text appended for the model alongside the decision (same field name as `PostToolUse`, see §4). This is distinct from `permissionDecisionReason` (which is tied to a denial).

---

## 4. `PostToolUse` — what it sees and can return

### 4.1 Input (verbatim from `types.py`)

```python
class PostToolUseHookInput(BaseHookInput, _SubagentContextMixin):
    hook_event_name: Literal["PostToolUse"]
    tool_name: str
    tool_input: dict[str, Any]
    tool_response: Any          # ← the tool RESULT (string or structured payload)
    tool_use_id: str
```

So `PostToolUse` sees the tool name, the (possibly already-mutated) input, **and the result** in `tool_response`. (`_SubagentContextMixin` adds `agent_id`/`agent_type` — see §5.)

There is also a dedicated failure variant:

```python
class PostToolUseFailureHookInput(BaseHookInput, _SubagentContextMixin):
    hook_event_name: Literal["PostToolUseFailure"]
    tool_name: str
    tool_input: dict[str, Any]
    tool_use_id: str
    error: str
    is_interrupt: NotRequired[bool]
```

### 4.2 Output (verbatim from `types.py`)

```python
class PostToolUseHookSpecificOutput(TypedDict):
    hookEventName: Literal["PostToolUse"]
    additionalContext: NotRequired[str]        # APPEND text to the result the model reads
    updatedToolOutput: NotRequired[Any]        # REPLACE the tool's output before Claude sees it
    updatedMCPToolOutput: NotRequired[Any]     # same, for MCP tool outputs
```

What `PostToolUse` can and cannot do:

- **Can it alter the result?** Yes — `updatedToolOutput` (and `updatedMCPToolOutput` for MCP tools) **replaces** the tool's output entirely before the model sees it. `additionalContext` **appends** without replacing. (Hooks page, "Outputs".)
- **Can it inject feedback to the model?** Yes — that is exactly what `additionalContext` does (this is the grounding-signal surface from §0).
- **Can it un-run / block the tool?** No. The tool has already executed by the time `PostToolUse` fires; there is no `permissionDecision` field in `PostToolUseHookSpecificOutput`. To stop the *next* model turn from happening, use the top-level `continue_: false` (see §6 — this halts the loop, it does not roll back the executed tool).

### 4.3 Minimal `PostToolUse` observe example (verbatim, webhook-on-completion)

```python
import asyncio, json, urllib.request
from datetime import datetime

def _send_webhook(tool_name):
    data = json.dumps({"tool": tool_name, "timestamp": datetime.now().isoformat()}).encode()
    req = urllib.request.Request("https://api.example.com/webhook", data=data,
                                 headers={"Content-Type": "application/json"}, method="POST")
    urllib.request.urlopen(req)

async def webhook_notifier(input_data, tool_use_id, context):
    if input_data["hook_event_name"] != "PostToolUse":
        return {}
    try:
        await asyncio.to_thread(_send_webhook, input_data["tool_name"])  # blocking I/O off the event loop
    except Exception as e:
        print(f"Webhook request failed: {e}")   # swallow — a failed webhook must not stop the agent
    return {}
```

Note the pattern the docs insist on: **catch errors inside the hook** ("an unhandled exception can interrupt the agent" — see §9), and push blocking I/O onto a thread so it doesn't stall the loop.

---

## 5. Hook input payload (what every callback receives)

### 5.1 Base fields (verbatim from `types.py`)

```python
class BaseHookInput(TypedDict):
    session_id: str
    transcript_path: str
    cwd: str
    permission_mode: NotRequired[str]
```

Every hook input, regardless of event, carries `session_id`, `transcript_path`, `cwd`, and (optionally) `permission_mode`, plus the discriminating `hook_event_name`. (`hook_event_name` is declared on each concrete subclass as a `Literal`, e.g. `Literal["PreToolUse"]`, so it is present on all of them.)

The docs summarize the shared set:
> "All hook inputs share `session_id`, `cwd`, and `hook_event_name`."

Tool hooks add `tool_name` and `tool_input` (and `tool_use_id`); the second callback argument also carries the `tool_use_id` independently.

### 5.2 Per-event input shapes (verbatim from `types.py`)

```python
class PreToolUseHookInput(BaseHookInput, _SubagentContextMixin):
    hook_event_name: Literal["PreToolUse"]
    tool_name: str
    tool_input: dict[str, Any]
    tool_use_id: str

class UserPromptSubmitHookInput(BaseHookInput):
    hook_event_name: Literal["UserPromptSubmit"]
    prompt: str

class StopHookInput(BaseHookInput):
    hook_event_name: Literal["Stop"]
    stop_hook_active: bool

class SubagentStopHookInput(BaseHookInput):
    hook_event_name: Literal["SubagentStop"]
    stop_hook_active: bool
    agent_id: str
    agent_transcript_path: str
    agent_type: str

class SubagentStartHookInput(BaseHookInput):
    hook_event_name: Literal["SubagentStart"]
    agent_id: str
    agent_type: str

class PreCompactHookInput(BaseHookInput):
    hook_event_name: Literal["PreCompact"]
    trigger: Literal["manual", "auto"]
    custom_instructions: str | None

class NotificationHookInput(BaseHookInput):
    hook_event_name: Literal["Notification"]
    message: str
    title: NotRequired[str]
    notification_type: str

class PermissionRequestHookInput(BaseHookInput, _SubagentContextMixin):
    hook_event_name: Literal["PermissionRequest"]
    tool_name: str
    tool_input: dict[str, Any]
    permission_suggestions: NotRequired[list[Any]]
```

### 5.3 `parent_tool_use_id` / subagent attribution — cross-ref issue #272

**This is the headline correction to topic 03's §8.4.** Topic 03 stated, citing [claude-agent-sdk-python#272](https://github.com/anthropics/claude-agent-sdk-python/issues/272), that *"hooks do not get `parent_tool_use_id` context — observability in hook callbacks cannot currently attribute a `PreToolUse` to its enclosing subagent."* As of the 0.2.87 / current-`main` snapshot read here, that gap is **substantially closed**, and **issue #272 is CLOSED** (verified via GitHub API, 2026-05-25 — state `CLOSED`, title "Feature Request: Add subagent context to hooks API for proper tool call attribution"; the original report was filed against `claude-agent-sdk` v0.1.4).

The mechanism is the `_SubagentContextMixin`. The current docs spell out the contract:
> "`agent_id` and `agent_type` are populated when the hook fires inside a subagent. In TypeScript, these are on the base hook input and available to all hook types. In Python, they are on `PreToolUse`, `PostToolUse`, and `PostToolUseFailure` only."

So:

| | Field name in hooks | Granularity |
| --- | --- | --- |
| **Python** | `agent_id`, `agent_type` (via `_SubagentContextMixin`) | On `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest` only |
| **TypeScript** | `agent_id`, `agent_type` on base hook input | All hook types |

Important nuance: the hook payload exposes **`agent_id` / `agent_type`**, NOT the literal field name `parent_tool_use_id` that the *event stream* uses (topic 03 §8). The stream attributes by the spawning `Agent` tool's `tool_use_id`; the hook attributes by the subagent's `agent_id`. They are different identifiers for the same "which subagent" question. So the precise current state is:

- **Stream-level attribution** (topic 03): via `parent_tool_use_id` on `AssistantMessage` / `UserMessage` / `StreamEvent`. Works.
- **Hook-level attribution** (this file): via `agent_id` / `agent_type` on the hook input. **Now works** (the #272 gap), but on the named subset of hooks in Python and on the base input in TS.

Because subagents cannot spawn subagents (topic 03 §8.4 / topic 06), `agent_id` is at most one level deep. There is no documented field that maps a hook's `agent_id` back to the parent's spawning `tool_use_id` directly; a substrate that needs the full tree must correlate `SubagentStart`'s `agent_id` with the stream's `Agent`-tool `tool_use_id` itself.

**Not found / ambiguous:** the docs do not state a `uuid` field on hook inputs (the stream events carry `uuid`; the hook `BaseHookInput` in `types.py` does not). `transcript_path` is present on hook inputs (a file path to the JSONL) where stream events instead carry `session_id` only.

---

## 6. `Stop` / `SessionEnd` — end-of-turn / end-of-session capability

### 6.1 `Stop`

`Stop` fires when the agent is about to stop (turn complete / idle). Input:

```python
class StopHookInput(BaseHookInput):
    hook_event_name: Literal["Stop"]
    stop_hook_active: bool      # True if a Stop hook is already mid-execution (loop guard)
```

**Can a `Stop` hook force another turn (block the stop)?** Yes. The top-level `SyncHookJSONOutput` (verbatim, §8.1) carries `decision: NotRequired[Literal["block"]]` and `reason: NotRequired[str]`. Returning `{"decision": "block", "reason": "..."}` from a `Stop` hook blocks the stop and feeds `reason` back to keep the model going. This mirrors Claude Code's shell-hook JSON contract ("SDK callback hooks use the same JSON output format as Claude Code shell command hooks", [hooks reference](https://code.claude.com/docs/en/hooks#json-output)).

`stop_hook_active` exists precisely so a `Stop` hook can detect it is being re-entered (it was already blocking) and avoid an infinite "block → re-run → block" loop. A substrate that uses `Stop` to enforce "you must run the verifier before stopping" should check `stop_hook_active` and give up after one forced continuation.

There is also the orthogonal top-level `continue_` (Python) / `continue` (TS) flag: returning `continue_: false` from *any* hook tells the agent to **stop running after this hook** — the inverse lever (force-stop rather than force-continue). The two are independent fields.

### 6.2 `SessionEnd` (and `SessionStart`)

- **TypeScript:** registrable SDK callback hooks. `SessionStart` → init logging/telemetry; `SessionEnd` → clean up resources. Observe-only lifecycle.
- **Python:** **not registrable as callback hooks** (omitted from `HookEvent`, §1.2). Use shell-command hooks via `setting_sources=["project"]` (reads `.claude/settings.json`), or trigger init off the first `client.receive_response()` message.

No documented field lets `SessionEnd` *prevent* termination — it is a teardown notification, not a gate. (The gate for "keep going" is `Stop`'s `decision: "block"`, not `SessionEnd`.)

---

## 7. `CanUseTool` vs `PreToolUse` — the two permission surfaces

The SDK has **two distinct caller-side intervention points** for tool calls, and topic 01 §5 named `CanUseTool` separately from hooks. They are genuinely different layers.

### 7.1 `CanUseTool` callback (verbatim from `types.py`)

```python
CanUseTool = Callable[
    [str, dict[str, Any], ToolPermissionContext],   # (tool_name, input, context)
    Awaitable[PermissionResult],
]

@dataclass
class ToolPermissionContext:
    signal: Any | None = None
    suggestions: list[PermissionUpdate] = field(default_factory=list)
    tool_use_id: str | None = None
    agent_id: str | None = None
    blocked_path: str | None = None
    decision_reason: str | None = None
    title: str | None = None
    display_name: str | None = None
    description: str | None = None

@dataclass
class PermissionResultAllow:
    behavior: Literal["allow"] = "allow"
    updated_input: dict[str, Any] | None = None         # can rewrite args (like PreToolUse updatedInput)
    updated_permissions: list[PermissionUpdate] | None = None   # persist a rule ("approve and remember")

@dataclass
class PermissionResultDeny:
    behavior: Literal["deny"] = "deny"
    message: str = ""
    interrupt: bool = False                             # True → abort the whole query, not just this call

PermissionResult = PermissionResultAllow | PermissionResultDeny
```

Registered via `ClaudeAgentOptions(can_use_tool=...)` (Python) / `{ canUseTool }` (TS). TS returns plain objects: `{ behavior: "allow", updatedInput }` / `{ behavior: "deny", message }`.

### 7.2 How the two differ

| Dimension | `PreToolUse` hook | `CanUseTool` callback |
| --------- | ----------------- | --------------------- |
| Purpose | Deterministic policy / automation (block, auto-approve, rewrite) | Interactive, runtime, human-in-the-loop approval |
| Registration | `options.hooks["PreToolUse"]` (list of matchers) | `options.can_use_tool` / `canUseTool` (single callback) |
| Matcher filtering | Yes — regex on tool name | No — fires for every unresolved tool |
| Return shape | dict with `hookSpecificOutput.permissionDecision` (`allow`/`deny`/`ask`/`defer`) | `PermissionResultAllow` / `PermissionResultDeny` dataclass/object |
| Can rewrite args | Yes (`updatedInput`, with `allow`/`ask`) | Yes (`updated_input` / `updatedInput`) |
| Can persist a rule | No | Yes (`updated_permissions` / `updatedPermissions`) |
| Can hard-abort | Via top-level `continue_: false` | Via `PermissionResultDeny(interrupt=True)` |
| Runs in `dontAsk` mode | **Yes** (hooks always run) | **No** — skipped; unresolved tools are denied |
| Fires for subagents | Yes | Yes (subagents don't inherit parent approvals — may re-prompt) |
| Can stay pending indefinitely | Not designed to (subject to `timeout`, default 60s) | **Yes** — "execution remains paused until your callback returns" |

### 7.3 Ordering / precedence — the canonical gate

The permissions page publishes the **exact evaluation order** when a tool is requested ([Configure permissions — "How permissions are evaluated"](https://code.claude.com/docs/en/agent-sdk/permissions)):

1. **Hooks** run first. "A hook can deny the call outright or pass it on. A hook that returns `allow` does **not** skip the deny and ask rules below; those are evaluated regardless of the hook result."
2. **Deny rules** (`disallowed_tools` + settings.json). Match → blocked, **even in `bypassPermissions`**. (Bare-name denies like `Bash` remove the tool from context entirely beforehand.)
3. **Permission mode** — `bypassPermissions` approves everything reaching here; `acceptEdits` approves file ops; others fall through.
4. **Allow rules** (`allowed_tools` + settings.json). Match → approved.
5. **`canUseTool` callback** — only if nothing above resolved it. **In `dontAsk` mode this step is skipped and the tool is denied.**

The docs cross-link is explicit:
> "To automatically allow or deny tools without prompting users, use [hooks] instead. Hooks execute **before** `canUseTool` and can allow, deny, or modify requests based on your own logic."

**Canonical gate:** there isn't a single one — they are *layered*. `PreToolUse` hooks are the **first and most deterministic** gate (and the right place for substrate policy: they run even in `dontAsk`/`bypassPermissions` and can't be bypassed by mode). `canUseTool` is the **last** gate and is the right place for human approval. Both **coexist**; the hook fires first, and a hook `deny` short-circuits before `canUseTool` is ever consulted. A hook `allow` does **not** short-circuit deny/ask rules — only a hook `deny` is final-early.

### 7.4 Python streaming-mode gotcha (important for substrate code)

In Python, `can_use_tool` **requires streaming mode and a dummy `PreToolUse` hook to keep the stream open**:
> "In Python, `can_use_tool` requires streaming mode and a `PreToolUse` hook that returns `{"continue_": True}` to keep the stream open. Without this hook, the stream closes before the permission callback can be invoked."

Verbatim workaround from the docs:
```python
async def dummy_hook(input_data, tool_use_id, context):
    return {"continue_": True}

options = ClaudeAgentOptions(
    can_use_tool=can_use_tool,
    hooks={"PreToolUse": [HookMatcher(matcher=None, hooks=[dummy_hook])]},
)
# prompt must be an async generator (streaming), not a plain string
```
This is a documented Python-only requirement; the TS SDK has no equivalent caveat.

---

## 8. Async hooks and I/O

### 8.1 Hook output type (verbatim from `types.py`)

```python
class AsyncHookJSONOutput(TypedDict):
    async_: Literal[True]                # TS: `async`; Python uses async_ (reserved keyword)
    asyncTimeout: NotRequired[int]       # ms

class SyncHookJSONOutput(TypedDict):
    continue_: NotRequired[bool]         # TS: `continue`; False → stop the agent after this hook
    suppressOutput: NotRequired[bool]
    stopReason: NotRequired[str]
    decision: NotRequired[Literal["block"]]     # Stop-hook: block the stop (§6)
    reason: NotRequired[str]
    systemMessage: NotRequired[str]      # message shown to the USER
    hookSpecificOutput: NotRequired[HookSpecificOutput]

HookJSONOutput = AsyncHookJSONOutput | SyncHookJSONOutput
```

### 8.2 Are callbacks allowed to be async?

**They MUST be async.** `HookCallback` is typed `-> Awaitable[HookJSONOutput]` (Python) and the TS `HookCallback` is an `async` function. There is no synchronous callback form. The SDK awaits the result before proceeding (it is a true intervention point in the loop).

### 8.3 Can they do I/O before deciding?

Yes — read a file, run a subprocess, query a DB, make an HTTP call. The docs explicitly demonstrate HTTP-from-hook (§4.3 webhook) and `asyncio.to_thread()` for wrapping blocking calls so they don't stall the event loop. A substrate's out-of-scope guard can `stat()` paths, a budget cap can read its own spend tally, etc., all inside the awaited callback before returning the decision.

### 8.4 Fire-and-forget (async output)

For pure side effects where the decision shouldn't block the loop, return the **async output** form so the agent proceeds immediately:

```python
async def async_hook(input_data, tool_use_id, context):
    asyncio.create_task(send_to_logging_service(input_data))
    return {"async_": True, "asyncTimeout": 30000}   # ms; agent does NOT wait
```
```typescript
const asyncHook: HookCallback = async (input, toolUseID, { signal }) => {
  sendToLoggingService(input).catch(console.error);
  return { async: true, asyncTimeout: 30000 };
};
```
Hard constraint (docs `<Note>`): **"Async outputs cannot block, modify, or inject context"** — the agent has already moved on. Use them only for logging/metrics/notifications, never for a deny/mutate decision.

### 8.5 Timeout

- Per-matcher `timeout` field on `HookMatcher` — **default 60 seconds** (per the matcher options table on the hooks page). Increase it if a hook does slow I/O.
- `asyncTimeout` (ms) bounds the fire-and-forget background op in async mode.
- TS: the third arg's `signal` (`AbortSignal`) fires on timeout/cancellation; pass it to `fetch(..., { signal })` so slow calls cancel cleanly. Python's `HookContext.signal` is "reserved for future use" — **no working cancellation signal in Python today** (a Python hook that exceeds `timeout` is timed out by the SDK but cannot itself observe the signal).

---

## 9. Failure behavior

What happens on raise / timeout / malformed return is only **partially documented**; the rest is inferred and flagged.

- **Unhandled exception in a hook → can interrupt the agent.** The docs state directly: "Catch errors inside your hook instead of letting them propagate, since an unhandled exception can interrupt the agent." The webhook examples (§4.3) wrap everything in `try/except` and *swallow* the error precisely so "a failed webhook doesn't interrupt the agent." So a raising hook is treated as fatal-ish to the current turn, not silently ignored. The exact blast radius (tool call vs. turn vs. session) is **not precisely specified** in the docs.
- **Timeout.** A hook that exceeds its `timeout` is cancelled by the SDK. In TS the `AbortSignal` fires so the hook can clean up; in Python there is no observable signal (§8.5). The docs' remedy ("Increase the `timeout` value … Use the `AbortSignal`") implies a timed-out hook does not silently succeed — but whether the timeout denies the tool, allows it, or aborts the turn is **not documented**. Treat a hook timeout as undefined-but-not-safe.
- **Malformed return.** Not documented. The output is a `TypedDict`; returning a shape that doesn't validate (e.g. `updatedInput` at top level instead of inside `hookSpecificOutput`, or `permissionDecision` outside `allow|deny|ask|defer`) is silently ineffective per the troubleshooting section ("Modified input not applied → ensure `updatedInput` is inside `hookSpecificOutput`"). There is no documented hard error for a malformed-but-parseable dict — it just fails to take effect.
- **`max_turns` interaction.** "Hooks may not fire when the agent hits the `max_turns` limit because the session ends before hooks can execute." So a `Stop`/`SessionEnd` hook is **not** a reliable place to run mandatory teardown when a turn budget can be hit — the substrate must also handle teardown on the `ResultMessage` (topic 03 §4) with `subtype: "error_max_turns"`.
- **`systemMessage` visibility.** By default the SDK does **not** surface hook output (incl. `systemMessage`) in the message stream. To see hook lifecycle in the stream, set `include_hook_events=True` (Python) / `includeHookEvents: true` (TS) — which emits `SDKHookStartedMessage` / `SDKHookProgressMessage` / `SDKHookResponseMessage` (topic 03 §1.3 listed these as TS-only stream variants). To pass text to the **model** instead of the user, use `additionalContext`, not `systemMessage`. A substrate that must reliably observe its own hook decisions should log them out-of-band rather than rely on stream surfacing.

---

## External references

- [Intercept and control agent behavior with hooks (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/hooks) — primary: full event matrix, registration, `PreToolUse`/`PostToolUse` examples, async output, troubleshooting, Python `SessionStart`/`SessionEnd` omission, matcher semantics, parallel-hook precedence.
- [Configure permissions (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/permissions) — the 5-step permission evaluation order (Hooks → Deny → Mode → Allow → `canUseTool`); permission modes incl. `dontAsk` skipping `canUseTool`; `bypassPermissions` still runs hooks.
- [Handle approvals and user input (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/user-input) — `canUseTool` signature, `PermissionResultAllow`/`PermissionResultDeny`, `updated_input`/`updated_permissions`, the "hooks execute before `canUseTool`" note, the Python streaming + dummy-hook workaround.
- [`types.py` on `main` (anthropics/claude-agent-sdk-python)](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/types.py) — canonical: `HookEvent` literal, `HookMatcher`, `HookCallback`, `HookContext`, `BaseHookInput`, all `*HookInput` TypedDicts, `_SubagentContextMixin`, `PreToolUseHookSpecificOutput`, `PostToolUseHookSpecificOutput`, `SyncHookJSONOutput`/`AsyncHookJSONOutput`, `CanUseTool`, `ToolPermissionContext`, `PermissionResultAllow`/`Deny`, `ClaudeAgentOptions.hooks` field.
- [TypeScript SDK reference (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/typescript) — `hooks?: Partial<Record<HookEvent, HookCallbackMatcher[]>>` on `Options`; `includeHookEvents`; the TS `SDKHook*Message` stream variants.
- [Claude Code hooks reference (code.claude.com)](https://code.claude.com/docs/en/hooks#json-output) — the shared shell-hook JSON output contract the SDK callbacks reuse (`decision: "block"`, `continue`, `defer`).
- [claude-agent-sdk-python#272 (CLOSED)](https://github.com/anthropics/claude-agent-sdk-python/issues/272) — original request for subagent context in hooks; now addressed via `agent_id`/`agent_type` (`_SubagentContextMixin`). Verified CLOSED 2026-05-25.

---

## Caveats / Not found

- **Failure blast radius is under-documented.** The docs say an unhandled hook exception "can interrupt the agent" but do not specify whether it fails the tool call, the turn, or the whole session. Timeout and malformed-return behavior are likewise not precisely specified. The §9 statements past the documented sentence are inference, flagged as such.
- **TS hook input/output field shapes are not on the TS reference page.** The TS reference exposes only `hooks?: Partial<Record<HookEvent, HookCallbackMatcher[]>>` and `includeHookEvents`. The TS `HookInput` / `BaseHookInput` / `PreToolUseHookInput` field lists were taken from the hooks-page prose ("these are on the base hook input … available to all hook types") and from the cross-SDK parity the hooks page asserts. The authoritative TS shapes live in the TS package's generated `.d.ts`, which was not directly extractable this pass. Field *names* are assumed snake_case on input (matching the hooks-page TS examples: `input.hook_event_name`, `preInput.tool_input`, `subInput.agent_id`) and camelCase on output — verified against the doc code samples, not against the `.d.ts`.
- **`agent_id` ≠ `parent_tool_use_id`.** The hook payload attributes subagents via `agent_id`/`agent_type`; the *stream* (topic 03) attributes via `parent_tool_use_id` (= the spawning `Agent` tool's `tool_use_id`). No documented field directly maps one to the other; correlating them requires watching `SubagentStart` and the `Agent` tool-use block. The earlier topic-03 claim that "hooks lack `parent_tool_use_id` context" is now **stale** — hooks have `agent_id` (Python: on the tool-lifecycle subset; TS: on all hooks), and issue #272 is closed.
- **Python `permission_mode` on `BaseHookInput` is `NotRequired`** — a hook cannot assume it is always present in the payload (use `input_data.get("permission_mode")`).
- **Python `HookContext.signal` is inert** ("reserved for future use"). Cancellation-aware hooks are TS-only today.
- **`PermissionUpdate` schema** (referenced by `ToolPermissionContext.suggestions` and `PermissionResultAllow.updated_permissions`) was not extracted field-by-field here; it belongs to topic 05 (tools-and-permissions). Only its role ("echo a suggestion back to persist a rule") is established.
- **Whether `updatedToolOutput` from `PostToolUse` is also written to the persisted JSONL transcript** (vs. only shown to the model in-memory) is not documented. Relevant if a substrate replays transcripts; flagged for topic 10 (persistence).
- **`SubagentStop.stop_hook_active` semantics for subagents** vs. the main-session `Stop` are assumed identical (re-entry guard) but not separately documented.
- Version note: no hook-API delta is documented between TS 0.3.142 (topic 03's pin) and 0.3.150 (topic 01's pin). If a newer release than 0.2.87 (Py) / 0.3.150 (TS) is out at read time, re-verify the `HookEvent` literal and the `permissionDecision` enum, since both have grown across minor releases.
