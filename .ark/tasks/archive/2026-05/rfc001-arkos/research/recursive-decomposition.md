# Research: Recursive / Hierarchical Task Decomposition (Prior Art for ArkOS)

- Query: history & current state of recursive task decomposition for software engineering and LLM agents; what's borrowable, what's failed, what's never been tried at the scale ArkOS is aiming for ("build a POSIX-compatible OS that passes LTP")
- Scope: external (literature & blog survey)
- Date: 2026-05-10

## 1. Survey

This survey is organized in three layers:

1. **Classical software engineering & planning** — pre-LLM, where the algorithms are crisp and the size data is real.
2. **AI-agent decomposition** — 2022-2026 LLM literature, where most algorithms are ad-hoc but the engineering blogs ("don't build multi-agents," Anthropic's research system) are the closest thing to applied wisdom.
3. **OS-construction prior art** — explicitly: has anyone done this? What size has been demonstrated end-to-end?

Each entry below cites the primary source. Where the source is a paper, the arXiv URL is given; where it's an engineering blog, the canonical URL is given.

---

### 1.1 Classical software engineering & planning

#### Hierarchical Task Network (HTN) planning — STRIPS lineage, SHOP / SHOP2

HTN planning is the formal ancestor of every recursive-decomposition idea in this document. A planning problem is specified as a set of *tasks*, classified into:

- **primitive tasks** — directly executable (≈ STRIPS actions).
- **compound tasks** — must be decomposed into a network of simpler tasks via *methods*.
- **goal tasks** — the top-level objective.

A *solution* is an executable sequence of primitive tasks obtainable by recursively decomposing compound tasks under ordering and precondition constraints. SHOP (totally-ordered) and SHOP2 (partially-ordered, the standard reference) are the canonical implementations from the University of Maryland. SHOP2 has been used outside toy planning — most notably for OWL-S Web-service composition, where the analogy "compound task ↔ composite service" maps directly onto today's "compound task ↔ subagent invocation" framing.

Sources:
- [Hierarchical task network — Wikipedia](https://en.wikipedia.org/wiki/Hierarchical_task_network)
- [SHOP (Simple Hierarchical Ordered Planner) — UMD](http://www.cs.umd.edu/projects/shop/description.html)
- [Georgievski & Aiello, "HTN planning: Overview, comparison, and beyond," AIJ 222 (2015)](https://www.sciencedirect.com/science/article/pii/S0004370215000247)
- [Bercher et al., "An Overview of Hierarchical Task Network Planning," arXiv:1403.7426](https://arxiv.org/pdf/1403.7426)

What HTN gives ArkOS: a vocabulary (primitive vs. compound, method, decomposition) and a known-hard observation — *method authoring* (which compound→subtasks rule fires when?) is where domain knowledge actually lives. LLMs currently substitute for the method library by generating decompositions on demand; this is the single biggest deviation from classical HTN.

#### Work Breakdown Structures (PMI / project management)

WBS is the project-management ancestor. It is *deliverable-oriented* (not action-oriented) and governed by two rules that are operationally relevant to ArkOS:

- **The 100% Rule** — "the sum of the work at the *child* level must equal 100% of the work represented by the *parent*, and the WBS should not include any work that falls outside the actual scope of the project." (PMI)
- **The 8/80 Rule** — informal heuristic: leaf work-packages should take between 8 hours and 80 hours of effort. Practitioners aim for 3-5 levels of nesting; beyond 5 the WBS is considered over-decomposed.

Sources:
- [PMI — "Work Breakdown Structure (WBS) — Basic Principles"](https://www.pmi.org/learning/library/work-breakdown-structure-basic-principles-4883)
- [Wikipedia — Work breakdown structure](https://en.wikipedia.org/wiki/Work_breakdown_structure)
- [Asana — WBS guide & rules](https://asana.com/resources/work-breakdown-structure)

Key for ArkOS: the 100% rule is a *correctness invariant* on decomposition that LLM-driven systems routinely violate (drift, missing-step errors). It is not a soft heuristic; it can be checked.

#### Feature-Driven Development (FDD) — Coad / De Luca / Palmer

FDD (Coad & De Luca, 1997, scaled at a 50-person Singapore bank project) explicitly treats *feature* as the unit of decomposition with a hard time-box: a feature must complete in **≤ 2 weeks**. The five FDD activities — develop overall model → build feature list → plan by feature → design by feature → build by feature — are themselves a fixed five-level decomposition pipeline.

Sources:
- [Wikipedia — Feature-driven development](https://en.wikipedia.org/wiki/Feature-driven_development)
- [Palmer & Felsing, *A Practical Guide to Feature-Driven Development* (2002)](https://www.amazon.com/Practical-Guide-Feature-Driven-Development/dp/0130676152)

Borrowable: time-boxed leaf definition, separation of "design by X" and "build by X" as distinct phases — Ark already does this (PRD/PLAN/REVIEW vs. EXECUTE/VERIFY).

#### Story splitting heuristics — INVEST, SPIDR

INVEST (Bill Wake, 2003) and SPIDR (Mike Cohn, ~2017) are the practitioner-facing splitting heuristics. INVEST: *Independent, Negotiable, Valuable, Estimable, Small, Testable* — "small" defined by the property that 6-10 stories fit in a sprint. SPIDR: five concrete techniques — **S**pikes, **P**aths, **I**nterfaces, **D**ata, **R**ules.

- *Spikes*: when knowledge is missing, split off a research story first.
- *Paths*: split a workflow with multiple branches into one story per branch.
- *Interfaces*: deliver one OS / one browser / one client at a time.
- *Data*: simplify or restrict the data domain.
- *Rules*: separate business rules from core flow.

Sources:
- [Mountain Goat Software — SPIDR](https://www.mountaingoatsoftware.com/blog/five-simple-but-powerful-ways-to-split-user-stories)
- [Mountain Goat Software — SPIDR poster (PDF)](https://www.mountaingoatsoftware.com/uploads/blog/spidr-poster.pdf)
- [SOC ADK — design-practice-repository: Story Splitting](https://socadk.github.io/design-practice-repository/activities/DPR-StorySplitting.html)

Borrowable: **SPIDR is essentially a fixed library of decomposition methods**, in HTN terms. ArkOS could prompt-engineer leaf agents to attempt SPIDR splits before committing to "this is atomic."

#### "Split until each leaf takes < 1 day" — empirical evidence

The user's brief asked for evidence of any specific size threshold. There is **no rigorous evidence** that any specific number (1 day, 1 ideal day, 8 hours, 1 sprint, 1 PR) is causally optimal. Sutherland's claim that story points outperform hours is empirically contested:

- A Microsoft paper Sutherland cites in support of story-points actually estimated *person hours* with planning poker, not story points (a documented mis-citation).
- Magne Jørgensen's independent reviews recommend *comparing to similar past projects* and using *work hours*, finding story points led to less accuracy.
- The Standish CHAOS Report does not list estimation method among top success/failure factors.

Sources:
- [Vitalii Oborskyi, "Story Points vs. Time-Based Estimation: What Does Research Say?"](https://medium.com/agileinsider/story-points-vs-time-based-estimation-what-does-research-say-84613cc91c7f)
- [Mike Cohn — *Agile Estimating and Planning* (summary)](https://williammeller.com/agile-estimating-and-planning-by-mike-cohn/)

Implication for ArkOS: *don't claim* a specific token / time / LOC threshold for "atomic." Pick one operationally (it's a knob) and accept that the literature does not endorse a magic number.

#### Two-pizza team & module sizing

The two-pizza team (Bezos / Amazon, popularized by Fowler) is 5-8 people, ~15 max. Coupled with low-coupling / high-cohesion module heuristics from Constantine, the rule of thumb for "what fits in one head" maps roughly onto modules of a few KLOC. Operationally this is what corresponds to a single Ark task today.

Sources:
- [Martin Fowler — Two Pizza Team](https://martinfowler.com/bliki/TwoPizzaTeam.html)
- [Wikipedia — Coupling (computer programming)](https://en.wikipedia.org/wiki/Coupling_(computer_programming))

#### Bottom-up integration testing

The classical software-engineering model for assembling a tree of components is bottom-up integration: test leaves with stubs, integrate upward, replace stubs with real callers, test the next level. The mechanism that makes this work is **the test suite as integration oracle** — when you replace a stub with a real module, the test suite tells you whether the integration broke. This is the *only* part of recursive software composition where the field has a robust, scalable solution.

Source:
- [GeeksforGeeks — Steps in Bottom Up Integration Testing](https://www.geeksforgeeks.org/software-engineering/steps-in-bottom-up-integration-testing/)

For ArkOS this is the most important thing to borrow and the most important thing to budget for: **without a test suite at every level of the tree, integration becomes guesswork.** The user's POSIX-OS example is well-suited because LTP itself is the integration oracle at the root; the question is what the oracles are at intermediate levels (driver self-tests? syscall conformance? per-subsystem fuzzers?).

---

### 1.2 AI-agent decomposition (2022-2026)

#### Plan-and-Solve prompting (Wang et al., ACL 2023)

Plan-and-Solve (PS) replaces the "Let's think step by step" prompt with "Let's first understand the problem and devise a plan, then carry out the plan." Two-phase, single-LLM, no recursion. PS targets *missing-step errors* — the failure mode where CoT skips an intermediate step because the model never enumerated it.

Source:
- [Wang et al., "Plan-and-Solve Prompting," arXiv:2305.04091](https://arxiv.org/abs/2305.04091)

#### Tree of Thoughts (Yao et al., NeurIPS 2023)

ToT generalizes CoT into a search tree of *thoughts*. Decomposition, generation, evaluation, and search are independently parameterizable. ToT achieves 74% on Game-of-24 vs. 4% for CoT with GPT-4 — but it operates over *thoughts* (~50-100 token units) rather than over engineering tasks, and the depth budget is small (≤ 5 in published experiments).

Source:
- [Yao et al., "Tree of Thoughts," arXiv:2305.10601](https://arxiv.org/abs/2305.10601)
- [GitHub — princeton-nlp/tree-of-thought-llm](https://github.com/princeton-nlp/tree-of-thought-llm)

Limitation for ArkOS: ToT's evaluator is a same-size LLM critiquing thoughts; it does not generalize to "is this engineering subtask atomic?" without external grounding (test suites, compilation success).

#### ReAct (Yao et al., ICLR 2023)

ReAct interleaves *Reason* and *Act* steps. The reasoning steps "(1) decompose the goal, (2) track subgoal completion, (3) determine the next subgoal, (4) reason via commonsense" are flat — there is no recursion, but the trace is the substrate every later agent (SWE-agent, OpenHands, Claude Code) sits on.

Source:
- [Yao et al., "ReAct," arXiv:2210.03629](https://arxiv.org/abs/2210.03629)

#### Reflexion (Shinn et al., NeurIPS 2023)

Reflexion adds a self-critique loop: actor proposes, evaluator judges, self-reflection adds a verbal lesson to memory, retry. +22% on AlfWorld over 12 trials, +20% on HotpotQA, +11% on HumanEval. Mechanism for "agent claims it's done but isn't" — the evaluator says no, the reflection feeds back into the next attempt.

Source:
- [Shinn et al., "Reflexion," arXiv:2303.11366](https://arxiv.org/abs/2303.11366)
- [GitHub — noahshinn/reflexion](https://github.com/noahshinn/reflexion)

#### ADaPT (Prasad et al., NAACL 2024) — the most ArkOS-shaped paper found

ADaPT ("As-Needed Decomposition and Planning") is the paper that most directly addresses ArkOS's halting question. The mechanism:

1. Executor LLM attempts an atomic task.
2. LLM self-assesses whether it succeeded.
3. If it failed, the planner LLM decomposes the task (with a logical operator: AND requires all subtasks; OR requires any).
4. Recurse.

This is the recursive decomposition pattern with a **failure-driven halting condition**: don't decompose preemptively, decompose when the executor proves it can't handle the leaf as given. Reported gains: +28.3% on ALFWorld, +27% on WebShop, +33% on TextCraft.

Source:
- [Prasad et al., "ADaPT: As-Needed Decomposition and Planning with Language Models," arXiv:2311.05772](https://arxiv.org/abs/2311.05772)
- [Project page — allenai.org/adaptllm](https://allenai.github.io/adaptllm/)

The atomicity criterion ADaPT uses: "the LLM executor should reliably execute atomic skills" — environment-defined (in ALFWorld: take, put, clean, heat, …). For software engineering the analogue would be: "the leaf is whatever the EXECUTE phase's coding agent reliably completes given the PLAN."

#### HuggingGPT (Shen et al., NeurIPS 2023)

HuggingGPT uses ChatGPT as a router: parse user request → decompose into subtasks → select Hugging Face model per subtask → execute → summarize. Single decomposition level (no recursion); subtasks are leaf-level model calls. The contribution is the *catalog-based dispatch* — subtasks are typed against a registry of available executors. ArkOS-relevant insofar as the registry constrains decomposition: you only split into work the system knows how to dispatch.

Source:
- [Shen et al., "HuggingGPT," NeurIPS 2023](https://openreview.net/forum?id=yHdTscY6Ci)

#### MetaGPT (Hong et al., ICLR 2024)

MetaGPT encodes a Standard Operating Procedure as roles: Product Manager → Architect → Project Manager → Engineer → QA Engineer. Each role consumes structured artifacts (PRD, design docs, flowcharts) from the previous role. Reported: 85.9% / 87.7% pass@1 on HumanEval / MBPP. Key claim: **structured intermediate artifacts** (not just messages) materially improve target code generation.

Source:
- [Hong et al., "MetaGPT," ICLR 2024 — arXiv:2308.00352](https://arxiv.org/abs/2308.00352)
- [GitHub — FoundationAgents/MetaGPT](https://github.com/FoundationAgents/MetaGPT)

Comparison to Ark: Ark already mandates structured artifacts (PRD.md, PLAN.md, REVIEW.md, VERIFY.md) per task. MetaGPT validates this design choice but operates at the level of *one* deliverable; ArkOS's question is what happens when you nest it.

#### ChatDev (Qian et al., ACL 2024)

ChatDev is the explicit "AI software company" — waterfall lifecycle (design → coding → testing → documenting) executed by communicating role-agents. Notable for "communicative dehallucination": the next agent must request specific details from the previous before responding. Same shape as MetaGPT; flat (single layer of role decomposition); evaluated on small applications (snake game, Gomoku, etc., not multi-KLOC systems).

Source:
- [Qian et al., "ChatDev," ACL 2024 — arXiv:2307.07924](https://arxiv.org/abs/2307.07924)
- [GitHub — OpenBMB/ChatDev](https://github.com/OpenBMB/ChatDev)

#### AgentCoder (Huang et al., 2023→2024)

Three roles: programmer, test designer, test executor. Iterative: programmer codes → test designer writes tests → test executor runs → programmer fixes. Pass@1: 91.5% (GPT-4) / 84.1% (GPT-3.5) on HumanEval. Mechanism analogous to ADaPT but specialized to code.

Source:
- [Huang et al., "AgentCoder," arXiv:2312.13010](https://arxiv.org/abs/2312.13010)
- [GitHub — huangd1999/AgentCoder](https://github.com/huangd1999/AgentCoder)

#### SWE-agent (Yang et al., NeurIPS 2024) and OpenHands / OpenDevin (Wang et al., ICLR 2025)

SWE-agent introduced the Agent-Computer Interface (ACI): a deliberately-restricted action space (file viewer with iterative search, structured editor, guarded shell) with concise feedback. 12.47% on SWE-bench (vs. prior 3.8%) — empirically the ACI matters more than chain-of-thought tricks at this scale.

OpenHands (formerly OpenDevin) extends this to a multi-agent platform with `AgentDelegateAction` for hierarchical delegation; CodeActAgent is the generalist, BrowserAgent and others are specialists. The platform now ships an SDK that separates agent logic / execution / interface.

Sources:
- [Yang et al., "SWE-agent," NeurIPS 2024 — arXiv:2405.15793](https://arxiv.org/abs/2405.15793)
- [GitHub — SWE-agent/SWE-agent](https://github.com/SWE-agent/SWE-agent)
- [Wang et al., "OpenHands," ICLR 2025 — arXiv:2407.16741](https://arxiv.org/abs/2407.16741)
- [GitHub — OpenHands/OpenHands](https://github.com/OpenHands/OpenHands)

#### Devin / Cognition — the "don't build multi-agents" position

Cognition's published position is the strongest counter-current to recursive multi-agent decomposition in the literature. Two posts:

- ["Don't Build Multi-Agents"](https://cognition.ai/blog/dont-build-multi-agents) — Walden Yan, Cognition. Core argument: *parallel sub-agents make implicit, conflicting decisions because they don't share context*. Recommended principles:
  - **Share context.** Pass full agent traces (not summaries) to all agents.
  - **Single-threaded writes.** Multiple agents may *read* / *retrieve*; only one *writes*.
  - **Beware decision fragmentation.** Each parallel sub-agent makes implicit choices the others cannot see.
- ["Multi-Agents: What's Actually Working"](https://cognition.ai/blog/multi-agents-working) — narrower class that does work: read-only sub-agents (web search, code search) feeding a single writer.

Devin itself (SWE-bench: 13.86%) was reported as a more-or-less single-agent pipeline with extensive tooling.

Sources:
- [Cognition — "Don't Build Multi-Agents"](https://cognition.ai/blog/dont-build-multi-agents)
- [Cognition — "Multi-Agents: What's Actually Working"](https://cognition.ai/blog/multi-agents-working)
- [Cognition — "Introducing Devin"](https://cognition.ai/blog/introducing-devin)

This is the most important position for ArkOS to engage with seriously, because it argues against the basic recursive-fanout pattern.

#### Anthropic's multi-agent research system — the counter-position

Anthropic's published architecture (Research feature) uses an orchestrator-worker pattern: a Lead Researcher (Claude Opus 4) decomposes the query, spawns subagents (Claude Sonnet 4), each subagent executes independently and returns findings. Reported: 90.2% improvement over single-agent on internal evaluations; **token usage explains 80% of the variance in performance**.

Critical caveats Anthropic publishes:
- Subagents need explicit objective, output format, tool guidance, task boundaries — *otherwise effort is mis-scoped*.
- Embedded "scaling rules" in prompts because agents can't judge appropriate effort.
- Multi-agent works for *research* (read-only, parallelizable, low integration cost) — Anthropic does not extend the claim to coding.

Source:
- [Anthropic — "How we built our multi-agent research system"](https://www.anthropic.com/engineering/multi-agent-research-system)

The Cognition / Anthropic split is the live debate: read-only fanout is fine; write fanout is structurally hazardous.

#### AgentOrchestra (Zhang et al., 2025)

AgentOrchestra is a hierarchical multi-agent framework with an explicit central Planning Agent that decomposes objectives, delegates to specialists, aggregates feedback, and replans dynamically. SOTA on GAIA (82.42%), SimpleQA (95.3%), HLE (25.9%). Distinct from MetaGPT/ChatDev: the planner *retains a global perspective* and can update the plan mid-execution.

Source:
- [Zhang et al., "AgentOrchestra," arXiv:2506.12508](https://arxiv.org/abs/2506.12508)

#### Chain of Agents (Zhang et al., NeurIPS 2024)

Long-context analogue: workers handle text chunks, manager synthesizes. Up to +10% over RAG / long-context / multi-agent baselines. Mechanism for ArkOS: this is what *integration of leaf outputs* looks like when leaves return text rather than code.

Source:
- [Zhang et al., "Chain of Agents," NeurIPS 2024 — arXiv:2406.02818](https://arxiv.org/abs/2406.02818)

#### AutoGen / LangGraph / CrewAI — the orchestration platforms

AutoGen (Microsoft) frames it as a *conversation*; LangGraph (LangChain) frames it as a *graph* with explicit transition probabilities. CrewAI is the role-based orchestrator. None of these prescribes a decomposition algorithm — they're substrate.

Sources:
- [Wu et al., "AutoGen," COLM 2024](https://arxiv.org/pdf/2308.08155)
- [LangGraph — multi-agent workflows](https://blog.langchain.com/langgraph-multi-agent-workflows/)

#### AutoGPT / BabyAGI — what failure looks like

The 2023 first-wave autonomous agents are documented failures of naïve recursion:

- **Infinite looping** — vague termination criteria; "perfection bias" leads to endless self-improvement loops.
- **Memory drift** — BabyAGI lost track of completed tasks and re-planned in circles.
- **Cost explosion** — recursive calls compound at $0.03-0.06/1K tokens.
- Amazon's 2023 evaluation: AutoGPT 24% success on a shopping task.

Sources:
- [unite.ai — AutoGPT/BabyAGI integrate recursion](https://www.unite.ai/open-source-auto-gpt-babyagi-integrate-recursion/)
- [BabyAGI issue #56 — task decomposition (one task to many tasks)](https://github.com/yoheinakajima/babyagi/issues/56)
- [Srikanth Machiraju — "Notorious Agent Loops"](https://techtalkwithsriks.medium.com/notorious-agent-loops-c4cc05b859b5)

#### Goal drift in language model agents — quantified

Hilton et al. ("Evaluating Goal Drift in Language Model Agents," AIES 2025): drift is a function of context length and competing objectives. **Claude 3.5 Sonnet maintains goal adherence up to 100K tokens; GPT-4o-mini drifts at all tested lengths.** The primary mechanism is *pattern-matching*, not goal reasoning.

Source:
- [Hilton et al., "Evaluating Goal Drift in Language Model Agents," arXiv:2505.02709](https://arxiv.org/abs/2505.02709)

Implication for ArkOS: every level of recursion adds context that compounds drift risk. Re-anchoring goal at each level is mandatory; otherwise drift grows with depth.

#### Recursive self-improvement — the theoretical limits

Recent theory ("On the Limits of Self-Improving in LLMs," 2026) formalizes self-training as a dynamical system and shows that under autonomy (the system relies predominantly on its own outputs), it converges to a degenerate fixed point regardless of architecture. Practical implication: an external grounding signal (tests, compilation, benchmark) is structurally necessary for recursion to remain stable.

Source:
- [Sahin Ahmed — "Engineering Challenges and Failure Modes in Agentic AI Systems"](https://medium.com/@sahin.samia/engineering-challenges-and-failure-modes-in-agentic-ai-systems-a-practical-guide-f9c43aa0ae3f)
- [arXiv:2601.05280 — "On the Limits of Self-Improving in LLMs"](https://arxiv.org/html/2601.05280v2)

---

### 1.3 Git worktrees & parallel agent isolation

Ark's worktree-per-task model has industry parallels. Worktrees give each agent an isolated working directory + git index sharing one object store, deferring conflict to merge time. The 2025 practitioner consensus:

- Worktrees prevent file-level *silent overwrites* but do not warn when two worktrees touch the same file.
- "Rebase Before PR" is the dominant integration convention.
- Worktrees isolate branches, *not* runtimes — port collisions and shared databases still need explicit handling.
- `git merge-tree` enables pre-flight conflict detection without merging — usable by an orchestrator to redirect / serialize conflicting work.

Sources:
- [Augment Code — "How to Use Git Worktrees for Parallel AI Agent Execution"](https://www.augmentcode.com/guides/git-worktrees-parallel-ai-agent-execution)
- [Penligent — "Git Worktrees Need Runtime Isolation"](https://www.penligent.ai/hackinglabs/git-worktrees-need-runtime-isolation-for-parallel-ai-agent-development/)
- [Zylos Research — "Git Worktree Isolation Patterns for Parallel AI Agent Development"](https://zylos.ai/research/2026-02-22-git-worktree-parallel-ai-development)

Ark's worktree model maps onto this directly. ArkOS would inherit it; the open question is whether `git merge-tree`-driven pre-flight conflict detection is added as a first-class step before parallel sub-agent dispatch.

---

### 1.4 OS-construction prior art

The honest finding: **no project has produced a POSIX-compatible OS end-to-end via LLM-driven recursive decomposition.** Adjacent work exists at three levels:

1. **LLM ↔ kernel boundary, narrow tasks (yes, working).**
   - SchedCP — autonomous LLM agents synthesize and deploy eBPF Linux scheduler policies; 1.79× speedup, 13× cost reduction. *Generates one scheduler module*, not a kernel.
     - [arXiv:2509.01245](https://arxiv.org/html/2509.01245v2)
   - LLM-Driven Kernel Evolution (Linux drivers) — four cooperative agents (Prompt, Coding, Patch Fix, Static Analysis) iteratively refine driver patches. *Driver-scale, not OS-scale.*
     - [arXiv:2511.18924](https://arxiv.org/pdf/2511.18924)
   - KernelFalcon — autonomous GPU kernel generation against a numerical correctness oracle. *Single-kernel scope.*
     - [PyTorch blog — KernelFalcon](https://pytorch.org/blog/kernelfalcon-autonomous-gpu-kernel-generation-via-deep-agents/)

2. **AIOS — "LLM Agent Operating System" (Ge et al., COLM 2025).** This is *not* an LLM-built OS; it's an OS *for LLM agents*. Resource isolation, scheduling, memory management for multi-agent runtimes. Up to 2.1× speedup serving agent frameworks.
   - [Ge et al., arXiv:2403.16971](https://arxiv.org/abs/2403.16971)

3. **End-to-end software generation benchmarks — small scale.**
   - CLI-Tool-Bench (2026) — 100 real CLI repos, 0-to-1 generation evaluation. CLI-tool scale (typically <5KLOC).
   - SWE-bench / SWE-bench Pro / SWE-EVO — *modify* existing repos. Long-horizon evolution is now the active frontier; greenfield OS generation is not.
   - MetaGPT / ChatDev demos: small applications (Snake, Gomoku, simple CRUD). Not multi-KLOC, not stateful, no kernel-level concerns.

Reference points for OS scale:
- xv6 (MIT, teaching OS): ~10K LOC.
- Linux 0.01: 8,413 lines.
- TempleOS (one person): ~100K LOC including GUI, drivers, userspace.
- LFS (Linux From Scratch): a *recipe* for building Linux from source — not generation, integration only.

Sources:
- [OSnews — "An operating system in 1000 lines"](https://www.osnews.com/story/141502/an-operating-system-in-1000-lines/)
- [Linux From Scratch](https://www.linuxfromscratch.org/)

The user's "POSIX OS that passes LTP" target therefore sits **at least one order of magnitude beyond** anything published. A POSIX-compliant kernel + userland that passes a meaningful subset of LTP is in the 100K-1M LOC range; the largest LLM-generated coherent system documented is in the 1-10K LOC range.

---

## 2. Comparison table

| System | Split criterion | Leaf definition | Integration model | Failure mode handled | Demonstrated scale |
|---|---|---|---|---|---|
| HTN / SHOP2 | Method library — author-supplied compound→subtask rules | Primitive task (matches STRIPS action) | Sequential plan execution | Plan-search backtracking; precondition violation | Toy planning + Web service composition |
| WBS (PMI) | Deliverable boundary; 100% rule | 8-80 hours; 3-5 nesting levels | Project schedule rollup | Cost / schedule variance reporting | Industrial — KLOC to MLOC projects |
| FDD | Client-valued feature | ≤ 2 weeks per feature | Five-phase pipeline (model → list → plan → design → build) | Per-feature milestone tracking (6 milestones × N features) | 50-person, 15-month projects |
| INVEST / SPIDR | Spike / Path / Interface / Data / Rule | "6-10 fit in a sprint" | Sprint-level integration | Story dependency, vertical slicing | Sprint-scale teams |
| Plan-and-Solve | Single LLM, two-phase | LLM decides | Concatenated reasoning | Missing-step error | Math / CSR benchmarks |
| Tree of Thoughts | LLM generates k thoughts per node | Thought (50-100 tokens) | Search over tree (BFS/DFS) | Local reasoning errors | Game of 24, mini-crosswords |
| ReAct | Verbal subgoal in trace | "Determine next subgoal" | Linear trace | Exception handling via reasoning | ALFWorld, HotpotQA |
| Reflexion | N/A (retry-based) | Trial = full episode | Retry with reflection memory | Goal not achieved on attempt | +22% on AlfWorld in 12 trials |
| ADaPT | **Failure-driven**: decompose only when executor fails | Executor's reliable atomic skill | AND/OR composition tree | Executor capability mismatch | +28-33% on ALFWorld / WebShop / TextCraft |
| HuggingGPT | Catalog dispatch | Hugging Face model invocation | LLM summary aggregation | Multi-modal task decomposition | Single layer; no recursion |
| MetaGPT | Role-based SOP | Engineer-coded artifact | Structured artifact pipeline | Inconsistent intermediate communication | HumanEval/MBPP single-app scale |
| ChatDev | Waterfall phase | Phase output | Sequential phase chain | Hallucination via dehallucination dialog | Snake / Gomoku / small apps |
| AgentCoder | Programmer / test-designer / test-executor | Function with tests | Iterative refinement | Test-driven correction | HumanEval / MBPP |
| SWE-agent | Single-agent (no recursion) | LM action under ACI | Repo-state evolution | Tool error → ACI feedback | SWE-bench (1-issue tasks) |
| OpenHands | Hierarchical via AgentDelegateAction | Sub-agent termination | Delegation semantics | Specialist routing | SWE-bench scale |
| Devin / Cognition | **Single-threaded writer** | Whatever the writer can do | Single coherent context | Sibling write conflict avoided by design | SWE-bench; production paid users |
| Anthropic Research | Read-only sub-agent fanout | Sub-agent's "objective + output format + tools + boundary" | Lead synthesizes findings | Effort scoping via embedded scaling rules | Research feature; 90.2% over single-agent |
| AgentOrchestra | Central planner; replan on feedback | Specialist agent's task | Aggregated feedback + replan | Plan staleness via replanning | GAIA 82.42% |
| Chain of Agents | Long-context chunking | One chunk + reading task | Manager synthesis | Long-context focus loss | NeurIPS 2024; up to +10% |
| AutoGPT / BabyAGI | LLM picks next subtask | "When agent decides done" | None coherent | None — fails on loops, drift | <24% on benchmarks |
| Cognition Devin (recommended) | Don't fanout writes | Whatever the single writer handles | N/A (single thread) | Sibling-decision conflict | Production |

---

## 3. Patterns to borrow

### 3.1 Failure-driven, not preemptive, decomposition (ADaPT)

Don't decompose "because it's a deep task." Decompose *when* the executor at the current level demonstrably can't handle the leaf. This converts decomposition depth from a static parameter into a function of executor capability — and makes the tree shallower as models improve, without re-architecting.

### 3.2 100% rule as a checkable invariant (PMI WBS)

The 100% rule says: children sum to parent. This is checkable: if a parent task says "implement filesystem" and the children are "implement VFS layer + implement ext2," can you justify that the union covers the parent? LLMs routinely produce decompositions that violate the 100% rule (omit, double-count). A coverage check at each split is a cheap, real correctness gate.

### 3.3 Fixed library of split methods (SPIDR / HTN methods)

Don't ask the LLM to "decompose creatively." Constrain it to a small library of named decomposition methods (paths, interfaces, data, rules, spikes, plus software-specific: layer-by-layer, module-by-module, test-first, feature-by-feature). Forces structured choice; makes decomposition auditable.

### 3.4 Structured artifacts at every level (MetaGPT)

MetaGPT's measured contribution is not roles — it's *structured intermediate outputs*. Ark's PRD/PLAN/REVIEW/EXECUTE/VERIFY artifacts already realize this; ArkOS should preserve and recurse it (each non-leaf node has its own PRD/PLAN/REVIEW summarizing the children).

### 3.5 Read-only fanout, single-threaded writes (Cognition)

Strongest practitioner consensus 2025-2026: parallel sub-agents are safe when they *gather*, hazardous when they *write*. ArkOS should preserve fanout for research / context-gathering / review while keeping writes single-threaded per worktree. Worktree-per-task already enforces this at the filesystem level; the design question is whether sub-tasks within a worktree fan out.

### 3.6 Bottom-up integration with test oracles at every level

The test suite is the *only* reliable composition oracle the field has. Every parent task needs an integration test that consumes the children's outputs; without it, integration is hand-waving. For an OS this means: per-syscall conformance tests (LTP-style at the leaf), per-subsystem test harnesses (mid-level), full-system boot+LTP at the root. Budget for the test suite *as a deliverable* of decomposition, not an afterthought.

### 3.7 Re-anchoring at every level (goal drift)

Drift grows with context length. Each recursion level should restate the root intent in each sub-task's prompt, not just the immediate parent's intent. Hilton et al.'s 100K-token boundary on Claude 3.5 Sonnet is the operating budget; cross it and drift dominates.

### 3.8 Pre-flight conflict detection between siblings (`git merge-tree`)

`git merge-tree` allows checking whether two worktrees would conflict without performing the merge. ArkOS could run this between sibling sub-tasks before dispatch and either redirect or serialize on detected conflict. This is a concrete answer to sibling interference that doesn't require any agent intelligence.

### 3.9 Embed scaling rules in prompts (Anthropic)

Anthropic's published lesson: agents can't judge appropriate effort, so encode it. Per-task "expected-size" guidance — "this leaf should produce ≤ 200 LOC; if you're producing more, halt and request decomposition" — is a cheap, effective floor.

---

## 4. Patterns to avoid

### 4.1 Preemptive deep recursion without grounding

AutoGPT-style "decompose to whatever depth the LLM thinks needed without external feedback." Documented to loop, drift, and burn tokens with <25% success on benchmarks. Always pair recursion with an external grounding signal (test, compile, benchmark) at each level — see "On the Limits of Self-Improving in LLMs."

### 4.2 Parallel writers (Cognition)

Two sub-agents editing the same file with no shared context will make incompatible implicit choices. The sibling-interference problem is structural, not a prompt-engineering problem. Worktree isolation defers conflict to merge time but doesn't eliminate it.

### 4.3 The "boring middle layer" / hand-waving abstraction

In top-down decomposition, the top is strategy ("build a POSIX OS"), the leaves are code ("write `sys_open`"), and the middle is where decomposition output reads like *"design the filesystem subsystem"* — non-actionable vocabulary that neither plans nor executes. The practitioner principle from agentic-code work: "*The right amount of complexity is the minimum needed for the current task — three similar lines of code is better than a premature abstraction.*" Translated to decomposition: avoid mid-tree nodes that exist only to chunk the tree; each non-leaf should carry an integration test that proves its children compose.

Source for the principle:
- [arXiv:2603.05344 — Building Effective AI Coding Agents for the Terminal](https://arxiv.org/html/2603.05344v2)

### 4.4 Flat decomposition that loses architectural coherence

Splitting an OS into 200 leaves in one shot loses cross-cutting concerns (boot order, ABI compatibility, security model). The 100% rule alone doesn't catch this — it checks coverage, not coherence. Architectural decisions need to be *committed at the parent level* before children are dispatched, and pinned in a parent-level artifact (Ark's PRD) the children must respect.

### 4.5 Goal-restatement-by-summary

Summarizing the root goal into shorter and shorter forms at each level is exactly the pattern Hilton et al.'s drift evaluation flags as worst. Re-anchor by *quoting* the root, not paraphrasing.

### 4.6 Self-assessed atomicity without verification

ADaPT's executor-self-assesses-success works because the environment provides a clean true/false signal (ALFWorld task succeeded?). For software, "I think this is atomic" is unreliable. Atomicity should be operationalized — *the EXECUTE phase produces a green test suite within budget*. If it doesn't, decompose. If it does, the leaf was atomic *enough*, regardless of what the planner thought.

### 4.7 Recursive self-improvement without external grounding

Theoretically proven (2026) to converge to a degenerate fixed point. Practically: never let a sub-tree improve itself with only its own outputs as signal; always inject a test, a benchmark, or a human review.

---

## 5. What's never been done at this scale

The user's brief named "build a POSIX-compatible OS that passes LTP" as an example. An honest reading of the literature:

1. **No published agent system has produced a coherent multi-100K-LOC system end-to-end.** MetaGPT, ChatDev, AgentCoder ship demos in the snake-game / CRUD-app range. SWE-agent, Devin, OpenHands handle GitHub *issues* (single-feature edits) on *existing* codebases. SWE-bench Pro and SWE-EVO push toward *long-horizon evolution* of existing systems — still not greenfield generation at OS scale.

2. **No published agent system has produced a kernel.** Adjacent work generates *one* kernel module: SchedCP (eBPF scheduler policy), KernelFalcon (one GPU kernel), the Linux driver-evolution paper (one driver patch). All operate against an existing kernel. None has produced a boot-to-shell OS.

3. **The integration story is unsolved at scale.** The literature stops at "structured artifacts" (MetaGPT) or "manager synthesis" (Chain of Agents); neither is tested on integrating dozens of mutually-dependent KLOC-scale modules. Bottom-up integration with a comprehensive test suite is the classical answer, but *constructing the test suite itself is part of the task* and the literature does not describe how to bootstrap it.

4. **The decomposition algorithm is unspecified at scale.** Every published system either (a) decomposes one level (HuggingGPT, MetaGPT, ChatDev) or (b) decomposes recursively at small depth (ADaPT, AgentOrchestra) or (c) explicitly avoids fanout (Cognition / Devin). No system in the literature reports operating recursively at depth ≥ 4 on a coding task at multi-KLOC scale.

5. **Halting / atomicity has no agreed definition for software.** ADaPT's "executor self-assesses" works in ALFWorld because the environment grades the attempt. Software analogues — "the test suite passes," "the code compiles and reviewer accepts," "≤ N LOC," "≤ T tokens spent" — exist but are not benchmarked against each other in any published study.

6. **Goal drift over deep recursion is unmeasured for engineering tasks.** Hilton et al. measured drift in conversational / instruction-following contexts up to 100K tokens. A POSIX-OS decomposition tree of depth ≥ 5 with ≥ 100 leaves easily exceeds this in cumulative context. The drift curve in this regime is not characterized in the literature.

7. **Sibling interference under deep recursion has no quantified study.** The Cognition position is "don't"; the Anthropic position is "only for reads." Neither addresses what happens when 50 leaf coding agents touch overlapping subsystems of a kernel concurrently, even with worktree isolation.

What this means for ArkOS positioning: the RFC honestly states that ArkOS is exploring a regime *no published system has demonstrated success in*. The closest cousins are AgentOrchestra (hierarchical multi-agent) and ADaPT (failure-driven recursion); the closest applicable engineering wisdom is Cognition's "share context, single-threaded writes" plus Anthropic's "embed scaling rules." None of these have been stretched to OS scale, and the existence of the gap is itself the most defensible claim the RFC can make about prior art.

---

## 6. Caveats / Not found

- **No quantitative study comparing decomposition algorithms head-to-head on a software-engineering benchmark.** Each system reports against a different benchmark (HumanEval, MBPP, SWE-bench, ALFWorld, GAIA). Cross-system comparisons are apples-to-oranges.
- **No published "leaf-size" benchmark.** Whether the right leaf is 50 LOC or 500 LOC is anecdotal across the field; nobody has run the experiment.
- **No agent-built kernel exists, even at xv6 scale (~10K LOC).** Searches for "LLM kernel from scratch" surface only AIOS (an OS *for* agents) and per-module work (drivers, schedulers, GPU kernels). A "build xv6 by recursive agent decomposition" project would itself be novel.
- **Cognition's blogs were not directly fetched (WebFetch denied at runtime).** Quoted summaries are from secondary sources (Jason Liu's writeup, Threads post by Sung Kim, Vellum's context-engineering article). The quote attribution to Walden Yan is third-hand; the RFC should cite Cognition's blog directly when authored. URLs verified: [Cognition — "Don't Build Multi-Agents"](https://cognition.ai/blog/dont-build-multi-agents), [Cognition — "Multi-Agents: What's Actually Working"](https://cognition.ai/blog/multi-agents-working).
- **Empirical research on the 8/80 rule, 1-day stories, etc., is folklore-grade.** The Standish CHAOS report does not list estimation method as a top success factor; Jørgensen disputes story-point efficacy. The RFC should not claim a leaf size has empirical support.
- **Goal-drift evaluation over multi-day agent sessions on software tasks is not in the literature reviewed.** Hilton et al. is the closest and uses conversational tasks.
- **Patch-to-PoC kernel work** (arXiv:2602.07287) was found but not deeply read; relevant if the RFC discusses *security-relevant* OS construction.
