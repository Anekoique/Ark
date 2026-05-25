# Research: Claude Agent SDK — limits and gaps (the "what a substrate must build" inventory)

- Query: compile the definitive inventory of what the Claude Agent SDK does NOT provide that a multi-agent workflow substrate (ArkOS stage-1) must implement itself. Synthesize the limitations already flagged across corpus files 01–09, plus research the gaps no per-topic file covered (observability, multi-user/access control, grounding/evaluation).
- Scope: mixed — primarily synthesis of the corpus (files 01–09), with fresh confirmations for categories not owned by any per-topic file (observability §8, multi-user §9, grounding/eval §10).
- Date: 2026-05-25
- Version pin (snapshot): Python `claude-agent-sdk` **0.2.87** (PyPI, 2026-05-23; bundles Claude CLI 2.1.150), TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.150** (npm, 2026-05-23). Both latest at snapshot. No newer release observed.

## 0. How to read this file

This is the **gap inventory**, not a design document. It lists, by substrate concern, what the SDK does NOT do, cites which corpus file established each fact (or marks it fresh-researched here), and gives one line on what a substrate must build. It makes **no architecture choices** — that is the SYNTHESIS file (99) and the follow-up ArkOS task.

The framing of the whole corpus (file 01 §1): the Agent SDK is "**Claude Code as a library**" — it provides the agent loop, tool inventory, hooks, session model, MCP client, and one level of subagent. It is **not** an agent-orchestration framework, a task system, a memory layer, a distributed runtime, or a multi-tenant platform. Everything in those categories is the substrate's job.

Each gap is tagged with a confidence marker carried over from the source files: `[DOC]` stated in docs, `[SRC]` read from SDK source, `[ISSUE]` from a GitHub issue, `[INFERRED]` deduced. Where a fact is newly confirmed in this file (not in 01–09) it is marked `[FRESH 2026-05-25]`.

---

## 1. Orchestration

The SDK gives one model-driven agent loop with one optional level of subagents. It has no concept of a workflow, a graph, a pipeline, or host-deterministic dispatch.

### 1.1 No multi-agent DAG / graph / workflow framework
- **SDK does NOT:** provide any orchestration DSL, graph/DAG runner, pipeline primitive, or workflow engine. `query()` runs one agent loop to a single terminal `ResultMessage`; that is the entire unit of execution. There is no "stage → stage → stage" or "if A then B" construct.
- **Established by:** file 06 §8 (subagent vs separate `query()` comparison — the only composition primitives are "spawn a subagent" or "open another `query()`"); file 09 §6 (fan-out/join is hand-written `asyncio.gather`/`Promise.all`, not an SDK primitive).
- **Substrate must build:** the workflow state machine itself — phase sequencing, conditional transitions, the task lifecycle. (Ark already has this in `ark agent task`'s legal-transition table; the SDK contributes nothing here.)

### 1.2 Dispatch is model-decided, not host-deterministic
- **SDK does NOT:** let host code force "run agent X now" inside a single `query()`. Subagents are invoked by the **parent model** calling the built-in `Agent` tool when it judges a subtask matches an agent's `description`. The two host levers — prompt phrasing ("Use the code-reviewer agent…") and `allowedTools` gating — are **steering, not forcing**; even "explicit invocation" is mediated by the model's tool-call decision.
- **Established by:** file 06 §2 ("Invocation is MODEL-DECIDED, not host-forced"); file 06 §2.3 (the only deterministic path is running a whole session *as* the agent via `--agent`, i.e. a separate `query()`).
- **Substrate must build:** deterministic role dispatch by opening a **separate `query()`** per role (`--agent`/`agent` setting pins the session as that agent). The host owns "run agent X now."

### 1.3 Recursion depth = 1 (subagents cannot spawn subagents)
- **SDK does NOT:** allow nested delegation. Three verbatim doc statements confirm "Subagents cannot spawn their own subagents." The experimental fork mode is also capped ("A fork cannot spawn further forks").
- **Established by:** file 06 §4 (confirmed definitively); file 03 §8.4 (`parent_tool_use_id` is at most one level deep).
- **Substrate must build:** the recursive task spine host-side — when a sub-task needs sub-sub-tasks, the host opens a new `query()` per level and owns the call graph, depth tracking, and per-level budget/cancellation.

### 1.4 No fan-out / join primitive
- **SDK does NOT:** provide a structured fan-out/join. In-session subagent fan-out is model-driven (the model decides the degree), text-only on return, with **no documented numeric concurrency cap**, and all results bloat back into the one parent context (capped by the parent context window).
- **Established by:** file 06 §5 (model-driven fan-out, no documented limit, parent-context-capped); file 09 §6 (deterministic fan-out is host-orchestrated `asyncio.gather`/`Promise.all` over independent `query()` calls — not an SDK feature).
- **Substrate must build:** the fan-out/join orchestrator — dispatch N independent `query()` calls (each with its own cwd/worktree), bound the degree with a semaphore, collect each `ResultMessage` host-side, integrate. "First-result-wins" and "at most K concurrent" are both host code.

---

## 2. Task structure

The SDK has a *session* (a conversation transcript) and an in-session *task tracker tool* (`TaskCreate`/`TaskUpdate`, model-facing todo items). Neither is a task tree, and there is no relationship model between sessions.

### 2.1 No task tree / no parent-child task relationships
- **SDK does NOT:** model tasks as a tree or graph of related units. `TaskCreate`/`TaskUpdate` are **model-facing todo-tracking tools** (file 05 §1 orchestration category), not a host-queryable task store with parent/child edges. Sessions relate only via `resume` (linear continuation) and `fork` (one branch point) — neither expresses a task hierarchy. Subagent transcripts nest under a session (`<sessionId>/subagents/agent-<id>.jsonl`, file 02 §5) but that is a transcript-storage detail, not a typed task-relationship API.
- **Established by:** file 05 §1 (`TaskCreate`/`TaskUpdate` are tools, not a store); file 02 §2–§3 (resume/fork are the only session relationships); file 06 §4 (one level of subagent only).
- **Substrate must build:** the task tree itself — parent/child task records, the relationship store, traversal. (Ark already has this in `.ark/tasks/<slug>/` + `task.toml`; the SDK contributes nothing.)

### 2.2 No focus tracking / no "current task" concept
- **SDK does NOT:** track which task is "active" or maintain any host-level focus pointer. The SDK knows only the current `query()`'s `session_id`; it has no notion of a selected task a host orchestrator is working within.
- **Established by:** file 02 (sessions are keyed by UUID + cwd-derived project key; there is no "focus" field anywhere in `SDKSessionInfo` — §6); file 01 §5 (the session helpers are list/get/rename/tag, no "current").
- **Substrate must build:** focus/orientation state. (Ark's `ark context` + checkout focus-slug is exactly this; the SDK has no equivalent.)

### 2.3 No cross-task / cross-session shared state
- **SDK does NOT:** share state between sessions. Each `query()` is stateless and independent (file 02 §0: "Each query is independent, no conversation state"); even `resume`d calls within one logical session report cost independently and do not auto-accumulate (file 08 §6.1). There is no shared scratchpad, no inter-session variable store.
- **Established by:** file 02 §0; file 08 §6.1; file 09 §2.3 ("no shared settings cache, no shared HTTP client" in-process — the only shared surfaces are unintended global ones).
- **Substrate must build:** any cross-task state — shared artifacts, a blackboard, pass-the-baton handoff data. The SDK's only inter-session channel is what the host writes to disk and re-reads.

---

## 3. Memory

The SDK persists conversation transcripts (JSONL) and loads a project-instruction file plus an auto-memory file into context. It has no semantic/retrieval memory of any kind.

### 3.1 No semantic / episodic / vector memory; no retrieval
- **SDK does NOT:** provide embeddings, a vector store, semantic search, episodic recall, or any retrieval layer. What it persists is the **raw conversation JSONL** (file 02 §5) and what it loads is `CLAUDE.md` + `.claude/rules/*.md` + an auto-memory markdown file (`~/.claude/projects/<project>/memory/`, loaded verbatim into the system prompt — file 02 §8). "Memory" in the SDK = files-injected-as-text, not retrieval.
- **Established by:** file 02 §5 (persistence is JSONL transcripts only); file 02 §8 (auto-memory is a markdown file loaded into the prompt; the `memory` field on `AgentDefinition` is a per-agent memory *directory*, file 06 §7 — still file-based, not retrieval).
- **Substrate must build:** the entire memory/retrieval layer — embeddings, vector index, chunking, relevance ranking, any KB. The SDK gives a transcript and a text-file injection point, nothing more.

### 3.2 No cross-project knowledge base
- **SDK does NOT:** maintain knowledge that spans projects. Sessions are partitioned by cwd-derived project key (`~/.claude/projects/<encoded-cwd>/`, file 02 §5); auto-memory is per-project. There is no cross-project recall, no global KB surface.
- **Established by:** file 02 §5, §8, §9 (everything is project-keyed by cwd; sessions are machine-local).
- **Substrate must build:** any cross-project/global knowledge store and the retrieval that reads it into a prompt.

### 3.3 Session JSONL is the only automatic persistence, and it is opaque
- **SDK does NOT:** expose a parseable, stable memory record — the JSONL line schema is **private and explicitly unstable** ("implementation details not guaranteed to remain stable"). It must be read via the SDK functions, never parsed directly.
- **Established by:** file 02 §5 ("PRIVATE / unstable… do NOT parse the JSONL directly").
- **Substrate must build:** its own durable, query-able memory records if it needs structured recall — it cannot lean on the transcript format as a memory store.

---

## 4. State & persistence

Sessions persist, but machine-locally, in a private format, with the durable store being a best-effort mirror of a mandatory local write.

### 4.1 Sessions are machine-local by default
- **SDK does NOT:** make sessions portable across hosts by default. A session lives at `~/.claude/projects/<encoded-cwd>/<id>.jsonl` on the machine that created it; to resume you must replay the **same `cwd`** (so the project key matches) **and** have the JSONL present locally. "Sessions are machine-local by default."
- **Established by:** file 02 §9 ("Cross-host resume requires either moving the JSONL to the identical path or using a store adapter. Sessions are machine-local by default"); file 02 §5.
- **Substrate must build:** cross-host session transport — a `SessionStore` adapter or explicit file movement — plus cwd-replay discipline.

### 4.2 The persisted format is private and unstable
- **SDK does NOT:** publish the JSONL line schema as a contract. (Same fact as §3.3, restated under persistence.) `SessionStore` adapters treat lines as opaque deep-equal blobs.
- **Established by:** file 02 §5, §7.
- **Substrate must build:** read access only via SDK functions (`get_session_messages`, etc.); never a direct-parse dependency.

### 4.3 No distributed / shared state store; `SessionStore` is mirror-not-source
- **SDK does NOT:** offer a distributed or shared state backend as a first-class store. The `SessionStore` adapter (S3/Redis/Postgres reference impls exist) is a **dual-write mirror**: the subprocess always writes local disk first, *then* forwards to `append()`, **best-effort, not retried** (a failed batch emits a `mirror_error` system event and the query continues). A truly disk-free backend is not the design.
- **Established by:** file 02 §7 ("Dual-write / mirror, not replacement… Best-effort… Failed batches are NOT retried"); file 03 §1.3 (`SDKMirrorErrorMessage`).
- **Substrate must build:** treat the store as durable redundancy + cross-host transport, not source of truth; monitor `mirror_error`; own its own authoritative state if it needs one.

### 4.4 No automatic pruning / GC / retention; no native age filter
- **SDK does NOT:** delete from the store ever ("The SDK never deletes from store; implement TTLs, S3 lifecycle policies, or scheduled cleanup"). Session listing has no native age filter (filter `created_at`/`last_modified` client-side).
- **Established by:** file 02 §6.
- **Substrate must build:** retention, GC, and any age-based queries.

---

## 5. Portability / interop

The SDK is an MCP *client* and an in-process tool host, bound to Claude. It cannot publish a server, surfaces only MCP tools (not resources/prompts), does not load `AGENTS.md`, and locks to one model vendor.

### 5.1 Cannot publish an MCP server (consume only)
- **SDK does NOT:** publish a standalone MCP server that *external* agents dial into. `create_sdk_mcp_server`/`createSdkMcpServer` produces an **in-process tool host consumed only by THIS SDK process's own agent** — it is never bound to a listening transport (no port, no `listen()`, no publish API; the `instance` is stripped before crossing any boundary). Confirmed from source, not inferred.
- **Established by:** file 07 §5 ("Definitively no, and confirmed from source").
- **Substrate must build:** to expose substrate primitives to foreign agents, a **separate** MCP server against standalone `mcp`/`fastmcp`/`@modelcontextprotocol/sdk` bound to a transport. The Agent SDK sits only on the client side of that boundary.

### 5.2 MCP consumption is tools-only (no resources, no prompts)
- **SDK does NOT:** surface MCP **resources** or **prompts** to the model — only **tools**. The in-process JSON-RPC bridge hard-codes exactly `initialize`/`tools/list`/`tools/call` and advertises only the `tools` capability; there is no `resources/list` or `prompts/get`, and no documented surface for binding an external server's resources/prompts.
- **Established by:** file 07 §3 ("TOOLS ONLY… treat MCP-as-consumed-by-the-SDK as 'tools only'").
- **Substrate must build:** if it needs MCP resources/prompts, fetch them itself (its own MCP client) and inject as tool output or prompt text; do not expect the SDK to bind them.

### 5.3 No OAuth for remote MCP; headers only
- **SDK does NOT:** handle OAuth flows for HTTP/SSE MCP servers — auth is HTTP headers only; the caller runs the OAuth dance and injects the bearer token. No token refresh, no discovery.
- **Established by:** file 07 §2.3.
- **Substrate must build:** the OAuth/token-refresh machinery for any authed remote MCP server.

### 5.4 `AGENTS.md` is not loaded
- **SDK does NOT:** load `AGENTS.md` via `settingSources` — no settings table, no source loader, no docs page names it. The SDK's project-instruction file is `CLAUDE.md` (+ `.claude/rules/*.md`, `CLAUDE.local.md`). `AGENTS.md` is the OpenCode/Codex-ecosystem convention; treated as undocumented for this SDK.
- **Established by:** file 02 §8 ("explicit gap… no evidence the Claude Agent SDK loads `AGENTS.md`").
- **Substrate must build:** if cross-runtime instruction parity matters, the host loads `AGENTS.md` itself (e.g. concatenate into the prompt or mirror to `CLAUDE.md`); verify empirically before relying on it.

### 5.5 Vendor lock to Claude
- **SDK does NOT:** abstract over model providers. It is Claude-only; the model knob takes Claude family identifiers, and the four "providers" (Bedrock / Vertex / Azure Foundry / Claude-Platform-on-AWS) are all hosting surfaces for Anthropic-built Claude weights. The `Transport` ABC swaps *how messages reach the Claude Code subprocess*, not the model API. No adapter for OpenAI/Gemini/Llama/etc.
- **Established by:** file 01 §4 ("Claude-only… If a substrate needs cross-model support, that abstraction must live above the Agent SDK").
- **Substrate must build:** any cross-model abstraction above the SDK. Also note the claude.ai subscription auth path is unavailable to third-party SDK integrators (file 01 §3) — substrate must use API-key/cloud-provider auth.

---

## 6. Concurrency safety

N concurrent `query()` calls work (structurally), but the SDK provides no coordination for the host-global surfaces they share, and (Python) does not retry 429.

### 6.1 No file-write coordination between concurrent sessions
- **SDK does NOT:** lock, transaction, or conflict-detect file writes across concurrent sessions. Two sessions in the same `cwd` share the git working tree and the on-disk project key; overlapping `Edit`/`Write`/`Bash` writes interleave at the OS level with **no SDK-level lock**. The hosting docs admit it: "you will have to prevent agents from overwriting each other." Permission rules are also not OS enforcement — they miss arbitrary subprocesses that open files themselves (file 05 §5).
- **Established by:** file 09 §3 ("cwd collision = silent file-write race… zero file isolation between concurrent same-cwd sessions"); file 05 §5, §8 (rules ≠ OS enforcement; sandbox is Bash-only/opt-in/not-a-complete-boundary).
- **Substrate must build:** a distinct `cwd`/worktree per concurrent writing session (Ark's worktree-per-task), plus any write-coordination/locking. The SDK won't prevent races.

### 6.2 `~/.claude/` config-dir leak across calls in one process
- **SDK does NOT:** isolate per-session CLI runtime state by default — the `~/.claude/` config dir and `~/.claude.json` are process-global, and state written by one call is visible to (and races with) the next (e.g. a `firstStartTime` marker breaks `TRACEPARENT` nesting on the 2nd+ call, #952).
- **Established by:** file 09 §2.1 (`[ISSUE]` python#952); file 02 §8 (multi-tenant warning: "Do not rely on default `query()` options for multi-tenant isolation").
- **Substrate must build:** per-session config isolation — set `CLAUDE_CONFIG_DIR`/`HOME` per call in `options.env` (the issue author's workaround, not a documented isolation API).

### 6.3 No built-in 429 retry (Python 0.2.87 crashes)
- **SDK does NOT:** retry on rate-limit (429) in Python 0.2.87 — it **crashes fatally** (subprocess exit code 1 with the 429 body in stderr; the fix PR #973 is open/unmerged). The SDK also does not queue/throttle/admission-control concurrent sessions against the shared org rate-limit budget. (TS retry was not separately confirmed; assume none.)
- **Established by:** file 09 §7 (`[ISSUE]` python#812 open, #973 unmerged); file 03 §9.2 (rate-limit event shape typed-but-undocumented, claude-code#26392).
- **Substrate must build:** host-side concurrency cap (semaphore sized to the API tier), 429 detection + backoff (respect `Retry-After`), and resume-by-`session_id` after a rate-limited crash.

### 6.4 Shared `atexit` registry tears down all sessions together
- **SDK does NOT:** offer per-session opt-out of the process-global `_ACTIVE_CHILDREN` cleanup set — its `atexit` handler SIGTERMs **every** live child at once, so a process-wide exit/fatal kills all concurrent sessions together. (Benign for normal teardown; a blast-radius hazard with a fatal — cf. TS #318 EPIPE killing the host process.)
- **Established by:** file 09 §2.2, §2.4, §8.3 (`[SRC]` + `[ISSUE]` typescript#318).
- **Substrate must build:** explicit per-session close before exit (don't rely on `atexit`); in TS, a defensive `uncaughtException` EPIPE filter; process-per-isolation-domain where blast radius matters.

### 6.5 Thread safety undocumented; async-cleanup is fragile
- **SDK does NOT:** document thread safety (docs explicitly silent). The supported model is many `query()` coroutines on one event loop (anyio/asyncio); multi-OS-thread use is unvalidated. The async-cleanup path is the SDK's most fragile area (a cluster of bugs around early `break`/cancel/back-to-back calls; docs warn against `break`-ing out of the iterator early).
- **Established by:** file 09 §5 (`[INFERRED]` + `[ISSUE]` python#890/#810/#746).
- **Substrate must build:** single-event-loop-per-interpreter discipline (or process-per-domain); cancel rather than `break`; verify clean teardown empirically before aggressive fan-out cancellation.

---

## 7. Budget & cost

The SDK has a per-`query()` budget cap and reports cost at the terminal result, but cost is a client-side estimate and there is no cumulative/session-level total.

### 7.1 Native budget cap is per-`query()` only (no cumulative)
- **SDK does NOT:** provide a session-level or cross-call cumulative budget. `maxBudgetUsd`/`max_budget_usd` caps a single `query()` (terminates with `error_max_budget_usd`); the docs state explicitly "The SDK does not provide a session-level total… accumulate the totals yourself" — even `resume`d calls within one session do not auto-sum.
- **Established by:** file 08 §4, §6.1.
- **Substrate must build:** host-side cumulative accumulation — read `total_cost_usd` off every `ResultMessage` (present on success *and* error), gate the next `query()` against the running total, set per-query `max_budget_usd` to remaining headroom as defense-in-depth.

### 7.2 Cost is a client estimate, not billing
- **SDK does NOT:** report authoritative cost — `total_cost_usd`/`costUSD` are client-side estimates from a price table bundled at build time, "not authoritative billing data" (drifts on pricing changes, unknown models → possibly 0, un-modeled billing rules). Also: `usage` and `modelUsage` disagree even for a single model; `total_cost_usd` reconciles with `modelUsage`, not `usage` (ts#112, open).
- **Established by:** file 08 §7 (estimate warning); file 08 §3.3 (`[ISSUE]` ts#112).
- **Substrate must build:** for real billing, the Usage and Cost API / Console; never bill end users from SDK fields. Track which SDK version produced a figure (price table is version-pinned). Trust `modelUsage`+`total_cost_usd` over `usage` for cost.

### 7.3 No live per-turn cost; per-subagent cost not itemized
- **SDK does NOT:** surface a live USD figure mid-`query()` (only per-step *tokens* live; cumulative cost only at `ResultMessage`), and does NOT itemize per-subagent cost (subagent cost folds into the parent `total_cost_usd`/`modelUsage`; only per-*model* attribution exists).
- **Established by:** file 08 §1.1, §8; file 06 §8.
- **Substrate must build:** for clean per-role cost, run each role as its own `query()` and read that query's total. Intra-`query()` cost-based abort is approximate (overshoots by up to one step); keep individual queries small.

---

## 8. Observability

The SDK can export OpenTelemetry, but produces no telemetry of its own and ships no dashboard/metrics backend — the host runs the collector and the analytics.

### 8.1 The SDK emits no telemetry itself; it only passes OTel config to the CLI
- **SDK does NOT:** produce telemetry of its own. `[FRESH 2026-05-25]` Verbatim from the observability page: "**The SDK does not produce telemetry of its own.** Instead, it passes configuration through to the CLI process, and the CLI exports directly to your collector." Telemetry is off until `CLAUDE_CODE_ENABLE_TELEMETRY=1` and an exporter is chosen; it is configured via env vars (process env or `options.env`).
- **Established by:** `[FRESH]` [observability doc](https://code.claude.com/docs/en/agent-sdk/observability); corroborated by file 01 §5 ("OpenTelemetry observability flags" listed as an SDK-only knob) and file 09 §2.1 (TRACEPARENT propagation, #952).
- **Substrate must build:** the OTel wiring (enable flags, exporter config, per-agent `OTEL_SERVICE_NAME`/resource attributes).

### 8.2 No built-in metrics store, dashboard, or trace backend
- **SDK does NOT:** ship any dashboard, metrics database, or trace UI. It exports OTLP (metrics / log events / traces) to an **external** backend the host must run (Honeycomb, Datadog, Grafana, Langfuse, self-hosted collector). Traces are **beta** ("Span names and attributes may change between releases"; require `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1`). Content (prompts, tool I/O) is *not* recorded by default — opt-in via `OTEL_LOG_*` vars.
- **Established by:** `[FRESH]` [observability doc](https://code.claude.com/docs/en/agent-sdk/observability) (OTLP-export-only; beta traces).
- **Substrate must build:** the collector + storage + dashboards; any metric aggregation, alerting, SLOs.

### 8.3 The event stream is the in-process trace; structured logging is thin
- **SDK does NOT:** offer a richer in-process observability surface than the typed event stream. To observe live, the host iterates `query()` and treats the `SDKMessage`/`Message` stream as the trace — but there is **no turn-end event** (detect turn boundary by the arrival of a typed `AssistantMessage`), the rate-limit event shape is typed-but-undocumented, and Python's `Message` union hides many CLI-emitted variants (hook-progress, status, retry, …) that ride the same wire but aren't typed in Python. Hook output is not surfaced in the stream by default (`include_hook_events` needed).
- **Established by:** file 03 §3 (no turn-end event), §9.2 (rate-limit shape undocumented), §1.3 (Python union narrower); file 04 §9 ("log hook decisions out-of-band rather than rely on stream surfacing").
- **Substrate must build:** its own event→trace mapping, turn-boundary detection, and out-of-band logging of hook decisions; do not depend on the rate-limit payload schema.

---

## 9. Multi-user / access control

The SDK is single-tenant by construction. It offers telemetry *labels* for end users but no auth, no isolation, no quotas.

### 9.1 No auth / no identity model
- **SDK does NOT:** authenticate or model end-user identity. It authenticates *to Anthropic* with one credential (API key / cloud provider); `[FRESH 2026-05-25]` the observability page is explicit that telemetry identity attributes "identify your service's credential, **not the end user** on whose behalf the agent acted." The claude.ai login path is unavailable to third-party integrators (file 01 §3).
- **Established by:** `[FRESH]` [observability doc](https://code.claude.com/docs/en/agent-sdk/observability) (credential ≠ end user); file 01 §3.
- **Substrate must build:** all authentication, the end-user identity model, and session→user binding.

### 9.2 No tenant isolation by default
- **SDK does NOT:** isolate tenants. The docs warn verbatim: "Do not rely on default `query()` options for multi-tenant isolation… For multi-tenant deployments, run each tenant in its own filesystem and set `settingSources: []` plus `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`." Process-global surfaces (`~/.claude/`, auto-memory, the config file) leak across calls unless explicitly partitioned per call.
- **Established by:** file 02 §8 (multi-tenant `<Warning>`); file 09 §2.1, §2.4 (shared config dir + atexit).
- **Substrate must build:** per-tenant filesystem + config-home isolation, hardened `settingSources`, auto-memory disable, and process/worktree partitioning. (The `SessionStore` `project_key` is meant to be set to a tenant ID — file 02 §7 — but that is storage scoping, not enforcement.)

### 9.3 No per-user quota / rate budget
- **SDK does NOT:** provide per-user (or per-tenant) quotas or rate budgets. The rate-limit budget is org-level and shared at Anthropic across all sessions; the SDK does not partition or meter it per user, and (Python) does not even retry 429 (§6.3).
- **Established by:** file 09 §7 (shared org budget, no throttle); file 08 (budget is per-`query()`, no cumulative — §7.1).
- **Substrate must build:** per-user/per-tenant quota accounting and admission control (combine the §7.1 cumulative-cost accumulator with the §6.3 host-side semaphore, keyed by user/tenant).

---

## 10. Grounding / evaluation

The natural SDK pattern is Claude-grades-Claude. The SDK ships no eval harness and no external-grader integration; it gives the hook points on which one is built.

### 10.1 No built-in eval harness / eval framework
- **SDK does NOT:** ship an evaluation harness, test runner for agent behavior, golden-dataset runner, or grading framework. `[FRESH 2026-05-25]` Anthropic publishes eval *patterns* (the "Demystifying evals for AI agents" engineering post) and a separate Console **Evaluation tool** (a prompt-testing product, not part of the Agent SDK); third-party harnesses (e.g. `claude-evals`, `claude-eval-toolkit`) are built **on top of** the SDK using its hooks (`PreToolUse`/`PostToolUse`/`SubagentStop`) and the event stream. The SDK itself provides none of this.
- **Established by:** `[FRESH]` web search 2026-05-25 ([Anthropic — Demystifying evals for AI agents](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents); [TribeAI/claude-evals](https://github.com/TribeAI/claude-evals) — "native SDK hooks"; [Console eval tool](https://platform.claude.com/docs/en/test-and-evaluate/eval-tool)).
- **Substrate must build:** the eval harness — task fixtures, the run loop, graders, aggregation. The SDK contributes only the hook points and the result/event stream a harness reads.

### 10.2 No external-grader integration
- **SDK does NOT:** integrate any external/deterministic grader or LLM-as-judge service. There is no grading API, no rubric runner, no pass/fail verdict surface. Grounding feedback into a run is achievable only by host code: a `PostToolUse` hook returning `additionalContext` (append) or `updatedToolOutput` (replace) is the only SDK surface that can splice grounding signal ("tests still failing", "spec says X") the model reads next turn.
- **Established by:** file 04 §0, §4.2 (`PostToolUse` `additionalContext`/`updatedToolOutput` is the grounding-signal surface — the substrate must compute the signal and feed it).
- **Substrate must build:** the grader integration (deterministic checks, external judge, or a separate-model critic) and the wiring that injects its verdict via `PostToolUse` or a follow-up `query()`.

### 10.3 The self-grading-bias problem (Claude-grades-Claude)
- **SDK does NOT:** provide any guard against self-grading bias. The SDK's natural verification pattern — a subagent or a follow-up `query()` reviewing the same model's work — is **Claude judging Claude**: subagents inherit the same model family, return free-form text (no structured verdict, file 06 §3), and a parent "may summarize" a subagent's result rather than carry its verdict verbatim (file 06 §3.3). The LLM-as-judge eval pattern (§10.1) is likewise same-vendor by default.
- **Established by:** file 06 §3 (text-only subagent return, no structured verdict; parent may summarize); §10.1 above (LLM-as-judge is the default eval shape).
- **Substrate must build:** if independent grounding matters, an **external** grader (deterministic checks the model can't talk past — tests, linters, type-checkers, spec-conformance) and/or a different-vendor/different-context critic, with a structured verdict the host reads from disk (file 06 §3.3 file-write contract) rather than trusting an in-band model summary. (Ark's verifier-writes-VERIFY-then-host-gates pattern and deterministic `cargo test`/`clippy` gating are exactly this substrate-side answer.)

---

## Prioritized gap table (severity for stage-1 ArkOS)

Severity legend: **blocker** = stage-1 cannot function without it; **build-required** = stage-1 needs it and it is non-trivial; **acceptable-defer** = real gap but stage-1 can ship a stub or live without it. Cost is rough relative effort (S/M/L). No solutions designed here — weight only.

| # | Gap | Established by | Severity (stage-1) | Rough cost |
| - | --- | -------------- | ------------------ | ---------- |
| 1.1 | No workflow/DAG framework | 06 §8, 09 §6 | build-required | M (Ark has the lifecycle SM already) |
| 1.2 | Dispatch model-decided, not host-deterministic | 06 §2 | **blocker** (deterministic role dispatch is core) | M (separate `query()` per role) |
| 1.3 | Recursion depth = 1 | 06 §4 | build-required | M (host-orchestrated recursion) |
| 1.4 | No fan-out/join primitive | 06 §5, 09 §6 | build-required | M (`gather` + semaphore + join) |
| 2.1 | No task tree / parent-child | 05 §1, 02 §2–3 | build-required (Ark has it) | S–M |
| 2.2 | No focus tracking | 02 §6 | build-required (Ark has it) | S |
| 2.3 | No cross-task shared state | 02 §0, 08 §6.1 | build-required | M |
| 3.1 | No semantic/vector/retrieval memory | 02 §5, §8 | acceptable-defer (stage-1) | L |
| 3.2 | No cross-project KB | 02 §5, §8 | acceptable-defer | L |
| 3.3 | JSONL opaque/unstable | 02 §5 | build-required (read via SDK only) | S |
| 4.1 | Sessions machine-local | 02 §9 | build-required (single-host ok stage-1) | M |
| 4.2 | Private/unstable format | 02 §5, §7 | build-required (discipline) | S |
| 4.3 | No shared store; mirror best-effort | 02 §7 | acceptable-defer (single-host) | M |
| 4.4 | No GC/retention/age filter | 02 §6 | build-required | S |
| 5.1 | Cannot publish MCP server | 07 §5 | acceptable-defer (stage-1 consumes; publish later) | M–L |
| 5.2 | MCP tools-only (no resources/prompts) | 07 §3 | acceptable-defer | M |
| 5.3 | No OAuth for remote MCP | 07 §2.3 | acceptable-defer | S–M |
| 5.4 | `AGENTS.md` not loaded | 02 §8 | build-required (cross-runtime parity) | S |
| 5.5 | Vendor lock to Claude | 01 §4 | acceptable-defer (stage-1 is Claude) | L |
| 6.1 | No file-write coordination | 09 §3, 05 §5 | **blocker** (worktree isolation mandatory for write fan-out) | M (Ark has worktrees) |
| 6.2 | `~/.claude/` config leak | 09 §2.1, 02 §8 | build-required | S (`CLAUDE_CONFIG_DIR`/`HOME` per call) |
| 6.3 | No 429 retry (Python crashes) | 09 §7 | **blocker** (long runs destroyed by one 429) | S–M (catch+backoff+resume) |
| 6.4 | Shared `atexit` teardown | 09 §2.2, §8.3 | build-required | S (explicit close; TS EPIPE filter) |
| 6.5 | Thread safety undocumented; fragile cleanup | 09 §5 | build-required (discipline) | S |
| 7.1 | Budget per-`query()` only (no cumulative) | 08 §4, §6.1 | **blocker** (cost runaway across phases) | S (host accumulator) |
| 7.2 | Cost is estimate, not billing | 08 §7 | acceptable-defer (insight ok; don't bill) | S |
| 7.3 | No live/per-subagent cost | 08 §1.1, §8 | build-required (per-role cost) | S (role = own `query()`) |
| 8.1 | SDK emits no telemetry itself | observability doc [FRESH] | build-required | S (enable OTel) |
| 8.2 | No metrics store/dashboard/backend | observability doc [FRESH] | build-required | M–L (run collector+backend) |
| 8.3 | Event-stream-is-trace; no turn-end event; thin logging | 03 §3, 04 §9 | build-required | M |
| 9.1 | No auth / identity | observability doc [FRESH], 01 §3 | acceptable-defer (single-user stage-1) | M |
| 9.2 | No tenant isolation | 02 §8, 09 §2 | acceptable-defer (single-tenant stage-1) | M–L |
| 9.3 | No per-user quota | 09 §7, 08 §7.1 | acceptable-defer (single-user stage-1) | M |
| 10.1 | No eval harness | [FRESH] | build-required | M |
| 10.2 | No external-grader integration | 04 §0, §4.2 | **blocker** (grounding is the point of a substrate) | M |
| 10.3 | Self-grading-bias (Claude-grades-Claude) | 06 §3, 10.1 | **blocker** (deterministic external gates needed) | M (tests/clippy/spec gates; Ark has the pattern) |

**Blockers for stage-1 (must be solved before the substrate functions):** deterministic dispatch (1.2), file-write coordination / worktree isolation (6.1), 429 survival (6.3), cumulative budget (7.1), external-grader integration (10.2), and the self-grading-bias guard via deterministic external gates (10.3). Everything else is build-required (needed, non-trivial, no hard blocker) or acceptable-defer (real gap, stage-1 can stub or live without).

---

## Caveats / Not found

- **This file makes no design choices.** Severities and costs are *weights for the SYNTHESIS file (99)* and the ArkOS architecture task, not decisions. "Blocker/build-required/acceptable-defer" reflects stage-1 ArkOS as framed by the PRD + RFC 001 (single-host, Claude-only bootstrap), and would shift for a multi-tenant or multi-host stage.
- **Topic file 13 (`13_telemetry-and-observability.md`) was NOT written at this snapshot** (only 01–09 exist on disk). §8 of this file therefore sources observability from a fresh read of the [observability doc](https://code.claude.com/docs/en/agent-sdk/observability) (2026-05-25) rather than cross-referencing 13. If 13 is later written, reconcile §8 against it.
- **Topic files 10 (persistence-and-memory), 11 (skills-and-agents-md), 12 (extended-thinking-and-model-config) were NOT written at this snapshot.** Memory facts in §3–§4 are drawn from file 02 (sessions) where they were established; `AGENTS.md` (§5.4) from file 02 §8. If 10–12 are written later, they may sharpen these gaps but should not contradict 02.
- **§10 (grounding/evaluation) is the least corpus-covered category** — no per-topic file owns it. §10.1/§10.2 rest on a fresh web search (2026-05-25) confirming Anthropic ships eval *patterns* and a separate Console product, and third parties build harnesses on SDK hooks; the SDK itself ships no harness. §10.3 (self-grading bias) is a synthesis of file 06 §3 facts, not a new SDK claim.
- **No new SDK claims that contradict the per-topic files.** Every inherited gap cites its source file; the three fresh confirmations (§8.1/§8.2 observability, §9.1 identity, §10.1 eval) are external-doc/web-sourced and dated, and are additive (they fill categories no per-topic file covered), not corrective.
- **Version pin holds:** Python 0.2.87 / TS 0.3.150, both latest at 2026-05-25. Several gaps are version-sensitive and should be re-verified on upgrade: 429 handling (#973 unmerged — may land), the `HookEvent`/`permissionMode` enums (have grown across minors, file 04), beta traces (span names may change, §8.2), and the alpha `task_budget` (file 08 §4.4).
- **TS-vs-Python asymmetries that touch these gaps** (carried from the per-topic files, flagged so a substrate doesn't assume parity): Python lacks `SessionStart`/`SessionEnd` callback hooks (file 04 §6.2), lacks `persist_session: false` (file 02 §4), lacks `startup()`/`WarmQuery` (file 09 §4.4), lacks `forwardSubagentText` and `criticalSystemReminder_EXPERIMENTAL` (file 06 §1.3, §3.4), and cannot return `structuredContent` from in-process tools (file 05 §6). The 429-crash (§6.3) is the Python-specific one confirmed; TS 429 retry was not confirmed (assume none).

## Primary sources

- Corpus files (inherited gaps, cited inline per §): `01_overview-and-relationship-to-claude-code.md`, `02_sessions.md`, `03_streaming-events.md`, `04_hooks.md`, `05_tools-and-permissions.md`, `06_subagents.md`, `07_mcp-integration.md`, `08_cost-and-budget.md`, `09_concurrency-and-parallelism.md` (all in this `research/` dir, snapshot 2026-05-25).
- [Observability with OpenTelemetry (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/observability) — `[FRESH 2026-05-25]` §8: "The SDK does not produce telemetry of its own"; OTLP-export-only; three signals; beta traces; end-user attributes identify the credential not the end user (§9.1).
- [Demystifying evals for AI agents (anthropic.com)](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents) — `[FRESH 2026-05-25]` §10.1: Anthropic publishes eval *patterns*, not an SDK harness.
- [Using the Evaluation Tool (platform.claude.com)](https://platform.claude.com/docs/en/test-and-evaluate/eval-tool) — `[FRESH 2026-05-25]` §10.1: the Console eval tool is a separate prompt-testing product, not part of the Agent SDK.
- [TribeAI/claude-evals (GitHub)](https://github.com/TribeAI/claude-evals) — `[FRESH 2026-05-25]` §10.1: third-party eval harness built *on top of* the SDK via native hooks (`PreToolUse`/`PostToolUse`/`SubagentStop`) — evidence the harness is not in the SDK.
