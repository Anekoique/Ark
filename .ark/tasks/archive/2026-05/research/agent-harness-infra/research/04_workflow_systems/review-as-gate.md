# Review-as-Gate

## The premise

Review is the workflow's "no" button. If review can't block, it's a ritual, not a gate. Three review modes have shipped:

1. **Human gate.** Diff goes to a person; they accept/reject. Traditional code review.
2. **Agent gate.** Diff goes to an LLM (or fleet of LLMs); they accept/reject (or surface findings).
3. **Self-critique gate.** Same model that produced the work critiques it; outer loop re-prompts on rejection.

Each has tradeoffs. The 2026 frontier is **multi-agent review at PR time**, with confidence-score gates filtering false positives.

## Constitutional AI — the academic ancestor

Anthropic's Constitutional AI (Bai et al., 2022) introduced the self-critique loop as a *training* technique:

1. Model generates a response.
2. Model critiques its own response against a "constitution" (set of natural-language principles).
3. Model revises based on the critique.
4. Critique-revise pairs become fine-tuning data.

This is RLHF-without-humans. The supervised self-revision stage is the prototype of inference-time self-critique loops.

From <https://mbrenndoerfer.com/writing/constitutional-ai-principle-based-alignment-through-self-critique>:

> Rather than relying solely on human preferences, Constitutional AI proposed training models to follow a "constitution", a set of explicit principles that guide the model's behavior through self-critique and self-correction.

By 2025-2026, CAI is part of Claude's training pipeline. From the same source:

> Claude 4 (released May 2025) and Claude 4.5 Sonnet combine constitutional principles with human feedback (RLHF) and additional fine-tuning stages to balance safety, helpfulness, and performance.

Known critique: "RL-AIF in the paper relies on fine-tuning models using its own self-critic outputs, which may lead to model collapse" (Kazdan et al., 2025; <https://arxiv.org/pdf/2504.04918>). For inference-time use, the analogous risk is reward-hacking — the critique loop converges on whatever the critic likes, not what's correct.

## Debate, refine, and self-consistency

A family of prompt-time techniques that put an LLM against itself:

- **Self-Refine** (Madaan et al., 2023) — generate → critique → revise → repeat. Variant of Constitutional AI applied at inference.
- **Debate** (Irving et al., 2018; revived 2023+) — two models argue opposing positions; a third (or human) judges.
- **Self-consistency** (Wang et al., 2022) — sample multiple chains-of-thought, vote on the answer.

None of these are workflow gates by themselves — they're techniques. They become gates when wrapped in a loop with a halt condition (Reflexion is the canonical example, see `plan-execute-verify-loops.md`).

## Anthropic Code Review (March 2026)

The first vendor-shipped multi-agent code review system. Launch: March 2026. <https://thenewstack.io/anthropic-launches-a-multi-agent-code-review-tool-for-claude-code/>

Architecture:

> When a review runs, multiple agents analyze the diff and surrounding code in parallel on Anthropic infrastructure. A fleet of specialized agents examine the code changes in the context of your full codebase, looking for logic errors, security vulnerabilities, broken edge cases, and subtle regressions. Each agent looks for a different class of issue, then a verification step checks candidates against actual code behavior to filter out false positives.

Key features:

- **Specialised agents.** One per issue class (security, logic, regressions, edge cases).
- **Parallel dispatch.** Agents run concurrently, not sequentially.
- **Verification subagent.** Filters candidates against actual code behavior.
- **Confidence-score gate.** Default threshold 80/100 (configurable in `commands/code-review.md`).
- **Per-repo opt-in.** Admins enable per-repo; runs in cloud on PR open.

Cost: $15-$25 per review on average (Anthropic Teams/Enterprise).

Quote from the InfoQ writeup (<https://www.infoq.com/news/2026/04/claude-code-review/>): the system "dispatches agent teams to catch the bugs that skim reads miss."

The model converges on what Ark calls VERIFY — multiple specialised passes against the diff, gated by a confidence threshold.

## Sub-agent review patterns

Ark's `subagent-support` SPEC (`.ark/specs/features/subagent-support/SPEC.md`, promoted 2026-05-10) installs `ark-researcher`, `ark-reviewer`, and `ark-verifier` subagents on Claude / Codex / OpenCode. The pattern:

- `ark-reviewer` invoked during deep tier's REVIEW phase.
- `ark-verifier` invoked during VERIFY phase.
- Both are dispatched by the main session; they read structured context (PRD, PLAN, related SPECs) and emit a structured finding list (`R-NNN` or `V-NNN`).

Comparison to Anthropic Code Review:

| Dimension | Anthropic Code Review | Ark `ark-reviewer` |
| --------- | --------------------- | ------------------ |
| Trigger | PR opened (cloud) | DESIGN → PLAN → REVIEW phase (local) |
| Agents | Many specialised, parallel | One subagent, sequential (user picks) |
| Reviews | Diff (post-implementation) | PLAN (pre-implementation) |
| Output | Confidence-scored findings | Severity-tagged findings + verdict |
| Gate | Confidence ≥ threshold | Verdict ≠ "Rejected" + no open CRITICAL |
| Cost | $15-$25/review | depends on chosen model |
| Iteration | One review per PR (resubmit triggers re-run) | Bounded loop (`max_iterations`, 3-5) |

Ark's bet is **pre-implementation, structured, iterated**. Anthropic's is **post-implementation, parallel, scored**. Both can coexist (and likely should in 2026).

## GitHub spec-kit's constitutional gates

Spec-kit doesn't dispatch a reviewer agent. Instead, it embeds review-style checklists into the workflow itself. From `reference/spec-kit/spec-driven.md`:

```markdown
### Phase -1: Pre-Implementation Gates

#### Simplicity Gate (Article VII)

- [ ] Using ≤3 projects?
- [ ] No future-proofing?

#### Anti-Abstraction Gate (Article VIII)

- [ ] Using framework directly?
- [ ] Single model representation?
```

The LLM is required to tick the boxes or document violations in "Complexity Tracking". This is **review-as-self-checklist**, not multi-agent. The constitution (`memory/constitution.md`) is the rubric.

Quote: "These gates prevent over-engineering by making the LLM explicitly justify any complexity. If a gate fails, the LLM must document why in the 'Complexity Tracking' section, creating accountability for architectural decisions."

## Ark's `NN_PLAN ⇄ NN_REVIEW` loop in detail

From `.ark/workflow.md` lines 122-149:

```
DESIGN → PLAN → [REVIEW ⇄ PLAN] → EXECUTE → VERIFY → COMMIT
```

Deep-tier specifics:

- `00_PLAN.md` seeded after DESIGN.
- `ark agent task review` transitions to REVIEW phase.
- Workflow doc says: **"STOP. Ask the user which reviewer to use: `ark-reviewer` subagent, a different model, or self-review. Do not pick on the user's behalf."**
- Reviewer fills `00_REVIEW.md` with:
  - **Verdict** — Approved / Approved with Revisions / Rejected.
  - **Findings (`R-NNN`)** — Severity, Section, Problem, Why it matters, Recommendation.
  - **Trade-off Advice (`TR-N`)**.
- HIGH severity rule: "Reject as HIGH if the latest PLAN's `## Spec` references prior iterations instead of restating in full." (workflow.md line 136).
- If Rejected/Approved with Revisions:
  - Copy `NN_PLAN.md` → `(NN+1)_PLAN.md`.
  - Copy `NN_REVIEW.md` → `(NN+1)_REVIEW.md`.
  - Bump `task.toml.iteration`, reset phase = "plan".
  - New PLAN's `## Log` Response Matrix lists every prior CRITICAL/HIGH finding with Accepted/Rejected/Deferred + reasoning.

The Response Matrix is the contract: **no review finding is silently dropped.** Ark forces explicit acknowledgement, even if the decision is "Rejected — disagree because X."

Cap: `task.toml.max_iterations` (typically 3-5). On exhaustion, halt and ask the user.

## Human-in-loop gating

All four reviewed harnesses keep humans in the loop:

- **Kiro** — "Proposed changes are presented as readable diffs and you can request modifications, accept part of a batch, or reject it entirely, keeping human review as the final gate." (search snippet)
- **OpenSpec** — proposals reviewed by human before `/opsx:apply`; archive requires human consent.
- **Trellis** — "Run checks before handoff" — checks then human signoff.
- **Ark** — slash commands at every transition; workflow.md repeatedly says "ask the user" (REVIEW reviewer choice, COMMIT message confirmation).

Anthropic Code Review is the partial exception — agent fleet runs autonomously on PR open, but **humans still see the findings**; agents don't auto-merge.

## Automatic critique

The spectrum from least to most autonomous:

| Pattern | Autonomy | Example |
| ------- | -------- | ------- |
| Self-checklist (constitutional gates) | Lowest | spec-kit |
| Self-review (one model critiques itself) | Low | Reflexion |
| Sub-agent review (separate model/agent) | Medium | Ark `ark-reviewer`, Cursor Bugbot |
| Multi-agent fleet (specialised parallel) | High | Anthropic Code Review |
| Auto-merge on green | Highest | Devin (with config); not generally recommended |

The 2026 sweet spot appears to be **specialised parallel with human approval** — Anthropic Code Review's shape. Sub-agent review is one step below.

## Trade-offs

### Pre- vs post-implementation review

Discussed in `plan-execute-verify-loops.md`. Ark reviews PLAN (pre); Anthropic reviews diff (post). Pre catches intent bugs cheaply; post catches implementation bugs. Both useful; Ark's deep tier has both (PLAN review + VERIFY).

### Self-review vs other-model review

Reflexion paper showed self-review works for programming/reasoning tasks. But the same-model-same-bug problem is real — a model that misunderstands the spec is unlikely to catch its own misunderstanding. Mitigations:

- Different model for review (Opus reviews Sonnet).
- Different system prompt for review (force adversarial stance).
- Multi-agent (Anthropic Code Review).

Ark's workflow.md leaves it to the user: "ask the user which reviewer to use: `ark-reviewer` subagent, a different model, or self-review." This is correct but unguided — Ark could ship a default recommendation per tier.

### False positive cost

Reviews that surface noise train agents (and humans) to ignore reviews. Anthropic's confidence gate (default 80) is the explicit countermeasure. Ark has no analogous filter — every `R-NNN` is a finding to be resolved or explicitly rejected.

For deep-tier multi-iteration loops, this is the right call (every finding deserves a Response Matrix entry). For lighter tiers, it would be over-ceremony — which is why standard tier skips REVIEW.

### Iteration cost

PLAN ⇄ REVIEW loops can take 3-5 iterations. Each iteration:

- Reads prior PLAN + REVIEW (context cost).
- Writes new PLAN (token cost).
- Reads new PLAN + writes REVIEW (token cost).

Capped at `max_iterations` to bound the cost. But the cost is real and is one reason deep tier is opt-in.

## Failure modes

- **Rubber-stamp review.** Reviewer (subagent or self) approves without depth. Mitigations: structured finding template (forces specific Section/Problem/Why fields); HIGH-severity catch-all rule (workflow.md line 136).
- **Endless loop.** Reviewer always finds new things to fix. `max_iterations` cap.
- **Drift between PLAN and code.** Review approves a PLAN; EXECUTE diverges. Ark mitigates with VERIFY's "Plan Fidelity — one item per Goal" check.
- **Reviewer model not strong enough.** Subagent reviewer misses bugs a stronger model would catch. Workflow.md's "ask the user" rule lets users escalate. Could be more prescriptive.
- **Reviewer ceremony overwhelms small tasks.** Hence Ark's tier system — standard tier skips REVIEW; quick tier skips PLAN and REVIEW.

## Directions for Ark

1. **Default reviewer recommendation per tier.** Workflow.md says "ask the user". Could be more opinionated: e.g., deep tier defaults to `ark-reviewer` subagent + suggests "use Opus if Sonnet is the EXECUTE model"; standard tier (if mini-REVIEW is added — see plan-execute file) defaults to self-review. Ship as a config knob (`.ark/config.toml: [review] default = "subagent"`).
2. **Confidence-score gate on findings.** Borrow from Anthropic Code Review. Today every `R-NNN` is equally weighted. Adding a Confidence field (alongside Severity) would let `task review` filter at a threshold. Especially useful for VERIFY findings (`V-NNN`) where false positives are noisier.
3. **Parallel specialised reviewers (deep tier opt-in).** Anthropic's fleet model. Ark could dispatch 2-3 `ark-reviewer` invocations with different system prompts (security, logic, style) and merge findings. Useful for very large PLAN diffs in deep tier.
4. **Persisted reviewer-model identity.** Today the reviewer choice is per-iteration. Recording it in `NN_REVIEW.md` (model name + system prompt) makes the loop auditable — useful for retroactive analysis of "did stronger reviewers catch more bugs?".
5. **REVIEW → VERIFY linkage.** Findings approved/rejected in REVIEW could seed VERIFY items. Today they're disjoint: REVIEW closes when verdict = Approved; VERIFY starts fresh. A "Plan Fidelity" subsection that mirrors the final REVIEW's accepted findings ("R-001 — Accepted; verify in EXECUTE") would close the loop.

Sources:

- [Anthropic Code Review launch (The New Stack)](https://thenewstack.io/anthropic-launches-a-multi-agent-code-review-tool-for-claude-code/) — March 2026
- [Claude Code Review docs (Anthropic)](https://code.claude.com/docs/en/code-review)
- [InfoQ on Claude Code Review](https://www.infoq.com/news/2026/04/claude-code-review/)
- [Constitutional AI explainer (Brenndoerfer)](https://mbrenndoerfer.com/writing/constitutional-ai-principle-based-alignment-through-self-critique)
- [Constitution-or-Collapse (Kazdan et al., 2025)](https://arxiv.org/pdf/2504.04918) — collapse risk
- [Cursor Bugbot Autofix](https://cursor.com/blog/bugbot-autofix) — sub-agent review
- [spec-kit `spec-driven.md`](../../../../reference/spec-kit/spec-driven.md) — constitutional gates
- [Reflexion paper](https://arxiv.org/abs/2303.11366) — verbal self-critique
