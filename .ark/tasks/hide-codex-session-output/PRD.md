# `hide-codex-session-output` PRD

---

[**What**]

Make Ark's Codex `SessionStart` hook stop printing the `ark context` payload into the visible Codex session transcript.

[**Why**]

Codex currently displays the completed `SessionStart` hook warning and full hook context even though Ark emits `suppressOutput: true`; Codex parses that field but does not yet implement suppression.

[**Outcome**]

`ark init` / `load` / `upgrade` install a Codex hook whose command produces no stdout, so Codex has no hook payload to display. Existing Ark-owned Codex hooks using the previous command are replaced instead of duplicated. Claude hook behavior remains unchanged. Targeted Rust tests pass.

[**Related Specs**]

- `.ark/specs/features/codex-support/SPEC.md` — defines the Codex `.codex/hooks.json` SessionStart integration.

[**SPEC Path**]

codex-support/hide-codex-session-output
