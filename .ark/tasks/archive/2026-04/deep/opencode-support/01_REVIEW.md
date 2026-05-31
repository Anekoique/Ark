# `opencode-support` REVIEW `01`

> Status: Open
> Feature: `opencode-support`
> Iteration: `01`
> Owner: Reviewer (Opus 4.7)
> Target Plan: `01_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: 1
- Non-Blocking Issues: 3

Per the verdict rule (no open CRITICAL → not Rejected; one open HIGH → Approved with Revisions). The Response Matrix audit confirms every prior finding (R-001 through R-011, TR-1 through TR-6) was resolved correctly: the two-hook plugin contract is pinned in G-9 and the Runtime section, `conflicts_with` is gone, `owned_dirs() -> [PathBuf; 4]` matches the existing shape, V-F-4 is dropped, all "continues codex-support X" cross-references inlined in `## Spec`, V-IT-2 has a positive sanity check, `bun build --no-bundle` replaces the fictitious `bun check`, T-6 documents the TS choice with the user's DESIGN-phase direction as rationale, V-UT-13/V-UT-14 added, and Phase 3 step 16 targets `resolve_platforms_pure`. TR-3 is correctly resolved by deleting T-3, TR-6's rejection is justified (codex skills ship only `SKILL.md` files — no JS — so the reviewer's "JS convention" argument was based on a misread that the planner correctly identified). One new HIGH issue: V-IT-2 (c)'s assertion text uses the bare substring `# /ark:<name> $ARGUMENTS`, but the actual Claude templates use `` # `/ark:<name> $ARGUMENTS` `` (backtick-quoted), so the assertion as written will fail on every command body. Three non-blocking refinements below.

## Summary

The plan is now coherent, implementable, and self-contained. The two-hook plugin contract correctly mirrors the Trellis reference (`session-start.js:350–453`); the per-session `Set<string>` + `Map<string, string>` mechanism is race-free under JS's single-threaded event loop; the experimental-hook fragility is documented in C-15 with a one-line migration path. CLI flag handling matches the existing positive-wins resolution. The `[PathBuf; 4]` extension to `owned_dirs` is a one-line additive change that requires no caller refactor. The `## Spec` section is now extractable to `specs/features/opencode-support/SPEC.md` without dangling references. The single HIGH issue is mechanical: V-IT-2 (c)'s asserted substring does not match the actual heading shape in the Claude templates, so the "positive sanity check" added per R-006 will fail on every command body unless the assertion text is corrected (or C-6's body-translation rule is changed, which would be the wrong direction). Three MEDIUM/LOW notes follow on hook-firing-order assumption (a small clarification request), V-UT-14's near-vacuity (still acceptable as a regression guard), and a minor wording nit on Phase 5 #22.

## Findings

### R-101 V-IT-2 (c) substring `# /ark:<name> $ARGUMENTS` does not match actual Claude template heading shape — assertion will fail on every command body

- Severity: HIGH
- Section: `## Validation` V-IT-2 (the (c) clause); cross-referenced by `## Spec` G-12; `## Implementation` Phase 4 step 18
- Problem:
  V-IT-2 (c) asserts:
  > "the body contains the literal `# /ark:<name> $ARGUMENTS` heading where `<name>` is the file's stem (e.g. for `quick.md` the body must contain `# /ark:quick $ARGUMENTS`)."
  The actual heading shape in `templates/claude/commands/ark/quick.md:6` (verified) is:
  ```
  # `/ark:quick $ARGUMENTS`
  ```
  with backtick-quoted slash invocation. `design.md:6` and `archive.md:6` use the same shape. C-6 mandates "keep body verbatim including `# /ark:<name> $ARGUMENTS`" — but the verbatim body has backticks. A substring search for the bare token `# /ark:quick $ARGUMENTS` against any of the three rendered command files will NOT find a match (the file contains `` # `/ark:quick $ARGUMENTS` ``, not `# /ark:quick $ARGUMENTS`). The assertion as worded fails on every command body, immediately on first run.
- Why it matters:
  R-006's resolution (Option B, the stricter sanity check) was accepted in this iteration. If the executor writes the test exactly as specified, it fails; if they "fix" it by stripping the backticks from the template bodies, that's a SPEC drift (the bodies are no longer verbatim Claude copies, violating C-6). Either way the test will not catch what it claims to catch on first commit. The Acceptance Mapping rows for G-4 / G-12 / C-6 all lean on V-IT-2; a broken V-IT-2 weakens all three.
- Recommendation:
  Two equivalent fixes — pick one and update G-12 (b), V-IT-2 (c), and C-6 in lockstep:
  (a) Match the actual shape: change V-IT-2 (c) to assert "the body contains the literal `` # `/ark:<name> $ARGUMENTS` `` heading (with surrounding backticks) where `<name>` is the file's stem." Update the inline example accordingly. This is the minimal change and keeps C-6's "verbatim" rule honest.
  (b) Loosen the assertion to a regex or trimmed-substring check: "the body contains a heading line whose trimmed contents include `/ark:<name> $ARGUMENTS` (in or out of backticks)." This is more permissive but accommodates either rendering.
  Default: (a). The verbatim rule is the source of truth.



### R-102 Hook-firing-order assumption (`chat.message` before `experimental.chat.messages.transform`) is not stated as a contract requirement

- Severity: MEDIUM
- Section: `## Spec` G-9 (Hooks used (two, complementary)); `## Runtime` "Main Flow — runtime" steps 4–5
- Problem:
  The plan's correctness hinges on `chat.message` firing **before** `experimental.chat.messages.transform` for the same user message — `chat.message` populates `pendingContext.set(sessionID, value)` and `transform` reads `pendingContext.get(sessionID)`. If the order were reversed, `transform` would find an empty map and skip; `chat.message` would then populate the map but the message has already been sent. The Trellis reference (`session-start.js:387` then `:430`) implicitly relies on the same ordering, but neither the plan nor the reference asserts it.
  Looking at the Runtime "Main Flow" section the plan says "Immediately after, opencode fires `experimental.chat.messages.transform`" — that's a description of observed behavior, not a contract. If a future opencode release reorders the hooks (e.g. fires transform before notifying via chat.message to enable mutation pipelines), the plugin silently stops injecting context.
- Why it matters:
  This is the second silent-regression risk after C-15. C-15 covers the rename/removal case; ordering reversal is a separate failure mode the spec does not name. Phase 5 #22 (the manual smoke) would catch it but only on each release.
- Recommendation:
  Add to G-9 (Hooks used) or to C-15:
  > "Ordering assumption: `chat.message` fires before `experimental.chat.messages.transform` for the same user message. If a future opencode release reverses this order, `pendingContext.get(sessionID)` returns `undefined` in transform and injection silently stops; mitigation matches C-15 (no SPEC delta; rework plugin source to populate `pendingContext` from a different hook or to mutate inline)."
  Alternatively, if the planner has a reference confirming the order from opencode's docs, cite it inline (similar to how C-6 cites the `$ARGUMENTS` doc text). Either is acceptable; documenting the assumption is the gating ask.



### R-103 V-UT-14 second clause (no `.opencode/package.json` on disk) is borderline vacuous

- Severity: LOW
- Section: `## Validation` V-UT-14
- Problem:
  V-UT-14 reads: "No `package.json` is reachable via `OPENCODE_TEMPLATES` (`OPENCODE_TEMPLATES.get_file("package.json").is_none()` and `OPENCODE_TEMPLATES.get_file("ark/package.json").is_none()`). After `apply_managed_state`, `.opencode/package.json` does not exist on disk."
  The first clause is non-vacuous — it directly checks the templates tree shape. The second clause, "after `apply_managed_state`, `.opencode/package.json` does not exist on disk," cannot be falsified by any code in this task: no `extra_files` entry writes a `package.json`, `OPENCODE_TEMPLATES` is rooted at `commands/` so no template file can extract to `package.json`, and `apply_managed_state`'s body is unchanged. The on-disk check passes vacuously.
- Why it matters:
  Vacuous tests cost nothing to run but mislead future readers about what is being validated. The first clause already guards G-15's "no `package.json` shipped" claim; the second clause is a regression guard that activates only if a future commit adds a `package.json` to `extra_files` or roots `OPENCODE_TEMPLATES` higher. That's a real (if remote) failure mode, so the test is not pure noise — but the wording could be clearer about its role.
- Recommendation:
  Either:
  (a) Keep as is, note inline: "the on-disk assertion is a regression guard for future template-tree changes; vacuous against the current implementation by design."
  (b) Drop the on-disk clause; the templates-tree clause alone is sufficient.
  Default: (a). Cheap regression guards are net-positive even when vacuous against today's code, as long as their role is named.



### R-104 Phase 5 #22 wording — "Gating step before EXECUTE → VERIFY transition" reads as describing a workflow phase change, but Phase 5 is itself the final EXECUTE step

- Severity: LOW
- Section: `## Implementation` Phase 5 #22
- Problem:
  Phase 5 #22 ends with: "**Gating step before EXECUTE → VERIFY transition.**" The workflow phase ordering is design → plan ⇄ review → execute → verify. Phase 5 is the last EXECUTE phase. Calling step #22 "the gate before EXECUTE → VERIFY" is technically right (it's the last thing the executor does inside EXECUTE before invoking `ark agent task verify`), but reads as if it's a separate phase change. A reader scanning for gates may interpret this as a fourth quasi-phase between EXECUTE and VERIFY.
- Why it matters:
  Workflow §3.3 names two real gates inside the deep tier: PLAN gate (verdict Approved → EXECUTE) and VERIFY gate (verdict Approved → user runs `/ark:archive`). Adding informal mid-phase gate language risks confusion about what the agent's `ark agent task verify` invocation actually checks. Minor — does not affect implementation, only readability.
- Recommendation:
  Rephrase to "Last EXECUTE step; must pass before `ark agent task verify` is invoked." This keeps the gating intent without naming a transition that doesn't exist in the workflow.



## Trade-off Advice

### TR-1 Plugin file location — `extra_files` vs. embedded in `OPENCODE_TEMPLATES`

- Related Plan Item: T-1
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A
- Advice:
  Accept as resolved. T-1 retains Option A (extra_files), matching prior TR-1 advice and Codex's `config.toml` precedent.
- Rationale:
  Confirmed by 00 review. No new information.
- Required Action:
  None.



### TR-2 Plugin error handling — log-and-continue with one-shot stderr

- Related Plan Item: T-2
- Topic: Robustness vs Discoverability
- Reviewer Position: Prefer Option A (with TR-2 enhancement applied)
- Advice:
  Accept as resolved. T-2 now includes the one-shot stderr discoverability note per prior TR-2 advice, written into C-7.
- Rationale:
  The plan correctly threads the enhancement through C-7 and the Failure Flow section. The "first per-session failure → stderr; subsequent silent" rule is unambiguous.
- Required Action:
  None.



### TR-3 Plugin syntax test — drop the `#[ignore]`d test

- Related Plan Item: (T-3 deleted)
- Topic: Build hygiene vs Toolchain coupling
- Reviewer Position: Prefer Option B (drop the test)
- Advice:
  Accept as resolved. T-3 is deleted; C-12 now reads as "developer-runs-locally guidance, not a test." This matches prior TR-3 default (option b).
- Rationale:
  An `#[ignore]`d test was the worst-of-both-worlds option (documentation pretending to be a check). The plan correctly substitutes a `bun build --no-bundle` instruction in the developer's manual checklist.
- Required Action:
  None.



### TR-4 Body translation — keep slash idioms verbatim

- Related Plan Item: T-4
- Topic: Compatibility vs Correctness
- Reviewer Position: Prefer Option A
- Advice:
  Accept as resolved (with the R-101 fix applied to V-IT-2).
- Rationale:
  T-4 retains Option A. C-6 cites the opencode docs verifying `$ARGUMENTS` is the right token. The Phase 5 #22 manual check stays as the ground-truth verification. Note: if R-101 is resolved via Option (a) — match the backtick shape verbatim — TR-4 stays clean. If R-101 is resolved via Option (b) — loosen the regex — TR-4 stays clean too.
- Required Action:
  None beyond R-101.



### TR-5 AGENTS.md sharing — single shared block

- Related Plan Item: T-5
- Topic: Compatibility vs Future-Proofing
- Reviewer Position: Prefer Option A
- Advice:
  Accept as resolved. T-5 retains Option A (single shared block).
- Rationale:
  No new information.
- Required Action:
  None.



### TR-6 Plugin source language — TypeScript (rejected my prior JS advice)

- Related Plan Item: T-6 (newly added)
- Topic: Convention Consistency vs Type Ergonomics
- Reviewer Position: Concede — TS is acceptable
- Advice:
  Accept the rejection. T-6's reasoning is correct: codex skills ship only `SKILL.md` files (verified: `ls templates/codex/skills/` returns three subdirectories, each containing only `SKILL.md` — no `.js` files anywhere under the codex tree). My TR-6 advice was based on the assumption that codex skills shipped JS files, which they do not. The Trellis JS reference is external precedent, not internal convention. T-6's argument that TS types serve as documentation for the SessionStart envelope and opencode hook input/output shapes is plausible at the 80-line scale.
- Rationale:
  The reviewer's prior "JS convention parity" argument relied on a factual error about the codex artifact tree. T-6 correctly identifies and rebuts it. Bun runs `.ts` natively; no transpile in the user runtime path; the migration to JS later is mechanical (rename + drop type annotations).
- Required Action:
  None. T-6 stands as written. Carry the reviewer's concession into the archived SPEC's CHANGELOG when this lands.

