# Research: Claude Agent SDK — defining, invoking, and collecting subagents

- Query: how subagents are DEFINED (`AgentDefinition` vs filesystem `.claude/agents/*.md`) and INVOKED, and how their results return to the parent — plus recursion depth, concurrency, context isolation, per-subagent model/tool override, and the subagent-vs-separate-`query()` design fork.
- Scope: external (primary sources: `code.claude.com` Agent SDK + Claude Code docs; published SDK source; GitHub issues for contested behavior).
- Date: 2026-05-25
- Version pin:
  - Python `claude-agent-sdk` **0.2.87** (PyPI, released 2026-05-23).
  - TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.150** (npm).
  - Underlying Claude Code CLI: v2.1.x. Relevant landmarks: `Task`→`Agent` tool rename at v2.1.63; `forwardSubagentText` added at TS 0.2.119; `AgentToolInput.subagent_type` made optional (defaults to `general-purpose`) — see §2.
  - **Newer than the corpus snapshot:** none observed past 0.3.150 / 0.2.87 at fetch time. Where the two SDKs diverge in field coverage, this file flags it (notably `forwardSubagentText` / `criticalSystemReminder_EXPERIMENTAL`, both TS-only — see §1.3).

**Cross-references (do not duplicate):**
- **Topic 03 `03_streaming-events.md`** owns how subagent EVENTS are tagged on the stream: `parent_tool_use_id` attribution, the `Agent`/`Task` `tool_use` → `tool_result` thread, background-task events (`TaskStartedMessage` / `TaskProgressMessage` / `TaskNotificationMessage`), and `forward_subagent_text`/`forwardSubagentText` as a *stream-rendering* switch. This file references those mechanisms when explaining result return but does not re-enumerate the event taxonomy. **One correction to topic 03 is recorded in §3.4 and the Caveats: `forward_subagent_text` does NOT exist on the current Python surface — it is TS-only.**
- **Topic 02 `02_sessions.md`** owns subagent transcript on-disk storage (`~/.claude/projects/<project>/<sessionId>/subagents/agent-<id>.jsonl`) and the session-store `subpath` mechanics. This file references it only where result-collection depends on it.
- **Topic 01** named `AgentDefinition` and the `agents` option at one-liner depth; this file gives the full field list.
- **Topic 05 `05_tools-and-permissions.md`** owns the tool-restriction / permission-mode mechanism in general; §7 here cross-refs it for the per-subagent override path.

---

## TL;DR for the architecture fork

If you only read one section, read this — the rest is the evidence.

1. **Definition: two paths, programmatic wins.** `AgentDefinition` objects passed via the `agents` option (a `dict`/`Record` keyed by agent name) OR markdown files under `.claude/agents/` discovered through `settingSources`. **Programmatic agents take precedence over filesystem agents with the same name.** (§1, §1.4)
2. **Invocation is MODEL-DECIDED, not host-forced.** The parent **model** calls the built-in `Agent` tool (formerly `Task`) when it judges a subtask matches a subagent's `description`. **The host cannot deterministically say "run agent X now" from code in a single `query()`.** The strongest host levers are (a) phrasing the prompt to name the agent and (b) `allowedTools` gating — both are *steering*, not *forcing*. True determinism requires the host to open a **separate `query()`** (e.g. `--agent` / the `agent` setting to run a whole session *as* that agent). (§2)
3. **Result return shape: TEXT, not parsed data. THIS IS THE KEY FINDING.** The subagent's final assistant message returns to the parent **verbatim as the `Agent` tool result text block** in the parent conversation. There is **no per-subagent structured-output channel** — `outputFormat` / structured output applies only to the top-level `query()`, not to subagents ([TS issue #104](https://github.com/anthropics/claude-agent-sdk-typescript/issues/104), open). A host that needs "subagent wrote file X and concluded Y" as data must either (a) instruct the subagent to write a file/JSON to a known path and read it host-side, or (b) parse the `ToolResultBlock` whose `tool_use_id` matches the `Agent` `tool_use` out of the stream. (§3)
4. **Recursion depth = 1. Confirmed definitively.** Subagents **cannot** spawn subagents. A substrate needing recursive task trees must orchestrate recursion **host-side**. (§4)
5. **Concurrency: yes, the model can fan out, but it decides — no documented hard limit, and results all flow back into the one parent context.** (§5)
6. **Context isolation: fresh window.** The subagent inherits CLAUDE.md/memory/git-status and tool definitions, but NOT the parent's conversation history. The only parent→child data channel is the `Agent` tool's `prompt` string. (§6)
7. **Per-subagent model/tool/permission override: yes, fully.** `model`, `tools`, `disallowedTools`, `permissionMode`, `effort`, `maxTurns`, `mcpServers`, `skills`, `memory` are all per-`AgentDefinition`. (§7)
8. **Subagent vs separate `query()`:** subagent = cheap context isolation + automatic result-injection, but non-deterministic dispatch + text-only return + no recursion. Separate `query()` = deterministic, structured-output-capable, recursive, parallelizable host-side — at the cost of wiring the result plumbing yourself. **For a workflow substrate that dispatches named reviewer/verifier roles and needs recursive sub-tasks, separate `query()` is the more load-bearing primitive.** (§8)

---

## 1. Two definition paths

The subagents doc names **three** ways to create a subagent, but two are the
real definition surfaces (the third is just the always-present built-in):

> You can create subagents in three ways:
> * **Programmatically**: use the `agents` parameter in your `query()` options
> * **Filesystem-based**: define agents as markdown files in `.claude/agents/` directories
> * **Built-in general-purpose**: Claude can invoke the built-in `general-purpose` subagent at any time via the Agent tool without you defining anything
> — [Subagents in the SDK](https://code.claude.com/docs/en/agent-sdk/subagents)

The SDK doc states the recommendation directly: "This guide focuses on the
programmatic approach, which is recommended for SDK applications."

### 1.1 Path A — programmatic `AgentDefinition`

Passed via the `agents` option, which is a map of **agent name → definition**:

- Python: `ClaudeAgentOptions.agents: dict[str, AgentDefinition] | None = None`
  — docstring: *"Programmatically define custom subagents invokable via the Agent tool. Keys are agent names, values are agent definitions."*
- TypeScript: `Options.agents?: Record<string, AgentDefinition>`.

**Minimal Python definition** (verbatim shape from the SDK subagents doc):

```python
from claude_agent_sdk import query, ClaudeAgentOptions, AgentDefinition

async for message in query(
    prompt="Review the authentication module for security issues",
    options=ClaudeAgentOptions(
        # Auto-approve Agent so subagent invocations don't hit a permission prompt
        allowed_tools=["Read", "Grep", "Glob", "Agent"],
        agents={
            "code-reviewer": AgentDefinition(
                description="Expert code review specialist. Use for quality, security, and maintainability reviews.",
                prompt="You are a code review specialist...",
                tools=["Read", "Grep", "Glob"],   # read-only
                model="sonnet",                    # override parent model
            ),
        },
    ),
):
    if hasattr(message, "result"):
        print(message.result)
```

**Minimal TypeScript definition** (equivalent):

```typescript
for await (const message of query({
  prompt: "Review the authentication module for security issues",
  options: {
    allowedTools: ["Read", "Grep", "Glob", "Agent"],
    agents: {
      "code-reviewer": {
        description: "Expert code review specialist...",
        prompt: "You are a code review specialist...",
        tools: ["Read", "Grep", "Glob"],
        model: "sonnet",
      },
    },
  },
})) {
  if ("result" in message) console.log(message.result);
}
```

### 1.2 `AgentDefinition` fields (verbatim)

The doc's field table and the published source agree. Note **Python uses
camelCase field names** on the dataclass "to match the wire format" — i.e.
`disallowedTools`, `mcpServers`, `maxTurns`, `permissionMode`, `initialPrompt`
are camelCase even in Python (only `description`, `prompt`, `tools`, `model`,
`skills`, `memory`, `background`, `effort` happen to be single-word/lowercase).

Python dataclass (verbatim from `src/claude_agent_sdk/types.py`, main @ 2026-05-25):

```python
@dataclass
class AgentDefinition:
    """Agent definition configuration."""
    description: str
    prompt: str
    tools: list[str] | None = None
    disallowedTools: list[str] | None = None        # noqa: N815
    model: str | None = None                          # alias or full ID or "inherit"
    skills: list[str] | None = None
    memory: Literal["user", "project", "local"] | None = None
    mcpServers: list[str | dict[str, Any]] | None = None   # noqa: N815
    initialPrompt: str | None = None                  # noqa: N815
    maxTurns: int | None = None                       # noqa: N815
    background: bool | None = None
    effort: EffortLevel | int | None = None
    permissionMode: PermissionMode | None = None      # noqa: N815
```

TypeScript type (verbatim from the [TS reference](https://code.claude.com/docs/en/agent-sdk/typescript)):

```typescript
type AgentDefinition = {
  description: string;
  tools?: string[];
  disallowedTools?: string[];
  prompt: string;
  model?: string;
  mcpServers?: AgentMcpServerSpec[];
  skills?: string[];
  initialPrompt?: string;
  maxTurns?: number;
  background?: boolean;
  memory?: "user" | "project" | "local";
  effort?: "low" | "medium" | "high" | "xhigh" | "max" | number;
  permissionMode?: PermissionMode;
  criticalSystemReminder_EXPERIMENTAL?: string;   // TS-only, experimental
};
```

Field-by-field (doc table, verbatim "Description" column):

| Field             | Type (TS)                                                   | Required | Meaning                                                                                                                             |
| ----------------- | ----------------------------------------------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `description`     | `string`                                                    | **Yes**  | "Natural language description of when to use this agent" — **this is what the parent model matches against to decide to delegate.** |
| `prompt`          | `string`                                                    | **Yes**  | "The agent's system prompt defining its role and behavior"                                                                          |
| `tools`           | `string[]`                                                  | No       | "Array of allowed tool names. If omitted, inherits all tools" (allowlist)                                                           |
| `disallowedTools` | `string[]`                                                  | No       | "Array of tool names to remove from the agent's tool set" (denylist)                                                               |
| `model`           | `string`                                                    | No       | "Model override for this agent. Accepts an alias such as `'sonnet'`, `'opus'`, `'haiku'`, `'inherit'`, or a full model ID. Defaults to main model if omitted" |
| `skills`          | `string[]`                                                  | No       | "List of skill names to preload into the agent's context at startup. Unlisted skills remain invocable through the Skill tool"      |
| `memory`          | `'user' \| 'project' \| 'local'`                            | No       | "Memory source for this agent" (persistent per-agent memory dir)                                                                   |
| `mcpServers`      | `(string \| object)[]`                                      | No       | "MCP servers available to this agent, by name or inline config"                                                                    |
| `initialPrompt`   | `string`                                                    | No       | First auto-submitted user turn when the agent runs as the **main** session (`--agent`); prepended to user prompt                   |
| `maxTurns`        | `number`                                                    | No       | "Maximum number of agentic turns before the agent stops"                                                                            |
| `background`      | `boolean`                                                   | No       | "Run this agent as a non-blocking background task when invoked" (→ topic 03 task events)                                            |
| `effort`          | `'low' \| 'medium' \| 'high' \| 'xhigh' \| 'max' \| number` | No       | "Reasoning effort level for this agent"                                                                                             |
| `permissionMode`  | `PermissionMode`                                            | No       | "Permission mode for tool execution within this agent" (→ topic 05)                                                                |

`PermissionMode` (Python `types.py`): `Literal["default", "acceptEdits", "plan", "bypassPermissions", "dontAsk", "auto"]`. `EffortLevel`: `Literal["low", "medium", "high", "xhigh", "max"]`.

The `tools` field doubles as the **deprecated** Skill-preload channel — the
Python source comments `# Deprecated: passing "Skill" here is deprecated; use 'skills' instead.`

### 1.3 Python ↔ TS divergence in `AgentDefinition`

| Field                                  | Python 0.2.87 | TS 0.3.150 | Note |
| -------------------------------------- | ------------- | ---------- | ---- |
| All 13 documented fields above         | ✅            | ✅         | parity |
| `criticalSystemReminder_EXPERIMENTAL`  | ❌ (absent)   | ✅         | TS-only, experimental |
| `forwardSubagentText`                  | ❌            | ❌         | **NOT on `AgentDefinition` in either SDK** — it is an `Options`/`query()` field, TS-only (see §3.4) |

### 1.4 Path B — filesystem `.claude/agents/*.md` discovery

Markdown files with YAML frontmatter + a Markdown body that becomes the system
prompt. From [Create custom subagents](https://code.claude.com/docs/en/sub-agents):

```markdown
---
name: code-reviewer
description: Reviews code for quality and best practices
tools: Read, Glob, Grep
model: sonnet
---

You are a code reviewer. When invoked, analyze the code and provide
specific, actionable feedback on quality, security, and best practices.
```

**Frontmatter fields** (verbatim from the doc; only `name` and `description`
are required): `name`, `description`, `tools`, `disallowedTools`, `model`,
`permissionMode`, `maxTurns`, `skills`, `mcpServers`, `hooks`, `memory`,
`background`, `effort`, `isolation`, `color`, `initialPrompt`.

Two frontmatter fields have **no `AgentDefinition` programmatic equivalent**:
- `hooks` — lifecycle hooks scoped to the subagent (frontmatter only).
- `isolation: worktree` — run the subagent in a temporary git worktree
  (frontmatter / `--agents` JSON only). "The worktree is automatically cleaned
  up if the subagent makes no changes." (Relevant to Ark's own worktree feature.)
- `color` — UI display only.

**Discovery is `settingSources`-gated** (cross-ref topic 02 §8). Filesystem
agents load only when the active `settingSources` includes the scope that owns
the directory. Per topic 02: omitting `settingSources` on `query()` is
equivalent to `["user", "project", "local"]` (all load); passing `[]` disables
filesystem agent discovery entirely. The subagents doc says filesystem agents
"are loaded at startup only. If you create a new agent file while Claude Code
is running, restart the session to load it."

**Scope precedence** (doc table; higher priority wins on name collision):

| Location                     | Scope             | Priority    |
| ---------------------------- | ----------------- | ----------- |
| Managed settings             | Organization-wide | 1 (highest) |
| `--agents` CLI flag (JSON)   | Current session   | 2           |
| `.claude/agents/`            | Current project   | 3           |
| `~/.claude/agents/`          | All your projects | 4           |
| Plugin's `agents/` directory | Plugin scope      | 5 (lowest)  |

Discovery walks up from cwd for project agents and scans `.claude/agents/` and
`~/.claude/agents/` **recursively** (subfolders allowed; identity comes only
from the `name` frontmatter field, NOT the path — except for plugins, where a
subfolder becomes part of a scoped id like `my-plugin:review:security`).

**Programmatic vs filesystem precedence** (verbatim):

> Programmatically defined agents take precedence over filesystem-based agents with the same name.

So an `AgentDefinition` named `code-reviewer` masks a `.claude/agents/code-reviewer.md`.

> **For Ark:** Ark currently ships filesystem agents (`ark-researcher`,
> `ark-reviewer`, `ark-verifier` under `.claude/agents/`, per the session
> context). Those are Path-B agents. The SDK's Path A (`AgentDefinition`) is
> the in-process equivalent — same fields minus `hooks`/`isolation`/`color`.
> A substrate can ship either; Path A avoids the disk round-trip and the
> "restart to reload" caveat, and overrides any same-named filesystem card.

---

## 2. How a parent invokes a subagent — model-decided, not host-forced

**The critical distinction asked: does the PARENT MODEL decide (non-deterministic),
or can HOST CODE force "run agent X now"?**

**Answer: the parent model decides. The host cannot force a named subagent run
deterministically inside a single `query()`.** Invocation is the built-in
`Agent` tool (renamed from `Task` at v2.1.63), which the model calls
autonomously when it judges the task matches a subagent's `description`:

> When you define subagents, Claude determines whether to invoke them based on each subagent's `description` field. Write clear descriptions that explain when the subagent should be used, and Claude will automatically delegate appropriate tasks.

> Claude invokes subagents through the `Agent` tool, so include `Agent` in `allowedTools` to auto-approve subagent invocations without a permission prompt.

### 2.1 The two host-side levers (steering, not forcing)

The doc offers "automatic" and "explicit" invocation, but **even "explicit" is
a prompt-phrasing nudge**, not a code-level guarantee inside a `query()`:

> **Automatic invocation** — Claude automatically decides when to invoke subagents based on the task and each subagent's `description`.

> **Explicit invocation** — To guarantee Claude uses a specific subagent, mention it by name in your prompt: `"Use the code-reviewer agent to check the authentication module"`. This bypasses automatic matching and directly invokes the named subagent.

The word "guarantee" in the doc is doing heavy lifting — it is a *prompt*
("Use the code-reviewer agent to…"), still mediated by the model's tool-call
decision. The Troubleshooting section confirms it is not hard-deterministic:

> **Claude not delegating to subagents** — If Claude completes tasks directly instead of delegating to your subagent: 1. Check Agent invocations are approved … 2. Use explicit prompting … 3. Write a clear description …

So the two real host levers are:
1. **Prompt phrasing** — naming the agent (the closest to "force," still soft).
2. **`allowedTools` gating** — include `Agent` to auto-approve; omit it and
   subagent calls "fall through to your `canUseTool` callback or, in `dontAsk`
   mode, are denied." This is a *can/can't* gate, not a *which-one-now* gate.

### 2.2 The `Agent` tool input shape

When the model calls the tool, the input carries the target agent name and the
task prompt. From the TS CHANGELOG: `AgentToolInput.subagent_type` was **changed
to optional — defaults to the `general-purpose` agent when omitted.** The
detection snippet in the doc reads `block.input.get('subagent_type')` (Python)
/ `block.input.subagent_type` (TS). So the tool input is roughly
`{ subagent_type?: string, prompt: string, description?: string }` — the model
fills `subagent_type` with the agent name and `prompt` with the task text it
authors. **The host does not pre-fill this input** in the normal `query()`
flow; the model emits it.

> The tool name was renamed from `"Task"` to `"Agent"` in Claude Code v2.1.63. Current SDK releases emit `"Agent"` in `tool_use` blocks but still use `"Task"` in the `system:init` tools list and in `result.permission_denials[].tool_name`. Checking both values in `block.name` ensures compatibility across SDK versions.

### 2.3 The deterministic alternative: run a whole session AS the agent

The **only** documented way for host code to deterministically force a specific
agent's prompt/tools/model is to make it the **main thread** of a *separate*
session — not to spawn it as a subagent:

> **Run the whole session as a subagent.** Pass `--agent <name>` to start a session where the main thread itself takes on that subagent's system prompt, tool restrictions, and model … The subagent's system prompt replaces the default Claude Code system prompt entirely.

Programmatically this is the `agent` setting / `--agent` route. It is **not a
subagent invocation** — it is "open a new `query()` configured as agent X." This
is the bridge to §8: deterministic role dispatch ⇒ separate `query()`.

The Query interface also exposes `supportedAgents(): Promise<AgentInfo[]>` (TS)
— "Returns available subagents." This lets host code *enumerate* what agents are
registered, but enumeration is not dispatch; there is no `query.runAgent(name)`.

### 2.4 Detecting an invocation on the stream (cross-ref topic 03)

Per topic 03 §8: watch for a `ToolUseBlock` with `name in ("Agent","Task")`,
capture its `id`; messages with `parent_tool_use_id == <that id>` are inside the
subagent. This file does not re-cover the event mechanics — see topic 03.

---

## 3. Result return shape — TEXT, not parsed data (the key question)

**Answer (b): the subagent's result returns ONLY as a text block injected into
the parent conversation.** There is no parsed-data channel from a subagent.

### 3.1 The verbatim contract

> The parent receives the subagent's final message verbatim as the Agent tool result, but may summarize it in its own response.

> Each subagent runs in its own fresh conversation. Intermediate tool calls and results stay inside the subagent; only its final message returns to the parent.

So the flow is:
1. Model emits `Agent` `tool_use` (id = `T`).
2. Subagent runs to completion in its own context.
3. The subagent's **final assistant message text** comes back as the
   `tool_result` for `tool_use` id `T` — i.e. a `ToolResultBlock` whose
   `tool_use_id == T`, delivered inside a synthetic `UserMessage` on the parent
   stream (topic 03 §5.1, §8.2).
4. The parent model reads that text and "may summarize it in its own response."

### 3.2 No per-subagent structured output (contested → confirmed gap)

The top-level `query()` supports structured output (`ResultMessage.structured_output`,
topic 03 §4). **Subagents do not.** This is an open, acknowledged gap:

> **[TS issue #104](https://github.com/anthropics/claude-agent-sdk-typescript/issues/104) — "Support outputFormat for Task tool / subagents" (OPEN):** "When using the Task tool to spawn subagents, the full subagent response is returned to the main agent's context. This causes context bloat … Currently, `outputFormat` with `json_schema` only works for the top-level `query()`, not for subagent results. … Add `outputFormat` support to either the Task tool parameters, or programmatic agent definitions via the `agents` option."

The issue is unresolved as of 2026-05-25. **There is no `outputFormat` /
`json_schema` field on `AgentDefinition`** (confirmed against both the Python
dataclass and the TS type in §1.2). A subagent cannot be constrained to return
machine-parseable JSON via the SDK; it returns free-form assistant text.

### 3.3 How a host reliably extracts "subagent wrote file X and concluded Y"

Two patterns, in order of robustness:

1. **Side-channel via the filesystem (most reliable).** Instruct the subagent
   in its `prompt`/`AgentDefinition.prompt` to **write its structured output to
   a known path** (e.g. `research/<topic>.md`, or a JSON file). The host reads
   that file directly after the parent `query()` completes. The "wrote file X"
   half is then deterministic on disk; the "concluded Y" half lives in that
   file in whatever format you mandated. This is exactly the pattern issue #104
   describes as the workaround ("Each worker reads sources, writes analysis to
   disk, and should return only a compact JSON summary"). **This is the pattern
   Ark's own researcher agents already use** (write to `research/*.md`, return a
   path-plus-summary contract) — it is the recommended shape precisely because
   the SDK gives no structured subagent return.

2. **Parse the `ToolResultBlock` out of the stream.** Match the `Agent`
   `tool_use` id to the `ToolResultBlock.tool_use_id` (topic 03 §8.2) and treat
   `ToolResultBlock.content` (string or list of content parts) as the
   subagent's verbatim final message. This gets you the raw text, but it is
   still free-form prose unless the subagent's prompt forces a parseable shape.
   Fragile: the doc warns the parent "may summarize it," so reading
   `ResultMessage.result` (the parent's final text) is **not** a faithful copy
   of the subagent's output — read the `ToolResultBlock`, not the parent result.

> **For Ark:** mandate a file-write contract in every subagent prompt (the
> researcher already does). Do NOT rely on the parent's `ResultMessage.result`
> to carry subagent conclusions verbatim — it is the parent's summary, not the
> subagent's output. If you must read from the stream, key on the `Agent`
> `tool_use_id` → `ToolResultBlock.tool_use_id` thread.

### 3.4 `forwardSubagentText` — rendering, not data (corrects topic 03)

Topic 03 §8.3 cited `forward_subagent_text` (Python) / `forwardSubagentText`
(TS). **Correction confirmed this pass:**

- `forwardSubagentText` exists as a **TS `Options` field** (NOT on
  `AgentDefinition`), added at **TS 0.2.119**. Verbatim from the TS reference:
  > "Forward subagent text and thinking blocks as assistant and user messages with `parent_tool_use_id` set, so consumers can render a nested transcript. By default only `tool_use` and `tool_result` blocks from subagents are emitted." Default `false`.
- **`forward_subagent_text` does NOT exist anywhere in the Python 0.2.87
  source** (`grep` of `types.py` and a repo code search both return nothing).
  Topic 03's snake_case Python reference is **not present in the current
  Python SDK** — treat it as TS-only.

Crucially, `forwardSubagentText` is a **stream-visibility switch for rendering
a nested transcript**, NOT a result-return mechanism. With it off, the parent
still receives the subagent's final message as the `Agent` `tool_result`; the
flag only controls whether the subagent's *intermediate* text/thinking blocks
are also surfaced on the parent stream for the consumer to display. It does
nothing to make the result parseable.

---

## 4. Recursion depth — confirmed 1 (subagents cannot spawn subagents)

**Verified against current docs and source. Definitive: depth is at most 1.**

Three independent verbatim statements:

> **(SDK subagents doc, `<Note>`):** Subagents cannot spawn their own subagents. Don't include `Agent` in a subagent's `tools` array.

> **(Claude Code subagents doc, "Choose between…" `<Note>`):** Subagents cannot spawn other subagents. If your workflow requires nested delegation, use Skills or chain subagents from the main conversation.

> **(Claude Code subagents doc, "Restrict which subagents…"):** Subagents cannot spawn other subagents, so `Agent(agent_type)` has no effect in subagent definitions.

Even the built-in Plan agent exists partly to enforce this:

> This prevents infinite nesting (subagents cannot spawn other subagents) while still gathering necessary context.

The experimental fork mode is also capped: "A fork cannot spawn further forks."

### 4.1 Implication for a recursive task substrate (load-bearing)

A substrate that needs **recursive task trees** (a task spawns sub-tasks that
spawn sub-sub-tasks) **cannot lean on nested SDK subagents** — the SDK gives
exactly one level of model-driven delegation. Recursion must be orchestrated
**host-side**:

- The parent `query()` runs the top task.
- When a sub-task is needed, the **host** opens a *new* `query()` (or `--agent`
  session) for it — host code owns the call graph, depth tracking, and
  budget/cancellation per level.
- The SDK's one-level subagent is then usable only for *leaf* fan-out (parallel
  read-only analysis), not for the recursive spine.

This is the same conclusion as §8 and the central architecture fork: **the
recursive spine of a workflow substrate is host-orchestrated separate
`query()` calls, not nested subagents.**

---

## 5. Concurrency — model-driven fan-out, no documented hard limit

**Multiple subagents can run in parallel within one parent session — but the
parent MODEL decides to fan out, and all results return into the one parent
context.**

> **Parallelization** — Multiple subagents can run concurrently, dramatically speeding up complex workflows. **Example:** during a code review, you can run `style-checker`, `security-scanner`, and `test-coverage` subagents simultaneously, reducing review time from minutes to seconds.

> **Run parallel research** — For independent investigations, spawn multiple subagents to work simultaneously: *"Research the authentication, database, and API modules in parallel using separate subagents."* Each subagent explores its area independently, then Claude synthesizes the findings. This works best when the research paths don't depend on each other.

### 5.1 Who dispatches and how

The **model** dispatches concurrently by emitting multiple `Agent` `tool_use`
calls (and/or by using `background: true` agents). The host does not control
the fan-out degree directly — it is again a prompt-steering outcome ("…in
parallel using separate subagents"). Foreground vs background:

> **Foreground subagents** block the main conversation until complete. Permission prompts are passed through to you as they come up.
> **Background subagents** run concurrently while you continue working. They run with the permissions already granted in the session and auto-deny any tool call that would otherwise prompt.

Background-task progress surfaces as the `TaskStartedMessage` /
`TaskProgressMessage` / `TaskNotificationMessage` stream events (topic 03 §1.1,
§5.3). A relevant fix in the TS CHANGELOG: "Fixed `Session.stream()` returning
prematurely when background subagents are still running, by holding back
intermediate result messages until all tasks complete" — i.e. the parent waits
for outstanding background subagents before its terminal result.

### 5.2 Fan-out limit

**No documented numeric fan-out limit.** The docs warn about *cost*, not a cap:

> **(Warning)** When subagents complete, their results return to your main conversation. Running many subagents that each return detailed results can consume significant context.

> For tasks that need sustained parallelism or exceed your context window, [agent teams](https://code.claude.com/docs/en/agent-teams) give each worker its own independent context.

So the practical ceiling is the **parent context window** (every subagent's
final text lands back in it), not a documented `maxConcurrentSubagents`-style
knob. `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1` disables background concurrency
entirely (forces synchronous spawns). For true unbounded parallelism with
isolated contexts, the docs point to **agent teams** (experimental,
`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`) — out of scope for this corpus, but
noted as the SDK's own answer to "fan-out beyond one context."

> **For Ark:** parallel reviewer/verifier fan-out within one parent is possible
> but (a) non-deterministic in degree and (b) capped by the parent context.
> Host-orchestrated parallel `query()` calls (topic 09 territory) give
> deterministic degree and per-worker isolated contexts — the better fit for a
> substrate dispatching N sibling roles.

---

## 6. Context isolation — fresh window, narrow inheritance

> A subagent's context window starts fresh (no parent conversation) but isn't empty. The only channel from parent to subagent is the Agent tool's prompt string, so include any file paths, error messages, or decisions the subagent needs directly in that prompt.

The doc's inheritance table, **verbatim**:

| The subagent **receives**                                                    | The subagent **does not receive**                                  |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Its own system prompt (`AgentDefinition.prompt`) and the Agent tool's prompt | The parent's conversation history or tool results                  |
| Project CLAUDE.md (loaded via `settingSources`)                              | Preloaded skill content, unless listed in `AgentDefinition.skills` |
| Tool definitions (inherited from parent, or the subset in `tools`)           | The parent's system prompt                                         |

The Claude Code doc's "What loads at startup" expands the inheritance list for
a **non-fork** subagent:

- **System prompt:** the agent's own prompt + environment details Claude Code
  appends — **not** the full Claude Code system prompt.
- **Task message:** "the delegation prompt Claude writes when it hands off the
  work" — i.e. the `Agent` tool's `prompt` string (the only parent→child channel).
- **CLAUDE.md and memory:** "every level of the memory hierarchy the main
  conversation loads, including `~/.claude/CLAUDE.md`, project rules,
  `CLAUDE.local.md`, and managed policy files." (Built-in Explore and Plan agents
  skip this; custom agents do not.)
- **Git status:** "a snapshot taken at the start of the parent session." Absent
  when not a git repo or when `includeGitInstructions` is `false`.
- **Preloaded skills:** full content of any skill in `AgentDefinition.skills`.

### 6.1 cwd inheritance

> A subagent starts in the main conversation's current working directory. Within a subagent, `cd` commands do not persist between Bash or PowerShell tool calls and do not affect the main conversation's working directory.

So the subagent **inherits the parent's cwd** (unless `isolation: worktree`
gives it an isolated checkout — frontmatter only). `cd` inside the subagent is
non-persistent.

### 6.2 Can the parent pass specific context in?

**Yes — but only as the `Agent` tool `prompt` string.** "The only channel from
parent to subagent is the Agent tool's prompt string." The parent *model*
authors that prompt (the host does not, in the normal flow). So host code
cannot directly inject a structured context payload into a subagent — it can
only (a) put data in files/CLAUDE.md the subagent will load, or (b) steer the
parent prompt so the model writes the needed context into the delegation prompt.

> **For Ark:** because the only inbound channel is a model-authored prompt
> string, a substrate that needs to hand a subagent *exact* inputs (a task
> spec, a file manifest) is better served by writing those inputs to disk
> (which the subagent loads deterministically) or by running the role as a
> separate `query()` whose prompt the host controls verbatim.

### 6.3 Forked subagents (experimental) — the isolation exception

`CLAUDE_CODE_FORK_SUBAGENT=1` enables forks, which **invert** isolation: a fork
"inherits the entire conversation so far instead of starting fresh … sees the
same system prompt, tools, model, and message history as the main session." A
fork's own tool calls still stay out of the parent; only its final result
returns. Forks are experimental (v2.1.117+), honored "in interactive mode and
via the SDK or `claude -p`," and **cannot spawn further forks** (§4).

---

## 7. Tool / model / permission override per subagent

**Yes — fully per-subagent, independent of the parent.** Every capability knob
on `AgentDefinition` is an override:

- **Model:** `model` accepts an alias (`'sonnet'`, `'opus'`, `'haiku'`),
  `'inherit'`, or a full ID. "Model override for this agent. … Defaults to main
  model if omitted." The doc's "dynamic agent configuration" example explicitly
  uses a cheaper/more-expensive model per strictness:
  > "use a more capable model for high-stakes reviews … `model="opus" if is_strict else "sonnet"`"
  Resolution order (filesystem path, from the Claude Code doc):
  `CLAUDE_CODE_SUBAGENT_MODEL` env → per-invocation `model` param → definition's
  `model` frontmatter → main conversation's model.
- **Tools:** `tools` (allowlist) and/or `disallowedTools` (denylist). "If both
  are set, `disallowedTools` is applied first, then `tools` is resolved against
  the remaining pool." Omitting `tools` ⇒ inherit all. (Cross-ref topic 05 for
  the general tool-restriction mechanism — `AgentDefinition.tools` is the same
  allowlist concept scoped to one subagent.)
- **Permission mode:** `permissionMode` per agent — BUT the parent can override
  the child: "If the parent uses `bypassPermissions` or `acceptEdits`, this takes
  precedence and cannot be overridden. If the parent uses auto mode, the subagent
  inherits auto mode and any `permissionMode` in its frontmatter is ignored."
- **Effort / turns:** `effort` and `maxTurns` per agent.
- **MCP servers:** `mcpServers` per agent — inline servers connect when the
  subagent starts and disconnect when it finishes; string references share the
  parent's connection. (Cross-ref topic 07.) This lets a subagent see MCP tools
  the parent does not, keeping their tool descriptions out of the parent context.
- **Skills / memory:** `skills` (preload) and `memory` (persistent dir) per agent.

The canonical use case — **a cheaper model for review** — is directly
supported: set `model: "haiku"` (or `"sonnet"`) on the reviewer
`AgentDefinition` while the parent runs Opus. Likewise a read-only reviewer:
`tools: ["Read", "Grep", "Glob"]`.

---

## 8. Subagent vs separate `query()` — the design-relevant comparison

This is the architecture fork for a workflow substrate that dispatches
reviewer/verifier roles and may need recursive sub-tasks.

| Dimension                     | SDK subagent (`Agent` tool)                                              | Separate `query()` / `--agent` session                                          |
| ----------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| **Dispatch determinism**      | ❌ model decides (prompt-steered); host cannot force "run X now"          | ✅ host calls `query(...)` for exactly the role it wants (`--agent`/`agent` setting pins the whole session as agent X) |
| **Result return shape**       | ❌ free-form **text** as the `Agent` tool result; no structured channel ([#104](https://github.com/anthropics/claude-agent-sdk-typescript/issues/104)) | ✅ full `ResultMessage`, incl. `structured_output` (top-level `outputFormat`/`json_schema` works) |
| **Result ergonomics**         | ✅ auto-injected into parent conversation (no host plumbing) — but text only | ❌ host must read `ResultMessage` and route it (more wiring)                     |
| **Recursion**                 | ❌ depth 1 — subagents can't spawn subagents (§4)                          | ✅ host orchestrates arbitrary depth; each level is its own `query()`            |
| **Concurrency**               | ✅ model can fan out; ❌ degree non-deterministic, capped by parent context | ✅ host fans out N `query()` calls deterministically, each isolated context (topic 09) |
| **Context isolation**         | ✅ fresh window, results bloat back into parent on completion             | ✅ fully isolated; parent never sees child context unless host forwards it       |
| **Context cost**              | ❌ every subagent final text lands in parent context (the §5.2 warning)   | ✅ child context never touches parent unless host chooses                        |
| **Observability**             | ⚠ stream-tagged via `parent_tool_use_id` (topic 03); hooks lack subagent attribution per [py#272](https://github.com/anthropics/claude-agent-sdk-python/issues/272) | ✅ each session has its own `session_id`, JSONL, full event stream, cost totals  |
| **Cost accounting**           | ⚠ folded into parent `ResultMessage.total_cost_usd` (no per-subagent total surfaced directly) | ✅ per-`query()` `ResultMessage.total_cost_usd` / `modelUsage` (topic 08)         |
| **Setup cost / latency**      | ✅ lightweight; forks even share the parent prompt cache                  | ❌ each `query()` pays init/handshake (mitigated by `startup()`/`WarmQuery` in TS) |
| **Per-role model/tools**      | ✅ `AgentDefinition` overrides                                            | ✅ per-`query()` `options`                                                       |

### 8.1 When to use which

**Use an SDK subagent when:** the subtask is a self-contained, model-judged
*leaf* (e.g. "explore the codebase and summarize"), you want its verbose
intermediate work kept out of the parent context, and a free-form text summary
back is acceptable. Cheapest for context isolation; zero result-plumbing.

**Use a separate `query()` when:** the host must (a) **deterministically**
dispatch a specific role, (b) read a **structured/parseable** result, (c)
**recurse** (sub-tasks of sub-tasks), or (d) get **per-role cost/observability**.
This is the load-bearing primitive for a workflow substrate's spine —
reviewer/verifier dispatch and recursive task trees both want determinism +
structured return + recursion, none of which the subagent path provides.

### 8.2 The synthesis-relevant verdict (facts only)

The corpus reader deciding the ArkOS architecture fork now has the facts:

- A substrate dispatching **named reviewer/verifier roles deterministically**
  ⇒ separate `query()` (or `--agent`), not subagents. The host owns "run agent
  X now."
- A substrate needing **recursive task trees** ⇒ host-orchestrated recursion of
  separate `query()` calls; SDK subagents give exactly one level.
- A substrate needing **structured results** from sub-work ⇒ separate `query()`
  with `outputFormat`, OR the file-write side-channel (§3.3); subagents return
  text only.
- SDK subagents remain a **good fit for leaf fan-out** (parallel read-only
  analysis whose summary is fine as text) — a pleasant default, not the spine.

(No design choice is made here — this is the comparison the SYNTHESIS file and
the ArkOS architecture task will weigh.)

---

## External references

- [Subagents in the SDK (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/subagents) — primary: `agents` option, `AgentDefinition` field table, programmatic vs filesystem vs general-purpose, automatic/explicit invocation, "What subagents inherit," "result … verbatim as the Agent tool result," recursion `<Note>`, parallelization, dynamic agent config, resume, tool restrictions, troubleshooting.
- [Create custom subagents (code.claude.com/sub-agents)](https://code.claude.com/docs/en/sub-agents) — filesystem frontmatter fields (incl. `hooks`, `isolation`, `color`), scope precedence table, discovery (recursive, name-keyed), `--agent` whole-session mode, model resolution order, "What loads at startup," fork mode, "Subagents cannot spawn other subagents" (×3).
- [TypeScript SDK reference (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/typescript) — `AgentDefinition` TS type, `Options.agents`, `Options.forwardSubagentText` (default false), `Query.supportedAgents()`.
- [Python SDK reference (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/python) — `AgentDefinition` Python dataclass.
- [`types.py` on main (anthropics/claude-agent-sdk-python)](https://raw.githubusercontent.com/anthropics/claude-agent-sdk-python/refs/heads/main/src/claude_agent_sdk/types.py) — `AgentDefinition` verbatim; `ClaudeAgentOptions.agents`; confirmed **no `forward_subagent_text`** field (visited 2026-05-25).
- [TS issue #104 — outputFormat for Task tool / subagents (OPEN)](https://github.com/anthropics/claude-agent-sdk-typescript/issues/104) — confirms structured output works only top-level, not for subagents; context-bloat motivation; file-write workaround.
- [Python issue #627 — Subagents appear to stop with no reason (OPEN)](https://github.com/anthropics/claude-agent-sdk-python/issues/627) — background subagents emitting `SubagentStop` early; relevant reliability caveat for background fan-out.
- [Python issue #272 — hooks lack `parent_tool_use_id` context (cited via topic 03)](https://github.com/anthropics/claude-agent-sdk-python/issues/272) — hook-level subagent attribution gap.
- TS CHANGELOG (anthropics/claude-agent-sdk-typescript) — `forwardSubagentText` added 0.2.119; `AgentToolInput.subagent_type` made optional (defaults `general-purpose`); `Task`→`Agent` rename context; `supportedAgents()` added; `task_started` system message.
- [Agent teams (code.claude.com)](https://code.claude.com/docs/en/agent-teams) — SDK's answer to sustained parallelism beyond one context (experimental, `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`); referenced, not researched in depth.

---

## Caveats / Not found

- **`forward_subagent_text` (Python) does NOT exist** in 0.2.87 (`types.py` grep
  + repo code search both empty). Topic 03 §8.3's snake_case Python reference is
  inaccurate for the current SDK — the field is **TS-only** (`Options.forwardSubagentText`,
  added 0.2.119). This file (§3.4) records the correction. Re-verify if Python
  parity lands in a later release.
- **`AgentToolInput` exact field schema** is not on the TS public reference page
  (the page returned "not found" for it). Field names (`subagent_type` optional →
  defaults `general-purpose`; `prompt`; `description`) are reconstructed from the
  TS CHANGELOG and the doc's detection snippets (`block.input.subagent_type`).
  Treat the precise shape as inferred, not contractually published.
- **No documented numeric fan-out / concurrency limit** for parallel subagents.
  The practical ceiling is the parent context window (every subagent's final
  text returns into it) plus `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS`. If a hard
  cap exists internally, it is undocumented.
- **"Explicit invocation" is not hard-deterministic.** Naming the agent in the
  prompt is the documented "guarantee," but it is still mediated by the model's
  tool-call decision (confirmed by the Troubleshooting section). The only truly
  deterministic agent dispatch is running a whole *session* as the agent
  (`--agent` / `agent` setting), which is a separate `query()`, not a subagent.
- **Per-subagent cost is not surfaced as a distinct field.** Subagent token/cost
  folds into the parent `ResultMessage.total_cost_usd` / `modelUsage`. There is
  no per-`Agent`-call cost line in the SDK result (topic 08 may revisit). To get
  per-role cost cleanly, run the role as a separate `query()`.
- **Reliability caveat for background subagents:** [py#627](https://github.com/anthropics/claude-agent-sdk-python/issues/627)
  (open) reports background subagents emitting `SubagentStop` prematurely with
  no clear reason and context not near full. A substrate relying on background
  fan-out should treat early-stop as a real failure mode to detect and retry.
- **Resume requires same session + same agent definition.** Resuming a subagent
  needs `resume: sessionId` AND (for custom agents) re-passing the same `agents`
  definition; the SDK doc's resume flow parses `agentId` out of message content
  via regex (`/agentId:\s*([a-f0-9-]+)/`) — itself a string-scrape, reinforcing
  that subagent identity/return are not first-class typed data on the parent
  surface. `SendMessage`-based resume needs `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`.
- **`isolation: worktree`, `hooks`, `color` are filesystem/`--agents`-JSON only**
  — no `AgentDefinition` programmatic equivalents. A purely programmatic
  substrate loses per-subagent worktree isolation and per-subagent frontmatter
  hooks unless it uses filesystem agent cards.
- **Doc snapshot only:** `code.claude.com` pages do not print per-page "last
  updated" dates; version pin is via package versions and CHANGELOG landmarks as
  of 2026-05-25 (Python 0.2.87 / TS 0.3.150). The `forwardSubagentText` default
  (`false`) and `AgentDefinition` TS fields are from the reference page at that
  snapshot.
