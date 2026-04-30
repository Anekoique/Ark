# Codex

[Codex](https://developers.openai.com/codex/) integration ships out of the box. The platform model is *description-routed skills* rather than slash commands; Ark's wrappers translate the Claude flow accordingly.

## What gets installed

```
.codex/
├── skills/
│   ├── ark-quick/SKILL.md
│   ├── ark-design/SKILL.md
│   ├── ark-archive/SKILL.md
│   └── ark-record/SKILL.md
├── config.toml           # Codex hook config
└── hooks.json            # Ark's SessionStart hook entry

AGENTS.md                 # managed block pointing at .ark/ (created if absent)
```

## Skills

Codex doesn't have user-typed slash commands; instead, the user describes what they want and Codex routes to a matching skill based on its YAML `description` frontmatter. Each Ark skill carries a description Codex can match:

```yaml
---
name: ark-quick
description: Use this skill when the user asks for a quick, reversible change with no
  new abstractions (typo fixes, doc edits, trivial bug fixes, dependency bumps).
---

# ark-quick

... skill body walks the agent through the quick-tier flow ...
```

The body of each skill is a mechanical translation of the matching `templates/claude/commands/ark/<name>.md` body: drop Claude's frontmatter, prepend Codex's `name`/`description` frontmatter, rewrite any inline `/ark:foo` references to `ark-foo` skill names.

Body content parity between Claude commands and Codex skills is **not** mechanically asserted (Claude bodies use slash-invocation idioms that don't translate cleanly to Codex's description-routed model). Two structural tests catch the failure modes that matter:

- `every_claude_command_has_a_codex_skill_sibling` — adding a Claude command without a Codex twin is a test-time failure.
- `codex_skill_bodies_have_codex_frontmatter_not_claude_frontmatter` — copy-paste regressions where a Claude body lands in the Codex tree fail this.

## SessionStart hook

`.codex/hooks.json`:

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

Note `timeout` is **30 seconds**, not milliseconds — Codex's hook schema differs from Claude's (Codex uses seconds, defaults to 600 when omitted). 30 seconds gives `ark context` more than enough budget for any project size.

The same surgical-upsert / surgical-removal contract as Claude: sibling user hooks under any other `hooks.<event>` array are preserved across `unload` / `load` round-trips.

## `.codex/config.toml`

This file enables Codex's `SessionStart` hook by pointing at `hooks.json`. It's not user-editable in any meaningful way; it ships as a shipped template and gets re-applied unconditionally on every `ark init` / `ark load` / `ark upgrade` (not hash-tracked).

## AGENTS.md managed block

Codex reads `AGENTS.md` (parallel to Claude's `CLAUDE.md`). `ark init` creates the file if absent and writes the same managed block:

```markdown
<!-- ARK:START -->
Ark is installed in this project. Use `/ark:quick` or `/ark:design` to start tasks.

See `.ark/workflow.md` for the full workflow.

@.ark/specs/INDEX.md
<!-- ARK:END -->
```

When both Codex **and** OpenCode are installed, they share the `AGENTS.md` file — the managed block is recorded once in the manifest (`(file, marker)` dedupe), and both platforms read the same block.

## What `unload` / `remove` do

- **`ark unload`** captures the entire `.codex/` tree (skills, config, hooks), the `AGENTS.md` managed block, and the Codex `SessionStart` entry from `hooks.json`. Then deletes the live copies. Sibling user hook content (entries Ark didn't install) stays on disk.
- **`ark remove`** removes the same live footprint without capturing. Empty parents are pruned.

A Codex-only project has an `AGENTS.md` but no `CLAUDE.md`; a both-platforms project has both, with the same managed block content in each.

## Claude-only / Codex-only / both

`ark init` lets you pick. Per-platform flags (`--claude` / `--no-claude` / `--codex` / `--no-codex`) work additively:

```bash
ark init --no-codex                # Claude only
ark init --no-claude               # Codex only
ark init                           # interactive prompt (TTY) or all platforms (with --no-developer non-TTY)
```

A Claude-only project upgraded with a CLI version that knows about Codex stays Claude-only — `ark upgrade` does not silently install Codex artifacts. Run `ark init --codex` (additive, idempotent) to opt into Codex on an already-installed project.
