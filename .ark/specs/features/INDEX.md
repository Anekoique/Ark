# Feature Specs

Feature specifications extracted from deep-tier tasks at commit. Layout: `<feature>/SPEC.md`.

The table below is managed by `ark agent spec register` — new rows appear when a deep-tier task is committed with a promoted SPEC. **Do not hand-edit rows between the markers.** Edit outside the block, or let the CLI do it.

## Index

<!-- ARK:FEATURES:START -->
| Feature                    | Scope                                     | Promoted                                        |
| -------------------------- | ----------------------------------------- | ----------------------------------------------- |
| `ark-agent-namespace`      | add `ark agent` tool for agents to invoke | 2026-04-24 from task `ark-agent-namespace`      |
| `ark-upgrade`              | add `ark upgrade` support                 | 2026-04-24 from task `ark-upgrade`              |
| `ark-context`              | Add ark context command                   | 2026-04-27 from task `ark-context`              |
| `codex-support`            | add Codex platform support                | 2026-04-27 from task `codex-support`            |
| `opencode-support`         | add OpenCode platform support             | 2026-04-28 from task `opencode-support`         |
| `worktree`                 | add worktree support                      | 2026-04-28 from task `worktree-support`         |
| `project-spec`             | add project-spec                          | 2026-04-30 from task `project-spec`             |
| `task-concurrency-control` | task concurrency control                  | 2026-05-01 from task `task-concurrency-control` |
| `ark-workflow-refactor`    | refactor ark-workflow                     | 2026-05-02 from task `ark-workflow-refactor`    |
| `workspace` | add workspace support | 2026-05-02 from task `workspace` |
| `subagent-support` | add researcher/reviewer/verifier subagents across claude/codex/opencode | 2026-05-10 from task `subagent-support` |
| `detachable-feature-spec` | support detachable feature SPEC | 2026-05-18 from task `detachable-feature-spec` |

<!-- ARK:FEATURES:END -->

---

## How to use

- **Read:** scan the table; open the SPEC for any feature you'll touch.
- **Modify a feature SPEC:** append a `[**CHANGELOG**]` entry. Ark re-writes the `Promoted` column with the latest touch date.
