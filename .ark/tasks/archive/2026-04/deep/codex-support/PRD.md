# `codex-support` PRD

---

[**What**]

Add OpenAI Codex CLI as a first-class Ark platform alongside Claude Code. Refactor `init` / `upgrade` / `unload` / `load` / `remove` around a small platform registry so Claude and Codex are driven by the same code path. Establish parity as a standing invariant: every shipped Claude feature gets a Codex twin in the same release.

[**Why**]

Ark is currently Claude-only. Users on Codex have no integration path. Beyond reach, this is also an architectural smell: `init.rs` and `upgrade.rs` hard-code `[ARK_TEMPLATES, CLAUDE_TEMPLATES]` two-element loops, and the SessionStart hook surface in `io/fs.rs` is named for Claude's settings.json shape. A generic "platform" abstraction with two implementors today is the right time to introduce the seam — adding a third platform later (Cursor, OpenCode) becomes a registry entry, not a refactor.

The product stance is parity, not best-effort: the user has committed that future features land on both platforms concurrently. This forces a clean abstraction now rather than letting drift accumulate.

[**Outcome**]

Observable success criteria, partitioned by surface:

**`ark init`** (interactive)
- First run with no flags shows a TTY prompt: "Install for which platforms? [x] Claude Code  [x] Codex" — both checked by default.
- `--claude` / `--codex` (and `--no-claude` / `--no-codex`) skip the prompt for scripted installs.
- Non-TTY stdin without explicit flags installs both (matches `ConflictPolicy::Interactive` non-TTY → safe-default precedent in upgrade C-7).
- Installing both platforms produces:
  - `.claude/commands/ark/{quick,design,archive}.md` (unchanged)
  - `.codex/prompts/ark-{quick,design,archive}.md` (new)
  - `.codex/config.toml` containing `project_doc_fallback_filenames = ["AGENTS.md"]`
  - `.codex/hooks.json` with the SessionStart entry running `ark context --scope session --format json`
  - `AGENTS.md` managed block via `update_managed_block` (file created if absent), parallel to today's `CLAUDE.md` block

**`ark upgrade`**
- Codex hook entry in `.codex/hooks.json` is re-applied on every upgrade, surgically (sibling user hook entries preserved).
- `.codex/config.toml` is re-applied unconditionally (matches the unhash-tracked precedent for `.claude/settings.json`).
- `AGENTS.md` managed block is re-applied via `update_managed_block`, parallel to `CLAUDE.md`.
- Codex prompt files are hash-tracked the same as Claude command files.

**`ark unload` / `ark load`**
- Snapshot captures both Claude *and* Codex hook entries via `Snapshot::hook_bodies`. The Codex entry round-trips losslessly: a project unloaded with both platforms installed and reloaded ends up byte-identical (modulo timestamps).
- Sibling user entries in either `.claude/settings.json` or `.codex/hooks.json` survive unload→load untouched.

**`ark remove`**
- Removes both `.claude/commands/ark/` and `.codex/prompts/` entries, plus the SessionStart hook entries from both platforms' hook files.
- Sibling user hook entries in both files are preserved.
- Both `CLAUDE.md` and `AGENTS.md` managed blocks are removed.

**Authoring + parity invariant**
- Codex prompt bodies are mechanical translations of the Claude command bodies: same prose, YAML frontmatter dropped (Codex prompts don't support it), `:` → `-` in any `/ark:foo` references.
- A test asserts `templates/claude/commands/ark/<name>.md` exists iff `templates/codex/prompts/ark-<name>.md` exists. Adding a new slash command without a Codex twin is a build-time failure.

**Tests**
- Existing init / upgrade / unload / load / remove tests are forked: each Claude-only assertion gets a Codex sibling.
- Round-trip test: install both platforms, unload, load, assert filesystem byte-identical (excluding timestamps).
- Platform-flag tests: `--no-codex` produces only Claude artifacts; `--no-claude` produces only Codex artifacts; neither (via prompt declined) produces neither.

**Non-goals (deferred)**
- Codex `.codex/agents/*.toml` custom subagents (Trellis ships them; Ark has no Claude equivalent today).
- Codex `.codex/skills/*/SKILL.md` auto-routed skills (different invocation model from prompts).
- Cursor, OpenCode, Gemini, etc. (registry must leave room, but no third platform ships here).
- Per-prompt user customization (which prompts to install/skip).
- Single-source-of-truth templating with includes — Trellis confirmed parallel template trees with manual sync is the right call.

[**Related Specs**]

- `specs/features/ark-context/SPEC.md` — defines `update_settings_hook` / `remove_settings_hook` / `Snapshot::hook_bodies` / `ARK_CONTEXT_HOOK_COMMAND`. Codex needs the same surgical-hook helper machinery, generalized to any hook file. The settings-hook helper today is hard-coded to a Claude-shaped JSON path; this task either parameterizes it or introduces a parallel Codex helper.
- `specs/features/ark-upgrade/SPEC.md` — defines `Manifest`, `hashes`, conflict policy, and the C-8 invariant that `CLAUDE.md` block + Claude SessionStart hook are re-applied unconditionally. Codex hook + config.toml + AGENTS.md block extend C-8 symmetrically. Manifest schema does not need to change — Codex files just appear as additional rows in `manifest.files` and `manifest.hashes`.
- `specs/features/ark-agent-namespace/SPEC.md` — defines slash-command structure (`/ark:quick`, `/ark:design`, `/ark:archive`) shipped under `templates/claude/commands/ark/`. Codex prompts are siblings under `templates/codex/prompts/` with mechanically translated bodies.
