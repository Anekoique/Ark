# Agent Infrastructure Platforms

- Date: 2026-05-20
- Scope: external

These platforms are **not direct peers to Ark** — Ark is workflow middleware; these are infra. But they shape what "agent infra" means in 2026 and inform what Ark might depend on (sandbox), what it can borrow (durable execution primitives), and what it should not try to be (low-level container runtime). Each platform gets a shorter (~80 line) treatment.

---

## E2B (e2b.dev)

### Identity

- **URL:** https://e2b.dev — repo at https://github.com/e2b-dev/E2B
- **License:** Open source (Apache-2.0); commercial cloud offering.
- **Maintainer:** E2B (independent, Czech-based founders; ~50% Fortune 500 adoption claim).
- **Momentum (as of 2026-05):** Reference sandbox-for-agents primitive. Firecracker microVM-based; 150ms cold start; <200ms full provision; SDKs in Python + JS/TS. ~5,000+ MCP servers integrate. The default answer to "where does my agent's code run?"

### What it provides

- **Isolated Linux microVMs** for arbitrary code execution.
- **Dockerfile-based templates** — build once, snapshot, restore per sandbox at boot time.
- **SDKs (Python, JS/TS)** — `Sandbox.create()`, `sandbox.run_code()`, `sandbox.filesystem.read()`, etc.
- **Lifecycle webhooks**, SSH access, interactive PTY, ability to connect to running sandboxes.
- **MCP server packaging** — E2B's SDK is the canonical way to wrap "run code" as an MCP tool.

### Strengths

- Hardware-isolated (Firecracker), not container-isolated; safer for untrusted AI-generated code than Docker.
- Sub-200ms provision. Snapshots make this nearly free at scale.
- SDK-first, no YAML.

### Weaknesses

- Compute-billed (no free tier above a small free allowance).
- Self-hostable but operationally complex; most users stay on the cloud.

### What Ark could borrow

- **Treat sandbox as a feature SPEC, not a built-in.** A `sandbox-execution` feature SPEC could describe how Ark optionally hands a task's EXECUTE phase to a remote sandbox (E2B, Daytona, or Modal). Today Ark assumes the host CLI runs locally and tools execute on the user's machine. A pluggable sandbox primitive widens Ark's reach to "tasks where the user shouldn't trust local execution."
- **Template-as-snapshot pattern.** E2B's "Dockerfile → snapshot → fast restore" model is the right shape for any future "task setup" caching. Ark's `worktree` already copies files; a snapshot abstraction would let `task new --worktree` start from a pre-built environment.

### Sources

- [E2B Homepage — The Enterprise AI Agent Cloud](https://e2b.dev/)
- [GitHub — e2b-dev/E2B](https://github.com/e2b-dev/E2B)
- [E2B Docs](https://e2b.dev/docs)
- [DeepWiki — Firecracker Integration](https://deepwiki.com/e2b-dev/infra/3.2-firecracker-integration)
- [Northflank — E2B vs Modal (2026)](https://northflank.com/blog/e2b-vs-modal)
- [Northflank — E2B vs Vercel Sandbox (2026)](https://northflank.com/blog/e2b-vs-vercel-sandbox)
- [Northflank — E2B vs Sprites (2026)](https://northflank.com/blog/e2b-vs-sprites-dev)
- [LikeClaw — What Is E2B Sandboxed Execution](https://likeclaw.ai/blog/what-is-e2b-sandboxed-execution/)

---

## Modal

### Identity

- **URL:** https://modal.com — docs at https://modal.com/docs
- **License:** Proprietary, with open-source SDK clients.
- **Maintainer:** Modal Labs.
- **Momentum (as of 2026-05):** General availability for Sandboxes; 50,000+ concurrent sandbox sessions cited; gVisor isolation, sub-second starts. Python-first; JS/TS and Go SDKs available.

### What it provides

- **Sandbox primitive** — `modal.Sandbox.create()` returns an isolated execution environment for untrusted / agent-generated code.
- **Python-defined images** — define container images in Python (no Dockerfiles required), with version pinning and dynamic image construction at runtime.
- **gVisor isolation** — userspace kernel for container security.
- **Per-sandbox egress policies** — control network access from within an agent's code.
- **Both "agent inside sandbox" and "agent outside sandbox" deployment models** — flexibility around security boundary placement.
- **Volumes / networks / scheduling / image building** as composable primitives.
- **devlooper example** — Modal-published reference of a program-synthesis agent that fixes its own test failures.

### Strengths

- Code-first IaC. No YAML, no Kubernetes manifests.
- Strong Python ergonomics — defining sandboxes feels like writing a function.
- Scaling story (50k+ concurrent) backed up by Modal's general-purpose AI infrastructure focus.

### Weaknesses

- Python-first; other languages are second-tier.
- gVisor (container) rather than Firecracker (VM); arguably less isolation than E2B.
- Proprietary control plane.

### What Ark could borrow

- **Python-defined / declarative images.** If Ark grows a sandbox-execution feature, "Dockerfile or Python or both" is a deliberate trade-off — Modal demonstrates one end of the spectrum that's friendlier to non-DevOps users.
- **`devlooper`-style self-fixing loop as a verifier pattern.** Modal's devlooper is the simplest published reference for "agent runs tests, observes failures, fixes code, retries." Ark's VERIFY phase could embed this — feed the verifier subagent a failing test, let it iterate within a budget, surface results.

### Sources

- [Modal Docs — Sandboxes Guide](https://modal.com/docs/guide/sandboxes)
- [Modal Docs — modal.Sandbox Reference](https://modal.com/docs/reference/modal.Sandbox)
- [Modal Docs — Run arbitrary code in a sandboxed environment](https://modal.com/docs/examples/safe_code_execution)
- [Modal Docs — Build a coding agent with Modal Sandboxes and LangGraph](https://modal.com/docs/examples/agent)
- [Modal Blog — Sandboxes are generally available](https://modal.com/blog/sandbox-launch)
- [Modal Blog — Top AI Code Sandbox Products in 2025](https://modal.com/blog/top-code-agent-sandbox-products)
- [Modal Blog — What is an AI code sandbox?](https://modal.com/blog/what-is-ai-code-sandbox)
- [Modal Blog — Best Code Execution Sandboxes for Coding Agents 2026](https://modal.com/resources/best-code-execution-sandboxes-coding-agents)
- [GitHub — modal-labs/devlooper](https://github.com/modal-labs/devlooper)

---

## Daytona

### Identity

- **URL:** https://daytona.io — repo at https://github.com/daytonaio/daytona
- **License:** Open core; pivot in Feb 2025 from "Dev environment manager" to "secure infrastructure for AI-generated code."
- **Maintainer:** Daytona Platforms, Inc.
- **Momentum (as of 2026-05):** Reference integration with OpenHands at openhands.daytona.io. Kubernetes + Helm + Terraform-based; powerful but ops-heavy. Sub-90ms environment launch (per their marketing).

### What it provides

- **Workspaces / sandboxes** — isolated full computers (Linux / Windows / macOS desktops with programmatic and visual control).
- **Sub-90ms environment launch** via snapshots.
- **Persistent state across sessions** — sandboxes can be snapshotted and resumed.
- **Agent-agnostic infrastructure** — exposes the standard primitives any agent needs (shell session, FS, exec feedback). Not tied to one agent.
- **Governance + ops controls** at the org level.
- **OpenHands reference integration** — `openhands.daytona.io` is a one-click try of OpenHands + Daytona.

### Strengths

- "Agent-agnostic" framing — Daytona is positioned as the *infrastructure layer*, not a competitor to coding agents. This is exactly the layer Ark *doesn't* want to be.
- Persistent snapshots = long-running agent sessions across machine boundaries.
- Open source core.

### Weaknesses

- Operationally complex (Kubernetes / Terraform). High bar for self-hosting.
- Aimed at organisations, not individual developers.

### What Ark could borrow

- **The "agent-agnostic infrastructure" positioning is itself instructive.** Ark's parallel framing is "workflow-agnostic-of-host-agent." Both refuse to be locked to one model / agent / framework. Communicating this is half the strategic battle.
- **Snapshot + resume.** Daytona's persistent snapshots are the cleanest way to keep a long-running agent alive across sessions. If Ark's task lifecycle gains "pause / resume mid-EXECUTE," it needs more than `task.toml` — it needs to capture working-tree state. `ark.db` (the snapshot file `ark unload` produces) is close in spirit; extending it to per-task lifecycle would mirror Daytona's pattern.

### Sources

- [Daytona Homepage](https://www.daytona.io/)
- [GitHub — daytonaio/daytona](https://github.com/daytonaio/daytona)
- [Daytona — From Dev Environments to AI Runtimes](https://www.daytona.io/dotfiles/from-dev-environments-to-ai-runtimes)
- [Daytona — Sandboxing AI Development with Agent-Agnostic Infrastructure](https://www.daytona.io/dotfiles/sandboxing-ai-development-with-agent-agnostic-infrastructure)
- [Daytona — Instant AI Development with Daytona and OpenHands](https://www.daytona.io/dotfiles/instant-ai-development-with-daytona-and-openhands)
- [Daytona — Building a Secure OpenHands Runtime with Daytona Sandboxes](https://www.daytona.io/dotfiles/building-a-secure-openhands-runtime-with-daytona-sandboxes)
- [OpenHands + Daytona](https://openhands.daytona.io/)
- [ToolDirectory — Daytona: Containerized Dev Environments for AI Agents](https://tooldirectory.ai/tools/daytona)
- [Northflank — Top Daytona.io Alternatives](https://northflank.com/blog/top-daytona-io-alternatives-for-running-ai-code-in-secure-sandboxed-environments)

---

## Coder Workspaces

### Identity

- **URL:** https://coder.com — repo at https://github.com/coder/coder
- **License:** AGPL (open source); commercial tier for enterprise.
- **Maintainer:** Coder Technologies, Inc.
- **Momentum (as of 2026-05):** Major repositioning announced in 2026 — now sold as "Coder Agents + Coder AI Governance + Coder Workspaces" with **AI Bridge** and **Agent Boundaries** as production GA features. Identity shifted from "dev productivity tool" to "AI developer infrastructure."

### What it provides

- **Self-hosted cloud development environments** — workspaces defined in Terraform / OpenTofu, connected via Wireguard.
- **VS Code Extension + JetBrains Gateway** — connect from local IDE.
- **Automatic workspace shutdown** when idle (cost-saving).
- **AI Bridge / Agent Boundaries (2026)** — governance layer turning workspaces into a control plane for AI coding agents: identity, observability, enforcement.
- **Same workspace primitive for humans and agents** — a developer and the agent they dispatched share infrastructure and policies.
- **Coder Agents (2026)** — agents that can generate the Terraform/IaC artifacts needed to provision their own environments. Meta-agency.

### Strengths

- **Governance-first.** RBAC, audit logs, BYOC, network controls, GPU support. Designed for compliance-constrained environments.
- **Same infrastructure for humans + agents** — eliminates the "dev env drift" between developer and agent workflows.
- **Terraform / OpenTofu standard.** Workspaces are normal IaC, not bespoke Coder DSL.

### Weaknesses

- Self-hosted means infra ownership burden.
- Aimed at orgs / enterprises; individual developer is not the primary persona.
- Agent governance is a 2026 add-on, not yet battle-tested.

### What Ark could borrow

- **Governance overlay separate from execution.** Coder splits "what the workspace does" (Terraform) from "what's allowed" (AI Bridge / Agent Boundaries). Ark's slash commands enforce phase transitions; an equivalent *policy layer* (e.g. "this org disallows deep-tier tasks without code review by a human reviewer") would expose Ark to enterprise constraints.
- **Same primitive for agent and human.** Coder's insight that developers and their agents should share infrastructure is the same argument for `ark` running both human-driven and agent-dispatched tasks through the same lifecycle. Stay disciplined about this; resist "human-only" and "agent-only" branches of the workflow.

### Sources

- [Coder Homepage](https://coder.com/)
- [Coder — Governed Workspaces for AI Coding Agents](https://coder.com/solutions/workspaces)
- [Coder Blog — AI Agents Are Already in Your Codebase. Is Your Infrastructure Ready?](https://coder.com/blog/ai-agents-are-already-in-your-codebase-is-your-infrastructure-ready)
- [Coder Blog — Secure Agentic AI, Now Production-Ready](https://coder.com/blog/secure-agentic-ai-now-production-ready)
- [GitHub — coder/coder](https://github.com/coder/coder)
- [Efficiently Connected — Coder Expands into AI Developer Infrastructure](https://www.efficientlyconnected.com/coder-ai-developer-infrastructure-agents-governance/)
- [Coder Docs — Terraform Modules](https://coder.com/docs/admin/templates/extending-templates/modules)
- [The Coders Blog — Coder: Revolutionizing Remote Development with Open Source (2026)](https://thecodersblog.com/coder-open-source-remote-development-environment-2026/)

---

## AWS Bedrock AgentCore

### Identity

- **URL:** https://aws.amazon.com/bedrock/agentcore — SDK at https://github.com/aws/bedrock-agentcore-sdk-python
- **License:** AWS service; SDK Apache-2.0 (open).
- **Maintainer:** AWS.
- **Momentum (as of 2026-05):** Heavily marketed inside the AWS ecosystem. Framework-agnostic primitives + AWS-managed infrastructure. Identity-centric, with first-class IAM / OAuth2 integration.

### What it provides — five primitives

- **Runtime** — containerised hosting for an agent or tool. Long-running (up to 8 hours), real-time + async workloads, fully managed scaling and session isolation.
- **Memory** — managed conversation + long-term memory. ~21 tools in the SDK for memory resources / records.
- **Identity** — OAuth2, API key credential providers, token vaults. AgentCore agents access AWS / third-party services as themselves or on behalf of users.
- **Gateway** — transforms existing APIs / Lambdas / Smithy / OpenAPI specs into MCP-callable tools. The "expose any API as an agent tool" surface.
- **Tools (Browser, Code Interpreter)** — managed primitives.

### Strengths

- **Composable, framework-agnostic primitives.** Works with Strands, LangGraph, custom Python agents, etc.
- **Managed everything** — no Kubernetes, no self-hosted runtime.
- **First-class identity story.** OAuth2-on-behalf-of-user with token vaults is more sophisticated than most agent platforms ship.
- **Gateway is the strongest "API → MCP tool" converter** — and is fully managed.

### Weaknesses

- AWS-locked.
- Pricing is opaque; multiple primitives compound.
- Documentation framing is enterprise-oriented; getting started is heavier than E2B/Modal.

### What Ark could borrow

- **Identity / credential vault primitive.** Ark today has no story for agent credentials. The `ark agent` CLI runs as the user; any task with the host agent runs with the user's GitHub token, API keys, etc. AgentCore's Identity primitive points at a future feature: per-task credential scoping, secrets manager integration, OAuth-on-behalf-of-user. Probably out-of-scope for an MVP but worth flagging.
- **Gateway-style API → tool converter.** Ark provides the workflow; an "expose any project tool (test runner, deploy script, etc.) as an MCP tool" helper would turn project-specific scripts into agent-callable verbs. Coupling this with the existing `ark agent` namespace makes the verb space declarative.

### Sources

- [AWS — Amazon Bedrock AgentCore](https://aws.amazon.com/bedrock/agentcore/)
- [AWS — AgentCore FAQs](https://aws.amazon.com/bedrock/agentcore/faqs/)
- [AWS Docs — Host agent or tools with AgentCore Runtime](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/agents-tools-runtime.html)
- [AWS Docs — Runtime How It Works](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/runtime-how-it-works.html)
- [AgentCore Starter Toolkit — Welcome](https://aws.github.io/bedrock-agentcore-starter-toolkit/index.html)
- [AgentCore Starter Toolkit — Runtime Overview](https://aws.github.io/bedrock-agentcore-starter-toolkit/user-guide/runtime/overview.html)
- [AgentCore Starter Toolkit — Identity API Reference](https://aws.github.io/bedrock-agentcore-starter-toolkit/api-reference/identity.html)
- [GitHub — aws/bedrock-agentcore-sdk-python](https://github.com/aws/bedrock-agentcore-sdk-python)
- [DEV.to — Amazon Bedrock AgentCore Runtime Part 1](https://dev.to/aws-heroes/amazon-bedrock-agentcore-runtime-part-1-introduction-e5i)
- [DEV.to — Amazon Bedrock AgentCore Gateway Part 1](https://dev.to/aws-heroes/amazon-bedrock-agentcore-gateway-part-1-introduction-1pjl)
- [DEV.to — Amazon Bedrock AgentCore Identity Part 1](https://dev.to/aws-heroes/amazon-bedrock-agentcore-identity-part-1-introduction-and-overview-di1)
- [Joud W. Awad on Medium — AWS Bedrock AgentCore Deep Dive](https://joudwawad.medium.com/aws-bedrock-agentcore-deep-dive-6822e4071774)

---

## Cloudflare Agents

### Identity

- **URL:** https://developers.cloudflare.com/agents — repo at https://github.com/cloudflare/agents
- **License:** Cloudflare SDK / docs open under Apache-2.0; runs on Cloudflare Workers (paid platform).
- **Maintainer:** Cloudflare, Inc.
- **Momentum (as of 2026-05):** Agents Week 2026 (April 14–20) shipped 20+ features. Persistent + stateful execution backed by Durable Objects. Hibernation by default. Voice pipeline experimental. Strongest "agent as a long-lived stateful runtime" framing in the field.

### What it provides

- **Persistent stateful execution environments** — each agent is a Durable Object with its own state, storage, lifecycle. Hibernates when idle; wakes on demand.
- **MCP-native** — `McpAgent` class gives you a Durable Object per session with state and elicitation support; `createMcpHandler` for stateless servers.
- **WebSockets Hibernation** — agents only consume compute when actively processing; preserved state across hibernation cycles.
- **Cost model** — millions of agents, one per user/session/game-room, cost nothing when inactive.
- **Workflows v2** — durable execution engine for multi-step apps; integrates with `AgentWorkflow` for bidirectional agent communication.
- **Durable Object Facets (2026)** — dynamic workers can instantiate Durable Objects with their own SQLite databases. "AI can now generate not just executable code, but also the persistent storage logic for an application."
- **Voice pipeline** — STT + TTS in ~30 lines of server code.

### Strengths

- **Stateful agents as a first-class primitive** with hibernation, scaling-to-zero, and SQLite per-agent. Few competitors match this shape.
- **MCP is a first-class object**, not a sidecar.
- **Edge-deployed** — global low-latency execution.
- **Cost story** is novel: pay only when an agent is doing work, even though "you can run millions of them."

### Weaknesses

- Vendor-locked to Cloudflare.
- Worker runtime constraints (no full POSIX, JS-first).
- Less suited for "run untrusted Python ML code" (use E2B / Modal there).

### What Ark could borrow

- **Stateful, hibernating long-running agents.** Ark's tasks today are *durable but not active* — when a task is in EXECUTE, nothing happens until a user types a slash command. Cloudflare's model — a task is an always-alive (or always-hibernating) state machine that wakes on event — would make `ark` a daemon, not just a CLI. Likely overkill for Ark's current scope, but the right shape for "task as long-running agent worker."
- **MCP-native phase transitions.** `ark agent task plan|execute|verify` could be exposed as MCP tools to the host agent (the same direction noted in Roo). Cloudflare's `McpAgent` pattern (state lives in a Durable Object; tools defined in `init()`) is a clean reference for "stateful MCP server."
- **Per-task SQLite database.** Cloudflare's Durable Object Facets give each agent its own SQLite store. If Ark's tasks gain real per-task state (checkpoints, observability records, per-task secrets), a per-task SQLite (e.g. inside `.ark/tasks/<slug>/state.db`) is the right shape. The current `task.toml` is plain text + key/value; SQLite would enable richer queries (`ark context` over task state) without changing the on-disk philosophy.

### Sources

- [Cloudflare Docs — Agents Overview](https://developers.cloudflare.com/agents/)
- [Cloudflare Docs — McpAgent API Reference](https://developers.cloudflare.com/agents/model-context-protocol/mcp-agent-api/)
- [Cloudflare Docs — McpAgent (API Reference, alternate path)](https://developers.cloudflare.com/agents/api-reference/mcp-agent-api/)
- [Cloudflare Docs — Tools (MCP)](https://developers.cloudflare.com/agents/model-context-protocol/tools/)
- [Cloudflare Docs — Build a Remote MCP server](https://developers.cloudflare.com/agents/guides/remote-mcp-server/)
- [Cloudflare Docs — createMcpHandler](https://developers.cloudflare.com/agents/api-reference/mcp-handler-api/)
- [Cloudflare Docs — Build a Durable AI Agent (Workflows)](https://developers.cloudflare.com/workflows/get-started/durable-agents/)
- [Cloudflare Docs — What are Durable Objects?](https://developers.cloudflare.com/durable-objects/concepts/what-are-durable-objects/)
- [Cloudflare Docs — Durable Objects Release Notes](https://developers.cloudflare.com/durable-objects/release-notes/)
- [GitHub — cloudflare/agents](https://github.com/cloudflare/agents)
- [Cloudflare — Agents Week 2026 Updates and Announcements](https://www.cloudflare.com/agents-week/updates/)
- [Cloudflare Blog — Durable Object Facets / Dynamic Workers](https://blog.cloudflare.com/durable-object-facets-dynamic-workers/)
- [Cloudflare Blog — Piecing together the Agent puzzle: MCP, AuthN/AuthZ, Durable Objects free tier](https://blog.cloudflare.com/building-ai-agents-with-mcp-authn-authz-and-durable-objects/)
- [Softprom — Cloudflare Agents Week 2026: 20+ New Features for AI Agents](https://softprom.com/cloudflare-agents-week-2026-20-new-features-for-ai-agents)

---

## Cross-platform takeaways

Six platforms, different layers, recurring themes:

1. **Sandboxing as a primitive** is settled — Firecracker microVM (E2B), gVisor container (Modal), WebContainer (StackBlitz), Durable Object (Cloudflare). Ark should *use* one of these, not build one.
2. **Identity / credentials** are the next frontier (AgentCore) — Ark's current "agent runs as user" story will eventually need scoping.
3. **Snapshot + resume** is becoming table stakes (Daytona, E2B) — Ark's `.ark.db` snapshot is the right shape; extending to per-task lifecycle is a natural next step.
4. **MCP as the universal extension protocol** is now assumed across all six — none of these platforms invented a new protocol; they all picked MCP.
5. **Governance separate from execution** (Coder) is enterprise table-stakes; relevant if Ark targets teams.
6. **Stateful, hibernating long-running agents** (Cloudflare) is the most novel paradigm — and the furthest from Ark's current CLI-only shape. Worth watching, not yet adopting.

The thread tying them all: **agent infrastructure is converging on a small set of primitives** (sandbox, memory, identity, gateway, runtime). Ark sits *above* this layer as workflow middleware and should avoid becoming a sandbox / memory store / identity provider itself. The right Ark stance is "any of these are pluggable backends."
