# 01_prior_art — Index

CLI-class and terminal-class agent harnesses — the closest peers to Ark. Plus adjacent peers (IDE-class, web-class, cloud-platform) for full landscape coverage.

Compiled 2026-05-20. The nine **required** profiles for this corpus pass land in this round (rows 1-9 below). Rows 10+ are pre-existing entries from prior research runs covering adjacent peers — surfaced here so the corpus is navigable as a whole.

Each file follows a fixed template (Identity / Positioning / Primitives / Workflow / Context & memory / Tools / Integration / Multi-agent / Spec system / Strengths / Weaknesses / Directions for Ark / Sources). End-of-file "Directions for Ark" is the load-bearing section for synthesis under `99_directions/`.

## Required (this pass)

| # | Name | Type | Primary primitive | Workflow model | Integration surface | Closest-to-Ark dimension |
| - | ---- | ---- | ----------------- | -------------- | ------------------- | ------------------------ |
| 1 | [aider](aider.md) | OSS (Apache 2.0, 45.1k★) | Chat session + tree-sitter repo map + auto-commit per turn | One-shot edit-then-commit; architect/editor split optional | Terminal-only (Python) | Repo-map idea worth porting; deliberate counter-position on workflow ceremony |
| 2 | [cline](cline.md) | OSS (Apache 2.0, 62.1k★) | Task + Plan/Act modes + MCP Marketplace + Memory Bank | Plan → Act with checkpoints | VS Code primary; JetBrains/Zed/Cursor/Neovim/CLI | Plan/Act mirrors Ark's DESIGN/PLAN vs EXECUTE; Marketplace as template-distribution analog |
| 3 | [openhands](openhands.md) | OSS (MIT, 74.2k★) | Event stream of Actions/Observations + pluggable Runtime + Microagents | One-shot autonomous CodeActAgent; AgentDelegateAction sub-agents | SDK + CLI + GUI + Cloud (Docker/E2B/Modal/K8s) | Event-stream architecture; microagents as JIT context loading; AgentDelegateAction for subagent dispatch |
| 4 | [swe-agent](swe-agent.md) | Research (MIT, 19.3k★) | Agent-Computer Interface (bounded LM-centric commands) + Docker-per-task harness | One-shot eval-driven; Docker-isolated; lint-before-edit | Standalone benchmark + agent framework | "Design the interface, not the model"; bounded outputs; lint-as-edit-gate |
| 5 | [goose](goose.md) | OSS (Apache 2.0, 45.6k★, Block) | Recipes (YAML) + Extensions (MCP) + Subagents + Skills | Chat-driven; recipes capture procedures; `/plan` `/compact` `/recipe` | CLI + Electron desktop (Rust core); IDE bridges via MCP | Closest peer in *language* (Rust); recipe pattern; subagent isolation with own ExtensionManager |
| 6 | [codename-goose-or-similar](codename-goose-or-similar.md) — Plandex chosen, with Continue notes | OSS (MIT, 15.4k★ — Plandex) | Plans (named, server-side) + Cumulative diff sandbox + Tree-sitter project maps + 2M ctx | Plan-then-apply with cumulative review | Client-server CLI (Go+Python); self-hostable | Cumulative diff review ≈ Ark's VERIFY-as-gate; server-side state hints at future ArkOS direction |
| 7 | [claude-code-native](claude-code-native.md) | Commercial (Anthropic, closed binary) | Skills + Hooks + Subagents + MCP + Settings hierarchy + Plugins | Workflow-agnostic; user wires it (this is where Ark plugs in) | Terminal + VS Code + SDK + Plugin Marketplace | The integration target. Use hooks more aggressively; permission-per-phase enforcement; subagent definitions with effort/isolation/memory |
| 8 | [codex-cli](codex-cli.md) | OSS (Apache 2.0, 84.0k★, OpenAI) | AGENTS.md + Skills + Subagents (TOML) + MCP + OS sandbox + Worktrees | Workflow-agnostic; same primitive stance as Claude Code | CLI + VS Code + Desktop + Web + MCP-server self-exposure (Rust) | Closest peer in *language and primitives*. OS-level sandboxing; expose `ark agent` as MCP server is the obvious move |
| 9 | [devin-and-cognition](devin-and-cognition.md) | Commercial (Cognition, closed cloud) | Session + VM + Playbook + Knowledge Base + Machine Snapshot + ACU + Multi-Devin orchestration | Async manager-style: submit task → review PR | Cloud only (Web/Slack/Linear/Jira/GitHub/API) | Persistence-rich memory architecture; manager-style async UX as long-term direction; blockdiff is directly adoptable |

## Adjacent peers (pre-existing from prior research runs)

These files exist from earlier passes; reading them rounds out the IDE-class, web-class, and cloud-platform landscape.

| # | Name | Type | Primary primitive | Workflow model | Integration surface | Closest-to-Ark dimension |
| - | ---- | ---- | ----------------- | -------------- | ------------------- | ------------------------ |
| 10 | [cursor](cursor.md) | Commercial (Anysphere, closed) | Composer + Agent mode + Rules (.mdc) + Cloud agents per worktree | Chat → Composer → Agent (≤25 tool calls); cloud agents async | VS Code fork (desktop IDE) | Background-Agent-per-worktree is the precise pattern Ark's worktree feature targets |
| 11 | [zed](zed.md) | OSS-then-commercial editor with AI built-in | Threads + Assistant + Tools + Slash commands | Chat-driven; multi-buffer edits | Native editor (Rust) | Rust-native; multi-buffer abstraction worth studying for diff-review UX |
| 12 | [continue-dev](continue-dev.md) | OSS (Apache 2.0, 33.3k★) | Assistants (config.yaml) + Rules + CLI (`cn`) + Hub | Chat + CI-friendly headless `cn` agent | IDE extension + CLI + Hub catalog | CI-headless agent pattern (`cn --agent`); config.yaml format for portable assistant definitions |
| 13 | [roo-cline](roo-cline.md) | OSS Cline fork | Modes (architect/code/ask/debug) + Custom modes + MCP | Mode-scoped tool palettes | VS Code fork of Cline | Per-mode tool-restrictions = Ark per-phase permissions-profile direction |
| 14 | [copilot-workspace-and-agent](copilot-workspace-and-agent.md) | Commercial (Microsoft/GitHub) | Workspace plans + Issue-to-PR agent | Spec → plan → implementation → PR; review-mediated | Web (GitHub) + IDE | Issue-to-PR workflow as the next-step beyond Ark task lifecycle |
| 15 | [replit-agent](replit-agent.md) | Commercial (Replit) | Sandboxed VM + Agent v2 + Deploy buttons | Spec → app shell → iterate → deploy | Browser-only IDE | Vertically integrated agent + sandbox + hosting; counter-position lesson |
| 16 | [bolt-and-v0](bolt-and-v0.md) | Commercial (StackBlitz / Vercel) | Browser WebContainer + UI-first generative loop | Prompt → preview → refine | Browser-only | UI-first generation lessons; not directly applicable to Ark's CLI surface |
| 17 | [agent-platforms](agent-platforms.md) | Multi-vendor survey | Cross-cutting agent-platform features | n/a — survey | Various | Broader 'agent infra' picture; complements `02_infra_primitives/` |

## Quick navigation

- **By philosophy:**
  - Minimalist / git-native: aider
  - IDE-first / sidebar: cline, cursor, zed, roo-cline
  - Architecture-grade: openhands
  - Research / benchmark: swe-agent
  - General-purpose Rust agent: goose, codex-cli
  - Large-codebase specialist: plandex
  - Vertically integrated app builder: replit, bolt, v0
  - Closed but trend-setting: claude-code, codex-cli (OSS), devin (cloud), copilot

- **By language:** Rust (goose, codex-cli, zed), Python (aider, openhands, swe-agent), TypeScript (cline, claude-code, continue, roo-cline), Go+Python (plandex), closed/cloud (devin, cursor, copilot, replit, bolt, v0).

- **By multi-agent maturity** (low → high): aider, swe-agent, plandex, continue, zed → cline (`new_task`), roo-cline (modes) → claude-code (subagents), goose (subagents+TasksManager), codex-cli (TOML subagents + worktrees), cursor (background agents per worktree) → openhands (AgentDelegateAction), devin (multi-Devin orchestration).

- **By workflow opinionatedness** (none → strong): aider, claude-code, codex-cli, openhands, goose ≈ cline (plan/act), roo-cline (modes) → plandex (cumulative diff), zed → swe-agent (single-issue framing), copilot-workspace (spec→plan→impl), devin (PR-based async), replit (deploy-target-driven), bolt/v0 (preview-driven) → **Ark (tiered PRD/PLAN/REVIEW/VERIFY)**.

## Cross-cutting observations (from the nine required profiles)

1. **Skills (SKILL.md) is becoming a cross-platform standard.** Claude Code, Codex, Cursor, Goose, Gemini CLI all consume the same format from `~/.agents/skills/` or `.claude/skills/` etc. Ark should consider emitting SKILL.md siblings for promoted feature SPECs so every compatible agent picks them up automatically.
2. **Worktrees per parallel subagent is the convergent answer to concurrent agent execution.** Codex CLI ships this; Cursor's Background Agents ship this; Ark's worktree feature aligns. OpenHands uses Docker for the same purpose. Devin uses VMs.
3. **MCP-everywhere on the consumer side; less universally as a server-expose pattern.** Codex CLI exposes itself as an MCP server. Ark `ark agent` is a near-term candidate to do the same — this is probably the single biggest move available.
4. **No OSS peer ships Ark's tiered workflow (quick/standard/deep/research) or its REVIEW-loop / Response Matrix discipline.** This is Ark's clearest defensive differentiator. Copilot Workspace (commercial) is the closest in spirit but vertically integrated and closed.
5. **Persistence is fragmented across peers** — Memory Bank (Cline), microagents (OpenHands), AGENTS.md+Skills (Codex/Claude Code), Playbooks+KB (Devin), Recipes (Goose), Rules (Cursor/Roo). No peer has a story as comprehensive as Devin's, but Devin is closed and cloud-only. Open opportunity: Ark's workspace journal + project SPECs + feature SPECs + PRDs + (future) trajectory events is *already* the richest OSS persistence stack — needs surfacing.
6. **OS-level sandboxing is rare** — only Codex CLI on macOS/Linux at kernel level. Devin uses VMs. Everyone else relies on application-layer permission models. Worth a deeper look in `02_infra_primitives/`.
7. **Event-streams + condensers + sub-agent delegation as the three abstractions OpenHands gets right** — these compose. Ark's state-machine is the user-facing model but an event-log backing store (similar to OpenHands' append-only EventLog) would give us trajectories for free.
8. **Bounded outputs (SWE-agent's ACI lesson) is universally good** — every observation returned by `ark agent` should be length-capped with `--full` to override. Audit this.
9. **Lint-before-commit (SWE-agent) is the cleanest pre-edit safety pattern.** Ark's `task commit` could optionally invoke project lint/test before staging — opt-in via `.ark/config.toml`.
10. **Async / manager-style UX (Devin, Copilot Workspace) is a long-term direction for Ark.** Today Ark is interactive-chat-driven; the architectural ceiling is "submit task, dispatch agents, surface PRs asynchronously." Worth a design memo, not a near-term task.
