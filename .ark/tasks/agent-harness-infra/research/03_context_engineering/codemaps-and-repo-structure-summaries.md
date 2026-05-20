# Codemaps and Repo-Structure Summaries

The 2025–2026 consensus alternative to RAG-for-code: build a structural map of the repo (symbols, file roles, call graphs) and inject the relevant slice into the agent's context. Cheaper, more predictable, and grounded in syntax rather than embedding-space approximation.

## What a codemap actually is

A codemap is a structured, agent-readable summary of *what a repository contains and how its parts relate*. Implementations vary along three axes:

1. **Granularity** — file-level, symbol-level (functions/classes/types), or block-level (sub-symbol).
2. **Source** — tree-sitter parse, LSP server, custom AST walker, plain README.
3. **Refresh** — pre-computed (cron, git hook, post-write hook), on-demand (per query), lazy (when stale).

The minimal codemap is `find . | head` + `wc -l`. The maximal is Aider's tree-sitter PageRank graph weighted by symbol references. The useful middle is "a file tree with one line of role-description per file plus a list of exported symbols per file".

## Aider's repo-map — the prototype

Aider, since 2023, has shipped the most-studied codemap implementation. Outline:

1. Tree-sitter parses every file in the repo, extracting symbol definitions and references.
2. Build a directed graph: edge from file A → file B if A references a symbol defined in B.
3. Apply PageRank weighted by reference count. Top-N files float to the top.
4. Render an outline (symbol names + line ranges, no bodies) of the top files into the prompt.
5. Budget capped by `--map-tokens` (default 1024).

The clever part is *adaptive selection*: Aider re-runs the PageRank with the *files-currently-being-edited* boosted, so the map biases toward what is relevant to the immediate edit.

**Why it works for code:** PageRank captures the structural importance the way embedding similarity captures topical similarity, but unlike embedding similarity, it does not get fooled by name collisions (`User` in /auth vs. `User` in /billing) and is not affected by code style.

**Why it does not replace JIT:** the map gives the agent a *table of contents*. The agent still needs to read individual files to write code. Codemap + read-on-demand is the working pattern, not codemap alone.

## Other production codemap implementations

### Continue (`@codebase`)

Continue ships `@codebase` as a slash-context. Original implementation (2023) was embedding-RAG; the 2025 pivot deprecated that and moved to a tree-sitter symbol index. As of 2026 `@codebase` consults the symbol index for relevant files, then reads them on-demand. Mirrors the Aider architecture but with VS Code IDE-native integration.

### Cursor (`@codebase`)

Cursor's `@codebase` symbol uses an internal index (proprietary; details not public). User reports indicate hybrid retrieval — symbol-based for code, embedding-based for prose docs.

### Sourcegraph Cody

Cody historically led on code-RAG. By late 2025 it had de-emphasised vector retrieval in favour of code-graph navigation (Sourcegraph's broader product line is built on symbol graphs). The 2026 Cody pitch is "code graph as agent context" — closer to a codemap than a RAG.

### Plandex

Tree-sitter project maps. ~2M-token context to put the whole map in-context for large repos. Plandex's bet: huge context + symbol map > selective retrieval.

### OpenHands microagents

Not a codemap per se, but a related primitive: small knowledge fragments keyed by trigger words. When the agent's message contains a trigger, the matching microagent's content is appended to context. *Symbolic, conditional, JIT-injected.*

## The doc-as-source-of-truth pattern

A growing 2026 pattern: codemaps as *first-class documentation* committed to the repo, refreshed by a doc-updater agent.

Outline:

1. `docs/CODEMAPS/<module>.md` files describe each module's files, symbols, and call graph.
2. A `doc-updater` agent (or hook) regenerates these on relevant changes.
3. The repo's CLAUDE.md / AGENTS.md `@`-references `docs/CODEMAPS/INDEX.md` so the map is always loaded.

Variants:
- **`/update-codemaps` slash command** (referenced in some Cursor configurations, internal team docs).
- **A `doc-updater` agent definition** (the user's global rules file in this repo references it: "Documentation and codemap specialist. Use PROACTIVELY for updating codemaps").
- **`tree`-based file lists committed as `docs/STRUCTURE.md`** (minimal version, common across many repos).

**Why this pattern is growing:** Codemaps generated only at agent-invocation time are stale relative to actual structure; committed codemaps with explicit refresh contracts are auditable, diff-reviewable, and survive across sessions / agents / harnesses.

## Generation strategies

### Tree-sitter (the dominant choice)

Tree-sitter is a fast, incremental parser library with grammar packages for ~200 languages. Bindings exist for Rust (`tree-sitter` crate), Python, Node, Go. Trade-offs:
- Pro: language-agnostic, fast (incremental), no language-server runtime needed.
- Con: parse-only (no type info, no cross-file resolution out of the box).

Used by: Aider, Continue, Plandex, many tooling projects.

### LSP servers

Talk to a language server (rust-analyzer, gopls, pyright, etc.) over LSP, ask for symbols / references / hover info. Trade-offs:
- Pro: type-aware, can resolve cross-file references.
- Con: heavy (per-language server runtimes), slow startup, brittle in CI.

Used by: Some Cursor / JetBrains AI features; less common in standalone codemap tools.

### AST scanners (per-language)

Hand-written Python `ast`, JavaScript `acorn`, Rust `syn` walkers. Trade-offs:
- Pro: language-native, easy to extract custom metadata.
- Con: one-per-language; maintenance burden grows with target list.

Used by: Older Continue versions, internal tools.

### Plain text (`tree`, `find`, README)

Just commit a file tree + manually-curated module descriptions. Trade-offs:
- Pro: zero infra, human-maintainable.
- Con: drifts from reality; only as good as the author's discipline.

Used by: Most early-stage projects, including (effectively) Ark today.

## Refresh policy

Three strategies, each with failure modes:

| Strategy | Mechanism | Failure mode |
| -------- | --------- | ------------ |
| **Manual** | User runs `aider --map-refresh` or equivalent | Stale by default; users forget |
| **Hook-driven** | Git pre-commit / post-commit / PostToolUse hook regenerates | Slow commits; non-deterministic if generation fails |
| **Lazy** | Regenerate on next read if stale (per-file mtime check) | First read pays the cost; cache invalidation bugs |
| **Agent-on-demand** | A `doc-updater` agent runs as a scheduled / triggered job | Adds an agent run to every refresh |

Aider does lazy: invalidates per-file via mtime check, regenerates the affected subset on the next prompt. Continue does hook + lazy. Most "committed CODEMAPS" repos rely on manual + occasional agent runs.

## Where Ark stands

Ark today has *no codemap*. The closest analog is `.ark/specs/features/INDEX.md` — a hand-maintained (well, CLI-maintained) table of feature SPECs and their scopes. That table is a *workflow* map (what's been built), not a *structural* map (what's in the code).

The gap is real: when the user's deep-tier task says "refactor the auth layer", neither the agent nor `ark context` knows what files comprise the auth layer. The agent has to grep, read, and discover the structure each session.

## Trade-offs

| Approach | Cost to build | Cost per session | Staleness risk | Cross-platform portability |
| -------- | ------------- | ---------------- | -------------- | -------------------------- |
| No codemap | 0 | High (agent re-discovers each session) | n/a | High (no artifact to port) |
| Tree-sitter codemap shipped as committed `docs/CODEMAPS/` | Medium (one-time generator + hook) | Low (file read) | Low if hook fires; medium if manual | High (just markdown) |
| LSP-based codemap | High (per-language) | Low | Low | Medium (depends on LSP runtime) |
| Hand-maintained `docs/STRUCTURE.md` | Low | Low | High (drifts) | High |
| Embed-and-retrieve (RAG) | Medium (vector store) | Medium (vector query per turn) | Medium (re-embed cost) | Low (vector store is infra) |

The middle option — tree-sitter codemap as committed markdown with a refresh hook — has the best risk-adjusted shape for a project like Ark.

## Directions for Ark

1. **Ship `ark codemap` as a new subcommand.** Tree-sitter-based, writes to `docs/CODEMAPS/<module>.md` by default. Idempotent. Per-language tree-sitter grammars loaded by default-feature. The deliverable is a side-effect-free generator the user can invoke or wire into a hook.

2. **Define a `docs/CODEMAPS/` convention.** Even before the generator ships, declare the convention so projects can hand-write codemaps to it. `.ark/specs/project/LAYOUT.md` could add a "Codemaps" subsection.

3. **Hook the `doc-updater` agent rule into Ark.** The user's `~/.claude/rules/` references a `doc-updater` agent that runs `/update-codemaps`. Ark could ship a `doc-updater` subagent template that maintains the codemap on demand.

4. **`ark context --scope codemap` projection.** When `docs/CODEMAPS/` exists, surface its contents (or relevant slice) in `ark context` output. Closes the loop between codemap-as-committed-artifact and codemap-as-context.

5. **Document codemap as the "structural memory" layer.** Memory hierarchy (project SPECs = behavioural memory) lacks a structural-memory layer today. Codemap fills that gap. Add a one-paragraph treatment in `docs/book/src/reference/memory-hierarchy.md` (proposed in section 03 INDEX).
