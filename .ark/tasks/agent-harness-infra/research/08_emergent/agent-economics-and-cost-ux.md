# Agent Economics and Cost UX

The cost surfaces production agent harnesses ship — and the gap Ark has by being silent on cost. Bridging the gap is cheap; the question is whether to.

## What harnesses surface today

### Aider — `/tokens`

Aider prints token counts per turn. Optional `--show-cost` adds rough dollar estimates based on configured provider rates. Lives in the chat surface.

User behaviour: most users glance at it occasionally, calibrate "this session is small / medium / large", and don't track total cost rigorously.

### Claude Code — `/cost`

Claude Code's `/cost` slash command shows current session token usage + estimated cost. Anthropic's billing dashboard is the canonical truth.

User behaviour: similar — occasional glance; the per-session signal is informative even if the per-API-call detail is in the dashboard.

### Cursor — spend limits

Cursor allows per-user, per-month spend caps in the billing dashboard. Hitting the cap pauses agent activity until the user raises it.

User behaviour: power users set caps explicitly; casual users don't notice until they hit one.

### Devin — ACU billing

Cognition bills Devin in "ACUs" (Autonomous Compute Units). Each task displays estimated ACUs at submission and actual ACUs on completion. The ACU framing abstracts over the underlying model invocations.

User behaviour: ACU as a unit lets users budget per-task ("this is a 5-ACU task") more readily than "tokens".

### OpenAI Codex CLI — issue tracker

Codex CLI does not surface cost in-CLI; issue #5085 tracks "Cost Tracking & Usage Analytics". Users currently rely on OpenAI's API dashboard.

### Replit Agent — "200-minute autonomy"

Replit's framing is time-based, not cost-based. Each agent run gets a window; users budget time, not tokens.

## What Ark surfaces today

Nothing direct. The closest signals are:
- Tier name in task metadata (quick / standard / deep / research) — *implicit* cost prediction (deep = more turns).
- `max_iterations` in deep-tier task.toml — *implicit* iteration cap.
- Phase progression — *implicit* "how far through".

The user has no in-Ark signal of estimated cost.

## Why this matters

Three drivers:

1. **User trust.** Users running deep-tier tasks for the first time don't know whether they're spending $1 or $20. Surprise bills erode trust in the harness.
2. **Self-throttling.** When the agent sees its own cost surface, it can choose lower-cost paths (use Haiku for read-only research; reserve Opus for the verifier).
3. **Tier validation.** "Deep tier costs ~10× standard" is a load-bearing claim; if Ark can't show it, the tier UX feels arbitrary.

The counter-argument: cost surfaces add complexity, may give users wrong numbers (provider pricing changes), and may discourage use ("$5 to verify this PR? I'll skip").

For Ark the right balance is *tier-implicit estimates*, not per-token tracking.

## Proposed cost UX for Ark

A minimal surface in `ark context`:

```json
{
  "schema": 2,
  "scope": "session",
  "...": "...",
  "cost_estimate": {
    "current_task_tier": "deep",
    "typical_iteration_cost_usd": "0.50-2.00",
    "expected_iterations": "3-5",
    "rough_total_usd": "1.50-10.00",
    "currency_disclaimer": "Estimates based on Claude Sonnet 4.6 at 2026-Q2 rates."
  }
}
```

Tier × typical-iteration-cost × expected-iterations gives a range, not a number. Users learn the shape.

Plus a `task.toml` field for actual recorded cost (if the host platform exposes it):

```toml
[cost_actual]
recorded_at = "2026-05-20T13:00:00Z"
estimated_usd_total = 3.42
notes = "From Claude Code /cost at commit time."
```

Optional; populated only if the platform allows. Lets the user reconcile estimate vs. actual.

## Per-tier cost shape (illustrative)

Drawing on order-of-magnitude estimates:

| Tier | Typical session shape | Rough cost range (USD) |
| ---- | --------------------- | ---------------------- |
| Quick | 1 PRD + 1 EXECUTE pass | $0.05–$0.50 |
| Standard | PRD + PLAN + EXECUTE + VERIFY | $0.30–$2.00 |
| Deep (1 iteration) | PRD + PLAN + REVIEW + EXECUTE + VERIFY | $1.00–$5.00 |
| Deep (3 iterations) | Above × ~3 | $3.00–$15.00 |
| Research (10 topics, parallel) | 10 sub-agents × medium-cost each | $2.00–$10.00 |

These are illustrative — actual cost depends on model, file count, dispatch parallelism. The tiers communicate *shape*, not precision.

This very corpus generation cost: a research-tier task with 10 parallel sub-agents. Per the user's session, the total token cost is on the upper end of the "Research" range above.

## Cost as a workflow nudge

A subtle benefit of tier-implicit estimates: it nudges users to *pick the right tier*. Today the tier-selection guidance is qualitative ("pick the smallest tier that fits"). With cost surfaces it becomes quantitative ("Quick: $0.05–$0.50; Standard: $0.30–$2.00 — am I sure I need Standard?").

The hard part is keeping the estimates honest as model prices change. A configuration field in `.ark/config.toml` could let users override:

```toml
[cost_model]
input_per_million = 3.00      # USD
output_per_million = 15.00
provider_label = "Claude Sonnet 4.6"
```

`ark context` reads this; if unset, uses sensible defaults documented as of a specific date.

## The plan-execute split as cost arbitrage

A pattern in production: cheap model plans, expensive model executes. Aider's architect/editor split was the prototype.

Ark already has this structurally (DESIGN/PLAN phases produce artifacts that the EXECUTE phase consumes). The cost arbitrage isn't surfaced — the user can't easily configure "use Haiku for design, Opus for execute".

Adding model-per-phase as an optional `.ark/config.toml` field:

```toml
[models.phase]
design = "claude-haiku-4-5"
plan = "claude-haiku-4-5"
review = "claude-opus-4-7"
execute = "claude-sonnet-4-6"
verify = "claude-sonnet-4-6"
```

Ark wouldn't enforce this — the host agent picks the model — but `ark context` could surface the recommendation in the output for the current phase. Lets the user (or the host agent) pick optimally.

## Failure modes of cost surfaces

1. **Stale prices.** Vendor changes pricing; Ark's defaults become wrong. Mitigation: document the as-of date; let users override.

2. **Multi-provider confusion.** User runs on multiple providers (Anthropic, OpenAI, Google). Per-provider estimates need per-provider config. Mitigation: keep cost surface single-provider with override.

3. **Discouragement effect.** "Deep tier costs $10" makes users skip deep tier. Sometimes appropriate (use the smallest tier), sometimes counterproductive (deep tier is the *right* choice for a refactor). Mitigation: frame as ranges, not points.

4. **Accuracy criticism.** "Your $5 estimate became $12 in reality" — users complain. Mitigation: explicit disclaimer; estimates are approximate.

5. **Cost-as-control creep.** Once cost is surfaced, users want hard caps. Implementing hard caps requires deeper hook integration. Mitigation: surface, don't enforce.

## Directions for Ark

1. **Add a `cost_estimate` field to `ark context --scope session --format json`.** Tier-implicit, ranged, disclaimer-marked. No new infrastructure; just metadata + a config file.

2. **Document tier cost shape in `workflow.md`.** A table near the tier explanations: "Quick ~ $0.05–$0.50; Deep ~ $1–$15". Sets expectations.

3. **Optional `[cost_actual]` field in `task.toml`.** Populate at commit time if the host exposes a cost number. Lets users compare estimate vs. actual; helps future estimates.

4. **`[models.phase]` config in `.ark/config.toml`.** Optional per-phase model recommendation. Surfaces in `ark context`. Lets the host agent (or user) pick the right model.

5. **Skip hard cost caps for now.** Surface, don't enforce. Hard caps need host-platform cooperation; the leverage isn't there yet.
