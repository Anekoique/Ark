# `codeagent-cli-support` PRD

---

[**What**]

Add CodeAgent CLI as a 4th platform in Ark's platform registry alongside Claude Code, Codex, and OpenCode.

[**Why**]

CodeAgent CLI is a new AI coding assistant platform that uses `.cac/` as its root directory. It has a JSON-based hook surface (like Claude/Codex), markdown commands with YAML frontmatter, and markdown subagents with YAML frontmatter — all patterns that fit the existing `Platform` struct and `HookFileSpec` without architectural changes. Adding it as a registry entry + template tree lets Ark users on CodeAgent CLI get the same workflow experience.

[**Outcome**]

- `ark init --codeagent` scaffolds `.cac/commands/ark/*.md`, `.cac/agents/ark-*.md`, installs a `SessionStart` hook in `.cac/settings.json`, and writes an `AGENTS.md` managed block.
- `ark unload` / `ark load` round-trips CodeAgent CLI artifacts losslessly.
- `ark remove` cleans up all CodeAgent CLI artifacts.
- `ark upgrade` refreshes CodeAgent CLI templates alongside other platforms.
- `ark context --scope session` reports CodeAgent CLI as an installed platform.
- All existing tests pass; new tests cover CodeAgent CLI platform shape, hook entry, command parity, and agent parity.

[**Related Specs**]

- `specs/features/codex-support/SPEC.md` — CodeAgent CLI mirrors Codex's hook pattern (seconds timeout, `settings.json` shape). The CHANGELOG in that SPEC will need an entry noting the 4th platform.
- `specs/features/opencode-support/SPEC.md` — CodeAgent CLI shares `AGENTS.md` as the managed-block target (manifest dedupes on `(file, marker)`).
- `specs/features/subagent-support/SPEC.md` — CodeAgent CLI ships three agents (`ark-researcher`, `ark-reviewer`, `ark-verifier`) as YAML-frontmatter markdown at `.cac/agents/`, same format as Claude's agents.

[**SPEC Path**]

codeagent-cli-support
