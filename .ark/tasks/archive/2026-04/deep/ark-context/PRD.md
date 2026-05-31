# `ark context` PRD

---

[**What**]
Add a top-level `ark context` command that emits a structured snapshot of git + `.ark/` workflow state, with a `--scope {session|phase}` × `--for {design|plan|review|execute|verify}` filter matrix and JSON/text output modes, plus a rendered `SessionStart` hook that auto-invokes it at the start of every Claude Code session.

[**Why**]

Every slash command under `templates/claude/commands/ark/` currently reconstructs project state by issuing ad-hoc `git status` / `ls .ark/tasks/` / `cat` calls from its prompt. That's non-deterministic, token-heavy, and duplicates logic across command files. A single typed command collapses those calls into one stable-schema JSON payload, which:

- Agents parse cheaply instead of re-deriving from raw tool output every turn.
- The harness can auto-inject via a `SessionStart` hook so the agent wakes up oriented.
- Becomes the foundation for Phase 1's `ark task list/show/current` (projections over the same engine) and Phase 2's other hooks.

This is the highest-leverage Phase 1 roadmap item (`docs/ROADMAP.md:71`) and unblocks the hook-rendering path for Phase 2 (`docs/ROADMAP.md:81`).

[**Outcome**]

- `ark context` exists as a top-level public subcommand (visible in `ark --help`, semver-covered).
- `ark context --scope session [--format {json|text}]` prints a session-bootstrap snapshot: git state, active-tasks list (flat), project specs index, feature specs index, recent archive summary.
- `ark context --scope phase --for {design|plan|review|execute|verify} [--format {json|text}]` prints a phase-scoped snapshot: current task + the subset of state that phase's slash command needs.
- JSON output carries `"schema": 1` and a deterministic, documented field shape; text output is a human-readable rendering of the same data.
- Payload contains **paths and summaries only** — no file bodies inlined; callers read files they need.
- A `SessionStart` hook is rendered into `.claude/settings.json` at `ark init` / `ark load` time and invokes `ark context --scope session --format json`. Hook failure is non-fatal.
- All five shipped slash commands (`/ark:quick`, `/ark:design`, `/ark:archive`, and any future `/ark:plan`/`/ark:review` if present) are updated to call `ark context --scope phase --for <phase>` at their entry points instead of shelling out to raw git/ls.
- Round-trip tested: `ark init` renders the hook; `ark unload` captures it; `ark load` restores it; `ark remove` removes it. `ark upgrade` refreshes the command's embedded templates without clobbering user edits.
- `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` all green.
- End-to-end smoke: `ark context --scope session --format json` on this repo returns parseable JSON that a downstream `jq` pipeline can extract `currentTask.slug`, `git.branch`, `tasks.active[].slug` from.

[**Related Specs**]

- `.ark/specs/features/ark-agent-namespace/SPEC.md` — establishes the `Display`-summary pattern for `ark agent` subcommands. `ark context` is a **top-level** (non-`agent`) command on a separate stability tier (semver + versioned JSON schema), so its output contract is `--format json` (stable schema) and `--format text` (human-readable). The spec's no-ad-hoc-println constraint still applies in spirit: text output routes through a `Display`-impl summary type; JSON output routes through a single `serde_json::to_writer_pretty` call.
- `.ark/specs/features/ark-upgrade/SPEC.md` — `ark upgrade` must refresh `templates/claude/settings.json` (the file that now carries the `SessionStart` hook block) as a hash-tracked template file. No new interaction; the existing upgrade machinery handles it because the settings file is added to the embedded template tree. Verify via round-trip test that a user-edited `settings.json` is preserved per existing conflict-policy semantics.
- `.ark/specs/project/INDEX.md` — currently empty; no project-level conventions to satisfy beyond `AGENTS.md` code conventions (`PathExt`, `Layout`, `Display`-summary, managed blocks via `io::fs`).
