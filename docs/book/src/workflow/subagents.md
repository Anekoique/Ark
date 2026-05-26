# Subagents

Ark ships three subagents that the main session can dispatch at specific phases. They are read-mostly audit and research roles with tight write scopes — they never touch code, never run git, and never spawn each other.

| Subagent        | Phase                | Writes only to                                | Role                                                       |
| --------------- | -------------------- | --------------------------------------------- | ---------------------------------------------------------- |
| `ark-researcher` | DESIGN / PLAN, research tier | `.ark/tasks/<slug>/research/*.md`     | Gather third-party / prior-art / codebase-pattern findings |
| `ark-reviewer`   | REVIEW (deep)        | the current iteration's `NN_REVIEW.md`        | Judge the latest PLAN; write verdict + findings            |
| `ark-verifier`   | VERIFY               | `VERIFY.md`                                   | Audit shipped work against PLAN/PRD; run build/test/lint   |

All three ship for every installed platform — Claude Code (`.claude/agents/*.md`), Codex (`.codex/agents/*.toml`), and OpenCode (`.opencode/agents/*.md`). The prompt body is identical across platforms; only the per-platform frontmatter differs.

## Reserved stems

The filenames `ark-researcher`, `ark-reviewer`, and `ark-verifier` (with each platform's extension) are **reserved**. Ark re-applies them on `init`, `upgrade`, and `load` — a user-authored file at one of those stems is overwritten. Sibling agents at any other stem are preserved untouched.

## Who picks them

The slash commands don't auto-dispatch. At REVIEW and VERIFY entry, `/ark:design` **stops and asks you** which to use — the Ark subagent, a different model, or do it yourself. The choice is yours per phase per task.

After dispatching any subagent, the main session checks `git status` and reverts (via `git restore`) anything written outside the agent's documented write-scope before incorporating its findings.

## Research tier vs. embedded researcher

Both gather knowledge, but they answer different questions:

- **Research tier (`/ark:research`)** — you *cannot yet write a PRD*. The open question is what to build, or whether to build at all. The corpus under `research/` is the deliverable; follow-up implementation is optional. See [Tiers → Research](./tiers.md).
- **Embedded `ark-researcher`** — you can already write the PRD's What / Why / Outcome; you just need to fill technical unknowns *inside* a tiered task (a third-party library's shape, a prior-art comparison, a codebase pattern map). It stays inside the parent task and persists findings to that task's `.ark/tasks/<slug>/research/<topic>.md`.

Rule of thumb: if the research changes *whether* there's a task, use the research tier. If it changes *what goes in the PRD/PLAN* of a task you already know you're doing, dispatch the embedded researcher.

## What they don't do

- No subagent self-fixes. `ark-verifier` reports FAIL items; the main session resolves them.
- No subagent runs git or edits code, SPECs, or the PLAN it's reviewing.
- No subagent spawns another subagent — every prompt carries a recursion guard. Only the main session dispatches.
