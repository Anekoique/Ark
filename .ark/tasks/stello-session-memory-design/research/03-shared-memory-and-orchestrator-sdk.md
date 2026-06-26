# Research: Stello shared agent-memory + orchestrator-facing data SDK

- Query: Stello's agent-wide SHARED memory (Claude-Code-auto-memory layer) AND the orchestrator-facing data SDK (who owns reflection)
- Scope: external (read-only reference corpus at `reference/stello/stello/`)
- Date: 2026-06-25

> **Important divergence up front.** The design spec
> (`2026-05-17-shared-memory-design.md`) and the **as-shipped code** disagree on
> three concrete points. The spec describes a *summary-indexed + body-lazy-loaded*
> model with three tools; the shipped code is a *full-body-injected* model with one
> tool. Both are documented below side by side. Where they conflict, the **code is
> the ground truth** for what Stello actually does today; the spec is the stated
> design intent / north star. This gap is itself a finding for Ark.

---

## Findings

### Files (external, read-only)

| Path | Role |
| ---- | ---- |
| `docs/superpowers/specs/2026-05-17-shared-memory-design.md` | THE primary design spec for agent-level shared memory (index + lazy-load intent) |
| `docs/superpowers/specs/2026-05-16-decouple-main-session-design.md` | §6 orchestrator-facing SDK categories; §7.5 item 5 = shared-memory forward-reference |
| `packages/core/src/shared-memory/types.ts` | shipped `SharedMemoryEntry` + `SharedMemoryStore` interface |
| `packages/core/src/shared-memory/in-memory-shared-memory-store.ts` | default/test store: `Map` + writeLock |
| `packages/core/src/shared-memory/render-shared-memory.ts` | shipped index/context renderer |
| `packages/core/src/builtin-tools/memory-edit-tool.ts` | shipped **single** `stello_memory_edit` tool |
| `packages/core/src/agent/stello-agent.ts` | 4 flat SDK methods, injection wiring, `requireSharedMemory` guard |
| `packages/session/src/context-utils.ts` | `assembleSessionContext` — the actual injection-order site |
| `.claude/skills/scheduler-design/SKILL.md` | reflection-loop-is-app-layer stance + code sketch |
| `.claude/skills/orchestrator-usage/SKILL.md` | StelloAgent-as-facade, opinionated-but-injectable boundary |
| `docs/rfcs/consolidate-integrate-redesign.md` | KitKit relationship (SaaS built on stello) |
| `CLAUDE.md` | 编排层 / 外部注入点 architecture sections |

---

### 1. Shared-memory data model — "zero application-domain modeling"

**Spec intent** (`2026-05-17` §3.1): entry = `{ slug, summary, body }`.

```
SharedMemoryEntry { slug: string; summary: string; body: string }
```

**Shipped code** (`types.ts:5-8`): entry = `{ slug, body }` — **`summary` was
dropped**.

```ts
export interface SharedMemoryEntry {
  slug: string
  body: string
}
```

What both agree on (the durable design decisions):

- **No `type` / `tags`** — `2026-05-17` §3.1: *"无 `type` / `tags`：违背'零应用域建模'，分类需求由 agent 在 summary 里写前缀解决"*. Categorization is the agent's job via a prefix, not a schema field.
- **No `createdAt` / `updatedAt`** — *"不维护时间元数据，调用方需要时间感知应在 body 里自己写"*. Time awareness, if needed, goes inside `body`.
- **No nested KV / schema** — the legacy `MemoryEngine.core.json` point-path idea is explicitly *not* carried forward (§3.1).
- **`slug` = kebab-case primary key**, app/agent-chosen; framework does **not** validate the charset, only requires non-empty (§3.1; enforced at tool boundary `memory-edit-tool.ts:31-32`).
- **Insertion-order (FIFO) list; upsert keeps position.** `types.ts:14` JSDoc: *"list() 按'插入顺序'返回；upsert 已存在 slug 时**不改变其顺序位置**"*. Backed by JS `Map` (`in-memory-shared-memory-store.ts:10,31` — *"JS Map.set 在已有 key 上不改变插入位置"*).

Design principle 3 (`2026-05-17` §2): **"零应用域建模"** — shared memory solves exactly one problem (agent-scope, agent-writable, lazy detail) and refuses to model the application domain.

---

### 2. Index-injected + body-lazy-load — SPEC vs SHIPPED

This is the single biggest spec/code divergence.

**Spec intent** (`2026-05-17` §3.3, §4.2): inject only `slug + summary` lines every
send (re-rendered, never cached), fetch full `body` on demand via a
`stello_memory_recall` tool. The spec's index render format:

```
<shared_memory_index>
- prefer-concise: 用户偏好简短回答
- user-profile: 大三本科生 CS 专业
</shared_memory_index>

调用 stello_memory_recall 工具按 slug 查阅完整内容；
调用 stello_memory_remember / stello_memory_forget 工具维护此处条目。
```

**Shipped code** (`render-shared-memory.ts`): there is **no index and no lazy
load**. Every entry's **full `body`** is rendered into context on every send, one
`## {slug}` heading block per entry:

```ts
const HINT = `调用 stello_memory_edit 工具新增、修改或删除上面的条目。`
const blocks = entries.map(e => `## ${e.slug}\n${e.body}`).join('\n\n')
return `<shared_memory>\n${blocks}\n</shared_memory>\n\n${HINT}`
```

The tool's own description confirms the shift (`memory-edit-tool.ts:3-4`):
*"内容已直接在 `<shared_memory>` 段呈现给你，**无需另外查询**"* — i.e. there is
deliberately no recall round-trip in the shipped version.

What both versions keep:

- **Re-rendered every send, not cached.** `2026-05-17` §4.2: *"每次 send 都全量重新渲染并注入，不缓存（索引体积小，渲染开销可忽略）"*. Shipped: `sharedMemoryContextProvider: () => renderSharedMemoryContext(agent.sharedMemory)` is a thunk re-invoked per send (`stello-agent.ts:166`).
- **Not part of L3 / not compressed.** `2026-05-17` §4.3: it's system-segment content, never enters L3 history, never participates in history compression. Pulled fresh from the store each send.
- **Empty state = segment fully omitted.** `2026-05-17` §3.4 (*"entries 数组为空时，索引段（含 hint 文本）完全不注入"*) and code (`render-shared-memory.ts:14-15`: `if (entries.length === 0) return undefined`; caller skips the slot). Tools stay registered/available either way.

---

### 3. Injection position — above session_identity, below system prompt

**Spec intent** (`2026-05-17` §4.1) ordering:

```
[system prompt]
[shared_memory_index]      ← new slot
[session_identity]
[insight if present, consume]
[memory if present]
[L3 history with sanitize]
[user message]
```

**Shipped order** (`context-utils.ts:181`) — slightly richer because the
topology slot also landed:

```
systemPrompt → sharedMemoryContext → topologyContext → session_identity
→ insight → history → user
```

The *relative* placement the spec argues for is preserved: shared memory sits
**below system prompt, above session_identity**. The stated reasoning (§4.1):

- **Above `session_identity`**: *"shared memory 是 agent 范围共享认知，比'这个 session 是谁'更全局"* — agent-scope cognition is more global than "who is this session."
- **Below `system prompt`**: *"避免覆盖应用层固化指令"* — must not override the app's pinned system instructions.

**Strictly separated slots** (§4.1) — the three "memory-like" concepts never
share a slot:

| Slot | Scope | Lifecycle |
| ---- | ----- | --------- |
| `shared` (shared_memory) | agent-wide, all sessions | persistent, agent-writable |
| `memory` (per-session) | one session | persistent (and notably: **does not enter that session's own LLM context** — it's an external-view description, see CLAUDE.md 三个上下文槽位) |
| `insight` (per-session) | one session | **one-shot inbox**, consumed + cleared on send |

---

### 4. Builtin tool(s), factory+ctx opt-in, error-as-string, SDK, concurrency

**Tools — SPEC (3) vs SHIPPED (1).**

- Spec (§6.1): three tools — `stello_memory_recall(slug)`, `stello_memory_remember(slug, summary, body)`, `stello_memory_forget(slug)` — each its own factory (`memoryRecallTool()` etc.), **no `memoryToolSet()` bundle** (apps may want recall-only on some sessions).
- Shipped (`memory-edit-tool.ts`): **one** `memoryEditTool()` → tool name `stello_memory_edit`, with `delete?: boolean`. `delete: true` → remove; otherwise `body` required → upsert. No recall tool exists because the full body is already in context (§2). The `required: ['slug']` schema makes slug mandatory; `body` validated as required-when-not-deleting (`:42-44`).

**Factory + ctx opt-in pattern** (both): tools follow the existing
`createSessionTool()` / `activateSkillTool(skills)` convention — a factory returning
a registry entry, pulling the store off `ctx.agent.sharedMemory` at execute time
(`memory-edit-tool.ts:33`). The app **explicitly opts in** when building the
ToolRegistry; it is not auto-registered (CLAUDE.md 外部注入点 / 设计决策 #13).

**Error-as-string, not throw** (tool convention, `2026-05-17` §5.3/§6.3, code
`memory-edit-tool.ts:32,34,48`): the tool **returns** `{ success: false, error }`
strings — empty slug → `"slug is required and must be non-empty"`, missing store →
`"sharedMemory not configured"`, store throw → `` `failed: ${reason}` ``. The agent
decides whether to retry; **the conversation is not interrupted**.

**Four flat SDK methods on `StelloAgent`** (`stello-agent.ts:423-439`), names
matching the existing `putMemory` / `getSessionMetadata` style:

| Method | Returns |
| ------ | ------- |
| `listSharedMemory()` | `SharedMemoryEntry[]` (insertion order) |
| `getSharedMemoryEntry(slug)` | `SharedMemoryEntry \| null` |
| `upsertSharedMemoryEntry(slug, body)` | `Promise<void>` |
| `removeSharedMemoryEntry(slug)` | `Promise<void>` |

(Spec §7 listed `upsert` with a `summary` arg; shipped dropped it, consistent with
the `summary` removal in §1.) Unlike the tool, **SDK methods throw** synchronously.
The store-missing guard (`stello-agent.ts:413-419`) throws
`"sharedMemory not configured"`, mirroring the existing `requireStorage` pattern.
`StelloAgentConfig.sharedMemory?: SharedMemoryStore` is the optional injection point
(`:118`); omitting it = SDK throws, tool returns error string, slot not injected.

**Concurrency — writeLock, dirty reads allowed** (`2026-05-17` §5.2,
`in-memory-shared-memory-store.ts:11-18`): writes (`upsert`/`remove`) are serialized
through a per-store promise-chain `writeLock` (reusing the `SessionTree.writeLock`
idiom — *"认知成本零"*). Each write is RMW (read-all → modify → write-back), lock
guarantees atomicity. **Reads (`list`/`get`) take no lock — dirty reads permitted**
(§2 principle 5; `types.ts:15`). Deliberately **no** transaction/batch, no
fine-grained summary-only/body-only update, no rename (delete+add), no
subscribe/events — all marked YAGNI (§5.1, §7.2).

**No `FileSystemSharedMemoryStore`** is shipped (§8.2): like `SessionStorage`, the
on-disk strategy (per-entry `.md` / single JSON / SQLite) varies too much across
apps, so it's left to the application layer. §8.3 *suggests* (non-binding) a
`shared-memory/INDEX.md` + `entries/<slug>.md` layout.

---

### 5. Orchestrator-facing SDK — "framework owns data, app owns reflection"

The governing stance (CLAUDE.md 项目定位; `2026-05-16` §1.2, §6.2; scheduler-design
SKILL): **reflection / cross-session synthesis is the APPLICATION LAYER's job, NOT
the framework's.** The `2026-05-16` refactor *deleted* the old "Main Session" and its
`integrate()` LLM orchestration entirely; Stello retreated to "topology + single-
session dialogue + L2/L3 data layer." The framework exposes only **pure-data IO with
zero implicit LLM** (`2026-05-16` §6.2: *"零隐式 LLM 调用：所有方法都是数据 IO"*).

SDK categories (`2026-05-16` §6.1; live signatures in `stello-agent.ts` /
CLAUDE.md 编排层):

| Category | Method(s) | Note |
| -------- | --------- | ---- |
| Topology query | `getTopology` (forest), `getTopologyNode(id)`, `listRoots` | |
| Session listing | `listSessions(filter?)` → `SessionMeta[]` | |
| Single-session view | `getSessionMetadata(id)` → `{ memory, insight }` | `SessionMetadataView` |
| Batch digest view | `listSessionDigests(filter?)` → `SessionDigest[]` (`{id,label,status,memory,insight}`) | replaces old `getAllSessionL2s` |
| L3 read | `listMessages(id, opts?)` | |
| Single-session write | `putMemory` / `putInsight` / `clearInsight` | |
| Consolidate trigger | `consolidateSession(id)` | explicit; consolidateFn is app-injected, not framework LLM |

Key constraints (§6.2): every method is data IO (no implicit `send`/`integrate`);
all sessions treated alike (no root/child special-casing — caller infers from
topology); everything hangs flat off `StelloAgent` (no new top-level class);
storage-backend-agnostic. Notably `listSessionDigests` is **composed in the SDK**
from the two storage lines (`SessionTree.listAll()` × `SessionStorage.getMemory/
getInsight`) — storage needs no dedicated batch method (CLAUDE.md 存储设计).

**How an app builds a reflection loop on top** (scheduler-design SKILL, code
sketch):

```ts
async function reflect() {
  const digests = await agent.listSessionDigests({ status: 'active' })
  // call ANY LLM, parse with ANY schema ...
  for (const [id, content] of Object.entries(insightsByTarget)) {
    await agent.putInsight(id, content)   // targeted write-back to a session
  }
}
```

Loop = `listSessionDigests` → reflect with any LLM/tier/prompt/schema → `putInsight`
back into target sessions' one-shot inbox. The framework holds **no cross-session
state** and makes **no assumption about frequency/prompt/schema/LLM tier** — those
"强烈与应用业务耦合" so are deliberately pushed out (scheduler-design 设计决策).
Only auto-trigger kept in-framework: `consolidateEveryNTurns` (per-N-turns
consolidate), inlined in the Factory hook, fire-and-forget.

The `memory` slot ties the two halves together: per-session `memory` is *not*
injected into that session's own context — it's the **external-view description**
the reflection loop reads in bulk, then derives `insight` to write back. The session
itself is unaware of the loop (CLAUDE.md 关键不变量; 设计决策 #1, #16).

---

### 6. KitKit relationship

KitKit is a **SaaS application built ON stello** (`docs/rfcs/consolidate-integrate-
redesign.md:3,8`): *"KitKit 是基于 stello 的 SaaS 应用，每个 Space（Kit）对应一个
StelloAgent。"* The RFC is sourced from KitKit's production pain points and drove the
consolidate/integrate redesign — e.g. needing **per-session** (not per-agent)
consolidate prompts, and create-time-bound rather than call-time-passed
`consolidateFn`/`integrateFn`. KitKit is also the named example of the product-layer
that uses `topologyContextDecorator` to prepend its own `<space>` concept *without
stello knowing about it* (`stello-agent.ts:125-137`). It is a concrete instance of
"app owns reflection / app owns domain modeling" — the boundary §5 describes.

---

## Caveats / Not found

- **Spec ≠ implementation (load-bearing).** Shipped Stello has **no lazy-load index** and **no `summary` field** and **one** `stello_memory_edit` tool — not the spec's 3-tool recall/remember/forget index design. I verified by reading `types.ts`, `render-shared-memory.ts`, and `builtin-tools/` directly (not snippets). The `stello_memory_recall` tool referenced in the task brief exists **only in the spec**, not in the codebase. Treat the spec as design intent, the code as current behavior.
- I did not find a shipped `FileSystemSharedMemoryStore` (confirmed absent by spec §8.2 and by directory listing of `shared-memory/`). Only `InMemorySharedMemoryStore` exists.
- The `render-shared-memory.ts` HINT still references `stello_memory_edit` (singular), consistent with the shipped single-tool design.

---

## Why this matters for Ark

- **Index-vs-full-body is the core decision Ark already half-made differently.** Ark's `MEMORY.md` is exactly Stello's *spec* model: a slug+summary index (`- [slug](file.md) — one-line`) with bodies in separate files, loaded on demand. Stello *intended* this but *shipped* full-body injection. Ark's file-backed index/lazy-load is arguably the more disciplined version of Stello's own north star — worth noting Stello backed off it (likely because in-context recall beat a tool round-trip at their scale).
- **"Zero application-domain modeling" is a transferable constraint.** Stello's entry = `{slug, body}` with no type/tags/timestamps, categorization-by-prefix, is a sharp counterpoint to richer schemas. Ark's MEMORY.md entries are similarly flat (slug + one-liner). If Ark ever adds structure to memory, this is the explicit argument *against* doing so.
- **The slot-separation discipline maps onto Ark's layers.** Stello's strict 3-way split (agent-shared / per-session-persistent / per-session-one-shot) is a clean model for distinguishing Ark's global `MEMORY.md` (agent-shared) from per-task `research/` corpus (session-scoped persistent) from transient dispatch instructions (one-shot). Injection *position* matters: shared cognition above identity, below pinned system instructions.
- **"Framework owns data IO, app owns reflection (zero implicit LLM)" is a clean boundary for Ark's own tooling.** Stello refuses to run synthesis; it only exposes `listSessionDigests` → app reflects with any LLM → `putInsight` back. The analog: Ark's `ark context` / `ark agent` should stay pure-data (read/structure state), and any cross-task synthesis ("what did we learn across tasks") should be an agent-driven loop over those primitives, not baked into the binary.
- **Concurrency idiom is cheap to copy if Ark ever needs it.** Serialize-writes / allow-dirty-reads via a promise-chain writeLock is the minimal correct concurrency story for a single-process memory store — relevant if Ark's memory ever gets concurrent writers (parallel subagents writing research files), though Ark currently sidesteps this by making memory file-per-topic.
