# Replit Agent (and the Ghostwriter Evolution)

- Date: 2026-05-20
- Scope: external

## Identity

- **Name:** Replit Agent (currently Agent 3); legacy product line called Ghostwriter (chat / inline / debug)
- **URL:** https://replit.com — blog at https://blog.replit.com
- **License:** Proprietary, cycles-based pricing on Replit's platform. Ghostwriter / Agent are bundled in paid plans.
- **Maintainer:** Replit, Inc.
- **Momentum (as of 2026-05):** Agent 3 launched 2025-09; introduced "extended autonomy" (200+ minute self-driving sessions). Per Replit's 2026 statistics piece, ~$9B platform valuation, ~5M apps built with Ghostwriter v2 in 2025, ~250k deployed to production via one-click hosting. **Agent 3 can build other agents** — the first widely-shipped product to treat agent-creation as an agent capability.

## Positioning

Replit's positioning is the **anti-IDE play**: don't run Cursor on your laptop, don't fork VS Code — run everything in the cloud, agent-first, browser-only. The differentiator vs Cursor: Replit owns the runtime. The agent doesn't just write code; it runs the resulting app in a Replit REPL, tests it, deploys it, monitors it.

The Ghostwriter → Agent evolution is the clearest "autocomplete grew into autonomous agent" arc in the space:

- **Ghostwriter (2022)** — completions + chat, Copilot-class.
- **Ghostwriter Chat / Inline (2023)** — conversational, proactive debugging.
- **Agent v1 (2024)** — autonomous multi-step builds inside Replit.
- **Agent v2 (2025)** — Claude 3.5 Sonnet integration, real-time preview, autonomous env config.
- **Agent 3 (2025-09)** — REPL-based self-testing, 200-min autonomous sessions, **builds other agents**.

## Primitives

- **Repl** — the runtime sandbox. Owns the filesystem, terminal, network, package manager, browser preview.
- **Agent** — top-level autonomous workflow. Reads the user's natural-language request, plans, edits, executes, tests, deploys.
- **Dynamic Intelligence** — set of three capabilities (extended thinking, high-power model, web search) that the agent can switch on for harder tasks.
- **Autonomy Level** — user-controllable knob: Low / Medium / Max. Max enables 200+ min sessions.
- **REPL-based verification** — a Replit-proprietary self-testing system that combines code execution and browser automation. Cheaper than Computer Use, 3× faster.
- **Agent generation** — Agent 3 can produce *new* agents (in-Repl), letting users define repeatable automations in natural language.
- **One-click deploy** — push the result to Replit's hosting.
- **Workspaces / Teams** — multi-developer projects with shared Repls.

## Workflow model

Replit's flow is **single-prompt-to-running-app**, optimised for non-developers and quick prototyping:

1. User describes the app in natural language.
2. Agent provisions a Repl (env, dependencies, scaffolding).
3. Agent writes code, runs it, observes errors in the terminal.
4. Periodically (autonomously) spins up a real browser, interacts with the running app (clicking buttons, submitting forms, navigating logins) to verify it works — the **Potemkin interface** defence: a UI that *looks* right but is wired up wrong gets caught.
5. Iterates until the app passes its self-tests.
6. User reviews and either approves changes, asks for revisions, or deploys.

In Max autonomy mode, the loop runs for hours with minimal user touch points.

This is **closer to an agent-driven full-stack builder than a coding assistant**. The PRD/PLAN/EXECUTE/VERIFY ceremony is implicit: PRD = user prompt; PLAN/EXECUTE/VERIFY happen inside the agent's loop without external artifacts.

## Context & memory

- **Repl-local state** — files, env vars, sessions persist in the Repl.
- **Conversation history** — per-Repl agent thread.
- **Project memory** — agent maintains a mental model of the project across sessions (per Replit; details opaque).
- **No `.replit/rules/` equivalent.** Replit historically ships configuration via `.replit` and `replit.nix` files for the *runtime*, not for *agent behaviour*. Project-level agent instructions are typically embedded in the agent prompt or stored in a `README.md` the agent reads.

## Tool / capability surface

- **REPL filesystem + shell** — agent has full POSIX access inside the Repl.
- **Browser automation** — for self-testing.
- **Package manager** — npm / pip / poetry / nix.
- **Deployment** — one-click push to Replit Hosting.
- **Search / web** — via Dynamic Intelligence.
- **Database** — Replit-managed Postgres, key-value store.
- **Secrets manager** — encrypted env vars.
- **MCP** — not as central as Cursor / Continue. Replit's tool surface is primarily Replit-native (the Repl itself is the sandbox).

## Integration model

Replit is **closed-loop**: it owns the IDE, the runtime, the agent, the deployment, and the hosting. The closest analogue is Bolt.new but with persistent backend Repls rather than ephemeral WebContainers. The integration story is "everything is in Replit" — there is no real story for using Replit's agent against a repo hosted on GitHub from a local VS Code instance.

Mobile support — Replit's iOS / Android apps let you supervise an agent from a phone. This is the most aggressive "agent-as-coworker" UX in the field — text the agent, it builds the app.

## Multi-agent / orchestration

- **Agent generation** — Agent 3 builds new agents that the user can invoke later. This is multi-agent *generation* rather than multi-agent *coordination*.
- **No formal Orchestrator pattern.** Subtasks happen inside the agent's reasoning loop, not as separate visible agents.
- **Teams** — multiple developers can supervise the same Repl, but only one agent thread runs at a time per Repl.

## Spec / artifact system

- **No spec system.** The conversation is the artifact.
- **README.md** as informal long-term memory.
- **Repl-level config** (`.replit`) is runtime-not-workflow.
- **Deployment artifacts** are the durable output (the deployed app).

## Strengths over Ark

1. **Owns the runtime end-to-end.** Replit is the only player here that controls every layer from prompt to deployed app. Ark depends on the host environment for everything below the workflow layer.
2. **Self-testing via real browser automation.** Agent 3's REPL-based verification is a genuinely novel primitive. It catches the Potemkin-interface failure mode — code that compiles, types check, tests pass, but the UI is wired wrong. Ark's VERIFY phase is checklist-based and depends on the user / verifier subagent reading code. No automated browser-driven verification.
3. **Long-horizon autonomy.** 200+ minute sessions with minimal supervision. Ark's lifecycle assumes the user is present at each phase transition.
4. **Agent-builds-agent.** Replit treats meta-agency as a feature. Ark has no equivalent: users can't ask Ark to generate a new project SPEC or a new tier of workflow on the fly.
5. **Mobile-first supervision.** Drive a build from your phone. Ark is terminal-only.
6. **Zero local setup.** Browser-only. Ark requires a local repo + git + a host CLI.

## Weaknesses / gaps

1. **Vendor lock-in.** Apps run on Replit's runtime. Migrating off is awkward.
2. **Not suited for existing codebases.** The story optimises for greenfield apps inside a Repl, not for adopting AI agent workflows into an existing local repo.
3. **No spec / planning artifacts.** Conversations are ephemeral. The "what did the agent do" history is opaque if the user wants to audit.
4. **Proprietary.** Closed source, closed runtime.
5. **Single-developer UX.** Teams supervise *together*, not in parallel. No worktree / branch isolation story.
6. **Pricing is consumption-based and unpredictable** in long autonomous sessions.

## Directions for Ark

1. **Browser-based or browser-aware verification.** Agent 3's REPL-based verification is the most concrete novel idea here. Ark's VERIFY phase could add a category — **Behavioural Verification** — that runs the built artifact (binary, server, web page) and validates it interactively. Concretely: a `VERIFY.md` item could be "smoke-tested by `ark verify run-smoke` which spawns the binary and runs <command-list>." For web work, integration with a headless browser (Playwright) would mirror Agent 3's approach. Ark's research-tier corpus is the natural place to evaluate this further (see `02_infra_primitives/sandboxing.md` for sandbox options).

2. **Long-horizon supervision: checkpoints that survive process death.** Replit can run for 200 minutes because state is durably checkpointed. Ark's `task.toml` + filesystem state is *durable but not granular* — a crash mid-EXECUTE leaves the task in EXECUTE with no record of how far through the implementation phases it was. A `task.toml.checkpoint` writing per-step state would let a host agent recover or hand off mid-execution.

3. **Autonomy level as a `task.toml` field.** Replit's Autonomy slider is a single UX knob with real semantic weight (per-tool approval prompts on Low, fully autonomous on Max). Ark could expose `task.toml.autonomy = "low" | "medium" | "max"`. `low` would mean "require user confirmation between phases"; `max` would mean "host agent may walk all phases without confirmation." This is orthogonal to tier (which controls *artifacts*); autonomy controls *gating*.

4. **Agent-generated SPECs (meta-agency).** Replit's "Agent 3 builds other agents" is intriguing precedent. Ark could explore an `ark spec draft <name>` that asks the host agent to draft a project SPEC by reading the codebase. Today writing a new project SPEC is purely manual; agent-assisted drafting is a natural fit.

5. **A mobile / remote supervision surface.** A long-running Ark task today is unobservable from outside the developer's terminal. Replit's mobile supervision is built on a stable HTTP API. If Ark gains a `ark serve` mode (see Zed's ACP direction) the same daemon could power a "watch from anywhere" view. Lower priority but increasingly expected.

## Sources

- [Replit Blog — Introducing Agent 3: Our Most Autonomous Agent Yet](https://blog.replit.com/introducing-agent-3-our-most-autonomous-agent-yet)
- [Replit Blog — Enabling Agent 3 to Self-Test at Scale with REPL-Based Verification](https://blog.replit.com/automated-self-testing)
- [Replit Blog — Introducing Dynamic Intelligence for Replit Agent](https://blog.replit.com/dynamic-intelligence)
- [Replit Blog — Ghostwriter AI & Complete Code Beta](https://blog.replit.com/ai)
- [Replit Blog — Meet Replit Ghostwriter, your partner in code](https://blog.replit.com/ghostwriter)
- [Replit Blog — Building Ghostwriter Chat](https://blog.replit.com/ghostwriter-building)
- [Replit Blog — Announcing Ghostwriter Chat: The first conversational AI programmer](https://blog.replit.com/gw-chat-launch)
- [Replit Blog — Improving the Inline Ghostwriter Experience](https://blog.replit.com/ghostwriter-inline)
- [Replit Docs — Autonomy Level](https://docs.replit.com/replitai/autonomy-level)
- [Replit Learn — Intro to Ghostwriter](https://replit.com/learn/intro-to-ghostwriter)
- [InfoQ — Replit Introduces Agent 3 for Extended Autonomous Coding and Automation](https://www.infoq.com/news/2025/09/replit-agent-3/)
- [Skywork — Replit Agent 3: A Deep Dive into the Future of Autonomous Coding](https://skywork.ai/blog/replit-agent-3-a-deep-dive-into-the-future-of-autonomous-coding/)
- [LeaveIt2AI — Replit Agent 3 (2026): 200-Minute Autonomy & Self-Healing Code](https://leaveit2ai.com/ai-tools/code-development/replit-agent-v3)
- [Taskade — What is Replit? From JSRepl to $9B AI Platform (2026)](https://www.taskade.com/blog/replit-ai-history)
- [Index.dev — Replit Statistics in 2026](https://www.index.dev/blog/replit-usage-statistics)
- [SearchYour.ai — Replit Introduces Agent 3, Its most autonomous AI agent to date](https://www.searchyour.ai/en/replit-agent-3-autonomous-ai-agent-automatic-testing)
