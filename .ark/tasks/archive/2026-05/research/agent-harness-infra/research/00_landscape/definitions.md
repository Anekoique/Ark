# Definitions — fixing the vocabulary

Compiled 2026-05-20. The terms below are contested. This file pins the meanings the rest of the corpus uses, sourced from vendor docs, prominent essays, and observed product positioning. Where definitions differ across sources, both are recorded.

## Why this matters

In 2024 the field had two terms — "agent" and "framework". By mid-2025 there were a dozen overlapping terms ("harness", "infra", "runtime", "OS", "platform", "IDE") because the underlying space split into layers and each layer wanted its own brand. Most disputes about Ark ("is it a framework? a harness? infra?") are really vocabulary disputes. Fix the vocabulary, the disputes evaporate.

The load-bearing distinction is **harness vs framework** — see below. Everything else (runtime, OS, infra, IDE) is positioning a player above or below the harness layer.

---

## Primary terms

### Agent

An LLM running in a loop with tools, environment access, and some autonomy over its own next action. Distinguished from "chat completion" by: (a) multi-turn tool use without per-turn human input, (b) the model deciding what to do next from observation rather than user prompt. Anthropic's *Building Effective Agents* (Dec 2024) distinguishes "agents" from "workflows" — agents pick their own path; workflows follow a fixed graph. Both are commonly called "agents" in product positioning; the strict sense is the autonomous-loop one.

### Coding agent

An agent specialised for code work. Specialisation comes from tools (file I/O, shell, git), a system prompt that establishes coding norms, and grounding artifacts (CLAUDE.md / AGENTS.md / project rules). Examples: Aider, Cline, Claude Code, Cursor agent mode, Devin, OpenHands. As of 2026 this is the dominant commercial agent vertical.

### Agent framework

An SDK or library for *building* agents. Provides primitives — tool definitions, message types, run loops, planning patterns, multi-agent orchestration — but does not itself ship a runnable agent. The user writes Python/TypeScript that imports the framework and instantiates an agent. Examples: LangChain, LangGraph, OpenAI Agents SDK, AutoGen, CrewAI, Pydantic AI.

> Frameworks are *libraries*. You import them.

The pure-framework category is shrinking in 2026 — see `market_map.md`'s "dying" bucket. Most surviving frameworks (LangGraph, OpenAI Agents SDK) now ship adjacent runtimes / tracing / UI so they straddle into platform territory.

### Agent harness

A *running system* that wraps a model with the surrounding machinery — state management, tool use, conversation loop, context-window management, retries, sandboxing, observability, guardrails — and exposes a usable interface (CLI, IDE, web UI). The user runs the harness; they do not import it.

> Harnesses are *applications*. You install them.

The term "harness" comes from ML evaluation (SWE-bench harness, ARC harness): the scaffolding that lets a model interact with a task environment. As of 2025 the term broadened to mean any productionisation wrapper around a model. Examples: Aider, Cline, OpenHands, Claude Code, Cursor agent, Devin, Goose, Plandex, Ark.

A harness can ship its own framework internally, but its primary deliverable is the running system, not the SDK.

### Agent runtime

The execution substrate an agent runs on. Spans:

- Model inference (provider APIs, vLLM, llama.cpp)
- Tool execution (filesystem, shell, network, browsers)
- Sandboxing (Docker, Firecracker, gVisor, WebContainers, OS sandboxes)
- Persistence (sessions, state, memory)
- Observability (traces, spans, replays)

A runtime is below the harness — the harness configures and consumes a runtime. Some vendors ship combined runtime+harness (Devin); some ship pure runtime (E2B, Modal, Daytona); some ship harnesses that bring their own runtime (OpenHands ships pluggable Docker/E2B/Modal/K8s runtimes).

> Runtime = the box the agent runs in.

### Agent infra ("agent infrastructure")

Loose umbrella covering: agent runtime, deployment surfaces (Docker images, serverless functions, durable execution), evaluation pipelines, monitoring, billing, secrets, identity, A2A discovery. Sometimes used interchangeably with "agent runtime"; sometimes broader (includes evaluation, billing).

In vendor positioning circa 2026, "agent infra" tends to mean *cloud platforms* (AWS Bedrock AgentCore, Cloudflare Agents, Modal, E2B, Daytona). Ark is usually called a "harness", not "infra", because it is local and unhosted — but a maximalist reading of the term puts harnesses *inside* the infra layer (just the local part).

### Agent OS

Aspirational positioning for "a complete environment in which agents are first-class citizens, the way processes are in Unix". Includes runtime, scheduler, file system, IPC, identity, capability system. Not really shipping in 2026 — closest are Anthropic's MCP + Skills + memory stack, Cloudflare's Durable-Objects-as-agent-bodies, and Ark's own RFC 0001 (`docs/rfcs/001-arkos.md`) which sketches a two-stage evolution from harness to OS.

> Most "agent OS" claims as of 2026 are marketing. The actual shape is unsettled.

### AI IDE

An IDE designed around AI assistance — agent panels, inline edits, multi-file diff review, agent-driven file navigation. Examples: Cursor, Windsurf, Zed (post-Agent Panel), Theia AI, JetBrains AI Assistant. Distinct from "IDE with AI plugin" (VS Code + Copilot, JetBrains + GitHub Copilot) — the IDE is built agent-first, not retrofit.

### Coding-agent platform

Cloud-hosted, often async, where an agent works on a repo without the developer's local machine in the loop. Examples: Devin, Replit Agent, GitHub Copilot Coding Agent (async / issue→PR), Cursor Background Agents, Codex Cloud. Differs from local harnesses (Aider, Cline, Ark) in that the user submits a task and walks away; results return as a PR or notification.

---

## Where Ark sits

Ark is a **coding-agent harness with a workflow opinion**. Specifically:

- **Harness**, not framework — you `cargo install ark-cli` and run `ark init`; you don't `import ark-core`. (The library is re-usable but its primary consumer is the CLI binary.)
- **Coding-specialised**, not general-purpose — it scaffolds artifacts (PRD/PLAN/SPEC) explicitly oriented at code work; the tiered lifecycle (quick/standard/deep/research) maps to coding-task sizes.
- **Workflow-opinionated**, not minimal — most peers (Aider, Cline) impose ~no workflow; Ark imposes a tiered, phase-gated one (DESIGN → PLAN → REVIEW → EXECUTE → VERIFY → COMMIT → ARCHIVE).
- **Multi-platform-by-templates** — Ark is itself a harness, but it *layers on* hosts (Claude Code / Codex / OpenCode) via per-platform scaffolds. So Ark is "the workflow harness above the chat-agent harness" — a meta-layer.
- **Local, not cloud** — installs to `.ark/`, runs on your machine; no hosted component.

If pressed for a one-line positioning: *"a workflow-opinionated, multi-platform harness layer that turns a coding agent into a software-engineering process"*.

---

## Glossary of supporting terms

Twenty-five sub-terms used throughout the corpus. Each gets one paragraph; deeper treatment lives in the section noted.

### Tool use / function calling

The mechanism by which an LLM invokes a discrete function with a JSON-validated argument schema, then incorporates the result into its next turn. Provider-specific shapes (Anthropic tool_use, OpenAI function_call, Gemini function_call) converged on near-identical semantics by mid-2025. Tools are the agent's only interface to the world outside its context. See `02_infra_primitives/mcp-and-tool-registries.md`.

### MCP — Model Context Protocol

Anthropic's open protocol (released Nov 2024) standardising how LLM clients discover and call tools, fetch resources, and use prompts hosted by external servers. JSON-RPC over stdio / SSE / streamable-HTTP. Adopted by Anthropic, OpenAI (Mar 2025), Google DeepMind (Apr 2025), Cursor, Zed, Continue. The "USB-C of agent tools". Deep-dive: `02_infra_primitives/mcp-and-tool-registries.md`.

### A2A — Agent-to-Agent protocol

Google's proposal (donated to Linux Foundation Jun 2025) for agents to discover and call other agents. JSON-RPC over HTTP+SSE; capabilities advertised via `/.well-known/agent.json`. Complementary to MCP (MCP = agent↔tool, A2A = agent↔agent). Still early; `05_orchestration/agent-to-agent-protocols.md`.

### ACP — Agent Client Protocol

Zed and JetBrains' proposal (2025) for an "LSP for agents" — editor talks to agent over JSON-RPC stdio, agent emits structured edit/file actions back. Adopted by Zed, JetBrains, Kiro, OpenCode. Different surface from A2A: ACP is editor↔agent, A2A is agent↔agent. `06_platform_integration/`.

### Context window

The maximum token count an LLM can attend to in one inference. Frontier models in 2026 ship 200K–2M+ windows. *Effective* context — the slice within which recall is reliable — is much smaller; see `03_context_engineering/context-window-management.md`.

### Context engineering

The discipline of allocating the finite context window to maximise the next-step decision quality. Karpathy coined the term in mid-2025; Anthropic operationalised it Sep 2025. Spans compaction, JIT loading, structured summaries, sub-agent isolation, memory-vs-context distinction. See `03_context_engineering/`.

### Sub-agent (subagent)

A child agent dispatched by a parent for a bounded task — research, review, verification. Typically spawned with a fresh context window, restricted tools, and a return contract (often: "write a markdown summary to a known path"). Implementations: Claude Code Task tool, OpenHands AgentDelegateAction, Cline subagents, Ark's `ark-researcher`/`ark-reviewer`/`ark-verifier`. See `05_orchestration/`.

### Hook

A user-defined shell command that fires on a lifecycle event (PreToolUse, PostToolUse, SessionStart, SubagentStop, Stop, etc.). Claude Code documents 27 event types; Codex ~6; OpenCode exposes 25+ via TS plugins. Ark uses one hook (SessionStart) to inject `ark context` output. See `02_infra_primitives/hooks-and-lifecycle-events.md`.

### Slash command

An in-chat command (e.g. `/ark:design`, `/cost`) defined as a markdown file in the host platform's command directory. Resolves to a templated prompt. Ark ships ~8 slash commands per platform. As of 2026 the trend is "slash command → skill" — slash commands declined in Claude Code's docs in favour of skills.

### Skill

A self-contained behaviour pack: one directory per skill containing a `SKILL.md` (procedural instructions) plus optional scripts/templates. Discovered by file name; loaded on demand. Anthropic, Codex (`openai/skills`), Block Goose, and Cursor (skills directory) all adopted the format. Ark ships skills under `.codex/skills/ark-*/SKILL.md`. See `01_prior_art/claude-code-native.md`.

### Sandbox

An isolated execution environment that limits what an agent can affect. Layers: git worktree (file scope), container (process), microVM (kernel), capability tokens (per-call permission). E2B and Modal ship microVM sandbox-as-a-service. Ark's worktree is the file-scope layer. See `02_infra_primitives/sandboxing-and-isolation.md`.

### Worktree

A linked git working copy at a separate filesystem location, sharing the same `.git/`. Lets N branches be checked out concurrently. Used by Ark for parallel deep-tier tasks, by Cursor cloud agents (up to 8 worktrees per session), by ccswarm, by Codex parallel subagents. File-level isolation but not process / port / cache isolation.

### Codemap

A structured summary of a repository's symbols and their relationships, generated by tree-sitter or similar AST tooling, surfaced into the agent's context. Aider's repo-map (PageRank over tree-sitter symbols) is the prototype. Continue, Cursor, Plandex ship variants. Distinct from RAG: codemap is *symbolic*, RAG is *semantic*. See `03_context_engineering/codemaps-and-repo-structure-summaries.md`.

### RAG — Retrieval-Augmented Generation

Embed corpus into a vector store, retrieve top-k by query similarity, inject into prompt. Common in code agents circa 2023–2024 (Sourcegraph Cody, early Continue); the 2025–2026 consensus is that *for code* RAG is brittle (variable names ≠ semantic intent, embeddings struggle with control flow). Most coding agents now favour codemap + grep + JIT read over embeddings. See `03_context_engineering/rag-for-codebases.md`.

### JIT context loading

Loading repo content into the prompt only when the agent asks for it via tools (Read, Grep), rather than embedding chunks upfront. Claude Code's default model. Cheaper than RAG, more responsive than upfront codemap, but burns turn-count. See `03_context_engineering/jit-and-progressive-context-loading.md`.

### Memory

Durable, file-backed knowledge that survives across sessions. Distinct from context (ephemeral, in-prompt). Forms: CLAUDE.md / AGENTS.md (always-loaded), auto-memory (`/remember` in Claude Code), vector memory (mem0, Letta/MemGPT), structured memory (Ark's project + feature SPECs). See `03_context_engineering/memory-vs-context.md`.

### Journal

A workspace log of session activity — what was done, what was changed, what is in flight. Ark writes per-developer journals (`.ark/workspace/<dev>/journal-N.md`); Cline has a Memory Bank that combines memory + journal. Conceptually closer to ADRs than to chat history.

### PRD — Product Requirements Document

A short doc capturing *what* and *why* before code is written. In Ark, every task tier (quick/standard/deep/research) starts with a PRD. The "intent before edits" school (spec-kit, OpenSpec, Trellis, Ark) all use PRD-like artifacts. See `04_workflow_systems/prd-adr-feature-spec.md`.

### ADR — Architecture Decision Record

A doc capturing *one decision*, its context, and consequences. Originated in Michael Nygard's 2011 blog post. Sometimes used interchangeably with feature SPECs; the strict ADR is decision-scoped, the strict feature SPEC is feature-scoped. Ark does not formally ship ADRs but the deep-tier SPEC is ADR-shaped.

### Feature SPEC

A doc capturing what a feature *is* — goals, non-goals, architecture, constraints, validation. Ark auto-extracts feature SPECs from deep-tier PLANs at commit (`task_commit`), upserts them into `specs/features/<path>/SPEC.md`. The deep-tier `## Spec` section is promoted verbatim. See `04_workflow_systems/`.

### Plan mode

A model state where the agent thinks aloud before acting — explicit planning before tool use. Anthropic enables it via system-prompt instruction (Claude Code), OpenAI via reasoning models (o1, o3), OpenHands via explicit planner agents, Devin via a Planner module. Ark's deep tier mandates a PLAN artifact and a REVIEW loop before EXECUTE — equivalent to plan-mode but at the workflow layer, not the model layer.

### Phase

Ark-specific: a position in the task lifecycle (Design, Plan, Review, Execute, Verify, Committed, Archived, Research). Distinct from "tier" (quick/standard/deep/research) — phases are per-task, tiers are properties of the task. The `ark agent task` namespace enforces legal transitions per tier.

### Tier

Ark-specific: the ceremony level of a task (quick / standard / deep / research). Determines which phases exist and which artifacts are produced. Picked at `task new` time; promotable mid-flight via `task promote --to <tier>` (research excepted).

### Managed block

A range inside a user-owned text file delimited by HTML-comment markers (`<!-- ARK:START --> ... <!-- ARK:END -->`) that the tool may rewrite while leaving the surrounding content alone. Ark uses these to inject the CLAUDE.md / AGENTS.md preamble; Spec-kit uses similar patterns; chezmoi has analogous fences. See `02_infra_primitives/templates-and-scaffolding.md`.

### Session

A logical conversation boundary; one session = one chat history + tool-use trace + context window. Distinct from "task" (which can span many sessions). Resumed via `claude --continue`, `codex resume`, etc. Multi-checkout coherence is solved differently per harness; Ark uses per-checkout `.state.toml` files. See `02_infra_primitives/sessions-state-and-resumption.md`.

### Snapshot / checkpoint

A capture of state at a point in time. Three flavours: install snapshot (Ark's `.ark.db` capturing scaffolded files), edit snapshot (Claude `/rewind`, Cline checkpoints — restore prior file state), environment snapshot (Devin VM snapshots, Firecracker pre-warm pools). Disambiguated in `02_infra_primitives/snapshots-and-checkpoints.md`.

### Observability

Structured logs, traces, and replays for agent runs. OpenTelemetry GenAI semantic conventions standardised the schema; Langfuse, Phoenix, LangSmith, Laminar host the UI. As of 2026 Helicone is EOL (acquired or wound down). Ark emits no structured telemetry today. See `02_infra_primitives/observability-and-telemetry.md`.

### Acceptance mapping

Ark-specific (deep tier): a table in `PLAN.md` mapping every Goal (`G-N`) to one or more Validation checks (`V-*-N`). The basis of the VERIFY phase's audit. Equivalent to acceptance criteria in agile, but operationalised as a literal table.

---

## Disputed terms

A few terms are still genuinely contested and the corpus picks a side:

- **"Agent" vs "workflow"**: Anthropic's *Building Effective Agents* draws the line at autonomy. Industry uses "agent" loosely. The corpus follows Anthropic — Ark's deep tier is a *workflow*, not an *agent*, even though the artifact-filling steps inside it are agentic.
- **"Multi-agent" vs "sub-agent dispatch"**: Same concept, different connotations. We use "multi-agent" for peer/router/orchestrator-worker shapes; "sub-agent" specifically for read-only child dispatch (Ark's pattern).
- **"Infra" vs "platform"**: Vendors use both. We treat "infra" as the substrate (runtime, sandbox), "platform" as a hosted product offering that infra (E2B, AWS Bedrock).

---

## Directions for Ark

1. **Publish a positioning page using this vocabulary.** README.md and AGENTS.md describe Ark feature-by-feature; they do not stake the harness-vs-framework-vs-OS claim. A 1-page `docs/book/src/introduction.md` rewrite that opens "Ark is a coding-agent harness with a workflow opinion" makes every later doc easier to anchor.

2. **Adopt "phase" and "tier" as load-bearing public vocabulary.** They are unique to Ark — peers don't have them. Lean into the distinction in `--help` output, slash-command descriptions, and `ark context` text rendering.

3. **Surface the "harness layer above the host platform" framing.** Ark sits *between* the user's coding agent (Claude Code, Codex, OpenCode) and the user's repo. That's a teachable position — analogous to how `gh` sits between git and GitHub. Use this framing in onboarding.

4. **Audit the README for casual term-mixing.** "Workflow", "harness", "framework", and "agent" appear interchangeably today. Pick one term per concept and use it consistently — the cost is low, the legibility gain is high.

5. **Reserve "agent OS" as RFC-only.** It's already RFC 0001's framing. Don't let the term creep into shipping copy until the OS layer actually exists — the field is full of "agent OS" claims that are not OSes.
