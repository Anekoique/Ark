# Claude Agent SDK — Telemetry & Observability

> Snapshot: 2026-05-25 · Python `claude-agent-sdk` 0.2.87 (bundles Claude Code CLI ≥ 2.0.0) · TS `@anthropic-ai/claude-agent-sdk` 0.3.150
> Primary source: [Observability with OpenTelemetry](https://code.claude.com/docs/en/agent-sdk/observability) (read offline from a local capture) + SDK source `github.com/anthropics/claude-agent-sdk-python` at 0.2.87.
> Cross-refs: `03_streaming-events.md` (the typed message stream), `08_cost-and-budget.md` (cost/token/timing fields), `14_limits-and-gaps.md` (the gap framing this file expands).

This file covers observability **beyond** raw consumption of the message stream: structured logging, OpenTelemetry, per-tool/per-turn timing, recommended external patterns, and session replay/audit. The message-stream taxonomy itself lives in topic 03; this file treats that stream as one of several observability surfaces.

---

## 0. The one-paragraph truth

The Agent SDK **produces no telemetry backend of its own**. What it has is a **passthrough OpenTelemetry pipeline**: the bundled Claude Code CLI is instrumented (it records spans around each model request and tool execution, emits metrics for token/cost/session counters, and emits structured log events for prompts and tool results), and the SDK both (a) lets you configure the CLI's OTLP exporters via `ClaudeAgentOptions.env`, and (b) **propagates W3C trace context** into the CLI subprocess so the agent run nests inside *your* application's distributed trace. Everything else — a metrics store, a dashboard, an audit query layer, replay tooling — is the host's to build. For ArkOS, the practical reading is: the substrate gets OTLP traces/metrics/logs for free if it runs a collector, gets the typed event stream + JSONL transcript as a second independent audit surface, and must build any *queryable* event-log service itself.

---

## 1. Structured logging & debug output (SDK-side)

Distinct from the OTel pipeline (§2), the SDK process has ordinary Python-logging-level diagnostics.

### 1.1 Python `logging`

The SDK's internals use the standard library logger throughout `_internal/` (e.g. `logger.debug("Read task cancelled")`, `logger.debug("Error streaming input: ...")`, `logger.debug("stderr stream read failed", exc_info=True)`). There is no custom logging framework. Verbosity is therefore controlled the normal way:

```python
import logging
logging.basicConfig(level=logging.DEBUG)   # surfaces the SDK's internal debug lines
logging.getLogger("claude_agent_sdk").setLevel(logging.DEBUG)
```

The logger names are the module paths under `claude_agent_sdk`; the SDK does not document a single canonical logger name, so filter at the package root.

### 1.2 `debug_stderr` (Python option)

`ClaudeAgentOptions` carries `debug_stderr: Any = sys.stderr` (`types.py:1739`). It is the sink the SDK writes subprocess-stderr / debugging output to. Redirect it to capture CLI diagnostics:

```python
import io
buf = io.StringIO()
options = ClaudeAgentOptions(debug_stderr=buf)   # CLI stderr / debug captured into buf
```

Default is the process's real stderr. **TS divergence:** the TS SDK has no `debug_stderr` field of this name; stderr handling there is internal. Treat `debug_stderr` as Python-only (confirmed in the Python clone; not found as a TS option).

### 1.3 Subprocess identity

The SDK tags the spawned CLI with `CLAUDE_CODE_ENTRYPOINT=sdk-py` on the subprocess env (`subprocess_cli.py:433`). The TS SDK uses an analogous entrypoint tag (inferred — the Python clone only shows `sdk-py`). This is how downstream telemetry / logs can tell an SDK-driven run from an interactive CLI run.

### 1.4 What SDK-side logging does NOT give you

No structured (JSON) log records, no metrics, no spans. Python `logging` output is free-text developer diagnostics. For anything you want to *aggregate or query*, use the OTel pipeline (§2) or build off the message stream (§3) — not the debug logger.

---

## 2. OpenTelemetry (the real observability surface)

### 2.1 The split: CLI emits, SDK configures + propagates

Verbatim from the observability doc:

> "[The CLI] records spans around each model request and tool execution, emits metrics for token and cost counters, and emits structured log events for prompts and tool results. **The SDK does not produce telemetry of its own** [...] The Agent SDK emits the same data because it runs the same CLI."

So the instrumentation lives in the bundled Claude Code CLI. The SDK's role is twofold:

1. **Configuration injection** — you set OTel env vars through `ClaudeAgentOptions.env`, which the SDK forwards to the CLI subprocess.
2. **Trace-context propagation** — the SDK injects W3C `TRACEPARENT`/`TRACESTATE` into the child process so the CLI's root span becomes a child of your active span (§2.6).

This matches `14_limits-and-gaps.md`: no own telemetry, OTLP-export-only, and (see §2.7) telemetry attributes identify the *credential*, not the end user, unless you add resource attributes yourself.

### 2.2 Three independent signals

Each signal has its own enable switch and its own exporter — turn on only what you need:

| Signal | What it contains | Enable with |
|---|---|---|
| **Metrics** | Counters for tokens, cost, sessions, lines of code, and tool decisions | `OTEL_METRICS_EXPORTER` |
| **Log events** | Structured records for each prompt, API request, API error, and tool result | `OTEL_LOGS_EXPORTER` |
| **Traces** | Spans for each interaction, model request, tool call, and hook (beta) | `OTEL_TRACES_EXPORTER` **plus** `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1` |

Telemetry is **off** until `CLAUDE_CODE_ENABLE_TELEMETRY=1` AND at least one exporter is chosen. (Note from the doc: `CLAUDE_CODE_ENABLE_TELEMETRY=1` is *required for traces* — which are beta — but metrics and log events do not strictly need it; the canonical config sets it anyway.)

### 2.3 Canonical config (verbatim shape from the doc)

```python
import asyncio
from claude_agent_sdk import query, ClaudeAgentOptions

OTEL_ENV = {
    "CLAUDE_CODE_ENABLE_TELEMETRY": "1",
    "CLAUDE_CODE_ENHANCED_TELEMETRY_BETA": "1",   # required for traces (beta)
    # one exporter per signal — use `otlp` for the SDK
    "OTEL_TRACES_EXPORTER": "otlp",
    "OTEL_METRICS_EXPORTER": "otlp",
    "OTEL_LOGS_EXPORTER": "otlp",
    # standard OTLP transport
    "OTEL_EXPORTER_OTLP_PROTOCOL": "http/protobuf",
    "OTEL_EXPORTER_OTLP_ENDPOINT": "http://collector.example.com:4318",
    "OTEL_EXPORTER_OTLP_HEADERS": "Authorization=Bearer your-token",
}

async def main():
    options = ClaudeAgentOptions(env=OTEL_ENV)
    async for _ in query(prompt="...", options=options):
        ...

asyncio.run(main())
```

TS is the same env-map shape passed to `ClaudeAgentOptions`/options `env`.

### 2.4 CRITICAL SDK constraint — do not use the `console` exporter

> "The `console` exporter writes telemetry to standard output, which the SDK uses as its message channel. **Do not set `console` as an exporter value when running through the SDK.**"

Because the SDK reads the CLI's stdout as the JSONL message stream, a `console` exporter corrupts that channel. **Always use `otlp`** for SDK runs. To inspect telemetry locally, point `OTEL_EXPORTER_OTLP_ENDPOINT` at a local collector or an all-in-one Jaeger container — never `console`.

### 2.5 Span taxonomy (traces, beta)

When `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1` is set, each step of the agent loop becomes a span:

- **`claude_code.interaction`** — wraps a single turn of the agent loop, from receiving a prompt to producing a response. (Root span for the run.)
- **`claude_code.llm_request`** — wraps each call to the Claude API, with **model name, latency, and token counts** as attributes.
- **`claude_code.tool`** — wraps each tool invocation, with child spans:
  - `claude_code.tool.blocked_on_user` — the permission-wait interval.
  - `claude_code.tool.execution` — the actual execution.
- **`claude_code.hook`** — wraps each hook execution (requires detailed beta tracing: `ENABLE_BETA_TRACING_DETAILED=1` and a `BETA_TRACING_ENDPOINT`).

`llm_request`, `tool`, and `hook` spans are children of the enclosing `claude_code.interaction` span. Spans carry a `session.id` attribute by default (omittable via `OTEL_METRICS_INCLUDE_SESSION_ID` falsy). The doc warns: **tracing is beta; span names and attributes may change between releases.**

For the complete enumerated list of metric names, event names, and attributes, the doc defers to the **Claude Code Monitoring reference** (not captured here — fetch when needed).

### 2.6 Subagent span nesting (delegation chain in one trace)

> "When the agent spawns a subagent through the `Task` tool, the subagent's `llm_request` and `tool` spans **nest under the parent agent's `claude_code.tool` span**, so the full delegation chain appears as one trace."

This is significant for ArkOS: an in-session subagent's work is *observable as a sub-tree of the parent trace*, even though (per `06_subagents.md`) its result returns only as opaque text to the host. So **traces give you subagent visibility that the message stream and result-return shape do not.** (The `Task` tool was renamed `Agent` in the tool inventory per topic 05/06; the doc text still says `Task` — same mechanism.)

Note this is the *trace-level* nesting. A host-orchestrated separate `query()` (the deterministic-dispatch pattern from topic 06 §8) would instead appear as a sibling trace unless you propagate context into it yourself (§2.6 propagation, applied per child session).

### 2.7 W3C trace-context propagation (verified in source)

The SDK auto-links the agent run into your application's trace. From `subprocess_cli.py:438-462`:

```
# Propagate active OTEL trace context to the CLI so its spans
# parent under the caller's distributed trace. No-op if
# opentelemetry-api is not installed or there's no active span.
    from opentelemetry import propagate
    ...
    propagate.inject(carrier)
    if "traceparent" in carrier:
        for key in ("TRACEPARENT", "TRACESTATE"):
            ...   # injected into child process env
    ...
    logger.debug("OTEL trace context injection failed", exc_info=True)
```

Behavior, precisely:
- If `opentelemetry-api` is installed AND there is an active span, the SDK injects `TRACEPARENT`/`TRACESTATE` into the CLI subprocess env. The CLI reads them so its `claude_code.interaction` span becomes a child of your span.
- **No-op** if `opentelemetry-api` isn't installed or there's no active span (graceful — just `logger.debug` on failure, never raises).
- **`ClaudeAgentOptions.env` always wins**: auto-injection is skipped when you set `TRACEPARENT` explicitly in `options.env`. The code gates on the `traceparent` key (not carrier truthiness) so an inherited `TRACESTATE` isn't paired with a new `TRACEPARENT`.
- Bash-tool subprocesses that emit their own OTel spans nest those under the `claude_code.tool.execution` span that wraps the command.

### 2.8 Sensitive-content env vars (off by default — privacy gate)

These add content to exported data; the doc says leave unset unless your pipeline is approved to store it:

| Variable | Adds |
|---|---|
| `OTEL_LOG_USER_PROMPTS=1` | Prompt text on `claude_code.user_prompt` events and on the `claude_code.interaction` span |
| `OTEL_LOG_TOOL_DETAILS=1` | Tool input arguments (file paths, shell commands, search patterns) on `claude_code.tool_result` events |
| `OTEL_LOG_TOOL_CONTENT=1` | Full tool input/output bodies as span events on `claude_code.tool`, truncated at 60 KB (requires tracing enabled) |
| `OTEL_LOG_RAW_API_BODIES` | Full Anthropic Messages API request/response JSON as `claude_code.api_request_body` / `claude_code.api_response_body` log events. `1` = inline bodies truncated at 60 KB; `file:<dir>` = untruncated bodies on disk |

### 2.9 Resource attributes & end-user attribution

By default spans/metrics/events carry the *credential* identity, not the end user. To attribute per user/tenant, set resource attributes via `OTEL_RESOURCE_ATTRIBUTES` / `OTEL_SERVICE_NAME` in `options.env`:

```python
from urllib.parse import quote
options = ClaudeAgentOptions(env={
    # ... exporter config ...
    "OTEL_SERVICE_NAME": "support-triage-agent",
    "OTEL_RESOURCE_ATTRIBUTES":
        f"enduser.id={quote(request.user_id)},tenant.id={quote(request.tenant_id)}",
})
```

These are applied as OpenTelemetry resource attributes on **every** span, metric, and event the agent emits. (Directly relevant to ArkOS multi-tenant gap in topic 14: per-user attribution is host-supplied, not automatic.)

### 2.10 Flushing from short-lived calls

Default export intervals: **metrics every 60 s; traces and logs every 5 s.** A short task can finish before the buffer flushes, losing telemetry. Lower the intervals for short runs:

```python
OTEL_ENV = {
    # ... exporter config ...
    "OTEL_METRIC_EXPORT_INTERVAL": "1000",
    "OTEL_LOGS_EXPORT_INTERVAL": "1000",
    "OTEL_TRACES_EXPORT_INTERVAL": "1000",
}
```

For ArkOS phase calls (often short), this matters: a 20 s PLAN phase would otherwise drop its metrics under the 60 s default.

---

## 3. The message stream + JSONL as the audit trail

Independent of OTel, the SDK gives two host-readable surfaces (covered in topics 03 and 02 respectively):

1. **The live typed message stream** — `SystemMessage`, `AssistantMessage` (with `ToolUseBlock`/`ToolResultBlock`), `ResultMessage`, etc. A host consuming `async for msg in query(...)` sees every turn, every tool call, every result, in order. This is the *push* audit surface.
2. **The persisted session JSONL** — `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl` (topic 02). Same data, durable, but in an **explicitly private/unstable format** — read it via `list_sessions` / `get_session_messages`, never by parsing the file. This is the *pull* audit surface.

Building metrics from the stream (no OTel needed):

```python
metrics = {"tool_calls": 0, "turns": 0, "cost_usd": 0.0}
async for msg in query(prompt="...", options=options):
    cls = type(msg).__name__
    if cls == "AssistantMessage":
        metrics["turns"] += 1
        for block in msg.content:
            if type(block).__name__ == "ToolUseBlock":
                metrics["tool_calls"] += 1
    elif cls == "ResultMessage":
        metrics["cost_usd"] = msg.total_cost_usd      # see topic 08
```

This stream-derived approach is what `ark run` does today (it hand-parses Claude Code's `--output-format stream-json`); the SDK's typed stream is the same data with types instead of a hand-rolled parser.

**When to use which:** OTel (§2) for fleet-level aggregation, dashboards, cross-service traces. The message stream for in-process control-flow decisions (the driver needs to *act* on a tool result, not just record it) and for the per-phase artifact-completion checks ArkOS does. They are complementary, not redundant.

---

## 4. Per-tool & per-turn timing

### 4.1 Run-level timing (on the result message)

`ResultMessage` carries (Python `types.py:1149-1152`):
- `duration_ms: int` — total wall-clock of the run.
- `duration_api_ms: int` — time spent in API calls (excludes local tool execution / waiting).
- `num_turns: int` — number of agent-loop turns.

`duration_ms - duration_api_ms` approximates time spent in tool execution + overhead. These are *per-`query()`* aggregates, available only at the terminal `ResultMessage` (consistent with topic 08: cost is also terminal-only).

### 4.2 Per-tool timing

There is **no per-tool duration field on the message stream.** Two ways to get it:

1. **OTel traces (§2.5)** — `claude_code.tool.execution` spans carry their own duration; `llm_request` spans carry latency. This is the documented per-tool timing surface.
2. **PostToolUse hooks (topic 04)** — time it yourself: record a timestamp in `PreToolUse`, diff in `PostToolUse`. The hook sees the tool name + input + result. This is the hook-based metric-emission pattern (§5) and the only per-tool timing available without enabling beta tracing.

---

## 5. Recommended external patterns

Built-in observability is a passthrough + a stream. Idiomatic host patterns:

1. **OTLP to a collector** (§2) — the lowest-effort fleet observability. Run an OTel Collector / Jaeger / Honeycomb / Datadog / Grafana / Langfuse endpoint, set the four exporter env vars, get traces+metrics+logs. The doc explicitly names OTLP-accepting backends (Honeycomb, Datadog, Grafana, Langfuse).
2. **Stream-wrapping** (§3) — wrap `query()` in a consumer that records every message to your own event log / DB. Gives you control-flow hooks AND audit in one pass; format is yours (not the private JSONL).
3. **Hook-based metric emission** (topic 04) — `PreToolUse`/`PostToolUse` hooks emit counters/timings to your metrics system per tool call, and enforce policy in the same place (the out-of-scope-write guard and budget cap already live here per topic 04). One hook, two jobs: gate + observe.
4. **Resource-attribute tagging** (§2.9) — always set `OTEL_SERVICE_NAME` + per-task/per-tenant resource attributes so traces are queryable by task slug / tenant.
5. **Lower export intervals for short phase calls** (§2.10) so a fast phase doesn't drop telemetry.

For ArkOS specifically: the substrate's **event-log service** (an RFC-named primitive) would most naturally be pattern (2) — a stream-wrapper that writes a substrate-owned, queryable, stable-schema event log — *plus* pattern (1) OTLP for distributed tracing across phases/sessions. The SDK gives neither a queryable store nor a stable schema; both are substrate-build.

---

## 6. Session replay & audit

Given JSONL persistence (topic 02), a host can inspect past sessions **read-only** via the session functions (Python names; TS camelCase equivalents):
- `list_sessions` / `listSessions` — enumerate sessions (filter by cwd/project).
- `get_session_messages` / `getSessionMessages` — re-read a session's full message history.
- `get_session_info` / `getSessionInfo` — metadata.

This is *inspection*, not true *replay*: you can read what happened, but re-executing a session deterministically (same tool results, same model outputs) is not supported — model calls are non-deterministic and tool side-effects already happened. For ArkOS, "replay for substrate-revision comparison" (an RFC goal) means re-running the *workload* under a new substrate revision and comparing outcomes, not replaying a recorded trace. The recorded trace is for audit/debugging, not deterministic replay.

The private/unstable JSONL format (topic 02) is the reason to read via the SDK functions, not the files — and the reason a substrate that needs a *stable, queryable* audit log must maintain its own (pattern 5.2), treating the SDK JSONL as a secondary/recoverable source only.

---

## 7. Python ↔ TS divergence summary

| Aspect | Python 0.2.87 | TS 0.3.150 |
|---|---|---|
| OTel env-var config via `options.env` | ✅ | ✅ (same env-map shape) |
| W3C trace-context auto-propagation | ✅ verified in source (`subprocess_cli.py`) | Inferred equivalent (clone not re-inspected; doc presents both languages identically) |
| `debug_stderr` option | ✅ (`types.py:1739`) | Not found under this name — treat as Python-only |
| Standard-logging diagnostics | ✅ `logging` / `logger.debug` | Uses its own logging; not enumerated here |
| Session inspection functions | `list_sessions` / `get_session_messages` / `get_session_info` | `listSessions` / `getSessionMessages` / `getSessionInfo` |

---

## 8. Caveats & unknowns

- **Traces are beta.** `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1` required; span names/attributes "may change between releases." Don't hard-code span-name assumptions in a substrate without a version pin.
- **Full metric/event/attribute catalog not captured here.** The doc defers to the *Claude Code Monitoring reference* for the complete enumerated list. This file captured the span taxonomy and the env-var surface; fetch the Monitoring reference for exact metric names (token counters, cost counters, "lines of code", "tool decisions") and their attribute keys when implementing dashboards.
- **`BETA_TRACING_ENDPOINT` / `ENABLE_BETA_TRACING_DETAILED`** are named for the detailed `claude_code.hook` spans but their exact value formats weren't fully captured — verify against the Monitoring reference.
- **TS source not re-inspected** for this file; TS-side claims (trace propagation, entrypoint tag) are inferred from the doc presenting Python and TS identically. Flagged in §7.
- **Detailed CLI logging env vars** beyond the OTel set (e.g. any `ANTHROPIC_LOG`-style switch) were not confirmed present for the SDK path — the SDK's own diagnostics go through Python `logging` (§1), and the CLI's structured telemetry goes through OTel (§2); a separate free-text CLI debug-log env var was not confirmed and should be treated as undocumented for the SDK path.

---

## 9. Bottom line for ArkOS stage 1

- **Free:** OTLP traces/metrics/logs (run a collector), automatic trace-context linking into a parent app trace, subagent delegation visible as one trace, run-level timing on `ResultMessage`, read-only session inspection.
- **Substrate-build:** a *queryable, stable-schema* event-log service (the RFC primitive) — the SDK's JSONL is private/unstable and its OTel schema is beta/changing; a substrate owns its own event log (stream-wrapper pattern), optionally mirroring to OTLP. Per-tool timing without beta tracing = host-timed via hooks. Per-user/tenant attribution = host-supplied resource attributes. Deterministic replay = not provided (and not the right model — re-run workloads, don't replay traces).
- **Watch out:** never use the `console` OTel exporter through the SDK (corrupts the message channel); lower export intervals for short phase calls or lose their telemetry.
