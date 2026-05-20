# Trajectory and Event-Log Architecture

The pattern: agents' actions write to an append-only event stream. The stream is the source of truth — UI, audit, replay, fine-tuning data all derive from it. OpenHands ships this; most others don't. Could be a unifying architecture move for Ark.

## OpenHands as the reference

OpenHands' core abstraction is an **event stream** of `Action` and `Observation` events. Every agent decision (call this tool, write this file) is an Action; every result (file contents, command output, error) is an Observation. The agent loop:

1. Read recent events from the stream.
2. Decide next Action.
3. Append the Action.
4. Execute it; append the Observation.
5. Repeat.

The stream is persistent (per session, on disk). Multiple consumers can read it: the agent itself, the UI, an evaluator, a debugger.

**What this enables:**
- **Replay.** Re-run a session by feeding the prior stream to the model. Useful for debugging, regression testing.
- **Audit.** Every action is logged with metadata (timestamp, agent ID, parent action).
- **Condensers.** Compress old events into summaries; insert the summary as a synthetic event.
- **Fine-tuning data.** Successful sessions become training examples.
- **Sub-agent coordination.** Parent and child share the same stream; child's events visible to parent.

The cost: every action has to be a recordable event. Schema discipline matters.

## What Ark has today

Half of an event log, scattered:

- **`task.toml`** — captures task state changes (phase transitions, timestamps).
- **Journal files** — per-developer log of session summaries (one entry per task commit).
- **Git history** — code changes, commit messages.
- **Artifact files** — PRD.md, PLAN.md, VERIFY.md, NN_REVIEW.md — the *outputs* of each phase.

These are the *summary layer*. The *event layer* (every tool call, every read, every write) lives only in the host platform's session JSONL — which Ark doesn't read or own.

## What an Ark event log would look like

A `.ark/.events.jsonl` file (or per-task `.ark/tasks/<slug>/events.jsonl`). Each line is a JSON-encoded event:

```json
{"ts":"2026-05-20T12:00:00Z","kind":"task_new","slug":"foo","tier":"deep","trigger":"cli"}
{"ts":"2026-05-20T12:01:23Z","kind":"phase_transition","slug":"foo","from":"design","to":"plan"}
{"ts":"2026-05-20T12:02:45Z","kind":"subagent_dispatched","slug":"foo","subagent":"ark-researcher","topic":"foo-bar"}
{"ts":"2026-05-20T12:05:11Z","kind":"subagent_returned","slug":"foo","subagent":"ark-researcher","files_written":["..."]}
{"ts":"2026-05-20T12:10:33Z","kind":"task_commit","slug":"foo","commit_sha":"abc123"}
```

Schema-versioned (`schema: 2` field per line); rotated periodically (one file per month); structured for grep + jq + tools.

**Captured events** (illustrative subset):
- `task_new`, `task_commit`, `task_archive`, `task_discard`, `task_promote`, `task_resume`
- `phase_transition`
- `subagent_dispatched`, `subagent_returned`, `subagent_failed`
- `spec_extracted`, `spec_registered`, `spec_modified`
- `worktree_created`, `worktree_cleaned`
- `verify_passed`, `verify_pending`
- `commit_attempted`, `commit_succeeded`, `commit_failed`
- `init`, `upgrade`, `unload`, `load`, `remove`

These are *workflow events*, not tool-call events. (Tool-call events live in the host platform's session log.)

## What the event log buys

### Replay and debug

`ark replay --events <path>` could re-render the workflow state at any prior point. Useful for:
- Debugging "what did Ark do here?" without grepping multiple files.
- Reproducing bug reports from a user's event log.
- Demonstrating a workflow run.

### Audit trail

Currently auditability requires reading the journal + git log + task.toml. An event log unifies them. For enterprise users: "show me every Ark action by user X this week".

### Metrics

Aggregate events to compute:
- Tier usage distribution.
- Average iterations per deep task.
- Verify pass / fail rates.
- Subagent dispatch counts.

`ark metrics` could surface these. Today the data is recoverable from journals + task dirs but not aggregated.

### Fine-tuning data

Successful workflow runs (where deep-tier converged in 1–2 iterations, SPECs extracted cleanly, VERIFY passed) become training examples for a hypothetical Ark-tuned model. Not on the near-term roadmap, but the event log makes it possible.

### Cross-section state

An event log makes `ark context` more responsive — instead of re-scanning task.toml + journals on every call, read the event log's tail. Faster, simpler.

## The costs

### Schema discipline

Every state-changing operation has to *also* emit an event. Cross-cutting concern. Easy to forget; tests catch some misses.

### Storage growth

Per-task: ~20–100 events. Per-month: hundreds to thousands. Manageable on disk; could need rotation policy.

### Atomic writes

Event append must be atomic per line (POSIX O_APPEND covers this). A torn write would corrupt the parser. Mitigation: small-line schema; reject malformed lines in reader.

### Multi-checkout coordination

Two worktrees writing to the same event log = potential interleave. Mitigation: per-checkout event logs OR a single lock-protected log (mirroring `.state.toml`'s pattern).

### Backward compatibility

Event schema changes break old readers. Mitigation: schema versioning per line.

## Architectural fit

An event log fits Ark's existing patterns well:

- **Append-only:** like git, like journals.
- **Plain text (JSONL):** like markdown artifacts, easy to grep.
- **Per-checkout state:** mirrors `.state.toml`.
- **Locking when needed:** same pattern as `state_mutate`.
- **Schema versioning:** mirrors `Snapshot::SCHEMA_VERSION`.

A `crates/ark-core/src/state/events.rs` module fits the existing `state/` directory naturally.

## Migration cost

If event log is added now (vs. later):
- Lower coupling cost — fewer features depend on alternative state sources.
- One-time cost: ~1 week of dev to wire events into existing command implementations.
- Test coverage: integration tests for "every operation emits an event".

Adding later means retrofitting more code; the cost grows with the codebase.

## What this corpus exercise revealed

The research-tier dispatch had a failure recovery story that the event log would have improved:

- 4 sub-agents failed mid-way; the parent (main session) had to inspect disk to learn what succeeded.
- An event log with `subagent_dispatched` / `subagent_returned` / `subagent_failed` events would have made the recovery state queryable without disk-scanning.
- The decision to re-dispatch vs. write-in-session would be cleaner with an event log to drive it.

This is a real use case where the event log pays off near-term, not just long-term.

## Failure modes

1. **Events fire but don't reach disk.** Crash between operation and append. Mitigation: append before commit (eventually-consistent model).
2. **Replay gives wrong state.** Event log doesn't capture all relevant state; replay diverges from actual. Mitigation: complete event coverage; tests.
3. **Event schema drifts.** New events added; old readers don't recognise them. Mitigation: forward-compatible JSONL parsing.
4. **Storage runs away.** Long-running projects accumulate huge event logs. Mitigation: rotation policy (per month / per N events).
5. **Replay doesn't capture model output.** Re-running gives different model output; replay only goes so far. Mitigation: treat replay as workflow-state replay, not full conversation replay.

## Comparison to peers

| Tool | Event log? | Shape |
| ---- | ---------- | ----- |
| OpenHands | Yes (full) | Action/Observation stream; canonical reference |
| Cline | Partial | Checkpoint log; not event-stream-shaped |
| Aider | No | git is the audit trail |
| Claude Code | Partial | Session JSONL captures conversation; not workflow events |
| Codex | Partial | Similar to Claude Code |
| Devin | Yes | Internal session logs; closed |
| Ark | Half | task.toml + journals; not unified |

Ark adding a full event log would put it ahead of every CLI peer except OpenHands.

## Directions for Ark

1. **Spec out an event log feature.** A `docs/rfcs/00X-event-log.md` proposing the schema, capture points, replay semantics. Concrete enough to evaluate; non-binding until accepted.

2. **Start with subagent dispatch events.** The failure-recovery use case is real and demonstrated by this corpus. `subagent_dispatched` / `subagent_returned` / `subagent_failed` would have made re-dispatch decisions cleaner.

3. **Phase-transition events as the next batch.** `task_new`, `phase_transition`, `task_commit`, `task_archive` — easy to capture in existing command implementations; high value for `ark metrics`.

4. **Keep event log per-checkout, lock-protected.** Mirror `.state.toml`'s pattern: one event log per worktree, locked at append. Avoids multi-checkout interleave.

5. **Defer fine-tuning use case.** Event log can serve replay/audit/metrics today; fine-tuning is a 2027 use case. Don't optimise prematurely for it.
