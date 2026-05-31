# 08 — Emergent Topics

Topics that surfaced across multiple sections of this corpus but did not warrant a dedicated section of their own. Each gets one file here. Curated *after* sections 00–07 were drafted, with knowledge of what was already covered and what cross-cutting threads recurred without a home.

Compiled 2026-05-20.

## Files

| File | One-line takeaway |
| ---- | ----------------- |
| [`agent-economics-and-cost-ux.md`](agent-economics-and-cost-ux.md) | Token / ACU / spend surfaces in production harnesses. Aider's `/tokens`, Claude Code's `/cost`, Cursor spend limits, Devin's ACU model. Ark is silent on cost today — should it stay silent or surface tier-based estimates? |
| [`evaluation-for-harnesses-not-models.md`](evaluation-for-harnesses-not-models.md) | The harness-quality thesis: same model varies ±20% SWE-bench score across harnesses. SWE-bench-Verified, Aider polyglot, TerminalBench. Could Ark be benchmarked? Should it be? |
| [`security-and-threat-model.md`](security-and-threat-model.md) | The threat model Ark assumes (vs. Codex's OS sandboxing, Devin's VM-per-task). When threat models diverge between user-trust-the-agent and run-untrusted-code shapes. |
| [`agent-os-visions.md`](agent-os-visions.md) | "Agent OS" claims in 2026 — most marketing, some substantive. Ark's RFC 0001 trajectory; Cloudflare Agents, Bedrock AgentCore, OpenAI's pivot. Where the term might mean something by 2027. |
| [`trajectory-and-event-log-architecture.md`](trajectory-and-event-log-architecture.md) | OpenHands' event-stream pattern + condensers. Append-only logs as agent infrastructure. What an Ark trajectory log would buy — replay, audit, fine-tuning data — and what it would cost. |
| [`lint-before-commit-and-pre-edit-safety.md`](lint-before-commit-and-pre-edit-safety.md) | SWE-agent's lint-before-edit; Aider's `--test-cmd`; Cursor's grind hook. Pre-edit safety patterns as the cleanest defence against bad LLM diffs. Could Ark's `task commit` invoke project lint/test? |

## Cross-cutting findings from the emergent set

1. **The "harness as the load-bearing layer" thesis is everywhere.** SWE-agent's ACI argument, the ±20% SWE-bench variance across harnesses, Cline's "the harness is where the alpha is" community posts. Ark is on the right side of this.

2. **Cost surfaces are an undersold UX feature.** Aider, Claude Code, Cursor, Devin all surface cost in some form; the absent surface in Ark is unusual for a 2026 harness. Cheap to add tier-implicit estimates.

3. **Security is bimodal.** Most coding harnesses (Aider, Cline, Claude Code, Ark) assume "user trusts the agent". Some (Codex with sandboxes, Devin with VMs, OpenHands with Docker) assume "run untrusted code". The threat models are different; tooling for the "untrusted code" case is a different product category. Ark sits cleanly in the user-trust camp.

4. **Event logs / trajectories are the natural backing store for everything.** Replay, audit, fine-tuning data, debug. OpenHands has this; most others don't. Ark's `task.toml` + journal is half-there; a real event log would unify many features.

5. **Pre-edit safety patterns are universally good.** Linter before commit, test before commit, drift detection before commit. Cheap; high signal; rarely controversial. Ark's `task commit` is the natural hook.

6. **"Agent OS" is a 2027 conversation, not a 2026 conversation.** Real "OS"-shaped products (Cloudflare Durable Objects for agents, Bedrock AgentCore primitive set) exist but the term is mostly marketing. Ark's RFC 0001 framing — long-term aspiration, near-term humility — is the right posture.

## Reading order

1. `agent-economics-and-cost-ux.md` — the cheapest near-term gap.
2. `lint-before-commit-and-pre-edit-safety.md` — the cheapest near-term safety win.
3. `evaluation-for-harnesses-not-models.md` — the positioning angle.
4. `security-and-threat-model.md` — the contained scope.
5. `trajectory-and-event-log-architecture.md` — the medium-term architecture move.
6. `agent-os-visions.md` — the long-term strategic frame.

## Cross-references

- `01_prior_art/INDEX.md` — sourced the lint-before-commit and trajectory observations from cross-cutting observations.
- `02_infra_primitives/observability-and-telemetry.md` — adjacent to trajectory-and-event-log.
- `04_workflow_systems/evaluation-and-quality-gates.md` — adjacent to evaluation-for-harnesses.
- `07_developer_ux/pricing-cost-and-budget-ux.md` — direct overlap; this section's file is the "infra/architecture" angle, that section's is the UX angle.
- `docs/rfcs/001-arkos.md` (in the repo) — direct reference for agent-os-visions.

## Directions for Ark (section-level)

Each file ends with its own list. Cross-section, the recurring directions worth surfacing here:

1. **Surface tier-implicit cost estimates.** Cheap addition; closes the cost-UX gap.
2. **Add a `task commit --lint` / `--test` pre-edit-safety gate.** Cheap addition; high signal.
3. **Plan an event-log backing store.** Medium-term architectural move; unifies trajectory, journal, audit, replay.
4. **Don't pursue agent-OS as shipping copy.** RFC 0001 frame is right; resist marketing creep.
5. **Decide what evaluation Ark itself should ship to.** SWE-bench-Verified is for models, not harnesses; Ark needs its own evaluation story.
