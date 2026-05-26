# `improve-ark-upgrade` PRD

---

[**What**]

Give `ark upgrade` a user-declared per-path file strategy (`ejected` / `merged`) configured in `.ark/config.toml`, a non-mutating `--dry-run` preview, a 3-way merge for non-managed-block files, and a backup-and-restore safety net so a failed or regretted upgrade is recoverable.

[**Why**]

Today `ark upgrade` offers exactly two real outcomes per conflicting file: overwrite or skip/`.new`. A user who meaningfully edits a shipped file (a slash command, `workflow.md`) must re-resolve the same prompt on every release forever — there is no first-class "I own this now, stop managing it." The only escape hatch is `is_seed_only`, a hardcoded list the user cannot extend. Three further gaps compound this:

- **No preview.** The plan is computed and applied in one shot; a user cannot see what an upgrade *would* do before it mutates the tree. Upgrades that touch many files are effectively un-auditable until after the fact.
- **All-or-nothing conflicts.** When both Ark and the user changed a file, the only choices are lose-the-user-edit (overwrite) or lose-the-Ark-update (skip). For files without a managed block, there is no way to keep both.
- **No recovery.** `NG-2` explicitly disclaims rollback. A `--force` run that overwrites the wrong file, or a merge the user dislikes, has no undo.

The agent-infra reference project (`reference/agent-infra/`) solves the first three with a declared `managed` / `merged` / `ejected` per-path strategy in its config. Ark already has the harder half — content-hash classification and managed-block splicing — so adding a declared strategy layer is a natural, philosophy-aligned extension that keeps Ark's machine-enforced rigor while closing the upgrade-friction gap.

[**Outcome**]

- A user can list paths under `[upgrade] ejected = [...]` in `.ark/config.toml`; `ark upgrade` never classifies, prompts about, writes, or deletes those paths. Ejection supersedes every policy including `--force`.
- A user can list paths under `[upgrade] merged = [...]`; for a `merged` file that both sides changed, `ark upgrade` performs a diff3 3-way merge (base = the template bytes from the recorded version, ours = on-disk, theirs = new template). A clean merge applies silently and is counted; a conflicting merge writes the file with Git-style conflict markers and is reported as a conflict (not silently overwritten or skipped).
- `merged` applies only to files **without** an Ark managed block. Managed-block files (`CLAUDE.md`, `AGENTS.md`) keep their existing splice strategy unchanged — the user's block body is preserved, the surrounding template refreshed — and a `merged` entry naming a managed-block file is rejected with a clear error.
- The 3-way merge needs the **base** (the template bytes the user started from). The manifest records only a hash, not the bytes, so this task must establish where the base comes from; a `merged` file with no recoverable base falls back to the existing overwrite/skip/`.new` conflict path rather than producing a bogus merge.
- `ark upgrade --dry-run` prints the full planned action set (per path: add / update / overwrite / merge-clean / merge-conflict / preserve / `.new` / delete / orphan / eject-skip) and exits without writing, deleting, or touching the manifest, managed blocks, or hooks.
- A non-dry-run `ark upgrade` captures a backup of every file it is about to mutate or delete before any write; if any step fails, the backup is restored so the tree returns to its pre-upgrade state. The user can restore the most recent backup on demand after a completed-but-regretted upgrade.
- Invalid `[upgrade]` config (unknown keys, unsafe paths, a `merged` entry that is a managed-block file or also appears in `ejected`) fails fast with a clear error before any mutation.
- `.ark/config.toml` remains preserved across upgrade (already seed-only); the new `[upgrade]` section round-trips untouched.
- All four CLI policy controls (`--force`, `--skip-modified`, `--create-new`, interactive) continue to behave as before for files that are neither `ejected` nor `merged`. Existing upgrade tests pass unchanged except where behavior is intentionally extended.

[**Related Specs**]

- `specs/features/ark-upgrade/SPEC.md` — this task modifies the upgrade feature directly. The current SPEC defines hash classification (`G-2`), the overwrite/skip/`.new` conflict model (`G-3`), removal safety (`G-4`), managed-block re-application (`G-5`, `C-8`), and the seed-only exemption (`C-3`). This task extends the Goals/Constraints with the declared-strategy layer, `--dry-run`, 3-way merge, and backup/restore; `NG-2` (no rollback) is revised. The promoted `## Spec` lands as a CHANGELOG entry on this SPEC, not a new feature.
- `specs/features/worktree/SPEC.md` — the `[worktree]` section already lives in `.ark/config.toml`; the new `[upgrade]` section is a sibling. The config loader/validator must coexist with `[worktree]` parsing without disturbing it.

[**SPEC Path**]

ark-upgrade
