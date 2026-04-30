# `ark context`

Print a structured snapshot of git state and `.ark/` workflow state. Read-only.

## Synopsis

```
ark context [OPTIONS]
```

## Description

`ark context` is the single entry point for orientation. Slash commands invoke it at every phase boundary; you can invoke it manually any time. It never mutates state.

The output covers:

- Git state: current branch, HEAD, dirty file count and a capped list of dirty paths, recent commits.
- Active task: slug, tier, phase, iteration, artifact paths.
- Other active tasks (across worktrees).
- Project specs index.
- Feature specs index, optionally filtered by the active PRD's `[**Related Specs**]`.
- Recent archive (last 5).

## Flags

| Flag                    | Description                                                                                       |
| ----------------------- | ------------------------------------------------------------------------------------------------- |
| `--dir <path>`          | Project root. Walks ancestors looking for `.ark/`.                                                |
| `--scope {session\|phase}` | What to project. Default `session`. Phase-targeted slices require `--for`.                        |
| `--for <phase>`         | Required iff `--scope phase`. One of `design`, `plan`, `review`, `execute`, `verify`.             |
| `--format {text\|json}` | Output shape. Default `text`. Both modes derive from the same in-memory `Context` struct.         |

## Scopes

`--scope session` (default) returns full orientation:

```
## GIT STATUS
## CURRENT TASK
## ACTIVE TASKS
## SPECS
## ARCHIVE
```

Sections absent from the projection are omitted entirely.

`--scope phase --for <phase>` narrows to the slice that phase needs:

| Phase     | Specs included                                              | Archive | Tasks         |
| --------- | ----------------------------------------------------------- | ------- | ------------- |
| `design`  | full project + features unfiltered                          | yes     | none          |
| `plan`    | full project; features filtered to PRD's Related Specs      | no      | none          |
| `review`  | full project; features filtered to PRD's Related Specs      | no      | none          |
| `execute` | full project; features = `[]`                               | no      | none          |
| `verify`  | full project; features = `[]`                               | no      | none          |

The PRD's `[**Related Specs**]` parsing is by regex (`specs/features/[a-z0-9_-]+/SPEC\.md`); only paths that match are kept in the filtered list.

## JSON schema

`--format json` produces a stable JSON document with `"schema": 1` as the first field. Schema is **additive-only** going forward: new fields can appear; existing fields' names and types are stable across patch and minor releases. A field rename or removal requires bumping `SCHEMA_VERSION`.

The payload contains paths and summaries only — never file bodies. Artifacts appear as `{kind, iteration?, path, lines}`; specs as index-row data; archives as `{slug, title, tier, archived_at, path}`. Callers `Read` files they need.

`--scope session --format json` is wrapped in Claude Code's `SessionStart` hook envelope:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "<stringified projection JSON>"
  }
}
```

The inner `additionalContext` is the actual projection (still starts with `"schema": 1`), serialized as a JSON string. Every other `(scope, format)` combination emits raw output.

## SessionStart hook

`ark init` / `ark load` / `ark upgrade` install a `SessionStart` hook into `.claude/settings.json` (and platform-equivalents) that runs `ark context --scope session --format json`. The output is injected as additional context at the start of every Claude / Codex / OpenCode session — so the agent sees the project's current state without any user interaction.

The hook entry's identity is the canonical command string `ark context --scope session --format json`. Sibling user hooks at the same `SessionStart` array are preserved across `unload` / `load` round-trips.

## Examples

```bash
# Default human-readable session orientation.
ark context

# JSON for tooling.
ark context --format json

# Phase-targeted: what does the design phase need?
ark context --scope phase --for design --format json

# What does plan need (filtered features)?
ark context --scope phase --for plan
```

## Caching

None. Every invocation re-reads state. Cost is dominated by `git log -n 5 --oneline` and a directory walk of `.ark/specs/`.

## See also

- [Specs](../workflow/specs.md) — what gets included per phase.
- [Lifecycle](../workflow/lifecycle.md) — what each phase reads.
