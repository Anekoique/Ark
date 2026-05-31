# OpenHands (formerly OpenDevin)

## Identity

- **Name:** OpenHands (project renamed from OpenDevin during 2024)
- **Repos:** https://github.com/OpenHands/OpenHands (main app) and https://github.com/OpenHands/software-agent-sdk (composable SDK, V1)
- **License:** MIT
- **Primary maintainer:** All Hands AI (commercial entity); academic roots with collaborators across CMU, UIUC, Berkeley
- **Language:** Python (core), TypeScript (frontend)
- **Stars / momentum:** 74,235 stars (as of 2026-05-20, queried via `gh repo view`). Most-starred OSS coding-agent project after Codex CLI. The SDK has its own MLSys 2026 paper ("The OpenHands Software Agent SDK: A Composable and Extensible Foundation for Production Agents", arXiv 2511.03690).
- **Homepages:** https://openhands.dev (cloud), https://docs.openhands.dev

## Positioning

OpenHands is the **most architecturally serious** open-source coding agent and the one that most closely resembles a research platform. Originally launched as "OpenDevin" to be an OSS reproduction of Cognition's Devin, it pivoted into a general-purpose autonomous agent platform with four delivery modes:

1. **Python SDK** for programmable agents
2. **CLI** ("comparable to Claude Code or Codex" — their words)
3. **Desktop GUI** (React frontend + FastAPI backend)
4. **Cloud platform** (Slack/Jira/Linear integrations, paid tier)

Its core innovation is a clean **event-stream architecture**: agents emit Actions, runtimes return Observations, and the whole conversation is an append-only log of typed events. Memory, microagents, sub-agent delegation, security review, and condensation are all "small auxiliary services hanging off the event stream." Among prior art, this is the cleanest separation between "agent loop" and "everything around it."

## Primitives

User-facing nouns:

- **Conversation / Session** — the running agent loop bound to a workspace.
- **Agent** — stateless object that emits Actions given the event history. Default is `CodeActAgent`.
- **Action** — a typed command (RunCommandAction, FileEditAction, BrowseURLAction, MessageAction, IPythonRunCellAction, AgentDelegateAction…). Roughly 15 built-in action types.
- **Observation** — the typed response from the Runtime after executing an Action.
- **Event** — the parent abstraction over Action and Observation. The conversation is `events: List[Event]`.
- **Runtime / Workspace** — the sandbox where Actions run. Choices include local process, Docker, E2B, Modal, Daytona, Kubernetes.
- **Microagent** — a small markdown file that injects context when triggered.
- **Condenser** — a service that summarizes old events to save tokens.
- **Trajectory** — a saved (events, outcomes) pair, often used for fine-tuning or benchmarking.

User-facing verbs (depending on delivery mode):

- SDK: `Conversation.run()`, `agent.step()`, `runtime.execute(action)`
- CLI: `openhands` (chat); flags for runtime, agent, model, sandbox
- GUI: chat box + workspace pane + action timeline
- Cloud: Slack DMs, Jira ticket assignment

## Workflow model

Representative flow (SDK / CLI variant):

1. **Configure** the agent (model via LiteLLM, runtime, microagents, condenser).
2. **User message** → enters event log as `MessageAction`.
3. **Agent.step()** reads the event log + microagents, calls the LLM, returns the next `Action`.
4. **Runtime.execute(action)** returns an `Observation`. For `RunCommandAction` it runs in the sandbox shell; for `FileEditAction` it edits via diff-apply; for `BrowseURLAction` it drives a headless browser; for `AgentDelegateAction` it spawns a sub-agent.
5. **Loop** until the agent emits `MessageAction(content=..., wait_for_response=True)` or `AgentFinishAction`.
6. **Condenser** may summarize old events between iterations.
7. **Save trajectory** for replay/eval.

There is **no formal PLAN/REVIEW/VERIFY phase split**. The agent prompt encodes the workflow. The default `CodeActAgent` is one-shot autonomous; specialized agents (BrowsingAgent, planning variants) can be swapped in.

## Context & memory

**Context window management** — three coordinated mechanisms:

1. **Event log** is the source of truth. The prompt is built fresh from events each step.
2. **Condenser** — pluggable summarizer. Default condenser "cuts API spend by ~2× on long sessions with no measurable quality loss" (per their MLSys paper).
3. **Microagents** — keyword- or repo-triggered context injection. Two flavors:
   - **Repo microagents** — auto-loaded on repo open; provide project conventions.
   - **Knowledge microagents** — keyword-triggered; e.g., a "kubernetes" microagent loads only when the user mentions k8s.

**Persistent memory:**

- Trajectory files (`.openhands/trajectories/*.json`).
- Microagents (markdown, checked into repo at `.openhands/microagents/`).
- Cloud tier: session history per user/workspace.

**No native RAG over codebase** in the default agent — the agent must navigate via shell commands (`find`, `grep`, `cat`). This is by design (research finding: LLM-centric simple commands beat heavy retrieval in their SWE-Bench numbers).

## Tool / capability surface

**Built-in actions/tools (CodeActAgent default set):**

- `RunCommandAction` — shell command in sandbox
- `IPythonRunCellAction` — Jupyter cell in sandbox kernel
- `FileEditAction`, `FileReadAction`
- `BrowseURLAction`, `BrowseInteractiveAction` — Chromium headless
- `AgentDelegateAction` — spawn sub-agent (with own Conversation)
- `MessageAction`, `AgentFinishAction`
- `RecallAction` — query memory store

**MCP support:** Yes — the SDK ships first-class MCP client support and ~24 examples. MCP servers can register additional actions.

**Plugin model:** Three avenues:

- **Custom Agents** — subclass `BaseAgent` and override `step()`.
- **Microagents** — markdown, no code.
- **MCP servers** — language-agnostic.

**Sandbox boundaries:** Mature. Choices include:

- **DockerRuntime** (default) — builds an "OH runtime image" per session, runs an internal API server inside the container that listens for action execution requests.
- **E2B** — managed cloud sandboxes.
- **Modal** — gVisor-based sandboxes with GPU on demand.
- **Daytona, Kubernetes, local** — also supported.

All sandboxes implement the same Runtime API so swapping is config-only.

## Integration model

**All of them.** SDK for embedding, CLI for terminal, GUI for desktop, Cloud for managed teams. Slack/Jira/Linear integrations on the cloud tier dispatch agents from chat or tickets.

## Multi-agent / orchestration

**First-class via `AgentDelegateAction`.** Any agent can spawn a child agent with a focused task; the child gets its own event log; the parent receives the child's final result as an Observation. This is the cleanest "delegate to a sub-agent" primitive in the OSS space.

Patterns observed in the wild:

- **CodeActAgent → BrowsingAgent** delegation for tasks that need both.
- **Planner → Executor** patterns via two specialized agents.
- **N parallel `Conversation`s** across the cloud control plane for batch work.

## Spec / artifact system

**Microagents are the artifact system.** They are markdown files (frontmatter + body) checked into `.openhands/microagents/` of a project. Two types:

- **Repo microagent** — always loaded for that repo (project-level conventions).
- **Knowledge microagent** — triggered by keywords (declared in frontmatter).

No formal PRD/PLAN/REVIEW/VERIFY pipeline. The cloud product has a notion of "tasks" (Linear-style), but they're chat-rooted.

## Strengths

- **Architecture is the best in show.** Event stream + pluggable Runtime + pluggable Agent + condenser as a service is a textbook design.
- **Multiple runtimes (Docker / E2B / Modal / Kubernetes) with the same API.** No other OSS agent has this breadth.
- **Real isolation by default.** Docker per session is uncommon among "terminal-first" peers; this puts OpenHands closer to Devin's security posture than to Aider/Cline.
- **Microagents are elegant.** Keyword-triggered context injection is JIT-loading done right.
- **SDK + CLI + GUI + Cloud.** A coherent four-mode product from a single codebase.
- **Strong benchmarks.** 72% on SWE-Bench Verified with Claude Sonnet 4.5 + extended thinking (their MLSys paper).
- **Multi-LLM via LiteLLM** as a first-class concern.

## Weaknesses / gaps

- **Heavy.** Docker-per-session is the right answer for security but the wrong default for "I want to refactor one file." Local-runtime mode exists but the documentation pushes Docker.
- **No explicit workflow ceremony.** The default agent is one-shot autonomous; users who want plan-review-execute have to author it themselves via prompts or specialized agents.
- **No SPEC promotion.** Microagents are user-authored, not extracted from completed work.
- **No journal.** No equivalent of Ark's workspace per-checkout journal.
- **No tier-based ceremony.** "Quick patch" vs "deep refactor" goes through the same agent path.
- **Onboarding is harder than Aider/Cline.** Docker prerequisite, runtime selection, agent selection — more knobs.
- **The Python SDK is the source of truth; CLI/GUI lag features.**

## Directions for Ark

1. **Event-stream model as a Rust analog.** Ark's task state machine is phase-based (DESIGN/PLAN/...); a parallel event-stream representation (typed events per `ark agent` invocation, append-only under `.ark/tasks/<slug>/events.jsonl`) would give us trajectories for free — replay, audit, fine-tuning data. The state machine remains the user-facing model; events are the backing store.
2. **Microagent-style trigger-based context.** Ark's project SPECs are always-loaded; feature SPECs are listed in the PRD. A "knowledge SPEC" tier that auto-loads only when keywords appear in the PRD or active conversation would let Ark grow a richer library without ballooning per-task context. (Microagents are mostly equivalent to a registry of `if matches(keyword) → include(file)` rules.)
3. **Runtime abstraction.** Ark currently assumes the agent runs in the user's shell. As Ark grows toward Phase 1 (ArkOS positioning), pluggable runtimes (local, Docker, gVisor sandbox, remote VM via SSH) would be the cleanest path. Steal OpenHands' Runtime API shape (`execute(Action) -> Observation`).
4. **Condenser as a service.** OpenHands' condenser is a generic compression layer. Ark could ship a `ark agent context compact` command that takes a conversation transcript (or `.ark/tasks/<slug>/events.jsonl`) and emits a summary artifact, parametrized by `--keep-recent N` and `--target-tokens M`. Subagents would use it before long handoffs.
5. **AgentDelegate pattern formalization.** Ark already has researcher/reviewer/verifier subagents (per `specs/features/subagent-support/SPEC.md`). OpenHands' `AgentDelegateAction` shows that any agent should be able to delegate to *any* other agent, not just the three named roles. A generalized `ark agent dispatch --agent <name> --prompt <p>` slash command would let users define ad-hoc subagents per task in `.claude/agents/` and have Ark wire them.

## Sources

- [OpenHands/OpenHands on GitHub](https://github.com/OpenHands/OpenHands) (queried 2026-05-20)
- [OpenHands/software-agent-sdk on GitHub](https://github.com/OpenHands/software-agent-sdk/)
- [OpenHands Docs: Software Agent SDK](https://docs.openhands.dev/sdk)
- [The OpenHands Software Agent SDK paper (arXiv 2511.03690)](https://arxiv.org/abs/2511.03690) — MLSys 2026
- [Runtime Architecture — All Hands Docs](https://docs.all-hands.dev/modules/usage/architecture/runtime)
- [E2B Runtime — OpenHands Docs](https://docs.openhands.dev/openhands/usage/v0/runtimes/V0_e2b)
- [OpenHands Deep Dive — DEV Community](https://dev.to/truongpx396/openhands-deep-dive-build-your-own-guide-1al0)
- [Best Code Execution Sandbox for OpenHands — Modal](https://modal.com/resources/best-sandbox-openhands)
- [Product Variants — DeepWiki](https://deepwiki.com/OpenHands/OpenHands/1.3-product-variants)
