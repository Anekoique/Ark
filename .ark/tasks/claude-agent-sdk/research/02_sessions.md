# Research: Claude Agent SDK session lifecycle

- Query: How do Claude Agent SDK sessions work — fresh / resume / fork / one-shot; on-disk JSONL format; setting sources (CLAUDE.md, AGENTS.md, skills, commands); session listing and store adapters; cwd pinning
- Scope: external (primary docs + published SDK source)
- Date: 2026-05-25

## Version snapshot

| Surface | Version researched | Source |
| ------- | ------------------ | ------ |
| Python SDK (`claude-agent-sdk`) | **0.2.87** (bundles Claude CLI 2.1.150) | `anthropics/claude-agent-sdk-python` CHANGELOG.md + `main` source, fetched 2026-05-25 |
| TypeScript SDK (`@anthropic-ai/claude-agent-sdk`) | **0.3.150** | `anthropics/claude-agent-sdk-typescript` CHANGELOG.md, fetched 2026-05-25 |
| Docs | `code.claude.com/docs/en/agent-sdk/*` | `sessions`, `session-storage`, `overview`, `agent-loop`, `claude-code-features`, `modifying-system-prompts`, fetched 2026-05-25 |

Doc-host note: `docs.claude.com/en/api/agent-sdk/*` 301→ `platform.claude.com` 307→ **`code.claude.com/docs/en/agent-sdk/*`**. Use the `code.claude.com` URLs directly.

Streaming event types (`SystemMessage`/`AssistantMessage`/`UserMessage`/`StreamEvent`/`ResultMessage`) are owned by neighbor file `03_streaming-events.md`; this file references them only where session lifecycle depends on them (session-ID capture, `init`/`result` subtypes).

---

## Findings

### 0. The two API shapes

There are exactly two entry points; everything below is expressed through them.

| | Python | TypeScript |
| --- | --- | --- |
| Stateless / one-shot / unidirectional | `query(*, prompt, options=None, transport=None)` → `AsyncIterator[Message]` | `query({ prompt, options })` → `AsyncGenerator` |
| Stateful multi-turn (in-process, auto session tracking) | `ClaudeSDKClient(options=None, transport=None)` | *(no client object — use `continue: true` on each `query()`)* |

Verbatim from the Python `query()` docstring (`src/claude_agent_sdk/query.py`):

> Query Claude Code for one-shot or unidirectional interactions. … **Stateless**: Each query is independent, no conversation state … **No interrupts**: Cannot interrupt or send follow-up messages

The experimental TS **V2 session API** (`unstable_v2_createSession` / `unstable_v2_resumeSession` / `unstable_v2_prompt` / `SDKSession` / `SDKSessionOptions`) was **removed in TS 0.3.142** (deprecated since 0.2.133). Docs note verbatim:

> The experimental V2 session API, which provided `createSession()` with a `send` / `stream` pattern, was removed in TypeScript Agent SDK 0.3.142. Use the `query()` function and the session options described on this page instead.

→ Do not design ArkOS against the V2 API. It is gone.

---

### 1. Fresh session start

**Call:** `query()` with no `resume` / `continue` / `fork` option. Each `query()` with a fresh prompt mints a new session ID and writes a new JSONL file.

```python
# Python — minimal fresh session
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

```typescript
// TypeScript — minimal fresh session
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Find and fix the bug in auth.ts",
  options: { allowedTools: ["Read", "Edit", "Bash"] }
})) {
  console.log(message);
}
```

**What loads automatically at a fresh start** (docs `agent-loop`, `claude-code-features`):

- **Agent loop step 1** yields a `SystemMessage` with `subtype: "init"` carrying session metadata, *then* the loop runs. The session ID is in that init message (see §"Session ID capture").
- **cwd:** defaults to `process.cwd()` (TS) / the process's current directory (Python). See §9.
- **Tools:** the full built-in tool inventory is *available* by default — `Read`, `Write`, `Edit`, `Bash`, `Monitor`, `Glob`, `Grep`, `WebSearch`, `WebFetch`, `AskUserQuestion`, plus orchestration tools `Agent`, `Skill`, `TaskCreate`, `TaskUpdate`, and `ToolSearch` (`agent-loop` built-in table). `allowed_tools`/`allowedTools` only *auto-approves* a subset; unlisted tools still exist but require permission. (Tool inventory/permissions belong to file `05`.)
- **System prompt:** **minimal default** unless you opt into the preset. Docs `modifying-system-prompts`, verbatim:
  > **Minimal default**: when you don't set `systemPrompt` … the SDK uses a minimal prompt that covers tool calling but omits Claude Code's coding guidelines, response style, and project context. **This differs from `claude -p`**, which uses the full Claude Code prompt by default.
  To match the CLI: `systemPrompt: { type: "preset", preset: "claude_code" }`.
- **Filesystem settings (CLAUDE.md, skills, hooks, commands, agents):** loaded by default in `query()` (equivalent to `["user","project","local"]`) — see §8. Note: this is `query()`'s default; the *SDK-wide* breaking change in 0.1.0 was "no filesystem settings by default" — reconcile in §8.

---

### 2. Resume an existing session

**Call:** `query()` with `options.resume = <session_id>` (Python `resume`, TS `resume`). Picks up a *specific* session by ID.

```python
# Python — resume by ID
async for message in query(
    prompt="Now implement the refactoring you suggested",
    options=ClaudeAgentOptions(
        resume=session_id,
        allowed_tools=["Read", "Edit", "Write", "Glob", "Grep"],
    ),
):
    ...
```

```typescript
// TypeScript — resume by ID
for await (const message of query({
  prompt: "Now implement the refactoring you suggested",
  options: { resume: sessionId, allowedTools: ["Read", "Edit", "Write", "Glob", "Grep"] }
})) {
  ...
}
```

**What gets restored** (docs `agent-loop` §"Sessions and continuity", verbatim):

> When you resume, the full context from previous turns is restored: files that were read, analysis that was performed, and actions that were taken.

This is **conversation** restoration, not filesystem restoration. Docs `sessions`, verbatim:

> Sessions persist the **conversation**, not the filesystem. To snapshot and revert file changes the agent made, use file checkpointing.

**Two sibling forms of resume:**

| Mechanism | Python | TS | Finds |
| --- | --- | --- | --- |
| Resume a *specific* session | `resume="<id>"` | `resume: "<id>"` | the session with that ID |
| **Continue** the *most recent* session in the directory (no ID) | `continue_conversation=True` | `continue: true` | most recent JSONL in the cwd's project dir |

`continue` / `continue_conversation` is the no-ID path: "Continue the most recent conversation" (TS `Options` default `false`; Python `ClaudeAgentOptions.continue_conversation` default `False`).

**Session ID capture** (required before you can resume/fork). The ID is on the result message always; also on the init system message:

- Python: `ResultMessage.session_id`, or `SystemMessage.data["session_id"]` on the `init` message.
- TypeScript: `message.session_id` on the result, and **directly** on the init `SystemMessage` (`message.type === "system" && message.subtype === "init"`).

```python
session_id = None
async for message in query(prompt="Read the authentication module",
                           options=ClaudeAgentOptions(allowed_tools=["Read","Glob"])):
    if isinstance(message, SystemMessage) and message.subtype == "init":
        session_id = message.data["session_id"]
    if isinstance(message, ResultMessage):
        session_id = message.session_id   # present on success AND error
```

Docs note: `session_id` is present on *every* `ResultMessage`/`SDKResultMessage` "regardless of success or error", so you can resume even after `error_max_turns` / `error_max_budget_usd` (raise the limit and resume — a documented use case).

**Automatic session tracking** (no manual ID handling) for multi-turn in one process:

- Python `ClaudeSDKClient` — each `client.query()` continues the same session automatically; "handles session IDs internally." Typically an async context manager:
  ```python
  async with ClaudeSDKClient(options=options) as client:
      await client.query("Analyze the auth module")
      async for m in client.receive_response(): ...
      await client.query("Now refactor it to use JWT")   # same session, full context
      async for m in client.receive_response(): ...
  ```
  Client surface (`src/claude_agent_sdk/client.py`): `connect()`, `query(prompt, session_id="default")`, `receive_messages()`, `receive_response()`, `interrupt()`, `disconnect()`.
- TypeScript — no client object. Pass `continue: true` on each subsequent `query()`. Docs verbatim: "The TypeScript SDK doesn't have a session-holding client object like Python's `ClaudeSDKClient`."

---

### 3. Fork a session

**Call:** `query()` with **both** `resume=<id>` **and** the fork flag (`fork_session=True` / `forkSession: true`). Supported in **both** Python and TS.

```python
# Python — fork; the fork gets a NEW id, original untouched
forked_id = None
async for message in query(
    prompt="Instead of JWT, implement OAuth2 for the auth module",
    options=ClaudeAgentOptions(resume=session_id, fork_session=True),
):
    if isinstance(message, ResultMessage):
        forked_id = message.session_id     # distinct from session_id
```

```typescript
// TypeScript — fork
let forkedId: string | undefined;
for await (const message of query({
  prompt: "Instead of JWT, implement OAuth2 for the auth module",
  options: { resume: sessionId, forkSession: true }
})) {
  if (message.type === "system" && message.subtype === "init") {
    forkedId = message.session_id;        // the fork's id
  }
}
```

**Semantics** (docs `sessions` §"Fork to explore alternatives", verbatim):

> Forking creates a new session that starts with a copy of the original's history but diverges from that point. The fork gets its own session ID; **the original's ID and history stay unchanged**. You end up with two independent sessions you can resume separately.

**Concrete use case (as asked):** try a different prompt continuation without losing the trunk. Fork from `session_id` into a new OAuth2 line (`forked_id`); the JWT trunk (`session_id`) is still resumable afterward → two independent histories.

**Filesystem caveat** (verbatim): "Forking branches the conversation history, **not the filesystem**. If a forked agent edits files, those changes are real and visible to any session working in the same directory." → fork gives no file isolation; pair with file checkpointing if you need to branch file state.

**Option defaults:** `fork_session`/`forkSession` default `False`/`false`. Fork is meaningless without `resume`; docs describe it strictly as "When resuming with `resume`, fork to a new session ID instead of continuing the original session."

**Standalone helpers** also exist (changelog): `fork_session(...)` (Python, since 0.1.51) and `forkSession(sessionId, opts?)` (TS, since 0.2.76) — these fork from a point without driving a new `query()` turn; both accept a `sessionStore`. The option-on-`query()` form above is the one the docs lead with.

---

### 4. One-shot mode

**There is no separate "one-shot" API — one-shot is the default shape of `query()`.** Docs `sessions` "Choose an approach" table, verbatim:

> | One-shot task: single prompt, no follow-up | Nothing extra. One `query()` call handles it. |

Within a single `query()` call the agent already takes as many turns as it needs; permission prompts and `AskUserQuestion` are handled **in-loop** (they don't end the call). So "one-shot" = "one `query()` call, no `resume`/`continue`/`fork`, don't capture the ID."

Distinction from "a short session": a one-shot still **persists to disk** by default (a JSONL is written, resumable later). To make it truly ephemeral:

- **TypeScript:** `persistSession: false` (default `true`). Docs verbatim: "the session exists only in memory for the duration of the call. … **Sessions cannot be resumed later**."
- **Python:** **no `persist_session` option exists** (confirmed: `grep persist_session src/` returns nothing in 0.2.87). Docs verbatim: "**Python always persists to disk.**" → in-memory-only sessions are a **TS-only** capability. For Python the workaround is a custom in-memory `SessionStore` plus directing the on-disk write elsewhere, but the local-disk write itself is not suppressible via an option.

---

### 5. Persistence format (on-disk JSONL)

**Location** (verified against `code.claude.com/docs/en/agent-sdk/sessions` and Python source `_internal/sessions.py`):

```
~/.claude/projects/<encoded-cwd>/<session-id>.jsonl
```

- `~/.claude` is the config home, overridable by `CLAUDE_CONFIG_DIR` (source `_get_claude_config_home_dir`); projects live under `<config>/projects/`.
- `<encoded-cwd>` ("project key" / "projectKey"): the **absolute cwd with every non-alphanumeric char replaced by `-`**. Docs verbatim: "`<encoded-cwd>` is the absolute working directory with every non-alphanumeric character replaced by `-` (so `/Users/me/proj` becomes `-Users-me-proj`)." Source confirms: `_SANITIZE_RE = re.compile(r"[^a-zA-Z0-9]")`, applied in `_sanitize_path`. Paths > **200 chars** (`MAX_SANITIZED_LENGTH`) are truncated and suffixed with a portable djb2-style base-36 hash so the same path yields the same key across runtimes.
- `<session-id>`: a UUID. Source validates with `_UUID_RE` (`^[0-9a-f]{8}-...{12}$`); writes go to `f"{session_id}.jsonl"`.
- File mode is `0o600` (source `_write_jsonl` chmods).

**Subagent transcripts** live in a per-session sidecar tree:

```
~/.claude/projects/<project>/<sessionId>/subagents/agent-<agentId>.jsonl
~/.claude/projects/<project>/<sessionId>/subagents/agent-<agentId>.meta.json   (sidecar metadata)
```

(Source `_internal/sessions.py` docstrings; `agent-*.jsonl` collected recursively; `.meta.json` holds the last `agent_metadata` entry minus its synthetic `type` field.)

**What each line contains.** One JSON object per line (`json.dumps(e, separators=(",", ":"))` + `\n`). Record types the SDK recognizes — source `_TRANSCRIPT_ENTRY_TYPES`:

```python
_TRANSCRIPT_ENTRY_TYPES = frozenset({"user", "assistant", "progress", "system", "attachment"})
```

Transcript-line fields (source comment, "mirrors the TS `TranscriptEntry` type"):

> fields: `type`, `uuid`, `parentUuid`, `sessionId`, `message`, `isSidechain`, `isMeta`, `isCompactSummary`, `teamName`

- The `parentUuid` chain links records into a tree; `get_session_messages` walks `uuid`/`parentUuid` to reconstruct the chronological `user`/`assistant` thread and follows sidechains.
- **Non-transcript metadata lines** also appear, keyed by `type`:
  - `{"type":"tag", "tag": "..."}` — session tag (set by `tag_session`).
  - title lines carrying `customTitle` (user-set, wins) or `aiTitle` (auto-generated).
  - `lastPrompt`, `summary`, `gitBranch`, `cwd`, `timestamp` (ISO; first one → `created_at` in epoch ms).
- Auto/system prompts the listing logic skips when finding the "first meaningful prompt": `<local-command-stdout>`, `<session-start-hook>`, `<tick>`, `<goal>`, `[Request interrupted by user…]`, `<ide_opened_file>`, `<ide_selection>`, and `<command-name>…</command-name>` blocks (source `_SKIP_FIRST_PROMPT_PATTERN`, `_COMMAND_NAME_RE`).

**Is the format stable / documented / private?** **PRIVATE / unstable.** The line schema is not published as a contract. Docs `session-storage`, verbatim:

> Treat them as opaque JSON-safe values … the internal JSONL format and detailed entry schemas are implementation details not guaranteed to remain stable.

The `SessionStoreEntry` TypedDict docstring (source `types.py`) calls itself "One JSONL transcript line as observed by a `SessionStore` adapter." `load()` need only return entries **deep-equal** to what was appended (byte-equal not required) — confirming the SDK treats lines as opaque blobs. **→ For ArkOS: do NOT parse the JSONL directly. Read sessions only via the SDK functions in §6.**

---

### 6. Session listing and discovery

Both SDKs expose read/enumerate and mutate functions over the on-disk store (and over a custom `SessionStore`, §7). All take an optional directory and an optional `sessionStore`.

| Purpose | Python | TypeScript |
| --- | --- | --- |
| List sessions w/ light metadata | `list_sessions(directory=None, limit=None, include_worktrees=True)` → `list[SDKSessionInfo]` (sync) | `listSessions(options?)` → `Promise<SDKSessionInfo[]>` (`options.dir`, `options.limit`, `options.includeWorktrees=true`) |
| Read full message history | `get_session_messages(session_id, directory=None, limit=None, offset=0)` → `list[SessionMessage]` (sync) | `getSessionMessages(sessionId, options?)` → `Promise<SessionMessage[]>` (`dir`, `limit`, `offset`) |
| Read one session's metadata | `get_session_info(session_id, directory=None)` → `SDKSessionInfo \| None` (sync) | `getSessionInfo(sessionId, options?)` → `Promise<SDKSessionInfo \| undefined>` |
| Rename (human title) | `rename_session(session_id, title, directory=None)` | `renameSession(sessionId, title, opts?)` |
| Tag | `tag_session(session_id, tag, directory=None)` (`tag=None` clears) | `tagSession(sessionId, tag, opts?)` (`tag=null` clears) |
| Delete | `delete_session(...)` (changelog 0.1.51) | `deleteSession(...)` (changelog 0.2.113) |
| Subagent transcripts | `list_subagents()`, `get_subagent_messages()` (0.1.60) | `listSubagents()`, `getSubagentMessages()` |

**Filter by cwd / project:** pass `directory` (Python) / `options.dir` (TS). **When omitted, both search across all projects** under `~/.claude/projects/` (source comments + docs). `include_worktrees`/`includeWorktrees` (default `true`): when `dir` is in a git repo, include sessions from all worktree paths of that repo (source sorts worktree project dirs longest-sanitized-prefix-first).

**Filter by age:** no native age filter. `SDKSessionInfo` carries `last_modified`/mtime and `created_at` (epoch ms; added Python 0.1.50 / TS 0.2.75) — filter client-side. `listSessions` also supports `limit` and (TS) `offset` for pagination.

`SDKSessionInfo` fields (from source `SDKSessionInfo(...)` construction): `session_id`, `summary` (customTitle → lastPrompt → summary → first_prompt fallback chain), `last_modified`, `file_size`, `custom_title`, `first_prompt`, `git_branch`, `cwd`, `tag`, `created_at`.

**Pruning / GC:** **None automatic.** Docs (`session-storage`) verbatim: "**The SDK never deletes from store**; implement TTLs, S3 lifecycle policies, or scheduled cleanup per compliance requirements." On-disk sessions accumulate until you call `delete_session`/`deleteSession` or remove files yourself. → ArkOS owns retention.

---

### 7. Session store adapters (custom backend)

**Yes — the on-disk default can be mirrored to a custom backend** via the `SessionStore` adapter interface (TS alpha since 0.2.113; Python at TS parity since 0.1.64).

**Wire-up:** `options.session_store` (Python) / `options.sessionStore` (TS), default `None`/`undefined`. Accepted by `query()`, `startup()`, and all the session functions in §6, plus `forkSession`/`fork_session`. Python adds `session_store_flush: "batched" | "eager"` (default `"batched"`; `"eager"` flushes after every frame for live-tailing / cross-process resume / crash durability — added 0.1.73).

**Interface (TypeScript)** — verbatim from `session-storage`:

```typescript
type SessionKey = { projectKey: string; sessionId: string; subpath?: string };

type SessionStore = {
  // Required
  append(key: SessionKey, entries: SessionStoreEntry[]): Promise<void>;
  load(key: SessionKey): Promise<SessionStoreEntry[] | null>;
  // Optional
  listSessions?(projectKey: string): Promise<Array<{ sessionId: string; mtime: number }>>;
  delete?(key: SessionKey): Promise<void>;
  listSubkeys?(key: { projectKey: string; sessionId: string }): Promise<string[]>;
};
```

**Interface (Python)** — `SessionStore` Protocol with 5 methods: `append`, `load`, `list_sessions`, `delete`, `list_subkeys` (changelog 0.1.64). Conformance suite shipped in-package:

```python
from claude_agent_sdk.testing import run_session_store_conformance

@pytest.mark.asyncio
async def test_my_store_conformance():
    await run_session_store_conformance(MyRedisStore)
```

**Required vs optional methods** (docs):

| Method | Required? | Called when |
| --- | --- | --- |
| `append(key, entries)` | yes | after each batch of transcript entries is written locally — mirror to external storage |
| `load(key)` | yes | once before subprocess spawns when `resume` is set; return `null` if unknown |
| `listSessions(projectKey)` | optional | enables `listSessions({sessionStore})` and `continue: true` over the store |
| `delete(key)` | optional | enables `deleteSession`; deleting the main key **must cascade** to subkeys |
| `listSubkeys(key)` | optional | discover subagent transcripts on resume; without it, only the main transcript restores |

**`SessionKey` fields** (source `types.py`): `project_key` ("Caller-defined scope. Default: sanitized cwd. Multi-tenant deployments should set this to a tenant ID or project name."); `session_id`; `subpath` (omit for main transcript; `"subagents/agent-{id}"` for subagent files; opaque to the adapter).

**Behavioral notes** (docs, load-bearing for ArkOS):

- **Dual-write / mirror, not replacement.** The subprocess always writes local disk first, *then* the SDK forwards to `append()`. The store is a mirror.
- **Best-effort.** If `append()` fails, the error is logged, a `{type:"system", subtype:"mirror_error"}` message is emitted, and the query continues. **Failed batches are NOT retried.** Monitor `mirror_error`.
- Built-ins: `InMemorySessionStore` (both SDKs); `importSessionToStore()` / `import_session_to_store()` to migrate local→remote.
- Reference adapters shipped in the repos: **S3** (one JSONL part file per `append()`; `load()` lists+sorts+concatenates), **Redis** (`RPUSH`/`LRANGE` list + sorted-set index, `ioredis`/Python redis), **Postgres** (one `jsonb` row per entry, `pg`/`asyncpg`). TS under `examples/session-stores/`, Python under `examples/session_stores/`.

→ **For ArkOS:** S3/DB-backed cross-host resume is supported, but only as a *mirror* of a mandatory local write (best-effort, no retry). A truly disk-free backend is not the design; treat the store as durable redundancy + cross-host transport.

---

### 8. `settingSources` — what loads from the filesystem at session start

**Option:** `setting_sources: list[SettingSource] | None` (Python, default `None`) / `settingSources: SettingSource[]` (TS). Values: **`"user"`, `"project"`, `"local"`**.

**Defaults — reconcile the two statements:**

- The SDK-wide breaking change in **v0.1.0** ("No filesystem settings by default … Settings files, slash commands, and subagents no longer load automatically") means the *raw option default* is "load nothing extra."
- But **`query()` itself** applies the CLI defaults when you **omit** `settingSources`. Docs `claude-code-features`, verbatim: "When you omit `settingSources`, `query()` reads the same filesystem settings as the Claude Code CLI: user, project, and local settings, CLAUDE.md files, and `.claude/` skills, agents, and commands." And: "**Omitting `settingSources` is equivalent to `["user", "project", "local"]`.**"
- → Omit = all three load. Pass `[]` = disable user/project/local. Pass an explicit list = exactly those.

**What each source loads** (docs table, verbatim):

| Source | Loads | Location |
| --- | --- | --- |
| `"project"` | Project `CLAUDE.md`, `.claude/rules/*.md`, project skills, project hooks, project `settings.json` | `<cwd>/.claude/` for settings.json+hooks; `<cwd>` + every parent for CLAUDE.md/rules; `<cwd>` + parents up to repo root for skills |
| `"user"` | User `CLAUDE.md`, `~/.claude/rules/*.md`, user skills, user settings | `~/.claude/` |
| `"local"` | `CLAUDE.local.md`, `.claude/settings.local.json` | `<cwd>/.claude/` for settings.local.json; `<cwd>` + parents for CLAUDE.local.md |

- **CLAUDE.md** is injected into the **conversation** (project context), **not** the system prompt — so it loads regardless of `systemPrompt` choice, controlled only by setting sources. "It is not loaded if you pass an empty `settingSources` array."
- **Skills:** discovered via setting sources; descriptions load at startup, full body on demand. The `skills` option (`"all"` | name list | `[]`) further gates which are enabled; when `skills` is set the Skill tool is auto-enabled (no need to add to `allowedTools`). `.claude/skills/<name>/SKILL.md` only — **no programmatic skill-registration API**.
- **Slash commands:** `.claude/commands/*.md`, loaded with project/user sources.
- **Hooks (filesystem):** shell commands in `settings.json`, loaded via setting sources, run side-by-side with programmatic hooks. (Hook API → file `04`.)

**AGENTS.md — explicit gap.** None of the session/settings docs name `AGENTS.md`. The Claude-Code-feature loaders enumerate `CLAUDE.md`, `.claude/CLAUDE.md`, `CLAUDE.local.md`, `.claude/rules/*.md`, skills, commands, agents, hooks, `settings.json` — **`AGENTS.md` is not in any `settingSources` table.** The Python source's auto/system-prompt skip patterns and title logic also never reference `AGENTS.md`. **→ Not found: no evidence the Claude Agent SDK loads `AGENTS.md` via `settingSources`.** (`AGENTS.md` is an OpenCode/Codex-ecosystem convention; in Ark's own templates it is the Codex/OpenCode analog of `CLAUDE.md`.) Treat as **undocumented for this SDK** — do not assume it loads. The SDK's project-instruction file is `CLAUDE.md`.

**Per-session vs process-wide:** `settingSources` is a **per-`query()` / per-options field** — set independently on each call/session. There is no process-wide global; isolation is per-options.

**What does NOT load (regardless of `settingSources`)** — docs `claude-code-features` table, load-bearing for multi-tenant ArkOS:

| Input | Behavior | To disable |
| --- | --- | --- |
| Managed policy settings | Always loaded when present on host | Remove the managed settings file |
| `~/.claude.json` global config | Always read | Relocate via `CLAUDE_CONFIG_DIR` in `env` |
| Auto memory `~/.claude/projects/<project>/memory/` | Loaded by default into system prompt | `autoMemoryEnabled: false`, or `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1` in `env` |

Docs `<Warning>` verbatim: "Do not rely on default `query()` options for multi-tenant isolation. … For multi-tenant deployments, run each tenant in its own filesystem and set `settingSources: []` plus `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1` in `env`." (Ark's auto-memory at `~/.claude/projects/.../memory/MEMORY.md` is exactly this auto-memory path — it loads even with `settingSources: []`.)

---

### 9. Cwd / working directory

**Pin a session to a directory:** set `cwd` in options.

- Python: `ClaudeAgentOptions.cwd: str | Path | None = None`. Source `types.py:1699`: *"Current working directory for the session. Defaults to the process cwd."*
- TypeScript: `Options.cwd: string`, default `process.cwd()`.

**Why cwd is structurally load-bearing, not cosmetic:**

1. **It determines the on-disk project key.** Sessions are stored under `~/.claude/projects/<encoded-cwd>/`. The encoded segment is derived from the **absolute cwd** (§5). Two different cwds → two different project directories.
2. **It scopes filesystem-settings discovery.** `<cwd>/.claude/`, `<cwd>` + parents for `CLAUDE.md`/rules, up to repo root for skills (§8). "The `cwd` option determines where the SDK looks for project-level inputs."
3. **It scopes `list_sessions`/`get_session_messages` lookups** when `directory` is passed.

**When the SDK process's own cwd differs from `options.cwd`:** `options.cwd` wins for session storage, settings discovery, and the agent's file operations — the SDK passes it to the subprocess (source `subprocess_cli.py`: `self._cwd = str(options.cwd) if options.cwd else None`, and it errors if the path doesn't exist: `if self._cwd and not Path(self._cwd).exists()`). When `options.cwd` is **unset**, it falls back to the process's current directory. So the process cwd matters only as the *default*; once `options.cwd` is set, the process cwd is irrelevant to session placement.

**Classic resume failure = cwd mismatch.** Docs `<Tip>` verbatim:

> If a `resume` call returns a fresh session instead of the expected history, the most common cause is a mismatched `cwd`. … If your resume call runs from a different directory, the SDK looks in the wrong place. The session file also needs to exist on the current machine.

→ **For ArkOS:** to resume a session you must replay the **same `cwd`** (so the project key matches) **and** have the JSONL present locally — or use a `SessionStore` (§7) that ignores local paths. Cross-host resume requires either moving `~/.claude/projects/<encoded-cwd>/<id>.jsonl` to the identical path or using a store adapter. Sessions are **machine-local by default**.

---

## Caveats / Not found

- **`AGENTS.md` loading: NOT FOUND / undocumented for this SDK.** No `settingSources` table, no source loader, and no docs page names `AGENTS.md`. The SDK's project-instruction file is `CLAUDE.md` (+ `.claude/rules/*.md`, `CLAUDE.local.md`). Do not assume `AGENTS.md` is read; verify empirically if ArkOS needs it.
- **Python `persist_session`: does not exist** (grep of 0.2.87 source is empty). In-memory-only / non-persisted sessions are **TypeScript-only** (`persistSession: false`); docs state "Python always persists to disk."
- **JSONL line schema is private and explicitly unstable** — documented as an implementation detail "not guaranteed to remain stable." Field list (`type`, `uuid`, `parentUuid`, `sessionId`, `message`, `isSidechain`, `isMeta`, `isCompactSummary`, `teamName`) is from a Python source comment, not a published contract. Read via SDK functions (§6), never by parsing files.
- **No automatic pruning/GC and no native age filter** on session listing. `created_at`/`last_modified` exist; filter client-side. Retention is the host's job.
- **`SessionStore` is a mirror, best-effort, not retried** — not a drop-in replacement for local disk. The subprocess always writes local disk first.
- **V2 session API is removed** (TS 0.3.142) — `createSession`/`SDKSession` no longer exist. Anything referencing them is stale.
- **Doc snapshot only:** code snippets and option tables are from `code.claude.com` and the SDK `main` branches as of 2026-05-25 (Python 0.2.87 / TS 0.3.150). The `code.claude.com` docs do not print a per-page "last updated" date; version pin is via the CHANGELOGs.
- **Did not separately verify the TypeScript on-disk encoding in TS source** (only Python source inspected for `_sanitize_path`); the docs `<Tip>` states the same `<encoded-cwd>` rule for both, and the cross-SDK `SessionStore`/`SessionKey` shapes match, so the encoding is treated as shared. Confirm in TS source if a byte-exact match matters.
