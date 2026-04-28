# `opencode-support` VERIFY `01`

> Status: Closed
> Feature: `opencode-support`
> Owner: Verifier (Opus 4.7)
> Target Task: `opencode-support`
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

The shipped code delivers the OpenCode integration as 02_PLAN promised. Workspace tests are green (231 ark-core + 5 ark-cli + the rest), the three platform trees materialize on `init --claude --codex --opencode`, the AGENTS.md block is recorded once across Codex+OpenCode (verified by both `shared_agents_block_deduped_when_both_platforms_apply` and the round-trip on disk), and the registry / layout / templates additions are surgical: zero changes to `Snapshot`, `HookFileSpec`, `apply_managed_state`'s body, or any command body. The two corrections recorded inline in 02_PLAN's `## Spec` (G-2 `dest_dir = ".opencode/commands"`, G-3 negative-wins flag semantics) are present in the spec body and accurately describe the shipped code. Three non-blocking findings remain, all bounded to the plugin file: a SPEC drift on the stderr-notice scope (TR-2 says "first failure per **session**", code is "first failure per **process**"), an over-cap on the plugin's line count (89 vs G-9's "≤80"), and a test-depth gap — `buildEnvelopePrefix` and `shouldInject` are exported as the SPEC's "named for unit-testability" helpers but no unit test imports or exercises them. None block ship; they're code-quality follow-ups.



## Findings

### V-001 `Plugin stderr-notice scope is process-wide, not per-session`

- Severity: MEDIUM
- Scope: SPEC Drift
- Location: `templates/opencode/plugins/ark-context.ts:22, 32-40`
- Problem:
  G-9 / C-7 / TR-2 specify "On the first swallowed failure **per session** (per TR-2), the plugin additionally writes a single-line stderr note... Subsequent failures **in the same session** are silent on stderr (still logged)." The shipped plugin uses a module-level boolean:
  ```ts
  let stderrNoticeShown = false
  // ...
  if (!stderrNoticeShown) {
    stderrNoticeShown = true
    process.stderr.write("ark-context: skipped context injection ...\n")
  }
  ```
  This fires the stderr note exactly once **per opencode process** (which spans many sessions), not once **per session**. A user who hits a transient `ark` failure in session A will not see a stderr note in session B even if it fails too — the SPEC says they should.
- Why it matters:
  The drift is small in practice (one opencode process typically equals one user-visible terminal), but it diverges from the Spec section that gets promoted verbatim to `specs/features/opencode-support/SPEC.md` on archive. Either the plugin should track stderr notices in a `Set<string>` keyed by `sessionID` (matching the existing `processedSessions` shape), or the SPEC's TR-2 / G-9 language should be amended to "per process" before archive. Letting the SPEC and code disagree silently degrades the SPEC's value as the source of truth.
- Expected:
  Follow-up task. Either (a) change `stderrNoticeShown: boolean` to `stderrNoticeShown: Set<string>` keyed by sessionID and have `warn(client, message, sessionID)` check `!stderrNoticeShown.has(sessionID)`, or (b) edit 02_PLAN's G-9 / C-7 / T-2 wording to "first swallowed failure per process" before archive extracts the SPEC. (a) is the lower-friction path because it matches the SPEC's intent; the existing `processedSessions: Set<string>` already establishes the per-session pattern.



### V-002 `Plugin file exceeds G-9 size cap of 80 lines`

- Severity: LOW
- Scope: SPEC Drift
- Location: `templates/opencode/plugins/ark-context.ts:1-89`
- Problem:
  G-9 names "Size: ≤80 lines including license/header comment." `wc -l` reports 89 lines on the shipped file. The overage is small (9 lines) and the file is still tight, but the SPEC is a hard cap and the verifier-of-reviewers role flags it because the same Spec section is the future feature SPEC.
- Why it matters:
  Same archive-promotion concern as V-001. The cap is a guardrail against the plugin growing into a meaningful TS module that would need its own tests / package.json / build step (NG-6 / G-15 invariants). 89 lines is well within the spirit of the cap, but the literal number is over.
- Expected:
  Follow-up task. Either (a) tighten the plugin (collapse the type annotations on lines 32 and 67 — they expand to multi-line wrap-arounds; the `idx` lookup on lines 84-87 can be one expression), or (b) revise 02_PLAN G-9 to "≤90 lines including license/header comment" before archive. (b) is honest about what landed; (a) preserves the original constraint.



### V-003 `Pure helpers exported but no unit test imports them`

- Severity: LOW
- Scope: Quality
- Location: `templates/opencode/plugins/ark-context.ts:24-30` (helpers); no test file
- Problem:
  G-9 names `buildEnvelopePrefix` and `shouldInject` as "Pure helpers (named for unit-testability)... exported (`export function ...`) for test harnesses to import." The functions are exported as required, and PRD outcome #5 references "a unit test that exercises the plugin's pure-function helpers." However, no Rust-side or TS-side test imports either helper. V-F-3 in 02_PLAN's Validation matrix names plugin runtime as "documented; Phase 5 #22 manual" — accepted at plan time — so this is consistent with the validation design, but the PRD outcome explicitly named the unit test as a success criterion.
- Why it matters:
  The discrepancy is between the PRD outcome (#5) and 02_PLAN V-F-3, not between PLAN and code. The PLAN takes precedence per the verify scope, so this is at most a LOW finding. But the helpers being live exports with no consumer turns them into dead-code surface area — future maintainers will not know they're load-bearing, and a refactor could rename or inline them silently.
- Expected:
  Follow-up task. Add a tiny TS unit test (e.g. `templates/opencode/plugins/ark-context.test.ts` driven by `bun test` and gated behind a developer-runs-locally Phase 5 step parallel to the existing `bun build` syntax check), or document explicitly in the plugin head comment that the exports exist as a stable contract for future test harnesses. Alternatively, drop the `export` keyword on the helpers since no consumer exists today.



## Follow-ups

- FU-001 : `opencode-plugin-stderr-scope` — Reconcile V-001: either change `stderrNoticeShown` to a `Set<string>` keyed by sessionID (matching SPEC's per-session intent) or update 02_PLAN G-9/C-7/TR-2 wording to "per process" before SPEC extraction.
- FU-002 : `opencode-plugin-line-cap` — Reconcile V-002: either tighten the plugin file to ≤80 lines or revise 02_PLAN G-9's size cap to match shipped reality (89) before SPEC extraction.
- FU-003 : `opencode-plugin-helper-tests` — Reconcile V-003: either add a Bun-driven unit test that imports `buildEnvelopePrefix` / `shouldInject` (covering PRD outcome #5's explicit "unit test that exercises the plugin's pure-function helpers" claim) or drop the `export` keyword on the helpers and document the stability contract in the plugin head comment.
