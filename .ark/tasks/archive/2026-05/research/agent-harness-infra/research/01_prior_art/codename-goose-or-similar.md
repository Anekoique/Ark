# Plandex (the chosen "one more relevant CLI peer")

> The brief allowed picking one more CLI peer: continue.dev's `cn`, plandex, or "whatever you discover is the most relevant CLI peer." I picked **Plandex** because it occupies a different design point than every other file: client-server architecture, 2M-token context budget, cumulative diff sandbox, tree-sitter indexing of huge repos. The next-closest unfilled slot would be `continue.dev`'s CLI; a short comparison to it is in the "Adjacent peers" section at the bottom.

## Identity

- **Name:** Plandex
- **Repo:** https://github.com/plandex-ai/plandex
- **License:** MIT
- **Primary maintainer:** Dane Schneider (founder, `plandex-ai`)
- **Language:** Go (CLI + server) + Python (embedded LiteLLM proxy)
- **Stars / momentum:** 15,384 (as of 2026-05-20, via `gh repo view`). Smaller than Aider/Cline/Goose but with a distinct technical positioning around *large codebases*.
- **Homepage:** https://plandex.ai

## Positioning

Plandex's tagline: "open-source AI coding agent designed for large projects and real-world tasks." Where Aider optimizes the per-turn diff format, Plandex optimizes for **scale of context and reviewability of change.** Three design choices set it apart:

1. **Client-server architecture.** The CLI is a thin client; a Go server holds plans, context, conversation history, and routes through an embedded LiteLLM proxy. Plans persist across machines and can be self-hosted.
2. **Cumulative diff sandbox.** AI-generated changes accumulate in a sandbox (not your tree). You review, edit, accept, or reject — the working tree is never touched until you apply.
3. **2M-token context window with tree-sitter project maps.** Plandex aggressively indexes the codebase (20M+ tokens of indexable surface) and selects relevant material on demand.

This is the "for big monorepos" answer in the prior-art space.

## Primitives

User-facing nouns:

- **Plan** — the named unit of work. A plan has a conversation, a context set, a model config, and a sandbox of pending changes.
- **Branch** — a fork inside a plan (different exploration paths).
- **Context** — files, directories, URLs, or pieces of text loaded into the plan; tree-sitter-mapped at directory granularity.
- **Sandbox** — the staging area where AI-generated edits live before being applied to the user's tree.
- **REPL** — the interactive shell mode with fuzzy auto-complete.
- **Server** — the long-running backend (local Docker by default, self-hostable to Railway/cloud).

User-facing verbs:

- `plandex new` (create plan), `plandex cd <plan>` (switch plan), `plandex tell` (chat turn)
- `plandex load <path>` (add to context), `plandex map <dir>` (tree-sitter map a dir)
- `plandex diff`, `plandex apply`, `plandex reject`
- `plandex log` (plan history), `plandex rewind` (undo to a snapshot)
- REPL: `\tell`, `\load`, `\diff`, `\apply`, autocomplete with `Tab`

## Workflow model

Representative flow:

1. **Server running** — either Docker-locally (`plandex start`) or self-hosted.
2. **Create plan**: `plandex new` (named, optionally branched).
3. **Load context**: `plandex load src/auth/ --recursive` (tree-sitter project map for the dir; selective file inclusion based on relevance).
4. **Chat mode** (default): REPL drops into chat-only — Plandex reasons about the plan without making edits.
5. **Tell mode**: `\tell "implement JWT refresh"` — Plandex generates changes into the sandbox.
6. **Diff review**: `\diff` shows cumulative pending changes across all sandbox files.
7. **Iterate** — refine via more `\tell`s; changes update cumulatively.
8. **Apply**: `\apply` writes the sandbox to disk and (optionally) runs configured commands. Each apply is a rewind point.
9. **Commit** outside Plandex (or via configured post-apply command).

The cumulative-sandbox-then-apply model is the workflow innovation. Aider applies immediately; Cline applies-per-tool-call-with-checkpoint; Plandex accumulates *across many turns* and applies in one batch.

## Context & memory

**Tree-sitter project maps** at directory granularity:

- Maps include file paths + symbol signatures (functions, classes, types) — no bodies unless loaded.
- 20M+ tokens indexable (the dir-map is much smaller than the bodies).
- Model picks relevant maps per turn.

**2M-token context window** by default (LiteLLM-routable to providers that support it; falls back to smaller windows on others).

**Per-file context limit: ~100k tokens.** A single huge generated file (e.g., a transpiled bundle) can be loaded without blowing context if it's under that ceiling.

**Persistent plan state** — server-side. Plans are databases (Postgres in the deployment template).

**No vector RAG**, no embedding store. Tree-sitter symbol indexing + LLM selection. This is the "code-aware structural retrieval" position.

## Tool / capability surface

**Built-in tools:**

- File read / write / diff / apply (via sandbox)
- Tree-sitter mapping (30+ languages)
- Command execution (configurable per-plan)
- Image attachment (paste screenshots into chat)
- URL loading

**MCP support:** Limited / via LiteLLM extensions. The plugin surface is the server, not MCP per se. As of 2026 mainline Plandex does not expose itself as an MCP server.

**Plugin model:** Provider routing (LiteLLM) is the extension point. No formal plugin SDK.

**Sandbox boundaries:** The *diff sandbox* is the safety property — your tree is never mutated until `apply`. Command execution can be confined via configured allowlists per plan. No Docker-per-session.

## Integration model

**Terminal-first, client-server.** The CLI is your interface; the server is a daemon (local Docker or remote). No IDE integration. The web admin UI is a recent addition (cloud tier).

## Multi-agent / orchestration

**Solo, with plan branching as the parallelism primitive.** Branches let you explore multiple approaches within a plan; they share earlier context but diverge. No subagent dispatch.

## Spec / artifact system

- **Plans are durable.** Server-side state survives reboots; plans can be exported (`plandex export`).
- **No PRD/PLAN/VERIFY decomposition.** The plan is a single conversation with optional branches.
- **No SPEC promotion.** Plans don't get distilled into reusable conventions.

## Strengths

- **Cumulative diff review is the standout UX.** "Generate a lot of changes, then review them all at once before they touch my tree" is what reviewing a real PR feels like — much closer to human-friendly than Aider's commit-per-turn or Cline's approve-per-call.
- **2M context + tree-sitter mapping is the right answer for large repos.** Aider's repo map is smaller; Plandex's combined approach scales further.
- **Server-side state is portable.** Same plan across machines.
- **Rewind to snapshot** at any apply is a clean undo.
- **Self-hostable.** Postgres + Go server is a small operational footprint.

## Weaknesses / gaps

- **Operationally heavier.** A Docker server is a real prerequisite; Aider/Goose/Codex/Claude Code are pure-process.
- **No editor integration.**
- **Modest community.** 15k stars vs. 60-80k for the leaders. Less plugin / community gravity.
- **No MCP — the server's plugin model didn't pivot when MCP took off.**
- **No multi-agent or subagent dispatch.**
- **No tier/ceremony.** Every change goes through one plan.

## Directions for Ark

1. **Cumulative-diff review as a `task verify` extension.** Plandex's "generate many changes, then review them all" maps onto Ark's EXECUTE→VERIFY transition. Today, Ark's VERIFY is "audit shipped code" via prose checklist items. A `ark agent task verify --diff` projection that emits a structured diff (every modified file × every modified region, with cross-references back to PLAN goals) would give reviewers a Plandex-style staging view *inside the existing workflow*.
2. **Tree-sitter project maps for `ark context`.** Same direction as Aider, but Plandex shows it scales further. Specifically: maintain a `.ark/cache/code_map.json` (gitignored) that holds dir-level symbol signatures, regenerate on file changes, and surface it via `ark context --scope code` for any agent that asks. Compose with Aider-style PageRank salience.
3. **Plan branching as a worktree shortcut.** Plandex's branching lets users explore alternatives without `git checkout`. Ark already has worktrees; consider whether a `ark agent task fork --slug <new>` that clones an active task (PRD + active PLAN + a fresh worktree) would be useful for "let me try a different approach without losing the current one."
4. **Server / daemon mode evaluation (deferred).** Plandex's client-server architecture is heavy *and* enables remote state. Ark today is a pure CLI; a daemon mode is not a near-term need but is the natural evolution if Ark grows into ArkOS (per `reference/arkos/`). Note the architectural trade-off without committing.
5. **Self-host story for teams.** Plandex's Postgres-backed plan store has team-share implications. Ark's state is per-checkout (workspace feature); cross-developer sharing happens via git. This is a deliberate choice — but worth a SPEC entry that names it as "Ark's collaboration model is git, not server-mediated state."

## Adjacent peers (briefly)

These weren't picked for a full file but warrant a mention for the INDEX:

### continue.dev CLI (`cn`)

- Repo: https://github.com/continuedev/continue (33,281 stars, Apache 2.0, TypeScript)
- **Headline:** "Source-controlled AI checks, enforceable in CI." The CLI is positioned at CI / async use rather than interactive coding.
- **Primitives:** Assistants (defined in `config.yaml`), Rules (concatenated into system message), Hub (cloud catalog of assistants).
- **Permission model:** Tool permissions persist in `~/.continue/permissions.yaml`; CI-friendly.
- **Differs from Ark:** ships as a library + CLI for embedding AI checks in CI; less interactive than Ark/Aider/Cline/Goose.

### opencode

- Already a Ark integration target (`specs/features/opencode-support/SPEC.md`). Not researched as a peer since Ark already targets it.

## Sources

- [plandex-ai/plandex on GitHub](https://github.com/plandex-ai/plandex) (queried 2026-05-20)
- [Plandex docs](https://docs.plandex.ai/)
- [Context Management — Plandex Docs](https://docs.plandex.ai/core-concepts/context-management/)
- [Plandex Review — VibeCodingHub](https://vibecodinghub.org/tools/plandex)
- [Deploy Plandex — Railway](https://railway.com/deploy/plandex) — deployment topology
- [continuedev/continue on GitHub](https://github.com/continuedev/continue)
- [How to Use Continue CLI (cn) — Continue Docs](https://docs.continue.dev/guides/cli)
- [Building Cloud Agents with Continue CLI — Continue Blog](https://blog.continue.dev/building-async-agents-with-continue-cli)
