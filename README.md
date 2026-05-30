# Ark

An agent harness and development workflow for orchestrating AI-driven programming tasks.

Use the simple CLI `ark` to define your AI workflow and manage coding agents in your project.

📖 **See the [deck](https://anekoique.github.io/Ark/ark-deck.html)** for the design.

> **Status**: Ark is experimental and some features are unstable.

## Why Ark ?

Ark defines the next generation of AI-native software engineering work: structured workflows, agent collaboration, and tool orchestration.

Ark provides:

- A simple CLI for managing AI-native software projects.
- A workflow harness for coordinating Claude Code, Codex, and other coding agents.
- Sandbox for isolated execution environment.
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
├── config.toml           # worktree, workspace, upgrade, sandbox settings
├── templates/            # PRD, PLAN, REVIEW, VERIFY, SPEC
├── tasks/                # active + archived tasks
└── specs/
    ├── project/          # user-authored project conventions
    └── features/         # feature specs promoted from deep-tier tasks

.claude/commands/ark/     # slash commands (Claude Code; Codex/OpenCode equivalents)
├── quick.md              # /ark:quick
├── design.md             # /ark:design [--deep]
├── research.md           # /ark:research
└── …                     # commit, resume, discard, record, extract-spec

CLAUDE.md                 # managed block pointing the agent at .ark/
```

Open your agent in the project and start a task:

```
/ark:quick    fix typo in readme
/ark:design   add rate-limit middleware
/ark:design --deep  refactor auth layer
/ark:research compare WAL strategies
```

See `.ark/workflow.md` for the full workflow.

## Tiers

Pick the smallest tier that fits.

| Tier         | For                                            |
| ------------ | ---------------------------------------------- |
| **Quick**    | Reversible in one commit, no new abstractions  |
| **Standard** | Feature work with testable scope               |
| **Deep**     | Architectural or new subsystem                 |
| **Research** | Knowledge-gathering; corpus is the deliverable |

## Lifecycle

`ark` provides the following commands for managing its presence in a project:

| Command       | What it does                                                       |
| ------------- | ------------------------------------------------------------------ |
| `ark init`    | Driven project with ark                                            |
| `ark load`    | Restore from `.ark.db` if present.                                 |
| `ark unload`  | Snapshot `.ark/`, managed blocks, and the Ark hook into `.ark.db`. |
| `ark remove`  | Wipe Ark fully.                                                    |
| `ark upgrade` | Refresh embedded templates to the current CLI version.             |
| `ark context` | Get current project context which driven ark                       |
| `ark archive` | Stabilize some features.                                           |
| `ark cleanup` | List or remove worktrees of closed tasks.                          |
| `ark sandbox` | Run agent inside a isolated enviroment.                            |

User-authored files inside owned dirs survive an `unload` → `load` round-trip losslessly.

## Sandbox

`ark sandbox` provide isolated execution environment.

```bash
ark sandbox create   # start the box
ark sandbox enter    # launch the agent CLI inside (--shell for bash)
ark sandbox rm       # tear it down
```

Configure the image, env passthrough, and host-config sharing under `[sandbox]` in `.ark/config.toml`. Requires `docker` on `PATH`.

## Inspiration

Ark is highly inspired by and learns from:

- [trellis](https://github.com/mindfold-ai/trellis)

- [superpowers](https://github.com/obra/superpowers)

- [openspec](https://github.com/Fission-AI/OpenSpec) / [spec-kit](https://github.com/github/spec-kit)

- [humanize](https://github.com/humania-org/humanize)

## LICENCE

MIT
