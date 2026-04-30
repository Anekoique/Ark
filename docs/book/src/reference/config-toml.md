# `.ark/config.toml`

User-editable consolidated config. One file, sectioned by feature.

## Synopsis

```toml
[worktree]
worktree_dir = ".ark/worktrees"
branch_prefix = "feat"
copy = []
post_create = []

[workspace]
journal_max_lines = 2000
auto_record_on_archive = true
```

## Lifecycle

Created by `ark init` from the embedded template. **Never overwritten by `ark upgrade`** — your edits to `[worktree]` and `[workspace]` settings persist across CLI updates. To pick up a new default that ships in a later Ark version, edit your `config.toml` by hand.

If the file is missing entirely (e.g. the project predates this config), every section falls through to its default. If a specific `[section]` is absent, that feature uses defaults. If the file exists but is malformed TOML, the next operation that needs a config (e.g. `ark agent task new --worktree`) errors with `WorktreeConfigCorrupt` or `WorkspaceConfigCorrupt`.

## `[worktree]`

| Field           | Type        | Default          | Description                                                                                       |
| --------------- | ----------- | ---------------- | ------------------------------------------------------------------------------------------------- |
| `worktree_dir`  | string      | `.ark/worktrees` | Project-relative path where worktrees go. Absolute paths and `..` traversal rejected.             |
| `branch_prefix` | string      | `feat`           | Default branch prefix when neither `--branch-type` nor `--branch` is passed to `task new --worktree`. |
| `copy`          | `[]string` | `[]`             | Project-relative paths to copy into each new worktree on creation (e.g. `.env`, `.envrc`).         |
| `post_create`   | `[]string` | `[]`             | Shell commands run in the worktree dir after `git worktree add`. Sequential. Abort on first non-zero. |

**Validation.** `worktree_dir` must stay inside the project. Absolute paths → `InvalidConfigField`. `..` traversal → `InvalidConfigField`. The aim is to prevent a malicious or careless config from making `task new --worktree` write outside the project tree.

**`copy` semantics.** Sources missing on disk → `WorktreeCopySourceMissing` (hard fail; rolls back the worktree). Source paths are resolved relative to the parent checkout's project root. Destination is the same relative path under the new worktree.

**`post_create` semantics.** Each command runs via `sh -c <cmd>` with cwd set to the new worktree. Stdout/stderr inherit. The first non-zero exit aborts the rest of the list and rolls back the worktree dir (`git worktree remove --force`).

## `[workspace]`

| Field                     | Type   | Default | Description                                                                                       |
| ------------------------- | ------ | ------- | ------------------------------------------------------------------------------------------------- |
| `journal_max_lines`       | u32    | `2000`  | Lines per journal file before rotation. Must be ≥ 100 (smaller caps cause index re-render to thrash). |
| `auto_record_on_archive`  | bool   | `true`  | When `false`, `task archive` skips workspace auto-record entirely.                                 |

**Validation.** `journal_max_lines < 100` → `InvalidConfigField` with `field = "journal_max_lines"`.

**`auto_record_on_archive = false` semantics.** `task archive` returns `SkippedDisabled` without reading `.ark/.developer` or invoking `git log`. Use this if you've initialized a developer for some other purpose but don't want auto-records — e.g. you prefer to record only via explicit `/ark:record`.

## Examples

```toml
# A team that copies .env into each worktree and runs a build to warm caches.
[worktree]
copy = [".env"]
post_create = ["cargo build"]

# Tighter rotation; record only when explicit.
[workspace]
journal_max_lines = 500
auto_record_on_archive = false
```

## Adding a new section

If you're extending Ark itself (see [Contributing](../contributing/workspace-layout.md)), each feature owns its own raw shape and falls through to its `Default` impl when its section is absent. Adding a new feature's section is independent of existing ones.
