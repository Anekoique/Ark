# `project-spec` REVIEW `01`

> Status: Closed
> Feature: `project-spec`
> Iteration: `01`
> Owner: Reviewer (independent `code-reviewer` agent)
> Target Plan: `01_PLAN.md`
> Review Scope:
>
> - Prior-finding verification (R-001..R-007 from REVIEW 00)
> - Regression check
> - Targeted questions on V-IT-6, E-15, NG-1

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: 1 (R-101 — HIGH)
- Non-Blocking Issues: 2 (R-102, R-103)

## Summary

All seven prior findings (R-001..R-007) are properly addressed. One HIGH regression introduced by the iteration: V-F-4's instruction to use `git commit --amend` assumes commit 1 is always still HEAD when a SPEC amendment is needed; that assumption fails if commit 2 has already been created by the time Phase 6 reveals a conflict. Two non-blocking issues: E-15's exception clause is logically self-defeating (a bare variant has no field to interpolate, so its Display can never contextualize), and NG-1's new "reference document" category lacks a definition for future authors.

## Prior-Finding Verification

| Finding | Severity | Status         | Evidence                                                                      |
| ------- | -------- | -------------- | ----------------------------------------------------------------------------- |
| R-001   | HIGH     | Closed         | CN-2 + V-UT-1 reframed as section-scoped counts.                              |
| R-002   | HIGH     | Closed         | E-15 added in Phase 4; G-6 calls it out explicitly.                           |
| R-003   | MEDIUM   | Partially open | CN-5 anchored by mapping table; CN-4's V-F-4 has new procedure issue (R-101). |
| R-004   | MEDIUM   | Closed         | F-6 + Phase-6g pre/post `--help` diff + V-IT-6 verification.                  |
| R-005   | MEDIUM   | Closed         | V-UT-2 requires recognized source token.                                      |
| R-006   | LOW      | Closed         | CN-7 + V-F-3 exempt `// CITATION:` and doc-comment citations.                 |
| R-007   | LOW      | Closed         | G-7 / CN-8 / V-E-1 / Phase 6g all incorporate 400-LOC soft target.            |

## Findings

### R-101 — V-F-4 assumes `git commit --amend` is always viable

- Severity: HIGH
- Section: V-F-4; CN-4; F-3; Phase 7
- Problem:
  V-F-4 instructs the executor to land late SPEC amendments in commit 1 "via `git commit --amend`." But commit 1 is created in Phase 5, and Phase 6 (the refactor) follows. If Phase 6 reveals a SPEC conflict after commit 2 has been created, commit 1 is no longer HEAD; `--amend` silently rewrites the wrong commit. F-3 already permits "relax via `[**Exceptions**]`" without amending — the V-F-4 wording is structurally inconsistent with F-3.
- Why it matters:
  Procedural correctness. If the executor follows V-F-4 literally on a non-HEAD commit, they corrupt history. Worse, if commit 1 is pushed, `--amend` requires a force-push, which is forbidden by the user's global git rules.
- Recommendation:
  Replace V-F-4's mechanism with: "If a SPEC amendment is required during Phase 6, the amendment lands in a commit that precedes commit 2. Acceptable mechanisms: (a) amend commit 1 only if it is the HEAD and unpushed; (b) create an adjacent fixup commit between commit 1 and commit 2 that touches only `.ark/specs/project/`. In either case, `git log --oneline -- .ark/specs/project/rust/` must show the change in a commit that precedes commit 2." This keeps F-3's path open and never rewrites a non-HEAD commit.

### R-102 — E-15 permitted-exception clause is underspecified

- Severity: MEDIUM
- Section: Phase 4 — E-15 outline
- Problem:
  E-15 reads: "A bare `#[from] source: io::Error` variant is permitted only when the variant's `Display` template includes a contextualizing field." A Display template can only interpolate fields from the variant itself. A bare variant has no fields beyond `source`. The exception is self-defeating — there is no path to satisfy it.
- Why it matters:
  Vague rule wording is a vector for future drift.
- Recommendation:
  Either:
  - **(a)** Reword to: "A bare `#[from] source: io::Error` variant is permitted only in single-purpose error enums whose enum-name itself supplies the context (e.g. `enum ConfigLoadError { #[from] Io(io::Error) }`)."
  - **(b)** Delete the exception entirely. The existing `ark-core` `Error` enum has no bare-`#[from]` variants — there is nothing to preserve, and dropping the exception simplifies the rule.
  Reviewer slight preference for (b).

### R-103 — "Reference document" category undefined for future authors

- Severity: LOW
- Section: NG-1; LAYOUT.md drafting
- Problem:
  NG-1 distinguishes "reference document" (permitted under `specs/project/`) from "template" (reserved for `.ark/templates/`) but provides no criterion. A future author adding `specs/project/TESTING-PATTERNS.md` won't know which side of the line it falls on.
- Why it matters:
  Forward-looking; not blocking this task. But the boundary will be litigated by the next agent.
- Recommendation:
  In `LAYOUT.md` itself, add one sentence: "A reference document describes a convention or format. A template contains placeholder sections intended to be copied and filled in." That single line makes the boundary durable.

## Trade-off Advice

No new trade-offs arise from the iteration-01 revisions. T-2 remains the only open trade-off; it stays at default.
