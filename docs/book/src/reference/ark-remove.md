# `ark remove`

Unconditionally wipe Ark from a project. No snapshot, no recovery.

## Synopsis

```
ark remove [OPTIONS]
```

## Description

`ark remove` removes:

- `.ark/` (entire tree).
- Each platform's owned dir (`.claude/commands/ark/`, `.codex/skills/`, `.codex/config.toml`, `.codex/hooks.json`, `.opencode/`).
- Every recorded `<!-- ARK -->` managed block (in `CLAUDE.md`, `AGENTS.md`, etc.). The surrounding file is preserved; only the block content + delimiters are removed. Empty parents (e.g. an `AGENTS.md` that contained only the Ark block) are deleted.
- The Ark `SessionStart` hook entry from every platform's hook file. Sibling user hooks (`PreToolUse`, etc.) survive.
- `.ark.db` if present.

This is the destructive twin of [`ark unload`](./ark-unload.md). Use `unload` if you want to come back; use `remove` if you don't.

## Flags

| Flag           | Description                                        |
| -------------- | -------------------------------------------------- |
| `--dir <path>` | Project root. Walks ancestors looking for `.ark/`. |

## Idempotence

Safe to run on a clean repo. `ark remove` reports a no-op summary if there's nothing to remove.

## Examples

```bash
ark remove                       # remove from current project
ark remove --dir /path/to/proj   # remove from elsewhere
```

## See also

- [`ark unload`](./ark-unload.md) — capture state into `.ark.db` first.
- [`ark init`](./ark-init.md) — re-scaffold from scratch.
