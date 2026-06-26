# Research: Stello per-session context assembly & the three-slot memory model

- Query: Stello's per-session context assembly and the three-slot memory model — the heart of its memory design
- Scope: external (third-party reference, READ-ONLY at `reference/stello/stello/`)
- Date: 2026-06-25

## Findings

### Files (reference)
| Path | Role |
| ---- | ---- |
| `reference/stello/stello/CLAUDE.md` | Architecture charter — "三个上下文槽位" table + KEY invariant + confirmed design decisions |
| `reference/stello/stello/packages/session/src/context-utils.ts` | `assembleSessionContext` — the actual fixed assembly + compression logic |
| `reference/stello/stello/packages/session/src/create-session.ts` | `buildSession` — send/stream, insight consume, `consolidate()`, replay path |
| `reference/stello/stello/packages/session/src/types/storage.ts` | `SessionStorage` — the three slots + L3 + compression-cache methods |
| `reference/stello/stello/.claude/skills/session-usage/SKILL.md` | Slot semantics + cross-session communication model |
| `reference/stello/stello/.claude/skills/llm-call-sites/SKILL.md` | Message-array shape for send / compress / consolidate / reflection |
| `reference/stello/stello/.claude/skills/scheduler-design/SKILL.md` | Consolidate triggering (auto N-turns / manual) + fire-and-forget |
| `reference/stello/stello/docs/superpowers/specs/2026-05-16-decouple-main-session-design.md` | §4.5 assembly, §4.6 external data view — Main-Session deletion rationale |
| `reference/stello/stello/docs/rfcs/consolidate-integrate-redesign.md` | consolidateFn bound at create-time, fire-and-forget |

---

### 1. The three content slots

Each Session has three independent content slots in `SessionStorage`, plus L3 history (§4.4 spec; `storage.ts:33-48`). Slot semantics (CLAUDE.md:35-41, session-usage SKILL:38-46):

| Slot | Writer | Consumer | Lifecycle |
| ---- | ------ | -------- | --------- |
| `systemPrompt` | fork synthesis chain hardens it / app layer (`putSystemPrompt`) | `Session.send()` — injected **every** send | persistent (read each send) |
| `insight` | Orchestrator / app layer via `putInsight` | `Session.send()` — injected **once**, then `clearInsight` | one-shot inbox |
| `memory` (L2) | app layer via `consolidateFn` output / direct `putMemory` | **Orchestrator-facing reflection layer** (`listSessionDigests`) — **does NOT enter send context** | persistent (read externally) |

- **systemPrompt** — `getSystemPrompt(sessionId)`, one per session; in fork-compress scenarios it carries a `<parent_context>` block synthesized into the field (llm-call-sites SKILL:38-47).
- **insight** — `getInsight(sessionId)`; storage doc literally says "一次性，send 消费后调用 clearInsight" (`storage.ts:38`). Consume is wired in `create-session.ts:255-258`:
  > `if (assembled.insightConsumed) { await storage.clearInsight(currentMeta.id) }`
- **memory** — `getMemory/putMemory` (`storage.ts:45-48`), commented "原 L2 / 原 synthesis 统一槽位". Written by `consolidate()` (`create-session.ts:471-482`):
  > `const newMemory = await options.consolidateFn(currentMemory, messages); await storage.putMemory(currentMeta.id, newMemory)`

### 2. L3 = raw conversation history

L3 is the raw turn log, three storage methods (`storage.ts:26-31`):
> `appendRecord(sessionId, record)` / `listRecords(sessionId, options?)` / `trimRecords(sessionId, keepRecent)`

Each persisted record gets `{ turnId, turnSeq }` metadata (`create-session.ts:40-50`, `:302-305`). On read for prompt assembly, L3 is sanitized by `removeIncompleteToolCallGroups` — incomplete tool-call groups (assistant emitted toolCalls but the matching `tool` result never landed, e.g. abort/crash) are dropped wholesale to keep the prompt protocol-legal for OpenAI/Anthropic (`context-utils.ts:17-46`, `:233`). `trimRecords` is exposed on the Session API (`create-session.ts:484-492`), keeping only the most recent N.

### 3. The FIXED context assembly order

It is a **fixed rule with NO assembler extension hook** (CLAUDE.md design decision #7: "Session 上下文组装为固定规则，不暴露 assembler 扩展点").

Canonical documented order (session-usage SKILL:22-24, spec §4.5, CLAUDE.md:46):
```
system prompt → session_identity(label) → insight(若有，消费后清除) → L3 历史 → 当前用户消息
```

**Live source has evolved past the doc** — `assembleSessionContext` (`context-utils.ts:181`, `:195-249`) interleaves two **agent-level** slots rendered externally and passed via `SessionSendOptions`:
```
systemPrompt → sharedMemoryContext → topologyContext → session_identity → insight → history → user
```
Order in code (`context-utils.ts:198-233`): 1. system prompt, 2. `sharedMemoryContext` (agent-level shared memory, externally rendered), 3. `topologyContext` (you-are-here marker), 4. `session_identity(label)` via `buildSessionIdentityMessages`, 5. `insight` (sets `insightConsumed=true`), then sanitized L3 `history`, then the `user` message. `sharedMemoryContext`/`topologyContext` are skipped when empty/undefined. This realizes the "StelloAgent-level shared memory" future slot flagged as undecided in spec §7.5 item 5 — it is the agent-shared memory index inserted above system prompt, NOT the per-session `memory` slot.

**Per-session `memory` is still absent from this normal send path.** The only place `getMemory` is injected is `assembleSessionReplayContext` (`create-session.ts:119-122`), used solely for tool-result continuation replay — not the user-facing turn. So the invariant "memory does not enter the session's own LLM context" holds for the conversational `send()`/`stream()` path.

### 4. The central invariant — memory is outward-facing

CLAUDE.md:41 (verbatim):
> **关键不变量**：`memory` 不进入 Session 自身的 LLM 上下文。它是面向外部视角的描述——上层批量收集所有 Session 的 memory 做反思、规划、调度，再通过 `putInsight` 把派生的洞察定向回写给目标 Session。Session 自身不感知这个回路。

The loop (session-usage SKILL:52-65, scheduler-design SKILL:44-56):
```
all Sessions' memory ──┐
                       ├─→ app-layer reflection (any LLM) ──→ putInsight(targetId, content)
                       ┘
(StelloAgent.listSessionDigests fetches {id, label, memory, insight}[] in one call)
```
- The reflection layer is **app-implemented** — any frequency, any LLM, any schema (spec §6.2 "零隐式 LLM 调用"; scheduler-design SKILL:60-64). The framework holds **no cross-session state** (CLAUDE.md decision #16).
- memory is the **read side** of the loop (external layer consumes it); insight is the **write side** that closes it (external layer writes a targeted insight back). The Session reads insight on its next send but never reads its own memory and is "unaware of this loop" (CLAUDE.md decision #1).
- External data view is uniform regardless of storage split (spec §4.6): `SessionMetadataView { memory: string|null /*持久*/, insight: string|null /*一次性*/ }`.

### 5. Compression + consolidate scheduling

- **Compression** fires when estimated tokens cross `maxContextTokens * 0.8` (`COMPRESS_THRESHOLD = 0.8`, `context-utils.ts:144`, `:241`). `compressFn` summarizes the history head into one `system` summary, concatenated with recent L3 selected by `selectHistoryByBudget` (which never truncates mid tool-call group), then `[...prefix, summaryMessage, ...finalRecent, userMessage]` (`context-utils.ts:300-313`). Result is cached as `CompressionCache {summary, compressedCount}` so each `send()` does not re-call `compressFn`; cache is hydrated/flushed fire-and-forget via optional `getCompressionCache`/`putCompressionCache` (`context-utils.ts:324-356`, `create-session.ts:195-225`). Token estimate is chars/4 (`context-utils.ts:78-80`).
- **consolidate (L3 → L2 memory)** is **fire-and-forget** — "不阻塞对话" (CLAUDE.md decision #5; Engine fire-and-forget at CLAUDE.md:93). Triggers (scheduler-design SKILL): auto via `orchestration.consolidateEveryNTurns: N` (the only framework auto-policy, inlined in a Factory hook, no separate scheduler) or manual `agent.consolidateSession(sessionId)`.
- **consolidateFn bound at session create-time, not call-time** (RFC consolidate-integrate-redesign §1; `create-session.ts:475-481` uses `options.consolidateFn`). Passed through `Create/LoadSessionOptions.consolidateFn`; `Session.consolidate()` takes no fn argument. Removes the call-site routing burden and enables per-session consolidate prompts.
- **App layer picks the LLM tier** — `ConsolidateFn`/`CompressFn` do NOT receive an injected LLM; the app supplies the tier via closure (CLAUDE.md decisions #12, #1; spec §4 retained list). Framework is "对 memory 内容格式完全无感知" (CLAUDE.md:148) — whatever format `consolidateFn` emits is what the reflection loop consumes. Consolidate output target ~100-150 chars summary written back to the session's memory slot (llm-call-sites SKILL:85-86).

### 6. insight semantics

- **Replace, not append** (CLAUDE.md decision #3 "insights 替换策略（不追加）— 写入即覆盖上一次"; session-usage SKILL:62 "重复 reflect 不会累积"). `putMemory` is likewise replace-not-append (session-usage SKILL:64).
- **One-shot consume** — injected once into the next send, then `clearInsight` (`storage.ts:38`, `create-session.ts:255-258`). `Session.insight()` getter reads without consuming; only send consumes (spec §4.2).
- **Immutable one-time injection** — callbacks injected once as immutable config (CLAUDE.md decision #4 "回调一次性注入（immutable config）").

---

## Why this matters for Ark

- **Scoped, per-session memory vs. Ark's flat auto-memory.** Stello gives *each* session its own `memory` slot whose content is deliberately walled off from that session's own prompt; Ark's auto-memory (`~/.claude/.../memory/MEMORY.md`) is one flat, self-injected pool. The Stello split (read-by-others vs. injected-to-self) is the design lever worth studying if Ark wants per-task memory that doesn't pollute every session's context.
- **The memory/insight read-write split is the key invariant.** Memory = outward-facing description an external reflection layer reads; insight = the targeted, one-shot write-back that closes the loop. Ark currently has no equivalent "reflect across tasks, write a targeted note back to one task" channel — memory and self-context are the same surface.
- **Fixed assembly order, no extension hook, is a deliberate constraint.** Stello refuses an assembler plugin point (decision #7) and instead grows fixed agent-level slots (`sharedMemoryContext`, `topologyContext`) in code. If Ark formalizes context assembly, the lesson is to enumerate slots explicitly rather than expose a generic hook.
- **Fire-and-forget consolidation + create-time fn binding** keeps the conversational turn cheap and lets the app pick a cheaper LLM tier for summarization — a pattern Ark could mirror for any background "summarize task into memory" step.

## Caveats / Not found
- `packages/core/src/types/memory.ts` does **not** exist (no `memory.ts` under `packages/core/src/types/`); the memory contract lives entirely in `packages/session/src/types/storage.ts` (`getMemory`/`putMemory`) plus the SDK-level `listSessionDigests` aggregation described in CLAUDE.md/spec — there is no dedicated core memory-type module.
- The doc-vs-source drift on assembly order (agent-level `sharedMemoryContext`/`topologyContext` slots present in `context-utils.ts` but absent from CLAUDE.md/session-usage SKILL/spec §4.5) is real and called out in §3; the per-session `memory` invariant is unaffected. `getMemory` injection exists only in the replay path (`create-session.ts:119-122`), not the normal turn.
