# Research: Stello memory planes and their mapping to Ark

- Query: Distill Stello's session/topology/memory/insight architecture and map it to Ark's automatic extraction and retention of session/task insights and procedural skills.
- Scope: mixed
- Date: 2026-07-29

## Findings

### Evidence baseline

- The vendored source examined here is `reference/stello/stello` at commit
  `e171783909ef70a129247c385543198a853f14895` (core `0.10.2`, session
  `0.8.1`).
- The current public Stello `main` was verified at
  `3bc949360585e64ecb0fe09c6f5afb92a8f172a9`, dated 2026-07-24 (core
  `0.11.0`, session `0.9.0`). It is two commits ahead of the vendored
  snapshot. The intervening functional commit makes streaming the sole LLM
  transport; the memory-plane behavior described below was checked against
  current public source.
- "L2" below names the per-session `memory` string. Stello's current API uses
  `memory`; L2 is useful only as a conceptual label inherited from its earlier
  model.
- Stello's former "Main Session"/L1 global synthesis is not the current
  agent-level shared-memory store. Current cross-session reflection is
  application-owned, while shared memory is an independent explicit store.

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `reference/stello/stello/packages/session/src/types/storage.ts` | Defines the session-scoped raw-record, insight, memory, prompt, and config storage surfaces. |
| `reference/stello/stello/packages/session/src/context-utils.ts` | Assembles a normal user turn and documents the actual prompt order. |
| `reference/stello/stello/packages/session/src/create-session.ts` | Implements insight consumption, replay behavior, and L3-to-memory consolidation. |
| `reference/stello/stello/packages/core/src/orchestrator/default-engine-factory.ts` | Implements the optional N-turn consolidation trigger and session skill filtering. |
| `reference/stello/stello/packages/core/src/stello-agent.ts` | Exposes session digests, insight writes, manual consolidation, shared-memory APIs, and topology rendering. |
| `reference/stello/stello/packages/core/src/shared-memory/types.ts` | Defines agent-wide shared memory as ordered `{slug, body}` entries. |
| `reference/stello/stello/packages/core/src/shared-memory/in-memory-shared-memory-store.ts` | Shows default volatility, overwrite semantics, serialized writes, and dirty reads. |
| `reference/stello/stello/packages/core/src/shared-memory/render-shared-memory.ts` | Renders every shared-memory body into every eligible prompt. |
| `reference/stello/stello/packages/core/src/skill/skill-loader.ts` | Loads already-authored `SKILL.md` files from disk. |
| `reference/stello/stello/packages/core/src/skill/skill-router.ts` | Registers and queries skills; it performs no extraction or intent matching. |
| `reference/stello/stello/packages/core/src/types/session.ts` | Separates content metadata from forest/topology metadata. |
| `reference/stello/stello/packages/core/src/engine/topology-render.ts` | Renders the current root subtree with a "YOU ARE HERE" marker. |
| `reference/stello/stello/CLAUDE.md` | Records Stello's intended memory, reflection, topology, and project-cognition invariants. |
| `reference/stello/stello/docs/migration-main-session-decouple.md` | Explains the removal of framework-owned L1/Main Session and the replacement application reflection loop. |
| `reference/stello/stello/docs/superpowers/specs/2026-05-17-shared-memory-design.md` | Design proposal whose indexed/lazy-recall model differs materially from shipped source. |
| `.ark/tasks/session-insight-retention/PRD.md` | Defines Ark's target problem: durable insight/skill extraction, provenance, lifecycle controls, bounded surfacing, and portability. |
| `.ark/workflow.md` | Defines current task tiers, artifacts, commit/archive behavior, journal closure, project/feature SPEC roles, and context scopes. |
| `.ark/specs/features/ark-research/SPEC.md` | Defines research corpus retention without SPEC promotion or structured recall. |
| `.ark/specs/features/subagent-support/SPEC.md` | Makes research markdown the durable subagent output contract. |
| `.ark/specs/features/ark-context/SPEC.md` | Constrains `ark context` to a read-only, body-free snapshot of workflow state. |
| `crates/ark-core/src/commands/context/model.rs` | Shows that the current context schema has no insight, memory, skill, research-body, or evidence-summary model. |
| `crates/ark-core/src/commands/context/gather.rs` | Classifies only PRD/PLAN/REVIEW/VERIFY/task.toml as current-task artifacts. |
| `crates/ark-core/src/commands/context/projection.rs` | Shows session and record scopes are separate and neither supplies semantic recall. |
| `templates/codex/agents/ark-researcher.toml` | Requires research findings to survive compaction as topic files, not chat. |
| `templates/codex/skills/ark-research/SKILL.md` | Defines human/main-agent-driven corpus collection and prose citation from later implementation tasks. |
| `templates/codex/skills/ark-record/SKILL.md` | Defines manual inter-task notes and compact journal entries. |
| `.ark/tasks/archive/INDEX.md` | Indexes archived tasks by workflow metadata, not learned insight or procedure. |

### Code patterns

#### Fact: Stello has four separate memory-like planes, plus skills

The session storage interface keeps raw records, a one-shot insight, and a
persistent memory as different values:

> `reference/stello/stello/packages/session/src/types/storage.ts:26-48`
>
> ```ts
> appendRecord(sessionId: string, message: Message): Promise<void>
> listRecords(sessionId: string): Promise<Message[]>
> trimRecords(sessionId: string, keepRecent: number): Promise<void>
> // ...
> getInsight(sessionId: string): Promise<string | null>
> putInsight(sessionId: string, content: string): Promise<void>
> clearInsight(sessionId: string): Promise<void>
> getMemory(sessionId: string): Promise<string | null>
> putMemory(sessionId: string, content: string): Promise<void>
> ```

The agent-wide plane has a much smaller schema and a wider scope:

> `reference/stello/stello/packages/core/src/shared-memory/types.ts:5-25`
>
> ```ts
> export interface SharedMemoryEntry {
>   slug: string
>   body: string
> }
> // ...
> list(): Promise<SharedMemoryEntry[]>
> get(slug: string): Promise<SharedMemoryEntry | null>
> upsert(slug: string, body: string): Promise<void>
> remove(slug: string): Promise<void>
> ```

These planes have different consumers and lifetimes:

| Plane | Scope | Producer | Normal consumer | Lifetime/update rule |
| ----- | ----- | -------- | --------------- | -------------------- |
| L3 raw records | One session ID | Normal sends/tool turns | That session's replay/compression and consolidation | Append, then optional trim/compression |
| Per-session memory ("L2") | One session ID | `consolidateFn(currentMemory, messages)` | External reflection/digest readers; exceptional internal replay path | Persistent single string; `putMemory` replaces |
| Per-session insight | One session ID | External reflection via `putInsight` | The target session's next assembled turn | One slot; write replaces; read is followed by clear |
| Shared memory | One `StelloAgent` instance, all its roots/sessions | Explicit SDK or `stello_memory_edit` tool | Every eligible session prompt | Ordered slug/body upsert; default store is process memory |
| Skills | Global loaded registry, then session-filtered | Authored `SKILL.md` files | LLM-selected activation tool | Loaded content; no session-derived generation path |

This separation is the most transferable part of Stello's model: evidence,
digest, targeted delivery, broadly shared knowledge, and executable procedure
are not treated as one artifact.

#### Fact: normal prompt assembly excludes the session's own memory

Normal user-turn assembly documents and implements this order:

> `reference/stello/stello/packages/session/src/context-utils.ts:181-183`
>
> ```ts
> // systemPrompt → sharedMemoryContext → topologyContext →
> // session_identity → insight → history → user
> ```

The implementation reads `getInsight` but never `getMemory`:

> `reference/stello/stello/packages/session/src/context-utils.ts:198-222`
>
> ```ts
> const sysPrompt = await storage.getSystemPrompt(sessionId)
> // ...
> if (sharedMemoryContext) {
>   prefixMessages.push({ role: 'system', content: sharedMemoryContext })
> }
> if (topologyContext) {
>   prefixMessages.push({ role: 'system', content: topologyContext })
> }
> // ...
> const insightContent = await storage.getInsight(sessionId)
> if (insightContent) {
>   prefixMessages.push({ role: 'system', content: insightContent })
>   insightConsumed = true
> }
> ```

Stello therefore treats memory as an outward-facing digest, rather than a
permanent self-prompt, on the normal send path. Its stated reason is to avoid a
session recursively reinforcing its own lossy summary while it still has L3
history.

That invariant is not universal in shipped source. Tool-result continuation
uses a separate replay assembler and injects the session's own memory:

> `reference/stello/stello/packages/session/src/create-session.ts:113-122`
>
> ```ts
> const insightContent = await storage.getInsight(sessionId)
> // ...
> const memory = await storage.getMemory(sessionId)
> if (memory) {
>   messages.push({ role: 'system', content: memory })
> }
> ```

Accordingly, "memory is never self-context" is documentation shorthand. The
source-backed rule is: normal user turns exclude it; the tool replay path does
not.

#### Fact: insight is targeted, one-slot, and at-most-once rather than acknowledged

The normal send clears an insight immediately after context assembly, before
the downstream LLM call completes:

> `reference/stello/stello/packages/session/src/create-session.ts:244-258`
>
> ```ts
> const assembled = await assembleSessionContext(/* ... */)
> // ...
> if (assembled.insightConsumed) {
>   await storage.clearInsight(currentMeta.id)
> }
> ```

The in-memory session storage implements both memory and insight with
`Map.set`, so a later write to the same session overwrites the earlier value
(`packages/session/src/mocks/in-memory-storage.ts:73-91`). Consequences:

- an insight targets exactly one session and one future assembly;
- multiple pending insights are not queued;
- a failed or aborted LLM request after the clear can lose the insight;
- there is no receipt, retry, provenance, expiry, or conflict record;
- reflection writers can race semantically even when an individual store
  operation is safe.

This is an inbox slot, not a durable retained insight store.

#### Fact: only L3-to-memory consolidation has a built-in automatic trigger

Consolidation reads the current memory and all records, delegates judgment to
an application-supplied function, and replaces the stored string:

> `reference/stello/stello/packages/session/src/create-session.ts:471-482`
>
> ```ts
> const currentMemory = await storage.getMemory(currentMeta.id)
> const messages = await storage.listRecords(currentMeta.id)
> const newMemory = await options.consolidateFn(currentMemory, messages)
> await storage.putMemory(currentMeta.id, newMemory)
> ```

The optional automatic policy is N turns:

> `reference/stello/stello/packages/core/src/orchestrator/default-engine-factory.ts:97-107`
>
> ```ts
> const n = this.options.consolidateEveryNTurns;
> // ...
> if (next % n === 0) {
>   session.consolidate().catch(() => {});
> }
> ```

This hook is fire-and-forget and swallows consolidation failure. Manual
consolidation is also exposed by `StelloAgent.consolidateSession`.

The larger reflection loop is not automatic:

```text
session L3 records
      |
      | configured N-turn hook or explicit consolidate()
      v
per-session memory (outward digest, replacement string)
      |
      | application chooses sessions, frequency, model, schema, and policy
      v
external batch reflection
      |
      | putInsight(targetSession, text)
      v
target session's next normal prompt, then clear
```

`StelloAgent.listSessionDigests()` supplies IDs, labels, status, memory, and
insight to make that application loop possible
(`packages/core/src/stello-agent.ts:146-153,335-376`). The framework stores no
cross-session reflection state and does not validate the reflected content.
The migration guide explicitly leaves ordering and atomicity of multiple
insight writes to the application
(`docs/migration-main-session-decouple.md:170-218,410-446`).

#### Fact: shipped shared memory is eager global prompt material

The default store is volatile:

> `reference/stello/stello/packages/core/src/shared-memory/in-memory-shared-memory-store.ts:9-17`
>
> ```ts
> export class InMemorySharedMemoryStore implements SharedMemoryStore {
>   private readonly entries = new Map<string, string>()
>   private writeLock: Promise<unknown> = Promise.resolve()
>   // ...
> }
> ```

Writes are serialized, reads permit dirty state, and `Map.set` overwrites a
body while preserving the slug's insertion position
(`in-memory-shared-memory-store.ts:20-42`). Persistence requires an
application-provided store.

Every stored body is rendered eagerly:

> `reference/stello/stello/packages/core/src/shared-memory/render-shared-memory.ts:11-18`
>
> ```ts
> const entries = await store.list()
> if (entries.length === 0) return undefined
> const blocks = entries.map(e => `## ${e.slug}\n${e.body}`).join('\n\n')
> return `<shared_memory>\n${blocks}\n</shared_memory>\n\n${HINT}`
> ```

The built-in memory-edit tool performs explicit upsert/delete and tells the
model that the full content is already visible
(`packages/core/src/builtin-tools/memory-edit-tool.ts:3-12,24-51`). There is
no automatic path from L3, consolidation, or reflection into shared memory.
The shipped entry has no type, source, timestamp, confidence, review state,
staleness marker, version, or relation to a task/commit.

#### Fact: topology determines visibility but is not itself memory

Stello separates session content metadata from a forest node:

> `reference/stello/stello/packages/core/src/types/session.ts:8-39`
>
> ```ts
> export interface SessionMeta {
>   id: string
>   label?: string
>   status: SessionStatus
>   // no parent or child fields
> }
> export interface TopologyNode {
>   sessionId: string
>   parentId: string | null
>   children: string[]
>   refs: string[]
>   depth: number
>   index: number
>   sourceSessionId?: string
> }
> ```

`SessionTree` supports multiple roots, while the topology prompt renders the
current root's subtree and marks the current node
(`packages/core/src/types/session-tree.ts:87-175,195-199,249-275`;
`packages/core/src/engine/topology-render.ts:3-19`). The resulting scope
boundaries are:

| Material | Visibility boundary |
| -------- | ------------------- |
| L3, system prompt, memory, insight | Session ID |
| Topology prompt | Current root subtree in the agent's forest |
| Shared memory | Entire `StelloAgent`, across roots |
| Skill registry | Agent-wide registry, narrowed by persisted session whitelist |
| Reflection inputs/targets | Whatever set the application chooses |

Forked sessions persist only a serializable subset of configuration, notably
system prompt and the tri-state skills policy (`undefined` inherit, `[]`
disable, names whitelist)
(`packages/core/src/types/session-config.ts:15-41`;
`default-engine-factory.ts:75-89`). Thus topology, storage scope, and skill
scope are related but intentionally non-identical.

#### Fact: Stello does not automatically extract durable skills

The skill loader parses existing files:

> `reference/stello/stello/packages/core/src/skill/skill-loader.ts:40-64`
>
> ```ts
> export async function loadSkillsFromDirectory(dir: string): Promise<Skill[]> {
>   // ...
>   const skillPath = join(entryPath, 'SKILL.md');
>   const raw = await readFile(skillPath, 'utf-8').catch(() => null);
>   // ...
> }
> ```

The router is deliberately just a registry:

> `reference/stello/stello/packages/core/src/skill/skill-router.ts:5-15`
>
> ```ts
> /**
>  * 纯注册 + 查询，不做意图匹配。匹配由 LLM 通过 Skill Tool 自行决定。
>  */
> export class SkillRouterImpl implements SkillRouter {
>   private skills = new Map<string, Skill>();
>   register(skill: Skill): void {
>     this.skills.set(skill.name, skill);
>   }
> }
> ```

The project guide asks maintainers to package stable project cognition into
`.agents/skills` manually (`CLAUDE.md:193-224`). Repository-wide source search
found no session-to-skill extractor, skill candidate store, automatic
`SKILL.md` writer, validation gate, or skill evolution mechanism.

Stello is therefore evidence for automatic L3-to-digest consolidation and an
application-mediated targeted insight loop. It is not evidence for automatic
procedural-skill retention.

### Current Ark destination model

#### Fact: Ark retains workflow evidence, not a semantic memory plane

Ark already preserves several strong source artifacts:

| Ark artifact | Current responsibility | Retention/recall boundary |
| ------------ | ---------------------- | ------------------------- |
| PRD | Intent, problem, outcomes, scope | Current task/archive; surfaced as path/status |
| PLAN | Implementation specification and validation plan | Standard/deep task; deep plan may produce feature SPEC |
| REVIEW / VERIFY | Design and implementation evidence/gates | Task corpus; not semantic recall material |
| Research corpus | Topic findings with citations | Recursively committed under research task; later tasks cite the slug in prose |
| Project/feature SPEC | Durable constraints/current behavior | Explicitly surfaced by context; feature SPEC promotion is deep-tier only |
| Journal | Compact closure and inter-task record | Append-only workspace file; not loaded as learned context |
| Archive index | Traceability of finished tasks | Metadata rows; recent context is capped and body-free |
| Shipped Ark skills | Workflow commands and agent procedures | Product/template assets, not learned project procedures |

The workflow calls archive "memory" in the traceability sense
(`.ark/workflow.md:377-382`), but no current artifact implements semantic
extraction, targeted delivery, relevance recall, or procedural promotion.

Research tasks deliberately do not promote a SPEC
(`.ark/workflow.md:234-267`;
`.ark/specs/features/ark-research/SPEC.md:4-18,97-114`). Subagent findings
survive conversation compaction only because every topic is written to a
research file (`templates/codex/agents/ark-researcher.toml:5-15`;
`.ark/specs/features/subagent-support/SPEC.md:187-200`). That is durable
evidence retention, not insight extraction.

Journal entries record a one-line effect and at most four changes at task
close (`.ark/workflow.md:203-219`;
`templates/codex/skills/ark-record/SKILL.md:18-52`). The archive index records
title/slug/month/tier. Neither is a repository of debugging recipes, failed
approaches, environmental quirks, or reusable procedures.

#### Fact: `ark context` cannot currently serve Stello-like digests or recall

The schema exposes workflow state only:

> `crates/ark-core/src/commands/context/model.rs:40-65`
>
> ```rust
> pub struct Context {
>     pub schema: u32,
>     pub scope: Scope,
>     pub generated_at: String,
>     pub git: GitContext,
>     pub tasks: Option<TasksContext>,
>     pub specs: Option<SpecsContext>,
>     pub archive: Option<ArchiveContext>,
>     pub current_task: Option<CurrentTaskContext>,
>     pub checkout: Option<CheckoutContext>,
>     pub subagents: Option<SubagentsContext>,
>     pub record: Option<RecordContext>,
> }
> ```

`CurrentTaskArtifact` recognizes PRD, PLAN, REVIEW, VERIFY, and task.toml, but
not research topics, journal semantics, learned insights, candidates, or
skills (`model.rs:131-178`; `gather.rs:505-589`). Archive context contains
recent metadata only and is capped at five (`model.rs:31-32,258-277`).
Session projection omits `record`; record projection omits current task,
specs, and archive (`projection.rs:66-84,147-202`).

Therefore no current Ark command is equivalent to Stello's
`listSessionDigests`, and there is no write primitive equivalent to
`putInsight` or shared-memory upsert.

### Source-to-destination taxonomy

This is a descriptive mapping, not a claim that Ark should copy each Stello
object literally:

| Stello plane | Closest Ark material today | Missing capability relevant to this research |
| ------------ | -------------------------- | -------------------------------------------- |
| L3 raw session records | External harness transcript, plus task files, diff, tests, VERIFY, and research evidence | Ark owns no transcript capture/index; task evidence is available but not normalized for extraction |
| Per-session memory/L2 | Journal closure and archived task/research corpus | No outward task digest containing learned recipes/candidates; journal is shallow and not recalled |
| One-shot insight | Main-agent/subagent dispatch prompt or current-turn instruction | No typed, persisted, targeted, retry-safe pending insight |
| Agent-wide shared memory | No true equivalent; SPECs are only a constraint/current-design layer | No project learned-memory store, relevance index, provenance, lifecycle, or bounded load |
| Skill registry | Shipped `.codex/skills` and workflow templates | No learned skill candidate, verification/promotion gate, project/user scope, or cross-harness representation |
| Session topology | Flat task directories, one focused task per checkout, optional worktrees/subagent descriptors | No explicit evidence lineage or source/derived relations; a full session forest is not currently required |
| External reflection | Agent reasoning over `ark context` and file artifacts | Context lacks task digests/candidates; no typed extraction/merge/delivery operation |

Two boundaries must stay explicit:

1. Ark SPECs constrain current and future implementation. Automatically
   learned observations must not silently mutate that authority layer.
2. Ark's shipped skills define workflow behavior. Automatically extracted
   procedural candidates must not directly overwrite those product assets.

### Source-grounded inferences

The following are architectural inferences from the facts above, not shipped
behavior in either project:

1. Stello's useful abstraction is the separation of planes, not its storage
   schema. Evidence, outward digest, next-use advice, shared knowledge, and
   executable skill have different review, loading, and invalidation needs.
2. Ark has a stronger semantic consolidation boundary than Stello's N-turn
   counter. VERIFY/COMMIT knows the task outcome, changed paths, test evidence,
   failed approaches retained in artifacts, and final commit identity.
3. Ark's git-backed corpus can provide provenance, diffable review, rollback,
   and conflict visibility that Stello's replacement strings do not provide.
4. Ark should not need the harness transcript as the only evidence source.
   PRD/PLAN/research/VERIFY/diff/test output are more structured and often less
   sensitive. Transcript ingestion, if supported, is an additional source with
   its own redaction policy.
5. A task digest analogous to `SessionDigest` can be compact and outward-facing
   without being injected into the task that produced it. However, Ark must
   continue to expose its authoritative PRD/PLAN/VERIFY to that task; Stello's
   self-context rule should be understood as a role boundary, not copied as an
   absolute exclusion.
6. An insight candidate is not yet a skill. A reusable procedure needs a
   separate stability, safety, parameterization, validation, scope, and
   portability decision.
7. Full eager injection turns retained knowledge into an unbounded prompt and
   prompt-injection surface. Ark's PRD requirement for bounded surfacing
   implies an index/summary/relevance step before loading bodies.

### Adopt / adapt / reject option classification

These are options for the main research synthesis and later design; this
research does not select an implementation.

#### Adopt as a conceptual contract

| Stello lesson | Why it transfers |
| ------------- | ---------------- |
| Separate evidence, digest, targeted delivery, shared knowledge, and skills | Prevents one artifact from simultaneously serving audit, prompt, authority, and executable-procedure roles |
| Make scope and lifetime explicit for every item | Task, project, user, and one-shot material have different leakage and staleness risks |
| Keep storage/IO deterministic and leave semantic extraction to an agent policy | Matches Ark's typed Rust mutations plus agent-authored artifact-content boundary |
| Derive outward-facing knowledge rather than treating raw working history as the memory | Reduces prompt size and avoids using private/noisy history as the normal recall object |
| Expose explicit consolidation and reflection operations | Makes extraction timing, failure, retry, and user control inspectable |
| Keep skill loading separate from skill authoring | Allows candidate review and cross-harness compilation without redefining runtime activation |

#### Adapt for Ark's task/commit model

| Stello mechanism | Ark-oriented adaptation to evaluate |
| ---------------- | ----------------------------------- |
| N-turn L3 consolidation | Trigger candidate extraction at VERIFY/COMMIT/task close, plus explicit manual/on-demand runs; preserve outcome and commit provenance |
| `listSessionDigests()` | Provide bounded archived-task digests/candidate metadata rather than replaying raw transcripts |
| Clear-before-call insight | Use persisted pending/delivered/acknowledged state or another retry-safe protocol if one-shot targeting is needed |
| Replacement memory string | Use identity, source task/commit, evidence links, merge/supersede history, staleness, and review state; let git preserve versions |
| Normal self-memory exclusion | Exclude a derived digest from silently steering its own extraction, while retaining access to authoritative task artifacts and allowing explicit inspection |
| Full shared-memory prompt | Surface a compact index and load selected bodies by relevance/scope; this resembles Stello's unshipped design proposal more than its current implementation |
| Reflection writes straight to insight | Extract candidates first; validate/deduplicate/scope before retention or delivery |
| Authored `SKILL.md` registry | Promote a verified procedural candidate into a harness-neutral source, then render adapters for Codex/Claude-style skill formats |
| Agent-wide store | Distinguish repository/project scope from user-global scope, with explicit export/import and no accidental cross-project visibility |

#### Reject as a basis for automatic durable retention

| Shipped Stello behavior | Reason it is insufficient for Ark's stated outcome |
| ----------------------- | ------------------------------------------------ |
| Inject every shared-memory body into every eligible turn | Unbounded context, weak relevance, and excessive exposure to stale or hostile retained text |
| Store durable knowledge as provenance-free `{slug, body}` | Cannot support audit, validation, staleness, rollback policy, or source-sensitive security |
| Clear insight before successful consumption | At-most-once loss is unsuitable for workflow-critical retained advice |
| Allow one last-write-wins insight slot | Parallel extractors or multiple pending lessons can overwrite one another |
| Swallow automatic consolidation failures | Task close needs visible, retryable status rather than silent loss |
| Automatically write directly into shipped `.codex/skills` | Conflates untrusted observation with executable workflow instruction and is not portable |
| Require Stello's complete session-forest abstraction | Ark can extract at task/commit scope; topology is useful only if evidence lineage or targeted delivery requires it |
| Depend on a host harness's private automatic memory as Ark's feature | It is neither Ark-owned nor portable, inspectable, or consistently available |

### Documentation/source drift

Stello's documentation is useful for intent but cannot be treated as the
current executable contract without checking source:

1. Current README memory sections accurately describe L3, per-session memory,
   insight, application-owned reflection, and normal self-memory exclusion.
2. The README quickstart still passes `memory: /* MemoryEngine */`, while the
   current `StelloAgentConfig` exposes `sharedMemory?: SharedMemoryStore` and
   no such `memory` field (`packages/core/src/stello-agent.ts:94-124`).
3. `CLAUDE.md:43-47` and
   `.agents/skills/session-usage/SKILL.md:18-30` show an older prompt order
   that omits agent-level shared memory and topology; `context-utils.ts:181-222`
   is the shipped order.
4. Documentation states self-memory exclusion categorically, but
   `assembleSessionReplayContext` injects memory for tool-result continuation.
5. A current source comment near `StelloAgent.putMemory`
   (`packages/core/src/stello-agent.ts:385`) says memory is injected on every
   send, contradicting the normal assembly source.
6. The shared-memory design document proposes `{slug, summary, body}`, summary
   index injection, lazy body recall, and three recall/remember/forget tools
   (`docs/superpowers/specs/2026-05-17-shared-memory-design.md:25-38,53-90,164-188`).
   Shipped source has `{slug, body}`, injects all bodies, and offers one edit
   tool. The proposal is a design alternative, not evidence of current
   behavior.
7. That design document also proposed removing legacy `MemoryEngine`, but
   `packages/core/src/types/memory.ts`, README examples, and seeded memory
   topology remain. These remnants should not be used to reconstruct the
   current reflection contract.

### External references

- [Stello current public head, 2026-07-24](https://github.com/stello-agent/stello/commit/3bc949360585e64ecb0fe09c6f5afb92a8f172a9) — pins the public version used to check the vendored snapshot.
- [Stello README — session memory and reflection](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/README_EN.md#L48-L99) — primary public description of L3, memory, insight, and application reflection.
- [Session context assembly](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/packages/session/src/context-utils.ts#L172-L249) — authoritative normal prompt order and self-memory exclusion.
- [Session send/replay/consolidation implementation](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/packages/session/src/create-session.ts) — authoritative insight-clear timing, replay exception, and L3-to-memory replacement.
- [StelloAgent digest and reflection APIs](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/packages/core/src/stello-agent.ts) — primary API for reading session digests, writing insights, manual consolidation, and shared memory.
- [Automatic consolidation hook](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/packages/core/src/orchestrator/default-engine-factory.ts#L97-L109) — source for the N-turn, fire-and-forget policy.
- [Shared-memory shipped types](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/packages/core/src/shared-memory/types.ts) — authoritative `{slug, body}` schema and consistency semantics.
- [Shared-memory shipped renderer](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/packages/core/src/shared-memory/render-shared-memory.ts) — evidence that all bodies are injected eagerly.
- [Main-session decoupling migration](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/docs/migration-main-session-decouple.md) — primary explanation of application-owned cross-session reflection.
- [Shared-memory design proposal](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/docs/superpowers/specs/2026-05-17-shared-memory-design.md) — useful indexed/lazy-recall alternative, explicitly distinguished from shipped code.
- [Skill loader](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/packages/core/src/skill/skill-loader.ts) and [skill router](https://github.com/stello-agent/stello/blob/3bc949360585e64ecb0fe09c6f5afb92a8f172a9/packages/core/src/skill/skill-router.ts) — evidence that skills are loaded authored artifacts, not automatically extracted session knowledge.

## Caveats / Not found

- No automatic session/task-to-`SKILL.md` extraction or evolution mechanism was
  found in current Stello source, documentation, or built-in tools.
- No built-in automatic memory-to-insight reflection scheduler was found.
  Stello automates only L3-to-memory consolidation when configured; the batch
  reflection loop is application-owned.
- No automatic path from per-session memory/insight into agent shared memory
  was found. Shared-memory writes are explicit.
- No provenance, source evidence, timestamp, type, confidence, validation,
  staleness, expiry, review, or rollback metadata was found in Stello's
  per-session memory/insight or shipped shared-memory schemas.
- No relevance retrieval or context-budget selection was found in shipped
  shared memory. Indexed summaries and lazy recall exist only in a design
  document.
- No Ark-owned semantic-memory or learned-skill artifact, context field,
  extraction trigger, candidate lifecycle, or recall operation was found in
  current production source/specs. Host-harness memory files are outside Ark's
  portable contract and were not treated as an existing Ark capability.
- Ark's benchmark document mentions transcript capture for benchmark runs
  (`docs/BENCHMARK.md:49,172-175,235,271`), but that is not current general
  session capture or insight retention.
- This topic intentionally does not compare Hermes Agent or other systems;
  those belong in their own research topics so their terminology and evidence
  are not conflated with Stello.
- The adopt/adapt/reject tables expose design choices for synthesis. They do
  not select a storage path, schema, CLI surface, or implementation sequence.
