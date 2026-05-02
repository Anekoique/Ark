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

| Flag             | Description                                                          |
| ---------------- | -------------------------------------------------------------------- |
| `--dir <path>`   | Target directory. Defaults to current working directory.             |
| `--force`        | Overwrite user-modified template files. Default behavior is to skip. |
| `--claude`       | Include Claude Code integration.                                     |
| `--no-claude`    | Exclude Claude Code integration.                                     |
| `--codex`        | Include Codex integration.                                           |
| `--no-codex`     | Exclude Codex integration.                                           |
| `--opencode`     | Include OpenCode integration.                                        |
| `--no-opencode`  | Exclude OpenCode integration.                                        |

## Platform selection

If no positive flags are passed and stdin is a TTY, `ark init` prompts interactively to pick platforms. Non-TTY without flags errors out with a message naming all three flag pairs.

When platform flags are mixed:

- Positive flags select that platform.
- Negative flags exclude that platform.
- `--<flag> --no-<flag>` for the same platform: negative wins (excluded).
- At least one platform must remain after filtering, or init errors out.

## Examples

```bash
# Interactive: prompts for platforms.
ark init

# All three platforms.
ark init --claude --codex --opencode

# Claude only.
ark init --no-codex --no-opencode

# Force overwrite of user-modified templates.
ark init --force
```

## What gets written

See [Quick Start → What gets scaffolded](../getting-started/quick-start.md) for the full layout.

## Errors

- `init requires at least one platform` — all platforms excluded after flag filtering.
- File-conflict errors when re-running over user-modified files without `--force`.

## See also

- [`ark load`](./ark-load.md) — restore from a snapshot or scaffold fresh.
- [`ark upgrade`](./ark-upgrade.md) — refresh templates after a CLI update.
