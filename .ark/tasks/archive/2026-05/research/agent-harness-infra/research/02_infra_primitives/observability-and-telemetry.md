# Observability and Telemetry

## What the primitive means

"Observability" for a coding-agent harness is the answer to: *when an
autonomous run goes wrong — wrong file edited, wrong test asserted, wrong
commit message — can I figure out why?* It is the **debugger** for an
agent run.

The space splits cleanly along the question being asked:

| Concern | Primitive | Example tool |
| ------- | --------- | ------------ |
| What did the model see + emit? | Per-call request/response log | Helicone, Portkey gateway |
| In what order, with what timing? | Span / trace tree | LangSmith, Phoenix, Langfuse |
| Why did the agent decide *that*? | Reasoning trace + tool args | Laminar transcript view |
| What was the *cost*? | Token + dollar aggregation | All of the above |
| Can I replay it deterministically? | Trace + seed + tools | Phoenix replay; Codex `experimental_resume` |
| What broke in production? | Alert on eval signals | Maxim AI evals; Langfuse scores |

Coding-agent harnesses sit one layer *above* these — they emit events that
observability platforms ingest. The platforms then add the schema-shaped
analysis (span trees, replay, evals).

## Standards: OpenTelemetry GenAI semantic conventions

In early 2026 the OTel GenAI client semconv exited experimental
(`opentelemetry.io/docs/specs/semconv/gen-ai/`). Three span kinds matter
for agents:

- **`invoke_agent`** — top-level agent invocation; children are
  `chat` spans and `execute_tool` spans.
- **`invoke_workflow`** — when the instrumenter can reliably distinguish a
  workflow (group of agent invocations) from a single agent invocation.
- **`execute_tool {tool_name}`** — per-tool-call span.

Plus chat spans (`chat`, `embeddings`), and standardized attributes
(`gen_ai.request.model`, `gen_ai.system`, `gen_ai.usage.input_tokens`,
…). This is the same model OpenLLMetry / Phoenix / Langfuse converged on.

Phoenix specifically supports 10 span kinds: `CHAIN`, `LLM`, `TOOL`,
`RETRIEVER`, `EMBEDDING`, `AGENT`, `RERANKER`, `GUARDRAIL`, `EVALUATOR`,
and one more for protocols.

The takeaway: **OTel semconv is now the right wire format for "agent did
a thing"**. Anything that emits these attributes is consumable by every
observability tool.

## How leading harnesses implement it

### Claude Code

**Native:** transcript files (JSONL) under
`~/.claude/projects/<cwd>/*.jsonl`. Each event line carries
{role, content, tool_use_id, usage, …}. This IS observability at the
"what did the model see/emit" layer.

**Hook-based:** the `disler/claude-code-hooks-multi-agent-observability`
project (`github.com/disler/...`) wires `PreToolUse`, `PostToolUse`,
`SubagentStart`, `SubagentStop` to a real-time event stream + UI. The
event capture surface IS the hook system.

**No native OTel emission.** Users wire their own via a `PostToolUse` hook
that POSTs to a collector.

### OpenAI Codex CLI

**Native:** rollout JSONL with token usage embedded (`developers.openai.com/codex/cli/features`).

**Hook-based:** same shape as Claude — users wire PostToolUse / Stop
hooks to forwarders.

**Cost telemetry:** `usage` field in each event line. Aggregation is the
consumer's job.

### LangSmith (LangChain)

- Deep integration with LangChain / LangGraph: node-by-node state diffs,
  full agent execution graphs.
- Replays against new model versions.
- OpenTelemetry bridge: bi-directional with OTel stacks.
- No self-hosted deployment option — SaaS-only is the major caveat.

### Langfuse

- Open source, self-hostable. Cloud or Docker Compose.
- Data model: traces, observations, sessions, scores. Observations typed
  as `generation`, `span`, `event`.
- OpenTelemetry-native — any OTel-emitting agent works out of the box.
- Open-source frontrunner — "self-hostable in minutes."

### Arize Phoenix

- Open source, OTel-native.
- 10 span kinds (CHAIN / LLM / TOOL / RETRIEVER / EMBEDDING / AGENT /
  RERANKER / GUARDRAIL / EVALUATOR / …).
- **Replay traces to inspect failures** — closest thing to a debugger.
- Embedding-clustering + drift detection for behavioural analysis.

### Helicone

- LLM gateway model — sit in front of OpenAI / Anthropic; log every
  request.
- Simplest integration: base URL change.
- **Went into maintenance mode on March 3, 2026** — historically
  important but no longer recommended for new work.

### Laminar

- Open source. "Transcript view instead of span trees."
- **Agent rollout** — re-run from any span. Closest analogue of a
  step-debugger for an agent run.
- "Signals" — natural-language outcome tracking.
- SQL over traces for ad-hoc analysis.
- 5% overhead in production.

### AgentOps

- Purpose-built for agents (not adapted from LLM observability).
- Captures LLM calls, costs, latency, multi-agent interactions, tool
  usage, session statistics.

### Maxim AI

- Simulation + evaluation + observability. Broader than pure obs.
- Semantic evaluation: output quality, factual accuracy, alignment with
  intent.

### OpenHands

The condenser ("persistent EventLog → full replay even after compression")
is internal observability scaffolding rather than a user-facing surface.

## "Replay" — the holy grail

What people actually want is *a debugger for an agent run* — pause,
rewind, change a tool result, replay from there. Today's state of the art:

| Tool | Replay surface | Determinism |
| ---- | -------------- | ----------- |
| Phoenix | Re-run from inspected span | Probabilistic (LLM seeds) |
| Laminar | "Agent rollout from any span" | Same |
| LangSmith | Re-execute with new model version | Same |
| Claude Code `/rewind` | File-level checkpoint, not run replay | Deterministic (file state) |
| Codex `experimental_resume` | JSONL → replay-and-continue (broken on main) | Currently nonfunctional |

The honest assessment: **no surveyed tool gives you a true deterministic
agent debugger.** The model is non-deterministic, so "replay" actually
means "re-run." This is a category-defining gap in the field, not a
shortcoming of any single tool.

## Three layers of "an agent debugger"

| Layer | Granularity | Currently exists? |
| ----- | ----------- | ----------------- |
| File / commit | "what changed on disk" | Yes — git, Aider's atomic commits, Ark's `.ark.db` |
| Conversation / span | "what the agent said and did" | Yes — Langfuse, Phoenix, Laminar via OTel |
| Decision / branch | "why did it pick this tool" | Partial — chain-of-thought logs, but non-replay |

Ark sits at layer 1 (commit-shaped). Layers 2 and 3 are largely *outside*
Ark's scope — they belong to the host harness or to a separate
observability product.

## What Ark does today

Ark has **no structured telemetry**. Effectively:

- **Logs:** the binary emits text via `Display` impls on command summaries
  (AGENTS.md: "Commands return summaries that `impl Display`. The CLI
  calls one `render(summary)` per dispatch"). No JSON-by-default mode
  except `ark context --format json`.
- **Audit trail:** the file system itself — `task.toml.committed_at`,
  `archived_at`, git history of `.ark/specs/features/`,
  `.ark/workspace/<dev>/journal-N.md`.
- **Event emission:** none. Ark does not emit OTel spans, JSON-Lines
  events, or any other structured event stream when phases transition.
- **`ark context` is the closest thing to a structured-state surface** —
  emitted from a `SessionStart` hook so the host harness reads it. Not
  observability in the agent-loop sense, but is *state-projection*.

The shape that *would* be observability — "task `<slug>` transitioned
plan → execute at <timestamp>" — does not exist as a stream anywhere. The
information is recoverable from `task.toml` mtimes and git logs, but
nothing aggregates it.

## Directions for Ark

1. **Emit a JSON-Lines audit log per project.** `.ark/.audit.jsonl`,
   append-only, one line per `ark agent task <verb>` execution
   {timestamp, slug, from_phase, to_phase, exit_code, duration_ms}.
   No new dependency; the existing `state_mutate` site already centralises
   transitions. Code site: `crates/ark-core/src/commands/agent/task/phase.rs`.
2. **OTel span emission behind a feature flag.** Add a `tracing-opentelemetry`
   integration that emits `invoke_workflow` (overall task run),
   `execute_tool` (one Ark verb), spans only when
   `OTEL_EXPORTER_OTLP_ENDPOINT` is set. Zero-overhead when absent.
   Code site: new `crates/ark-core/src/telemetry.rs`; wire from
   `commands/agent/task/mod.rs`.
3. **`ark context --format otel`.** New `--format` value emitting the
   session-scope state as OTel span attributes. Lets a host hook ship
   the projection straight into Langfuse / Phoenix without a translator.
   Code site: `crates/ark-core/src/commands/context/render.rs`.
4. **Workflow-aware replay.** A `ark agent task replay --from <commit>`
   verb that walks git history of `.ark/tasks/<slug>/`, recovers each
   intermediate `task.toml` + PRD + PLAN + VERIFY state, and shows the
   transitions. Not LLM replay — *workflow* replay. Useful for post-
   mortems on rejected REVIEW iterations. Code site:
   `crates/ark-core/src/commands/agent/task/`.
5. **Adopt OTel GenAI semconv vocabulary.** Even before emitting OTel,
   align internal naming: `ark.task.tier`, `ark.task.phase`,
   `ark.spec.path` — matching the `gen_ai.*` pattern. Makes future OTel
   adoption a relabel, not a refactor. Code site: documentation only —
   AGENTS.md / CLAUDE.md notes.

## Caveats / Not found

- I did not benchmark the actual cost of OTel emission overhead in Ark's
  one-shot CLI invocation model; "1 ms" is a guess, not a measurement.
- The Helicone maintenance-mode date (2026-03-03) is from secondary
  sources; verify against Helicone's blog before citing externally.
- LangSmith's self-hosted-deployment story may have changed by 2026;
  verify against `langchain.com/langsmith/observability`.
- No primary-source benchmark of Phoenix replay determinism — replay
  treats temperature > 0 as inherently non-replayable.
- "True deterministic agent debugger" — I did not find any system that
  claims this; treat absence as confirmation rather than oversight.

## Sources

- [OpenTelemetry GenAI agent and framework spans](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-agent-spans/)
- [OpenTelemetry GenAI client spans](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/)
- [Inside the LLM Call: GenAI Observability with OpenTelemetry](https://opentelemetry.io/blog/2026/genai-observability/)
- [LangSmith Observability](https://www.langchain.com/langsmith/observability)
- [Langfuse — OSS LLM engineering platform](https://github.com/langfuse/langfuse)
- [Arize Phoenix](https://github.com/Arize-ai/phoenix)
- [Laminar — Open-source observability](https://laminar.sh/)
- [Top 5 Agent Observability Tools 2025 (Maxim)](https://www.getmaxim.ai/articles/top-5-leading-agent-observability-tools-in-2025/)
- [Agent Observability: LangSmith / Langfuse / Arize 2026](https://www.digitalapplied.com/blog/agent-observability-platforms-langsmith-langfuse-arize-2026)
- [Top 6 Agent Observability Platforms (Laminar)](https://laminar.sh/article/2026-04-23-top-6-agent-observability-platforms)
- [Claude Code hooks multi-agent observability](https://github.com/disler/claude-code-hooks-multi-agent-observability)
- [Best AI Agent Observability Tools 2026 (Latitude)](https://latitude.so/blog/best-ai-agent-observability-tools-2026-comparison)
