# `ark unload`

Capture every Ark-owned file, managed block, and hook entry into `.ark.db`, then remove the live footprint.

## Synopsis

```
ark unload [OPTIONS]
```

## Description

`ark unload` produces a portable snapshot of the entire Ark footprint:

1. **Files** — every regular file under owned dirs (`.ark/`, `.claude/commands/ark/`, `.codex/`, `.opencode/`), excluding `.ark/worktrees/` and `.ark/.developer`.
2. **Managed blocks** — every `<!-- ARK -->` block recorded in `.ark/.installed.json` (or every shipping platform's managed-block target as a fallback if the manifest is missing).
3. **Hook entries** — every Ark-owned `SessionStart` entry across every platform's hook file. Stage A captures known platforms' entries; Stage B scans every `*.json` file under owned dirs for orphan Ark-identity entries (e.g. from a future-version platform whose plumbing this Ark binary doesn't know about).

After capture, the snapshot is written to `.ark.db` (TOML at the project root), then the live footprint is deleted. Sibling user content stays on disk: user-added files inside owned dirs are captured-and-then-deleted (they survive on round-trip via `load`), but user hook siblings (e.g. a `PreToolUse` entry in `.claude/settings.json`) stay live — only the Ark `SessionStart` entry is surgically removed.

## Flags

| Flag           | Description                                              |
| -------------- | -------------------------------------------------------- |
| `--dir <path>` | Project root. Walks ancestors looking for `.ark/`.       |

## Worktree exclusion

`unload` skips `.ark/worktrees/` to avoid recursing into per-task git checkouts (which would capture their `target/`, uncommitted edits, etc.). The skip path comes from `[worktree].worktree_dir` in `.ark/config.toml`, so a non-default setting is honored.

The gitignored `.ark/.developer` identity file is also excluded — it's per-machine, not portable.

## VCS

`.ark.db` is the user's responsibility to gitignore. Ark doesn't write to your `.gitignore` to add it. The file is a checkpoint, not a permanent artifact; commit it intentionally if you want to share state with collaborators (rare), or `.gitignore` it.

## Examples

```bash
# Full capture.
ark unload

# Then restore.
ark load
```

## Errors

- `NotLoaded` — `.ark/` directory doesn't exist at the discovered project root.

## See also

- [`ark load`](./ark-load.md) — restore from snapshot.
- [`ark remove`](./ark-remove.md) — destroy state without saving.
