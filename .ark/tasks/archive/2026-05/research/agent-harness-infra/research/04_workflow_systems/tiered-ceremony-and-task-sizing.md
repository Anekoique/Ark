# Tiered Ceremony and Task Sizing

## The problem

Software tasks span six orders of magnitude in complexity:

- **Fix typo in README** — 10 seconds, no review needed.
- **Rename a private function** — 5 minutes, run tests, done.
- **Add a new CLI flag** — an hour, write tests, update docs, review the diff.
- **Refactor module structure** — a day, plan first, review the plan, iterate.
- **Add a new subsystem** — a week, multi-iteration plan/review, integration tests, migration plan.
- **Rewrite the auth model** — a month, RFC, prototype, security review, staged rollout.

A workflow that forces full ceremony on the typo wastes hours. A workflow that allows the auth rewrite to skip planning produces disasters. The right answer is **scale ceremony to task size** — but most agent harnesses don't.

## Ark's tier system

From `.ark/workflow.md` lines 38-49:

```
- Quick     — reversible in one commit, no new abstractions. Artifact: PRD.md.
- Standard  — feature work with testable scope, no API/architecture break. Artifacts: PRD.md, PLAN.md, VERIFY.md.
- Deep      — architectural, cross-cutting, or new subsystem. Artifacts: PRD.md, NN_PLAN.md ⇄ NN_REVIEW.md (looped), VERIFY.md, promoted SPEC.md.
- Research  — knowledge-gathering; corpus IS the deliverable; follow-up implementation optional. Artifacts: PRD.md, research/.
```

Lifecycles per tier (workflow.md line 78):

- Quick: `DESIGN → EXECUTE → COMMIT → ARCHIVE`
- Standard: `DESIGN → PLAN → EXECUTE → VERIFY → COMMIT → ARCHIVE`
- Deep: `DESIGN → PLAN → [REVIEW ⇄ PLAN] → EXECUTE → VERIFY → COMMIT → ARCHIVE`
- Research: `RESEARCH → COMMIT → ARCHIVE`

Tier-specific rules:

- **Quick:** PRD only. No PLAN, no REVIEW, no VERIFY. Designed to be "reversible in one commit."
- **Standard:** PRD + PLAN + VERIFY. No REVIEW loop (skip-REVIEW is the standard-tier optimisation).
- **Deep:** Adds `[REVIEW ⇄ PLAN]` iteration loop and SPEC promotion. Required to use `--worktree` (parallel-safe).
- **Research:** No PLAN/REVIEW/EXECUTE/VERIFY at all. Corpus IS deliverable.

Promotion: `ark agent task promote --to <tier>` (workflow.md line 46) is supported mid-flight. **Research tier does not participate in promotion.** Cross-over between research and tiered implementation is by `task new` referencing the research slug.

Principle (workflow.md line 390): **"Right ceremony for the right task. Three tiers — pick the smallest that fits."** Lower-tier-when-in-doubt is the recommended bias: "Promotion is cheap; demotion is awkward."

## How other tools handle the spectrum

### One-ceremony tools (the majority)

Most published harnesses pick a single ceremony level and apply it universally:

- **Aider** — chat-only, no formal artifacts. One ceremony level: "ask, edit, run tests, repeat." Works for typos to mid-size refactors. Falls apart for cross-cutting changes that need planning.
- **Cursor** — agent mode with grind hook. Same: one autonomous-loop level. Documentation-of-changes is opt-in (Checkpoint extensions etc.).
- **OpenHands** — Plan mode + Execute mode toggle, but the toggle is *within* a single conversation, not tier-of-ceremony.
- **Devin** — Ask mode + Agent mode toggle. Plan goes through Ask; execution through Agent. One overall pipeline.
- **SWE-agent** — research benchmark configuration, not a user-facing harness with tiers.
- **spec-kit** — `/speckit.constitution → specify → plan → tasks → implement`. Mandatory full pipeline. Constitutional gates check complexity but don't let you skip phases.
- **Kiro** — three-phase staircase (requirements → design → tasks). You cannot skip phases; the IDE forces them.
- **Trellis** — one workflow (`/start`, `continue`, `finish-work`). Skills route the work; ceremony is uniform.

### Two-tier tools (a few)

- **OpenSpec** — `default` and `expanded` profiles (selectable via `openspec config profile`). Default ships `/opsx:propose`, `/opsx:apply`, `/opsx:archive`. Expanded adds `/opsx:new`, `/opsx:continue`, `/opsx:ff` (fast-forward), `/opsx:verify`, `/opsx:sync`, `/opsx:bulk-archive`, `/opsx:onboard`. README: "If you want the expanded workflow ... select it with `openspec config profile` and apply with `openspec update`."

  This is project-level, not task-level. Once you pick a profile, every task uses it. Closer to "developer preference" than "task sizing."

- **Anthropic Code Review** — per-repo opt-in. You can enable/disable code review per repo, but within an enabled repo, every PR gets the same multi-agent fleet treatment.

### Variants close to Ark

Ark's tiers most closely resemble:

- **Trellis's per-task PRD model + ceremony defaults.** Trellis ships templates for "spec-light tasks" and "spec-heavy tasks" but the workflow itself is one-shape.
- **spec-kit extensions like "Fleet Orchestrator"** — community add-ons that introduce phase-level orchestration on top of the mandatory pipeline.

The reality: **as of 2026, Ark is one of the few harnesses with task-level tier selection.** OpenSpec's profile system is project-level. Most others are one-ceremony-for-everything.

## Spectrum of approaches

| Approach | Granularity | Example | Trade-off |
| -------- | ----------- | ------- | --------- |
| One ceremony for everything | None | Aider, Cursor, OpenHands, spec-kit (strict), Kiro | Simple, predictable; mismatches small or large tasks |
| Per-mode toggle (Plan/Execute) | Within-session | OpenHands, Devin | Useful within a task; doesn't scale ceremony to task size |
| Per-flag opt-in (e.g., `--deep`) | Per-task | Ark | Explicit; user chooses |
| Per-profile (project setting) | Per-project | OpenSpec profiles | Set once; uniform per project |
| Per-agent (different agents for different tasks) | Per-task | spec-kit subagent extensions | Composable; complex |

Ark sits in the "per-flag opt-in" camp. The slash command `--deep` flag selects the tier at task creation; promotion mid-flight is supported.

## Ark's tier matrix

The fully-expanded matrix:

| Tier | When | Artifacts | Worktree | SPEC promoted | Lifecycle |
| ---- | ---- | --------- | -------- | ------------- | --------- |
| Quick | Trivial, reversible, no new abstractions | PRD only | Optional | No | DESIGN → EXECUTE → COMMIT |
| Standard | Testable feature, no API/architecture break | PRD + PLAN + VERIFY | Optional | No | DESIGN → PLAN → EXECUTE → VERIFY → COMMIT |
| Deep | Architectural, cross-cutting, new subsystem | PRD + iterated PLAN/REVIEW + VERIFY + promoted SPEC | **Required** | **Yes** | DESIGN → PLAN → [REVIEW ⇄ PLAN] → EXECUTE → VERIFY → COMMIT |
| Research | Cannot yet write a PRD's What/Why/Outcome | PRD + `research/` corpus | Optional | No | RESEARCH → COMMIT |

The "deep tier requires worktree" rule (workflow.md line 296) is uncommon — it forces isolation for the highest-ceremony work, recognising that PLAN ⇄ REVIEW iteration generates many revisions worth isolating in a dedicated branch.

## Trade-offs of tiered ceremony

### Pros

- **Right-sized ceremony.** No PLAN ⇄ REVIEW for typos. No PRD-skipping for auth rewrites.
- **Onboarding glide.** New users can start with `/ark:quick`, learn standard later, learn deep when needed.
- **Visible cost.** Tier name signals expected effort.
- **Composable lifecycle.** Quick lacks PLAN/REVIEW/VERIFY phases; standard adds PLAN/VERIFY; deep adds REVIEW. Each phase is reused.

### Cons

- **Tier-picking is itself a task.** Users have to choose. Workflow.md line 48 mitigates ("when in doubt, pick lower; promotion is cheap").
- **Tier boundaries are fuzzy.** What's "reversible"? What's "architectural"? Workflow.md offers heuristics but no algorithm.
- **Tier-promotion costs.** Mid-flight promotion is supported, but generates legacy artifacts (quick PRD → standard now also needs a PLAN). Workflow.md handles this; user friction remains.
- **Tier-specific verb tables.** Each tier has different legal phase transitions. `ark agent task plan` errors with `WrongTier` on quick tier. More user-visible surface area.

### Specific failure modes

- **Tier inflation.** "Just in case" picks of higher tiers. Combated by the "pick lower" recommendation.
- **Tier deflation.** "It's just a quick fix" picks of quick tier for tasks that turn into deep refactors. Combated by promotion support.
- **Quick-PRD-as-prose.** Quick tier asks for only a PRD, so users skimp on it. Workflow.md enforces What/Why/Outcome filled as a DESIGN gate.

## Alternative approaches considered

### Per-agent ceremony

Instead of tiers, dispatch different agents for different task shapes. Spec-kit's "conduct extension" works this way: a router agent decides which specialised sub-agent handles a task. Pros: leverages specialised models. Cons: harder to reason about; agent boundaries don't always align with ceremony boundaries.

### Per-flag mix-and-match

Instead of named tiers, let users pick artifacts à la carte: `/ark --plan --review --verify --no-spec`. Pros: maximum flexibility. Cons: explosive combinatorial UX; most combos are useless.

### Auto-tier inference

Have the agent itself choose the tier based on the task description. Pros: removes user friction. Cons: prone to mis-classification; users lose visibility into expected cost.

Ark's chosen approach (named tiers with mid-flight promotion) splits the difference: small set of tiers, explicit user choice, escape valve via promotion.

## The "ceremony scaling" insight

The deeper claim Ark makes:

> Ceremony is a tool, not a virtue. The right amount depends on:
> - Reversibility (can we undo the change?)
> - Blast radius (what breaks if we get it wrong?)
> - Spec drift risk (will future tasks contradict this one?)
> - Review value (will a second pair of eyes catch real bugs?)

A typo fix scores low on all four — quick tier is right. An auth rewrite scores high on all four — deep tier is right.

This is **not** a 2026 industry consensus. The dominant view (spec-kit, Kiro) is that *any* task large enough to need an agent deserves full ceremony. The minority view (Aider, Cursor) is that ceremony is overhead.

Ark's middle position is principled but unproven at scale. Real-world telemetry would help — how often do users pick each tier? How often do they promote? Are quick-tier tasks more likely to be re-opened later?

## Comparison summary

| Tool | Ceremony levels | Selection granularity | Mid-flight change |
| ---- | --------------- | --------------------- | ----------------- |
| Aider | 1 (chat) | None | N/A |
| Cursor | 1 (agent mode) | None | N/A |
| OpenHands | 1 (Plan ⇄ Execute toggle within session) | Session-level | Within session |
| Devin | 1 (Ask ⇄ Agent toggle within session) | Session-level | Within session |
| spec-kit | 1 (full pipeline) | None | Constitutional gates only |
| Kiro | 1 (3-phase staircase) | None | None |
| OpenSpec | 2 (default / expanded profiles) | Project-level | Profile change requires re-apply |
| Trellis | 1 (single workflow) | None | Skills routing |
| **Ark** | **4 (quick / standard / deep / research)** | **Per-task** | **`task promote --to <tier>` supported** |

## Directions for Ark

1. **Tier-selection helper.** Workflow.md offers heuristics ("reversible", "architectural") in prose. A `ark agent task suggest-tier --description "<text>"` could read the description and recommend a tier. The agent already makes this judgment implicitly; surfacing it would help new users.
2. **Telemetry on tier-distribution.** If anonymous opt-in telemetry shipped (OpenSpec ships this with `OPENSPEC_TELEMETRY=0` opt-out), Ark could learn how often each tier is picked, how often promotion happens, how often quick-tier tasks become standard/deep retroactively. Trade-off: privacy/scope creep. Counter: invaluable for tuning the tiers.
3. **Standard-tier mini-review (cross-ref to plan-execute file).** Currently REVIEW is deep-only. Some standard tasks would benefit from a single-pass review. Candidate: `--review` flag on `/ark:design` that adds one REVIEW iteration without promoting to deep. Lighter alternative to the full deep-tier loop.
4. **Document tier-picking heuristics with examples.** Workflow.md says "pick the smallest tier that fits" but gives only abstract descriptions. Adding a "tier picker by example" table — "renaming a function = quick; adding a flag = standard; new subcommand = deep" — would reduce guess-work.
5. **Tier-aware project SPEC defaults.** A project SPEC like `specs/project/ceremony/SPEC.md` could express tier-policy: "auth-related changes default to deep tier; doc changes default to quick." Today this is purely user judgment. Trade-off: more configuration surface; benefit: codifies tribal knowledge.

Sources:

- [Ark workflow.md](../../../../../.ark/workflow.md) — tier definitions
- [OpenSpec profiles (README)](../../../../reference/OpenSpec/README.md) — `default` vs `expanded`
- [Kiro Specs](https://kiro.dev/docs/specs/) — three-phase staircase
- [spec-kit `spec-driven.md` constitutional gates](../../../../reference/spec-kit/spec-driven.md)
- [Trellis README](../../../../reference/Trellis/README.md) — single-workflow approach
- [OpenHands Plan mode (Issue #557)](https://github.com/OpenHands/software-agent-sdk/issues/557)
- [Devin 2.0 (Cognition)](https://cognition.ai/blog/devin-2) — Ask/Agent toggle
- [Cursor agent best practices](https://cursor.com/blog/agent-best-practices)
