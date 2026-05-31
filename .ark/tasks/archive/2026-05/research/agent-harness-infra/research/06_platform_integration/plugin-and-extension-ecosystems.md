# Plugin and Extension Ecosystems

How agent harnesses grow ecosystems — registries, marketplaces, naming conventions, distribution. What it would take for Ark to ship as a community plugin (or grow one).

## The ecosystems in 2026

### Claude Code plugin marketplace

Anthropic ships a public marketplace at `claude.ai/plugins` (and via `/plugins` slash command). Plugins are bundles installable into `.claude/plugins/<name>/` that can ship: slash commands, sub-agents, hooks, MCP servers, skills.

As of mid-2026:
- Official `claude-plugins-official` repo: 55+ first-party plugins.
- Community plugins growing; no full census but the rate is "weekly new ones".
- Install command: `claude install <plugin-name>`.

Pattern: a plugin is a directory + manifest; `claude install` clones it into `.claude/plugins/`.

### Cursor's Awesome list

Cursor doesn't ship a managed marketplace; the de-facto registry is community-curated `Awesome-Cursor-Rules` GitHub repos. Users browse, copy `.mdc` files into their own `.cursor/rules/`.

The Awesome list also includes MCP servers — ~5000+ as of 2026. Cursor's MCP integration is well-developed; the awesome list serves as the catalogue.

### Codex's `openai/skills` catalog

OpenAI maintains `openai/skills` repo as a curated catalogue. Each skill is a directory under `<lang>/<skill-name>/` with a SKILL.md + assets. Install is copy-into-`.codex/skills/`.

Less marketplace-y than Claude Code's; more like a community-contrib catalogue.

### Goose recipes

Block ships `goose-recipes` — community recipes for goose. Install via CLI; runtime-discovered.

### Continue Hub

Continue's 2026 pivot established `hub.continue.dev` — a registry of configurations, skills, MCP servers. Maps closest to "package manager for agent config".

### npm-style for OpenCode

OpenCode plugins are `.ts` files; community distribution piggybacks on npm. No dedicated registry yet (2026-Q2).

### MCP server registries

Anthropic, Cursor, others maintain partial registries of MCP servers. No single canonical list; the field is *too fragmented* for one registry to dominate. The closest to canonical is the `modelcontextprotocol/servers` repo + the various Awesome lists.

## What "shipping as a plugin" means

For Ark to be installable as a plugin in (e.g.) Claude Code's marketplace:

1. **Bundle definition.** A directory containing the slash commands, sub-agent files, hook config, and (ideally) the MCP server binary + config. Manifest declares dependencies and install steps.

2. **Install hook.** The plugin's install step runs `ark init` (or equivalent) to scaffold the per-project artifacts. Some plugin systems run installs at user-level; some at project-level.

3. **Version pinning.** The plugin's version pins to an Ark binary version. Upgrades happen via the plugin manager.

4. **Discoverability.** A description, screenshot, README in the registry.

5. **Distribution.** The Ark binary itself needs to be installable on the user's machine. The plugin can declare a dependency on `ark` being on PATH, or ship a bundled binary.

The Claude Code marketplace pattern is well-suited to Ark: a `claude-plugin/ark` package that installs the templates and registers the MCP server.

## Trade-offs of going plugin-marketplace

| Pro | Con |
| --- | --- |
| Discoverability — users in Claude Code find Ark | Maintenance — plugin manifest tracks Ark releases |
| Install in one step — no `cargo install` + `ark init` | Loss of platform-agnosticism — plugin is Claude-specific |
| Future updates via plugin manager | Plugin marketplace can change rules |
| Reach to non-CLI-comfortable users | Plugin might *replace* CLI as the primary install path; long-term, this could fragment the user base |

The current install path (`npm install -g @anekoique/ark` / `cargo install`) is platform-agnostic and works without any plugin manager. A plugin offering is *additive*: ship both, let users choose.

## How an Ark plugin would look

Concretely, a `claude-plugins/ark` bundle could contain:

```
claude-plugins/ark/
├── plugin.json                # manifest
├── commands/                  # mirrored from .claude/commands/ark/
│   ├── design.md
│   ├── commit.md
│   └── ...
├── agents/                    # mirrored from .claude/agents/
│   ├── ark-researcher.md
│   ├── ark-reviewer.md
│   └── ark-verifier.md
├── mcp/                       # MCP server config
│   └── ark-mcp.json
└── hooks/                     # hook config templates
    └── session-start.json
```

The `plugin.json` manifest declares: name, version, dependencies (Ark binary version range), install script (calls `ark init --no-platform-prompts`).

User installs via `claude install ark` → marketplace fetches the bundle → install script runs `ark init` → user is ready.

Ongoing updates: `claude update ark` → fetches new bundle → calls `ark upgrade`.

## The single-canonical-source angle (again)

If Ark builds the proposed single-source-of-truth for slash commands / skills / subagents (see `cross-platform-portability.md`), the plugin bundle becomes a *generated artifact*. Generate it from the canonical source as part of release builds.

This connects with:
- The platform registry (paths and hooks).
- The canonical templates (content).
- The MCP server (capability).
- The plugin manifest (distribution metadata).

A clean release pipeline generates: binary + Claude plugin + Codex skills catalogue entry + OpenCode npm package + (future) MCP marketplace listing, all from one source.

That's the long-term vision. Today the divergent templates are the friction.

## Risk: ecosystem fragmentation

Multiple registries means the "Ark for Claude Code" plugin is distinct from "Ark for Codex" skills bundle, distinct from "Ark MCP server for Cursor". Maintaining listings across multiple registries is overhead.

Mitigation: pick *one* primary distribution channel (Claude Code marketplace, given it's the largest harness audience), maintain partial presence on others, point users to `ark` CLI as the canonical "I want all platforms" path.

## Risk: plugin manifest drift

Plugin marketplaces change schema. The plugin needs maintenance separate from Ark's main release cycle.

Mitigation: keep the plugin manifest minimal; let `ark init` do the heavy lifting. The plugin is just a thin installer; the work is in the Ark binary.

## What other CLI tools have done

For comparison:

- **`gh` (GitHub CLI):** No plugin marketplace; users install via Homebrew / apt / etc. Lives in the OS package manager. Works for them because `gh` is universally installable and the GitHub web UI is the marketing channel.
- **`cargo` (Rust toolchain):** Distributed via rustup; no plugin marketplace. The crates.io ecosystem is the *library* layer; cargo itself is the tool.
- **`jj` (Jujutsu):** No plugin marketplace; distributed via Homebrew / cargo. Community VS Code extensions exist (community-distributed).
- **`devcontainer` / `dev`:** Spec-based; tool implementations are interchangeable. Not really a plugin model — a protocol model.

The pattern: CLI tools with broad reach rely on OS package managers; tool-specific marketplaces (like Claude Code's) are useful for *agent-host-specific* features but not the primary distribution.

For Ark: keep `npm install -g @anekoique/ark` and `cargo install ark-cli` as the canonical paths. Add Claude Code plugin marketplace as an *adjacent* distribution for users who want Ark to install via their host agent.

## Directions for Ark

1. **Skip ecosystem-building for the binary itself; stay on cargo/npm.** OS package managers + cargo/npm are the right path for `ark` distribution. Don't fragment by inventing a new registry.

2. **Plan for a Claude Code plugin bundle once MCP ships.** When `ark-mcp` exists, the plugin bundle (Ark + MCP server + templates) becomes a single-step install for the Claude Code audience. Wait for MCP; ship the plugin after.

3. **Don't invest in a custom Ark plugin marketplace.** Riding existing ecosystems (Claude plugin marketplace, OpenAI's skills catalogue, Cursor's MCP awesome list) is leverage; building a new one is overhead with no clear win.

4. **Use the single-canonical-source pattern in release tooling.** Once Ark has canonical sources for slash commands / skills / subagents, the release build emits per-platform bundles. This is the prerequisite for low-cost plugin maintenance.

5. **Watch for "AGENTS.md skill packs" emerging.** A community pattern that may emerge: skill packs distributed via AGENTS.md references (e.g. "load `https://example.com/ark-skills`"). If this convention forms, Ark should publish a skill pack URL.
