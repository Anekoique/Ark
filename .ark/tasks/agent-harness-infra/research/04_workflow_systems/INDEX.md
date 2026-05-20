# 04 — Workflow Systems

How agent harnesses structure the development loop: spec-first artifacts, plan/execute/verify separation, TDD adaptation, review gates, artifact taxonomies, ceremony tiers, and quality benchmarks.

The section's organising question: **what is the "right" workflow shape for an agent harness in 2026, and where can Ark's tiered + spec-driven bet pull ahead?**

| File | One-line takeaway |
| ---- | ----------------- |
| [`spec-driven-development.md`](spec-driven-development.md) | "Spec-first" is real but factional: spec-kit (constitution + phase gates), OpenSpec (change proposals + delta merge), Kiro (3-phase IDE), Trellis (progressive specs + workflow). Ark's lineage is Trellis + OpenSpec + spec-kit; bet is **tier-aware** spec extraction. |
| [`plan-execute-verify-loops.md`](plan-execute-verify-loops.md) | PEV is industry consensus. ReAct (2022) → Reflexion (2023) → Plan-and-Solve → Devin / OpenHands "Plan mode". Ark's deep PLAN ⇄ REVIEW loop is one of the **few** harnesses that gates progress on review verdict, not on iteration count. |
| [`tdd-for-agents.md`](tdd-for-agents.md) | TDD adapted = tests-as-spec + autonomous green-loop. Aider's `--test-cmd`, Cursor's grind hook, Reflexion's reward signal. Failure modes: agents gaming tests, brittle prompts, tests-as-overspec. Ark's `G-N → V-N` Acceptance Mapping is the lighter cousin. |
| [`review-as-gate.md`](review-as-gate.md) | Review as workflow gate is converging. Anthropic Code Review (multi-agent fleet + confidence gate, 2026), Constitutional AI self-critique loop, debate/refine patterns. Ark's `NN_PLAN ⇄ NN_REVIEW` is **pre-implementation** review — a rarer position. |
| [`prd-adr-feature-spec.md`](prd-adr-feature-spec.md) | The artifact zoo: PRDs (product), ADRs (decisions), RFCs (proposals), feature SPECs (Ark / OpenSpec). Promotion patterns: OpenSpec delta-merge, Kiro's three-phase staircase, Ark's deep-commit SPEC extraction. "Intent before edits" is the unifying creed. |
| [`tiered-ceremony-and-task-sizing.md`](tiered-ceremony-and-task-sizing.md) | Most tools pick ONE ceremony level. Ark and OpenSpec's `/opsx` profile system are the visible exceptions. Ark's quick/standard/deep/research split is the strongest claim to "right ceremony for the right task". |
| [`evaluation-and-quality-gates.md`](evaluation-and-quality-gates.md) | VERIFY = "is it done?" gate. SWE-bench Verified (500-case, OpenAI-curated), Aider polyglot (225 Exercism), Anthropic Code Review's confidence gate. Ark's `V-NNN` findings and acceptance mapping are local versions of the same idea. |

## Cross-cutting findings

1. **Spec-first is converging on three-phase staircase.** Requirements → Design → Tasks (Kiro), spec → plan → tasks (spec-kit), proposal → tasks → archive (OpenSpec). Ark's PRD → PLAN → EXECUTE → VERIFY is a four-phase variant.
2. **Plan/Execute mode toggle is now table-stakes.** OpenHands, Devin 2.0, Cursor, Claude Code all separate "thinking mode" from "doing mode".
3. **Review is moving from human-only to multi-agent.** Anthropic Code Review (March 2026) dispatches agent teams in parallel with a confidence-score gate.
4. **TDD-for-agents has settled.** Aider `--test-cmd`, Cursor grind hook, Devin self-verification — all run-tests-until-green loops.
5. **Promotion patterns differ.** OpenSpec merges deltas into specs at archive; spec-kit dumps everything into the project tree; Kiro keeps specs in `.kiro/`; Ark extracts SPEC from PLAN at deep commit.
6. **Ceremony tiers are rare.** OpenSpec ships `default`/`expanded` profiles; Ark ships quick/standard/deep/research. Most tools pick one.

## Where Ark could differentiate

- **Tier-aware spec extraction.** Only deep tier promotes a SPEC. Quick/standard write throwaway artifacts. No other surveyed tool gates SPEC promotion on ceremony level.
- **Pre-implementation review.** Ark reviews the PLAN before EXECUTE. Anthropic, Devin, Cursor review the diff after EXECUTE. Both are valid; Ark's position is uncommon.
- **Research tier as first-class workflow.** OpenSpec / Trellis support research notes but not a separate lifecycle. Ark's `Research → Committed → Archived` is the cleanest split.
- **Acceptance Mapping (`G-N → V-N`).** Forcing every Goal to map to a Verify item is a verbatim implementation of the "executable spec" idea — fewer tools enforce it as a hard gate.
