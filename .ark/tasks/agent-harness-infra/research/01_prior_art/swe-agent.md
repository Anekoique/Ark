# SWE-agent & SWE-bench

## Identity

- **Project:** SWE-agent (the agent) + SWE-bench (the benchmark + evaluation harness)
- **Repos:**
  - https://github.com/SWE-agent/SWE-agent (formerly `princeton-nlp/SWE-agent`; transferred to the `SWE-agent` org during 2025)
  - https://github.com/SWE-bench/SWE-bench (formerly `princeton-nlp/SWE-bench`)
- **License:** MIT
- **Primary maintainers:** Princeton NLP group (John Yang, Carlos Jimenez, Alexander Wettig, et al.); now organized as the `SWE-agent` and `SWE-bench` GitHub orgs
- **Language:** Python
- **Stars / momentum:** SWE-agent at 19,257 stars (as of 2026-05-20). NeurIPS 2024 paper. Academic-led project with active maintenance.
- **Homepages:** https://swe-agent.com, https://www.swebench.com

## Positioning

SWE-agent is the **research-grade** point of comparison. Born from the question "what does the LLM need from its environment to be a good software engineer?" the project's headline contribution is the **Agent-Computer Interface (ACI)** concept — the observation that you can substantially improve agent performance *without changing the model* by carefully designing the commands and output formats it sees. SWE-bench is the matching evaluation harness: 2,294 real GitHub issues from popular Python repos, each containerized for reproducible scoring.

This is the academic ancestor of every modern coding agent's "tool design" page. Aider's diff-format obsession, Cline's plan/act split, OpenHands' microagents, and Claude Code's skills all descend conceptually from the ACI thesis. Ark's hidden `ark agent` namespace (typed Rust commands targeted at agent callers, not humans) is an ACI move.

## Primitives

User-facing nouns:

- **Task / Problem Instance** — a SWE-bench entry (repo + issue + golden patch + tests).
- **Agent** — a configured ACI + prompt template + LLM.
- **ACI (Agent-Computer Interface)** — the bash-like command set the agent sees.
- **Trajectory** — the recorded sequence of actions and observations.
- **Harness** — the Docker-based evaluation runner.

ACI commands (the LM-centric set):

- `find_file <name>` — locate file by name across repo
- `search_file <pattern> [<file>]` — search within file (or open file)
- `search_dir <pattern> [<dir>]` — search across directory tree (ripgrep-backed)
- `open <file> [<line>]` — open file at a line; shows a 100-line window
- `goto <line>`, `scroll_up`, `scroll_down` — navigate within the open file
- `create <file>` — create empty file
- `edit <start>:<end> << EOF ... EOF` — replace lines start..end with heredoc content; **auto-linted before commit, agent must fix errors**
- `submit` — emit final patch

Every command has bounded output (e.g., 100 lines) to prevent context overflow.

## Workflow model

Representative flow (SWE-bench task):

1. **Harness pulls Docker image** for the task instance (specific commit of e.g. `astropy/astropy`).
2. **Agent prompt** seeded with the issue text and an ACI usage guide.
3. **Loop**: agent picks an ACI command → harness runs it → returns bounded observation → agent picks next command.
4. **Edit cycle** — agent issues `edit`; ACI runs flake8 lint; if errors, observation includes them and the edit is rejected.
5. **Submit** — agent calls `submit`; harness extracts the diff.
6. **Grading** — harness runs the project's test suite (in the same container) on the patch; PASS_TO_PASS and FAIL_TO_PASS test sets determine resolution.

No PLAN/REVIEW artifacts. The trajectory file is the record. The agent's prompt encodes the "workflow" (often: "first explore, then localize, then edit, then verify, then submit").

## Context & memory

**Bounded outputs as the primary mechanism.** Every ACI command truncates. `open` shows 100 lines max; `search_dir` shows ~50 matches max with file:line:preview format. The agent has to issue follow-up commands to widen its view, which forces JIT loading.

**No long-term memory** — each task is independent. Trajectory persistence is for post-hoc analysis (paper figures, fine-tuning datasets), not for the agent to recall.

**No repo map** — the LM-centric design *deliberately* avoids dumping codebase structure. The bet (validated by benchmark improvements) is that targeted exploration via find/search beats global context.

## Tool / capability surface

**Built-in tools:** The ACI command set (above). That's it. No browser, no web search, no shell-without-bounds.

**MCP support:** Not in mainline SWE-agent — predates MCP and the project is research-focused. Forks exist that bridge to MCP.

**Plugin model:** Custom ACI commands by editing the agent config YAML; custom prompt templates the same way.

**Sandbox boundaries:** **Strong.** Every task runs in a per-instance Docker container. The harness builds three layers of images:

- **Base** — common dependencies
- **Environment** — Python version + interpreter setup per group
- **Instance** — repo at the specific commit with build deps installed

This three-layer caching is what makes SWE-bench reproducible at scale (2,294 tasks runnable in parallel without rebuilding everything).

## Integration model

**Standalone evaluation harness + a research-friendly Python framework.** No IDE plugin, no daemon. Run it from a terminal or in CI. The `swebench.harness.run_evaluation` entry point is the primary user surface for evaluators; the agent's `sweagent run` is the primary user surface for agent developers.

## Multi-agent / orchestration

**None in the original design.** Each task = one agent. SWE-bench is embarrassingly parallel (run N tasks in N containers) but each task is solo.

Recent forks add multi-agent patterns (planner-executor variants), but these are research extensions, not mainline.

## Spec / artifact system

**Trajectory files are the artifact.** A JSON object per task instance recording every (action, observation, reasoning_block) tuple plus final patch and grade. Useful for:

- Debugging "why did the agent fail this task"
- Fine-tuning datasets (action prediction targets)
- Paper figures (loss curves, action distributions)

No PRD/PLAN/SPEC equivalents — the *issue text* is the PRD, the *tests* are the spec.

## Strengths

- **The ACI thesis itself is the contribution.** "Design the interface, not the model" is now table-stakes thinking.
- **Reproducibility.** SWE-bench Verified is the gold-standard evaluation for autonomous coding agents (used by OpenAI, Anthropic, every OSS competitor).
- **Bounded outputs as a first principle.** Most other tools learn this lesson after running over context limits in production.
- **Edit linting before commit.** The agent gets a syntax-error observation back on bad edits, must fix them inline. This is a clean error-recovery pattern most agents lack.
- **Docker-per-task with image caching.** The harness architecture (base → env → instance layers) is widely copied.

## Weaknesses / gaps

- **Not a developer tool.** SWE-agent is meant to be run on a benchmark, not on your repo. The interactive UX is rough; documentation assumes you've read the paper.
- **No persistent memory.** Every task starts cold.
- **No multi-agent.** Solo agent, one shot.
- **No MCP, no IDE, no slash commands.** Pure research harness.
- **Python-only.** The ACI is hardcoded for Python repos (lint = flake8, env = Python venv).
- **Tied to GitHub-issue framing.** The agent expects an issue+repo input. Real dev work isn't always shaped that way.

## Directions for Ark

1. **Bounded-output ACI for `ark agent`.** SWE-agent's biggest research lesson is "every observation must be bounded; agents drown in unbounded output." Ark's `ark agent` commands return `Display` summaries — formalize a hard length cap (e.g., 200 lines, with `--full` to override), and have the commands return a stable structured shape (already partly true via `--format json`). Audit each `ark agent` command for unbounded paths.
2. **Lint-before-commit as a `task commit` invariant.** SWE-agent rejects edits that fail flake8 and surfaces the errors to the agent. Ark's `task commit` already gates on VERIFY completion; adding a *configurable* pre-commit lint stage (run project lint/test, fail commit if non-clean) would map straight onto this pattern. Per `.ark/config.toml`, opt-in.
3. **Trajectory files for `ark agent` runs.** SWE-agent's trajectory is a debugging gold mine. Ark could write `.ark/tasks/<slug>/trajectory.jsonl` recording every `ark agent` invocation (verb, args, before-state, after-state, error) for future replay/audit. This is the event-log idea from OpenHands at lower granularity.
4. **Evaluation harness for Ark itself.** The Ark CLI has integration tests but no end-to-end "did an agent successfully complete a task" benchmark. Designing a small ArkBench (5-10 canonical tasks: "add tier promotion", "extract feature SPEC", "open task with worktree") and running it nightly against Claude Code + Codex would catch regressions in slash-command prompts that unit tests miss.
5. **Counter-positioning: Ark is a workflow harness; SWE-agent is a benchmark harness.** They are complementary, not competitive. Cite SWE-bench results when evaluating Ark's integration partners ("on tasks involving deep refactor, Claude Code + Ark deep-tier passes X% of internal benchmarks") — but don't try to compete on SWE-bench Verified directly, that's a different game.

## Sources

- [SWE-agent/SWE-agent on GitHub](https://github.com/SWE-agent/SWE-agent) (queried 2026-05-20)
- [SWE-bench/SWE-bench on GitHub](https://github.com/SWE-bench/SWE-bench)
- [Agent-Computer Interface — SWE-agent docs](https://swe-agent.com/0.7/background/aci/)
- [SWE-agent paper (arXiv 2405.15793)](https://arxiv.org/pdf/2405.15793) — NeurIPS 2024
- [SWE-bench Evaluation Harness reference](https://www.swebench.com/SWE-bench/reference/harness/)
- [SWE-bench Docker Setup](https://www.swebench.com/SWE-bench/guides/docker_setup/)
- [Introducing SWE-bench Verified — OpenAI](https://openai.com/index/introducing-swe-bench-verified/)
