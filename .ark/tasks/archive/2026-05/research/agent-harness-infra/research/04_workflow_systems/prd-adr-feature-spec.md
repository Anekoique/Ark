# PRDs, ADRs, RFCs, Feature SPECs — The Artifact Zoo

## Why so many artifact types

Different intents need different shapes:

- **What to build** — PRD.
- **Why we chose X over Y** — ADR.
- **Proposal for review by peers** — RFC.
- **Specifications of how a feature behaves** — feature SPEC.
- **Where in the codebase / who owns / how to maintain** — `AGENTS.md` / `CLAUDE.md`.

In agent contexts, all five are markdown files in the repo. The difference is the *role they play in the workflow*, not the file format.

## Pioneers and origins

| Artifact | Origin | When |
| -------- | ------ | ---- |
| PRD (Product Requirements Document) | Marty Cagan, *Inspired* (2008); preceded by product management practice at HP, Microsoft | 1990s–2008 codification |
| ADR (Architecture Decision Record) | Michael Nygard, "Documenting Architecture Decisions" (2011) | 2011 |
| RFC (Request for Comments) | Steve Crocker, ARPANET (1969); revived as engineering proposal format by Rust/Kubernetes/Django ~2014 | 1969 / 2014 |
| Design Doc | Google internal practice; later popularised externally | 2000s |
| Feature SPEC | Practice term, no single inventor; codified by spec-kit/OpenSpec/Ark in 2024-2026 | 2024+ |

In agent contexts, the relevant rebirth dates are 2023+ — when spec-driven dev tooling started forcing these artifacts into the agent loop.

## PRD — "what to build"

Classic structure (Cagan-style):

- Problem / pain point
- Users / personas
- Solution overview
- Requirements (functional + non-functional)
- Success metrics
- Out of scope

Spec-kit's `/speckit.specify` produces a feature spec that overlaps with PRD. From `reference/spec-kit/spec-driven.md`:

> The AI asks clarifying questions, identifies edge cases, and helps define precise acceptance criteria. What might take days of meetings and documentation in traditional development happens in hours of focused specification work.

OpenSpec's `proposal.md` is also PRD-shaped — "why we're doing this, what's changing." From OpenSpec README:

```
✓ proposal.md — why we're doing this, what's changing
✓ specs/       — requirements and scenarios
✓ design.md    — technical approach
✓ tasks.md     — implementation checklist
```

Kiro's `requirements.md` (phase 1) is the closest 1:1 to a classic PRD — user stories with detailed acceptance criteria.

Ark's `PRD.md` is required at every tier. Sections from the template:

- **What** — what we're building.
- **Why** — motivation.
- **Outcome** — measurable success criteria.
- **Related Specs** — feature SPECs the task touches (used by VERIFY).
- **SPEC Path** — deep tier only; relative path under `specs/features/` for promotion.

Notably terse — closer to a one-page brief than a Cagan PRD. Designed to be a session-startup orientation doc for the agent, not a stakeholder doc.

## ADR — "why we chose X over Y"

Nygard's original template:

- Title
- Status (proposed / accepted / superseded)
- Context
- Decision
- Consequences

Live in `docs/adr/` numbered sequentially (0001, 0002, …). Each ADR is immutable once accepted — superseded ADRs link to the new one rather than being edited.

Adoption is patchy in the agent era:

- **AGENTS.md as the new ADR.** From <https://ai.gopubby.com/agents-md-is-the-ew-architecture-decision-record-adr-3cfb6bdd6f2c>:
  > While traditional ADRs answer "What did we decide, and why?", AGENTS.md answers "Given what we decided, what must never happen, and what must always happen, when changing this code?"

  This conflates two things: ADRs are point-in-time decisions; AGENTS.md is a perpetually-evolving rulebook. But for agent contexts, the latter is more actionable.
- **AI-generated ADRs.** Repos like `macromania/adr-agent` exist; AgenticAKM paper (<https://arxiv.org/pdf/2602.04445>) proposes multi-agent extraction → retrieval → generation → validation for ADRs. Quote from search: "Specialized agents for architecture extraction, retrieval, generation, and validation collaborate in a structured workflow to generate architecture knowledge."
- **ADR layers.** The "SDD workflow" layering some practitioners cite: BRD (Layer 1), PRD (Layer 2), EARS (Layer 3, Easy Approach to Requirements Syntax), BDD (Layer 4), ADR (Layer 5). ADRs end up downstream of PRDs.

Ark **does not have ADRs as a first-class artifact.** Architectural decisions are documented in:

- Project SPECs (`specs/project/<name>/SPEC.md`) — conventions/rules.
- Feature SPECs (`specs/features/<...>/<name>/SPEC.md`) — per-feature behavioural specs.
- Deep-tier PLAN's `## Trade-offs` section — options considered with adv/disadv.
- `docs/` — design notes (free-form, not in workflow).
- `archived/<task>/` — point-in-time PRD/PLAN/REVIEW/VERIFY.

The trade-off: ADRs are decision-centric (cite which alternatives were considered); SPECs are state-centric (what currently holds). Both have value; Ark currently leans entirely on the SPEC side.

## RFC — "proposal for peer review"

Engineering practice (Rust, Kubernetes, Django, etc.):

- Long-form proposal document.
- Lives in `rfcs/` directory; PR-driven review.
- Author drives consensus; merged RFC = accepted.

Ark has an `rfc001-arkos` archived task (2026-05-20 archive entry, see session context). The pattern: an RFC is a heavyweight design proposal that doesn't (yet) commit to implementation. Sits between research and design.

OpenSpec's `proposal.md` is structurally an RFC, just renamed. The change-proposal model is "everything's an RFC until archived."

## Design Doc — Google-style

Looser shape than ADRs. Typically includes:

- Background
- Goals / non-goals
- Proposed design (with diagrams)
- Alternatives considered
- Risks

Lives in docs/ or a wiki. Reviewed asynchronously by collaborators.

Ark's deep-tier `## Spec` section in PLAN — Goals, Non-goals, Architecture, Data Structure, API Surface, Constraints — is structurally a design doc. The "self-contained every iteration" rule (workflow.md line 109) forces it to remain coherent across PLAN iterations.

## Feature SPEC — the spec-driven-dev artifact

The youngest artifact. Spec-kit, OpenSpec, Ark, Trellis all use the term but mean slightly different things:

- **spec-kit:** `specs/<feature>/spec.md` — generated by `/speckit.specify`; functional requirements + acceptance criteria. Plan and tasks are downstream.
- **OpenSpec:** `openspec/specs/<capability>/spec.md` — canonical capabilities/scenarios; updated by archived change deltas.
- **Ark:** `specs/features/<path>/SPEC.md` — extracted verbatim from the deep-tier PLAN's `## Spec` section at commit.
- **Trellis:** `.trellis/spec/` — team standards loaded by agents; not per-feature.

The point of contention: is a feature SPEC the *requirements* (what the feature must do, user-facing) or the *specification* (how the feature behaves, internal contract)?

- spec-kit and OpenSpec lean requirements-shaped.
- Ark and Trellis lean specification-shaped.

Ark's SPECs include Architecture, Data Structure, and API Surface — internal contract details, not just acceptance criteria. This is closer to a published interface than a user-facing requirements doc.

## When agents should produce each

| Artifact | Who writes? | When? | How long-lived? |
| -------- | ----------- | ----- | --------------- |
| PRD | Agent + user, DESIGN phase | At task start | Task-scoped (commits / archives) |
| Plan / design doc | Agent, PLAN phase | After PRD | Task-scoped + promoted (deep) |
| Feature SPEC | Auto-extracted (Ark) / authored (spec-kit/Kiro) | At commit (Ark) / at start (others) | Permanent (project-scoped) |
| ADR | Agent or user | At point of architectural decision | Permanent (immutable) |
| RFC | User (mostly) | Before a research/deep task | Until accepted/withdrawn |
| `AGENTS.md` / `CLAUDE.md` | User (mostly), agent (corrections) | Project setup + ongoing | Project lifetime |

Spec-kit's pattern: agent drafts feature spec; user reviews; agent generates plan, tasks, implementation in sequence. Each phase produces a checked-in artifact.

OpenSpec's pattern: agent drafts proposal + delta-specs + design + tasks together (a "change"). On archive, deltas merge into specs.

Ark's pattern: PRD per task (agent + user); PLAN per task with `## Spec` block (agent); on deep commit, `## Spec` extracted to feature SPEC. Other tiers don't extract.

## Promotion / extraction patterns

The "how do we get from task artifacts to durable project artifacts" question.

### OpenSpec delta-merge

From <https://github.com/Fission-AI/OpenSpec> docs:

```
openspec/changes/<id>/
├── proposal.md
├── specs/          # delta-spec files: ADDED-foo.md, MODIFIED-bar.md
├── design.md
└── tasks.md
```

On `/opsx:archive`:

> The CLI automatically applies the Spec Deltas (ADDED/MODIFIED requirements) to the main `openspec/specs/` directory. Your delta specs become part of the main specs, documenting how your system works. When you archive a change, its deltas merge cleanly into the source of truth.

This is a "diff-and-merge" model. Each change is a patch against the canonical specs.

### Spec-kit staircase

Spec-kit doesn't promote — `specs/<feature>/` lives forever where it was created. Plan, tasks, contracts all live alongside the spec. When the feature ships, the dir remains as documentation.

### Kiro three-phase

`.kiro/specs/<feature>/{requirements.md, design.md, tasks.md}`. Each phase produces a phase-named file; no promotion to project root. Same model as spec-kit.

### Ark commit-time extraction

From `.ark/workflow.md` lines 232-238:

> The CLI does, in order:
> 1. VERIFY gate.
> 2. Deep: parse PRD `[**SPEC Path**]`; extract `## Spec` to `specs/features/<path>/SPEC.md`; upsert every INDEX along the leaf-to-root path (seeding missing subtree INDEXes from the template).
> 3. Save `task.toml` with `phase = Committed`, `committed_at = now`.
> 4. Stage exactly the Ark-managed files (no `git add -A`).
> 5. `git commit -m "<message>"`.

This is a third model: **extract a section from the task PLAN, write it to a permanent location.** Modifying an existing SPEC appends a `[**CHANGELOG**]` entry instead of full overwrite.

Trade-offs:

| Model | Brownfield fit | History preserved? | Reviewability |
| ----- | -------------- | ------------------ | ------------- |
| OpenSpec delta-merge | Excellent | In archived `changes/` dir | Per-delta diff |
| Spec-kit / Kiro static | Greenfield bias | In git log | Per-file diff |
| Ark commit-time extract | Good | In archived task + git log | Per-task |

OpenSpec is the most rigorous brownfield model. Ark's CHANGELOG-on-overwrite is a softer middle ground.

## "Intent before edits" — the unifying creed

All five tools share this principle:

- spec-kit: "specifications become executable, directly generating working implementations rather than just guiding them" (`spec-driven.md`).
- OpenSpec: "human and AI align on specs before code gets written" (README).
- Kiro: "Beyond Vibe Coding" — InfoQ tagline.
- Trellis: "Capture the task as a PRD" (step 1 of the core loop).
- Ark: "**Intent before edits.** Write the PRD before touching code." (workflow.md line 391, Principle 2).

The disagreement is over *how much intent* is required before edits start. Quick tier (Ark) requires only a PRD; deep tier requires PRD + PLAN + REVIEW. Spec-kit requires constitution + spec + plan + tasks. OpenSpec requires proposal + delta-specs + design + tasks. Kiro forces the three-phase staircase.

## Comparison table

| Tool | PRD-like | ADR-like | Feature SPEC | Extraction model |
| ---- | -------- | -------- | ------------ | ---------------- |
| spec-kit | `specs/<feat>/spec.md` | constitution + `memory/` | same as spec | Static (lives where authored) |
| OpenSpec | `proposal.md` | none (design.md) | `openspec/specs/<cap>/spec.md` | Delta-merge on archive |
| Kiro | `requirements.md` | none | `requirements.md` (rebranded) | Static (`.kiro/specs/`) |
| Trellis | per-task PRD | project specs (`.trellis/spec/`) | project specs | "Promote lessons" (manual) |
| **Ark** | **per-task `PRD.md`** | **project SPECs + `[**SPEC Path**]`** | **`specs/features/<path>/SPEC.md`** | **Commit-time extract (deep only)** |

## Failure modes

- **Spec sprawl.** Every task produces a SPEC; the spec tree grows without consolidation. OpenSpec mitigates with delta-merge (changes consolidate into canonical specs); spec-kit doesn't. Ark's tier-gated extraction (only deep tier promotes) is a different solution to the same problem.
- **Spec rot.** SPECs that disagree with current code. Ark's VERIFY phase has a "SPEC Drift" checklist item (workflow.md line 187) and a `[**CHANGELOG**]` entry rule on SPEC modification. Enforcement is reviewer-driven.
- **ADR neglect.** Without first-class ADRs, architectural decisions get embedded in PLAN's Trade-offs section and lost on archive. Ark's archived tasks preserve them but they're not searchable as decisions.
- **PRD-as-prose.** A PRD that's all narrative and no acceptance criteria. Ark's PRD template enforces What/Why/Outcome structure but doesn't enforce measurability of Outcome.

## Directions for Ark

1. **ADR support as a project SPEC pattern.** Today architectural decisions are scattered. Candidate: a project SPEC convention `specs/project/decisions/<NNNN>-<slug>.md` modeled on Nygard's ADR template. Promoted from deep tasks when explicitly opted into (PRD field). Trade-off: more ceremony; counter: ADRs are exactly the kind of artifact that *should* outlive the task.
2. **Make `Outcome` testable.** PRD's Outcome field is freeform prose. Add a convention: each Outcome bullet must be a verifiable statement (e.g., "When the user runs `ark unload`, the `.ark.db` file appears in the root and the `.ark/` directory is removed"). VERIFY's "PRD Constraints" section already iterates over Outcome — making each bullet testable hardens the check.
3. **Delta-style SPEC updates.** Borrow from OpenSpec. Today a modified SPEC gets a `[**CHANGELOG**]` entry; the SPEC body is overwritten. A delta-style alternative: the deep PLAN's `## Spec` could be split into `[**Spec Diff**]` blocks (ADDED/MODIFIED/REMOVED) that merge into the existing SPEC. Trade-off: more complex commit logic; benefit: better brownfield evolution.
4. **`AGENTS.md` is already Ark's de facto rulebook.** Worth documenting this in workflow.md or README — when does a rule belong in `AGENTS.md` vs a project SPEC vs the Rust style SPECs? Today the answer is "tribal knowledge". A short rubric would help.
5. **RFC tier (between research and deep).** The recent `rfc001-arkos` archive entry suggests RFCs are happening informally. Candidate: a fifth tier (research / RFC / quick / standard / deep) where RFC is "research-tier corpus plus a proposed design but no implementation commitment." The corpus + design doc lives until accepted/withdrawn.

Sources:

- [Marty Cagan, *Inspired*](https://www.svpg.com/inspired-how-to-create-products-customers-love/) — PRD canon
- [Michael Nygard, Documenting Architecture Decisions (2011)](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) — ADR origin
- [AGENTS.md is the New ADR](https://ai.gopubby.com/agents-md-is-the-ew-architecture-decision-record-adr-3cfb6bdd6f2c)
- [AI-generated ADRs (Adolfi.dev)](https://adolfi.dev/blog/ai-generated-adr/)
- [adr-agent repo (macromania)](https://github.com/macromania/adr-agent)
- [AgenticAKM paper](https://arxiv.org/pdf/2602.04445) — multi-agent ADR generation
- [OpenSpec workflow docs](https://thedocs.io/openspec/concepts/workflow/) — delta-merge model
- [Spec-kit `spec-driven.md` (in repo)](../../../../reference/spec-kit/spec-driven.md)
- [Kiro Specs](https://kiro.dev/docs/specs/) — three-phase staircase
- [Tech-world ADR newsletter](https://newsletter.techworld-with-milan.com/p/the-art-and-science-of-architectural)
