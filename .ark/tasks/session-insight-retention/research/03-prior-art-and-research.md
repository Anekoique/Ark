# Research: Prior art and research for session-to-insight and session-to-skill retention

- Query: Compare Hermes Agent, Oh My Pi, claude-mem, Devin, and relevant research on automatically extracting durable insights or procedural skills from agent sessions and implementation trajectories.
- Scope: mixed
- Date: 2026-07-29

## Findings

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `reference/oh-my-pi/docs/memory.md` | User-facing contract for Oh My Pi's autonomous, project-scoped, two-phase memory pipeline. |
| `reference/oh-my-pi/packages/coding-agent/src/prompts/memories/stage-one-system.md` | Per-session extraction prompt and its explicit no-signal result. |
| `reference/oh-my-pi/packages/coding-agent/src/prompts/memories/consolidation.md` | Cross-session schema that emits long-term memory, prompt summary, and skill packages. |
| `reference/oh-my-pi/packages/coding-agent/src/prompts/memories/read-path.md` | Read-time policy that treats retained knowledge as advisory and requires current repository evidence. |
| `reference/oh-my-pi/packages/coding-agent/src/memories/index.ts` | Message filtering, model calls, artifact synchronization, pruning, secret redaction, and path sanitation. |
| `reference/oh-my-pi/packages/coding-agent/src/memories/storage.ts` | SQLite job queue, per-project isolation, leases, retries, source watermarks, and idempotent stage-one updates. |
| `reference/oh-my-pi/packages/coding-agent/test/memories-runtime.test.ts` | End-to-end coverage for generation, injection, pruning, and empty-corpus cleanup. |
| `reference/claude-mem/README.md` | Product-level description of lifecycle hooks, observations, summaries, SQLite/Chroma storage, and progressive retrieval. |
| `reference/claude-mem/plugin/hooks/hooks.json` | Claude Code hook wiring for session start, prompt submit, tool observation, file context, and stop-time summary. |
| `reference/claude-mem/src/sdk/prompts.ts` | Structured observation and stop-summary prompts used by the observer agent. |
| `reference/claude-mem/tests/services/worker/session-message-buffer.test.ts` | FIFO buffering and tool-use-ID deduplication behavior. |
| `reference/claude-mem/tests/compat/sessions-observations-adapter.test.ts` | Durable event/outbox creation plus idempotent session and summary behavior. |

The inspected Oh My Pi snapshot was commit `304a9346e924764f460931da1d23ab42c25209f5`; the inspected claude-mem snapshot was `84636894740724cb424e993d2e37a5a06a2aff2e`. These hashes matter because both projects are moving quickly and their current behavior is not safely inferred from older descriptions.

### Direct conclusion: extraction and publication have different evidence thresholds

Automatic **candidate extraction** is well supported. Hermes reviews completed turns in the background, Oh My Pi emits a per-session candidate or an explicit no-signal result, claude-mem continuously captures typed evidence, and Devin classifies completed sessions. The common property is that these outputs can remain source-linked, inspectable, and non-authoritative.

Automatic **ungated publication as an active procedural skill** has materially weaker support. Hermes can write freely by default but also supplies optional approval, diff visibility, pinning, and recoverable lifecycle; its docs do not describe a mandatory held-out execution gate. Oh My Pi can regenerate active skill packages from consolidation output, but the inspected code validates structure rather than transfer effectiveness. Devin keeps promotion to a Playbook human-triggered and recommends testing it with additional sessions. Research systems with the strongest reported transfer evidence add outcome gates or cross-trajectory evidence: Voyager self-verifies execution, online AWM uses a task evaluator, ExpeL aggregates multiple successes and failures, and CODESKILL learns from downstream execution reward.

The evidence therefore supports treating "this session contains a reusable lesson" as a lower-risk automatic judgment than "this lesson should immediately constrain future tasks as an active skill." The sources show several publication gates—human approval, repeated support, task verification, held-out reuse, and versioned pending state—but do not establish one universally superior gate.

### Code patterns

#### Oh My Pi: derive per-session evidence, then consolidate across sessions

The first model pass is explicitly permitted to produce no artifact. That is a useful distinction between an extraction opportunity and a requirement to save something:

> `reference/oh-my-pi/packages/coding-agent/src/prompts/memories/stage-one-system.md:5-8`
>
> ```text
> Extraction goals:
> - You MUST distill reusable durable knowledge from rollout history.
> - You MUST keep concrete technical signal (constraints, decisions, workflows, pitfalls, resolved failures).
> - You NEVER include transient chatter and low-signal noise.
> ```

> `reference/oh-my-pi/packages/coding-agent/src/prompts/memories/stage-one-system.md:17-21`
>
> ```text
> Rules:
> - rollout_summary: compact synopsis of what future runs should remember.
> - rollout_slug: short lowercase slug (letters/numbers/_), or null.
> - raw_memory: detailed durable memory blocks with enough context to reuse.
> - If no durable signal exists, you MUST return empty strings for rollout_summary/raw_memory and null rollout_slug.
> ```

The second pass has a different responsibility: it can emit both declarative memory and procedural skill packages, including supporting files:

> `reference/oh-my-pi/packages/coding-agent/src/prompts/memories/consolidation.md:7-29`
>
> ```text
> Produce strict JSON only with this schema — you NEVER include any other output:
> {
>   "memory_md": "string",
>   "memory_summary": "string",
>   "skills": [
>     {
>       "name": "string",
>       "content": "string",
>       "scripts": [{ "path": "string", "content": "string" }],
>       "templates": [{ "path": "string", "content": "string" }],
>       "examples": [{ "path": "string", "content": "string" }]
>     }
>   ]
> }
> ...
> - Only include files worth keeping long-term. Omit stale assets so they are pruned.
> - Preserve useful prior themes. Remove stale or contradictory guidance.
> ```

The implementation preserves a source-level intermediate corpus rather than only the final synthesis. Each rollout summary carries its thread ID and source update time:

> `reference/oh-my-pi/packages/coding-agent/src/memories/index.ts:656-679`
>
> ```ts
> const summariesDir = path.join(memoryRoot, "rollout_summaries");
> ...
> const body = [`thread_id: ${row.threadId}`, `updated_at: ${row.sourceUpdatedAt}`, "", row.rolloutSummary].join(
>     "\n",
> );
> ...
> const rawBody = buildRawMemoriesMarkdown(outputs);
> await Bun.write(path.join(memoryRoot, "raw_memories.md"), rawBody);
> ```

Model input is deliberately filtered. Conversational messages survive, but only selected, bounded tool results are retained:

> `reference/oh-my-pi/packages/coding-agent/src/memories/index.ts:542-553`
>
> ```ts
> if (role === "system" || role === "developer" || role === "user" || role === "assistant") {
>     return true;
> }
> if (role !== "toolResult") return false;
> const toolName = (message as { toolName?: string }).toolName;
> if (toolName === "bash" || toolName === "eval" || toolName === "read" || toolName === "search") {
>     const text = extractMessageText(message);
>     return text.length > 0 && text.length <= 32_000;
> }
> ```

The probabilistic model pass is surrounded by deterministic controls. Stage-one work is isolated by project, leased, watermarked, and updated only when the incoming source is at least as new:

> `reference/oh-my-pi/packages/coding-agent/src/memories/storage.ts:39-45`
>
> ```ts
> /**
>  * Per-project job key so Phase 2 consolidation is isolated to a single cwd.
>  * Previously a single "global" key caused cross-project memory contamination.
>  */
> function globalJobKey(cwd: string): string {
>     return `global:${cwd}`;
> }
> ```

> `reference/oh-my-pi/packages/coding-agent/src/memories/storage.ts:327-339`
>
> ```sql
> INSERT INTO stage1_outputs (thread_id, source_updated_at, raw_memory, rollout_summary, rollout_slug, generated_at)
> VALUES (?, ?, ?, ?, ?, ?)
> ON CONFLICT(thread_id) DO UPDATE SET
>     source_updated_at = excluded.source_updated_at,
>     raw_memory = excluded.raw_memory,
>     rollout_summary = excluded.rollout_summary,
>     rollout_slug = excluded.rollout_slug,
>     generated_at = excluded.generated_at
> WHERE excluded.source_updated_at >= stage1_outputs.source_updated_at
> ```

The read path also encodes a strong epistemic boundary:

> `reference/oh-my-pi/packages/coding-agent/src/prompts/memories/read-path.md:4-9`
>
> ```text
> 1) Read `memory://root/memory_summary.md` first.
> 2) If needed, inspect `memory://root/MEMORY.md` and `memory://root/skills/<name>/SKILL.md`.
> 3) Trust memory for heuristics and process context. Trust current repo files, runtime output, and user instruction for factual state and final decisions.
> ...
> 6) Escalate confidence only after repository verification. Memory alone is NEVER sufficient proof.
> ```

What the code validates is mostly structural: exact JSON schemas, sanitized names and relative paths, a small regex secret-redaction pass, and deterministic pruning of omitted artifacts (`index.ts:742-850,975-1044`). I did not find an execution test that runs each generated skill against a held-out task before it becomes available.

#### claude-mem: capture observations continuously, summarize at a lifecycle boundary

claude-mem is an observation memory rather than a procedural skill synthesizer. Its observer emits typed XML records with facts, narrative, concepts, and files:

> `reference/claude-mem/src/sdk/prompts.ts:42-73`
>
> ```ts
> <observation>
>   <type>[ ${mode.observation_types.map(t => t.id).join(' | ')} ]</type>
>   <title>${mode.prompts.xml_title_placeholder}</title>
>   <subtitle>${mode.prompts.xml_subtitle_placeholder}</subtitle>
>   <facts>
>     <fact>${mode.prompts.xml_fact_placeholder}</fact>
>     ...
>   </facts>
>   <narrative>${mode.prompts.xml_narrative_placeholder}</narrative>
>   <concepts>...</concepts>
>   <files_read>...</files_read>
>   <files_modified>...</files_modified>
> </observation>
> ```

Each tool-use observation may be skipped, while concrete debugging evidence is explicitly named as durable:

> `reference/claude-mem/src/sdk/prompts.ts:103-112`
>
> ```ts
> Return either one or more <observation>...</observation> blocks, or an empty response if this tool use should be skipped.
> Concrete debugging findings from logs, queue state, database rows, session routing, or code-path inspection count as durable discoveries and should be recorded.
> ```

At stop time, the artifact changes shape from a tool observation to a session checkpoint:

> `reference/claude-mem/src/sdk/prompts.ts:123-146`
>
> ```ts
> <summary>
>   <request>...</request>
>   <investigated>...</investigated>
>   <learned>...</learned>
>   <completed>...</completed>
>   <next_steps>...</next_steps>
>   <notes>...</notes>
> </summary>
> ```

The stored corpus is retrieved progressively instead of being injected wholesale: compact search results first, a timeline around selected results second, and full observations only for chosen IDs (`reference/claude-mem/README.md:234-266`). The compatibility tests verify that repeated observations reuse the same server session and repeated summarization reuses the same generation job (`tests/compat/sessions-observations-adapter.test.ts:240-264,323-338`).

I searched the snapshot for an automatic path that transforms observations or summaries into newly authored `SKILL.md` procedures. None was found. The repository ships a `mem-search` skill for querying its memory, but that is a retrieval interface, not evidence that claude-mem promotes learned procedures into generated skills.

### Mechanism comparison

The table treats a "skill" narrowly as reusable procedural guidance with invocation conditions or executable steps. Session summaries, facts, and searchable observations are recorded separately even when products call all of them "memory."

| System | Extraction trigger and evidence | Durable artifact | Retrieval / injection | Update, deduplication, and lifecycle | Validation or reported evaluation |
| ------ | ------------------------------- | ---------------- | --------------------- | ------------------------------------ | --------------------------------- |
| **Hermes Agent** | Foreground agent writes plus an after-turn background self-improvement review. The review replays the conversation (or a compact digest on an auxiliary model) and looks for repeated corrections or durable workflow lessons. Turn-count nudges can initiate memory/skill review. `/learn` is a separate user-triggered path that can source the current conversation, documents, URLs, or a repository. | Bounded factual `MEMORY.md` and user-profile `USER.md`; procedural `SKILL.md` packages with optional references, templates, scripts, and assets; complete sessions remain searchable in SQLite/FTS5. | Memory is a frozen system-prompt snapshot at session start. Skill metadata is listed compactly, with full skill content and support files loaded on demand. Session search is on demand and returns stored messages. | Memory has exact-duplicate rejection, substring replace/remove, and hard capacity limits that force explicit consolidation. `skill_manage` can create/edit/patch/delete. Optional write approval stages a diff before memory or skill changes land. The Curator records use state, deterministically moves skills `active → stale → archived`, never auto-deletes, supports pinning, and optionally performs LLM consolidation. | Skill format asks authors for a Verification section, but current product docs do not describe a mandatory held-out execution gate for every autonomously created or patched skill. User-visible notifications and optional approval validate the write, not transfer effectiveness. |
| **Oh My Pi autonomous memory** | At startup, changed sessions that are idle but not too old are claimed by a leased queue. Stage 1 extracts per-session durable signal; stage 2 periodically consolidates all current outputs. | Per-rollout raw memory and synopsis with thread provenance; consolidated `MEMORY.md`; compact startup summary; generated skill packages containing `SKILL.md`, scripts, templates, and examples. | Compact summary is injected at startup. Full memory and individual skills are read progressively via `memory://` only when needed. | Source timestamps and job watermarks prevent stale overwrites; per-project global keys prevent cross-project contamination; retries and leases handle concurrency. Consolidation fully regenerates the derived view and prunes omitted stale skills/files. | End-to-end tests cover generation, injection, stale pruning, and empty cleanup. Schema, path, and limited secret checks exist. No held-out task-success gate for generated skills was found. |
| **claude-mem** | Lifecycle hooks capture most tool uses as observations, initialize/continue a session at user prompts, and request a semantic summary at Stop. | Structured observations, session summaries, event/outbox records in SQLite, plus Chroma/FTS indexes. It does not automatically author procedural skills in the inspected snapshot. | Three-layer progressive disclosure: compact search index → surrounding timeline → selected full observations. | Tool-use IDs and content-session IDs provide transport/session idempotency; repeated summarize calls reuse jobs. Retrieval indexes are additive. No cross-observation procedural reconciliation was found. | Extensive storage/queue/integration tests. Product claims token savings for staged retrieval, but no benchmark demonstrating that retained observations improve future task completion was found. |
| **Devin Session Insights + Playbooks/Skills** | A lightweight classification is generated automatically when a session ends. Full analysis is automatic for larger sessions and manually/API triggered for smaller ones. Users inspect failure timelines, improved prompts, action items, and knowledge use. Promotion to a Playbook is a separate human-triggered generalization workflow; Devin may suggest new skills after learning something. | Session insight report; manually saved reusable Playbooks; repository or user Skills. | Playbooks are explicitly attached or invoked; Skills can be discovered and loaded based on their descriptions. | Playbooks have manual editing and version history/revert. Official guidance recommends testing a Playbook with at least two Devin runs and iterating. The docs do not expose an automatic transcript-to-skill merge/deduplication algorithm. | Session Insights evaluates one run; Playbook reuse is validated by additional sessions. No controlled public benchmark for automatically suggested skills was found. |
| **Reflexion (NeurIPS 2023)** | After a failed or scored trial, the agent verbalizes task feedback into a reflection. | Short text reflections in an episodic buffer for subsequent attempts on the same task. | The buffer is injected into later trials. | Append/replace behavior is task-local; no durable cross-project skill bank, provenance registry, or lifecycle is central to the method. | Feedback is grounded by environment/compiler/task signals. The paper reports 91% HumanEval pass@1 versus 80% for the cited GPT-4 baseline, plus gains on decision and reasoning tasks. |
| **Voyager (2023 arXiv/open-source system)** | A curriculum proposes tasks; iterative environment feedback includes execution errors and an explicit self-verification result. A code skill is committed only after the task verifies successfully. | Temporally extended executable JavaScript skills plus text descriptions in an embedding-indexed library. | The current task/context retrieves relevant skills by description similarity; skills are compositional and callable by later generated programs. | Successful skills accumulate. The paper does not center versioned editing, provenance beyond the originating task, or stale-skill retirement. | Minecraft environment execution is the gate. The paper reports 3.3× more unique items, up to 15.3× faster milestone discovery, and transfer to new Minecraft worlds. |
| **ExpeL (AAAI 2024)** | A separate experience-gathering stage collects trajectories. Insight extraction compares/critiques successful and failed experiences to produce general natural-language rules. | A scored global rule set ("insights") plus stored successful experiences. | Global insights are placed in context; task-similar successful experiences are retrieved under a token budget. | The extractor emits explicit `ADD`, `EDIT`, `REMOVE`, and `AGREE` operations. Rule strengths increase on agreement/edit, decrease on removal proposals, and rules at non-positive strength are pruned; substring matching supplies a simple duplicate check. | Evaluated on ALFWorld, WebShop, HotpotQA, and FEVER. The central evidence is benchmark improvement from experience-derived insights, but its training/extraction loop is offline rather than a product session lifecycle. |
| **Agent Workflow Memory (ICML 2025)** | Offline mode induces workflows from demonstrations. Online mode streams attempted tasks and uses a binary task evaluator; successful trajectories are candidates for induction. | A natural-language workflow description plus a generalized sequence of environment state, reasoning, and executable action. | Workflows are added to text memory and supplied to future web tasks, with grouping/selectivity based on task or website setting. | The published method is primarily induce-and-append; explicit semantic deduplication, source versioning, contradiction resolution, and retirement are not central. | On Mind2Web and WebArena, the paper reports 24.6% and 51.1% relative success improvements and 8.9–14.0 absolute-point cross-website/domain gains. It also notes that an irrelevant workflow can bias the agent toward a wrong action. |
| **CODESKILL (May 2026 arXiv preprint; not established peer-reviewed evidence)** | Coding-agent trajectories feed an LLM management policy trained with reinforcement learning. Dense rubric feedback is combined with sparse, verifiable execution reward from a frozen downstream agent. | Multi-granularity procedural skills in a compact evolving skill bank. | Relevant skills guide later coding tasks; the bank is maintained iteratively instead of recreated by a fixed prompt alone. | Selection, abstraction, evolution, and compact-bank maintenance are learned as a policy. The abstract reports stable bank size, but deployment-grade provenance and approval semantics are not its focus. | Reports +9.69 average pass-rate points over no skill and +4.01 over the strongest prompt/memory baseline on EnvBench, SWE-Bench Verified, and Terminal-Bench 2. This is a recent preprint result. |
| **Anything2Skill (June 2026 arXiv preprint; not established peer-reviewed evidence)** | Heterogeneous records—including manuals, examples, logs, and trajectories—are split into evidence windows, then processed with plan-and-expand extraction under a skill-tree prior. | Structured skill contracts containing invocation conditions, contraindications, action moves, workflow, constraints, output specification, evidence, and confidence. | At inference, task passages from the original corpus and relevant SkillBank procedures are retrieved together, separating declarative evidence from procedural guidance. | Taxonomy-aware compilation, registry reconciliation, lifecycle tracking, and versioned updates are explicit parts of SkillBank. This is the clearest surveyed artifact model for provenance and contradictions, but it is broader knowledge compilation rather than session-only learning. | Reports 98.85% qsv and 94.10% GitHub CLI success when combined with RAG, above RAG-only agents. This is a recent preprint result on two tool domains. |
| **Trace2Skill (March–June 2026 arXiv work in progress; not peer reviewed)** | Parallel analysts examine broad execution trajectories, including recurrent failures and workarounds, then consolidate trajectory-local patches into a unified skill directory. | Portable skill directories and standard operating procedures that deepen an existing skill or replace a weak initial draft. | The consolidated skill is reused directly; the method emphasizes a portable artifact rather than test-time retrieval of many memories. | Parallel consolidation is intended to avoid the order sensitivity of sequential editing. The paper does not supply a mature operational approval/provenance lifecycle comparable to a product harness. | Reports transfer across domains/models and up to +57.65 percentage points on one WikiTableQuestions transfer setting. The arXiv page explicitly labels the work "Work in Progress," so the number should not be treated as settled evidence. |

### Primary-system observations

#### 1. Hermes Agent is a closed product loop, but its strongest mechanisms are consent and maintenance rather than transfer proof

Hermes is the closest direct precedent for the requested Ark capability.

- **Classification boundary:** official docs distinguish factual memory from procedural skills. Memory is always injected and deliberately small; skills may be much larger and are loaded only when relevant. Complete transcripts remain in a third store, searchable on demand. This avoids forcing every useful session detail into one artifact class.
- **Automatic trigger:** the background review runs after a turn and can write or patch memory and skills. Periodic nudges are designed to catch lessons the foreground agent failed to retain. A cheaper auxiliary model can receive a compact digest of older conversation plus verbatim recent turns.
- **Manual source compilation:** `/learn` is complementary rather than redundant. It lets a user explicitly generalize "the workflow you just walked the agent through" or source a skill from local code, a URL, or pasted procedure.
- **Write gate:** `memory.write_approval` and `skills.write_approval` can make every automatic write pending, inspectable, approvable, or rejectable. Notifications can be generic or include a compact diff preview.
- **Lifecycle:** the Curator separates deterministic aging from optional LLM judgment. By default, inactive skills become stale at 30 days and are recoverably archived at 90 days; LLM consolidation is opt-in. Pinned skills are protected.
- **Retrieval:** Hermes keeps always-on memory bounded and uses progressive skill disclosure (`skills_list` metadata, then `skill_view`, then a support file). Full transcripts use FTS5 session search rather than model-generated summaries as the canonical record.

The documented loop does **not** show that every newly created or patched skill must pass an independent task replay before becoming active. It has safety scanning, approval, a Verification section convention, usage tracking, and retirement, but these answer different questions from "does this procedure transfer and improve outcomes?"

The current docs also do not document a first-class link from every generated skill version back to exact source session turns and runtime evidence. Hub-installed skill provenance and content hashes are tracked, while agent-authored change provenance is less explicit. This is an evidence gap, not a claim that no internal metadata exists.

#### 2. Oh My Pi is strongest on reproducible derivation and project isolation

Oh My Pi's pipeline is closer to a build system for derived knowledge:

1. Register session sources.
2. Claim only eligible changed sessions.
3. Produce a versioned per-session extraction or an explicit no-output result.
4. Materialize the source corpus (`raw_memories.md` and rollout summaries).
5. Rebuild memory, summary, and skill packages from that corpus.
6. Delete artifacts no longer emitted.

That architecture makes the final skill library reproducible from retained intermediates and naturally handles removal of stale themes. Its job-watermark and lease design also addresses concurrent agent processes. The cost is that a whole-library LLM consolidation can rewrite many artifacts at once, and the inspected implementation does not attach an execution result or confidence record to each generated skill.

Two implementation details should not be generalized as stronger guarantees than they are:

- The secret scan covers key-like strings, JWT-shaped tokens, and AWS key prefixes. It is a useful last-line filter, not a general sensitive-data classifier.
- Full regeneration plus omission-based pruning is deterministic after model output, but the model's decision to omit or rewrite a skill is still probabilistic.

#### 3. claude-mem solves recall and continuity, not procedural promotion

claude-mem demonstrates the value of capturing evidence at the point where it is generated. Tool parameters/results, file paths, facts, and a semantic narrative can later be searched without replaying a whole transcript. Stop-time summaries create checkpoints, and progressive retrieval reduces prompt cost.

That makes it relevant as a potential **source corpus** for skill extraction. It is not, in the inspected snapshot, an example of automatically turning repeated implementation experience into an authored, maintained procedural skill. Treating its shipped `mem-search` skill as proof of session-to-skill synthesis would conflate the search interface with generated content.

#### 4. Devin deliberately separates diagnosis from reusable procedure

Devin's Session Insights automatically classifies a completed session and can produce a fuller failure analysis: timeline, issue category, improved prompt, action items, and assessment of knowledge use. Its official Playbook guidance then asks a user to select a reusable incident, start a new session with the source transcript link, tell Devin what to generalize, test it on another incident, and iterate.

This is a meaningful product boundary:

- extraction/diagnosis can be automatic;
- promotion to organizational procedure is deliberate;
- validation uses more than the source session;
- Playbook versions can be inspected and reverted.

The separate Skills product adds description-based discovery and documents automatic skill **suggestions**, but the primary docs do not disclose an automatic transcript-to-skill merge policy, quality threshold, or duplicate resolution algorithm.

### Research lineage

The research systems form an increasingly structured progression:

1. **Reflexion:** write a compact lesson after a trial and reuse it on the next attempt. This establishes that text feedback can substitute for weight updates, but the memory is episodic and task-local.
2. **Voyager:** promote an executable procedure only after environment self-verification, then retrieve by semantic task description. This adds a concrete success gate and compositional code artifacts.
3. **ExpeL:** aggregate multiple successful and failed experiences into global rules, with explicit rule edit/remove/agreement operations and strength-based pruning. This adds cross-trajectory consolidation and rudimentary lifecycle.
4. **Agent Workflow Memory:** induce abstract workflows from successful trajectories and reuse them across web tasks, websites, and domains. This provides peer-reviewed evidence for transfer, while also showing that a retrieved workflow can mislead when the task differs.
5. **CODESKILL:** learn the skill-bank management policy from downstream execution reward instead of relying entirely on a fixed extraction prompt. This connects extraction quality to actual coding outcomes, but is a recent preprint.
6. **Anything2Skill:** make the artifact itself a contract with triggers, contraindications, evidence, confidence, taxonomy, versioning, and lifecycle. This addresses several operational gaps, but the source can be any corpus and the evidence is a recent preprint on two tool domains.
7. **Trace2Skill:** use parallel analysis of failures and successes to produce a consolidated portable skill rather than retrieving a bag of local memories. Its large reported transfer gains are promising but explicitly work-in-progress.

This lineage also exposes a recurring tradeoff. Success-only promotion gives a clean executable demonstration, but it can discard the failed branches that contain the most reusable pitfalls. Failure-aware systems preserve corrective knowledge, but need stronger attribution so that a local workaround is not promoted as a universal rule.

### Patterns supported strongly enough to carry into an Ark design discussion

These are recurring patterns supported by multiple independent systems or by both implementation and evaluation. They are not a selection of one final design.

1. **Keep source evidence and derived guidance as different layers.** Oh My Pi keeps per-rollout outputs under a consolidated view; claude-mem keeps observations and summaries; Devin links generalized procedures back to sessions; Anything2Skill includes supporting evidence. A transcript or tool record should remain inspectable after a compact insight is produced.

2. **Separate extraction from consolidation/promotion.** A per-session candidate can preserve local context and say "no durable signal." A later pass can compare candidates, resolve contradictions, and decide whether a pattern is repeated enough to become a project insight or skill. Oh My Pi, ExpeL, Anything2Skill, and Trace2Skill all use multi-stage processing.

3. **Route facts, user preferences, project constraints, and procedures to distinct artifact classes.** Hermes explicitly separates memory, user profile, session archive, and skills. claude-mem's observations are evidence, not instructions. Mixing these classes makes retrieval noisy and can turn an old fact into an unsafe procedure.

4. **Use outcome evidence in addition to an extractor's confidence.** Reflexion uses feedback, Voyager uses self-verifying environment execution, online AWM uses a task evaluator, CODESKILL trains against downstream execution, and Devin recommends testing reusable procedures on further sessions. "The model thought this was useful" is a weaker gate than "the procedure was associated with a verified outcome."

5. **Preserve negative evidence.** Corrections, failed commands, root causes, and successful recoveries are explicitly durable in Hermes and Oh My Pi, compared across experiences in ExpeL, and consolidated into workarounds in Trace2Skill. A successful final state alone does not explain which tempting paths should be avoided.

6. **Make retrieval selective and progressive.** Hermes and Oh My Pi inject only compact guidance and load a skill on demand; claude-mem searches an index before fetching full observations; Voyager and ExpeL retrieve task-relevant artifacts. Full-library injection is simple but scales poorly and increases irrelevant-procedure bias.

7. **Put deterministic state management around model judgment.** Useful controls include project/workspace scoping, content/source watermarks, idempotent job keys, leases, exact schemas, path constraints, content hashes, and recoverable archive states. Oh My Pi and claude-mem provide implementation evidence; Hermes adds approval and lifecycle controls.

8. **Make durable writes observable and reversible.** Hermes offers staged approval and diff-like notifications; Devin Playbooks have version history; Hermes Curator archives instead of deleting; Anything2Skill models versioned updates. Automatic extraction need not imply invisible, irreversible promotion.

9. **Treat lifecycle as part of the feature, not cleanup added later.** Skills can become stale even if originally correct. Usage state, upstream drift, contradictory new evidence, explicit pinning, versioning, and recoverable retirement appear across Hermes, Devin, Oh My Pi, and Anything2Skill.

10. **Evaluate at three levels.** Structural validity asks whether an artifact parses and has safe paths; procedural validity asks whether its commands or steps execute; transfer validity asks whether it improves a distinct future task. Existing products often cover the first, research systems often cover the third on benchmarks, and a production workflow needs to distinguish all three.

### Patterns not yet supported strongly enough

The surveyed evidence does not justify the following as generally safe defaults:

- **Automatic promotion from one completed task directly into an active project-wide skill.** Completion does not prove which actions caused success, whether the path was accidental, or whether the procedure transfers.
- **Automatic overwrite of an existing skill without a source-linked diff, version, and recovery path.** Current product controls show why approval and archival matter; research extraction scores do not replace auditability.
- **Treating all successful trajectories as clean demonstrations.** Successful sessions may contain dead ends, unnecessary commands, secrets, environment-specific paths, or unsafe workarounds.
- **Treating only successful trajectories as useful.** ExpeL, Reflexion, Hermes, Oh My Pi, and Trace2Skill all derive important signal from correction and failure.
- **Using exact-string or substring deduplication as semantic contradiction resolution.** Hermes memory and ExpeL rules use simple matching successfully for bounded operations, but paraphrases and scope differences remain unresolved.
- **Using model-generated confidence as validation.** Anything2Skill's confidence field improves inspectability, but environment/task outcomes are stronger evidence.
- **Executing model-generated supporting scripts merely because they passed schema and path checks.** Structural sanitation does not establish safety or correctness.
- **Injecting every retained artifact into every task.** AWM explicitly observes wrong-action bias from irrelevant workflows; product systems increasingly use progressive disclosure.
- **Relying on a regex secret scrubber as the privacy boundary.** Sessions can contain private prose, repository content, credentials in uncommon formats, and derived inferences that no token regex catches.
- **Assuming benchmark transfer directly predicts Ark repository-task transfer.** Voyager is Minecraft-specific; AWM is web navigation; ExpeL spans four controlled environments; the 2026 coding systems are recent preprints with bounded benchmarks.
- **Running cross-project consolidation under a single global namespace.** Oh My Pi's source comment records that this caused contamination and was replaced with a per-`cwd` key.
- **Equating frequent use with correctness.** Usage is relevant to staleness, but a frequently invoked bad or overly broad skill can remain harmful without outcome feedback.

### Open design dimensions surfaced by the evidence

The sources support multiple plausible choices; they do not settle these choices for Ark:

| Dimension | Options evidenced in prior art |
| --------- | ------------------------------ |
| Trigger | Every tool use (claude-mem); end/stop of session (claude-mem, Devin); idle/startup batch (Oh My Pi); periodic after-turn nudge (Hermes); explicit `/learn` or manual promotion (Hermes, Devin); verified task completion (Voyager/AWM). |
| Scope | User-global facts; repository/project insights; feature/task-local evidence; portable skill; organization playbook. |
| Candidate gate | Extractor says durable; repeated correction; successful task evaluator; paired success/failure comparison; user request; human approval; held-out replay. |
| Artifact | Searchable observation; task summary; atomic fact; natural-language rule; generalized workflow; `SKILL.md` contract; executable code skill; package with scripts/templates/examples. |
| Promotion | Immediate active write; pending proposal; cross-session consolidation; minimum support count; separate validation run; learned management policy. |
| Retrieval | Always-on bounded summary; metadata-only catalog; lexical/semantic search; task-similar example retrieval; project/taxonomy filter; explicit invocation. |
| Update | Append; exact/substring replace; scored ADD/EDIT/REMOVE; full regeneration from sources; versioned registry reconciliation; agent patch; human edit. |
| Retirement | Omission during rebuild; score threshold; deterministic stale/archive timer; manual deletion; pinned exemption; upstream hash drift. |
| Provenance | Session ID and source timestamp; observation/tool-use ID; source transcript link; evidence windows; successful task/evaluator result; skill version and change reason. |
| Quality signal | Schema validity; safety scan; user approval; source task success; held-out task success; repeated use; downstream reward; transfer benchmark. |

## External references

- [Hermes Agent — Persistent Memory](https://hermes-agent.nousresearch.com/docs/user-guide/features/memory/) — Official current documentation for bounded memory, background self-improvement review, frozen injection, session search, duplicate prevention, notifications, and write approval.
- [Hermes Agent — Skills System](https://hermes-agent.nousresearch.com/docs/user-guide/features/skills/) — Official current documentation for `/learn`, `SKILL.md` contracts, progressive disclosure, agent-authored skills, and skill update/install lifecycle.
- [Hermes Agent — Curator](https://hermes-agent.nousresearch.com/docs/user-guide/features/curator/) — Official current documentation for use tracking, `active → stale → archived`, pinning, dry-run, deterministic pruning, and optional LLM consolidation.
- [Hermes Agent — Configuration: write approval](https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/configuration.md) — Primary repository documentation for staging agent-created memory and skill writes, security scanning, and approval.
- [Devin — Session Insights](https://docs.devin.ai/product-guides/session-insights) — Official documentation for automatic end-of-session classification, full analysis triggers, timelines, improved prompts, action items, and knowledge feedback.
- [Devin — Create a Playbook from a successful session](https://docs.devin.ai/use-cases/gallery/create-playbook-from-session) — Official workflow showing human selection/generalization from a source session followed by reuse testing.
- [Devin — Creating Playbooks](https://docs.devin.ai/product-guides/creating-playbooks) — Official procedure structure, two-or-more-run validation guidance, editing, and version history.
- [Devin — Skills](https://docs.devin.ai/product-guides/skills) — Official distinction between discoverable skills and manually authored/invoked Playbooks, including automatic skill suggestions.
- [Reflexion — NeurIPS 2023 proceedings](https://papers.neurips.cc/paper_files/paper/2023/hash/1b44b878bb782e6954cd888628510e90-Abstract-Conference.html) — Peer-reviewed source for feedback-derived textual reflections in episodic memory and reported HumanEval results.
- [Voyager — arXiv 2305.16291](https://arxiv.org/abs/2305.16291) — Primary paper for environment-verified executable code skills, semantic retrieval, composition, and Minecraft transfer results.
- [ExpeL — AAAI 2024 article](https://ojs.aaai.org/index.php/AAAI/article/view/29936) — Peer-reviewed paper on learning reusable insights from collected success/failure experience.
- [ExpeL — official implementation](https://github.com/LeapLabTHU/ExpeL) — Primary code and runbook for experience gathering, separate insight extraction, retrieval, and rule-update behavior.
- [Agent Workflow Memory — ICML 2025 / PMLR 267](https://proceedings.mlr.press/v267/wang25ag.html) — Peer-reviewed source for offline/online workflow induction and cross-task/site/domain evaluation.
- [Agent Workflow Memory — full arXiv HTML](https://arxiv.org/html/2409.07429v1) — Primary method detail for workflow representation, successful-trajectory induction, memory use, and failure modes.
- [CODESKILL — arXiv 2605.25430](https://arxiv.org/abs/2605.25430) — May 2026 preprint on RL-trained skill extraction and compact skill-bank maintenance using downstream execution feedback.
- [Anything2Skill — arXiv 2606.09316](https://arxiv.org/abs/2606.09316) — June 2026 preprint on evidence-backed skill contracts, taxonomy, registry reconciliation, confidence, lifecycle, and versioned updates.
- [Trace2Skill — arXiv 2603.25158](https://arxiv.org/abs/2603.25158) — 2026 work-in-progress preprint on parallel trajectory analysis and portable consolidated skill directories.

## Caveats / Not found

- **Hermes efficacy:** official docs and repository material establish behavior, controls, and lifecycle. I did not find a controlled public evaluation showing the background review's autonomously authored skills improve held-out software tasks, nor a documented mandatory execution gate before an auto-generated skill becomes active.
- **Hermes provenance:** current docs establish session persistence, skill usage/lifecycle state, write approval, and source hashes for Hub skills. They do not clearly specify per-version links from every agent-created skill patch to exact source turns, evidence, and a rollback reason.
- **Oh My Pi efficacy:** the local code and tests establish the two-phase pipeline and its operational properties. I did not find a benchmark for future-task accuracy or a held-out execution validator for generated skills.
- **claude-mem procedural extraction:** searched its hooks, SDK prompts, storage tests, README, and skill references. Automatic `SKILL.md` synthesis from observations was not found; only durable observation/summary capture and retrieval were found.
- **Devin internals:** official docs expose triggers and user workflows, not the internal prompts, merge policy, duplicate handling, or confidence thresholds behind Session Insights and skill suggestions.
- **ExpeL deployment fit:** it is a staged offline learning pipeline over benchmark experiences, not a drop-in session-close lifecycle with privacy, approval, or per-repository retention semantics.
- **Research status:** Reflexion, ExpeL, and AWM have peer-reviewed venue evidence. Voyager is cited here from its arXiv/open-source publication. CODESKILL and Anything2Skill are recent 2026 arXiv preprints; Trace2Skill explicitly says "Work in Progress." Their reported numbers should be treated as research claims pending replication and peer review.
- **Benchmark scope:** Minecraft, web navigation, QA, embodied environments, and fixed coding benchmarks exercise only parts of Ark's likely setting: long-running repository work, human corrections, changing code state, overlapping tasks, secrets, and project-local conventions.
- **Privacy and security:** all automatic session mining expands the amount of sensitive material copied into derived stores. Product-level token/path scanning does not by itself answer retention duration, deletion propagation, access control, or whether user/private source text may be synthesized into a portable skill.
- **Contradiction semantics:** no surveyed production system demonstrates a general, deterministic method for deciding when two natural-language procedures are duplicates, scoped variants, or genuinely contradictory. Taxonomy, scoring, regeneration, and LLM consolidation are partial strategies.
