# Roadmap

## What we may need ?

**Some ideas**
- Quick install and uninstall (ark load & ark unload)
- AI workflow with `DESIGN(BrainStorm) -> PLAN(REVIEW) -> EXECUTE -> REVIEW`
  (/ark:design -> /ark:plan -> /ark:execute -> /ark:review) (draft -> plan -> spec)
  draft proposed task
    -> split and organize subtasks
    -> PLAN task and dispatch SUBPLAN to subtask
    -> call codex/create subagent claude to review
    -> pass with limited loop and execute
    -> call codex/create subagent claude to review
    -> FINAL REVIEW with higher request
    (previous mainly about function, currently include code quality / organization / abstract design)
    -> final commit and record spec/log/mm to archive
- Memory and log System (day drive or task drive?) (/ark:mem /ark:log)
  managed by ark mems / ark logs
- Consider stello (Agent Cognitive Topology Engine) or streamlined
- Multi-agent Orchestrate.
- Workspace management (member drive or task drive?)
- Multi platform support
- System level / project level management?
- A General purpose to harness and control the coding-agents and improve coding works.

[Worflow enhancement]

- Currently REVIEW will ask for self-review or spawn an sub-agent for review. We should add configurable options for invoking codex review, human review, or creating sub-agent reviews.
  Human intervention during circulation.
- Provide user-defined workflows like building blocks, instead of predefined ones.
  Add Workspace support. See trellis.
- Better memory(spec and tasks) management, learn idea stello.
- Add a spec extraction mechanism through docs/codes to support older projects.
- Add Hook support which useful for codebase-overview before any tasks.
  ...

[Cli enhancement]

- Cli extensions for memory management (ark mem) , task management (ark task) which provide cli tools for Agent invoke directoly without understanding natural language.
- Convenience management to coding-agent settings (cross-platform) with simple cli. Consider a ark skill add apply skill to all platforms or manage skill through ./ark/skills. See cc-switch.
  ...

[Platform support]

- Add agent and more commands to .claude
- Add support for codex, opencode...
  ...

## Phase 0: Basic Framework.

Ship the minimum scaffold: `ark init`, template set, three-tier workflow.

- CLI: `ark init` only
  - Scaffolds `.ark/` into the project
  - Renders slash commands into `.claude/commands/ark/`
  - Appends managed block to `CLAUDE.md`
  - Writes manifest `.ark/.installed.json` for future clean unload
- Three tiers documented in `workflow.md`
  - Quick → `/ark:quick` → `PRD.md`
  - Standard → `/ark:design` → `PLAN.md` + `REVIEW.md`
  - Deep → `/ark:design --deep` → iterated `NN_PLAN.md` / `NN_REVIEW.md`, SPEC extracted at archive
- Templates: `PRD.md`, `PLAN.md`, `REVIEW.md`, `SPEC.md`
- Directory layout: `specs/project/`, `specs/features/`, `tasks/`, `tasks/archive/`
- Slash commands: `/ark:quick`, `/ark:design` (basic, no subagent dispatch)

## Phase 1: CLI Service Surface

Extend CLI so slash commands stop shelling out to raw git/ls and instead call typed `ark` subcommands.

- `ark context [--format json|text] [--for design|plan|review]` — bundle git + current task + active tasks + relevant specs
- `ark task create <title> [--tier quick|standard|deep]`
- `ark task list [--status active|archived] [--format json]`
- `ark task show <id>` / `ark task current`
- `ark task advance --to <phase>`
- `ark task archive [--promote-spec]`
- `ark unload` (respects `.installed.json`)
- `ark update` (template refresh with non-destructive patching)

## Phase 2: Hooks & Multi-Platform

- `ark hook session-start|session-end|pre-tool-use|subagent-stop` (stdin JSON, exit-code contract)
- Render hooks into `.claude/settings.json` during `ark init`
- Render `.cursor/` and `.codex/` configurations

## Phase 3: Orchestration & Memory Bridge

- Multi-agent orchestration (worktrees, parallel subagents)
- `ark mem claude list|show|search|promote` — manage Claude Code's auto-memory
- `ark search` — unified query across tasks + specs + Claude memory + git
- Cross-repo workspace management
