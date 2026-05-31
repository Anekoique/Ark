# Claude Code (Anthropic's native CLI)

## Identity

- **Name:** Claude Code
- **Source / docs:** https://code.claude.com/docs (https://github.com/anthropics/claude-code holds plugin SDK examples, hooks tooling, and limited public source)
- **License:** Proprietary (the CLI binary); plugin examples and SDK pieces under permissive licenses
- **Primary maintainer:** Anthropic
- **Language:** TypeScript (primary)
- **Momentum:** As of February 2026, ~4% of public GitHub commits (~135,000 per day) are authored by Claude Code — reported as a 42,896× growth in 13 months since its research-preview launch. The most-deployed coding-agent CLI by daily commit volume.

## Positioning

Claude Code is the **reference design** Ark targets first. Anthropic-built, terminal-first, with a layered customization stack (CLAUDE.md → slash commands → skills → subagents → hooks → MCP → plugins) that 2026 peers (Codex, Gemini CLI, Cursor) have mostly copied. Ark's integration target list (`templates/claude/commands/ark/*.md`) and slash-command-as-primary-UX choice is a direct consequence of Claude Code being the trend-setter on each surface.

For this research corpus Claude Code matters for two reasons:

1. **As a harness target** — Ark scaffolds `.claude/` files that Claude Code consumes. Ark must understand each surface to scaffold correctly.
2. **As a reference design** — Anthropic's choices set the bar for "good enough" primitives. Ark's own primitives (tiers, PRDs, REVIEW loops, SPEC promotion) sit *above* this stack; many of them could in principle be reified as Claude Code skills/agents instead of bespoke Ark concepts.

## Primitives

User-facing nouns (the "five core systems"):

- **Configuration hierarchy** — settings.json across user/project/managed scopes
- **Permissions** — allow/deny lists evaluated by Claude Code itself (model can't override)
- **Hooks** — deterministic event handlers (PreToolUse, PostToolUse, SessionStart, UserPromptSubmit, Stop, SubagentStop, …)
- **MCP** — open standard for external tools and data
- **Subagents** — specialized assistants with their own context, prompt, tools

Plus the customization layers:

- **CLAUDE.md** — project-level conventions loaded into context (per-project, plus `~/.claude/CLAUDE.md` for personal)
- **Slash commands** — markdown files in `.claude/commands/` invokable as `/<name>`
- **Skills** — `SKILL.md`-format folders that Claude auto-invokes based on task context (or by explicit `/skill-name`)
- **Plugins** — bundles of skills + commands + agents + hooks distributable as installable units

User-facing verbs:

- `/slash-command` invocations
- `claude` (CLI entry; opens chat REPL)
- `claude -p "<prompt>"` (non-interactive one-shot)
- `claude --agents <json>` (run with custom agent config)
- `/agents` to list/manage subagents inside a session
- `/memory` to inspect persistent memory
- `/compact` to compress conversation
- Hook lifecycle is *implicit* (event-driven)

## Workflow model

Claude Code is **workflow-agnostic by design** — no built-in PRD/PLAN/VERIFY phases. The workflow is whatever the user (or a tool like Ark) wires via slash commands + skills.

Representative flow without Ark:

1. **Run** `claude` in a project. SessionStart hook fires; CLAUDE.md loads.
2. **Chat** to describe intent. Claude reads files, runs tools, writes edits.
3. **Permissions** prompt the user (or auto-allow per settings); PreToolUse hooks may block dangerous operations.
4. **Skills auto-invoke** when the task description matches a SKILL.md description. Subagents can be spawned via the Task tool or `/agents`.
5. **Stop hook** fires when the agent decides it's done.
6. **User commits** manually.

Representative flow with Ark wired in:

1. User runs `/ark:design "<title>"` (an Ark-shipped slash command living in `.claude/commands/ark/design.md`).
2. The command instructs Claude to call `ark agent task new --slug ...`, then read `ark context`, then fill PRD.
3. Phase transitions happen via `/ark:plan`, `/ark:review`, `/ark:execute`, `/ark:verify`, `/ark:commit` — each is a slash command that calls into `ark agent`.
4. Ark's `SessionStart` hook (installed in `.claude/settings.json` by `ark init`) injects context every session.

## Context & memory

**Context window management:**

- CLAUDE.md auto-loads at session start; sub-CLAUDE.md files in subdirs load when those dirs are referenced.
- Subagents have isolated contexts (the whole point); they return only summaries to the parent.
- `/compact` summarizes when full.
- Skills are loaded *on demand* — only the SKILL.md description (frontmatter) is always-on; bodies load when triggered.

**Persistent memory:**

- Per-project `~/.claude/projects/<project-path>/memory/MEMORY.md` (first 200 lines auto-loaded each session).
- CLAUDE.md (checked into git).
- `.claude/settings.json` (project + user + managed scopes).
- User can add memory via `/memory` slash command.

This is more memory infrastructure than any peer except Cline.

## Tool / capability surface

**Built-in tools:**

- Read, Edit, Write, Glob, Grep, Bash
- WebFetch, WebSearch
- Task (spawn subagent), TodoWrite (todo list management)
- NotebookEdit (Jupyter)
- BashOutput, KillBash (background bash management)
- SlashCommand (invoke a slash command programmatically)

**MCP support:** First-class consumer of MCP servers. `.mcp.json` declares servers; `mcp__<server>__<tool>` namespace surfaces them.

**Plugin model:** Mature.

- **`.claude/skills/`** — skills (one folder per skill, with SKILL.md + optional scripts)
- **`.claude/commands/`** — slash commands (markdown)
- **`.claude/agents/`** — subagents (markdown with YAML frontmatter)
- **`.claude/hooks/`** — hook scripts (Bash or other)
- **`.claude/settings.json`** — permissions, hook bindings, MCP servers, plugin enables
- **`.mcp.json`** — MCP server declarations
- **Plugins** — distributable bundles (a `.claude-plugin/plugin.json` manifest + the components at plugin root)

The 2026 unification: slash commands and skills converged — every skill gets a `/<name>` interface automatically. Slash commands are still supported but the recommended path is skills.

**Sandbox boundaries:**

- Permission model (allow/deny lists) is the primary enforcement.
- Subagents get their own allowed/disallowed tool lists per agent definition.
- No OS-level sandboxing (unlike Codex CLI's Seatbelt/Landlock).
- Hooks (PreToolUse) provide deterministic blocking — `exit 2` from a PreToolUse hook denies the tool call.

## Integration model

**Terminal-first CLI**, with:

- **VS Code extension** ("Claude Code in VS Code") — same backend, GUI wrapper.
- **SDK** (`@anthropic-ai/claude-code-sdk`, plus an Elixir port `claude_agent_sdk` and others) for embedding.
- **Plugin Marketplace** — distributable plugins discoverable via the plugin browser.

## Multi-agent / orchestration

**Subagents are first-class.**

- Defined in `.claude/agents/` as markdown + frontmatter.
- Each has: name, description, prompt, tools, disallowedTools, model, permissionMode, mcpServers, hooks, maxTurns, skills, initialPrompt, memory, effort, background, isolation, color.
- Parent agent spawns via `Task` tool or `--agents` flag.
- **Background subagents** can run asynchronously (return a handle; parent polls or awaits).
- **SubagentStart / SubagentStop hooks** track lifecycle.
- The 2026 doc describes "agent teams" — multi-agent patterns coordinated by a parent agent.

## Spec / artifact system

**Skills are the artifact system at Anthropic's layer.**

- One folder per skill, with SKILL.md (frontmatter: description, allowed-tools, optional examples).
- Skills are *recipes-as-context* — when the description matches the task, the body loads.
- Skills can ship helper scripts (Python, Bash) the agent invokes.
- Skills can be distributed via plugins.

**No PRD/PLAN/VERIFY built in.** Ark provides those *on top* via slash commands.

## Strengths

- **Best primitives in the field.** Skills, hooks, subagents, MCP, settings hierarchy, plugins — each is well-designed and they compose.
- **Hook system is uniquely deterministic.** PreToolUse can block tool calls; PostToolUse can mutate state; SessionStart can inject context. No peer matches the cleanness.
- **Permission hierarchy with admin/managed override.** Enterprise-ready.
- **Subagent definitions are rich** — per-agent model, tools, mcp servers, memory, effort budget.
- **Plugin distribution** with a manifest and Marketplace is mature.
- **Memory.** Per-project memory file with auto-load is real persistence.

## Weaknesses / gaps (relative to Ark's value-add territory)

- **No opinion on workflow.** Workflow is whatever the user wires via skills + commands. Ark fills this gap.
- **No tier ceremony.** Skills are all-or-nothing in scope; no "this is small, skip ceremony" affordance.
- **No formal PRD/PLAN/REVIEW/SPEC artifacts.** Memory and CLAUDE.md exist; structured per-task workflow artifacts don't.
- **No git lifecycle integration** beyond what skills can write.
- **No multi-task isolation per checkout.** Sessions share workspace state.
- **No journaling abstraction** — memory exists but doesn't auto-stamp who/when/what.
- **Closed source.** The CLI binary itself isn't OSS; behavior changes between releases without public diff.

## Directions for Ark

1. **Push Ark concepts as Claude Code skills.** Ark's slash commands (`/ark:design`, etc.) are already skills-in-disguise. Consider whether they should be packaged as a proper Claude Code Plugin (`.claude-plugin/plugin.json`) for distribution outside of `ark init`. Users could `claude plugin install ark` to get the slash commands without ever running `ark init`. (Risks: the plugin would have to know nothing about the Rust CLI; degrades the experience.)
2. **Use hooks more aggressively.** Ark already installs a `SessionStart` hook. Other lifecycle points worth wiring:
   - **PreToolUse** for `Edit`/`Write` during DESIGN/PLAN/REVIEW phases — refuse writes outside `.ark/tasks/<slug>/`.
   - **Stop** — confirm phase status; warn if the user is leaving a task mid-iteration.
   - **SubagentStop** — record the result of `ark-researcher`/`ark-reviewer`/`ark-verifier` runs into `.ark/tasks/<slug>/agent_log.jsonl`.
3. **Permission scoping per phase.** Use Claude Code's permission system to enforce phase invariants. Each Ark phase ships with a `permissions` profile in its slash command that constrains tool access (DESIGN: no `Bash`, no `Write` outside `.ark/`; EXECUTE: full; VERIFY: read-only except for `VERIFY.md`).
4. **Subagent definitions for Ark roles.** Ark already ships researcher/reviewer/verifier under `.claude/agents/ark-*.md`. Audit whether their frontmatter uses the *full* surface (model, effort, isolation, memory) or leaves it sparse. Specifically:
   - `effort: high` for `ark-reviewer` (deep reasoning).
   - `isolation: true` for `ark-researcher` (don't pollute parent context with web search noise).
   - `memory: <task-scoped-path>` if we want subagent recall across iterations.
5. **Track Claude Code's Skill standard as a portable export format.** Codex, Gemini CLI, Cursor, and Goose all consume SKILL.md. If Ark's feature SPECs (currently `specs/features/<...>/SPEC.md`) emitted a sibling `SKILL.md` (with the SPEC body as the skill description), every Ark project would automatically be discoverable by *every* compatible agent. Promotion on deep commit could include this synthesis.

## Sources

- [Claude Code Overview — official docs](https://code.claude.com/docs/en/overview)
- [Hooks reference — Claude Code docs](https://code.claude.com/docs/en/hooks)
- [Configure permissions — Claude Code docs](https://code.claude.com/docs/en/permissions)
- [Create custom subagents — Claude Code docs](https://code.claude.com/docs/en/sub-agents)
- [Plugins reference — Claude Code docs](https://code.claude.com/docs/en/plugins-reference)
- [Claude Code settings.json reference (community)](https://gist.github.com/mculp/c082bd1e5a439410158974de90c89db7) (Apr 2026)
- [anthropics/claude-code GitHub](https://github.com/anthropics/claude-code) — plugin dev examples
- [Claude Code Customization Guide — alexop.dev](https://alexop.dev/posts/claude-code-customization-guide-claudemd-skills-subagents/)
- [Claude Code: Hooks, Subagents, Skills — ofox.ai](https://ofox.ai/blog/claude-code-hooks-subagents-skills-complete-guide-2026/)
- [Claude Code Hooks: 12 Lifecycle Events — claudefa.st](https://claudefa.st/blog/tools/hooks/hooks-guide)
- [Claude Code Enterprise Rollout Playbook — systemprompt.io](https://systemprompt.io/guides/claude-code-organisation-rollout)
- [Claude Code Advanced Patterns PDF — Anthropic](https://resources.anthropic.com/hubfs/Claude%20Code%20Advanced%20Patterns_%20Subagents,%20MCP,%20and%20Scaling%20to%20Real%20Codebases.pdf)
