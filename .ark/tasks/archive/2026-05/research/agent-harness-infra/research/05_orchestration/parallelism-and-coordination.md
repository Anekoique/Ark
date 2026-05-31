# Parallelism and Coordination

Running N agents at once: the isolation patterns, the coordination primitives, the failure modes.

## Why parallelise at all

Three drivers:

1. **Wall-clock latency.** A research-tier task with 8 topics runs ~3× faster if 4 children work in parallel vs. one child sequentially.
2. **Context isolation.** Each child has its own context window; parent stays small. (See `subagent-isolation-and-context.md`.)
3. **Specialisation.** Different children can use different models, prompts, tools.

The cost: coordination complexity. The harder the coordination, the less of the latency win you keep.

## Isolation primitives

### Git worktree (file-scope)

Each task gets its own working tree at `.ark/worktrees/<branch>/` (Ark), or via similar in other tools. Shared `.git/`, separate file checkout.

**Used by:** Ark (`task new --worktree`), Cursor Background Agents (up to 8), ccswarm, Codex parallel subagents, OpenHands AgentDelegateAction with worktree backend.

**Isolates:** File state, branch, build outputs (if config'd).
**Does NOT isolate:** Network, secrets in `.env`, port bindings, system caches.

**Failure modes:**
- Port collision on dev servers (two worktrees both `npm run dev` on :3000).
- `node_modules` cache thrash (npm rebuilds per worktree unless config'd).
- Secrets directory pollution (each worktree reads `.env` from parent or its own copy).

Cheap; ships in seconds; covers ~80% of the common cases.

### Container (process-scope)

OpenHands Docker runtime: each AgentDelegateAction spawns a container. Devin VM-per-session is the extreme case (VM, not container).

**Used by:** OpenHands, Devin, Replit, GitHub Actions (each agent run gets a fresh runner), AWS Bedrock AgentCore.

**Isolates:** Process tree, filesystem, network namespace (with proper config).
**Does NOT isolate:** Kernel, hardware resources.

**Cost:** Multi-second startup, image management, more infrastructure.
**Benefit:** Strong process / port / cache isolation. Can run untrusted code.

### MicroVM (kernel-scope)

E2B (Firecracker), Modal sandboxes, Cloudflare Workers Browser Rendering.

**Used by:** E2B (Firecracker, 3-8ms cold start), Modal, Devin (custom VM image), Replit (Nix-based per-session).

**Isolates:** Everything containers do + kernel.
**Cost:** Higher startup, infrastructure complexity, hypervisor dep.
**Benefit:** Untrusted code is genuinely safe to run.

### Capability tokens (per-call permission)

MCP roots (filesystem scope), Anthropic computer-use's "stop me" interrupts, Cline's tool-call approvals.

**Used by:** MCP (roots prevent file access outside scope), Cline (per-tool approval prompts), Cursor (auto-approve toggles).

**Isolates:** Per-action permission scope.
**Doesn't isolate:** Misuse within permitted scope.
**Benefit:** Fine-grained user control over what the agent can do.

## Choosing isolation

For *internal-trust* parallel runs (agent is doing what you asked, on your repo):
- Worktrees suffice. Use them.

For *cross-task isolation with cache / port sensitivity:*
- Worktrees + per-worktree `.env` copies + per-worktree-distinct dev ports.
- Tools to add: `direnv` for env, port-randomising npm scripts.

For *untrusted code:*
- Containers minimum (Docker / Podman).
- MicroVMs ideal (E2B-class).

Ark's worktree pattern hits the first case well. The second case has a known gap (port collisions; see `feature/worktree/SPEC.md` for current scope). The third case is out of Ark's scope (it doesn't run untrusted code).

## Coordination primitives

### Locks

Exclusive file lock on a shared resource. Ark uses this on `.ark/.state.toml.lock` — `state_mutate` acquires before reading-and-writing. Standard pattern, well-understood; brittle under crash (lock leak), mitigated by OS-released-on-process-exit semantics.

Bazel uses lock files. Cargo uses lock files. Almost every build system uses lock files.

### Journals / append-logs

Append-only audit trails. Ark's per-developer journals are this. Atomic appends (file open in O_APPEND on POSIX) are race-free.

Used for: post-hoc visibility into what each agent did. Not really for coordination ("did agent A finish before agent B?") — locks are better for that.

### Atomic file writes

Write to a temp file, fsync, rename. POSIX rename is atomic within the same filesystem. Ark's `state_mutate` does this: write `.state.toml.tmp.<pid>`, fsync, rename.

Pattern: when multiple agents might write the same file, only the rename is the commit. Readers see the old or new version, never a torn intermediate.

### Idempotent operations

Operations that produce the same result when run multiple times. Ark's `task plan`, `task review`, etc. are idempotent given the same starting state. Lets re-runs after failures be safe.

### Foreground await (the simplest coordination)

Parent dispatches N children, awaits all, processes results in order. No coordination needed between children if they don't share state.

This is the *default safe pattern*. Ark's research-tier parallel dispatch uses it (this corpus's 9 parallel dispatches).

## Failure modes

### Torn writes

Two agents write the same file without coordination. Last writer wins; earlier writer's content lost. Mitigation: atomic temp+rename, or locks.

Ark's `.state.toml` is protected. Per-task files (`PLAN.md`, `VERIFY.md`) are not currently lock-guarded — they rely on single-writer assumption (one focused task per checkout).

### Lost updates

Reader gets state A; computes update; writer commits as A' assuming starting state was A. But meanwhile another agent committed B. A's update overwrites B. Classic read-modify-write race.

Mitigation: optimistic concurrency (compare-and-swap on a version field) or pessimistic locking.

Ark's state file uses pessimistic locking; per-task files do not (single-writer assumption).

### Deadlock

Agent A holds lock X, wants lock Y. Agent B holds lock Y, wants lock X. Both wait forever.

Mitigation: lock ordering, timeouts. Ark has one lock; no ordering risk.

### Port collisions

Two worktrees both run `npm run dev` on :3000. One fails. Mitigation: port-randomising scripts, per-worktree environment variables.

### Secret reuse

Two parallel agents both authenticate against a rate-limited API. Hit the limit. Mitigation: per-agent credentials (Devin does this), or queueing.

### Cache thrash

Each parallel build invalidates a shared cache. Mitigation: per-worktree cache directories.

## Lessons from build systems

Bazel, Buck, Pants — large-scale parallel build systems — solved most of these problems for code generation. The lessons that transfer:

1. **Immutable artifacts.** Outputs are content-hashed; cached by hash. Two parallel builds of the same input produce the same hash; the cache is safe.
2. **Pure functions.** A build action's output is determined by its input. No environment dependence.
3. **Explicit dependencies.** Each action declares what it reads/writes. The scheduler enforces ordering.
4. **Per-action sandboxing.** Each action runs in an isolated environment (Bazel sandboxing).

Translating to agent dispatch:
1. Sub-agent output should ideally be content-hashable.
2. Sub-agent runs are *not* pure (LLMs are stochastic), but the *spec* of a run can be.
3. Sub-agents should declare what they write (Ark's C-7..C-10 do this).
4. Sub-agents already run in fresh-context sandboxes (Claude Code Task tool does this).

The mismatch: agents are stochastic. Bazel-like reproducibility is impossible. But the structural disciplines transfer.

## Ark's current parallelism story

- **Worktree-per-task** isolation. Solid.
- **Per-checkout `.state.toml`** prevents focus collision.
- **File-locked state mutations** prevent torn writes on shared state.
- **Sub-agent fresh-context dispatch** isolates child reasoning.
- **C-7..C-10 write scopes** declare what each child writes (catches scope violations after the fact).
- **C-28 git verification** lets the parent revert children's out-of-scope writes.

What's missing:
- No port-conflict mitigation (worktree dev servers).
- No declared-up-front "what files this dispatch will write" (caught post-hoc only).
- No re-dispatch-aware tracking (failed dispatch + partial state requires manual recovery).

## Directions for Ark

1. **Document the worktree port-collision pattern.** Pick a recommendation (per-worktree port files, per-worktree env files), document it in `docs/book/src/concepts/worktrees.md`. Cheap; helps every user who has run into it.

2. **Declared-write contracts in subagent dispatch.** Extend the subagent template prompts so the parent records (in `task.toml` or `.state.toml`) which files each in-flight dispatch is expected to write. Detect out-of-scope writes proactively (not just C-28 post-hoc revert).

3. **Re-dispatch-aware sub-agent template.** When `ark-researcher` is re-dispatched on an existing corpus, the template should explicitly say "list existing files, write only missing ones". Saves token cost and reduces overwrite risk.

4. **Surface in-flight dispatch state in `ark context`.** When sub-agents are running, `ark context --scope session` should include `in_flight_dispatches: [{slug, topic, started_at, expected_files}]`. Helps the user, the parent, and recovery.

5. **Capture the worktree-isolation gap explicitly.** The `worktree` feature SPEC should document what it does NOT isolate (ports, caches, secrets), so users know to address those at the project level. Currently this is implicit.
