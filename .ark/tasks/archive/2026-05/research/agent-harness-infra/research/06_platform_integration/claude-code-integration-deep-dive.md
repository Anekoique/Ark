# Claude Code Integration — Deep Dive

Anthropic's Claude Code is Ark's primary target. As of 2026 it ships the richest extension surface in the field. This file catalogues every extension point, notes which Ark uses, and identifies where adopting more would pay off.

> Reference: `01_prior_art/claude-code-native.md` for project-level overview; this file is the integration-point catalogue.

## Eight extension points

Claude Code exposes (as of 2026-Q2):

1. **Slash commands** — `.claude/commands/`
2. **Sub-agents** — `.claude/agents/`
3. **Hooks** — `.claude/settings.json` `hooks` field
4. **MCP servers** — `.claude/settings.json` `mcpServers` field (or per-user)
5. **Skills** — `.claude/skills/` (also user-global at `~/.claude/skills/`)
6. **Memory / CLAUDE.md** — project / user / global hierarchy
7. **Settings.json hierarchy** — project / user / global merging
8. **Plugin marketplace** — `.claude-plugin/` install bundles + the public registry

Ark uses 1, 2, 3 (one hook), 6 (managed block in CLAUDE.md). It does not use 4 (MCP), 5 (skills — partially via Codex side), 7 explicitly, or 8.

## 1. Slash commands

**Mechanism:** Markdown files in `.claude/commands/<namespace>/<name>.md`. Filename = command name. Frontmatter optional.

Ark ships 8 commands under `.claude/commands/ark/`: design, quick, research, commit, discard, extract-spec, record, resume.

Each command's body is a templated prompt with `$ARGUMENTS`. Body usually shells to `ark agent <verb>` and orchestrates the agent's filling of artifacts.

**Discoverability:** `/` in chat brings up a picker; user types `/ark:design` and the body expands.

**What Ark does right:**
- Namespace under `ark/` keeps Ark's commands grouped.
- Bodies shell to the CLI for the structural mutation; agent does the prose work.
- Consistent template across commands.

**What could improve:**
- Frontmatter is unused. Could add `argument-hint`, `description` fields for better picker UX.
- No conditional discovery (always-listed) — could shift toward skills (loaded on relevance) for steps that aren't always needed.

## 2. Sub-agents

**Mechanism:** Markdown files in `.claude/agents/<name>.md`. Frontmatter defines `name`, `description`, `tools` (allow-list), optional `model`.

Ark ships three: `ark-researcher.md`, `ark-reviewer.md`, `ark-verifier.md`. Each has a write-scope discipline (`research/<topic>.md` for researcher; `NN_REVIEW.md` for reviewer; `VERIFY.md` for verifier).

**Invocation:** Parent agent calls the `Task` tool with `subagent_type: "ark-researcher"`.

**What Ark does right:**
- Clear name + description (good for parent's Task-tool reasoning).
- Restricted tool-sets (read-only for reviewer/verifier).
- Disk persistence as the return channel.

**What could improve:**
- No `model` override — Anthropic lets you set a different model per subagent (e.g. Haiku for cheap research, Opus for review). Ark inherits the parent's model.
- No `tools` allow-list — Anthropic supports restricting to specific tools; Ark relies on prompt enforcement.

## 3. Hooks

**Mechanism:** `.claude/settings.json` has a `hooks` field that maps event names to shell commands. Event taxonomy:

- `SessionStart` — when a new session begins.
- `UserPromptSubmit` — before each user prompt.
- `PreToolUse` — before a tool runs.
- `PostToolUse` — after a tool runs.
- `Stop` — when the session ends.
- `SubagentStop` — when a sub-agent run completes.
- `Notification` — for notifications.
- (Plus subtypes: `PreBashRun`, `PreWriteFile`, etc.)

Ark installs ONE hook: `SessionStart` runs `ark context` and injects the output. The hook is what gives every session orientation context.

**What Ark does right:**
- One hook is enough for what Ark does today.
- Hook output is structured (JSON or text).
- Hook is documented in `02_infra_primitives/hooks-and-lifecycle-events.md`.

**What could improve:**
- `PostToolUse` for `Edit` / `Write` could auto-detect SPEC drift (file modified but no `[**CHANGELOG**]` entry) and warn.
- `PreToolUse` for `Bash` could detect unsafe `git` patterns (e.g. push to main without confirmation).
- `SubagentStop` could verify the subagent's declared write scope was honoured.
- `Stop` could write a journal entry summarising the session.

These would be opt-in (configurable in `.ark/config.toml` `[hooks]`), not unconditional.

## 4. MCP servers

**Mechanism:** `.claude/settings.json` has an `mcpServers` field listing MCP servers to spawn at session start. Each server defines a command + args; Claude Code talks to them via stdio.

Ark does NOT ship any MCP server today.

**What Ark could do:**
- Ship `ark-mcp` exposing the `ark agent` namespace as MCP tools, the task list / SPECs / context as resources, and templated prompts as prompts.
- Register it in `.claude/settings.json` during `ark init`.
- Result: agent can call Ark via MCP *in addition to* CLI; particularly useful for cross-host use cases.

This is the highest-leverage extension point Ark doesn't currently use.

## 5. Skills

**Mechanism:** `.claude/skills/<name>/SKILL.md`. Frontmatter for metadata (name, description, when-to-use). Optional resources (scripts, templates) in the skill directory. Loaded conditionally based on context.

Ark does NOT ship Claude-side skills (it ships Codex-side skills via `.codex/skills/ark-*/SKILL.md`).

**Why this is a gap:**
- Skills are the converging format (Claude Code, Codex, Goose, Cursor all use them).
- Skills load conditionally; slash commands are always-listed. For phase-specific workflows (e.g. /ark:commit only relevant in COMMIT phase), conditional loading is the right shape.
- A single SKILL.md source could emit to both `.claude/skills/` and `.codex/skills/` — reduces template divergence.

**What Ark could do:**
- Migrate the per-phase slash commands to skills, keeping slash commands as discoverable entry points but deferring the meat to a SKILL.md.
- Single-source the skill bodies; emit per-platform.

## 6. Memory / CLAUDE.md

**Mechanism:** Three layers — project (`./CLAUDE.md`), user (`~/.claude/CLAUDE.md`), global. Loaded into every session, merged top-down.

Plus auto-memory via `/remember`: small markdown files in `~/.claude/projects/<project>/memory/` indexed by topic, loaded conditionally.

Ark uses managed blocks in `./CLAUDE.md` (project layer) — a 5-line note pointing to `.ark/workflow.md` and `.ark/specs/INDEX.md`. The block is owned by `ark init` / `ark upgrade`; user-edited content outside the block is preserved.

**What Ark does right:**
- Project-layer footprint is small (~5 lines).
- Managed block makes ownership clear.
- `@.ark/specs/INDEX.md` reference lazy-loads SPECs on demand (good — no upfront cost).

**What could improve:**
- Ark could also write to AGENTS.md on Claude installs (cross-platform memory convergence).
- Auto-memory bridge: Ark could surface `/remember`-written memory files in `ark context` so multi-platform projects share memory.

## 7. Settings.json hierarchy

**Mechanism:** `~/.claude/settings.json` (user), `./.claude/settings.json` (project), `./.claude/settings.local.json` (project-local, gitignored). Merged at session start.

Ark writes to `./.claude/settings.json` for hook registration. Uses managed-block patterns at the JSON level (the hook entry has an `_ark_managed: true` marker; `unload` removes only those).

**What Ark does right:**
- Project-level only; doesn't touch user / global.
- Marked entries; safe round-trip.

**What could improve:**
- Could expose more configurable settings (allowedTools, hooks-by-event opt-in) via `ark config` subcommand. Today users edit settings.json by hand.

## 8. Plugin marketplace

**Mechanism:** Anthropic ships a plugin registry (claude.ai/plugins). Plugins are bundles installable into `.claude/plugins/<name>/`. Each plugin can ship slash commands, agents, hooks, MCP servers, skills.

Ark is NOT currently a Claude Code plugin. It is a separate CLI tool that scaffolds Claude Code files.

**What Ark could do:**
- Ship as a `claude-plugin/ark` package — install Ark + its templates + its MCP server in one step.
- Maintain installation parity: `ark init` from the CLI = installing the plugin from the marketplace.
- Discoverability: users find Ark via the marketplace, not just via word of mouth.

This is medium-term work; the marketplace is still young and the install-via-plugin path overlaps with `ark init`'s job. But for users who live in Claude Code, marketplace presence opens reach.

## Summary — what Ark uses

| Extension point | Ark uses? | Gap |
| --------------- | --------- | --- |
| 1. Slash commands | Yes (8) | Frontmatter unused; consider skills migration |
| 2. Sub-agents | Yes (3) | No `model` override, no `tools` allow-list |
| 3. Hooks | Yes (1: SessionStart) | More events available (PostToolUse, Stop, SubagentStop) |
| 4. MCP servers | No | The biggest single gap |
| 5. Skills | No (Claude side) | Migrate selected slash commands |
| 6. Memory / CLAUDE.md | Yes (managed block) | Also write AGENTS.md; surface auto-memory |
| 7. Settings.json | Yes (hook) | Could expose more |
| 8. Plugin marketplace | No | Medium-term move once Ark stabilises |

## Trade-offs of adopting more

| Adoption | Benefit | Cost |
| -------- | ------- | ---- |
| MCP server | Cross-host portability; typed surface | New crate, schema design, deployment story |
| Skills migration | Aligns with Claude Code's documented direction | Multi-format emission complexity |
| More hooks | More automated guardrails | Hook output quality affects every session |
| Plugin marketplace | Discoverability | Plugin packaging story; release pipeline |
| Subagent `model` override | Cost optimisation | Per-subagent config in subagent-support SPEC |

The cheapest high-impact moves: subagent `tools` allow-list (one-time SPEC edit), AGENTS.md on Claude installs (cross-platform consistency).

The biggest move: MCP server (opens Ark to every MCP host).

## Directions for Ark

1. **Stand up `ark-mcp` and register it via `.claude/settings.json` on init.** The biggest gap; the biggest leverage. Existing typed `ark agent` namespace makes this a translation layer, not a redesign.

2. **Add `tools` allow-list to each subagent definition.** Currently prompt-only enforcement. Anthropic supports declarative restriction; Ark should use it.

3. **Also write AGENTS.md during Claude installs.** Cross-platform memory convergence; one-line change to `init.rs` platform-write paths.

4. **Pilot the slash-command → skill migration on one command.** Pick `/ark:design` (highest-value, most phase-shaped). Ship as `.claude/skills/ark-design/SKILL.md`. Learn what breaks. Decide whether to migrate the rest.

5. **Add a `Stop` hook that writes a journal entry.** Today journal writes happen at commit; adding a session-end journal entry captures sessions that didn't commit (research, exploration). The hook payload includes session metadata Ark can use.
