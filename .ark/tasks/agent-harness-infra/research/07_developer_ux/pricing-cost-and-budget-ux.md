# Research: Pricing, Cost, and Budget UX

- Query: Cost surfaces in agent UX — token usage display, run-cost projections, budget caps, free-tier ergonomics. Aider's token meter, Claude Code's `/cost`, Cursor's spend limits, the plan-execute split as cost control. Is Ark too silent on cost?
- Scope: external (primary) + internal (Ark's role)
- Date: 2026-05-20

## Findings

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `crates/ark-cli/src/main.rs` | Nothing cost-related. No `/cost`-style subcommand, no token meter. |
| `crates/ark-core/src/commands/agent/` | Workflow operations. Free of cost vocabulary. |
| `.ark/workflow.md` | The tier system (`quick` < `standard` < `deep`) is implicitly cost-graded — deep tasks are more agent work — but never described as such. |
| `templates/claude/commands/ark/*.md` | Slash command bodies that orchestrate agent work. No token budgets, no cost projections. |
| `crates/ark-core/src/error.rs` | No cost-related error variants. |

The honest answer: Ark is silent on cost because Ark itself brokers no model calls. The CLI sets up workflows and tracks state; the model spend happens inside Claude Code / Codex / OpenCode, governed by those platforms' own cost tooling.

### Code patterns

There is no Ark cost surface to cite. The relevant "cost-shaped" content lives in conceptual decisions:

- **Tier grading** in `.ark/workflow.md:40-49`:
  > - **Quick** — reversible in one commit, no new abstractions. Artifact: `PRD.md`.
  > - **Standard** — feature work with testable scope, no API/architecture break. Artifacts: `PRD.md`, `PLAN.md`, `VERIFY.md`.
  > - **Deep** — architectural, cross-cutting, or new subsystem. Artifacts: `PRD.md`, `NN_PLAN.md` ⇄ `NN_REVIEW.md` (looped), `VERIFY.md`, promoted `SPEC.md`.

  Each higher tier produces more artifacts → more model output → more tokens. The "pick the smallest tier that fits" guidance (l. 49) is implicitly a cost-control prompt, but never says so.

- **The PLAN ⇄ REVIEW loop** (deep tier) is conceptually a plan-execute pattern: a planner generates a PLAN, a reviewer critiques, the planner revises. The reviewer is potentially a separate (cheaper or more rigorous) model. Workflow:200-204:
  > `task.toml.max_iterations` (typically 3–5). If exhausted, halt and ask the user.

  The iteration cap exists. It bounds the maximum cost of a deep task. Not explicitly cost-framed, but functionally a budget.

- **The `ark-researcher` / `ark-reviewer` / `ark-verifier` subagents** (`subagent-support` feature SPEC) — Ark's structural acknowledgment that not every step needs the same model. Different agents can run with different cost profiles. The CLI doesn't enforce this; it provides the slots.

### External references

#### Aider's `/tokens` command

From [aider.chat *In-chat commands*](https://aider.chat/docs/usage/commands.html):

> "Use `/tokens` to see token usage. The `/tokens` command shows you the context and the cost, letting you see what each prompt will cost you."

The output includes:
- Current context size (tokens)
- Per-file breakdown of token cost
- Estimated dollar cost for the next prompt

Aider also supports `/thinking-tokens 4k` or `/reasoning-effort low` to throttle reasoning-model token spend ([aider *Reasoning models*](https://aider.chat/docs/config/reasoning.html)). `/thinking-tokens 0` disables thinking entirely.

Quote: "Aider never enforces token limits, it only reports token limit errors from the API provider" ([aider *Token limits* docs](https://aider.chat/docs/troubleshooting/token-limits.html)). The user sees the cost; the API enforces the ceiling.

The estimate is approximate: "It's important to note that the token counts that aider reports are estimates."

#### Claude Code's `/cost`

From [How Do I Use AI, *What Does /cost Do in Claude Code?*](https://www.howdoiuseai.com/blog/2026-04-16-what-does-cost-do-in-claude-code-token-tracking):

> "/cost shows you exactly how many tokens you've used and an estimated dollar amount for your current session. The dollar estimate is calculated locally from your token counts and the model's published rates."

Aliases: `/usage`, `/stats`. Behavior differs by plan:

- **API users**: shows token consumption + estimated USD.
- **Pro/Max subscribers**: shows token counts but no USD (cost data not relevant for fixed subscription).

Important caveat from [Claude Code SDK *Track cost and usage*](https://code.claude.com/docs/en/agent-sdk/cost-tracking):

> "The `total_cost_usd` and `costUSD` fields are client-side estimates, not authoritative billing data. The SDK computes them locally from a price table bundled at build time, so they can drift from what you are actually billed."

#### Cursor's spend limits

From [Cursor Docs *Spend limits*](https://cursor.com/docs/account/billing/spend-limits) and [DEV Community *Set a Spending Limit Before Your Cursor Agent Goes Rogue*](https://dev.to/ai-agent-economy/set-a-spending-limit-before-your-cursor-agent-goes-rogue-3od6):

- Per-month spend cap, configurable in account settings.
- Team and individual member spend limits.
- Usage thresholds (alert at 50% / 75% / 100% of monthly budget).
- "You can disable overages in settings to hard-cap your spending at the plan price."

Cursor's billing model shifted from a "fast requests" pool to actual token-based billing. The implication: cost visibility had to become first-class because users were getting surprised by usage they couldn't predict.

Cost-saving tips from [CodePick *How to Save Money on Cursor*](https://codepick.dev/en/guides/cursor-cost-saving/):

> "Track token counts since the longer the prompt or context window you send to a model, the more tokens it consumes; establish internal usage rules for how to use Cursor… Use Max mode only when needed as it increases context window… be mindful with agents since they perform multi-step operations that draw credits for each step and file processed."

#### The plan-execute split as cost control

From [LangChain *Plan-and-Execute Agents*](https://blog.langchain.com/planning-agents/) and [Medium *Separating AI Agents into Planner and Executor*](https://medium.com/@jaouadi.mahdi1/separating-ai-agents-into-planner-and-executor-7705b58d79fd):

> "The plan-execute model uses an expensive reasoning model to plan while a cheap fast model executes, which cuts costs significantly compared to running a frontier model on every ReAct step."

> "A 10-step task that needs 10 LLM calls in ReAct needs 1-2 in Plan-and-Execute."

The cost arithmetic:
- ReAct: 10 LLM calls × frontier-model price.
- Plan-and-Execute: 1 frontier-model planning call + N small-model execution calls. The total cost can be ~5-10x lower.

Quote from [JumpCloud *Understanding the Plan-and-Execute AI Agent Framework*](https://jumpcloud.com/it-index/understanding-the-plan-and-execute-ai-agent-framework):

> "The planner-executor model delegates repetitive work to cheaper, faster models, which lowers the average cost per transaction."

Decoupled planner training (e.g. EAGLET) achieves "approximately 8× reduction in RL cost compared to standard baselines" (from the *Goal Without a Plan* arXiv paper at https://arxiv.org/pdf/2510.05608).

#### Continue.dev's per-model selection

Continue.dev's `config.yaml` lets users specify different models for different roles ([Continue Docs *Agent Mode model setup*](https://docs.continue.dev/ide-extensions/agent/model-setup)):

```yaml
models:
  - role: chat
    model: claude-3-5-sonnet-latest
  - role: edit
    model: claude-3-5-haiku-latest
  - role: autocomplete
    model: codestral-latest
```

Different role, different model, different cost profile. Cost-aware by configuration rather than by command.

#### Codex cost tracking (open issue)

From [openai/codex#5085 *Cost Tracking & Usage Analytics*](https://github.com/openai/codex/issues/5085) — an open feature request for cost tracking in OpenAI Codex CLI. The fact that this is *requested* (rather than shipped) signals: even tools from OpenAI/Anthropic don't always have cost surfaces by default.

#### Token-monitoring tools as a category

Community-built cost trackers exist as a meta-category:

- `ccusage` — reads Claude's local JSONL logs, shows usage by date/session/project.
- `cccost` — instrument Claude Code to track actual token usage and cost (https://github.com/badlogic/cccost).
- `claude-usage` (https://github.com/phuryn/claude-usage) — local dashboard for tracking Claude Code token usage, costs, session history.
- `Claude-Code-Usage-Monitor` (https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor) — real-time monitor with predictions and warnings.

This is a tell: the official tools' cost surfaces are insufficient enough that an ecosystem of monitors has grown around them. Cost visibility is in demand.

#### Free-tier ergonomics

Different tools handle free tiers differently:

- **Claude Code Free**: limited messages per day; gracefully degrades to "you've hit your limit, try again in N hours."
- **Cursor Free**: 200 slow requests; once exhausted, the agent stops working until reset.
- **GitHub Copilot Free**: 2k completions / 50 chats per month, with hard cutoffs.
- **Aider**: pure BYO-API-key; free tier is whatever the underlying provider gives.

The pattern: tools that own the model relationship (Cursor, Copilot, Claude Code) define their own free tier. Tools that BYO API key (Aider, Continue.dev, Cline) inherit whatever the provider offers.

#### "Silent burn" — the agentic-cost horror

Recurring story in [Vantage *Cursor Pricing Explained*](https://www.vantage.sh/blog/cursor-pricing-explained) and similar coverage: users wake up to surprise bills from agents that ran overnight on a stuck loop. Agentic systems compound cost faster than chat-style systems because each step can trigger more steps. The "agent that thinks it should call itself" pattern.

Mitigations:
1. Hard spend caps (Cursor's "disable overages").
2. Per-action approval (Cline's confirmation-before-execute).
3. Iteration counters (Ark's `max_iterations`, LangChain's `max_iterations`).
4. Plan-then-execute (one planning call + bounded execution).
5. Cost projections before run (Aider's `/tokens`).

Ark uses (3) explicitly. (4) is structural to the workflow. (1), (2), (5) are not present and arguably can't be (Ark doesn't broker the calls).

### What Ark does today, in cost terms

| Cost lever | Ark today |
| --- | --- |
| Tier grading (cheap to expensive paths) | Yes — `quick`/`standard`/`deep` |
| Iteration cap (deep tier) | Yes — `max_iterations` (3-5 default) |
| Plan/execute split (separate planner+reviewer) | Yes — `task plan` → `task review` → `task execute` |
| Per-role agent selection | Partial — `ark-researcher` / `ark-reviewer` / `ark-verifier` subagent slots exist |
| Token meter / cost display | No — Ark brokers no calls |
| Spend cap | No |
| Cost-before-run projection | No |
| Cost-history view | No |

The four "yes" rows are structural cost-control levers that exist *because* the workflow demands them, not *because* anyone designed for cost. The tier system was about ceremony-fit, not cost. The iteration cap was about runaway-loop prevention, not budget. The subagent slots were about specialization, not cost arbitrage. But all four function as cost levers.

### Should Ark surface cost at all?

The argument for: cost is now table-stakes UX in agent tooling. Aider, Cursor, Claude Code, Continue.dev, and the third-party monitor ecosystem all surface it. A user adopting Ark in 2026 has those expectations.

The argument against: Ark brokers no model calls; surfacing cost requires either (a) reading platform-specific log files (Claude Code's JSONL, Codex's logs) or (b) injecting an LLM-call wrapper that Ark doesn't currently have. Both deepen Ark's coupling to specific platforms.

A middle path:

- **Surface tier-shaped cost expectation in the slash commands.** `/ark:design --deep` could print "this tier typically uses N tokens of agent output across the PLAN ⇄ REVIEW loop; consider `/ark:design` (standard) if scope is testable." Encourage tier-fit thinking with cost framing.
- **Read platform telemetry on `ark context`.** If `~/.claude/logs/usage.jsonl` exists, summarize session-level usage in the context output. Read-only, opt-in by file presence, no scope expansion.
- **Provide a cost-vocabulary in the workflow doc.** Explicitly name the tier-cost gradient. "Quick is one PRD + one execute; standard adds a PLAN; deep adds the loop. Each tier costs more tokens." Make the gradient visible without measuring tokens.

### Caveats / Not found

- No data on actual cost-per-task for Ark workflows. The tier gradient is described but unmeasured.
- The Codex cost-tracking issue (#5085) was the strongest signal that even first-party tools lack cost surfaces; not all platforms were checked individually.
- Whether reading platform logs (Claude Code's JSONL, etc.) constitutes a stable interface is unclear — these formats may change without notice.
- The community cost-monitor tools (ccusage, cccost) were not surveyed for accuracy or coverage.
- No empirical analysis of "silent burn" frequency in production agent use; the framing rests on Cursor / Vantage coverage rather than measurement.

## Directions for Ark

1. **Add a "cost expectation" paragraph to `.ark/workflow.md`'s tier section.** One sentence per tier framing the token cost qualitatively:
   ```
   - Quick: one PRD + one execute pass; small token spend.
   - Standard: adds a PLAN; moderate spend (typical: 2-3 model calls before EXECUTE).
   - Deep: adds the PLAN ⇄ REVIEW loop; large spend, especially with max_iterations near 5.
   ```
   Zero CLI work, modest doc work, makes the tier gradient cost-legible.

2. **Print iteration-count + cost-shape in `ark context --scope phase --for review`.** Each REVIEW invocation is a separate model call. Showing "iteration 2/5" in the context output reminds the user / agent that a budget is being consumed. The data is already in `task.toml.iteration` and `task.toml.max_iterations`; the surface change is one rendered line.

3. **Add `ark context --include cost` (optional)** that reads Claude Code's JSONL (when present) and summarizes session-level token use. Opt-in via flag, no fail-on-absence. Low coupling: if the log format changes, the feature degrades to "cost unavailable." Surfaces the missing cost view without owning the call path.

4. **Encourage subagent role-cost specialization.** The `subagent-support` feature already provides `ark-researcher`, `ark-reviewer`, `ark-verifier`. The implicit pattern: cheap research, expensive review. Document this explicitly in the subagent SPEC: "Configure `ark-researcher` with a lower-cost model (Haiku); `ark-reviewer` with a frontier model." Doesn't require Ark code changes — just a documented convention.

5. **Provide a `task.toml` budget knob.** Add `[budget]` section with `max_iterations` (already exists), `max_review_tokens` (advisory; ignored by Ark but exposed for slash-command consumption), `model_for_review` (slash-command-readable hint). Slash commands can read it and respect it; Ark just stores it. Treats budget as workflow vocabulary rather than runtime enforcement — consistent with Ark's "the agent does the work, the CLI tracks state" model.
