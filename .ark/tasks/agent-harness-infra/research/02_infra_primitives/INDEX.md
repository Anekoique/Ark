# Infra Primitives — Index

This subdirectory surveys the foundational primitives that every coding-agent
harness either implements, vends, or punts on: **isolation, tool wiring,
lifecycle hooks, session state, memory, observability, snapshots, and
scaffolding**. Each file follows the same shape — define the primitive, survey
how leading harnesses build it (Claude Code, Codex, Aider, Cline, OpenHands,
Cursor, Continue, Goose, MCP), contrast against what Ark already does today
(citing `crates/ark-core/src/...`), and end with **Directions for Ark** —
concrete candidate features tied to real code locations or feature SPECs.

> Scope rule: this section is about *infrastructure*. Context engineering
> (RAG, codemaps, JIT loading) lives in `03_context_engineering/`. Workflow
> ceremony (plan-execute-verify, spec promotion) lives in `04_workflow_systems/`.

## Files

| File | Takeaway |
| ---- | -------- |
| [`sandboxing-and-isolation.md`](sandboxing-and-isolation.md) | Worktrees are the dominant *file* isolation; containers add network / process isolation; microVMs (Firecracker) raise the floor when you don't trust the agent. Ark sits at the worktree tier. |
| [`mcp-and-tool-registries.md`](mcp-and-tool-registries.md) | MCP is now the lingua franca for tool wiring; transports converged on stdio + Streamable HTTP; authorization piggybacks on OAuth 2.1. Ark consumes nothing yet — *exposing* `ark agent` as MCP would multiply reach. |
| [`hooks-and-lifecycle-events.md`](hooks-and-lifecycle-events.md) | Claude Code and Codex converged on a ~10-event hook taxonomy (PreToolUse, PostToolUse, SessionStart, Stop, UserPromptSubmit, …); Ark uses *one* hook today (`SessionStart` for context injection). |
| [`sessions-state-and-resumption.md`](sessions-state-and-resumption.md) | Codex stores rollouts as JSONL; Claude Code does in-process; OpenHands binds session ↔ container; Ark's `.state.toml` + per-worktree-focus model is unusually disciplined for multi-checkout coherence. |
| [`memory-systems.md`](memory-systems.md) | CLAUDE.md / AGENTS.md / `.openhands/microagents/` files are the cross-harness shape; auto-memory (Claude's `/memory`) and Cline-style memory-banks bolt persistence on top. Ark's SPEC tree is a *structured* memory the others lack. |
| [`observability-and-telemetry.md`](observability-and-telemetry.md) | OpenTelemetry's GenAI semconv shipped 2026; Langfuse / Phoenix / LangSmith implement it. Ark emits nothing structured today. |
| [`snapshots-and-checkpoints.md`](snapshots-and-checkpoints.md) | Claude Code's `/rewind` checkpoints edits per-prompt; Ark's `.ark.db` is install-snapshot (different concept). Containers/VMs offer image snapshots. Git is the universal fallback. |
| [`templates-and-scaffolding.md`](templates-and-scaffolding.md) | Cookiecutter generates once; Copier supports `update`; Ark's `include_dir!` + manifest-hash-tracking + managed blocks is closer to Copier's update model than to Cookiecutter's. |

## Cross-cutting threads

- **Authorization is converging on OAuth 2.1 + PKCE.** MCP's authorization spec
  (`modelcontextprotocol.io/specification/draft/basic/authorization`) is the
  shared baseline; hook permissions (Claude Code's allow/deny/ask/defer) follow
  the same model at a different layer.
- **Per-checkout state is the right granularity for multi-task work.** Ark
  enforces it (`.state.toml` per worktree, `crates/ark-core/src/state/checkout/`);
  most competitors let two terminals stomp each other.
- **Snapshots come in three flavours.** Install snapshots (Ark's `.ark.db`),
  edit checkpoints (Claude `/rewind`), and environment snapshots (Firecracker
  pre-warmed VM images). Each solves a different recovery problem.
- **Templates need a `migrate` story.** Cookiecutter ignored it for a decade;
  Copier built a whole tool around it; Ark already pays this cost via
  `manifest.hashes` + `update_managed_block`.

## Reading order

If you only read three: `sandboxing-and-isolation.md`,
`hooks-and-lifecycle-events.md`, `sessions-state-and-resumption.md`. They
define the *runtime envelope* Ark sits inside. The others refine it.
