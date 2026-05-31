# OpenCode Integration — Deep Dive

OpenCode (`sst/opencode`, TypeScript/Bun, OSS) is Ark's third platform target. It is the smallest of the three by reach but the most architecturally distinctive: its extension model is **runtime TypeScript plugins**, not file-based prompt fragments.

> Reference: `01_prior_art/agent-platforms.md` (if relevant) and `02_infra_primitives/hooks-and-lifecycle-events.md` for hook-event taxonomy.

## Extension points

OpenCode exposes:

1. **Commands** — `.opencode/commands/<name>.md` (markdown slash commands, similar to Claude/Codex)
2. **Sub-agents** — `.opencode/agents/<name>.md`
3. **Plugins** — `.opencode/plugins/<name>.ts` (runtime Bun/TypeScript hooks)
4. **AGENTS.md / opencode.config** — context + config

The big distinction is point 3.

## Plugins — what makes OpenCode different

Where Claude and Codex have static hook *files* (one shell command per event), OpenCode's plugins are *runtime modules*:

```typescript
// .opencode/plugins/ark-context.ts
import { OpenCodePlugin, SessionStartEvent } from "@opencode/sdk";

export default {
  name: "ark-context",
  events: {
    SessionStart: async (e: SessionStartEvent) => {
      // run `ark context`, inject the output
      const ctx = await runShell("ark context --format json");
      return { messages: [{ role: "system", content: ctx }] };
    },
    // ... 25+ other events available
  },
} satisfies OpenCodePlugin;
```

This is genuinely different from Claude's `hooks: { SessionStart: ["ark context"] }` or Codex's similar JSON.

**What plugins enable that hooks don't:**
- **Stateful behaviour across events.** A plugin can carry state between `SessionStart` and later `PostToolUse` events.
- **Rich transformations.** Plugins can edit messages, inject context, redirect tools, transform outputs.
- **Async TypeScript.** Full Bun runtime — fetch APIs, file ops, third-party libs.
- **Type safety** (in development) — TS catches missing fields.

**What plugins cost:**
- Bun runtime dependency (small but real).
- TypeScript / Bun maintenance — Claude/Codex hooks are language-agnostic shells.
- Slower start (Bun loads plugins per session).

## Event taxonomy

OpenCode plugins can subscribe to ~25 events (broader than Claude's ~27 hooks; both are richer than Codex's ~6).

Major event categories:

- **Session lifecycle:** `SessionStart`, `SessionEnd`, `SessionResume`.
- **Message events:** `BeforeMessage`, `AfterMessage`, `MessageStream`.
- **Tool events:** `PreToolUse`, `PostToolUse`, `ToolError`.
- **File events:** `BeforeFileWrite`, `AfterFileWrite`, `BeforeFileRead`, `AfterFileRead`.
- **Agent events:** `AgentDispatch`, `AgentReturn`.

Some are unique to OpenCode (the file-level events are finer-grained than Claude's `PreWriteFile`).

## What Ark currently uses

Ark ships exactly one plugin: `.opencode/plugins/ark-context.ts`, which subscribes to `SessionStart` and injects `ark context` output. Equivalent in effect to the Claude/Codex hook setup.

Slash commands: yes, the 8 standard ones under `.opencode/commands/ark/`.
Subagents: yes, the trio under `.opencode/agents/`.

## What Ark could do with plugins

This is where OpenCode shines uniquely. Some patterns:

### Cross-event state

A plugin could maintain state across events:

```typescript
let inProgressDispatches = new Set<string>();

export default {
  events: {
    AgentDispatch: (e) => {
      inProgressDispatches.add(e.agentId);
      return null;
    },
    AgentReturn: async (e) => {
      inProgressDispatches.delete(e.agentId);
      // detect zombie dispatches periodically
    },
  },
};
```

Ark's "in-flight dispatches" surface (proposed in `05_orchestration/dispatch-models.md`) could be implemented as an OpenCode plugin in a way that hooks can't match.

### Conditional context refresh

A plugin could detect phase transitions (the user just ran `ark agent task plan`) and proactively re-inject context:

```typescript
events: {
  PostToolUse: async (e) => {
    if (e.toolName === "Bash" && e.command.includes("ark agent task")) {
      const ctx = await runShell("ark context --format json");
      return { injectMessage: { role: "system", content: ctx } };
    }
  },
}
```

Hooks could fire on `PostToolUse` but reasoning about *which* shell command was Ark-related requires plugin logic.

### Sub-agent budget enforcement

A plugin could track sub-agent dispatches and enforce per-task budgets:

```typescript
events: {
  AgentDispatch: async (e) => {
    const task = await readTask();
    if (task.subagentDispatches >= task.maxDispatches) {
      return { block: true, reason: "Dispatch budget exhausted" };
    }
  },
}
```

This is OpenCode-only behaviour Claude / Codex can't match.

### File-write SPEC-drift detection

A plugin could intercept feature SPEC writes and verify CHANGELOG entries:

```typescript
events: {
  BeforeFileWrite: (e) => {
    if (e.path.includes("specs/features/") && e.path.endsWith("SPEC.md")) {
      if (!e.newContent.includes("[**CHANGELOG**]")) {
        return { warn: "Modified SPEC without CHANGELOG entry" };
      }
    }
  },
}
```

Today this is a VERIFY phase concern; with a plugin it could be a real-time check.

## Plugin maintenance trade-offs

| Concern | Impact |
| ------- | ------ |
| TypeScript / Bun dependency | Adds runtime; minor on developer machines, possibly bigger on CI |
| Type safety | Catches misuse at edit time; valuable for non-trivial plugins |
| Per-platform divergence | OpenCode plugins ≠ Claude hooks ≠ Codex hooks — three formats for the same logical event |
| Test surface | Plugins are code, can be tested; hooks are config, harder to test |
| Discoverability | Plugins are first-class in `.opencode/plugins/` listings |

For Ark's current scope (one SessionStart-equivalent), plugins are overkill. For the proposed future scope (cross-event state, budget enforcement, drift detection), plugins are the right substrate — and OpenCode is the only platform that natively supports them.

## OpenCode positioning

As of 2026:
- Smaller install base than Claude / Codex.
- TypeScript-first community, distinct from Anthropic / OpenAI orbit.
- ACP-compatible (one of the early adopters).
- Strong on extensibility (plugins).

Ark's three-platform strategy is *additive*: shipping support for OpenCode adds a niche but capable third surface. The plugin model is the most distinctive thing Ark gets from it.

## Where to lean in

If Ark were to *lean into* OpenCode (rather than just maintain parity):

1. **Build the proposed cross-event features as OpenCode plugins first.** Tracking in-flight dispatches, budget enforcement, drift detection — all easier in plugins than in Claude / Codex hooks.
2. **Use OpenCode as a "rich integration sandbox".** Features that prove themselves in OpenCode can be back-ported to Claude / Codex as best-effort hooks.
3. **Contribute to the OpenCode plugin ecosystem.** Other people writing plugins benefits from Ark's existence.

If Ark were to *de-prioritise* OpenCode:
- Maintain current integration (1 plugin, 8 commands, 3 subagents).
- Don't invest further; focus engineering on Claude + Codex.

The current investment is fine; the OpenCode plugin model is worth knowing about even if Ark doesn't use it for everything.

## Trade-offs of MCP via OpenCode

OpenCode does support MCP. If Ark ships `ark-mcp`, OpenCode is one of the three targets that can consume it natively.

The MCP path partly substitutes for plugin investment: instead of writing OpenCode-specific plugins, expose the capability via MCP and let OpenCode (and every other MCP client) consume it.

For Ark this means: prefer MCP for cross-host capabilities; reserve OpenCode plugins for genuinely OpenCode-unique features (cross-event state).

## Directions for Ark

1. **Use OpenCode plugins as the experimental venue for cross-event features.** Track in-flight dispatches, enforce budgets, detect drift — implement in OpenCode first; learn what's worth porting to Claude/Codex hooks (or MCP).

2. **Don't over-invest in OpenCode-specific complexity.** The plugin model is powerful but OpenCode's install base is smaller. Use plugins for things that genuinely benefit from runtime state; keep simple integrations as hooks/commands.

3. **Register the `ark-mcp` server in OpenCode's config.** When `ark-mcp` ships, OpenCode is one of the early consumers. The plugin and the MCP server can coexist (plugin handles event-driven state; MCP handles request-driven capabilities).

4. **Add type definitions to the OpenCode plugin SDK if missing.** Ark's `ark-context.ts` benefits from typed event payloads. Contributing types upstream helps every Ark-on-OpenCode user.

5. **Document the plugin pattern for contributors.** Even if Ark doesn't add many plugins, the *existence* of OpenCode's richer extensibility model is teachable — users on Claude/Codex who want the same behaviour see where the platform gaps are.
