# Evaluation and Quality Gates

## What "done" means

For an agent harness, "done" is a moving target. Three definitions in circulation:

1. **Tests pass.** Lowest bar. The autonomy claim ("agent fixes its own bugs") is grounded on this.
2. **Diff is acceptable.** Reviewer accepts. Adds human judgment.
3. **Acceptance criteria met.** Each PRD/spec acceptance criterion verified against the implementation. Adds intent traceability.

Industrial harnesses converge on #3 with elements of #1 and #2 baked in. Ark's VERIFY phase is structured around #3 — every Goal `G-N` mapped to a Verify item `V-*-N`, plus cross-cutting `V-NNN` findings.

## Public benchmarks — the agent leaderboard era

### SWE-bench / SWE-bench Verified

- Repo: <https://github.com/SWE-bench/SWE-bench>
- Origin paper: Jimenez et al., "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?" (2023).
- Composition: 2294 real GitHub issues with associated patches, drawn from 12 popular Python repos.
- Evaluation: agent receives issue text + repo state; produces a patch; patch evaluated by running:
  - `FAIL_TO_PASS` tests — encoded the bug; should now pass.
  - `PASS_TO_PASS` tests — unrelated; should not break.

**SWE-bench Verified** (OpenAI, August 2024) is the human-validated 500-task subset:

> SWE-bench Verified is a human-validated section of the SWE-bench dataset released by OpenAI in August 2024, consisting of 500 high-quality test cases from the original benchmark.

Released after early benchmark runs revealed test/infrastructure issues that made tasks unsolvable. From <https://openai.com/index/introducing-swe-bench-verified/>: cases were validated by software engineers with support from OpenAI's Preparedness Team.

Why this matters for workflow systems: SWE-bench *is* a workflow gate. It defines "done" as "FAIL_TO_PASS passes + PASS_TO_PASS still passes." This is **tests-as-spec** scaled to a benchmark. Every published agent harness (Aider, SWE-agent, OpenHands, Devin) reports its SWE-bench Verified score.

Leaderboard (Epoch AI, 2025-2026): the top of the leaderboard is around 60-70% resolution rate; the original SWE-agent paper hit 12.29% on the full set. Order-of-magnitude improvement in two years.

### Aider Polyglot

- Repo: <https://github.com/Aider-AI/polyglot-benchmark>
- Origin: Aider project (Paul Gauthier).
- Composition: 225 Exercism exercises across C++, Go, Java, JavaScript, Python, Rust.
- Evaluation: each model gets two attempts; failed first attempt receives test-error feedback before second.

Leaderboard at <https://aider.chat/docs/leaderboards/> and <https://llm-stats.com/benchmarks/aider-polyglot>. GPT-5 leads at 88.0% (as of late 2025/early 2026); Gemini 2.5 Pro Preview at 82.2%; o3 at 81.3%.

Notable: the benchmark *is* a TDD-for-agents loop. Each attempt = code + tests + observe failures + fix. The benchmark validates the workflow shape.

There's also a separate **Aider code-editing leaderboard** measuring "can the model edit code correctly?" — narrower than polyglot, focused on diff-application accuracy.

### Other benchmarks shaping the gate definitions

- **SWE-MERA** — "dynamic benchmark for agentically evaluating LLMs on software engineering tasks" (<https://arxiv.org/abs/2507.11059>). Generates new tasks over time to combat memorisation.
- **SWE-bench-CL** — continual-learning variant; agents must learn across tasks (<https://arxiv.org/abs/2507.00014>).
- **SWE-ABS** — adversarial benchmark strengthening; reveals tests-gaming (<https://arxiv.org/abs/2603.00520>). "Induces an average decline of 14.56 percentage points in resolve rates across systems."
- **Refact.ai polyglot top score** — 76.4% (Claude 3.7 Sonnet, Refact.ai agent + claude); later updated to 92.9%. Shows top-of-leaderboard is a moving target.

## How shipped harnesses do quality gates

### Aider — auto-test + auto-lint

`--test-cmd` and `--lint-cmd` are the canonical quality gates. The agent loop runs them after every change. If either fails, the failure output is fed back into the chat. This is **run-test-until-green** as the operating principle.

The gate is binary: tests pass, you ship; tests fail, the agent iterates. No nuance. Effective when tests are good; brittle when tests are weak (see `tdd-for-agents.md`).

### Cursor — grind hook + Bugbot

Cursor's gate is two-layered:

1. **Grind hook** (`.cursor/hooks/grind.ts`) — autonomous test loop, similar to Aider.
2. **Bugbot Autofix** — reviewer findings drive a second loop. From <https://cursor.com/blog/bugbot-autofix>: closes the review loop after PR open.

The two-layer model maps to Ark's EXECUTE → VERIFY split, but Cursor runs them in series within one autonomous run.

### SWE-agent — linter + hidden tests

SWE-agent's gates are evaluation-time (hidden tests) and edit-time (syntax linter that rejects bad edits). The agent never sees the hidden tests, only the issue description. From the paper:

> The system adds a linter that runs when an edit command is issued, and does not let the edit command go through if the code isn't syntactically correct.

This is a **micro-gate inside the execute loop**. Similar in spirit to property-based tests (Kiro) or `cargo clippy -D warnings` (Ark's `AGENTS.md` line 85).

### OpenHands — re-run tests + security review subagent

Quote (search snippet): "Verification involves re-running tests after editing, ensuring that changes are validated rather than assumed to be correct."

OpenHands's auxiliary services include a "security review" subagent — a parallel critic that flags security issues. This is structurally similar to Anthropic Code Review's specialised agent fleet, but inside one harness.

### Devin — autonomous verify

Devin runs project tests as part of its agent loop. No published threshold or gate config — Devin self-decides when work is "done" (subject to user approval before PR open).

### spec-kit — constitutional gates

From `reference/spec-kit/spec-driven.md`:

> ### Phase -1: Pre-Implementation Gates
>
> #### Simplicity Gate (Article VII)
> - [ ] Using ≤3 projects?
> - [ ] No future-proofing?
>
> #### Anti-Abstraction Gate (Article VIII)
> - [ ] Using framework directly?
> - [ ] Single model representation?

These are *design-time* gates, not execution-time. Different gate class than tests. They check **architectural alignment**, not behavioural correctness.

### OpenSpec — `/opsx:verify`

In the expanded profile. Validates that the proposal's spec deltas correctly reflect the implementation. From the OpenSpec workflow docs: "verify changes match their applied implementation."

### Kiro — diff/review + hooks

> A diff/review gate is the requirement that proposed code changes be presented as diffs for human approval before application.

Plus the hooks system: "Set up quality gates that run automatically, catching issues before they become problems. Hooks are planned tasks for tests, docs, performance, accessibility, etc."

### Anthropic Code Review — confidence-score gate

From the CLAUDE Code docs: "The default threshold is 80. To adjust, modify the command file at commands/code-review.md."

This is the only published harness with a **tunable confidence gate**. Findings below 80/100 are filtered before being shown.

## Ark's VERIFY phase in detail

From `.ark/workflow.md` lines 171-194:

```
### VERIFY — audit shipped code

VERIFY.md is seeded with auto-populated checklist sections. Resolve each item:

- Project Spec Compliance      — one item per registered SPEC. PASS / FAIL / N/A with explanation.
- Related Feature Spec Compliance — one item per SPEC the PRD listed.
- PRD Constraints              — one item per Outcome criterion.
- Plan Fidelity                — one item per Goal G-N. PASS when delivered, FAIL when not, N/A when withdrawn.
- SPEC Drift                   — PASS once any modified feature SPEC has a [**CHANGELOG**] entry.

Add Findings (V-NNN) for cross-cutting issues that don't map to a single seeded item:
Severity, Location, Problem, Why it matters, Recommendation,
Resolution (PENDING / FIXED in <ref> / ACCEPTED — <reason>).

Gate: no item is PENDING. No verdict line. Quality bar covers plan fidelity,
correctness, code quality, abstraction, SPEC drift — not just "does it work".
```

Three structural choices worth highlighting:

### 1. Seeded checklist + free-form findings

The seeded sections (project SPECs, feature SPECs, PRD constraints, Goals) are auto-populated. Each is concrete and small. The `V-NNN` findings catch what slips between.

This is more structured than Aider's binary "test-cmd passes". Closer to spec-kit's constitutional gates, but executed at VERIFY (post-implementation) rather than at PLAN (design-time).

### 2. No verdict line, only no-PENDING gate

Workflow.md says "No verdict line." The gate is granular: every checklist item resolves. Reviewers (or self-verifiers) cannot say "good enough, ship it" — every item is itemised PASS/FAIL/N/A with reasoning.

Contrast: Anthropic Code Review filters findings by score; Cursor's Bugbot dispenses verdicts. Ark forces resolution of every item.

### 3. Tiered: VERIFY is standard + deep only

Quick tier skips VERIFY. Workflow.md: "VERIFY.md — standard + deep." Quick-tier tasks are "reversible in one commit, no new abstractions" — the gate cost would outweigh the benefit.

This is Ark's tier-aware contribution to the quality-gate landscape. Other tools don't tier their gates.

## The acceptance-criteria → test-mapping pattern

Ark's distinctive enforcement: every Goal must have at least one Verify item.

From workflow.md line 116: **"Gate: every `G-N` mapped to ≥1 `V-*-N` in Acceptance Mapping."**

This is enforced before PLAN → EXECUTE transition. Concrete shape:

```
## Validation
- V-U-1: Test foo() returns Err when input is empty.       (Unit; maps G-1)
- V-I-1: ark unload then ark load preserves all files.     (Integration; maps G-2, G-3)
- V-F-1: Removed file appears unmodified in restore.       (Failure; maps G-2)
- V-E-1: Empty .ark/ directory ignored, not removed.       (Edge; maps G-1)

## Acceptance Mapping
G-1 → V-U-1, V-E-1
G-2 → V-I-1, V-F-1
G-3 → V-I-1
```

VERIFY's Plan Fidelity section (one item per Goal G-N) then closes the loop — at verify time, each Goal's mapped Verify items either passed (PASS) or didn't (FAIL).

This is a verbatim implementation of the **"executable spec"** idea from spec-driven dev. From `spec-driven.md`: "Acceptance scenarios become tests. This merges development and testing through specification—test scenarios aren't written after code, they're part of the specification that generates both implementation and tests."

Ark doesn't generate tests from Goals automatically. It enforces the mapping (Goals → Verify items → tests in EXECUTE → checked again in VERIFY).

## Trade-offs

### Coverage vs noise

A finer-grained gate catches more, but generates more checklist items. Ark's seeded items are bounded (one per SPEC, one per Goal) — coverage scales with project size, not arbitrarily.

Anthropic Code Review's confidence gate trades noise (filtered) for missed bugs (low-confidence findings dropped).

### Pre- vs post-implementation gate

Spec-kit gates pre-implementation (constitutional). Anthropic gates post-implementation (code review). Ark does both: PLAN's Acceptance Mapping is pre; VERIFY is post.

Both gates have value. Pre-gates catch intent mistakes cheaply. Post-gates catch implementation mistakes after they exist. Ark's two-stage design is the more thorough but also more expensive choice.

### Self-verify vs other-verify

Workflow.md (line 177): "STOP. Ask the user which verifier to use: `ark-verifier` subagent, a different model, or self-verify. Do not pick on the user's behalf."

Same problem as REVIEW. Self-verify is cheap but may miss bugs the same model made. Other-verify (subagent or different model) is stronger but costlier. Ark leaves it to the user.

### Acceptance Mapping as ceremony

Forcing `G-N → V-N` mapping costs effort. For very simple Goals, it can feel pedantic ("G-1: command runs without error → V-U-1: test that command runs without error"). Workflow.md mitigates by gating before PLAN → EXECUTE — the cost is at PLAN time, the benefit is at VERIFY time when the mapping is consulted.

## Failure modes

- **Checklist tick-without-inspection.** Items resolved PASS without depth. Ark relies on reviewer discipline; subagents (ark-verifier) help by being deterministic about scope.
- **N/A creep.** Every item marked N/A defeats the gate. Workflow.md doesn't quantify; project culture has to enforce "N/A requires explanation."
- **`V-NNN` overflow.** Free-form findings can blow up. Severity tagging helps (filter by Severity for triage); workflow.md doesn't enforce a count cap.
- **Test-pass ≠ correctness.** Same problem as tests-as-spec. Acceptance Mapping helps: even if tests pass, the Verifier must explicitly state how each Goal was delivered.
- **SPEC Drift gap.** SPEC Drift is checked only when a feature SPEC is modified. If a task subtly changes implementation in ways the SPEC describes but the developer forgot to update the SPEC, drift is invisible. Anthropic Code Review's multi-agent comparison helps; Ark's local reviewer might miss it.

## Comparison

| Tool | Gate type | Tunability | Coverage |
| ---- | --------- | ---------- | -------- |
| Aider | Binary (test-cmd pass/fail) | Off / On | What tests cover |
| Cursor (grind + Bugbot) | Two-stage (tests + review) | Iteration cap | Tests + reviewer judgment |
| SWE-agent | Edit-time (linter) + final (hidden tests) | Hidden | Linter + benchmark tests |
| OpenHands | Re-run tests + security subagent | Subagent enable/disable | Tests + security |
| Devin | Autonomous (self-decided) | None | Self-judgment |
| spec-kit | Pre-impl constitutional + tests | Constitution articles | Constitution + tests |
| OpenSpec | `/opsx:verify` (expanded profile) | Profile selection | Spec deltas + impl |
| Kiro | Diff/review + hooks | Per-hook | Hooks per phase |
| Anthropic CR | Post-impl confidence-scored | Threshold (default 80) | Multi-agent specialisations |
| **Ark** | **Pre (Acceptance Mapping) + Post (VERIFY checklist + findings)** | **Tier-gated (quick skips)** | **SPEC compliance + Goals + Outcomes + drift** |

## Directions for Ark

1. **Confidence field on `V-NNN` findings.** Borrow from Anthropic Code Review. Add optional Confidence (0-100) to findings; allow filtering at threshold. Reduces noise from low-confidence cross-cutting findings; preserves the no-PENDING gate for high-confidence items. Trade-off: more fields; mitigation: optional, defaults to 100.
2. **`ark agent task verify` runs test commands.** Today VERIFY is artifact-only; tests are run by the agent during EXECUTE. The CLI could optionally invoke a configured `test_cmd` and surface results in `ark context --scope phase --for verify`. Cross-ref: tdd-for-agents.md direction 1.
3. **Auto-generate `V-NNN` seeds from external linters.** `cargo clippy` warnings, dependency-audit results, SPEC-Drift dry-run results — all could seed Verify items. Reduces verifier workload; surfaces issues the agent might miss.
4. **Benchmark Ark itself.** SWE-bench-style — produce a small set of tasks (e.g., 10 fixtures using `ark init` projects) with known-good outcomes. Run Ark's workflow end-to-end against them in CI. Validates the workflow doesn't regress. Currently nothing of this kind exists for harnesses-themselves (as opposed to underlying models).
5. **Publish a "Verify quality bar" SPEC.** Workflow.md line 189: "Quality bar covers plan fidelity, correctness, code quality, abstraction, SPEC drift — not just 'does it work'." This is the most important sentence in the entire workflow doc and it's buried. A project SPEC `specs/project/quality-bar/SPEC.md` could expand it with examples of what each dimension looks like in practice.

Sources:

- [SWE-bench Verified (OpenAI, August 2024)](https://openai.com/index/introducing-swe-bench-verified/)
- [SWE-bench Verified leaderboard (Epoch AI)](https://epoch.ai/benchmarks/swe-bench-verified)
- [SWE-bench Verified (Vals AI)](https://www.vals.ai/benchmarks/swebench)
- [Aider polyglot benchmark](https://github.com/Aider-AI/polyglot-benchmark)
- [Aider leaderboard](https://aider.chat/docs/leaderboards/)
- [Aider polyglot leaderboard (llm-stats)](https://llm-stats.com/benchmarks/aider-polyglot)
- [SWE-agent paper](https://arxiv.org/abs/2405.15793) — linter gates
- [SWE-ABS adversarial benchmark](https://arxiv.org/abs/2603.00520) — tests-gaming evidence
- [SWE-MERA dynamic benchmark](https://arxiv.org/abs/2507.11059)
- [Anthropic Code Review confidence threshold](https://code.claude.com/docs/en/code-review)
- [Refact.ai polyglot results](https://refact.ai/blog/2025/refact-ai-agent-claude-3-7-sonnet-ranked-1-aider-polyglot/)
- [Kiro Specs (property-based testing + hooks)](https://kiro.dev/docs/specs/)
