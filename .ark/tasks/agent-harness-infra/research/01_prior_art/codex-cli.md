# OpenAI Codex CLI

## Identity

- **Name:** Codex CLI (often "OpenAI Codex CLI"; not to be confused with the deprecated 2021 Codex code-completion model)
- **Repo:** https://github.com/openai/codex
- **License:** Apache 2.0
- **Primary maintainer:** OpenAI
- **Language:** Rust (~95% of the codebase as of 2026; started TypeScript-first in mid-2025 and was rewritten)
- **Stars / momentum:** 84,030 stars (as of 2026-05-20, queried via `gh repo view`). The most-starred OSS coding agent. ~4 million weekly active users per Sam Altman, April 2026. Launched April 16, 2025.
- **Homepage:** https://developers.openai.com/codex

## Positioning

Codex CLI is the **closest peer to Ark in *language*** (Rust) and the **closest peer to Claude Code in *primitives***. OpenAI shipped it as a direct response to Claude Code's mid-2025 takeover, and the design choices show that lineage: AGENTS.md (their CLAUDE.md), skills (the cross-platform standard), subagents (TOML-defined, parallel-spawnable), MCP integration, and a CLI/Desktop/IDE unified config. The differentiating choice is **OS-level sandboxing** — Codex CLI uses Apple Seatbelt on macOS and Landlock/seccomp on Linux to enforce filesystem and network constraints at the kernel level, not via application-layer hooks.

For Ark, Codex CLI matters in three ways:

1. **As a Rust-architecture reference** (codex-rs Cargo workspace with ~70 crates).
2. **As an integration target** (Ark's `codex-support` SPEC).
3. **As the most credible competitor on "Rust-native, model-flexible, terminal-first agent harness"** positioning.

## Primitives

User-facing nouns:

- **Session** — the running codex process; named, resumable.
- **AGENTS.md** — project conventions file (and `~/.codex/AGENTS.md` for global).
- **Skill** — `SKILL.md`-folder; cross-platform with Claude Code / Cursor / Gemini CLI.
- **Subagent** — TOML-defined custom agent in `.codex/agents/` or `~/.codex/agents/`.
- **MCP servers** — declared in `~/.codex/config.toml`.
- **Sandbox mode** — `read-only` / `workspace-write` / `danger-full-access`; with optional OS-level kernel enforcement (Seatbelt/Landlock).
- **Plugin** — distributable bundle (AGENTS.md + skills + MCP config + subagents).

User-facing verbs:

- `codex` (interactive)
- `codex run "<prompt>"` (one-shot)
- `codex resume <session>`
- `codex mcp` — Codex itself can run *as* an MCP server, exposing `codex()` / `codex-reply()` tools to other MCP clients
- Slash commands inside session
- `codex skill` management

## Workflow model

Representative flow:

1. **Configure once.** `~/.codex/config.toml` holds provider, sandbox mode, MCP servers, skill enables.
2. **Per-project**, drop `AGENTS.md` at the root (or `.codex/AGENTS.md`). Codex reads it in this order: `AGENTS.override.md`, `AGENTS.md`, `TEAM_GUIDE.md`, `.agents.md`.
3. **Run** `codex` in the repo. Session starts; AGENTS.md + skill descriptions load.
4. **Chat / tell** Codex what to do. It reads, edits, runs commands inside the configured sandbox.
5. **Skills auto-invoke** when description matches. The model can call subagents (defined as TOML files) for parallel work.
6. **OS sandbox enforces** — even if the model decides to do something disallowed, Landlock/Seatbelt blocks the syscall.
7. **Commit** manually.

No PRD/PLAN/REVIEW/VERIFY ceremony. AGENTS.md and skills do the work that CLAUDE.md and Claude Code skills do — same conceptual layer.

## Context & memory

**Context window:**

- Default 272K tokens with GPT-5.4 (configurable up to 1M).
- AGENTS.md merging is hierarchical (project + global + override).
- Skills are lazy-loaded (description always-on; body on match).
- Subagents have isolated contexts (per-TOML config).

**Persistent memory:**

- Session JSON files in `~/.codex/sessions/`, resumable.
- No first-class "memory" feature like Claude Code's `MEMORY.md` (as of 2026-Q2 — skills are recommended for cross-session knowledge).
- Skills can include `references/` and `scripts/` directories that effectively act as durable memory.

## Tool / capability surface

**Built-in tools:**

- Read, Edit, Write, Bash, Glob, Grep
- Network access (configurable by sandbox)
- MCP tool invocation
- Subagent invocation
- Image input

**MCP support:** First-class. Both as a *client* (consumes MCP servers via `config.toml`) and as a *server* (Codex itself exposes `codex()` / `codex-reply()` so other MCP clients can orchestrate it).

**Plugin model:** Five composable layers (per Codex's own description of its "customisation stack"):

1. **AGENTS.md** — instructions
2. **Skills** — workflows / behaviors
3. **MCP** — external context
4. **Subagents (TOML)** — parallel execution
5. **Plugins** — distributable bundles

**Sandbox boundaries (the standout feature):**

- Sandbox mode is per-session, configurable in `config.toml` or per subagent.
- **macOS:** Apple Seatbelt (`sandbox-exec`) restricts filesystem and network at syscall level.
- **Linux:** Landlock for filesystem + seccomp for syscall filtering.
- **Windows:** application-layer enforcement only (as of 2026).

This is the only major coding agent enforcing security at the kernel level. Comparable to Devin's VM-per-session but lighter-weight.

## Integration model

**Multi-surface, one config:**

- **CLI** (`codex`) — primary
- **VS Code extension** — embedded
- **Desktop app** — Codex.app
- **Web** — codex.openai.com
- **MCP** — `codex mcp` exposes Codex as an MCP server

The same `~/.codex/` config (AGENTS.md, skills, MCP, subagents) is shared across all surfaces. This is the cleanest cross-surface configuration story among peers (Claude Code is per-binary; Goose has CLI vs Desktop UX divergence).

## Multi-agent / orchestration

**TOML-defined subagents.** A `.codex/agents/<name>.toml` file contains:

- `name`, `description`
- `nickname_candidates` (for auto-routing)
- `developer_instructions` (the system prompt)
- `model` (override)
- `sandbox_mode` (per-agent sandbox)
- Tool/skill enable lists

Subagents can run in parallel on the same repo using **isolated worktrees** (Codex auto-creates a git worktree per parallel subagent). This is *exactly* the same pattern Ark's worktree SPEC implements at a higher level.

Audit-friendly handoffs: parent and subagent both log their actions to the session record.

## Spec / artifact system

**Skills + AGENTS.md.** No PRD/PLAN equivalents. Codex itself follows the same "we set the primitives, you wire the workflow" stance as Claude Code.

## Strengths

- **Rust workspace with 70 crates** — clean engineering, fast cold start, small binary.
- **OS-level sandboxing.** Unique among peers; the only one to enforce at kernel level.
- **Subagents-with-worktrees out of the box.** Parallel safe execution.
- **Cross-surface config.** Same AGENTS.md / skills / MCP / subagents on CLI, VS Code, Desktop, Web.
- **MCP both ways** (client and server).
- **Massive distribution.** 4M weekly active users; first-mover among hyperscaler CLIs since the OSS pivot.
- **Skills standard alignment.** SKILL.md works across Codex, Claude Code, Gemini CLI, Cursor.

## Weaknesses / gaps

- **No workflow opinion** beyond AGENTS.md. Same "harness, you wire the process" stance as Claude Code.
- **No PRD/PLAN/REVIEW/SPEC artifacts.**
- **No journaling.**
- **No persistent cross-session memory abstraction** (skills compensate).
- **AGENTS.md merging is implicit** — debugging "which AGENTS.md won?" is non-trivial. (Ark's `ark context` solves the equivalent problem deterministically.)
- **Sandbox is Linux/macOS only at kernel level**; Windows fallback is weaker.
- **TOML subagents config has shipped-bug history** — e.g., Issue #14161 reports `[[skills.config]]` enable/disable overrides being ignored in subagent contexts.

## Directions for Ark

1. **OS-level sandbox during EXECUTE.** Ark currently relies on the host agent's permission model (Claude Code's allow/deny, Codex's sandbox_mode). Consider whether `ark agent` itself should *enforce* a sandbox profile during EXECUTE — wrap subagent tool calls with `sandbox-exec` (macOS) or `landlock` (Linux) when a `[security] sandbox = "strict"` is set in `.ark/config.toml`. This complements the harness's existing permission model rather than replacing it.
2. **Parallel subagents via worktrees — already aligned.** Ark's worktree SPEC + subagent-support SPEC together cover this; Codex CLI's TOML subagent + worktree pattern validates the design. Audit whether Ark dispatches subagents into their own worktrees in deep tasks; if not, that's a near-term fix.
3. **TOML subagent definitions as a portable export.** Ark already ships `.claude/agents/ark-researcher.md` etc. Generating sibling `.codex/agents/ark-researcher.toml` (and Goose/Gemini equivalents) from a single canonical Ark definition would unify the cross-platform story. Specifically: introduce `ark agent define <name>` that emits matching files in each target platform's format.
4. **Expose `ark agent` as an MCP server.** Codex can run as MCP. Ark could too — making every `ark agent` verb (`task new`, `task plan`, `task review`, `task verify`, `task commit`) callable by any MCP client (Claude Code, Codex, Cline, Cursor, Zed). This is *the* natural integration play and probably the biggest single move available.
5. **Steal the codex-rs Cargo workspace shape.** Codex has ~70 crates in a `codex-rs/` workspace. Ark has 2 (`ark-cli`, `ark-core`). As `ark agent` grows, the natural slicing is by feature area (`ark-task-state`, `ark-spec-promotion`, `ark-context`, `ark-templates`, …). Not a near-term priority but the trajectory is similar.

## Sources

- [openai/codex on GitHub](https://github.com/openai/codex) (queried 2026-05-20)
- [CLI — Codex docs](https://developers.openai.com/codex/cli)
- [Custom instructions with AGENTS.md — Codex docs](https://developers.openai.com/codex/guides/agents-md)
- [Subagents — Codex docs](https://developers.openai.com/codex/subagents)
- [Agent Skills — Codex docs](https://developers.openai.com/codex/skills)
- [Advanced Configuration — Codex docs](https://developers.openai.com/codex/config-advanced)
- [The codex-rs Architecture — Codex Blog](https://codex.danielvaughan.com/2026/03/28/codex-rs-rust-rewrite-architecture/)
- [The Codex CLI Customisation Stack — Codex Blog](https://codex.danielvaughan.com/2026/04/12/codex-cli-customisation-stack-unified-system/)
- [How Codex is built — Pragmatic Engineer](https://newsletter.pragmaticengineer.com/p/how-codex-is-built)
- [OpenAI Codex CLI: The Rust-Powered Terminal Agent — Botmonster](https://botmonster.com/posts/openai-codex-cli-rust-powered-ai-agent/)
- [OpenAI Codex CLI Architecture and Multi-Runtime Agent Patterns — Zylos Research](https://zylos.ai/research/2026-03-26-openai-codex-cli-architecture-multi-runtime-patterns)
