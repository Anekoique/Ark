# Researcher / Reviewer / Verifier — the trio Ark ships

Compiled 2026-05-20. Why these three roles and not others; how the split compares with other shipped trios; the economics of specialization; the anti-pattern of over-specializing.

## What Ark ships

Three Markdown/TOML files per platform under `templates/{claude,codex,opencode}/agents/`:

- **`ark-researcher`** — DESIGN/PLAN-phase knowledge gathering. Reads code, web, prior art. Writes findings to `.ark/tasks/<slug>/research/<topic>.md`. Returns *paths plus one-line summaries* (literal contract phrase per C-14). Allowed: `research/*.md`. Forbidden: code, SPECs, PRD, PLAN, REVIEW, VERIFY, `task.toml`, git mutations.
- **`ark-reviewer`** — deep-tier REVIEW gate. Reads PRD + `NN_PLAN.md` + project SPECs + related feature SPECs. Writes verdict + `R-NNN` findings into the seeded `NN_REVIEW.md`. Verdict gates progress. C-12 (HIGH: PLAN's `## Spec` references prior iterations) and C-13 (CRITICAL: PLAN contradicts existing feature SPEC) are mandatory rejection rules.
- **`ark-verifier`** — standard+deep VERIFY gate (final check before commit). Reads VERIFY.md + PRD + PLAN + every project SPEC + related feature SPECs. Discovers project's verification commands (CLAUDE.md, manifest files, CI config). Runs build/test/lint/format. Writes PASS/FAIL/N/A per seeded item + `V-NNN` findings. C-11: does not self-fix; FAIL items return to main session.

C-22 asserts the prompt body is **byte-identical across platforms** after stripping per-platform frontmatter — the three agents are a single shared specification, three packagings.

## Why three? Specialization economics

Splitting work across specialists pays when the benefit of role-specific context exceeds the cost of dispatch + integration. The four levers:

### 1. Token economics — fresh context per role

Every subagent dispatch buys a fresh context window. The parent does not pay for the children's intermediate tokens in its own context budget. For Ark:

- The researcher reads docs, runs searches, opens 10–30 files. None of that lands in the parent's context.
- The reviewer reads every project SPEC + related feature SPECs + the full PLAN. That's easily 20–50 kB of markdown.
- The verifier reads every project SPEC + every related feature SPEC + PRD + PLAN + the diff + runs build/test/lint. The biggest context per dispatch.

If the parent did all three in-line, the conversation would compact aggressively and lose nuance. Children eat their own context, return summaries.

Source: Claude Code's Task tool design — "Each task gets its own context window, preventing pollution of your main conversation" ([MindStudio guide](https://www.mindstudio.ai/blog/sub-agents-claude-code-context-management)).

### 2. Attention isolation — prompt specialization

Each agent's system prompt is *narrowly* scoped:
- Researcher prompt is "go find things, write them down, do not write code".
- Reviewer prompt is "judge a plan, do not write or fix it".
- Verifier prompt is "audit shipped code against every rule, do not patch".

A single combined prompt would dilute each role's attention. The reviewer prompt's *mandatory rejection rules* (C-12, C-13) sit at the top of the reviewer's context every dispatch — the parent's prompt is too busy to keep them in working memory across phases.

### 3. Per-role model selection

Master-worker dispatch lets each role pick its model. Ark today uses platform-defaults but the design permits it:
- Researcher = fast/cheap (Haiku-class — lots of search + read, less reasoning).
- Reviewer = strongest reasoning (Opus-class — judgment is the product).
- Verifier = strong + tool-use (Opus or Sonnet — must run commands accurately).

Anthropic's published patterns ([How AI Is Transforming Work at Anthropic](https://www.anthropic.com/research/how-ai-is-transforming-work-at-anthropic)) explicitly recommend per-task model selection. Aider's architect/editor data confirms even minimal role-split pays off.

### 4. Hard-coded recursion guards

Three named roles + parent-only dispatch (C-15) make recursion impossible by construction. A single generalist-agent prompt would either need a longer recursion-guard section or risk children re-spawning specialists.

## Comparison with other shipped trios

### Devin's Planner / Coder / Critic / Browser

Devin's internal architecture per Cognition's published material ([Agent-Native Development deep-dive](https://medium.com/@takafumi.endo/agent-native-development-a-deep-dive-into-devin-2-0s-technical-design-3451587d23c0)): a compound AI system, not a single model, but a swarm of specialized models orchestrating a workflow.

- **Planner** — high-reasoning model; outlines strategy.
- **Coder** — specialized on high-quality code generation.
- **Critic** — reviews for security vulnerabilities + logic errors.
- **Browser** — scrapes and synthesizes documentation.

Maps to Ark:
- Planner ≈ main session in DESIGN/PLAN.
- Coder ≈ main session in EXECUTE.
- Critic ≈ `ark-reviewer` (PLAN-time) + `ark-verifier` (code-time).
- Browser ≈ `ark-researcher` (the only role that ships with `WebSearch`/`WebFetch`).

Devin splits the *writer* roles (Planner/Coder) into separate agents; Ark keeps them in the main session. Devin's pattern fits async-manager UX (submit task → review PR); Ark's pattern fits interactive-pair UX (developer in the loop).

### Recon / Exploit / Cleanup (security agents)

The pen-test trio:
- Recon = enumerate the target.
- Exploit = leverage findings to gain access.
- Cleanup = remove traces.

Maps cleanly onto Researcher (enumerate) / Coder-as-main (act) / Verifier (audit). The security domain validates the "scout, actor, auditor" decomposition.

### Red team / Blue team (RL self-play, cybersecurity)

Red = attacker; Blue = defender. Used in safety-critical and adversarial control scenarios ([Macropraxis on AI Self-Play](https://macropraxis.org/published-research/ai-self-play-enhancing-cybersecurity-using-redblue-team-ai-driven-simulations)).

The Ark analog: reviewer = "red team" attacking the plan; the planner (main session writing the PLAN) = "blue team" defending. The mandatory rejection rules (C-12, C-13) are the red-team's playbook. The iteration loop is exactly the coevolution dynamic the RL literature studies.

### planner / coder / tester (classic SE trio)

Long-standing software-engineering decomposition. Maps to Ark's PLAN / EXECUTE / VERIFY phases — *but as phases, not as agents*. Ark deliberately keeps planning + coding in the main session; only reviewer and verifier are agents.

Why? **Author-bias removal.** A reviewer that's a different prompt sees the plan with author-bias removed; a tester that's a different prompt grades it strictly. The *writer* role doesn't benefit from prompt-isolation in the same way — the main session has the context that produced the plan and the code, and that context is load-bearing for follow-up edits.

## Anti-pattern: too many specialists

Dispatch overhead scales sub-linearly with number of roles but integration overhead scales super-linearly. Anti-pattern fingerprints:

1. **Cross-role drift.** Each specialist has a fragment of the picture; the parent stitches them; the stitch drops nuance. Solution: fewer, broader specialists.
2. **Dispatch storm.** Generalist parent calls 8 specialists for 8 phases — token cost + latency multiply. Devin's compound-AI design includes a *router* explicitly to prevent this.
3. **Coordination prompts swelling.** If the parent's dispatch logic needs 500 lines of "when to call which", you've under-specified the roles. Ark's three are workflow-phase-bound (DESIGN/PLAN → researcher; REVIEW → reviewer; VERIFY → verifier) — the dispatch logic is one line per phase.

**Goldilocks evidence.** Cline ships *one* subagent type (read-only researcher). Claude Code ships *four* built-in (Explore, Plan, general-purpose, Bash) but encourages users to define more. Ark's three is on the small end of the shipping range; the workflow ties each to a phase, so there's no role-selection burden.

OpenAI's design rules in [A practical guide to building agents](https://openai.com/business/guides-and-resources/a-practical-guide-to-building-ai-agents/) prescribe a similar minimum: start with one agent, add specialists only when role-bleed or context exhaustion forces it.

## Where the trio leaves room

### No EXECUTE agent

NG-4 in the SPEC: "No agent for EXECUTE; main session retains full context there." The decision rests on two arguments:

1. EXECUTE needs the *parent's* context (the PLAN, the PRD, the SPECs, prior tool calls) intact. Delegating to an EXECUTE child means re-priming all of that, and the child needs the parent's *ability to ask follow-up questions of the user* — which subagent platforms generally don't provide.
2. Cline's read-only research subagent model points the same way: writers stay in the main loop; readers are subagents.

### No fixer / patcher agent

NG-5: "No reviewer/verifier 'self-fix' mode — gates are read-only audit roles." A fixer agent would need write access to the same files the parent edits, creating a coordination problem (who owns the file?). The chosen pattern: the gate flags, the parent fixes.

Risk: the parent might disagree with the reviewer and the loop stalls (mitigated by `max_iterations`). Or the reviewer might be wrong (mitigated by main session's "do not invent findings" discipline in the reviewer prompt).

### No planner agent

By design — the planner *is* the main session in DESIGN/PLAN. A separate planner agent would essentially be Devin's Planner + Critic combo, which is async-manager UX, not interactive-pair UX.

## Directions for Ark

1. **Document the trio's economics in `workflow.md`.** The current doc says "ask the user which reviewer to use" but doesn't explain *why* you'd want a reviewer subagent over self-review. The four levers above (token economics, attention isolation, model selection, recursion guard) are the explainer.
2. **Per-role model annotations in agent frontmatter.** Today the Claude frontmatter has an optional `model` field; Ark's three don't set it. Recommend a default mapping: researcher = haiku, reviewer = opus, verifier = opus. Surface this as a config knob, not a hard-coded constant.
3. **Reject "fixer agent" requests.** When a user asks "can the verifier auto-fix FAIL items?" — point to NG-5 + the coordination argument. Read-only specialists is the dominant pattern (Cline, Codex `multi_agent_v2=false`, Ark).
4. **Add a researcher-time prompt for "this is the third dispatch on the same task — re-read prior `research/*.md` first."** The researcher today is stateless across dispatches; users see redundant work. Cheap fix: enumerate prior files in the dispatch prompt or have the researcher list its own dir first.
5. **Consider a fourth role only if a measurable gap exists.** Candidate: `ark-doc-writer` for module-level docs (a recurring code-review finding). Run a 30-day experiment dispatching the main session for this; if rework rate is high, promote to a dedicated agent.

## Sources

- [`subagent-support` SPEC](file:///Users/anekoique/Agent/Ark/.ark/specs/features/subagent-support/SPEC.md) — Ark's three agents (C-1..C-28)
- [Ark `ark-researcher` template](file:///Users/anekoique/Agent/Ark/templates/claude/agents/ark-researcher.md) — write scope, output contract
- [Ark `ark-reviewer` template](file:///Users/anekoique/Agent/Ark/templates/claude/agents/ark-reviewer.md) — mandatory rejection rules
- [Ark `ark-verifier` template](file:///Users/anekoique/Agent/Ark/templates/claude/agents/ark-verifier.md) — audit rubric (10 dimensions)
- [Devin Agent-Native Development deep-dive (Endo, Medium)](https://medium.com/@takafumi.endo/agent-native-development-a-deep-dive-into-devin-2-0s-technical-design-3451587d23c0) — Planner/Coder/Critic/Browser swarm
- [Cline subagents docs](https://docs.cline.bot/features/subagents) — read-only research pattern
- [Anthropic — How AI Is Transforming Work at Anthropic](https://www.anthropic.com/research/how-ai-is-transforming-work-at-anthropic) — per-role model selection
- [Aider Architect/Editor results](https://aider.chat/2024/09/26/architect.html) — role split SOTA evidence
- [AI Self-Play in Cybersecurity (Macropraxis)](https://macropraxis.org/published-research/ai-self-play-enhancing-cybersecurity-using-redblue-team-ai-driven-simulations) — red/blue analogy
- [A practical guide to building agents (OpenAI)](https://openai.com/business/guides-and-resources/a-practical-guide-to-building-ai-agents/) — minimum-specialists principle
