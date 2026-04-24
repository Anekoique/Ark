# `ark-upgrade` PRD

---

[**What**]
Add an `ark upgrade` subcommand that refreshes an initialized project's embedded templates (under `.ark/templates/`, `.ark/workflow.md`, `.claude/commands/ark/`, and the `CLAUDE.md` managed block) to match the installed CLI version, using content hash tracking to tell "user hasn't touched this file" from "user modified this file" and prompting interactively when the latter conflicts with a template change.

[**Why**]
Today, a project initialized with an older Ark gets stuck on that version's templates and slash commands. Users who `npm i -g` or rebuild the CLI have a newer binary, but no way to pull the updated workflow/slash-command/template text into their project without re-running `ark init --force` (which blows away their edits). `ark upgrade` closes that gap: re-apply new template content where the user hasn't modified anything, and give the user an explicit choice when their edits conflict with a template change. This unlocks the ability to ship improvements to the shipped workflow without breaking existing installs.

Migrations (structural renames/deletes between versions) are deliberately out of scope for this task — to be added once we have a first real rename.

[**Outcome**]
- `ark upgrade` is a top-level subcommand visible in `ark --help`.
- In a v0.1.1-initialized project, running the v0.1.2 CLI's `ark upgrade` after a template change:
  - Updates the managed `CLAUDE.md` block contents.
  - Rewrites template/workflow/slash-command files the user has not modified (hash match) to the new content.
  - For each template file the user has modified, prompts the user: overwrite / skip / write as `.new`. With `--force` / `--skip-modified` / `--create-new` flags, chooses non-interactively.
  - Prints a `Display` summary of counts (updated, unchanged, user-modified, prompted:overwrite|skip|new, added, removed-from-template).
  - Stores a fresh hash for every file it writes so the next `ark upgrade` can distinguish modifications again.
- Hashes and the CLI version are carried inside the existing installation manifest (`.ark/.installed.json`) — no sidecar files. `Manifest.version` is read/written as the project's last-applied CLI version; `Manifest.hashes` is a new field keyed by project-relative path. Existing v0.1.1 manifests deserialize cleanly (empty `hashes` → treated as pre-hash install).
- `ark init` on a fresh project records a hash for every file it writes, so the first `ark upgrade` has a reliable baseline.
- `ark upgrade` on an existing-but-unhashed project (pre-upgrade Ark) backfills hashes by comparing current content to the embedded template — exact match refreshes the hash and updates the file; mismatch goes through the conflict prompt.
- `ark upgrade` refuses with a named error when `.ark/.installed.json` is not present (not initialized) or when the CLI version is older than the project's recorded version (downgrade refused unless `--allow-downgrade`).
- `ark unload` and `ark remove` continue to work unchanged — the manifest is already one of the artifacts they handle; extending it with hashes requires no plumbing changes elsewhere.
- The only file upgrade never touches (regardless of what any list says) is `.ark/.installed.json` itself. Active tasks under `.ark/tasks/` and user-authored files under `.ark/specs/features/<slug>/` or `.ark/specs/project/<name>/` are untouched because they are not in either the embedded template set or the manifest's file list; no explicit prefix filter is needed.
- All filesystem access goes through `io::PathExt`; all path composition goes through `layout::Layout`; errors are named `thiserror` variants.

[**Related Specs**]

- `.ark/specs/features/ark-agent-namespace/SPEC.md` — upgrade must not clobber files under `.ark/tasks/` (active task state, including `.current`) or user-authored SPECs under `.ark/specs/features/<slug>/` and `.ark/specs/project/<name>/`. This falls out of the design: upgrade only acts on files that are either in the embedded template set OR in `manifest.files`, and those user-authored paths are in neither. The three shipped `INDEX.md` files under `.ark/specs/` ARE templates and remain upgradable.
