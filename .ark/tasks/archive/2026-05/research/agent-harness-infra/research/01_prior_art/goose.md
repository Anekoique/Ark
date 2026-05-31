# Goose (codename goose)

## Identity

- **Name:** Goose (often "codename goose" in marketing material)
- **Repo:** https://github.com/block/goose (canonical; an org rename surfaced as `aaif-goose/goose` in some API responses — both URLs resolve to the same repo as of 2026-05)
- **License:** Apache 2.0
- **Primary maintainer:** Block (formerly Square, Inc.); positioned as Block Open Source.
- **Language:** Rust (core/CLI) + Electron+React (desktop)
- **Stars / momentum:** 45,578 stars (as of 2026-05-20, via `gh repo view`). Block launched it as an open framework in early 2025; significant velocity in 2026 around recipes, subagents, and skills unification.
- **Homepages:** https://goose-docs.ai, https://block.github.io/goose

## Positioning

Goose is the **Rust-native, model-agnostic** answer to Claude Code from a non-AI company (Block runs payments). The pitch: a general-purpose AI agent — not just for code — that you point at any LLM (Claude, OpenAI, Gemini, Ollama, …) and any MCP server, get a CLI + desktop UI, and run real engineering tasks end-to-end (install software, edit files, run tests). It's the closest peer to Ark in *language* (Rust) and in *philosophy* (build the harness layer well, swap models freely). Recipes (portable YAML configs), subagents (parallel-spawned isolated agents), and skills (custom context bundles) are its three composability primitives.

## Primitives

User-facing nouns:

- **Session** — an interactive chat; can be named (`-n my-session`), forked, resumed.
- **Extension** — an MCP server (builtin Rust, platform-bound, or external stdio/HTTP). Extensions provide tools, prompts, resources.
- **Recipe** — a YAML config bundling instructions, extensions, parameters, optional subrecipes. Sharable, runnable in CI.
- **Subrecipe** — a recipe invoked from another recipe.
- **Subagent** — an in-process spawned Agent instance with isolated context, ExtensionManager, ToolMonitor.
- **Skill** — a SKILL.md-format custom-context module (aligned with the Claude Code skill standard).
- **Provider** — the LLM backend (anthropic, openai, gemini, ollama, etc.).
- **Tasks Manager** — internal coordinator for subagent task definitions.

User-facing verbs:

- `goose session start` / `resume` / `fork`
- `goose recipe run`, `goose recipe generate` (extract a recipe from a finished session)
- Slash commands inside a session: `/plan`, `/prompt`, `/recipe`, `/compact`, `/help`
- `goose configure` — provider/extension config TUI

## Workflow model

Representative flow:

1. **Configure** provider once (`goose configure` → pick Anthropic, set API key, pick MCP extensions).
2. **Start session**: `goose session start -n auth-refactor`.
3. **Chat** — Goose has tools (filesystem, shell, MCP extensions). It executes commands directly (no approval-by-default, configurable).
4. **(Optional) Plan mode** — `/plan` switches into read-only planning, similar to Cline.
5. **(Optional) Subagent dispatch** — Goose can ask its own LLM "should I delegate this?" and spawn a subagent with a focused prompt. The subagent runs in a separate Agent instance with its own session, returns a summary.
6. **(Optional) Compact** — `/compact` summarizes the session transcript when context fills up.
7. **(Optional) Save as recipe** — `/recipe` extracts the current session into a reusable YAML. The recipe captures instructions + extensions + parameters and can be re-run with different inputs.
8. **Commit** — manual `git commit` outside Goose; Goose doesn't own git lifecycle.

No formal PRD/PLAN/REVIEW/VERIFY phases. The recipe *is* the only formal artifact, and it's optional.

## Context & memory

**Per-session context** held in the conversation thread; `/compact` is the manual lever.

**Persistent memory:**

- Session JSONL files saved by name; resumable.
- Recipes (project-controlled, can be checked into git).
- Skills (markdown + assets, in `~/.config/goose/skills/` or per-project).
- Memory MCP extension (one of the builtin extensions) provides knowledge-graph storage.

**No native repo map / tree-sitter indexing.** Like Cline, Goose relies on on-demand file reads via tools.

**Context engineering** primarily lives in the *recipe* layer — recipes can pre-declare instructions and required extensions, so a fresh session running the recipe starts pre-configured.

## Tool / capability surface

**Built-in extensions (compiled-in MCP servers):**

- Developer (shell, file ops)
- Memory (knowledge graph)
- Computer controller (mouse/keyboard automation, OS-level)
- Tutorial / docs lookup
- JetBrains / VS Code bridges

**External extensions:** Any MCP server. Goose treats stdio and SSE MCP servers as first-class.

**MCP support:** Yes, deeply. Goose's entire extension model is built on MCP — even builtins implement `McpClientTrait`. They were among the earliest non-Anthropic adopters of MCP at scale.

**Plugin model:** Three layers:

- **Extensions** (MCP) — capability injection
- **Recipes** (YAML) — workflow / config injection
- **Skills** (SKILL.md) — context / behavior injection

**Sandbox boundaries:** None at the OS layer by default. Goose has a permission system (configurable per-tool auto-allow); the desktop GUI adds approval-by-default popups. No Docker-per-session.

## Integration model

**Two delivery surfaces:**

- **CLI** (`goose` binary, Rust) — terminal-first.
- **Desktop GUI** (Electron + React) — chat sidebar, multi-session, recipe browser.

The same backend (`goose-server`) powers both. Bridges to JetBrains and VS Code exist as MCP extensions (the IDE *is* an extension target, not Goose's host).

## Multi-agent / orchestration

**First-class subagents.** Implementation: `Agent::new()` creates a new instance with its own `ExtensionManager`, `ToolMonitor`, communication channels, and isolated context. The `TasksManager` coordinates task definitions and spawns "separate goose instances for execution (each in its own isolated session), and aggregates results back to the parent."

Subagents are typically used for:

- Parallel research ("look at these 5 files and summarize each")
- Specialized tools ("use the test runner extension; report results")
- Recovery loops ("if subagent fails, parent re-dispatches with refined prompt")

The unification of *recipes* + *subagents* + *skills* + *subrecipes* under a single "task execution" model is an active design discussion (see `block/goose` Discussion #6202).

## Spec / artifact system

**Recipes** are the artifact system. YAML, checked into git, parameterized, runnable in CI. A recipe captures:

- `instructions` — system prompt
- `prompt` — user prompt (with `{{parameter}}` templating)
- `extensions` — MCP servers to start
- `parameters` — typed inputs
- `subrecipes` — nested recipe invocations

This is **closer to Cursor's "rules" + Continue's "agents" than to Ark's PRD/PLAN.** Recipes describe *how* to do something, not *what we're doing* in a specific task.

**No SPEC promotion** — recipes are authored, not extracted from finished work (though `/recipe` extracts one from session history).

## Strengths

- **Rust core.** Fast cold start; small binary; pairs well with Ark's own engineering choices.
- **MCP-everything.** Even builtins are MCP. Cleanest unified plugin model in the field.
- **Recipes are genuinely reusable.** Capture a workflow once, run it in CI on hundreds of repos.
- **Subagents with full isolation** (separate ExtensionManager per subagent) — strong architecture.
- **`/compact` and `/recipe` slash commands** show good session-hygiene UX.
- **Model agnosticism.** Switch providers via `goose configure` without rewriting prompts.
- **Backed by a real company (Block) but Apache-licensed and community-friendly.**

## Weaknesses / gaps

- **No tier/ceremony layered on top.** Recipes are flat — no "small task" vs "deep architectural task" distinction.
- **No PRD/PLAN/VERIFY workflow.** Recipes describe automation, not project context.
- **No SPEC layer.** Conventions = skills + recipe instructions; no spec extraction or promotion.
- **No worktree-equivalent multi-task isolation per checkout.**
- **Recipe YAML is verbose** for one-off work; the slash-command UX is more pleasant.
- **No journal.**
- **Documentation is split** between `goose-docs.ai` and `block.github.io/goose` — friction for newcomers.

## Directions for Ark

1. **Recipe-style YAML for `ark` task templates.** Ark currently scaffolds tasks from embedded templates by tier. A user-extensible "task recipe" — YAML describing tier, suggested PRD section seeds, required SPECs to cite, expected EXECUTE phase checks — would let teams capture project-specific task shapes ("internal-API-change", "doc-only-fix") and share them. Ark's `ark agent task new` would gain a `--recipe <name>` flag.
2. **`/compact` for `ark context`.** Goose's `/compact` is a single-keystroke way to handle context bloat. Ark could ship `ark context compact --task <slug>` that summarizes a long-running task's events/transcripts into a compressed `MEMO.md` at task close.
3. **Subagent ExtensionManager isolation.** Ark's subagent feature spec (`subagent-support`) gives researcher/reviewer/verifier their own agent processes, but they likely share tool config with the parent. Audit whether each subagent should have its own MCP server config / tool whitelist — Goose's `ExtensionManager` per subagent is the textbook isolation answer.
4. **Skills as a shared standard.** Goose, Codex, Claude Code, Cursor, and Gemini CLI all consume SKILL.md from `~/.agents/skills/`. Ark scaffolds `.claude/skills/` (and equivalents) but doesn't *itself* author skills as a Ark-tier artifact. Consider whether feature SPEC promotion should *also* emit a SKILL.md sibling for editors that prefer that format. Backwards-compatible: SPEC.md is the source of truth; SKILL.md is a generated view.
5. **Counter-positioning vs. Goose: "we do the *workflow*, you do the *automation*."** Goose's recipes are great at "automate this 50-step task." Ark's workflow is great at "manage a project's design + plan + review cadence over weeks." Use Goose as a tool *within* an Ark task ("execute the migration via this Goose recipe") rather than competing.

## Sources

- [block/goose on GitHub](https://github.com/block/goose) (queried 2026-05-20)
- [Goose docs (goose-docs.ai)](https://goose-docs.ai/)
- [Recipe Reference Guide](https://block.github.io/goose/docs/guides/recipes/recipe-reference/)
- [CLI Commands — goose docs](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Extension Types and Configuration | DeepWiki](https://deepwiki.com/block/goose/5.3-extension-types-and-configuration)
- [Goose Subagents — Advent of AI Day 11](https://www.nickyt.co/blog/advent-of-ai-day-11-goose-subagents-2n2/)
- [Block introduces "codename goose"](https://block.xyz/inside/block-open-source-introduces-codename-goose)
- [Unified Tooling for Recipes, Subrecipes, Claude Skills, Subagents — Discussion #6202](https://github.com/block/goose/discussions/6202)
