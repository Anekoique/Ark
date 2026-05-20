# Spec-Driven Development — The 2026 Landscape

## What "spec-driven" means

The phrase covers four overlapping ideas:

1. **Specs are the source of truth, not code.** Code is regeneratable; specs are not. (Spec-kit's "power inversion".)
2. **Intent before edits.** Write what you want, run a workflow, then accept the diff.
3. **Specs are executable.** Tests + acceptance criteria + structured prose generate (or validate) implementation.
4. **Specs are durable.** Live in-repo as versioned markdown, not chat history.

These ideas all predate 2025 (RFCs, ADRs, PRDs, model-driven engineering, BDD). What's new is **agent-driven authoring** — the LLM is the one drafting specs, and the spec is loaded back into the LLM's context to drive implementation. This is what "spec-driven development" (SDD) refers to in 2026 vocabulary.

## Movement or label?

A real movement, but factional. Four schools in 2026:

| School | Flag-carrier | Where artifacts live | Promotion model |
| ------ | ------------ | -------------------- | --------------- |
| **Constitutional / phase-gated** | GitHub spec-kit (`/speckit.*`) | `specs/<feature>/` per-feature dirs | Phase-gate checklist; constitution overrides |
| **Change-proposal / delta-merge** | OpenSpec (`/opsx:propose`) | `openspec/changes/<id>/` then archive into `openspec/specs/` | Archive merges deltas (ADDED/MODIFIED) into source-of-truth specs |
| **Three-phase IDE** | Kiro (AWS, GA 2025) | `.kiro/specs/<feature>/` | requirements.md → design.md → tasks.md staircase with hooks |
| **Progressive wiki** | Trellis | `.trellis/spec/` + `.trellis/tasks/` | Promote reusable lessons back into specs; per-task PRDs |

Ark sits between OpenSpec and Trellis: PRD-per-task (Trellis), promote-on-deep-commit (OpenSpec delta-merge analogue), but with the twist that **only deep tier promotes**.

## The four spec-driven projects in detail

### GitHub spec-kit (`github/spec-kit`)

- Repo: <https://github.com/github/spec-kit>
- Branding: "Specify CLI" with `specify init` scaffolder.
- Slash commands: `/speckit.constitution`, `/speckit.specify`, `/speckit.plan`, `/speckit.tasks`, `/speckit.implement`.
- Companion essay: `spec-driven.md` in repo (≈580 lines, see `reference/spec-kit/spec-driven.md`).
- Position: **specifications generate code, not the other way around.** "Code becomes its expression in a particular language and framework."

Workflow:

1. `/speckit.constitution` — establish project principles (`memory/constitution.md`, 9 articles).
2. `/speckit.specify` — feature spec from natural language; auto-numbers + creates branch.
3. `/speckit.plan` — technical plan; constitutional-compliance gates ("Simplicity Gate", "Anti-Abstraction Gate").
4. `/speckit.tasks` — break plan into executable tasks.
5. `/speckit.implement` — execute.

Phase gates are checklist-driven; the LLM must explicitly justify violations in a "Complexity Tracking" section. Quote from `spec-driven.md`:

> These gates prevent over-engineering by making the LLM explicitly justify any complexity. If a gate fails, the LLM must document why in the "Complexity Tracking" section, creating accountability for architectural decisions.

Extension ecosystem in `extensions/catalog.community.json` (40+ contributions): archive, cleanup, fix-findings, conduct (sub-agent delegation), checkpoint, fleet orchestrator. The phase-gate model is heavyweight; extensions soften it.

OpenSpec's README cites spec-kit as "thorough but heavyweight. Rigid phase gates, lots of Markdown, Python setup. OpenSpec is lighter and lets you iterate freely."

### OpenSpec (`Fission-AI/OpenSpec`)

- Repo: <https://github.com/Fission-AI/OpenSpec>
- npm: `@fission-ai/openspec`.
- Philosophy (from README, `reference/OpenSpec/README.md` lines 30-36):
  - "fluid not rigid"
  - "iterative not waterfall"
  - "easy not complex"
  - "built for brownfield not just greenfield"
  - "scalable from personal projects to enterprises"

Workflow (rebuilt as "opsx" in late 2025):

```
/opsx:propose <id>     → openspec/changes/<id>/{proposal.md,specs/,design.md,tasks.md}
/opsx:apply            → implement tasks
/opsx:archive          → move to openspec/changes/archive/<date>-<id>/, merge spec deltas into openspec/specs/
```

The **change proposal / delta-merge** model is distinct: each change has its own folder with proposal + design + delta-specs + tasks. On archive, ADDED/MODIFIED deltas merge into the canonical specs tree.

Profiles: `openspec config profile` switches between `default` and `expanded` (`/opsx:new`, `/opsx:continue`, `/opsx:ff`, `/opsx:verify`, `/opsx:sync`, `/opsx:bulk-archive`, `/opsx:onboard`). This is the closest analogue to Ark's tiers.

Quote on positioning vs Spec Kit (README): "Thorough but heavyweight. Rigid phase gates, lots of Markdown, Python setup. OpenSpec is lighter and lets you iterate freely."

Telemetry is opt-out (`OPENSPEC_TELEMETRY=0`); model recommendation in `Usage Notes`: Opus 4.5 and GPT 5.2.

### Kiro (AWS)

- IDE (VS Code fork), GA 2025, in-IDE only — `kiro.dev/docs/specs/`.
- Three-phase workflow:
  1. **Requirements / bug analysis** — user stories + acceptance criteria.
  2. **Design** — technical architecture, schemas, diagrams.
  3. **Implementation tasks** — trackable sequence.
- Hooks at each phase boundary; diff-review gate before applying changes.
- Property-based test generation as a quality lever.

OpenSpec README's compare line: "Powerful but you're locked into their IDE and limited to Claude models."

Kiro's contribution is **forcing the staircase**: you cannot skip to design without requirements, cannot skip to implementation without design. Ark's PLAN gate ("every `G-N` mapped to ≥1 `V-*-N`") is a softer version.

### Trellis (`mindfold-ai/Trellis`)

- Repo: <https://github.com/mindfold-ai/Trellis>
- npm: `@mindfoldhq/trellis@beta`
- Self-description (README line 10): "Make AI coding reliable at team scale. A team AI coding harness for progressive specs, custom workflows, task context, and memory across Claude Code, Cursor, Codex, OpenCode, Pi Agent, and more."

Layout (README lines 49-56):

| Layer                  | Purpose                                                                                |
| ---------------------- | -------------------------------------------------------------------------------------- |
| `.trellis/spec/`       | Team standards and coding guidelines that agents can load automatically.               |
| `.trellis/tasks/`      | PRDs, task context, status, review notes, and acceptance criteria.                     |
| `.trellis/workspace/`  | Developer-level journals, decisions, and handoff notes for session continuity.         |
| `.trellis/workflow.md` | The shared lifecycle for planning, building, checking, finishing, and learning.        |
| Platform adapters      | Generated commands, hooks, skills, prompts, workflows, and agent files for your tools. |

Loop (README lines 58-66):

1. Capture the task as a PRD.
2. Inject the relevant project specs.
3. Let the agent implement inside a clear boundary.
4. Run checks before handoff.
5. Promote reusable lessons back into specs.
6. Record the session so the next agent starts with the decisions and context it needs.

This is **the closest published harness to Ark's model.** PRD-per-task, project specs as conventions, workspace journals, platform adapters. Differences:

- Trellis is JS/TS + Python hooks; Ark is Rust with embedded templates.
- Trellis is "skill-first" in 0.5+; Ark is slash-command-first.
- Trellis has no tier system — one ceremony for everything.
- Trellis spec promotion is "promote reusable lessons" (manual); Ark extracts SPEC verbatim from deep PLAN at commit.

The behavioural guidelines in `reference/Trellis/CLAUDE.md` are notable — they bias agents toward "minimum code that solves the problem", "surgical changes", and "goal-driven execution with verifiable success criteria". Ark has nothing equivalent installed in shipped templates.

## Anthropic / OpenAI internal practice

Less visible — most signals are sub-agent shape and tool-call defaults.

- **Anthropic Code Review** (March 2026) — a multi-agent code-review system that dispatches a fleet of specialised agents against a diff, then runs a verification step to filter out false positives with a confidence-score gate (default 80/100). Configurable per-repo. <https://thenewstack.io/anthropic-launches-a-multi-agent-code-review-tool-for-claude-code/>
- **Anthropic Skills** (Late 2025) — declarative `.skill` files in `~/.claude/skills/`. Conceptually overlap with spec-kit's per-feature spec dirs; in practice are more like agent-side commands than spec artifacts.
- **OpenAI AGENTS.md / Codex** — `AGENTS.md` replaces per-repo coding guidelines. The blog post "AGENTS.md is the New Architecture Decision Record (ADR)" frames it as the agent-readable counterpart to ADRs. <https://ai.gopubby.com/agents-md-is-the-ew-architecture-decision-record-adr-3cfb6bdd6f2c>

Neither vendor publishes a formal "spec-driven" workflow; both ship the *runtime* and let frameworks like spec-kit / OpenSpec / Trellis / Ark sit on top.

## The "Specstack" framing

A loose term used in 2026 to describe the bundle of:

- spec format (markdown with conventional sections)
- spec storage (per-feature dir, per-change dir, or single tree)
- spec lifecycle (draft → reviewed → applied → archived)
- spec validation (lint, schema check, completeness check)
- spec promotion (change → canonical specs)

No single "Specstack" project exists — it's the umbrella for the four-school landscape above. The convergence direction:

- Markdown with conventional sections (consistent across all four).
- Per-feature dirs (spec-kit, Kiro) vs per-change dirs (OpenSpec) vs per-task dirs (Trellis, Ark).
- Promotion at archive/commit (OpenSpec, Ark) vs no promotion (spec-kit, Kiro, Trellis).

## Ark's lineage

Ark's README and `reference/` directory cite **trellis + openspec + spec-kit** as the projects it draws from. Concrete inheritances:

| From | Ark feature |
| ---- | ----------- |
| Trellis | `.ark/` directory layout (`workflow.md`, `templates/`, `tasks/`, `specs/`), per-task PRD, "promote lessons back into specs" model |
| OpenSpec | Change-as-a-folder pattern (each task is a folder), profile-style tier selection (OpenSpec has default/expanded; Ark has quick/standard/deep/research) |
| Spec-kit | Phase-gate vocabulary (PRD/PLAN/REVIEW/VERIFY parallels constitution/specify/plan/tasks/implement) |

Ark's distinctive additions:

- **Tier-gated SPEC extraction** — only deep tier produces a SPEC. Other tools either promote every change (OpenSpec) or never auto-promote (spec-kit, Kiro, Trellis).
- **`G-N → V-N` Acceptance Mapping as gate** — every Goal in PLAN must map to a Verify item before PLAN → EXECUTE transition. Closest analogue: spec-kit's constitutional gates.
- **Research as a separate tier** — corpus-as-deliverable lifecycle is unique. OpenSpec / Trellis support research notes inside a change; Ark gives it its own three-phase lifecycle.

## Is "spec-first" real in 2026?

Yes — but only in agentic contexts. Pre-agent SDD (model-driven engineering, ADRs, RFCs) was always optional; agents make it functionally mandatory because:

1. Agents need an in-repo, version-controlled source of intent (chat history is lost on session end).
2. Specs reduce hallucinated requirements during long-running tasks.
3. Specs are the only durable artifact when the agent rewrites all code.

GitHub spec-kit's adoption (open-source repos, community extensions catalog with 40+ entries) and OpenSpec's npm downloads suggest real traction. Kiro's GA at AWS re:Invent 2025 signals vendor commitment. The pattern is here to stay; the disagreement is over **how heavy** the ceremony should be.

## Directions for Ark

1. **Document the lineage explicitly.** Ark's README mentions trellis/openspec/spec-kit in passing. A `docs/lineage.md` (or section in `README.md`) showing what we kept from each and what we diverged on would help positioning. Candidate inputs: Trellis's `CLAUDE.md` behavioral guidelines (cited above) as a model for terseness.
2. **Surface "spec-driven" as a project tagline.** Currently Ark calls itself "an agent harness". The spec-driven-development frame is mature enough in 2026 that explicit alignment ("a tiered spec-driven workflow for agent harnesses") would clarify positioning vs Trellis (no tiers) and spec-kit (one ceremony).
3. **Profile system parity with OpenSpec.** OpenSpec ships `default` and `expanded` profiles selectable via `openspec config profile`. Ark's tiers are per-task; we could ship a project-level *profile* (quick-only, standard+, full) that defaults `/ark:design --deep` or hides it. Useful for solo developers who want quick-only.
4. **Acceptance-Mapping lint.** The `G-N → V-N` rule is enforced verbally in `workflow.md`. A real check (CLI or hook) that parses the PLAN and reports unmapped Goals would harden the gate. Cite: spec-kit's constitutional-compliance checklists are similar.
5. **"Detachable" SPEC vs delta-merge.** The recent `detachable-feature-spec` commit changed how SPEC bodies separate from PLAN bodies. Worth comparing the delta-merge model (OpenSpec) against Ark's "PLAN's Spec is verbatim the SPEC" model in a follow-up SPEC. Trade-off: delta-merge supports brownfield SPEC evolution; verbatim supports clean point-in-time snapshots.

Sources:

- [GitHub spec-kit](https://github.com/github/spec-kit) — repo + `spec-driven.md` essay
- [Fission-AI/OpenSpec](https://github.com/Fission-AI/OpenSpec) — README at `reference/OpenSpec/README.md`
- [Trellis (mindfold-ai)](https://github.com/mindfold-ai/Trellis) — README at `reference/Trellis/README.md` + `CLAUDE.md`
- [Kiro Specs docs](https://kiro.dev/docs/specs/)
- [Anthropic Code Review launch](https://thenewstack.io/anthropic-launches-a-multi-agent-code-review-tool-for-claude-code/) (March 2026)
- [InfoQ on Kiro](https://www.infoq.com/news/2025/08/aws-kiro-spec-driven-agent/) — "Beyond Vibe Coding: Amazon Introduces Kiro" (Aug 2025)
- [OpenSpec vs Spec Kit (Hashrocket)](https://hashrocket.com/blog/posts/openspec-vs-spec-kit-choosing-the-right-ai-driven-development-workflow-for-your-team)
- [AGENTS.md is the New ADR (gopubby)](https://ai.gopubby.com/agents-md-is-the-ew-architecture-decision-record-adr-3cfb6bdd6f2c)
