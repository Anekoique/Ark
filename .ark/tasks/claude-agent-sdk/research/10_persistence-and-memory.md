# Research: Claude Agent SDK persistence and memory primitives

- Query: What does the SDK persist for recall across sessions (auto-memory dir, the `memory` option / Memory tool, CLAUDE.md-as-memory), what it does NOT provide (semantic / episodic / KB / retrieval), how a host plugs in its own memory layer, and what happens to memory under compaction.
- Scope: external (primary docs + published SDK source)
- Date: 2026-05-25

## Version snapshot

| Surface | Version researched | Source |
| ------- | ------------------ | ------ |
| Python SDK (`claude-agent-sdk`) | **0.2.87** (bundles Claude CLI 2.1.150) | `/private/tmp/claude-agent-sdk-python` @ `9970096` ("docs: update changelog for v0.2.87"); `pyproject.toml` `version = "0.2.87"` |
| TypeScript SDK (`@anthropic-ai/claude-agent-sdk`) | **0.3.150** | per neighbor 02 CHANGELOG pin, fetched 2026-05-25 |
| Docs | `code.claude.com/docs/en/{memory,context-window,sub-agents}` + `code.claude.com/docs/en/agent-sdk/claude-code-features` | fetched 2026-05-25 |
| API Memory tool (separate product) | `memory_20250818` beta (Messages API) | `platform.claude.com` / `docs.claude.com` tool-use docs, fetched 2026-05-25 |

**Scope boundary with neighbor `02_sessions.md`.** Session-transcript persistence (JSONL at `~/.claude/projects/<encoded-cwd>/<id>.jsonl`, the `SessionStore` mirror adapter, listing/discovery, the private/unstable line schema) is owned by topic 02 and is NOT re-derived here. This file is about **memory** — durable, recall-oriented state the SDK carries across sessions — and what the SDK explicitly does not do. Where session persistence is load-bearing for a memory claim (compaction lines, the `MEMORY.md` auto-load path) it is cited, not re-explained.

**Headline (for the impatient).** Beyond session transcripts, the SDK gives you exactly one durable memory primitive that the model populates itself: **Claude Code auto-memory** (a per-repo `MEMORY.md` directory), plus an opt-in **per-subagent persistent memory directory** that is the same mechanism scoped to one agent. Both are flat markdown the agent reads/writes with file tools, auto-loaded (first 200 lines / 25 KB) at session start. There is **no semantic/vector memory, no episodic recall index, no cross-project knowledge base, and no retrieval layer** in the Agent SDK. The Anthropic **API Memory tool (`memory_20250818`)** is a *different* product (Messages API, client-side backend) and is **not surfaced by the Agent SDK** at this snapshot. Everything ArkOS wants for working/episodic/semantic/procedural memory is build-your-own.

---

## Findings

### 1. What auto-persists across sessions

Two things survive a session ending, and only two:

1. **Session transcripts** — the JSONL files (topic 02). These are *conversation* persistence, not memory: you `resume`/`fork` a *specific* session by ID. They are not recall-by-content; there is no "find the session where I learned X." Retention is the host's job (topic 02 §6: "The SDK never deletes from store").
2. **Auto-memory** — a per-repository markdown directory the *model* writes to and that auto-loads into every future session in that repo. This is the SDK's only content-addressed, cross-session recall primitive.

#### 1a. The auto-memory directory — what it is and who owns it

**It is a Claude Code feature, surfaced through the SDK** — not an SDK-original API. The mechanism is documented on the **Claude Code** memory page (`code.claude.com/docs/en/memory`), and the SDK inherits it because the SDK runs the same agent loop / bundled CLI. The SDK's only *handle* on it is the `settingSources` gate plus two kill-switches (below); there is no `query()` option that creates, queries, or seeds auto-memory programmatically.

Docs `code.claude.com/docs/en/memory`, verbatim — the CLAUDE.md-vs-auto-memory split:

> |                      | CLAUDE.md files       | Auto memory                              |
> | **Who writes it**    | You                   | Claude                                   |
> | **What it contains** | Instructions and rules| Learnings and patterns                   |
> | **Scope**            | Project, user, or org | Per repository, shared across worktrees  |
> | **Loaded into**      | Every session         | Every session (first 200 lines or 25KB)  |

**Storage location** (docs `memory` §"Storage location", verbatim):

> Each project gets its own memory directory at `~/.claude/projects/<project>/memory/`. The `<project>` path is derived from the git repository, so all worktrees and subdirectories within the same repo share one auto memory directory. Outside a git repo, the project root is used instead.

(This is the same path Ark's own `MEMORY.md` lives at — see `~/.claude/projects/-Users-anekoique-Agent-Ark/memory/MEMORY.md`.)

**Directory shape** (docs, verbatim):

```text
~/.claude/projects/<project>/memory/
├── MEMORY.md          # Concise index, loaded into every session
├── debugging.md       # Detailed notes on debugging patterns
├── api-conventions.md # API design decisions
└── ...                # Any other topic files Claude creates
```

**What writes to it / what reads from it / when** (docs `memory` §"How it works", verbatim):

> The first 200 lines of `MEMORY.md`, or the first 25KB, whichever comes first, are loaded at the start of every conversation. … Claude keeps `MEMORY.md` concise by moving detailed notes into separate topic files.
>
> Topic files like `debugging.md` or `patterns.md` are not loaded at startup. Claude reads them on demand using its standard file tools when it needs the information.
>
> Claude reads and writes memory files during your session. When you see "Writing memory" or "Recalled memory" in the Claude Code interface, Claude is actively updating or reading from `~/.claude/projects/<project>/memory/`.

So the writer is **the model itself**, using ordinary Read/Write/Edit file tools, deciding what is worth remembering: "Claude doesn't save something every session. It decides what's worth remembering based on whether the information would be useful in a future conversation." The reader is **session startup** (auto-injects the `MEMORY.md` head) plus **the model on demand** (topic files). There is no embedding, no similarity search — it is "load the index, then the model greps/reads files." Recall is model-mediated file I/O, not retrieval.

**Where in context it lands.** Auto-memory loads as part of the **system-prompt startup block** (docs `context-window` event list: "Auto memory (MEMORY.md)" is an `auto` startup event alongside the system prompt and environment info). CLAUDE.md, by contrast, is delivered as a **user message after the system prompt** (docs `memory` §troubleshoot, verbatim: "CLAUDE.md content is delivered as a user message after the system prompt, not as part of the system prompt itself."). Different injection points; both load every session.

**Version floor.** Docs `memory`, verbatim: "Auto memory requires Claude Code v2.1.59 or later." The Python SDK 0.2.87 bundles CLI 2.1.150, so it is well past the floor.

**Toggles / relocation** (docs `memory` + `claude-code-features`):

- On by default. Disable per-project via settings `{"autoMemoryEnabled": false}`, or via env `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`.
- Relocate via user-settings `{"autoMemoryDirectory": "~/my-custom-memory-dir"}`. Docs verbatim: "The value must be an absolute path or start with `~/`. This setting is accepted from policy and user settings, and from the `--settings` flag. **It is not accepted from project or local settings**, since both files live inside the project directory and a cloned repository could supply either to redirect auto memory writes to sensitive locations." (Security note: a hostile repo cannot redirect your memory writes.)
- Auto-memory **loads regardless of `settingSources`** — it is in the "read regardless" tier with managed policy settings and `~/.claude.json` (topic 02 §8; docs `claude-code-features` table). To suppress it in a multi-tenant SDK process you MUST set `autoMemoryEnabled: false` / `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`; `settingSources: []` alone does not stop it. Docs `<Warning>` verbatim: "For multi-tenant deployments, run each tenant in its own filesystem and set `settingSources: []` plus `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1` in `env`."

**Not in SDK source.** `grep -rni "autoMemory|DISABLE_AUTO_MEMORY|MemoryTool|memory_20250818"` over `claude-agent-sdk-python@0.2.87 src/` returns **nothing**. These knobs are CLI/settings-level, passed through `env`/settings, not first-class SDK options. The SDK has no typed `auto_memory_enabled` field. → ArkOS controls auto-memory through `env` and settings files, not through `ClaudeAgentOptions`.

**Observability handle.** The one SDK-typed surface that *names* memory is `ContextUsageResponse.memoryFiles` (`client.get_context_usage()`), source `types.py:787`:

> `memoryFiles: list[dict[str, Any]]` — "CLAUDE.md and memory files loaded, with path, type, and token counts."

This is read-only telemetry (the `/context` breakdown), not a write/query API. It tells you *which* memory files were loaded and their token cost — useful for budgeting, useless for retrieval.

---

### 2. The `memory` option / "Memory tool" — disambiguating two products

Topic 06 noted a per-subagent `memory` override. There are **two distinct things** that both get called "memory tool," and conflating them is the trap:

#### 2a. The per-subagent `memory` field (Agent SDK) — REAL, named, present

`AgentDefinition.memory: Literal["user", "project", "local"] | None = None` (Python source `types.py:94`; TS `memory?: "user" | "project" | "local"` per topic 06 §7). Added Python 0.1.0/0.1.x era (CHANGELOG: "AgentDefinition: Added `skills`, `memory`, and `mcpServers` fields (#684)").

This is **the same auto-memory mechanism, scoped to one subagent**, on a **separate path tree** from the main session's auto-memory. Docs `sub-agents` §"Enable persistent memory", verbatim:

> The `memory` field gives the subagent a persistent directory that survives across conversations. The subagent uses this directory to build up knowledge over time, such as codebase patterns, debugging insights, and architectural decisions.

Scope → location table (docs `sub-agents`, **verbatim**):

| Scope     | Location                                      |
| :-------- | :-------------------------------------------- |
| `user`    | `~/.claude/agent-memory/<name-of-agent>/`     |
| `project` | `.claude/agent-memory/<name-of-agent>/`       |
| `local`   | `.claude/agent-memory-local/<name-of-agent>/` |

Note: this is `agent-memory/`, **distinct** from the main session's `~/.claude/projects/<project>/memory/`. Per-agent memory is keyed by *agent name*, not by repo project key.

Behavior when enabled (docs `sub-agents`, verbatim):

> - The subagent's system prompt includes instructions for reading and writing to the memory directory.
> - The subagent's system prompt also includes the first 200 lines or 25KB of `MEMORY.md` in the memory directory, whichever comes first, with instructions to curate `MEMORY.md` if it exceeds that limit.
> - Read, Write, and Edit tools are automatically enabled so the subagent can manage its memory files.

Format = flat markdown `MEMORY.md` + topic files, identical mechanism to §1. Recommended default scope is `project` (docs: "makes subagent knowledge shareable via version control"). It is **opt-in** (omitted → no persistent memory; the subagent still inherits the main session's CLAUDE.md/memory per topic 06 §6, but has no *write-back* directory of its own).

→ **For ArkOS:** the `memory` field is the closest thing the SDK has to a "give this role a durable scratchpad." It is per-agent-name, flat-file, model-curated. It is NOT a structured store and NOT queryable except by the model reading files.

#### 2b. The API Memory tool `memory_20250818` (Messages API) — NOT in the Agent SDK

There is a **first-class Memory tool in the Anthropic *Messages API*** — type **`memory_20250818`** — launched beta 2025-09-29. It is client-side: Claude issues structured commands (`view`, `create`, `str_replace`, `insert`, `delete`, `rename`) against a `/memories` directory, and *you* implement the backend (subclass `BetaAbstractMemoryTool` in the Python *Anthropic SDK*, or `betaMemoryTool` in the TS Anthropic SDK). It requires beta header `context-management-2025-06-27`.

**This is a different SDK and a different product.** It lives in `anthropic` / `@anthropic-ai/sdk` (the raw Messages API client), **not** in `claude-agent-sdk` / `@anthropic-ai/claude-agent-sdk` (the Agent SDK). Verification: `grep -rni "memory_20250818|BetaAbstractMemoryTool|betaMemoryTool|/memories|context-management|context_management|clear_tool_uses"` over `claude-agent-sdk-python@0.2.87 src/` returns **zero matches**. The Agent SDK does not expose `memory_20250818`, does not let you register a `BetaAbstractMemoryTool` backend, and does not surface the `/memories` command protocol.

→ **Verbatim names to keep straight:**
> - Agent SDK durable memory = **`AgentDefinition.memory`** (subagent) + **Claude Code auto-memory** (`MEMORY.md` dir). Both are *file-tool-mediated flat markdown*, not a tool-call protocol.
> - Messages-API Memory tool = **`memory_20250818`** + **`BetaAbstractMemoryTool`** / **`betaMemoryTool`**, `/memories` dir, beta header `context-management-2025-06-27`. **Not reachable through the Agent SDK at this snapshot.**

If ArkOS wants the structured `memory_20250818` command protocol, it must call the Messages API directly (bypassing the Agent SDK) or build an MCP server that re-implements the same commands as MCP tools (see §5). The Agent SDK gives no shortcut.

---

### 3. CLAUDE.md / AGENTS.md as persistent project context

CLAUDE.md is **author-written persistent context**, distinct from session memory and from auto-memory:

| | Session transcript (topic 02) | Auto-memory (`MEMORY.md`) | CLAUDE.md / rules |
| --- | --- | --- | --- |
| Author | the conversation | the **model** | **you** (human) |
| Recall mechanism | explicit `resume`/`fork` by ID | auto-load head + model reads topic files | auto-load full at session start |
| Scope key | session ID | git repo project key | cwd + parents (per `settingSources`) |
| Loaded as | (not auto-loaded into new sessions) | system-prompt startup block | **user message after system prompt** |
| Mutable by host? | no (opaque) | indirectly (edit files) | yes (you own the file) |

CLAUDE.md / `.claude/rules/*.md` load via `settingSources` (topic 02 §8): omit → user+project+local; `[]` → none. They load **in full** ("CLAUDE.md files are loaded in full regardless of length" — docs `memory`), unlike `MEMORY.md`'s 200-line/25 KB cap. For ArkOS this is the deterministic channel: write durable project facts to CLAUDE.md and they appear verbatim every session, no model judgement involved. (Detailed loading mechanics — load order, `@import` syntax, `claudeMdExcludes`, managed policy CLAUDE.md — belong to neighbor topic 11; cited here only to contrast with memory.)

#### AGENTS.md — confirming and restating topic 02's finding

Topic 02 §8 marked AGENTS.md as "NOT FOUND / undocumented for this SDK." The dedicated memory docs now **confirm this explicitly and positively** — it is documented behavior, not just an absence. Docs `code.claude.com/docs/en/memory` §"AGENTS.md", **verbatim**:

> Claude Code reads `CLAUDE.md`, not `AGENTS.md`. If your repository already uses `AGENTS.md` for other coding agents, create a `CLAUDE.md` that imports it so both tools read the same instructions without duplicating them.

The documented bridge patterns:

```markdown
@AGENTS.md

## Claude Code
Use plan mode for changes under `src/billing/`.
```
or a symlink (`ln -s AGENTS.md CLAUDE.md`), or `/init` in a repo with an existing `AGENTS.md` (which "reads it and incorporates the relevant parts into the generated `CLAUDE.md`").

**Current truth (restated):** The Claude Agent SDK (and Claude Code) do **not** load `AGENTS.md` directly. AGENTS.md only reaches the model if a `CLAUDE.md` `@`-imports it or is symlinked to it. This is now an *affirmative documented statement* ("reads `CLAUDE.md`, not `AGENTS.md`"), upgrading topic 02's "undocumented/not-found" to "documented as not-loaded." → For Ark, whose Codex/OpenCode templates use `AGENTS.md` as the CLAUDE.md analog: on the Claude surface, AGENTS.md is inert unless bridged.

---

### 4. What is NOT provided (the BYO inventory)

The SDK provides flat-file, model-curated, repo-scoped memory and nothing more. None of the following exist in `claude-agent-sdk@0.2.87`:

- **Semantic / vector memory.** No embeddings, no similarity search, no `memory.search(query)` API. Auto-memory recall is the model opening files by name, not vector retrieval. (Confirmed by absence in source + docs; `memoryFiles` is token telemetry only.)
- **Episodic recall / experience index.** No structured "what happened in past sessions" store beyond raw JSONL transcripts, which are keyed by session ID, not by content, and explicitly *not* meant to be parsed (topic 02 §5: "internal JSONL format … not guaranteed to remain stable"). There is no index that answers "show me sessions where the build failed."
- **Cross-project knowledge base.** Auto-memory is **per-repository** by design (docs: "Per repository, shared across worktrees"; "Files are not shared across machines or cloud environments"). `user`-scope subagent memory (`~/.claude/agent-memory/<name>/`) is the *only* cross-project surface, and it is per-agent-name flat markdown, not a KB. No global, queryable knowledge store.
- **Retrieval / RAG plumbing.** No document chunking, no retriever, no reranker, no context-assembly service. Retrieval is whatever the model does with file tools.
- **Procedural-memory store.** Skills (`.claude/skills/*/SKILL.md`, topic 11) are the closest analog to procedural memory, but they are author-written filesystem artifacts with no programmatic registration API and no auto-acquisition — the model cannot *learn* a new skill into the store; it can only write prose into auto-memory.
- **Working-memory service.** Working memory = the live context window. There is no API to snapshot/restore/branch working memory independent of a session resume/fork. Compaction (§6) is the SDK's only context-management lever, and it is lossy.

**What the docs explicitly say about limits** (load-bearing quotes):

- Memory is "context, not enforced configuration" (docs `memory`): "Claude treats them as context, not enforced configuration. … there's no guarantee of strict compliance."
- Auto-memory is machine-local and non-distributed (docs `memory`): "Auto memory is machine-local. … Files are not shared across machines or cloud environments."
- The `MEMORY.md` auto-load is capped: "first 200 lines or 25KB, whichever comes first … Content beyond that threshold is not loaded at session start." Anything past the cap is invisible until the model chooses to read it. There is no "load the most *relevant* 200 lines" — it is the literal head of the file.

→ **For ArkOS:** working / episodic / semantic / procedural memory as a substrate service must be **built**. The SDK contributes: (a) raw transcripts for episodic *raw material* (un-indexed), (b) auto-memory as a per-repo model scratchpad, (c) the `SessionStore` mirror (topic 02 §7) as a *transport* for getting transcripts into your own backend. None of these is a memory *service*; they are inputs to one you write.

---

### 5. Extension points — how a host plugs in its own memory layer

There is no `MemoryProvider` interface to implement. The practical avenues, in rough order of leverage:

**(a) Custom MCP tools — the structured-memory path (cross-ref topic 07).** Publish an in-process SDK MCP server (`create_sdk_mcp_server` / `createSdkMcpServer`, topic 07) exposing tools like `memory_search`, `memory_write`, `memory_recall`. These run in your host process, can hit a vector DB / Postgres / KB, and return structured results the model can call by name. This is the way to give the agent *semantic/episodic* recall the SDK lacks — you re-implement retrieval as tools. (You could even re-implement the `memory_20250818` command set as MCP tools if you want that exact protocol, since the Agent SDK won't give you the native one — §2b.) Cost: every tool description occupies context (topic 07); use `ToolSearch`/deferred schemas to keep them out of the prefix until needed.

**(b) Inject context into the system prompt or first user turn.** `systemPrompt` (string, or preset `+ append`, or `{type:"file"}`) lets the host write retrieved memory directly into the prompt at session start — host-controlled, deterministic, no model judgement. Pair with `exclude_dynamic_sections` (Python `SystemPromptPreset`, source `types.py:42`) when many sessions share a preset so the cache prefix hits cross-user: "Strip per-user dynamic sections (working directory, auto-memory, git status) … re-injected into the first user message." This is the lever for a host that runs its *own* retrieval and wants to feed results in, rather than letting the model fish in `MEMORY.md`.

**(c) Write to the auto-memory / CLAUDE.md files the agent loads.** Since auto-memory and CLAUDE.md are plain markdown on a known path, the host can pre-seed `~/.claude/projects/<project>/memory/MEMORY.md` or `CLAUDE.md` before a `query()` and the agent loads it deterministically. Crude but real — the same channel topic 06 §6.2 noted for handing a subagent exact inputs ("write those inputs to disk which the subagent loads deterministically").

**(d) `SessionStore` adapter as memory transport (cross-ref topic 02 §7).** The store mirrors transcript entries to your backend (`append`/`load`). It is best-effort and a *mirror*, not a memory API — but it is the supported way to get every turn into S3/Redis/Postgres where your *own* indexer can build an episodic/semantic layer on top. The SDK won't index it; it just hands you the stream.

**(e) Per-subagent `memory` for role-scoped scratchpads (§2a).** When a role genuinely benefits from model-curated cross-session notes (a reviewer accumulating recurring-issue patterns), `AgentDefinition.memory: "project"` is the zero-code option. Bounded by the 200-line/25 KB auto-load and model curation discipline.

**No interface to implement.** Unlike `SessionStore` (a typed Protocol) or MCP servers (a typed transport), there is **no memory Protocol** in the SDK. A host cannot register a "memory backend" the way it registers a session store. The extension story is "expose memory as tools (a) or inject it as context (b/c)." Both put the host in the retrieval seat.

---

### 6. Compaction interaction — what happens to memory when a session compacts

Compaction is triggered manually (`/compact`) or automatically (autocompact at a token threshold; `ContextUsageResponse.isAutoCompactEnabled` / `autoCompactThreshold`, source `types.py:784,799`). The **`PreCompact`** hook (topic 04) fires first; SDK input shape (source `types.py:362`):

```python
class PreCompactHookInput(BaseHookInput):
    hook_event_name: Literal["PreCompact"]
    trigger: Literal["manual", "auto"]
    custom_instructions: str | None
```

So a host learns compaction is imminent and which kind, and can read `custom_instructions` — the documented use is "Archive transcript before summarizing" (topic 04). PreCompact is the host's one chance to snapshot live context elsewhere before it is summarized.

**What survives compaction** (docs `context-window`, takeaway string, **verbatim**):

> Compaction replaces the conversation with a structured summary. System prompt, CLAUDE.md, memory, and MCP tools reload automatically. The skill listing is the one exception. Only skills you actually invoked are preserved.

And per docs `memory` §"Instructions seem lost after `/compact`", **verbatim**:

> Project-root CLAUDE.md survives compaction: after `/compact`, Claude re-reads it from disk and re-injects it into the session. Nested CLAUDE.md files in subdirectories are not re-injected automatically; they reload the next time Claude reads a file in that subdirectory.
> If an instruction disappeared after compaction, it was either given only in conversation or lives in a nested CLAUDE.md that hasn't reloaded yet.

So under compaction:

- **Auto-memory and project-root CLAUDE.md are re-injected** from disk — they are durable and survive because they reload, not because they are preserved in the summary. (This is precisely why model-curated memory matters: a fact written to `MEMORY.md` survives compaction; a fact only stated in chat may not.)
- **Nested/subdir CLAUDE.md** is NOT re-injected; it reloads lazily on next read in that subtree.
- **Skill listing** is the documented exception — only invoked skills are kept.
- **Conversation-only context is summarized** — replaced by a structured summary. This is **lossy in-context**: the live window now holds a summary, not the original turns.

**Is the compacted detail recoverable?** Yes — on disk, not in context. The pre-compaction turns are **not deleted from the JSONL transcript**; compaction appends a summary record rather than rewriting history. Evidence: the transcript line schema carries an **`isCompactSummary`** boolean (topic 02 §5 field list; SDK source `session_summary.py:76,83` skips `isCompactSummary` entries when deriving titles), and the `parentUuid` chain (topic 02 §5) preserves the original `user`/`assistant` records before the summary node. So:

- **In the live context window:** compacted detail is *lost* (replaced by summary). The model can no longer see it without re-reading.
- **On disk (the JSONL):** the original turns *remain*, with a summary record marked `isCompactSummary` layered in. A host that captured the transcript (directly, or via `SessionStore` mirror, or in a PreCompact hook) can recover the full pre-compaction history. `get_session_messages` walks the full `uuid`/`parentUuid` tree (topic 02 §6), so the raw turns are still enumerable.

→ **For ArkOS:** compaction is the SDK's only working-memory-management lever, and it is **lossy in-context but recoverable on-disk**. If a substrate needs durable episodic recall across compactions, it must (a) persist what matters to auto-memory *before* compaction so it reloads, and/or (b) capture the transcript via `SessionStore`/PreCompact and index it externally. Relying on the in-context summary alone loses detail. Note also (open question carried from topic 04): whether `PostToolUse` `updatedToolOutput` is written to the persisted JSONL or only shown in-memory is undocumented — relevant if replaying transcripts to reconstruct compacted state.

---

## Caveats / Not found

- **API Memory tool `memory_20250818` is NOT in the Agent SDK.** Confirmed by empty `grep` over `claude-agent-sdk-python@0.2.87 src/` for `memory_20250818`, `BetaAbstractMemoryTool`, `betaMemoryTool`, `/memories`, `context-management`. It is a *Messages API* feature (the `anthropic` SDK), a separate product. Do not assume the Agent SDK can register a `memory_20250818` backend. If ArkOS wants that protocol, call the Messages API directly or re-expose the commands as MCP tools.
- **No `auto_memory_enabled` / `auto_memory_directory` typed SDK option.** These are settings-file / env knobs (`autoMemoryEnabled`, `autoMemoryDirectory`, `CLAUDE_CODE_DISABLE_AUTO_MEMORY`), passed through `env`/settings, not fields on `ClaudeAgentOptions`. Verified by source absence.
- **TS-vs-Python divergence on memory: none material found.** `AgentDefinition.memory` exists in both (Python `types.py:94`; TS per topic 06). Auto-memory is a CLI/settings feature shared by both surfaces. The one *general* persistence divergence (Python always persists sessions to disk; TS has `persistSession: false`) is topic 02's, not memory-specific. The `SessionSummaryEntry`/`fold_session_summary` incremental-summary helper (source `_internal/session_summary.py`) is a *SessionStore* utility (for adapters to maintain per-session display summaries), **not** a memory feature — flagged so it is not mistaken for episodic memory; its `data` field is "opaque SDK-owned state … MUST NOT interpret."
- **`memoryFiles` in `ContextUsageResponse` is telemetry, not an API.** It reports loaded CLAUDE.md/memory files + token counts (the `/context` breakdown). It cannot write or query memory.
- **AGENTS.md not-loaded is now affirmatively documented** (docs `memory`: "reads `CLAUDE.md`, not `AGENTS.md`"), upgrading topic 02's "undocumented/not-found." The bridge is a `@AGENTS.md` import or symlink in CLAUDE.md.
- **Auto-memory write *cadence* / heuristic is not precisely documented.** Docs say the model "decides what's worth remembering" but give no rule, threshold, or guaranteed-write event. Treat auto-memory as best-effort, model-discretionary — not a reliable write-through cache.
- **Compaction summary fidelity is not specified.** Docs say conversation is "replaced with a structured summary"; the summary's completeness/format is not contracted. On-disk recovery (via `isCompactSummary`-marked transcript + parent chain) is the reliable path, not the summary.
- **TS source not inspected for this file.** Memory claims for TS rest on docs + topic 06's earlier TS reading; only the Python SDK clone (`/private/tmp/claude-agent-sdk-python@0.2.87`) was grepped here. The `AgentDefinition.memory` field and auto-memory behavior are documented as cross-SDK; confirm in TS source if a byte-exact field check matters.
- **Doc snapshot only:** `code.claude.com` pages print no per-page "last updated" date; version pin is the bundled-CLI 2.1.150 (Python 0.2.87) and the docs fetched 2026-05-25.
