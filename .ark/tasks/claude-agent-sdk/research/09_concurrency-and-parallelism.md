# Research: Claude Agent SDK concurrency and parallelism

- Query: can one host process run N SDK sessions/`query()` calls concurrently? Isolation between concurrent sessions; cwd collision; subprocess model and resource cost; thread safety; fan-out/join pattern; rate-limit/backpressure interaction; cancelling one among many. The host-orchestrated parallelism that in-session subagents (topic 06) do NOT provide.
- Scope: external (primary: `code.claude.com` Agent SDK docs + published SDK source on GitHub; GitHub issues for behavior the docs omit).
- Date: 2026-05-25

## Version snapshot

| Surface | Version | Source |
| ------- | ------- | ------ |
| Python `claude-agent-sdk` | **0.2.87** (bundles Claude CLI **2.1.150**) | PyPI / `anthropics/claude-agent-sdk-python` CHANGELOG, release `v0.2.87` published 2026-05-23 |
| TypeScript `@anthropic-ai/claude-agent-sdk` | **0.3.150** | npm / `anthropics/claude-agent-sdk-typescript` release `v0.3.150` published 2026-05-23 |
| Docs | `code.claude.com/docs/en/agent-sdk/*` | `streaming-vs-single-mode`, `hosting`, `python`, `typescript`, fetched 2026-05-25 |

**Newer than snapshot:** none. `v0.2.87` (Py) and `v0.3.150` (TS) are the latest releases at fetch time (both 2026-05-23). Nothing past them observed.

**Confidence labelling used throughout:** `[DOC]` = stated in docs; `[SRC]` = read from SDK source; `[ISSUE]` = from a GitHub issue (community-reported, may be unconfirmed by Anthropic); `[INFERRED]` = deduced from architecture, not stated. **The official docs are explicitly silent on multi-session concurrency, thread safety, and `asyncio.gather` — confirmed by direct search of the Python reference, the TS reference, and the streaming-vs-single-mode page (all returned "no mention").** Most of this file is therefore `[SRC]` + `[INFERRED]` + `[ISSUE]`, flagged inline.

**Cross-references (do not duplicate):**
- **Topic 01** — TS/Py both bundle a native Claude CLI binary; each `query()` spawns it as a subprocess. This file owns the *per-concurrent-session* process cost.
- **Topic 02 `02_sessions.md`** — cwd → on-disk project key (`~/.claude/projects/<encoded-cwd>/`), machine-local sessions, `SessionStore`. §3 here builds the cwd-collision argument on top of that.
- **Topic 03 `03_streaming-events.md`** — async-iterator consumption (no sync API), `interrupt()` / `AbortController` mechanics, `SDKRateLimitInfo`/`RateLimitEvent` (typed-but-undocumented). §7–§8 here reuse those mechanisms in a multi-session context.
- **Topic 06 `06_subagents.md`** — in-session subagent fan-out is **model-driven, depth-1, text-only return**. This file is the OTHER fan-out axis: **host-orchestrated parallel `query()` calls**. §6 here is the answer to topic 06's repeated "topic 09 territory" pointers.

---

## TL;DR

1. **Yes — N concurrent `query()` calls in one process work**, both Python (`asyncio.gather` over N async iterators) and Node (`Promise.all` over N async generators). This is **not documented** but is structurally sound: each `query()` builds a fresh `InternalClient()` and a fresh subprocess transport, with **no shared client, cache, or singleton** `[SRC]`. The hosting docs implicitly bless it: "running *multiple* Claude Agent processes inside of the container" `[DOC]`.
2. **Isolation is per-`query()` at the SDK-object level, but NOT at the filesystem level.** Concurrent sessions share three host-global surfaces: the **`~/.claude/` config dir** (state leaks across calls — `[ISSUE]` #952), the **module-level `_ACTIVE_CHILDREN` subprocess registry** (cleanup only, benign) `[SRC]`, and **the cwd / git working tree** if you point two at the same directory `[DOC]`.
3. **cwd collision = silent file-write race.** The SDK does NOT lock or coordinate file writes between two sessions in the same directory. Two sessions sharing a cwd also share an on-disk project key and can clobber each other's edits. This is the SDK-level fact behind Ark's worktree-per-task design (RFC Q6). `[DOC]` + `[INFERRED]`
4. **Subprocess model: one CLI subprocess per concurrent `query()`. No shared daemon.** `[SRC]` Cost ≈ one OS process + the CLI's RAM per live session; the hosting docs recommend **~1 GiB RAM / 1 CPU per SDK instance** `[DOC]`. Practical ceiling is RAM/CPU and API rate limits, not a documented cap.
5. **Thread safety: docs silent; async-within-one-event-loop is the only supported model `[INFERRED]`.** The SDK is built on `anyio`/`asyncio`; the safe concurrency unit is **many `query()` coroutines on one event loop**, not many OS threads. Per-process global state (`~/.claude/`, `_ACTIVE_CHILDREN`) means cross-thread use is unvalidated. Use a process-per-isolation-domain or one-event-loop-many-tasks model.
6. **Fan-out/join = `asyncio.gather` (Py) / `Promise.all` (TS) over independent `query()` calls, each with its own cwd.** This is the deterministic fan-out that subagents (topic 06) do not give. §6 has the snippet.
7. **Rate limits: the SDK does NOT queue/throttle concurrent sessions, and (Python 0.2.87) does NOT retry 429 — it crashes** `[ISSUE]` #812 (open). A community PR (#973, open, unmerged) would add 429+`Retry-After`+exponential backoff. The host must cap concurrency (semaphore) and handle 429 itself today.
8. **Cancel one among many: `Query.interrupt()` / `controller.abort()` (TS) or `ClaudeSDKClient.interrupt()` / `asyncio.Task.cancel()` (Py) — each is scoped to ONE session**, so aborting one leaves siblings running. Known quirks: early-abort corrupts resume (topic 03), and serial back-to-back `query()` in one loop hit a cleanup bug (#890, fixed). `[DOC]` + `[ISSUE]`

---

## 1. Multiple concurrent `query()` calls in one process

### 1.1 Is it supported?

**Docs: silent.** Direct searches of the Python reference, the TS reference, and `streaming-vs-single-mode` for "concurrent / parallel / `asyncio.gather` / thread / multiple queries" all return **no mention** (verified 2026-05-25). There is no documented support statement and no documented warning against it.

**Source: structurally yes.** `[SRC]` Two facts from the Python SDK make concurrent `query()` calls sound:

- `query()` constructs a **fresh `InternalClient()` per call** — not a module singleton. (Topic 03 already noted this; the `query.py` body is `client = InternalClient()` then `client.process_query(...)`.)
- `InternalClient.process_query` creates a **fresh transport per call** when none is supplied:
  > `if transport is not None: chosen_transport = transport else: chosen_transport = SubprocessCLITransport(prompt=prompt, options=configured_options)`
  and there is **no module-level mutable state, singleton, or shared cache** in `client.py` `[SRC]`.

So two `query()` coroutines do not contend on any in-process SDK object. They contend only on host-global surfaces (§2).

**Docs: implicitly blessed by the hosting page.** `[DOC]` `code.claude.com/docs/en/agent-sdk/hosting`, Pattern 2 ("Long-Running Sessions"):
> "Maintain persistent container instances for long running tasks. Often times running *multiple* Claude Agent processes inside of the container based on demand."

and Pattern 4 ("Single Containers"):
> "Run multiple Claude Agent SDK processes in one global container. … This is likely the least popular pattern because you will have to prevent agents from overwriting each other."

That last clause is the SDK's own acknowledgement of the collision risk this file documents (§2–§3).

### 1.2 Minimal `asyncio.gather` parallel-query snippet (Python)

```python
import asyncio
from claude_agent_sdk import query, ClaudeAgentOptions, ResultMessage

async def run_one(prompt: str, cwd: str) -> str | None:
    result = None
    async for message in query(
        prompt=prompt,
        options=ClaudeAgentOptions(cwd=cwd, allowed_tools=["Read", "Grep", "Glob"]),
    ):
        if isinstance(message, ResultMessage):
            result = message.result
    return result

async def main():
    # Two queries run concurrently on one event loop; distinct cwds = no file race (§3).
    a, b = await asyncio.gather(
        run_one("Summarize auth.py", "/repo/worktree-a"),
        run_one("Summarize db.py",   "/repo/worktree-b"),
    )
    print(a, b)

asyncio.run(main())
```

Each `run_one` drives its own async iterator to its `ResultMessage`; `asyncio.gather` interleaves them on the single event loop while each awaits its subprocess. **Note the distinct `cwd` per call** — this is the host-added isolation §3 argues is mandatory.

### 1.3 Node / TypeScript equivalent

`query()` returns an `AsyncGenerator`; run N concurrently with `Promise.all` over consumer functions. `[INFERRED]` from the async-generator shape; docs silent.

```typescript
import { query, type Options } from "@anthropic-ai/claude-agent-sdk";

async function runOne(prompt: string, cwd: string): Promise<string | undefined> {
  let result: string | undefined;
  for await (const message of query({ prompt, options: { cwd, allowedTools: ["Read", "Grep", "Glob"] } })) {
    if (message.type === "result" && message.subtype === "success") result = message.result;
  }
  return result;
}

const [a, b] = await Promise.all([
  runOne("Summarize auth.ts", "/repo/worktree-a"),
  runOne("Summarize db.ts",   "/repo/worktree-b"),
]);
```

Node is single-threaded with an event loop; concurrency here is cooperative I/O multiplexing over two child processes, same shape as the Python case.

### 1.4 Third-party confirmation (not primary)

Community guides describe exactly this pattern: "`asyncio.gather()` runs independent agent conversations in parallel. While one request awaits the network response, the event loop processes others" (Augment Code, CodeSignal). These are **secondary** sources; cited only as corroboration that the gather pattern is the de-facto approach. The authoritative facts are the per-`query()` fresh-client/fresh-transport source reads above.

---

## 2. Isolation between concurrent sessions — what is shared

Each `query()` is independent **at the SDK-object level** (§1.1). But concurrent sessions in one process share **three host-global surfaces**. This is the load-bearing section for "what could leak / collide."

### 2.1 Shared: the `~/.claude/` config dir and `~/.claude.json` — STATE LEAKS `[ISSUE]`

Sessions, the global config (`~/.claude.json`), and auto-memory all live under one config home (`~/.claude`, overridable per call via `CLAUDE_CONFIG_DIR` in `options.env` — topic 02 §8). Because this directory is **process-global by default**, state written by one `query()` is visible to the next.

- **Confirmed leak across calls in one process:** `[ISSUE]` [python#952](https://github.com/anthropics/claude-agent-sdk-python/issues/952) (open) — the bundled CLI honours an inbound `TRACEPARENT` (OTel trace context) **only on the first `query()` in the process's lifetime**; second-and-later calls silently orphan their spans. Root cause: a `firstStartTime` / migration marker in `~/.claude.json` written on first run. Verbatim:
  > "Wiping that directory between calls restores correct nesting; leaving it reproduces the bug 100% of the time."
  Workaround in the issue: per-call `HOME=/tmp/agent-cli-<uuid>` in `ClaudeAgentOptions.env`.
- **Implication for concurrent (not just serial) sessions** `[INFERRED]`: if a `firstStartTime`-style marker can leak serially, two **concurrent** sessions sharing `~/.claude/` race on the same files. The SDK provides no per-session config isolation by default. To isolate, give each session its own config home: `env={"CLAUDE_CONFIG_DIR": "/tmp/cfg-<id>"}` (or `HOME` per the issue's workaround).
- Topic 02 §8 already flags the multi-tenant warning verbatim: "Do not rely on default `query()` options for multi-tenant isolation. … run each tenant in its own filesystem and set `settingSources: []` plus `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`." That warning is about *settings*; #952 shows the same shared-dir hazard extends to *CLI runtime state*.

### 2.2 Shared: `_ACTIVE_CHILDREN` module-global — benign (cleanup only) `[SRC]`

The Python subprocess transport keeps a **module-level mutable set** of live child processes for atexit cleanup. Verbatim:

```python
_ACTIVE_CHILDREN: set[Process] = set()

def _kill_active_children() -> None:
    for p in list(_ACTIVE_CHILDREN):
        with suppress(Exception):
            p.send_signal(signal.SIGTERM)
    _ACTIVE_CHILDREN.clear()

atexit.register(_kill_active_children)
```

This *is* shared across all concurrent `query()` calls in the process, but it is a **cleanup registry**, not session state — each transport adds/removes its own `Process`. The collision risk is at process exit only: `_kill_active_children` SIGTERMs **every** live child, so a process-wide `atexit` (or an uncaught fatal) tears down **all** concurrent sessions at once, not selectively. `[INFERRED]` There is no per-session opt-out of this global teardown. For graceful per-session shutdown, cancel/close each session explicitly before exit (§8) rather than relying on atexit.

### 2.3 Shared: nothing else in-process `[SRC]`

`client.py` has "no module-level mutable state or singletons" and "no global caches"; the write-lock is **per-transport-instance** (`self._write_lock: anyio.Lock = anyio.Lock()`), so it serialises stdin writes *within one session*, not across sessions. There is no shared settings cache, no shared HTTP client, no shared event loop object beyond the one the caller provides.

### 2.4 Summary table

| Surface | Shared across concurrent `query()`? | Risk | Host mitigation |
| --- | --- | --- | --- |
| `InternalClient` / transport | No — fresh per call `[SRC]` | none | — |
| `self._write_lock` (stdin) | No — per transport `[SRC]` | none | — |
| `_ACTIVE_CHILDREN` + atexit | **Yes** (process-global) `[SRC]` | atexit kills ALL sessions together | explicit per-session close before exit |
| `~/.claude/` config + `~/.claude.json` | **Yes** (process-global by default) `[ISSUE]` #952 | state/marker leak & race | per-session `CLAUDE_CONFIG_DIR`/`HOME` in `env` |
| cwd / git working tree | **Yes if you point two at the same dir** `[DOC]` | file-write race (§3) | per-session cwd / worktree |
| API rate-limit budget (org-level) | **Yes** (shared at Anthropic) `[ISSUE]` | 429 cascade (§7) | host-side semaphore + backoff |

---

## 3. cwd collision — the file-write race

**The SDK does not prevent concurrent sessions from racing on the filesystem.** Topic 02 established that cwd is structurally load-bearing: it sets the on-disk project key (`~/.claude/projects/<encoded-cwd>/`), scopes settings discovery, and scopes session listing. Two concurrent sessions with the **same `cwd`**:

1. **Share the same on-disk project directory** `[INFERRED from topic 02 §5]` — both write `*.jsonl` into `~/.claude/projects/<same-encoded-cwd>/`. Session files are keyed by distinct UUIDs so the JSONL files themselves don't collide, but `continue: true` / `continue_conversation=True` ("most recent session in the directory") becomes ambiguous — two concurrent sessions racing to be "most recent."
2. **Share the same git working tree and files** — and the SDK does nothing to serialise edits. If both sessions run `Edit`/`Write`/`Bash` against overlapping files, the writes interleave at the OS level with **no SDK-level lock, transaction, or conflict detection.** The docs admit this for the analogous fork case (topic 02 §3, verbatim): "If a forked agent edits files, those changes are real and visible to any session working in the same directory." The hosting Pattern 4 caveat says it directly: "you will have to prevent agents from overwriting each other." `[DOC]`

**Confirmation from the field** `[ISSUE]`: community guidance (CodeSignal) states "`asyncio.gather()` alone cannot prevent merge conflicts… When multiple agents modify overlapping files, prompt-level coordination breaks down quickly." Secondary, but consistent with the source-level fact that no SDK lock exists.

### Implication for a substrate fanning out sibling tasks (RFC Q6)

`[INFERRED]` — this is the SDK-level basis for Ark's design:

- The SDK gives **zero file isolation** between concurrent same-cwd sessions. Any substrate that fans out N sibling tasks that *write* must give each task its own filesystem view, or accept races.
- The clean mechanism is **a distinct `cwd` per concurrent session**, e.g. **one git worktree per task** (Ark's existing `worktree` feature, `specs/features/worktree/SPEC.md`). Worktree-per-task gives each session (a) an isolated working tree → no file-write race, and (b) a distinct on-disk project key → no `continue` ambiguity and clean per-task session storage.
- Read-only fan-out (parallel analysis with no writes) can safely share a cwd; **write fan-out cannot.** A reviewer + verifier dispatched in parallel against the same checkout are read-mostly, but if either writes (e.g. a verifier that runs a formatter, or a reviewer that edits), they need separate worktrees.
- This is exactly the gap the SDK does not close and the substrate must: **the SDK won't prevent file-write races; the host adds worktrees/separate cwds.**

---

## 4. Subprocess model and resource cost

### 4.1 One subprocess per concurrent `query()` — no shared daemon `[SRC]` `[DOC]`

`[SRC]` Each `SubprocessCLITransport` instance spawns **its own** CLI subprocess via `anyio.open_process(cmd, stdin=PIPE, stdout=PIPE, …, cwd=self._cwd, env=process_env)`. There is **no shared daemon and no global subprocess** — "each transport maintains its own `_process` instance variable." Since each `query()` builds a fresh transport (§1.1), **N concurrent `query()` calls ⇒ N live CLI subprocesses.**

`[DOC]` The hosting page confirms the per-instance native binary: "Both SDK packages bundle a native Claude Code binary for the host platform, so no separate Claude Code or Node.js install is needed for the spawned CLI." (Topic 01 owns the bundling detail.)

CLI binary resolution order `[SRC]`: (1) bundled platform binary, (2) `shutil.which("claude")` on PATH, (3) hardcoded common install paths; else `CLINotFoundError`.

### 4.2 Resource cost per concurrent session `[DOC]`

The hosting page's "System Requirements / Each SDK instance requires" gives the budgeting figure:
> "Recommended: 1GiB RAM, 5GiB of disk, and 1 CPU (vary this based on your task as needed)."

So a rough planning rule `[INFERRED from that]`: **~1 GiB RAM and ~1 CPU per concurrent live session** is the SDK's own recommendation, plus one OS process each. Cost note from the FAQ: "The dominant cost of serving agents is the tokens; … a minimum cost is roughly 5 cents per hour" per container.

### 4.3 Practical ceiling on parallel sessions per host

`[INFERRED]` — **no documented hard cap.** The ceiling is the binding of three limits:
1. **RAM/CPU** — at ~1 GiB/1 CPU recommended each, a 16 GiB / 8-core host comfortably holds well under ~16 concurrent sessions before contention.
2. **OS process / file-descriptor limits** — N subprocesses each with stdin/stdout/stderr pipes.
3. **API rate limits** — frequently the *real* ceiling (§7); the org-level token/request budget is shared across all sessions and is hit long before RAM on a small tier.

### 4.4 Reducing per-session startup cost (TS only) `[DOC]`

`[DOC]` TS exposes `startup(params?) → Promise<WarmQuery>` to amortise the spawn+initialize handshake:
> "Pre-warms the CLI subprocess by spawning it and completing the initialize handshake before a prompt is available. … so the first `query()` call resolves without paying subprocess spawn and initialization cost inline."

`WarmQuery extends AsyncDisposable` with `.query(prompt) → Query` and `.close()`. **This is one warm subprocess per `WarmQuery` handle** — it lowers latency, not the per-session process count. **No Python equivalent** appears on the Python reference (Python pays spawn cost inline each call). For pooling N warm sessions you create N `WarmQuery` handles.

---

## 5. Thread safety

**Docs: silent.** `[DOC-SILENT]` No thread-safety statement on the Python reference, TS reference, or streaming-vs-single-mode page (verified). There is no "the SDK is thread-safe" or "not thread-safe" claim anywhere fetched.

**Inferred from architecture: async-within-one-event-loop is the only supported concurrency model.** `[INFERRED]`

- The Python SDK is built on **`anyio`** over `asyncio` (the transport uses `anyio.open_process`, `anyio.Lock`, `anyio.create_task_group`) `[SRC]`. The intended unit of concurrency is **multiple coroutines on one event loop**, driven by `asyncio.run` / `anyio.run`. The community guidance explicitly says to prefer `anyio.run()` because the async layer is anyio.
- **GIL implication (Python)** `[INFERRED]`: even if the SDK were thread-safe, OS threads buy little here — the agent's real work is in the **subprocess** (the CLI) and in **network I/O to the API**, both of which release the GIL or run out-of-process. The natural parallelism is therefore I/O-bound concurrency on one event loop (each `query()` awaits its subprocess), not CPU threads. `asyncio.gather` over N `query()` calls already achieves true wall-clock parallelism because the N subprocesses run as separate OS processes; the host event loop merely multiplexes their I/O.
- **Per-process global state argues against multi-threaded use** `[SRC]`+`[INFERRED]`: `_ACTIVE_CHILDREN` (a plain `set`, no documented lock around add/remove across threads) and the shared `~/.claude/` dir (#952) are process-global. Mutating them from multiple OS threads is **unvalidated** by the SDK and not covered by any documented guarantee. Treat the SDK as **single-event-loop, single-thread per event loop.**
- **Cross-event-loop / serial caveat** `[ISSUE]`: even *sequential* `query()` calls in one event loop hit cleanup bugs — [python#890](https://github.com/anthropics/claude-agent-sdk-python/issues/890) (closed): a second `query()` after the first closes can surface `BaseExceptionGroup[CancelledError]` from the stderr task group's `__aexit__` because `suppress(Exception)` doesn't catch a `BaseException`-derived group. Related cluster: [#810](https://github.com/anthropics/claude-agent-sdk-typescript) (closed), [#746](https://github.com/anthropics/claude-agent-sdk-python/pull/746) (the partial fix), [#454/#776]. The async-cleanup path is the SDK's most fragile area — see also §8 and the docs' own warning below.

**The one documented async-hygiene rule** `[DOC]` (Python reference, `ClaudeSDKClient`):
> "When iterating over messages, avoid using `break` to exit early as this can cause asyncio cleanup issues. Instead, let the iteration complete naturally or use flags to track when you've found what you need."

This is the closest the docs come to a concurrency caveat: the SDK relies on the async generator running to completion for clean teardown. Breaking early (common in a `gather` where you want first-result-wins) risks the cleanup bugs above. `[INFERRED]` For fan-out where you cancel losers, prefer explicit `interrupt()`/cancel (§8) over a bare `break`.

**Bottom line:** docs silent on thread safety; inferred from the anyio/asyncio architecture and process-global state that the supported model is **many `query()` coroutines on one event loop**, optionally backed by N subprocesses for true parallelism — **not** many OS threads sharing one SDK. For thread/process isolation beyond one event loop, run a **separate OS process** (the hosting Pattern 1/2 container model) rather than threading inside one interpreter.

---

## 6. Fan-out / join pattern (the real host-orchestrated parallelism)

Topic 06 established that in-session subagents are **model-driven, depth-1, and return text only** — so "dispatch 3 independent sub-tasks, wait for all, integrate" is **not** reliably done with subagents. The deterministic mechanism is **host-orchestrated parallel `query()` calls** — exactly the "topic 09 territory" topic 06 deferred to.

### 6.1 Shape (Python)

```python
import asyncio
from claude_agent_sdk import query, ClaudeAgentOptions, ResultMessage

async def subtask(prompt: str, cwd: str) -> str | None:
    out = None
    async for m in query(prompt=prompt, options=ClaudeAgentOptions(cwd=cwd)):
        if isinstance(m, ResultMessage):
            out = m.result
    return out

async def fan_out_join(tasks: list[tuple[str, str]]) -> list[str | None]:
    # Each (prompt, cwd) is an isolated session. gather = dispatch-all + wait-all.
    return await asyncio.gather(*(subtask(p, cwd) for p, cwd in tasks))

# integrate host-side:
results = asyncio.run(fan_out_join([
    ("Review module A for security", "/repo/wt-a"),
    ("Review module B for security", "/repo/wt-b"),
    ("Review module C for security", "/repo/wt-c"),
]))
# results[i] is subtask i's ResultMessage.result — structured by the host, not the model.
```

### 6.2 Why this beats subagent fan-out (cross-ref topic 06 §8)

| Property | Host `gather` over `query()` (this file) | In-session subagent fan-out (topic 06) |
| --- | --- | --- |
| Dispatch determinism | host picks exactly N tasks `[INFERRED]` | model decides degree `[DOC]` |
| Degree control | host-controlled (semaphore) | non-deterministic `[DOC]` |
| Per-task isolated context | yes — separate session each | results bloat back into one parent context `[DOC]` |
| Result shape | full `ResultMessage` per task (incl. `structured_output`) `[DOC]` | free-form text only `[DOC]` |
| Per-task cwd/worktree | host sets per call (§3) | all share parent cwd `[DOC]` |
| Per-task cost/observability | own `session_id`, own `total_cost_usd` `[DOC]` | folded into parent `[DOC]` |
| Recursion | host orchestrates any depth | depth 1, cannot recurse `[DOC]` |

So for **parallel reviewer + verifier dispatch** (the RFC's parallel-review case): run them as **separate concurrent `query()` calls**, each in its own worktree, then join their `ResultMessage`s host-side. The model-driven subagent path gives neither the determinism nor the per-role structured return.

### 6.3 First-result-wins / bounded fan-out

`[INFERRED]` For "take the first to finish, cancel the rest," use `asyncio.wait(..., return_when=FIRST_COMPLETED)` then cancel pending tasks (§8). For "at most K concurrent," wrap each `subtask` in an `asyncio.Semaphore(K)` — also the §7 rate-limit lever. Avoid bare `break` inside a `query()` iterator (the §5 cleanup-bug warning); cancel the outer task instead.

---

## 7. Rate limits and backpressure

### 7.1 Does the SDK queue / throttle concurrent sessions? No. `[INFERRED]`+`[ISSUE]`

There is **no documented concurrency-vs-rate-limit guidance** and **no built-in throttle, queue, or admission control** across concurrent sessions. Each `query()` drives its subprocess independently; the org-level API rate-limit budget is **shared at Anthropic** across all of them (§2.4). N concurrent sessions consume the shared token/request budget N-ways with no SDK coordination.

### 7.2 Does the SDK retry 429? Python 0.2.87: NO — it crashes. `[ISSUE]`

`[ISSUE]` [python#812](https://github.com/anthropics/claude-agent-sdk-python/issues/812) (**open**), verbatim title: "Agent SDK should handle 429 rate limits gracefully instead of crashing." The report:
> "The SDK … crashes fatally when the API returns a 429 rate limit error … treats the 429 as a fatal exception rather than backing off and retrying. … multi-turn autonomous agent sessions that accumulate 10+ minutes of work are destroyed by a single rate limit hit."

The surfaced error is a subprocess crash (`Command failed with exit code 1`) with the 429 body in stderr — i.e. the caller's `try/except` around `query()` often can't intercept it cleanly because it originates in the subprocess. A fix PR exists but is **unmerged**: [python#973](https://github.com/anthropics/claude-agent-sdk-python/issues/973) (open) — "SDK now catches 429 rate limit errors, reads Retry-After header, and retries with exponential backoff (max 5 attempts) instead of crashing." **As of 0.2.87 this is NOT in the shipped SDK.** `[INFERRED]` Re-check the CHANGELOG in later releases.

### 7.3 What the SDK *does* surface `[DOC]`+`[ISSUE]`

Cross-ref topic 03: the SDK emits a typed `RateLimitEvent` (Python) / `SDKRateLimitEvent` (TS) when "rate limit info changes," carrying `RateLimitInfo` / `SDKRateLimitInfo`. **But topic 03 flagged this type as typed-but-undocumented** ([claude-code#26392](https://github.com/anthropics/claude-code/issues/26392)): the per-bucket field schema (`utilization`, `resetsAt`, buckets like `five_hour`/`seven_day`) is inferred from issue discussion, not official docs. So a host *can* observe rate-limit pressure as a stream event, but the payload contract is not stable. The CLI also emits a TS-only `SDKAPIRetryMessage` for lower-level retry visibility (topic 03 §7.2).

### 7.4 Concurrency-vs-rate-limit guidance (host must add) `[INFERRED]`+`[ISSUE]`

Because the SDK neither throttles nor (today) retries:
- **Cap concurrency host-side** with an `asyncio.Semaphore(K)` around `query()` dispatch (§6.3). Community guidance: "Application-level concurrency controls (semaphores, request queues) should cap simultaneous agent sessions based on your API tier's rate limits."
- **Handle 429 host-side** on Python 0.2.87: catch the subprocess failure, back off (respect `Retry-After` if you can read it), and **resume** the session by ID (topic 02 §2 — `session_id` is on every `ResultMessage` "regardless of success or error," so a rate-limited run is resumable). Don't rely on the SDK to retry.
- **Watch `RateLimitEvent`** for early backpressure signal, but treat its payload as unstable.

---

## 8. Cancelling one among many

The interrupt/abort mechanisms (topic 03 §7.3) are **each scoped to a single session**, so cancelling one concurrent session leaves the others running — exactly what a fan-out join needs for "cancel the losers."

### 8.1 TypeScript — per-`Query` `interrupt()` and per-`Options` `abortController` `[DOC]`

- `Query.interrupt(): Promise<void>` — on the handle returned by *that* `query()` call only. The `Query` interface (verbatim, TS reference) exposes `interrupt()`, `close()`, plus `setModel`, `setPermissionMode`, `stopTask(taskId)`, `supportedAgents()`, etc. Calling `interrupt()` on one handle does not touch sibling queries.
- `options.abortController: AbortController` (default `new AbortController()`) — "Controller for cancelling operations." `[DOC]` Per topic 03 §7.3, the signal is wired down to every API call, tool process, and child subagent **of that query**. Give each concurrent `query()` its **own** `AbortController`; `controller.abort()` cancels just that session and emits `ResultMessage` with `subtype: "error_during_execution"`.
- `Query.stopTask(taskId)` `[DOC]` — stops a single background task within a session (topic 06 background subagents), a finer-grained lever than aborting the whole query.

### 8.2 Python — `ClaudeSDKClient.interrupt()` or `asyncio.Task.cancel()` `[DOC]`+`[INFERRED]`

- `ClaudeSDKClient.interrupt() -> None` `[DOC]` — interrupts that client's session. Topic 03 quoted the buffer caveat: "`interrupt()` sends a stop signal but does not clear the message buffer. Messages already produced … including its `ResultMessage` (subtype `error_during_execution`) remain in the stream. You must drain them with `receive_response()` before reading the response to a new query."
- The bare `query()` function has **no `interrupt()`** `[DOC]` (topic 03). To cancel one among many in a `gather`, cancel that coroutine's `asyncio.Task`:
  ```python
  tasks = [asyncio.create_task(subtask(p, cwd)) for p, cwd in items]
  done, pending = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
  for t in pending:
      t.cancel()           # cancels just the losing sessions; winners untouched
  ```
  `[INFERRED]` This relies on `query()`'s async generator honouring cancellation and tearing down its subprocess via the transport's cleanup path — the same path implicated in the §5 cleanup bugs. Cancel rather than `break` (the docs' §5 warning).

### 8.3 Multi-session caveats `[ISSUE]`

- **Early abort corrupts resume** (topic 03 §7.3): [claude-agent-sdk-typescript#69](https://github.com/anthropics/claude-agent-sdk-typescript/issues/69) — aborting right after `init` can corrupt the resume cursor. In a fan-out where you cancel slow starters, the cancelled session may not be cleanly resumable.
- **No soft interrupt** (topic 03 §7.3): [typescript#120](https://github.com/anthropics/claude-agent-sdk-typescript/issues/120) — there is no "pause but keep the session open" in the current API; interrupt terminates the turn.
- **Write-after-exit race on subprocess stdin** (TS) `[ISSUE]`: [typescript#318](https://github.com/anthropics/claude-agent-sdk-typescript/issues/318) (open) — during a startup-crash → retry race, an uncaught `EPIPE` from a stdin write to an already-exited subprocess can **kill the host process** (no JS frame; bypasses generic `unhandledRejection` filters). In a multi-session host this is a blast-radius hazard: one session's subprocess crash can take down the whole process and thus *all* concurrent sessions. The issue's workaround is a narrow `process.on('uncaughtException')` filter for `code === 'EPIPE' && syscall === 'write'`. A multi-session TS host should install this defensively.

---

## Caveats / Not found

- **Docs are explicitly silent on multi-session concurrency, `asyncio.gather`, and thread safety.** Verified by direct search of the Python reference, TS reference, and streaming-vs-single-mode page — all returned "no mention." The "yes, concurrent `query()` works" conclusion is `[SRC]` (fresh client/transport per call, no in-process singleton) + `[DOC]` (hosting page's "multiple Claude Agent processes in one container") + `[INFERRED]`, **not** an explicit support statement.
- **Thread safety: docs silent; inferred from architecture.** The supported model is many `query()` coroutines on one event loop (anyio/asyncio), optionally over N subprocesses. Multi-OS-thread use is unvalidated; process-global `_ACTIVE_CHILDREN` and `~/.claude/` argue against it. No SDK guarantee either way.
- **`~/.claude/` state leak (#952) is reported for *serial* calls in one process; its extension to *concurrent* calls is `[INFERRED]`,** not separately reproduced. The mitigation (per-session `CLAUDE_CONFIG_DIR`/`HOME`) is the issue author's workaround, not an Anthropic-documented isolation API.
- **429 handling is `[ISSUE]`-sourced.** #812 (crash, open) and #973 (fix PR, open/unmerged) are community reports; treat "Python 0.2.87 crashes on 429" as the current observed behavior, but it is not an Anthropic doc statement. Whether the TS SDK retries 429 was **not separately confirmed** — assume no built-in retry on either SDK until a CHANGELOG entry says otherwise.
- **`RateLimitInfo` payload schema is unstable** (topic 03 / claude-code#26392) — observable as a stream event but do not build hard logic on its fields.
- **TS issue #300 ("4 of 5 parallel sessions fail") is NOT about the Claude Agent SDK.** It targets `@anthropic-ai/sdk`'s `beta.sessions` API (a *different* product surface), not `@anthropic-ai/claude-agent-sdk`'s `query()`. **Excluded as evidence for this file** — flagged here only so a future reader doesn't mis-cite it as an Agent-SDK parallel-query failure.
- **No documented numeric ceiling on concurrent sessions per host.** The ~1 GiB/1 CPU-per-instance figure is the hosting page's per-instance *recommendation*, from which the practical ceiling is `[INFERRED]`. The binding limit is often the org API rate budget, not local RAM.
- **No Python `startup()`/`WarmQuery` equivalent** found on the Python reference; warm-pooling to amortise spawn cost is TS-only at this snapshot. Python pays subprocess spawn inline per `query()`.
- **Per-`query()` cancellation in a `gather` via `Task.cancel()` is `[INFERRED]`** — it depends on the async-generator cleanup path, which the §5 bug cluster (#890/#810/#746/#454) shows is the SDK's most fragile area. Verify clean teardown empirically before relying on aggressive fan-out cancellation in production.
- **Snapshot only:** Python 0.2.87 (CLI 2.1.150) / TS 0.3.150, both released 2026-05-23; docs fetched 2026-05-25. `code.claude.com` pages carry no per-page "last updated" date; version pin is via package releases + CHANGELOG.

## External references

- [Hosting the Agent SDK (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/hosting) — "running *multiple* Claude Agent processes inside of the container"; Pattern 4 "prevent agents from overwriting each other"; per-instance ~1GiB/1CPU/5GiB requirement; bundled native binary.
- [Streaming Input — Streaming vs Single mode (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode) — `query()` vs `ClaudeSDKClient`; silent on concurrency (confirmed).
- [Agent SDK reference — Python (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/python) — `query()` / `ClaudeSDKClient.interrupt()` signatures; the `break`-during-iteration cleanup warning; silent on concurrency/threads (confirmed).
- [Agent SDK reference — TypeScript (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/typescript) — `Query` interface (`interrupt`, `close`, `stopTask`, `supportedAgents`); `Options.abortController`; `startup()` / `WarmQuery`; silent on concurrency (confirmed).
- [`query.py` (anthropics/claude-agent-sdk-python, main)](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/query.py) — fresh `InternalClient()` per call.
- [`_internal/client.py` (main)](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/client.py) — fresh transport per `process_query`; "no module-level mutable state or singletons."
- [`_internal/transport/subprocess_cli.py` (main)](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/transport/subprocess_cli.py) — per-instance subprocess via `anyio.open_process`; module-global `_ACTIVE_CHILDREN` + `atexit`; per-transport `anyio.Lock` write lock; `self._cwd` handling + existence check.
- [python#812 — Agent SDK should handle 429 gracefully instead of crashing (OPEN)](https://github.com/anthropics/claude-agent-sdk-python/issues/812) — current 429 = crash.
- [python#973 — fix: handle 429 with Retry-After + exponential backoff (OPEN, unmerged)](https://github.com/anthropics/claude-agent-sdk-python/issues/973) — proposed 429 retry; not shipped in 0.2.87.
- [python#952 — bundled CLI drops TRACEPARENT on 2nd+ query() when ~/.claude/ has prior state (OPEN)](https://github.com/anthropics/claude-agent-sdk-python/issues/952) — `~/.claude/` state leaks across calls in one process.
- [python#890 — BaseExceptionGroup escapes transport.close() on second query() in same event loop (CLOSED)](https://github.com/anthropics/claude-agent-sdk-python/issues/890) — serial back-to-back query cleanup bug; minimal repro.
- [python#810 — stderr task group leaks cancel scope on query() completion (CLOSED)](https://github.com/anthropics/claude-agent-sdk-python/issues/810) and [PR #746 (#454)](https://github.com/anthropics/claude-agent-sdk-python/pull/746) — the async-cleanup fragility cluster.
- [python#956 / #910 — hook dispatch is concurrent, not sequential (CLOSED)](https://github.com/anthropics/claude-agent-sdk-python/issues/956) — confirms the CLI fires hooks for one event in parallel (fire-and-forget); relevant to in-session concurrency, cross-ref topic 04.
- [typescript#318 — uncaught EPIPE on subprocess stdin write during startup-crash race kills host process (OPEN)](https://github.com/anthropics/claude-agent-sdk-typescript/issues/318) — multi-session blast-radius hazard.
- [typescript#286 — user message written before awaiting initialization, stdin race at CLI (OPEN)](https://github.com/anthropics/claude-agent-sdk-typescript/issues/286) — subprocess init-ordering race.
- [typescript#69 / #120 (via topic 03)](https://github.com/anthropics/claude-agent-sdk-typescript/issues/69) — early-abort corrupts resume; no soft interrupt.
- [claude-code#26392 (via topic 03)](https://github.com/anthropics/claude-code/issues/26392) — `SDKRateLimitInfo` typed but undocumented.
