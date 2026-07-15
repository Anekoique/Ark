# CodeAgent CLI Integration Surface

## Directory Layout

```
.cac/
├── commands/             # Slash commands (markdown files)
│   └── ark/              # Ark namespaced commands
│       ├── commit.md
│       ├── design.md
│       ├── quick.md
│       └── ...
├── agents/               # Custom subagents (markdown with YAML frontmatter)
│   ├── ark-researcher.md
│   ├── ark-reviewer.md
│   └── ark-verifier.md
├── settings.json         # Project-scoped hook + permission config (Git-tracked)
└── settings.local.json   # Local-only config (not Git-tracked)
```

## Commands

- **Format:** Markdown with YAML frontmatter (`description` field only)
- **Path:** `.cac/commands/ark/<name>.md`
- **Frontmatter:**
  ```yaml
  ---
  description: Start a standard or deep-tier task...
  ---
  ```
- **Body:** Same body as Claude commands, uses `$ARGUMENTS` token
- **Invocation:** `/ark:<name> $ARGUMENTS`

## Hooks

- **File:** `.cac/settings.json` (project-scoped, Git-tracked)
- **Structure:** Same shape as Claude's `.claude/settings.json`:
  ```json
  {
    "hooks": {
      "SessionStart": [
        {
          "matcher": "",
          "hooks": [
            {
              "type": "command",
              "command": "ark context --scope session --format json",
              "timeout": 30
            }
          ]
        }
      ]
    }
  }
  ```
- **Timeout unit:** Seconds (like Codex, unlike Claude which uses milliseconds)
- **Identity key:** `command` (same as Claude/Codex)
- **27 hook events** supported, including SessionStart, PreToolUse, PostToolUse, etc.

## Subagents

- **Format:** Markdown with YAML frontmatter (same as Claude's `.claude/agents/`)
- **Path:** `.cac/agents/<name>.md`
- **Frontmatter fields:**
  ```yaml
  ---
  name: ark-researcher
  description: |
    Use during DESIGN and PLAN to gather knowledge...
  permissionMode: bypassPermissions
  tools:
    - Read
    - Write
    - Edit
    - Bash
    - Glob
    - Grep
    - WebFetch
  ---
  ```
- **Additional optional fields:** `model`, `color`, `memory`, `disallowedTools`, `effort`, `maxTurns`, `skills`, `mcpServers`, `hooks`, `background`, `initialPrompt`, `isolation`

## Project Doc

- **File:** `AGENTS.md` (same as Codex/OpenCode)

## Comparison Table

| Aspect | Claude Code | Codex | OpenCode | CodeAgent CLI |
|--------|------------|-------|----------|---------------|
| Root dir | `.claude/` | `.codex/` | `.opencode/` | `.cac/` |
| Commands | `.claude/commands/ark/*.md` | `.codex/skills/ark-*/SKILL.md` | `.opencode/commands/ark/*.md` | `.cac/commands/ark/*.md` |
| Command frontmatter | Claude-specific | Codex-specific | `description` only | `description` only |
| Hook file | `.claude/settings.json` | `.codex/hooks.json` | None (TS plugin) | `.cac/settings.json` |
| Hook array key | `SessionStart` | `SessionStart` | N/A | `SessionStart` |
| Hook timeout unit | Milliseconds | Seconds | N/A | Seconds |
| Identity key | `command` | `command` | N/A | `command` |
| Agents dir | `.claude/agents/*.md` | `.codex/agents/*.toml` | `.opencode/agents/*.md` | `.cac/agents/*.md` |
| Agent format | YAML frontmatter + md | TOML | YAML frontmatter + md | YAML frontmatter + md |
| Project doc | `CLAUDE.md` | `AGENTS.md` | `AGENTS.md` | `AGENTS.md` |
| Config file | None | `.codex/config.toml` | None | None |

## Key Observations

1. **Hook surface is JSON-based** (like Claude/Codex) — fits existing `HookFileSpec` perfectly
2. **Command format** is nearly identical to Claude's (YAML frontmatter with `description`, body with `$ARGUMENTS`)
3. **Agent format** is identical to Claude's (YAML frontmatter + markdown body) — no new parser needed
4. **Timeout in seconds** (like Codex) — need a new `ark_codeagent_hook_entry()` function
5. **Managed block target** is `AGENTS.md` (shared with Codex/OpenCode) — manifest dedupes on `(file, marker)`
6. **No extra config file** needed (unlike Codex's `.codex/config.toml`)
7. **`removal_root`** is `.cac/` (wholly Ark-owned, like Codex/OpenCode)
8. **Agents live under removal_root** so no `extra_dirs` needed (like Codex/OpenCode)
