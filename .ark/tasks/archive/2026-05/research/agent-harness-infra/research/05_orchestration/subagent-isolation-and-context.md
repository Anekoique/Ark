# Sub-agent Isolation and Context

Sub-agents are not just specialists — they are *context firewalls*. Each child runs in a fresh context window; the parent never sees the child's reasoning trace. The pattern is *load-bearing for context engineering*, not just orchestration.

## The "context firewall" framing

A parent agent that does everything itself accumulates context per turn — every tool call, every file read, every intermediate thought. By the time it finishes a long task, its context is bloated, the cache is stale, and effective recall drops (lost-in-the-middle).

A parent that dispatches a sub-agent gets a different shape:

- Parent's context = parent's own decisions + sub-agent's return summary.
- Sub-agent's context = sub-agent's own work, never visible to the parent.

The parent stays small. The child runs hot. *Specialisation enables context economy.*

This is why Claude Code's Anthropic-authored docs describe the Task tool as a way to keep the parent's context lean, not (only) as a way to delegate.

## What "fresh context" means

A child sub-agent starts with:
- The system prompt for that subagent type (Cline researcher, Claude Code Explore, etc.).
- The user prompt provided by the parent (the dispatch payload).
- (Optionally) tool definitions tailored to the subagent's scope.

It does NOT start with:
- The parent's conversation history.
- The parent's current tool state.
- The parent's pending tool results.

This is structurally similar to forking a process: shared resources (filesystem) but separate process memory.

## Persistence patterns

How information crosses the parent ↔ child boundary:

### Disk persistence (the safest)

Child writes files to a known path. Parent reads them after the child returns. The child's full reasoning is *not* in the parent's context — only the artifact's content.

Ark's pattern: `ark-researcher` writes `.ark/tasks/<slug>/research/<topic>.md`. Parent reads back via `Read`.

Pros: unbounded size, audit trail, survives failure.
Cons: requires explicit write path, coordination if multiple children write to the same area.

### Structured return (typed JSON / strings)

Child returns a JSON object; parent ingests it as a tool result. Type-checked, schema-validated.

Used by: OpenHands `AgentDelegateAction` (returns structured `inputs/outputs`), LangGraph supervisor (state-dict returns), OpenAI Agents SDK (`handoff_output`).

Pros: machine-parseable; schema guarantees consistency.
Cons: capped by max output tokens; complex schemas hard to maintain.

### Final-message string (the simplest)

Child's last assistant message becomes the tool result string. Parent reads it as text.

Used by: Claude Code Task tool, Cline subagents, Codex subagents (default mode).

Pros: trivial to implement.
Cons: capped by output tokens; lossy if child needed to convey a lot.

### Mixed (disk + string summary)

Child writes big artifacts to disk + returns a string summary. Parent uses both.

Used by: Ark (researcher writes to disk; Task tool returns a summary line).

Pros: best of both — unlimited size on disk + quick parent ingestion.
Cons: coordination if disk path is ambiguous; double-write cost.

## Read-only sub-agents

A common pattern: a sub-agent has *read* tools but no *write* tools (no Edit, no Write, no Bash that mutates).

Examples:
- **Cline subagents** can't write files, can't browse, can't MCP, can't web-search, can't spawn sub-agents.
- **Ark's `ark-reviewer`** is gate-only — verdict in `NN_REVIEW.md`; cannot edit code.
- **Ark's `ark-verifier`** is read-only outside `VERIFY.md`.
- **Claude Code Explore** is read-only by design.

Rationale:
1. **Reduces blast radius** if the sub-agent goes wrong.
2. **Makes parent the integrator** — parent decides what gets applied based on child's reading.
3. **Enables parallel reads** — multiple read-only children don't conflict.

The cost: the child can't fix the things it sees. But that's often a feature; reviewer should not fix, only flag.

## Scoped permissions

Beyond read-only, child sub-agents often have *positively-scoped* permissions: can read these paths, write to these paths, run these commands, nothing else.

Implementations:
- **Cline:** per-subagent tool allow-list.
- **Codex:** TOML config per subagent declares tool permissions.
- **Claude Code:** subagent definition can specify tool restrictions.
- **Ark's `ark-researcher`:** prompt-enforced — "Read-only outside `tasks/<slug>/research/<topic>.md`".

Ark's enforcement is prompt-only; the *real* check is C-28 (post-dispatch git verification). The parent can revert out-of-scope writes after the fact. Belt + suspenders.

## Sandboxed file access

Stronger isolation: child runs in a container / chroot / MCP-roots-bounded filesystem view. Then write-scope is OS-enforced, not prompt-enforced.

- **OpenHands Docker runtime:** child is in its own container; file access scoped to the workspace volume.
- **MCP roots:** the file-system MCP server accepts a `roots` list; tools refuse access outside roots.
- **Devin VMs:** each session has its own VM filesystem; cross-task isolation is OS-level.
- **Ark:** no OS-level sandboxing. Worktrees are file-path isolation, not process isolation.

For Ark this is a real gap. A misbehaving sub-agent could write outside `.ark/tasks/<slug>/research/`; only post-hoc C-28 verification catches it. For untrusted code this would be unacceptable; for Ark's threat model (the user's own coding agent doing roughly what they asked) it is acceptable but not ideal.

## Token / time budgets per child

Different sub-agents can have different budgets:

- Researcher: high token budget, many turns (large corpus to produce).
- Reviewer: medium budget (read PLAN, write review).
- Verifier: low budget (run checks, mark items).

Implementations:
- **Cline:** per-subagent turn limit.
- **Codex:** per-subagent token cap.
- **Claude Code Task tool:** no explicit per-task budget in the public API; container-level limits apply.
- **Ark:** no explicit per-subagent budget today.

A 2026 best practice: declare budgets in the subagent definition; surface them in `ark context`; warn when a child exceeds expected usage.

## Failure modes

### Child finishes silently with no work

Child returns, no files written, no useful return string. Parent has no idea what happened.

Mitigation: contract that child MUST write at least one file or return a non-trivial summary. Surface as a failure if neither.

### Child writes the wrong files

Child writes to paths the parent didn't expect. Subsequent runs trip over them.

Mitigation: declared write scopes (Ark C-7..C-10), post-hoc verification (C-28), or OS sandboxing.

### Child reads stale state

Parent is mid-task; some files have updated. Child reads via tools and gets a partial view. Reasoning is grounded in stale state.

Mitigation: ensure child reads canonical sources (`ark context` is regenerated per-call) + freeze-on-dispatch semantics (dispatch a snapshot, not a live filesystem) where feasible.

### Child returns garbage

The model fails; output is malformed; parent can't ingest. Mitigation: retry policy, defensive parsing.

### Recursion-guard violation

Despite C-15, a child somehow dispatches its own child. Loop or budget blowup.

Mitigation: structural — Ark's subagent prompts say "do not spawn subagents". Some host platforms also surface a depth counter.

## What Ark already does

- **Fresh context per child** (host platform's subagent tool guarantees this).
- **Read-mostly children:** reviewer / verifier are gate-only; researcher writes to a bounded directory.
- **Declared write scopes** in `subagent-support` SPEC (C-7..C-10).
- **Post-hoc git verification** (C-28) reverts out-of-scope writes.
- **Recursion guard** (C-15) — children do not dispatch grandchildren.

## What Ark could add

- **Pre-dispatch scope declaration** — record in `task.toml` or `.state.toml` what each in-flight dispatch is expected to write, with timestamp. Catch divergence proactively.
- **Per-child budget surfaces** — in `task.toml`, per-subagent `[budget]` table with `max_turns`, `max_tokens`, `max_files`. Surface in `ark context`.
- **MCP roots integration** — when the host supports MCP roots, generate a per-dispatch roots list. OS-level enforcement of write scope.
- **Failure-aware re-dispatch** — when a dispatch ends without expected outputs, the parent should know how to surface the failure (note in journal, mark task with warning).

## Directions for Ark

1. **Pre-dispatch scope declaration field.** Subagent template should require the parent to declare expected outputs before invoking the child. Mismatch on completion is a flagged failure.

2. **Per-subagent budget configuration in `task.toml`.** A `[subagents.<name>]` table with `max_turns`, `max_tokens`. Surface in `ark context` so the parent knows the budget before dispatching.

3. **MCP-roots-aware dispatch.** When the host supports MCP filesystem roots, Ark passes per-dispatch root scopes derived from the subagent's declared write zone.

4. **Failure-aware journaling.** When a dispatch returns without expected outputs, append a journal entry: `dispatch.<subagent>.<topic> failed: no expected files written`. Auditable, recoverable.

5. **Document the context-firewall pattern in `subagent-support` SPEC.** Today the SPEC focuses on write scope and recursion guards; adding a "Context isolation" section explains *why* the pattern exists, not just *what* it does.
