# Compaction and Handoff

The strategies for keeping a long-running agent session productive when context overflows or coherence degrades. Two families: *compaction* (summarise older content in place) and *handoff* (spawn a fresh session/sub-agent and persist the relevant artifacts via disk).

## When compaction kicks in

Context window pressure shows up gradually:

- 50–60% capacity — minor effect; cache hit-rate drops as new content displaces cached prefixes.
- 70–85% — noticeable degradation; agent forgets earlier conversation, makes contradictory edits.
- 85–95% — provider-side soft limits in some clients; warnings surface.
- 95%+ — hard limit; either auto-compact fires or the next message rejects.

Claude Code's threshold is documented at ~95%; the auto-compact step replaces old conversation with a digest.

## Compaction implementations

### Claude Code auto-compact (since v2.0.64)

Triggers at ~95% capacity. Mechanism:
1. Summarise the conversation so far via a separate inference (cheap model).
2. Replace older turns with the summary in the new session prefix.
3. Continue.

Loss: detail of older tool outputs, exact phrasing of older user messages. Preserves: high-level decisions, recent context, current tool state.

`/compact` (manual) is the same routine, invoked deliberately at a clean break.

### Strands SummarizingConversationManager

LangChain-adjacent. Drops oldest N turns; summarises the dropped slice; writes the summary back into the prompt. Same pattern, configurable threshold.

### Sliding window (LangChain)

Drop oldest N turns; no summary. Cheaper than summarising; bigger fidelity loss.

### Anthropic memory + clear tool uses

A Claude API cookbook pattern: aggressively drop intermediate tool outputs (cheapest signal) before touching conversation. Keeps decisions intact at the expense of "show your work" traces.

### Semantic message selection

Embed each turn; re-rank by relevance to the current query; drop least-relevant rather than oldest. More accurate; more expensive (embedding per turn).

## Handoff implementations

A different family of strategies: instead of compressing old context, *bypass* it by spawning a fresh session whose only inheritance is what was written to disk.

### Claude Code sub-agent dispatch

Parent dispatches a child via the Task tool. Child has its own fresh context window. Child does work, writes results to a known location (file or return string). Parent's context is unchanged (it gains only the child's return summary, not the child's full trace).

### OpenHands AgentDelegateAction

Standard tool in `openhands.tools`. Parent spawns sub-agent; blocks until complete. Same handoff pattern.

### Cline subagents

Mode-as-role primitive (`.roomodes`). Parent invokes child mode; child works in fresh context; returns a summary.

### Ark research-tier dispatch

`ark-researcher` is invoked from the parent agent (Claude/Codex/OpenCode). Child has fresh context, writes to `.ark/tasks/<slug>/research/<topic>.md`. Parent reads the file back; parent's context grew by the file's content (not the child's reasoning trace).

This is the "*compaction by handoff*" pattern: the child does the heavyweight thinking, the parent gets only the distillation.

### `claude --continue` / session resume

Pure save-and-resume. The session is persisted (JSONL conversation file); a later session loads it. No compaction; the session is structurally the same. Limited by per-session token budget.

### Checkpoint-and-resume (Cursor, Cline)

Edit-level checkpoints: each tool use snapshots changed files. The user can revert. Not a context-compaction primitive — a state-snapshot primitive. See `02_infra_primitives/snapshots-and-checkpoints.md`.

## The compaction vs. handoff trade-off

| Dimension | Compaction | Handoff |
| --------- | ---------- | ------- |
| Fidelity loss | Yes (summary loses detail) | None (disk preserves) |
| Token cost | One extra inference | One full sub-agent run |
| Latency | Mid-session pause | Sub-agent run latency |
| Implementation cost | LLM call + prompt swap | Sub-agent definition + persistence layer |
| User-visible | Often (a pause / message) | Often (a tool-use card) |
| Coherence preservation | Medium (summary may drop subtle context) | High (artifacts are unchanged) |
| Best for | Long single-thread conversations | Multi-step tasks with clean boundaries |

Compaction is the *general-purpose* lever. Handoff is the *workflow-aware* lever — it works when there is a natural task boundary.

## Ark's position

Ark relies almost entirely on handoff:

- **Sub-agent dispatch** for research / review / verify. Child context is fresh; results land on disk. Parent gains the file content only.
- **Phase boundaries** are natural compaction points — moving from DESIGN to PLAN, the agent's context shifts from PRD-writing to PLAN-writing, and `ark context --scope phase --for plan` provides a fresh orientation packet.
- **Deep-tier iteration** uses the latest `NN_PLAN.md` as the canonical state; older plans are still readable but the agent works from the current one. Prior plans are *history*, not *context*.
- **Worktrees** isolate by branch — each task's worktree has its own conversation history with the host agent.

What Ark does *not* do:
- No explicit compaction at intra-phase boundaries.
- No guidance on when to use `/compact` (Claude Code) or its equivalent.
- No multi-session continuity guarantee — if the host agent's session ends mid-phase, the next session starts with `ark context` but no conversation history.

## Failure modes

### 1. Auto-compact eats critical context

If the host agent auto-compacts away the PRD's nuances, the agent forgets *why* it was doing something. Mitigated by structured artifacts (PRD/PLAN persist regardless of conversation state).

### 2. Handoff drops critical context

Child sub-agent doesn't include all relevant detail in its return. Parent acts on incomplete info. Mitigated by *file persistence* (parent can re-read the full corpus on disk).

### 3. Deep-tier `NN_PLAN.md` accumulation

After several iterations, the task dir contains many `NN_PLAN.md` / `NN_REVIEW.md` files. Effective context for the host agent doing EXECUTE has to know which is canonical. The "latest plan is canonical, earlier are history" convention works *if* the agent reads `ark context` first. Otherwise risk of confusion.

### 4. Cross-session amnesia

Host agent session ends. Next session: fresh conversation, `ark context` re-loaded, but the model has no memory of *why* prior decisions were made (only the artifacts they produced). Mitigated by ensuring PLAN documents capture rationale, not just decisions.

### 5. Worktree split-brain

User runs Ark in main checkout + worktree. Each has its own `.state.toml` focus. If sessions cross-talk (open both in different terminals), the agent might confuse which task is active. Mitigated by `ark context`'s `project_root` field.

## What survives boundaries (catalogue)

| Boundary | Survives | Lost |
| -------- | -------- | ---- |
| Auto-compact | Recent turns, summary digest, in-flight tool state | Older turn detail, intermediate tool outputs |
| Sub-agent dispatch | Files written to disk, return summary | Child's reasoning trace, mid-step decisions |
| Phase transition | All artifacts (PRD, PLAN, etc.), `ark context` re-projects | Conversation since last phase boundary |
| Deep-tier iteration | Latest plan, all prior plans on disk | Conversation about why iterations happened |
| Session end / start | `task.toml`, all artifacts, journal entry, `.state.toml` focus | Entire conversation history |
| Worktree switch | Per-worktree state file, separate task | Cross-worktree conversation continuity |

The pattern: *what is committed to disk survives; what is in the conversation does not*. Ark's design embraces this — every meaningful state change is a file change.

## Directions for Ark

1. **Document the compaction-by-handoff pattern.** Ark is doing it; users do not know to articulate it. A `docs/book/src/concepts/context-strategy.md` covering "Ark prefers handoff over compaction" with rationale would teach the model and the user.

2. **Add `ark context --scope resume` for cross-session reorientation.** When a user opens a new session in an in-flight task, give them (and the agent) a structured "where we left off" packet that includes: current task, current phase, latest plan, last 3 verification items resolved, time since last commit. The "session-resume packet".

3. **Cap deep-tier `NN_PLAN.md` accumulation.** When iteration > 5, surface a warning in `ark context` ("PLAN iteration 6 — consider closing the task or splitting"). Reduces split-brain risk where many plans coexist.

4. **Capture conversation rationale in PLAN's `## Log`.** The current `## Log` field is "what changed since last iteration"; expanding to "what changed since last iteration AND why" makes the artifact self-describing across session boundaries.

5. **Detect host-agent auto-compact and warn.** If the host platform exposes "auto-compact fired" as a hook event (Claude Code may), Ark could log to the journal: "Auto-compact ran during phase X; consider re-reading PLAN before continuing."
