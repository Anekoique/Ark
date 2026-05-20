# `ark-research` REVIEW `01`

> Status: Open
> Feature: `ark-research`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: 0
- Non-blocking: 4

## Summary

The revised PLAN absorbs all seven 00-REVIEW findings (R-001..R-007) and all three trade-off items (TR-1..TR-3) with explicit Response Matrix decisions and resolution prose; the rewrites are honest and traceable. The `## Spec` section is self-contained per iteration — no "see 00_PLAN" references, no carried-over phrasing. The two HIGH issues from iteration 00 (slash-command parity reality, `task_promote` policy) are addressed with new constraints and validations. The remaining issues are MEDIUM/LOW polish: the Codex substitution map in C-11 is incomplete (missing `/ark:design → ark-design`), C-17's enumeration of `Tier::Deep`-checking sites is non-exhaustive (misses commit.rs:146/149, new.rs:358, phase.rs:122, promote.rs:110 — each is benign but C-17 should not claim exhaustiveness without enumerating them), the defensive `artifact_for` arm `(_, Tier::Research, _) => None` is impossible to add as written because the function is a single-variable `match phase` not a tuple match, and C-11 still says `$ARGUMENTS → <topic>` while the established Codex convention uses `<task description>` for `/ark:quick`. None of these is blocking; the design remains sound.

---

## Findings

### R-001 `Codex substitution map in C-11 omits /ark:design`

- **Severity:** MEDIUM
- **Section:** `## Spec` C-11; `## API Surface` slash-command body (the "## See Also" line that references `workflow.md` is fine, but the body's "## If the corpus turns into implementation" section references `/ark:quick` and `/ark:design`); `## Implementation` Phase 5 step 5 (Codex variant).
- **Problem:** C-11's substitution map names three substitutions: `/ark:research → ark-research`, `/ark:commit → ark-commit`, `$ARGUMENTS → <topic>`. But the body proposed under `[**API Surface**]` contains two additional `/ark:`-prefixed cross-references the Codex variant must rewrite: `/ark:quick` (in Preconditions: "use `/ark:quick` or `/ark:design` instead") and `/ark:design` (same line plus the "If the corpus turns into implementation" subsection). The existing `templates/codex/skills/ark-quick/SKILL.md` rewrites every `/ark:*` cross-reference (`/ark:quick`, `/ark:design`, `/ark:commit`) to the `ark-*` form, not just the command's own name. V-IT-3 ("the parity test applies the inverse substitution to the Codex body and asserts byte-equality with Claude's") will therefore fail unless the substitution map covers every cross-reference, not just the three named.
- **Why it matters:** The Executor will write the Codex SKILL.md to match C-11's literal map, leave `/ark:quick` and `/ark:design` unsubstituted, and the parity test will diverge. Alternatively the Executor will silently extend the map, drifting from C-11. Either way the SPEC and the shipped body diverge.
- **Recommendation:** Restate C-11's map as a closed set covering every cross-reference: `/ark:research → ark-research`, `/ark:quick → ark-quick`, `/ark:design → ark-design`, `/ark:commit → ark-commit`, `$ARGUMENTS → <topic>`, and the H1 line transform. The simplest closed form is "every `/ark:<name>` substring in the body is rewritten to `ark-<name>`; `$ARGUMENTS` is rewritten to `<topic>`." Confirm the established `<topic>` vs the existing `<task description>` convention is intentional (see R-002).

### R-002 `$ARGUMENTS → <topic> conflicts with existing Codex convention of <task description>`

- **Severity:** LOW
- **Section:** `## Spec` C-11; `## API Surface` H1 line transform.
- **Problem:** C-11 maps `$ARGUMENTS → <topic>` in the Codex variant; the H1 transform shows `# \`ark-research <topic>\``. But the existing `templates/codex/skills/ark-quick/SKILL.md` ships `# \`ark-quick <task description>\`` — i.e. the established convention substitutes `$ARGUMENTS` with `<task description>`, not with the command-specific noun. The PLAN's choice (`<topic>`) is more natural for research tier semantics, but it breaks symmetry with `/ark:quick`'s shipping body.
- **Why it matters:** Mostly cosmetic. The user reading both SKILLs side by side will notice the divergence ("why does quick say `<task description>` while research says `<topic>`?"). The parity test isn't affected — V-IT-3 only applies the inverse substitution to Codex's body. But it's a noise source for future consistency reviews.
- **Recommendation:** Pick one and document the reason. Two reasonable options: (a) keep `<topic>` and add a one-line `## Log` annotation noting the divergence from `/ark:quick`'s `<task description>` is deliberate because research's argument *is* the topic, not a description of work to do; or (b) align with the existing convention and use `<task description>` in the Codex H1 to match `/ark:quick`. Either is fine; just declare intent.

### R-003 `Defensive artifact_for arm "(_, Tier::Research, _) => None" cannot be added as written`

- **Severity:** MEDIUM
- **Section:** `## Spec` Architecture (phase.rs annotation); `## Implementation` Phase 1 step 1.5; `## Log` Added bullet 5; V-UT-14.
- **Problem:** The PLAN proposes adding a defensive arm `(_, Tier::Research, _) => None` to `phase.rs::artifact_for`. The current function signature is `fn artifact_for(phase: Phase, tier: Tier, iteration: u32) -> Option<(&'static str, String)>` and the body is a single-variable `match phase { Phase::Plan => { let name = match tier { Tier::Deep => ..., _ => ... }; ... } Phase::Review => ..., Phase::Verify => ..., _ => None }`. A tuple-pattern arm `(_, Tier::Research, _) => None` cannot be added without restructuring the outer match to `match (phase, tier, iteration)` (or equivalently, adding an early-return at the function head: `if matches!(tier, Tier::Research) { return None; }`). The PLAN's literal pattern is not addable to the existing structure.
- **Why it matters:** An Executor following Phase 1 step 1.5 verbatim will produce code that does not compile. The fix is one of: (a) add an early-return `if matches!(tier, Tier::Research) { return None; }` at the head of the function — simplest, matches the intent; (b) restructure the outer match to `match (phase, tier, iteration)` and rewrite every existing arm — invasive and unrelated to the new behavior; (c) add `if matches!(tier, Tier::Research) { return None; }` inside each `Phase::Plan` / `Phase::Review` / `Phase::Verify` arm — verbose and easier to forget on future phases. Option (a) is clearly best. V-UT-14 ("`artifact_for` returns `None` for any `(_, Tier::Research, _)` triple even if called directly") is testable under (a) but the assertion phrasing should not mislead the reader into thinking a tuple-pattern arm exists.
- **Recommendation:** Rewrite Phase 1 step 1.5 to: "Add an early-return at the head of `artifact_for`: `if matches!(tier, Tier::Research) { return None; }`. This guarantees Research-tier callers receive `None` regardless of the future shape of the outer match." Update the Architecture annotation accordingly. V-UT-14's prose can stay (it tests behavior, not arm shape).

### R-004 `C-17's enumeration of Tier::Deep production sites is non-exhaustive`

- **Severity:** MEDIUM
- **Section:** `## Spec` C-17; `## Log` Changed bullet 7 ("C-17 enumerates the four production `Tier::Deep` sites").
- **Problem:** C-17 cites four sites: `promote.rs:78`, `promote.rs:91`, `commit.rs:174`, `spec/extract.rs:84`. A grep of the worktree at the production layer (excluding `#[cfg(test)]` blocks) finds additional production `Tier::Deep`-checking sites that C-17 does not enumerate: `commit.rs:146` (`matches!(prev_toml.tier, Tier::Standard | Tier::Deep)` — VERIFY gating), `commit.rs:149` (`prev_toml.tier == Tier::Deep && !counts.is_clean()` — Deep tier hard-fail on dirty VERIFY), `new.rs:358` (`Tier::Deep => Some(3)` — `max_iterations` seed in `build_task_toml`), `phase.rs:122` (`Tier::Deep => format!("{iteration:02}_PLAN.md")` — artifact naming), and `promote.rs:110` (`(Tier::Deep, _)` arm in `phase_exists_in_tier`). C-17 says "all four short-circuit correctly for Tier::Research without a Research-specific arm" — that's true for the four it lists, and is *also* true for the five it omits (Research never reaches the VERIFY block because `check_phase_for_commit` rejects every other phase first; Research never seeds `max_iterations` because `build_task_toml` already special-cases it per Phase 2; Research never reaches `phase.rs::artifact_for` with `Phase::Plan` because `check_transition` rejects first, and R-003's defensive arm covers the rest; Research never reaches `phase_exists_in_tier` because C-18's `task_promote` early-return fires first). But the *claim of exhaustiveness* ("the four production `Tier::Deep` sites") is incorrect.
- **Why it matters:** A Verifier scoring C-17 by greppping for `Tier::Deep` in production code will find more than four sites and either (a) mark C-17 as not-done because the enumeration is wrong, or (b) approve it on the substantive claim and ignore the enumeration. Neither outcome is ideal. The "verified during this iteration" framing in `## Log` Changed bullet 7 also overstates the verification.
- **Recommendation:** Restate C-17 without the enumeration: "Every production site that branches on `Tier::Deep` short-circuits correctly for `Tier::Research` because Research-tier reachability is gated by `check_phase_for_commit` (commit.rs), `check_transition` + the defensive `artifact_for` arm (phase.rs), the `Tier::Research` early-return in `task_promote` (promote.rs C-18), and `build_task_toml`'s per-tier initial phase (new.rs)." Drop the four line-number citations from C-17 itself and move the full site inventory to a one-paragraph appendix or a comment in the Architecture file tree. The line numbers will rot on the next refactor anyway; the invariant ("Research is never reached at any Deep-only site") is what survives.

---

## Trade-off Advice

### TR-1 `Codex argument substitution: <topic> vs <task description>`

- **Related Plan Item:** C-11 substitution map.
- **Topic:** Compatibility vs Clean Design.
- **Reviewer Position:** Neutral (lean toward `<topic>`).
- **Advice:** Either choice is defensible; document which is intended.
- **Rationale:** `<topic>` reads more naturally on research tier — the argument literally is a research topic, not a description of work to perform. But `<task description>` matches the established Codex skill convention shipped by `/ark:quick`. Choosing `<topic>` is a one-line precedent break that other tier-specific commands (a future `/ark:research-foo`?) may follow; choosing `<task description>` keeps the existing convention intact. There's no correctness consequence either way.
- **Required Action:** Adopt with clarification. Pick one explicitly and add a sentence in C-11 (or `## Log`) declaring the choice and the reason. R-002 captures this as a LOW finding for the same reason.

### TR-2 `Defensive guard: early-return vs tuple-match restructure`

- **Related Plan Item:** Phase 1 step 1.5; R-005 (00_REVIEW.md's defensive arm recommendation).
- **Topic:** Code Shape vs Faithfulness to Reviewer Wording.
- **Reviewer Position:** Prefer the early-return (option a in R-003).
- **Advice:** Use an early-return at the function head rather than restructuring the outer match.
- **Rationale:** The reviewer in 00_REVIEW.md's R-005 wrote "(_, Tier::Research, _) => None" as a shorthand sketch, not a literal arm spec. The intent is "Research-tier callers always receive None, regardless of phase or iteration." An early-return preserves that invariant in one line, leaves the existing match structure untouched, and is trivially testable. Restructuring the outer match to a tuple match would touch every existing arm and add nothing the early-return does not already provide.
- **Required Action:** Adopt the early-return shape; rewrite Phase 1 step 1.5's prose accordingly. R-003 captures this as a MEDIUM finding.
