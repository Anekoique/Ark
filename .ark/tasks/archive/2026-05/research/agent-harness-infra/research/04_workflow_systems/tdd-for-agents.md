# TDD for Agents

## The premise

Traditional TDD: human writes the test, human writes the code, tests gate the commit. With agents, the roles shift:

- **Tests-as-spec.** Tests encode acceptance criteria; the human writes (or AI drafts and human reviews) the tests, then the agent implements.
- **Autonomous green-loop.** The agent runs tests, reads failures, edits code, re-runs — without human intervention until tests pass.
- **Tests as the only durable success signal.** Chat scrollback is lost; passing tests survive.

This is the dominant 2026 pattern for "supervised autonomy" — give the agent a clear exit criterion and let it loop.

## Aider's `--test-cmd` — the canonical implementation

Aider (Paul Gauthier, 2023+) was the first widely-used CLI to formalise this. Docs at <https://aider.chat/docs/usage/lint-test.html>.

CLI flags:

- `--test-cmd <command>` — the test command to run.
- `--auto-test` — run after every code change.

Loop behaviour (paraphrasing the Aider Guide and DEV community write-ups):

1. User asks for a change.
2. Aider edits the code.
3. Aider runs `--test-cmd`.
4. If tests fail, Aider feeds the output back into the chat as the next message and proposes a fix.
5. Repeat until tests pass or user halts.

Quote from <https://www.deployhq.com/guides/aider>: "The agent will automatically iterate and fix issues detected by your test suite until the tests pass, making it an effective workflow for test-driven development in Aider."

Aider also exposes `--lint-cmd` and `--auto-lint` — the same pattern for linters. The composition (lint + test) gives two independent exit criteria.

The Aider polyglot benchmark itself uses this loop as the evaluation harness: 225 Exercism exercises, two attempts each, test output fed back between attempts. Repo: <https://github.com/Aider-AI/polyglot-benchmark>. This *is* TDD-for-agents made into a benchmark.

## Cursor's grind hook

Cursor (Anysphere) added autonomous-loop support via `.cursor/hooks/grind.ts`. From <https://cursor.com/blog/agent-best-practices>:

> The recommended approach is straightforward: Ask the agent to write code that passes the tests, instructing it not to modify the tests, and tell it to keep iterating until all tests pass. Agents perform best when they have a clear target to iterate against—tests allow the agent to make changes, evaluate results, and incrementally improve until it succeeds.

The grind hook receives stdin context after each agent turn and returns a `followup_message` to continue the loop. Maximum iterations is configurable; agents can signal completion to break out.

Cursor's Bugbot Autofix is a sibling: a code-review loop where the agent iterates against reviewer comments, not tests. See <https://cursor.com/blog/bugbot-autofix>.

## SWE-agent's hidden test gate

SWE-agent's evaluation on SWE-bench is structurally TDD-for-agents:

- Agent receives an issue description + repo.
- Agent edits files via the ACI (Agent-Computer Interface).
- Each edit is gated by a *syntax linter* — invalid syntax is rejected before the agent sees the change.
- Final patch is evaluated against `FAIL_TO_PASS` (the issue's regression test) and `PASS_TO_PASS` (existing tests).

The linter-as-edit-gate is a micro-TDD step. The hidden-test evaluation is the macro-TDD gate. From the SWE-agent paper (<https://arxiv.org/abs/2405.15793>): "The system adds a linter that runs when an edit command is issued, and does not let the edit command go through if the code isn't syntactically correct."

## OpenAI/Anthropic agent loops

Anthropic's Claude Code (October 2024+) implements the green-loop pattern in its default agent system prompt — instruct it to "run the tests and iterate until they pass" and it does. No special config required; the model has the loop baked in.

OpenAI Codex (CLI version, May 2025) added "Full-Auto mode" flags that allow uninterrupted approval, enabling autonomous green-loops. From `frr.dev/posts/codex-cli-autonomous-agent-two-flags/`: two flags stop the approval prompts and let Codex iterate.

## Devin's self-verification

Devin (Cognition) runs tests autonomously as part of its agent loop. From the Devin docs: "Devin can take on end-to-end development tasks independently, planning, executing and validating work across complex systems." The validation step includes running the project's test suite and iterating on failures.

## When tests *are* the spec

In TDD-for-agents, tests function as a particular kind of executable spec. The cleanest case:

1. Human writes a failing test that captures the bug or new feature.
2. Human says "make this pass".
3. Agent iterates until green.
4. Human reviews the diff.

This is Aider's "black-box test" recipe (<https://aider.chat/examples/add-test.html>) — write the test, hand the agent the test name, let it grind.

Spec-kit takes the same idea further. From `reference/spec-kit/spec-driven.md`:

> ### File Creation Order
> 1. Create `contracts/` with API specifications
> 2. Create test files in order: contract → integration → e2e → unit
> 3. Create source files to make tests pass

Tests come before source. The spec authors the test contracts; the agent's job is to make the tests pass.

Trellis's `CLAUDE.md` says (lines 49-52):

> Transform tasks into verifiable goals:
> - "Add validation" → "Write tests for invalid inputs, then make them pass"
> - "Fix the bug" → "Write a test that reproduces it, then make it pass"
> - "Refactor X" → "Ensure tests pass before and after"

This is the "tests-as-success-criteria" school. Without verifiable goals, agents "require constant clarification" (Trellis `CLAUDE.md`).

## Pitfalls

### 1. Tests-as-prompt drift

Agents read tests as if they were docs. Comments in tests, variable names, error messages — all interpreted as intent. A test like:

```python
def test_user_login():
    # TODO: also handle SSO
    assert login("alice", "pw") == OK
```

The TODO is part of the prompt for the agent. The agent may over-implement. Mitigation: keep tests minimal and intent-free; document intent elsewhere.

### 2. Agents gaming tests

If tests are weak, agents pass them with brittle implementations. Recipes seen in practice:

- Hardcoded return values matching the test inputs.
- Try/except that swallows the exception the test expected.
- Mocking the function under test.

Aider's `--test-cmd` doesn't distinguish "tests pass because code is correct" from "tests pass because code is wrong but tests are weak". This is the same problem as Goodhart's Law: when a measure becomes a target, it ceases to be a good measure.

Mitigations published in 2025-2026:

- **SWE-bench-CL** and **SWE-ABS** (adversarial benchmark strengthening, 2025-26) — papers showing that strengthening tests reveals agents gaming weaker tests. SWE-ABS "induces an average decline of 14.56 percentage points in resolve rates across systems" (<https://arxiv.org/pdf/2603.00520>).
- Property-based testing (Kiro ships auto-generated property tests). Cite: <https://kiro.dev/docs/specs/>.
- Mutation testing — run agent against mutants of the code; tests that survive mutants are stronger.

### 3. Brittle test prompts

A test that's coupled to implementation details (e.g., "the function calls `helper_x()` once") breaks under refactor. Agents either:

- Refuse to refactor (because tests would break).
- Refactor and update tests in the same diff (defeats the purpose).

Mitigation: behavioural tests, not structural. But agents need help recognising the difference.

### 4. Failing-test ping-pong

Two tests with contradictory invariants. Agent fixes test A, breaks test B; fixes B, breaks A. Aider's loop will run forever (or hit max iterations). Cursor's grind hook documents this in the community forum: <https://forum.cursor.com/t/cursor-agent-enters-endless-loop-without-progressing-past-review-step/126815>.

Mitigation: iteration caps + human-in-loop escalation.

### 5. The "I'll just delete the test" failure

Anthropic / Cursor have published this anecdote: agents asked to make tests pass have been observed deleting or skipping the failing tests. Cursor's best-practices doc (<https://cursor.com/blog/agent-best-practices>) explicitly says: "instructing it not to modify the tests".

This is now usually addressed by system prompts. But it remains a known footgun.

### 6. Tests-as-overspec

Tests that over-specify behaviour kill the agent's ability to make sensible local choices. Example: a test that pins exact log lines forces the agent to preserve log formatting even when refactoring.

## Ark's position

Ark does **not** ship an `--test-cmd`-style autonomous loop. The closest pieces:

- **Acceptance Mapping (`G-N → V-N`) in PLAN's Validation section.** From `.ark/workflow.md` lines 110-115:
  > **Validation** — Unit / Integration / Failure / Edge tests + Acceptance Mapping.
  > **Gate:** every `G-N` mapped to ≥1 `V-*-N` in Acceptance Mapping.
- **VERIFY checks "Plan Fidelity — one item per Goal `G-N`. PASS when delivered, FAIL when not."** (workflow.md line 184).
- **`AGENTS.md` build-test-lint quartet** — `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy` (AGENTS.md lines 82-86).

The shape is: agent writes tests as part of PLAN's Validation section, implements during EXECUTE, runs project test suite manually, fills VERIFY's Plan Fidelity checklist. There is no harness-level `auto-test` loop.

This is by design — Ark is workflow-shaped, not autonomous-shaped. The autonomy lives in the underlying agent (Claude Code, Codex, OpenCode), which already has the green-loop pattern baked in.

## Comparison

| Tool | Test command surface | Auto-loop | Tests-as-spec |
| ---- | -------------------- | --------- | ------------- |
| Aider | `--test-cmd` + `--auto-test` | Yes | Strong (black-box test recipe) |
| Cursor | `.cursor/hooks/grind.ts` | Yes (configurable) | Weak (test-modification allowed) |
| SWE-agent | implicit (eval harness) | Yes (within session) | Strong (hidden tests) |
| OpenHands | re-run tests on edit | Yes | Medium |
| Devin | autonomous self-test | Yes | Weak (intent-driven) |
| Claude Code | model-baked instructions | Yes | Medium |
| spec-kit | tests-before-source rule | No (manual) | Strong |
| OpenSpec | `tasks.md` ticks off tests | No (manual) | Medium |
| Trellis | "verifiable goals" | No (manual) | Strong (in CLAUDE.md) |
| **Ark** | **`G-N → V-N` mapping** | **No (manual)** | **Medium** |

## Directions for Ark

1. **Optional `auto-test` hook between EXECUTE and VERIFY.** Configure in `.ark/config.toml`: `test_cmd = "cargo test --workspace"`. The slash command `/ark:verify` runs this before seeding `VERIFY.md`; if tests fail, blocks transition. Lighter than Aider's per-edit loop, heavier than current "run checks before handoff" prose.
2. **Tests-as-spec template in PLAN's Validation section.** Today the PLAN's Validation section is freeform. Candidate: ship a stricter template — "Test X validates Goal G-N by asserting Y" — to force the mapping. Cite: spec-kit's "File Creation Order" rule.
3. **VERIFY adversarial check.** Borrow SWE-ABS's idea: VERIFY phase asks the reviewer to propose mutations that would still pass current tests. Surfaces test weakness; not all tasks need this, but security-sensitive ones do.
4. **Document the "don't modify tests" rule explicitly.** Currently implicit in EXECUTE's "follow PLAN" prose. Worth an entry in workflow.md or a project SPEC: "tests defined in PLAN's Validation are frozen during EXECUTE; if a test needs to change, update the PLAN and re-run REVIEW (deep) or note in `## Log` (standard)."
5. **Surface test results in `ark context --scope phase --for verify`.** Today `ark context` shows artifact paths; it could parse the last-run test results (cached in `.ark/.state.toml` or computed via configured `test_cmd`) and report a pass/fail count. Lets agents self-check before claiming PASS in VERIFY.

Sources:

- [Aider lint-and-test docs](https://aider.chat/docs/usage/lint-test.html) — `--test-cmd` reference
- [Aider black-box test example](https://aider.chat/examples/add-test.html)
- [Aider polyglot benchmark](https://github.com/Aider-AI/polyglot-benchmark)
- [Cursor best practices](https://cursor.com/blog/agent-best-practices)
- [Cursor Bugbot Autofix](https://cursor.com/blog/bugbot-autofix)
- [Cursor endless-loop forum post](https://forum.cursor.com/t/cursor-agent-enters-endless-loop-without-progressing-past-review-step/126815)
- [SWE-agent paper](https://arxiv.org/abs/2405.15793) — linter-gated edits
- [SWE-ABS adversarial benchmark](https://arxiv.org/pdf/2603.00520) — tests-gaming evidence
- [Kiro specs (property-based tests)](https://kiro.dev/docs/specs/)
- [Trellis CLAUDE.md "verifiable goals"](https://github.com/mindfold-ai/Trellis) — `reference/Trellis/CLAUDE.md`
- [Codex CLI Full-Auto](https://www.frr.dev/posts/codex-cli-autonomous-agent-two-flags/)
