# Multi-Agent Architectures — the zoo

Compiled 2026-05-20. The architecture taxonomy across shipped agent harnesses, with Anthropic's "Building Effective Agents" essay (Dec 2024) as the spine.

## Anthropic's taxonomy

Anthropic distinguishes **workflows** (LLMs orchestrated via pre-defined patterns) from **agents** (LLMs that dynamically direct their own processes). The essay enumerates five workflow patterns:

1. **Prompt chaining** — sequential steps, each step's output feeds the next. Good when subtasks decompose cleanly. Example: outline → draft → polish.
2. **Routing** — a classifier picks one of N specialists per input. Good when inputs are heterogeneous. Example: support ticket → billing-agent or refund-agent.
3. **Parallelization** — same input fanned out to N workers (sectioning) or to N voters (voting). Good for independent subtasks or quality-via-consensus.
4. **Orchestrator-Workers** — central LLM dynamically breaks down tasks, delegates to worker LLMs, synthesizes results. Good when subtasks are not predictable upfront (multi-file coding, research).
5. **Evaluator-Optimizer** — one LLM generates, another critiques, loop until passing. Good when iteration improves output (writing, code-review).

Then "**agents**" — autonomous loop, plan-act-observe, with tools. Higher cost and latency for higher autonomy.

The essay's design rule: *start with the simplest pattern that solves the task. Composability is the goal; complexity is the cost.*

## The four shipped architectures

### Solo agent — Aider, swe-agent baseline

One model, one loop, no delegation. Aider runs as a terminal pair-programmer: the model reads a repo-map, edits files, commits per turn. No second LLM in the picture except optionally the architect/editor split.

**When it works.** Single-file edits, well-scoped diffs, repo small enough to fit in context. Aider's `--map-tokens` heuristic plus tree-sitter symbol extraction keeps context tight.

**Variant — architect/editor split.** Aider's two-inference mode: Architect (high-reasoning model) proposes; Editor (cheap precise model) emits diffs. SOTA Exercism polyglot benchmark score with o1-preview architect + DeepSeek/o1-mini editor was 85%. Even same-model-both-roles improves over solo because the model gets two turns instead of one.

This is the *minimal multi-agent shape* — still one logical worker, but two prompts.

### Master-worker (orchestrator-worker) — Claude Code, OpenHands, Cline, Ark

A parent LLM dispatches specialist children, awaits their summaries, integrates. **The dominant pattern in shipping products.**

Examples:
- **Claude Code Task tool.** Single mechanism that spawns all subagents. Parent picks a subagent type (Explore, Plan, general-purpose, Bash), provides a prompt + short description; tool spins up a fresh Claude with its own context window, restricted tools, configured model. Up to 10 concurrent tasks. Parallel subagents are context-isolated — they cannot see each other.
- **OpenHands AgentDelegateAction.** Standard tool in the `openhands.tools` package. Parent spawns sub-agents and blocks until all complete. Each sub-agent inherits the parent's model + workspace context but runs as an independent conversation. Returns a structured summary.
- **Cline subagents.** Each subagent operates in its own session with configurable permissions, tool access, turn limits. Cannot write files, cannot use browser/MCP/web-search, cannot spawn their own subagents. Read-only research role.
- **Ark.** Parent (main session) dispatches `ark-researcher` / `ark-reviewer` / `ark-verifier` via the platform's subagent tool (Task on Claude, subagent tool on Codex, `task` on OpenCode). Children persist to disk; parent reads back. C-15 forbids children from spawning more subagents.

**When it works.** Workflows where the parent needs author-control of integration but the children's work can be parallelized or scoped. Token economy improves because children get fresh context.

### Peer-to-peer (group chat) — AutoGen v1, CrewAI Crews

Multiple agents converse with each other in a shared transcript; a moderator or coordinator picks the next speaker.

- **AutoGen.** Microsoft Research. v1 was conversation-centric: two-agent chat, sequential group chat, nested chat, hierarchical coordination. **As of 2026, AutoGen v1 is in maintenance mode; Microsoft has consolidated onto the broader Microsoft Agent Framework.** v2 API reached 1.0 GA with major architectural changes.
- **CrewAI Crews.** Autonomous teams where agents have agency — they decide when to delegate, when to ask questions, how to approach tasks. Hierarchical process mode (early 2024) auto-generates a manager agent that coordinates worker agents. CrewAI Flows mode is the event-driven peer for production workloads.

**When it works.** Open-ended problem-solving, role-play simulations, brainstorming. **Caveat:** in shipping coding products, peer-chat has lost ground to master-worker. CrewAI's hierarchical mode is converging back toward orchestrator-worker (auto-generated manager = supervisor).

### Debate / self-critique — Constitutional AI, evaluator-optimizer

Multiple LLM passes critique each other or themselves until a quality bar is met.

- **Constitutional AI (Anthropic).** In the supervised phase, the model revises harmful responses via self-critique. In the RL phase, two responses to the same prompt are compared by an AI evaluator (constitution-guided); AI-generated preferences train the reward model.
- **Multi-agent debate.** Research direction (Du et al. 2023, "Improving Factuality and Reasoning in Language Models through Multiagent Debate"). Multiple instances argue, converge. **Failure mode from "Talk Isn't Always Cheap" (Sep 2025, arXiv 2509.05396): debate can decrease accuracy over time — agents anchor on confident-but-wrong claims and reinforce them.**

**When it works.** Critique quality. Aider's architect/editor and Ark's PLAN ⇄ REVIEW are evaluator-optimizer instances; the explicit evaluator (different prompt, different role) tends to outperform single-model self-correction.

## Where Ark sits

**Master-worker, three read-only specialists, parent-only dispatch.**

- One parent (main session in Claude Code / Codex / OpenCode) holds the executive loop.
- Three named specialists with documented scopes (researcher / reviewer / verifier).
- Children are read-only outside narrow write paths (C-7..C-10).
- No peer chat; no debate among children; no child-spawns-grandchild.
- Persistence is filesystem (`<task>/research/*.md`, `NN_REVIEW.md`, `VERIFY.md`), not in-memory.

This places Ark cleanly in Anthropic's *orchestrator-workers* slot, with **fixed (not dynamic) decomposition** — Ark knows at design-time which roles exist; the parent picks among the three based on workflow phase.

## Comparative table

| Architecture | Examples | Decomposition | Comms medium | Failure tendency | Token cost |
| ------------ | -------- | ------------- | ------------ | ---------------- | ---------- |
| Solo | Aider, swe-agent | n/a | n/a | Context exhaustion | Lowest |
| Architect/Editor | Aider | Fixed 2-step | Plan → diff handoff | Plan-diff drift | Low |
| Master-worker | Claude Code, OpenHands, Cline, Ark, LangGraph supervisor | Dynamic OR fixed roles | Tool call + summary; or file | Lost context across handoff | Medium |
| Peer (group chat) | AutoGen v1, CrewAI Crews | Emergent, agents decide | Shared transcript | Drift, deadlock, dispatch storm | High |
| Hierarchical w/ auto-manager | CrewAI hierarchical mode | Manager-generated | Manager → workers | Convergence to master-worker | High |
| Debate / Self-critique | Constitutional AI, Du et al. multi-agent debate | Multiple passes on same task | Critique loop | Confident-wrong reinforcement | Highest |

## Failure modes by architecture

- **Solo.** Context exhaustion; can't parallelize independent subtasks; one prompt for all roles dilutes specialization.
- **Master-worker.** Parent becomes bottleneck; child summaries lossy; integration logic in parent must reason about heterogeneous outputs.
- **Peer.** Conversation drift, dispatch storms (agent A pings B pings C pings A), no clear stop condition. AutoGen v1 → maintenance mode is one data point.
- **Debate.** Accuracy degradation with rounds when agents anchor on wrong claims. Cost grows linearly with rounds.

## Directions for Ark

1. **Document Ark's architectural position explicitly in `AGENTS.md`.** "Ark is a master-worker harness with three read-only specialist children" is a useful one-liner; today the SPEC encodes it but the user-facing docs do not name the pattern.
2. **Parallel researcher dispatch.** Anthropic's parallelization pattern (sectioning) fits research-tier perfectly — N independent topics fanned out to N researcher invocations. Today main session dispatches researchers serially. Open question: can `/ark:research` enumerate topics and announce-then-dispatch in parallel? See `parallelism-and-coordination.md` for the worktree-vs-shared-context tradeoff.
3. **Evaluator-optimizer for VERIFY.** `ark-verifier` is currently a one-shot audit. The evaluator-optimizer pattern would run the verifier, let the parent fix, and re-run until PASS — bounded by `task.toml.max_iterations`. The risk per "Talk Isn't Always Cheap" is accuracy decay; a hard cap mitigates.
4. **Reject peer-chat as a future direction.** Production data (AutoGen → maintenance, CrewAI hierarchical auto-generates a manager) says peer chat is dominated by master-worker for coding workloads. Ark should not chase that pattern.
5. **Architect/editor split for cheap-model paths.** Aider's data shows a planner-emits-prose + editor-emits-diff split improves results when the budget model can't produce well-formed diffs. Could be a workflow.md option: "PLAN with model X, EXECUTE with model Y" — Ark already supports per-role model selection via platform configs; document it.

## Sources

- [Building Effective Agents (Anthropic, Dec 2024)](https://www.anthropic.com/research/building-effective-agents) — five-pattern taxonomy
- [Five Agentic Workflow Patterns (Daniel Davenport)](https://danieldavenport.medium.com/five-agentic-workflow-patterns-9f03e356d031) — restatement of the taxonomy
- [Aider Architect/Editor mode (2024-09-26)](https://aider.chat/2024/09/26/architect.html) — solo → 2-inference split
- [Claude Code Subagents — Task tool architecture (Aeon Flex, Medium)](https://medium.com/@neonmaxima/claude-code-subagents-how-the-task-tool-actually-distributes-work-e5fe19f48584) — Task as the dispatch primitive
- [OpenHands AgentDelegateAction docs](https://docs.openhands.dev/sdk/guides/agent-delegation) — sub-agent delegation pattern
- [Cline subagents docs](https://docs.cline.bot/features/subagents) — read-only specialist pattern
- [AutoGen vs CrewAI 2026 comparison (is4.ai)](https://is4.ai/blog/our-blog-1/autogen-vs-crewai-comparison-2026-332) — current status of both frameworks
- [LangGraph Supervisor patterns (LangChain reference)](https://reference.langchain.com/python/langgraph-supervisor) — StateGraph supervisor mode
- [Talk Isn't Always Cheap — Multi-Agent Debate failure modes (arXiv 2509.05396)](https://arxiv.org/abs/2509.05396) — accuracy degradation evidence
- [Constitutional AI: Harmlessness from AI Feedback (Anthropic, 2022)](https://arxiv.org/abs/2212.08073) — self-critique architecture
