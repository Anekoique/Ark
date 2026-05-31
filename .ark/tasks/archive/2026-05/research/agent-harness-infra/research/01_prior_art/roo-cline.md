# Roo Code (formerly Roo-Cline)

- Date: 2026-05-20
- Scope: external

## Identity

- **Name:** Roo Code (renamed from Roo Cline in 2025; the fork's origin was the Cline VS Code extension)
- **URL:** https://docs.roocode.com — repo at https://github.com/RooCodeInc/Roo-Code
- **License:** Apache-2.0 (open source).
- **Maintainer:** Roo Veterinary, Inc. (RooVetGit / RooCodeInc). Active community via Discord.
- **Momentum (as of 2026-05):** The dominant Cline fork. Final official release per public references is v3.53.0 (2026-04-23). Free VS Code extension with optional Roo Code Cloud agents (GitHub / Slack / web triggers). Multiple downstream forks (hodlen, lupuletic, alarno) suggest a healthy hacker ecosystem.

## Positioning

Roo Code is the **"whole dev team in your editor" pitch**: a VS Code extension whose distinguishing feature is **multiple specialized agent modes** (Architect, Code, Ask, Debug, Test, custom) that hand off subtasks to each other. Against Cursor: Roo is open-source, multi-mode, multi-model. Against Cline: Roo adds Boomerang Tasks / Orchestrator mode and a marketplace.

The fork's identity is "Cline plus orchestration" — Cline gave us the autonomous coding agent; Roo gave it a manager.

## Primitives

- **Mode** — a configured persona with `slug`, `name`, `roleDefinition`, `customInstructions`, `groups` (tool group access), and optional file-regex restrictions. Each mode is a different prompt + tool scope.
- **Built-in modes** — `Code` (default), `Architect` (high-level design, no code/commands), `Ask` (Q&A), `Debug`, `Test`, `Orchestrator` (Boomerang).
- **Custom modes** — defined in `.roomodes` (YAML/JSON) at the project root or in user-global config.
- **Boomerang Tasks / Subtasks** — Orchestrator mode breaks a big task into subtasks; each runs in its own conversation context, in a tailored mode. Parent receives only the subtask's summary on completion.
- **Rules** — workspace-level instructions in `.roo/rules-{modeSlug}/` markdown files; mode-scoped.
- **Slash commands** — `/` to invoke saved workflows.
- **Marketplace** — in-extension marketplace for MCP servers (one-click install). Currently small relative to Cursor's MCP ecosystem; community PRs add servers.
- **`switch_mode` tool** — agents can request a mode switch as part of their reasoning.
- **Approval controls** — per-tool approve/deny prompts; auto-approve policies per group.
- **Checkpoint navigation** — improved in v3.53.0; visit/revert prior agent states.

## Workflow model

The Roo workflow centres on **mode-as-role**:

1. Pick a mode (or let Orchestrator pick).
2. Submit a task.
3. The agent in that mode reads code, writes code, runs commands within its `groups`.
4. If the task needs different expertise, the agent calls `switch_mode` or — in Orchestrator mode — `new_task` to spawn a subtask in a different mode.
5. Subtask completes; parent receives a summary; orchestrator decides next step.

This explicit role-switching is closer to a **multi-agent process model** than Cursor's "one agent with many tools." The mode definitions are persistent, repo-shippable, and tied to model selection (different models per mode).

The community has layered SPARC (Specification → Pseudocode → Architecture → Refinement → Completion) on top of Boomerang mode as a workflow methodology — but Roo itself ships the *primitives*, not the methodology.

## Context & memory

- **Per-subtask context** — each Boomerang subtask gets its own conversation history. Isolation by design.
- **Mode-scoped rules** — `.roo/rules-{modeSlug}/` directories let you author rules that apply only when in that mode (e.g. Test-mode rules describe TDD conventions, Architect-mode rules describe ADR templates).
- **Summary-only handoff** — orchestrator sees the subtask's *summary*, not its full conversation. Prevents context-window blow-up when delegating.
- **Checkpoints** — versioning agent state.

The summary-only handoff is the most interesting context-engineering move: it puts a forced compression layer at the orchestrator boundary, which means deep task trees don't bloat the parent's context.

## Tool / capability surface

- **MCP** — first-class via the marketplace and config.
- **Tool groups** — `read`, `edit`, `command`, `browser`, `mcp`. Modes are configured with subsets and can restrict file regex per group.
- **Custom tools via MCP** — anything an MCP server provides.
- **Approval policies** — per-tool, per-group, or fully auto.
- **`switch_mode` / `new_task`** — meta-tools that let agents change role or spawn subtasks.
- **Sandbox** — none; tools execute on host. Roo Code Cloud provides remote execution but the dominant pattern is local.

## Integration model

- **VS Code extension** — the primary surface; a sidebar panel.
- **Roo Code Cloud** — paid optional layer for longer-running / team-shared agents triggered from GitHub, Slack, or the web. Not required for local use.
- **Model-agnostic** — supports any provider (Anthropic, OpenAI, Google, Bedrock, Ollama, OpenRouter, custom). v3.53.0 added GPT-5.5 via OpenAI Codex and Claude Opus 4.7 via Vertex AI.

The extension is the entire product surface; there is no headless CLI equivalent for Roo. (The original Cline ecosystem has experimental CLIs — Roo has not invested as heavily.)

## Multi-agent / orchestration

This is **Roo's defining feature**. Boomerang Tasks (also called subtasks or Orchestrator mode):

- **Orchestrator** spawns subtasks via the `new_task` tool, specifying the subtask's mode and prompt.
- Parent task **pauses**; subtask runs in isolation.
- When subtask completes (via the `attempt_completion` tool with a summary), control returns to the parent, which sees only the summary.
- By default, the user must approve subtask creation and completion; this can be auto-approved per policy.

The design choice that distinguishes Roo: Orchestrator mode **cannot read files by default**. It only delegates. This forces the architecture: orchestrator is a router/planner, code-doing modes are workers. Stops the orchestrator from absorbing context it doesn't need.

Compared to Cursor's parallel agents (sibling agents in worktrees) or Claude Code's subagents (which fan out from a single main agent), Roo's model is a **call stack with summary-passing** — closer to a function-call abstraction.

## Spec / artifact system

- **No PRD / PLAN / SPEC system** built in.
- **SPARC overlay** — community methodology layered on top of Roo's primitives. Not a Roo feature, but documented in community guides.
- **Rules** — declarative conventions, mode-scoped.
- **Checkpoints** — historical agent states but not lifecycle artifacts.

## Strengths over Ark

1. **Mode-as-role primitive.** Roo's modes are *persistent personas with tool scopes and (optionally) different models*. Ark's tiers (`quick` / `standard` / `deep` / `research`) shape workflow ceremony but don't bind to model selection or tool scope. Roo's model maps more directly to "specialised agents" than Ark's does.
2. **Boomerang Tasks with summary-passing handoff.** A clean compositional primitive for multi-step work that Ark's subagent dispatch doesn't replicate. Ark's researcher/reviewer/verifier subagents return text; the parent re-reads files. Roo's compression-by-default at the orchestrator boundary is a context-engineering win.
3. **Per-mode rules (`.roo/rules-{modeSlug}/`).** Scoped instruction injection without frontmatter ceremony. Easier to reason about than Cursor's `globs` matching.
4. **Mode-switch is an agent-callable tool (`switch_mode`).** The agent decides when to change role. Ark's phase transitions are user-driven via slash commands; the agent can't say "I need to enter REVIEW now."
5. **Open source under Apache-2.0.** Same advantage as Continue.
6. **Marketplace UX.** One-click MCP server install. Ark has no install / discovery surface for MCP or any other extension.

## Weaknesses / gaps

1. **VS Code-locked.** The primary product is the extension; no headless / multi-IDE story.
2. **No formal lifecycle ceremony.** Boomerang is composition, not process. There is no PRD / VERIFY equivalent.
3. **Orchestrator quality is task- and prompt-dependent.** Without a workflow methodology imposed (SPARC, custom), orchestrator decomposition can be erratic.
4. **No git-aware worktree story.** Subtasks share the working copy; concurrency lives at the conversation layer, not the filesystem layer.
5. **Approval prompts pile up.** Roo's safer-by-default approval pattern is great in short bursts and tedious in long Boomerang chains. Auto-approve policies exist but require care.

## Directions for Ark

1. **Adopt mode-as-tool: agent-callable phase transitions.** Today Ark phase transitions are driven by user-typed slash commands (`/ark:plan`, `/ark:execute`). Roo lets the agent itself call `switch_mode`. Ark could expose `ark agent task transition --to <phase>` as a tool the host agent calls when it decides to advance. The legality table (per-tier transition rules) would still gate the call — but the agent can drive the lifecycle instead of asking the user to type the slash command. Concrete: bind the existing `ark agent task plan / review / execute / verify / commit` verbs as a single MCP-style tool the host agent can invoke.

2. **Per-mode / per-phase model hint.** Roo modes have model selection. Ark could surface `model_hint` per phase in `task.toml` (e.g. `[phases.review] model_hint = "opus"`) — advisory, host-respected. Ties to the Cursor-directions item; Roo confirms the pattern works in practice.

3. **Subagent dispatch with summary-only handoff.** Ark's `subagent-support` already dispatches researcher/reviewer/verifier subagents. Roo's stronger pattern: enforce summary-only return at the boundary. The subagent's full conversation is discarded; only its summary is appended to the parent's context. This requires defining a "summary contract" for each subagent type — what shape the parent expects. Worth codifying in the subagent SPEC.

4. **Mode-scoped rule files.** Ark already has project SPECs (always-on) and feature SPECs (PRD-listed). A third class — **phase-scoped** rules in `.ark/specs/project/phase-<name>/` that load only during a given phase (e.g. TDD conventions only loaded in EXECUTE) — would let project conventions be more precisely targeted. Lower priority but a natural extension.

5. **Orchestrator-as-research-tier.** Ark's research tier currently has a single agent gathering a corpus. A "research orchestrator" variant could dispatch subagents per top-level corpus section (each section runs in isolation, each returns a summary, the orchestrator weaves the synthesis). This is essentially Boomerang for research — and would scale `ark research` to bigger questions without overflowing one agent's context. The current task (this one!) is a natural test case.

## Sources

- [GitHub — RooCodeInc/Roo-Code](https://github.com/roovetgit/roo-code)
- [Roo Code Docs — Customizing Modes](https://docs.roocode.com/features/custom-modes)
- [Roo Code Docs — Using Modes](https://docs.roocode.com/basic-usage/using-modes)
- [Roo Code Docs — Boomerang Tasks](https://docs.roocode.com/features/boomerang-tasks)
- [Roo Code Docs — Slash Commands](https://docs.roocode.com/features/slash-commands)
- [Roo Code Docs — switch_mode](https://roocodeinc.github.io/Roo-Code/advanced-usage/available-tools/switch-mode/)
- [Roo Code Docs — Marketplace](https://docs.roocode.com/features/marketplace)
- [Roo Code Docs — Release Notes v3.3](https://docs.roocode.com/update-notes/v3.3)
- [Roo Code Docs — FAQ](https://docs.roocode.com/faq)
- [This Dot Labs — Roo Custom Modes](https://www.thisdot.co/blog/roo-custom-modes)
- [DataCamp — Roo Code: A Guide With 7 Practical Examples](https://www.datacamp.com/tutorial/roo-code)
- [HubPy — Roo Code Review 2026: The Multi-Mode AI Coding Agent](https://hubpy.io/blog/roo-code-guide-2026)
- [MorphLLM — Roo Code vs Cline (2026)](https://www.morphllm.com/comparisons/roo-code-vs-cline)
- [GitHub Issue — Aggregate subtask costs in Orchestrator/Boomerang](https://github.com/RooCodeInc/Roo-Code/issues/5376)
- [Medium — Boomerang Tasks = New AI-Powered Development](https://mychen76.medium.com/boomerang-tasks-make-ai-agent-powered-development-fun-again-522bf8962dc4)
- [Gist — SPARC + Boomerang Orchestration (community methodology)](https://gist.github.com/ruvnet/a206de8d484e710499398e4c39fa6299)
