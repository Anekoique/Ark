# Evaluation for Harnesses, not Models

The 2026 evaluation landscape was built for *models* (SWE-bench Verified, MRCR, RULER). But the same model gets ±20% SWE-bench score across different harnesses — meaning the *harness* is now the load-bearing variable. The field lacks a good evaluation methodology for harnesses themselves.

This file: what model evaluations look like; why they don't suffice for harnesses; what a harness evaluation would measure; whether Ark should ship to one.

## Model evaluations recap

- **SWE-bench / SWE-bench Verified** — Real-world GitHub issues; verified subset filtered for clean grading. Industry standard.
- **SWE-bench Multilingual / Polyglot** — Aider's multi-language extension.
- **TerminalBench** — Anthropic / Stanford. Terminal command sequencing.
- **Aider Leaderboard** — Diff-format quality across models.
- **MRCR v2 / RULER** — Multi-needle context recall.
- **SWE-bench Adversarial (SWE-ABS)** — Adversarial subset.

The framing: hold the harness constant; vary the model; measure success rate.

## The harness variance discovery

Through 2024–2025, multiple groups noticed: same model + same task + different harness = different success rate. Notable data points:

- **SWE-agent paper (2024):** introduced the ACI (Agent-Computer Interface) thesis. Showed that designing the interface (bounded outputs, lint-before-edit, clean tool taxonomy) moved scores meaningfully.
- **Anthropic's Claude Code SWE-bench numbers:** Claude Code as harness consistently outperforms Claude models in other harnesses on the same underlying model.
- **OpenHands' SDK paper (2025):** documents that the harness's event-stream + condenser + sub-agent dispatch architecture is responsible for substantial fraction of measured improvement.

By 2026 the "harness as alpha" thesis is community consensus. The implication: a benchmark that holds *harness* constant and varies *model* tells you about models; a benchmark that holds *model* constant and varies *harness* tells you about harnesses.

## What a harness evaluation would measure

Hypothetical harness-bench dimensions:

1. **Success rate.** Same as SWE-bench Verified, but pin the model; vary the harness.
2. **Time-to-completion.** Harnesses with better orchestration finish faster.
3. **Token cost per success.** Cost-normalised quality.
4. **Recovery from failure.** What fraction of tasks succeed *after* a first attempt fails?
5. **Workflow fidelity.** For workflow-opinionated harnesses (Ark, spec-kit), does the harness produce the artifacts it claims (PRD, PLAN, SPECs)?
6. **Spec adherence.** Does the harness produce code consistent with its declared SPECs?
7. **Reversibility.** Can the harness's outputs be cleanly removed without breaking the repo?
8. **Cross-platform parity.** Does the harness work the same on Claude Code, Codex, OpenCode?

No public benchmark hits all of these. SWE-agent's ACI work is closest to (1, 2, 3). The rest are largely uncharted.

## Why no one ships a harness bench

A harness benchmark is harder than a model benchmark:

- **Harnesses are heterogeneous.** Aider, Cline, Claude Code, Ark all have different surface areas. Defining "the same task" across them is hard.
- **Harnesses require setup.** Each one needs its config files, its host platform. A benchmark runner needs to scaffold each fresh.
- **Closed-source harnesses opt out.** Cursor, Devin, Replit won't run on a community benchmark; they have their own internal evals.
- **The metric is contested.** "Did the harness do well?" depends on what you wanted (speed vs. correctness vs. spec-fidelity vs. token cost).

## What Ark could ship

Three options, ordered by ambition:

### Option A — internal regression suite

Pin a small set of canonical tasks (10–20). For each: run the full Ark workflow end-to-end (quick / standard / deep). Verify expected artifacts exist, code-changes-roughly-correct.

**Pro:** Catches harness regressions in development.
**Con:** Doesn't claim external comparison.

This is the most realistic near-term move.

### Option B — Ark-specific benchmark + score

Define an "Ark workflow score" — does the deep tier produce a SPEC, do REVIEW iterations converge, does VERIFY catch known issues. Run against a fixed task set. Publish numbers.

**Pro:** Marketing artifact ("Ark workflow score: 0.87 on our benchmark").
**Con:** Self-published numbers; no external validation.

### Option C — SWE-bench-style cross-harness benchmark

Define a task suite where Ark, plain Claude Code, plain Cline, plain Aider all attempt the same tasks. Measure success rate. Publish a leaderboard.

**Pro:** External comparison; positions Ark as a quality harness.
**Con:** Significant infrastructure; risk of poor showing on tasks not aligned with Ark's tier model; ongoing maintenance.

## What Ark's workflow opinion lets us evaluate cleanly

Ark's distinctive features map to measurable claims:

- **Tier sizing is correct.** Quick-tier tasks should finish faster than standard-tier; users should pick the smallest tier that works.
- **PLAN ⇄ REVIEW iterations converge.** Average iteration count; rare runaway loops.
- **VERIFY catches PENDING items.** What fraction of pre-commit VERIFY runs surface real issues?
- **SPEC promotion produces correct SPECs.** Quality of auto-extracted feature SPECs vs. hand-written ones.
- **Multi-platform parity.** Same task on Claude vs. Codex vs. OpenCode — same artifacts produced.

A small benchmark could validate all five. None requires SWE-bench-class infrastructure.

## The evaluation gap as a positioning opportunity

The field lacks harness-level evaluation methodology. A workflow-opinionated harness like Ark could *propose* one — write a position paper or RFC laying out:

- The dimensions a harness benchmark should measure.
- A reference task suite.
- A reference scoring rubric.

Even if no one else adopts the methodology, the framing helps Ark explain its design choices ("we optimised for X dimension which prior model-benchmarks don't surface").

## Adjacent: evaluation as a feature, not a benchmark

A different framing: ship evaluation as a *feature*, not a benchmark. Examples:

- **`ark verify --against <fixtures-dir>`** — given a directory of past tasks with known-good outputs, run the workflow and compare.
- **`ark agent task verify --strict`** — auto-fail VERIFY if any acceptance check is `N/A` without justification.
- **`ark replay <task-archive>`** — re-run an archived task on the current code; see what changes.

Each is a different angle. None requires participating in a public benchmark.

## What this corpus exercise tells us

This research task is itself an evaluation data point:

- **Workflow worked:** the research tier produced a deliverable directory.
- **Sub-agent dispatch worked when it worked, failed loudly when it failed.** Watchdog timeouts dropped 4 of 9 dispatches; disk persistence saved partial work; re-dispatch (or main-session writes) recovered.
- **Acceptance check:** all 8 corpus sections have content; INDEX.md cross-references work; per-file Directions for Ark accumulate.

If Ark had a benchmark suite, "research-tier corpus produces all expected sections" could be a test case.

## Directions for Ark

1. **Ship an internal regression suite first.** Option A above. ~20 canonical task fixtures, run as integration tests. Catches workflow regressions; cheap to maintain.

2. **Define an "Ark workflow score" with explicit dimensions.** Tier sizing, iteration convergence, VERIFY catch rate, SPEC quality, multi-platform parity. Even if self-published, the framework communicates what Ark values.

3. **Avoid SWE-bench participation until the cross-harness shape settles.** Running Ark on SWE-bench-Verified would compare it to plain Claude Code; the result reflects model + workflow combined. Hard to attribute. Wait for the field to develop harness-bench.

4. **Frame the evaluation gap publicly.** A `docs/book/src/concepts/evaluating-harnesses.md` page laying out the dimensions a harness should be measured on. Positions Ark; helps the field; cheap to write.

5. **`ark verify --against` fixtures-mode as a near-term feature.** Lets users define their own test fixtures and validate Ark's workflow output. Doubles as Ark's internal regression test runner.
