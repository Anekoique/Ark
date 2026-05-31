# Hooks and Lifecycle Events

## What the primitive means

A *hook* is a user-defined callback the agent harness fires at a specific
moment in the agent loop — *before a tool runs, after the LLM emits, when
the session starts*. Hooks are how non-LLM logic injects guardrails,
context, telemetry, and policy without modifying the harness itself.

The canonical use cases:

- **Context injection** — at session start, dump project state into the
  conversation (Ark's only hook today).
- **Tool gating** — before a tool call, validate inputs / consult policy /
  deny outright.
- **Auto-format / lint** — after a Write or Edit, run `cargo fmt`, `eslint
  --fix`, etc.
- **Telemetry** — emit OpenTelemetry spans, log to Langfuse, count costs.
- **Memory pinning** — at stop, ask the model to summarise insights into a
  memory file.
- **Compaction guard** — back up the full transcript before the harness
  compacts it.

The shape — *event name → shell command → JSON response* — converged
remarkably fast. Claude Code shipped first (May 2025); Codex copied the
shape (October 2025); VS Code Copilot copied them both (late 2025). The
event *names* differ slightly across harnesses; the *behaviour* is the
same.

## How leading harnesses implement it

### Claude Code (Anthropic) — the canonical 27-event surface

As of May 2026, Claude Code documents **27 distinct events** (`thepromptshelf.dev/blog/claude-code-hooks-complete-reference-2026/`,
`code.claude.com/docs/en/hooks`). Grouped by cadence:

**Once per session:**
- `SessionStart` — first prompt of a new session (sources: `startup`, `clear`, `compact`, `resume`)
- `Setup` — environment-prep slot
- `SessionEnd` — session terminates

**Once per turn:**
- `UserPromptSubmit` — fires on every user message *before* Claude processes
- `UserPromptExpansion` — after slash-command expansion
- `Stop` — Claude believes it has finished responding
- `StopFailure` — error termination
- `Elicitation` / `ElicitationResult` — for interactive sub-prompts
- `Notification` — generic UI notification

**Per tool call:**
- `PreToolUse` — *can deny / modify input*; returns `{decision, hookSpecificOutput}`
- `PostToolUse` — fires after success
- `PostToolUseFailure` — fires after tool error
- `PostToolBatch` — after a batch of parallel tool calls
- `PermissionRequest` — user-permission gate
- `PermissionDenied` — when the gate denies

**Subagent lifecycle:**
- `SubagentStart` — Task tool spawns a subagent
- `SubagentStop` — subagent finishes

**Workspace / context:**
- `InstructionsLoaded` — CLAUDE.md loaded
- `ConfigChange` — settings changed
- `CwdChanged` — working dir moved
- `FileChanged` — Claude observes external file change
- `WorktreeCreate` / `WorktreeRemove`
- `PreCompact` / `PostCompact` — context-window compaction
- `TeammateIdle` — multi-agent: peer is idle
- `TaskCreated` / `TaskCompleted` — Task-tool lifecycle

**Configuration shape** (`.claude/settings.json`):

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [{ "type": "command", "command": "scripts/format.sh", "timeout": 5000 }]
      }
    ]
  }
}
```

Timeout is in **milliseconds**. Multiple matching hooks all run; for
`PreToolUse` the decision is a *resolution* across them.

`PreToolUse` is special: it returns a `hookSpecificOutput` with one of
`{allow, deny, ask, defer}` and optionally a modified `tool_input`. This is
the deterministic policy layer Anthropic gave up trying to bolt onto the
LLM.

**Exit-code semantics**: `0` = continue; non-zero blocks (esp. exit `2`
forces Stop hooks to keep working — see `producttalk.org/give-claude-code-a-memory/`).

### OpenAI Codex CLI

Smaller surface, same shape (`developers.openai.com/codex/hooks`):

- `SessionStart`
- `UserPromptSubmit`
- `PreToolUse`
- `PostToolUse`
- `PermissionRequest`
- `Stop`

Configured via `hooks.json` **or** an inline `[hooks]` table in
`config.toml`. **Timeout is in seconds** (default 600 if omitted) — this is
the schema delta Ark's `platforms.rs` accounts for
(`ark_codex_hook_entry()`: `"timeout": 30` seconds; vs Claude's `"timeout":
5000` ms).

Codex hooks ship with `/hooks` CLI to inspect / trust / disable hooks at
runtime — useful when a malicious template tries to slip a hook in. Hooks
are enabled by default; admins can force-off via `requirements.toml`'s
`[features].hooks = false`.

**Concurrency model**: matching hooks from multiple files all run; hooks
for the same event are launched **concurrently** — so one slow hook does
not gate another. Claude Code is sequential within a matcher.

### VS Code Copilot (Preview, late 2025)

Mirror of Claude Code's surface (`code.visualstudio.com/docs/copilot/customization/hooks`):
`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PreCompact`,
`SubagentStart`, `SubagentStop`, `Stop`. Plugin hooks fire *in addition to*
workspace and user hooks. Cross-pollination is explicit: Microsoft adopted
the JSON shape and event names.

### Continue.dev

No native hook surface comparable to Claude / Codex. Customization is via
**Rules** in `.continue/rules/` — system-prompt fragments applied per
mode (Chat / Edit / Agent). Closer to "configuration" than "callbacks."

### OpenHands

Lifecycle is centered on the **microagent** model — `.openhands/microagents/`
files with `trigger: always | keyword | manual` frontmatter. Not a hook in
the Claude / Codex sense (no PreToolUse semantics) but covers the
context-injection use case.

### Aider, Cline, Cursor

No exposed hook surface. Aider has its own commit lifecycle (auto-commit
after each Claude edit). Cursor's "rules" are configuration files,
not callbacks. Cline relies on MCP server lifecycle.

### Cross-harness summary

| Harness | Event count | PreToolUse gate? | Concurrency | Config |
| ------- | ----------- | ---------------- | ----------- | ------ |
| Claude Code | 27 | Yes (`allow/deny/ask/defer`) | Sequential | `.claude/settings.json` |
| Codex CLI | 6 | Yes | Concurrent | `~/.codex/hooks.json` or inline TOML |
| VS Code Copilot | 8 | Yes | Per-event | Workspace + plugin hooks |
| Continue | n/a | n/a | n/a | Rules only |
| OpenHands | n/a (microagents) | n/a | n/a | `.openhands/microagents/` |
| Cursor / Aider / Cline | None native | n/a | n/a | n/a |

## What Ark does today

Ark installs exactly **one** hook per platform — `SessionStart` — and
captures it through the snapshot pipeline. There is no Ark-defined hook
*surface*; the host platforms own that.

### The `SessionStart` install

- Canonical Claude entry — `crates/ark-core/src/io/fs/hook.rs:45`:

```rust
pub fn ark_session_start_hook_entry() -> serde_json::Value {
    serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": ARK_CONTEXT_HOOK_COMMAND,
                "timeout": 5000,
            }
        ],
    })
}
```

- Canonical Codex entry (`hook.rs:64`) uses `"timeout": 30` (seconds, not ms).
- Identity command — `ARK_CONTEXT_HOOK_COMMAND = "ark context --scope session
  --format json"` (`hook.rs:15`). The full command IS the identity key —
  Ark recognises its own hook by exact string match on `command`.
- OpenCode has no native hook surface; Ark ships a Bun-loaded TypeScript
  plugin (`OPENCODE_PLUGIN_FILE = ".opencode/plugins/ark-context.ts"`,
  `layout.rs:149`) as the moral equivalent.

### Hook lifecycle in `init` / `unload` / `load` / `remove`

- `init` / `upgrade` → `Platform::apply_managed_state` calls
  `HookFileSpec::apply_canonical` which idempotently inserts the entry
  (`platforms.rs:123` + `hook.rs:95`).
- `unload` → `Platform::capture_hook` reads the entry into a
  `SnapshotHookBody`, records it in `.ark.db`, removes the live entry but
  preserves sibling user entries (`platforms.rs:153`).
- `load` → on restore, `SnapshotHookBody::apply` splices the captured entry
  back; if the snapshot lacks one, `apply_managed_state` writes the
  canonical entry afresh (`platforms.rs:263`).
- `remove` → `Platform::remove_hook` surgically removes Ark's entry, sibling
  user hooks survive (`platforms.rs:181`).

### Hook *exposure* — none

Ark does not define its own hook surface. The closest analogues:

- **Slash commands** as user-facing entry points (`templates/claude/commands/ark/`).
- **Subagents** that the *host* invokes (`templates/claude/agents/ark-*.md`).
- **CLI verbs** (`ark agent task verify`, etc.) that templates call.

A user who wants `PreToolUse` policy goes to the host platform's hook
config, not to Ark.

### Discipline that already pays off

- **One identity per hook file** — Ark identifies its entry by
  `command` field equality, so two Ark installs in the same project would
  *not* duplicate the entry.
- **Per-platform schema delta** — `hook.rs` accommodates ms vs s for
  timeouts; canonical entries are immutable functions.
- **Snapshot replay is verbatim** — captured `serde_json::Value` is
  spliced back unchanged, so users who hand-edit their hook entry have
  their edits preserved across unload/load.

## What Ark could plausibly hook

Today there is *no* user-facing Ark hook surface. The natural events
emerge from the workflow state machine in `crates/ark-core/src/commands/agent/state.rs`
(legal-transition table) and the per-task lifecycle.

### Candidate event taxonomy

**Task lifecycle:**
- `task.created` — after `task new` allocates the directory
- `task.phase.changed` — every `task plan / review / execute / verify / commit`
- `task.committed` — after the atomic commit succeeds
- `task.archived` — after `ark archive` moves the task

**Spec lifecycle:**
- `spec.promoted` — after deep-tier `task commit` extracts SPEC + upserts INDEXes
- `spec.changed` — when an existing SPEC body diverges

**Worktree lifecycle:**
- `worktree.created` — natural pre-existing slot (`[worktree].post_create` in
  `config.toml` already)
- `worktree.cleanup` — `ark cleanup` removed a worktree

**Workspace / journal:**
- `journal.session.recorded` — after a developer journal entry stamps

This taxonomy parallels Claude Code's. The hooks would be in
`.ark/config.toml`, command-shaped:

```toml
[[hooks.task_committed]]
command = "scripts/notify.sh"
timeout = 30

[[hooks.spec_promoted]]
command = "scripts/update-changelog.sh"
```

### Concurrency stance

For Ark hooks, **sequential within an event, concurrent across events** is
the natural fit — Ark's commands themselves are sequential, and we want
deterministic ordering when an upstream hook produces output the next one
reads (e.g. a CHANGELOG hook before a release-notes hook).

## Directions for Ark

1. **`[hooks]` table in `.ark/config.toml`.** Land a thin Codex-shaped hook
   surface keyed by Ark events (`task_committed`, `spec_promoted`,
   `worktree_created`). Reuse `crates/ark-core/src/io/fs/hook.rs` patterns
   for JSON parsing and identity. This is the smallest extension that
   gives users a CI / notification surface. Code site:
   `crates/ark-core/src/commands/agent/state.rs` (where transitions fire).
2. **Promote `[worktree].post_create` to first-class.** The slot exists in
   the workflow contract but is not implemented in `commands/agent/task/worktree/`.
   Even before a full hook surface, this single hook is the highest-leverage
   one — users want it for `direnv`, `npm install`, container boot.
3. **`PreCommit` policy hook.** Before `ark agent task commit` runs the
   atomic commit, fire a hook with the staged-file list; non-zero exit
   blocks. Pairs naturally with Direction 5's compliance use case.
   Code site: `crates/ark-core/src/commands/agent/task/commit.rs`.
4. **Hook telemetry side-channel.** Have `ark context` accept a `--hook-event
   <name>` arg and emit a JSON-Lines audit trail to
   `.ark/.hook-log.jsonl`. Lets users replay an Ark session purely from
   the log, which is a workflow analogue of Claude's transcript
   capture. Pairs with the `observability-and-telemetry.md` directions.
5. **Reserve Ark's identity key per host platform.** Today
   `ARK_CONTEXT_HOOK_COMMAND` is the identity, which means changing the
   command (e.g. adding `--scope record`) breaks unload/load round-trip
   on existing installs. Migrate to a stable identity tag (`"id": "ark"`
   field) and parse Ark's entry by that, freeing the command string to
   evolve. Code site: `crates/ark-core/src/io/fs/hook.rs:15` + the
   `update_hook_file` identity matching.

## Caveats / Not found

- I could not locate a *spec* document for Claude Code's exit-code
  semantics beyond community write-ups; verify against the source before
  building tooling around them.
- The "27 events" enumeration is from a community reference; Anthropic's
  primary docs list a subset that gets re-counted under aliases — the real
  count is likely smaller after de-duplication.
- VS Code Copilot's hook surface is in *preview* — schema may move.
- OpenCode's TS plugin model has not converged to a hook surface in the
  Claude-Code sense; treat it as a separate primitive (extension, not
  callback).

## Sources

- [Claude Code Hooks Reference](https://code.claude.com/docs/en/hooks)
- [Claude Code Agent SDK Hooks](https://code.claude.com/docs/en/agent-sdk/hooks)
- [Claude Code Hooks Complete Reference 2026](https://thepromptshelf.dev/blog/claude-code-hooks-complete-reference-2026/)
- [Claude Code Hooks: 12 Lifecycle Events](https://claudefa.st/blog/tools/hooks/hooks-guide)
- [Claude Code Hooks Guide (SmartScope)](https://smartscope.blog/en/generative-ai/claude/claude-code-hooks-guide/)
- [OpenAI Codex Hooks](https://developers.openai.com/codex/hooks)
- [Codex Hooks DeepWiki](https://deepwiki.com/openai/codex/3.11-hooks-system)
- [Codex Hooks Make the Harness Real](https://blakecrosley.com/blog/codex-hooks-make-the-harness-real)
- [VS Code Copilot Hooks (Preview)](https://code.visualstudio.com/docs/copilot/customization/hooks)
- [OpenHands Microagents Overview](https://docs.openhands.dev/openhands/usage/microagents/microagents-overview)
- [Continue.dev Rules](https://docs.continue.dev/customize/deep-dives/rules)
- [Cupcake — Claude Code reference](https://cupcake.eqtylab.io/reference/harnesses/claude-code/)
