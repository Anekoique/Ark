# Zed

- Date: 2026-05-20
- Scope: external

## Identity

- **Name:** Zed
- **URL:** https://zed.dev — docs at https://zed.dev/docs
- **License:** Source-available under the GPLv3 (editor); Zed-developed components under Apache-2.0 / MIT for select crates. ACP (Agent Client Protocol) is Apache-2.0.
- **Maintainer:** Zed Industries, Inc. Founded by Atom / Tree-sitter / Electron alumni (Nathan Sobo, Max Brunsfeld, Antonio Scandurra).
- **Momentum (as of 2026-05):** Major AI repositioning over the past 12 months. Built-in **Agent Panel** is a first-class editor feature; **Parallel Agents** (multiple agents on different branches simultaneously) shipped in early 2026. Co-promoting the **Agent Client Protocol (ACP)** with JetBrains as the "LSP for agents." ACP Registry launched 2026-01.

## Positioning

Zed pitches itself as "the editor built for the AI era" — a from-scratch Rust editor whose performance budget (120 fps canvas, multi-threaded GPU rendering) survives co-resident agent activity. Against Cursor: Zed is open / Rust-native / multiplayer-from-day-one and now AI-native rather than AI-bolted-on.

The strategic bet that distinguishes Zed: **protocols, not products**. Where Cursor ships a vertically integrated stack, Zed pushes ACP as an open protocol so any agent works in any editor that speaks it. Zed becomes "the reference ACP client."

## Primitives

- **Agent Panel** — built-in chat / agent surface inside the editor. Multi-file edits stream into the canvas at 120fps.
- **Thread** — a conversation with one agent in the Agent Panel. Threads have history, checkpoints (for built-in agent), and a "follow" mode to jump to files the agent touches.
- **Profile** — a saved subset of tools the agent is allowed to use. Acts as a permission preset.
- **Agent** — either Zed's built-in agent or an external agent (Claude Code / Gemini CLI / Codex / GitHub Copilot) connected via ACP. Each thread can use a different agent.
- **ACP (Agent Client Protocol)** — JSON-RPC-over-stdio protocol between editor (client) and agent (server), modelled on LSP.
- **Parallel Agents** — multiple threads running in separate worktrees/branches at the same time; output streams back into the main editor view.
- **MCP servers** — registered via the Zed settings; the built-in agent can use them. External agents bring their own MCP stacks.
- **Slash commands** — `/` inside Agent Panel for built-in actions (`/file`, `/diagnostics`, `/symbols`, `/terminal`, etc.).
- **`@`-mentions** — add files, symbols, threads as context. `Selection` adds the current selected text from the buffer.

## Workflow model

Zed's workflow is **agent-and-canvas**: developer writes a request in the Agent Panel; agent reads files, generates a multi-file edit; edits appear *live in the editor*, not in a separate diff pane; user accepts or rejects per-file or per-hunk; "follow the agent" mode jumps the cursor as the agent reads.

Parallel Agents extends this to multiple concurrent threads — e.g. one agent refactoring backend, another updating frontend types, a third running tests on a separate branch. The editor shows aggregate progress.

There is no PRD / PLAN / VERIFY ceremony. Zed leaves workflow up to the user.

## Context & memory

- **`@`-mentions** add files, symbols, threads, and selections to a thread. The `+` menu in the message editor exposes the same set.
- **Thread history** — past conversations are listed and resumable. Restoration of external-agent threads is limited (Codex / Claude Code do not all support it).
- **Rules** — Zed supports a `.rules/` directory (analogous to Cursor's `.cursor/rules`) but the ecosystem is thinner.
- **AGENTS.md** — Zed reads this as a project-level convention file (cross-tool standard).
- **No first-class memory primitive.** Long-term memory is what the external agent (Claude Code session memory, Codex profiles) provides. Zed is the *client*, not the memory holder.
- **Vision** — image inputs supported for vision-capable models (Anthropic Claude 3+, GPT-4o, Gemini 1.5/2.0, Bedrock vision).

## Tool / capability surface

- **MCP** — Zed has direct MCP integration for the built-in agent. Configure via JSON in settings.
- **ACP** — external agents bring their own tool stacks via the protocol. The protocol passes file edits, tool calls, diagnostics, and follow events between agent and editor.
- **Sandbox** — none built into Zed. External agents may sandbox themselves (Claude Code's bash tool runs locally, Codex has its own sandbox). Parallel Agents use git worktrees for isolation; that is the closest Zed comes to sandboxing.
- **Permission model** — Profiles let you scope which tools an agent can use within a thread. The built-in agent honours per-tool approval prompts.

## Integration model

Zed's integration story has two layers:

1. **Built-in agent** — a Zed-developed agent using whichever model the user configures (Anthropic / OpenAI / Google / Bedrock / Ollama). This is the "default" experience.
2. **External agent via ACP** — Claude Code, Codex, Gemini CLI, GitHub Copilot, custom agents. Zed runs the agent as a subprocess and speaks ACP over stdio.

`@zed-industries/claude-code-acp` is an npm package that wraps the Claude Agent SDK and exposes it over ACP. This is the canonical example of "wrap a CLI agent in ACP, plug into Zed." OpenCode shipped ACP support in 2026 (per their docs).

The ACP Registry (launched 2026-01) is a directory of ACP-compatible agents, integrated into both Zed and JetBrains so developers can browse and install agents.

## Multi-agent / orchestration

- **Parallel Agents** — the explicit feature. Multiple threads on different branches/worktrees at the same time. Each thread can run a different agent (one thread on Claude Code, another on Codex, etc.).
- **Per-thread model selection** — you can choose model + agent per thread.
- **No subagent / dispatch model inside Zed itself.** Subagent support is whatever the external agent provides (Claude Code's subagents, Codex's profile system). Zed surfaces them but doesn't own them.

## Spec / artifact system

Zed has **no** spec-driven workflow. Like Cursor, all ceremony is on the user. The closest analogues are rules files and AGENTS.md.

## Strengths over Ark

1. **Native UX for agent activity.** Edits stream into the canvas, the cursor follows the agent's reads, diagnostics update live. Ark has no editor — it depends on whatever the host platform shows.
2. **ACP and the open-protocol bet.** Zed is reinventing the agent-editor boundary as an open protocol. Long-term this is a defensible position: any agent works in any editor. Ark has no protocol of its own; it ties itself to the slash-command surfaces of three host agents (Claude Code / Codex / OpenCode) and breaks if those evolve.
3. **Parallel Agents UX.** Multiple worktrees with live status in the editor. Ark's worktree feature is the same plumbing without the UI.
4. **Performance.** Rust-native, GPU-accelerated rendering. Multiplayer collaboration since launch. Cursor (and most IDEs) cannot match.
5. **First-class multiplayer / shared workspaces.** Two developers can be in the same Zed session debugging the same agent thread. Ark has no notion of shared sessions; the workflow is single-developer.

## Weaknesses / gaps

1. **Ecosystem is thin.** VS Code / Cursor have an order-of-magnitude more extensions, themes, and integrations. ACP Registry is growing but small as of 2026-05.
2. **External-agent feature parity is uneven.** Thread restoration, checkpoints, token-usage display etc. are not guaranteed for non-Zed agents. The "any agent, any editor" promise comes with caveats.
3. **No spec / workflow ceremony.** Same gap as Cursor.
4. **Macro adoption still building.** Zed is a viable Cursor alternative but Cursor leads on raw usage and ecosystem reach. Convincing a team to switch the editor itself is a heavier lift than switching agents.
5. **No memory primitive.** Memory is delegated to the agent. Long-term context retention across threads is whatever Claude Code / Codex provide.

## Directions for Ark

1. **Speak ACP.** Ark is currently a CLI + slash-command system. If ACP becomes the dominant editor-agent boundary, Ark's slash-command + `ark agent` CLI surface could be exposed as an ACP server — making the entire Ark workflow drive-able from Zed, JetBrains, Neovim, Emacs, and any future ACP client. The host-agent dispatcher (Claude Code / Codex / OpenCode) would still run, but Ark's structural mutations become editor-callable. The "LSP-for-agents" framing fits Ark's role as middleware between agent and project. Concrete shape: `ark serve --acp` exposes phase transitions and context queries; the editor's agent calls them rather than invoking the CLI.

2. **Per-thread model / per-tier model hint.** Zed lets you pick model per thread. Ark could let you pick model per tier / per phase in `task.toml` — an advisory hint the host agent respects ("Use Opus for REVIEW, Sonnet for EXECUTE"). This is independent of any platform's user-facing model picker.

3. **Live multi-task status surface.** Ark's `ark context` and `ark agent task worktree list` already enumerate active tasks. A "watch mode" (`ark watch`) that streams updates as each task progresses would mirror Zed's Parallel Agents view in a CLI-friendly way. Useful for the case where a developer dispatches three background tasks and wants to know which finished first.

4. **AGENTS.md as a primary entry-point doc.** Ark already writes `CLAUDE.md` (and equivalent for Codex / OpenCode). The convergence on AGENTS.md as a cross-tool standard suggests Ark should write / maintain that file directly and have it cross-reference the platform-specific files — making Ark projects portable to ACP-compatible editors out of the box.

5. **Image / vision support in research-tier corpus.** Zed's first-class image handling reminds us that "research" sometimes means reading a screenshot or diagram. The `research/` corpus today is text-only. If `ark research` accepted attachments and the host agent's vision capability was surfaced, the corpus could hold a diagram per file. (Lower priority; mostly a usability nicety.)

## Sources

- [Zed Docs — Agent Panel](https://zed.dev/docs/ai/agent-panel)
- [Zed Docs — External Agents](https://zed.dev/docs/ai/external-agents)
- [Zed Docs — Parallel Agents](https://zed.dev/docs/ai/parallel-agents)
- [Zed Docs — Agent Settings](https://zed.dev/docs/ai/agent-settings)
- [Zed — Agent Client Protocol](https://zed.dev/acp)
- [Zed Blog — Claude Code: Now in Beta in Zed (via ACP)](https://zed.dev/blog/claude-code-via-acp)
- [Zed Blog — ACP Brings JetBrains on Board](https://zed.dev/blog/jetbrains-on-acp)
- [Zed Blog — The ACP Registry is Live](https://zed.dev/blog/acp-registry)
- [JetBrains AI Blog — JetBrains × Zed: Open Interoperability](https://blog.jetbrains.com/ai/2025/10/jetbrains-zed-open-interoperability-for-ai-coding-agents-in-your-ide/)
- [JetBrains AI Blog — ACP Agent Registry is Live](https://blog.jetbrains.com/ai/2026/01/acp-agent-registry/)
- [OpenCode Docs — ACP Support](https://opencode.ai/docs/acp/)
- [Zed — The AI Code Editor Built for Speed](https://zed.dev/ai)
- [Builder.io — Is Zed ready for AI power users in 2026?](https://www.builder.io/blog/zed-ai-2026)
- [Markaicode — Setup MCP Servers in Zed (2026)](https://markaicode.com/mcp-zed-editor-setup/)
- [npm — @zed-industries/claude-code-acp](https://www.npmjs.com/package/@zed-industries/claude-code-acp)
