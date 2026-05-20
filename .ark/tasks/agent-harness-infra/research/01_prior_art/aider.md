# Aider

## Identity

- **Name:** Aider (AI pair programming in your terminal)
- **Repo:** https://github.com/Aider-AI/aider (formerly `paul-gauthier/aider`; transferred to the `Aider-AI` org during 2025)
- **License:** Apache 2.0
- **Primary maintainer:** Paul Gauthier (paul-gauthier on GitHub), now under the `Aider-AI` org
- **Language:** Python
- **Stars / momentum:** 45,061 (as of 2026-05-20, queried via `gh repo view`). One of the most-starred terminal-first agents; comparable in star count to Goose (45.6k) and well above Plandex (15.4k). Active maintenance — last update mid-May 2026.
- **Homepage:** https://aider.chat

## Positioning

Aider is the canonical "terminal pair programmer" — a Python CLI that you launch inside a git repo, hand a list of files (or let it discover them via repo map), and chat with via an LLM until your working tree contains the change you wanted. It treats the LLM as a peer at the keyboard, not as a planner: edits land directly in the tree (or are applied as diffs), and Aider auto-commits each accepted change with a generated commit message. The model interface is provider-agnostic via LiteLLM; the editing protocol (diff format) is what makes it work well even on small models. Among the CLI peers, Aider is the most "git-native" — every chat turn is a commit, and you navigate via `git`, not via Aider state.

## Primitives

User-facing nouns:

- **Chat session** — the running aider process; no formal persistence besides chat history JSONL.
- **Added files** — explicitly named files Aider may edit (`/add path`). The whitelist is the safety perimeter.
- **Read-only files** — context-only files Aider may not edit (`/read path`). CONVENTIONS.md typically lives here.
- **Repo map** — auto-generated, token-budgeted summary of the codebase Aider isn't allowed to edit but should be aware of.
- **Models** — "main" model for code edits + optional "weak model" for commit messages + optional "architect/editor" pair.
- **Commits** — every accepted change is its own git commit; undo via `/undo` (resets HEAD~1).

User-facing verbs (slash commands):

- `/add`, `/drop`, `/read`, `/clear`, `/reset`
- `/diff`, `/commit`, `/undo`, `/git`
- `/run` (run a shell command, optionally feed output back)
- `/test` (run a configured test command and feed failures back to the LLM)
- `/architect` (toggle architect/editor split)
- `/model`, `/models`, `/web` (paste URL into context)
- `/help`, `/exit`

## Workflow model

A representative flow from "I want to change X" to "X is committed":

1. **Launch** — `aider --model claude-sonnet-4` (or with `.aider.conf.yml` defaults) inside a git repo.
2. **Seed context** — `/read CONVENTIONS.md`, `/add src/foo.py tests/test_foo.py`. The repo map auto-populates background coverage for everything else.
3. **State intent** in chat: "Refactor `foo` to use an iterator instead of a list."
4. **Aider reasons + emits a diff** in its preferred edit format (search/replace block, unified diff, whole-file rewrite, or "udiff-simple" — chosen per-model).
5. **Apply** — Aider parses the diff, writes the file, runs the configured lint/test command if set, and feeds failures back to the LLM for another round.
6. **Commit** — on success, Aider auto-runs `git add` for touched files and `git commit -m "<LLM-generated subject>"` (uses `--weak-model` for cheap subjects).
7. **Iterate or exit** — `/undo` reverts the commit; `/diff` shows since-last-msg; user can `git push` outside Aider.

No "plan" artifact, no "review" artifact. The git log *is* the journal. Architect/editor mode breaks the LLM round-trip into two calls but doesn't introduce a separate planning state.

## Context & memory

**Context window management — the repo map is the headline feature.** Aider analyzes the entire codebase with tree-sitter, builds a graph where files are nodes and symbol references are edges, runs PageRank to score importance, then emits a token-budgeted summary of the top symbols (signatures, class headers, key definitions — no bodies). Budget defaults to 1k tokens (`--map-tokens`). When the user types a message, Aider biases the map toward symbols referenced in the message, then trims to budget. Originally ctags-based (2023); the tree-sitter version (Oct 2023 blog post) is what made Aider competitive on large repos.

**No persistent memory across sessions** beyond:

- The git history (which Aider authored, so it can `git log` to remember).
- `.aider.chat.history.md` — append-only conversation transcript.
- `.aider.input.history` — readline history.
- Auto-loaded `CONVENTIONS.md` if `read: CONVENTIONS.md` is in `.aider.conf.yml`.

No RAG over commits/PRs, no vector store, no "memory bank" abstraction. Cache is prompt-cache-friendly: read-only files are sorted first so they hit cache.

## Tool / capability surface

**Built-in tools:**

- File read, file edit (diff apply with multiple format dialects)
- Shell `/run` (with optional output capture into chat)
- Lint and test runners (`--lint-cmd`, `--test-cmd`)
- Git operations (add/commit/diff/undo)
- Web page ingestion (`/web URL`)
- Voice input (`/voice` — speech-to-text via OpenAI Whisper)
- Image attachment (drop a screenshot or URL into chat)

**MCP support:** No native MCP client integration in mainline Aider as of 2026-05. The architecture predates MCP and the maintainer has stayed focused on the diff-format core. Third-party wrappers expose Aider as an MCP server (e.g. RepoMapper) but Aider does not consume MCP servers.

**Plugin model:** None. Behavior is configured via flags, env vars, and `.aider.conf.yml`. Custom tools = shelling out via `/run`.

**Sandbox boundaries:** None. Aider runs in your shell with your filesystem permissions. The `--yes` flag auto-approves shell commands and edits.

## Integration model

**Terminal-only.** No IDE plugin from the maintainer. Some community ports exist (vscode-aider as an external launcher) but the canonical experience is a TTY chat window. Browser mode (`--browser`) launches a Streamlit UI but it's a thin wrapper over the same Python core.

## Multi-agent / orchestration

Effectively **solo, with one optional split.** The architect/editor mode is a two-model pipeline inside a single Aider process: a strong reasoning model proposes the change in natural language, and a weaker model translates that into the diff format. There is no peer-to-peer agent protocol, no sub-process orchestration, no parallel agents.

## Spec / artifact system

**None by design.** The PRD-equivalent is whatever the user types in chat. The "plan" lives in the architect's natural-language output, but is not persisted as a file. Aider does not emit ADRs, journals, or feature specs. The opinionated artifact is `CONVENTIONS.md` — a read-only rules file the user authors and Aider re-reads at each turn.

## Strengths

- **Diff format is the moat.** Aider has spent years tuning per-model edit formats. SWE-Bench 2024-Q4 results for "Aider + Claude Sonnet" beat much fancier harnesses precisely because the patch-apply path is reliable.
- **Repo map is genuinely smart.** Tree-sitter + PageRank for symbol salience is still the most cited approach to codebase context. Most other tools either dump the full tree or rely on RAG.
- **Git as state.** Zero "Aider directory" pollution. `.aider.tags.cache.v3/` and `.aider.chat.history.md` are the only on-disk traces.
- **Provider portability.** LiteLLM under the hood means Aider works with ~all hosted and local LLMs.
- **No-ceremony onboarding.** `pip install aider-chat && aider` and you're editing.
- **Architect/editor as a cost optimization.** A clear pattern other tools have copied (Cline's plan mode, Continue's planner, even Cursor's chat-then-apply).

## Weaknesses / gaps (where Ark already does or could do better)

- **No persistent task/spec system.** Aider's "what are we doing" lives in chat history, not artifacts. Ark's PRD → PLAN ⇄ REVIEW → EXECUTE → VERIFY is much more durable.
- **No tier ceremony.** Every change gets one commit; no concept of "this is small, skip the plan; this is architectural, write a spec." Ark's quick/standard/deep tiers explicitly target this.
- **No multi-task isolation.** Aider expects one repo, one branch, one chat. Ark's worktree feature solves parallel tasks per checkout.
- **No MCP.** A large gap vs. 2026 peers.
- **No structured review.** Architect mode is one-shot; nothing like Ark's REVIEW loop with severity-graded findings and Response Matrix.
- **No journaling/identity.** Ark's workspace feature (developer identity + per-checkout journals) has no Aider equivalent.
- **No subagent dispatch.** Architect/editor is the closest analog, but it's hardcoded, not a generalized researcher/reviewer/verifier model.

## Directions for Ark

1. **Steal the repo map.** Specifically: tree-sitter + PageRank symbol ranking, token-budgeted, computed once per session and warm-cached. Ark's `ark context` is structural (git + tasks + specs); a code-aware "code context" projection would close the most-cited gap vs. Aider/Plandex.
2. **Adopt CONVENTIONS.md-style read-only pinning at the harness layer.** Ark already has project specs and feature specs; consider whether a per-task "always include" file list (declared in PRD or task.toml) would help subagents stay grounded.
3. **Auto-commit per phase transition is worth evaluating as an option.** Aider's "every accepted change is a commit" model gives a free git-log audit trail. Ark currently bundles into one atomic commit per task; a `--journal-commits` opt-in that creates a commit per PLAN/REVIEW iteration could give similar traceability without changing the default.
4. **Diff-format guardrails for subagent edits.** Aider negotiates edit format per model. Ark's subagents currently emit unstructured text/file writes — formalizing a diff protocol (e.g., require unified-diff for any code modification by a subagent) would reduce silent corruption.
5. **Counter-positioning: lean into the workflow ceremony Aider deliberately rejects.** Aider's design thesis is "agent + git is enough." Ark's thesis is "agent + git + workflow artifacts (PRD/PLAN/REVIEW/VERIFY) compound." Both can coexist; Ark's pitch should be "you outgrew Aider once you needed to plan, review, or split work across tasks."

## Sources

- [Aider-AI/aider on GitHub](https://github.com/Aider-AI/aider) — current repo (queried 2026-05-20)
- [Building a better repository map with tree sitter](https://aider.chat/2023/10/22/repomap.html) — original repo-map blog post (2023-10-22)
- [Repository map | aider docs](https://aider.chat/docs/repomap.html) — current docs
- [Repository Mapping System | DeepWiki](https://deepwiki.com/Aider-AI/aider/4.1-repository-mapping) — independent analysis
- [Specifying coding conventions | aider docs](https://aider.chat/docs/usage/conventions.html) — CONVENTIONS.md pattern
- [Aider LLM Leaderboards](https://aider.chat/docs/leaderboards/) — architect/editor benchmark numbers (2026)
- [Aider AI: Terminal Pair Programmer with Atomic Git Commits — DeployHQ](https://www.deployhq.com/guides/aider) — 2026 overview
