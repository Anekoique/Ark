# Just-in-Time and Progressive Context Loading

The pattern: don't pre-load repo content. Let the agent pull files into context via tools (`Read`, `Grep`, `Glob`) as it works. The default model in Claude Code, the implicit model in most CLI harnesses that don't ship RAG.

## Why JIT became the default

Three forces converged in 2024–2025:

1. **Tool use matured.** Claude Code, Codex, OpenHands all standardised on a small toolkit (read file, search, list, write file, run shell) that lets the agent fetch its own context.
2. **RAG for code disappointed.** Embeddings don't capture code semantics well; symbol names and variable choices dominate retrieval noise. See `rag-for-codebases.md`.
3. **Long context made prefill expensive.** A 200K-token codebase pre-loaded into context costs ~$0.60 per turn just for input tokens at 2026 rates. Reading only the 5 files that matter costs ~$0.005. JIT is the cost-control move.

The 2026 trade-off frame: *every token in the prompt should earn its keep, or it's draining the attention budget*. JIT is the formal version of that principle.

## How JIT actually works

In a typical session the agent does, per turn:

1. **Receives** a user message or tool result.
2. **Decides** what it needs to know next.
3. **Calls** `Grep` / `Read` / `Glob` to pull in the bytes it needs.
4. **Synthesises** from the tool result + prior context.
5. **Acts** (file write, tool call, response).

Step 3 is the JIT step. The agent is making *retrieval decisions per turn*, in natural language reasoning, not via a pre-computed vector index.

Anthropic's prompt for Claude Code explicitly trains this behaviour: "use Read to view a known path", "use Grep for a specific symbol", "use Explore agent for open-ended search". The instruction set is the retrieval policy.

## Trade-offs vs. upfront loading

| Dimension | JIT | Upfront (RAG / codemap) |
| --------- | --- | ----------------------- |
| Per-turn cost | Low (only what's needed) | High (full chunk set every turn) |
| First-turn latency | Low (no preload) | High (preload index / embed query) |
| Retrieval quality | Depends on agent's exploration skill | Depends on retrieval algorithm |
| Determinism | Low (agent might miss files) | Higher (index is fixed) |
| Repo-size scaling | Excellent (only touches what's needed) | Degrades (index growth, query latency) |
| Token-budget tightness | Tight (no wasted prefill) | Loose (always pays prefill cost) |
| Effectiveness with bad search tooling | Poor (agent flounders) | Better (index does the work) |
| Effectiveness with good model reasoning | Excellent (smart agent grabs the right files) | Good (but expensive) |

JIT wins on cost and scaling. Upfront wins on determinism for fixed pipelines.

The hybrid is winning: **JIT for code reads, upfront for high-level structure (codemap, AGENTS.md, project specs)**. The codemap tells the agent *what exists*; JIT pulls *what's needed*.

## Implementations

### Claude Code (canonical JIT)

Default tools: Read, Grep, Glob, Bash, Write. No upfront index. Agent reasons from CLAUDE.md + tool exploration. Anthropic's blog posts emphasise this as a design choice — they considered shipping a code-RAG and decided against it.

Strengths: Cheap per turn, scales to large repos, model decides what's worth reading.
Weakness: Cold start — the agent's first few turns are spent orienting if there's no CLAUDE.md. Hence the standard pattern of writing CLAUDE.md / AGENTS.md.

### Codex CLI

Same shape — read, search, write, shell. AGENTS.md as upfront context. No vector retrieval.

### Aider (hybrid)

JIT for file reads, upfront for the repo-map outline. The map is the "table of contents"; JIT is the "open the chapter". Aider's `--map-tokens` budget is the explicit hybrid knob.

### OpenHands

JIT for file ops; microagents trigger on keywords to inject conditional context. Half-JIT, half-conditional-RAG.

### Cursor (hybrid with proprietary indexing)

`@codebase` is upfront-ish (uses an internal index); plain agent mode is JIT. The user picks per query.

### Devin

Closed source; observed behaviour suggests a heavy upfront context (Playbooks, Knowledge Base) plus JIT for unfamiliar files.

## The "tool budget" sub-pattern

A 2026 refinement: agent has a *budget* of tool calls per task, surfaced in the prompt. Claude Code's Background Agents and Cursor's Background Agents both default-limit tool counts to avoid runaway. JIT works best with a *budget*; without one, the agent over-explores.

The budget is also a *cost surface*: the user sees "this task used 47 tool calls" and learns whether their work is efficient. Tightly related to `pricing-cost-and-budget-ux.md` in section 07.

## Failure modes

1. **Cold start.** Without CLAUDE.md / AGENTS.md / codemap, JIT first turns are noisy. Mitigated by always-loaded structural context.
2. **Missed files.** Agent doesn't grep widely enough; relevant code goes unread. Worse for refactors than features.
3. **Over-exploration.** Agent reads 20 files when 3 suffice. Burns tokens, slows the session. Mitigated by tool budgets and explicit prompt guidance.
4. **Cache misses.** Reading the same file across turns without prompt caching duplicates cost. Anthropic's `cache_control` mitigates if the host harness supports it. Claude Code does; Codex does; OpenCode partially.
5. **Lost-in-the-middle as JIT loads grow.** Reading 30 files into context puts the early reads in the lost-middle. Mitigated by sub-agent dispatch (fresh context per child).

## Where Ark stands

Ark's pattern is **static JIT** — `ark context` delivers a pre-curated session-orientation packet (git state, tasks, specs, archive). It is JIT in the sense that not the whole repo is loaded; it is static in the sense that the projection is fixed per phase, not retrieved per turn.

Then the *host agent* does live JIT (Read, Grep) for actual file content. Ark stays out of that loop.

This split is good: Ark handles "what should be loaded once per phase"; the host agent handles "what should be loaded mid-turn". Clean separation of concerns.

The gap: Ark has no opinion on *what the host agent should read next*. There's no `ark next-files` advisor that says "for this phase, the agent should probably read X, Y, Z first". That would be the workflow-aware companion to a codemap.

## Directions for Ark

1. **Document the static-JIT pattern as a feature.** It is intentional and good, but README/AGENTS.md don't name it. Naming it lets users reason about when to extend `ark context` vs. when to add a CLAUDE.md hint.

2. **Surface "suggested reads" per phase.** `ark context --scope phase --for execute` could include a `suggested_reads: ["<file>", ...]` field populated from the latest PLAN's Implementation phase. The host agent reads those upfront, then JIT-reads the rest.

3. **Audit `ark context` payload size.** Large repos may produce large projections (many active tasks, many SPECs). If the JSON output approaches 10K tokens, it crowds the agent's effective window. A `--terse` mode that elides scopes for SPECs not referenced by the current task would help.

4. **Pair with a codemap.** Static JIT works best when the agent knows *what exists*. A `docs/CODEMAPS/` (see `codemaps-and-repo-structure-summaries.md`) is the structural complement.

5. **Track tool-budget hooks.** As host platforms expose tool-call counts to hooks (Claude Code's `SubagentStop` includes counts), Ark could record per-phase tool budgets in `task.toml` and surface them in `ark context`.
