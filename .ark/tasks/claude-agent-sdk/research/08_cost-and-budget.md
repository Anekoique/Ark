# Research: Claude Agent SDK cost, token, and budget surfaces

- Query: enumerate every place the SDK exposes cost/token data, when each is observable (live during the stream vs. only at the final result), the verbatim `usage` / `modelUsage` shapes, the native budget cap (`maxBudgetUsd` / `max_budget_usd`) and turn cap (`maxTurns` / `max_turns`), the host-side cumulative-budget accumulation pattern across multiple `query()` calls, cost-accuracy caveats, and subagent cost attribution.
- Scope: external (primary: docs.claude.com / code.claude.com cost-tracking doc; SDK source for option + field names — TS `.d.ts` from the published 0.3.150 tarball, Python `types.py` on `main`).
- Date: 2026-05-25
- SDK versions referenced:
  - Python `claude-agent-sdk` **0.2.87** (PyPI latest as of 2026-05-25 — matches pin; recent line `0.2.82`…`0.2.87`).
  - TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.150** (npm `latest` *and* `next` as of 2026-05-25 — matches pin). Field/type quotes below are extracted from `package/sdk.d.ts` inside the published `0.3.150` tarball.
  - No newer release on either registry at snapshot time.
- Builds on topic **03** (`03_streaming-events.md`), which established that `ResultMessage` / `SDKResultMessage` carries `total_cost_usd`, `usage`, `modelUsage`, and that `error_max_budget_usd` is one of four error result subtypes. This file does **not** re-derive the event taxonomy; it is the cost/token/budget topic specifically. Cross-refs topic **06** (`06_subagents.md` §8) for subagent cost attribution.

---

## 0. TL;DR for ArkOS

1. Cost and cumulative token totals are observable **only at the terminal `ResultMessage`** of each `query()` call. Per-step *token* usage rides on each assistant message live; per-step *cost* does not.
2. The SDK has a **native budget cap** — TS `maxBudgetUsd?: number`, Python `max_budget_usd: float | None`. When the **client-side cost estimate** crosses it, the query terminates with a `ResultMessage` of `subtype == "error_max_budget_usd"` (a clean terminal event, **not** an exception). Partial usage/cost is still reported.
3. The native cap is **per-`query()` only**. The SDK explicitly **does not** provide a session-level or cross-call total ("The SDK does not provide a session-level total… accumulate the totals yourself"). For ArkOS's cumulative budget across many phases/sessions, the host **must** read `total_cost_usd` off each `ResultMessage` and accumulate, aborting before the next `query()` if over. That is the only cross-session option.
4. `total_cost_usd` / `costUSD` are **client-side estimates from a bundled price table**, explicitly flagged "not authoritative billing data." Do not bill end users from them.
5. Subagent cost is **folded into the parent's `total_cost_usd` / `modelUsage`** — there is no per-subagent cost line. Per-model split (via `modelUsage`) is the only built-in attribution; running a role as a separate `query()` is the only way to get a clean per-role total (topic 06 §8).
6. Known accuracy bug: `usage` and `modelUsage` **disagree even for a single model**, and `total_cost_usd` reconciles with `modelUsage`, not `usage` ([ts#112](https://github.com/anthropics/claude-agent-sdk-typescript/issues/112)).

---

## 1. Where cost and tokens surface

There are exactly **two** observation points, with different content:

| Observation point | Token data | Cost data | Cumulative or per-step | When readable |
| ----------------- | ---------- | --------- | ---------------------- | ------------- |
| Each **assistant message** (`AssistantMessage` / `SDKAssistantMessage`) | yes — `usage` object with token counts | **no** | **per-step** (one "step" = one request/response cycle; may span multiple assistant messages sharing one `id`) | **live, during the stream**, as each turn completes |
| The terminal **result message** (`ResultMessage` / `SDKResultMessage`) | yes — cumulative `usage` | yes — `total_cost_usd` + per-model `modelUsage[*].costUSD` | **cumulative across the whole `query()`** | **only at end of the `query()`** |

The [cost-tracking doc](https://code.claude.com/docs/en/agent-sdk/cost-tracking) is explicit on the split:

> **TypeScript** provides per-step token breakdowns on each assistant message (`message.message.id`, `message.message.usage`), per-model cost via `modelUsage` on the result message, and a cumulative total on the result message.
> **Python** provides per-step token breakdowns on each assistant message (`message.usage`, `message.message_id`), per-model cost via `model_usage` on the result message, and the accumulated total on the result message (`total_cost_usd` and `usage` dict).

### 1.1 When can a host read a *running cost total*?

**Not live.** There is no per-turn cost field. A host wanting a running **cost** figure inside a single `query()` cannot get one from the SDK — it can only:

- accumulate **tokens** live (deduplicating by message `id`, see §2.3) and price them itself against a table it maintains, or
- wait for the terminal `ResultMessage.total_cost_usd`.

The doc's own diagram caption confirms cost lands once, at the end: *"the final result message shows the estimated `total_cost_usd`."* Per-step assistant messages show only token usage, not USD.

For ArkOS this means **intra-`query()` cost-based abort is approximate** unless you let the SDK's own `maxBudgetUsd` cap handle it (§4). A *host-side* running cost can only step forward at `ResultMessage` boundaries, i.e. once per `query()`.

### 1.2 Cost is reported on *both* success and error results

> Both success and error result messages include `usage` and `total_cost_usd`. If a conversation fails mid-way, you still consumed tokens up to the point of failure. Always read cost data from the result message regardless of its `subtype`.

Confirmed in the TS type: both `SDKResultSuccess` and `SDKResultError` carry `total_cost_usd: number`, `usage: NonNullableUsage`, `modelUsage: Record<string, ModelUsage>` (see §5.3). So a budget accumulator reads `total_cost_usd` on **every** `ResultMessage`, not just successes.

---

## 2. Token fields

### 2.1 The `usage` object shape (cumulative, on the result message)

The result-message `usage` field is the Anthropic Messages API usage object, wrapped so its fields are non-null. From `package/sdk.d.ts` (0.3.150):

```typescript
// sdk.d.ts
import type { BetaUsage } from '@anthropic-ai/sdk/resources/beta/messages/messages.mjs';

export declare type NonNullableUsage = {
    [K in keyof BetaUsage]: NonNullable<BetaUsage[K]>;
};
```

So `usage` keys are exactly the Anthropic `BetaUsage` keys. The four load-bearing ones, named verbatim as they appear (snake_case — this object is the **raw API shape**, not the camelCase `ModelUsage`):

| Field (snake_case) | Meaning |
| ------------------ | ------- |
| `input_tokens` | standard (uncached) prompt tokens |
| `output_tokens` | generated tokens |
| `cache_creation_input_tokens` | tokens written to a new cache entry (billed **higher** than standard input) |
| `cache_read_input_tokens` | tokens served from an existing cache entry (billed **lower**) |

From the cost-tracking doc:

> The usage object includes two additional fields for cache tracking:
> * `cache_creation_input_tokens`: tokens used to create new cache entries (charged at a higher rate than standard input tokens).
> * `cache_read_input_tokens`: tokens read from existing cache entries (charged at a reduced rate).

In **Python**, `ResultMessage.usage` is a `dict[str, Any]` (topic 03 §1.1) — read keys defensively, e.g. `message.usage.get("cache_read_input_tokens", 0)`. In **TypeScript** these are typed on `usage` via `NonNullableUsage`.

### 2.2 The per-step `usage` (live, on each assistant message)

Per-step usage rides on the assistant message and is **per request/response cycle**, not cumulative:

- **TS:** `message.message.usage` (nested inside the Anthropic `BetaMessage`), with `message.message.id` as the dedup key. Fields are the same Anthropic `Usage` keys (`input_tokens`, `output_tokens`, plus cache fields).
- **Python:** `AssistantMessage.usage: dict[str, Any] | None` directly, with `AssistantMessage.message_id` as the dedup key (topic 03 §1.1 quotes the dataclass).

### 2.3 Deduplicate by message id (parallel tool calls)

A documented trap: parallel tool calls in one turn produce **multiple assistant messages sharing one `id` with identical usage**. Summing naively double-counts.

> Parallel tool calls produce multiple assistant messages whose nested `BetaMessage` shares the same `id` and identical usage. Always deduplicate by ID to get accurate per-step token counts.

The doc's per-step accumulator (TS, verbatim):

```typescript
const seenIds = new Set<string>();
let totalInputTokens = 0;
let totalOutputTokens = 0;

for await (const message of query({ prompt: "Summarize this project" })) {
  if (message.type === "assistant") {
    const msgId = message.message.id;
    if (!seenIds.has(msgId)) {            // parallel tool calls share the same ID
      seenIds.add(msgId);
      totalInputTokens += message.message.usage.input_tokens;
      totalOutputTokens += message.message.usage.output_tokens;
    }
  }
}
```

The doc's own guidance is to prefer the result-message totals over hand-summing (§7.2): *"the `total_cost_usd` in the result message reflects the SDK's accumulated estimate across all steps, so it is more reliable than summing per-step values yourself."*

---

## 3. `modelUsage` — per-model breakdown

When a run touches more than one model (e.g. an Opus main agent dispatching a Haiku subagent), `modelUsage` (TS) / `model_usage` (Python) splits usage and cost by model name. It lives **only on the result message**.

### 3.1 `ModelUsage` shape (verbatim, `package/sdk.d.ts` 0.3.150)

```typescript
export declare type ModelUsage = {
    inputTokens: number;
    outputTokens: number;
    cacheReadInputTokens: number;
    cacheCreationInputTokens: number;
    webSearchRequests: number;
    costUSD: number;
    contextWindow: number;
    maxOutputTokens: number;
};
```

Note the **camelCase** field names here — distinct from the snake_case keys of the top-level `usage` object (§2.1). The map is keyed by model name:

```typescript
// on both SDKResultSuccess and SDKResultError:
modelUsage: Record<string, ModelUsage>;   // e.g. modelUsage["claude-haiku-4-5-20251001"]
```

`ModelUsage` is the **only** place the SDK exposes a per-something `costUSD` other than the single grand `total_cost_usd`. It is also the only place `webSearchRequests`, `contextWindow`, and `maxOutputTokens` appear.

### 3.2 Iterating per-model cost (verbatim TS, from the doc)

```typescript
for await (const message of query({ prompt: "Summarize this project" })) {
  if (message.type !== "result") continue;
  for (const [modelName, usage] of Object.entries(message.modelUsage)) {
    console.log(`${modelName}: $${usage.costUSD.toFixed(4)}`);
    console.log(`  Input tokens: ${usage.inputTokens}`);
    console.log(`  Output tokens: ${usage.outputTokens}`);
    console.log(`  Cache read: ${usage.cacheReadInputTokens}`);
    console.log(`  Cache creation: ${usage.cacheCreationInputTokens}`);
  }
}
```

Python: same data under `message.model_usage` (a `dict`), with the same per-model camelCase keys inside each entry. Topic 03 §1.1 shows `ResultMessage.model_usage: dict[str, Any] | None`.

### 3.3 `modelUsage` is the authoritative cost basis (bug context)

[ts#112](https://github.com/anthropics/claude-agent-sdk-typescript/issues/112) (open, labeled bug + question, no maintainer fix at snapshot) reports that for a **single-model** run `usage` and `modelUsage` **do not match**, e.g.:

```text
usage:       input_tokens=33  cache_creation_input_tokens=53995  cache_read_input_tokens=230827  output_tokens=904
modelUsage:  inputTokens=39   cacheCreationInputTokens=123506    cacheReadInputTokens=230827     outputTokens=1284
```

and that `total_cost_usd` reconciles against `modelUsage`, **not** `usage`. Practical consequence for ArkOS: **treat `modelUsage[*]` (and the SDK's own `total_cost_usd`) as the cost basis; treat the top-level `usage` token figures as advisory.** Do not re-price from `usage` and expect it to equal `total_cost_usd`.

---

## 4. Built-in budget cap

### 4.1 The option (verbatim)

**TypeScript** (`package/sdk.d.ts` 0.3.150, on the `Options` type):

```typescript
    maxTurns?: number;
    /**
     * Maximum budget in USD for the query. The query will stop if this
     * budget is exceeded, returning an `error_max_budget_usd` result.
     */
    maxBudgetUsd?: number;
    /**
     * API-side task budget in tokens. When set, the model is made aware of
     * its remaining token budget so it can pace tool use and wrap up before
     * the limit. Sent as `output_config.task_budget` with the
     * `task-budgets-2026-03-13` beta header.
     * @alpha
     */
    taskBudget?: {
        total: number;
    };
```

**Python** (`ClaudeAgentOptions`, `types.py` on `main`):

```python
    max_budget_usd: float | None = None
    """Maximum budget in USD for the query.

    The query will stop if this budget is exceeded, returning an
    ``error_max_budget_usd`` result.
    """

    task_budget: TaskBudget | None = None   # class TaskBudget(TypedDict): total: int
```

So the verified names are: **TS `maxBudgetUsd`**, **Python `max_budget_usd`**. (Not `maxBudgetUSD`, not `budget_usd`.) There is also a distinct, **alpha**, *token*-denominated budget — TS `taskBudget: { total }` / Python `task_budget: TaskBudget` — which behaves differently (see §4.4).

### 4.2 What happens when hit

- The query **terminates with a `ResultMessage` whose `subtype == "error_max_budget_usd"`** — a clean terminal event, the same single-`ResultMessage`-per-query contract as success. It is **not** raised as a Python exception (those are reserved for transport/spawn failures, topic 03 §7.1) and **not** a JS throw.
- `is_error` is `true`; `errors: string[]` is populated; `result` (the final-text field) is **absent** on the error variant (the TS `SDKResultError` type has no `result` member — only `errors`).
- `total_cost_usd`, `usage`, and `modelUsage` **are still present** on the error result — you get the partial spend up to the abort (§1.2, §5.3).

It is the **partial-result-with-clean-terminal** model: you do not lose the cost accounting for the work already done; you just do not get a final assistant answer.

### 4.3 Per-turn vs. continuous checking

The doc comment is "The query will stop if this budget is **exceeded**," and the cost-tracking doc says `maxBudgetUsd` is *"Stop the query when the client-side cost estimate reaches this USD value. Compared against the same estimate as `total_cost_usd`."* The estimate the SDK tracks is the same accumulator that ends up in `total_cost_usd`, which only advances when a step's usage is known (i.e. at turn/step boundaries). So the practical granularity is **checked at step/turn boundaries, not mid-generation** — a single very expensive turn can overshoot the cap before the check fires. The docs do **not** state an explicit "checked every N ms" guarantee; treat it as "checked after each step, may overshoot by up to one step." (Undocumented exact granularity — flagged.)

Note also `TerminalReason` (verbatim) enumerates `'max_turns'` but has **no** `max_budget` member:

```typescript
export declare type TerminalReason = 'blocking_limit' | 'rapid_refill_breaker'
  | 'prompt_too_long' | 'image_error' | 'model_error' | 'aborted_streaming'
  | 'aborted_tools' | 'stop_hook_prevented' | 'hook_stopped' | 'tool_deferred'
  | 'max_turns' | 'completed';
```

So for a budget abort, discriminate on `result.subtype == "error_max_budget_usd"`, **not** on `terminal_reason` (which carries no budget value).

### 4.4 `taskBudget` / `task_budget` is a different mechanism

This is **not** a hard host-side cap — it is an *advisory token budget passed to the model* so it self-paces. It is **alpha**, token-denominated (`{ total: number }`), and sent via the `task-budgets-2026-03-13` beta header as `output_config.task_budget`. It does **not** produce an `error_max_budget_usd` abort. For ArkOS hard enforcement, `max_budget_usd` is the relevant knob; `task_budget` is a soft hint only. (Treat as experimental — the alpha flag means the surface can change.)

---

## 5. `maxTurns` / `max_turns` — the turn-count ceiling

### 5.1 The option (verbatim)

**TypeScript** (`Options`):

```typescript
    /**
     * Maximum number of conversation turns before the query stops.
     * A turn consists of a user message and assistant response.
     */
    maxTurns?: number;
```

**Python** (`ClaudeAgentOptions`):

```python
    max_turns: int | None = None
    """Maximum number of conversation turns before the query stops.

    A turn consists of a user message and assistant response.
    """
```

Verified names: **TS `maxTurns`**, **Python `max_turns`**. (A separate `maxTurns?: number` also exists on `AgentDefinition` — *"Maximum number of agentic turns (API round-trips) before stopping"* — that one caps an individual subagent, distinct from the top-level query cap.)

### 5.2 What happens when hit

The query terminates with `ResultMessage.subtype == "error_max_turns"` (one of the four error subtypes, topic 03 §1.3). Same clean-terminal semantics as the budget cap: `is_error == true`, `errors` populated, no `result` text, but `total_cost_usd` / `usage` / `modelUsage` present. `TerminalReason` includes `'max_turns'` here (unlike the budget case).

### 5.3 Interaction with the budget cap

Both `maxTurns` and `maxBudgetUsd` are independent ceilings on the **same** `query()`. Whichever trips first wins and sets the corresponding `subtype` (`error_max_turns` vs. `error_max_budget_usd`). There is no documented ordering/priority between them; they are separate guards. The error `ResultMessage` shape is identical apart from `subtype` and the `errors` strings. From `sdk.d.ts` (0.3.150), the discriminated error variant:

```typescript
export declare type SDKResultError = {
    type: 'result';
    subtype: 'error_during_execution' | 'error_max_turns'
           | 'error_max_budget_usd' | 'error_max_structured_output_retries';
    duration_ms: number;
    duration_api_ms: number;
    is_error: boolean;
    num_turns: number;
    stop_reason: string | null;
    total_cost_usd: number;          // present on errors too
    usage: NonNullableUsage;         // present on errors too
    modelUsage: Record<string, ModelUsage>;  // present on errors too
    permission_denials: SDKPermissionDenial[];
    errors: string[];               // error strings (no `result` field here)
    terminal_reason?: TerminalReason;
    fast_mode_state?: FastModeState;
    origin?: SDKMessageOrigin;
    uuid: UUID;
    session_id: string;
};
```

(Contrast `SDKResultSuccess`, which adds `ttft_ms?`, `api_error_status?`, `result: string`, `structured_output?`, `deferred_tool_use?` and omits `errors`.)

---

## 6. Enforcing a custom (cross-session) budget

### 6.1 Why the native cap is necessary-but-insufficient

`maxBudgetUsd` / `max_budget_usd` is scoped to **one `query()` call**. The cost-tracking doc is explicit that there is **no** session-level total and that cross-call accumulation is the host's job:

> Each `query()` call returns its own `total_cost_usd`. **The SDK does not provide a session-level total**, so if your application makes multiple `query()` calls (for example, in a multi-turn session or across different users), accumulate the totals yourself.

and on sessions specifically:

> A series of `query()` calls linked by a session ID (using the `resume` option). **Each `query()` call within a session reports its own cost independently.**

So even within a single resumed session, `resume`d `query()` calls do **not** sum automatically. A substrate running many phases across many sessions therefore **must** accumulate `total_cost_usd` host-side and gate the *next* `query()` against the running total. This host-side accumulation is the **only** cross-session enforcement option the SDK offers — there is no global/process budget knob.

### 6.2 The accumulation + auto-abort pattern (Python)

The doc's accumulator (verbatim) only sums; ArkOS needs the abort guard added. The minimal host-side cumulative cap:

```python
from claude_agent_sdk import query, ClaudeAgentOptions, ResultMessage
import asyncio

CUMULATIVE_BUDGET_USD = 5.00
PER_QUERY_CAP_USD = 1.00          # native per-query() guard (defense in depth)

async def run_phases(phases: list[str]) -> float:
    total_spend = 0.0
    for prompt in phases:
        # Pre-flight gate: refuse to start the next query() if already over.
        if total_spend >= CUMULATIVE_BUDGET_USD:
            raise RuntimeError(
                f"cumulative budget ${CUMULATIVE_BUDGET_USD} reached "
                f"(spent ${total_spend:.4f}); aborting before next phase"
            )
        # Native per-query cap = ceiling so one runaway phase cannot blow the budget.
        opts = ClaudeAgentOptions(
            max_budget_usd=min(PER_QUERY_CAP_USD,
                               CUMULATIVE_BUDGET_USD - total_spend),
        )
        async for message in query(prompt=prompt, options=opts):
            if isinstance(message, ResultMessage):
                # total_cost_usd is present on BOTH success and error subtypes.
                total_spend += message.total_cost_usd or 0.0
                if message.subtype == "error_max_budget_usd":
                    # native per-query cap tripped — phase truncated, cost still booked
                    break
    return total_spend

asyncio.run(run_phases(["plan the work", "execute step 1", "verify"]))
```

Key points the pattern relies on (all verified above):
- Gate **before** dispatching the next `query()` (pre-flight), because cost is only known after a `ResultMessage`, never mid-query.
- Set the native `max_budget_usd` to the **remaining** cumulative headroom (`CUMULATIVE_BUDGET_USD - total_spend`, capped per-phase) so a single phase cannot overshoot the whole budget — defense in depth, since the host gate alone can only stop *between* queries.
- Add `total_cost_usd` from **every** `ResultMessage` regardless of `subtype` (§1.2) — a budget-aborted phase still spent money.
- Use `message.total_cost_usd or 0.0` in Python (the field is `float | None`).

### 6.3 TS equivalent (accumulator from the doc, abort guard added)

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

const CUMULATIVE_BUDGET_USD = 5.0;
let totalSpend = 0;

for (const prompt of phases) {
  if (totalSpend >= CUMULATIVE_BUDGET_USD) {
    throw new Error(`cumulative budget reached; spent $${totalSpend.toFixed(4)}`);
  }
  const maxBudgetUsd = Math.min(1.0, CUMULATIVE_BUDGET_USD - totalSpend);
  for await (const message of query({ prompt, options: { maxBudgetUsd } })) {
    if (message.type === "result") {
      totalSpend += message.total_cost_usd;        // present on success + error
      if (message.subtype === "error_max_budget_usd") break;
    }
  }
}
```

(The doc's published TS accumulator is the inner `totalSpend += message.total_cost_usd` loop without the pre-flight gate or per-phase `maxBudgetUsd`; those two additions are what turn "track" into "enforce.")

### 6.4 Caveat on the host-side gate's resolution

Because the host can only observe cost at `ResultMessage` boundaries (§1.1), the host-side cumulative gate has a granularity of **one `query()` call**. It cannot stop a phase mid-flight; the native `max_budget_usd` is the only mid-`query()` brake, and even that overshoots by up to one step (§4.3). For tight enforcement, keep individual `query()` calls small and lean on `max_budget_usd` as the inner guard.

---

## 7. Cost accuracy

### 7.1 `total_cost_usd` is an estimate, not billing data

The cost-tracking doc carries a prominent warning (verbatim):

> The `total_cost_usd` and `costUSD` fields are client-side estimates, not authoritative billing data. The SDK computes them locally from a price table bundled at build time, so they can drift from what you are actually billed when:
> * pricing changes
> * the installed SDK version does not recognize a model
> * billing rules apply that the client cannot model
>
> Use these fields for development insight and approximate budgeting. For authoritative billing, use the [Usage and Cost API](https://platform.claude.com/docs/en/build-with-claude/usage-cost-api) or the Usage page in the [Claude Console](https://platform.claude.com/usage). Do not bill end users or trigger financial decisions from these fields.

Concrete implications for ArkOS:
- The price table is **pinned to the installed SDK version**. A newer model the bundled table doesn't know about → cost may be **0 / wrong**. (This is a direct argument to track which SDK version produced a given cost figure.)
- Pricing changes after the SDK build are not reflected until upgrade.
- Cache pricing **is** modeled (the table prices `cache_creation_input_tokens` higher and `cache_read_input_tokens` lower — §2.1), so cache savings *are* reflected in the estimate, but still as an estimate.

### 7.2 Token-count discrepancies (known issues)

- **Same-id output-token drift.** The doc acknowledges: *"In rare cases, you might observe different `output_tokens` values for messages with the same ID."* Guidance: use the highest value (final message in the group is usually accurate), and prefer the result-message `total_cost_usd` over hand-summing.
- **`usage` vs. `modelUsage` mismatch.** [ts#112](https://github.com/anthropics/claude-agent-sdk-typescript/issues/112) (open, no fix at snapshot): the two disagree even for a single model, and `total_cost_usd` reconciles with `modelUsage`. Trust `modelUsage` + `total_cost_usd`, not the top-level `usage` token figures, for cost (§3.3).
- The doc invites filing further inconsistencies at the [Claude Code repo](https://github.com/anthropics/claude-code/issues).

### 7.3 Does it match the dashboard?

No guarantee. The warning's whole point is that it can drift from the actual bill; for reconciliation the doc points to the **Usage and Cost API** / Console Usage page as authoritative, not `total_cost_usd`.

---

## 8. Subagent cost attribution

Cross-ref topic **06** (`06_subagents.md` §8 "Limits and gaps"), which states:

> **Per-subagent cost is not surfaced as a distinct field.** Subagent token/cost folds into the parent `ResultMessage.total_cost_usd` / `modelUsage`. There is no per-`Agent`-call cost line in the SDK result. To get per-role cost cleanly, run the role as a separate `query()`.

This file confirms and sharpens it:

- A subagent invoked via the `Agent` tool runs inside the **parent's** `query()`. Its cost is **rolled up** into the parent's single `total_cost_usd`. There is **no** `ResultMessage` field that itemizes "this Agent call cost $X."
- The **only** built-in attribution is `modelUsage`: if the subagent runs on a different model (e.g. Haiku subagent under an Opus main), its tokens/cost appear under that model's key in `modelUsage` (§3). The cost-tracking doc itself motivates `modelUsage` with exactly this case: *"useful when you run multiple models (for example, Haiku for subagents and Opus for the main agent)."* But if the subagent shares the parent's model, `modelUsage` does **not** separate them — they merge under one model key.
- Stream-level attribution exists separately via `parent_tool_use_id` (topic 03 §8) and `SDKAssistantMessage.subagentType` (the `.d.ts` has a `subagentType` field flagged *"Subagent type that produced this message"*), but those tag **messages**, not **cost** — there is no per-message USD to sum from them (only per-step *tokens*, which a host could itself price and bucket by `parent_tool_use_id` if it wanted DIY per-subagent cost).
- **Therefore:** for clean per-role / per-subagent cost in ArkOS, run each role as its **own** `query()` and read that query's `total_cost_usd` (topic 06's recommendation). The in-parent subagent path gives only model-granular attribution.

---

## External references

- [Track cost and usage (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/cost-tracking) — primary source: the estimate warning, per-step vs. result observation split, dedup-by-id, per-model `modelUsage` example, cross-call accumulation snippet, cache-token fields, `ENABLE_PROMPT_CACHING_1H`, failed-conversation cost handling. (visited 2026-05-25)
- [Agent SDK reference — TypeScript (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/typescript) — `Options.maxTurns`, `Options.maxBudgetUsd`, `Options.taskBudget`, `SDKResultMessage` discriminated subtypes, `ModelUsage` / `NonNullableUsage` references.
- `@anthropic-ai/claude-agent-sdk` **0.3.150** `package/sdk.d.ts` (extracted from the npm tarball, 2026-05-25) — verbatim source of: `ModelUsage`, `NonNullableUsage` (= `NonNullable<BetaUsage[K]>`), `Options.maxTurns` / `maxBudgetUsd` / `taskBudget` doc comments, `SDKResultSuccess` / `SDKResultError` field lists, `TerminalReason` union.
- [`types.py` on main (anthropics/claude-agent-sdk-python)](https://raw.githubusercontent.com/anthropics/claude-agent-sdk-python/refs/heads/main/src/claude_agent_sdk/types.py) — `ClaudeAgentOptions.max_turns`, `max_budget_usd`, `task_budget`; `TaskBudget`, `TaskUsage` TypedDicts; `ResultMessage` / `AssistantMessage` usage fields (cross-ref topic 03).
- [claude-agent-sdk-typescript#112](https://github.com/anthropics/claude-agent-sdk-typescript/issues/112) — open bug: `usage` vs. `modelUsage` disagree for a single model; `total_cost_usd` reconciles with `modelUsage`.
- [Usage and Cost API (platform.claude.com)](https://platform.claude.com/docs/en/build-with-claude/usage-cost-api) — the authoritative billing source the SDK warning points to (not the SDK fields).
- [Prompt caching (platform.claude.com)](https://platform.claude.com/docs/en/build-with-claude/prompt-caching) — cache-token pricing referenced by the cache fields.

---

## Caveats / Not found

- **Exact budget-check granularity is undocumented.** The doc says the query stops "when the client-side cost estimate reaches" `maxBudgetUsd`, but never states the polling cadence. Inferred to be checked at step/turn boundaries (the same accumulator that feeds `total_cost_usd`), implying possible single-step overshoot. No "checked every N ms" or "hard pre-flight per turn" guarantee found.
- **`taskBudget` / `task_budget` behavior is alpha.** It is an advisory **token** budget surfaced *to the model* via a dated beta header (`task-budgets-2026-03-13`), not a hard host cap, and not the source of `error_max_budget_usd`. The `@alpha` flag means its name/shape may change. Did not find docs detailing what the model does when it nears the task budget beyond "pace tool use and wrap up."
- **No live per-turn cost.** Confirmed there is no per-assistant-message USD field — only per-step *tokens* live, and cumulative cost only at `ResultMessage`. A truly live cost meter requires the host to price tokens itself against a table it maintains (with the same drift risk as the SDK's table).
- **`Usage` (top-level result `usage`) exact key list** is defined by Anthropic's `BetaUsage` (imported, not redefined in the SDK). The four keys cited (`input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`) are the documented/relevant ones; `BetaUsage` may carry additional optional keys (e.g. server-tool-use counters) not enumerated here because the SDK only re-exports the Anthropic type by reference.
- **Python `model_usage` per-entry shape** is `dict[str, Any]` (topic 03 §1.1) — the camelCase keys (`costUSD`, `inputTokens`, …) are inferred to match the TS `ModelUsage` since both SDKs "use the same underlying cost model" per the doc, but the Python facade does not export a typed `ModelUsage` dataclass. Read keys defensively.
- **`maxBudgetUsd` precedence vs. `maxTurns`** when both could trip on the same step is not documented; treated as "whichever condition is detected first sets the subtype." No tie-break rule found.
- **Whether the native `max_budget_usd` estimate uses `usage` or `modelUsage`** internally: the doc says it's "compared against the same estimate as `total_cost_usd`," and `total_cost_usd` reconciles with `modelUsage` per ts#112 — so the cap is presumably driven by the `modelUsage`-based estimate, but this is inferred from the bug report, not stated.
