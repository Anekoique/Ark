# 03 — Context Engineering

Compiled 2026-05-20. The discipline of allocating the finite context window to maximise the next-step decision quality. Anthropic operationalised the term in *Effective Context Engineering* (Sep 2025); Karpathy coined the framing mid-2025. As of 2026 every shipping harness has an explicit context-engineering strategy.

The organising question this section asks: **does Ark's bet on structured artifacts + phase projection hold up against RAG, codemaps, JIT loading, and compaction — or should it absorb some of those primitives?**

> Ark today: no RAG, no embeddings, no codemap. Context is delivered via `ark context --scope phase --for <phase> --format json` (a structured projection) + the `@.ark/specs/INDEX.md` reference in CLAUDE.md. Memory is file-backed (CLAUDE.md, AGENTS.md, project SPECs, feature SPECs, journals).

## Files

| File | One-line takeaway |
| ---- | ----------------- |
| [`context-window-management.md`](context-window-management.md) | The 2026 frame is *attention budget management*, not bigger windows. Lost-in-the-middle and context rot make effective context << capacity. Strategies: auto-compact, sliding window, summarising window, JIT loading, sub-agent isolation. Ark's phase projection IS a context-management strategy disguised as a workflow primitive. |
| [`rag-for-codebases.md`](rag-for-codebases.md) | RAG-for-code (Sourcegraph Cody, early Continue) fell out of favour 2024→2026. Code embeddings are brittle (names ≠ semantics). The replacement: codemaps + grep + JIT read. Ark's no-RAG stance is now the consensus. |
| [`codemaps-and-repo-structure-summaries.md`](codemaps-and-repo-structure-summaries.md) | Aider's PageRank-over-tree-sitter repo-map is the prototype; Continue, Plandex, Cursor ship variants. Doc-as-source-of-truth (the `docs/CODEMAPS/` pattern, Ark's `doc-updater` agent rule). Generation strategies, refresh policies. Where Ark could ship a `codemap` subcommand. |
| [`jit-and-progressive-context-loading.md`](jit-and-progressive-context-loading.md) | Just-in-time context — agent pulls files via tools rather than loading upfront. Claude Code's default. Cheaper than RAG, more responsive, but burns turn-count. Ark's phase projection is "static JIT" — pre-curated, not retrieved. |
| [`structured-summaries-and-projections.md`](structured-summaries-and-projections.md) | The pattern Ark embodies: deliver a *session-orientation packet* (curated, schemaed, versioned) rather than raw chunks. Spec-kit, OpenSpec, Continue Hub all converging here. Ark's `ark context` is the most mature concrete instance — JSON schema, scope/phase variants, semver-stable surface. |
| [`compaction-and-handoff.md`](compaction-and-handoff.md) | Long-session strategies: auto-compact (Claude Code), explicit handoff via sub-agent dispatch (Ark, Cline), checkpoint-and-resume. What survives boundaries, what doesn't. Ark's research-tier sub-agent dispatch IS a handoff mechanism that bypasses compaction entirely. |
| [`memory-vs-context.md`](memory-vs-context.md) | Distinction: context = ephemeral, in-prompt; memory = durable, file-backed. CLAUDE.md / AGENTS.md as always-loaded memory; auto-memory (`/remember`) as runtime-accreted memory; vector memory (mem0, Letta/MemGPT) as semantic memory. Ark's project + feature SPECs are *structured* memory — the rare path. |

## Cross-cutting findings

1. **Effective context << capacity.** 1M-token windows ship; useful recall windows are 32K–128K. MRCR v2 / RULER measurements show every model degrades long before its declared cap. *Treat declared window size as marketing, effective window as engineering.*

2. **Three layers of context allocation matter more than retrieval algorithm:** (a) *always-loaded* (CLAUDE.md, AGENTS.md), (b) *curated for this session* (`ark context`, OpenSpec projection), (c) *just-in-time* (tool calls). Most harnesses ship (a) + (c); workflow-opinionated harnesses also ship (b). Ark is the canonical (b).

3. **RAG-for-code is a dead end as a primary strategy.** Code embeddings underperform symbol indexes + grep + JIT for variable-name-driven retrieval. Aider documented this in 2023; the consensus solidified through 2024–2025. Sourcegraph Cody pivoted away. Continue's 2026 pivot dropped vector retrieval as central.

4. **Sub-agent dispatch is a context-management tool, not just an orchestration tool.** Spinning a child agent with a fresh window means the *parent* keeps its tokens for higher-order decisions. Ark's `ark-researcher` and `ark-reviewer` are doing context engineering, not just specialisation.

5. **Compaction is brittle; explicit handoffs are robust.** Auto-compact summarises, which loses fidelity. Sub-agent dispatch + disk persistence loses *nothing* — the corpus survives intact. Ark's research-tier pattern (corpus IS the deliverable, written to disk) is more robust than any auto-compact strategy.

6. **The "session-orientation packet" pattern is converging.** Spec-kit, OpenSpec, Continue Hub, Ark's `ark context` all deliver a curated snapshot at session start. Vocabulary differs (spec-kit "spec stack", OpenSpec "projection", Ark "context"); shape is identical: JSON-schemaed, versioned, machine-readable.

## Where Ark already aligns

- **No RAG.** Avoids the dead end the field is converging away from.
- **Phase projection (`ark context`).** Canonical instance of the session-orientation packet pattern.
- **Structured memory (project + feature SPECs).** Hits the rare third layer — most harnesses only have always-loaded + JIT.
- **Sub-agent dispatch for context isolation.** Ark-researcher/-reviewer/-verifier all use fresh-context children that persist to disk.
- **Disk-as-channel.** Researcher writes markdown; parent reads it back. Bypasses compaction.

## Where Ark could differentiate or close gaps

- **No codemap.** This is the biggest gap. Aider, Continue, Cursor, Plandex all ship some form of repo-map. Ark could ship `ark codemap` as a side-effect-free generator + a `docs/CODEMAPS/` convention.
- **No effective-window awareness.** Ark does not advise on which model to use or which phase fits which window. A `task.toml.context_budget` field could surface this.
- **No compaction guidance.** Ark's deep tier accumulates many `NN_PLAN.md` / `NN_REVIEW.md` files; effective context can shrink mid-iteration. A documented "what to summarise into the latest plan" pattern would help.
- **No memory hierarchy convention.** CLAUDE.md / AGENTS.md / project SPECs all exist but their interaction is undocumented. A `memory-vs-context.md`-style "where does what live" doc shipped in `docs/book/src/` would help users.

## Reading order

1. `context-window-management.md` — the substrate (what context actually does, what falls off).
2. `memory-vs-context.md` — the distinction (durable vs. ephemeral).
3. `structured-summaries-and-projections.md` — Ark's pattern at one level of abstraction.
4. `rag-for-codebases.md` — the dead end (and why Ark avoiding it is correct).
5. `codemaps-and-repo-structure-summaries.md` — the live alternative to RAG.
6. `jit-and-progressive-context-loading.md` — the other live alternative.
7. `compaction-and-handoff.md` — what to do when sessions get long.

Cross-references:
- `02_infra_primitives/memory-systems.md` — adjacent treatment of memory.
- `02_infra_primitives/sessions-state-and-resumption.md` — how sessions are bounded.
- `05_orchestration/subagent-isolation-and-context.md` — sub-agents as context-management tool.
- `04_workflow_systems/spec-driven-development.md` — workflow side of the structured-summary pattern.

## Directions for Ark (section-level)

1. **Ship `ark codemap`.** The biggest section-level gap. A side-effect-free tree-sitter-based symbol map written to `docs/CODEMAPS/<module>.md`, refreshable on a hook. Cheap to implement (tree-sitter crates exist in Rust), high leverage (closes the gap with every peer).

2. **Document the memory hierarchy.** CLAUDE.md / AGENTS.md / project SPECs / feature SPECs / journals all exist; their interaction is implicit. A `docs/book/src/reference/memory-hierarchy.md` page would close the documentation gap.

3. **Add `--effective-window` advisory to `ark context`.** Phase-by-phase, name the effective window each phase needs and the model class it implies. Lets the agent pick the right model at the right step.

4. **Promote sub-agent dispatch as a context-management strategy in `workflow.md`.** Today the workflow doc treats `ark-researcher` as a specialisation. Renaming it "context-isolation specialisation" and adding a one-paragraph rationale aligns the doc with the actual mechanism.

5. **Surface compaction guidance for deep-tier iteration.** When `NN_PLAN.md` reaches N=4+, the cumulative deep-tier context risks lost-in-the-middle. Document the "self-contained `## Spec` every iteration" rule (already in `workflow.md`) as a context-engineering decision, not just style.
