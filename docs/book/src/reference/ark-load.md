# `ark load`

Restore Ark from a `.ark.db` snapshot, or scaffold fresh if no snapshot exists.

## Synopsis

```
ark load [OPTIONS]
```

## Description

`ark load` is the inverse of [`ark unload`](./ark-unload.md). It picks one of two paths:

1. **`.ark.db` exists** → restore every captured file under owned dirs, every managed block, and every Ark-owned hook entry. The snapshot file is consumed (deleted) on success.
2. **`.ark.db` absent** → behave like [`ark init`](./ark-init.md) with `WriteMode::Force`.

In both paths, after content is in place, each installed platform's hook entry is re-applied to the *current* canonical shape — so a snapshot from an older Ark version restores cleanly even if the hook schema (e.g. `timeout` value) has shifted. Older snapshots that pre-date the per-entry `hook_bodies` capture deserialize successfully (defaulting to empty) and the canonical entries are still re-applied.

## Flags

| Flag           | Description                                                                                                            |
| -------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `--dir <path>` | Target directory. Without `--force`, walks ancestors looking for `.ark/`. With `--force`, treats `--dir` as the target. |
| `--force`      | Wipe the live footprint (owned dirs) before restoring. Required if `.ark/` already exists.                             |

## Refusal

Without `--force`, `ark load` refuses if `.ark/` already exists at the target — the implication is that you want to merge restored state into a live install, which is more error-prone than starting fresh.

## Examples

```bash
# Round-trip: capture, then restore.
ark unload
ark load                  # restores from .ark.db (which `ark unload` wrote)

# Re-load over an existing install.
ark load --force          # wipes .ark/ then restores

# Initial bootstrap from a snapshot file.
ark load --dir /path/to/project --force
```

## Round-trip preservation

User-edited and user-added files inside owned dirs (`.ark/tasks/...`, custom slash commands) survive `unload` → `load` losslessly. The snapshot captures full file contents; managed blocks (the parts of `CLAUDE.md` / `AGENTS.md` between `<!-- ARK:START -->` and `<!-- ARK:END -->`) are captured separately and re-spliced on restore.

User siblings of the Ark hook entry (e.g. a custom `PreToolUse` hook in `.claude/settings.json`) survive a round-trip too: `unload` only surgically removes the Ark `SessionStart` entry and `load` only surgically re-adds it. Other top-level keys and sibling hook entries stay on disk untouched.

## See also

- [`ark unload`](./ark-unload.md) — capture into `.ark.db`.
- [`ark init`](./ark-init.md) — scaffold without a snapshot.
- [`ark remove`](./ark-remove.md) — wipe completely.
