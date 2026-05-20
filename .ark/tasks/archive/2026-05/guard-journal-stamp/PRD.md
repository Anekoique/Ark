# `guard-journal-stamp` PRD

---

[**What**]

Add a CLI-side guard to `ark agent task commit` that detects when the agent failed to append a fresh `## Session N: <title>` heading to the workspace journal before invoking the command, and refuses the commit with a clear actionable error. The guard catches the contract violation observed in commit `fcfd341` (rfc001-arkos), where the absence of a new heading caused `stamp_task` to inject metadata under the previous session's heading and produce a duplicate personal-index row.

[**Why**]

The workspace-journal stamping flow has a fragile contract: the slash command (`templates/{claude/commands/ark/commit.md, codex/skills/ark-commit/SKILL.md, opencode/commands/ark/commit.md}`) tells the agent to append a `## Session N: <title>` heading and a body block before invoking `ark agent task commit`. The CLI then injects auto-fields (Date, Slug, Branch, Base Branch, Start Head, Closing Commit, Git Commits table) beneath the last `## Session ...` heading. When the agent forgets the heading, the CLI silently injects beneath the *previous* session's heading, producing two metadata blocks under one heading, a duplicated personal-index row (off-by-one in `scan_session_count`), and the previous session's title attributed to the current task.

This is not a regression — the fragility has shipped since `6a796a1` (workspace feature) and was never enforced. Investigation in `.ark/tasks/rfc001-arkos/research/journal-write-bug.md` (commit `cd50a33`) identifies the root cause as a contract-violation-the-CLI-does-not-detect. The slash-command prompts already describe what the agent must write; the gap is enforcement. Adding the guard:

1. **Prevents silent journal corruption.** No more invisible duplicate-row/wrong-title damage at commit time.
2. **Feeds back to the agent.** A descriptive error tells the agent exactly what to add (`## Session N: <title>` plus Summary and Main Changes) so it can fix and retry without needing a human to diagnose.
3. **Matches existing enforcement shapes.** Sibling `task commit` failure modes (`NothingStaged`, `VerifyIncomplete`, `CommitMessageRequired`) already use hard-error-with-actionable-message; this becomes a fourth peer.

The slash-command templates already state the contract; the CLI gains the enforcement.

[**Outcome**]

- `task commit` aborts with a new `Error::JournalEntryMissing` (or equivalently-named variant) when the workspace journal exists and its last `## Session N: <title>` heading is already followed by stamped metadata (auto-fields previously injected by a prior `stamp_task` call) rather than by an unstamped fresh heading.
- The error message names the journal path, the slug, and tells the agent the missing line: append `## Session N: <title>` followed by `### Summary` and `### Main Changes` to the journal, then re-run `ark agent task commit -m "<message>"`.
- The guard fires before any file mutation: a failed `task commit` invocation leaves the workspace journal and personal index untouched (no partial state).
- When the agent has correctly appended a fresh heading, the existing happy-path flow proceeds unchanged: `stamp_task` writes the metadata, the personal-index row gets the correct session number and title, the commit lands atomically.
- Test coverage:
  - Unit test in the workspace-stamp module: `stamp_task` (or whichever module hosts the guard) called against a journal whose last heading is already stamped must return the new error variant and leave the file's bytes unchanged.
  - Unit test in the same module: a journal whose last heading is fresh (unstamped) still produces the correct stamped output (regression guard on the happy path).
  - End-to-end test under `crates/ark-cli/tests/`: `ark agent task commit` invocation against a fixture journal violating the contract surfaces the new error variant; same fixture with the heading appended succeeds.
- No slash-command template changes. The prompts already describe the contract; this task adds the enforcement only.
- No changes to the empty-Git-Commits-table rendering (`stamp.rs::render_git_commits_block`); that cosmetic remains for a separate task.
- No changes to historical journal files; the rfc001-arkos data fix already landed in `cd50a33` on `docs/rfc001-arkos`.

[**Related Specs**]

- `specs/features/workspace/SPEC.md` — Workspace feature ships the journal-stamping contract this task enforces. The SPEC describes the contract advisorily; this task adds CLI enforcement. The SPEC will need a CHANGELOG entry on commit recording that the contract is now CLI-enforced (failure surfaces as `Error::JournalEntryMissing`). The CHANGELOG line is part of this task's delivery.
- `specs/features/ark-agent-namespace/SPEC.md` — `ark agent task commit` is one of the verbs in this namespace; adding a new error variant expands its observable contract. The new failure mode joins `NothingStaged`, `VerifyIncomplete`, `CommitMessageRequired` in the documented failure-mode set. No SPEC body change required; failure-mode enumeration is implementation-detail-stable.
- `rust/ERRORS.md` (project SPEC) — The new error variant must follow project error-handling conventions: structured fields (path, slug), `#[error(...)]` message that names what to do, no `unwrap`/`expect` in production code paths.
- `rust/STYLE.md` (project SPEC) — Function and module shape: guard logic is small (≤30 LOC); place it where it reads cleanly (PLAN decides between `record.rs` and `stamp.rs`); preserve immutability — the guard is a read-only check, no mutation of input arguments.
