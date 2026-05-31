# `ark-research` REVIEW `02`

> Status: Closed
> Feature: `ark-research`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved
- Blocking: 0
- Non-blocking: 0

## Summary

Iteration 02 absorbs all four findings (R-001..R-004) and both trade-off items (TR-1, TR-2) from `01_REVIEW.md` with honest, mechanical edits to C-11, C-17, Phase 1 step 1.5, and the Architecture annotation; the Response Matrix accurately reflects what changed. The `## Spec` section remains self-contained per iteration (no "see 01_PLAN" or "as before" references inside Spec; Log references prior iterations as designed), every Constraint and Validation introduced through iteration 01 is preserved intact (C-1..C-18, V-UT-1..V-UT-15, V-IT-1..V-IT-5, V-F-1..V-F-4, V-E-1..V-E-4), and the structural changes are compile-correct against the current source tree. No new contradictions with `ark-agent-namespace`, `subagent-support`, `task-concurrency-control`, `detachable-feature-spec`, or `worktree` SPECs. The design is ready to ship to EXECUTE.

---

## Findings

None. Each 01-REVIEW finding was addressed in the Spec body, not just acknowledged in the Response Matrix:

- **R-001 (closed-form substitution map)** — C-11 now reads "every `/ark:<name>` substring in the body is rewritten to `ark-<name>`; `$ARGUMENTS` is rewritten to `<topic>`; the H1 line `# \`/ark:research $ARGUMENTS\`` is rewritten to `# \`ark-research <topic>\``." The `/ark:<name>` substrings actually present in the slash-command body sketch under `[**API Surface**]` are `/ark:quick`, `/ark:design`, `/ark:research`, and `/ark:commit` — all covered by the closed-form rule. The "Removed" bullet 1 in the Log explicitly drops the brittle three-substitution enumeration.

- **R-002 / TR-1 (`<topic>` vs `<task description>`)** — C-11 now records the choice and the one-line rationale inline: "`<topic>` is chosen over the existing `/ark:quick`-convention `<task description>` because the argument on research tier is literally a topic, not a description of work to perform." TR-1's Required Action ("Adopt with clarification") is satisfied.

- **R-003 / TR-2 (`artifact_for` early-return shape)** — Phase 1 step 1.5 and the Architecture annotation both describe a head-of-function early-return: `if matches!(tier, Tier::Research) { return None; }`. Verified addable: `artifact_for(phase: Phase, tier: Tier, iteration: u32) -> Option<...>` in `crates/ark-core/src/commands/agent/task/phase.rs:118` takes `tier` by value and returns `Option`; the early-return slots in above the existing `match phase { ... }` body without touching any existing arm. The literal `(_, Tier::Research, _) => None` pattern (Log Removed bullet 2) is explicitly dropped. V-UT-14's behavior assertion ("returns `None` for any `(_, Tier::Research, _)` triple") stands and is satisfied by the early-return.

- **R-004 (C-17 invariant)** — C-17 is now an invariant statement enumerating four independent gating mechanisms (a) `check_phase_for_commit` in commit.rs, (b) `check_transition` + the `artifact_for` early-return in phase.rs, (c) the `task_promote` early-return per C-18, (d) `build_task_toml`'s per-tier initial phase in new.rs. No line numbers. The brittle four-citation list and the "verified during this iteration" framing from iteration 01 are both dropped (Log Removed bullet 3 + Log Changed bullet 4). Each of the four gates is real and load-bearing in the production source:
  - (a) `check_phase_for_commit` is the gate at `commit.rs:303`, called at `commit.rs:130` before the VERIFY block (lines 146-166).
  - (b) `transition()` at `phase.rs:71` calls `check_transition(toml.tier, from, to)?` at line 83 strictly before `artifact_for(...)` at line 89.
  - (c) `task_promote` at `promote.rs:53` calls `phase_exists_in_tier` at line 66; the proposed early-return slots in between `let from = toml.tier;` (line 64) and the existing check.
  - (d) `build_task_toml` at `new.rs:349` currently hardcodes `phase: Phase::Design`; Phase 2 step 2's switch to a per-tier expression matches C-4.

Self-containment of `## Spec`: scanned for "see 00_PLAN", "see 01_PLAN", "as before", "see iteration", "see previous". None appear inside `## Spec`. The Spec preamble correctly notes "The Spec is self-contained per iteration — no 'see 00_PLAN' references appear here." `## Log` references prior iterations, which is the Log's contract.

Constraint and validation preservation: C-1..C-18 are all present with identical numbering. C-11 and C-17 are the only constraints with changed bodies; both changes are the absorption demanded by R-001/R-002/R-004. V-UT-1..V-UT-15, V-IT-1..V-IT-5, V-F-1..V-F-4, V-E-1..V-E-4 are all preserved verbatim. The Acceptance Mapping table is unchanged and remains complete.

Final-iteration discipline: max_iterations=3 with iteration=2 means this is the last budgeted iteration. All four 01-REVIEW findings were MEDIUM/LOW polish — none required architectural redrafts. The absorption is mechanical and traceable. Verdict is Approved.

---

## Trade-off Advice

None for this iteration. TR-1 and TR-2 from `01_REVIEW.md` were the only open trade-offs; both are now resolved in the Spec body (TR-1: `<topic>` chosen with rationale recorded in C-11; TR-2: early-return shape adopted in Phase 1 step 1.5 and the Architecture annotation). T-1..T-6 in the PLAN's Trade-offs section continue to capture the design choices accurately.
