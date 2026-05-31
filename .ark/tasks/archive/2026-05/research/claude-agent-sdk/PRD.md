# `claude-agent-sdk` PRD

---

[**What**]

Build a reference corpus on the Claude Agent SDK — concrete, current, API-level — covering the SDK surface needed to construct a multi-agent workflow substrate.

[**Why**]

ArkOS stage-1 (per RFC 001) bootstraps on an existing agent runtime. Claude Code is the most mature runtime available today, and the Claude Agent SDK is its programmable form — the same agent loop, tool inventory, hooks, session model, and subagent primitives Claude Code uses, exposed as a library in Python and TypeScript. Before committing implementation decisions for ArkOS (session lifecycle, subagent dispatch, hook-based gating, MCP integration, persistence, observability, budget enforcement), the SDK's actual capabilities and limits must be known with the same precision the rest of Ark's design work runs at.

The corpus answers concrete questions that will recur during ArkOS implementation:

- How do sessions actually work — fresh, resume, fork, one-shot? What does the persisted JSONL format contain?
- What events stream during a session, and where can a host program intercept them?
- What does the hook API look like in code, what can a hook return, and what state does it observe?
- How are subagents defined, dispatched, and how does their result return to the parent?
- How does the SDK consume MCP servers? Can the SDK itself publish MCP tools, or must that be done separately?
- What does the SDK provide for cost, tokens, and budget enforcement?
- What does the SDK NOT do that a substrate must build itself?

Conversational summaries are not enough. The corpus must cite SDK function names, hook signatures, event types, and config keys at the level required to write code against them.

[**Outcome**]

A focused corpus under `.ark/tasks/claude-agent-sdk/research/` covering, at minimum, the topics below. Each topic file cites docs.claude.com / code.claude.com (or the published SDK source where docs are thin), names actual SDK identifiers, and includes one or more minimal code snippets where they clarify the API shape.

Topic set (one file per topic, kebab-case names; topics may merge if the natural unit is shared):

1. `01_overview-and-relationship-to-claude-code.md` — what the SDK is, what it shares with Claude Code, what's library-only vs. CLI-only, languages, model and provider lock-in posture.
2. `02_sessions.md` — fresh / resume / fork / one-shot patterns; the on-disk JSONL session format; settings sources (CLAUDE.md, AGENTS.md, skills, commands); session listing and store adapters.
3. `03_streaming-events.md` — the typed event stream (system / assistant / tool-use / tool-result / result / error); how to consume it asynchronously; how to detect "turn complete" and "session complete"; what telemetry each event carries.
4. `04_hooks.md` — `PreToolUse` / `PostToolUse` / `Stop` / `SessionStart` / `SessionEnd` / `UserPromptSubmit`; exact signatures; what a hook can return (deny / approve / mutate); what context the hook sees (cwd, tool args, session id).
5. `05_tools-and-permissions.md` — built-in tool inventory; how to enable / disable tools per session; how to scope permissions; the SDK equivalent of `--dangerously-skip-permissions` and finer-grained alternatives.
6. `06_subagents.md` — `AgentDefinition` vs filesystem `.claude/agents/*.md`; how a parent invokes a subagent; whether the subagent's result returns as parsed data or only as a chat text block; whether subagents can spawn subagents (recursion depth); concurrency between subagents in one parent session.
7. `07_mcp-integration.md` — configuring MCP servers via `mcpServers` option / `.mcp.json`; transports (stdio / HTTP / SSE); tool / resource / prompt consumption from the agent's side; whether the SDK provides any helper for *publishing* an MCP server (and, if not, what to use instead).
8. `08_cost-and-budget.md` — per-message / per-session token and cost surfaces; when these are observable (live during stream vs. at result); patterns for enforcing a budget ceiling with auto-abort.
9. `09_concurrency-and-parallelism.md` — running multiple SDK sessions in one process; thread / async safety; per-session isolation guarantees; fan-out patterns for parallel subagents or parallel sibling tasks.
10. `10_persistence-and-memory.md` — what the SDK persists automatically (session JSONL, transcripts), what it does not (semantic memory, KB, cross-project recall), and adapter / extension points for plugging in a memory layer.
11. `11_skills-and-agents-md.md` — filesystem `.claude/skills/` and `AGENTS.md`/`CLAUDE.md` loading via `settingSources`; how skills are discovered and invoked; portability across SDK and CLI surfaces.
12. `12_extended-thinking-and-model-config.md` — model selection, extended-thinking budget knobs, per-agent overrides, fallback / retry behavior.
13. `13_telemetry-and-observability.md` — what's emitted for logs / metrics / traces; OpenTelemetry hooks (if any); recommended patterns for external observability.
14. `14_limits-and-gaps.md` — what the SDK explicitly does NOT provide that a substrate of many agents will need: orchestration DSL, agent registry, multi-user isolation, cost tracking beyond per-message, distributed state, task queues, persistent semantic memory. This is the "what we'd build on top" inventory.
15. `99_SYNTHESIS.md` — cross-corpus reading: which SDK primitives are load-bearing for ArkOS stage 1, which are pleasant defaults, which are friction (vendor lock-in, self-grading bias, single-level subagents), and which gaps require explicit substrate-side implementation. Direct citations into the per-topic files. No new claims that aren't in those files.

Corpus discipline (inherited from `agent-harness-infra`):

- Cite primary docs (docs.claude.com, code.claude.com, anthropic/claude-agent-sdk-python, anthropic/claude-agent-sdk-typescript) over secondary commentary.
- Name SDK identifiers (functions, classes, hook names, event types) verbatim.
- Where docs are ambiguous, say so — do not guess.
- Pin version: state which SDK version (or doc snapshot date) the topic was researched against.
- Code snippets are minimal — just enough to anchor the API shape, not full programs.

The corpus does NOT describe what ArkOS will be built on top of the SDK; it describes what the SDK is. ArkOS architecture choices happen in a follow-up task once the corpus stabilizes.

[**Related Specs**]

- `specs/features/subagent-support/SPEC.md` — current Ark uses subagents via Claude Code's filesystem `.claude/agents/*.md` discovery. The corpus's topic 06 (`subagents.md`) clarifies the SDK side of the same concept (`AgentDefinition` programmatic registration, subagent recursion limits, parent-result return shape) — relevant context for whether ArkOS's substrate-level subagent service borrows or replaces this pattern.

[**SPEC Path**]

ignored — research tier
