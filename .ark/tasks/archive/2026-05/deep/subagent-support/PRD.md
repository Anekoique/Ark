# `subagent-support` PRD

---

[**What**]

Ship three Ark-native subagents — `ark-researcher`, `ark-reviewer`, `ark-verifier` — across all three supported platforms (Claude Code, Codex, OpenCode). Wire dispatch instructions into `/ark:design`'s DESIGN/PLAN/REVIEW/VERIFY phases so the main session can offload knowledge gathering and gate-judging to fresh contexts.

[**Why**]

Three high-leverage moments in the workflow are bottlenecked by main-session limitations:

1. **DESIGN/PLAN brainstorm** — main session has the user's intent loaded but lacks broad/deep external knowledge (library docs, prior art, patterns) and tends to shortcut on codebase exploration to save context budget. A dedicated researcher with web access and a persist-to-files contract removes this bottleneck.
2. **REVIEW (deep)** — judging a PLAN benefits from a context that did not steer the PLAN's authoring; reduces author bias. workflow.md already says *"preferably a fresh agent"* but provides no agent definition.
3. **VERIFY (standard + deep)** — auditing shipped code suffers the same author-bias problem. workflow.md again says *"preferably a fresh agent"* without an agent to dispatch.

Today both gates fall back to self-review with the same prompt the author was using. The agents codify the gate rubric (verdict semantics, severity scale, write-scope walls) into a fresh-context prompt that survives across sessions and platforms.

ROADMAP.md calls this out explicitly: *"Currently REVIEW will ask for self-review or spawn a sub-agent for review. We should add configurable options for invoking codex review, human review, or creating sub-agent reviews."* and *"Strengthen VERIFY"*. This task delivers the agent surface those items depend on.

[**Outcome**]

- All three platforms ship `ark-researcher`, `ark-reviewer`, `ark-verifier` agent definitions at their canonical paths (`.claude/agents/`, `.codex/agents/`, `.opencode/agents/`); templates embedded into `ark-core` via `include_dir!` and emitted on `ark init` / `ark upgrade`.
- `/ark:design` (all three platform variants) explains in DESIGN Step 1.2 / 1.4 *and* PLAN Step 2.3 *when and how* the main session should dispatch `ark-researcher`. Researcher findings persist to `.ark/tasks/<slug>/research/<topic>.md` (checked into git).
- `/ark:design` Step 3.2 (REVIEW) keeps the existing *"self-review or run the reviewer?"* prompt but, when the user picks the agent path, names `ark-reviewer` as the dispatch target and points at the platform-specific invocation idiom.
- `/ark:design` Step 5.2 (VERIFY) does the same for `ark-verifier`.
- Each agent enforces a tight scope wall via prompt: researcher writes only under `<task>/research/`; reviewer writes only the seeded `NN_REVIEW.md`; verifier writes only `VERIFY.md`. None spawn other agents (recursion guard).
- `codex-support` SPEC's NG-2 (*"No `.codex/agents/*.toml` custom subagents"*) is superseded — a `[**CHANGELOG**]` entry is appended noting the supersede.
- `opencode-support` SPEC adds an `extra_files` (or equivalent) entry covering `.opencode/agents/`; CHANGELOG entry appended.
- All existing tests still pass; new parity tests assert every platform ships the same three agents and each agent's frontmatter matches its platform's idiom.
- `ark init` on a clean project installs the new agent files into all selected platforms; `ark upgrade` on an existing project syncs them in.
- New `subagent-support` feature SPEC is promoted on commit (deep-tier behavior).

[**Related Specs**]

- `specs/features/codex-support/SPEC.md` — supersedes NG-2 (no `.codex/agents/`); adds `.codex/agents/` to the Codex template tree and registry. CHANGELOG entry appended.
- `specs/features/opencode-support/SPEC.md` — adds `.opencode/agents/` to the OpenCode template tree; existing `OPENCODE_TEMPLATES` static and platform shape extended. CHANGELOG entry appended.
- `specs/features/ark-context/SPEC.md` — agents call `ark context --scope phase --for <phase>` per existing schema; no schema change required.
- `specs/features/worktree/SPEC.md` — deep-tier work runs in worktree; agent dispatch and `<task>/research/` live inside the worktree's task dir. No conflict.
- `specs/features/task-concurrency-control/SPEC.md` — focus is per-checkout; agents inherit checkout focus. No conflict.
