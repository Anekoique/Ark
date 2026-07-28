# Research: Pi fit for current Ark

- Query: Evaluate Pi against Ark's current platform, command, context, subagent, workflow, worktree, sandbox, and security contracts; classify adoption paths and define a narrow validation spike.
- Scope: mixed
- Date: 2026-07-22

## Findings

### Decision summary

- **[Recommendation — prototype, not adopt now]** Do not add Pi to `PLATFORMS` as a fifth first-class target yet. Exact `/ark:<verb>` extension commands and a wholly owned project-extension subtree are source-verified fits, but first-class parity still depends on a TypeScript adapter for dynamic context and on an upstream *example* extension (or an Ark-maintained equivalent) for subagents. Pi's default `AGENTS.md` traversal also deterministically crosses Ark's nested worktree boundary. The current repository/package move and the v0.81.1 compatibility repair make that runtime coupling premature.
- **[Recommendation — adopt/continue now]** Continue using portable `AGENTS.md` instructions where a host's traversal boundary matches the checkout boundary, and Agent Skills-compatible content where it cannot activate Ark implicitly. Borrow Pi's project-trust boundary for executable project resources, explicit package pinning, and small inspectable extension pattern as integration criteria rather than adding Pi-specific behavior immediately.
- **[Recommendation — watch]** Watch for a stable, documented core subagent/agent-profile contract, a stable extension API, and a context-root/git-root stop or deduplication control. Those developments would materially reduce Ark-owned code and remove the confirmed nested-worktree conflict.
- **[Recommendation — reject]** Do not substitute Pi sessions, branches, compaction, plan-mode extensions, or tool-confirmation extensions for Ark's `task.toml`, PRD/PLAN/REVIEW/VERIFY/SPEC artifacts, human gates, git index boundary, or Docker sandbox.
- **[Inference]** Pi can likely become a first-class Ark host without changing Ark's lifecycle model, but only after a pinned adapter proves the host contract. The likely integration is medium-to-high coupling, not the registry-only addition that the current platform abstraction suggests at first glance.

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `crates/ark-core/src/platforms.rs` | Canonical platform registry, lifecycle ownership, managed instructions/hooks, agent installation, and destructive-removal boundary. |
| `crates/ark-cli/src/main.rs` | Per-platform `init`/`load` CLI flags and explicit mapping that a Pi target would extend. |
| `crates/ark-core/src/layout.rs` | Project-relative constants/getters and owned-directory derivation for all current hosts and worktrees. |
| `crates/ark-core/src/templates.rs` | Embedded per-host command, agent, config, and plugin template trees. |
| `crates/ark-core/src/commands/init.rs` | Core and platform template extraction followed by canonical managed-state application. |
| `crates/ark-core/src/commands/context/model.rs` | Stable schema-1 context payload, including the current task and platform subagent sets. |
| `crates/ark-core/src/commands/context/gather.rs` | One-pass collection of git, task, SPEC, checkout, and subagent state. |
| `crates/ark-core/src/commands/context/subagents.rs` | Registry-derived installed/user subagent enumeration; non-Codex profiles are Markdown. |
| `crates/ark-core/src/commands/agent/state.rs` | Durable tier/phase state machine and task metadata, including branch/worktree/journal fields. |
| `.ark/workflow.md` | Authoritative human gates, artifact lifecycle, commit boundary, research lifecycle, and worktree rules. |
| `.ark/specs/features/subagent-support/SPEC.md` | Three-role subagent contract, explicit dispatch policy, recursion/write walls, and post-dispatch status check. |
| `.ark/specs/features/codeagent-cli-support/SPEC.md` | Most recent first-class platform precedent and its expected registry/template/test footprint. |
| `templates/claude/commands/ark/design.md` | Concrete command behavior for researcher dispatch and user-selected review/verification. |
| `templates/claude/agents/ark-researcher.md` | Persisted-output, write-scope, recursion, and paths-only return contract. |
| `templates/opencode/plugins/ark-context.ts` | Existing TypeScript precedent for fail-open, once-per-session dynamic `ark context` injection. |
| `crates/ark-core/src/io/fs/hook.rs` | Declarative JSON hook schemas and canonical `ark context --scope session --format json` commands. |
| `crates/ark-core/src/commands/sandbox/platform_argv.rs` | Sandbox launch contract, currently limited to hosts with verified bypass/yolo flags. |
| `crates/ark-core/src/commands/sandbox/config.rs` | Sandbox credential/config sharing policy, currently Claude/Codex-specific. |
| `crates/ark-core/src/commands/sandbox/resolve.rs` | Host-config bind mounts, currently lacking Pi's home/config paths. |
| `sandbox/Dockerfile` | Release image currently installs only Claude Code and Codex CLI. |
| `reference/pi/packages/coding-agent/src/core/extensions/loader.ts` | Read-only Pi source: `registerCommand` stores the supplied name directly with no validation. |
| `reference/pi/packages/coding-agent/src/core/extensions/runner.ts` | Read-only Pi source: exact invocation-name resolution and colon-based duplicate suffixes. |
| `reference/pi/packages/coding-agent/src/core/agent-session.ts` | Read-only Pi source: slash-command parsing stops only at the first space and performs an exact extension-command lookup. |
| `reference/pi/packages/coding-agent/src/core/resource-loader.ts` | Read-only Pi source: `AGENTS.md` walks to filesystem root and deduplicates only exact paths; extension-contributed resource paths are supported. |
| `reference/pi/packages/coding-agent/src/core/package-manager.ts` | Read-only Pi source: `.pi/extensions/*/index.ts` auto-discovery and package-resource resolution. |
| `reference/pi/packages/coding-agent/src/core/prompt-templates.ts` | Read-only Pi source: native prompt directories are scanned non-recursively, while explicit prompt directories are accepted. |

### Current contracts versus Pi

Facts about Pi below refer to the current canonical `earendil-works/pi` repository and the latest observed release, v0.81.1 (2026-07-21). The source follow-up used read-only `reference/pi` HEAD `dd6bea41`; the relevant command/context/resource files have no diff from local tag v0.81.1. The former `badlogic/pi-mono` URL redirects there.

| Capability / boundary | Pi v0.81.1 fact | Ark contract | Fit and boundary | Call |
| ---- | ---- | ---- | ---- | ---- |
| Static project instructions | Pi loads the global context file and then one `AGENTS.md`/`CLAUDE.md` per directory from filesystem root through cwd. It does not stop at a git root; deduplication is exact-path only; context files load regardless of project trust. | Ark already maintains an `<!-- ARK -->` block in `AGENTS.md` for Codex, OpenCode, and CodeAgent. | The format fits, but the default discovery boundary does not: from `.ark/worktrees/<branch>`, Pi loads both the primary checkout's and worktree's distinct `AGENTS.md` files, parent first. They can duplicate or contradict as branches diverge. | **Adopt/continue** the format outside nested worktrees; **block first-class deep-tier parity** until traversal is scoped safely. |
| Explicit workflow commands | Project prompt templates map filenames to commands. Separately, `registerCommand` stores any string without name validation; dispatch extracts the exact token up to the first space. Pi itself uses colons for `skill:<name>` and duplicate-command suffixes. | Nine shipped entries use `/ark:<verb>` and explicit invocation is required to activate Ark. | `registerCommand("ark:design", ...)` is source-verified to resolve exact `/ark:design`; the same holds for all Ark verbs. Arguments are passed as the remaining string. Native prompt filenames remain unsuitable for portable colon names, and dropping the namespace would collide semantically with Pi `/resume`. | **Prototype** runtime/autocomplete/argument behavior, but command-name compatibility is resolved. |
| Skills as workflow entry | Pi advertises discovered skills to the model and may read a matching skill; users can force `/skill:name`. | Ark activation must come from an explicit skill/slash-command invocation. | An automatically selected skill can cross Ark's activation boundary. A forced skill also changes the current `/ark:<verb>` UX. | **Reject** skills as the primary Ark entry; keep them only for portable supplementary guidance. |
| Dynamic session context | Pi has no documented JSON `SessionStart` hook. TypeScript extensions receive lifecycle/resource events such as `session_start`, `before_agent_start`, `context`, and `resources_discover`, and can send messages or alter the system prompt. Project extensions execute only after project trust. | Every host receives fresh `ark context --scope session --format json` state at session orientation; OpenCode already uses a small TS bridge when no native hook exists. | Semantically feasible through a dedicated, fail-open extension. It must inject once without causing an unsolicited agent turn and must refresh correctly for new/resumed/forked/reloaded sessions. | **Prototype** a pinned minimal bridge; do not infer parity from event names alone. |
| Subagent profiles | Core Pi does not document declarative `.pi/agents` discovery. The official subagent *example extension* discovers Markdown profiles, prompts for project-agent use, and launches a separate `pi --mode json -p --no-session` process. | Every verified host ships exactly `ark-researcher`, `ark-reviewer`, and `ark-verifier`; main-session choice, recursion guards, per-role write walls, and read-only reviewer/verifier roles are mandatory. | Agent Markdown can carry Ark's prompt contract, and child processes can inherit the worktree cwd. Files alone are inert without the example or an Ark adapter. Depending on an example or vendoring a runner is the largest coupling point. | **Prototype** only. First-class status requires the full three-role contract and post-dispatch status audit, not merely profile discovery. |
| Task state | Pi sessions are JSONL trees with continue/resume/fork/branch/compaction semantics. | `task.toml` carries tier, legal phase, branch/worktree/base, timestamps, and journal state; the CLI enforces transitions. | Orthogonal. Pi history is conversational state and may coexist, but cannot authorize or represent an Ark transition. | **Reject** substitution; **adopt** coexistence only. |
| Structured artifacts and gates | Pi extensions can add plan mode, commands, confirmation UI, and custom tools. Those are optional runtime features. | PRD, PLAN, REVIEW, VERIFY, research, SPECs, user staging, and explicit confirmation are persisted artifacts and load-bearing gates. | Prompts/extensions can guide the same commands, but Pi UI or mode state cannot replace artifact checks. Non-interactive Pi modes also cannot supply all interactive UI methods. | **Reject** replacement; require Ark CLI/artifacts as authority. |
| Git boundary | Pi ships `bash` and expects ordinary git/checkpoint use; extensions have process access. | The user stages work; Ark commits only after the gate and stages only Ark-managed artifacts itself. | No conflict if Pi only drives Ark's existing commands. An extension that stages/commits autonomously would violate the boundary. | **Adopt** existing Ark behavior; **reject** auto-stage/auto-commit helpers. |
| Worktree cwd | Pi and its example child runner can operate with a chosen cwd, but default context discovery walks from that cwd to filesystem root. | Deep tasks must run in `.ark/worktrees/<branch>/`; worktree creation/cleanup is deliberate Ark state. | Child cwd is compatible; static context is not. The source guarantees both checkout paths are loaded when both contain context files because they are distinct paths. | **Prototype a mitigation**, not the default behavior; deep-tier support cannot claim parity while both contexts remain. |
| Project trust | Pi gates project extensions, prompts, skills, themes, and package installation on project trust. Extensions still have full system access once trusted. | Ark templates are local project artifacts; executable hooks/plugins already exist for current hosts. | Useful defense against drive-by project code, but not isolation. Ark must keep generated executable resources reserved/inspectable and pin any dependency. | **Adopt/continue** trust as a criterion; do not market it as a sandbox. |
| Tool permissions | Pi intentionally has no built-in permission system; it delegates isolation to OS/container boundaries. Confirmation extensions are policy/UI. | `ark sandbox` is a Docker worktree environment; current in-box agent launching assumes a verified bypass/yolo argument. | Philosophically compatible with Ark's separate container boundary, but technically unsupported today. Pi has no yolo flag to fit the current helper's premise. | **Watch/prototype later** after the host adapter passes. |
| SDK/RPC embedding | Pi exposes an SDK and JSON RPC mode, including custom resources, events, tools, and UI request messages. | Ark is a CLI workflow harness; the platform layer currently installs host-native files rather than embedding agent runtimes. | Useful for a future agent runtime/ArkOS investigation, but excessive coupling for Ark host support. | **Watch**; **reject** as the first integration route. |

### Exact first-class surface map

#### Commands

- **[Fact]** Ark currently ships nine host commands: `quick`, `design`, `commit`, `discard`, `record`, `research`, `resume`, `spec-audit`, and `spec-extract` under the `ark` namespace.
- **[Fact]** Pi prompt-template discovery is non-recursive under `.pi/prompts`, and a template's filename becomes its slash command.
- **[Inference]** Copying nine files as `.pi/prompts/ark-*.md` is the smallest behavioral proof, but it exposes `/ark-design` rather than Ark's current `/ark:design` convention. Top-level names such as `/resume` must not be used because Pi has its own session-resume semantics.
- **[Fact — source verified]** `registerCommand` performs no character validation: it stores the supplied string as a `Map` key. The dispatcher removes the leading slash, splits only on the first space, and performs an exact lookup. Therefore `registerCommand("ark:design", ...)` is invoked by `/ark:design`; its handler receives everything after the first space as `args`. Colon names are also exercised by Pi's own `skill:<name>` commands and duplicate-command suffixes.
- **[Inference]** A Pi adapter can preserve all nine public Ark names by registering extension commands and keeping the command Markdown as private data beneath the extension root. A runtime smoke is still warranted for autocomplete, conflicts, and byte-preserving argument handling, but naming support is no longer an open question.

#### Agents

- **[Fact]** Pi's official subagent example accepts Markdown agent profiles with `name`, `description`, `tools`, and optional `model`, and supports single, parallel, and chained child runs.
- **[Fact]** Project `.pi/agents/*.md` profiles are consumed by that example extension, not documented as a built-in Pi core surface. Project-agent scope is explicit and confirmation is enabled by default.
- **[Inference]** Ark's three Markdown prompt bodies can be adapted without changing their semantic contract, but the runtime must also enforce/mainline all of these behaviors:
  - only the main session dispatches the three roles;
  - researcher output persists only under the active task's `research/` directory;
  - reviewer/verifier remain read-only except for their seeded report;
  - recursion remains disabled;
  - child cwd is the active checkout/worktree;
  - results return as the expected report/path contract;
  - the main session checks `git status` and rejects out-of-scope writes.
- **[Inference]** Pi confirmations around project-controlled agent profiles align with caution but do not replace Ark's explicit “self-review or reviewer?” and “self-verify or verifier?” gates.

#### Hooks and context

- **[Fact]** Pi extensions are executable TypeScript modules with broad process access. The lifecycle API is richer than a JSON startup hook, but extension exceptions normally log and allow the agent to continue.
- **[Inference]** The closest current Ark precedent is `templates/opencode/plugins/ark-context.ts`: run `ark context`, inject once for the session, warn and fail open. A Pi proof should use either `session_start` plus a queued context message or the first `before_agent_start`; it must not call a mode that triggers an extra model turn.
- **[Inference]** Context injection should remain a thin transport for Ark's schema-1 payload. Reconstructing task/spec/git state inside TypeScript would duplicate the authoritative Rust gatherer and create drift.

#### Lifecycle ownership

- **[Fact]** Ark's `Platform.removal_root` is removed wholesale. Shared `extra_dirs` receive selective cleanup only for known agent-template files.
- **[Fact]** Pi's native project prompt directory `.pi/prompts` is shared with user content and prompt discovery is non-recursive.
- **[Fact — source verified]** Pi auto-discovers `.pi/extensions/*/index.ts`. A subdirectory extension may contain helper/data files, and `resources_discover` may contribute explicit prompt/skill directories after startup. Explicit prompt directories are scanned non-recursively.
- **[Inference]** A straightforward `dest_dir = removal_root = ".pi/prompts"` remains unsafe because `ark remove` would delete unrelated user prompts. Native flat resources and current Ark ownership do not compose safely.
- **[Verified option A — fits the current `Platform` ownership fields]** Use one wholly owned `.pi/extensions/ark/` subtree: `index.ts` is the only auto-discovered entry, while nested command Markdown and three agent profiles are private adapter data. The extension registers exact `ark:<verb>` commands, performs context injection, and loads its own profiles. `dest_dir` and `removal_root` can both be the dedicated subtree, so sibling `.pi/extensions/*` and `.pi/prompts/*` survive removal. The existing `AGENTS.md` managed block remains separate.
- **[Option B — requires lifecycle support for shared files]** Install native flat `.pi/prompts/ark-*.md` files and teach Ark to capture/remove only reserved files in a shared directory. This keeps more behavior declarative but changes the current `removal_root`/`extra_dirs` cleanup contract and still exposes Pi-specific hyphenated command names unless an extension aliases them.
- **[Option C — requires a shared-settings merge]** Store an Ark Pi package in a dedicated directory with a `package.json` `pi` manifest and register its local path in `.pi/settings.json`. Pi supports this, but Ark would need surgical ownership/capture/removal of one entry in a user-shared JSON file. It also adds package reconciliation behavior that option A does not require.

Option A resolves filesystem ownership with the current platform abstraction, but concentrates commands, context, and subagents in a Pi API adapter. Options B and C reduce different parts of that adapter at the cost of new lifecycle/settings coupling. No production option is selected in this research task.

### Conflict analysis

#### Human gates and persistent state

- **[Fact]** Ark's standard/deep flows require persisted plan/verification state; deep REVIEW and standard/deep VERIFY explicitly ask the user whether to self-audit or dispatch the corresponding agent. Commit generation requires showing a message and asking before invoking the CLI, while staging remains the user's step.
- **[Inference]** Pi's autonomous command/tool composition does not inherently conflict. Conflict begins only if an adapter treats a Pi mode, confirmation dialog, session branch, or agent completion as authorization to advance Ark state.
- **Required boundary:** every phase transition continues through `ark agent task ...`; Pi is transport and presentation only.

#### Structured artifacts

- **[Fact]** Ark research, reviews, verification, and feature SPEC extraction persist in repository files and survive session boundaries. Pi sessions persist model/tool conversation events and support branching/compaction.
- **[Inference]** Session compaction can discard conversational detail without harming Ark when the artifacts are authoritative. Reversing that authority would make compaction and client-specific session storage workflow-critical.

#### Worktrees

- **[Fact]** Ark requires deep-tier work inside `.ark/worktrees/<branch>/` and stores worktree/branch/base metadata in `task.toml`.
- **[Fact]** Pi's example subagent spawns a child CLI process and can pass a cwd.
- **[Inference]** Child isolation by process is sufficient for conversational separation, but not for filesystem isolation. Ark's role prompts and post-dispatch `git status` check remain necessary.
- **[Fact — confirmed conflict]** `loadProjectContextFiles` starts at resolved cwd, calls `dirname` until filesystem root, and deduplicates only identical absolute paths. It prepends each ancestor so broader-parent instructions appear before nearer instructions. From Ark's nested worktree, the primary checkout's `AGENTS.md` and the worktree's `AGENTS.md` are different paths, so both load. If their bytes match, rules and token cost duplicate; if branches differ, the main checkout can inject stale or contradictory instructions into the worktree session.
- **[Mitigation options, none currently native and selected]** (A) an Ark-controlled Pi launch can use `--no-context-files` and have the trusted adapter inject only explicitly selected current-checkout instructions; (B) Ark can place worktrees outside the primary checkout ancestry; or (C) Pi can gain a context-root/git-root stop or canonical-content dedupe option. A normal `pi` launch with current nested worktrees has no setting that scopes only `AGENTS.md` traversal while retaining the worktree file.

#### Sandbox and permissions

- **[Fact]** `ark sandbox enter --agent` currently recognizes only Claude's `--dangerously-skip-permissions` and Codex's `--yolo`; other platforms return `AgentYoloUnsupported`. The release image installs only those two CLIs.
- **[Fact]** Pi has no built-in permissions sandbox. It recommends using OS/container isolation, while project extensions and skills can execute arbitrary actions once trusted.
- **[Inference]** First-class Pi support outside the sandbox does not automatically imply sandbox parity. Full parity adds:
  - a pinned `@earendil-works/pi-coding-agent` install in the release image;
  - launch semantics that do not pretend a nonexistent yolo flag is a safety contract;
  - explicit handling of Pi's `~/.pi/agent` configuration/authentication and relevant provider environment variables;
  - `share_host_config` rules or a deliberate isolated-volume-only policy;
  - startup-network/update/telemetry policy, including evaluation of Pi's offline option.
- **[Inference]** The existing named sandbox HOME volume should preserve Pi state in isolated mode, but that does not solve host credential import. Ark's sandbox network is not a confinement boundary, so Pi extensions still retain network/process power inside the container.

### Size, coupling, stability, maintenance, and security

| Area | Minimum touchpoints | Relative cost | Coupling / risk |
| ---- | ---- | ---- | ---- |
| Registry and CLI selection | Pi layout/template constants, one `Platform`, `PLATFORMS`, two positive/negative CLI flags, flag mapping, registry tests | Small | Low if resources have a safe wholly owned root; otherwise ownership semantics expand. |
| Static instructions | Existing `AGENTS.md` managed block plus a nested-worktree traversal mitigation | Small outside worktrees; unresolved for deep-tier parity | Pi loads the parent checkout and worktree files by construction; Ark's managed-block dedupe does not dedupe two filesystem paths at Pi runtime. |
| Nine commands | Nine Markdown bodies as adapter data plus nine `registerCommand` calls and argument/autocomplete tests | Medium | Exact colon namespace is supported. Coupling is to the extension API rather than native prompt filenames. |
| Dynamic context | One fail-open TS extension plus lifecycle-mode tests | Medium | Medium; analogous to OpenCode, but Pi events across resume/fork/reload/non-interactive modes must remain stable. |
| Three Ark agents | Profiles plus a child-process/tool adapter, scope/confirmation/result/cwd tests | Medium-high to high | Highest ongoing coupling. Current upstream implementation is an example, not a core declared contract. |
| Lifecycle round-trip | init/load/unload/remove/upgrade and user-sibling preservation tests | Small-to-medium with a dedicated extension subtree; medium otherwise | A wholly owned `.pi/extensions/ark/` fits current removal semantics; flat native resources remain high-risk without selective cleanup. |
| Sandbox parity | Image package, argv model, config/auth/env mounts, smoke tests | Medium | Security-sensitive and release-coupled; Pi supports many providers and lacks a yolo permission mode. |
| Upstream tracking | Pin/version policy, compatibility CI, namespace migration handling | Ongoing | Medium-high today: current release is 0.x, canonical repo/package namespace changed, and v0.81.1 explicitly restores compatibility for pre-0.81 extension behavior. |

Security conclusions:

- **[Fact]** Pi project trust prevents automatic loading/installing of protected project resources before trust, but explicitly does not gate `AGENTS.md`/`CLAUDE.md`; extensions have full system permissions and skills can instruct arbitrary actions.
- **[Recommendation]** If prototyped, pin the exact Pi package/revision, keep the adapter dependency-free where practical, and do not rely on project settings that auto-install unpinned packages.
- **[Recommendation]** Treat Ark-generated Pi extension/agent stems as reserved, keep the files inspectable, and reapply their canonical content just as current Ark agent templates do.
- **[Recommendation]** Preserve OS/container isolation as the actual security boundary; neither project trust nor a tool-confirmation extension is equivalent.

### Adoption paths

#### Adopt/continue now — patterns independent of Pi support

1. `AGENTS.md` as the common static project-instruction surface where the host's traversal boundary matches the checkout boundary; Pi's default traversal is not safe for Ark's nested worktrees.
2. Agent Skills-compatible reusable guidance only where model-selected activation cannot mutate Ark lifecycle state.
3. Trust-before-executable-project-resources as a host acceptance criterion.
4. Pinned, inspectable, fail-open adapters that transport `ark context` rather than reimplement it.
5. Distinguish conversational sessions from durable workflow state and container boundaries from permission UI.

#### Prototype — narrow host-contract adapter

Prototype Pi v0.81.1 in a disposable fixture; do not register it as a production platform yet. The proof should contain only:

- two representative exact extension commands (`ark:design` for arguments/context and `ark:commit` for a hard human gate), plus autocomplete/conflict registration for all nine reserved names;
- one minimal dynamic-context extension;
- the three canonical Ark profiles exercised through the unmodified upstream subagent example first, with a thin proof adapter only if the example cannot satisfy the contract;
- one normal checkout and one nested Ark worktree fixture;
- no SDK embedding, no RPC controller, no package auto-install, and no sandbox-image change in the first stage.

Validation matrix:

| Probe | Pass condition | Stop condition |
| ---- | ---- | ---- |
| Command dispatch | All nine exact `/ark:<verb>` names appear in autocomplete and invoke the intended handler; arguments arrive byte-for-byte; no other extension causes numeric collision suffixes. | A target release diverges from the source-verified exact-name behavior or a collision changes an Ark invocation name. |
| Explicit activation | Merely mentioning a workflow does not start it; only the invoked command runs Ark instructions. | A discovered skill/extension starts a lifecycle without explicit invocation. |
| Dynamic context | Fresh schema-1 context appears exactly once on new, resumed, forked, and reloaded sessions; no extra model turn; absent/failing `ark` produces one concise warning and continues. | Duplicate/stale context, hidden failure, or unsolicited model execution. |
| Non-interactive mode | `--mode json -p` either receives equivalent context or is explicitly unsupported without hanging on UI. | Extension waits on unavailable interactive UI. |
| Three roles | All three profiles are discoverable; main session chooses dispatch; child cwd/model/tools/result are bounded; recursion is disabled. | Requires a substantial fork/vendor of the upstream example or cannot express Ark's roles. |
| Write wall | Researcher writes only its assigned research file; reviewer/verifier only their seeded report; main detects any deviation with `git status`. | Child runtime obscures writes or cannot retain role-specific scope. |
| Worktree mitigation | With default discovery treated as a known failure, the selected mitigation gives main and child the worktree's active task and exactly the intended instruction sources without parent-checkout content. | Support requires silently accepting duplicate/stale parent instructions or globally suppressing context that cannot be restored safely. |
| Resource trust | Project prompt/extension/agent resources are inert before trust and visibly activated after trust. | Executable project code runs before trust. |
| Lifecycle cleanup | User-created `.pi` siblings survive init/unload/load/upgrade/remove byte-for-byte. | Safe cleanup requires owning/deleting a shared Pi directory. |

Promotion threshold: all probes pass against one exact pinned release, the subagent implementation remains a small adapter rather than a fork, and compatibility can be exercised in CI. Only then is a second-stage sandbox spike warranted.

#### Watch

- A core/documented project-agent profile loader and child-agent API, rather than an example extension.
- Extension API stability across minor releases and the post-rename package/repository maintenance trajectory.
- A context-root/git-root stop, exclude rule, or canonical deduplication control for context files.
- Pi's offline/update/telemetry and credential-storage contracts for container deployment.

#### Reject

- Treating Pi `/resume`, `/fork`, or session branching as `ark agent task resume` or task/worktree state.
- Advancing phases because a Pi plan/agent/tool completed instead of because Ark's CLI gate passed.
- Automatic review/verification dispatch or reviewer/verifier self-fix.
- Automatic git staging/commit outside Ark's documented user confirmation and scoped commit path.
- Calling a confirmation extension, project trust, or Pi's lack of permissions an Ark sandbox.
- Deleting `.pi`, `.pi/prompts`, `.pi/extensions`, or another shared directory as Ark's `removal_root`.
- Auto-installing an unpinned project package or depending on upstream example code as an undocumented stable API.

### Code patterns

The current registry makes the normal addition path intentionally small:

> `crates/ark-core/src/platforms.rs:9-12`
>
> “Adding a new platform is a registry entry ... plus a new template tree. The command bodies ... iterate this slice.”

That promise is conditional on exclusive directory ownership. Removal is deliberately stronger than manifest-file deletion:

> `crates/ark-core/src/platforms.rs:190-197`
>
> “`removal_root` is wholly Ark-owned and removed wholesale. `extra_dirs` ... are shared with the user; only files that exactly match an Ark-shipped template ... are unlinked.”

Pi's shared, flat prompt discovery does not fit the current owned-root rule directly. The dedicated extension-root option avoids the shared directory; the flat-resource and local-package options require additional lifecycle or settings ownership.

Pi's extension command path accepts the exact Ark namespace. Registration stores the string directly:

> `reference/pi/packages/coding-agent/src/core/extensions/loader.ts:254-260`
>
> `extension.commands.set(name, { name, sourceInfo: extension.sourceInfo, ...options });`

Dispatch does not apply a command-name grammar; it splits only at the first space and looks up the remaining exact name:

> `reference/pi/packages/coding-agent/src/core/agent-session.ts:1265-1282`
>
> `const commandName = spaceIndex === -1 ? text.slice(1) : text.slice(1, spaceIndex);`
>
> `const command = this._extensionRunner.getCommand(commandName);`

Colon invocation names are first-class in resolution, not merely tolerated by the editor:

> `reference/pi/packages/coding-agent/src/core/extensions/runner.ts:595-645`
>
> A unique command keeps `invocationName = command.name`; duplicate commands become `${command.name}:${occurrence}`, and `getCommand` matches `invocationName` exactly.

Pi's context walk has no repository boundary and only path-level deduplication:

> `reference/pi/packages/coding-agent/src/core/resource-loader.ts:85-119`
>
> `seenPaths` records loaded file paths; the loop advances with `dirname(currentDir)` until `parentDir === currentDir`, then appends every distinct ancestor context file.

This makes the nested-worktree outcome deterministic: the parent checkout file and worktree file are different absolute paths and both survive `seenPaths`.

A dedicated extension root is natively discoverable:

> `reference/pi/packages/coding-agent/src/core/package-manager.ts:546-583`
>
> A subdirectory resolves its declared package entries or `index.ts`/`index.js`; auto-discovery uses that entry as the extension.

The adapter may also expose a private prompt directory explicitly:

> `reference/pi/packages/coding-agent/src/core/agent-session.ts:2256-2277`
>
> `resources_discover` results are converted to extension resource paths and passed to `resourceLoader.extendResources`.

> `reference/pi/packages/coding-agent/src/core/prompt-templates.ts:135-175,240-255`
>
> An explicit directory is accepted and its top-level Markdown files are loaded non-recursively.

Managed instructions, hooks, agent templates, and extra executable files converge through one path:

> `crates/ark-core/src/platforms.rs:116-149`
>
> `apply_managed_state` updates the managed block, applies the hook, writes reserved agent templates unconditionally, and then writes `extra_files`.

The OpenCode precedent already demonstrates the acceptable shape for a host with no declarative startup hook:

> `crates/ark-core/src/platforms.rs:366-381`
>
> “`SessionStart`-equivalent context injection rides a Bun-loaded TS plugin ... OpenCode has no native JSON hook surface.”

The platform registry currently contains four verified hosts:

> `crates/ark-core/src/platforms.rs:283-292`
>
> `PLATFORMS = [CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM, CODEAGENT_PLATFORM]`.

Ark's workflow authority is explicit and persistent:

> `.ark/workflow.md:74-80`
>
> “Each phase: pull context, run the CLI, write the artifact, advance.”

> `.ark/workflow.md:179-183`
>
> “Stage your work ... then run `/ark:commit` ... Do not commit automatically — staging is the user's step.”

> `.ark/workflow.md:199-202`
>
> If the message is generated, “Show the message and ask for confirmation before invoking the CLI.”

The subagent contract explicitly rejects automatic orchestration:

> `.ark/specs/features/subagent-support/SPEC.md:4-16`
>
> Ark ships three roles; “No automatic dispatch from CLI or from slash command; main session decides,” no EXECUTE agent, and no reviewer/verifier self-fix.

> `.ark/specs/features/subagent-support/SPEC.md:183-200`
>
> Each role carries recursion/write walls, while the researcher returns “paths plus one-line summaries.”

The sandbox has a separate and currently narrower host contract:

> `crates/ark-core/src/commands/sandbox/platform_argv.rs:1-30`
>
> Only Claude and Codex have verified bypass/yolo launch arguments; every other platform returns `AgentYoloUnsupported`.

> `sandbox/Dockerfile:16-22`
>
> The image installs `@anthropic-ai/claude-code` and `@openai/codex`, but no Pi package.

### External references

- [Pi v0.81.1 release](https://github.com/earendil-works/pi/releases/tag/v0.81.1) — current observed release (2026-07-21); its compatibility note restores fallback behavior for extensions using the pre-0.81 agent-core API, relevant to API-stability risk.
- [Pi coding-agent package metadata at v0.81.1](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/package.json) — canonical package `@earendil-works/pi-coding-agent`, `pi` executable, `.pi` configuration identity, MIT license, and current repository namespace.
- [Pi coding-agent README at v0.81.1](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/README.md) — built-in tools, command surface, context-file discovery, session capabilities, offline mode, and “extensions implement subagents/plan mode/sandbox” boundary.
- [Prompt templates](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/docs/prompt-templates.md) — `.pi/prompts`, trust gating, filename-to-command mapping, argument substitutions, and non-recursive discovery.
- [Agent Skills](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/docs/skills.md) — `.pi/skills`/`.agents/skills`, model discovery, forced `/skill:name`, and arbitrary-action warning.
- [Extensions](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/docs/extensions.md) — full-permission TypeScript modules, project trust, lifecycle/resource events, commands, messages, tools, UI, error behavior, and interactive/non-interactive differences.
- [Packages](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/docs/packages.md) — project package installation after trust, pinned npm/git references, conventional resource directories, and full-access warning.
- [Official subagent example](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/examples/extensions/subagent/README.md) — example-only `.pi/agents` discovery, project-agent confirmation, agent frontmatter, and single/parallel/chain modes.
- [Official subagent example implementation](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/examples/extensions/subagent/index.ts) — separate `pi --mode json -p --no-session` child process, cwd/tool/model plumbing, and explicit project scope.
- [Session format](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/docs/session-format.md) — JSONL tree, resume/fork/branch/compaction semantics; relevant to the boundary from Ark's durable task state.
- [SDK](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/docs/sdk.md) — programmable sessions, custom resource loading, events, and tools; relevant as a future runtime option, not a necessary host adapter.
- [RPC](https://github.com/earendil-works/pi/blob/v0.81.1/packages/coding-agent/docs/rpc.md) — JSON control and UI request semantics, including degraded/unavailable UI behavior outside the interactive TUI.
- [Canonical Pi repository](https://github.com/earendil-works/pi) — current upstream location, MIT licensing, supply-chain guidance, and explicit no-built-in-permissions security model; the former `badlogic/pi-mono` location redirects here.

## Caveats / Not found

- No documented built-in/core Pi contract was found that automatically loads project `.pi/agents/*.md`; the only primary implementation found is the shipped subagent example extension.
- No native declarative Pi `SessionStart` hook equivalent to Claude/Codex/CodeAgent JSON hooks was found; extension lifecycle events are the closest surface.
- No Pi option was found that stops only context-file discovery at the nearest git root while retaining the current worktree's `AGENTS.md`; `--no-context-files` disables all such discovery.
- The dedicated `.pi/extensions/ark/` ownership shape is source-supported but was not exercised through Ark's init/unload/load/upgrade/remove lifecycle in this research task.
- No Pi-specific support exists in Ark's current registry, templates, context tests, sandbox image, sandbox argv, host-config mounts, or default credential environment list.
- No end-to-end Pi runtime spike was run. Exact colon command dispatch, filesystem-root context traversal, exact-path deduplication, and extension-subdirectory discovery are verified from source; adapter lifecycle/event behavior remains an inference until the pinned validation matrix passes.
- Pi's current 0.x API, repository/package namespace change, and v0.81.1 compatibility repair are current observations, not a forecast of future instability. The maintenance rating is an inference from those signals.
