# `agent-harness-infra` PRD

---

[**What**]

A reference corpus mapping the "agent harness" and "agent infrastructure" landscape — what other projects build, what primitives they expose, and what design directions Ark should evaluate next.

[**Why**]

Ark positions itself as an agent harness: a runtime + workflow environment that makes coding agents (Claude Code, Codex, OpenCode) work better. The space is moving fast in 2025–2026 — Aider, Cline, OpenHands, Devin, SWE-agent, Cursor, Goose, MCP, plus a wave of "agent infra" platforms (Lambda's agent runtime, Anthropic's Skills, sub-agent dispatch, etc.). Before we plan the next wave of Ark features we need a grounded picture of: what primitives the field treats as table-stakes, what novel ideas are worth porting, where Ark already has a defensible edge, and which directions are dead-ends or premature.

[**Outcome**]

A directory-organized corpus under `research/` that we can scan when planning future features. Success = when someone proposes a new Ark feature, we can open one or two corpus files and find prior art + tradeoffs already documented, instead of starting from scratch on the web.

Not required to lead directly to implementation — but each top-level corpus section ends with a "Directions for Ark" subsection that names concrete candidate features (with no commitment to build them).

[**Related Specs**]

All current feature SPECs are inputs (they define what Ark *already* does, so the gap analysis works):

- `specs/features/ark-agent-namespace/SPEC.md` — the `ark agent` hidden CLI we contrast against
- `specs/features/ark-context/SPEC.md` — context surface we'll compare to others' context APIs
- `specs/features/worktree/SPEC.md` — multi-task isolation we'll compare to sandbox patterns
- `specs/features/subagent-support/SPEC.md` — multi-agent orchestration baseline
- `specs/features/workspace/SPEC.md` — journals / developer identity baseline
- `specs/features/detachable-feature-spec/SPEC.md` — spec extraction baseline
- `specs/features/ark-research/SPEC.md` — research-tier baseline (this is the tier we're using)

[**SPEC Path**]

n/a — research tier ignores this block.

---

## Scope (corpus structure)

The corpus is a tree, not a flat list. Top-level sections, each a subdirectory:

1. `00_landscape/` — the map: definitions, taxonomy, market snapshot, glossary.
2. `01_prior_art/` — one file per comparable harness (Aider, Cline, Roo, Cursor, OpenHands, Devin, SWE-agent, Goose, Continue, others). Each: what it is, primitives, workflow model, integration surface, what Ark could borrow.
3. `02_infra_primitives/` — sandboxing, worktrees, sessions, memory, hooks, tool registries, MCP, observability, snapshots.
4. `03_context_engineering/` — context-window management, RAG for codebases, codemaps, JIT loading, structured summaries, compaction, attention budgeting.
5. `04_workflow_systems/` — spec-driven dev, plan-execute-verify, TDD-for-agents, review-as-gate, PRD/ADR/feature-spec patterns, promotion/extraction.
6. `05_orchestration/` — multi-agent patterns, researcher/reviewer/verifier, dispatch models, parallelism, agent-to-agent protocols.
7. `06_platform_integration/` — slash commands vs CLI vs MCP vs IDE plugins, how integrations are layered, plugin ecosystems.
8. `07_developer_ux/` — onboarding, scaffolding, install/upgrade/uninstall, brownfield adoption, learning curve.
9. `08_emergent/` — topics discovered during research that don't fit above (likely candidates: agent economics, evaluation harnesses, security model, agent OS visions).
10. `99_directions/` — synthesis. Cross-cutting candidate directions for Ark, ranked by leverage × confidence.

Each top-level section has an `INDEX.md` listing its files and a one-line takeaway per file. Each file ends with a **Directions for Ark** subsection (concrete, non-committal candidates).

## Stop conditions

- Each top-level section has at least one file landed.
- The synthesis file `99_directions/SYNTHESIS.md` exists and references ≥10 cross-corpus findings.
- More searching wouldn't change the top-5 ranked directions.

## Out of scope

- No code changes. No PLAN. No SPEC promotion.
- No vendor evaluation we'd publish externally — this is internal corpus, biased toward "what should we build next?".
- Not a literature review for academic publication; cite informally.
