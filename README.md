# Ark

An Agent Harness and Development Workflow Designed to Orchestrate AI-driven Programming Tasks.

Use the simple CLI `ark` to define your AI workflow and manage your coding agent.

> Current status: Ark is experimental and currently only supports Claude Code.

## Why Ark ?

AI coding agents work better with a harness.

- **Right ceremony, right task.** Three tiers — quick fix, feature, deep refactor — each with the minimum process that fits.
- **Reviewed, not rubber-stamped.** PLAN ↔ REVIEW iteration on deep work; a VERIFY gate before every archive.
- **Plain markdown, no hidden magic.** Tasks and specs live in `.ark/`, diffable and git-tracked; Ark writes only what it tracks.

## Installation

Prebuilt binaries ship for macOS, Linux, and Windows on every tagged release.

### npm

```bash
npm install -g @anekoique/ark
```

### Cargo (requires Rust toolchain)

```bash
cargo install --git https://github.com/Anekoique/ark ark-cli --locked
```

Confirm the install:

```bash
ark --version
```

## Quick Start

From the root of a project you want Ark in:

```bash
ark init
```

On first run this scaffolds:

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
/ark:quick fix typo in readme
/ark:design add rate-limit middleware
/ark:design --deep refactor auth layer
```

See `.ark/workflow.md` for the full workflow.

## Lifecycle

`ark` has four commands for managing its presence in a project:

| Command      | What it does                                                                     |
| ------------ | -------------------------------------------------------------------------------- |
| `ark init`   | Scaffold `.ark/` and Claude Code integration from the embedded templates.        |
| `ark load`   | Restore from `.ark.db` or init.                                                  |
| `ark unload` | Snapshot everything under `.ark/` + managed blocks into `.ark.db`.               |
| `ark remove` | Wipe Ark fully: `.ark/`, `.claude/commands/ark/`, managed blocks, and `.ark.db`. |

## Inspiration

The project is highly inspired and learn from the project: [trellis](https://github.com/mindfold-ai/trellis)

Also inspired by those projects:

[superpowers](https://github.com/obra/superpowers)

[openspec](https://github.com/Fission-AI/OpenSpec) / [spec-kit](https://github.com/github/spec-kit)

[humanize](https://github.com/humania-org/humanize)
