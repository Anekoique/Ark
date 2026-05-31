# Memory Systems

## What the primitive means

"Memory" in coding-agent harnesses is the answer to: *what does the agent
remember between turns, between sessions, between projects?* It is
deliberately distinct from **context** — context is what's in the prompt
this turn; memory is the *durable surface from which context is drawn*.

A useful taxonomy:

| Kind | Lifetime | Scope | Who writes |
| ---- | -------- | ----- | ---------- |
| **Conversation memory** | One session | One agent | The agent loop itself |
| **Auto-memory** | Across sessions | One project (usually) | The agent (with user nudges) |
| **Project memory** | Across sessions | One project | The user |
| **User memory** | Across projects | One user | The user |
| **Global / org memory** | Across users | Org-wide | Admin |
| **Structured memory** | Across sessions | Domain-specific schema | Workflow rules |

Most harnesses cover 2-4 of these; the *interesting* ones cover all of them
with explicit precedence (e.g. Claude Code's three-layer CLAUDE.md merge).
Ark covers a slice that no other harness covers — *structured workflow
memory* — and is comparatively silent on the others.

## Memory vs context (one-paragraph distinction)

Context is *transient* — it fills the LLM's window this turn and is
discarded. Memory is *persistent* — it survives the session and is
re-loaded next time. Confusingly, *loading* memory into the prompt makes it
context. The distinction matters because **memory is the unit you version
and migrate**; context is the unit you budget. RAG systems blur the line:
they're memory that masquerades as just-in-time context.

## How leading harnesses implement it

### Claude Code (Anthropic) — three-layer CLAUDE.md + auto-memory

**Project memory** — `<project>/CLAUDE.md`. Committed to VCS; team-shared.
Best for build commands, conventions, architecture notes.

**Local memory** — `<project>/.claude/CLAUDE.md`. Per-machine, not
committed. Personal preferences that shouldn't affect teammates.

**User memory** — `~/.claude/CLAUDE.md`. Cross-project. Style preferences,
editor knowledge.

**Merge order** — "Claude merges all applicable memory files at session
start, so your global preferences combine with project-specific context
without conflicts" (`code.claude.com/docs/en/memory`). Concretely: read
order is user → project → local, with later ones augmenting earlier.

**Auto-memory** (`/memory`, `/remember`) — "Claude saves notes for itself
as it works: build commands, debugging insights, architecture notes, code
style preferences, and workflow habits." Categorised into:

- user preferences
- feedback (corrections)
- project context
- reference pointers

Stored in the same CLAUDE.md tree but written by the agent rather than the
user. The `/memory` command opens the file in the editor; conversational
"remember that …" appends to the right file.

**Hard cap.** "200 lines per CLAUDE.md file." Longer files reduce
adherence (DEV community writeup, `dev.to/.../claude-codes-memory-4-layers-of-complexity`).
This is a soft policy, not enforced — but the heuristic is widely repeated.

**Memory tool (API).** Separate from CLAUDE.md is the Memory **tool**
exposed via Claude API (`platform.claude.com/docs/en/agents-and-tools/tool-use/memory-tool`)
— a stateful API the model can call to write key/value memory entries.
Conceptually closer to RAG than to a markdown file.

### OpenAI Codex CLI

Project memory = `AGENTS.md` at project root + nested overrides up the
directory tree. Codex pioneered the AGENTS.md shape (`developers.openai.com/codex/guides/agents-md`).
No native auto-memory equivalent to Claude's `/memory`; users hand-edit
AGENTS.md.

### Cursor — Project Rules + User Rules + Memories

**Project Rules** — `.cursor/rules/*.mdc`. Version-controlled, scoped to
the project. Replaces the deprecated `.cursorrules` file. Subdirectories
can have nested rules.

**User Rules** — `~/.cursor/rules/*` (in settings). Cross-project personal
preferences.

**Memories** — "Automatically generated rules based on your conversations
in Chat, scoped to your project and maintaining context across sessions"
(`docs.cursor.com/context/rules`). Toggle: Cursor Settings → Rules →
Memories.

Format note: `.mdc` is markdown with YAML frontmatter — `description`,
`globs` for auto-attach. Closer to Anthropic's Skills file shape than to
plain markdown.

### OpenHands — microagents

`.openhands/microagents/*.md` with frontmatter:

- `trigger: always` — load every session.
- `trigger: keyword` + `keywords: [...]` — load when conversation mentions
  one of them.
- `trigger: manual` — load only on explicit invocation.

Subtler than CLAUDE.md because it bakes in **just-in-time loading** via the
keyword trigger. Closer to a structured-memory pattern than to a single
file.

### Cline — Memory Bank (MCP server)

Cline doesn't have native memory; it relies on the **Memory Bank MCP
server** (`github.com/dazeb/cline-mcp-memory-bank`). Core files:

| File | Purpose |
| ---- | ------- |
| `projectbrief.md` | Project foundation |
| `activeContext.md` | Current work focus (updated most often) |
| `systemPatterns.md` | Architecture decisions |
| `techContext.md` | Technologies used |
| `progress.md` | What works, what's left |

This is the **closest thing in the field to Ark's structured-memory
pattern** — but it's user-curated markdown, not workflow-extracted.

### Aider

Project conventions live in `CONVENTIONS.md` (user-supplied,
`--read CONVENTIONS.md`). No auto-memory; Aider's conventions are static.

### Continue.dev

`.continue/rules/*.md` system-prompt fragments per mode (Chat / Edit /
Agent). Not auto-curated.

### Anthropic Agent Skills (open standard, Dec 2025)

Each skill = directory with `SKILL.md` + optional scripts. Skills are
*invoked*, not loaded; sit between memory and tools — closer to capability
packs than memory. But notable here because some skills (e.g.
"summarize-pr") *are* memory shape: reusable instructions activated by
context.

## Scope ladders compared

| Harness | Global / user | Local / project | Sub-project | Auto-written |
| ------- | ------------- | --------------- | ----------- | ------------ |
| Claude Code | `~/.claude/CLAUDE.md` | `<proj>/CLAUDE.md`, `<proj>/.claude/CLAUDE.md` | (nested supported via `@` imports) | Yes (`/memory`) |
| Codex CLI | `~/.codex/AGENTS.md` | `<proj>/AGENTS.md` | Nested AGENTS.md up the tree | No |
| Cursor | `~/.cursor/rules/*` | `.cursor/rules/*.mdc` | Subdir-scoped rules | Yes (Memories toggle) |
| OpenHands | n/a | `.openhands/microagents/*.md` | Trigger-keyed | No |
| Cline | n/a (relies on MCP) | Memory Bank MCP files | n/a | Partial |
| Continue | n/a | `.continue/rules/*.md` | n/a | No |
| Aider | n/a | `CONVENTIONS.md` | n/a | No |

## Memory vacuum / decay

A subtle problem most harnesses skip: **what happens when memory becomes
wrong?** No surveyed harness has a *decay* mechanism — auto-memory only
grows. Users prune manually.

- Claude Code's 200-line soft cap is the closest thing to a budget.
- Cursor's `globs` field at least restricts *when* a rule fires, narrowing
  blast radius.
- OpenHands' keyword triggers similarly narrow.

The absence of decay is one reason "stale memory" is a known failure mode
across all harnesses — users routinely complain that the agent follows an
old convention.

## What Ark does today

Ark's memory model is **structured by tier**, and unusually rich for the
workflow domain.

### Three memory layers

1. **Project SPECs** — `.ark/specs/project/<name>/SPEC.md`. User-authored,
   always-read conventions ("`.ark/specs/INDEX.md`"). Workflow rule:
   "Read every entry in `specs/project/INDEX.md` before any task."
   These are Ark's analogue of **project memory**.

2. **Feature SPECs** — `.ark/specs/features/<...>/<name>/SPEC.md`.
   *Auto-extracted* from deep-tier PLANs at commit. Recursive tree.
   These are Ark's analogue of **auto-memory** — but written by the
   *workflow*, not by the LLM. Code site:
   `crates/ark-core/src/commands/agent/spec/extract.rs`.

3. **CLAUDE.md / AGENTS.md managed block** — see
   `crates/ark-core/src/layout.rs:161`:
   ```
   pub const MANAGED_BLOCK_BODY: &str = "\
   Ark is installed in this project. Use `/ark:quick` or `/ark:design` to start tasks.
   
   See `.ark/workflow.md` for the full workflow.
   
   @.ark/specs/INDEX.md";
   ```
   This is the *bridge* into the host harness's memory layer. The `@` import
   syntax is Claude's; Codex / OpenCode share the same `AGENTS.md` via
   manifest dedupe (`platforms.rs:tests/shared_agents_block_deduped_when_both_platforms_apply`).

### Other memory-shaped surfaces

- **Workflow doc** — `.ark/workflow.md`. Static, ships with the binary;
  refreshed by `ark upgrade`. The "rules of how Ark works" memory.
- **Workspace journals** — `.ark/workspace/<dev>/journal-N.md`,
  per-developer session log appended on `task commit` when
  `.ark/.developer` exists. This is the **conversation memory** layer —
  except it's per-developer-per-task summary, not the full transcript.
  Code site: `crates/ark-core/src/commands/agent/workspace/`.

### What Ark does NOT have

- **No auto-memory written by the LLM.** Nothing equivalent to Claude's
  `/memory` writing into CLAUDE.md.
- **No user-global / personal layer.** Ark scopes to one project. There is
  no `~/.ark/CLAUDE.md` analogue.
- **No memory decay / pruning.** SPECs grow monotonically; user has to
  prune by hand.
- **No JIT loading.** All project SPECs are read every task. Feature SPECs
  are referenced explicitly in the PRD's `[**Related Specs**]` block —
  closer to OpenHands' manual trigger than to a glob.

## The structural memory that Ark has that others don't

Ark's feature-SPEC tree is unusual: it's **schema-shaped memory**. A SPEC
has a fixed shape (Goals `G-N`, Constraints `C-N`, Architecture, etc.)
because the workflow extracts it from a PLAN of fixed shape. This means
the memory is *queryable* — `ark context --scope phase --for verify`
projects every SPEC's compliance state.

No other harness's memory is queryable this way. CLAUDE.md, AGENTS.md,
microagents, Cursor rules — all are unstructured markdown blobs read in
their entirety. Ark's combination of (a) workflow-controlled extraction
and (b) structured shape is the differentiator.

## Directions for Ark

1. **User-global SPECs.** Add a `~/.ark/specs/user/` tier merged after
   project SPECs. Solves the "I keep telling every project I prefer
   functional combinators" friction. Pairs with the existing `Layout`
   path machinery — add a `Layout::user_specs_dir(home: &Path)`
   returning `~/.ark/specs/user/`. Code site:
   `crates/ark-core/src/layout.rs`, `crates/ark-core/src/commands/context/gather.rs`.
2. **Glob-scoped feature SPECs.** Today the PRD's `[**Related Specs**]`
   block names exact SPECs. Allow glob matchers (`specs/features/auth/**`)
   so a task that touches the whole `auth/` subtree loads the right
   things without enumeration. Code site:
   `crates/ark-core/src/commands/context/related_specs.rs`.
3. **SPEC freshness / decay surface.** Add a `last_verified_at` field
   stamped on every VERIFY pass; `ark context` flags SPECs older than N
   days or N commits. Pure information surface — no enforcement — but
   makes stale memory visible. Code site: SPEC frontmatter is currently
   none; would need a managed block at SPEC top.
4. **`ark agent memory record`.** Equivalent of Claude's `/remember` but
   workflow-aware: writes into the active task's `research/` or the
   developer journal, not into SPECs. Keeps the auto-vs-curated separation
   that Ark inherits from its tier model. Code site: new verb under
   `crates/ark-core/src/commands/agent/`.
5. **Cross-harness memory dedupe.** Today the `AGENTS.md` block is shared
   between Codex / OpenCode (`platforms.rs:tests/shared_agents_block_deduped`).
   Extend dedupe to a *meta* file `AGENTS.md` ↔ `CLAUDE.md` where the
   non-platform-specific content (workflow ref, SPEC index) lives once
   and is `@`-imported from each. Pairs with the AGENTS.md adoption story
   in `templates-and-scaffolding.md`. Code site: `crates/ark-core/src/platforms.rs`.

## Caveats / Not found

- I did not find Anthropic primary-source documentation for the exact
  "memory categories" (user preferences / feedback / project context /
  reference pointers) — the categorisation is from a community write-up.
- The Memory tool (API-facing) is separate from CLAUDE.md memory and was
  not deeply explored here; treat as adjacent.
- Cursor's `.mdc` frontmatter spec is undocumented at the schema level
  beyond `description` and `globs`.
- "Memory decay" as a primitive is conspicuously missing from every
  surveyed harness — confirmed by absence in their docs.

## Sources

- [Claude Code Memory](https://code.claude.com/docs/en/memory)
- [Claude Code Auto-Memory (MindStudio)](https://www.mindstudio.ai/blog/what-is-claude-code-auto-memory)
- [Claude Code Memory Explained (Parreó García)](https://joseparreogarcia.substack.com/p/claude-code-memory-explained)
- [Claude Memory 3-Layer Guide (Shareuhack)](https://www.shareuhack.com/en/posts/claude-memory-feature-guide-2026)
- [Claude Code Memory 4 Layers (DEV)](https://dev.to/chen_zhang_bac430bc7f6b95/claude-codes-memory-4-layers-of-complexity-still-just-grep-and-a-200-line-cap-2kn9)
- [Claude Memory Tool (API)](https://platform.claude.com/docs/en/agents-and-tools/tool-use/memory-tool)
- [AGENTS.md spec](https://agents.md/)
- [Cursor Rules docs](https://docs.cursor.com/context/rules)
- [Cursor Memories (Forum)](https://forum.cursor.com/t/rules-vs-memories-and-global-vs-project/137149)
- [OpenHands Microagents Overview](https://docs.openhands.dev/openhands/usage/microagents/microagents-overview)
- [Cline Memory Bank MCP](https://github.com/dazeb/cline-mcp-memory-bank)
- [Aider Conventions](https://aider.chat/docs/usage.html)
- [Anthropic Agent Skills](https://www.anthropic.com/news/skills)
