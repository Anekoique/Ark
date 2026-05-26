# `.ark/config.toml`

User-editable consolidated config. One file, sectioned by feature.

## Synopsis

```toml
[worktree]
worktree_dir = ".ark/worktrees"
branch_prefix = "feat"
copy = []
post_create = ["git submodule update --init --recursive"]

[workspace]
journal_max_lines = 2000
```

## Lifecycle

Created by `ark init` from the embedded template. **Never overwritten by `ark upgrade`** — your edits persist across CLI updates. To pick up a new default that ships in a later Ark version, edit your `config.toml` by hand.

If the file is missing entirely (e.g. the project predates this config), every section falls through to its default. If a specific `[section]` is absent, that feature uses defaults. If the file exists but is malformed TOML, the next operation that needs a config (e.g. `ark agent task new --worktree`) errors with `WorktreeConfigCorrupt`.

## `[worktree]`

| Field           | Type        | Default          | Description                                                                                       |
| --------------- | ----------- | ---------------- | ------------------------------------------------------------------------------------------------- |
| `worktree_dir`  | string      | `.ark/worktrees` | Project-relative path where worktrees go. Absolute paths and `..` traversal rejected.             |
| `branch_prefix` | string      | `feat`           | Default branch prefix when neither `--branch-type` nor `--branch` is passed to `task new --worktree`. |
| `copy`          | `[]string` | `[]`             | Project-relative paths to copy into each new worktree on creation (e.g. `.env`, `.envrc`).         |
| `post_create`   | `[]string` | `["git submodule update --init --recursive"]` | Shell commands run in the worktree dir after `git worktree add`. Sequential. Abort on first non-zero. Set to `[]` to disable. |

**Validation.** `worktree_dir` must stay inside the project. Absolute paths → `InvalidConfigField`. `..` traversal → `InvalidConfigField`. The aim is to prevent a malicious or careless config from making `task new --worktree` write outside the project tree.

**`copy` semantics.** Sources missing on disk → `WorktreeCopySourceMissing` (hard fail; rolls back the worktree). Source paths are resolved relative to the parent checkout's project root. Destination is the same relative path under the new worktree. Don't list `.ark/.developer` here — it is synced into each worktree automatically.

**`post_create` semantics.** Each command runs via `sh -c <cmd>` with cwd set to the new worktree. Stdout/stderr inherit. The first non-zero exit aborts the rest of the list and rolls back the worktree dir (`git worktree remove --force`). The shipped default initializes git submodules (a no-op in repos without `.gitmodules`).

## `[workspace]`

| Field               | Type   | Default | Description                                                                      |
| ------------------- | ------ | ------- | -------------------------------------------------------------------------------- |
| `journal_max_lines` | int    | `2000`  | Lines per journal file before rotation (`journal-N.md` → `journal-{N+1}.md`). Must be > 0. |

Workspace journals and developer identity are covered in [Workspace & Journals](../workflow/workspace.md). Developer identity itself lives in `.ark/.developer` (gitignored), not in this file — though `[workspace] developer` is accepted as a fallback when `.ark/.developer` is absent.

## Examples

```toml
# A team that copies .env into each worktree and runs a build to warm caches.
[worktree]
copy = [".env"]
post_create = ["cargo build"]
```

## Adding a new section

If you're extending Ark itself (see [Contributing](../contributing/workspace-layout.md)), each feature owns its own raw shape and falls through to its `Default` impl when its section is absent. Adding a new feature's section is independent of existing ones.
