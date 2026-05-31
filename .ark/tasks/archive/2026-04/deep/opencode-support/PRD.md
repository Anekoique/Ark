# `opencode-support` PRD

---

[**What**]

Add OpenCode as a third platform in the Ark platform registry, alongside `claude-code` and `codex`. `ark init` accepts `--opencode` / `--no-opencode` flags; selected projects get `.opencode/commands/ark/{quick,design,archive}.md` slash commands, an `AGENTS.md` managed block (shared with Codex), and a TypeScript SessionStart-equivalent plugin that injects `ark context` output into the user's first message.

[**Why**]

The Codex SPEC explicitly carved space for a third platform (codex-support NG-3, G-3). OpenCode has a growing community of terminal-AI users on the same ergonomic tier as Claude Code and Codex, and Ark's value proposition — workflow-shaped slash commands plus auto-loaded session context — translates one-to-one onto OpenCode's command + plugin surfaces. Adding it now also exercises the registry abstraction (`PLATFORMS` slice + `Platform` struct) under load: a third entry shakes out any Codex-only assumptions hiding in `init` / `upgrade` / `unload` / `load` / `remove`.

[**Outcome**]

Observable success when:

1. `cargo build && cargo test --workspace` is green; new and existing tests all pass.
2. `ark init --opencode` on a clean repo produces `.opencode/commands/ark/{quick,design,archive}.md`, `.opencode/plugins/ark-context.ts`, and an `AGENTS.md` `ARK` managed block. Re-running is idempotent (zero diff).
3. `ark init --opencode --codex` produces the union: both `.codex/` and `.opencode/` trees, one shared `AGENTS.md` managed block (recorded once in the manifest by the `(file, marker)` dedupe path).
4. `ark init` with no platform flags on a TTY interactively offers Claude / Codex / OpenCode (all checked by default); `--no-opencode` suppresses the OpenCode prompt and the install.
5. A project initialized with `--opencode` and run under OpenCode injects the `ark context --scope session --format json` output as a `<ark-context>...</ark-context>`-tagged prefix on the first user message of every new chat session — verified by reading the plugin source and by a unit test that exercises the plugin's pure-function helpers.
6. `ark unload` followed by `ark load` round-trips an OpenCode-installed project byte-for-byte (modulo timestamps): `.opencode/` tree restored, AGENTS.md block re-applied, plugin file re-applied, no double-write of the AGENTS.md block.
7. `ark remove` on a Codex+OpenCode project removes both `.codex/` and `.opencode/` trees, removes the AGENTS.md `ARK` managed block once, and surgically removes the Codex `SessionStart` hook entry from `.codex/hooks.json` while preserving sibling user entries.
8. `ark upgrade` on an existing Claude+Codex project (no OpenCode) leaves the OpenCode flag-set untouched: no `.opencode/` directory is created. Adding OpenCode is an explicit `ark init --opencode` (idempotent on already-installed Claude/Codex artifacts).
9. Two parity tests pin the OpenCode command surface in lockstep with Claude: `every_claude_command_has_an_opencode_command_sibling` (existence) and `opencode_command_bodies_have_opencode_frontmatter` (frontmatter shape — `description:` only, no Claude-specific `argument-hint:` field).
10. The shipped `.opencode/plugins/ark-context.ts` is pure TypeScript (no `.js` file in the embedded tree), uses OpenCode's `experimental.chat.messages.transform` hook for first-message mutation (paired with the stable `chat.message` notification hook for gate-and-store), and depends only on Node/Bun built-ins (`node:child_process`) — no `package.json`, no npm dependencies bundled. **(Corrected post-archive: the original wording said "uses no `experimental.*` hook"; that turned out to be impossible — `chat.message` cannot mutate the user message, so `experimental.chat.messages.transform` is required. The two-hook contract is documented in the promoted SPEC's G-9 and the archived 02_PLAN's G-9.)**

[**Related Specs**]

- `specs/features/codex-support/SPEC.md` — direct precedent: registry shape, `Platform` struct fields, `apply_managed_state`, `capture_hook`, hook-file plumbing, source-scan tests, parity tests. This task extends `PLATFORMS` from 2 entries to 3 and adds a `managed_block_only_already_recorded` dedupe guard so OpenCode and Codex can share `AGENTS.md`.
- `specs/features/ark-context/SPEC.md` — defines the `ARK_CONTEXT_HOOK_COMMAND` constant the plugin will shell out to, plus the `--scope session --format json` output shape (with the SessionStart envelope) the plugin parses and unwraps.
- `specs/features/ark-upgrade/SPEC.md` — `upgrade` semantics for not-hash-tracked, re-applied-every-run files (the OpenCode plugin file mirrors Codex `config.toml`'s treatment).
- `specs/features/ark-agent-namespace/SPEC.md` — no direct interaction; the `ark agent` surface is unchanged by this task.
