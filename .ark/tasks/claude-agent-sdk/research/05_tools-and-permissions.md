# Research: Claude Agent SDK — tool inventory and permission policy

- Query: built-in tool inventory; enable/disable tools per session (`allowedTools`/`disallowedTools`); permission modes (`permissionMode`/`PermissionMode`); the `--dangerously-skip-permissions` equivalent; scoped Bash/Read permissions; custom in-process tools; tool-result truncation; sandbox; precedence vs hooks.
- Scope: external (primary docs `code.claude.com`; SDK identifier names cross-checked against neighbor topics 01/02/03 which read the SDK source).
- Date: 2026-05-25
- Doc snapshot: `code.claude.com/docs/en/agent-sdk/*` + `code.claude.com/docs/en/{permissions,settings,sandboxing,permission-modes}` fetched 2026-05-25.
- SDK versions pinned: Python `claude-agent-sdk` **0.2.87** (PyPI, released 2026-05-23); TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.150** (npm, current 2026-05-23). If a newer release changes any literal below, re-verify against that version.

Why this matters (one line): a substrate that wants to confine an autonomous phase to a safe tool set and enforce "no writes outside the worktree" must know which of four mechanisms — tool-availability allowlist, permission mode, scoped permission rule, or hook — actually carries each guarantee. §9 and the closing table answer that directly.

Boundaries with neighbor files:
- **Topic 04 (`04_hooks.md`)** owns the deep hook internals (signatures, return shapes, `PreToolUse` body). This file references hooks only at the precedence boundary (§9) and cites the documented evaluation order.
- **Topic 06 (`06_subagents.md`)** owns `AgentDefinition` mechanics; this file notes only the permission-inheritance rule for subagents (§3).
- **Topic 07 (`07_mcp-integration.md`)** owns MCP-server *publishing* / external transports; this file covers only defining one in-process custom tool the agent can call (§6).

---

## 1. Built-in tool inventory

The SDK ships the same built-in tools that power Claude Code. The canonical
table is on the agent-loop page ("Built-in tools"), grouped by category. The
overview page's "Built-in tools" tab lists a "key tools" subset with one-line
descriptions; the two are consistent. Tool names are **case-sensitive bare
identifiers** (`Read`, not `read`) — the same strings you pass to
`allowed_tools`/`allowedTools`.

Verbatim from the agent-loop "Built-in tools" table (`code.claude.com/docs/en/agent-sdk/agent-loop`):

| Category            | Tools                                                            | What they do                                                                |
| :------------------ | :-------------------------------------------------------------- | :-------------------------------------------------------------------------- |
| **File operations** | `Read`, `Edit`, `Write`                                          | Read, modify, and create files                                              |
| **Search**          | `Glob`, `Grep`                                                   | Find files by pattern, search content with regex                            |
| **Execution**       | `Bash`                                                           | Run shell commands, scripts, git operations                                 |
| **Web**             | `WebSearch`, `WebFetch`                                          | Search the web, fetch and parse pages                                       |
| **Discovery**       | `ToolSearch`                                                     | Dynamically find and load tools on-demand instead of preloading all of them |
| **Orchestration**   | `Agent`, `Skill`, `AskUserQuestion`, `TaskCreate`, `TaskUpdate` | Spawn subagents, invoke skills, ask the user, track tasks                   |

Per-tool one-liners (merging the overview "key tools" tab with the agent-loop categories — verbatim where the doc gives a description):

| Tool              | Verbatim / paraphrased description                                                                      | Source |
| :---------------- | :----------------------------------------------------------------------------------------------------- | :----- |
| `Read`            | "Read any file in the working directory"                                                               | overview |
| `Write`           | "Create new files"                                                                                     | overview |
| `Edit`            | "Make precise edits to existing files"                                                                 | overview |
| `Bash`            | "Run terminal commands, scripts, git operations"                                                       | overview |
| `Monitor`         | "Watch a background script and react to each output line as an event"                                  | overview |
| `Glob`            | "Find files by pattern (`**/*.ts`, `src/**/*.py`)"                                                     | overview |
| `Grep`            | "Search file contents with regex"                                                                      | overview |
| `WebSearch`       | "Search the web for current information"                                                               | overview |
| `WebFetch`        | "Fetch and parse web page content"                                                                     | overview |
| `AskUserQuestion` | "Ask the user clarifying questions with multiple choice options" (triggers `canUseTool`; see §6/§4)    | overview |
| `ToolSearch`      | Dynamically find and load tools on-demand instead of preloading all of them                            | agent-loop |
| `Agent`           | Spawn subagents (the tool a parent calls to dispatch a subagent — see topic 06)                        | agent-loop |
| `Skill`           | Invoke a `.claude/skills/*/SKILL.md` skill (auto-enabled when the `skills` option is set — see topic 02) | agent-loop |
| `TaskCreate`      | Create a tracked task (todo-style orchestration)                                                       | agent-loop |
| `TaskUpdate`      | Update a tracked task                                                                                  | agent-loop |

**Naming notes (verified against the corpus):**
- **`Monitor`** appears in the overview "key tools" tab but is **not** in the agent-loop category table. Topic 01 lists `Monitor` in the inventory; topic 03 notes the tool rename `Task → Agent` landed at CLI v2.1.63. Treat `Monitor` as a real built-in (overview is authoritative) that the agent-loop table simply omits from its grouped view.
- **`Agent` is the current name; `Task` is the old name.** Topic 03: "tool rename 'Task' → 'Agent' landed at v2.1.63." Older code/docs that say `Task` in `allowedTools` refer to the same tool. The subagents tab in the overview confirms: "Subagents are invoked via the Agent tool, so include `Agent` in `allowedTools` to auto-approve those invocations."

**Tools NOT found in this SDK's built-in inventory (explicit negatives):**
- **`MultiEdit`** — **NOT FOUND** in either the overview or agent-loop tables for the Agent SDK at this snapshot. Claude Code historically had a `MultiEdit` tool; it is not enumerated in the Agent SDK's documented built-in inventory at 0.2.87 / 0.3.150. The documented file-edit surface is `Edit` + `Write`. Do not assume `MultiEdit` is available; verify empirically if needed.
- **`TodoWrite`** — **NOT FOUND** as a named built-in in the Agent SDK tables. The orchestration/task-tracking surface here is `TaskCreate` / `TaskUpdate`, not `TodoWrite`. `TodoWrite` is a Claude Code CLI tool name; the SDK's documented equivalent is the `TaskCreate`/`TaskUpdate` pair. Treat `TodoWrite` as **undocumented for the SDK** at this snapshot.
- **`NotebookEdit`** — **NOT FOUND** in the Agent SDK built-in tables. Not enumerated; treat as undocumented for the SDK.
- **`KillShell` / `BashOutput`** (background-shell management seen in some CLI builds) — not in the SDK tables; the SDK's background-script surface is `Monitor`.

→ The PRD asked to "verify the actual current set." The verified current documented set is the 15 tools in the category table **plus** `Monitor`. `MultiEdit`, `TodoWrite`, and `NotebookEdit` are **not** in the documented SDK inventory at this snapshot.

**Default availability:** at a fresh session start, the **full** built-in inventory is *available* by default (topic 02 §1: "the full built-in tool inventory is *available* by default"). `allowed_tools`/`allowedTools` does not gate availability — it only auto-approves a subset (see §2). To actually *remove* a built-in from the model's context, use the `tools` availability option or a bare-name `disallowed_tools` entry (§2, §6).

---

## 2. Enabling / disabling tools per session

Three distinct options interact (agent-loop "Tool permissions"; custom-tools
"Configure allowed tools"). Critically, two of them change **availability**
(whether the tool appears in the model's context) and the rest change
**permission** (whether a call is auto-approved once attempted).

| Option (Python / TS)                          | Layer        | Effect |
| :-------------------------------------------- | :----------- | :----- |
| `tools` (Python `tools` / TS `tools`)         | Availability | Only the listed built-ins are in Claude's context; unlisted built-ins are removed. `tools: []` removes all built-ins (MCP tools unaffected). |
| `allowed_tools` / `allowedTools`              | Permission   | Listed tools run **without a permission prompt** (auto-approved). Unlisted tools are still available and fall through to the permission flow. |
| `disallowed_tools` / `disallowedTools`        | Both         | A **bare** name (`"Bash"`) removes the tool from context entirely. A **scoped** rule (`"Bash(rm *)"`) leaves the tool visible but denies matching calls — in *every* mode, including `bypassPermissions`. |

Verbatim from the custom-tools page ("Configure allowed tools" table):

> | `tools: ["Read", "Grep"]` | Availability | Only the listed built-ins are in Claude's context. Unlisted built-ins are removed. MCP tools are unaffected. |
> | `tools: []` | Availability | All built-ins are removed. Claude can only use your MCP tools. |
> | allowed tools | Permission | Listed tools run without a permission prompt. Unlisted tools remain available; calls go through the permission flow. |
> | disallowed tools | Both | A bare tool name such as `"Bash"` removes the tool from Claude's context… A scoped rule such as `"Bash(rm *)"` leaves the tool in context and denies only matching calls. |

Verbatim from the permissions page ("Allow and deny rules" table):

> | `allowed_tools=["Read", "Grep"]`  | `Read` and `Grep` are auto-approved. Tools not listed here still exist and fall through to the permission mode and `canUseTool`. |
> | `disallowed_tools=["Bash"]`       | The `Bash` tool definition is removed from the request. Claude does not see the tool and cannot attempt it. |
> | `disallowed_tools=["Bash(rm *)"]` | `Bash` stays available. Calls matching `rm *` are denied in every permission mode, including `bypassPermissions`. Other `Bash` calls fall through to the permission mode. |

**Value format:** plain tool-name strings (`"Read"`), OR scoped rules of the
form `Tool(specifier)` (`"Bash(npm run *)"`, `"Read(./src/**)"`), OR MCP tool
names (`"mcp__weather__get_temperature"`, wildcard `"mcp__weather__*"`). The
scoped-rule grammar is detailed in §5.

**The two confining strategies (load-bearing for ArkOS):**

1. **Restrict to a safe set AND deny everything else** = `allowedTools` (the safe
   set) **+** `permissionMode: "dontAsk"`. Verbatim (permissions page):
   > For a locked-down agent, pair `allowedTools` with `permissionMode: "dontAsk"`. Listed tools are approved; anything else is denied outright instead of prompting.
2. **Restrict what the model can even *see*** = the `tools` availability option
   (or bare-name `disallowedTools`). This keeps the model from wasting a turn
   attempting a tool it cannot use.

**Snippet — read-only agent (Read + Grep only).** The overview's own
read-only example uses `["Read", "Glob", "Grep"]`; the PRD asked for Read+Grep.
Both forms shown; the second is the hardened form.

Python:

```python
from claude_agent_sdk import query, ClaudeAgentOptions

# Soft form: Read+Grep auto-approved; other tools still exist and fall through
# to the permission mode / canUseTool.
options = ClaudeAgentOptions(allowed_tools=["Read", "Grep"])

# Hardened form: only Read+Grep are even visible, and anything unlisted is a
# hard deny (no prompt, no canUseTool).
options = ClaudeAgentOptions(
    tools=["Read", "Grep"],            # availability: nothing else in context
    allowed_tools=["Read", "Grep"],    # permission: run without prompting
    permission_mode="dontAsk",         # deny-instead-of-prompt for anything else
)
```

TypeScript:

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

const soft = { allowedTools: ["Read", "Grep"] };

const hardened = {
  tools: ["Read", "Grep"],
  allowedTools: ["Read", "Grep"],
  permissionMode: "dontAsk" as const,
};
```

Note: `allowed_tools` **does not** make an agent read-only on its own — an
unlisted `Write`/`Bash` still falls through to the permission mode, and under
`bypassPermissions` would run. Read-only requires either `tools`/bare-name
denial of write tools, or `permissionMode: "dontAsk"`/`"plan"`, or deny rules.

---

## 3. Permission modes

The mode option is `permission_mode` (Python) / `permissionMode` (TypeScript).
The Python type alias is `PermissionMode` (topic 01 §5):

```
# Python (from topic 01, read against src/claude_agent_sdk/types.py)
PermissionMode = "default" | "acceptEdits" | "plan" | "dontAsk" | "bypassPermissions"

# TypeScript (topic 01 §5) — adds "auto"
PermissionMode = "default" | "acceptEdits" | "bypassPermissions" | "plan" | "dontAsk" | "auto"
```

**`"auto"` is TypeScript-only.** The permissions page tags it "(TypeScript
only)"; the agent-loop mode table tags it "(TypeScript only)". The PRD's note
from topic 01 ("TS-only `auto`") is **confirmed**. Python's `PermissionMode`
alias does not include `"auto"` at 0.2.87.

What each literal does to tool-call gating (verbatim from the permissions page
"Available modes" table, reconciled with the agent-loop and mode-details text):

| Mode (literal)             | What it does to tool-call gating |
| :------------------------- | :------------------------------- |
| `"default"`                | "No auto-approvals; unmatched tools trigger your `canUseTool` callback." No callback ⇒ deny. |
| `"acceptEdits"`            | "File edits and filesystem operations (`mkdir`, `rm`, `mv`, etc.) are automatically approved." Auto-approves `Edit`/`Write` and the filesystem-mutating Bash commands `mkdir, touch, rm, rmdir, mv, cp, sed` — **but only for paths inside the working directory or `additionalDirectories`**; outside-scope and protected-path writes still prompt. Other (non-filesystem) Bash still follows default rules. |
| `"plan"`                   | "Read-only tools run; Claude analyzes and plans without editing your source files." Claude may use `AskUserQuestion` to clarify before finalizing the plan. |
| `"dontAsk"`                | "Anything not pre-approved by `allowed_tools` or rules is denied; `canUseTool` is never called." Converts every would-be prompt into a denial. |
| `"bypassPermissions"`      | "All tools run without permission prompts (use with caution)." See §4. |
| `"auto"` (TS only)         | "A model classifier approves or denies each tool call." Background safety checks verify actions align with the request. Availability is gated — see [Auto mode](https://code.claude.com/docs/en/permission-modes#eliminate-prompts-with-auto-mode). |

**Set at query time or mid-session:**
- At query time: `permission_mode=` / `permissionMode:` in options.
- Mid-session: Python `await client.set_permission_mode("acceptEdits")` on
  `ClaudeSDKClient`; TS `await q.setPermissionMode("acceptEdits")` on the
  `Query` object. "The new mode takes effect immediately for all subsequent
  tool requests." (Lets a host start restrictive and loosen as trust builds.)

**Subagent inheritance (load-bearing — confine carefully).** Verbatim
`<Warning>` from the permissions page:

> **Subagent inheritance:** When the parent uses `bypassPermissions`, `acceptEdits`, or `auto`, all subagents inherit that mode and it cannot be overridden per subagent. Subagents may have different system prompts and less constrained behavior than your main agent, so inheriting `bypassPermissions` grants them full, autonomous system access without any approval prompts.

→ For ArkOS: a parent in `bypassPermissions` cannot hand a *more* restricted mode
to a subagent. If you need subagents confined, the parent must not be in
`bypassPermissions`/`acceptEdits`/`auto`; gate at the rule/hook layer instead.

---

## 4. The `--dangerously-skip-permissions` equivalent

**Yes — `permission_mode="bypassPermissions"` / `permissionMode:
"bypassPermissions"` is the SDK equivalent of the CLI's
`--dangerously-skip-permissions`.** This is confirmed both by the SDK docs
("Bypass all permission checks … All tools run without permission prompts") and
by the broader Claude Code docs that describe `--dangerously-skip-permissions`
as the CLI surface for bypass mode.

But the mapping is **not byte-identical** — the docs draw a deliberate line
between the SDK mode and the raw CLI flag (sandboxing page "Permission modes"
comparison table, verbatim):

> | `bypassPermissions` (mode) | … `bypassPermissions` mode skips all permission prompts. Removals targeting the filesystem root or home directory, such as `rm -rf /` and `rm -rf ~`, **still prompt as a circuit breaker** against model error. |
> | `--dangerously-skip-permissions` (CLI flag) | … "Nothing. Protected path checks are **also skipped**; only removing `/` or your home directory still prompts." |

So `bypassPermissions` keeps a few protected-path / `.git` / `.claude` circuit
breakers that the raw CLI flag relaxes further. For an embedded SDK substrate
the relevant value is `bypassPermissions`.

**Safety statements the docs make (quote, don't paraphrase):**

Permissions page `<Warning>`:
> Use with extreme caution. Claude has full system access in this mode. Only use in controlled environments where you trust all possible operations.
> `allowed_tools` does not constrain this mode. Every tool is approved, not just the ones you listed. Deny rules (`disallowed_tools`), explicit `ask` rules, and hooks are evaluated before the mode check and can still block a tool.

Agent-loop mode table:
> Runs all allowed tools without asking. **Cannot be used when running as root on Unix.** Use only in isolated environments where the agent's actions cannot affect systems you care about.

Underlying-CLI note (sandboxing troubleshooting): `--dangerously-skip-permissions`
"is blocked when running as root or via sudo on Linux and macOS… The check is
skipped automatically inside a recognized sandbox." Administrators can hard-disable
bypass via managed setting `permissions.disableBypassPermissionsMode: "disable"`.

**What still blocks under `bypassPermissions`** (so it is not a total escape hatch):
1. **Deny rules** (`disallowed_tools` scoped + bare) — evaluated *before* the mode (§5, §9).
2. **`ask` rules** — still prompt.
3. **Hooks** — "Hooks still execute and can block operations if needed." (§9; topic 04 owns hook internals.)
4. **Root circuit breaker** + `rm -rf /` / `rm -rf ~` prompt.

→ For ArkOS: even in `bypassPermissions` you can still enforce "never delete X"
or "never touch path Y" via `disallowed_tools` scoped rules and/or a
`PreToolUse` hook. `bypassPermissions` removes the *prompt*, not the *deny layer*.

---

## 5. Scoped permissions (Bash, Read, Edit, WebFetch, MCP, Agent)

**Yes — `Bash(cargo:*)`-style scoping works, and it applies to several tools,
not just Bash.** The grammar is the Claude Code permission-rule syntax
(`code.claude.com/docs/en/permissions` "Permission rule syntax"), shared by the
SDK's `allowed_tools`/`disallowed_tools` and by `.claude/settings.json`
`allow`/`ask`/`deny` lists.

**Rule format:** `Tool` (bare; matches all uses) or `Tool(specifier)` (scoped).

### Bash scoping — exact syntax and the two wildcard forms

Verbatim (permissions page "Wildcard patterns" + "Bash"):

> | `Bash(npm run build)` | Matches the exact command `npm run build` |
> | `Bash(npm run test *)` | Matches Bash commands starting with `npm run test` |
> | `Bash(npm *)` | Matches any command starting with `npm ` |
> | `Bash(* install)` | Matches any command ending with ` install` |
> | `Bash(git * main)` | Matches commands like `git checkout main` … |

**The `:*` vs ` *` question (the PRD's `Bash(cargo:*)` form) — resolved:**

> The `:*` suffix is an equivalent way to write a trailing wildcard, so `Bash(ls:*)` matches the same commands as `Bash(ls *)`. … The `:*` form is **only recognized at the end of a pattern**. In a pattern like `Bash(git:* push)`, the colon is treated as a literal character and won't match git commands.

So: **`Bash(cargo:*)` ≡ `Bash(cargo *)`** (allow anything starting `cargo `),
and **`Bash(rm:*)` ≡ `Bash(rm *)`** (deny anything starting `rm `). Topic 01's
`Bash(git:*)` example is valid as a *trailing* wildcard. To deny `rm`:
`disallowed_tools=["Bash(rm *)"]` or equivalently `["Bash(rm:*)"]`.

Word-boundary nuance (verbatim): "`Bash(ls *)` matches `ls -la` but not `lsof`,
while `Bash(ls*)` matches both." The space before `*` (and the `:*` form)
enforces a word boundary.

**Snippet — allow `cargo *`, deny `rm *` (the PRD's exact ask):**

```python
options = ClaudeAgentOptions(
    allowed_tools=["Bash(cargo:*)"],   # auto-approve any `cargo …`
    disallowed_tools=["Bash(rm:*)"],   # hard-deny any `rm …`, even under bypass
)
```

```typescript
const options = {
  allowedTools: ["Bash(cargo:*)"],
  disallowedTools: ["Bash(rm:*)"],
};
```

**Bash scoping is fragile — the docs say so explicitly.** Quote the
`<Warning>` so ArkOS does not over-trust prefix rules:

> Bash permission patterns that try to constrain command arguments are fragile. For example, `Bash(curl http://github.com/ *)` … won't match variations like: Options before URL `curl -X GET …`; Different protocol `curl https://…`; Redirects `curl -L http://bit.ly/xyz`; Variables `URL=… && curl $URL`; Extra spaces `curl  http://…`.

Mitigations the docs recommend: deny network Bash tools (`curl`,`wget`) + use
`WebFetch(domain:…)`; a `PreToolUse` hook; or OS-level sandboxing (§8).

Two more Bash subtleties that affect rule reliability:
- **Compound commands** are split on `&& || ; | |& &` and newlines; *every*
  subcommand must match a rule. `Bash(safe-cmd *)` does **not** authorize
  `safe-cmd && other-cmd`.
- **Process-wrapper stripping:** `timeout, time, nice, nohup, stdbuf` (and bare
  `xargs`) are stripped before matching, so `Bash(npm test *)` also covers
  `timeout 30 npm test`. But env runners (`npx`, `docker exec`, `devbox run`,
  `mise exec`, `direnv exec`) are **not** stripped — `Bash(devbox run *)`
  authorizes `devbox run rm -rf .`. Write `Bash(devbox run npm test)` instead.
- **Read-only Bash commands** (`ls cat echo pwd head tail grep find wc which
  diff stat du cd` and read-only `git`) run **without a prompt in every mode**;
  to require a prompt add an explicit `ask`/`deny` rule.

### Read / Edit / Write scoping — yes, with gitignore-style paths

`Read(./src/**)` works. `Edit` rules apply to all built-in file-edit tools;
`Read` rules apply best-effort to `Read`, `Grep`, `Glob`, `@file` mentions, and
IDE context. The four anchor forms (verbatim "Read and Edit" table):

| Pattern            | Meaning                                | Example                          |
| :----------------- | :------------------------------------- | :------------------------------- |
| `//path`           | **Absolute** path from filesystem root | `Read(//Users/alice/secrets/**)` |
| `~/path`           | Path from **home** directory           | `Read(~/Documents/*.pdf)`        |
| `/path`            | Path **relative to project root**      | `Edit(/src/**/*.ts)`             |
| `path` or `./path` | Path **relative to current directory** | `Read(*.env)` ⇒ `<cwd>/*.env`    |

Critical gotcha (verbatim `<Warning>`): "A pattern like `/Users/alice/file` is
**NOT** an absolute path. It's relative to the project root. Use
`//Users/alice/file` for absolute paths."

`*` matches within one directory; `**` matches recursively. `Read(.env)` ≡
`Read(**/.env)` (bare filenames match at any depth). To allow all file access
use the bare tool name `Read` / `Edit` / `Write`.

**"No writes outside the worktree" — how to express it (directly answers the PRD):**
The closest *rule-layer* expression is an **`Edit` deny** anchored outside the
project plus an `Edit` allow inside, e.g. deny `Edit(//**)` is too broad to be
useful, so the practical shape is `acceptEdits` mode (which already restricts
auto-approved edits to "paths inside the working directory or
`additionalDirectories`") combined with deny rules for sensitive in-tree paths.
**But the docs are explicit that rules are not OS enforcement** (verbatim):

> Read and Edit deny rules apply to Claude's built-in file tools and to file commands Claude Code recognizes in Bash, such as `cat`, `head`, `tail`, and `sed`. They **do not apply to arbitrary subprocesses** that read or write files indirectly, like a Python or Node script that opens files itself. For OS-level enforcement that blocks all processes from accessing a path, enable the sandbox.

→ So "no writes outside the worktree" is **partly** a permission-rule concern
(built-in tools + recognized Bash file commands) and **fully** a sandbox concern
only at the OS level (§8). A subprocess that opens files itself escapes the rule
layer. This is the key finding for ArkOS worktree confinement.

### WebFetch, MCP, Agent scoping

- `WebFetch(domain:example.com)` — matches fetches to that domain. Bare
  `WebFetch` matches all.
- MCP: `mcp__puppeteer` (any tool from server `puppeteer`),
  `mcp__puppeteer__*` (wildcard, same effect), `mcp__puppeteer__puppeteer_navigate`
  (one tool). Custom in-process tools use the same `mcp__{server}__{tool}` form (§6).
- Agent (subagents): `Agent(Explore)`, `Agent(my-custom-agent)` — control which
  subagents the parent may invoke; add to `deny` to disable a specific agent.
- PowerShell rules mirror Bash (`PowerShell(Get-ChildItem *)`, `:*` suffix,
  alias canonicalization, case-insensitive).

---

## 6. Custom tools via in-process MCP (`tool` + `create_sdk_mcp_server`)

A custom tool the agent can call is defined with the `@tool` decorator
(Python) / `tool()` helper (TS), bundled into an **in-process** MCP server with
`create_sdk_mcp_server` / `createSdkMcpServer`, and registered via the
`mcp_servers` / `mcpServers` option. The handler runs **inside the caller's
process** — no subprocess, no shelling out. (External MCP servers and
publishing are topic 07.)

A tool is four parts (custom-tools page): **name**, **description**, **input
schema** (TS: a Zod schema; Python: a `{name: type}` dict or a full JSON Schema
dict), **handler** (async; returns `{content: [...], structuredContent?, isError?}`).

**Minimal Python snippet (verbatim from the custom-tools page, trimmed to the API shape):**

```python
from typing import Any
from claude_agent_sdk import tool, create_sdk_mcp_server, query, ClaudeAgentOptions

@tool("get_temperature", "Get the current temperature at a location",
      {"latitude": float, "longitude": float})
async def get_temperature(args: dict[str, Any]) -> dict[str, Any]:
    # ... do work ...
    return {"content": [{"type": "text", "text": "Temperature: 64F"}]}

weather_server = create_sdk_mcp_server(
    name="weather", version="1.0.0", tools=[get_temperature],
)

options = ClaudeAgentOptions(
    mcp_servers={"weather": weather_server},
    allowed_tools=["mcp__weather__get_temperature"],   # pre-approve the tool
)
```

**Minimal TypeScript snippet:**

```typescript
import { tool, createSdkMcpServer, query } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";

const getTemperature = tool(
  "get_temperature",
  "Get the current temperature at a location",
  { latitude: z.number(), longitude: z.number() },
  async (args) => ({ content: [{ type: "text", text: "Temperature: 64F" }] }),
);

const weatherServer = createSdkMcpServer({
  name: "weather", version: "1.0.0", tools: [getTemperature],
});

// options: { mcpServers: { weather: weatherServer },
//            allowedTools: ["mcp__weather__get_temperature"] }
```

**Tool-name format and allow:** the `mcpServers` key becomes the server segment:
`mcp__{server_name}__{tool_name}`. Pre-approve with that exact name in
`allowed_tools`, or wildcard a whole server with `mcp__weather__*`.

**Handler return shape (the load-bearing details):**
- `content` (required): array of blocks, each `type` ∈ `"text" | "image" | "resource"`.
- `structuredContent` (optional, **TS only for in-process**): machine-readable
  JSON. Verbatim Python caveat: "The Python `@tool` decorator forwards only
  `content` and `is_error` from the handler's return dict. To return
  `structuredContent` from Python, run a standalone MCP server instead." →
  Python/TS divergence: **in-process Python custom tools cannot return
  `structuredContent`**; TS can.
- `isError` (TS) / `is_error` (Python): set `true`/`True` to report a tool
  failure as data and **keep the loop alive**. Verbatim: an uncaught throw/exception
  "ends the whole `query()` call"; returning `isError` lets Claude react and retry.

**Parallelism annotation:** pass `readOnlyHint: true` (TS: fifth-arg
`{ annotations: { readOnlyHint: true } }`; Python: `annotations=ToolAnnotations(
readOnlyHint=True)`) so a side-effect-free custom tool can batch with other
read-only tools. Custom tools default to **sequential** execution. Other hints
(`destructiveHint`, `idempotentHint`, `openWorldHint`) are informational only —
"Annotations are metadata, not enforcement."

---

## 7. Tool result size / truncation

**No explicit byte/line cap on tool output is documented in the Agent SDK
pages read here.** The closest the docs come is the context-window discussion
(agent-loop "What consumes context"):

> Large tool outputs consume significant context. Reading a big file or running a command with verbose output can use thousands of tokens in a single turn. Context accumulates across turns…

There is no documented "tool output is truncated to N bytes/tokens before
feeding back to the model" rule on the agent-loop, custom-tools, permissions, or
overview pages. The SDK relies on **automatic compaction** (agent-loop
"Automatic compaction": summarizes older history when the window approaches its
limit; emits `system`/`compact_boundary`) to manage accumulated context rather
than per-result truncation.

- The `Read` tool itself takes `offset`/`limit` parameters (user-input "tool
  input fields" table: `Read` → `file_path, offset, limit`), i.e. truncation is
  **caller/model-controlled at read time**, not an automatic post-hoc clip.
- `ToolSearch` and MCP **tool-search** defer *tool-schema* loading (not result
  payloads) to save context — a different mechanism.

→ **For ArkOS: do not assume the SDK clips oversized tool results.** If a
substrate needs a hard output ceiling (e.g. to bound a single Bash result), it
must enforce that itself — via a custom-tool wrapper, a `PostToolUse` hook
(topic 04), or by reading in bounded chunks. **NOT FOUND / treat as undocumented:
a built-in tool-result size limit.** Re-verify against SDK source if a guarantee
is needed; the docs at this snapshot state none.

---

## 8. Sandbox

**The SDK does offer sandboxing for the Bash tool — it is the same sandbox that
ships in Claude Code, an OS-level isolation layer — but it is opt-in, Bash-only,
and platform-limited.** Source: `code.claude.com/docs/en/sandboxing` ("Configure
the sandboxed Bash tool").

**What it isolates** (verbatim "How sandboxing works"):
- **Filesystem:** default = read/write to the cwd and subdirectories, read
  access to the rest of the machine *except* denied dirs. "cannot modify files
  outside the current working directory without explicit permission." Note the
  default read policy "still allows reading credential files such as
  `~/.aws/credentials` and `~/.ssh/`" unless added to `denyRead`.
- **Network:** proxy outside the sandbox; "no domains are pre-allowed"; first
  use of a new domain prompts; pre-allow via `allowedDomains`.
- **Coverage:** "These OS-level restrictions ensure that all child processes
  spawned by Claude Code's commands inherit the same security boundaries." This
  is the key difference from permission rules — it catches subprocesses that open
  files themselves (§5).

**OS-level enforcement / platform support** (verbatim):
- macOS: Seatbelt (nothing to install).
- Linux: `bubblewrap` (+ `socat` for network relay).
- WSL2: bubblewrap. **WSL1 and native Windows are NOT supported.**

**How it is enabled / modes:**
- Settings, not an SDK option literal: `sandbox.enabled: true` in `settings.json`
  (user/project/managed scope). The `/sandbox` slash command writes
  `.claude/settings.local.json`.
- **Two sandbox modes** (verbatim "Sandbox modes"): **Auto-allow mode**
  (sandboxed commands run without prompting; non-sandboxable commands fall back
  to the permission flow) and **Regular permissions mode** (all Bash commands
  still go through the permission flow even when sandboxed).
- `autoAllowBashIfSandboxed: true` is the **default** (permissions page): "When
  sandboxing is enabled with `autoAllowBashIfSandboxed: true`, which is the
  default, sandboxed Bash commands run without prompting even if your
  permissions include `ask: Bash(*)`. The sandbox boundary substitutes for the
  per-command prompt. Explicit deny rules still apply, and `rm`/`rmdir`
  targeting `/`, home, or critical paths still trigger a prompt."
- Hardening keys: `failIfUnavailable: true` (hard-fail if sandbox can't start),
  `allowUnsandboxedCommands: false` (ignore the `dangerouslyDisableSandbox`
  escape hatch — "Strict sandbox mode"), `excludedCommands` (run named tools
  outside the sandbox), `sandbox.filesystem.{allowWrite,denyWrite,denyRead,allowRead}`,
  `sandbox.network.{allowedDomains,deniedDomains,httpProxyPort,socksProxyPort}`,
  managed-only `allowManagedReadPathsOnly` / `allowManagedDomainsOnly`.

**Scope limits (what the sandbox does NOT cover) — verbatim "Scope":**
- "**Built-in file tools**: Read, Edit, and Write use the permission system
  directly rather than running through the sandbox." → the sandbox isolates
  **Bash subprocesses only**; `Write`/`Edit` are governed by permission rules, not
  the sandbox.
- "**Subagents** run in the same process as the parent session and use the same
  sandbox configuration. Bash commands inside a subagent are sandboxed when
  sandboxing is enabled in the parent session."
- Env: sandboxed Bash inherits parent env (incl. credentials) unless
  `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` is set.

**On untrusted tool calls, the docs are explicit it is not a hard boundary**
(verbatim "Security limitations"):

> Sandboxing reduces risk but is **not a complete isolation boundary**. … the built-in proxy does not terminate or perform TLS inspection … code running inside the sandbox can potentially use domain fronting … to reach hosts outside the allowlist.

And the broader guidance: "Reserve `bypassPermissions` for CI, containers, or
other isolated environments." For stronger isolation the docs point to dev
containers / VMs ([Sandbox environments](https://code.claude.com/docs/en/sandbox-environments))
and the standalone `@anthropic-ai/sandbox-runtime` package that wraps the whole
Claude Code process.

→ **For ArkOS:** Bash isolation is available and OS-enforced, but (a) opt-in via
`settings.json`, not a first-class SDK option literal; (b) Bash-only — `Write`/
`Edit` confinement is still a permission-rule concern; (c) not a complete
boundary. "No writes outside the worktree" enforced *against all processes*
requires the sandbox (`sandbox.filesystem` write boundary = cwd by default);
enforced *against the built-in tools only* is a permission-rule concern. **Ambiguity:**
whether the bundled CLI inside the SDK exposes `sandbox.*` settings exactly as
the standalone CLI does was not separately verified in SDK source at this
snapshot — the sandboxing page is written for Claude Code; the SDK uses the same
bundled binary and `settings.json`, so it should apply, but confirm empirically.

---

## 9. Interaction with hooks — precedence

The permissions page publishes the exact evaluation order ("How permissions are
evaluated"). This is the authoritative answer to "which wins when they
conflict." (Hook *internals* are topic 04; here is only the ordering.)

Order, verbatim sequence:

1. **Hooks first.** "Run hooks first. A hook can deny the call outright or pass
   it on. A hook that returns `allow` does **not** skip the deny and ask rules
   below; those are evaluated regardless of the hook result." A `PreToolUse`
   hook that **blocks** (exits code 2 / returns deny) stops the call before the
   rule evaluation — it takes precedence over allow rules.
2. **Deny rules.** "Check `deny` rules (from `disallowed_tools` and
   settings.json). If a deny rule matches, the tool is blocked, **even in
   `bypassPermissions` mode**. Bare-name deny rules like `Bash` remove the tool
   from Claude's context before this evaluation begins, so only scoped rules
   like `Bash(rm *)` are checked at this step."
3. **Permission mode.** "`bypassPermissions` approves everything that reaches
   this step. `acceptEdits` approves file operations. Other modes fall through."
4. **Allow rules.** "Check `allow` rules (from `allowed_tools` and
   settings.json). If a rule matches, the tool is approved."
5. **`canUseTool` callback.** "If not resolved by any of the above, call your
   `canUseTool` callback for a decision. In `dontAsk` mode, **this step is
   skipped and the tool is denied**."

**One-paragraph precedence summary (the PRD's ask):** Hooks run *first* and a
blocking hook wins over everything downstream, but a hook that merely returns
"allow" does **not** short-circuit the deny/ask rules. **Deny rules** (from
`disallowed_tools` and `settings.json`) are next and are absolute — they block
even under `bypassPermissions` and even when an allow rule or hook said yes; a
*bare-name* deny removes the tool entirely (it never reaches step 2 scoped
checking). **Permission mode** then resolves what's left: `bypassPermissions`
approves, `acceptEdits` approves file ops, others fall through. **Allow rules**
(`allowed_tools`/settings allow) approve a match. Finally the **`canUseTool`
callback** decides anything still unresolved — except in `dontAsk`, where this
step is skipped and the call is denied. So the conflict-resolution pecking order
is: **blocking hook > deny rule > permission mode (bypass/acceptEdits) > allow
rule > canUseTool**, with deny rules being the only thing that cannot be
overridden by any other layer (managed-settings denies cannot be loosened even
by CLI args). For Ark's "safe tool set + no writes outside worktree" the durable
guarantee therefore lives in **deny rules and/or a blocking PreToolUse hook**,
not in `allowed_tools` (which only auto-approves) and not in `permissionMode`
(which `bypassPermissions` can nullify).

Settings precedence (when rules come from files): managed > CLI args > local
project (`settings.local.json`) > project (`settings.json`) > user
(`~/.claude/settings.json`). "If a tool is denied at any level, no other level
can allow it." The SDK can supply additional managed policy via the
`managedSettings` option (with `parentSettingsBehavior: "merge"`); embedder
values can **tighten** but not loosen policy.

---

## Decision table — where each ArkOS guarantee lives

| Goal | Mechanism | Why |
| :--- | :-------- | :-- |
| Auto-approve a fixed safe set (no prompts) | `allowed_tools` / `allowedTools` | permission layer; does not restrict, only approves |
| Remove a tool from the model's view | `tools` option, or bare-name `disallowed_tools` | availability layer |
| Hard-deny a dangerous command even under bypass | scoped `disallowed_tools` (`Bash(rm:*)`) | deny rules outrank mode |
| "Deny anything not explicitly allowed" | `allowed_tools` + `permissionMode:"dontAsk"` | dontAsk converts prompts to denials |
| Read-only / planning phase | `permissionMode:"plan"` (or `tools=[read-only set]`) | plan runs read-only tools only |
| `--dangerously-skip-permissions` equivalent | `permissionMode:"bypassPermissions"` | confirmed; circuit breakers differ slightly (§4) |
| No writes outside worktree — built-in tools only | `Edit`/`Read` deny rules + `acceptEdits` cwd scoping | rule layer; misses raw subprocesses |
| No writes outside worktree — ALL processes | OS sandbox (`sandbox.filesystem`, Bash-only) | OS enforcement; Bash subprocess scope only |
| Runtime per-call decision | `canUseTool` callback | last resort in the flow; skipped in dontAsk |
| Unconditional block regardless of mode | blocking `PreToolUse` hook + deny rule | hooks first; deny absolute (topic 04 for internals) |
| Bound tool-output size | substrate-side (custom-tool wrapper / PostToolUse hook) | no built-in result cap documented (§7) |

---

## Caveats / Not found

- **`MultiEdit`, `TodoWrite`, `NotebookEdit`: NOT FOUND** in the Agent SDK's
  documented built-in inventory at 0.2.87 / 0.3.150. The documented set is the
  agent-loop category table (15 tools) + `Monitor` (overview). These three are
  Claude Code CLI tool names not enumerated for the SDK here. Verify empirically
  before relying on them.
- **`Monitor`** is on the overview "key tools" tab but absent from the
  agent-loop category table — a doc inconsistency; treated as a real built-in.
- **`"auto"` permission mode is TypeScript-only** (confirmed). Also a "research
  preview" per the broader Claude Code docs; availability is gated.
- **`bypassPermissions` ≠ raw `--dangerously-skip-permissions`** at the edges:
  the SDK mode keeps `rm -rf /`/`~` and protected-path circuit breakers; the
  raw CLI flag skips protected-path checks too. The substrate-relevant value is
  `bypassPermissions`.
- **`Bash(x:*)` and `Bash(x *)` are equivalent**, but `:*` is recognized **only
  as a trailing wildcard**; mid-pattern colons are literal. Topic 01's
  `Bash(git:*)` is valid (trailing).
- **Permission rules are not OS enforcement.** `Read`/`Edit` deny rules cover
  built-in file tools and recognized Bash file commands (`cat`, `sed`, …) but
  **not** arbitrary subprocesses that open files themselves. OS-level "no writes
  outside path X" requires the sandbox (§8).
- **No documented tool-result size/truncation limit** in the SDK (§7). Context
  is managed by compaction, not per-result clipping. Substrate must enforce its
  own output ceiling if needed.
- **Sandbox is Bash-only, opt-in via `settings.json`, macOS/Linux/WSL2 only,
  and explicitly "not a complete isolation boundary."** `Write`/`Edit` go
  through the permission system, not the sandbox.
- **Python in-process custom tools cannot return `structuredContent`** (only
  `content` + `is_error` forwarded); TS in-process tools can. Divergence noted.
- **Python `can_use_tool` requires streaming mode + a dummy `PreToolUse` hook**
  returning `{"continue_": True}` to keep the stream open (user-input page
  `<Note>`); without it the stream closes before the callback fires. This is a
  Python-only wiring quirk.
- **`canUseTool` is not available inside subagents** for `AskUserQuestion`
  (user-input "Limitations"). Subagent permission is inherited mode + rules.
- **SDK-source verification not done in this pass.** Enum literals and option
  names are taken from the docs and cross-checked against neighbor topics
  01/02/03 (which read the Python `types.py`). The `sandbox.*` settings page is
  written for Claude Code; the SDK uses the same bundled binary and
  `settings.json`, so it should apply, but a byte-exact SDK-source check of the
  sandbox keys and the `PermissionMode` TS union was not performed here.

## Primary sources

- [Configure permissions (Agent SDK)](https://code.claude.com/docs/en/agent-sdk/permissions) — evaluation order, modes table, allow/deny rules, `dontAsk` pairing, `bypassPermissions` warning.
- [How the agent loop works](https://code.claude.com/docs/en/agent-sdk/agent-loop) — built-in tools category table, `tools`/`allowed`/`disallowed` layers, permission-mode table, parallel execution, context/compaction.
- [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview) — "key tools" one-liners, read-only permission example, `Monitor`/`AskUserQuestion`.
- [Give Claude custom tools](https://code.claude.com/docs/en/agent-sdk/custom-tools) — `@tool`/`tool()`, `create_sdk_mcp_server`/`createSdkMcpServer`, `mcp__server__tool` naming, return shape, `readOnlyHint`, Python `structuredContent` caveat, availability-vs-permission table.
- [Handle approvals and user input](https://code.claude.com/docs/en/agent-sdk/user-input) — `canUseTool` signature, `PermissionResultAllow`/`PermissionResultDeny`, `updatedInput`/`updatedPermissions`, Python streaming-mode requirement.
- [Configure permissions (Claude Code)](https://code.claude.com/docs/en/permissions) — full rule-syntax reference: Bash `:*` vs ` *`, gitignore path anchors, WebFetch/MCP/Agent rules, compound-command + process-wrapper semantics, settings precedence, hook-vs-rule precedence.
- [Permission settings (settings reference)](https://code.claude.com/docs/en/settings) — `permissions.{allow,ask,deny}` JSON shape.
- [Configure the sandboxed Bash tool](https://code.claude.com/docs/en/sandboxing) — sandbox modes, OS enforcement, `sandbox.*` keys, `autoAllowBashIfSandboxed`, scope limits, security limitations, `bypassPermissions` vs `--dangerously-skip-permissions` comparison.
- [Choose a permission mode](https://code.claude.com/docs/en/permission-modes) — mode semantics, protected paths, auto-mode availability.
- Neighbor corpus files (SDK-identifier cross-check): `01_overview-and-relationship-to-claude-code.md` (§5 `PermissionMode` literals, `CanUseTool` alias), `02_sessions.md` (§1 default tool availability, `settingSources`), `03_streaming-events.md` (`Task → Agent` rename at CLI v2.1.63).
