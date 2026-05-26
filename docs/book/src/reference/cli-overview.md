# CLI Overview

The `ark` binary has these visible top-level commands:

| Command       | What it does                                                                                         |
| ------------- | ---------------------------------------------------------------------------------------------------- |
| `ark init`    | Scaffold `.ark/` and per-platform integrations from embedded templates.                              |
| `ark load`    | Restore from `.ark.db` snapshot, or scaffold like `init` if no snapshot exists.                      |
| `ark unload`  | Snapshot everything under `.ark/` + managed blocks + Ark hooks into `.ark.db`.                       |
| `ark remove`  | Wipe Ark fully: `.ark/`, platform dirs, managed blocks, `.ark.db`, hooks.                            |
| `ark upgrade` | Refresh embedded templates to the current CLI version.                                               |
| `ark context` | Print a structured snapshot of git + `.ark/` workflow state. Read-only.                              |
| `ark archive` | Bulk-move every `phase = Committed` task into its `committed_at` month bucket. Manager-only.         |
| `ark cleanup` | List (dry-run) or remove (`--apply`) worktrees whose backing task is closed or whose branch is gone. |

Plus one hidden internal command:

| Command     | Purpose                                                            |
| ----------- | ------------------------------------------------------------------ |
| `ark agent` | Task lifecycle and spec management. **Not semver-stable.**         |

## Stability policy

- **Visible commands** (those above) are semver-stable. Flags don't disappear within a major version; output formats (especially `ark context --format json`) follow additive-only schema versioning.
- **`ark agent`** is internal. Its callers are the shipped slash commands and the workflow doc. The CLI surface is *not* covered by semver — the binary and its shipped templates version together.

End users should drive workflow through slash commands, not by calling `ark agent` directly. See [`ark agent`](./ark-agent.md) for when you do need to reach for it (mostly: rare lifecycle operations the slash commands don't cover).

## Project discovery

Most commands find the project root by walking ancestors looking for a `.ark/` directory:

- `ark context`, `ark unload`, `ark remove`, `ark upgrade`, `ark load` (without `--force`) — walk up from cwd.
- `ark init` and `ark load --force` — operate on the explicit target (defaulting to cwd if `--dir` is omitted). They scaffold a project; they don't locate one.

Pass `--dir <path>` to override discovery. When provided, it always wins.

## Conventions

- **Errors** print to stderr with `error: <message>` and a chained `caused by:` for each source. Exit code 1 on any failure.
- **`--help`** is supported on every level. `ark --help` lists top-level commands; `ark <cmd> --help` lists flags for that command. `ark agent --help` is the only way to see the hidden namespace.
- **JSON output.** Only `ark context --format json` produces machine-readable output; the rest are human-targeted summaries via `Display`.
