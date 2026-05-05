# Ark

Ark is an agent harness and development workflow designed to orchestrate AI-driven programming tasks.

Use the simple CLI `ark` to define your AI workflow and manage coding agents in your project.

> Current status: Ark is experimental and some features are unstable.

## Why Ark ?

Ark defines the next generation of AI-native software engineering work: structured workflows, agent collaboration, and tool orchestration.

Ark provides:

- A simple CLI for managing AI-native software projects.
- A workflow harness for coordinating Claude Code, Codex, and other coding agents.
- Session-level journals for tracking agent activity and workspace state.
- Task-level development records with integrated worktree management.
- Specification management for projects and features.

## Installation

Prebuilt binaries are available for macOS, Linux, and Windows on every tagged release.

### npm

```bash
npm install -g @anekoique/ark
```

### Cargo

```bash
cargo install --git https://github.com/Anekoique/ark ark-cli --locked
```

Confirm the installation:

```bash
ark --version
```

## Quick Start

From the root of a project where you want to use Ark:

```bash
ark init
```

On first run, Ark scaffolds:

```
.ark/
├── workflow.md           # the rules of the game
├── templates/            # PRD, PLAN, REVIEW, VERIFY, SPEC
├── tasks/                # active + archived tasks
└── specs/
    ├── project/          # user-authored project conventions
    └── features/         # feature specs promoted from deep-tier tasks

.claude/commands/ark/
├── quick.md              # /ark:quick
└── design.md             # /ark:design [--deep]

CLAUDE.md                 # managed block pointing Claude Code at .ark/
```

Open Claude Code in the project and start a task:

```
/ark:quick  fix typo in readme
/ark:design add rate-limit middleware
/ark:design --deep refactor auth layer
```

See `.ark/workflow.md` for the full workflow.

## Lifecycle

`ark` provides the following commands for managing its presence in a project:

| Command       | What it does                                                 |
| ------------- | ------------------------------------------------------------ |
| `ark init`    | Scaffold `.ark/` and Claude Code integration from the embedded templates. |
| `ark load`    | Restore from `.ark.db` or initialize Ark if no snapshot exists. |
| `ark unload`  | Snapshot `.ark/`, managed blocks, and the Ark hook into `.ark.db`. |
| `ark remove`  | Wipe Ark fully: `.ark/`.                                     |
| `ark upgrade` | Refresh embedded templates to match the current CLI version. |
| `ark context` | Current `ark` related context for the project                |

## Inspiration

Ark is highly inspired by and learns from:

- [trellis](https://github.com/mindfold-ai/trellis)

- [superpowers](https://github.com/obra/superpowers)

- [openspec](https://github.com/Fission-AI/OpenSpec) / [spec-kit](https://github.com/github/spec-kit)

- [humanize](https://github.com/humania-org/humanize)

## LICENCE

MIT