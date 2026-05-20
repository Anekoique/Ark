# Lint-Before-Commit and Pre-Edit Safety

The patterns harnesses use to catch bad LLM diffs *before* they land. SWE-agent's lint-before-edit, Aider's `--test-cmd`, Cursor's grind hook, Cline's checkpoints. Universally good; cheap to add; surprisingly absent from Ark's `task commit` today.

## The patterns in production

### SWE-agent — lint before edit

SWE-agent runs a linter *before* the model's proposed edit is applied. If the lint fails, the edit is rejected and the model gets the lint error as context for the next attempt.

Why it works:
- Catches obvious syntax errors before they pollute the repo.
- Closes the "agent writes broken code → reads broken code → writes worse code" loop.
- The linter's error message is structured feedback the model handles well.

Cost: per-edit linter run. Most linters are fast (<1s); negligible.

### Aider — `--test-cmd` for iteration

Aider's `--test-cmd <cmd>` flag runs the supplied test command after each edit. If tests fail, Aider feeds the failure back to the model and iterates.

Why it works:
- Tests are the spec.
- Iteration converges on green tests.
- Failure mode (tests as prompts; agent gaming) documented in `04_workflow_systems/tdd-for-agents.md`.

Cost: each iteration runs the test suite. For slow suites, this is significant.

### Cursor — "grind" hook

Cursor's autopilot mode iterates until "the task is done" with implicit success signals (tests pass, build green, no errors). The hook is configurable; users supply project-specific success checks.

Why it works:
- Removes the need for the user to manually re-prompt.
- Agent self-validates.

Failure mode: agent loops forever on flaky tests, or gives up after N tries with no escalation.

### Cline — checkpoints + revert

Cline checkpoints file state per tool use. The user (or the agent) can revert to any prior checkpoint if a change goes wrong.

Why it works:
- Reversibility per edit, not just per session.
- Catches "agent edited the wrong file" cases.

Cost: storage (filesystem snapshots); usually managed automatically.

### Claude Code — `/rewind`

Equivalent to Cline's checkpoint-revert pattern. Anthropic's docs reference snapshot retention windows.

### Continue — CI integration

Continue's 2026 pivot positioned CI as a quality gate — pre-merge `cn` commands run in CI to validate agent-generated changes before merge.

Why it works:
- Trusted CI environment validates before merge.
- Failures block merge; agent (or user) fixes.

## The unifying principle

**The agent shouldn't be the final judge of its own correctness.** External validation (linter, test, build, type-check) catches errors the agent itself can't see (or chose to ignore).

Three layers:
1. **Per-edit:** lint, type-check (SWE-agent style).
2. **Per-task:** test suite (Aider's `--test-cmd`).
3. **Per-commit / per-merge:** full CI (Continue, GitHub).

Each layer catches different errors. Together they form defence-in-depth against bad agent output.

## What Ark has today

Ark's `task commit` does:
1. VERIFY gate — checks VERIFY.md has no PENDING items.
2. SPEC extraction (deep tier).
3. Stage Ark-managed files.
4. Run `git commit`.

It does NOT:
- Run a project linter.
- Run project tests.
- Type-check.
- Build.

The user is expected to run these themselves before `git add`. Reasonable for trust-the-user; missed opportunity for trust-the-agent.

## What Ark could do

A `task commit --lint` / `--test` mode that invokes project hooks before commit:

```toml
# .ark/config.toml
[commit_hooks]
lint = "cargo clippy --workspace -- -D warnings"
test = "cargo test --workspace"
typecheck = "cargo check --workspace"
build = "cargo build --workspace"
```

`task commit` runs these (in declared order) before staging. Any failure aborts the commit; agent gets the error output as feedback for the next iteration.

**Opt-in default:** mode-flag (`--lint`, `--test`) or config-driven (`[commit_hooks]` table with `auto: true`).

**Failure UX:** clean error surface, like `VerifyIncomplete`. Tell the user what failed; offer the command to re-run.

## Where this fits in the workflow

Today's `task verify` already audits the work (correctness, code quality, SPEC drift). The lint/test gate is *adjacent* — it catches *mechanical* failures (does it compile, do tests pass) that VERIFY's audit might miss because the auditor reads markdown, not runs code.

The right split:
- **VERIFY:** logical audit (reviewer judgement).
- **`task commit --lint`:** mechanical gate (compiler, tests).

Both run; both must pass.

## The SWE-agent insight applied to Ark

SWE-agent ran lint *before* every edit. Ark runs at commit time (the latest point). The difference:

- **Before-edit:** catches errors early; rejects edits before they land. Slower per-iteration; faster overall (fewer bad iterations).
- **At-commit:** catches errors late; lets bad edits accumulate. Faster per-iteration; possibly slower overall if rework is needed.

For Ark's tier model:
- Quick tier — at-commit is fine (one edit, one commit).
- Standard / Deep — at-commit is fine, but a *per-phase* gate (e.g. run tests at the end of EXECUTE before VERIFY) would catch issues earlier.

The phased approach:
- `task execute` ends with optional `--lint`.
- `task verify` includes lint-pass / test-pass as automated VERIFY items.
- `task commit` re-checks (belt+suspenders).

This puts gates at all three layers without much added complexity.

## The "agent reads its own lint output" loop

When the lint fails and the agent sees the error:
- The error message becomes prompt input.
- The agent re-edits.
- Loops until green or until iteration cap.

This is Aider's `--test-cmd` model. Works well when the lint output is structured (Rust's compiler errors are excellent in this regard; clippy is excellent). Less well when errors are vague.

For Ark's environment (Rust project as the *Ark* repo; agent-edited code in *user* repos):
- The user's repo might be any language; Ark needs to be language-agnostic.
- The `[commit_hooks]` config table is the right abstraction: project declares its lint/test commands; Ark runs them.

## Pre-edit safety vs. trust

Lint-before-commit is *defence-in-depth*, not *replacement-for-trust*. Cline/Aider users still trust the agent; the lint just catches the cases where the agent's trust was misplaced.

For Model-A harnesses (Ark's posture), pre-edit safety is the cheap top-up that doesn't change the trust model. The user still reviews diffs; the lint just removes a class of obvious errors before review.

For Model-B harnesses (Codex sandboxed, Devin VMs), pre-edit safety is one of many layers; the OS sandbox is the primary defence.

## What about CI?

CI is the *last* layer. It runs after commit, often after push. It's slow (minutes), expensive (compute), and reactive.

For Ark, running CI-equivalent checks at commit time captures *most* of CI's value without the latency/cost. The user still pushes to CI for the full suite; the commit-time check catches the ~80% common case.

## Failure modes

1. **Slow lint / test.** If `cargo test --workspace` takes 5 minutes, `task commit --test` is a slow operation. Mitigation: optional; user-selectable hooks; fast-test subset for commit-time.

2. **Flaky tests.** Lint pass; test pass occasionally fails. Mitigation: retry policy; or user disables `[commit_hooks].test` for known-flaky cases.

3. **Linter false positives.** Clippy occasionally complains about things the user accepted. Mitigation: user's existing `#[allow]` annotations; Ark doesn't re-judge.

4. **Path mismatch.** `task commit` from a worktree; lint configured in parent. Mitigation: hook commands resolve relative to worktree root.

5. **Hook command failures unrelated to agent output.** Network down; CI service unavailable. Mitigation: explicit error surface; user retries or skips.

## Directions for Ark

1. **Add `[commit_hooks]` table to `.ark/config.toml`.** Project-declared lint/test/typecheck/build commands. Run by `task commit` when present and `--lint` / `--test` flags are set.

2. **Surface lint/test status in VERIFY items.** When VERIFY is seeded, add "Lint passes" / "Tests pass" rows. Default PASS markers but populated by hook output.

3. **`task execute --lint` for mid-phase gate.** Optional. Runs lint at the end of EXECUTE before transitioning to VERIFY. Catches issues earlier.

4. **`task commit --strict` for belt+suspenders.** All configured hooks must pass; any failure aborts commit. The default mode is warning-only; `--strict` is opt-in.

5. **Document the layered-defence pattern in `workflow.md`.** Pre-edit (lint) + phase-end (test) + commit (final check) + CI (external). Make the layers visible; let users opt into each.
