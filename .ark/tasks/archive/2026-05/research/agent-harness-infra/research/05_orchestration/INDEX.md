# 05 — Orchestration

How agent harnesses coordinate multiple agents — from solo loops through master-worker patterns to peer chats and inter-vendor protocols. Covers the architecture zoo, dispatch models, parallelism, A2A protocols, sub-agent isolation, and the failure modes that bite when orchestration goes wrong.

Section asks: **what shape of multi-agent orchestration should Ark commit to, and where are the next-step gains?**

Ark today: ships three subagents (`ark-researcher`, `ark-reviewer`, `ark-verifier`) per platform (Claude, Codex, OpenCode); main session is the sole dispatcher; persistence is file-based (markdown under `<task>/research/`); worktrees provide per-task isolation. `subagent-support` SPEC at `.ark/specs/features/subagent-support/SPEC.md` encodes C-6 (recursion guard), C-7..C-10 (per-agent write scope), C-11 (no self-fix), C-15 (parent-only dispatch), C-28 (post-dispatch git verification of scope).

| File | One-line takeaway |
| ---- | ----------------- |
| [`multi-agent-architectures.md`](multi-agent-architectures.md) | Architecture zoo — solo (Aider), master-worker (Claude Code Task tool, OpenHands AgentDelegateAction), peer (AutoGen group chat, CrewAI crew), debate (Constitutional AI / self-critique). Anthropic's "Building Effective Agents" five-pattern taxonomy. Ark sits in master-worker — single parent, three read-only specialist children. |
| [`researcher-reviewer-verifier.md`](researcher-reviewer-verifier.md) | The trio pattern Ark ships. Specialization rationale: token economics + attention isolation + per-role prompts + per-role model selection. Compared with planner/coder/tester (Devin), recon/exploit/cleanup (security), red/blue (RL). Anti-pattern: too many specialists (dispatch overhead > benefit). Ark's three is intentionally minimal. |
| [`dispatch-models.md`](dispatch-models.md) | Task-tool (Claude Code), Cline's CLI subprocess spawn, AutoGen group chat, LangGraph supervisor StateGraph, OpenAI Agents SDK Handoff. Sync vs async; foreground (await) vs background (notify). Ark's pattern: synchronous foreground dispatch, parent waits, child persists, parent reads back. |
| [`parallelism-and-coordination.md`](parallelism-and-coordination.md) | Running N agents in parallel: worktree isolation (Ark, ccswarm, Cursor cloud agents), container isolation (OpenHands runtime, Devin VMs), shared task queues. Coordination via locks, journals, atomic file writes. Ark's per-checkout `.state.toml`. Failure: torn writes, port collisions, lost updates. |
| [`agent-to-agent-protocols.md`](agent-to-agent-protocols.md) | Emerging A2A: Google's Agent2Agent (Linux Foundation, JSON-RPC + SSE), ACP (Zed/JetBrains, JSON-RPC over stdio), MCP as A2A-adjacent (tools vs. peer comms). Vertical (MCP) vs. horizontal (A2A). Where Ark could expose `ark agent` as MCP server or ACP-compatible agent. |
| [`subagent-isolation-and-context.md`](subagent-isolation-and-context.md) | Sub-agents as context firewalls. Fresh-context principle (Claude Code, Cline). Disk persistence as the cross-context channel (Ark's `research/<topic>.md`, deepagents file offload). Read-only sub-agents (Cline subagents, Ark's reviewer/verifier). Scoped permissions, sandboxed file access. |
| [`orchestration-failure-modes.md`](orchestration-failure-modes.md) | The hall of pain: AutoGPT infinite loops + billing blowouts, BabyAGI plan-reinvention cycles, multi-agent debate accuracy degradation, dispatch storms, lost context across handoffs, hallucinated state. Mitigations: budgets, depth limits, cycle detection, recursion guards. Ark's existing guards (C-6/C-15) audited. |

## Cross-cutting findings

1. **Master-worker dominates production.** Claude Code Task tool, Cline subagents, OpenHands AgentDelegateAction, LangGraph supervisor, Devin Planner+Coder+Critic. Peer chat (AutoGen v1) was moved to maintenance mode; CrewAI's hierarchical mode auto-generates a manager. The "flat peer society" pattern is rare in shipping products.
2. **Read-only specialists are the safe default.** Cline subagents cannot write files. Ark's reviewer/verifier are gate-only. Codex's `multi_agent_v2.enabled = false` disables nested-agent spawning. The lesson from AutoGPT runaway loops is: limit autonomy at the leaves.
3. **Files are the universal handoff medium.** Persistence-to-disk (Ark `research/`, Cursor checkpoints, deepagents file offload) beats in-context passing — context windows are too small and per-token cost too high to round-trip large payloads through the parent.
4. **Sandbox/worktree-per-agent is the default isolation pattern.** Cursor cloud agents, Devin VMs, ccswarm worktrees, OpenHands Docker runtimes, Ark's `--worktree`. Linked git worktrees handle file conflicts but NOT port/cache/secret collisions.
5. **A2A is real but young.** Google A2A donated to Linux Foundation (Jun 2025); ACP shipped v1 with Zed+JetBrains+Kiro+OpenCode adoption. MCP and A2A are complementary (tools vs. peer comms). Ark could expose `ark agent` over either.
6. **Failure modes are well-catalogued.** Token blowouts (AutoGPT), plan-reinvention (BabyAGI), debate accuracy decay (Sep 2025 paper), dispatch storms, attribution loss. Hard caps and recursion guards are table-stakes.

## Where Ark already aligns

- **Master-worker.** Parent-dispatches-children, no peer chat. Matches the dominant pattern.
- **File-based handoff.** Researcher writes markdown; parent reads back. Matches "persistence beats in-context".
- **Read-only specialists.** Reviewer/verifier are gate-only (C-11). Researcher is read-only outside `research/`. Recursion guard (C-6) on every agent.
- **Worktree-per-task isolation.** Deep-tier mandate; cleanup is explicit.
- **Tight scope walls.** Each agent's prompt enumerates Write-ALLOWED + Write-FORBIDDEN; main session reverts out-of-scope writes via `git restore` (C-28).

## Where Ark could differentiate

See per-file **Directions for Ark** sections. High-signal candidates that recur:

- **Expose `ark agent` over A2A or ACP** so other agents (Cursor cloud, Zed, JetBrains AI) can drive Ark's workflow primitives.
- **Per-phase agent dispatch budgets** in `task.toml` — depth limit + token ceiling per role, surfaced in `ark context`.
- **Parallel researcher dispatch** for the corpus-building case (research tier ships N topics; today they're serial).
- **Background-agent UX** for long-running researchers (notify-when-done; no await-blocking the parent).
- **ACP-compatible Ark adapter** so the binary can be the agent half of an editor↔agent pair.
