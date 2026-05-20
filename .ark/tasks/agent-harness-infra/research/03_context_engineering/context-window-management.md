# Context Window Management

- Query: How does the field manage finite context windows, what falls off, and where does Ark's phase projection sit?
- Scope: external + Ark self-context
- Date: 2026-05-20

## What "context engineering" became

Andrej Karpathy's June 2025 tweet recast the discipline: "+1 for 'context engineering' over 'prompt engineering'. … context engineering is the delicate art and science of filling the context window with just the right information for the next step." Anthropic operationalised the term in its September 2025 Effective Context Engineering post: *context is a finite resource with diminishing returns*. The 2026 shape is **attention budget management**, not prompt tweaking — every token spent on background is a token not spent on the next decision.

The frame matters because it inverts the old assumption. Prompt engineering treated the prompt as a single artefact authored once; context engineering treats the *entire trajectory* (system prompt + tool defs + tool results + memory + user message) as a *load-bearing system component* designed and re-designed mid-session.

## Attention falloff: why bigger windows did not solve it

Three measurable effects:

1. **Lost-in-the-middle.** LLMs show a U-shaped attention curve: information at positions 30–70% of the context shows a 5–15 point retrieval drop versus head/tail. Attributed to causal attention + RoPE position encodings.
2. **Context rot.** Chroma tested 18 frontier models (mid-2025) and found *every one* degrades as input length grows, even far below the hard window cap. Anthropic confirmed in its Sep 2025 post: token count drains the "attention budget" linearly.
3. **Multi-needle scaling.** Single-needle benchmarks (Greg Kamradt's NIAH) scale gracefully; multi-needle (RULER, MRCR v2) degrade faster than linearly. Claude Opus 4.6 scores 78.3% on MRCR v2 at 1M tokens — best in class, but down from ~95%+ at 32K.

The implication: a 1M-token window is a *capacity* number; the effective window for high-fidelity recall is much smaller, and shrinks as the prompt gets noisier.

## Strategies in production harnesses

| Strategy | Where seen | Mechanism |
| -------- | ---------- | --------- |
| **Auto-compact** | Claude Code (~95% capacity trigger, instant since v2.0.64) | Summarise older turns into a digest; clear tool outputs first, then conversation, then start a new session with the digest pre-loaded |
| **Manual compact** | `/compact` in Claude Code | User-driven version of the above at a clean break (e.g. between tasks) |
| **Sliding window** | LangChain, Strands Agents | Drop oldest N turns when limit hit; abrupt forgetting |
| **Summarising window** | Strands `SummarizingConversationManager` | LLM-summarises the dropped slice and writes the summary back |
| **Tool result clearing** | Claude API "memory + clear tool uses" cookbook | Drop intermediate tool outputs first (cheapest signal) before touching conversation |
| **Structured note-taking** | Anthropic Cookbook, Claude Code memory tool, Letta archival memory | Agent writes durable notes to disk; pulls back when needed; raw trajectory can be discarded |
| **Semantic message selection** | Anthropic cookbook (advanced) | Embed each turn; re-rank for relevance to current query; drop the least-relevant rather than the oldest |
| **Sub-agent context isolation** | Claude Code Task tool, OpenHands AgentDelegateAction, Goose subagents | Spin a child agent with a clean window; only its final report enters the parent context |
| **JIT loading via tools** | Claude Code (grep/read), OpenHands microagents on keywords | Don't pre-load — let the agent pull files as needed (see `jit-and-progressive-context-loading.md`) |

## Prompt caching: the lever that changes the calculus

Anthropic's `cache_control: { type: "ephemeral" }` markers cache prefixes between turns. Cache writes cost 1.25× input, cache reads cost 0.1× input. The TTL silently dropped from 60 min → 5 min in early 2026 — production workloads that hadn't adapted saw 30–60% cost increases.

What this means for context engineering: **stable prefixes pay for themselves on the first repeat call**. Harnesses now design their context with caching in mind — system prompt + tool defs + project rules go first (cacheable), volatile user turns last. Claude Code keeps its tool schema + CLAUDE.md + AGENTS.md at the head of every request precisely so the cache hits.

Ark's `ark context --scope phase --for X` payload is small (~2–10 KB), so the *direct* cache savings are modest. But the *placement* matters: by hooking on `SessionStart` (see `02_infra_primitives/hooks-and-lifecycle-events.md`), Ark injects its packet *before* user turns, putting it in the cacheable prefix.

## Compaction as a craft, not a feature

Three observations from Anthropic's blog + the Claude Code compaction docs:

1. **Recall before precision.** A compaction prompt's first job is to lose no architectural decisions; once that's safe, tighten by trimming redundant outputs. Reverse the order and you'll throw out the one fact that mattered.
2. **What survives compaction is the *summary*, not the conversation.** The next turn after a compact event is reasoning against a *re-told* story, not the original transcript. Quality of compaction = quality of subsequent decisions.
3. **Sub-agent dispatch is a flavour of compaction.** Instead of compressing post-hoc, you isolate ahead of time: the subagent's 50K-token exploration never enters the parent. (See `compaction-and-handoff.md`.)

## How Ark's phase projection compares

`ark context` is a *projection*, not a compaction — different mechanism, same goal. Ark never holds a long conversation; it re-projects fresh state per phase:

- `Scope::Session` (default) — full snapshot for orientation (`crates/ark-core/src/commands/context/projection.rs:135-148`).
- `Scope::Phase(PhaseFilter::Plan | Review)` — drops `tasks` and `archive`; *filters* `features` by the PRD's `[**Related Specs**]` block (`projection.rs:205-217`, `filter_features_by_related` at `:238-251`).
- `Scope::Phase(Execute | Verify | Commit)` — drops feature specs entirely; keeps project SPECs (`projection.rs:223-229`).

The "what falls off" question Ark answers: features SPECs unrelated to the current task are excluded by *relevance* (related-specs match), not by *recency*. That's structurally different from sliding-window compaction. Compare:

| | Compaction (Claude Code) | Phase projection (Ark) |
| - | ------------------------ | ---------------------- |
| Trigger | Token-count threshold (~95%) | User invocation at phase boundary |
| Discriminator | Recency + LLM summarisation | PRD-declared relevance + phase rules |
| Lossy? | Yes — summary replaces transcript | No — re-derived from filesystem each call |
| Versionable? | Implicit | Yes — `SCHEMA_VERSION = 1` in `model.rs:21` |
| Cost | LLM call to summarise | Cheap I/O + serialization |

The trade-off: Ark cannot remember what the *agent* did in the last turn (no in-prompt conversation history). It can only remember what was written to disk. That forces a hygiene discipline — every important decision lands as an artefact, not a chat reply. The "discipline" is the bet.

## Caveats / Not found

- No public data on the *exact* compaction prompt Claude Code uses. The cookbook example is illustrative.
- "Effective context window" numbers (MRCR v2 at 1M) vary by model release; the 78.3% figure is the Opus 4.6 Feb 2026 number.
- Did not find quantitative evidence that PhaseFilter::Plan's related-specs filter improves outcomes vs unfiltered — would need an A/B study.

## Directions for Ark

1. **Cache-friendly ordering of the projection payload.** Verify the JSON field order produced by `serde` on `ProjectedContext` (`projection.rs:87-120`) is stable across calls within a session so the cacheable prefix actually hits. A field reordering would silently bust the cache.
2. **A `--budget <tokens>` flag on `ark context`.** Render a smaller projection by dropping `archive`, then capping `dirty_files` (already capped at `DIRTY_FILES_CAP = 20` in `model.rs:26`) further, then truncating `recent_commits` (`RECENT_COMMITS_CAP = 5`). Surface `truncated: true` in the output (the field already exists at `projection.rs:118-120`).
3. **Phase-aware tool result clearing hook.** A `PostToolUse` hook (see `02_infra_primitives/hooks-and-lifecycle-events.md`) could rewrite stale `ark context` outputs in the conversation log to a pointer ("re-run `ark context --scope phase --for execute`"), since the projection is idempotent.
4. **Sub-agent invocations get their own fresh projection.** `ark-researcher` / `ark-reviewer` / `ark-verifier` (see `02_infra_primitives/sessions-state-and-resumption.md`) are spawned with a clean Claude Code context. Document in `subagent-support/SPEC.md` that they MUST call `ark context` themselves; the parent's projection does not transitively apply.
5. **Treat `ark context` as the harness's compaction primitive.** When the agent feels the parent session getting noisy, the answer is "summarise to the filesystem (PLAN/VERIFY/research/*.md) then re-orient with `ark context`" — *not* "ask Claude Code to /compact". The filesystem is durable; compaction summaries are not.

## Sources

- [Karpathy on context engineering (X)](https://x.com/karpathy/status/1937902205765607626) — definition origin
- [Anthropic — Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) — Sep 2025
- [Anthropic Cookbook — context engineering: memory, compaction, tool clearing](https://platform.claude.com/cookbook/tool-use-context-engineering-context-engineering-tools)
- [Claude API Docs — Compaction](https://platform.claude.com/docs/en/build-with-claude/compaction)
- [Claude API Docs — Prompt caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
- [Claude Code Compaction explained (2026)](https://okhlopkov.com/claude-code-compaction-explained/)
- [Chroma — Context Rot (Morph summary)](https://www.morphllm.com/context-rot)
- [Understanding AI — Context rot the emerging challenge](https://www.understandingai.org/p/context-rot-the-emerging-challenge)
- [Long-Context Retrieval 2026: Needle-in-Haystack](https://www.digitalapplied.com/blog/long-context-retrieval-needle-in-haystack-2026)
- [Anthropic 2026 Agentic Coding Trends — Opus 4.6 MRCR v2 numbers](https://hivetrail.com/blog/anthropic-2026-agentic-coding-report/)
- [Strands Agents — Conversation Management](https://strandsagents.com/latest/documentation/docs/user-guide/concepts/agents/conversation-management/)
- Ark code: `crates/ark-core/src/commands/context/projection.rs`, `model.rs`
