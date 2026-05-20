# IDE Extension Surfaces

How major IDEs expose extension points, and whether Ark should ship companion plug-ins for them. As of 2026 most CLI tools layer thin VS Code or JetBrains extensions on top — the question is whether the marginal value over slash commands inside Claude Code (or equivalent) is worth the build.

## The four major IDE families

### 1. VS Code

- **Extension API:** TypeScript / JavaScript; runs in extension host (Node.js).
- **Surface:** Activity bar items, side panels, status bar, commands (Ctrl+Shift+P), terminals, file decorations.
- **Distribution:** VS Code Marketplace + Open VSX.
- **Maturity:** Mature, dominant.

Most CLI-tool extensions wrap the CLI:
- `gh` (GitHub CLI) — official VS Code extension wraps `gh` commands.
- `jj` (Jujutsu) — community extensions wrap `jj` commands.
- `atuin` (shell history) — extension surfaces command history in VS Code.

The pattern: extension provides UI affordances; CLI does the work.

### 2. JetBrains (IntelliJ family)

- **Extension API:** Kotlin / Java; IntelliJ Platform.
- **Surface:** Tool windows, gutter actions, intentions, run configurations.
- **Distribution:** JetBrains Marketplace.
- **Maturity:** Mature, dominant in Java / Kotlin / Python (PyCharm) audiences.

Heavier than VS Code extensions; plugins ship as JARs.

### 3. Zed

- **Extension API:** Rust / WebAssembly.
- **Surface:** Slash commands, custom panels, agent panel actions.
- **Distribution:** Zed extension registry.
- **Maturity:** Newer; growing.

Zed extensions are interesting because they can compile from Rust to WASM — for a Rust project like Ark, the language match makes extension development cheaper than other IDEs.

### 4. Helix / Neovim / classic editors

- **Extension API:** Lua (Neovim), config + plugin packages (Helix).
- **Surface:** Commands, keymaps, status line.
- **Distribution:** Plugin manager ecosystems (lazy.nvim, packer).
- **Maturity:** Mature; dedicated audiences.

## Agent IDEs as extension layers

Cursor, Windsurf, Kiro, Trae are all VS Code forks. Their extensions are *additive on top of* VS Code's: Cursor adds Composer, Background Agents, Rules; Windsurf adds Cascade.

For Ark this means: a VS Code extension would *also* work on Cursor / Windsurf / forks (with minor compatibility caveats). One extension, broad reach.

Zed is *not* a VS Code fork — separate extension model.

## What an Ark IDE extension would do

The candidate features for an Ark VS Code extension:

1. **Status bar item:** current task, phase, branch (sourced from `ark context`).
2. **Side panel:** task list, active task's PRD/PLAN/VERIFY at a glance.
3. **Commands:** Ctrl+Shift+P → "Ark: New Task", "Ark: Verify", "Ark: Commit" — invokes the CLI in the integrated terminal or as a background process.
4. **File-tree decorations:** colored badge on tasks in `.ark/tasks/` (active = green, archived = grey).
5. **Inline cues:** when editing a feature SPEC, show whether CHANGELOG is updated.

Would users use this? Some would — power users who live in VS Code and want one-click task management without leaving the editor. But:

- Users already in Claude Code (CLI) don't need it.
- Users in Cursor get most value from Composer + Rules, not Ark UI.
- Users in JetBrains would need a separate plugin.

The likely audience is *VS Code + GitHub Copilot* users who don't use Claude Code as the primary harness. That's a real audience — Copilot is dominant in enterprise — but it's a different category from Ark's current "user runs Claude Code, Ark layers on top".

## Cost-benefit

| Investment | Benefit | Cost |
| ---------- | ------- | ---- |
| VS Code extension (basic: status + commands) | Reach to VS Code/Cursor/Windsurf users | ~2–4 weeks dev; marketplace listing; per-version maintenance |
| JetBrains plugin | Reach to JetBrains audience | ~4–8 weeks dev (less ergonomic than VS Code); maintenance |
| Zed extension | Reach to Zed audience | ~1–2 weeks (Rust→WASM is easy); smaller audience |
| Helix / Neovim | Reach to terminal-power-users | Small communities; effort vs. reach unfavourable |

**Highest leverage per week:** VS Code (because of Cursor/Windsurf inheritance).
**Cheapest:** Zed (because Rust-native; small extension scope).
**Skip:** Helix/Neovim/JetBrains until demand signals are clear.

## The MCP alternative

A different framing: instead of building IDE-specific extensions, ship an `ark-mcp` server and let each IDE's MCP integration do the integration.

Cursor speaks MCP. Continue speaks MCP. Zed speaks MCP. Claude Code can speak MCP. Most "I want Ark visible in my IDE" outcomes are achievable via MCP without writing native extensions.

The trade-off:
- MCP gives capability access, not UI. The IDE renders MCP resources / prompts in its agent panel, but doesn't get a status bar item / side panel / etc.
- A native extension gives UI; an MCP server gives capability.

For Ark, UI affordances are nice-to-have; capability access is must-have. *MCP is the higher priority.*

## CLI-companion extensions in the wild

What extensions look like in practice for similar CLI tools:

### `gh-vscode`

Official GitHub CLI extension. Provides:
- Status bar item showing PR review status.
- Side panel listing open PRs, issues.
- Commands: `GitHub: Create Pull Request` etc.

Implementation: VS Code TS extension that spawns `gh` subprocesses and parses JSON output.

Lessons:
- Simple architecture (spawn + parse).
- Schema-stable CLI output makes it work.
- Real value in status surfaces (always-visible).

### `Jujutsu Kaizen` (jj VS Code extension)

Community-maintained. Wraps `jj` commands; renders working-copy state in side panel.

Lessons:
- Community extensions can ship without official support.
- Domain-specific UI (working-copy state) is the differentiator.

### `Atuin` (shell history) VS Code extension

Pulls command history from `~/.local/share/atuin/`; provides command palette.

Lessons:
- Read-only extensions are simpler and less risky.
- Even read-only extensions are useful.

## What Ark's IDE story should be

Layered recommendation:

1. **Short-term (now):** Skip IDE extensions. Focus engineering on MCP server and skills migration. Users in IDEs get value via the host agent's existing integrations (Claude Code in terminal, MCP in Cursor/Continue/Zed).

2. **Medium-term (when MCP ships):** Ship a minimal Zed extension (Rust→WASM cheap; small audience but architecturally aligned). Status bar item + slash command surface. Mostly demo / proof-of-concept.

3. **Long-term (if VS Code reach is needed):** Ship a VS Code extension when the data signals demand. Track inbound requests; ship when ~5+ enterprise users ask.

## Risks of shipping native extensions

- **Per-version churn.** VS Code extension API changes; JetBrains has even more friction. Maintaining N extensions on each release of N IDEs is a real cost.
- **Schema drift between Ark and extension.** If the extension parses `ark context --format json` and Ark changes the schema, the extension breaks until updated.
- **Discoverability illusion.** A marketplace listing doesn't guarantee users find Ark; "Ark for VS Code" might languish.

The MCP-first path sidesteps most of these. MCP schema stability is owned by `ark-mcp` (one place). Hosts that consume MCP do the integration work.

## Directions for Ark

1. **Defer IDE extensions until MCP ships.** Most "IDE integration" value can be delivered by MCP without writing native plug-ins. Don't optimise prematurely.

2. **If shipping IDE extensions, start with Zed.** Rust-native, WASM extensions are cheap, Ark community-aligned. Use it as a sandbox for what an IDE-Ark experience could look like.

3. **Schema-stable CLI output is the prerequisite.** `ark context --format json` already is; preserve that promise. IDE extensions (if built) depend on it being load-bearing.

4. **VS Code extension only on demand signal.** Marketplace + maintenance cost is real; don't ship unless users are asking.

5. **Document the MCP-vs-extension trade-off.** Users / contributors who ask "why no VS Code extension?" should have a doc to read. `docs/book/src/integrations/ide.md` would suffice.
