# Research: Pi as a runtime below ArkOS

- Query: Where, if anywhere, should Pi fit in ArkOS without collapsing the RFC's substrate/runtime boundary? Assess current Pi runtime, server, storage, provider, session/event/compaction, extension, isolation, observability, and self-evolution surfaces; make adopt/prototype/watch/reject calls and define a de-risking experiment.
- Scope: mixed
- Date: 2026-07-22
- Pi source pin: release `v0.81.1`, commit `20be4b18d4c57487f8993d2762bace129f0cf7c6`. The local checkout is `dd6bea41efa8caa7a10fe5a6401676dc5699f83f` (`v0.81.1-1-gdd6bea41`); its only changes from the tag are `CHANGELOG.md` “Unreleased” headings, so the inspected implementation matches the release.

## Findings

### Executive finding

**Fact.** ArkOS is defined as workflow substrate above interchangeable agent runtimes, while Pi describes itself as an agent harness composed of a multi-provider LLM library, an agent loop, a coding-agent CLI/SDK/RPC surface, runtime-session persistence, and an experimental process server (`docs/rfcs/001-arkos.md:76-88`; `reference/pi/README.md:13-20`; `reference/pi/packages/server/README.md:1-5`).

**Inference.** Pi's natural position is the same runtime layer as Claude Code, Codex, and OpenCode. Pi is not an ArkOS implementation and none of Pi's current packages should own ArkOS lifecycle, task-tree, memory, SPEC, grounding, or recursion semantics. Its strongest ArkOS contribution is a replaceable execution engine: provider access, model/tool turns, runtime-local session history, compaction, and a rich live event stream.

**Recommendation.** Prototype a thin, out-of-process adapter directly against the pinned `pi --mode rpc` JSONL protocol. ArkOS remains the system of record and supplies a deterministic context snapshot; Pi executes an attempt inside an external sandbox and emits runtime observations that the adapter translates into ArkOS events. Do not begin with `@earendil-works/pi-server`, the SQLite backend, direct TypeScript SDK embedding, or Pi extensions as enforcement mechanisms.

The intended ownership boundary is:

```text
workload / autonomous orchestrator
             |
             v
ArkOS: lifecycle | task tree | memory | SPEC | context | event log
       grounding | recursion budgets | evaluator protection
             |
       versioned Pi adapter
       (IDs, event mapping, cancellation, recovery)
             |
             v
Pi coding-agent RPC: agent loop | tools | runtime session | compaction
             |
             v
Pi AI providers / model APIs

External sandbox: filesystem, process, network, and credential boundary
External evaluator: success signal outside both Pi and ArkOS's editable region
```

This is an ownership diagram, not a claim that Pi becomes an ArkOS “tool.” The RFC explicitly places hosted coding agents at the runtime layer beneath the substrate (`docs/rfcs/001-arkos.md:85-88`, `docs/rfcs/001-arkos.md:197-208`).

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `docs/rfcs/001-arkos.md` | Defines the ArkOS layer, service boundaries, staged runtime strategy, grounding rules, and unresolved interface questions. |
| `reference/pi/README.md` | Pi package map, permission model, container guidance, supply-chain posture, and MIT notice. |
| `reference/pi/package.json` | Workspace/build graph and Node `>=22.19.0` runtime floor. |
| `reference/pi/packages/ai/README.md` | Provider catalog, provider factories, custom providers, and provider-specific compatibility controls. |
| `reference/pi/packages/agent/README.md` | Core stateful agent loop, context conversion, hooks, tool scheduling, and event flow. |
| `reference/pi/packages/agent/src/harness/types.ts` | Session tree/storage contracts, harness phases, event union, hook results, and execution-environment abstractions. |
| `reference/pi/packages/agent/src/harness/session/session.ts` | Context reconstruction from compaction checkpoints and the active session path. |
| `reference/pi/packages/agent/CHANGELOG.md` | Recent pre-1.0 breaking changes to storage, streaming, auth, and public types. |
| `reference/pi/packages/coding-agent/package.json` | TypeScript/ESM SDK plus RPC entry point; version, dependencies, and Node floor. |
| `reference/pi/packages/coding-agent/docs/rpc.md` | Headless JSONL commands, responses, events, durable entry cursor, compaction, and observability limitations. |
| `reference/pi/packages/coding-agent/src/modes/rpc/rpc-types.ts` | Concrete RPC command/response union; no handshake or protocol-version field. |
| `reference/pi/packages/coding-agent/docs/session-format.md` | Versioned JSONL session tree, automatic migration, compaction/branch summaries, and context reconstruction. |
| `reference/pi/packages/coding-agent/docs/extensions.md` | In-process extension capabilities and explicit full-system-permission trust model. |
| `reference/pi/packages/coding-agent/docs/containerization.md` | Whole-process versus tool-routing isolation patterns and their extension boundary. |
| `reference/pi/packages/coding-agent/examples/extensions/subagent/` | Example subprocess-based subagents, fan-out limits, no child persistence, and shared-cwd behavior. |
| `reference/pi/packages/storage/sqlite-node/` | Newly added Node SQLite implementation of Pi's runtime-session repository. |
| `reference/pi/packages/server/` | Experimental local process supervisor, Unix-socket RPC bridge, JSON instance registry, and optional Radius presence integration. |

### Code patterns

#### ArkOS owns workflow shape; Pi belongs below it

The RFC places runtimes below the sibling Ark/ArkOS substrate layer:

> `docs/rfcs/001-arkos.md:76-88`
>
> ```text
> │  Agent runtimes
> │  - Claude Code, Codex, OpenCode (today)
> │  - native agent runtimes (later)
> │  - LLM API calls
> ...
> ArkOS is the layer above them, providing workflow shape to whatever runtime is in use.
> ```

It separately names the services an ArkOS connection must provide:

> `docs/rfcs/001-arkos.md:100-109`
>
> ```text
> Start work in a structured shape (lifecycle as a service)
> Track parent/child task relationships ... (task tree as a service)
> Remember what it learned ... (memory as a service)
> Read and write SPECs ... (SPEC storage as a service)
> ... append-only stream for audit, replay, metrics, and recovery
> ... grounding signals ... without grading itself
> ... recursion discipline that prevents loops, budget overruns, and unbounded depth
> ```

The boundary rule is explicit:

> `docs/rfcs/001-arkos.md:134-141`
>
> ```text
> The substrate does not provide:
> - Decomposition algorithms.
> - LLM-call semantics.
> - Workload-specific knowledge.
> - Benchmark ownership.
> ... substrate services are workflow shape, application code is workflow content.
> ```

Pi's core data model, in contrast, is a conversation/session tree. Its entries encode messages, model/tool changes, compaction and branch summaries, labels, and a current leaf:

> `reference/pi/packages/agent/src/harness/types.ts:343-432`
>
> ```typescript
> export interface SessionTreeEntryBase {
>   type: string;
>   id: string;
>   parentId: string | null;
>   timestamp: string;
> }
> ...
> export type SessionTreeEntry =
>   | MessageEntry
>   | ThinkingLevelChangeEntry
>   | ModelChangeEntry
>   | ActiveToolsChangeEntry
>   | CompactionEntry
>   | BranchSummaryEntry
>   | CustomEntry
>   | CustomMessageEntry
>   | LabelEntry
>   | SessionInfoEntry
>   | LeafEntry;
> ```

**Inference.** Pi's `parentId` describes conversational history and branch navigation, not recursively decomposed ArkOS tasks. It has no branch worktree/isolation, lifecycle state, SPEC anchor, grounding gate, or inherited recursion budget. Naming both structures a “tree” does not make them substitutable.

#### Pi supplies a useful runtime and event vocabulary

The headless interface is strict JSONL with correlated command responses and asynchronous events:

> `reference/pi/packages/coding-agent/docs/rpc.md:20-37`
>
> ```text
> Commands: JSON objects sent to stdin, one per line
> Responses: JSON objects with type: "response"
> Events: Agent events streamed to stdout as JSON lines
> All commands support an optional id field for request/response correlation.
> ```

The harness exposes state around provider requests, tools, context, compaction, retries, model changes, and settlement (`reference/pi/packages/agent/src/harness/types.ts:521-585`, `reference/pi/packages/agent/src/harness/types.ts:683-708`). Tool lifecycle events carry `toolCallId`, which provides a useful local correlation key (`reference/pi/packages/coding-agent/docs/rpc.md:954-997`).

The durable session view is append-only and cursor-readable:

> `reference/pi/packages/coding-agent/docs/rpc.md:692-720`
>
> ```text
> Get all session entries in append order ... an entry id works as a durable cursor ...
> Unlike get_messages, this includes pre-compaction history and abandoned branches.
> ```

**Inference.** These are good inputs to an ArkOS runtime adapter, but not an ArkOS event log. Live RPC events explicitly have no event ID (`reference/pi/packages/coding-agent/docs/rpc.md:830-857`), and a direct RPC `bash` command is stored in message state but emits no event (`reference/pi/packages/coding-agent/docs/rpc.md:454-512`). Pi sessions omit ArkOS lifecycle transitions, dispatches, grounding checks, commits, rollbacks, and recursion decisions. ArkOS must assign its own monotonic event sequence and durable identifiers as events cross the adapter.

#### Compaction is runtime context cache, not ArkOS memory

Pi session files have an explicitly versioned format and automatically migrate v1/v2 to v3 (`reference/pi/packages/coding-agent/docs/session-format.md:19-27`). A compaction stores an LLM-generated summary plus a retained tail/checkpoint (`reference/pi/packages/coding-agent/docs/session-format.md:227-246`). During context building, the latest compaction replaces earlier active-path content with that summary and retained tail (`reference/pi/packages/agent/src/harness/session/session.ts:59-90`, `reference/pi/packages/agent/src/harness/session/session.ts:103-147`). The complete entries remain queryable through `get_entries`.

**Inference.** This is an effective runtime context-window mechanism and replay aid. It is still lossy model context, so it fits the RFC's “conversation summaries are cache, not source of truth” rule (`docs/rfcs/001-arkos.md:124-129`). ArkOS task intent, SPEC/version anchors, accepted decisions, evaluator references, and recursion budgets must remain structured ArkOS records. The adapter may inject stable IDs or content hashes into Pi context, but must never depend on a generated Pi summary to preserve them.

#### Extensions are powerful but not a protection boundary

Pi extensions can intercept or modify tool calls and context, customize compaction, add tools, and persist extension state (`reference/pi/packages/coding-agent/docs/extensions.md:3-24`). They also execute arbitrary code with full process permissions:

> `reference/pi/packages/coding-agent/docs/extensions.md:109-152`
>
> ```text
> Extensions run with your full system permissions and can execute arbitrary code.
> ... Node.js built-ins (node:fs, node:path, etc.) are also available.
> ```

Pi itself has no built-in filesystem/process/network/credential permission system and recommends containerization or sandboxing (`reference/pi/README.md:37-45`). Its container guide warns that tool routing does not isolate other extensions running in the host Pi process (`reference/pi/packages/coding-agent/docs/containerization.md:3-17`).

**Inference.** A project-trust prompt controls whether repository-provided code loads; it does not reduce that code's authority after loading. Consequently, a Pi extension must not be the sole enforcement point for ArkOS grounding, protected evaluator files, task isolation, recursion budgets, or credential policy. Put the entire Pi process and extension set in an external sandbox, use a controlled Pi home, disable or allowlist auto-discovered extensions, and keep the evaluator outside the writable mount.

#### Pi subagents are an example facility, not recursion discipline

The bundled subagent example launches each child as a separate Pi process for context isolation (`reference/pi/packages/coding-agent/examples/extensions/subagent/README.md:1-12`). It caps a single parallel call at eight tasks and four concurrent children (`reference/pi/packages/coding-agent/examples/extensions/subagent/index.ts:33-36`). Children run with `--no-session` and inherit the selected/default cwd (`reference/pi/packages/coding-agent/examples/extensions/subagent/index.ts:294-296`, `reference/pi/packages/coding-agent/examples/extensions/subagent/index.ts:333-339`).

**Inference.** The example supplies process/context separation and local fan-out throttling, but not an ArkOS task tree: there is no durable parent/child task record, per-branch worktree, global/depth budget, inherited deadline, cycle detection, or substrate halting rule. ArkOS should dispatch and budget children itself; a Pi runtime attempt may execute one assigned node.

### ArkOS service-by-service fit

| ArkOS concern | Relevant Pi surface (fact) | Fit and ownership (inference) | Call |
| --- | --- | --- | --- |
| Runtime turn / tools | `pi-agent-core` provides a stateful loop, tool execution, hooks, and event streaming; coding-agent supplies read/bash/edit/write and SDK/RPC. | Strong runtime implementation beneath ArkOS. ArkOS assigns an attempt and observes it; Pi decides provider/tool-turn mechanics. | **PROTOTYPE** via RPC. |
| Provider portability | `pi-ai` documents many hosted providers plus OpenAI-compatible local endpoints and custom provider factories (`reference/pi/packages/ai/README.md:57-88`, `reference/pi/packages/ai/README.md:948-1058`). | Strong breadth below the boundary. Compatibility flags show that “OpenAI-compatible” is not behavior-identical (`reference/pi/packages/ai/README.md:1116-1135`); test families rather than claiming full semantic neutrality. | **PROTOTYPE** through the Pi runtime; do not make provider semantics an ArkOS service. |
| Lifecycle | Pi harness phase is only `idle | turn | compaction | branch_summary | retry` (`reference/pi/packages/agent/src/harness/types.ts:521`). | No PRD/PLAN/REVIEW/EXECUTE/VERIFY state machine, tier, or transition gate. | **REJECT** Pi ownership; ArkOS only. |
| Task tree / isolation | Session entries and forks form conversation branches; example subagents share cwd unless caller changes it. | Not recursive task identity, focus, worktree isolation, sibling interference control, or recovery. A task may have multiple Pi attempts/sessions; never key ArkOS tasks by Pi session IDs. | **REJECT** substitution; ArkOS only. |
| Memory | JSONL/SQLite retain runtime session entries, summaries, usage, and custom data. | Useful attempt transcript, not working/episodic/semantic/procedural memory or cross-project anchored facts. Import selected facts only through an ArkOS-owned promotion process. | **WATCH** as an ingestion source; **REJECT** as the memory service. |
| SPEC storage | No first-class SPEC/version/drift/gating surface was found. | Passing a SPEC as prompt text loses mutation gates and anchor semantics. ArkOS stores immutable/versioned references and projects read-only content. | **REJECT** Pi ownership. |
| Context surface | Pi accepts prompts/resources/custom messages and reconstructs an active session context with compaction. | Good consumer of an ArkOS context projection, not its authoritative producer. Persist the exact ArkOS projection/hash per attempt. | **ADOPT NOW** this direction of flow. |
| Event log | Rich live agent/tool/compaction/retry events; append-only durable session entries with a cursor. | Valuable raw observations, but live events lack IDs and not all actions emit events; workflow/grounding/commit events are absent. Adapter maps into an ArkOS-owned append-only envelope. | **PROTOTYPE** event bridge; **REJECT** session log as system log. |
| Grounding | Hooks may block/modify tools and extensions can implement gates. | Same-process mutable code is not an independent evaluator and cannot protect its own harness. ArkOS invokes externally protected evaluators and only reports results to Pi. | **REJECT** extensions as enforcement. |
| Recursion | Example extension offers subprocess children and per-call concurrency caps. | No depth/global budget/halting/task durability; context isolation is not workspace isolation. | **REJECT** as recursion primitive. |
| Self-evolution | Extensions can alter runtime behavior and Pi can edit code under its process permissions. | This is extensibility, not independently grounded substrate evolution. Revision comparison and evaluator protection remain outside Pi. | **REJECT** in-process self-grading. |

### Integration surface and TypeScript implications

#### Direct TypeScript SDK

**Facts.** `@earendil-works/pi-agent-core` and `@earendil-works/pi-coding-agent` are ESM TypeScript packages, version `0.81.1`, requiring Node `>=22.19.0` (`reference/pi/packages/agent/package.json:1-17`, `reference/pi/packages/agent/package.json:51-53`; `reference/pi/packages/coding-agent/package.json:1-21`, `reference/pi/packages/coding-agent/package.json:97-99`). The SDK offers the widest typed hook and environment surface.

**Inference.** Direct embedding is attractive for a TypeScript host, but Ark/likely ArkOS core is Rust. Embedding would either move runtime orchestration into Node or force a language bridge while still coupling ArkOS domain code to fast-changing TS types. It also places extensions and substrate adapters in one authority domain.

**Recommendation.** Do not bind ArkOS core to the SDK now. A future Stage-2 runtime service may internally use `pi-agent-core`/`pi-ai`, but it should still implement an ArkOS-owned, language-neutral runtime contract.

#### Coding-agent RPC subprocess

**Facts.** RPC is line-delimited JSON over stdin/stdout and is explicitly intended for embedding in non-interactive applications (`reference/pi/packages/coding-agent/docs/rpc.md:1-37`). It covers prompting, model selection, compaction, abort, session switch/fork, state/stats, durable entry reads, and streamed events (`reference/pi/packages/coding-agent/src/modes/rpc/rpc-types.ts:20-73`).

**Inference.** This is the narrowest viable Rust/TypeScript seam and makes crash containment, stdout capture, resource accounting, and whole-runtime sandboxing straightforward. It is preferable to importing Pi types or adding another daemon for a first experiment.

**Gap.** The command/response union contains no protocol version or capability negotiation (`reference/pi/packages/coding-agent/src/modes/rpc/rpc-types.ts:20-125`). Request IDs correlate responses, but streamed events explicitly lack IDs. The adapter therefore needs an out-of-band binary/version check, its own capability manifest, and tolerant decoding of unknown fields/events.

#### Experimental Pi server

**Facts.** The server README says its CLI, APIs, and behavior may change or disappear without notice (`reference/pi/packages/server/README.md:1-5`). It supervises a coding-agent RPC subprocess per instance, exposes spawn/list/status/stop/RPC/RPC-stream over a local Unix socket, persists machine/instance metadata as JSON, and optionally reports presence to `radius.pi.dev` (`reference/pi/packages/server/src/ipc/protocol.ts:10-50`; `reference/pi/packages/server/src/storage.ts:1-70`; `reference/pi/packages/server/src/radius.ts:8-13`). On server restart, instances previously marked online/starting are rewritten as stopped instead of being resumed (`reference/pi/packages/server/src/supervisor.ts:244-255`). The declared spawn `provider` and `model` fields are not passed to `spawnInstance` by the current handler (`reference/pi/packages/server/src/ipc/protocol.ts:10-16`; `reference/pi/packages/server/src/handler.ts:57-68`). Its parser JSON-casts requests without schema validation (`reference/pi/packages/server/src/ipc/protocol.ts:130-141`).

**Inference.** This is a useful local Pi process supervisor/presence experiment, not an ArkOS substrate service, durable scheduler, authenticated multi-tenant API, or event store. Adding it to the first spike creates a second unstable protocol and duplicates process ownership without solving recovery.

**Recommendation.** Watch it. Reconsider only after it publishes a versioned/capability-negotiated protocol, clear local authorization and schema validation, restart/reconciliation semantics, and stable licensing/deployment expectations. Do not make Radius a required ArkOS dependency.

#### SQLite session storage

**Facts.** `@earendil-works/pi-storage-sqlite-node` was added in `0.81.0`, one day before the inspected release, as a Node `node:sqlite` backend for agent-harness sessions (`reference/pi/packages/storage/sqlite-node/CHANGELOG.md:5-11`; `reference/pi/packages/storage/sqlite-node/README.md:1-5`). It uses WAL, `synchronous=FULL`, and a five-second busy timeout (`reference/pi/packages/storage/sqlite-node/src/sqlite/repo.ts:30-34`). Its initial schema stores sessions, sequenced session entries, branches, and materialized views (`reference/pi/packages/storage/sqlite-node/src/sqlite/migrations/001_initial.sql:1-59`).

**Inference.** This can improve Pi runtime-session concurrency/durability, but its schema is deliberately about Pi sessions, not ArkOS events/tasks/SPECs/memory. Sharing that database would couple ArkOS migrations and invariants to Pi's pre-1.0 storage API.

**Recommendation.** Keep JSONL session persistence for the first one-process spike because the RPC cursor is sufficient. Watch SQLite maturity; if later used, give Pi its own database and ingest through the adapter rather than querying its tables as ArkOS state.

### Stability, portability, and governance

#### API and protocol stability

- **Fact:** All inspected packages are pre-1.0 at `0.81.1`. The agent changelog records breaking `SessionStorage` and stream-function changes in `0.81.0`, followed the same day by `0.81.1` restoring compatibility (`reference/pi/packages/agent/CHANGELOG.md:5-21`). `0.80.0` also replaced the harness authentication/model path (`reference/pi/packages/agent/CHANGELOG.md:81-88`).
- **Fact:** Pi's durable session file has an explicit version and migration path, while RPC and server protocols have no visible version/handshake.
- **Inference:** Session files presently have a stronger compatibility story than public SDK or wire APIs. Pre-1.0 release numbers alone do not prove instability, but the documented breaking changes and server warning make a floating dependency inappropriate.
- **Recommendation:** Pin the exact release commit and npm shrinkwrap/source checksum. Put all Pi types behind an ArkOS-owned adapter contract. Maintain captured protocol fixtures and contract tests; reject unsupported versions at process start. Upgrade deliberately, never through `^0.81.1` resolution in an ArkOS distribution.

#### Provider portability

- **Fact:** Pi normalizes many provider families and offers explicit custom-provider/model/auth APIs. It also exposes numerous provider-specific compatibility flags for role mapping, reasoning controls, tool result shape, streaming usage, and session affinity (`reference/pi/packages/ai/README.md:948-1058`, `reference/pi/packages/ai/README.md:1116-1135`).
- **Inference:** Pi materially reduces provider integration work, but cannot make provider behavior identical. ArkOS should record provider/model/API identifiers and usage, while treating generated content and stop behavior as runtime-specific.
- **Recommendation:** The spike must run the same tool workload through two distinct provider implementations (one may be a deterministic faux/local test provider, one a real hosted family). Success means the ArkOS event envelope and lifecycle result are provider-neutral, not that text or token counts match.

#### License and project direction

- **Fact:** The inspected Pi packages are MIT (`reference/pi/README.md:105-107`; package manifests). Earendil's licensing RFC commits Pi core to MIT while explicitly allowing future adjacent/value-added components, including server-side services, to be Fair Source or proprietary.
- **Inference:** Core-code forkability is strong; relying on Radius or future hosted control-plane features could create a different portability/licensing risk than using Pi core locally.
- **Recommendation:** Depend, if at all, only on the pinned MIT core/runtime packages and an ArkOS-owned protocol boundary. Treat remotely hosted Pi services as optional integrations with a replaceable implementation and separate review. This is engineering risk analysis, not legal advice.

### Observability and recovery contract

Pi can provide model/provider identity, context, streaming messages, tool lifecycle and correlation, retries, compaction, session entries, token usage, and cost. ArkOS needs a superset. The adapter should emit a canonical envelope such as:

```text
event_id, monotonic_seq, occurred_at, observed_at,
arkos_task_id, arkos_attempt_id, runtime="pi", runtime_version,
pi_session_id, pi_entry_id?, request_id?, tool_call_id?,
kind, payload_schema, payload, redaction_policy, context_snapshot_hash
```

**Recommendation.** Persist the ArkOS envelope before downstream publication. Preserve raw Pi event kind and a redacted payload for forensic use, while deriving stable ArkOS event kinds separately. On process death, record an explicit `runtime_attempt_interrupted` boundary. Reconcile durable `get_entries` after restart, deduplicate by Pi entry ID, and label any unobservable live interval instead of pretending complete replay. Capture direct RPC `bash` responses because Pi emits no corresponding event. Never rely on Pi server's instance JSON as the event log.

One ArkOS task may have several Pi sessions/attempts, and one Pi session may be resumed for the same attempt only under an explicit mapping. Store that mapping in ArkOS, not in inferred cwd/session filenames.

### Self-evolution and grounding boundary

The RFC requires external workload grounding, prevents the substrate from editing its evaluation harness, and requires anchored SPEC versions (`docs/rfcs/001-arkos.md:166-181`). Pi extensions can change prompts, context, tools, and compaction inside the same process that performs the work.

**Recommendation.** Pi may propose or execute a candidate substrate change only as an untrusted workload runtime. ArkOS records the candidate revision/configuration, executes a fixed workload suite in a fresh sandbox, and accepts/rejects it from a protected evaluator outside the Pi/ArkOS candidate's writable region. Record evaluator version, workload corpus hash, baseline/candidate runtime pins, random seeds where available, cost, latency, and outcome. Neither a Pi self-review nor an extension hook is a grounding signal.

### Decision calls

| Call | Decision | Rationale / condition |
| --- | --- | --- |
| **ADOPT NOW** | Preserve Pi's placement as an eligible Stage-1 runtime beneath ArkOS, never as the substrate. | It directly follows the RFC layer model and prevents useful Pi runtime features from absorbing ArkOS policy. This is an architecture/documentation decision, not a package dependency. |
| **ADOPT NOW** | ArkOS-to-Pi context flow is one-way authoritative: ArkOS projects structured, hashed context; Pi consumes it and may return observations/candidate artifacts. | Keeps SPECs, task intent, memory, budgets, and evaluator references out of lossy compaction summaries. |
| **ADOPT NOW** | Use Pi session/event concepts as prior art only: append-only entries, stable entry cursors, tool-call correlation, compaction checkpoints, explicit usage/cost. | These patterns are useful, but ArkOS defines its own versioned schemas and IDs. |
| **PROTOTYPE** | Pinned `pi --mode rpc` subprocess behind a thin ArkOS runtime adapter and whole-process sandbox. | Smallest language-neutral seam; exercises actual agent loop, providers, tools, sessions, compaction, events, abort, and recovery. |
| **PROTOTYPE** | Provider switching through Pi with at least two provider implementations. | Tests whether Pi meaningfully reduces runtime/provider coupling without moving LLM semantics into ArkOS. |
| **WATCH** | `pi-agent-core`/`pi-ai` as implementation ingredients for a future Stage-2 native runtime service. | Richest APIs, but TypeScript/Node coupling and current breaking-change cadence are premature for ArkOS core. |
| **WATCH** | `pi-server`, Radius, and `pi-storage-sqlite-node`. | Server is explicitly unstable and not restart-resuming; Radius is optional hosted presence; SQLite is new and models only runtime sessions. |
| **REJECT** | Pi server or agent harness as the ArkOS control plane. | It lacks ArkOS lifecycle, task, SPEC, memory, grounding, recursion, and full audit semantics. |
| **REJECT** | Pi session branches as ArkOS task tree, or Pi JSONL/SQLite as ArkOS memory/event/SPEC storage. | Similar shapes carry different invariants; reuse would erase task isolation, anchors, and audit coverage. |
| **REJECT** | Pi extensions/subagent example as the sole policy, sandbox, grounding, or recursion mechanism. | They run with runtime authority and provide neither protected evaluation nor inherited depth/global budgets/worktree isolation. |
| **REJECT** | Direct SDK types in ArkOS domain APIs or an unpinned RPC/server dependency. | Would couple Rust substrate semantics to a pre-1.0 TypeScript release train and unversioned wire shapes. |

### De-risking experiment

#### Question

Can a pinned Pi process execute one ArkOS-owned task attempt while ArkOS retains every workflow invariant and obtains enough durable, correlated telemetry to recover and audit the attempt?

#### Scope

Time-box to two engineering days and keep it disposable. Build no production integration and do not use Pi server or shared Pi SQLite tables.

1. Pin Pi `v0.81.1` / `20be4b18…`; verify the binary/package version out of band before spawn. Launch `pi --mode rpc --session-dir <isolated-dir>` directly.
2. Run the entire Pi process in a sandbox with a fresh controlled Pi home, no auto-discovered project extensions, one writable task worktree, protected evaluator/SPEC inputs mounted read-only or absent, and explicit network/credential policy.
3. Implement only a mock ArkOS adapter: create `arkos_task_id` and `arkos_attempt_id`, inject a deterministic context bundle plus its hash, correlate RPC commands, wrap every received event/response in the canonical ArkOS envelope, and append it to a separate log.
4. Exercise four trajectories: (a) prompt → tool call → settled, (b) manual/forced compaction followed by another turn, (c) fork/switch and durable `get_entries` cursor replay, and (d) kill during a turn → explicit interruption → restart/reconcile/resume.
5. Execute the same bounded tool task through two provider implementations. Use a deterministic faux/local provider for repeatable adapter tests if necessary, plus one real hosted provider family for live compatibility.
6. Have the mock ArkOS controller create a child task request and enforce depth, fan-out, token/cost, and deadline budgets outside Pi. Confirm an over-budget/depth request is denied before a child runtime starts.
7. Attempt writes to the protected evaluator/SPEC location and outside the worktree. Confirm the sandbox, not an extension prompt/hook, denies them. Record the denial as an ArkOS grounding/security event.

#### Acceptance criteria

- The adapter refuses any Pi version other than the pin and never imports Pi TypeScript domain types into ArkOS-facing types.
- No Pi patch or private source import is required; the adapter uses documented RPC commands/events only.
- Every command response is correlated; every observed live item receives a unique ArkOS ID and monotonic sequence; duplicate durable entries after reconciliation are eliminated by stable Pi entry ID.
- A crash produces a durable, explicit interrupted interval. Durable session entries are recovered through `get_entries`; events that Pi cannot replay are reported as an observability gap, not silently reconstructed.
- Compaction and branching do not alter or lose ArkOS task ID, lifecycle state, context-snapshot hash, SPEC anchor, evaluator reference, or budget because those remain external structured records.
- The canonical ArkOS event schema is unchanged across the two provider implementations. Provider/model IDs, usage, cost, and provider-specific failures remain visible as attributes.
- Tool writes cannot escape the assigned worktree or modify the evaluator/SPEC source. Credentials are scoped to the runtime and absent from stored raw payloads after redaction.
- The external controller, not Pi, rejects the configured recursion limit and emits the corresponding task/grounding event.
- Adapter code contains translation, process supervision, and reconciliation only; no PRD/PLAN/VERIFY, task decomposition, memory promotion, SPEC gating, evaluator grading, or recursion policy is implemented inside Pi hooks/extensions.

#### Stop / reject criteria

Stop the Pi path if any of the following is required for the spike to pass:

- patching Pi or importing non-public internals;
- moving an ArkOS workflow invariant into a Pi extension;
- trusting a compaction/branch summary as the only copy of structured task state;
- granting Pi write access to its evaluator or cross-task state;
- accepting silent event loss or an unbounded/correlation-free attempt after restart;
- making Radius, Pi server, or Pi-owned database tables authoritative;
- provider switching changes the ArkOS lifecycle/event contract rather than only runtime attributes/content; or
- the adapter grows beyond a thin version/capability, process, event-translation, and recovery boundary.

Passing the spike justifies an ArkOS runtime-adapter RFC and a longer reliability/security test. It does not justify adopting Pi server, storage, extensions, or TypeScript SDK as ArkOS substrate components.

### External references

- [Pi release v0.81.1](https://github.com/earendil-works/pi/releases/tag/v0.81.1) — official release page and commit pin; dated 2026-07-21.
- [Pi repository at v0.81.1](https://github.com/earendil-works/pi/tree/v0.81.1) — primary source for the inspected packages and documentation.
- [Pi RFC 0015 — Licensing](https://rfc.earendil.com/0015/) — primary governance statement: core remains MIT; future value-added and server-side services may use other licensing/deployment models.
- [Pi documentation](https://pi.dev/docs/latest) — rolling official documentation; useful for discovery, but the commit-pinned repository is the authority for this report because “latest” can drift.

## Caveats / Not found

- No live provider call, sandbox escape test, crash/replay experiment, latency benchmark, or TypeScript/Rust adapter was run; this is source analysis and the proposed experiment is the next evidence-producing step.
- No RPC or Pi-server protocol version field, capability handshake, formal compatibility promise, durable event ID, authenticated multi-user server boundary, or complete replay of live events was found in the inspected release.
- No Pi implementation of ArkOS lifecycle tiers, structured recursive task tree, worktree isolation, multi-tier memory, anchored SPEC mutation gates, independent grounding evaluator, global recursion budget/depth/halting, or protected self-evolution harness was found.
- Pi's root README package table omits the new server and SQLite packages, although they are present in the v0.81.1 workspace/build and package manifests (`reference/pi/README.md:26-35`; `reference/pi/package.json:5-17`). This reinforces treating those surfaces as young.
- The SQLite backend has one initial migration and was introduced in `0.81.0`; production concurrency, corruption recovery, migration longevity, and multi-process behavior were not benchmarked here.
- The local checkout is one changelog-only commit after the release tag. Implementation citations therefore apply to v0.81.1, but future `main` changes should not be assumed compatible.
- Earendil's licensing RFC is a project-governance statement, not a substitute for checking the exact license and terms of every future package or hosted service.
