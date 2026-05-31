# Orchestration Failure Modes

The catalogue of what goes wrong when agents orchestrate other agents — and the mitigations that have settled into production-grade harnesses.

Every entry below has been seen in shipping systems. Where dates are mentioned, they correspond to documented incidents or reports.

## Failure 1: Dispatch storms

**Symptom:** Agent spawns a child; child spawns a grandchild; grandchild spawns its own children. Exponential blowup. Token budget exhausted in minutes; bill in days.

**Famous case:** AutoGPT (2023). Default config allowed unrestricted child spawning. Reddit threads from 2023-Q2 reported $40+ billed in 30 minutes from a single user.

**Production mitigations:**
- **Recursion guard** (Ark C-15, Cline subagents "cannot spawn subagents", Claude Code Task tool does not nest by default).
- **Depth limit** (LangGraph supervisor `recursion_limit` config).
- **Concurrent cap** (Claude Code Task tool: 10 max).
- **Hard token budget per session** (Cursor spend limits, Devin ACU caps).

## Failure 2: Plan-reinvention loops

**Symptom:** Agent reaches a milestone, summarises, then *re-plans from scratch* on the next turn — forgetting what was already done. Productivity → 0; tokens → ∞.

**Famous case:** BabyAGI (2023). The "task creation" step continually invented new tasks, often duplicating completed ones.

**Production mitigations:**
- **Persistent plan artifact** (Ark's `PLAN.md`; Cline's Memory Bank). Read at the start of each turn so the agent grounds in prior decisions.
- **Done-list in plan** (explicit "completed" markers; Ark's `## Log` with response matrix).
- **Single source of truth for state.** Ark's `task.toml.phase` enforces forward motion; you cannot regress to Design from Execute.

## Failure 3: Multi-agent debate accuracy degradation

**Symptom:** Two or more LLMs debate; their consensus is worse than any individual model's answer. The "wisdom of crowds" inverts for LLMs because they all share similar biases.

**Documented:** A September 2025 paper ("On the Limits of Multi-Agent Debate", arXiv) showed debate-based LLM systems regress on factual benchmarks compared to single-LLM-with-self-critique.

**Production mitigations:**
- **Single-LLM evaluator-optimizer** (Anthropic's Building Effective Agents pattern) instead of peer debate.
- **Human-in-the-loop** for contested verdicts.
- **Heterogeneous models** if debate is used (different model providers reduce shared bias).

Ark sidesteps this: the REVIEW phase is a single reviewer (subagent or user), not a debate.

## Failure 4: Prompt drift across handoffs

**Symptom:** Parent asks child to "fix the bug"; child interprets it as "rewrite the file"; parent gets back a 200-line diff and applies it.

**Cause:** Implicit context the parent has but the child doesn't. The child's fresh-context isolation works against fidelity to the parent's mental model.

**Production mitigations:**
- **Explicit dispatch payload** — parent must over-specify scope and constraints in the prompt.
- **Read-mostly children** so misinterpretation produces a report, not an edit.
- **Return contract** so the parent can see "this is what I did" before applying.
- **Post-hoc review** — Ark's C-28 lets the parent revert out-of-scope writes after the fact.

## Failure 5: Lost context across handoffs

**Symptom:** Parent passes a task to a child; child does the work; result returns. Six turns later, parent acts on stale assumptions because the child's work hasn't propagated to the parent's mental model.

**Cause:** The string return is too lossy. The parent saw "done" and assumed completion looked the way it expected.

**Production mitigations:**
- **Structured returns** (JSON with named fields) instead of free text.
- **Disk persistence** — parent reads the artifact, not just the summary.
- **Explicit "what changed" enumeration** in the return.

Ark uses disk persistence as the primary channel (research files, plan files); summary strings are courtesy.

## Failure 6: Hallucinated state

**Symptom:** Agent reports completing actions it did not take. "I have updated the database schema" — schema is unchanged.

**Cause:** LLM-internal confabulation. Especially common after compaction (model loses access to actual tool results, fills the gap with plausible-sounding claims).

**Production mitigations:**
- **Verify before claim:** harness re-runs the relevant check after the agent claims success.
- **Tool result attribution:** structured fields in the prompt mark "this was actually returned by a tool" vs. "this was your reasoning".
- **Audit logs:** Ark's journal records what *happened* (git diff, state changes), not what the agent *said* happened.

The VERIFY phase in Ark is precisely this — re-audit after EXECUTE before COMMIT.

## Failure 7: Cycle detection misses

**Symptom:** Parent dispatches child A; A dispatches child B; B dispatches a new "A" (different instance, same role). Logically a cycle, but each instance is new so cycle-detection by-pointer fails.

**Cause:** Cycle detection that relies on object identity, not role / dispatch-spec identity.

**Production mitigations:**
- **Role-aware cycle detection:** track `(role, prompt-hash)` tuples in the dispatch chain.
- **Maximum depth as fallback:** even if cycle-detection fails, depth limit fires.

Ark's C-15 (no nested subagents) makes this moot at the structural level.

## Failure 8: Race conditions on shared state

**Symptom:** Two parallel sub-agents both edit the same file; one overwrites the other.

**Cause:** Naive parallel dispatch without write-scope awareness.

**Production mitigations:**
- **Declared write scopes** (Ark C-7..C-10).
- **File locking** on shared state (Ark `.state.toml` lock).
- **Worktree isolation** (each child on its own branch / file checkout).
- **Atomic operations** (temp+rename pattern for shared writes).

Ark covers .state.toml and worktrees; per-task files (PLAN, VERIFY) rely on single-writer assumption.

## Failure 9: Notification failures

**Symptom:** Background agent finishes; the completion notification never reaches the parent. Parent assumes still-running forever.

**Cause:** Network drop, process exit, watchdog timeout (this corpus's research-tier runs experienced exactly this).

**Production mitigations:**
- **Heartbeat / polling fallback** — if no notification in N minutes, parent polls.
- **Disk-as-truth** — parent reads the disk state regardless of notification; the artifact tells the story.
- **Re-dispatch awareness** — if recovery is needed, re-dispatch is idempotent (it picks up where the prior run left off).

Ark's research-tier worked because of disk persistence: stalled agents left partial files on disk; the parent (this main session) recovered by inspecting disk and writing the remainder. Without disk persistence the entire dispatch would have been lost.

## Failure 10: Budget surprise

**Symptom:** Long-running agent runs up unexpected cost. User opens billing dashboard, sees $200 charged.

**Famous cases:**
- AutoGPT $40+/30min reports.
- Cursor Background Agents — early reports of unattended agents racking spend.
- "Cursor in agent mode forgot to stop" Reddit threads (multiple, 2024-2025).

**Production mitigations:**
- **Per-session spend limit** (Cursor's spend-limit dashboard).
- **ACU billing transparency** (Devin's per-task ACU display).
- **Token-per-turn caps** (Anthropic, OpenAI API-side rate limits).
- **Cost surfaces in the agent's context** — agent sees current spend, can self-throttle.

Ark today: nothing. The host platform handles billing. Ark could surface tier budgets in `ark context` (e.g. "this is a deep tier task; iterations consume more").

## Failure 11: Stale artifacts

**Symptom:** Plan was written 3 days ago; codebase has moved; agent works from stale plan and produces work that doesn't fit.

**Production mitigations:**
- **Re-read at phase boundaries** — when transitioning Design → Plan, re-read PRD.
- **Iteration log** — Ark's `## Log` in PLAN demands "what changed since last iteration", flushing stale assumptions.
- **Time-based warnings** — `ark context` could warn if a task's `updated_at` is N days old without a phase advance.

## Failure 12: Identity / authorisation drift

**Symptom:** Parent agent has user's API key with broad permissions; child sub-agent inherits it; child does something the user didn't intend (write to a different repo, push to remote).

**Production mitigations:**
- **Capability tokens** — per-subagent credentials with scoped permissions.
- **Confirm before destructive action** — Claude Code's PreToolUse hooks; user-driven approve.
- **OS-level sandboxing** — child can't touch what it doesn't have a token for.

Ark's threat model assumes parent and child are both running as the user; identity drift is not the primary concern. But for enterprise / shared-credential environments this matters.

## What Ark catches today

Read across the failures:

| Failure | Ark's mitigation |
| ------- | ---------------- |
| Dispatch storms | C-15 recursion guard, host-platform concurrent caps |
| Plan-reinvention | `PLAN.md` artifact + phase forward-only |
| Multi-agent debate degradation | Single reviewer (not debate) |
| Prompt drift | Explicit subagent prompts; C-28 post-hoc revert |
| Lost context across handoffs | Disk persistence (research files, plan files) |
| Hallucinated state | VERIFY phase audits before COMMIT |
| Cycle detection misses | C-15 makes moot |
| Race conditions on shared state | `.state.toml` locking; worktree isolation |
| Notification failures | Disk-as-truth; re-dispatch awareness (partial) |
| Budget surprise | Tier system implicit ("deep = more iterations") |
| Stale artifacts | `## Log` iteration matrix |
| Identity drift | Not addressed (out of scope today) |

Ark's structural design is solid for ~9 of the 12. The gaps are:
- **Budget surprise** — no cost surfaces.
- **Notification failure recovery** — re-dispatch isn't fully idempotent / aware.
- **Identity drift** — not a problem in Ark's threat model but worth tracking.

## Directions for Ark

1. **Re-dispatch awareness as a first-class feature.** When a subagent dispatch fails (timeout, network drop), the parent's next dispatch should know about partial state and write only missing files. Test case: this very corpus.

2. **Tier-based cost surfaces.** In `ark context`, surface "this task tier estimates N turns; current iteration is K of max M". Lets the agent self-throttle without complex budget tracking.

3. **Time-based staleness warnings.** When a task's `updated_at` is N days old without phase advance, `ark context` warns. Cheap to add, catches "agent picks up an old task and re-derives from stale assumptions".

4. **Document the disk-as-truth invariant.** Ark already does this implicitly; calling it out in `subagent-support` SPEC makes it teachable. Every meaningful state change goes to disk; conversation is courtesy.

5. **Capture failure modes in workflow doc.** A `docs/book/src/concepts/failure-modes.md` listing the above with Ark's mitigations would teach users *and* future Ark devs why design choices exist.
