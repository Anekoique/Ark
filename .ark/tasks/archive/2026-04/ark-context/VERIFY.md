# `ark-context` VERIFY

> Status: Closed
> Feature: `ark-context`
> Owner: Verifier
> Target Task: `ark-context`
> Verify Scope:
>
> - Plan Fidelity        — does the code deliver what the final PLAN promised?
> - Functional Correctness — does it work under the Validation matrix?
> - Code Quality         — readability, naming, error handling, test depth
> - Organization         — module boundaries, file placement, cohesion
> - Abstraction          — appropriate abstractions; no premature, no leaky
> - SPEC Drift           — does PLAN's Spec section still match the shipped code?

---

## Verdict

- Decision: Approved with Follow-ups
- Blocking Issues: 0
- Non-Blocking Issues: 3

## Summary

The implementation delivers what `02_PLAN.md` (carrying forward `01_PLAN.md`) promised. All 12 Goals (G-1 through G-12) and 26 Constraints (C-1 through C-26 + C-27 through C-29) are satisfied as specified, with one deliberate scope adjustment (V-IT-15 negative case became a positive case — see V-001 below). 191 unit/integration tests pass; the CLI smoke test (init → context → unload → load → remove) round-trips cleanly with the SessionStart hook captured and restored. Build, fmt, clippy, and test gates are all green. Three non-blocking follow-ups documented below: V-IT-15 plan/reality mismatch (better behavior than planned, plan should be amended), Phase 4 documentation is partial (READMAP marker added but the SPEC body amendment is in the live SPEC only — `templates/ark/specs/...` is unchanged because that file isn't templated), and the slash-command G-10 row remained "manual review" without a per-file mechanical check (carried forward from 01's Unresolved §1, no regression).

## Findings

### V-001 V-IT-15 documents a non-feature that is in fact a feature

- Severity: LOW
- Scope: Plan Fidelity / SPEC Drift
- Location: `crates/ark-core/src/commands/load.rs::tests::roundtrip_preserves_user_pretooluse_sibling`; conflicts with the plan's C-18 wording "User-edits to *unrelated* hook entries are NOT captured by `hook_bodies` and do NOT survive a round-trip".
- Problem:
  Implementation surprise: because `unload` calls `remove_settings_hook` (a precise edit) rather than wholesale-deleting `.claude/settings.json`, user-added sibling hook entries (e.g. `PreToolUse`) survive an unload → load round-trip on disk even though they are NOT captured into `Snapshot::hook_bodies`. The test asserts the actual behavior. Plan C-18's wording is now slightly misleading: the part about hook_bodies-only capture is correct, but the implication that siblings don't survive is wrong because the unload path is non-destructive at the file level (only removes the Ark entry).
- Why it matters:
  The actual behavior is strictly better than planned (less destructive). But the SPEC will be wrong if a future refactor re-implements unload as a wholesale-delete-then-snapshot, which would regress to plan-spec behavior.
- Expected:
  Follow-up task: amend the eventual feature SPEC's C-18 to match shipped behavior — "Snapshot::hook_bodies only captures Ark's entry; unrelated sibling entries persist in `.claude/settings.json` because `unload` performs a precise removal, not a file-level delete." Not blocking — the actual behavior is correct.

### V-002 `templates/ark/specs/features/ark-upgrade/SPEC.md` is not the embedded path

- Severity: LOW
- Scope: SPEC Drift
- Location: `templates/ark/` does not contain `specs/features/ark-upgrade/SPEC.md` (only `specs/INDEX.md`); the live `.ark/specs/features/ark-upgrade/SPEC.md` (which I amended) is repo-only state, not a host-distributed template.
- Problem:
  Phase 4 step 1(a) said "Amend `specs/features/ark-upgrade/SPEC.md` C-8". I edited the repo's live SPEC but there is no template counterpart for end users on `init` — feature SPECs are produced by `ark agent task archive --tier deep`, which extracts each task's PLAN's Spec section. The `templates/ark/specs/INDEX.md` does not seed individual feature SPECs (that's the design). So the amendment lives only in this repo's `.ark/specs/`, and any host that ran `ark init` after my change does not automatically get the amended `ark-upgrade/SPEC.md` (host's `ark-upgrade` SPEC is a fixed, archived artifact from the `ark-upgrade` task's archive moment).
- Why it matters:
  This is correct behavior under Ark's own model — feature SPECs are per-project archive products, not shipped templates — but means R-105's intent ("future readers of `ark-upgrade/SPEC.md` should see the C-8 extension") only applies to readers of *this repo's* SPEC. End users who archived their own `ark-upgrade` task with the older C-8 wording won't see the extension.
- Expected:
  Follow-up: document this in the workflow doc (templates/ark/workflow.md §5 — Specs) — "feature SPECs are archive products; cross-project SPEC amendments do not flow through `ark upgrade`." Already implicit in the architecture; making it explicit avoids future confusion. Non-blocking.

### V-003 G-10 slash-command updates lack a mechanical assertion

- Severity: LOW
- Scope: Quality / Plan Fidelity
- Location: `templates/claude/commands/ark/{quick,design,archive}.md`
- Problem:
  G-10's acceptance row in 02_PLAN was "Template content review + V-IT-7 (template content)". V-IT-7 (in `init.rs::init_writes_session_start_hook`) verifies that the hook is installed; it does NOT verify that the slash command files contain the `ark context --scope phase --for <phase>` recipe. There is no test asserting that `templates/claude/commands/ark/design.md` mentions `ark context`. This was carried forward as Unresolved §1 from 01_PLAN.
- Why it matters:
  A future template refactor could silently strip the `ark context` recipe from a slash command and CI would not catch it. For now this is honor-system, same as before.
- Expected:
  Follow-up task: add a templates-content unit test that asserts each `templates/claude/commands/ark/*.md` body contains the literal `ark context --scope phase --for`. Cheap, defensive, mechanical.

## Follow-ups

- FU-001 : `ark-context-spec-amend` — Amend the eventual extracted ark-context SPEC's C-18 to reflect that user-added sibling hook entries do persist on disk because `unload` is a surgical edit. Reword for accuracy.
- FU-002 : `ark-spec-distribution-doc` — Document in `templates/ark/workflow.md` §5 that feature SPEC amendments are per-project (archive products), not template-distributed.
- FU-003 : `ark-slash-command-content-test` — Add a templates-content unit test asserting each slash command references `ark context --scope phase --for <phase>`.
