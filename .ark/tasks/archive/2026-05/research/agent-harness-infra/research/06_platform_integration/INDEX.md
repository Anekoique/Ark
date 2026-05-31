# 06 — Platform Integration

How Ark layers itself onto host coding-agent platforms. Ark today is a CLI
+ scaffolded markdown across `.claude/`, `.codex/`, `.opencode/` plus a
`SessionStart` hook (Claude/Codex) or chat-message plugin (OpenCode). This
section asks: which integration surfaces exist, which does Ark already use,
which are worth adopting, and where does cross-platform lock-in bite.

The organising question: **does Ark stay multi-platform-by-templates, or
should it also speak a single shared protocol (MCP, ACP) so the per-platform
template trees become thinner?**

> Scope rule: this section is about *Ark-as-installed-into-a-host-platform*.
> Prior-art surveys of those platforms live in `01_prior_art/`. MCP-the-
> protocol is in `02_infra_primitives/mcp-and-tool-registries.md`. Workflow
> shape is in `04_workflow_systems/`.

## Files

| File | Takeaway |
| ---- | -------- |
| [`slash-commands-vs-cli-vs-mcp.md`](slash-commands-vs-cli-vs-mcp.md) | Three integration surfaces — slash command (markdown-as-prompt), external CLI (Ark's `ark agent`), MCP server (tools/resources/prompts). Trade-offs on discoverability, statelessness, schema enforcement; Ark's two-layer (slash → CLI) split is a hedge that an MCP surface could replace or augment. |
| [`claude-code-integration-deep-dive.md`](claude-code-integration-deep-dive.md) | Claude Code is the richest extension surface in the field — slash commands, subagents, skills, hooks, MCP, settings hierarchy, plugin marketplace, memory. Ark uses ~4 of these 8 surfaces; skills + plugin packaging + per-phase permissions are the obvious gaps. |
| [`codex-integration-deep-dive.md`](codex-integration-deep-dive.md) | Codex copied Claude's primitives (skills, agents, hooks, MCP) but with TOML configs, seconds-not-ms timeouts, OS-level Seatbelt/Landlock sandboxing, and AGENTS.md as the convention file. Ark's `CODEX_PLATFORM` registry already accounts for every schema delta — Codex maturity is high enough to depend on. |
| [`opencode-integration-deep-dive.md`](opencode-integration-deep-dive.md) | OpenCode's plugin model is *runtime* Bun/TS hooks (`.opencode/plugins/*.ts`), not file-based prompt fragments — 25+ lifecycle events exposed to JS code. Ark already ships exactly one TS plugin (`ark-context.ts`). The richer hook taxonomy is a near-term opportunity unique to OpenCode. |
| [`ide-extension-surfaces.md`](ide-extension-surfaces.md) | VS Code / JetBrains / Zed / Helix extension models differ in language, surface, packaging. Agent IDEs (Cursor, Windsurf) fork or extend these. CLI tools (gh, jj, atuin) routinely ship companion VS Code extensions. Ark could too — but the marginal value over slash commands in Claude Code is small. |
| [`cross-platform-portability.md`](cross-platform-portability.md) | Polyglot reality: 6+ context files (CLAUDE.md, AGENTS.md, .cursor/rules/, .github/copilot-instructions.md, GEMINI.md, .windsurfrules) per repo. 2026 consolidation around **AGENTS.md** (Linux Foundation steward, read by 16+ tools). SKILL.md is the portable behaviour-pack format. Ark already writes AGENTS.md; the lock-in story is OK but uneven. |
| [`plugin-and-extension-ecosystems.md`](plugin-and-extension-ecosystems.md) | How harnesses grow ecosystems: Claude Code plugin marketplaces (claude-plugins-official, 55+ plugins), Cursor's Awesome list (5,000+ MCP servers), Codex's openai/skills catalog, npm-style distribution. Ark has no ecosystem yet — but it has the building blocks (slash commands, subagents, skills) to ship as a single plugin. |

## Cross-cutting threads

- **Convergence on three primitives.** Every modern harness now ships:
  *one always-on context file* (AGENTS.md), *one skills directory*
  (SKILL.md per folder), *one MCP config* (`.mcp.json` / `config.toml`).
  Ark targets the first two on every platform; it consumes neither MCP
  nor exposes itself as MCP today.
- **Schema deltas are small but real.** Claude `timeout` is ms; Codex
  `timeout` is seconds. Claude skills live under `.claude/skills/`; Codex
  under `.codex/skills/`. Subagents are markdown+YAML for Claude/OpenCode,
  TOML for Codex. Ark's `platforms.rs` registry handles each per-platform
  delta in a single place — that pattern scales.
- **Slash-command vs skill is collapsing.** Claude Code 2026 docs say
  "skills are the recommended path; commands stay supported." Codex /
  OpenCode followed. Ark ships *both* — `.claude/commands/ark/*.md` and
  `.codex/skills/ark-*/SKILL.md` — and the divergence costs maintenance.
  A single canonical source emitting both would tighten the cost.
- **AGENTS.md is the only durable cross-platform contract.** Read by 16+
  tools as of 2026-Q2. Ark's `MANAGED_BLOCK_BODY` already lands in
  `AGENTS.md` for Codex/OpenCode and `CLAUDE.md` for Claude; broadening
  the Claude install to *also* write `AGENTS.md` is cheap insurance.
- **Plugin distribution is bifurcated.** Claude Code has a curated
  marketplace; Codex has openai/skills; OpenCode has npm packages; Cursor
  has its rules-as-templates Awesome list. Ark could ship as a single
  plugin in each — but the deeper investment is a *single canonical
  definition* that emits to all four formats.

## Reading order

If you only read three: `slash-commands-vs-cli-vs-mcp.md` (the integration-
surface framework), `cross-platform-portability.md` (the polyglot
reality), `plugin-and-extension-ecosystems.md` (how Ark could ship
outside `ark init`).
