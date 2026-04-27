# Feature Specs

Feature specifications extracted from deep-tier tasks on archive. Layout: `<feature>/SPEC.md`.

The table below is managed by `ark agent spec register` — new rows appear when a deep-tier task is archived with a promoted SPEC. Do not hand-edit rows between the markers; edit outside the block or let the CLI do it.

## Index

<!-- ARK:FEATURES:START -->
| Feature | Scope | Promoted |
|---------|-------|----------|
| `ark-agent-namespace` | add `ark agent` tool for agents to invoke | 2026-04-24 from task `ark-agent-namespace` |
| `ark-upgrade` | add `ark upgrade` support | 2026-04-24 from task `ark-upgrade` |
| `ark-context` | Add ark context command | 2026-04-27 from task `ark-context` |
| `codex-support` | add Codex platform support | 2026-04-27 from task `codex-support` |

<!-- ARK:FEATURES:END -->

---

## How to Use

**When reading:** scan the table, open the SPEC for any feature you'll touch.
**When a task modifies a feature SPEC:** update its `[**CHANGELOG**]` entry; Ark re-writes the table's `Promoted` column with the latest touch date.
