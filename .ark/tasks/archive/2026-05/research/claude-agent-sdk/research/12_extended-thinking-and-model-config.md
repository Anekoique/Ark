# Research: Claude Agent SDK — model configuration and extended-thinking controls

- Query: how the SDK selects a model, falls back on failure, controls extended thinking / reasoning effort, caps thinking tokens, pins a per-subagent/per-role model, and produces structured (JSON-schema) output — exact option names, values, defaults, Python↔TS divergence.
- Scope: external (primary: `code.claude.com` Agent SDK TS reference + structured-outputs page; authoritative for declarations: the local Python clone `claude-agent-sdk-python` @ `src/claude_agent_sdk/types.py`, package version **0.2.87**).
- Date: 2026-05-25
- Doc snapshot: `code.claude.com` Agent SDK docs fetched 2026-05-25.
- SDK versions pinned:
  - Python `claude-agent-sdk` **0.2.87** (confirmed in the local clone's `pyproject.toml:7`).
  - TypeScript `@anthropic-ai/claude-agent-sdk` **0.3.150** (npm).
- Builds on / cross-refs (does not re-derive):
  - **Topic 01** — provider lock-in (Bedrock/Vertex/Azure are Claude-only hosting surfaces), top-level entry points, `EffortLevel` alias.
  - **Topic 06** — per-`AgentDefinition` `model` / `effort` override mechanism (the per-role lever); restated in §4 below.
  - **Topic 08** — `modelUsage` per-model cost split; `error_max_budget_usd`; `max_budget_usd` / `max_turns`. §6 here adds the *fourth* error subtype `error_max_structured_output_retries`.
  - **Topic 03** — streaming/partial-message event taxonomy; §3.5 here records the one streaming-coupling fact (`set_model` requires streaming-input mode).

---

## 0. TL;DR for ArkOS

1. **Model select:** top-level option is `model` — Python `model: str | None = None`, TS `model?: string`. Default = **"the CLI default model"** (the SDK does not hardcode an ID). Takes a **full model ID** (`"claude-sonnet-4-5"`, `"claude-opus-4-5"`) at the top level; the *alias* forms (`"sonnet"`/`"opus"`/`"haiku"`/`"inherit"`) are documented specifically for the **per-subagent** `AgentDefinition.model` field (§1, §4). (§1)
2. **Fallback:** `fallback_model` (Python) / `fallbackModel` (TS), a `str`. "Fallback model to use if the primary model fails or is unavailable." Triggers on primary-model failure/unavailability. (§2)
3. **Extended thinking is controlled by TWO knobs that compose:** `thinking` (the structured switch — `adaptive`/`enabled`/`disabled`) and `effort` (depth dial — `low|medium|high|xhigh|max`). On thinking-capable models thinking defaults to **adaptive** (`{"type": "adaptive"}`) — i.e. **on by default, model-decided**; `effort` defaults to **`"high"`**. (§3)
4. **`max_thinking_tokens` / `maxThinkingTokens` is DEPRECATED** in favor of `thinking`. On newer models it degrades to on/off (0 = disabled, nonzero = adaptive). Use `thinking={"type":"enabled","budget_tokens":N}` for an explicit fixed budget. (§5)
5. **Per-role (cheaper reviewer/verifier) model:** set `model` on the `AgentDefinition` (or filesystem agent frontmatter). Accepts alias / full ID / `"inherit"`. `effort` is also per-`AgentDefinition`. This is the per-role mechanism (topic 06 §7). (§4)
6. **Structured output exists, top-level only:** `output_format` (Python) / `outputFormat` (TS) = `{"type":"json_schema","schema": <JSON Schema>}`. Validated result lands on `ResultMessage.structured_output`. On exhausting internal retries → terminal `ResultMessage` with `subtype == "error_max_structured_output_retries"` (the 4th error subtype). **No option to configure the retry count.** Works for the top-level `query()`, **not** subagents (topic 06 §3.2). (§6)
7. **Provider model-ID note:** model IDs differ on Bedrock/Vertex (region-qualified IDs); the `model` *string* is provider-specific, the SDK does not translate aliases for you on those routes. (§7, cross-ref topic 01)

---

## 1. Model selection — the `model` option

### 1.1 Top-level option (verbatim)

**Python** (`types.py:1673`, `ClaudeAgentOptions`):

```python
    model: str | None = None
    """Claude model to use. Defaults to the CLI default model.

    Examples: ``"claude-sonnet-4-5"``, ``"claude-opus-4-5"``.
    """
```

**TypeScript** (`Options`, from the TS reference page):

```typescript
model?: string;   // doc comment: "Claude model to use" — Default: "Default from CLI"
```

So:
- **Type:** plain `string` (nullable / optional). No enum constraint at the type level.
- **Default:** **not hardcoded by the SDK** — it is "the CLI default model" / "Default from CLI". The SDK defers to whatever the bundled Claude Code CLI resolves as default (which in turn honors `ANTHROPIC_MODEL` and provider routing — §7). The docs do **not** print the concrete default model ID.
- **Value format at the top level:** a **full model ID**. The Python docstring's own examples are full IDs: `"claude-sonnet-4-5"`, `"claude-opus-4-5"`. The `set_model` docstring (§1.3) additionally shows dated IDs `"claude-opus-4-1-20250805"`, `"claude-opus-4-20250514"`.

### 1.2 Aliases (`sonnet`/`opus`/`haiku`/`inherit`) — documented for the *per-agent* field

The short aliases are documented on the **`AgentDefinition.model`** field, not (in the docstring) on the top-level option. From `types.py:91` (the `AgentDefinition` dataclass):

```python
    # Model alias ("sonnet", "opus", "haiku", "inherit") or a full model ID.
    model: str | None = None
```

Topic 06 §7 quotes the doc's per-agent description verbatim: *"Accepts an alias such as `'sonnet'`, `'opus'`, `'haiku'`, `'inherit'`, or a full model ID. Defaults to main model if omitted."* The CLI itself accepts aliases for its top-level model too, so a top-level alias is very likely honored in practice — but the **top-level `ClaudeAgentOptions.model` docstring only shows full IDs**, so for the top-level option treat full IDs as the documented form and aliases as inferred-from-CLI-behavior. `"inherit"` is meaningful only on a subagent (inherit the parent's model).

### 1.3 Per-session override — `set_model()` / `setModel()`

The model can be changed mid-session.

**Python** (`client.py:346`, on `ClaudeSDKClient`):

```python
    async def set_model(self, model: str | None = None) -> None:
        """Change the AI model during conversation (only works with streaming mode).

        Args:
            model: The model to use, or None to use default. Examples:
                - 'claude-sonnet-4-5'
                - 'claude-opus-4-1-20250805'
                - 'claude-opus-4-20250514'
        """
```

**TypeScript** (`Query` interface, from the TS reference):

```typescript
setModel(model?: string): Promise<void>;
// doc comment: "Changes the model (only available in streaming input mode)"
```

**Streaming-input-mode requirement (both SDKs):** `set_model`/`setModel` works **only in streaming input mode** (i.e. when driving the query with a streaming/async-iterable prompt, not the one-shot string form). Passing `None`/`undefined` reverts to the default model.

### 1.4 Python ↔ TS divergence (model)

| Aspect | Python 0.2.87 | TS 0.3.150 |
| ------ | ------------- | ---------- |
| option name | `model` (snake/single word — same) | `model` |
| type | `str \| None` | `string?` |
| mid-session setter | `client.set_model(model=None)` on `ClaudeSDKClient` | `query.setModel(model?)` on `Query` |
| docstring example IDs | `claude-sonnet-4-5`, `claude-opus-4-5` | none printed ("Default from CLI") |

No divergence in semantics; only surface naming (`set_model` vs `setModel`) and where the setter hangs (the bidirectional client vs the `Query` object).

---

## 2. Fallback model — `fallback_model` / `fallbackModel`

**It exists.**

**Python** (`types.py:1679`):

```python
    fallback_model: str | None = None
    """Fallback model to use if the primary model fails or is unavailable."""
```

**TypeScript** (`Options`, TS reference):

```typescript
fallbackModel?: string;   // doc comment: "Model to use if primary fails"
```

- **Type:** `string` (full model ID; same value space as `model`).
- **Default:** unset (`None` / `undefined`) — no automatic fallback unless you specify one.
- **Trigger:** when the **primary model fails or is unavailable**. The Python docstring is the more specific of the two ("fails *or is unavailable*"); the TS comment says only "if primary fails." Neither doc enumerates the exact failure classes (e.g. overload/429 vs hard error) that count as "fails" — see Caveats.

> **For Ark:** a substrate that pins Opus for the spine can set `fallback_model` to a Sonnet ID so a transient Opus unavailability degrades gracefully rather than erroring the whole `query()`.

---

## 3. Extended thinking — `thinking` + `effort` (two composing knobs)

This is the most material section for ArkOS and the place the SDK changed shape recently: **`max_thinking_tokens` is deprecated; `thinking` is the new structured switch, and `effort` is the depth dial that "works with adaptive thinking."**

### 3.1 The `thinking` option (the structured switch)

**Python** (`types.py:1861`):

```python
    thinking: ThinkingConfig | None = None
    """Controls Claude's thinking/reasoning behavior.

    - ``{"type": "adaptive"}`` — Claude decides when and how much to think
      (Opus 4.6+). Default for models that support it.
    - ``{"type": "enabled", "budget_tokens": N}`` — Fixed thinking token budget
      (older models).
    - ``{"type": "disabled"}`` — No extended thinking.

    When set, takes precedence over the deprecated ``max_thinking_tokens``.
    See https://docs.anthropic.com/en/docs/build-with-claude/adaptive-thinking.
    """
```

The `ThinkingConfig` union (verbatim, `types.py:1555-1575`):

```python
# Controls whether thinking text is returned summarized or omitted. Opus 4.7+
# defaults to "omitted" (signature-only); pass "summarized" to receive text.
ThinkingDisplay = Literal["summarized", "omitted"]


class ThinkingConfigAdaptive(TypedDict):
    type: Literal["adaptive"]
    display: NotRequired[ThinkingDisplay]


class ThinkingConfigEnabled(TypedDict):
    type: Literal["enabled"]
    budget_tokens: int
    display: NotRequired[ThinkingDisplay]


class ThinkingConfigDisabled(TypedDict):
    type: Literal["disabled"]


ThinkingConfig = ThinkingConfigAdaptive | ThinkingConfigEnabled | ThinkingConfigDisabled
```

Three modes:
- **`{"type": "adaptive"}`** — model decides whether/how deeply to think. Available on Opus 4.6+. **This is the default on models that support it** (so extended thinking is *on by default, model-controlled* — see §3.3).
- **`{"type": "enabled", "budget_tokens": N}`** — fixed thinking-token budget; the doc tags this as the form for **older models**.
- **`{"type": "disabled"}`** — no extended thinking at all.

A separate `display` sub-field (`"summarized" | "omitted"`, on the adaptive/enabled variants) controls whether the thinking *text* comes back. Per the source comment, **Opus 4.7+ defaults to `"omitted"` (signature-only)** — pass `display: "summarized"` to actually receive thinking text in the stream.

### 3.2 The `effort` option (the depth dial)

**Python** (`types.py:1874`):

```python
    effort: EffortLevel | None = None
    """Controls how much effort Claude puts into its response.

    Works with adaptive thinking to guide thinking depth.

    - ``"low"`` — Minimal thinking, fastest responses.
    - ``"medium"`` — Moderate thinking.
    - ``"high"`` — Deep reasoning (default).
    - ``"xhigh"`` — Extended reasoning depth (Opus 4.7 only; falls back to
      ``"high"`` on other models).
    - ``"max"`` — Maximum effort.

    See https://docs.anthropic.com/en/docs/build-with-claude/effort.
    """
```

`EffortLevel` (verbatim, `types.py:33`):

```python
EffortLevel: TypeAlias = Literal["low", "medium", "high", "xhigh", "max"]
```

**TypeScript** (`Options`, TS reference): `effort?: 'low' | 'medium' | 'high' | 'xhigh' | 'max';` — doc comment "Controls how much effort Claude puts into its response. Works with adaptive thinking to guide thinking depth." TS reference states the **default is `'high'`**, matching the Python docstring's "`"high"` — Deep reasoning (default)."

Key per-value facts:
- **Default = `"high"`** (deep reasoning) — verified on both surfaces.
- **`"xhigh"` is Opus-4.7-only** and **falls back to `"high"` on other models** (so it is safe to pass cross-model, but only Opus 4.7 honors it).
- `effort` "works *with* adaptive thinking to guide thinking depth" — i.e. it is a hint that shapes how much the adaptive thinker spends; it is not an independent on/off (that is `thinking`).
- Note the **per-`AgentDefinition.effort` field additionally accepts an `int`** (`EffortLevel | int`, `types.py:100`), but the **top-level `ClaudeAgentOptions.effort` is `EffortLevel` only** (no `int`). Mild divergence: agent-level effort takes a numeric form the top-level option does not.

### 3.3 Is extended thinking on by default?

**Yes, effectively — model-decided.** On models that support adaptive thinking (Opus 4.6+), `thinking` defaults to `{"type": "adaptive"}` ("Default for models that support it"), meaning the model autonomously decides when/how much to think. Combined with `effort` defaulting to `"high"`, the out-of-the-box behavior on a current Opus model is **adaptive thinking at high effort**. To turn thinking off entirely you must explicitly pass `thinking={"type": "disabled"}`. On older (non-adaptive) models, thinking is governed by the `enabled`/`budget_tokens` form instead.

Caveat: "Default for models that support it" is the only default statement; the docs do not enumerate which exact model IDs flip the default (Opus 4.6+ is the floor cited). Older models do not default to adaptive.

### 3.4 `thinking` ↔ `effort` interaction (summary)

| Knob | Role | Default | Notes |
| ---- | ---- | ------- | ----- |
| `thinking` | structural switch: adaptive / fixed-budget / off | `{"type":"adaptive"}` on supporting models | replaces deprecated `max_thinking_tokens`; `display` controls text visibility |
| `effort` | depth dial within thinking | `"high"` | "works with adaptive thinking to guide thinking depth"; `xhigh` Opus-4.7-only |
| `max_thinking_tokens` | (deprecated) token cap | unset | superseded by `thinking`; see §5 |

### 3.5 Streaming incompatibility (restated from topic 03)

The one hard streaming coupling in *this* topic's surface is **`set_model`/`setModel` requires streaming-input mode** (§1.3) — it raises / is unavailable in one-shot mode. Topic 03 owns the broader partial-message / `includePartialMessages` story; nothing in the `thinking`/`effort`/`output_format` docstrings declares an incompatibility with streaming. The `display: "omitted"` default on Opus 4.7+ means that even with thinking on, the *thinking text* is not streamed unless you set `display: "summarized"` — a visibility, not a streaming-mode, constraint.

### 3.6 Python ↔ TS divergence (thinking/effort)

| Field | Python 0.2.87 | TS 0.3.150 | Note |
| ----- | ------------- | ---------- | ---- |
| `thinking` | `thinking: ThinkingConfig` (3-variant TypedDict union, with `display`) | present; exact `.d.ts` declaration **not re-verified this pass** (GitHub raw 404'd; TS reference confirmed `maxThinkingTokens` is `@deprecated: Use thinking instead` and that thinking "defaults to `{ type: 'adaptive' }` for supported models") | shape inferred to match Python by parity; flagged in Caveats |
| `effort` | `EffortLevel` only (top-level) | `'low'\|'medium'\|'high'\|'xhigh'\|'max'`, default `'high'` | parity |
| `effort` (per-agent) | `EffortLevel \| int` | `... \| number` | numeric form is agent-level only |

---

## 4. Per-subagent / per-role model (restated from topic 06)

**Mechanism: set `model` (and optionally `effort`) on the `AgentDefinition`** passed via the `agents` option, or on a filesystem `.claude/agents/*.md` card's frontmatter. This is the lever for pinning a cheaper model to a reviewer/verifier role.

`AgentDefinition.model` (Python, `types.py:91-92`):

```python
    # Model alias ("sonnet", "opus", "haiku", "inherit") or a full model ID.
    model: str | None = None
```

- **Accepts:** alias (`"sonnet"`/`"opus"`/`"haiku"`), `"inherit"` (use the parent's model), or a full ID. **Defaults to the main model** if omitted (topic 06 §7).
- **`AgentDefinition.effort`** (`types.py:100`) — `EffortLevel | int | None`, per-agent reasoning depth, independent of the parent.
- The canonical use case is directly documented (topic 06 §7): a read-only reviewer on a cheaper model — `AgentDefinition(model="haiku", effort="low", tools=["Read","Grep","Glob"])` while the parent runs Opus at high effort.

**Resolution order for a subagent's model** (topic 06 §7, from the Claude Code subagents doc): `CLAUDE_CODE_SUBAGENT_MODEL` env → per-invocation `model` param → the definition's `model` (frontmatter/`AgentDefinition`) → the main conversation's model.

> **For Ark:** the reviewer/verifier roles can each carry their own `model` + `effort`. Because subagent dispatch is model-decided and returns text only (topic 06 §2, §3), pinning a cheap model to a *separate role `query()`* is the deterministic alternative — the same `model`/`effort`/`thinking` options apply to that separate top-level `query()`.

---

## 5. `max_thinking_tokens` / `maxThinkingTokens` — deprecated

**Exact names:** Python `max_thinking_tokens`, TS `maxThinkingTokens`. **Both are DEPRECATED.**

**Python** (`types.py:1851`):

```python
    max_thinking_tokens: int | None = None
    """Maximum tokens the model may use for its thinking/reasoning process.

    .. deprecated::
       Use ``thinking`` instead. On newer models, this is treated as on/off
       (0 = disabled, any other value = adaptive). For explicit control, use
       ``thinking={"type": "adaptive"}`` or
       ``thinking={"type": "enabled", "budget_tokens": N}``.
    """
```

**TypeScript** (TS reference): `maxThinkingTokens?: number;` — doc comment `*Deprecated:* Use thinking instead. Maximum tokens for thinking process`.

- **Type:** `int` / `number`.
- **Default:** unset (`None`/`undefined`).
- **Relation to `effort`:** none directly — `max_thinking_tokens` is a *token ceiling*, `effort` is a *depth hint*. The replacement for a numeric ceiling is `thinking={"type":"enabled","budget_tokens":N}`.
- **Relation to `thinking`:** **`thinking` takes precedence when both are set** ("When set, takes precedence over the deprecated `max_thinking_tokens`," §3.1). On newer models `max_thinking_tokens` collapses to on/off semantics (0 = disabled, nonzero = adaptive), losing its fine-grained budget meaning.

> **For Ark:** do **not** write new code against `max_thinking_tokens`. For a hard thinking-token budget use `thinking={"type":"enabled","budget_tokens":N}`; for "let the model decide" use the default adaptive thinking and shape it with `effort`.

---

## 6. Structured output — `output_format` / `outputFormat`

**It exists** (the `error_max_structured_output_retries` subtype implied it; confirmed here).

### 6.1 The option (verbatim)

**Python** (`types.py:1889`):

```python
    output_format: dict[str, Any] | None = None
    """Output format configuration for structured responses.

    When specified, the agent returns structured data matching the schema.
    Matches the Messages API structure, e.g.
    ``{"type": "json_schema", "schema": {"type": "object", "properties": {...}}}``.
    """
```

**TypeScript** (`Options`, TS reference):

```typescript
outputFormat?: { type: 'json_schema', schema: JSONSchema };
```

- **Names:** Python `output_format`, TS `outputFormat`.
- **Shape:** `{ "type": "json_schema", "schema": <JSON Schema object> }`. The `schema` can be hand-written, or generated from **Zod** (`z.toJSONSchema(...)`, TS) or **Pydantic** (`Model.model_json_schema()`, Python).
- **Default:** unset → agent returns free-form text as usual.

### 6.2 Where the result lands

On the terminal **`ResultMessage`** (Python) / **`SDKResultMessage`** (TS), field **`structured_output`** (snake_case in both; TS declares `structured_output?: unknown`). Read it only when `subtype == "success"`:

```python
if isinstance(message, ResultMessage) and message.structured_output:
    print(message.structured_output)  # dict matching your schema
```

The SDK validates the model's output against the schema and **re-prompts on mismatch** before surfacing it.

### 6.3 Retry behavior + the error subtype

- The SDK re-prompts on validation failure up to an **internal retry limit**. If validation never succeeds within that limit, the result is an **error** instead of data.
- The terminal `ResultMessage.subtype` is then **`error_max_structured_output_retries`** — this is the **fourth** error subtype alongside `error_during_execution`, `error_max_turns`, `error_max_budget_usd` (topic 08 §5.3 lists the union). Per the structured-outputs doc table: *"Agent couldn't produce valid output after multiple attempts."*
- **There is NO documented option to configure the retry count.** The TS `Options` type has no `maxStructuredOutputRetries`-style field, and none appears in the Python `ClaudeAgentOptions`. The retry limit is internal/fixed. (Flagged in Caveats.)

### 6.4 Constraints

- **Supported JSON Schema features (verbatim):** *"all basic types (object, array, string, number, boolean, null), `enum`, `const`, `required`, nested objects, and `$ref` definitions."* For the full supported-feature list and limitations the doc defers to the API structured-outputs page (`platform.claude.com/.../structured-outputs#json-schema-limitations`).
- **Top-level only — NOT subagents.** Structured output applies to the top-level `query()`; subagents return free-form text with no structured channel (topic 06 §3.2, open issue [TS #104]). A tier-classifier that wants JSON must run as its **own top-level `query()`** with `output_format`, not as a subagent.
- **Compatible with multi-step tool use:** the agent may call any tools (Grep, Bash, WebSearch, …) during the run and still emit validated JSON at the end — the doc's TODO-tracker example demonstrates exactly this. No incompatibility with tools is stated.
- **Streaming:** no documented incompatibility with streaming; `structured_output` simply arrives on the terminal result message like any other result field.

> **For Ark's tier-classifier:** run a dedicated top-level `query()` with `output_format={"type":"json_schema","schema": <tier schema>}` (top-level type `object`, the classification field `required`). Handle `subtype == "error_max_structured_output_retries"` as a hard failure (retry with a simpler prompt or fall back to a default tier) — you cannot tune the retry count, so keep the schema small and the prompt unambiguous.

### 6.5 Python ↔ TS divergence (structured output)

| Aspect | Python 0.2.87 | TS 0.3.150 |
| ------ | ------------- | ---------- |
| option name | `output_format` | `outputFormat` |
| option type | `dict[str, Any]` (untyped dict) | `{ type: 'json_schema', schema: JSONSchema }` (typed) |
| result field | `ResultMessage.structured_output` | `SDKResultMessage.structured_output` (`unknown`) |
| schema gen helper | Pydantic `.model_json_schema()` | Zod `z.toJSONSchema()` |
| error subtype | `error_max_structured_output_retries` | same |
| retry-count option | none | none |

Note the typing asymmetry: TS gives a precise `{ type: 'json_schema', schema }` literal; Python takes a loose `dict[str, Any]`. Validate the dict shape yourself in Python.

---

## 7. Provider model-ID differences (cross-ref topic 01)

The `model` string is **provider-specific**, and the SDK does not translate it for you on the alternate routes. Topic 01 §4 established the routing env vars (`CLAUDE_CODE_USE_BEDROCK=1`, `CLAUDE_CODE_USE_VERTEX=1`, `CLAUDE_CODE_USE_FOUNDRY=1`, `CLAUDE_CODE_USE_ANTHROPIC_AWS=1`); all serve **Claude weights only** (no non-Claude models). The practical model-ID consequences:

- **Direct Anthropic API:** plain IDs like `claude-sonnet-4-5`, `claude-opus-4-5`, and dated IDs (`claude-opus-4-1-20250805`).
- **Amazon Bedrock:** Bedrock uses its own model identifiers (region-/inference-profile-qualified, e.g. an `anthropic.claude-*` or `us.anthropic.claude-*` form). The exact ID for a Bedrock deployment is set via the CLI's model env, not by passing a short alias.
- **Google Vertex AI:** Vertex uses its own model-name strings as well.
- The default model is resolved by the **bundled CLI**, which on these routes honors environment configuration (e.g. `ANTHROPIC_MODEL`, and a small/fast model var `ANTHROPIC_SMALL_FAST_MODEL` for the cheap-helper model) rather than a value baked into the SDK.

> **For Ark:** if a substrate must run on Bedrock/Vertex, the `model` and `fallback_model` values are **not portable** from the direct-API IDs — they must be the provider's model identifiers. Keep model IDs configurable (don't hardcode `claude-opus-4-5`) and let the deploy environment supply the provider-correct ID. (Exact Bedrock/Vertex ID strings were **not** pulled verbatim this pass — see Caveats.)

---

## External references

- [Agent SDK reference — TypeScript (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/typescript) — `Options.model` ("Default from CLI"), `fallbackModel` ("Model to use if primary fails"), `effort` (`'low'|'medium'|'high'|'xhigh'|'max'`, default `'high'`), `maxThinkingTokens` (`@deprecated: Use thinking instead`), `outputFormat: { type: 'json_schema', schema: JSONSchema }`, `structured_output?: unknown`, `setModel(model?): Promise<void>` ("only available in streaming input mode"). (fetched 2026-05-25)
- [Get structured output from agents (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/structured-outputs) — `outputFormat`/`output_format` quick start, Zod/Pydantic schema gen, `structured_output` result field, the `success` vs `error_max_structured_output_retries` subtype table, "re-prompting on mismatch," supported JSON Schema feature list, error-avoidance tips. (fetched 2026-05-25)
- `claude-agent-sdk-python` **0.2.87**, local clone `src/claude_agent_sdk/types.py` — VERBATIM source of: `model` (`:1673`), `fallback_model` (`:1679`), `thinking` + `ThinkingConfig` union + `ThinkingDisplay` (`:1555-1575`, `:1861`), `effort` + `EffortLevel` (`:33`, `:1874`), `max_thinking_tokens` deprecation (`:1851`), `output_format` (`:1889`), `AgentDefinition.model` alias comment (`:91`) and `AgentDefinition.effort` (`:100`), `SdkBeta` (`:29`, `context-1m-2025-08-07`). Version confirmed `pyproject.toml:7`. (read 2026-05-25)
- `claude-agent-sdk-python` `src/claude_agent_sdk/client.py:346` — `ClaudeSDKClient.set_model(model=None)` docstring with example IDs (`claude-sonnet-4-5`, `claude-opus-4-1-20250805`, `claude-opus-4-20250514`), "only works with streaming mode."
- Cross-corpus: topic 01 (provider routing/lock-in), topic 06 (per-`AgentDefinition` model/effort override, subagent structured-output gap [TS #104]), topic 08 (`error_max_budget_usd`/`error_max_turns`/`error_during_execution` subtypes; `modelUsage` cost split).
- Anthropic docs referenced by the docstrings (not fetched this pass): `build-with-claude/adaptive-thinking`, `build-with-claude/effort`, `platform.claude.com/.../structured-outputs#json-schema-limitations`.

---

## Caveats / Not found

- **TS `thinking` exact `.d.ts` declaration not re-verified this pass.** The GitHub raw `sdk.d.ts` path 404'd, and the TS reference page describes `thinking` only as "defaults to `{ type: 'adaptive' }` for supported models" while confirming `maxThinkingTokens` is `@deprecated: Use thinking instead`. The full TS `thinking` union (`adaptive`/`enabled`/`disabled` + `display`) is **inferred to match the Python `ThinkingConfig`** by SDK parity, not read verbatim from TS source. Re-verify against the 0.3.150 `package/sdk.d.ts` (npm tarball) if exact TS shape is load-bearing.
- **No documented default model ID.** Both SDKs say only "Default from CLI" / "the CLI default model." The concrete current default (which Opus/Sonnet build) is **not printed** in the SDK docs and is resolved by the bundled CLI + env. Do not assume a specific default.
- **`fallback_model` trigger conditions are coarse.** "If the primary model fails or is unavailable" (Python) / "if primary fails" (TS) — the docs do **not** enumerate which failure classes (overload/429, 5xx, context-length, content filter) trigger fallback vs. erroring. Treat the trigger set as undocumented detail.
- **Structured-output retry count is not configurable and not numerically stated.** The doc says "multiple attempts" / "within the retry limit" but gives no number and exposes **no `maxStructuredOutputRetries` option**. The limit is internal/fixed.
- **Top-level model aliases (`sonnet`/`opus`/`haiku`) are documented only on `AgentDefinition.model`**, not on the top-level `ClaudeAgentOptions.model` docstring (which shows full IDs). The CLI accepts top-level aliases, so they very likely work top-level too, but that is inference, not a quoted SDK statement.
- **Bedrock/Vertex exact model-ID strings not pulled verbatim.** §7 states the *principle* (provider-specific IDs, non-portable, env-resolved default) from topic 01 + the env-var set; the precise `anthropic.claude-*` / Vertex ID forms were not fetched this pass (fetch budget). Re-fetch the Bedrock/Vertex Agent SDK pages if exact IDs are needed.
- **`effort` numeric form divergence:** per-`AgentDefinition.effort` accepts `EffortLevel | int`; top-level `ClaudeAgentOptions.effort` is `EffortLevel` only (no int). Confirmed in Python source; assume the same agent-vs-top-level split holds in TS.
- **`display` / thinking-text visibility:** Opus 4.7+ defaults `display` to `"omitted"` (signature-only) — thinking text is **not** returned unless `display: "summarized"` is set. This is per a source-code comment (`types.py:1555-1557`), not a prose doc page; the precise model-version boundary ("Opus 4.7+") is from that comment.
- **Doc snapshot only:** `code.claude.com` pages print no per-page "last updated" date; pin is via package versions (Python 0.2.87 confirmed in the local clone; TS 0.3.150 per npm) as of 2026-05-25.
