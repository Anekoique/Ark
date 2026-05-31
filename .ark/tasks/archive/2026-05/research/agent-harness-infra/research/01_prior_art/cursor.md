# Cursor

- Date: 2026-05-20
- Scope: external

## Identity

- **Name:** Cursor (formerly "Anysphere"-branded code editor)
- **URL:** https://cursor.com — docs at https://cursor.com/docs
- **License:** Proprietary, closed-source. Free tier + paid Pro / Business / Enterprise. Built on a VS Code fork.
- **Maintainer:** Anysphere, Inc. (San Francisco, US). Funded; high-9-figure ARR by mid-2025.
- **Momentum (as of 2026-05):** Dominant IDE-native incumbent. Cursor 2.0 shipped 2025-10-29 with an agent-first redesign; Composer 2.5 (in-house coding model) released 2026-05-18. The category-leader for "AI editor", quoted in nearly every comparison piece. Roughly 5,000+ community-built MCP servers in the ecosystem as of 2026-03 (per truefoundry / NxCode pieces).

## Positioning

Cursor positions as the **default IDE for AI-first coding**. It is not a CLI, not a plugin, not a sandbox — it is a complete editor where the AI is a first-class collaborator rather than an autocomplete bolt-on. The market frame: "Cursor is to VS Code what VS Code was to Atom" — a fork that re-prioritised one capability (AI-driven edits) until it became a different product.

Two strategic moves shape it: (1) tightly couple model + editor + sandbox/worktree to deliver a UX no plugin can match; (2) keep the surface accessible enough that any developer can sit down and write a prompt — the rules system, MCP integration, and Skills are layered on top of an experience that works out of the box with zero config.

## Primitives

User-facing nouns and verbs:

- **Chat** — the basic conversational sidebar.
- **Composer** — the AI coding assistant pane; multi-file edits, plans, diffs.
- **Agent (mode)** — the autonomous mode inside Composer, invoked with `⌘.`. Up to 25 tool calls before stopping.
- **Background Agent / Cloud Agent** — agents that run asynchronously in isolated cloud environments off the developer's machine. Up to 8 in parallel, each in its own git worktree.
- **Rules** — `.cursor/rules/*.mdc` files with frontmatter (`description`, `globs`, `alwaysApply`). Conditional, file-scoped loading. Project-level and user-level.
- **Skills** — `SKILL.md` files (shipped 2.4) that bundle domain knowledge + custom commands + hooks for procedural tasks.
- **Commands** — Markdown files under `.cursor/commands/` invoked with `/`. Reusable workflows checked into git.
- **Hooks** — scripts that run before / after agent actions (introduced with Skills).
- **MCP servers** — registered in `.cursor/mcp.json`; one-click install from the Settings UI.
- **Modes** — Code / Ask / Custom — each can have its own model and tool scope.
- **Checkpoints** — automatic per-generation snapshots so you can revert any AI edit.
- **`@`-mentions** — context selectors: `@File`, `@Folder`, `@Code`, `@Docs`, `@Web`, `@Recommended` (auto-pull).
- **`#`-mentions** — explicit file focus inside the prompt.
- **`AGENTS.md`** — recently added: read at the project root as a cross-IDE fallback, recommended replacement for `.cursorrules`.

## Workflow model

Cursor does **not** impose a workflow. The dominant pattern is interactive:

1. User opens Composer (`⌘I`) or invokes Agent (`⌘.`).
2. Types prompt; agent decides which files to read, edits, runs tools, asks clarifying questions.
3. Checkpoints are dropped on every generation; user inspects diff and accepts/rejects.
4. Up to 25 tool calls per turn; the agent stops and asks if it needs more.

For longer or more autonomous work, the **Background Agent / Cloud Agent** pattern flips the model:

1. User assigns a task (often from a GitHub issue, Linear ticket, or Slack message).
2. Agent runs off-machine in an isolated cloud worktree.
3. Result returns as a PR draft. Developer reviews, merges or rejects.
4. Up to 8 in parallel; each agent has its own clone of the repo.

There is no explicit PRD / PLAN / VERIFY ceremony. The closest analogue is **Skills**, which bundle "how-to" procedures, and **Commands**, which encode reusable workflows. The user supplies discipline; Cursor supplies primitives.

## Context & memory

- **Conversational memory** lives in the chat thread. New threads start fresh.
- **`@Recommended`** auto-selects relevant context. The model decides.
- **Rules** are the persistent memory layer: project rules (`.cursor/rules/*.mdc`) and user rules (`~/.cursor/rules/`). MDC frontmatter:
  - `alwaysApply: true` — included in every chat in the project.
  - `alwaysApply: false` + `description: "..."` — agent decides whether to load it ("Apply Intelligently").
  - `globs: "app/routers/**/*.py"` — fires only when matching files are in context.
- **Checkpoints** persist within a Composer session so a user can roll back to any prior generation.
- **Skills** can include "domain knowledge" markdown that the agent pulls in on demand — closer to RAG-style on-demand context than always-on rules.

The rules-vs-skills split is deliberate: rules are *declarative* always-on context; skills are *procedural* opt-in instructions. This is one of the cleaner mental models in the field.

## Tool / capability surface

- **MCP integration** — Cursor was among the first IDEs to ship MCP support. Configuration in `.cursor/mcp.json`. The Settings UI shows every loaded server with a status indicator and supports one-click install from a registry. The well-known tool cap is **40 tools** across all servers — exceed it and Cursor stops registering new ones, which has become a recurring pain point as the MCP ecosystem grew past 5,000 servers.
- **Built-in tools** — file reading/editing, terminal execution, web search, web browsing, image generation (added 2.4).
- **Sandbox** — Background / Cloud Agents run in remote isolated environments. Local agent mode runs against the user's machine with explicit approval for terminal commands.
- **Hooks** — script-level intercepts before/after agent actions, shipped with Skills.
- **Commands** — markdown files that act like macros; invoked with `/` inside Composer.

## Integration model

Cursor is a closed editor. There is no public plugin API for agents to extend Cursor itself. Instead:

- **MCP** is the extension surface: any MCP server, anywhere, is a tool the Cursor agent can call.
- **Rules / Skills / Commands** are checked into the repo so a team standardises behaviour by committing markdown, not by writing extensions.
- **External CLIs** are invoked through the terminal tool — the agent shells out to `pytest`, `git`, `npm`, etc. just as a developer would.

The product is the **editor + AI**. There is no "Cursor as a library" or "Cursor as a daemon" or "headless Cursor" — though Background Agents move some of this off-machine and Cursor CLI (still beta in mid-2026) exists for terminal interactions.

## Multi-agent / orchestration

Cursor 2.0 (2025-10-29) shipped explicit multi-agent UX:

- **Parallel Agents** — up to 8 background agents simultaneously. Each gets its own git worktree.
- **Subagents** (2.4) — agents can spawn subagents for narrower tasks. The handoff model is similar to Roo Code's Boomerang Tasks but less explicit; the parent agent decides when to delegate.
- **Custom Modes** — separate "personas" with their own tool scopes and (optionally) models. An "Architect" mode might use a stronger model and read-only tools; a "Tester" mode might be allowed to run shell commands.

Cursor markets this as "you act as team lead, reviewing PRs from your agent team."

## Spec / artifact system

Cursor has no spec-driven workflow. The closest equivalents:

- **Skills** bundle procedural knowledge in `SKILL.md` files. Skills can declare commands, hooks, and instructions for specialised tasks. This is closer to "agent capability packs" than to a PRD/PLAN system.
- **Rules** encode standing conventions. They are not artifacts of a task — they apply across tasks.
- **Commands** encode reusable workflows but are not tied to lifecycle.
- **PRs** are the final artifact for Background Agents — Cursor does not generate a separate plan / spec / verification document.

There is **no analogue** of Ark's `PRD.md` / `NN_PLAN.md` / `NN_REVIEW.md` / `VERIFY.md`. Cursor leaves workflow ceremony to the user.

## Strengths over Ark

1. **Zero-config onboarding.** A developer opens Cursor and starts coding with AI. Ark requires a workflow installation, a slash command vocabulary, and reading `workflow.md`. Cursor sells convenience; Ark sells discipline.
2. **Editor-native UX.** Diff review, inline checkpoints, syntax-highlighted multi-file edits, `@`-mentions for context, `#` for file focus. Ark depends on the host platform (Claude Code / Codex) for any of this; it has no UX of its own.
3. **MCP ecosystem.** A 5,000+ server ecosystem with one-click install. Ark integrates with MCP only through the host agent (Claude Code's MCP support, etc.); it doesn't own the registry / install / discovery surface.
4. **Background agents and worktree isolation as a first-class product.** Cursor's 8-parallel-worktree model is what Ark's `worktree` feature aspires to be, but Cursor wraps it in a GUI: launch from a GitHub issue, watch progress in a sidebar, merge from the dashboard. Ark's worktree is a git operation hidden behind a CLI flag.
5. **Skills as procedural knowledge packs.** Ark's project SPECs and feature SPECs are declarative conventions. Skills are explicitly *procedural* — "when doing X, follow this script and these hooks." Ark has no procedural-knowledge primitive.
6. **A proprietary coding model (Composer / Composer 2.5).** Cursor controls the entire stack — model, editor, agent loop. Ark is intentionally platform-agnostic, but Cursor's vertical integration delivers latency and cost benefits Ark cannot match.

## Weaknesses / gaps

1. **Closed and proprietary.** Source is not auditable. Self-hosting impossible. Enterprise lock-in concerns are real.
2. **No explicit workflow ceremony.** Specification, planning, and review are entirely on the user. Cursor has tools (Skills, Rules) but no opinionated PRD → PLAN → EXECUTE → VERIFY pipeline. For teams that want process, this is a gap (and Ark's positioning).
3. **40-tool MCP cap.** Hard limit causes friction once a team integrates more than a handful of services. Workarounds (server grouping, hub MCP servers) are emerging but the cap remains a known constraint.
4. **Editor-locked.** All value lives inside Cursor's editor. A developer working in Vim, IntelliJ, or terminal cannot reuse the Rules / Skills / agent setup — they only ship as a `.cursor/` directory. AGENTS.md adoption is the partial response.
5. **No spec-extraction / artifact-promotion model.** A team can write a "feature design" inside a Cursor chat but Cursor will not promote it to a long-lived spec. Cursor sees rules as the long-lived layer and conversations as ephemeral.
6. **Tied to one IDE per developer.** A team standardising on Cursor must standardise on Cursor for everyone — there is no way to share the AI behaviour with a colleague using a different editor (beyond the markdown rules being readable elsewhere).
7. **Checkpoints are session-local.** They are not branches; they live only in the Composer session. A reload or thread switch loses them.

## Directions for Ark

1. **Layered context: declarative rules vs procedural skills.** Cursor's split — `.cursor/rules/` (always-on conventions) vs `SKILL.md` (procedural how-tos invoked on demand) — is conceptually distinct from Ark's project-SPEC / feature-SPEC split, which is both always-on. Consider adding a third class: **procedural recipes** the agent loads only when a relevant verb / domain matches. Maps naturally onto slash commands but framed as agent-loadable knowledge.

2. **Conditional rule loading with frontmatter.** Today Ark project SPECs are *all read* before any task and feature SPECs are *read only if the PRD lists them*. A frontmatter-driven middle ground (`alwaysApply: false` + `globs: "**/*.rs"`) would scope SPECs to files / verbs / phases without an explicit PRD listing. The MDC pattern is well-understood by users coming from Cursor.

3. **Background-agent dispatch backed by worktrees.** Ark already has worktrees and `task new --worktree`. Cursor's UX is: hand off a fully-formed task, let an agent run it off-host, come back to a PR. The CLI building blocks are there (`ark agent task new --worktree`, `ark agent task commit`); a one-shot `ark dispatch <issue-url>` that creates the task, worktree, and runs the host agent unattended would close the loop. This is the next obvious step after `subagent-support`.

4. **Tool-cap budget surfaced in `ark context`.** As MCP usage grows in Claude Code / Codex / OpenCode the same 40-tool problem will appear. `ark context` could project the active MCP tool count and warn at thresholds — turning a vague capability constraint into a CLI-observable signal.

5. **Per-mode model selection.** Cursor and Roo Code both let modes pick their own model (cheap for code edits, expensive for architecture). Ark's tier model (`quick`/`standard`/`deep`/`research`) is a similar axis, but Ark does not surface a model-selection knob — the host agent picks. A `task.toml.model_hint` (advisory, host-respected) would let users say "deep tier uses Opus, quick uses Haiku" without coupling Ark to any specific platform.

## Sources

- [Cursor Docs — Rules](https://cursor.com/docs/context/rules)
- [Cursor Docs — Agent Overview](https://cursor.com/docs/agent/overview)
- [Cursor Docs — Composer Overview](https://docs.cursor.com/composer/overview)
- [Cursor Docs — MCP](https://cursor.com/docs/mcp)
- [Cursor Changelog — Subagents, Skills, Image Generation (2.4)](https://cursor.com/changelog/2-4)
- [Cursor Docs — Models & Pricing (Composer 2.5)](https://cursor.com/docs/models-and-pricing)
- [Cursor Blog — Best Practices for Coding with Agents](https://cursor.com/blog/agent-best-practices)
- [Cursor Blog — Background Agents / Automations](https://cursor.com/blog/automations)
- [DeployHQ — Cursor 2026: Composer, Agent Mode, MCP & Background Agent](https://www.deployhq.com/guides/cursor)
- [Digitalapplied — Cursor 2.0 Agent-First Architecture Guide](https://www.digitalapplied.com/blog/cursor-2-0-agent-first-architecture-guide)
- [Awesome Cursor Rules MDC — Reference](https://github.com/sanjeed5/awesome-cursor-rules-mdc/blob/main/cursor-rules-reference.md)
- [Augment Code — How to Build Your AGENTS.md (2026)](https://www.augmentcode.com/guides/how-to-build-agents-md)
- [TrueFoundry — MCP Servers in Cursor (2026 Guide)](https://www.truefoundry.com/blog/mcp-servers-in-cursor-setup-configuration-and-security-guide)
