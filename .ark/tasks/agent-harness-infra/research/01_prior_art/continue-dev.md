# Continue.dev

- Date: 2026-05-20
- Scope: external

## Identity

- **Name:** Continue (formerly "Continue Dev")
- **URL:** https://continue.dev — repo at https://github.com/continuedev/continue
- **License:** Apache-2.0. Open source.
- **Maintainer:** Continue Dev, Inc.
- **Momentum (as of 2026-05):** ~33.3k GitHub stars, ~4.5k forks, 822 releases on the canonical repo; v1.2.22-vscode is the latest tagged release per the GitHub landing. Active across VS Code, JetBrains, and a CLI (`cn`). Recently pivoted product framing from "AI assistant" to "source-controlled AI checks, enforceable in CI" — a notable repositioning.

## Positioning

Continue is the **open-source counterweight** to Cursor: a multi-IDE extension (VS Code, IntelliJ) plus a CLI (`cn`), powered by user-configurable models, hosted on developer hardware. The pitch is sovereignty: you choose the model, you keep the data, you commit the rules into your repo.

The 2026 repositioning towards "source-controlled AI checks, enforceable in CI" is a hedge — moving up the stack from "another AI sidebar" to "an AI quality gate that runs in CI" (closer to spec-/check-driven workflows than chat-driven coding).

## Primitives

- **Modes** — `Chat`, `Edit`, `Agent`, `Autocomplete`. Each is a different UI surface with different prompts and tool access. Agent mode is the closest analogue to Cursor's Composer.
- **Assistants** — bundled configurations (model + rules + tools). Loaded from the Continue **Hub** (cloud config marketplace) or local `~/.continue/config.yaml`.
- **Config (`config.yaml`)** — schema-validated YAML defining models, rules, MCP servers, context providers, slash commands.
- **Rules** — markdown files in `.continue/rules/` (per-project) and via Hub. Detected automatically; apply across Agent / Chat / Edit modes.
- **Checks** — markdown files in `.continue/checks/`. Each defines an automated review agent that can run in CI as a GitHub status check. This is the 2026 differentiator.
- **MCP servers** — configurable in `config.yaml` or pulled from Hub. Used by both IDE extensions and the `cn` CLI.
- **Context providers** — pluggable sources of context (`@codebase`, `@docs`, `@files`, `@terminal`, custom).
- **Slash commands** — invokable workflows (e.g. `/edit`, `/comment`, `/test`).
- **`cn` CLI** — terminal-resident agent that uses the same `config.yaml` as the IDE extensions. Supports cloud agents triggered from CI / GitHub.

## Workflow model

Continue's workflow is **mode-dispatched, agent-configurable**:

- **Chat** for Q&A about the codebase.
- **Edit** for selection-scoped edits.
- **Agent** for autonomous multi-step work with tool calling.
- **Autocomplete** for inline completions.

The architectural shift in 2026 is `cn`-led: the same configuration that powers your IDE can run as a GitHub Action, producing PR-blocking status checks. You define a "Security Review" check (markdown describing what to verify), and it runs on every PR. This is *workflow as code* in a way Cursor and Zed don't attempt — though the workflow itself is single-pass review, not Ark-style PRD→PLAN→EXECUTE→VERIFY.

## Context & memory

- **Context providers** — extensible context sources. `@codebase` does repo-scope retrieval; `@docs` does documentation retrieval; users can add their own.
- **Rules** — declarative always-on or scoped context.
- **MCP servers** — including memory MCP servers from the Hub for long-term memory.
- **Hub configs** — assistants pulled from a cloud registry. Auto-syncing across IDE instances.
- **`.env` resolution** — secrets pulled from a local `.env` using mustache notation `{{ env.MY_KEY }}`. Hub configs use Mission Control for secret resolution.

The rules + context-providers split is similar to Cursor's rules + skills, but Continue's flavour is more configuration-centric (one YAML to rule them all) where Cursor leans on individual markdown files with frontmatter.

## Tool / capability surface

- **MCP** — first-class. Continue extensions, the `cn` CLI, and the CI checks all share the same MCP integration.
- **Built-in tools** — file ops, terminal execution, web (configurable), code search.
- **Tool permission system** — `cn` ships minimal permissions and adds policies to `~/.continue/permissions.yaml` as the user approves tool calls. This is more conservative and auditable than most agents' "ask once and remember" pattern.
- **Sandbox** — none owned by Continue. Tools execute on the host.

## Integration model

The architecture is the most interesting part of Continue and the cleanest reference for "agent harness across IDEs":

```
core <-> extension <-> gui
```

- **core** — IDE-agnostic business logic (model calls, RAG, context provider runtime, agent loop). Designed to be bundled as a binary.
- **extension** — VS Code / IntelliJ-specific glue. Sets up core + gui, implements the `IDE` interface (file system, diagnostics, terminal access etc.), routes messages between core and gui.
- **gui** — React-based UI shipping inside the extension. Renders chat, edit, agent panels.

Messages flow as:

- `ToCoreFromWebviewProtocol` (gui → core, via extension)
- `ToWebviewFromCoreProtocol` (core → gui, via extension)
- `ToIdeFromWebviewProtocol` (gui → IDE)
- `ToWebviewFromIdeProtocol` (IDE → gui)

This is essentially a Language-Server-Protocol-style separation: the IDE-agnostic core can be swapped onto any IDE that implements the `IDE` interface. The `cn` CLI is the headless version of `core` without `extension` or `gui`.

This decoupling is what Zed's ACP is trying to standardise as an *external* protocol; Continue has an *internal* equivalent.

## Multi-agent / orchestration

Continue is single-agent within a thread. Multi-agent orchestration is delegated to:

- **Cloud agents via `cn`** — kick off a longer-running agent from CI / a GitHub Action.
- **CI checks** — multiple checks can run on a PR in parallel; each is an independent agent.

There is no in-IDE subagent dispatch like Roo Code's Boomerang or Cursor's parallel agents.

## Spec / artifact system

- **Checks** are the closest analogue. Each check is a markdown file describing a review concern. They produce status-check artifacts on PRs but not long-lived specs.
- **Rules** are declarative conventions; the same role as Ark project SPECs but flatter (no INDEX / hierarchy).
- **Assistants** are reusable configurations — a layer of "this is the setup for the X domain" that Ark does not have.

No PRD / PLAN / REVIEW / VERIFY chain.

## Strengths over Ark

1. **Open source under Apache-2.0.** Auditable, forkable, self-hostable.
2. **Multi-IDE.** VS Code + JetBrains + CLI from a single configuration. Ark targets specifically Claude Code (and via templates Codex / OpenCode). Continue solves a different "multiple frontends" problem.
3. **CI-runnable.** `cn` in a GitHub Action turns AI review into a status check. Ark's review-as-gate model is local-only; no CI hook.
4. **Hub configs.** Marketplace of pre-built assistants pulled from a remote registry. Ark templates are embedded in the binary and updated per release; no remote / community-contributable registry.
5. **Schema-validated YAML config.** A single `config.yaml` with schema enforcement is simpler to teach than Ark's `task.toml` + `.ark/config.toml` + workflow.md + slash commands.
6. **Internal core/extension/gui split.** A more mature architectural model for "agent harness across many hosts" — useful prior art if Ark wants to abstract beyond CLI-host slash commands.

## Weaknesses / gaps

1. **No spec / planning workflow.** Continue has nothing like Ark's PRD → PLAN → REVIEW → VERIFY. The 2026 pivot to checks moves it closer to "AI process" but in the form of CI gates, not lifecycle ceremony.
2. **No multi-agent orchestration in-IDE.** No subagent dispatch, no parallel-agent UX, no Boomerang-style task decomposition.
3. **Configuration sprawl.** `config.yaml` + Hub configs + checks + rules + context providers = many places to look. Less opinionated than Ark's workflow.
4. **No worktree / sandbox primitive.** The agent runs on the host. No isolation story comparable to Cursor's Background Agents or Daytona/E2B sandboxes.
5. **No CLI-driven workflow state machine.** The IDE / `cn` is the entry point. Compared to Ark's `ark agent task plan|execute|verify|commit` state machine with legality checks, Continue is more freeform.

## Directions for Ark

1. **Steal the core/extension/gui separation as Ark's internal architecture model.** Today Ark is a CLI + templates. If Ark eventually grows non-CLI surfaces (an HTTP server, an ACP server, a VS Code extension), Continue's IDE-agnostic-core pattern is a proven blueprint: keep all business logic in `ark-core`, push platform-specific glue (slash commands, ACP server, web UI) into thin adapters. The `ark-cli` / `ark-core` split already gestures at this; formalising the boundary would future-proof.

2. **Ship a `ark check` mode that runs in CI.** Continue's most novel 2026 move is "AI review as enforceable CI status check." Ark's REVIEW phase is local. A `ark check` command that runs the latest PLAN's review heuristics or a project-SPEC compliance pass as a status check on a PR would make Ark's workflow legible to teams without forcing them into the full lifecycle.

3. **Hub / registry for project SPECs and feature SPECs.** Ark's project SPECs are user-authored per-repo. A community "Hub" of canonical project SPECs (Rust comments style, Python error handling, React component conventions) that users can pull into `.ark/specs/project/` would lower onboarding. Continue's Hub already proves the pattern works.

4. **Schema-validated configuration with frontmatter.** Continue uses YAML with a JSON schema. Ark uses TOML for `task.toml` and `.ark/config.toml` but no schema validation surfaced to users. A `ark config validate` (or implicit at parse time) with helpful diagnostics would mirror Continue's friendliness.

5. **Permission policy file (`permissions.yaml`).** Continue's `~/.continue/permissions.yaml` records cumulative tool approvals as the user grants them. Ark's host agents have their own per-platform permission systems (Claude `allowedTools`, Codex auto-accept). A unifying `ark permissions` surface that writes through to each platform's native config — using the existing template overlay model — would let Ark be the single place to manage agent capabilities.

## Sources

- [GitHub — continuedev/continue](https://github.com/continuedev/continue)
- [Continue Docs — config.yaml Reference](https://docs.continue.dev/reference)
- [Continue Docs — How to Configure Continue](https://docs.continue.dev/customize/deep-dives/configuration)
- [Continue Docs — How to Use Continue CLI (`cn`)](https://docs.continue.dev/guides/cli)
- [Continue Docs — CLI Configuration](https://docs.continue.dev/cli/configuration)
- [Continue Docs — Understanding Configs (Hub vs Local)](https://docs.continue.dev/guides/understanding-configs)
- [Continue Docs — Understanding Assistants](https://docs.continue.dev/guides/understanding-assistants)
- [Continue Docs — How to Create and Manage Rules](https://docs.continue.dev/customize/deep-dives/rules)
- [Continue Docs — Configuring Models, Rules, and Tools](https://docs.continue.dev/guides/configuring-models-rules-tools)
- [Continue Blog — Building Cloud Agents with Continue CLI](https://blog.continue.dev/building-async-agents-with-continue-cli)
- [DeepWiki — continuedev/continue overview](https://deepwiki.com/continuedev/continue)
- [DeepWiki — Core System](https://deepwiki.com/continuedev/continue/3-core-components)
- [DeepWiki — GUI System](https://deepwiki.com/continuedev/continue/2.3-gui-system)
- [DeepWiki — VS Code Extension](https://deepwiki.com/continuedev/continue/6-vs-code-extension)
- [DeepWiki — IntelliJ Plugin](https://deepwiki.com/continuedev/continue/7-development)
- [DeepWiki — IDE Integration Patterns / Communication Flow](https://deepwiki.com/continuedev/continue/2.4-communication-flow)
- [DeepWiki — Plugin Architecture](https://deepwiki.com/continuedev/continue/5.2-plugin-architecture)
- [Continue Hub — continuedev/rules-memory](https://hub.continue.dev/continuedev/rules-memory)
- [Cursor-Alternatives — Continue.dev Rules & Config (2026)](https://cursor-alternatives.com/blog/continue-dev-rules/)
