# Research: Stello storage layering & dependency-injection architecture

- Query: How is Stello's memory/storage actually wired? The "seams" that make storage and memory pluggable — the companion to the conceptual files (01–03).
- Scope: external (reference corpus under `reference/stello/`)
- Date: 2026-06-25

> Source root: `reference/stello/stello/`. All paths below are relative to it.
> This is the "how is it wired" file. For *what the slots mean* see `02-per-session-context-memory.md`; for the orchestrator SDK / shared-memory concepts see `03-shared-memory-and-orchestrator-sdk.md`.

---

## 1. The two-interface storage split

Stello cuts storage into **two independent interfaces**, owned by two different packages, injected separately by the app.

| Interface | Package | Owns | Source |
| --------- | ------- | ---- | ------ |
| **`SessionStorage`** | `@stello-ai/session` | Per-session *content*: SessionMeta CRUD, L3 records, the three context slots (systemPrompt / insight / memory), optional compression cache, **transactions** | `packages/session/src/types/storage.ts` |
| **`SessionTree`** | `@stello-ai/core` | Topology *structure*: node edges (`parentId`, `children`, `refs`, `depth`, `index`, **`sourceSessionId`**), persisted `SerializableSessionConfig`, cross-tree refs | declared in `@stello-ai/core` (`StelloEngine.sessions: SessionTree`), impl `packages/core/src/session/session-tree.ts` |

`SessionStorage` is purely content (`storage.ts:18-65`): `getSession/putSession/listSessions`, `appendRecord/listRecords/trimRecords` (L3), `getSystemPrompt/putSystemPrompt`, `getInsight/putInsight/clearInsight`, `getMemory/putMemory`, optional `getCompressionCache?/putCompressionCache?`, and `transaction<T>(fn)`. Its doc comment is explicit about the boundary:

> `storage.ts:13-17` — "所有 Session（含 root）共用同一个接口。拓扑节点 CRUD 由 core SessionTree 持有，不在此接口职责内。"

`SessionTree` owns only topology + the *serializable* config. The persisted config is deliberately tiny:

> `packages/core/src/types/session-config.ts:37-42` — `SerializableSessionConfig` keeps **only** `systemPrompt?` and `skills?`. Everything else in `SessionConfig` (`llm` / `tools` / `consolidateFn` / `compressFn` / `forkCompressFn`, lines 15-30) is a runtime reference re-synthesized on every fork — it is never written to disk.

**Both interfaces usually share one backend, but the interfaces stay separate.** From `CLAUDE.md:103-112`:

> "存储职责切成两条独立的线 … 两者通常共享同一份持久化后端，但接口分离让 Session 层（运行单条对话）与编排层（管理整棵森林）的职责互不耦合。"

**The id must be semantically identical across both lines.** `CLAUDE.md:110`:

> "两个接口的 `id` 必须语义一致——同一 Session 的 `SessionMeta.id === TopologyNode.id`。"

The skill restates this as the binding invariant the app must uphold (`.claude/skills/storage-design/SKILL.md:102`):

> "两个接口的 `id` 必须语义一致（同一个 Session 的 `SessionMeta.id` === `TopologyNode.id`）。应用层在创建/删除 Session 时需保证两条线同步。"

Note: the two packages each declare *their own* `SessionMeta` with non-identical field sets — the persistence layer stores a superset and each adapter projects (`storage-design/SKILL.md:41-43`; concretely in PG, `server-storage/SKILL.md:44-48`: `PgSessionStorage` → `id/label/status/createdAt/updatedAt`; `PgSessionTree` → core SessionMeta + the pure-tree `TopologyNode`).

**Why split at all** — the agent's batch APIs *compose* the two lines at the SDK layer so storage never needs join methods. `CLAUDE.md:112`:

> "`StelloAgent.listSessionDigests` 等批量 API 在 SDK 上组合两条线（`SessionTree.listAll()` × `SessionStorage.getMemory/getInsight`），存储层不需要专用方法。"

This is visible in code — `stello-agent.ts:350-365` `listSessionDigests` calls `this.sessions.listAll()` then `storage.getMemory(m.id)` / `storage.getInsight(m.id)` per row. The seam is the `m.id` shared across both calls.

---

## 2. The four-layer architecture

`CLAUDE.md:53-75` defines four layers, top to bottom, with DI entering at the bottom:

| Layer | Owns | Provided by |
| ----- | ---- | ----------- |
| **HTTP / SDK** | REST / WebSocket, multi-tenancy, cross-language clients. Transport-agnostic core below it. | `@stello-ai/server` (Hono REST + `ws`) — `server-design/SKILL.md` |
| **Application (应用层)** | Developer supplies: `SessionStorage`, `SessionTree`, `LLMAdapter`, `ConsolidateFn`, `CompressFn`, tool defs, **and the reflection loop**. | the integrating app |
| **Orchestration (编排层)** | `StelloAgent` (orchestrator-facing data SDK + Engine scheduling). `Engine`: tool-call loop, consolidation scheduling, fork orchestration, fire-and-forget side effects, events. | `@stello-ai/core` |
| **Session (Session 层)** | One conversation unit: `send()` (single LLM call), `consolidate()`, `fork()`. Does **not** know tree structure, does **not** run the tool-call loop. | `@stello-ai/session` |

The DI arrows enter at the bottom (`CLAUDE.md:73-75`): `SessionStorage`, `SessionTree`, `LLMAdapter` are injected upward into the orchestration/session layers.

**The one-directional downward-dependency rule** is design decision #17 (`CLAUDE.md:170`):

> "层级依赖单向向下 — Engine 不 import Orchestrator，共享类型定义在 `types/` 层。"

You can see this enforced structurally: `packages/core/src/types/engine.ts` imports only from `./session`, `./memory`, `./lifecycle`, `./session-config`, and the `@stello-ai/session` *types* — never upward into the orchestrator. Shared contracts live in `types/`. Coding rule reinforcing it (`CLAUDE.md:176`): "模块间只通过 interface 通信，不允许跨包 import 内部文件."

---

## 3. The external injection-point table

From `CLAUDE.md:133-148` (外部注入点), cross-checked against `StelloAgentConfig` in `stello-agent.ts:94-138` and `.claude/skills/stello-agent-creation/SKILL.md:40-57`:

| Injection point | What it pluggably provides | Wired at |
| --------------- | -------------------------- | -------- |
| **SessionStorage** | Per-session content persistence (L3 + the 3 slots + transactions). Enables the orchestrator-facing data SDK. | `StelloAgentConfig.storage?` (`stello-agent.ts:96-102`) |
| **SessionTree** | Topology + serialized-config persistence (the forest). | `StelloAgentConfig.sessions` (required) |
| **LLMAdapter** | The LLM interface (message array, tool-use, optional stream). Not injected at agent top level — flows in via `SessionConfig.llm` / `sessionDefaults` / closures. | `SessionConfig.llm` (`session-config.ts:19`) |
| **ConsolidateFn** | L3 → memory transform. **App defines the memory format; the fn picks its own LLM tier** (no LLM injected into it — decision #12). | `SessionConfig.consolidateFn` (`session-config.ts:25`) |
| **CompressFn** | History summarization when context exceeds `maxContextTokens * 0.8`. Also picks its own tier. | `SessionConfig.compressFn` (`session-config.ts:27`) |
| **sessionDefaults** | Agent-level default `SessionConfig` for every Session; **lowest priority** in the fork synthesis chain. | `StelloAgentConfig.sessionDefaults?` (`stello-agent.ts:119`) |
| **ToolRegistry** | App tool registration. Built-in tools (`createSessionTool()` / `activateSkillTool(skills)` / `memoryRecall/Remember/ForgetTool()`) are **explicit opt-in** as `ToolRegistryImpl([...])` ctor args (decision #13). | `capabilities.tools` (`stello-agent.ts:42`) |
| **SkillRouter** | Skill registry (two-stage progressive prompt fragments: name+desc always visible, `activate_skill` injects full content). | `capabilities.skills` (`stello-agent.ts:43`) |
| **ForkProfileRegistry** | Pre-registered fork config templates (systemPrompt synthesis strategy + LLM/tools/context/skills presets). Optional. | `capabilities.profiles?` (`stello-agent.ts:46-47`) |
| **SessionRuntimeResolver / sessionLoader** | Session load entry point. Every Session (incl. root) goes through one path. Either `runtime.resolver` (custom impl) **or** `session.sessionLoader` (pure I/O loader for `@stello-ai/session`). | `runtime.resolver` / `session.sessionLoader` (`stello-agent.ts:57-75`, resolved in `resolveRuntimeResolver` `:156-215`) |
| **sharedMemory** *(not in the original CLAUDE table but a real injection point)* | Agent-level `SharedMemoryStore`; when injected, enables 4 SDK methods + `stello_memory_edit` tool + auto-injection of the `<shared_memory>` block before every `send` (only on the default `sessionLoader` path). | `StelloAgentConfig.sharedMemory?` (`stello-agent.ts:103-118`) |

**The framework is format-agnostic about memory.** `CLAUDE.md:148`:

> "框架对 memory 内容格式完全无感知——`ConsolidateFn` 输出什么格式，应用层的 reflection 循环就消费什么格式。"

This is the load-bearing boundary. The framework moves bytes into the `memory` slot and reads them back out via `listSessionDigests`; it never parses them. The "L2 vs synthesis" distinction is an *app-layer label*, not a framework concept (`docs/migration-main-session-decouple.md:25,29`): "'L2' / 'synthesis' 是应用层标签，框架对内容无感知." The reflection loop that consumes memory and writes back `putInsight` lives entirely in the app (`stello-agent-creation/SKILL.md:449-469`; `migration-main-session-decouple.md:384-446`).

---

## 4. "Framework provides no filesystem/DB adapter" stance

Both content storage and shared memory ship **only an interface + an in-memory implementation**. Persistence shape is the app's call.

**`SessionStorage`** — only `InMemoryStorageAdapter` ships (`packages/session/src/mocks/in-memory-storage.ts`), explicitly "主要用于测试" (`:6`). It is plain JS `Map`s (`:9-13`) and its `transaction` just runs the fn inline (`:93-95`: "内存实现可直接执行 fn"). No FS / SQLite / PG adapter in `@stello-ai/session`.

**`SharedMemoryStore`** — same stance. Only `InMemorySharedMemoryStore` ships (`packages/core/src/shared-memory/in-memory-shared-memory-store.ts`), a `Map<slug, body>`. The spec is explicit it will **not** ship a filesystem impl (`docs/superpowers/specs/2026-05-17-shared-memory-design.md:258`):

> "**不提供** `FileSystemSharedMemoryStore`：与 SessionStorage 文件适配器一样的处理——落盘策略（per-entry .md / 单文件 JSON / SQLite）应用层差异大，留给应用层。"

**Suggested serialization layout for shared memory** — documentation guidance only, *not* an interface constraint (`spec:260-273`):

```
basePath/
  shared-memory/
    INDEX.md
    entries/
      prefer-concise.md
      user-profile.md
```

> "但 store 实现愿意把所有 entry 塞一个 JSON 也合法。" (`spec:273`)

So the per-entry `.md` / single-JSON / SQLite / PG decision is entirely the app's. The legacy `FileSystemMemoryEngine` was *deleted* in the same release (`spec:277-291`) — the framework deliberately stopped shipping a persistence engine.

**Exception — the server package DOES ship PG.** `@stello-ai/server` provides concrete PG implementations (`.claude/skills/server-storage/SKILL.md`): three adapters — `PgSessionStorage` (implements `SessionStorage`), `PgSessionTree` (implements `SessionTree`), `PgMemoryEngine` — over one 7-table schema (`users, spaces, sessions, records, session_data, session_refs, core_data`, `:18`). Key PG decisions worth noting for any Ark equivalent:

- No `topology_nodes` table — `TopologyNode` is derived from `sessions` (`SELECT id, parent_id, label`), children via `WHERE parent_id=`, refs via JOIN (`server-storage/SKILL.md:19-21`).
- One **unified slot table** `session_data(session_id, key)` with UPSERT `ON CONFLICT (session_id, key) DO UPDATE` rather than a column per slot (`:18,40-42`) — avoids column explosion as slots grow.
- `spaceId` isolation bound at adapter construction; every query `WHERE space_id = $1` (`:38-39`).
- `transaction()` distinguishes `Pool` vs `PoolClient` by type (`:52-53`).

So the layering is: **core ships interface + in-memory; server ships the production PG; the app is free to do anything in between.**

---

## 5. Transactions & write-lock concurrency at the storage seam

Two distinct concurrency mechanisms, one per interface:

**`SessionStorage.transaction<T>(fn)`** (`storage.ts:64`) — atomic multi-write for content. The in-memory impl runs `fn` inline (no real isolation, `in-memory-storage.ts:93-95`); the PG impl opens a real transaction, acquiring an exclusive client when handed a `Pool` (`server-storage/SKILL.md:52-53`). The migration doc shows the *use case*: the old framework wrapped `putMemory(rootId, synthesis)` + N×`putInsight` in `storage.transaction` for atomicity; post-refactor that atomicity is opt-in — "若需要原子性，自己用 `agent.storage.transaction(...)` 包裹" (`docs/migration-main-session-decouple.md:217`). So atomicity at the reflection write-back is now an *app choice*, exposed through the seam.

**Write-lock serialization** is the pattern for the *structural* stores (topology + shared memory) — not DB transactions but an in-process serialized promise queue. Identical `withWriteLock` in both:

- `SessionTreeImpl` — `packages/core/src/session/session-tree.ts:100-107`:
  ```ts
  private writeLock: Promise<unknown> = Promise.resolve();
  private withWriteLock<T>(fn: () => Promise<T>): Promise<T> {
    const next = this.writeLock.then(fn, fn);
    this.writeLock = next.catch(() => undefined);
    ...
  ```
- `InMemorySharedMemoryStore` — `packages/core/src/shared-memory/in-memory-shared-memory-store.ts:11-18` (same shape, comment: "沿用 SessionTreeImpl 的范式").

The `SharedMemoryStore` contract codifies the model (`shared-memory/types.ts:16`):

> "写操作（upsert / remove）由实现内部串行化（writeLock 范式），读操作允许脏读。"

So: **writes serialized, reads are dirty-read / lock-free, ordering (insertion order = FIFO) is a contract** that the index renderer depends on (`spec:248`). The lock is an implementation detail — never exposed to callers (`spec:249`).

---

## Why this matters for Ark

- **Pluggable storage interface vs Ark's fixed `.ark/` filesystem layout.** Stello's whole design assumes the persistence shape is the integrator's choice — it ships only an interface + in-memory, and lets the app pick per-entry `.md` / JSON / SQLite / PG. Ark is the opposite: a *fixed* on-disk `.ark/` layout owned by `layout::Layout`, routed through `PathExt`, with no DB. If Ark borrows Stello's slot model, the "adapter" is not pluggable — it is the `.ark/` directory convention. The seam Stello puts behind an interface, Ark hard-codes; decide deliberately whether that loses anything (multi-tenancy, swap-out testing).
- **The two-interface split (content vs topology) maps cleanly onto Ark's existing split.** Per-session content ≈ task `research/` corpus + per-session memory files; topology ≈ task/session tree metadata. The invariant worth stealing: **one id, two views, app keeps them in sync** (`SessionMeta.id === TopologyNode.id`). Ark already has this tension between `task.toml` state and on-disk artifacts.
- **The format-agnostic memory boundary is the cleanest idea to port.** Stello never parses memory content — `ConsolidateFn` writes any format, the app's reflection loop reads it. For Ark this argues for treating session-memory files as opaque markdown blobs at the framework layer, with any structure (headings, slots) being a *convention the agent follows*, not something `ark-core` validates — consistent with Ark's existing "managed block / SPEC body is agent's judgment" stance.
- **Synthesis is app-layer, not framework.** Stello deleted `MainSession.integrate()` and pushed cross-session reflection entirely into the orchestrator (`listSessionDigests` → app LLM → `putInsight`). The framework holds *no cross-session state*. If Ark adds session memory, resist building an automatic global-synthesis engine into `ark-core`; expose read/write primitives and let the agent/workflow drive reflection.
- **Atomicity is opt-in at the seam, not assumed.** Stello moved from framework-guaranteed transactional write-back to "wrap it yourself if you need it." Ark's commit path already does scoped atomic git commits + rollback — note that Stello's experience is that *per-write* atomicity for memory updates is rarely needed and costs an interface method; bias toward the simpler non-atomic write unless a concrete corruption case demands the transaction.
