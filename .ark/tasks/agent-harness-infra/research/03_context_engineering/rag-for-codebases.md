# RAG for Codebases

- Query: When does embedding-based retrieval work for code, when does it fail, what alternatives have the top harnesses chosen, and why does Ark have no RAG path at all?
- Scope: external
- Date: 2026-05-20

## The 2026 split

By mid-2026, coding-agent harnesses sort into three camps:

1. **Embedding-first** — Cursor, Continue, early Sourcegraph Cody.
2. **Code-graph / repo-map** — Aider (tree-sitter + PageRank), Plandex (tree-sitter project maps), Sourcegraph Cody Enterprise (deprecated embeddings → SCIP-based search).
3. **Agentic search (grep + read tools)** — Claude Code, Codex, Devin. *No persistent index; the agent searches at runtime.*

Each implements a different theory of "what does the agent need to know about this codebase?". The empirical answer in 2026 is that agentic search has won the *terminal-class* slot. Embeddings retain ground in IDE-class assistants where the user-experience demands sub-second @-context.

## Embedding-first: how Cursor does it

Cursor's `@codebase` is the canonical embeddings pipeline:

- **Chunking:** files split into syntactic chunks at file-change events (tree-sitter assisted).
- **Embedding:** chunks sent to a remote service, embedded, content-hashed; embeddings cached by chunk content (unchanged chunks hit the cache on re-index).
- **Storage:** chunk embeddings + `(file_path, start_line, end_line)` metadata in a remote vector DB (Turbopuffer). **Code itself is not stored** — Cursor sends a one-way obfuscated path so the server can return identifiers without ever holding source.
- **Query path:** user prompt → embedding → nearest-neighbour search → list of `(obfuscated_path, line_range)` → client reads those chunks locally → sends back to server with the prompt.

The privacy story (path obfuscation) is the load-bearing differentiator vs naive RAG-on-source. The retrieval story is otherwise textbook: chunk, embed, nearest-neighbour, hydrate.

Continue's `@codebase` (now deprecated in favour of `@code`) followed the same shape but local-first: embeddings via `transformers.js`, stored under `~/.continue/index/` in LanceDB. Migrated to LanceDB precisely because it was the only embedded TS vector store with disk-backed fast lookup and SQL filtering.

## Code-graph: how Aider does it

Aider's repo-map is the clearest case study of "structural over semantic":

- **Symbol extraction:** tree-sitter queries pull `name.definition.*` (class/function defs) and `name.reference.*` (call sites). Each tag → `(rel_fname, fname, line, name, kind)`. SQLite cache keyed by mtime.
- **Graph construction:** files = nodes; for each reference of symbol `S` defined in file `F`, add an edge `referring_file → F` weighted 1.0. Self-loops at weight 0.1 for definitions with no references (prevents disconnected nodes).
- **Ranking:** NetworkX `pagerank` with personalisation. Files in the current chat get a 50× weight bias; identifiers mentioned in conversation get 10×; symbols ≥8 chars long get 10×.
- **Output:** the top-N files' signatures (class names, function decls), formatted as a compact `ctags`-style summary. Sent to the LLM as a system message every turn.
- **Refresh:** lazy, on mtime change; cached results invalidate per-file.
- **Fallback:** for languages with weak tree-sitter grammars (C++ in early versions), uses Pygments lexer tokenisation.

Aider's [blog post](https://aider.chat/2023/10/22/repomap.html) explicitly compares: tried embeddings, tried ctags-only, settled on tree-sitter + PageRank because **the graph is the signal** — code's structure (who calls whom) is more predictive of relevance than semantic similarity.

## Agentic search: how Claude Code does it

Anthropic's blog and Cursor's lobste.rs thread converge on a public claim: **Claude Code does not embed or vector-search the codebase**. The agent uses:

- `Glob` for filename patterns
- `Grep` (ripgrep underneath) for content
- `Read` to hydrate matches
- `Bash` for arbitrary search (`find`, `git grep`, etc.)

The pitch: grep returns exact matches, works on any repo without preprocessing, fails loudly. The agent's *reasoning* picks the right query; the index is the filesystem itself.

The empirical claim Anthropic makes ("agentic search outperformed RAG by a lot") is unfalsified publicly, but the *cost* is well documented: token consumption scales with the search depth. On a million-line repo with a vague query, an agentic loop can chew through tens of thousands of tokens before converging. Critics (Milvus blog) point out the worst-case token blow-up.

## Sourcegraph Cody: the migration

Cody is the most instructive case because it visibly switched. Beta Cody used OpenAI text-embedding-ada-002 over source; **Cody Enterprise removed embeddings** in favour of Sourcegraph's native search (Zoekt + SCIP graph traversal). Stated reasons:

- Embeddings don't scale across thousands of repos without per-repo re-index pipelines.
- Sending source to embedding processors raised compliance friction.
- Native search delivered "equal or better quality retrieval" — for Cody's specific access patterns.

The lesson: at *Fortune 500 monorepo scale* (Palo Alto Networks, Qualtrics — both 1000–2000+ devs), the operational cost of embeddings dwarfs their retrieval-quality benefit. Smaller codebases get the opposite trade-off.

## When RAG works on code, when it doesn't

| Works | Fails |
| ----- | ----- |
| Concept queries ("where do we handle auth?") on a *cleanly chunked* codebase with per-function granularity | "Find the bug in this function" — embedding distance is uncorrelated with bug locality |
| Discovering similar patterns ("functions like this one") | Tracking call graphs / dependency chains (structure beats semantics) |
| Onboarding / exploration: a developer skimming, asking broad questions | Edits requiring exact symbol match (you don't want "similar" to a function name, you want *that* function) |
| Long-form documentation queries | Refactors spanning >1 file (the graph is what matters) |
| Stable codebases — index refresh cost amortises | High-churn codebases — stale embeddings drift, re-index latency hurts UX |

Greptile's blog ("Codebases are uniquely hard to search semantically") lists the systematic failure modes: code is *underspecified in natural language*, embedding similarity is dominated by surface features (`def __init__` appearing everywhere clusters every constructor together), and the gap between query language ("how do we deduplicate users?") and code language (`dedupe_by_email`) is large.

## Why Aider chose graph over embeddings

From the 2023 Aider blog post (still the cleanest articulation):

> "We discovered that PageRank on a graph of function/class definitions and references is a better proxy for relevance than embedding similarity — because it captures the structural relationships embeddings can't reliably encode."

Restated:

- **Embedding the *signature* loses the *graph*.** Cosine similarity between `def parse_config` and `def parse_yaml` is high, but a call to `parse_config` is unrelated to YAML.
- **PageRank gives "centrality" cheaply.** The functions everything else calls are obviously important; embeddings have no native notion of importance.
- **Refresh is cheap.** Re-running tree-sitter on changed files + recomputing PageRank scales to ~10K files; re-embedding does not.
- **No external dependencies.** Tree-sitter is in-process; embedding APIs are over the network.

## Why Ark has avoided RAG entirely

Ark sits at the *workflow* layer, not the *file retrieval* layer. The bet is:

1. **The PRD is the retrieval query.** The user/agent writes a PRD before any code change. The PRD's `[**Related Specs**]` block enumerates which feature SPECs apply. `ark context --scope phase --for plan` then *filters* SPECs by that block (`projection.rs:205-217`). The agent doesn't need to *retrieve* relevance — the PRD asserted it.
2. **The harness wraps Claude Code, which already has agentic search.** Building a separate RAG layer on top of grep+read would duplicate what's working. Ark adds workflow structure, not retrieval.
3. **Project SPECs are the durable knowledge layer, not embeddings.** What an embedding pipeline would surface ("how does this project handle errors?") is *already* declared in `specs/project/rust/ERRORS.md` and loaded into every phase projection.
4. **No index = no stale-index problem.** Ark has no `.ark/embeddings.db`. There's nothing to refresh, version, or invalidate. The cost is paid at agent inference time (grep tokens), not at index time.

This is an explicit counter-position. The benefit is operational simplicity and zero-coordination scaling. The cost is that *first-contact orientation* in a totally unfamiliar repo is on the agent — there's no "@codebase summarize this project" affordance.

## Caveats / Not found

- No benchmark publicly comparing Aider repo-map vs Cursor @codebase vs Claude Code agentic search on a fixed task set. Each vendor claims wins on their preferred metric.
- Cursor's embedding model is not publicly disclosed (likely proprietary or a Voyage variant); the chunking heuristics are also not documented.
- No data on whether Ark users have *requested* RAG. The decision appears principled (per the workflow philosophy) rather than measured.
- Continue's pivot from `@codebase` (embedding) to `@code` (presumably more structural) — couldn't find a definitive technical write-up of the new approach.

## Directions for Ark

1. **Document the no-RAG position explicitly in a project SPEC or `docs/`.** Right now the decision is implicit. A short rationale doc would let future agents and contributors stop asking "should we add @codebase?". Tie it to `specs/project/` style — e.g. a `specs/project/architecture/SPEC.md` "Knowledge layers: SPECs over embeddings".
2. **A `--summary` mode for first-contact orientation, *if* needed.** If users ask for "what is this project about?", a project-rooted summary built from `README.md + specs/project/INDEX.md + specs/features/INDEX.md` (no embeddings; pure I/O) would answer 90% of the question. Lives next to `context::render` in `commands/context/`.
3. **Project SPECs as the *retrieval surface*.** When `ark context --scope phase --for plan` filters features by `related_specs`, the agent gets exactly the SPECs the PRD declared relevant. Push this further: surface `related_specs` warnings ("you reference foo/SPEC.md but it doesn't exist") in `gather.rs::parse_features_index` — already partly done via `GatherWarning` (see `model.rs:222-241`).
4. **A repo-map fallback for `/ark:design` orientation, not in-loop retrieval.** A tree-sitter-based map (Rust crate `tree-sitter` already exists; no vector DB needed) printed once when the user can't articulate what to build. Would live as an optional `ark agent task new --explore` extension. Keeps RAG firmly out of the hot path; surfaces it only for the genuinely-disoriented case.
5. **If Ark ever does add embeddings, do it Aider-style.** Symbol-level extraction + PageRank ranking over the symbol graph. Avoid full-source embedding pipelines — they're the failure mode Cody walked away from. The local tree-sitter+SQLite cache pattern is well within Ark's existing scope.

## Sources

- [Aider — Building a better repository map with tree sitter (2023-10-22)](https://aider.chat/2023/10/22/repomap.html)
- [Aider docs — Repository map](https://aider.chat/docs/repomap.html)
- [DeepWiki — Aider Repository Mapping System](https://deepwiki.com/Aider-AI/aider/4.1-repository-mapping-system)
- [Cursor blog — How Cursor indexes codebases fast](https://towardsdatascience.com/how-cursor-actually-indexes-your-codebase/)
- [Cursor blog — Securely indexing large codebases](https://cursor.com/blog/secure-codebase-indexing)
- [Continue Docs — @Codebase embeddings (deprecated)](https://continue.dev/docs/walkthroughs/codebase-embeddings)
- [LanceDB blog — Continue's LanceDB-Powered Evolution](https://lancedb.com/blog/the-future-of-ai-native-development-is-local-inside-continues-lancedb-powered-evolution/)
- [Sourcegraph blog — How Cody understands your codebase](https://sourcegraph.com/blog/how-cody-understands-your-codebase)
- [Sourcegraph docs — Cody (Enterprise dropped embeddings)](https://sourcegraph.com/docs/cody)
- [MindStudio — Why Cursor, Claude Code, and Devin use grep, not vectors](https://www.mindstudio.ai/blog/is-rag-dead-what-ai-agents-use-instead)
- [Milvus — Why I'm against Claude Code's grep-only retrieval](https://milvus.io/blog/why-im-against-claude-codes-grep-only-retrieval-it-just-burns-too-many-tokens.md)
- [Greptile — Codebases are uniquely hard to search semantically](https://www.greptile.com/blog/semantic-codebase-search)
- [arXiv 2605.15184 — Is Grep All You Need? How Agent Harnesses Reshape Agentic Search](https://arxiv.org/abs/2605.15184v1)
- Ark code: `crates/ark-core/src/commands/context/projection.rs`, `gather.rs`, `model.rs`
