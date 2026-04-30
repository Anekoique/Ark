# Quick Start

From the root of a project you want Ark in:

```bash
ark init
```

On first run, `ark init` prompts you to pick which AI platforms to integrate (Claude Code / Codex / OpenCode) and asks for a developer name for the workspace journal. Pass `--no-developer` to skip the journal, or `--developer <name>` to bootstrap non-interactively.

## What gets scaffolded

```
.ark/
├── workflow.md           # the rules of the game
├── templates/            # PRD, PLAN, REVIEW, VERIFY, SPEC
├── tasks/                # active + archived tasks
├── specs/
│   ├── project/          # user-authored project conventions
│   └── features/         # feature specs promoted from deep-tier tasks
├── config.toml           # [worktree], [workspace] sections
├── .gitignore            # ignores .ark/worktrees/, .ark/.developer
└── .developer            # per-machine identity (gitignored)

.claude/                  # if Claude Code selected
├── commands/ark/         #   /ark:quick, /ark:design, /ark:archive, /ark:record
└── settings.json         #   SessionStart hook entry

.codex/                   # if Codex selected
├── skills/ark-quick/     #   ark-quick, ark-design, ark-archive, ark-record skills
├── config.toml           #   Codex hook config
└── hooks.json            #   SessionStart hook entry

.opencode/                # if OpenCode selected
├── commands/ark/         #   /ark:quick, /ark:design, /ark:archive, /ark:record
└── plugins/ark-context.ts

CLAUDE.md / AGENTS.md     # managed `<!-- ARK -->` block pointing at .ark/
```

What's user-owned vs. Ark-owned:

- **`.ark/specs/project/`, `.ark/tasks/`** — yours. Ark won't touch them on `upgrade`.
- **`.ark/templates/`, `.ark/workflow.md`, `.claude/commands/ark/`, `.codex/skills/`, `.opencode/commands/ark/`** — Ark-owned. `upgrade` refreshes them.
- **`.ark/config.toml`** — yours after first creation. `upgrade` will not overwrite your edits.
- **`CLAUDE.md` / `AGENTS.md`** — yours, with a managed block delimited by `<!-- ARK:START -->` / `<!-- ARK:END -->`. Edit anywhere outside the markers; `upgrade` re-renders inside them.

## Start a task

Open Claude Code (or Codex, or OpenCode) in the project and trigger a slash command:

```
/ark:quick fix typo in readme
/ark:design add rate-limit middleware
/ark:design --deep refactor auth layer
/ark:archive             # close out the current task
/ark:record              # log a manual session entry to your journal
```

Each command kicks off a workflow with the matching tier. The agent walks you through writing a PRD, then a PLAN (for standard/deep), then implements, then verifies. Tasks live in `.ark/tasks/<slug>/` while active and move to `.ark/tasks/archive/YYYY-MM/<slug>/` on close-out.

For the full mental model, jump to [Workflow → Tiers](../workflow/tiers.md). For a complete walkthrough of one task end-to-end, see [Your First Task](./first-task.md).

## What's next

- **Multiple in-flight tasks?** Pass `--worktree` to scaffold each task in a separate git worktree under `.ark/worktrees/<branch>/`. See [Worktrees and Workspaces](../workflow/worktrees-and-workspaces.md).
- **Customizing the workflow?** Drop a SPEC under `.ark/specs/project/<name>/SPEC.md`. The agent reads every project SPEC before starting any task.
- **Want the agent to log decisions?** Initialize a workspace journal at `ark init` time (or via `ark agent workspace init --name <you>` later) and call `/ark:record` after non-task work.
