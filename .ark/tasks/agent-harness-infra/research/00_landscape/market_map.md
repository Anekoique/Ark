# Market Map — the 2026 ecosystem at one zoom level

Compiled 2026-05-20. Five buckets, ~6-10 named players per bucket, plus an "evaluations" cross-cut. Each entry: one-sentence positioning, OSS/commercial/research, momentum snapshot. Detailed per-project profiles are in `01_prior_art/`.

The goal of this file is the *altitude* the per-project corpus loses: what is crowded vs. emerging vs. dying, where the field is converging, where the puck is going. Not all assertions are equally strong — qualifiers ("as of 2026-Q2") flag where the picture is moving fast.

---

## Bucket 1 — IDE-native agents

The dominant *user-facing* category circa 2026. AI is the primary interaction model; the IDE is built around it. Most users in this bucket are non-CLI-comfortable engineers; the bucket controls the median-developer relationship.

| Name | Type | Positioning |
| ---- | ---- | ----------- |
| **Cursor** | Commercial | Dominant incumbent. VS Code fork, Composer agent, Background Agents, `.cursor/rules` system. Multi-billion-dollar valuation. |
| **Windsurf** (Codeium) | Commercial | Cascade-flow IDE; merge-pending acquisition by OpenAI rumours (late 2025), then by Cognition (early 2026). Strong autopilot mode. |
| **Zed** | OSS (GPL-licensed core + commercial) | Rust-native AI-first editor. Sponsor of ACP. Agent Panel + Parallel Agents + Profiles. Smaller share but technically distinctive. |
| **GitHub Copilot Workspace / Agent Mode** | Commercial | Workspace was sunset 2025-05-30; primitives rebuilt as Coding Agent (async, issue→PR) and Agent Mode (sync, in-IDE). GitHub-native distribution. |
| **JetBrains AI Assistant** | Commercial | Bundled across all JetBrains IDEs. ACP adopter. Conservative but vast install base. |
| **Continue.dev** | OSS (Apache 2.0) | Open-source agent inside VS Code + JetBrains + a CLI (`cn`). Spec-driven 2026 pivot, hub for community config. |
| **Cline** (and **Roo-Cline**) | OSS (MIT) | VS Code extension; plan/act mode split, MCP Marketplace, Memory Bank, checkpoints. 60k+ stars. Roo-Cline is the autonomous fork. |

**State of the bucket:** Crowded and consolidating. Cursor leads on usage; Zed leads on architecture distinctiveness; Copilot leads on enterprise distribution. The plug-in-vs-fork question is settled — Cursor / Windsurf prove forking VS Code works; Copilot proves staying inside it works. Both viable; the diff is who you can ship to (enterprise => Copilot).

---

## Bucket 2 — CLI harnesses

The category Ark inhabits. Terminal-native, scripted into workflows, less polish, more power-user fit. Margin players by usage but disproportionately influential among the kind of people who build the next layer.

| Name | Type | Positioning |
| ---- | ---- | ----------- |
| **Aider** | OSS (Apache 2.0) | The originator. Tree-sitter repo-map, git-native commits, diff-format specialist. ~45k stars. Active development. |
| **Claude Code** | Commercial CLI (with thin OSS templates) | Anthropic's official CLI. Richest extension surface in the field — skills, hooks, MCP, subagents, plugins, plan mode, settings hierarchy. Sets the de-facto standard for "what a CLI harness should expose". |
| **OpenAI Codex CLI** | OSS (Apache 2.0, Rust) | OpenAI's official CLI. AGENTS.md, TOML subagents, OS-level Seatbelt/Landlock sandboxing, worktrees-per-subagent. ~84k stars. Direct competitor to Claude Code. |
| **OpenHands CLI** | OSS (MIT) | All-Hands-AI's CLI shell over the OpenHands core. Pluggable runtimes (Docker/E2B/Modal/K8s). 74k+ stars. Strong on autonomous workflows. |
| **Goose** | OSS (Apache 2.0, Rust) | Block's open-source agent. Recipes + extensions (MCP) + subagents + skills. Smaller but actively shipped. |
| **Plandex** | OSS (MIT, Go) | Plan-first agent with a cumulative-diff sandbox, 2M context, tree-sitter project maps, client-server architecture. ~15k stars. |
| **Ark** (this project) | OSS (MIT) | Workflow-opinionated harness layer above Claude Code / Codex / OpenCode. Tiered tasks (quick/standard/deep/research) + auto-promoted feature SPECs. |
| **SWE-agent** | OSS, research-leaning | Princeton-NLP. ACI thesis: agent-computer interfaces matter as much as model capability. The reference for "harness as substrate for benchmarks". |
| **OpenCode** (sst/opencode) | OSS | TS/Bun-native agent CLI with a unique runtime plugin model (`.opencode/plugins/*.ts`). Ark already integrates with it. |

**State of the bucket:** Crowded but differentiating. Aider invented the format; Claude Code and Codex are now the defining commercial entries; Ark/OpenHands/Plandex/Goose carve workflow-opinion niches. Convergence on AGENTS.md + skills + MCP as the lingua franca means the *primitives* are commoditising; *workflow opinion* is where remaining differentiation lives.

---

## Bucket 3 — Cloud autonomous agents

Hosted, async, "submit a task and walk away". The opposite of CLI harnesses on the spectrum from "developer drives every step" to "agent drives every step". Most ship a web UI; some integrate as a PR-bot.

| Name | Type | Positioning |
| ---- | ---- | ----------- |
| **Devin** (Cognition) | Commercial closed cloud | Pioneer of the autonomous-agent claim. VM-per-session, playbooks, Knowledge Base, machine snapshots, ACU billing, multi-Devin orchestration. Closed source. Released blockdiff (Rust) as open-source byproduct. |
| **Replit Agent** | Commercial | Agent 3 owns prompt → deployed app. Novel REPL self-testing (real browser automation catches Potemkin UIs). 200-min autonomy windows. |
| **GitHub Copilot Coding Agent** | Commercial | Issue → PR in the background. Spawned by `gh agent` or directly from issues. Closest to "spec-to-PR" formalism. |
| **Cursor Background Agents** | Commercial | Up to 8 parallel worktree-isolated agents per session, run inside Cursor's cloud. |
| **Codex Cloud** | Commercial | Hosted OpenAI Codex equivalent — same primitives as the CLI but on Codex-managed VMs. |
| **bolt.new** (StackBlitz) | Commercial | Web-native generator running on WebContainers (in-browser Node.js). Live preview + AI-driven scaffold. |
| **v0** (Vercel) | Commercial | React+Tailwind+shadcn opinionated stack. Agentic Mode multi-agent. Open Workflow SDK for durable execution. |
| **Augment** | Commercial | Enterprise positioning — "context engine" + agent. |

**State of the bucket:** Hot. Capital flowing in. Devin's 2024–2025 narrative ("autonomous engineer") legitimised the category; the open question through 2026 is reliability bar. Replit's behavioural verification (run the app, see if it works) is the most distinctive idea. Most bucket members charge per-task or per-ACU rather than per-token, which incentivises long-horizon autonomy work.

---

## Bucket 4 — Agent infra / runtime platforms

The substrate. Not user-facing as agents; rather, *the box agent code runs in*. Used directly by harness vendors and increasingly by enterprises building bespoke agents.

| Name | Type | Positioning |
| ---- | ---- | ----------- |
| **E2B** | Commercial (with OSS components) | Firecracker microVMs for code-interpreter / agent sandbox use cases. Used by OpenHands, Modal, internal tools. |
| **Modal** | Commercial | Python-first serverless with agent primitives (durable runs, sandboxed image execution). Used by OpenHands runtime, others. |
| **Daytona** | OSS (Apache 2.0) + commercial | Agent-agnostic dev environment infra. OpenHands integration. Self-hostable. |
| **Coder** | Commercial (with OSS workspace platform) | Governance-first cloud workspaces. AI Bridge / Agent Boundaries product layered on top. |
| **AWS Bedrock AgentCore** | Commercial | AWS's five-primitive stack: Runtime, Memory, Identity, Gateway, Tools. Enterprise distribution. |
| **Cloudflare Agents** | Commercial | Stateful hibernating Durable Objects as agent bodies. MCP-native. Strong serverless story. |
| **WebContainers** (StackBlitz) | Proprietary tech, freely available SDK | In-browser Node.js sandbox. Powers bolt.new; also licensed standalone. |

**State of the bucket:** Emerging. The 2024-era "you need a Docker sandbox" answer is now "you need an opinionated runtime with memory, identity, and MCP gateway". E2B and Modal lead on agent-specific tooling; AWS and Cloudflare lead on enterprise distribution. Likely consolidation as platform players pull-in features that started as boutiques.

---

## Bucket 5 — Framework SDKs

Libraries for *building* agents, not running them. The 2023–2024 dominant category; 2025–2026 it is fading as harnesses ship turn-key and frameworks fragment into supervisor/runtime/observability stacks.

| Name | Type | Positioning |
| ---- | ---- | ----------- |
| **LangChain / LangGraph** | OSS | LangChain is the original loose framework; LangGraph is the focused successor (StateGraph, supervisor pattern). Stack now includes LangSmith (observability) and LangServe (deploy) — straddling into platform. |
| **OpenAI Agents SDK** | OSS (TypeScript / Python) | OpenAI's official agent library. Handoffs primitive, tool definitions, traces. Re-positions older Assistants API. |
| **AutoGen** (Microsoft Research) | OSS | Multi-agent conversation framework. v0.4 rewrite around 2024-end. Less momentum than competitors. |
| **CrewAI** | OSS | Crew + role abstraction; hierarchical mode auto-generates a manager. Active but smaller install base. |
| **Pydantic AI** | OSS | Typed agent framework on top of Pydantic. Type-system-first; specialised audience. |
| **DSPy** | OSS / research | Stanford's prompt-as-code framework. Distinct philosophy (compile prompts), niche but influential. |
| **deepagents** | OSS | Letta/MemGPT lineage. File-backed deep-context agents. |

**State of the bucket:** Dying as pure category. The 2024 hot "agent framework" pitch ("LangChain for X") finds fewer takers in 2026 because most users buy a harness (Claude Code, Cursor, Ark) and don't write Python. Surviving frameworks straddle into adjacent layers (LangChain → LangSmith → LangServe; OpenAI Agents SDK → Codex CLI → Codex Cloud). Pure libraries with no shipping product are losing mind share.

---

## Cross-cut — Evaluations & benchmarks

Not a vendor bucket, but a layer every other bucket depends on. The benchmarks define what "agents that work" means.

| Name | Positioning |
| ---- | ----------- |
| **SWE-bench / SWE-bench Verified** | Princeton-NLP / OpenAI. Real-world GitHub issues; verified subset filtered for clean grading. Industry standard for code-agent ranking. |
| **SWE-bench Multilingual / Polyglot** | Aider's multi-language extension. Tests across Python / JS / Rust / Go / Java / C++ / Swift. |
| **Aider's polyglot benchmark** | The de-facto leaderboard for diff-format quality. |
| **TerminalBench** | Anthropic / Stanford. Tests terminal command sequencing. |
| **MRCR v2 / RULER** | Multi-needle context-recall benchmarks. The yardstick for effective context window. |
| **SWE-bench Adversarial (SWE-ABS)** | Newer adversarial subset designed to defeat memorisation. |

**State of the cross-cut:** Active and contested. Vendors increasingly self-publish numbers; community holds them to the verified subsets. The dominant benchmark question by 2026 is not "can the model solve X" but "can the *harness* solve X" — harness quality moves SWE-bench scores by ±20 percentage points with the same underlying model.

---

## Crowded / emerging / dying — at a glance

**Crowded (consolidating):**
- IDE-native agents (Cursor / Windsurf / Copilot / Continue / Cline)
- CLI harnesses (Aider / Claude Code / Codex / OpenHands)
- Framework SDKs (still many products, less and less mindshare)

**Emerging (still adding new entrants):**
- Cloud autonomous agents (Devin, Replit, Copilot Coding Agent, Cursor Background, Codex Cloud)
- Agent infra platforms (E2B, Modal, Daytona, Coder, Bedrock AgentCore, Cloudflare Agents)
- Workflow harnesses (Ark, OpenSpec-as-harness, spec-kit)

**Dying or consolidating away:**
- Pure framework SDKs without an adjacent harness/runtime story (AutoGen-as-library, early LangChain shape)
- Custom RAG-for-code products (Sourcegraph Cody re-positioning; standalone code-RAG vendors squeezed)
- "Agent" wrappers that are just GPT-4 + prompt templates (commoditised away by Claude Code / Cursor)

---

## Where the puck is going — 2026–2027 predictions

Five directions the trajectory points toward. Each has supporting evidence from the buckets above.

### 1. Cross-platform context files consolidate around AGENTS.md

Evidence: 16+ tools read AGENTS.md as of 2026-Q2 (per `01_prior_art/agent-platforms.md`). Codex made it official. OpenAI, GitHub Copilot Coding Agent, Goose, Continue all support it. CLAUDE.md / `.cursor/rules` / `.github/copilot-instructions.md` continue to exist but the *portable* file is AGENTS.md.

**Bet:** Ark should always write AGENTS.md, even on Claude-only installs.

### 2. Skills (SKILL.md) become the portable behaviour-pack format

Evidence: Claude Code, Codex (`openai/skills`), Goose, Cursor all converged on `<dir>/SKILL.md` + frontmatter as the format. Claude Code's 2026 docs explicitly favour skills over slash commands.

**Bet:** Ark's slash commands and Codex skills should be generated from one canonical source.

### 3. MCP wins as the agent↔tool protocol; A2A and ACP fight for agent↔agent

Evidence: MCP adoption by Anthropic, OpenAI, Google DeepMind, Cursor, Zed, Continue. A2A donated to Linux Foundation Jun 2025. ACP shipping in Zed/JetBrains/Kiro/OpenCode. The agent↔tool layer is settled; the agent↔agent layer is open.

**Bet:** Ark expose `ark agent` as an MCP server is a thin, defensible move; A2A/ACP support is too early to commit but worth tracking.

### 4. Harness quality matters more than model quality on SWE-bench

Evidence: Same underlying model varies ±20% SWE-bench score across harnesses. SWE-agent's ACI thesis. Aider's diff format research. Anthropic's Claude Code achieves SOTA partly via harness, not just model.

**Bet:** Ark's investment in workflow opinion is on the right side of this. The "intent before edits" school + tiered ceremony is a harness-level differentiator that is durable across model upgrades.

### 5. Workflow-opinionated harnesses are a real category, not a niche

Evidence: spec-kit, OpenSpec, Trellis, Kiro, Ark are all variations on "structured artifact-driven workflow above a coding agent". Aider's `/architect`/`/code` split is a primitive version. Continue's spec-driven pivot in 2026. Devin's playbooks. Replit Agent's owned-stack scaffolding.

**Bet:** Ark is in a *category*, not alone. Defensible positioning: tier system + auto-promoted feature SPECs + multi-platform-by-templates. This is genuinely distinctive within the category.

---

## Directions for Ark

1. **Always write AGENTS.md, regardless of platform.** Convergence evidence is strong; the cost is one extra file write per install; the option value is portability if the user switches host platforms. (Hook into `crates/ark-core/src/commands/init.rs` platform-write paths.)

2. **Treat the harness-quality thesis as a positioning claim.** The market map shows the field is moving from "model X is the best" to "harness X is the best". Ark's pitch — tiered, spec-driven, multi-platform — slots cleanly into that frame. Use it in the README rewrite proposed in `definitions.md`.

3. **Watch ACP carefully but do not commit yet.** Zed/JetBrains backing it gives ACP real reach into the IDE world. If ACP wins, Ark exposing an ACP-compatible adapter is a single-binary investment that opens IDE integration. If MCP-as-A2A wins instead, the same outcome via different protocol. Track quarterly; decide late.

4. **Position against the "agent OS" hype.** The category is full of premature claims. RFC 0001 is the right place to be — long-term aspiration, near-term humility. Continue resisting the temptation to ship copy that calls Ark an OS.

5. **Look at spec-kit and OpenSpec as direct peers, not just inspirations.** They sit in the same category Ark inhabits. Differentiate explicitly: spec-kit is project-level ceremony (one ceremony per repo); Ark is per-task tiers (right ceremony for each task). OpenSpec is propose→change loop; Ark is PRD→PLAN→SPEC extraction. The differences are real and shippable as positioning.
