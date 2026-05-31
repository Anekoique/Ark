# Codex Integration — Deep Dive

OpenAI Codex CLI (`openai/codex`, Rust, Apache 2.0, ~84k stars as of 2026-Q2) is Ark's second-tier target. Codex deliberately mirrors Claude Code's primitives but with deliberate divergences: TOML configs (not JSON), seconds-based timeouts (not milliseconds), OS-level sandboxing (Seatbelt on macOS, Landlock on Linux), AGENTS.md as the canonical instruction file.

> Reference: `01_prior_art/codex-cli.md` for project-level overview; this file catalogues the integration points.

## Extension points

Codex exposes:

1. **Slash commands / skills** — `.codex/skills/<name>/SKILL.md`
2. **Sub-agents** — `.codex/agents/<name>.toml`
3. **Hooks** — `.codex/hooks.json`
4. **MCP servers** — `.codex/config.toml` `mcpServers`
5. **Memory / AGENTS.md** — project-root canonical file
6. **Configuration** — `.codex/config.toml`
7. **Sandbox profiles** — built-in Seatbelt / Landlock profiles

Ark uses 1 (skills, not slash commands — Codex went skills-first), 2 (TOML subagents), 3 (one SessionStart-equivalent hook), 5 (managed block in AGENTS.md). It does not use 4, 6 extensively, or 7.

## Differences from Claude Code

### Config format: TOML, not JSON

Claude Code uses `settings.json`; Codex uses `config.toml`. Both express similar concepts (hooks, MCP servers, defaults) but the syntax differs. Ark's `platforms.rs` registry handles this with per-platform writers — the same logical configuration emits to different files.

### Timeouts: seconds, not milliseconds

Claude Code hook timeouts are in milliseconds; Codex hook timeouts are in seconds. A bug surfaced in Ark history where the same `5000` value meant "5 seconds" on Claude and "5000 seconds" on Codex — would have hung Codex sessions. Caught early; recorded as a per-platform delta in the platform registry.

### Sandboxing: OS-level

Codex ships with platform sandboxing primitives:
- **macOS:** Seatbelt profile (sandbox-exec) restricting file access.
- **Linux:** Landlock-based scoping.

This is a real distinguishing capability — Codex can safely run untrusted code that Claude Code can't.

For Ark this matters because:
- Ark's threat model doesn't currently require sandboxing.
- Codex's sandboxing is a *feature* Ark could surface — e.g. `ark agent task execute --sandbox` would map to Codex's profile in the EXECUTE phase for safety-sensitive tasks.

### Skills-first

Codex never had a separate slash-command directory; it started with skills. `.codex/skills/ark-design/SKILL.md` is the canonical path for Ark's commands on Codex.

Ark already ships per-Codex skills. The Claude-side equivalent (slash commands in `.claude/commands/`) is the gap — see `claude-code-integration-deep-dive.md`'s "skills migration" direction.

### AGENTS.md is the universal context

Codex was an early adopter of AGENTS.md (the Apache-licensed convention). It is read at every session start. Ark writes managed-block content there on Codex installs.

The convergence has spread — by mid-2026 16+ tools read AGENTS.md. Codex was early; Ark already aligns.

### Subagents in TOML

Codex's `.codex/agents/<name>.toml` files declare subagents:

```toml
name = "ark-researcher"
description = "..."
model = "gpt-5"            # optional, defaults to parent
tools = ["read", "write", "grep", "bash"]
system_prompt = """..."""
```

vs. Claude Code's markdown + YAML frontmatter:

```markdown
---
name: ark-researcher
description: ...
tools: read, write, grep, bash
model: claude-sonnet-4-6
---
...
```

Same semantic surface; different syntax. Ark's per-platform subagent emitters handle this.

### Self-exposure as MCP server

A 2025-Q4 Codex feature: Codex itself can run as an MCP server, exposing its capabilities to other agents. This is the "MCP-as-A2A" pattern (`05_orchestration/agent-to-agent-protocols.md`). Codex was one of the early movers here.

For Ark this is interesting because:
- Ark could co-host an MCP server alongside Codex's, multiplexing its capabilities.
- Or Ark could stand alone (`ark-mcp` as a standalone server) — same outcome, less coupling.

## Integration depth: what Ark uses today

| Codex extension | Ark uses? | Notes |
| --------------- | --------- | ----- |
| Skills | Yes (8: ark-design, ark-quick, ark-research, ark-commit, ark-discard, ark-extract-spec, ark-record, ark-resume) | Per-phase workflow steps |
| Subagents (TOML) | Yes (3: ark-researcher, ark-reviewer, ark-verifier) | Matches the Claude trio |
| Hooks (hooks.json) | Yes (one SessionStart-equivalent) | Calls `ark context` |
| MCP servers | No | Same gap as Claude side |
| AGENTS.md memory | Yes (managed block) | Canonical context |
| Config (config.toml) | Limited (defaults for codex subcommands) | Could expose more |
| Sandbox profiles | No | Codex's native; Ark doesn't surface it |

## Codex maturity assessment

As of 2026-Q2:

- Active development (multi-weekly releases).
- Skills + agents + hooks all stable.
- AGENTS.md adoption strong (Codex helped drive convergence).
- MCP integration solid.
- ~84k stars; comparable trajectory to Claude Code's first year.
- OpenAI committed via Codex Cloud (hosted variant) — funding stable.

Bet: Codex is durable. Ark's investment in per-Codex integration is safe.

## Where Codex teaches Claude

A few patterns Codex did better that Claude Code could learn from (and Ark could lobby for):

- **Skills-first** — no slash-command legacy to migrate from.
- **AGENTS.md** — canonical, portable, vendor-neutral.
- **Sandbox profiles** — opt-in OS-level isolation.
- **TOML over JSON for config** — cleaner for human editing.

Claude Code has more total extension points and a richer plugin marketplace; Codex has cleaner core primitives.

## Trade-offs of deeper Codex integration

| Move | Benefit | Cost |
| ---- | ------- | ---- |
| Expose `ark` as MCP from Codex side | Cross-host parity | Same as Claude side — new server |
| Use Codex sandbox profile for `task execute` | Safety in safety-sensitive tasks | Per-phase config; user opt-in story |
| Migrate Claude commands to skills-first like Codex | Cross-host convergence | Multi-format emission |
| Adopt TOML config conventions more broadly | Consistency with Codex's choices | Migrate Claude-side users |

The cheap moves: AGENTS.md on Claude installs (matches Codex's convention); single-source skills (Codex already has them; Claude needs them).

## Per-platform delta record (what `platforms.rs` handles)

For Ark contributors, the actual delta between Claude and Codex is small and well-localised:

```rust
// excerpt of platforms.rs concepts
CODEX_PLATFORM = Platform {
    id: "codex",
    templates: &CODEX_TEMPLATES,
    dest_dir: ".codex",
    removal_root: ".codex",
    cli_flag: "codex",
    managed_block_target: Some("AGENTS.md"),
    hook_file: Some(HookFileSpec { path: ".codex/hooks.json", ... }),
    // ...
}
```

The registry pattern keeps per-platform logic in one place. Each new platform = one `Platform` const + one template tree. No per-platform arms scattered through command implementations.

This is one of Ark's better internal design choices — and a model that other multi-platform tools could adopt.

## Directions for Ark

1. **Register an MCP server in `.codex/config.toml` on init.** Same `ark-mcp` server as on Claude; one extra config line. Multiplies reach without per-platform work.

2. **Surface Codex's sandbox profiles as an Ark option.** A `task execute --sandbox` mode that uses Codex's Seatbelt/Landlock profile during EXECUTE. Optional; opt-in. Safety net for tasks that might run untrusted code.

3. **Ensure subagent TOML stays aligned with the SKILL.md frontmatter source-of-truth.** When the proposed single-source skills land, also single-source the subagent definitions. Codex / Claude / OpenCode emit different syntax from one canonical input.

4. **Document the platforms.rs registry pattern as a reference design.** Other multi-platform tools (chezmoi, dotbot, etc.) have similar problems. Ark's solution is clean; a `docs/book/src/concepts/platform-registry.md` page would teach it.

5. **Track Codex's release notes for new extension points.** Codex moves fast; new event types in `hooks.json` or new fields in subagent TOML appear quarterly. Ark's platform registry should absorb these incrementally.
