# Roadmap

## What we may need ?

[Some ideas]

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

[Workflow enhancement]

- Currently REVIEW will ask for self-review or spawn a sub-agent for review. We should add configurable options for invoking codex review, human review, or creating sub-agent reviews.
  Human intervention during circulation.
- Provide user-defined workflows like building blocks, instead of predefined ones.
  Add Workspace support. See trellis.
- Better memory(spec and tasks) management, learn idea stello.
- Add a spec extraction mechanism through docs/codes to support older projects.
- Add Hook support which useful for codebase-overview before any tasks.
- **Strengthen VERIFY**. Today VERIFY checks plan-fidelity, correctness, and SPEC drift, but doesn't scrutinize function length, cross-file redundancy, or abstraction strength. The `codex-support` task surfaced this: VERIFY approved with minor follow-ups, but a separate cleanup pass found ~5 sites of structural redundancy that should have been findings. Enumerate code-quality dimensions in the verifier prompt; consider an "approved-after-refactor" verdict that gates archive on a cleanup pass.
  ...

[Cli enhancement]

- Cli extensions for memory management (ark mem) , task management (ark task) which provide cli tools for Agent invoke directoly without understanding natural language.
- Convenience management to coding-agent settings (cross-platform) with simple cli. Consider a ark skill add apply skill to all platforms or manage skill through ./ark/skills. See cc-switch.
  ...

[Platform support]

- Add agent and more commands to .claude
- Add support for codex, opencode...
  ...
