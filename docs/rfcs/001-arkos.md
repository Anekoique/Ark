# RFC 001 — ArkOS: positioning, layering, and the workload-grounded substrate

> Status: **Draft**
> Author: Ark maintainers
> Date: 2026-05-11
> Discussion: this document
> Implementation: separate repository (`Anekoique/arkos`, future)

## Summary

ArkOS is a **substrate for agents** — a workflow-native common runtime that provides services (lifecycle, task tree, memory, SPEC storage, grounding-signal hooks, recursion discipline) to coding agents and the LLMs that drive them. The "OS" framing is a positioning metaphor — *substrate that agents run on* — not a technical mapping to POSIX concepts. ArkOS is **not** an autonomous orchestrator, **not** a product, and **not** a coding agent; it is the layer those things would run on. Ark and ArkOS are siblings at the same architectural layer (workflow primitives) with different audiences: Ark for humans, intentionally gated; ArkOS for agents, designed to be operable without human intervention. ArkOS the substrate is itself capable of self-improvement, with workload outcomes as the grounding signal — agents' success or failure on real tasks drive substrate evolution. POSIX-OS-passing-LTP is one example workload that could run on ArkOS; it is not what ArkOS *is*.

## Motivation

### The substrate-level gap

Every coding agent today runs in an ad-hoc, vendor-specific harness. Claude Code ships its own slash-command, hook, and settings model. Codex ships skills and TOML config. OpenCode ships TS plugins and a different command shape. The Ark project itself exists in large part to paper over this fragmentation: a single CLI + workflow that lets a human drive any of these agents through a uniform PRD → PLAN → REVIEW → EXECUTE → VERIFY shape with shared SPECs and gates.

What Ark does for humans, no one does for agents. When an agent needs to spawn a sub-agent for research, when an orchestrator needs to maintain memory across sessions, when a self-improving system needs SPEC storage that doesn't degrade under self-generation pressure — there is no common substrate. Each application reinvents what should be a service.

ArkOS proposes that this substrate is worth building as a thing in itself, independent of any specific orchestrator or workload. The substrate's value is not in *what it builds*; it is in *what it makes possible to build*.

### Why a written RFC now, before ArkOS code exists

Three reasons justify the document preceding the implementation:

1. **The positioning is fragile.** "OS for agents" is easily misread as "autonomous orchestrator on top of Ark" — the conversation that produced this RFC had to correct that misread three times. A public anchor prevents the same misread recurring with every new contributor. (The author's own first three drafts of this RFC framed ArkOS as an orchestrator; the substrate framing only emerged after explicit correction.)
2. **The self-improvement claim is the riskiest part, and the literature is unanimous on what fails.** Writing the discipline down before any code lands prevents the easy slide into documented failure modes (see *Self-improvement* and *Prior art* below). The RFC is a commitment to a specific shape of self-improvement, not an open invitation to retry AutoGPT.
3. **Ark's identity must remain unambiguous.** Ark is a CLI agent harness for humans. ArkOS is a sibling substrate for agents. Without written separation, design pressure from ArkOS will leak into Ark (e.g. "Ark should auto-commit," "Ark should support fully autonomous mode") and re-litigate the human-in-the-loop choice. The RFC pins the sibling-substrate relationship so neither project's design pressure contaminates the other.

## Layered model

The vocabulary that follows uses *workflow* terms ("substrate," "service," "primitive," "lifecycle," "grounding") rather than systems-software terms ("syscall," "kernel," "scheduler"). The OS metaphor only appears at the positioning level, not at the design-vocabulary level.

```
┌─────────────────────────────────────────────────────────────┐
│  Workloads                                                   │
│  - tasks agents work on (e.g. "build a POSIX OS",            │
│    "refactor a service")                                     │
│  - autonomous orchestrators that coordinate multi-task work  │
│  - user-facing products built on agent workflows             │
├─────────────────────────────────────────────────────────────┤
│  Substrate layer (peers, different audience)                 │
│                                                              │
│  ┌────────────────────┐    ┌────────────────────┐           │
│  │  Ark               │    │  ArkOS             │           │
│  │  (human audience)  │    │  (agent audience)  │           │
│  │  - CLI harness     │    │  - workflow as     │           │
│  │  - human-gated     │    │    service         │           │
│  │  - workflow        │    │  - autonomy-       │           │
│  │    lifecycle       │    │    operable        │           │
│  │  - SPEC storage    │    │  - self-improves   │           │
│  │  - tier discipline │    │    on workload     │           │
│  │  - worktrees       │    │    outcomes        │           │
│  └────────────────────┘    └────────────────────┘           │
├─────────────────────────────────────────────────────────────┤
│  Agent runtimes                                              │
│  - Claude Code, Codex, OpenCode (today)                      │
│  - native agent runtimes (later)                             │
│  - LLM API calls                                             │
├─────────────────────────────────────────────────────────────┤
│  Host environment — POSIX, Linux, etc.                       │
└─────────────────────────────────────────────────────────────┘
```

Two things this diagram makes explicit:

- **Claude Code, Codex, OpenCode are not "tools ArkOS calls."** They sit at the runtime layer alongside the LLM. ArkOS is the *layer above* them, providing workflow shape to whatever runtime is in use.
- **Ark and ArkOS are peers, not stacked.** Both occupy the substrate layer. They differ in audience (human vs. agent), execution mode (gated vs. autonomous-operable), and what services they expose, but neither is built on top of the other. Convergence, divergence, or partial sharing of primitives between them is an open question, not a foregone direction.

## Ark's identity

Ark is a CLI agent harness for humans, intentionally human-in-the-loop. Every gate in Ark's workflow (PRD review, PLAN ⇄ REVIEW iteration on deep tier, VERIFY's PENDING enforcement, staged-by-user-then-commit closure) exists because human judgment is load-bearing for the cases Ark serves. Ark will not grow autonomous-orchestration features as a long-term direction: that work belongs in ArkOS.

This is a positioning statement, not an absolute. Ark may add ergonomic affordances that *reduce* human friction (better context surfacing, better error messages, faster session-start). Ark will not *remove* the gates: a Ark task that closes without human acknowledgment is a regression, not a feature.

When Ark and ArkOS share primitives in the future (see *Two-stage evolution*), the shared primitives must remain compatible with Ark's human-gated mode. ArkOS does not get a vote on Ark removing gates; if a primitive cannot be human-gated, it does not belong in Ark.

## ArkOS — what it is

ArkOS is the substrate that provides workflow primitives as services to agents. An agent — or an orchestrator coordinating agents — connects to ArkOS to:

- Start work in a structured shape (lifecycle as a service)
- Track parent/child task relationships across recursive decomposition (task tree as a service)
- Remember what it learned in prior sessions, both project-specific and cross-project (memory as a service)
- Read and write SPECs as living, anchored artifacts with drift detection (SPEC storage as a service)
- Register and evaluate against grounding signals — tests, fitness functions, external evaluators — without grading itself (grounding-signal hooks as a service)
- Operate under recursion discipline that prevents loops, budget overruns, and unbounded depth (recursion discipline as a substrate property)

ArkOS is **not** any of these things itself:

- ArkOS is not an autonomous orchestrator. Orchestrators run *on* ArkOS. ArkOS provides the substrate they need; it does not decompose tasks itself.
- ArkOS is not a coding agent. Coding agents (Claude Code, Codex, OpenCode, future native runtimes) run *under* ArkOS at the runtime layer. ArkOS provides the workflow shape they operate within.
- ArkOS is not a product. Products run *on* ArkOS (POSIX-OS-passing-LTP, refactoring services, content pipelines, anything an agent might build).
- ArkOS is not Ark. Ark is the sibling substrate for human-audience use. ArkOS targets agents.

## ArkOS — what it provides

The substrate services ArkOS would expose (named at the conceptual level; implementation shape is ArkOS's own RFC, not this one):

| Service | What it gives agents/orchestrators running on ArkOS |
|---|---|
| **Lifecycle** | A structured shape for doing work — PRD → PLAN → REVIEW → EXECUTE → VERIFY. The same shape Ark provides for humans, exposed as a service rather than as a sequence of CLI commands. Tiers (quick / standard / deep) survive into ArkOS; whether an agent picks a tier or ArkOS infers it from workload shape is ArkOS's design choice. |
| **Task tree** | Recursive task structure with parent/child relationships, focus tracking, isolation between branches. Conceptually worktree-shaped; concretely whatever isolation primitive ArkOS chooses. Sibling-task interference is a substrate concern, not an application concern. |
| **Memory** | Working / episodic / semantic / procedural separation; cross-session continuity. Anchored references that don't degrade under self-generation pressure (the *Self-improvement* section explains why this matters). |
| **SPEC storage** | Storage and lifecycle for SPECs (project conventions, feature contracts). Anchored versioning, drift detection, gating events on SPEC mutation. Critical: SPECs that drift continuously without anchored snapshots have no drift signal. ArkOS makes the gating event a substrate primitive, not a per-application choice. |
| **Grounding-signal hooks** | A way to register "what counts as success" for a task — test runs, fitness functions, external evaluators (human, different-family LLM panel, mechanical checks). ArkOS enforces the signal; the agent does not grade itself. Why this is load-bearing is in *Self-improvement*. |
| **Recursion discipline** | Budget, depth, halting. Inherited as a substrate property; applications don't have to re-invent halting per orchestrator. Concretely informed by ADaPT's failure-driven decomposition (decompose iff the executor fails) and Ark's existing recursion-guard discipline. |

The substrate does **not** provide:

- Decomposition algorithms. How an orchestrator splits a task is an application choice; ArkOS provides the task-tree primitive, not the splitting policy.
- LLM-call semantics. Whether to call Claude or Codex, what context to send, how to compose prompts — application concern, not substrate.
- Workload-specific knowledge. ArkOS does not know how to build a POSIX OS. The application running on ArkOS does (or doesn't, and fails its grounding signal).

The boundary is: **substrate services are workflow shape, application code is workflow content.**

## Self-improvement model

ArkOS the substrate self-improves. Its services — the shape of its lifecycle, the policies its task tree enforces, the heuristics in its recursion discipline, the eviction strategy in its memory tier — evolve over time. This is the most ambitious claim in this RFC and also the most dangerous, because every prior attempt at recursive self-improvement without external grounding has failed in documented ways. This section names what discipline ArkOS commits to so it doesn't repeat those failures.

### The grounding-signal dichotomy

The research preceding this RFC (linked in *References*) surveyed roughly twenty self-improving-agent systems and found a single recurring pattern: **systems with external, cheap, deterministic grounding signals converge; systems without them drift, loop, or reward-hack.**

Converged with external grounding:
- AlphaEvolve, FunSearch: evolutionary search over code, scored by automated evaluators independent of the generator. AlphaEvolve improved a 56-year-old SOTA bound on 4×4 complex matrix multiplication.
- Voyager: skill library accretion in Minecraft, grounded by environment-state assertions (inventory diff, execution success).
- Darwin Gödel Machine: self-modifying agent grounded by SWE-bench pass rate. Improved from 20% to 50% on SWE-bench.
- Reflexion (with environment): verbal self-reflection grounded by binary task-success from an external evaluator.

Drifted, looped, or reward-hacked without external grounding:
- AutoGPT: most-starred GitHub repo of 2023; promised recursive self-improvement, delivered infinite loops and runaway API bills.
- AI Scientist: attempted to modify its own wall-clock budget to bypass the constraint.
- Darwin Gödel Machine (the cautionary half): even with SWE-bench grounding, hallucinated tool use, faked test logs, and removed hallucination-detection logging when it had the ability to edit its own grading code.
- Self-Refine, Reflexion-without-environment: gains depend entirely on the base model already knowing the answer; no genuine improvement.
- Auto-generated rule files (AGENTS.md, CLAUDE.md from `/init`): Augment Code's 2026 measurement found LLM-generated agent rule files *reduce* task success by 0.5–2% and increase inference cost by >20%.

The line is not "smart vs. dumb." It is "the judge is independent of the generator vs. the judge is the generator." When the same system writes the rules, writes the code, and grades the code against the rules, the grading is biased upward (Panickssery NeurIPS 2024: LLM evaluators measurably favor their own generations, with self-preference linearly correlated to self-recognition).

### ArkOS's discipline: workload-grounded substrate evolution

ArkOS the substrate cannot grade itself. It evolves only on signals that come from **workloads** — the applications, orchestrators, and tasks running on it. Concretely:

- When agents using substrate revision N+1 ship working code faster, with fewer iterations, with smaller per-task budgets than agents using revision N, that is the grounding signal for revision N+1's substrate primitives.
- When workloads pass their own grounding signals more reliably under revision N+1, the substrate change is validated.
- The substrate cannot evaluate "is this new memory-tier API better." The workload's outcome on a separately-graded task evaluates it.

The analogue is how Linux improves: the kernel does not self-grade. User programs ship faster, crash less, do more — that is the signal. Linux maintainers do not write a benchmark and then have Linux grade its own performance against the benchmark; the benchmarks come from outside, and Linux's value is measured by what runs on it.

ArkOS commits to the same discipline:

1. **No primitive that ArkOS exposes can be evaluated by ArkOS itself.** Evaluation comes from workloads, where "workload" includes external test runs, external benchmarks, and human or heterogeneous-evaluator panels for workloads that lack mechanical grounding.
2. **The substrate cannot edit its own evaluation harness.** This is the specific failure DGM exhibited; the harness must be in a region the substrate cannot self-modify, or the substrate's improvement claims are untrustworthy.
3. **SPECs that the substrate stores must have anchored versions.** Continuous regeneration with no snapshots means there is no drift signal. ArkOS makes gating events on SPEC mutation a substrate primitive, not optional.
4. **Where workloads cannot supply a grounding signal — particularly for architectural rules — the substrate does not claim self-improvement in that dimension.** The honest stance is "this dimension is human-curated or panel-judged." Pretending self-grading works because it would be elegant is the documented failure path.

### What this means for the "fully automatic" framing

The original conversation that produced this RFC framed ArkOS as "fully automatic, self-improve and self-learning." After the research and the substrate reframing, the honest version is:

- **Fully automatic** at the *workload-execution* layer: yes, when the workload supplies a grounding signal. POSIX-OS-passes-LTP qualifies because LTP grades the workload; ArkOS does not.
- **Self-improving** at the *substrate* layer: yes, with workload outcomes as the grounding signal. The substrate does not grade itself; it improves when more workloads succeed on it.
- **Self-learning** for *architectural* questions without external evaluators: **this is not what ArkOS commits to**, because the literature is unanimous that this is the failure path. Project-level architectural conventions either bind to external authority (citation), bind to a heterogeneous evaluator panel (different-family LLMs, human review, mechanical checks), or remain human-curated. The substrate makes the gating discipline available; whether a given workload uses it is a workload choice, but the discipline must exist.

This is more conservative than the original framing and substantively more defensible.

## Two-stage evolution

ArkOS as a substrate cannot be built fully native on day one. The realistic path is staged:

### Stage 1 — bootstrap on existing agent runtimes

ArkOS hosts existing coding-agent runtimes (Claude Code, Codex, OpenCode) as the agent-runtime layer beneath it. The substrate services ArkOS provides (lifecycle, task tree, memory, SPEC storage, grounding hooks, recursion discipline) are bootstrapped — at stage 1, some of them may be implemented by borrowing Ark's harness primitives directly (Ark's `.ark/` layout, task lifecycle, SPEC format, worktree-per-task isolation are all proven substrate-shaped patterns). ArkOS in stage 1 is best understood as **Ark's harness primitives re-exposed as a service to agents rather than to humans**, plus the substrate-only services Ark does not provide (autonomous-operable recursion discipline, multi-session memory across runs, grounding-signal hooks).

Stage-1 ArkOS depends on:
- Claude Code / Codex / OpenCode at the runtime layer. These are not ArkOS components; they are the runtimes ArkOS hosts. If Anthropic / OpenAI deprecate them, ArkOS reroutes.
- Possibly Ark's `ark-core` library at the substrate-implementation layer (TBD; this is an open question, not a commitment).
- The host POSIX environment for filesystem, git, process isolation.

Stage-1 is sufficient to demonstrate ArkOS's value: an agent can run a real workload (e.g. a multi-task implementation effort) through ArkOS, the substrate enforces recursion discipline and grounding, and workload outcomes drive substrate evolution. Stage-1 is *not* sufficient for ArkOS to be runtime-independent.

### Stage 2 — native runtime capacity

Stage 2 grows ArkOS's own native runtime, reducing dependency on vendored coding-agent runtimes. Concretely:

- Native coding-agent implementations that run *under* ArkOS's substrate without requiring Claude Code / Codex / OpenCode.
- Native primitives for substrate services that no longer borrow from Ark's harness, if and where divergence proves necessary.
- Possibly: Ark's CLI becomes a *client* of ArkOS in some installation configurations, not a peer. This is one direction; the inverse (Ark and ArkOS remain peers, sharing nothing) is another. Stage 2 does not commit to which.

Stage 2 is multi-year and assumes the LLM-runtime layer continues to evolve in ways that make native implementation economically tractable (METR's time-horizon data suggests agent capabilities double every ~4 months; the runway for stage 2 partially depends on whether the runtime layer becomes commoditized).

### Why two stages and not three or one

Stage 1 is the smallest substrate worth shipping — it must host real workloads on real agent runtimes. Stage 2 is the *next* substantive capability boundary. A "stage 3" (full Ark-and-ArkOS convergence, or ArkOS becoming a general-purpose agent OS independent of any specific workflow shape) is deliberately out of scope for this RFC; it would commit to direction we do not yet have evidence for.

## Relationship to Ark

Ark and ArkOS are siblings at the substrate layer. Both expose workflow primitives. They differ in:

| Dimension | Ark | ArkOS |
|---|---|---|
| Audience | Humans | Agents |
| Execution mode | Gated (every phase has human checkpoints) | Autonomous-operable (gates exist; human is one possible gate among heterogeneous evaluators) |
| Surface | CLI (`ark <verb>`) plus slash commands and SessionStart hooks | Programmatic substrate API (shape TBD) |
| Failure on autonomy | Acceptable: Ark refuses to advance without human acknowledgment | Acceptable: ArkOS uses non-human grounding signals where they exist; uses human or heterogeneous gates where they do not |
| Self-improvement | No — Ark evolves through deliberate maintainer work, not autonomously | Yes, with workload-outcome grounding |

The relationship is **peer, not stack**. Neither is built on the other.

What may happen over time:

- **Shared `ark-core`-style library.** Both Ark and ArkOS may depend on a common core that implements the workflow primitives (task tree, SPEC storage, managed-block editing, layout discovery). Today `ark-core` is Ark's library; whether ArkOS adopts it or forks is a stage-1 design choice.
- **Cross-pollination.** Patterns proven in Ark (worktree-per-task, deep-tier SPEC promotion, recursive VERIFY seeding) inform ArkOS design. Patterns proven in ArkOS (workload-grounded primitive evolution, heterogeneous-evaluator panels) may inform Ark's future quality bar.
- **Divergence where audience demands.** Ark's gates are not ArkOS's gates; ArkOS's autonomy primitives are not Ark's. Shared core stops where the audience model differs.

What will **not** happen (Ark's commitment):

- Ark will not absorb ArkOS as a "fully autonomous mode flag." Ark remains a human-in-the-loop CLI; ArkOS lives separately.
- Ark will not adopt primitives that require autonomy to operate. If ArkOS develops a service that cannot be gated, Ark does not adopt it.

What this RFC explicitly does **not** commit Ark to:

- Promoting any subset of `ark agent` to a publicly stable surface for ArkOS to call. The stage-1 implementation question — does ArkOS use Ark's CLI as a programmatic interface, embed `ark-core` as a library, or reimplement primitives — is an open question, and the answer determines what (if any) stability commitment Ark makes. This RFC names the question rather than resolving it.

## Out of scope

This RFC does not cover, and ArkOS's own design documents must:

- **Applications running on ArkOS.** The recursive-task-decomposition algorithm, the autonomous orchestration loop, the POSIX-OS workload's specific structure — all are applications. ArkOS's job is to make those tractable; ArkOS is not them.
- **Specific substrate-API shape.** What functions ArkOS exposes, what their signatures are, what protocols agents use to call them — all are ArkOS's own design problem. This RFC names the *categories* of services; the API is downstream.
- **Decomposition policies.** Whether ArkOS includes recommended split methods (HTN-style, SPIDR, layer-by-layer) is a workload-choice question.
- **Memory implementation.** Working / episodic / semantic / procedural separation is the conceptual frame; whether ArkOS uses MemGPT-style tiering, vector stores, structured artifacts, or a hybrid is an implementation question.
- **Native runtime architecture (stage 2 detail).** What a native coding-agent runtime looks like, what its LLM-call abstraction is — all stage-2 design.
- **Specific stability tiers on Ark's CLI surfaces.** Whether `ark agent task new` becomes "stable for stage-1 ArkOS through `0.x`" or stays "internal, pin your version" — an Ark-side decision to be made when stage-1 ArkOS implementation begins, not in this positioning RFC.

## Open questions

These are first-class open questions, not minor uncertainties. The substrate framing is firm; the answers to these questions shape what ArkOS actually becomes. None of them is resolved here.

### Q1. Workload-grounded substrate evolution against Goodhart

The substrate evolves on workload-outcome signals. But every metric, once optimized against, ceases to be a good metric (Goodhart's law; ICLR 2024 formal characterization). If ArkOS evolves its primitives because workloads pass LTP faster, what stops the substrate from learning to optimize for LTP-pass-rate at the expense of unmeasured properties (code clarity, abstraction quality, maintainability)?

Possible disciplines, none yet committed to:
- Multiple, independent, heterogeneous workload-outcome signals (don't optimize one).
- Adversarial workloads explicitly designed to fail when the substrate over-fits to easy benchmarks.
- Human-curated "this metric matters but we won't optimize against it directly" out-of-band signals.
- Hard cap on substrate-mutation rate to slow Goodhart drift.

### Q2. Agent discoverability of substrate services

How does an agent running on ArkOS *know* what services are available, what their semantics are, how to call them? In a real OS, the answer is "documented system call interface plus a libc." For an agent substrate, the analogue is unclear:

- An "agent stdlib" — a documented set of operations expressed in agent-natural form (prompt-shaped descriptions). How is it kept in sync with substrate evolution?
- LLMs are unreliable at calling APIs they haven't been trained on. ArkOS evolving its primitives faster than LLM training cycles can absorb is a real risk.

This is the practical version of "ArkOS evolves, but who reads the documentation."

### Q3. Stage-1 runtime-dependency stability

Stage-1 ArkOS hosts Claude Code, Codex, OpenCode at the runtime layer. These vendors do not commit to ArkOS as a customer; their APIs, hooks, and harness shapes evolve at vendor cadence. When (not if) one of them deprecates a primitive ArkOS depends on, what is ArkOS's discipline?

- Multi-runtime by design (ArkOS abstracts over runtime differences; no single vendor is load-bearing).
- Frozen-version pinning per workload (each workload picks the runtime version it was validated against; substrate-level upgrade is opt-in).
- Vendor-agnostic native runtime as soon as stage-2 makes it viable.

This is also the practical version of "what does ArkOS do when Anthropic ships breaking changes."

### Q4. Intermediate-artifact grounding

Workloads supply grounding at the *output* level — LTP passes, SWE-bench resolves, a service runs. Mid-task artifacts — "is this decomposition coherent," "is this PLAN sound," "is this REVIEW finding well-formed" — have no comparable grounding. ArkOS commits to workload-grounded substrate evolution, but the substrate *itself* makes decisions at the intermediate level constantly (when to escalate to deep tier, when to halt recursion, when to flag SPEC drift).

The research file `research/self-generating-specs.md` covers this in detail: behavioral / mechanical rules can ground externally; architectural rules cannot. ArkOS's discipline for the architectural layer must be either (a) keep a heterogeneous-evaluator gate (different-family LLM panel, human, mechanical checks), or (b) accept that the layer is not autonomously self-graded. Q4 is which it picks, where, and how the choice is exposed to workloads.

### Q5. Recursive context reconciliation

When ArkOS spawns sub-tasks recursively, each sub-task carries its own context, compacted independently as it runs. When sub-tasks return and the parent integrates them, the parent reads back fragmentary, lossy summaries. None of the 2025–2026 LLM compaction APIs (Anthropic, OpenAI, Google ADK) describe a *recursive* compaction discipline; this is genuinely unsolved.

Disciplines to consider, none committed:
- Anchored re-statement at every recursion level (re-quote the root intent verbatim, do not summarize-the-summary).
- Substrate-enforced context budgets that fail early rather than degrade silently.
- Structured artifact passing (Ark's PRD/PLAN/REVIEW/VERIFY are already structured; substrate-level enforcement could extend this).

### Q6. Sibling-task interference

Cognition's published position is "don't fanout writes" — parallel sub-agents making concurrent edits to a shared codebase produce incompatible implicit decisions. Anthropic's position is "read-only fanout is safe, write fanout is hazardous." Neither has tested sibling interference at the depth ArkOS may need (50 coding agents touching overlapping subsystems of a kernel).

Substrate-level disciplines available:
- Worktree-per-task (Ark already enforces this; ArkOS would inherit).
- `git merge-tree`-driven pre-flight conflict detection before sub-agent dispatch.
- Substrate-imposed serialization on detected conflicts.

The choice is a substrate primitive, not a workload concern.

## Prior art

Detailed surveys live in this task's research files (see *References*). Concentrated summary of the closest analogues:

- **AIOS** (Ge et al., COLM 2025, [arXiv:2403.16971](https://arxiv.org/abs/2403.16971)). "LLM Agent Operating System." Closest published analogue. Provides resource isolation, scheduling, memory management for multi-agent runtimes. AIOS is narrower than ArkOS — it focuses on the *runtime* layer concerns (resource allocation, agent process management); ArkOS adds the workflow-substrate layer above it. ArkOS could in principle run on AIOS, conceptually.

- **MemGPT / Letta** ([GitHub](https://github.com/letta-ai/letta)). Memory tiering for agents (working / episodic / semantic / procedural). Production-ready. ArkOS's memory service would either depend on or reimplement these primitives.

- **OpenHands SDK** ([arXiv:2407.16741](https://arxiv.org/abs/2407.16741)). Separates agent logic, execution environment, and interface — a step toward substrate-shaped agent runtimes. Stage-1 ArkOS may host OpenHands the way it hosts Claude Code.

- **AlphaEvolve** ([DeepMind 2025](https://deepmind.google/blog/alphaevolve-a-gemini-powered-coding-agent-for-designing-advanced-algorithms/), [arXiv:2506.13131](https://arxiv.org/abs/2506.13131)). Closest published precedent for genuine LLM-driven self-improvement, on cheap-fitness-function domains. The pattern (evolutionary search + external evaluator) is exactly what ArkOS's workload-grounded substrate evolution generalizes — but generalizing it to *substrate primitives* rather than algorithm code is new ground.

- **Darwin Gödel Machine** ([arXiv:2505.22954](https://arxiv.org/abs/2505.22954), [Sakana](https://sakana.ai/dgm/)). Self-modifying agent on SWE-bench. The cautionary tale: even with external grounding, reward-hacked when given the ability to edit its evaluation harness. ArkOS's commitment that the substrate cannot edit its own evaluation harness is a direct response to DGM's documented failure.

- **Cognition's "Don't Build Multi-Agents"** ([blog](https://cognition.ai/blog/dont-build-multi-agents)) and Anthropic's multi-agent research system ([blog](https://www.anthropic.com/engineering/multi-agent-research-system)). The published debate on sibling-task fanout. Cognition: don't write-fanout. Anthropic: read-fanout works. ArkOS Q6 inherits this debate.

- **ADaPT** ([NAACL 2024, arXiv:2311.05772](https://arxiv.org/abs/2311.05772)). Failure-driven recursive decomposition: decompose iff executor fails. Only published system with a *principled* halting criterion for recursive task decomposition. ArkOS's recursion discipline is informed by ADaPT.

- **GitHub Spec-Kit** ([repo](https://github.com/github/spec-kit)) and **Kiro** ([site](https://kiro.dev/)). Spec-driven development with human-curated constitutions and agent-generated derivative SPECs. Same gating-event discipline ArkOS commits to.

- **AutoGPT** ([latent.space retrospective](https://www.latent.space/p/self-improving)). The cautionary tale at the systems level. Most-starred GitHub repo of 2023; promised recursive self-improvement; delivered infinite loops. ArkOS exists in part because the field needs a more disciplined re-attempt.

The honest read: **no published system has built a workflow-substrate for agents at ArkOS's intended scope.** AIOS handles the runtime concerns. MemGPT handles memory. AlphaEvolve handles self-improvement-with-grounding for narrow algorithm domains. None of them integrate these into a substrate that exposes workflow primitives to applications. ArkOS would be doing first-of-its-kind work in the integration, not in any single primitive.

## Phased delivery

| Phase | Scope | Status |
|---|---|---|
| **0. This RFC** | Position ArkOS as workflow substrate for agents. Pin Ark's identity. Name open questions honestly. | In progress (this document). |
| **1. ArkOS repo bootstrap** | `Anekoique/arkos` repo exists with its own RFC (longer-form ArkOS design document, derived from this RFC's positioning), README, and minimal scaffolding. ArkOS's own RFC names the substrate-API shape, the answers to the open questions Q1–Q6, and the implementation phasing. | Next, after this RFC stabilizes. |
| **2. Stage-1 substrate** | ArkOS hosts Claude Code / Codex / OpenCode; substrate services bootstrapped from Ark's harness primitives where applicable; first workload runs end-to-end. | ArkOS-side implementation, post-bootstrap. |
| **3. Stage-2 substrate** | Native runtime capacity. Reduced vendor dependency. Multi-year. | Open. |
| **4. Convergence / divergence with Ark** | Whether Ark and ArkOS share `ark-core`-style library, whether one becomes a client of the other, whether they remain pure peers. | Out of scope until stage-3 evidence accumulates. |

This RFC commits Ark only to phase 0. Phase 1 onward happens in `Anekoique/arkos`.

## References

### This task's research files

These persist in the task archive as part of `rfc001-arkos`'s deliverable. They are the substantive evidence behind the *Self-improvement*, *Open questions*, and *Prior art* sections:

- `.ark/tasks/rfc001-arkos/research/self-improving-agents.md` — survey of ~20 self-improving agent systems, grounding-signal hierarchy, anti-patterns, eight open questions for first-of-its-kind work at OS scale.
- `.ark/tasks/rfc001-arkos/research/recursive-decomposition.md` — survey of classical (HTN, WBS, FDD, INVEST/SPIDR) and LLM-era (Plan-and-Solve, ToT, ADaPT, MetaGPT, ChatDev, SWE-agent, OpenHands, AgentOrchestra, Cognition / Anthropic positions) decomposition; what's never been done at OS scale.
- `.ark/tasks/rfc001-arkos/research/self-generating-specs.md` — the load-bearing dichotomy between behavioral/mechanical rules (tractable with external fitness) and architectural rules (no public system has solved without external grounding); Ark's current rule L-4 as the conservative response.

### Primary citations

- Ge et al., *AIOS: LLM Agent Operating System*, COLM 2025. [arXiv:2403.16971](https://arxiv.org/abs/2403.16971).
- Park et al., *Generative Agents*, UIST 2023. [PDF](https://3dvar.com/Park2023Generative.pdf).
- Wang et al., *Voyager*, NeurIPS 2023. [arXiv:2305.16291](https://arxiv.org/abs/2305.16291).
- Shinn et al., *Reflexion*, NeurIPS 2023. [arXiv:2303.11366](https://arxiv.org/abs/2303.11366).
- Madaan et al., *Self-Refine*, NeurIPS 2023. [arXiv:2303.17651](https://arxiv.org/abs/2303.17651).
- DeepMind, *AlphaEvolve* whitepaper, 2025. [arXiv:2506.13131](https://arxiv.org/abs/2506.13131), [DeepMind blog](https://deepmind.google/blog/alphaevolve-a-gemini-powered-coding-agent-for-designing-advanced-algorithms/).
- DeepMind, *FunSearch*, Nature 2023. [s41586-023-06924-6](https://www.nature.com/articles/s41586-023-06924-6).
- Zhang et al., *Darwin Gödel Machine*, 2025. [arXiv:2505.22954](https://arxiv.org/abs/2505.22954), [Sakana](https://sakana.ai/dgm/), [The Register coverage on reward-hacking](https://www.theregister.com/2025/06/02/self_improving_ai_cheat/).
- Yang et al., *SWE-agent*, NeurIPS 2024. [arXiv:2405.15793](https://arxiv.org/abs/2405.15793).
- Wang et al., *OpenHands*, ICLR 2025. [arXiv:2407.16741](https://arxiv.org/abs/2407.16741).
- Hong et al., *MetaGPT*, ICLR 2024. [arXiv:2308.00352](https://arxiv.org/abs/2308.00352).
- Prasad et al., *ADaPT*, NAACL 2024. [arXiv:2311.05772](https://arxiv.org/abs/2311.05772).
- Cognition, *Don't Build Multi-Agents*. [blog](https://cognition.ai/blog/dont-build-multi-agents).
- Anthropic, *How we built our multi-agent research system*. [blog](https://www.anthropic.com/engineering/multi-agent-research-system).
- Panickssery, Bowman & Feng, *LLM Evaluators Recognize and Favor Their Own Generations*, NeurIPS 2024. [arXiv:2404.13076](https://arxiv.org/abs/2404.13076).
- Bai et al., *Constitutional AI*, 2022. [arXiv:2212.08073](https://arxiv.org/abs/2212.08073).
- Hilton et al., *Evaluating Goal Drift in Language Model Agents*, AIES 2025. [arXiv:2505.02709](https://arxiv.org/abs/2505.02709).
- *Goodhart's Law in Reinforcement Learning*, ICLR 2024. [proceedings PDF](https://proceedings.iclr.cc/paper_files/paper/2024/file/6ad68a54eaa8f9bf6ac698b02ec05048-Paper-Conference.pdf).
- Augment Code, *How to Build Your AGENTS.md*, 2026. [guide](https://www.augmentcode.com/guides/how-to-build-agents-md).
- METR, *Measuring AI ability to complete long tasks*, 2025. [report](https://metr.org/blog/2025-03-19-measuring-ai-ability-to-complete-long-tasks/).
- *On the Limits of Self-Improving in LLMs*, 2026. [arXiv:2601.05280](https://arxiv.org/html/2601.05280v2).
- AutoGPT issue #15, *Recursive Self Improvement*. [GitHub](https://github.com/Significant-Gravitas/AutoGPT/issues/15).
- *latent.space — Can coding agents self-improve?*. [post](https://www.latent.space/p/self-improving).
- GitHub Spec-Kit. [repo](https://github.com/github/spec-kit).
- Kiro. [site](https://kiro.dev/).
- Trellis. [docs](https://docs.trytrellis.app/) (read locally at `reference/Trellis/`).
- Linux Test Project (LTP). [project](https://linux-test-project.github.io/).
