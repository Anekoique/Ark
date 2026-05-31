# Research: Claude Agent SDK — Skills, CLAUDE.md/AGENTS.md loading, and portability

- Query: filesystem `.claude/skills/` and `AGENTS.md`/`CLAUDE.md` loading via `settingSources`; how skills are discovered and invoked; portability across SDK and CLI surfaces; whether skills can be registered programmatically.
- Scope: external (primary docs: `code.claude.com` Agent SDK + Claude Code; `platform.claude.com` Agent Skills overview; SDK source for options).
- Date: 2026-05-25

## Version snapshot

| Surface | Version | Source |
| ------- | ------- | ------ |
| Python SDK (`claude-agent-sdk`) | **0.2.87** | `anthropics/claude-agent-sdk-python` (bundles Claude CLI 2.1.150), fetched 2026-05-25 |
| TypeScript SDK (`@anthropic-ai/claude-agent-sdk`) | **0.3.150** | `anthropics/claude-agent-sdk-typescript` |
| Docs | `code.claude.com/docs/en/agent-sdk/{skills,modifying-system-prompts}`, `code.claude.com/docs/en/{skills,memory,sub-agents}`, `platform.claude.com/docs/en/agents-and-tools/agent-skills/overview` | fetched 2026-05-25 |

**Cross-references (do not duplicate):**

- **Topic 02 `02_sessions.md`** owns the full `settingSources` table (`"user"`/`"project"`/`"local"`, what each loads, omit-vs-`[]` semantics, the multi-tenant `<Warning>`, the always-loaded managed-policy / `~/.claude.json` / auto-memory channels). This file restates the skill/CLAUDE.md/AGENTS.md slices of that table and confirms topic 02's open AGENTS.md flag; it does not re-derive the full source semantics.
- **Topic 06 `06_subagents.md`** owns filesystem `.claude/agents/` discovery and the per-subagent `skills` preload override (`AgentDefinition.skills`). This file references the skill side of that override; it does not re-cover agent definition fields.
- **Topic 05 `05_tools-and-permissions.md`** owns the built-in tool inventory; this file references the `Skill` tool (auto-enabled when the `skills` option is set) but defers tool-permission mechanics to 05.
- **Topic 01 `01_overview-and-relationship-to-claude-code.md`** owns the CLI-only/SDK-only split table; §6 here cites it for portability and does not restate the whole split.

---

## TL;DR

1. **A skill is a filesystem artifact, not code.** A skill is a directory `<root>/.claude/skills/<name>/` containing a `SKILL.md` (YAML frontmatter + Markdown body) plus optional bundled files. The model autonomously invokes it via the built-in `Skill` tool when the request matches the `description`. (§1)
2. **No programmatic skill registration exists in this SDK.** Subagents can be defined in code (`agents` option); skills cannot. Verbatim: *"Unlike subagents (which can be defined programmatically), Skills must be created as filesystem artifacts. The SDK does not provide a programmatic API for registering Skills."* The only code surface is the `skills` *filter* option. (§2)
3. **Skills load through `settingSources` (`"user"`/`"project"`); the `skills` option is a separate filter.** Omit `settingSources` ⇒ all three sources load ⇒ skills discovered + Skill tool available. Pass `[]` ⇒ no skills. The `skills` option (`"all"` | name list | `[]`) then gates *which* discovered skills are enabled, and setting it auto-enables the Skill tool. (§1, §3)
4. **AGENTS.md is NOT loaded by this SDK — definitively confirmed.** Verbatim from the memory doc: *"Claude Code reads `CLAUDE.md`, not `AGENTS.md`."* The documented portability path is to make `CLAUDE.md` import it: `@AGENTS.md`. This closes topic 02's open flag. (§4)
5. **CLAUDE.md loads via setting sources into the *conversation* (not the system prompt), supports `@path` imports (depth 5), and concatenates root→cwd with managed > user > project > local precedence.** (§5)
6. **Filesystem skills/agents/commands authored for the CLI work unchanged in the SDK** — same `.claude/skills/`, `.claude/agents/`, `.claude/commands/` files — *as long as `settingSources` enables them*. The one documented SDK gap: `allowed-tools` frontmatter in SKILL.md is **ignored** by the SDK. (§6)
7. **Skills vs subagents vs MCP** are three different behavior-packaging mechanisms: skill = model-invoked instruction/procedure pack loaded into the *current* context; subagent = delegated fresh-context worker; MCP = external tool/resource provider. The RFC's "skill-style behavior packs" map to skills — portable prompt/procedure packs, not tools and not isolated workers. (§7)

---

## 1. The skills mechanism

### 1.1 What a skill is

An Agent Skill is a filesystem-packaged capability the model invokes autonomously when relevant. The SDK doc, verbatim:

> Agent Skills extend Claude with specialized capabilities that Claude autonomously invokes when relevant. Skills are packaged as `SKILL.md` files containing instructions, descriptions, and optional supporting resources.

Each skill is a **directory** whose entrypoint is `SKILL.md` (required); other files are optional:

```
.claude/skills/processing-pdfs/
└── SKILL.md
```

Or, with bundled resources (Claude Code skills doc):

```
my-skill/
├── SKILL.md           # Main instructions (required)
├── template.md        # Template for Claude to fill in
├── examples/
│   └── sample.md      # Example output
└── scripts/
    └── validate.sh    # Script Claude can execute
```

**Progressive disclosure — three load levels** (platform Agent Skills overview, verbatim table):

| Level | When loaded | Token cost | Content |
| ----- | ----------- | ---------- | ------- |
| Level 1: Metadata | Always (at startup) | ~100 tokens per Skill | `name` and `description` from YAML frontmatter |
| Level 2: Instructions | When Skill is triggered | Under 5k tokens | SKILL.md body with instructions and guidance |
| Level 3+: Resources | As needed | Effectively unlimited | Bundled files executed via bash without loading contents into context |

So the metadata (name + description) is the only always-on cost; the body loads on trigger; bundled files load only when referenced. This is the design reason a skill is cheaper than CLAUDE.md for long procedures — the Claude Code skills doc states it directly: *"Unlike CLAUDE.md content, a skill's body loads only when it's used, so long reference material costs almost nothing until you need it."*

### 1.2 The `.claude/skills/` directory and locations

Where a skill lives sets who can use it (Claude Code skills doc, verbatim table — "When skills share the same name across levels, enterprise overrides personal, and personal overrides project"):

| Location | Path | Applies to |
| -------- | ---- | ---------- |
| Enterprise | (managed settings) | All users in your organization |
| Personal | `~/.claude/skills/<skill-name>/SKILL.md` | All your projects |
| Project | `.claude/skills/<skill-name>/SKILL.md` | This project only |
| Plugin | `<plugin>/skills/<skill-name>/SKILL.md` | Where plugin is enabled |

**Nested + parent discovery** (verbatim): *"Project skills load from `.claude/skills/` in your starting directory and in every parent directory up to the repository root … Claude Code also discovers skills from nested `.claude/skills/` directories on demand."* (Monorepo support.) The SDK doc echoes this for `cwd`: skills load from `.claude/skills/` in `cwd` and every parent up to the repository root.

### 1.3 The minimal SKILL.md

The smallest useful skill is frontmatter (only `description` recommended) plus a Markdown body:

```markdown
---
name: api-conventions
description: API design patterns for this codebase. Use when writing or reviewing API endpoints.
---

When writing API endpoints:
- Use RESTful naming conventions
- Return consistent error formats
- Include request validation
```

`name` defaults to the directory name if omitted; `description` defaults to the first paragraph of the body if omitted. Both are technically optional, but `description` is what the model matches against to decide to invoke (platform overview: required fields are `name` and `description`; CLI doc: "Only `description` is recommended").

### 1.4 SKILL.md frontmatter fields (verbatim)

Two field sets exist because skills span surfaces. The **Claude Code (CLI/SDK) frontmatter reference** is the superset (`code.claude.com/docs/en/skills`, verbatim field list): `name`, `description`, `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `model`, `effort`, `context`, `agent`, `hooks`, `paths`, `shell`. All optional.

Field constraints worth pinning:

- `name` — "Lowercase letters, numbers, and hyphens only (max 64 characters)." Platform overview adds: cannot contain XML tags; cannot contain reserved words "anthropic", "claude".
- `description` — CLI: combined `description` + `when_to_use` "truncated at 1,536 characters in the skill listing." **Platform/API SKILL.md caps `description` at 1024 characters** (divergence — the API validates the field at 1024; the CLI's 1,536 is the *combined listing* budget). State which surface when authoring.
- `disable-model-invocation: true` — only the user can invoke (`/name`); the description is dropped from context. Also prevents preload into subagents.
- `user-invocable: false` — only Claude can invoke; hidden from the `/` menu.
- `allowed-tools` — pre-approves tools while the skill is active. **SDK note below: ignored by the SDK.**
- `context: fork` + `agent: <type>` — run the skill in a forked subagent (the SKILL.md body becomes the subagent prompt). Ties skills to the subagent mechanism (topic 06).
- `paths` — glob patterns that gate automatic activation (same format as `.claude/rules/` path-specific rules).

The platform/API minimal frontmatter is the strict subset — `name` + `description` only:

```yaml
---
name: pdf-processing
description: Extract text and tables from PDF files, fill forms, merge documents. Use when working with PDFs.
---
```

### 1.5 How the model invokes a skill — the `Skill` tool

Skills are **model-invoked**, not host-forced (the SDK doc lists "Model-invoked: Claude autonomously chooses when to use them based on context"). The vehicle is the built-in `Skill` tool (topic 05 inventory). Per topic 05 line 54: `Skill` "Invoke a `.claude/skills/*/SKILL.md` skill (auto-enabled when the `skills` option is set)."

Invocation paths:
- **Automatic** — the model reads the Level-1 metadata in its context and calls the `Skill` tool when a request matches a `description`.
- **Direct (`/skill-name`)** — a user invokes by name (CLI surface; the directory name becomes the command).

On trigger, the body enters context as a single message and **stays for the rest of the session** (Claude Code skills doc, "Skill content lifecycle": *"the rendered `SKILL.md` content enters the conversation as a single message and stays there … Claude Code does not re-read the skill file on later turns"*). This is the same model-decided dispatch posture as subagents (topic 06 §2): the host steers via descriptions and the `skills` filter, it does not force "run skill X now" from code in a single `query()`.

---

## 2. Programmatic skills — NOT supported

**Skills cannot be registered in code.** This is the load-bearing negative for ArkOS. SDK skills doc, verbatim:

> Unlike subagents (which can be defined programmatically), Skills must be created as filesystem artifacts. The SDK does not provide a programmatic API for registering Skills.

The only skill-related option on `ClaudeAgentOptions` / `Options` is the **`skills` filter** (§3), which selects among *already-discovered filesystem* skills — it does not accept inline skill definitions. There is no `skills={"name": SkillDefinition(...)}` analog to the `agents` map. A host that wants a skill available must write a `SKILL.md` to disk under a `settingSources`-visible directory (or ship it via a plugin path — see the `plugins` option below).

> **For ArkOS:** behavior packs delivered as skills must be materialized on disk. Ark already writes `.claude/skills/` (and `.claude/commands/`, `.claude/agents/`) from embedded templates — that filesystem-first model matches the SDK's only supported skill path. There is no in-process skill registry to lean on.

---

## 3. `settingSources` and what loads (skill/CLAUDE.md slice; cross-ref topic 02 §8)

Topic 02 §8 owns the full table. Restated precisely for this file's surfaces:

**The option:** `setting_sources: list[SettingSource] | None` (Python, default `None`) / `settingSources: SettingSource[]` (TS). Values: `"user"`, `"project"`, `"local"`.

**Defaults:**
- **Omit** `settingSources` ⇒ `query()` loads as the CLI does, equivalent to `["user", "project", "local"]` — CLAUDE.md, skills, commands, agents, and `settings.json` all load.
- Pass `[]` ⇒ none of user/project/local load: no CLAUDE.md, no filesystem skills, no commands, no filesystem agents.
- Pass an explicit list ⇒ exactly those sources.

**Skill-specific gating** (SDK skills doc `<Note>`, verbatim):

> Skills are discovered through the filesystem setting sources. With default `query()` options, the SDK loads user and project sources, so skills in `~/.claude/skills/`, `<cwd>/.claude/skills/`, and `.claude/skills/` in any parent directory of `<cwd>` up to the repository root are available. If you set `settingSources` explicitly, include `'user'` or `'project'` to keep skill discovery, or use the [`plugins` option] to load skills from a specific path.

So skill discovery specifically requires `"user"` and/or `"project"` in the active sources. The troubleshooting section makes the failure mode explicit:

```python
# Skills NOT loaded: setting_sources excludes user and project
options = ClaudeAgentOptions(setting_sources=[], skills="all")

# Skills loaded: user and project sources included
options = ClaudeAgentOptions(setting_sources=["user", "project"], skills="all")
```

**The `skills` option (the filter layer).** Distinct from `settingSources`. Verbatim:

> Set the `skills` option on `query()` to control which Skills are available to the session. When omitted, discovered Skills are enabled and the Skill tool is available, matching CLI behavior. Pass `"all"` to enable every discovered Skill, a list of Skill names to enable only those, or `[]` to disable all. When you set `skills`, the SDK enables the Skill tool automatically, so you do not need to list it in `allowedTools`.

```python
# Python — enable all discovered skills
options = ClaudeAgentOptions(
    setting_sources=["user", "project"],   # discovery
    skills="all",                          # filter: enable every discovered skill
    allowed_tools=["Read", "Write", "Bash"],
)
# or enable only named skills:
options = ClaudeAgentOptions(skills=["pdf", "docx"])
```

```typescript
// TypeScript — equivalent
const options = {
  settingSources: ["user", "project"],   // discovery
  skills: "all",                         // filter
  allowedTools: ["Read", "Write", "Bash"],
};
// or: { skills: ["pdf", "docx"] }
```

**Filter is not a sandbox** (verbatim): *"The `skills` option is a context filter, not a sandbox. Unlisted Skills are hidden from the model and rejected by the Skill tool, but their files remain on disk and are reachable through Read and Bash."*

**Names match the `name` field in SKILL.md or the directory name; use `plugin:skill` for plugin-provided skills.**

**`plugins` option escape hatch.** When `settingSources` is `[]` (e.g. multi-tenant isolation) but you still want specific skills, the docs point to the `plugins` option to load skills from an explicit path rather than via setting-source discovery.

**Multi-tenant warning (cross-ref topic 02 §8).** Even with `settingSources: []`, three channels still load regardless: managed policy settings, `~/.claude.json` global config, and auto-memory (`~/.claude/projects/<project>/memory/`). Topic 02 carries the verbatim `<Warning>`: for multi-tenant isolation, run each tenant in its own filesystem and set `settingSources: []` plus `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1` in `env`. Setting `settingSources: []` disables filesystem *skills* but does NOT disable auto-memory — these are separate channels.

---

## 4. AGENTS.md status — definitively NOT loaded by this SDK

**Confirmed: the Claude Agent SDK / Claude Code reads `CLAUDE.md`, not `AGENTS.md`.** This closes the open flag topic 02 §8 raised ("Not found: no evidence the Claude Agent SDK loads AGENTS.md").

The Claude Code memory doc (`code.claude.com/docs/en/memory`) has a dedicated **`### AGENTS.md`** section, verbatim:

> Claude Code reads `CLAUDE.md`, not `AGENTS.md`. If your repository already uses `AGENTS.md` for other coding agents, create a `CLAUDE.md` that imports it so both tools read the same instructions without duplicating them. You can also add Claude-specific instructions below the import. Claude loads the imported file at session start, then appends the rest:

```markdown
@AGENTS.md

## Claude Code

Use plan mode for changes under `src/billing/`.
```

> A symlink also works if you don't need to add Claude-specific content:

```bash
ln -s AGENTS.md CLAUDE.md
```

And for `/init`: *"Running `/init` in a repo that already has an `AGENTS.md` reads it and incorporates the relevant parts into the generated `CLAUDE.md`. It also reads other tool configs like `.cursorrules` and `.windsurfrules`."*

**Interpretation, precise:**
- AGENTS.md is **never read directly** by the SDK/CLI as a project-instruction source. It is not in any `settingSources` loader.
- The **only** documented way AGENTS.md content reaches the model is *indirectly*: a `CLAUDE.md` that does `@AGENTS.md` (the `@`-import expands it at session start, §5.2), or a `CLAUDE.md → AGENTS.md` symlink, or a one-time `/init` ingestion. In all three the loaded file is still `CLAUDE.md`.

> **For ArkOS (RFC names AGENTS.md as a portability surface):** AGENTS.md is portable *toward* Claude only through a `CLAUDE.md` shim. If ArkOS authors a single `AGENTS.md` as the cross-runtime instruction file, the Claude runtime will not read it unless a sibling `CLAUDE.md` imports or symlinks it. This is exactly Ark's existing template posture (Ark ships `CLAUDE.md` for Claude and `AGENTS.md` for Codex/OpenCode as parallel files). Do not assume a lone AGENTS.md is seen by the SDK — verify by shipping the `@AGENTS.md` shim or a symlink.

---

## 5. CLAUDE.md loading

### 5.1 When/where it loads, and into what

CLAUDE.md is gated by `settingSources` and injected into the **conversation**, not the system prompt (modifying-system-prompts doc, verbatim):

> CLAUDE.md takes a different path: the SDK reads it and injects its content into the conversation as project context, not into the system prompt, so it shapes behavior alongside whichever system prompt you choose.

> The SDK reads CLAUDE.md when the matching setting source is enabled: `'project'` loads `CLAUDE.md` or `.claude/CLAUDE.md` from the working directory, and `'user'` loads `~/.claude/CLAUDE.md`. Default `query()` options enable both sources, so CLAUDE.md loads automatically. … CLAUDE.md loading is controlled by setting sources, not by the `claude_code` preset.

So CLAUDE.md loads regardless of `systemPrompt` choice (minimal default, `claude_code` preset, or custom string) — it rides on the setting source, not the prompt. *"It is not loaded if you pass an empty `settingSources` array."* The memory doc adds the delivery mechanic: *"CLAUDE.md content is delivered as a user message after the system prompt, not as part of the system prompt itself."* (Consequence: it is context, not enforced configuration.)

```python
# Python — load project CLAUDE.md alongside the claude_code preset
options = ClaudeAgentOptions(
    system_prompt={"type": "preset", "preset": "claude_code"},
    setting_sources=["project"],   # loads ./CLAUDE.md or ./.claude/CLAUDE.md
)
```

```typescript
const options = {
  systemPrompt: { type: "preset", preset: "claude_code" },
  settingSources: ["project"],   // loads ./CLAUDE.md or ./.claude/CLAUDE.md
};
```

### 5.2 The `@path` import syntax

CLAUDE.md can pull in other files (memory doc, verbatim):

> CLAUDE.md files can import additional files using `@path/to/import` syntax. Imported files are expanded and loaded into context at launch alongside the CLAUDE.md that references them.

> Both relative and absolute paths are allowed. Relative paths resolve relative to the file containing the import, not the working directory. Imported files can recursively import other files, with a maximum depth of **five hops**.

Examples (verbatim):

```text
See @README for project overview and @package.json for available npm commands for this project.

# Additional Instructions
- git workflow @docs/git-instructions.md
```

Home-dir imports work (`@~/.claude/my-project-instructions.md`) — the documented way to share personal instructions across worktrees. First-time external imports trigger an approval dialog. This `@`-import is the mechanism behind the AGENTS.md shim (§4). Note: imports do **not** save context — *"imported files still load and enter the context window at launch."* Block-level HTML comments are stripped before injection.

### 5.3 Precedence and nesting

**Memory hierarchy, load order broadest→most-specific** (memory doc table). Files are **concatenated, not overridden** — later-loaded content is read last and effectively wins ties:

| Scope | Location | Notes |
| ----- | -------- | ----- |
| Managed policy | macOS `/Library/Application Support/ClaudeCode/CLAUDE.md`; Linux/WSL `/etc/claude-code/CLAUDE.md`; Windows `C:\Program Files\ClaudeCode\CLAUDE.md` | Loads before user/project; cannot be excluded by `claudeMdExcludes`. Also settable inline via `claudeMd` in managed-settings.json |
| User | `~/.claude/CLAUDE.md` | gated by `"user"` |
| Project | `./CLAUDE.md` or `./.claude/CLAUDE.md` | gated by `"project"` |
| Local | `./CLAUDE.local.md` | gated by `"local"`; gitignore it |

**Directory-tree walk** (verbatim): *"content is ordered from the filesystem root down to your working directory … instructions closer to where you launched Claude are read last. Within each directory, `CLAUDE.local.md` is appended after `CLAUDE.md`."* Subdirectory CLAUDE.md files below cwd load **on demand** when Claude reads files there, not at launch.

Adjacent: `.claude/rules/*.md` load with the same priority as `.claude/CLAUDE.md` (user rules before project rules); `paths:` frontmatter scopes a rule to matching globs. `claudeMdExcludes` (glob list, any settings layer, arrays merge) skips ancestor CLAUDE.md files; managed-policy CLAUDE.md cannot be excluded.

---

## 6. Portability across SDK and CLI

The governing claim is from the overview (topic 01 §"What translates 1:1"): *"Workflows translate directly between them."* Filesystem behavior packs authored for the Claude Code CLI work unchanged in the SDK **provided `settingSources` enables them**.

**Portable unchanged (CLI ↔ SDK), same files:**

| Artifact | Path | Portability note |
| -------- | ---- | ---------------- |
| Skills | `.claude/skills/<name>/SKILL.md` | same files; SDK discovers via `"user"`/`"project"` sources (§3) |
| CLAUDE.md / rules | `CLAUDE.md`, `.claude/CLAUDE.md`, `.claude/rules/*.md` | same files; conversation-injected (§5) |
| Subagents | `.claude/agents/*.md` | same files; SDK discovers via `settingSources` (topic 06 §1.4) |
| Slash commands | `.claude/commands/*.md` | same files; loaded when `settingSources` includes the owning source. Note (CLI skills doc): "Custom commands have been merged into skills" — a `.claude/commands/deploy.md` and a `.claude/skills/deploy/SKILL.md` both create `/deploy`; on name collision the skill wins |

**What is NOT portable / has caveats:**

1. **`allowed-tools` frontmatter in SKILL.md is SDK-ignored.** SDK skills doc `<Note>`, verbatim:
   > The `allowed-tools` frontmatter field in SKILL.md is only supported when using Claude Code CLI directly. **It does not apply when using Skills through the SDK**. When using the SDK, control tool access through the main `allowedTools` option in your query configuration.
   A skill that relies on `allowed-tools` to pre-approve `Bash(git *)` in the CLI will silently fall back to the session's `allowedTools` / `canUseTool` in the SDK. This is the one concrete skill-portability gap.

2. **Direct `/skill-name` invocation is a CLI/REPL UI affordance.** In the SDK, skills are model-invoked; there is no `/`-typed entry. (A host can still steer the prompt to name the skill — soft, not forced, like subagent dispatch in topic 06 §2.)

3. **`settingSources` must be opted into.** The CLI loads filesystem settings always; the SDK does so only when `settingSources` is omitted or includes the source. SDK-wide the *raw option default* was changed to "no filesystem settings" in 0.1.0; `query()` re-applies CLI defaults on omission (topic 02 §8). Passing `[]` for isolation disables every portable artifact above.

4. **Live reload differs.** The CLI watches skill directories and picks up edits within a session; filesystem subagents are "loaded at startup only … restart the session to load it" (topic 06 §1.4). An SDK `query()` is per-call, so reload is moot per call but matters for long-lived `ClaudeSDKClient` sessions.

5. **CLI-only / SDK-only surfaces (topic 01).** Output styles activate via `/config` (CLI) or `settings`/`outputStyle` (SDK; the Python SDK has *no* programmatic output-style selector — use `append` or a custom prompt). Programmatic `AgentDefinition`, `tool` decorator / `createSdkMcpServer`, `HookCallback`, `CanUseTool` are SDK-only (topic 01 §SDK-only).

**Cross-surface skill portability beyond Claude Code** (platform overview): the same SKILL.md concept spans claude.ai, the Claude API, and Claude Code, but they do **not auto-sync** — verbatim: *"Custom Skills do not sync across surfaces … Skills uploaded to claude.ai must be separately uploaded to the API … Claude Code Skills are filesystem-based and separate from both."* Claude Code uses only Custom (filesystem) skills, not the pre-built API skills (pptx/xlsx/docx/pdf). Runtime constraints also differ: API skills have no network access / no runtime package install; Claude Code skills have full local network access. The CLI skills doc frames skills as following the [Agent Skills](https://agentskills.io) open standard "which works across multiple AI tools," with Claude Code extensions (invocation control, subagent execution, dynamic context injection) layered on top.

> **For ArkOS:** filesystem skills/agents/commands/CLAUDE.md are the genuinely portable layer between Ark's CLI templates and an SDK-driven runtime — the *same files* load in both, contingent only on `settingSources`. The portability surface is the filesystem, not a programmatic registry. AGENTS.md is portable to Claude only via the `@AGENTS.md` CLAUDE.md shim (§4). The SKILL.md `allowed-tools` field does not survive into the SDK; encode tool gating in the SDK `allowedTools`/`canUseTool` layer instead.

---

## 7. Skills vs subagents vs MCP tools — when to use each

These are three distinct behavior-packaging mechanisms; the RFC's "skill-style behavior packs" map specifically to **skills**.

- **Skill** = a model-invoked **instruction/procedure/knowledge pack** that loads into the *current* conversation context (progressive disclosure: metadata always, body on trigger, files on demand). It does not get its own context window or its own tools (beyond the active session's). It is the right unit for "a repeatable procedure or a body of conventions the model should apply inline" — checklists, style guides, multi-step workflows, domain knowledge. Filesystem-only in the SDK; no code registration (§2). Portable as plain `SKILL.md` files (§6).

- **Subagent** (topic 06) = a **delegated worker** the model dispatches via the `Agent` tool. It runs in a *fresh, isolated context window*, can have its own model/tools/permissions/skills, and returns its final message as text to the parent. Use it when you want verbose work kept out of the parent context and a summarized result back — leaf fan-out, exploration. Depth is 1 (subagents can't spawn subagents); dispatch is model-decided. Can be defined programmatically (`AgentDefinition`) *or* as filesystem cards. (A skill with `context: fork` is the bridge — it runs its body *as* a subagent.)

- **MCP tool** (topic 07) = an **external capability provider** (tools/resources/prompts) the agent *calls*, connected over stdio/HTTP/SSE or via in-process `createSdkMcpServer`. Use it when the behavior is an *action against an external system* (a database, an API, a service) that must run as deterministic code, not as model-followed prose. MCP gives the model new verbs; skills give it new procedures for verbs it already has; subagents give it a fresh worker to run those procedures in isolation.

Rule of thumb: **knowledge/procedure → skill; isolated delegated work → subagent; external tool/integration → MCP.** For ArkOS portable behavior packs (the RFC's framing), skills are the matching primitive — but they are filesystem-bound, model-invoked, and (in the SDK) un-registerable in code, so the substrate must materialize them on disk and steer invocation through `description`s and the `skills` filter rather than forcing them programmatically.

---

## External references

- [Agent Skills in the SDK (code.claude.com)](https://code.claude.com/docs/en/agent-sdk/skills) — primary: "Skills must be created as filesystem artifacts; the SDK does not provide a programmatic API for registering Skills"; the `skills` option (`"all"`/list/`[]`, auto-enables Skill tool); `settingSources` discovery `<Note>`; `allowed-tools` SDK-ignored `<Note>`; Python+TS examples; `plugins` escape hatch.
- [Extend Claude with skills (code.claude.com/docs/en/skills)](https://code.claude.com/docs/en/skills) — SKILL.md frontmatter reference (full field list, constraints), directory/nesting/parent discovery, locations + precedence table, lifecycle, `context: fork`, dynamic `` !`cmd` `` injection, "custom commands merged into skills," `disableSkillShellExecution`.
- [Modifying system prompts (code.claude.com/docs/en/agent-sdk/modifying-system-prompts)](https://code.claude.com/docs/en/agent-sdk/modifying-system-prompts) — CLAUDE.md injected into conversation not system prompt; setting-source gating; `'project'`→`CLAUDE.md`/`.claude/CLAUDE.md`, `'user'`→`~/.claude/CLAUDE.md`; not loaded with `[]`; Python+TS load examples.
- [How Claude remembers your project (code.claude.com/docs/en/memory)](https://code.claude.com/docs/en/memory) — **the AGENTS.md section ("Claude Code reads `CLAUDE.md`, not `AGENTS.md`")**; `@path` import syntax, depth-5 recursion, relative/absolute/home paths; memory-hierarchy load order + precedence; concatenation rule; `claudeMdExcludes`; `.claude/rules/` + `paths:`; managed `claudeMd`.
- [Agent Skills overview (platform.claude.com)](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview) — progressive-disclosure 3-level table; SKILL.md required fields (`name` ≤64 chars, `description` ≤1024 chars, reserved-word rules); cross-surface (claude.ai / API / Claude Code) availability + no-sync + runtime constraints; "Claude Code supports only Custom Skills."
- [Subagents in the SDK (code.claude.com/docs/en/agent-sdk/subagents)](https://code.claude.com/docs/en/agent-sdk/subagents) — referenced for the subagent contrast in §7 (owned by topic 06).
- Agent Skills open standard — [agentskills.io](https://agentskills.io) — cited by the CLI skills doc as the cross-tool standard Claude Code skills follow (not independently audited here).

---

## Caveats / Not found

- **AGENTS.md: now CONFIRMED not-loaded** (resolves topic 02 §8's "Not found"). The memory doc states it verbatim and gives the `@AGENTS.md` import / symlink / `/init` ingestion as the only bridges. No `settingSources` value loads AGENTS.md directly.
- **No programmatic skill API** in either SDK at this snapshot — confirmed by the SDK skills doc's explicit statement. The `skills` option filters discovered filesystem skills only; it does not accept inline definitions. (Subagents differ — they have `AgentDefinition`.)
- **SKILL.md `description` length divergence:** platform/API caps the field at **1024** chars; the CLI's **1,536** is the combined `description`+`when_to_use` *listing* budget (configurable via `maxSkillDescriptionChars` / `skillListingBudgetFraction`). Different numbers, different scopes — pin per surface.
- **`allowed-tools` SKILL.md frontmatter is SDK-ignored** — the one concrete skill-portability gap CLI→SDK. Use the SDK `allowedTools`/`canUseTool` layer instead.
- **`skills` is a filter, not a sandbox** — unlisted skills' files remain readable via Read/Bash. Not an isolation mechanism.
- **Cross-surface skills do not sync** — Claude Code (filesystem), claude.ai (zip upload), API (Skills API upload) are separate stores; Claude Code uses only Custom skills, not the pre-built pptx/xlsx/docx/pdf API skills.
- **Did not inspect SDK source for skill internals** — skill discovery/loading is documented behavior, not exposed as a typed SDK function (unlike sessions in topic 02 §6); claims here rest on the docs, not source grep. The `skills` option name/values and the programmatic-skills negative are from the SDK skills doc; treat the doc as authoritative for this snapshot (Python 0.2.87 / TS 0.3.150, 2026-05-25).
- **agentskills.io open standard** cited by the doc but not independently verified here; the cross-tool-portability claim for SKILL.md beyond Anthropic surfaces rests on that doc statement.
- **Doc snapshot only** — `code.claude.com`/`platform.claude.com` pages do not print per-page "last updated" dates; version pin is via package versions as of 2026-05-25.
