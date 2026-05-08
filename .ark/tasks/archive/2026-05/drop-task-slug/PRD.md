# `drop-task-slug` PRD

---

[**What**]
Drop the `--slug` flag from `ark agent task {plan, review, execute, verify, commit, promote, archive}` and replace per-call session-id-driven slug lookup with topology-driven resolution: worktree path first, then this session's focus map. `resume` and `discard` keep `--slug` (they have no implicit target). Split the misleading `Error::NoCurrentTask` into two precise variants.

[**Why**]
`resolve_slug` in `crates/ark-cli/src/agent_cli.rs` calls `resolve_session_id` on every read-side invocation. When a parent Claude Code process exits and a fresh one runs `ark` in the same `.ark/`, the new ppid produces a fresh session UUID that does not appear in `.state.toml`'s `[sessions.*]` map. Result: `Error::NoCurrentTask` fires with a misleading "X is missing" message even though `.state.toml` exists, contains the focused slug, and is correctly read. The only escape today is for the agent to retry every call with `--slug <s>` — which is exactly the redundancy the agent shipped templates do, producing the double-call pattern (first call fails, second succeeds).

The session-id machinery exists for `state_mutate`'s write-side concurrency control (multiple shells in one checkout). It is incidental complexity for *reads*: a slug-resolution call doesn't need to know who's asking, only what the ambient topology says.

Worktrees give us a stronger signal anyway. When `task new --worktree` is used, the worktree directory's own `.ark/` carries a `.state.toml` whose `tasks.active` list is the authoritative single source. The path itself encodes the slug (`<wt>/.ark/tasks/<slug>/`).

[**Outcome**]

- `ark agent task plan|review|execute|verify|commit|promote|archive` no longer accept `--slug`. Removed from the clap definitions and from every shipped slash-command template.
- Slug resolution succeeds without `--slug` in three sane cases:
  1. Worktree topology: invoked inside `.ark/worktrees/<branch-type>/<slug>/` (or any depth under it, since `Layout::discover_from` already resolves the worktree as root).
  2. Single active task: `state.tasks.active.len() == 1`.
  3. Multiple active tasks with valid session focus: `state.sessions.<this-session>.focus` resolves.
- Failure is loud and accurate. The error message tells the user what's wrong and what to do:
  - **No active task**: "no active task in this checkout; run `ark agent task new` first" (replaces today's "X is missing" lie when state file is present-but-empty).
  - **Ambiguous active task**: "multiple active tasks: redesign-xtest, vdso-support; `cd` into the task's worktree or run `ark agent task resume --slug <s>` to focus this session" (raised when 2+ active tasks AND no worktree topology AND no live session focus).
- `resume <slug>` and `discard <slug>` are unchanged — slug remains required (no implicit target makes sense for them).
- Reproduce the original bug as a regression test: a `.state.toml` with active slugs but no entry matching the current session id, invoked from inside a worktree, returns the correct slug (not `NoCurrentTask`).
- All existing tests pass. New unit tests cover each fallback branch and the two new error variants.
- The double-call pattern in slash-command templates disappears: every `ark agent task <verb>` line is single-call.

[**Related Specs**]

- `.ark/specs/features/ark-agent-namespace/SPEC.md` — defines the hidden `ark agent` CLI surface (G-1, G-2, G-3). The spec table currently shows `[--slug <s>]` as optional on plan/review/execute/verify/archive/promote with the line "Every `--slug`-taking command defaults to `.ark/tasks/.current` when the flag is omitted. Missing `.current` → `Error::NoCurrentTask`." That paragraph + the table need updating: drop the column for these verbs, replace the fallback sentence with the topology-driven order, and rename the error.
- `.ark/specs/features/task-concurrency-control/SPEC.md` — G-9 currently reads "`--slug`-less commands resolve to *this session's* focused slug. With no focus, return `Error::NoCurrentTask`." This task revises G-9 to the layered fallback chain (worktree → single-active → focus → split error). C-21 and the call graphs around `task_new`/`task_archive` are unaffected (they're write-side and keep using `state_mutate` + `resolve_session_id`).
- `.ark/specs/features/worktree/SPEC.md` — G-9 already commits to "`Layout::discover_from(cwd)` correctly resolves the worktree as the project root when invoked from inside one." This task leans on that guarantee: inside a worktree, `layout.root()` ends with `.ark/worktrees/<branch-type>/<slug>/`, so the slug is extractable from the path component immediately above. No spec change needed here — the task uses the existing topology.
