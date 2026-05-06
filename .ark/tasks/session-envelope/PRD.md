# `session-envelope` PRD

---

[**What**]
Extend `ark context --scope session --format json` envelope to add `suppressOutput: true` and a one-line user-visible `systemMessage` summary, plus a 9,500-char trim guard on `additionalContext`.

[**Why**]
Today the SessionStart hook emits `{hookSpecificOutput: {hookEventName, additionalContext}}` only. Hosts that do not consume the envelope silently render the raw JSON in the transcript as a noisy wall (user-reported on Codex/OpenCode-shaped hosts when Ark was driven into Astervisor). Both fields are documented in Claude Code's and Codex CLI's hook contracts:
- `suppressOutput: true` keeps stdout out of transcript mode.
- `systemMessage` is the single user-visible one-liner.
- Claude documents a 10K-char cap on injected context; trimming at 9.5K preempts mid-payload truncation.

[**Outcome**]
`ark context --scope session --format json` output, on a normal Ark project, contains all four fields:
1. `hookSpecificOutput.hookEventName == "SessionStart"`
2. `hookSpecificOutput.additionalContext` (stringified `ProjectedContext`, ≤ 9,500 chars; if trimmed, the inner JSON has `"truncated": true`)
3. `suppressOutput == true`
4. `systemMessage` is a non-empty single-line summary derived from the projection (e.g. `"Ark: branch=main · current=session-envelope (quick/design) · 4 active · 10 specs"`)

Verification (recorded 2026-05-06):
- `cargo test -p ark-core context::` → 36 passed, 0 failed.
- `cargo test --workspace` → 403 passed, 0 failed.
- `cargo clippy --workspace --all-targets -- -D warnings` → no warnings.
- `cargo fmt --all -- --check` → clean.
- New tests added (all passing):
  - `context_session_json_wraps_in_session_start_envelope` extended to assert `suppressOutput == true`, `systemMessage` starts with `Ark: ` and is single-line, contains `branch=`, and `truncated` is absent on empty state.
  - `summary_one_line_with_current_task_includes_slug_tier_phase` covers the current-task path of `summary_one_line` (asserts `current=demo (quick/execute)`).
  - `context_session_json_trims_oversized_payload` seeds five archive entries with 2.5 KB titles, asserts inner length ≤ `ADDITIONAL_CONTEXT_CAP`, asserts `truncated: true`, asserts archive section is dropped.
- Manual smoke 1 — fresh tempdir (`/tmp/ark-env-smoke`, after `ark init --claude`): top-level keys are `['hookSpecificOutput', 'suppressOutput', 'systemMessage']`; `suppressOutput: True`; `systemMessage: "Ark: branch=unknown · 0 active · 0 specs"`; inner first key is `schema`; inner length 427 bytes; `truncated` absent.
- Manual smoke 2 — real Ark repo: same top-level keys; `systemMessage: "Ark: branch=main · 5 active · 10 specs"`; inner length 6,740 bytes (well under cap); inner first key is `schema`; archive present; no truncation.
- Non-session scopes (`--scope phase`, `--scope record`) still emit raw projection without the envelope (existing `context_phase_json_emits_raw_projection_without_envelope` test continues to pass).

[**Related Specs**]

- `.ark/specs/features/ark-context/SPEC.md` — owner of the `ark context` envelope contract; this task extends, not redesigns, the existing `wrap_session_start_envelope` helper.
