# Cline

## Identity

- **Name:** Cline (formerly Claude Dev)
- **Repo:** https://github.com/cline/cline
- **License:** Apache 2.0
- **Primary maintainers:** Saoud Rizwan (founder) and the `cline` GitHub org; sponsored by Cline Inc. (commercial entity at cline.bot)
- **Language:** TypeScript (primary), with Rust bridges for some host integrations
- **Stars / momentum:** 62,073 (as of 2026-05-20, via `gh repo view`). Self-reports 5M+ installs across editor markets. Tag-line: "Autonomous coding agent as an SDK, IDE extension, or CLI assistant." Currently shipping at v3.81; very high velocity (multiple releases per week).
- **Homepage:** https://cline.bot

## Positioning

Cline is the canonical IDE-extension agent. It started as a VS Code sidebar in 2024 (named "Claude Dev"), evolved into a multi-editor product (VS Code, JetBrains, Cursor, Windsurf, Zed, Neovim) and grew a preview CLI in 2026. The defining design choice is the **plan/act split**: the user converses in Plan mode (read-only context gathering, no edits) until satisfied with the approach, then switches to Act mode (executes the plan, edits files, runs commands). It was one of the first agents to ship MCP support natively, and its MCP Marketplace is the closest thing in the ecosystem to a curated app store for agent tools.

## Primitives

User-facing nouns:

- **Task** — a single user goal (e.g. "add JWT auth"). One task = one conversation = one context window. Tasks persist as JSON in the workspace.
- **Plan / Act mode** — task-level mode switch. Plan = read-only reasoning; Act = mutating actions.
- **Memory Bank** — six markdown files Cline (re-)reads at every task start (`projectBrief.md`, `productContext.md`, `systemPatterns.md`, `techContext.md`, `activeContext.md`, `progress.md`). Compensates for hard context resets between sessions.
- **Rules** (`.clinerules/*.md` or root `.clinerules`) — markdown protocols Cline applies during execution.
- **Workflows** (`.clinerules/workflows/`) — composable sequences invokable as slash commands inside a task.
- **MCP servers** — external capability providers, browsable in the in-app Marketplace.
- **Checkpoints** — snapshots Cline takes during execution so the user can roll back to any prior tool call.
- **Approval/permission state** — auto-approve list per tool; persisted per workspace.

User-facing verbs:

- Mode toggles (Plan ↔ Act)
- Slash commands invoking workflows
- "New task" / "Resume task"
- Checkpoint restore
- "Approve" / "Reject" / "Auto-approve" per tool call
- MCP server install/uninstall (one-click from Marketplace)

## Workflow model

Representative flow:

1. **Open sidebar** in VS Code. Cline reads Memory Bank files (if present) before the first user turn.
2. **Type intent** in Plan mode: "Add JWT auth to the FastAPI app."
3. **Plan mode** — Cline traverses files via `read_file`/`list_files`/`search_files`, builds an internal plan, asks clarifying questions, drafts the approach. The user iterates until the plan is acceptable. **No file writes happen here.**
4. **Toggle to Act mode.** Cline executes the plan turn-by-turn: `write_to_file`, `execute_command`, `browser_action`, MCP tool calls. Each call shows a diff/preview and (unless auto-approved) requires a user click.
5. **Checkpoints** stamp every tool call. The user can scroll back and "restore to here" if the agent goes off the rails.
6. **Memory Bank update** — best practice is to ask Cline to update `activeContext.md` and `progress.md` at task close.
7. **Commit** — user commits manually (Cline doesn't auto-commit, though it can be asked to).

The plan→act split is the workflow innovation. There's no formal "verify" stage, but rules can mandate test-running in Act mode.

## Context & memory

**Context window management:**

- Per-task; resets each "new task".
- **Memory Bank** (markdown files) is the user-authored persistence layer. Cline re-reads all six files at task start. This is a deliberate workaround for amnesia.
- **Cline ingest** — at task start it reads VS Code workspace state (open files, selection) and uses `list_files` / `search_files` (ripgrep) to traverse the codebase on demand.
- No native tree-sitter repo map (unlike Aider/Plandex); relies on demand-driven file reads.
- **Context compression** — once the conversation approaches the model context limit, Cline summarizes older turns. The user can also explicitly request a "new task with context from this one."

**Persistent memory:**

- Memory Bank (project-level; checked into git).
- Per-workspace task list (JSON, `.cline/` or VS Code globalState).
- MCP servers can themselves provide memory (vector DBs, knowledge graphs).

## Tool / capability surface

**Built-in tools (Cline's default tool set):**

- `read_file`, `list_files`, `search_files` (ripgrep)
- `write_to_file`, `replace_in_file` (search-replace patch)
- `execute_command` (terminal — captures stdout, streams to chat)
- `browser_action` (Puppeteer — launch, click, type, screenshot — for testing web UIs)
- `use_mcp_tool` / `access_mcp_resource` (the MCP bridge)
- `ask_followup_question`, `attempt_completion`
- `new_task` (split a sub-task)
- `plan_mode_response` (Plan-mode-only structured response)

**MCP support:** First-class. Cline was the first major agent to ship MCP natively. The MCP Marketplace (cline.bot/mcp-marketplace; the `cline/mcp-marketplace` GitHub repo for submissions) is a curated catalog with one-click install. Cline auto-handles cloning, dependency setup, and configuration when you install a server.

**Plugin model:** Two paths:

- **MCP servers** — primary plugin surface; language-agnostic.
- **`.clinerules/`** — markdown rules; the lightweight "plugin without code" path.

**Sandbox boundaries:** None at the OS layer. Cline relies on a permission system: each tool call (except read-only) shows the user the action and waits for approval; the user can set per-tool/per-pattern auto-approve. Plan mode is the strictest sandbox (no mutating calls allowed regardless of approval state).

## Integration model

**Multi-target by design**, with a shared core:

- VS Code extension (primary)
- JetBrains (IntelliJ, PyCharm, etc.)
- Cursor, Windsurf, Zed plugins
- Neovim integration
- **CLI** (preview, macOS/Linux only as of 2026-Q2)
- SDK (TypeScript) for embedding

**Architecture:** Monorepo with a shared "core" that runs in the VS Code extension host process. The webview UI lives in a separate process and communicates over gRPC via protobuf — the "Host Bridge" layer lets the core call back into editor-specific APIs. The CLI reuses the core directly. This is unusually principled engineering for a YC-style agent product: it explains how Cline shipped JetBrains support so fast after VS Code.

## Multi-agent / orchestration

- **`new_task` tool** — Cline can spawn a subordinate task in its own context window. The subordinate runs to completion and returns a summary. This is the lightweight subagent path.
- **No declarative subagent definitions** like Claude Code's `.claude/agents/`.
- The plan/act split is technically a one-process two-mode design, not multi-agent.

## Spec / artifact system

- **Memory Bank** is the closest analog to Ark's PRD/PLAN/SPEC: a structured set of markdown files the user maintains. But it's project-wide, not task-scoped.
- **No formal PRD per task.** The user types intent into chat.
- **No formal review artifact.**
- **No SPEC promotion.** Conventions live in `.clinerules/` and Memory Bank.

## Strengths

- **Plan/Act mode separation is the cleanest UX in the space.** Strict read-only-in-plan is a real safety property, not aspirational.
- **MCP Marketplace.** No other agent has a comparable curated discovery surface for tools.
- **Multi-IDE reach via shared core.** The gRPC Host Bridge is the right architectural answer.
- **Checkpoints + restore.** Most CLI peers (Aider/Codex/Goose) rely on git for undo; Cline's checkpoint UX is finer-grained.
- **Open ecosystem.** `.clinerules/` and Memory Bank are markdown — portable across editors and easy to put under version control.
- **Velocity.** Releases multiple times per week; clearly the best-funded OSS agent.

## Weaknesses / gaps

- **Editor-bound.** The CLI is a 2026 afterthought; the canonical experience is the sidebar. Ark targets terminals first, IDEs via slash commands.
- **No tier ceremony.** Every task gets full plan-act treatment whether it's "rename a variable" or "rebuild auth subsystem." Memory Bank exists, but task-level planning detail is implicit.
- **No formal review loop.** The user is the reviewer. Ark's REVIEW phase (with severity grading + Response Matrix) is more structured.
- **No SPEC system.** `.clinerules/` is a flat rulebook; no concept of feature SPECs extracted from completed work.
- **No worktree-equivalent multi-task isolation.** One sidebar = one task at a time per VS Code window.
- **Memory Bank is brittle.** Six fixed-name files; no schema enforcement; agents are notorious for forgetting to update them.
- **Permission UI gets fatiguing.** Hence the auto-approve list; hence the loss of safety in practice.

## Directions for Ark

1. **Marketplace pattern for SPECs / templates.** Cline's MCP Marketplace shows that a curated discovery surface drives adoption. Ark's project SPECs and templates are currently bring-your-own. A `ark template marketplace` (or even a static `awesome-ark-templates` repo) would close this gap cheaply.
2. **Checkpoints during EXECUTE.** Cline checkpoints every tool call. Ark's atomic commit is good for closure but offers no mid-execute rollback. A checkpoint-per-`PathExt::write_file` (debug-flag gated, stored under `.ark/checkpoints/<task>/`) could give a similar restore-to-here UX without polluting git history.
3. **Plan-mode-style read-only enforcement during DESIGN.** Ark's DESIGN phase already implies "just write the PRD, don't code yet," but there's no machinery to *prevent* code edits. Adding a `--strict` flag that refuses any non-`.ark/` write during DESIGN/PLAN/REVIEW phases would harden the workflow.
4. **Memory Bank counter-position.** Ark already has per-project SPECs (the user-authored convention layer), feature SPECs (extracted from deep tasks), PRDs (per-task intent), and the workspace journal. We have *more* persistence than Memory Bank — but the slots are scattered. A `ark context --scope memory` projection that bundles "what I'd want at task start" into a single view is worth scoping.
5. **MCP integration.** Ark targets Claude Code / Codex / OpenCode but doesn't *itself* consume MCP. Worth evaluating whether `ark agent` should be exposable as an MCP server so other editors (Cursor, Zed, Cline itself) can drive it. That's the inverse of the current "Ark scaffolds slash commands for them" pattern.

## Sources

- [cline/cline on GitHub](https://github.com/cline/cline) — current repo (queried 2026-05-20)
- [Plan & Act — Cline docs](https://docs.cline.bot/features/plan-and-act)
- [Memory Bank System | DeepWiki](https://deepwiki.com/cline/prompts/3.1-memory-bank-system)
- [Architecture Overview | DeepWiki](https://deepwiki.com/cline/cline/1.3-architecture-overview) — extension host / gRPC / Host Bridge
- [MCP Server Management | DeepWiki](https://deepwiki.com/cline/cline/9.2-mcp-server-management)
- [Introducing the MCP Marketplace](https://cline.ghost.io/introducing-the-mcp-marketplace-clines-new-app-store/)
- [Rule System Architecture | DeepWiki](https://deepwiki.com/cline/prompts/2.1-writing-effective-rules)
- [Cline review — VibeCoder Blog](https://blog.vibecoder.me/cline-ai-pair-programming-vs-code) (2026)
