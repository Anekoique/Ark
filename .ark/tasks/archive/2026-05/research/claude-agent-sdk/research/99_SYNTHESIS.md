# Claude Agent SDK — Synthesis (for ArkOS stage-1)

> Snapshot: 2026-05-25 · Python `claude-agent-sdk` 0.2.87 (bundles Claude Code CLI 2.1.150) · TS `@anthropic-ai/claude-agent-sdk` 0.3.150 — both latest at snapshot.
> This file reads across all 14 topic files (01–14) and connects them to ArkOS stage-1 (RFC 001, `docs/rfcs/001-arkos.md`). It introduces **no new SDK claims** — every assertion cites a per-topic file by number and section. It is a *reading*, not a design doc; the ArkOS architecture decisions belong to a follow-up implementation task.

---

## 0. The one-sentence finding

The Claude Agent SDK is **"Claude Code as a library"** (01 §1) — it hands you a production-grade agent loop, tool inventory, hooks, session persistence, an MCP *client*, and one level of subagent — but it is **not** an orchestrator, a task system, a memory layer, a portable substrate surface, or a multi-tenant runtime; so ArkOS stage-1 is best understood as **the Claude SDK as the per-Claude runtime driver, with the substrate (lifecycle state machine, task tree, grounding, event log, MCP-publish surface) built around it and owning every cross-session and cross-vendor concern.**

---

## 1. What the SDK gives ArkOS for free

These map cleanly onto RFC-named substrate services — adopt them, don't rebuild them:

| RFC substrate service | SDK primitive that covers it | Source |
|---|---|---|
| Lifecycle (within one phase) | The autonomous agent loop — `query()` runs prompt→tools→result without you implementing tool orchestration | 01 §1, 03 §2 |
| Event log (raw) | Typed message stream (`AssistantMessage`/`ToolUseBlock`/`ToolResultBlock`/`ResultMessage`) + persisted JSONL transcript | 03 §1, 02 §5 |
| Context surfaces | `settingSources` auto-loads `CLAUDE.md` + `.claude/rules/*` + skills + commands | 02 §8, 11 §3 |
| Grounding-signal *hook point* | `PostToolUse` `additionalContext`/`updatedToolOutput` — the surface to splice a grounding signal the model reads next turn | 04 §4.2, 10 (gap §10.2) |
| Tool gating | `allowedTools`/`disallowedTools` + `permissionMode` + deny-rules + `PreToolUse` hook | 05 §2–§3, 04 §0 |
| Per-role model assignment | `AgentDefinition.model` + `.effort` (cheap model for review, capable for execute) | 12 §4, 06 §7 |
| Cost insight (per query) | `ResultMessage.total_cost_usd` + `modelUsage` + `duration_ms`/`duration_api_ms` | 08 §1, 13 §4.1 |
| Observability (export) | OTLP traces/metrics/logs via the bundled CLI; auto W3C trace-context propagation | 13 §2 |
| Structured output (classifier) | `output_format`/`outputFormat` json_schema → `ResultMessage.structured_output` | 12 §6 |
| Custom tools (in-process) | `tool()` + `create_sdk_mcp_server` — give the agent ArkOS-specific tools in-process | 07 §4, 05 §6 |

That is roughly nine of the RFC's service categories at least *partially* covered. The SDK is a genuine accelerator for the per-runtime layer.

---

## 2. The six stage-1 blockers (must solve before the substrate functions)

Carried verbatim from 14's prioritized table; restated here with the architectural consequence:

### B1 — Dispatch is model-decided, not host-deterministic (14 §1.2, 06 §2)
Inside one `query()`, the *parent model* chooses whether to call the `Agent` (subagent) tool; host code cannot force "run ark-reviewer now." → **ArkOS dispatches each role (reviewer, verifier, executor) as its own `query()`/session**, pinned via the `agent` setting. The host owns the call graph. This is the single most structure-determining finding: it pushes ArkOS toward *host-orchestrated separate sessions*, not in-session subagents.

### B2 — No file-write coordination between concurrent sessions (14 §6.1, 09 §3, 05 §5)
Two sessions in one `cwd` race on the git working tree with no SDK lock; permission rules are not OS enforcement (a raw subprocess bypasses them). → **Worktree-per-task is mandatory** for any write fan-out. Ark already has this primitive (`worktree` feature); ArkOS inherits it.

### B3 — No 429 retry; Python 0.2.87 crashes on rate-limit (14 §6.3, 09 §7)
A single 429 kills the subprocess fatally (fix PR #973 unmerged). A long autonomous run is destroyed by one rate-limit hit. → ArkOS must add host-side **429 detect + backoff (respect `Retry-After`) + resume-by-`session_id`**, plus a concurrency semaphore sized to the API tier.

### B4 — Budget cap is per-`query()` only; no cumulative (14 §7.1, 08 §6.1)
`max_budget_usd` caps one query; the docs say "accumulate the totals yourself." A multi-phase run has no native cumulative ceiling. → ArkOS reads `total_cost_usd` off every `ResultMessage` (present on success *and* error), accumulates host-side, gates the next phase, sets per-query `max_budget_usd` to remaining headroom as defense-in-depth.

### B5 — No external-grader integration (14 §10.2, 04 §4.2)
There is no grading API; the only injection surface is a `PostToolUse` hook returning `additionalContext`/`updatedToolOutput`. → ArkOS must compute its own grounding signal (tests, linters, type-checkers, spec-conformance) and feed it via that hook or a follow-up `query()`.

### B6 — Self-grading bias: Claude-grades-Claude (14 §10.3, 06 §3)
The SDK's natural verification pattern is a same-family model reviewing its own work, returning free-form text (no structured verdict; a parent may *summarize* a subagent's result). → ArkOS's verification must rest on **deterministic external gates** the model can't talk past (`cargo test`/`clippy`, spec checks), with a **structured verdict read from disk** (the verifier-writes-`VERIFY.md`-then-host-gates pattern). This is exactly the RFC's "judge independent of the generator" commitment, and Ark already embodies it.

**Reading:** four of six blockers (B2 worktrees, B4 cumulative-budget-accumulator, B6 disk-verdict gating, and the lifecycle SM behind B1) are **patterns Ark already has**. The SDK port mostly relocates existing Ark discipline into an SDK-driven loop; it does not invent it. The genuinely new host code is B1's separate-session dispatcher, B3's 429 survival, and B5's grader-injection wiring.

---

## 3. The MCP question — decisive for the RFC's "MCP-first substrate surface"

The RFC names MCP as the intended portable substrate surface. The corpus answers the architecture fork definitively (07 §5, confirmed from source, not inferred):

- The Agent SDK is an MCP **client** (consumes external servers via `mcp_servers`, stdio/SSE/HTTP) and an **in-process tool host** (`create_sdk_mcp_server` — tools consumed by *this* SDK's own agent).
- The Agent SDK **cannot publish a standalone MCP server** other agents dial into — no listening transport, the `instance` is stripped before crossing any boundary.
- It surfaces **tools only** from consumed servers — not resources, not prompts (07 §3).

→ **Consequence for ArkOS:** the substrate's MCP-publish surface (the thing foreign agents call to use ArkOS primitives) must be a **separate** server built on standalone `mcp` 1.27.1 / `fastmcp` 3.3.1 / `@modelcontextprotocol/sdk` 1.29.0 — *beside* the Agent SDK, not via it. The Agent SDK already depends on `mcp>=1.23.0`, so the standalone library is in the dependency tree already.

This crystallizes the layering: **ArkOS-the-substrate publishes MCP; the Claude runtime driver consumes MCP (and is itself driven by the Agent SDK).** The SDK sits on the client side of the substrate boundary, never the server side.

---

## 4. The "fully automatic, no user interaction" emulation — what it takes on the SDK

The stage-1 goal is an autonomous workflow run with zero human gates. Mapping the workflow phases to SDK calls:

| Phase | SDK shape | Key knobs | Sources |
|---|---|---|---|
| Tier classify | one-shot `query()` with `output_format` json_schema | structured_output → `{tier,slug,title}` | 12 §6, 02 §4 |
| PRD / PLAN write | `query()`, fresh per phase, `bypassPermissions` for autonomous edits | `permissionMode="bypassPermissions"` + deny-rules + `PreToolUse` scope guard | 05 §3–§4, 04 §0 |
| REVIEW (deep) | **separate** `query()` pinned as `ark-reviewer` agent (deterministic dispatch, B1) | per-role `model`/`effort`; reviewer writes `NN_REVIEW.md`, host reads verdict from disk | 06 §2.3, 06 §3.3 |
| EXECUTE | `query()`, scope-guarded to the worktree | `PreToolUse` deny on out-of-worktree writes (runs even under bypass, 04 precedence) | 04 §0, 05 §5 |
| VERIFY | **separate** `query()` pinned as `ark-verifier` + deterministic gates (B5/B6) | external `cargo test`/`clippy`; verifier writes `VERIFY.md`; host gates on PASS/no-PENDING | 10 (gap §10.3), 06 §3.3 |
| COMMIT | host code (git), not the model | host accumulates cost (B4) and commits | 08 §6.1 |

Autonomy specifics the corpus pins down:
- **No-permission autonomous edits:** `permissionMode="bypassPermissions"` is the `--dangerously-skip-permissions` equivalent (05 §4). Pair with **deny-rules + a `PreToolUse` hook** for the out-of-scope-write guard, because those run *even under bypass* (04 precedence: Hooks → Deny → Mode → Allow → canUseTool).
- **Per-phase fresh sessions vs. continuity:** each phase can be a fresh `query()` (re-grounding from disk artifacts — the current Ark discipline) or a `resume`/`fork` of a prior session (02 §2–§3). Fresh-per-phase is simpler and matches the artifact-as-truth model; session continuity is available if a phase benefits from it (e.g. an EXECUTE-fix loop resuming the EXECUTE session).
- **Turn/loop termination:** there is **no turn-end event** (03 §3) — detect a completed turn by the arrival of a typed `AssistantMessage`, and session completion by the single terminal `ResultMessage`.
- **Short-phase telemetry:** lower `OTEL_*_EXPORT_INTERVAL` or a fast phase drops its telemetry under the 60 s metrics default (13 §2.10).

---

## 5. Three frictions to design against (not blockers, but they shape the build)

1. **Vendor lock to Claude (14 §5.5, 01 §4).** The Agent SDK is Claude-only; the four "providers" are all Claude-hosting surfaces. Building ArkOS *as* an Agent-SDK app would make Anthropic load-bearing — contradicting the RFC's "ArkOS reroutes if a vendor deprecates" (RFC §Stage-1). → Keep the SDK *inside a Claude driver behind an ArkOS runtime interface*; Codex/OpenCode/raw-API are sibling drivers. The cross-model abstraction lives **above** the SDK.

2. **Self-grading is the path of least resistance (14 §10.3, 06 §3).** The SDK makes Claude-grades-Claude the easy default (a subagent reviewing the same model's work, returning summarizable text). The RFC's whole self-improvement discipline (RFC §Self-improvement) depends on the opposite. → ArkOS must *choose* deterministic external gates + disk-read structured verdicts; the SDK won't push you there, you push yourself.

3. **`AGENTS.md` is not loaded (14 §5.4, 11 §4).** The SDK reads `CLAUDE.md`, not `AGENTS.md`, yet the RFC names `AGENTS.md` a portability surface. → If cross-runtime instruction parity matters, the host bridges it (`@AGENTS.md` import inside `CLAUDE.md`, a symlink, or concatenation) — verify empirically.

Plus the operational gotchas: never use the `console` OTel exporter through the SDK (corrupts the message channel, 13 §2.4); isolate `~/.claude/`/`HOME` per call for multi-tenant or even multi-run safety (14 §6.2); don't `break` out of the message iterator early — cancel instead (14 §6.5).

---

## 6. The layering this corpus implies

```
┌──────────────────────────────────────────────────────────────┐
│  ArkOS substrate (own code; publishes MCP via standalone mcp/  │
│  fastmcp — §3)                                                 │
│   - workflow lifecycle state machine        (Ark has it; B1)   │
│   - task tree + focus + cross-task state     (Ark has it; 14§2)│
│   - cumulative budget accumulator            (new; B4)         │
│   - grounding: deterministic external gates  (Ark pattern; B5/6)│
│   - queryable event log (stable schema)      (new; 14§8.3)     │
│   - 429 survival + concurrency semaphore     (new; B3)         │
│   - worktree-per-task isolation              (Ark has it; B2)  │
├──────────────────────────────────────────────────────────────┤
│  Runtime drivers (one per agent runtime; ArkOS runtime iface)  │
│   ┌─ Claude driver — uses the Claude Agent SDK ──────────────┐ │
│   │   query() per phase/role · hooks for scope-guard +       │ │
│   │   grounding inject · bypassPermissions + deny-rules ·     │ │
│   │   OTLP export · structured_output for classify           │ │
│   └──────────────────────────────────────────────────────────┘ │
│   ┌─ Codex driver ─┐  ┌─ OpenCode driver ─┐  (siblings; §5)     │
├──────────────────────────────────────────────────────────────┤
│  Agent runtimes — Claude Code (via SDK) / Codex / OpenCode      │
└──────────────────────────────────────────────────────────────┘
```

The Agent SDK occupies exactly one box: the Claude driver. Everything above it is substrate the SDK does not provide. Everything beside it is the vendor-neutrality the SDK does not give.

---

## 7. Bottom-line recommendation (carried from the facts, not a design decision)

- **Use the SDK as the Claude runtime driver** — it cleanly covers the per-phase agent loop, hooks (scope-guard + grounding inject), per-role model assignment, structured output, and OTLP observability. This is a real, large head start over the current subprocess + hand-parsed-stream-json approach.
- **Build the substrate around it**, not on it: lifecycle SM, task tree, cumulative budget, deterministic grounding, queryable event log, MCP-publish surface, 429 survival, worktree isolation. Most of these Ark already has as patterns; the net-new host code is the separate-session role dispatcher (B1), 429 survival (B3), and grader-injection wiring (B5).
- **Keep the SDK behind a runtime interface** so Claude is not load-bearing for the substrate (vendor-lock friction §5.1) and the RFC's reroute-on-deprecation commitment holds.
- **Publish ArkOS primitives via a standalone MCP server** (`mcp`/`fastmcp`), not via the Agent SDK, which cannot publish (§3).

The honest read of the whole corpus: **the SDK eliminates the per-runtime plumbing that `ark run` hand-rolls, but it provides none of the substrate the RFC defines.** ArkOS stage-1 is "Ark's proven workflow discipline, driven by the SDK instead of by subprocess, with the substrate-only services (cumulative budget, deterministic grounding, recursion-host-side, MCP-publish) added beside it."

---

## 8. Corpus map (where each fact lives)

| File | Owns |
|---|---|
| 01 | SDK identity, packages/versions, Claude Code relationship, provider lock-in, entry points |
| 02 | Sessions: fresh/resume/fork/one-shot, JSONL persistence, `settingSources`, store adapters, cwd binding |
| 03 | Streaming event taxonomy, turn/session boundaries, tool events, errors/aborts, subagent attribution |
| 04 | Hooks: taxonomy, registration, PreToolUse deny/mutate, PostToolUse grounding surface, **precedence order** |
| 05 | Tool inventory, allowedTools/disallowedTools, permissionMode, bypassPermissions, scoped perms, sandbox |
| 06 | Subagents: AgentDefinition vs filesystem, **model-decided dispatch**, **text-only result**, **depth=1**, per-role override |
| 07 | MCP: consume (client) vs **cannot-publish**, transports, **tools-only**, in-process server, namespace |
| 08 | Cost/token surfaces, **per-query budget cap**, maxTurns, **cumulative = host-accumulate**, cost-is-estimate |
| 09 | Concurrency: parallel query(), isolation gaps, cwd collision, subprocess model, **no 429 retry**, cancellation |
| 10 | Memory: auto-memory file, AgentDefinition.memory, **no semantic/vector/retrieval**, compaction recoverability |
| 11 | Skills: `.claude/skills/SKILL.md`, **no programmatic registration**, settingSources, **AGENTS.md not loaded**, portability |
| 12 | Model selection, fallback model, extended thinking (`thinking`/`effort`), structured output, provider model IDs |
| 13 | Telemetry: SDK emits none itself, OTLP passthrough, span taxonomy, trace-context propagation, replay/audit |
| 14 | **The gap inventory** — what the SDK does NOT provide, by substrate concern, with the prioritized severity table |
| 99 | This synthesis — connects all of the above to ArkOS stage-1 |

> Reconciliation note: file 14's caveats say files 10–13 were "not written at this snapshot." They are now all present (10, 11, 12, 13 on disk). File 14's §3–§4 memory facts (sourced from 02) are consistent with 10; its §8 observability facts (fresh-sourced) are consistent with 13; its §5.4 AGENTS.md gap is consistent with 11 §4. No contradictions — 14 was authored before those files landed but its facts hold.
