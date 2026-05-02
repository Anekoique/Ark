---
name: ark-record
description: Record a manual session entry into the developer's workspace journal. Use when the user wants to write down notes between tasks (explorations, observations, status updates).
---

# `ark-record`

Append a manual session entry to the developer's active journal under
`.ark/workspace/<dev>/journal-N.md`.

Task-driven entries are written automatically by `ark-commit`; do not run
`ark-record` for those.

## Preconditions

- `.ark/.developer` exists.

## Steps

### 1. Pull record context

```bash
ark context --scope record --format json
```

### 2. Append the entry

Append a block to the active journal containing three agent-authored
sections (heading, Summary, Main Changes table). Do not write auto-fields
(Date, Slug, Branch) — the CLI stamps them.

### 3. Stamp the auto-fields

```bash
ark agent workspace record --manual
```

The CLI locates the last `## Session N: <title>` heading, inserts the
auto-fields, and updates both indices. Transactional with a documented
drift exception (`JournalDriftDetected`).

## Failure modes

- `MissingIdentity` — run `ark init --developer <name>`.
- `EntryFileMalformed` — the journal does not end with a `## Session
  N: <title>` heading.
- `JournalDriftDetected` — concurrent appender; investigate manually.
