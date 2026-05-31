# Dispatch Models

How parent agents hand work to child agents — the mechanics, not the rationale. (Rationale lives in `multi-agent-architectures.md` and `researcher-reviewer-verifier.md`.)

## The dispatch verbs in production

| Harness | Dispatch primitive | Shape |
| ------- | ------------------ | ----- |
| **Claude Code** | `Task` tool | Parent calls Task with `description` + `prompt` + `subagent_type`. Synchronous (foreground) by default; `run_in_background: true` flips to async-notify. Up to 10 concurrent. Tool result is the child's final string. |
| **OpenHands** | `AgentDelegateAction` | Standard tool from `openhands.tools`. Parent fills `agent_class`, `inputs`. Synchronous, blocks until complete. Returns structured summary. |
| **Cline subagents** | Markdown `.cline/agents/<name>.md` files | Parent's chat invokes subagent by name; subagent runs in a separate session with its own config (model, tools, turn limit). Returns final assistant message. |
| **Codex CLI** | TOML subagent definitions under `.codex/agents/<name>.toml` | Parent invokes via the platform's subagent tool. Each subagent declares its own model + system prompt + tool permissions. |
| **OpenCode** | TS files in `.opencode/agents/<name>.ts` | Runtime invocation via the agent's task mechanism. Returns string result. |
| **AutoGen** (group chat) | Speaker selection within a `GroupChat` | Not really "dispatch" — the group chat scheduler picks the next speaker per turn. v0.4 introduced explicit handoff for the orchestrator-worker case. |
| **LangGraph supervisor** | `StateGraph` nodes; supervisor edge routes | Programmatic, not LLM-driven; supervisor is a node that decides which worker node to call next based on state. |
| **OpenAI Agents SDK** | `Handoff` primitive | Agent runtime supports `agent.handoff(target_agent, context)`. Designed for the OpenAI Agents Runtime. |
| **CrewAI** | Tasks assigned to Crew members | Each task names an assigned agent; the crew runner serialises. Manager mode generates a manager-agent that does the routing. |
| **Goose** | Subagent recipe invocations | Recipe file declares subagent + invocation parameters; goose CLI runs each. |
| **Ark** | Host platform's subagent tool | Parent (main session) invokes `ark-researcher` / `ark-reviewer` / `ark-verifier` via Claude Code's Task tool, Codex's subagent tool, or OpenCode's equivalent. Foreground by default; background available on Claude Code. |

## Synchronous vs. asynchronous

| Mode | Parent behaviour | Use case | Risk |
| ---- | ---------------- | -------- | ---- |
| **Synchronous foreground** | Blocks; awaits child's return | Short tasks (<2 min); need result before continuing | Idle wait |
| **Asynchronous background** | Continues; receives notification on completion | Long tasks (research, code review); independent work to do | Coordination complexity; "what did I dispatch?" tracking |
| **Fire-and-forget** | Continues; never reads back | Truly independent work (e.g. logging) | Rarely used in coding agents |

Most production harnesses ship sync-foreground as the default; async-background is opt-in. Claude Code added `run_in_background` in 2025; Anthropic's own docs caution: "useful when you have genuinely independent work to do in parallel".

Async-background is the harder mode to design well. The parent needs:
- A way to *enumerate* in-flight children (which it dispatched, what they're doing).
- A way to *receive* completion (notification, polling, await).
- A way to *handle partial results* (if child finishes after parent has moved on).

Claude Code's design: each background dispatch gets an agent ID; the parent receives notifications as system messages. SendMessage to a specific agent ID continues that agent. The harness tracks the in-flight set.

## Foreground vs. background — when to use which

Anthropic's guidance (from the harness brief):

> Foreground: when you need the agent's results before you can proceed.
> Background: when you have genuinely independent work to do in parallel.

In practice:

- **Code-review subagent** → foreground (need verdict to know if PLAN is approved).
- **Research subagent** → either; depends on whether other research can run in parallel.
- **CI poller** → background (polling something the harness can't notify on).
- **Verification subagent** → foreground (need to know PASS/FAIL before commit).

Ark's three subagents fit this:
- Researcher: foreground (parent waits, reads the file back).
- Reviewer: foreground (verdict blocks).
- Verifier: foreground (gate before commit).

For the research-tier parallel-dispatch case (this corpus is being built that way!), foreground-with-parallel-dispatch is the right shape: multiple children, each foreground, all running in parallel, parent awaits all.

## Return contracts

How the child gets information back to the parent:

| Channel | Mechanism | Pros | Cons |
| ------- | --------- | ---- | ---- |
| **String return** | The child's final assistant message becomes the tool result | Cheap, native | Capped by max output tokens; trace lost |
| **Disk persistence** | Child writes file(s); parent reads them | Unbounded size, audit trail | Coordination, atomicity |
| **Structured payload** | Child returns JSON / typed result | Schema-validated | Requires schema discipline |
| **Mixed** | Disk for big artifacts + string summary | Both worlds | More moving parts |

Ark uses *mixed*: researcher writes corpus files to disk *and* the host's Task tool returns a summary string. The parent's main signal is the file existence; the string is a courtesy summary.

This is the safest pattern. Disk persistence means the work survives:
- A failed subagent run (parent inspects what was written).
- A failed parent ingestion (artifacts on disk; re-read later).
- A different session (next session reads the same files).

The string-only pattern is brittle by comparison. AutoGen, CrewAI, OpenAI Agents SDK all lean string-only; works for short tasks, falls over for research-corpus-class work.

## Cost models

Dispatching a child adds cost: an extra inference (or many) plus the parent's tokens to compose the dispatch prompt and parse the return. Rough scaling:

- A simple dispatch: +1× a single-turn inference cost.
- A research subagent with ~50 turns and ~100K-token corpus: +1× a long-session inference cost. Often $0.50–$2.00 per dispatch.
- Parallel dispatch of N children: N× the per-child cost.

The cost trade-off: parent's *context cost* is reduced (child does the heavyweight thinking in fresh context, returns a distillation). For long parent sessions, dispatch is often net-cheaper than letting the parent do everything in one growing context.

Token economics get clearest at scale. Anthropic, Cognition, and Cursor have all written about parallel-agent dispatch as a cost-control lever, not just a parallelism lever.

## Dispatch storms

The most documented failure mode: agent spawns another agent, which spawns another, and so on. Without limits, this is exponential. AutoGen v0.x in unguarded configurations was a notorious offender.

Mitigations:

1. **Recursion guard.** Children cannot dispatch grand-children. Ark's C-15 + C-6: "subagents do not dispatch other subagents." Cline subagents: "cannot spawn their own subagents."
2. **Depth limit.** If recursion is allowed, cap the depth. LangGraph supervisor model has explicit depth bounds.
3. **Budget cap.** Max children per parent per session. Claude Code Task tool caps at 10 concurrent.
4. **Approval gate.** First N dispatches per session require user approval (Cursor Background Agents — first agent requires confirmation, subsequent are free).
5. **Cost ceiling.** Halt when token spend exceeds a threshold (Cursor spend limits, Devin ACU billing).

Production agents converge on **(1) + (3) + (5)**: recursion guard, count cap, cost ceiling. Ark has (1) explicitly (C-15) and (3) implicitly (host platforms cap); (5) is left to the host platform and the user's wallet.

## Implementation lessons from Ark's own dispatch failure (this corpus run)

This research corpus was generated by dispatching 9 parallel sub-agents. 4 of them stalled on watchdog timeout (~10 minutes of activity). Lessons:

1. **Per-agent scope matters more than parallelism.** An agent assigned 8 files in one prompt is at risk of stalling near the file-writing phase; an agent with 2-3 files completes more reliably.
2. **Foreground dispatch survives notification failures better than background.** Background dispatch with completion-notify requires the notification path to work; if it fails, the parent doesn't know. Foreground with explicit await gives the parent direct evidence.
3. **Disk persistence saves work even when dispatch fails.** The 4 stalled agents had landed 7 partial files on disk before dying. Those survived; without disk persistence the entire dispatch would have been lost.
4. **Re-dispatch must be diff-aware.** Re-running an agent that already wrote 2 files should skip those, not overwrite. Ark's research-tier re-dispatch lacks this awareness.

Practical recommendation for Ark's `ark-researcher` brief: cap topic-per-dispatch at 2-3 files for reliability; document the re-dispatch-on-failure recovery pattern.

## Directions for Ark

1. **Cap per-dispatch scope in `ark-researcher` template.** The template should suggest "one topic per dispatch", explicitly. The agent picks scope; the template constrains.

2. **Foreground-by-default; background only for genuinely-independent work.** Update the subagent definitions to emphasise foreground. Background should be reserved for cases where the parent has independent work to do.

3. **Add a `--continue` mode for re-dispatch.** When a research-tier task has partial files on disk, `ark-researcher` should be able to inspect the existing corpus and write only the missing files. Avoids the redundant-re-write problem.

4. **Document the disk-persistence-saves-work pattern in `subagent-support` SPEC.** Currently C-7..C-10 specify write scopes; adding "child must persist incrementally, not only on success" makes the dispatch storms / watchdog scenarios survivable.

5. **Surface dispatch metadata in `ark context`.** When children are dispatched, record their slug / topic / status in a `dispatched_subagents` field. Helps the parent track in-flight work and recover from failure.
