# `ark upgrade`

Refresh embedded templates to the current CLI version.

## Synopsis

```
ark upgrade [OPTIONS]
```

## Description

When you install a newer `ark` binary, run `ark upgrade` inside each Ark-managed project to bring its template files up to date. The command compares the project's recorded template hashes (in `.ark/.installed.json`) against the embedded templates of the running binary and produces a Plan: add new files, auto-update unchanged-on-disk files, prompt for files the user has edited, delete files the new version removed.

Three things are **always** re-applied unconditionally (they're not hash-tracked):

- Each platform's `<!-- ARK -->` managed block in `CLAUDE.md` / `AGENTS.md`.
- Each platform's Ark `SessionStart` hook entry.
- The shipped `extra_files` for any platform that has them (e.g. OpenCode's `ark-context.ts` plugin).

`.ark/config.toml` is **never** overwritten, even if its hash changed. User edits to `[worktree]` settings persist across upgrades.

## Flags

| Flag                 | Description                                                                                  |
| -------------------- | -------------------------------------------------------------------------------------------- |
| `--dir <path>`       | Project root. Walks ancestors looking for `.ark/`.                                           |
| `--force`            | On user-modified files, overwrite without prompting. Default behavior is to prompt.          |
| `--skip-modified`    | On user-modified files, skip silently. Default behavior is to prompt.                         |
| `--create-new`       | On user-modified files, write the new version to `<file>.new` so you can diff manually.       |
| `--allow-downgrade`  | Permit `manifest.version > cli_version`. Default behavior is to error with `DowngradeRefused`. |

`--force` / `--skip-modified` / `--create-new` are mutually exclusive.

## Conflict resolution

For each file:

| State                                       | Action                                                    |
| ------------------------------------------- | --------------------------------------------------------- |
| New file in CLI, absent from project        | Add (recorded in manifest).                               |
| Existing, on-disk hash matches recorded     | Auto-update to new template (recorded hash refreshed).    |
| Existing, on-disk hash differs from recorded | User-modified → prompt unless `--force` / `--skip-modified` / `--create-new` is set. |
| Existing, recorded but missing from CLI     | Delete (orphan template, e.g. removed in a CLI release).  |
| Existing in old CLI, absent in new CLI      | Drop manifest entry; file stays on disk if user-modified.  |

The plan is sorted: writes (Add → AutoUpdate → Overwrite) first, then `CreateNew`, then `RefreshHashOnly`, then `Preserve`, then `Delete`. The manifest is flushed durably **before** any delete can fail, so a partial failure mid-delete leaves the manifest consistent with disk state.

## Path safety

Every entry in `manifest.files` is normalized through `Layout::resolve_safe` before any I/O. A manifest pointing at `..//etc/passwd` errors out before the upgrade does anything. Embedded templates can't trip this check because their paths are construction-time, not data.

## Version policy

`ark upgrade` refuses to run if `manifest.version > cli_version` (you have a newer Ark recorded than the binary you're running). Override with `--allow-downgrade` if that's intentional.

## Examples

```bash
ark upgrade                    # interactive on user-modified files
ark upgrade --skip-modified    # leave user edits alone
ark upgrade --force            # overwrite user edits
ark upgrade --create-new       # write `.new` files for manual merge
```

## See also

- [`ark init`](./ark-init.md) — initial scaffold.
- [`.ark/config.toml`](./config-toml.md) — preserved across upgrades.
