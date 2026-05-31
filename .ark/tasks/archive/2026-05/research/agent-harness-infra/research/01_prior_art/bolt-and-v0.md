# Bolt.new (StackBlitz) and v0 (Vercel)

- Date: 2026-05-20
- Scope: external

Bolt.new and v0 are paired here because they share a strategic frame — **web-native agent platforms that generate full apps from prompts**, owning the runtime end-to-end. Different stacks; similar shape. Both ship features that map cleanly onto the worktree / sandbox / preview primitives Ark cares about, but in a browser-first form factor.

---

## Bolt.new

### Identity

- **Name:** Bolt.new
- **URL:** https://bolt.new — repo (UI / agent scaffolding) at https://github.com/stackblitz/bolt.new; community fork bolt.diy at https://github.com/stackblitz-labs/bolt.diy
- **License:** Proprietary product; bolt.new source (UI/agent scaffolding) under MIT; bolt.diy is a permissively-licensed community fork.
- **Maintainer:** StackBlitz, Inc.
- **Momentum (as of 2026-05):** Launched 2024-10-04. Per several 2026 review pieces and the Posthog newsletter, hit ~$40M ARR rapidly; one of the fastest-growing AI-app builder products to date. Built atop StackBlitz's **WebContainers** technology.

### Positioning

Bolt.new is the **anti-Replit play in the browser**: rather than a remote Repl + remote runtime, Bolt runs *everything inside the browser tab*. WebContainers — StackBlitz's in-browser Node.js runtime via WebAssembly — eliminate the cloud VM. The agent has filesystem, npm, terminal, and Node server access without a server.

The marketing pitch: "from prompt to deployed app in your browser, no setup." The technical pitch (the more interesting one): the agent has full control over an in-browser POSIX-like environment.

### Primitives

- **WebContainer** — in-browser Node.js VM (WebAssembly). The sandbox.
- **Project** — a Bolt session = files + WebContainer + chat history.
- **Chat** — the agent interface. Three-pane UI: chat / editor / live preview.
- **Live preview** — real browser iframe over the WebContainer, interactive.
- **Enhance prompt** — pre-process a user's prompt with an AI refinement step before sending.
- **Deploy** — push to bolt.host (free subdomain) or Netlify with a single click.
- **bolt.diy** — community fork that lets you bring any LLM (OpenAI / Anthropic / Ollama / etc.).

### Workflow model

Single-pass: user describes the app; the agent scaffolds the project, installs dependencies, writes code, runs the dev server, opens the preview. The user iterates by typing follow-up prompts. The agent "has complete control over the entire environment including the filesystem, node server, package manager, terminal, and browser console" (per the system prompt). No explicit plan or spec stage.

### Context & memory

- **Project files** — the entire WebContainer FS is in agent context.
- **Conversation history** — per-project.
- **System prompt** — extensive, public in the repo (`app/lib/.server/llm/prompts.ts`); includes instructions like "be specific about your stack."
- **No project rules / persistent memory** across projects.

### Tool / capability surface

- **Full POSIX-ish environment** in WebContainer: filesystem, terminal, npm, node.
- **Package CDN** — popular packages live in a pre-compressed Bolt CDN; npm install often completes in <500ms or is skipped entirely.
- **Compile / build via Web Workers** keeping the UI thread free.
- **CommonJS shim** — Rust/WASM kernel exposes a fake `process` and a slim VM API for Babel/TypeScript.

### Integration model

- **Browser-only.** The product is a webpage. The integration story is "any browser is the IDE."
- **No CLI / IDE plugin.** Bolt is its own surface.
- **Netlify integration** for deployments.

### Multi-agent / orchestration

Single-agent. No subagent / multi-agent UX.

### Strengths over Ark

1. **Zero local setup.** Browser-only.
2. **Owns the runtime.** Like Replit, runs the resulting app inline. Ark depends on user-provided builds.
3. **Live preview as agent feedback channel.** Agent can observe its own output via preview; closer to closed-loop iteration than Ark's "build, hope, retry."
4. **Deploy is one click.** Ark has no notion of deployment.

### Weaknesses / gaps

1. **No existing-codebase story.** Bolt is for greenfield apps. Importing an arbitrary repo and getting Bolt's agent to work on it is awkward at best.
2. **WebContainer constrains the stack.** Node-focused; other ecosystems (Python, Rust, Go) are second-class or unsupported.
3. **No spec / planning / rules system.** Conversational only.
4. **No durability across machines.** Switch browsers / close tab → state could be lost.

### Directions for Ark (Bolt-derived)

1. **Live preview / interactive verification.** Like the Replit-derived direction for browser-driven smoke tests, Bolt's three-pane (chat / editor / preview) UX is the most concrete demonstration of "agent watches its own UI." For Ark, the analogue is a VERIFY phase that *executes* the artifact (CLI binary, web server) and validates *behaviour*, not just code reading.
2. **Pre-warmed dependency cache.** Bolt's pre-compressed CDN layer for popular packages cuts install time to milliseconds. Ark's worktree creation copies files; it could pre-prime a cargo / npm cache directory so each worktree starts with dependencies resolved. Marginal but useful at scale.
3. **System prompt as project artifact.** Bolt's system prompt is public, in the repo, and reviewable. Ark's host-agent prompts are template-driven but not surfaced as part of the project's auditable artifacts. Consider treating the AGENTS.md / CLAUDE.md / equivalent as the system-prompt artifact, with `ark` writing structured sections.

---

## v0 (Vercel)

### Identity

- **Name:** v0 (the product) / v0.app (the URL) / v0 by Vercel
- **URL:** https://v0.app — AI SDK at https://ai-sdk.dev
- **License:** v0 product is proprietary, paid subscription; AI SDK is MIT (open source). Vercel's Workflow SDK is also open.
- **Maintainer:** Vercel, Inc.
- **Momentum (as of 2026-05):** v0 started as a UI generator (React + Tailwind + shadcn/ui) and expanded into "Agentic Mode" — full multi-agent app builder. AI SDK 3.0 open-sourced the Generative UI primitives. Vercel Workflow (workflow SDK) at https://github.com/vercel/workflow gives durable execution of multi-step apps and AI agents.

### Positioning

v0 is **the design-system-aware app builder**: trained heavily on Vercel's preferred stack (React, Tailwind, shadcn/ui), with deep integration into Vercel's deployment pipeline. Initially a single-purpose UI generator, evolved in 2026 into a multi-agent end-to-end builder ("Agentic Mode").

The strategic surface: deeply opinionated about the stack, deeply integrated with Vercel deployment. Less "blank canvas," more "scaffolds an app that matches our taste."

### Primitives

- **Project** — a v0 session.
- **Chat / Prompts** — the agent interface.
- **Generative UI** — components stream into the canvas as the agent produces them; not pure text streaming.
- **Agentic Mode** — multi-agent orchestrated planning + research + code generation + debugging. Auto-selects models per step.
- **Workflow blocks** — visual workflow editor, drag-and-drop steps, persistent state, compiled to TypeScript via the WDK (Workflow Development Kit).
- **AI SDK** — open primitives (`createStreamableUI`, `createStreamableValue`, `useChat`, etc.) for building Generative UI in your own apps.
- **Deploy** — push to Vercel.

### Workflow model

Initially: single-pass prompt → React component. Today: Agentic Mode runs **multiple coordinated agents** (planner, research, coder, debugger) automatically selecting the right model per task. User reviews and iterates.

The Workflow Builder template separately codifies workflows visually — a useful primitive on its own: drag-and-drop steps, "use workflow" and "use step" TypeScript directives that compile to a runtime execution graph with state management and error handling.

### Context & memory

- **Project files** in v0 are the agent's working context.
- **Vercel deployment data** can be referenced.
- **AI SDK level** — pluggable context providers; user-controlled.

### Tool / capability surface

- **React + Tailwind + shadcn/ui** is the default stack.
- **Database integration** (Vercel Postgres / KV).
- **Multi-model dispatch** via Vercel AI Gateway.
- **MCP** — supported through the AI SDK.
- **Workflow primitives** for durable execution outside v0 itself.

### Integration model

- **Browser-only** for v0 itself.
- **AI SDK** is the embeddable / open-source surface — Vercel's bet that the SDK becomes the standard library for building Generative UI agents elsewhere.
- **Workflow SDK** is the durable-execution layer for agents that need to outlive a request.

### Multi-agent / orchestration

Agentic Mode coordinates multiple agents (planner, researcher, coder, debugger) with auto-model selection. The visual Workflow Builder additionally lets users *design* multi-step / multi-agent flows.

This is the **most explicit "agent orchestration as product"** of any platform in this section. Cursor / Roo / Bolt have orchestration as a side effect; v0 sells orchestration as a UX.

### Strengths over Ark

1. **Generative UI primitives.** Agents stream *components*, not just text. Ark's surface is plain text.
2. **Visual workflow builder.** Users design multi-step workflows graphically. Ark's workflow is hard-coded into the CLI.
3. **AI SDK as an open ecosystem.** Vercel monetises the proprietary v0 while making the building blocks free. This is a stronger long-term play than Cursor's closed stack.
4. **Multi-model dispatch built in.** Auto-pick the model per step. Ark has no model selection.
5. **Durable workflow execution.** The Workflow SDK supports long-running agents with checkpointed state — what Ark's lifecycle gestures at but doesn't implement.

### Weaknesses / gaps

1. **Stack-locked.** React + Tailwind + shadcn or you're outside v0's strength zone.
2. **No persistent project rules / spec system.**
3. **No git integration.** Output is downloadable code; not a PR.
4. **Vercel-deploy-centric.**

### Directions for Ark (v0-derived)

1. **Workflow-as-code via a tiny SDK.** Vercel Workflow's "use workflow" / "use step" TypeScript directives compile a function definition into a durable execution graph. Ark's workflow today is **hard-coded** as a Rust state machine in `ark-core/src/commands/agent/state.rs`. A more pluggable design — where a project's workflow could be *declared* (TOML or YAML or even a tiny DSL) and the CLI consumed it — would let teams customise their own ceremonies without forking Ark. Not a near-term priority; longer-term, this is how Ark scales beyond Claude Code / Codex / OpenCode.

2. **Multi-model dispatch with explicit per-phase model selection.** v0 auto-selects models per agent step. Combined with the Cursor / Roo / Zed directions, the pattern is clear: agents are *cheaper* when stronger models are reserved for hard phases (planning, review) and cheap models handle bulk (edits, tests). `task.toml` could carry a `[models]` block hinting per phase.

3. **Visual workflow inspection (CLI-friendly).** v0's drag-and-drop workflow builder isn't directly applicable, but a `ark workflow show --slug <s>` that renders the task's lifecycle as ASCII / Mermaid (where it currently is, what phases remain, which gates are blocking) would mirror the same UX value in the terminal.

4. **Treat the system prompt + project SPECs as the durable agent "program."** Vercel Workflow makes the agent's program a *file you commit*. Ark's project SPECs already function this way for conventions; extending the framing to include workflow customisations (which phases, which gates, which subagents) would close the loop.

---

## Sources

- [Bolt.new — Help Center / Introduction to Bolt](https://support.bolt.new/building/intro-bolt)
- [GitHub — stackblitz/bolt.new](https://github.com/stackblitz/bolt.new)
- [GitHub — stackblitz/bolt.new system prompt source](https://github.com/stackblitz/bolt.new/blob/main/app/lib/.server/llm/prompts.ts)
- [GitHub — stackblitz-labs/bolt.diy](https://github.com/stackblitz-labs/bolt.diy)
- [PostHog Newsletter — How bolt.new works](https://newsletter.posthog.com/p/from-0-to-40m-arr-inside-the-tech)
- [DeepWiki — stackblitz/bolt.new](https://deepwiki.com/stackblitz/bolt.new)
- [DeepWiki — bolt.new User Guide](https://deepwiki.com/stackblitz/bolt.new/1.3-user-guide)
- [Bolt.new Prompting Guide (2026)](https://sureprompts.com/blog/bolt-new-prompting-guide)
- [v0 by Vercel — Product Page](https://v0.app/)
- [Vercel Blog — Announcing v0: Generative UI](https://vercel.com/blog/announcing-v0-generative-ui)
- [Vercel Blog — Introducing AI SDK 3.0 with Generative UI](https://vercel.com/blog/ai-sdk-3-generative-ui)
- [Vercel Blog — Workflow Builder: Build your own workflow automation platform](https://vercel.com/blog/workflow-builder-build-your-own-workflow-automation-platform)
- [Vercel Academy — UI with v0](https://vercel.com/academy/ai-sdk/ui-with-v0)
- [Vercel Academy — Multi-Step & Generative UI](https://vercel.com/academy/ai-sdk/multi-step-and-generative-ui)
- [Vercel Academy — Builders Guide to the AI SDK](https://vercel.com/academy/ai-sdk)
- [AI SDK Docs — Introduction](https://ai-sdk.dev/docs/introduction)
- [GitHub — vercel/workflow](https://github.com/vercel/workflow)
- [Vercel Templates — Workflow Builder](https://vercel.com/templates/ai/workflow-builder)
- [V0 vs Bolt.new vs Lovable (NxCode comparison)](https://www.nxcode.io/resources/news/v0-vs-bolt-vs-lovable-ai-app-builder-comparison-2025)
- [V0 vs Bolt: Index.dev review](https://www.index.dev/blog/v0-vs-bolt-ai-app-builder-review)
