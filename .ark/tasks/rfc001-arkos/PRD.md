# `rfc001-arkos` PRD

---

[**What**]

Add `docs/rfcs/001-arkos.md` — the first numbered RFC under a new `docs/rfcs/` directory — that positions ArkOS as a substrate for agents (a workflow-native common runtime, OS-shaped only metaphorically), names Ark and ArkOS as sibling substrates at the same architectural layer (human-audience vs. agent-audience), describes ArkOS's two-stage evolution (stage-1 hosts existing agent runtimes including Claude Code / Codex / OpenCode and bootstraps on Ark's harness primitives; stage-2 grows native runtime capacity), and grounds the substrate's self-improvement claim in workload-outcome feedback rather than self-evaluation.

[**Why**]

ArkOS is being bootstrapped in a separate repository (`reference/arkos` locally, future `Anekoique/arkos`) as a substrate for LLM/agent execution — workflow primitives (lifecycle, task tree, memory, SPEC storage, grounding-signal hooks, recursion discipline) provided as common services to agents and their orchestrators. The substrate framing — distinct from "autonomous orchestrator running on top of Ark" — emerged from the design discussion preceding this PRD and reorganizes what the RFC is for.

Three risks justify writing the RFC now rather than after ArkOS code lands:

1. **Positioning ambiguity.** Without a written substrate-framing, readers (including future contributors and external evaluators) will conflate ArkOS with the orchestrator-on-Ark reading the conversation already corrected once. A public anchor prevents the same misread recurring.
2. **Self-improvement claim is the riskiest part.** The research (`research/self-improving-agents.md`, `research/self-generating-specs.md`) documents that every prior recursive-self-improvement attempt without external grounding has reward-hacked, drifted, or looped (DGM hallucinated tool use; AI Scientist bypassed its wall-clock budget; AutoGPT looped infinitely; Augment's 2026 measurement shows auto-generated agent rules actively reduce task success). ArkOS's "substrate self-improves" claim must be grounded in workload outcomes, not self-judgment. Writing this discipline down before implementation prevents the easy slide into the documented failure modes.
3. **Ark identity preserved by separation.** Ark stays a human-in-the-loop CLI harness; ArkOS is a separate substrate at the same architectural layer with an agent audience. The RFC documents the sibling-substrate relationship so neither project's design pressure contaminates the other.

The RFC's contribution is positioning and honest acknowledgment of where prior art lands. ArkOS's substantive design — the specific shape of substrate primitives, the recursion model used by applications running on it, the SPEC-generation mechanism — lives in ArkOS's own repository when it stabilizes. This RFC is the Ark-side framing only.

[**Outcome**]

- `docs/rfcs/001-arkos.md` exists, ~400–600 lines, sectioned per the agreed outline: Status / Summary / Motivation / Layered model / Ark's identity / ArkOS — what it is / ArkOS — what it provides / Self-improvement model / Two-stage evolution / Relationship to Ark / Out of scope / Open questions / Prior art / Phased delivery / References.
- `docs/rfcs/` directory is established as the canonical location for future numbered RFCs (three-digit prefix per user direction, kebab-case slug).
- The RFC uses workflow-native vocabulary throughout — "substrate," "service," "primitive," "lifecycle," "grounding" — and explicitly avoids OS-technical jargon (no "syscalls," "kernel-equivalent," "scheduler," "memory manager," "process abstraction"). The OS framing appears only as a positioning metaphor in title/summary, not as design vocabulary.
- The Self-improvement section names the grounding-signal dichotomy from the research (external/independent fitness signals converge; self-judging loops drift) and commits ArkOS's substrate evolution to workload-outcome grounding, with explicit citations to the literature failures the discipline avoids (DGM reward-hacking, self-preference bias, AutoGPT loops).
- The Open Questions section enumerates substrate-level open questions (workload-grounded self-improvement discipline against Goodhart, agent-discoverability of substrate services, stage-1 runtime-dependency stability, intermediate-artifact grounding, recursive-context reconciliation) — without resolving them.
- The RFC explicitly excludes applications running on ArkOS (specific orchestrators, the POSIX-OS workload framing, decomposition algorithms used by applications, autonomy-vs-gates philosophy at the application layer). Those scope to ArkOS's own design documents.
- The References section links to all three research files at `.ark/tasks/rfc001-arkos/research/*.md` (which persist in the task archive) plus primary citations from each.
- No Rust source code changes; no test changes; no `templates/` or `.ark/` template changes; no `README.md` or `AGENTS.md` edits. Documentation-only delivery of a single file at `docs/rfcs/001-arkos.md`. If post-RFC identity-pinning in README/AGENTS becomes useful, a follow-up task handles it.

[**Related Specs**]

- `specs/features/ark-context/SPEC.md` — Stage-1 evolution notes that ArkOS reads Ark's CLI surfaces (including `ark context --format json`) when hosted alongside Ark; the SPEC's `SCHEMA_VERSION` is the existing stability mechanism. No SPEC change; the RFC is downstream context.
- `specs/features/ark-agent-namespace/SPEC.md` — Currently states `ark agent` is not semver-stable. The RFC's Stage-1 section frames ArkOS as bootstrapping on Ark's harness *primitives* (workflow lifecycle, task tree, SPEC storage) rather than calling `ark agent` as an external programmatic interface. Whether stage-1 ArkOS needs a stability commitment on `ark agent` is an Open Question, not a foregone yes; the RFC names this rather than resolving it. No SPEC CHANGELOG entry is required as part of this RFC delivery.
- `specs/features/ark-workflow-refactor/SPEC.md` — The RFC reinforces the workflow's human-gated philosophy for Ark while naming ArkOS as the agent-audience sibling. No SPEC change.
- `specs/features/subagent-support/SPEC.md` — Researcher/reviewer/verifier subagents are workflow primitives ArkOS may absorb in stage 2. Named in Two-stage Evolution as a future direction; no immediate SPEC change.
