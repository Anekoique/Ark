# Cross-Platform Portability

The polyglot reality: a single repo in 2026 often carries 4-6 context files (CLAUDE.md, AGENTS.md, `.cursor/rules/`, `.github/copilot-instructions.md`, GEMINI.md, `.windsurfrules`) because different agents read different files. Ark's bet is multi-platform-by-templates — own the per-platform layer so the user doesn't have to.

## The polyglot context-file problem

As of mid-2026 a "well-instrumented" repo in a multi-tool team commonly contains:

| File | Read by |
| ---- | ------- |
| `CLAUDE.md` | Claude Code, anything that respects Anthropic conventions |
| `AGENTS.md` | Codex, Goose, OpenCode, GitHub Copilot Coding Agent, Continue, Cursor (newer), 16+ tools |
| `.cursor/rules/*.mdc` | Cursor |
| `.github/copilot-instructions.md` | GitHub Copilot |
| `GEMINI.md` | Gemini Code Assist |
| `.windsurfrules` | Windsurf |
| `.kiro/specs/` | Kiro |
| `.opencode/AGENTS.md` | OpenCode (also reads root AGENTS.md) |

Most projects don't have all of these; teams using 2-3 different tools end up with 2-3 of them. Each one duplicates roughly the same content with different syntax.

This is a real problem. Real teams duplicate "use 2-space indent", "prefer functional style", "tests live in `__tests__/`" across files. The first time someone edits one and forgets the others, the agents diverge.

## The convergence on AGENTS.md

The 2026-Q2 picture: **AGENTS.md is winning** the cross-platform context-file role.

Evidence:
- 16+ tools read it. The list grows quarterly.
- It is now stewarded by the Linux Foundation (open governance).
- Codex adopted it early; OpenAI's commitment is high.
- OpenCode reads it.
- Cursor added support (some versions).
- GitHub Copilot Coding Agent reads it.
- Goose reads it.

What it isn't:
- Claude Code's primary file (still CLAUDE.md, though AGENTS.md is supplemental).
- Read by every IDE; some still use their own.

The realistic story: AGENTS.md is the *portable* file; CLAUDE.md remains Claude-specific. Best practice is to write BOTH, possibly with AGENTS.md as the canonical and CLAUDE.md as a short pointer + Claude-specific extras.

Ark currently writes a managed block to CLAUDE.md on Claude installs and to AGENTS.md on Codex / OpenCode. The gap: Ark doesn't write AGENTS.md on Claude installs even though Claude can read it.

## The SKILL.md format

SKILL.md (`<dir>/SKILL.md` + frontmatter) is the analogous convergence for *behaviour packs* — small reusable procedures.

Adopted by:
- Anthropic Claude Code (`.claude/skills/`).
- Codex (`.codex/skills/`).
- Block Goose.
- Cursor (skills directory).

Frontmatter typically declares: `name`, `description`, `when-to-use`, optional `globs`, optional `model`. Body is procedural markdown.

Ark ships per-Codex skills; not Claude skills (Claude side uses slash commands). Cross-platform consolidation would mean: single source of truth in a canonical SKILL.md format, emitting to each platform's expected location and syntax variant.

## The MCP convergence

MCP is the cross-tool *capability* layer. As of 2026:
- Anthropic, OpenAI, Google DeepMind all speak MCP.
- Cursor, Continue, Zed, Claude Code, Codex, OpenCode all integrate MCP servers.
- ~5000+ MCP servers exist (Cursor's Awesome list).

For Ark this means: an MCP server (`ark-mcp`) gives access to Ark capabilities from any MCP host without per-host scaffolding. The MCP server is the cross-platform capability gateway; AGENTS.md is the cross-platform context gateway; SKILL.md is the cross-platform behaviour gateway.

## Ark's current cross-platform shape

Ark uses the platform registry pattern (`crates/ark-core/src/platforms.rs`) to keep per-platform deltas in one place:

```rust
// conceptual
PLATFORMS = &[
    &CLAUDE_PLATFORM,
    &CODEX_PLATFORM,
    &OPENCODE_PLATFORM,
];

// each Platform declares:
// - templates: Dir<'static>
// - dest_dir, removal_root, extra_dirs
// - managed_block_target (CLAUDE.md / AGENTS.md / ...)
// - hook_file (per-platform hook config)
// - cli_flag (--<platform> / --no-<platform>)
```

Commands (`init`, `upgrade`, `unload`, `load`, `remove`) iterate this slice; no per-platform arms scattered through command bodies.

**Strengths:**
- Single source of truth for per-platform deltas.
- Adding a new platform = one registry entry + one template tree. No code changes in command logic.
- Per-platform managed-block patterns handled uniformly.

**Weaknesses:**
- Skill bodies, sub-agent definitions, and slash commands are *three* per-platform template variants. The platform registry handles the deltas, but the *content* is duplicated.
- Cross-platform memory bridges (e.g. Claude's `/remember` not seen by Codex) aren't bridged.

## The "single canonical source, multi-format emit" pattern

The interesting next step: a single declarative source per workflow concept that emits to per-platform formats.

Example for slash commands / skills:

```yaml
# templates/canonical/commands/design.yaml
name: ark-design
description: Start a standard or deep-tier task.
when-to-use: "User wants to design a feature with PRD + PLAN."
body: |
  Start by reading project SPECs and any related feature SPECs from `ark context`.
  ...
```

Then per-platform emitters:
- Claude Code: `.claude/commands/ark/design.md` (slash command) AND/OR `.claude/skills/ark-design/SKILL.md` (skill).
- Codex: `.codex/skills/ark-design/SKILL.md` (skill).
- OpenCode: `.opencode/commands/ark/design.md` (slash command).

The emitters know each platform's format quirks; the canonical source is authoritative.

This is the natural extension of the platform-registry pattern: registry handles paths and hooks; emitters handle content.

## Lock-in risk

Ark's lock-in story is OK but uneven:

- **Workflow primitives** (`task new`, `task plan`, etc.) are in the Ark binary; portable across hosts.
- **Artifact formats** (PRD.md, PLAN.md, VERIFY.md) are plain markdown; portable.
- **Feature SPECs** are plain markdown; portable.
- **Per-platform templates** (slash commands, agents, skills) are per-host syntax; if a user switches from Claude to Codex, Ark re-installs.

What happens if a user wants to abandon Ark and keep their work?

- Markdown artifacts survive (PRD, PLAN, VERIFY, SPECs, journals).
- The workflow disappears (`ark agent task verify` won't run).
- The slash commands disappear.

Reversibility is good — every artifact is a plain markdown file in the user's repo, committed normally. Ark doesn't lock you in beyond the integrations it ships.

This is a defensible position: *Ark adds workflow; doesn't capture data*.

## Convergence forecast

For 2026–2027 the most likely directions:

1. **AGENTS.md becomes universal.** Every shipping tool reads it. CLAUDE.md and the per-tool files become supplementary.
2. **SKILL.md becomes the behaviour-pack format.** Cross-platform skill registries (Awesome lists, marketplaces) build on it.
3. **MCP becomes the cross-host capability layer.** Per-tool integration code shrinks to "list MCP servers".
4. **ACP wins on the editor-agent axis** (Zed, JetBrains; possibly more). VS Code / Cursor watch from sidelines.
5. **Per-tool config files diverge in syntax but converge in shape.** Cursor JSON, Codex TOML, Claude JSON, OpenCode TS — but the schemas are 80% the same.

For Ark: align with AGENTS.md, SKILL.md, MCP. Watch ACP. Maintain the platform-registry pattern for the remaining 20% of platform-specific deltas.

## Directions for Ark

1. **Always write AGENTS.md on every install, regardless of platform.** Cross-platform convergence is strong; the cost is one file write. Hooks into existing platform-write paths in `init.rs`.

2. **Plan a SKILL.md canonical source.** A `templates/canonical/skills/<name>/SKILL.md` source that emits per-platform via the platform registry. Highest leverage for collapsing duplicate maintenance.

3. **Stand up `ark-mcp` as the cross-host capability layer.** Same recommendation that surfaces in many files; it deserves prioritisation because it touches portability, integration, and orchestration simultaneously.

4. **Document the lock-in story explicitly.** A `docs/book/src/concepts/reversibility.md` page that says "Ark adds workflow; doesn't capture data. You can leave with your artifacts intact." This is a strength worth marketing.

5. **Watch the AGENTS.md governance.** Linux Foundation stewardship means the format will evolve via proposal. Ark should track changes and be among the first to adopt new fields.
