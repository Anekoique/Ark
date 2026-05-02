# Feature Specs

Feature specifications extracted from deep-tier tasks on archive. Layout: `<feature>/SPEC.md`.

The table below is managed by `ark agent spec register` — new rows appear when a deep-tier task is archived with a promoted SPEC. Do not hand-edit rows between the markers; edit outside the block or let the CLI do it.

## Index

<!-- ARK:FEATURES:START -->
| Feature                    | Scope                                                 | Promoted                                        |
| -------------------------- | ----------------------------------------------------- | ----------------------------------------------- |
| `ark-agent-namespace`      | add `ark agent` tool for agents to invoke             | 2026-04-24 from task `ark-agent-namespace`      |
| `ark-upgrade`              | add `ark upgrade` support                             | 2026-04-24 from task `ark-upgrade`              |
| `ark-context`              | Add ark context command                               | 2026-04-27 from task `ark-context`              |
| `codex-support`            | add Codex platform support                            | 2026-04-27 from task `codex-support`            |
| `opencode-support`         | add OpenCode platform support                         | 2026-04-28 from task `opencode-support`         |
| `worktree`                 | add worktree support                                  | 2026-04-28 from task `worktree-support`         |
| `workspace`                | add workspace support (per-developer session journal) | 2026-04-29 from task `workspace`                |
| `project-spec`             | add project-spec                                      | 2026-04-30 from task `project-spec`             |
| `task-concurrency-control` | task concurrency control                              | 2026-05-01 from task `task-concurrency-control` |
| `ark-workflow-refactor`    | refactor ark-workflow                                 | 2026-05-02 from task `ark-workflow-refactor`    |
| `ark-workflow-refactor` | refactor ark-workflow | 2026-05-02 from task `ark-workflow-refactor` |

<!-- ARK:FEATURES:END -->

---

## How to Use

**When reading:** scan the table, open the SPEC for any feature you'll touch.
**When a task modifies a feature SPEC:** update its `[**CHANGELOG**]` entry; Ark re-writes the table's `Promoted` column with the latest touch date.
