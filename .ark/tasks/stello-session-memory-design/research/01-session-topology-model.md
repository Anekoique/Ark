# Research: Stello's Session & Topology Model

- Query: Stello's session & topology model — the structural backbone of how it organizes conversations.
- Scope: external (read-only reference project at `reference/stello/stello/`)
- Date: 2026-06-25

Stello is an open-source "conversation topology engine" (TypeScript SDK). It lets an AI
agent split a linear conversation into a **forest of tree-shaped Sessions**, exposing only
orchestrator-facing data APIs; cross-branch reflection/insight is left to the application
layer (`CLAUDE.md:9`).

## Findings

### Files (internal to the reference)
| Path | Role |
| ---- | ---- |
| `reference/stello/stello/CLAUDE.md` | Canonical architecture doc — single-Session model, three context slots, four layers, fork synthesis chain, confirmed design decisions |
| `reference/stello/stello/docs/superpowers/specs/2026-05-16-decouple-main-session-design.md` | The Main-Session removal spec — *why* root == child == one Session, multi-root forests, deletion list |
| `reference/stello/stello/.claude/skills/session-usage/SKILL.md` | Session as single-LLM-call primitive; context assembly; slot semantics; cross-session comms |
| `reference/stello/stello/.claude/skills/fork-design/SKILL.md` | Full fork mechanism: 4-layer synthesis chain, systemPrompt modes, skills tri-state, persistence boundary, `topologyParentId` vs `sourceSessionId` |
| `reference/stello/stello/.claude/skills/storage-design/SKILL.md` | SessionStorage vs SessionTree two-interface split; SessionMeta/TopologyNode decoupling |
| `reference/stello/stello/.claude/skills/engine-design/SKILL.md` | Engine = per-session-round driver; orchestrates the two-step fork |
| `reference/stello/stello/packages/core/src/types/session.ts` | `SessionMeta` (core), `TopologyNode`, `SessionTree` interface |
| `reference/stello/stello/packages/session/src/types/session.ts` | `SessionMeta` (session pkg), `ForkOptions`, `ForkContextFn` |
| `reference/stello/stello/packages/core/src/session/session-tree.ts` | `SessionTreeImpl` — topology persistence, multi-root, write-lock RMW |
| `reference/stello/stello/packages/session/src/create-session.ts` | `createSession` / `loadSession` factories + `session.fork()` impl |
| `reference/stello/stello/packages/core/src/orchestrator/session-orchestrator.ts` | Routes fork to the source session's engine; `topologyParentId ?? source` default |

---

### 1. The "single Session" model — root and child are runtime-isomorphic

Stello has exactly **one** kind of Session. The conversation origin is a root session with
`parentId === null` created via `agent.createSession()`; later branches hang under a parent
via `agent.forkSession()`. The **only** difference between root and child is
`TopologyNode.parentId` (`CLAUDE.md:18-20`, decision #11 at `CLAUDE.md:164`):

> "Root 与 child 在运行时完全同构——唯一差异是 `TopologyNode.parentId`。"

All sessions share one `SessionStorage` interface, one context-assembly rule, one fork
synthesis chain, and one `sessionLoader` (`CLAUDE.md:20-26`). Root has **no** privileged
methods — `storage-design/SKILL.md:19`: "Root 没有特权方法——它就是一个 `parentId === null`
的普通 Session." This is enforced in code: `SessionTreeImpl.createSession()` branches only on
whether `options.parentId` is empty (root: `parentId:null, depth:0, index:0`) vs. present
(child: `parentId:parent.id, depth:parent.depth+1`), with otherwise identical `StoredMeta`
construction (`session-tree.ts:119-175`).

**Multi-root forests are legal.** Under one agent, mutually independent trees coexist
(`CLAUDE.md:18`; type doc `// 'parentId === null' 即为 root。多 root 合法。` at
`session.ts:28-29`). `listRoots()` filters `parentId === null`; `getTree()` returns a
`SessionTreeNode[]` array (a forest, empty array when no roots) (`session-tree.ts:196-199`,
`253-276`). This multi-root support is explicitly the byproduct of deleting the old
`MAIN_SESSION_ID` root-uniqueness constraint (decouple spec §8.6, line 345).

**Historical context (why):** Stello *used* to have two Session types — a "normal" Session and
a privileged "Main Session" with its own `integrate()` method, `MAIN_SESSION_ID = 'main'`
constant, `MainStorage` superset, etc. (decouple spec §1.1, lines 13-18). The 2026-05-16 spec
deletes the Main Session concept entirely (§1.2, lines 22-27): the "conversation origin" is now
just "a root session = any topology node with `parentId === null` = an ordinary Session." The
old cross-session synthesis job is **outsourced to an external orchestrator client** (Claude
Code / Codex / user script) that calls pure-data SDK APIs and runs reflection on its own LLM.

---

### 2. Topology decoupled from session identity

`SessionMeta` carries **no** `parentId`, `children`, or `depth`. Topology lives in a separate
`TopologyNode` maintained by a separate interface (`CLAUDE.md:84`; decision #10 at
`CLAUDE.md:163`). The session-package `SessionMeta` is minimal — `id, label, status,
createdAt, updatedAt` (`packages/session/src/types/session.ts:2-8`). The core-package
`SessionMeta` adds runtime counters (`turnCount`, `lastActiveAt`) but still no tree fields
(`packages/core/src/types/session.ts:14-22`). The JSDoc states the invariant directly:

> "不包含树结构信息，Session 不感知自己在拓扑中的位置。" (`core/.../session.ts:11-12`)

`TopologyNode` is the pure tree structure: `id, parentId, children[], refs[], depth, index,
label, sourceSessionId?` (`core/.../session.ts:30-39`).

**The two-interface split** (`storage-design/SKILL.md:8-13`, `CLAUDE.md:103-113`):

| Interface | Package | Responsibility |
| --------- | ------- | -------------- |
| `SessionStorage` | `@stello-ai/session` | Single-Session **content**: L3 records, systemPrompt, insight, memory; transactions |
| `SessionTree` | `@stello-ai/core` | **Topology**: node relations (incl. `sourceSessionId`), persisted `SerializableSessionConfig` (only `systemPrompt`/`skills`), cross-tree refs |

**Why the separation:** the Session layer (running one conversation) and the orchestration
layer (managing the whole forest) must stay decoupled. The two interfaces are usually backed
by the same persistence store but their consumers get responsibility-trimmed views. Critical
invariant: the two `id`s must be semantically identical — `SessionMeta.id === TopologyNode.id`
for the same Session, and the app must keep both lines in sync on create/delete
(`CLAUDE.md:110`, `storage-design/SKILL.md:102`). Batch APIs like
`StelloAgent.listSessionDigests` compose the two lines on the SDK side
(`SessionTree.listAll()` × `SessionStorage.getMemory/getInsight`) so the storage layer needs
no dedicated batch method (`CLAUDE.md:112`).

App-domain fields (conflicts/relations/priority/flags) deliberately do **not** enter
`SessionMeta`. The app defines its own wrapper (composition: Stello Session + a private
side-table) — Stello does not model the application domain (decouple spec §4.7, lines 199-211).
Rationale incl. "cross-session relations are naturally **edges, not node properties**."

---

### 3. The fork synthesis chain

At fork time a final `SessionConfig` is synthesized via a fixed 4-layer later-wins chain;
later layers override earlier ones field-by-field, and `undefined` never overrides
(`CLAUDE.md:118-122`, `fork-design/SKILL.md:96-105`):

```
sessionDefaults → parent (persisted config) → ForkProfile → EngineForkOptions
low priority                                                 high priority
```

The base `SessionConfig` has 6 fields: `systemPrompt, llm, tools, skills, consolidateFn,
compressFn`. `ForkProfile extends SessionConfig` adds 4 template fields (`systemPromptFn`,
`systemPromptMode`, `context`, `prompt`). `EngineForkOptions extends SessionConfig` adds 6
runtime fields (`label` required, `prompt`, `context`, `topologyParentId`, `profile`,
`profileVars`) (`fork-design/SKILL.md:42-90`). All three share the same 6-field base — the
differing fields are responsibility-driven (a profile is a reusable template; options are
per-fork runtime args).

**Persistence boundary — only `systemPrompt` + `skills` are persisted.** The synthesis result
writes just those two fields into `sessions.putConfig` as a `SerializableSessionConfig`; the
other four (`llm/tools/consolidateFn/compressFn`) are runtime references (functions, adapters,
closures) re-synthesized live on every fork (`CLAUDE.md:124`,
`fork-design/SKILL.md:157-177`). If both fields are `undefined` the `putConfig` write is
skipped to avoid storage noise (`fork-design/SKILL.md:166-168`). **Consequence:** nested forks
do **not** auto-inherit a parent's `llm` — a grandchild's `llm` comes from
`sessionDefaults`/its own profile/options, not the child (`fork-design/SKILL.md:177`).

**`systemPrompt` 3 modes** via `ForkProfile.systemPromptMode` (default `prepend`)
(`CLAUDE.md:125`, `fork-design/SKILL.md:120-128`):

| Mode | Result |
| ---- | ------ |
| `preset` | profile prompt only; forkOptions systemPrompt ignored |
| `prepend` (default) | `{profilePrompt}\n\n{forkOptionsPrompt}` |
| `append` | `{forkOptionsPrompt}\n\n{profilePrompt}` |

The profile's prompt source is `profile.systemPromptFn?.(profileVars) ?? profile.systemPrompt`;
if neither profile nor options contribute a prompt it falls back to the ordinary
parent → defaults later-wins chain (`fork-design/SKILL.md:115-128`).

**`skills` tri-state** — the array is replaced wholesale, never merged
(`CLAUDE.md:126`, `fork-design/SKILL.md:133-142`):

| Value | Meaning |
| ----- | ------- |
| `undefined` | not configured — inherit lower layer; if all layers `undefined`, inherit the global SkillRouter (no whitelist) |
| `[]` | explicitly disabled — that session sees no skills; **can override a lower non-empty value** |
| `['a','b']` | whitelist — only these skills visible |

The key nuance: explicit `[]` overriding a lower `['a','b']` is standard behavior — the
"undefined doesn't override" rule does not block an explicit empty array from taking effect.

**`topologyParentId` vs `sourceSessionId` separation** — topology attach-point ≠
context-inheritance source (`CLAUDE.md:127`, decision #14 at `CLAUDE.md:167`,
`fork-design/SKILL.md:182-192`):

| Concept | Meaning | Source |
| ------- | ------- | ------ |
| `sourceSessionId` | context-origin session (systemPrompt + history inherited from) | always `= current session.id` |
| `topologyParentId` | tree parent (mount point in the star-map) | options-given; defaults to `sourceSessionId` |

When `topologyParentId` is omitted the two are equal (default tree shape: fork-from-X mounts
under X). Passing it explicitly lets a node mount under the root or any existing node while
context still inherits from the fork-initiating session. The orchestrator implements this
default via `options.topologyParentId ?? this.session.id` and does not rewrite topology routing
itself (`session-orchestrator.ts:38-40, 108-119`).

---

### 4. Fork = create independent Session + add topology node

A fork is two steps, orchestrated topology-first by the Engine
(decision #15 at `CLAUDE.md:168`, `engine-design/SKILL.md:32`, `storage-design/SKILL.md:71-74`):

```
Engine.forkSession(options)
  1. sessions.createSession({ parentId, label, sourceSessionId })  ← get ID first
  2. sessions.putConfig(childId, serializable)                      ← persist systemPrompt + skills
  3. session.fork({ id: childId, context, prompt })                 ← create the Session instance
```

`session.fork()` itself (`create-session.ts:494-544`) writes a fresh `childMeta`, sets the
child systemPrompt (`forkOptions.systemPrompt ?? parent's systemPrompt`,
`create-session.ts:509-512`), applies the **one-time** context strategy
(`create-session.ts:514-524`):

- `'none'` (default) — empty L3 history
- `'inherit'` — copy all parent L3 records
- `ForkContextFn` — custom subset of the parent message array

then writes the optional opening `prompt` as the child's first assistant message
(`create-session.ts:527-533`). Memory and insight slots are **not** copied
(`session-usage/SKILL.md:72-74`). After this one-time inheritance the two Sessions are fully
independent: child sessions are completely unaware of each other, and the **only** cross-session
channel is `insight` (decision #8 at `CLAUDE.md:161`; `session-usage/SKILL.md:51-64`):

```
all sessions' memory ──→ app reflection layer (any LLM) ──→ putInsight(targetId, content)
   (StelloAgent.listSessionDigests collects in one shot)
```

`insight` is a one-time inbox (`getInsight` injected once at `send()` then `clearInsight`d —
see `create-session.ts:256-258`); `memory` is persistent but **never enters the session's own
LLM context** — it is an outward-facing description consumed by the external reflection loop
(`CLAUDE.md:39-41`, three-slot table `CLAUDE.md:35-39`). `memory` semantics are opaque to the
framework: whatever `ConsolidateFn` outputs is what the app's reflection loop consumes
(`CLAUDE.md:148`).

> Note on the FS-backed `SessionTreeImpl`: each created session also gets seeded
> `memory.md` / `scope.md` / `index.md` files and a top-level `core.json`
> (`session-tree.ts:178-183, 144-147`) — an app-layer storage detail, not part of the
> framework contract.

---

### 5. How this differs from a flat linear chat and from Ark's flat task-directory model

**vs. a flat linear chat:** a normal chat is a single growing message list with one identity
and one ever-expanding context. Stello replaces that with a forest of independent Sessions:
each Session is a single-LLM-call primitive (`send()` = one call; the tool-call loop lives in
the Engine, decision #9 at `CLAUDE.md:162`), context is assembled by a fixed rule
(`system prompt → session_identity(label) → insight → L3 history → user msg`,
`CLAUDE.md:43-47`), and branching is first-class (fork = new Session + topology node).
Cross-branch knowledge does **not** flow through a shared transcript — it flows out as `memory`
(read by an external reflection layer) and back in as one-time `insight`. There is no global
"whole conversation" state inside the framework (decision #16 at `CLAUDE.md:169`).

**vs. Ark's flat task-directory model:** Ark organizes work as a flat set of task directories
under `.ark/tasks/<slug>/` (each holding `PRD.md`, `*_PLAN.md`, `task.toml`, `research/`, etc.,
per `CLAUDE.md` repo layout). Tasks are independent units with a lifecycle state machine
(`task.toml` phase transitions) but carry **no explicit parent/child topology** — relationships
between tasks are not a first-class edge; there is no `parentId`, no forest, no
context-inheritance chain, and no per-task "memory/insight" message-slot machinery. Ark's
"shared state across a session" is the filesystem corpus + `ark context` snapshot, not an
in-memory conversation tree. Stello, by contrast, makes topology (the parent/child/ref edges)
and context-inheritance (the fork synthesis chain) explicit, queryable runtime objects
decoupled from the conversation content itself.

## Caveats / Not found

- The `StelloAgent` top-level class source was not read directly (only its API surface from
  `CLAUDE.md` and the decouple spec); `agent.createSession` returns a `TopologyNode` per
  decouple spec §5.5 (line 253) and `agent.forkSession` returns a `TopologyNode` per
  `session-orchestrator.ts:111`.
- `mergeSessionConfig` implementation was not opened; the 4-layer chain and field-override
  rules are taken from `fork-design/SKILL.md` and the decouple spec §5.3, which describe the
  current (post-Main-removal) behavior.
- The Engine's concrete `forkSession` source (skill-filter wrapping, `_stello.allowedSkills`
  metadata) was described from `engine-design/SKILL.md:36`, not read line-by-line.
- The decouple spec is dated 2026-05-16; `CLAUDE.md` and the skills describe the post-refactor
  state and are mutually consistent — no contradictions found between spec, skills, and the
  TypeScript sources read.

## Why this matters for Ark

- **Topology as a first-class, decoupled object.** Stello proves you can separate "conversation
  identity/content" (SessionStorage) from "structural relationships" (SessionTree). If Ark ever
  wants task/session lineage (fork-a-task, sub-task trees), modeling the edge set separately
  from task content — rather than as a field on the task — is the cleaner path.
- **One uniform primitive, difference is only the edge.** Root == child with the sole
  distinction `parentId` collapses special-casing. Ark's main-session vs. subagent distinction
  could similarly be "same task unit, different topology position" instead of separate types.
- **Memory vs. insight split is directly relevant to this task.** Stello's invariant — `memory`
  is outward-facing (never re-injected into the owning session) while `insight` is a one-time
  inbox written by an external reflection loop — is a concrete pattern for "AI-native memory"
  that keeps a session's own context clean while still enabling cross-unit knowledge transfer.
  (Note: a `StelloAgent`-level *shared* memory mechanism, "Claude Code auto-memory route," is
  explicitly deferred to a future spec per decouple spec §7.5 item 5 — worth tracking.)
- **Zero implicit LLM in the data layer.** All orchestrator-facing APIs are pure data IO;
  cross-session synthesis is outsourced to the caller's own LLM. An Ark memory layer could
  mirror this: structural state stays mechanical/queryable, judgment stays with the agent.
