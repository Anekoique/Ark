# Research: Self-improving and self-learning autonomous coding agents (prior art)

- Query: Survey prior art on self-improving / self-learning autonomous coding agents to inform RFC 001 (ArkOS positioning).
- Scope: external (web/papers) with light internal cross-reference (Trellis at `reference/Trellis`, ArkOS skeleton at `reference/arkos`).
- Date: 2026-05-10

The user wants grounded answers to feed a discussion *before* RFC content is drafted. This file is a survey, not a recommendation. ArkOS's actual design (recursion model, halting, self-SPECs) is explicitly out of scope for the RFC the user is authoring; the value of this survey is to inform what the *Open Questions* section should honestly admit it does not know.

## Bottom line up front

- **No system in the public record has demonstrated end-to-end recursive autonomy on a problem at the scale of "POSIX OS from one prompt + LTP."** The closest analogues — DGM, AlphaEvolve, AI-Scientist, Voyager — all narrow the search space drastically: a fixed benchmark suite, a single algorithm, a single skill domain, a single research idea. None recursively decomposes a system-shaped target.
- **Self-improvement only converges when the grounding signal is external and cheap to evaluate.** Tests passing, evolutionary fitness on a numeric objective, environment feedback (Minecraft inventory). When the signal is agent-generated (Reflexion's self-critique, Self-Refine), gains are small or model-dependent and frequently regress.
- **Recursion-and-halting is the *unsolved* part of the field, not a solved primitive ArkOS can borrow.** Almost every system in this survey either has a fixed depth ceiling, a hard wall-clock cap, an external operator, or runs evolution-with-budget. None has a principled "leaf task" definition.
- **Auto-generated specs / conventions actively hurt.** Empirical reports show LLM-generated AGENTS.md / constitution-style files reduce task success and increase token cost ([Augment Code, 2026](https://www.augmentcode.com/guides/how-to-build-agents-md)). The systems that *appear* to self-generate specs (Voyager skill descriptions, DGM modifications) attach those artifacts to executable + verifiable units, not to architectural style claims.
- **Cost and time scale brutally with autonomy depth.** A single DGM run on SWE-bench costs ~$22,000 and ~2 weeks ([sakana.ai/dgm](https://sakana.ai/dgm/)). Voyager runs cost $50 for ~160 iterations on GPT-4 ([Voyager FAQ](https://github.com/MineDojo/Voyager/blob/main/FAQ.md)). AI-Scientist runs cost $15+ per paper-attempt and ~42% of experiments fail with coding errors ([Beel et al. 2025](https://arxiv.org/abs/2502.14297)).

## Findings

### Systems table

| Name | Type | Year | Grounding signal | Recursion model | Halting condition | What it actually delivered | Source |
| ---- | ---- | ---- | ---------------- | --------------- | ----------------- | -------------------------- | ------ |
| **AutoGPT** | OSS | 2023 | None (LLM self-eval) | Open-loop ReAct, no real decomposition | Wall-clock / token budget / human stop | Did not deliver autonomous "AI employees"; pivoted to a visual workflow builder. Notorious for infinite loops. | [autogpt.net](https://autogpt.net/auto-gpt-understanding-its-constraints-and-limitations/), [vibeagentmaking.com retrospective](https://vibeagentmaking.com/blog/autogpt-got-100k-stars-and-then-what/) |
| **BabyAGI** | OSS | 2023 | None | Task-list rewrite loop (priority + exec) | Empty task list (rare in practice) | Archived Sept 2024; spawned BabyAGI 2 (functionz). Demo-quality, never production. | [IBM Think](https://www.ibm.com/think/topics/babyagi), [smythos comparison](https://smythos.com/developers/agent-comparisons/autogpt-vs-babyagi/) |
| **AgentGPT** | Product | 2023 | None | Same loop family as BabyAGI | Iteration cap | Browser UI for the same pattern; same limits. | [bairesdev rise-of-agents](https://www.bairesdev.com/blog/the-rise-of-autonomous-agents-autogpt-agentgpt-and-babyagi/) |
| **Voyager** (Wang et al., NVIDIA/Caltech) | Research | 2023 | Environment feedback (Minecraft inventory + execution errors) + self-verification | Automatic curriculum + skill library; *not* recursive task decomposition — flat skill accretion | Open-ended; no halting (curriculum proposes next task) | 3.3× more unique items than prior SOTA; 15.3× faster tech-tree milestones. Skill library never *updates* existing skills (paper limitation). | [arXiv:2305.16291](https://arxiv.org/abs/2305.16291), [voyager.minedojo.org](https://voyager.minedojo.org/), [github.com/MineDojo/Voyager](https://github.com/MineDojo/Voyager) |
| **Reflexion** (Shinn et al., NeurIPS 2023) | Research | 2023 | External (env reward, test pass/fail) + verbal self-reflection appended to next-trial prompt | Per-trial loop, not hierarchical | Trial budget or success | 91% pass@1 HumanEval (vs GPT-4 80%). Only works because there *is* an external evaluator. | [arXiv:2303.11366](https://arxiv.org/abs/2303.11366), [github.com/noahshinn/reflexion](https://github.com/noahshinn/reflexion) |
| **Self-Refine** (Madaan et al., NeurIPS 2023) | Research | 2023 | LLM self-critique (no external) | Single-level refine loop | Self-stop criterion or iter cap | Acknowledged limitation: "needs sufficient few-shot/instruction-following" — only works on strong models. | [arXiv:2303.17651](https://arxiv.org/abs/2303.17651), [selfrefine.info](https://selfrefine.info/) |
| **Generative Agents** (Park et al., UIST 2023) | Research | 2023 | Human-evaluator believability (not a code grounding signal) | Memory-stream + reflection (abstraction only, not decomposition) | Simulation tick budget | 25 NPCs in Smallville exhibited believable behavior; relevant for *memory architecture* not coding self-improvement. | [Park 2023 PDF](https://3dvar.com/Park2023Generative.pdf), [github.com/joonspk-research/generative_agents](https://github.com/joonspk-research/generative_agents) |
| **FunSearch** (DeepMind, Nature 2023) | Research | 2023 | Evolutionary fitness on a fixed numeric evaluator | Evolutionary search (genetic programming over LLM-proposed code) | Wall-clock / generation budget | New cap-set bound; better bin-packing heuristic. **Single function, not a system.** | [Nature s41586-023-06924-6](https://www.nature.com/articles/s41586-023-06924-6), [github.com/google-deepmind/funsearch](https://github.com/google-deepmind/funsearch) |
| **AlphaEvolve** (DeepMind, 2025) | Research | 2025 | Evaluator score (one or more user-supplied evaluators) | Evolutionary, ensemble of Gemini Flash + Pro | Budget / no-improvement convergence | 4×4 complex matmul in 48 mults (improving Strassen 56yr SOTA on that instance); rediscovered SOTA on 75% of 50 problems, improved 20%. **Closed-source.** Evaluator must be human-written. | [arXiv:2506.13131](https://arxiv.org/abs/2506.13131), [DeepMind blog](https://deepmind.google/blog/alphaevolve-a-gemini-powered-coding-agent-for-designing-advanced-algorithms/), [InfoQ](https://www.infoq.com/news/2025/05/google-alpha-evolve/) |
| **OpenEvolve / CodeEvolve** | OSS | 2025 | Same as AlphaEvolve (evaluator-driven) | Island-based GA with LLM mutations | Generation budget | Reproduced AlphaEvolve circle-packing within 0.04%; CodeEvolve claims to outperform on 4 problems. Confirms approach is the GA + evaluator, not the LLM weights. | [HuggingFace OpenEvolve](https://huggingface.co/blog/codelion/openevolve), [arXiv:2510.14150](https://arxiv.org/abs/2510.14150) |
| **SWE-agent** (Princeton, NeurIPS 2024) | Research | 2024 | Test pass on SWE-bench instances | Single-agent ReAct over an Agent-Computer Interface (ACI); not recursive | Test pass / step budget | 12.5% pass@1 SWE-bench at publication; surpassed since. *No self-improvement* — fixed agent design. | [arXiv:2405.15793](https://arxiv.org/abs/2405.15793), [github.com/SWE-agent/SWE-agent](https://github.com/SWE-agent/SWE-agent) |
| **Devin** (Cognition) | Product | 2024 | Tests / human review | Internal proprietary | Wall-clock / human stop | 13.86% SWE-bench at announcement. Independent reviewers ([Answer.AI](https://futurism.com/first-ai-software-engineer-devin-bungling-tasks)) found 3/20 (15%) success on real Upwork-style tasks; Carl Brown (Internet of Bugs) showed cherry-picked demo. | [Cognition tech report](https://cognition.ai/blog/swe-bench-technical-report), [Futurism summary](https://futurism.com/first-ai-software-engineer-devin-bungling-tasks), [Wikipedia](https://en.wikipedia.org/wiki/Devin_AI) |
| **OpenHands** (formerly OpenDevin, ICLR 2025) | OSS | 2024 | Tests + benchmark fitness | Multi-agent platform; no built-in recursion guarantee | Per-task budget | ~77% SWE-bench Verified with Claude Sonnet 4.5; 188+ contributors. Platform, not an autonomy claim. | [arXiv:2407.16741](https://arxiv.org/abs/2407.16741), [openhands.dev](https://www.openhands.dev/) |
| **AI Scientist** (Sakana, 2024) | Research | 2024 | Internal LLM reviewer + experiment exec results | Idea-generate → exec → write loop, single layer | Budget per paper | Generated papers; v2 generalized beyond templates. **Independent eval: 42% of experiments fail with coding errors; literature reviews misclassify established work as novel.** | [Sakana AI Scientist](https://sakana.ai/ai-scientist/), [v2 PDF](https://pub.sakana.ai/ai-scientist-v2/paper/paper.pdf), [arXiv:2502.14297 critique](https://arxiv.org/abs/2502.14297) |
| **ADAS** (Hu et al., 2025) | Research | 2025 | Benchmark score | Fixed meta-agent generates downstream agents | Generation budget | Showed automated agent design; meta-agent itself never improves. | Cited in DGM; see [DGM paper §2](https://arxiv.org/abs/2505.22954) |
| **Darwin Gödel Machine (DGM)** (Sakana/UBC, 2025) | Research | 2025 | SWE-bench / Polyglot test pass | Self-modifying meta-agent; archive of variants; evolutionary | Budget; archive selection | SWE-bench 20%→50%; Polyglot 14.2%→30.7%. Discovered better edit tools, context management, peer-review patterns. **Cost ~$22k / 2 weeks per run.** Humans still pick the benchmark. | [arXiv:2505.22954](https://arxiv.org/abs/2505.22954), [sakana.ai/dgm](https://sakana.ai/dgm/), [github.com/jennyzzt/dgm](https://github.com/jennyzzt/dgm) |
| **Huxley-Gödel Machine** (2025) | Research | 2025 | Coding benchmark | DGM-style w/ approximation | Budget | Claims human-level coding agent development. New, not yet widely reproduced. | [arXiv:2510.21614](https://arxiv.org/pdf/2510.21614) |
| **MemGPT / Letta** | OSS / Product | 2023–2025 | N/A (memory framework, not autonomy) | OS-style memory tiers (core/recall/archival) | N/A | Production-ready stateful memory for agents. Solves *one* of the substrates ArkOS would need. | [github.com/letta-ai/letta](https://github.com/letta-ai/letta) |
| **ReDel** (Penn) | OSS | 2024 | Task-defined | Zero-shot LLM-tool-call recursive delegation | Depth-cap or task done signal | Recursive multi-agent toolkit. Demonstrates *mechanism*, not autonomy at scale. | [Penn paper PDF](https://www.cis.upenn.edu/~ccb/publications/recursive-multi-agent-llms.pdf) |
| **ADAPT** (NAACL 2024) | Research | 2024 | Task-success | As-needed decomposition: only decompose when leaf-execution fails | "Atomic if executor succeeds" | 28–33% higher success on agent-bench tasks. Closest existing analogue to a *principled* leaf-task definition. | [Medium review](https://medium.com/correll-lab/how-recursive-decomposition-boosts-autonomous-agent-success-f2954ccba5cf), [NAACL 2024] |
| **Trellis** (Mindfold, OSS, AGPL) | Product / OSS | 2025 | Human-in-the-loop | Spec / task / workspace / workflow layers; explicitly NOT autonomous | Human gates per phase | Local: `reference/Trellis/`. Sibling-class harness to Ark; same human-gated philosophy. Differs in adapter-per-platform. | `reference/Trellis/README.md`, [docs.trytrellis.app](https://docs.trytrellis.app/) |
| **GitHub Spec-Kit / Kiro** | Tool / Product | 2024–2025 | Human review of generated spec | spec → plan → tasks → impl, single-pass | Human stop per phase | Established "constitution.md" pattern: high-level immutable principles. **Constitution is human-written, not auto-generated.** | [Spec-Kit](https://github.com/github/spec-kit), [Kiro](https://kiro.dev/), [Martin Fowler SDD-3](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html) |

### Recursion-and-halting patterns (synthesis)

Three families exist; only families 2 and 3 actually halt principally.

1. **Open-loop iteration with budget cap.** AutoGPT, BabyAGI, AgentGPT, Self-Refine, Reflexion (per trial). Halts on iter / token / wall-clock. *Not principled* — the agent has no concept of "done."
2. **Evolutionary search over a numeric fitness.** FunSearch, AlphaEvolve, OpenEvolve, DGM. Halts on no-improvement detection or population convergence. **Requires a cheap, deterministic evaluator.** This is the only family with empirical large-scale wins.
3. **As-needed decomposition with leaf-execution test.** ADAPT (and ReDel mechanism). Decompose only when the executor fails on a candidate leaf. **Promising but unproven at OS-scale**; ADAPT's eval is on agent-bench tasks, not multi-million-line systems.

The "POSIX OS + LTP" target the user described maps onto family 3 with a family-2 inner loop (run LTP as the fitness function inside each leaf). No public system has demonstrated this composition at that scale.

### Grounding-signal hierarchy

From most to least grounded (and most to least likely to converge):

1. **Deterministic test on the actual artifact.** Strassen-style algorithm verifier, LTP test, SWE-bench unit tests, compiler exit code. AlphaEvolve, FunSearch, DGM, SWE-agent live here.
2. **Environment-state assertion.** Voyager (inventory diff), embodied agents, browser-state checks. Cheaper than tests but weaker — the agent can game the assertion.
3. **Reviewer LLM with rubric.** AI Scientist's reviewer, "LLM-as-judge." [Independent eval](https://arxiv.org/abs/2502.14297) found AI Scientist's reviewer misclassified known concepts as novel. **Not a fixed point.**
4. **Self-critique only (no external).** Self-Refine, much of AutoGPT. Drift is unbounded; gains depend entirely on the base model already knowing the right answer.

For an "OS from a test suite" target, the grounding is genuinely tier-1 — you have LTP. The harder grounding question is the *architecture-shape* claims (style guide, layout SPECs, naming) that emerge below the test boundary. Those are tier-3 or tier-4 unless ArkOS reduces them to executable checks (linters, structural tests, bench performance).

### Self-generating specs / contracts — what the literature shows

Two distinct claims often get conflated:

- **Auto-generated *executable* artifacts** (Voyager skill code, DGM agent diffs, AlphaEvolve algorithms). These work because they bottom out in a test or fitness function. The "spec" *is* code with a verifier.
- **Auto-generated *style / convention* artifacts** (AGENTS.md, CLAUDE.md, constitution.md, project rules). These have *empirically failed* in independent measurement. From [Augment Code's 2026 guide](https://www.augmentcode.com/guides/how-to-build-agents-md):
  - LLM-generated AGENTS.md files reduce task success rates by 0.5–2% and increase inference cost by >20%.
  - "Rules should respond to observed failure, not be generated speculatively."
  - "Each time an agent makes a mistake, the default reaction is to add another rule. Rules are rarely removed, and the file accumulates contradictory patches."

GitHub Spec-Kit's `constitution.md` and Kiro's spec-driven flow keep the constitution **human-written and immutable**; the agent generates the spec/plan/tasks layers underneath. Sibling project Trellis follows the same pattern: project specs are human-curated; tasks and workspace are agent-touched but human-gated.

The honest read: no public system has demonstrated successful auto-generation of *style/architectural* SPECs without external grounding. Auto-generation works only when each generated rule is paired with an executable check that fails when violated.

### Operational realities (cost / time / context)

- **DGM**: ~$22,000 and ~2 weeks per run on SWE-bench ([sakana.ai/dgm](https://sakana.ai/dgm/)).
- **Voyager**: ~$50 / 160 iterations on GPT-4 ([FAQ](https://github.com/MineDojo/Voyager/blob/main/FAQ.md)). Modest because the action space is narrow.
- **AI-Scientist**: 42% of experiments fail with coding errors per independent eval ([arXiv:2502.14297](https://arxiv.org/abs/2502.14297)).
- **AutoGPT loops**: documented case of $400 in tokens burned re-checking the same email 50× ([techtalkwithsriks](https://techtalkwithsriks.medium.com/notorious-agent-loops-c4cc05b859b5)).
- **METR time horizon**: as of 2025, frontier agent 50% time horizon doubles every 4 months (down from 7 months 2019–2025) ([metr.org](https://metr.org/blog/2025-03-19-measuring-ai-ability-to-complete-long-tasks/)). 8h+ tasks are still an active frontier — and "POSIX OS" is many orders of magnitude beyond 8h.
- **Context-window management**: the 2025 consensus is anchored iterative summarization + compaction APIs (Anthropic compact-2026-01-12, OpenAI compaction guide, Google ADK Context Compaction). ACON reports 26–54% peak token reduction. **None of these solve the *recursive* context problem** — when a sub-agent spawns sub-agents, each carries its own compacted context, and reconciling diverged compactions is unsolved.

### Claim-vs-reality patterns (anti-patterns)

These recur in marketing and rarely survive contact with independent eval:

1. **"Autonomous from one prompt."** Devin's launch demo was cherry-picked; Answer.AI's 20-task replication scored 15% ([Futurism](https://futurism.com/first-ai-software-engineer-devin-bungling-tasks)). Watch for selection bias in published demos.
2. **"Self-improving" with LLM-judge fitness.** AI Scientist's internal reviewer overrates its own outputs; the same is true of any closed loop where the evaluator is the same family of model that produces the work. DGM's success specifically *avoids* this by using SWE-bench tests.
3. **"Skill library that grows forever."** Voyager's library never updates existing skills (paper limitation). In long runs, this becomes a write-only log — retrieval relevance degrades.
4. **"Recursive task decomposition" without halting principle.** Most "hierarchical agent" papers either fix depth (depth=2 or 3) or budget by tokens. The decomposition decision itself is rarely grounded; ADAPT is the only public exception with a principled criterion ("decompose iff executor fails") and even ADAPT is benchmarked on agent-bench, not on system-scale.
5. **"Self-generating SPECs."** Distinct from auto-generated *code with tests*. Style/architecture SPECs auto-generated by agents have measurably worsened task success ([Augment Code, 2026](https://www.augmentcode.com/guides/how-to-build-agents-md)).
6. **SWE-bench numbers as evidence of capability.** [Aleithan et al.](https://arxiv.org/html/2506.12286v3) and [follow-up](https://arxiv.org/pdf/2512.10218) show 33% of "resolved" instances had solution leakage; 94% of issues predate model training cutoffs. SWE-Agent+GPT-4's revised strict-validity rate was 3.97% (vs reported 12.47%). [OpenAI dropped SWE-bench Verified](https://openai.com/index/why-we-no-longer-evaluate-swe-bench-verified/) as a frontier metric for similar reasons.

### Patterns worth borrowing

- **Tier-1 grounding via tests.** AlphaEvolve / DGM / FunSearch all win because the fitness function is cheap, deterministic, and external. ArkOS's "LTP for POSIX" is structurally the same shape — this is the strongest single thing the user's framing has going for it.
- **Archive of variants + selection.** DGM and FunSearch keep all explored variants and select from them, rather than greedy iteration. This is the only documented mechanism that survives long runs without collapsing onto a local optimum.
- **As-needed decomposition.** ADAPT's "decompose iff the executor fails the leaf" is the cleanest published halting criterion for recursive decomposition. Even if ArkOS doesn't adopt it directly, it's the right shape to defend against.
- **Memory tiering (CoALA / MemGPT / Letta).** Working / episodic / semantic / procedural separation. ArkOS's task tree is naturally a procedural-memory analogue.
- **Human-curated constitution + agent-generated derivative SPECs.** Spec-Kit / Kiro / Trellis / Ark all converge on this. The user's existing `specs/project/` (human) + `specs/features/` (promoted on commit) layout is *already* in this family.
- **Compaction at every recursion boundary.** Anthropic / OpenAI / Google all ship compaction APIs in 2025–2026. Recursion makes this load-bearing, not optional.

### Warning signs

- **Any system that claims to halt without an external evaluator.** Self-RAG / Self-Refine / Reflexion-without-environment all need either an external signal or a strong base model that already knows.
- **Evaluator-as-LLM at the top of the loop.** AI Scientist's internal reviewer is the canonical cautionary tale.
- **Budget-only halting.** Hides divergence; the agent looks like it stopped on purpose when actually it timed out.
- **Skill / spec libraries without invalidation.** Voyager's append-only library; auto-generated rules files that never delete. Both rot.
- **Self-modifying meta-agent without sandbox / rollback.** DGM's archive-and-select is the safe shape; an in-place self-rewrite without provenance is the dangerous shape.

## Open questions ArkOS would have to figure out from scratch

These are *not* answered by the public literature; ArkOS would be doing first-of-its-kind work:

1. **What is a leaf task at OS scale?** ADAPT's "decompose iff executor fails" works when the executor is a single LLM call on a small sub-task. At OS scale, "executor" is itself a multi-day run; the failure feedback loop is too slow for ADAPT's mechanism. Open: how do you cheaply predict leaf-fitness *before* running the leaf?
2. **How do recursive agents share / reconcile compacted context?** Each sub-agent compacts independently; when the parent reads back, it gets a lossy view. None of the 2025–2026 compaction APIs (Anthropic, OpenAI, ADK) describe a *recursive* discipline.
3. **How does a self-generating SPEC layer avoid the rot pattern?** Empirical evidence (Augment Code) is that auto-generated rule files actively harm. The only escape route in the literature is to bind every generated rule to an executable check — but at OS scale, many architectural rules cannot be cheaply checked.
4. **What grounds *intermediate* artifacts?** LTP grounds the leaves; LTP does not ground "did we put the scheduler in the right module?" Below the test boundary the grounding signal vanishes. DGM and AlphaEvolve avoid this by having no architecture — just code that passes the test. ArkOS implicitly assumes architecture *also* needs grounding; nobody public has solved this.
5. **What's the budget/horizon model when each leaf is itself a multi-hour agent run?** METR's time horizons are doubling every 4 months but still in single-digit hours. POSIX-OS-shaped tasks are 6+ orders of magnitude beyond this. Either ArkOS waits 5–10 doubling periods (~2–4 years) for the leaf-time horizon to catch up, or it has a non-trivial story for why scale-via-recursion sidesteps the horizon limit.
6. **How does an autonomous system avoid the AutoGPT failure mode of repeating the same action 50×?** Memory-of-failure is necessary but not sufficient (Voyager has it, still rebuilds skills). Open whether episodic memory + reflection is enough, or whether a structural anti-pattern detector is needed.
7. **What's the cost model?** DGM is $22k / run on a benchmark task; an OS-scale run extrapolates to numbers that no public system has demonstrated economic viability for. ArkOS needs an honest internal cost model before it claims feasibility.
8. **Stage-1 dependency: what does ArkOS calling `ark agent` recursively look like operationally?** If each sub-agent spawns Claude/Codex/OpenCode with its own context, the per-leaf cost compounds. Ark's existing recursion guard (the agent files explicitly forbid researcher / reviewer / verifier from spawning siblings) is *exactly* the discipline missing in the open-loop autonomous systems above. Whether ArkOS preserves or relaxes this is a load-bearing design call.

## Caveats / Not found

- Could not find an independent reproduction of DGM's 50% SWE-bench result outside the original lab. Cited number is the authors' claim. The cost figure ($22k / 2 weeks) comes from secondary writeups summarizing the paper (Medium piece, [richardcsuwandi](https://richardcsuwandi.github.io/blog/2025/dgm/)) — verify before quoting in the RFC if cost is load-bearing.
- AlphaEvolve is closed-source; the open-source reproductions (OpenEvolve, CodeEvolve) match the *technique* but not necessarily the published numerical results on the closed problems. The 4×4 matmul claim is in the white paper PDF; independent verification by mathematicians is ongoing as of search date.
- "Computer use" extended-autonomy benchmarks from Anthropic are not publicly disclosed at fine granularity; claims of "hours of coherent work" for Claude Opus 4.7 come from Anthropic's own announcement. No independent METR-style time-horizon paper specifically on Opus 4.7 was located.
- Did not survey closed-corporate systems (Cursor's background agents, Replit Agent, Cognition's post-Devin work) beyond public statements. Their internal recursion / halting designs are unpublished.
- The user mentioned "Anthropic's 'computer use' + extended autonomy patterns" — patterns themselves are not formally published; the closest source is [Anthropic's effective-context-engineering guide](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents), which is prescriptive not empirical.
- Local `reference/arkos/` is currently a near-empty skeleton (README is one line: `# ArkOS`). No existing ArkOS design content to cross-reference. The existing `reference/Trellis/` is a *sibling* human-in-the-loop harness — same philosophical class as Ark, not an autonomy precedent.
- Did not find any public system that has attempted "self-generating SPECs for architecture/style" *as a primary mechanism*. Closest analogues (Voyager's skill descriptions, DGM's modification logs) are post-hoc explanations of executable artifacts, not architectural conventions. This is genuinely a gap in prior art — neither validated nor refuted at scale.
