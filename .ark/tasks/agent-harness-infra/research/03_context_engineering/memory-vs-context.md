# Memory vs. Context — the load-bearing distinction

The line that splits the discipline. **Context** is ephemeral and in-prompt; lives for the duration of one conversation. **Memory** is durable and file-backed; survives across conversations, sessions, even host platforms.

Both feed the agent. Both are useful. Confusing them is the most common mistake in harness design.

## The clean definitions

### Context

- Lives in the LLM's input tokens during a single inference.
- Bounded by the model's context window (200K–2M tokens in 2026).
- Gone when the session ends, unless saved-and-resumed (and even then it's loaded back into a *new* context, not "preserved").
- Costs input tokens at every turn (with prompt caching for prefix).

### Memory

- Lives on disk, in files the host platform or harness reads at session start.
- Bounded by what the harness chooses to load (often: file size + sensible cap).
- Survives indefinitely. Versioned (if committed) or runtime-mutable (if not).
- Free to store; costs only when loaded into a session.

The cleanest test: *if I close my terminal and reopen tomorrow, does this state still exist?* If yes → memory. If no → context.

## Why the distinction matters

Conflating them produces three bad outcomes:

1. **Stuffing memory into context.** If everything goes into CLAUDE.md / system prompt, the agent pays the prompt cost every turn. The 50KB CLAUDE.md that loads on every Claude Code session burns ~10K tokens per turn × every turn × every session. Memory should be *available*, not *resident*.

2. **Losing context as memory.** If a great mid-session insight only lives in the conversation, it is lost at compaction or session end. Without a memory write, knowledge accreted in a session evaporates.

3. **Mistaking auto-memory for permanent memory.** Claude Code's `/remember` writes to a file (memory). The conversation also "remembers" within a session (context). Users conflate them and think the agent will "remember" things it actually only knows for this conversation.

## Memory implementations

### Always-loaded memory (CLAUDE.md / AGENTS.md)

The host agent reads this file at session start; its content is in every turn. Maximally available, maximally expensive.

Use for: project-level conventions, command quick-references, persona / role definitions. *Anything every turn needs.*

Best practice: keep under 5K tokens. Larger CLAUDE.md files crowd the effective window.

### Conditional memory (Cursor rules with frontmatter)

`.cursor/rules/<name>.mdc` files with `alwaysApply: true | false` / `globs:` / `description:`. Loaded conditionally based on the user message or files in scope.

Use for: file-pattern-specific rules ("when editing .py files, follow PEP 8"). *Knowledge that only applies sometimes.*

### Auto-memory (Claude Code's `/remember`)

A directory of small markdown files indexed by topic. Loaded when relevant via fuzzy matching.

Use for: durable but not always-relevant facts ("user prefers detailed commit messages", "this project uses Bun, not Node"). *Things you've learned about the project / user.*

### Structured memory (Ark's project + feature SPECs)

Files with defined schemas (SPEC.md template), indexed in tables (INDEX.md with managed blocks), referenced from the projection (`ark context` enumerates them).

Use for: enforceable conventions and feature specifications. *Things that get audited (Ark's VERIFY phase consults them).*

Distinctive in two ways:
- Schema discipline (SPEC.md template, INDEX.md format).
- Programmatic access (`ark context` parses them; subagents read them).

This is the *third layer* most harnesses lack.

### Vector memory (mem0, Letta/MemGPT)

Embed every fact; retrieve by semantic similarity at session start.

Use for: open-ended fact accumulation across many sessions. *User preferences, recurring patterns, long-term continuity.*

Failure modes: same as RAG-for-code — embedding-space retrieval is unreliable for specific lookups; produces "I think I saw this somewhere" matches.

### Journal memory (Ark's workspace journals)

`.ark/workspace/<dev>/journal-N.md` — append-only log of session summaries.

Use for: time-ordered history ("what I did Tuesday"). *Provenance, audit trail, retrospective.*

Less retrieved-by-agent, more read-by-human.

## Context implementations

These are covered in adjacent files:

- `context-window-management.md` — the substrate, attention budget, compaction.
- `rag-for-codebases.md` — retrieval into context (the dead end).
- `codemaps-and-repo-structure-summaries.md` — structural data injected into context.
- `jit-and-progressive-context-loading.md` — tool-driven on-demand context.
- `structured-summaries-and-projections.md` — Ark's `ark context` as the projection layer.

## When memory leaks into context (anti-pattern)

Symptoms:
- CLAUDE.md > 10K tokens. *Memory too eagerly loaded.*
- Auto-memory accumulates without pruning; first-session token costs balloon.
- Vector memory triggers irrelevant facts ("I see you previously discussed X" when X is unrelated).

Fixes:
- Move always-loaded knowledge to conditional/cursor-rules form.
- Cap auto-memory size and review periodically.
- Use vector memory only where similarity-based recall genuinely helps (chat-like assistants), not for code.

## When context leaks into memory (anti-pattern)

Symptoms:
- "I'll remember this for next time" — the agent says it; users believe it; but no file gets written.
- Mid-session decisions don't make it into PLAN / SPEC / journal — they live only in conversation.
- Auto-compact wipes the decision; next session has no record.

Fixes:
- Make memory writes explicit (run `/remember`, edit CLAUDE.md, append to journal).
- Bake memory writes into workflow phases — Ark's VERIFY phase + journal entry on commit is exactly this.
- Treat the conversation as ephemeral; write everything important to disk.

## Memory hierarchy lessons

Across the field, the converging hierarchy (top = closer to always-on, bottom = more conditional):

1. **System prompt** (harness-controlled, model-provider-shaped). Few tokens, foundational.
2. **Always-loaded project memory** (CLAUDE.md, AGENTS.md). ~5K tokens budget.
3. **Conditional memory** (Cursor rules, OpenHands microagents). Loaded on triggers.
4. **Structured memory** (SPECs, ADRs, design docs). Loaded on demand, persisted as files.
5. **Auto-memory / vector memory** (`/remember`, mem0). Semantic retrieval.
6. **Journals / logs** (per-developer journals, audit logs). Read mostly by humans.
7. **JIT context** (Read, Grep tool calls). Per-turn retrieval into context.

Most harnesses ship 1–3 + 7. Workflow-opinionated harnesses (Ark) add 4 + 6. Open-ended chat assistants tend toward 5.

## Where Ark stands

Ark's memory story is:

- **Layer 2:** CLAUDE.md / AGENTS.md with a managed block (~1K tokens body).
- **Layer 4:** Project SPECs (user-authored) + Feature SPECs (auto-promoted). The standout differentiator.
- **Layer 6:** Per-developer journals.

Ark *does not* ship:
- Conditional memory (no glob-keyed loading).
- Auto-memory (no `/remember` equivalent — though the user could run Claude Code's `/remember` and Ark would not interfere).
- Vector memory (deliberately).

The gap candidates:
- A glob-keyed conditional memory pattern (Cursor-rules-style) could complement Ark's flat project-SPECs layout.
- No "memory hierarchy" documentation page; users have to infer the layers from reading multiple files.

## Failure modes from the corpus

### CLAUDE.md ballooning

A user accumulates instructions into CLAUDE.md; it grows to 30KB. Every Claude Code session pays the cost. Ark's managed block helps (it caps Ark's own contribution to ~5 lines), but user-side growth is unbounded.

**Mitigation:** Ark could warn in `ark context` if CLAUDE.md exceeds a threshold.

### SPEC sprawl

`specs/features/INDEX.md` lists every promoted feature SPEC forever. As projects mature, the list grows past the point where scanning is cheap. Mitigation: cull / archive policy.

### Journal pile-up

Per-developer journals are append-only and uncapped. Eventually a journal hits multi-megabyte size. Mitigation: rotate (Ark's `journal-N.md` numbering supports this implicitly).

### Stale memory

A SPEC describes a feature that was renamed; the SPEC is unchanged. The next session reads stale memory. Mitigation: VERIFY phase consults SPECs (catches drift); explicit changelog on SPEC modifications (Ark requires this).

## Directions for Ark

1. **Ship a `docs/book/src/reference/memory-hierarchy.md` page.** Layer-by-layer explanation of what lives in CLAUDE.md vs. project SPECs vs. feature SPECs vs. journals. The single biggest documentation gap surfaced by this corpus.

2. **Warn on bloated CLAUDE.md.** `ark context` could emit a `warnings: [{kind: "claude_md_bloat", size: "47K"}]` field when the file exceeds 10K tokens. Cheap to implement; nudges users to use SPECs instead.

3. **Conditional-memory feature spec.** A `glob-rule` system would let users write "when the agent works on `crates/ark-cli/`, load this extra context" without bloating always-loaded files. Equivalent to Cursor's frontmatter rules but markdown-native.

4. **Auto-memory bridge.** When Claude Code's `/remember` writes a memory file, Ark could surface it in `ark context` so other platforms (Codex, OpenCode) inherit the same memory. Currently each platform has its own auto-memory; Ark could be the cross-platform memory layer.

5. **Promote journals to a memory layer in the architecture diagram.** Journals are listed as a workspace feature; they are also a memory layer. Re-framing them clarifies the design space.
