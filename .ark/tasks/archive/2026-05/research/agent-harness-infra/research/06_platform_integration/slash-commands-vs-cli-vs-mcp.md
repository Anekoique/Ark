# Slash Commands vs. External CLI vs. MCP

The three integration surfaces for layering a tool onto a coding agent. Each has different trade-offs on discoverability, statefulness, schema enforcement, and error surface. Ark today uses a *hybrid* — slash commands (per host platform) that shell out to an external CLI.

## The three surfaces

### Surface A — slash command (markdown-as-prompt)

The host agent exposes a slash command namespace (`/ark:design`, `/ark:commit`, etc.). The user types it; the host expands a markdown file (e.g. `.claude/commands/ark/design.md`) into a prompt that the LLM sees.

The slash command file *is* a prompt. It can contain $args, environment refs, embedded shell commands (host-dependent). It does not return structured data — its output is "the next agent turn".

**Used by:** Claude Code (`.claude/commands/`), Codex (`.codex/skills/`), OpenCode (`.opencode/commands/`), Cursor (custom commands).

**Pros:**
- Native discoverability (host's slash-command picker).
- Tight integration with the agent's flow (prompt-shaped).
- No extra process to manage.

**Cons:**
- Statelessness — the slash command is consumed once per turn.
- No schema — prompts can drift; agent must parse them.
- Host-platform-specific format (per-platform divergence).
- Limited error handling — if the prompt is wrong, the agent flails.

### Surface B — external CLI (subprocess)

The host agent's `Bash` tool runs an external CLI binary; the binary returns text/JSON; the agent ingests it.

**Used by:** `gh`, `git`, `cargo`, `npm`, anything in `/usr/local/bin/`. And: Ark (`ark agent <verb>`), Claude Code's reliance on shell tools.

**Pros:**
- Native CLI ergonomics — well-defined arguments, exit codes, stdin/stdout, --help.
- Schema enforceable via JSON output mode (`ark context --format json`).
- Cross-platform — same binary on Linux, macOS, Windows; no per-host scaffolding.
- Reusable outside agents — humans can run the CLI directly; CI can run it.

**Cons:**
- Discoverability — the agent must know to call the CLI; nothing surfaces it natively.
- Process boundary cost — each invocation spawns a process.
- Statelessness — process-per-call; persistent state must live on disk.

### Surface C — MCP server (tools / resources / prompts)

The host agent talks to an MCP server over JSON-RPC (stdio or HTTP). The server exposes typed tools (functions), resources (data), and prompts.

**Used by:** Many. MCP filesystem server, git server, GitHub server, etc. Cursor speaks MCP; Continue speaks MCP; Claude Code speaks MCP; Zed speaks MCP.

**Pros:**
- Typed schemas — input/output validated.
- Cross-host portability — one MCP server works on every MCP client.
- Resources are first-class — pull data, not just call functions.
- Prompts can be templated and parameterised server-side.

**Cons:**
- Heavier than a CLI — server lifecycle, transport handshake.
- Less direct than slash commands — agent needs to discover and invoke.
- 2026 maturity — most users don't yet have MCP config; bring-your-own-server is non-trivial.

## Comparison matrix

| Dimension | Slash command | External CLI | MCP server |
| --------- | ------------- | ------------ | ---------- |
| Discoverability | High (host UI) | Low | Medium (MCP listing) |
| Schema enforcement | None | Optional (CLI args + JSON) | Built-in |
| Statefulness | None | Disk-backed | Server holds session state |
| Cross-host portability | Low (per-host file format) | High (binary works everywhere) | High (one server, many clients) |
| Error surface | Vague prompt failures | Exit codes + stderr | Structured error responses |
| Onboarding cost | Low (host knows about it) | Medium (user installs binary) | High (user configures MCP) |
| Maintenance cost | Per-host templates | Single binary | Single server |
| Best for | Agent-invoked workflow steps | Anything CLI-shaped, dual-use | Long-running stateful interactions |

## Ark's current hybrid

Ark uses A + B:

1. **Slash commands** (`.claude/commands/ark/*.md`, `.codex/skills/ark-*/SKILL.md`, `.opencode/commands/ark/*.md`) define *what* to do at each workflow step.
2. **External CLI** (`ark agent <verb>`) implements *how*.

The slash command's body usually shells to `ark agent` and orchestrates around the result. Example: `/ark:design` calls `ark agent task new ...` then guides the agent through PRD-filling.

**Strengths:**
- Discoverability via slash command picker.
- CLI is the canonical engine; slash commands are thin orchestration.
- Per-host templates handle platform-specific UX.
- The CLI is also human-usable.

**Weaknesses:**
- Per-host template maintenance — 8 commands × 3 platforms = 24 files to keep in sync.
- No schema enforcement on slash-command bodies; drift is possible.
- Discoverability still requires per-host install (no native cross-host).

## What surface C (MCP) would buy

If Ark also hosted an MCP server (`ark-mcp` crate), the calculus shifts:

- **Resources:** task list, PRD, PLAN, VERIFY, INDEX.md — agent pulls them via MCP `resources/read`.
- **Tools:** `task_new`, `task_plan`, `task_verify`, `task_commit`, etc. — agent calls them via MCP `tools/call`.
- **Prompts:** templated prompts for each workflow phase — agent loads them via MCP `prompts/get`.

A host platform that speaks MCP (Cursor, Continue, Zed) could then use Ark *without per-platform templates*. The MCP surface replaces the slash-command surface on those hosts.

The slash-command + CLI surfaces stay for Claude Code / Codex / OpenCode (where MCP integration is less developed); MCP opens Ark to a larger set of hosts.

## The "slash command → skill" shift

A trend worth noting: Claude Code's 2026 docs increasingly favour skills (`.claude/skills/`) over slash commands (`.claude/commands/`). Skills have:
- Frontmatter for discoverability metadata.
- SKILL.md as the procedural body.
- Optional scripts / templates in the skill directory.
- Loaded conditionally when the agent decides it's relevant.

Codex made the same move. Goose, Cursor: similar. Slash commands stay supported but are being relabelled as legacy.

If Ark adopts the trend:
- `.claude/commands/ark/design.md` → `.claude/skills/ark-design/SKILL.md`.
- Single canonical source emits to both formats during a transition.
- Long-term: skills only.

The same source-of-truth question (single declarative source emitting per-platform format) applies to MCP as well — the MCP server's tool schema, the SKILL.md frontmatter, and the slash-command body could all come from one place.

## When each surface is right

**Slash command:**
- Quick, prompt-shaped step.
- Tight integration with the agent's flow.
- Per-host where users live in the host UI.

**External CLI:**
- Operations that should also work outside an agent (in CI, in a terminal).
- State-mutating operations that benefit from structured exit codes.
- Cross-host operations where one binary should run everywhere.

**MCP server:**
- Stateful interactions where session state matters.
- Cross-host portability is a primary goal.
- Resources / templated prompts are first-class.

The hybrid is often best. Ark's slash-command + CLI hybrid is good for its current 3-host install base; adding MCP would extend the install base without losing the existing surfaces.

## Directions for Ark

1. **Stand up `ark-mcp` as a thin server exposing the `ark agent` namespace.** The CLI surface is already typed; MCP exposure is a translation layer. High leverage, contained scope.

2. **Plan the slash-command → skill migration.** Audit the 8 per-host slash commands; pick the highest-value (e.g. `/ark:design`, `/ark:commit`) to ship as skills first. Keep slash commands as redirects during transition.

3. **Single-source the per-host templates.** A `templates/canonical/<command>.md` source-of-truth that emits to `.claude/commands/`, `.codex/skills/`, `.opencode/commands/`, plus future MCP `prompts/`. Eliminates the 8×3 maintenance tax.

4. **Document the three-surface hybrid as a design choice.** `docs/book/src/concepts/integration-surfaces.md` would teach users (and contributors) when to use which surface for new Ark features.

5. **Surface CLI vs. slash UX in `ark context`.** Tell the agent "you can call `ark agent task plan` directly via Bash, or `/ark:plan` via the slash interface." Removes ambiguity in mid-session decisions.
