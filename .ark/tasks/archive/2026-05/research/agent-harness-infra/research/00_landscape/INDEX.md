# 00_landscape — Index

The map. Read this before any other corpus section: it pins the vocabulary the rest
of the corpus uses, then sketches the 2026 ecosystem at one zoom level above the
per-project profiles in `01_prior_art/`.

Compiled 2026-05-20.

## Files

| File | One-line takeaway |
| ---- | ----------------- |
| [`definitions.md`](definitions.md) | "Agent harness", "agent runtime", "agent OS", "agent framework", "AI IDE", "coding agent" all overlap; harness vs framework is the load-bearing distinction (harness = wrap a model with state/tools/loops/guardrails; framework = SDK for building them). Ark is a *coding-agent harness with a workflow opinion*. |
| [`market_map.md`](market_map.md) | Five buckets: IDE-native agents · CLI harnesses · cloud autonomous agents · agent-infra platforms · framework SDKs (+ evaluations as cross-cut). CLI harnesses and IDE-native agents are crowded; agent-infra (sandbox-as-a-service) and spec-driven workflows are emerging; pure framework SDKs (AutoGen-style) are dying. |

## Cross-references to sibling sections

- **`01_prior_art/`** — per-project profiles for every name in `market_map.md`'s
  buckets (Aider, Cline, OpenHands, SWE-agent, Goose, Plandex, Claude Code, Codex,
  Devin, Cursor, Zed, Continue, Roo-Cline, Copilot Workspace, Replit, Bolt, v0, +
  agent-platforms survey). Open these when the bucket-level summary in
  `market_map.md` isn't enough.
- **`02_infra_primitives/`** — the primitives `definitions.md` glosses over:
  sandboxing, MCP, hooks, sessions, memory, observability, snapshots, scaffolding.
  Glossary entries that demand depth live there.
- **`04_workflow_systems/`** — the workflow-opinion axis from `market_map.md`
  (spec-driven, plan/execute/verify, TDD-for-agents, review-as-gate, PRD/ADR/SPEC,
  tiered ceremony, evaluation gates) expanded.
- **`07_developer_ux/`** — onboarding, scaffolding, install/upgrade lifecycle,
  brownfield adoption, error messages, cost UX — the user-visible surface
  `market_map.md`'s buckets compete on.

## Reading order

1. `definitions.md` — fix the vocabulary.
2. `market_map.md` — get the lay of the land at one altitude.
3. Then descend into `01_prior_art/` (specific peers) or
   `02_infra_primitives/` / `04_workflow_systems/` (specific primitives/workflow
   patterns) based on the question at hand.

## Directions for Ark (section-level)

1. **Publish a "What Ark is" one-page positioning** that uses the
   `definitions.md` taxonomy verbatim — current README and AGENTS.md read as
   feature lists, not as positioning statements. Rationale: terms are still
   contested in 2026; staking a clear position lowers the cost of every later
   conversation.
2. **Audit overlap with the SKILL.md / AGENTS.md / Skills cross-platform
   standard.** `01_prior_art/INDEX.md` finding #1 already flags this; the
   `market_map.md` "where the puck is going" reinforces it. Rationale: cross-
   compatibility with Claude Code / Codex / Goose / Cursor at zero cost is high
   leverage.
3. **Reserve a "research/landscape" link from `ark --help`** (or the workflow
   doc) pointing back to this corpus once committed. Rationale: discoverability
   is the bottleneck for these files becoming load-bearing during future
   planning.
