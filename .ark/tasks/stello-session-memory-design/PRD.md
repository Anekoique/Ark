# `stello-session-memory-design` PRD

---

[**What**]

Study how Stello (the open-source conversation-topology engine) organizes sessions, tasks, and memory, and distill the parts that inform an AI-native memory system for Ark.

[**Why**]

Ark's current memory story is a flat per-user file pile (`memory/*.md` + `MEMORY.md` index) plus per-task artifact directories. Stello has thought hard about the orthogonal axes — *topology* (how conversations branch into a session forest), *per-session memory layering* (L3 history → L2 memory → one-shot insight inbox), and *agent-wide shared memory* (index-always-injected, body-lazy-loaded via tools). These map cleanly onto problems Ark will hit as it scales: how to scope memory (task vs. session vs. project vs. user), how to surface it without blowing the context window, and who owns reflection/consolidation. This corpus gives us the vocabulary and the design trade-offs before we design Ark's own memory layer.

[**Outcome**]

A curated corpus under `research/` covering, at minimum:
- Stello's session/topology model (single-session, fork-synthesis chain, multi-root forest, topology decoupled from session identity).
- The three-slot per-session context model (systemPrompt / insight / memory) + L3 history, the assembly order, and the key invariant that memory does **not** re-enter the session's own context.
- Agent-wide shared memory (slug/summary/body, index-injected + tool-lazy-loaded body, writeLock concurrency, the three builtin tools).
- The orchestrator-facing data SDK and the "reflection is the application layer's job, not the framework's" stance.
- Storage layering (SessionStorage vs. SessionTree) and the injection-point architecture.
- A synthesis note mapping Stello's ideas onto Ark concepts (tasks/specs/journal/auto-memory) with concrete "adopt / adapt / reject" calls.

Stop when these are captured well enough that a follow-up `/ark:design` task could cite the corpus and start designing Ark's memory layer.

[**Related Specs**]

<none — research tier, no SPEC interaction>

[**SPEC Path**]

<ignored on research tier>
