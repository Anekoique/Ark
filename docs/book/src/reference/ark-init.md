# `ark init`

Scaffold `.ark/` and the integrations for each selected platform.

## Synopsis

```
ark init [OPTIONS]
```

## Description

`ark init` writes the embedded templates into the host project, installs each selected platform's managed block (`<!-- ARK -->` in `CLAUDE.md` / `AGENTS.md`), records every artifact in `.ark/.installed.json` so later commands can clean up without touching user work, and re-applies each platform's `SessionStart` hook entry.

Safe to re-run: files that already match are left untouched. Files that differ are skipped unless `--force` is set.

Additive on the manifest: if a manifest already exists, only the platform-neutral `.ark/` tree and the selected platforms' dirs are rewritten. Other-platform entries are preserved — `ark init --codex` on a Claude-installed project adds Codex without forgetting Claude.

## Flags

| Flag                  | Description                                                                                                  |
| --------------------- | ------------------------------------------------------------------------------------------------------------ |
| `--dir <path>`        | Target directory. Defaults to current working directory.                                                     |
| `--force`             | Overwrite user-modified template files. Default behavior is to skip.                                         |
| `--claude`            | Include Claude Code integration.                                                                             |
| `--no-claude`         | Exclude Claude Code integration.                                                                             |
| `--codex`             | Include Codex integration.                                                                                   |
| `--no-codex`          | Exclude Codex integration.                                                                                   |
| `--opencode`          | Include OpenCode integration.                                                                                |
| `--no-opencode`       | Exclude OpenCode integration.                                                                                |
| `--developer <name>`  | Bootstrap workspace identity at init time. Mutually exclusive with `--no-developer`. Validated.              |
| `--no-developer`      | Skip workspace identity. Use `ark agent workspace init --name <x>` later to bootstrap.                       |

## Platform selection

If no positive flags are passed and stdin is a TTY, `ark init` prompts interactively to pick platforms. Non-TTY without flags errors out with a message naming all three flag pairs.

When platform flags are mixed:

- Positive flags select that platform.
- Negative flags exclude that platform.
- `--<flag> --no-<flag>` for the same platform: negative wins (excluded).
- At least one platform must remain after filtering, or init errors out.

## Developer name

If neither `--developer` nor `--no-developer` is passed and stdin is a TTY, `ark init` prompts interactively. Non-TTY without flags treats it as `--no-developer` (skip identity bootstrap).

Names are validated: ASCII, starts with letter, `[A-Za-z0-9_-]` only, max 40 chars. A malformed name fails before any platform extraction so partial scaffolds can't be left behind.

## Examples

```bash
# Interactive: prompts for platforms and developer.
ark init

# All three platforms, alice as developer.
ark init --claude --codex --opencode --developer alice

# Claude only, no journal.
ark init --no-codex --no-opencode --no-developer

# Force overwrite of user-modified templates.
ark init --force
```

## What gets written

See [Quick Start → What gets scaffolded](../getting-started/quick-start.md) for the full layout.

## Errors

- `init requires at least one platform` — all platforms excluded after flag filtering.
- `InvalidDeveloperName` — `--developer <name>` failed validation.
- File-conflict errors when re-running over user-modified files without `--force`.

## See also

- [`ark load`](./ark-load.md) — restore from a snapshot or scaffold fresh.
- [`ark upgrade`](./ark-upgrade.md) — refresh templates after a CLI update.
- [`ark agent workspace init`](./ark-agent.md#workspace-init) — bootstrap identity later.
