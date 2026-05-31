# Plan-Execute-Verify Loops

## The pattern

Separate the act of *deciding what to do* from *doing it* and from *checking it worked*. In agent design, this is the dominant 2026 control-flow shape:

```
PLAN  ──▶  EXECUTE  ──▶  VERIFY
  ▲                          │
  └────── (rollback / re-plan)
```

Three benefits drive convergence on this shape:

1. **Cheaper reasoning early.** Planning is reasoning-heavy and tool-light; execution is reasoning-light and tool-heavy. Splitting lets you use different models for each.
2. **Auditable intent.** A persisted plan is reviewable before any code changes; verification is reviewable after.
3. **Smaller blast radius on retry.** Re-planning from a known-good plan is cheaper than re-executing from scratch.

## Lineage

### ReAct (2022)

Yao et al., "ReAct: Synergizing Reasoning and Acting in Language Models" (ICLR 2023). Interleaves *thought* tokens with *action* tokens in a single trajectory. The agent reasons, acts, observes the result, reasons again. The loop is:

```
Thought ─▶ Action ─▶ Observation ─▶ Thought ─▶ …
```

Plan and execute live inside the same trajectory — no separate "plan" artifact. This is the granular interleaved end of the spectrum.

### Reflexion (Shinn, Cassano, Labash, Gopinath — NeurIPS 2023)

Paper: <https://arxiv.org/abs/2303.11366>. Quote (from semantic-scholar abstract): "Reflexion agents verbally reflect on task feedback signals and maintain their own reflective text in an episodic memory buffer to induce better decision-making in subsequent trials."

Adds an outer loop:

```
Trial(N): ReAct trajectory → fail
Reflection: write natural-language self-critique → store in episodic buffer
Trial(N+1): ReAct trajectory primed with prior reflections
```

The reflection is the "review" artifact. Tests + task success is the reward signal. This is the **first published recipe for self-review as a workflow gate** in agent literature. GitHub: <https://github.com/noahshinn/reflexion>.

### Plan-and-Solve (Wang et al., 2023)

Two-step prompt: (1) "devise a plan", (2) "carry out the plan to solve the problem". Coarser than ReAct, finer than offline PRD/PLAN documents. Originally for math reasoning; ported to code agents soon after.

### Plan ⇄ Execute mode toggle (2024–2026)

OpenHands, Devin, Cursor, Claude Code all expose a UI-level toggle.

- **OpenHands "Plan mode" (Issue #557, software-agent-sdk)** — the agent operates read-only, maintains `PLAN.md` with success criteria, desirable UX, concrete implementation steps; on switch to Execute, it auto-summarises the planning conversation and hands off the PLAN. Quote (from search snippet): "When switching to Execute mode, the agent automatically summarizes the planning conversation and hands off the PLAN.md to the default Execute agent."
- **Devin 2.0 (Cognition, 2024-12)** — "Each time you start a session, Devin responds in seconds with relevant files, findings, and a preliminary plan." Plan-mode is the default entry point; Agent-mode for execution. Quote (Cognition docs): "Unless you already have a fully scoped plan, it's recommended to start with Ask mode to work with Devin on constructing a plan, then move to Agent mode to execute it."
- **Cursor (2025)** — Agent mode can run for hours, "iterate until tests pass". Hooks let users specify autonomous loops via `.cursor/hooks/grind.ts`. See: <https://cursor.com/blog/agent-best-practices>.

### SWE-agent (Yang et al., 2024)

Paper: <https://arxiv.org/abs/2405.15793>. Key contribution is the **Agent-Computer Interface (ACI)** — a minimal command surface (file viewer, file editor, navigation, linter-gated edits) tuned for LM use. Plan/execute is interleaved ReAct-style, but the ACI itself constrains how planning unfolds.

Notable: a syntax linter runs on every edit; non-syntactic edits are rejected before the agent observes them. This is a **micro-verify** gate inside the execute loop. On SWE-bench full set, SWE-agent achieved 12.29% resolution rate (SOTA at the time).

### OpenHands Software Agent SDK (2025-11)

Paper: <https://arxiv.org/abs/2511.03690>. Architecture is now a published reference:

> The architecture consists of a stateless Agent that emits Actions, a Conversation that runs the loop and stores an append-only EventLog, a Workspace that executes Actions and returns Observations, and an LLM wrapped by LiteLLM for provider portability. Additional features like memory compression, microagent knowledge, sub-agent delegation, security review, and stuck detection operate as auxiliary services on the event stream.

Plan/Execute/Verify is now infrastructure-grade. The `Action → Observation` event log is the audit trail.

## The verify step

Verify is the youngest piece. Three patterns dominate:

1. **Run tests.** Aider's `--test-cmd`, Cursor's grind hook, Devin's auto-verification, SWE-agent's pass/fail on hidden test set.
2. **Lint + static check.** SWE-agent runs a syntax linter on every edit; Cursor's Bugbot Autofix loops over reviewer findings.
3. **Self-critique / multi-agent review.** Reflexion's reward-then-reflect, Anthropic Code Review's parallel agent fleet.

Quote on OpenHands' verify step (search snippet): "Verification involves re-running tests after editing, ensuring that changes are validated rather than assumed to be correct."

## Industry convergence

| Tool | Plan artifact | Execute boundary | Verify gate |
| ---- | ------------- | ---------------- | ----------- |
| Aider | none (in-chat) | per-message | `--test-cmd` + `--auto-test` |
| Cursor | implicit (Agent mode) | per-prompt | grind hook + Bugbot |
| Devin | session plan | Agent mode | autonomous self-test |
| OpenHands | `PLAN.md` (Plan mode) | event stream | re-run tests + security review subagent |
| SWE-agent | implicit (ReAct trace) | per-tool-call | linter + pass/fail on hidden tests |
| spec-kit | `plan.md` + `tasks.md` | `/speckit.implement` | constitutional gates + tests |
| OpenSpec | `proposal.md` + `tasks.md` | `/opsx:apply` | `/opsx:verify` (in expanded profile) |
| Trellis | per-task PRD | implementation skill | "Run checks before handoff" |
| **Ark deep** | **`NN_PLAN.md` ⇄ `NN_REVIEW.md`** | **EXECUTE phase** | **`VERIFY.md` with `V-NNN` findings** |

Ark's separation is the most explicit. The `NN_PLAN ⇄ NN_REVIEW` loop is named, persisted, and gated by a reviewer verdict.

## Ark's deep tier as one instance

From `.ark/workflow.md` lines 76-79:

```
DESIGN  → PLAN  → [REVIEW ⇄ PLAN]  → EXECUTE  → VERIFY  → COMMIT  → ARCHIVE
            quick skips PLAN/REVIEW/VERIFY; standard skips REVIEW
```

Deep tier mechanics:

- `00_PLAN.md` seeded after DESIGN.
- `00_REVIEW.md` produced by the chosen reviewer (`ark-reviewer` subagent / different model / self).
- If verdict is Rejected or Approved-with-Revisions: copy `00_*` to `01_*`, fill Response Matrix in `01_PLAN.md`'s `## Log`, repeat.
- Hard cap: `task.toml.max_iterations` (typically 3-5).
- Pre-implementation review — the diff isn't written yet.

The Response Matrix (every prior CRITICAL/HIGH finding listed with Accepted/Rejected/Deferred + reasoning) is Ark's contribution to the lineage. **No other surveyed tool requires structured acknowledgement of prior review findings before re-iteration.**

VERIFY is post-implementation, structurally analogous to Anthropic Code Review but local to one repo:

- Seeded checklist sections: Project Spec Compliance, Related Feature Spec Compliance, PRD Constraints, Plan Fidelity (one per `G-N`), SPEC Drift.
- `V-NNN` findings for cross-cutting issues.
- Gate: no `PENDING` item.

## Trade-offs

### Ceremony cost

Every persisted artifact has a cost:

- Time to write (agent token spend).
- Time to read (next phase's context budget).
- Drift risk (artifact contradicts code).

Aider's chat-only approach has the lowest ceremony; spec-kit's constitutional gates the highest. Ark's tiers (quick = PRD only; deep = PRD + iterated PLAN/REVIEW + VERIFY + promoted SPEC) span the range.

### Reliability gain

The published evidence:

- Reflexion outperforms ReAct on programming and reasoning benchmarks. Ablations attribute most of the gain to the reflection step.
- SWE-agent's linter gate (a micro-verify) materially affects its SWE-bench score — agents without it get stuck in syntax-error spirals.
- Anthropic Code Review's confidence gate (default 80) is explicitly tuned against false-positive noise. Without it, the multi-agent review degrades into surface critique.

Open question: where's the inflection point where PEV ceremony stops paying off? Likely depends on task scope (fits-in-one-prompt → PEV is wasted; spans-multiple-files → PEV is essential). Ark's tier-pick is an explicit answer.

### Pre- vs post-implementation review

Ark reviews the PLAN. Anthropic / Cursor / Devin review the diff. Each catches different bug classes:

- **Plan review** catches *intent* bugs (wrong approach, wrong scope, wrong invariant). Cheap to fix (edit PLAN).
- **Diff review** catches *implementation* bugs (logic errors, edge cases, regressions). Cheaper to find (the code exists), expensive to fix (the code exists wrongly).

Industrial-strength workflows want **both**. Ark currently has both (deep tier: PLAN review + VERIFY); the pattern is rarer than either alone.

## Failure modes

- **Plan-drift.** Agent writes PLAN, implements differently, doesn't update PLAN. Ark's `workflow.md` instructs "if implementation reveals design gaps, update the latest PLAN's `## Spec`. Do not silently diverge." Enforcement is by reviewer discipline, not CLI.
- **Verify ritualism.** Checklist items get checked PASS without inspection. Ark mitigates with seeded items (one per SPEC, one per Goal) so the surface is concrete, not "did you verify?".
- **Iteration loops without progress.** Reflexion's authors flagged this; Ark caps `max_iterations`. OpenHands has explicit "stuck detection" as an auxiliary service.
- **Plan-as-overspec.** Plans become so detailed that re-planning is harder than re-implementing. spec-kit's "Implementation plan should remain high-level and readable" note (from `spec-driven.md`) is the canonical warning.

## Directions for Ark

1. **Stuck detection in EXECUTE.** OpenHands has it as an explicit auxiliary service. Ark relies on the model's own judgment. Candidate: a `ark agent task stuck` verb the agent invokes to surface a "I'm looping" signal to the user; the slash command then suggests Reflexion-style "write a reflection and retry" or "promote to deep tier and add a REVIEW".
2. **Standard-tier mini-REVIEW (opt-in).** Currently deep-only. Most "fits-in-a-PR" features could benefit from a single review pass without the iterated loop. Candidate: `/ark:design --review` that adds one REVIEW iteration without promoting to deep.
3. **VERIFY-as-execution-gate.** Today VERIFY runs after EXECUTE. Some tasks (security-sensitive, schema migrations) want VERIFY-style checks DURING execution (every commit, every test run). Candidate: pre-execute verify hooks similar to SWE-agent's linter gate.
4. **Make PLAN/REVIEW model-agnostic.** Ark already asks the user which reviewer to use (subagent / different model / self). The Plan/Execute mode toggles in OpenHands and Devin commit to a *different model* for planning vs executing. Candidate: document this as a first-class option in workflow.md (e.g., "Opus for PLAN, Sonnet for EXECUTE, Haiku for VERIFY").
5. **Persist micro-reflections inside EXECUTE.** Reflexion's contribution. Today the EXECUTE phase has no structured note-taking. Candidate: a `.ark/tasks/<slug>/execute-notes.md` for the agent to journal blockers and resolutions, archived alongside other artifacts.

Sources:

- [Reflexion (Shinn et al., NeurIPS 2023)](https://arxiv.org/abs/2303.11366) — original paper
- [SWE-agent (Yang et al., 2024)](https://arxiv.org/abs/2405.15793) — Agent-Computer Interface
- [OpenHands Software Agent SDK (Wang et al., 2025-11)](https://arxiv.org/abs/2511.03690)
- [OpenHands Plan mode issue #557](https://github.com/OpenHands/software-agent-sdk/issues/557)
- [Cursor best practices for coding agents](https://cursor.com/blog/agent-best-practices)
- [Devin 2.0 blog (Cognition)](https://cognition.ai/blog/devin-2)
- [Reflexion repo (noahshinn)](https://github.com/noahshinn/reflexion)
- [Reason-Plan-ReAct (2025)](https://arxiv.org/abs/2512.03560) — recent extension
- [ReflAct (2025)](https://arxiv.org/abs/2505.15182) — goal-state reflection
