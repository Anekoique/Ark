# Devin & Cognition Labs

## Identity

- **Name:** Devin (the product), Cognition Labs (the company; sometimes referred to as Cognition AI)
- **Repos (Cognition's open-source releases):**
  - https://github.com/CognitionAI/blockdiff — VM disk snapshot file format (Rust)
  - https://github.com/CognitionAI — org page; small handful of public repos
- **License (Devin):** Proprietary, commercial SaaS. Blockdiff is open source under permissive license.
- **Primary maintainer:** Cognition Labs (founded 2023; competitive-programming-gold-medalist team)
- **Momentum (as of 2026-05):** Cognition self-reports merging 659 Devin-authored PRs into their own codebase in a single 2026 week, up from 154 in their best 2025 week. ACU (Agentic Computing Unit) pricing model; significant enterprise revenue. Series-A through Series-C funding rounds.
- **Homepages:** https://devin.ai, https://cognition.ai/blog

> **Note on source quality:** Devin is closed-source. The information here comes from (a) Cognition's own engineering blog (cognition.ai/blog), (b) Devin docs (docs.devin.ai), and (c) third-party reviews. Some claims about internals are inferential. Where speculation is needed it's marked **(inferred)**.

## Positioning

Devin is the **commercial autonomous agent benchmark.** Pitched as "the AI software engineer" — you assign a Linear ticket, JIRA issue, or chat instruction, and Devin returns a PR. Distinguishing choices vs. the OSS field:

1. **VM-per-session.** Each Devin session runs in its own sandboxed VM with shell, editor, browser, and persistent FS.
2. **ACU-billed pricing.** "Agentic Computing Units" = ~15 minutes of active autonomous work; the user buys compute, not seats.
3. **Knowledge layer (Playbooks + Knowledge Base + Machine Snapshots).** Devin learns from prior sessions; playbooks capture recurring procedures; machine snapshots persist environment state.
4. **Multi-Devin orchestration.** Devin can manage other Devins, dispatched in parallel.
5. **Async, manager-style UX.** Devin works while you do other things; comes back with PRs.

The lesson for OSS peers (Ark included): Devin's product surface is **management of autonomous agents** more than **chatting with an agent.** That's a different UX game than the CLI peers play.

## Primitives (from public docs + reviews)

User-facing nouns:

- **Session** — one task = one VM = one ACU budget. Sessions can be archived; archiving a working session puts it (and child sessions) to sleep.
- **Devin** — the agent itself; you can run many in parallel ("Managed Devins").
- **Workspace** — the persistent disk image; resets to a machine snapshot at session start.
- **Machine Snapshot** — saved VM state (installed software, cloned repos, auth tokens, files on disk) used as session-start basis.
- **Playbook** — custom-system-prompt-style recipe for a repeated task. Devin can author and improve its own playbooks based on past sessions.
- **Knowledge Base** — searchable corpus of past sessions + user-curated notes.
- **ACU (Agentic Computing Unit)** — billing unit. 1 ACU ≈ 15 minutes of active work.
- **Tags** — session metadata for organization.

User-facing verbs:

- Submit task via web UI, Slack, Linear, or API
- Watch the session timeline (commands, file diffs, browser actions)
- Roll back to any prior timeline point (restores both files and memory state)
- Manage playbooks, knowledge base
- Cap ACU usage per session

## Workflow model

Representative flow:

1. **Submit task.** A Linear ticket gets `@Devin` tagged, or a user files a task in app.devin.ai with a prompt.
2. **Devin provisions a VM** from the configured machine snapshot (instant via blockdiff — see below).
3. **Devin plans.** Breaks the task into steps. May ask the user clarifying questions in the timeline.
4. **Devin executes.** Inside the VM: clones repo, installs deps, browses docs, edits files, runs tests. The browser is real — Devin reads StackOverflow, GitHub issues, framework docs.
5. **Devin opens a PR** when done.
6. **Async review:** the user reviews via GitHub like any other PR. If revisions needed, comment on the PR; Devin reads and iterates.
7. **Optional:** Devin extracts a playbook from the finished session if the same procedure is likely to recur.

The workflow innovation is **async, manager-style.** The user is not at a chat REPL; the user is reviewing PRs that arrive in their inbox.

## Context & memory

**Per-session:**

- VM provides persistent FS during the session.
- Browser provides live web context (docs, examples).
- Codebase is indexed at machine-snapshot creation time.

**Cross-session:**

- **Machine Snapshots** persist the environment (cloned repos, auth, installed tools).
- **Playbooks** persist procedures.
- **Knowledge Base** persists facts (project conventions, gotchas, decisions).
- **Codebase indexing** — Devin builds and maintains an index of the repo it's onboarded to.

This is the most **persistence-rich** memory architecture in the prior-art space, exceeding even Cline's Memory Bank.

## Tool / capability surface

**Within the VM:**

- Shell (full Linux env)
- Code editor (custom; not VS Code)
- Browser (Chromium-based; can navigate, click, fill forms, screenshot)
- File system (persistent across the session; reset to snapshot between sessions)
- Network (full unless restricted by enterprise config)

**MCP support:** Devin connects to user systems via integrations (Slack, Linear, Jira, GitHub) rather than via MCP — they predated MCP and the closed product hasn't pivoted to MCP as the universal integration layer (as of 2026-05; **inferred** from product positioning).

**Plugin model:** Playbooks + Knowledge Base. Closed product; no SDK for adding tools.

**Sandbox boundaries:** Full VM isolation. Stronger than Codex CLI's Seatbelt/Landlock; stronger than OpenHands' Docker. This is the security-posture upper bound for the field.

## Integration model

**Cloud-only.** No CLI to install, no local agent. Surfaces:

- Web app (app.devin.ai)
- Slack (`@Devin` in a channel)
- Linear, Jira, GitHub integrations
- REST API for programmatic dispatch

## Multi-agent / orchestration

**"Devin can manage Devins"** (Cognition blog post title, 2025). The orchestrator-Devin spawns sub-Devins for parallel work, each with its own VM and ACU budget. The orchestrator monitors progress, aggregates results, and surfaces failures.

This is the most operationally-mature multi-agent system in the field (it has to be, because each Devin is a literal VM, not a process).

Cognition writes about an internal "orchestration layer" that "took over three quarters of dedicated engineering to build and can manage thousands of concurrent VMs — handling provisioning, demand prediction, crash recovery, and teardown."

## Spec / artifact system

- **Playbooks** ≈ Codex/Goose/Claude Code skills, but auto-generatable.
- **Knowledge Base** ≈ Cline's Memory Bank, but searchable and structured.
- **Session timelines** are the equivalent of OpenHands trajectories — every command, diff, browser action recorded.
- **No PRD/PLAN/VERIFY phases.** The task description is the PRD; the resulting PR is the VERIFY artifact.

## Notable open-source contribution: Blockdiff

Cognition open-sourced **blockdiff** (https://github.com/CognitionAI/blockdiff) in mid-2025 to solve the VM-snapshot-speed problem:

- AWS EC2 snapshots took 30+ minutes; Devin needs snapshots per session for rapid VM provisioning.
- Blockdiff stores only the blocks of file B that differ from file A.
- Implementation: a few hundred lines of Rust over the Linux XFS filesystem's CoW operations.
- Result: 20 GB disk snapshot drops from 30 minutes to ~200 ms — a 200× speedup.

This is one of the most directly useful technical artifacts to come out of Cognition for the broader ecosystem. Any agent platform that needs cheap VM-state snapshots (OpenHands, future ArkOS, …) can adopt blockdiff.

## Strengths

- **Full VM isolation by default.** Strongest security posture in the field.
- **Persistence-rich.** Machine snapshots + playbooks + knowledge base = real cross-session memory.
- **Multi-Devin orchestration.** Production-quality, scaled to thousands of concurrent VMs.
- **Manager-style UX.** Async, PR-mediated review — fits real engineering teams.
- **Blockdiff.** Open-sourced, generally useful, well-engineered.
- **First-mover commercial credibility.** "The AI software engineer" framing has held.

## Weaknesses / gaps (for the OSS audience to learn from)

- **Closed source.** Internals are inferred.
- **Cloud-only.** No local mode for sensitive work.
- **VM-per-session is expensive.** Hence ACU billing — but this makes "small task" Devin runs uneconomical.
- **Less transparent.** When Devin gets stuck, debugging is hard.
- **Async UX has fatigue too.** Reviews accumulate; not all teams want PR-based-only interaction with the agent.

## Directions for Ark

Even though Devin is closed, multiple lessons port:

1. **Async / manager-style UX is a long-term direction for Ark.** Today Ark assumes interactive chat-driven tier flow (PRD → PLAN → ... in a Claude Code session). The architectural ceiling, if Ark grows into ArkOS, is "user submits a task, Ark dispatches to agents, Ark surfaces PRs/PR-equivalents asynchronously." Devin shows this is a viable UX. Not a near-term task; worth a design memo.
2. **Persistent environment as a first-class concept.** Devin's machine-snapshot model = "user's project environment, captured, replayable, durable." Ark's `ark unload` / `ark load` already snapshots `.ark/` state into a portable `.ark.db`. The next step would be optionally snapshotting *the project workspace* too (relevant when Ark is the entry point for a task running in a remote sandbox). Out of scope for Phase 0; worth scoping.
3. **Adopt blockdiff if Ark ever ships sandboxed runtimes.** The instant-snapshot capability is the unblocker for "spawn a clean sandbox per task" UX without VM-startup delay. Drop-in Rust crate, MIT-style license. No need to reimplement.
4. **Playbook = auto-extracted skill.** Devin generates playbooks from finished sessions. Ark already extracts feature SPECs from finished deep tasks (`detachable-feature-spec`). The conceptual analog is real. Consider whether Ark should *also* auto-extract a procedural-skill artifact (a SKILL.md fragment describing "the steps this task took") in addition to the static feature SPEC. This would feed the persistent-memory direction.
5. **ACU-style compute metering for `ark agent`.** Devin's ACU bills compute, not seats. For Ark's own usage today this is irrelevant. But for the trajectory: when subagents run in parallel under `ark agent`, knowing "this task spent 4 hours of agent compute, mostly in the REVIEW loop" is useful product telemetry. A `ark context --scope billing` projection (read-only, just exposes elapsed-time per phase) could enable this without changing the operational model.
6. **Counter-positioning: Ark is for the *open* workflow.** Devin's pitch is "submit a task, get a PR." Ark's pitch is "open-source workflow plumbing so any agent (Claude Code, Codex, ...) can do real engineering with the discipline of PRD/PLAN/REVIEW/VERIFY." Ark + Claude Code or Ark + Codex can match Devin's deliverable (a reviewed PR) at a fraction of the lock-in, with full source visibility.

## Sources

- [Devin Docs — Release notes 2026](https://docs.devin.ai/release-notes/2026)
- [Devin Docs — Advanced Capabilities](https://docs.devin.ai/work-with-devin/advanced-capabilities) — playbooks, knowledge base, machine snapshots
- [Devin Docs — Classic configuration / Repo setup](https://docs.devin.ai/onboard-devin/repo-setup)
- [Cognition Blog — Devin can now Manage Devins](https://cognition.ai/blog/devin-can-now-manage-devins)
- [Cognition Blog — How Cognition Uses Devin to Build Devin](https://cognition.ai/blog/how-cognition-uses-devin-to-build-devin)
- [Cognition Blog — Blockdiff: VM disk snapshots](https://cognition.ai/blog/blockdiff)
- [Cognition Blog — What We Learned Building Cloud Agents](https://cognition.ai/blog/what-we-learned-building-cloud-agents)
- [Cognition Blog — Devin's 2025 Performance Review](https://cognition.ai/blog/devin-annual-performance-review-2025)
- [CognitionAI/blockdiff on GitHub](https://github.com/CognitionAI/blockdiff)
- [Devin AI Guide 2026 — AI Tools DevPro](https://aitoolsdevpro.com/ai-tools/devin-guide/)
- [Devin Review 2026 — Idlen](https://www.idlen.io/blog/devin-ai-engineer-review-limits-2026/)
- [Devin Pricing — official](https://devin.ai/pricing/)
