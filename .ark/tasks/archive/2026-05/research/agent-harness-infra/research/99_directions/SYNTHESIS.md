# Synthesis — Directions for Ark

The corpus's payoff file. The other ~70 files end in per-topic "Directions for Ark" subsections; this file aggregates, deduplicates, ranks, and turns them into shippable bets.

Compiled 2026-05-20 from sections 00 through 08.

## What the corpus actually says

Reading across the corpus, **eleven cross-cutting findings** recur in three or more sections. These define the strategic frame; the per-direction rankings below are derived from them.

### Finding F1 — Harness quality is the load-bearing variable

Same model varies ±20% SWE-bench score across harnesses. SWE-agent's ACI thesis, Anthropic's Claude Code SOTA results, OpenHands' SDK paper all converge here. Ark is on the right side of this.

**Implication:** Ark's investment in workflow opinion is a durable bet. Model quality keeps moving; harness quality moves slower and rewards investment.

### Finding F2 — Skills (SKILL.md) is becoming the portable behaviour-pack format

Claude Code, Codex, Goose, Cursor — all converged. Slash commands are being relabelled as legacy. Cross-platform skill registries are emerging.

**Implication:** Ark's per-platform slash command duplication is a tax that can be removed by single-sourcing to SKILL.md emit per platform.

### Finding F3 — MCP wins as the agent↔tool protocol

Anthropic, OpenAI, Google DeepMind, Cursor, Continue, Zed all speak it. ~5000+ servers exist. ACP and A2A are open at the editor and agent-peer layers; MCP is settled at the tool layer.

**Implication:** Exposing `ark agent` as MCP is the highest-leverage single move available — multiplies reach without per-platform templating.

### Finding F4 — AGENTS.md is the cross-platform context-file standard

16+ tools read it. Linux Foundation steward. Codex's standard; broadly adopted.

**Implication:** Ark should always write AGENTS.md, regardless of platform.

### Finding F5 — Structured artifacts beat ephemeral memory

PLAN.md / SPEC.md / VERIFY.md persist; conversation evaporates. Compaction loses fidelity; disk persistence loses nothing. Ark's bet on structured artifacts is well-aligned with what the field is learning.

**Implication:** Keep doubling down on structured artifacts; resist any temptation to move state into conversation.

### Finding F6 — Sub-agent dispatch is a context-management tool, not just specialisation

Children run in fresh context; parent stays small. Ark already does this; the rationale is undocumented.

**Implication:** Document the context-firewall framing in subagent SPECs. Cheap; teaches users and future contributors.

### Finding F7 — Codemap + JIT beats RAG-for-code

Embeddings underperform symbol indexes + grep for code. Aider, Continue, Sourcegraph all moved away from RAG. Ark's no-RAG stance is now consensus.

**Implication:** Ship a codemap (`docs/CODEMAPS/`); avoid building any vector retrieval. Ark's instinct here is correct; making it explicit closes the field-gap.

### Finding F8 — Event logs are the natural backing store

OpenHands has them; most others don't. Replay, audit, fine-tuning data, debug all derive from one event stream.

**Implication:** A Stage-1 event log (workflow events: phase transitions, dispatches, commits) is feasible near-term and unblocks multiple medium-term features.

### Finding F9 — Pre-edit safety is universally good

SWE-agent's lint-before-edit, Aider's `--test-cmd`, Cursor's grind hook. Cheap; high signal; rarely controversial.

**Implication:** `task commit --lint` / `--test` is a low-effort, high-value add.

### Finding F10 — Cost surfaces are an undersold UX feature

Aider's `/tokens`, Claude Code's `/cost`, Cursor's spend limits. Ark is silent. Tier-implicit estimates are cheap to add.

**Implication:** Surface tier-shaped cost ranges in `ark context`. Builds trust without complex infrastructure.

### Finding F11 — Ark's tiered workflow is a defensible category

No OSS peer ships tiered ceremony + REVIEW iteration + auto-promoted SPECs. Copilot Workspace is closest in spirit (commercial, vertically integrated). This is the *category-of-one* differentiator.

**Implication:** Lean into the differentiator. Documentation, positioning, evaluation all should highlight it.

---

## Direction ranking

~80 individual directions surface across the corpus. Deduplicated, ranked, and grouped by *leverage × confidence × cost*.

**Notation:**
- *Leverage:* impact on Ark's positioning / capability.
- *Confidence:* corpus-evidence-backed certainty.
- *Cost:* engineering effort.

### Tier A — Ship now (Q3 2026)

These are high-leverage, high-confidence, low-to-medium cost moves. Recommended for the next quarter.

#### A1 — Stand up `ark-mcp` server

**Source sections:** 06 platform-integration, 05 orchestration, 02 infra primitives, 01 prior art cross-cutting #3.

**What:** A new crate `crates/ark-mcp` exposing the `ark agent` namespace as an MCP server. Resources for tasks/specs/context; tools for each task-namespace verb; prompts for each workflow phase.

**Why:** Single highest-leverage move. MCP is the converging cross-host capability layer. Existing typed `ark agent` namespace makes this a translation layer, not a redesign. Reduces per-platform templating cost long-term.

**Cost:** Medium. ~2-3 weeks for a first cut. Schema design is the longest part.

**Confidence:** High. F3 is corpus-wide; Codex already exposes itself as MCP.

#### A2 — Always write AGENTS.md

**Source sections:** 00 landscape, 06 platform-integration, 01 prior art cross-cutting #1.

**What:** Extend `init.rs` and `upgrade/mod.rs` to write `AGENTS.md` on every install, not just on Codex/OpenCode. Hook into existing managed-block patterns.

**Why:** Cross-platform convergence is real. F4. One file write per install. Insurance against future platform changes.

**Cost:** Low. ~1 day of code + tests + template update.

**Confidence:** High. F4 is well-supported.

#### A3 — Document the threat model + harness positioning explicitly

**Source sections:** 00 landscape, 06 platform-integration, 08 emergent.

**What:** Three docs pages:
1. `docs/book/src/concepts/positioning.md` — "Ark is a workflow-opinionated harness". The taxonomy from `definitions.md`.
2. `docs/book/src/concepts/threat-model.md` — Model-A trust; what Ark does and doesn't protect against.
3. `docs/book/src/concepts/integration-surfaces.md` — slash command + CLI + (future) MCP. When to use which.

**Why:** F11. The category-of-one differentiator needs explicit positioning. Threat model framing is currently implicit.

**Cost:** Low. ~3-5 days of writing.

**Confidence:** High.

#### A4 — Subagent `tools` allow-list (declarative)

**Source sections:** 06 platform-integration, 05 orchestration.

**What:** Add a `tools` field to each subagent definition (Claude markdown frontmatter, Codex TOML, OpenCode YAML). Specifies the allow-list of tool names. Today this is prompt-only enforcement.

**Why:** Aligns with platform-native declarative restriction. Cheap; harder to forget than prompt rules.

**Cost:** Low. Edit ~9 files (3 platforms × 3 subagents) plus the SPEC.

**Confidence:** High.

#### A5 — Ship `ark codemap` subcommand

**Source sections:** 03 context-engineering, 01 prior art (Aider, Continue, Plandex use codemaps).

**What:** Tree-sitter-based generator writing `docs/CODEMAPS/<module>.md`. Idempotent. Per-language grammars; reasonable defaults; opt-in refresh hook.

**Why:** F7. Biggest section-level gap. Closes parity with peers. Symbol-based, not embedding-based — aligns with the consensus.

**Cost:** Medium. ~2 weeks (tree-sitter integration + format design + tests).

**Confidence:** High. The path is well-trodden; multiple successful implementations to learn from.

#### A6 — Surface tier-implicit cost estimates

**Source sections:** 07 developer UX, 08 emergent.

**What:** Add a `cost_estimate` field to `ark context --scope session --format json`. Range-based (e.g. "Deep: $1-15"). Disclaimer-marked. Configurable per-provider in `.ark/config.toml`.

**Why:** F10. Closes a UX gap unique to Ark vs. every peer.

**Cost:** Low. Schema bump + a static table + config plumbing.

**Confidence:** Medium-high. The shape ("tier-implicit, not per-token") is right; the specific numbers will need tuning.

### Tier B — Ship next (Q4 2026)

Medium-leverage, high-confidence; OR high-leverage with more cost or uncertainty.

#### B1 — Pilot the slash-command → skill migration

**Source sections:** 06 platform-integration.

**What:** Pick `/ark:design` as the first slash command to ship also as a skill. Both formats coexist during transition. Learn what breaks. Decide whether to migrate the rest.

**Why:** F2. Aligns with Claude Code's documented direction. Reduces per-platform divergence long-term.

**Cost:** Medium. ~1 week per command; 8 commands total if full migration; pilot first.

**Confidence:** High about the direction; uncertain about the migration mechanics until the pilot.

#### B2 — Event log (Stage 1: workflow events)

**Source sections:** 08 emergent, 01 prior art cross-cutting #7, 05 orchestration.

**What:** `.ark/.events.jsonl` (or per-task) capturing workflow-level events: phase transitions, subagent dispatches, commits, etc. Schema-versioned, append-only, lock-protected.

**Why:** F8. Unblocks audit, replay, metrics, dispatch-recovery. Architectural foundation for future Stage-2 work.

**Cost:** Medium. ~2-3 weeks for the writer + readers + tests.

**Confidence:** Medium-high. OpenHands is the reference; the pattern is proven; the integration cost into existing Ark commands is the unknown.

#### B3 — Single canonical source for slash commands / skills / subagents

**Source sections:** 06 platform-integration, 00 landscape.

**What:** A `templates/canonical/<artifact>/<name>.yaml` source-of-truth + per-platform emitters that produce `.claude/commands/`, `.codex/skills/`, `.opencode/commands/`, etc.

**Why:** F2. Removes the 8×3 maintenance tax. Enables the skill migration cleanly.

**Cost:** Medium-high. ~3 weeks. Design + emitter implementation + migrating all current templates.

**Confidence:** Medium. The pattern is sound; the specific format will require iteration.

#### B4 — `task commit --lint` and `--test` pre-commit gates

**Source sections:** 08 emergent, 01 prior art cross-cutting #9.

**What:** `.ark/config.toml` `[commit_hooks]` table declaring lint/test/typecheck commands. Optional `--lint` / `--test` flags on `task commit` run them; failure aborts commit.

**Why:** F9. Cheap; high signal; defence-in-depth alongside VERIFY.

**Cost:** Low-medium. ~1 week.

**Confidence:** High about the value; the integration with task-commit's rollback semantics needs care.

#### B5 — Documentation pages for memory hierarchy + failure modes + reversibility

**Source sections:** 03 context-engineering, 05 orchestration, 06 platform-integration.

**What:** Three `docs/book/src/` pages:
- `reference/memory-hierarchy.md` — what lives in CLAUDE.md vs. AGENTS.md vs. SPECs vs. journals.
- `concepts/failure-modes.md` — orchestration / dispatch failures and Ark's mitigations.
- `concepts/reversibility.md` — Ark adds workflow; doesn't capture data. How to leave.

**Why:** Documentation is the biggest single gap. Concepts that the corpus surfaced as load-bearing are implicit in the code; making them explicit is cheap and high-leverage.

**Cost:** Low. ~1 week of writing across all three.

**Confidence:** High.

### Tier C — Plan, don't ship (2027)

Long-term aspirational, or contingent on field standards.

#### C1 — ACP-compatible adapter

**Source sections:** 00 landscape, 05 orchestration.

**What:** Ark speaks ACP so editors (Zed, JetBrains) can drive Ark's workflow primitives.

**Why:** F3 (partial). If ACP wins on the editor↔agent axis, this is the way in. Today the audience is too narrow.

**Confidence:** Medium. ACP adoption is concentrated in Zed/JetBrains; wider adoption is the contingency.

**Recommendation:** Track quarterly. Don't ship until Cursor or another major IDE adopts.

#### C2 — Per-phase model recommendations

**Source sections:** 08 emergent (cost UX), 03 context-engineering.

**What:** `.ark/config.toml` `[models.phase]` table letting users declare which model fits each phase. `ark context` surfaces the recommendation for the current phase.

**Why:** Cost arbitrage. Cheap model plans; expensive model executes. Aider's architect/editor split was the prototype.

**Confidence:** Medium. The pattern works; the specifics depend on model-provider pricing volatility.

**Recommendation:** Wait for the cost-estimate feature (A6) to ship and stabilise first; layer this on top.

#### C3 — Claude Code plugin marketplace bundle

**Source sections:** 06 platform-integration.

**What:** A `claude-plugins/ark` bundle that installs Ark + its templates + the MCP server in one step from Claude Code's marketplace.

**Why:** Discoverability for the Claude Code audience. Reach beyond CLI-comfortable users.

**Confidence:** Medium. Depends on `ark-mcp` (A1) shipping first and the Claude Code marketplace maturing.

**Recommendation:** Defer until A1 ships and demand signals emerge.

#### C4 — Internal regression suite + workflow score

**Source sections:** 08 emergent.

**What:** A canonical task fixture suite Ark runs end-to-end. A "workflow score" rubric (tier sizing, iteration convergence, VERIFY catch rate, SPEC quality, cross-platform parity).

**Why:** Self-validation. Marketing artifact. Foundation for any future cross-harness benchmark.

**Confidence:** Medium. The pattern is sound; the specific fixture set requires curation.

**Recommendation:** Worth a dedicated RFC; lower-priority than the operational moves above.

#### C5 — Event log Stage 2: trajectories for fine-tuning / cross-vendor agent process model

**Source sections:** 08 emergent, 00 landscape (Agent OS visions).

**What:** Extend the event log (B2) to capture fuller trajectories — tool calls, model decisions, intermediate observations. Use as training data for a hypothetical Ark-tuned model or as a portable agent process model.

**Why:** Long-term aspiration aligning with the "agent OS" framing in RFC 0001.

**Confidence:** Low. Depends on standards emergence + clear use case.

**Recommendation:** RFC-track; don't ship until use case becomes concrete.

### Tier D — Skip or postpone indefinitely

Directions that surfaced but where the corpus suggests caution.

#### D1 — Build embedding-based code RAG

**Why skip:** F7. The field is converging away from this. Building it now would be moving against the consensus.

#### D2 — Build Ark's own plugin marketplace

**Why skip:** Fragmentation risk. Riding existing ecosystems (Claude Code marketplace, OpenAI's skills, Cursor's awesome list) is better.

#### D3 — Heavy IDE extensions (JetBrains, VS Code)

**Why postpone:** MCP-first delivers most of the value at lower cost. Ship IDE extensions only on demand signal.

#### D4 — "Agent OS" shipping copy

**Why skip:** F11 + 00 landscape. The category is full of premature claims. Stay positioned as a harness.

#### D5 — Multi-agent debate / consensus

**Why skip:** Documented accuracy degradation (Sep 2025 paper). The evaluator-optimizer pattern (single reviewer, Ark's current) outperforms.

---

## Recommended near-term roadmap

If pushed for a single sentence: **ship `ark-mcp` (A1) + always write AGENTS.md (A2) + the documentation pages (A3 + B5) + the codemap (A5)**. Everything else is downstream of these.

A four-quarter sketch:

**Q3 2026:**
- A1 — `ark-mcp` (3 weeks)
- A2 — AGENTS.md universal (3 days)
- A3 — positioning + threat-model + integration-surfaces docs (1 week)
- A4 — subagent tools allow-list (3 days)
- A6 — cost-estimate surface (3 days)
- A5 — `ark codemap` (2 weeks; possibly Q4)

**Q4 2026:**
- A5 finish if not done.
- B1 — slash-command → skill pilot (1 week)
- B2 — event log Stage 1 (3 weeks)
- B4 — `task commit --lint/--test` (1 week)
- B5 — three docs pages (1 week)

**Q1 2027:**
- B3 — single canonical source for templates (3 weeks).
- Begin C-tier work: C1 (ACP tracking decision), C2 (per-phase models), C4 (internal regression suite RFC).

**Q2 2027:**
- C1/C2/C3/C4/C5 as the field clarifies.

This is one possible roadmap; the priorities will move as the field moves. The findings (F1–F11) are what stay stable.

---

## What this corpus would have said if it could only say one thing

> **Ark is the canonical workflow-opinionated harness layer.** The corpus's findings — harness quality is load-bearing (F1), skills + AGENTS.md + MCP are the converging cross-platform substrate (F2/F3/F4), structured artifacts beat ephemeral memory (F5), sub-agent dispatch is context engineering (F6), codemap+JIT beats RAG (F7), event logs unify the backing store (F8), pre-edit safety is universally good (F9), cost UX is undersold (F10), tiered ceremony is Ark's defensible differentiator (F11) — all point the same direction: *keep doing what Ark does, expose it through MCP, document it, and add the pre-edit safety + cost-UX features that close the field's other gaps*.

The single concrete recommendation: **ship `ark-mcp` next. Everything else builds on it.**

---

## Process notes (for the next research-tier run)

This corpus was generated by dispatching 9 parallel `ark-researcher` sub-agents. Lessons for next time:

1. **Cap per-dispatch scope at ~2-3 files.** 7-8-file dispatches hit watchdog timeouts (~10 minutes) in 4 of 9 cases. Smaller scope per dispatch increases completion rate.
2. **Foreground dispatch + disk persistence saves work.** Even when 4 agents stalled, partial files on disk survived. The main session recovered by inspecting disk and writing the remainder.
3. **Re-dispatch should be idempotent / aware of partial state.** Today's re-dispatch overwrites without inspecting; the recovery had to be partly manual.
4. **Sub-agent return summary is courtesy.** Disk is truth. This is the architectural insight to encode in `subagent-support` SPEC.
5. **A simple "expected outputs declared up front" contract would have made recovery cleaner.** Each dispatch should declare its expected files; the parent verifies on return.

These map to direction A4 (declarative scope), B2 (event log capturing dispatches), and to a proposed direction not in the ranking: **document the disk-as-truth pattern in `subagent-support` SPEC**.

---

## Cross-reference index

For traceability, each direction in the ranking traces back to one or more source files:

- A1 (`ark-mcp`): `01_prior_art/INDEX.md` cross-cutting #3; `06_platform_integration/` (slash-vs-cli-vs-mcp, claude/codex/opencode deep-dives); `05_orchestration/agent-to-agent-protocols.md`; `02_infra_primitives/mcp-and-tool-registries.md`.
- A2 (AGENTS.md universal): `00_landscape/market_map.md` "Where the puck is going" #1; `06_platform_integration/cross-platform-portability.md`.
- A3 (docs): `00_landscape/definitions.md`; `06_platform_integration/`; `08_emergent/security-and-threat-model.md`.
- A4 (tools allow-list): `06_platform_integration/claude-code-integration-deep-dive.md`; `05_orchestration/subagent-isolation-and-context.md`.
- A5 (`ark codemap`): `03_context_engineering/codemaps-and-repo-structure-summaries.md`; `01_prior_art/aider.md`, `continue-dev.md`.
- A6 (cost estimates): `07_developer_ux/pricing-cost-and-budget-ux.md`; `08_emergent/agent-economics-and-cost-ux.md`.
- B1 (skill migration): `06_platform_integration/claude-code-integration-deep-dive.md`; `00_landscape/market_map.md`.
- B2 (event log): `08_emergent/trajectory-and-event-log-architecture.md`; `02_infra_primitives/observability-and-telemetry.md`; `01_prior_art/INDEX.md` cross-cutting #7.
- B3 (canonical source): `06_platform_integration/cross-platform-portability.md`; `06_platform_integration/plugin-and-extension-ecosystems.md`.
- B4 (lint/test gates): `08_emergent/lint-before-commit-and-pre-edit-safety.md`; `01_prior_art/swe-agent.md`.
- B5 (docs): cross-corpus.
- C1 (ACP): `00_landscape/market_map.md`; `05_orchestration/agent-to-agent-protocols.md`.
- C2 (per-phase models): `08_emergent/agent-economics-and-cost-ux.md`; `03_context_engineering/jit-and-progressive-context-loading.md`.
- C3 (Claude plugin bundle): `06_platform_integration/plugin-and-extension-ecosystems.md`.
- C4 (regression suite): `08_emergent/evaluation-for-harnesses-not-models.md`.
- C5 (event log Stage 2): `08_emergent/trajectory-and-event-log-architecture.md`; `08_emergent/agent-os-visions.md`.

Every direction is grounded in corpus evidence; nothing originated from speculation. The roadmap is as defensible as the corpus.
