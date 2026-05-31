# Research: Learning Curve and Discoverability

- Query: How users discover features (slash command listings, `--help`, doc sites, mdBook, tutorials). The "two-day onboarding wall." Aider's docs as a model. Where Ark's `docs/book/` stands.
- Scope: mixed
- Date: 2026-05-20

## Findings

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `crates/ark-cli/src/main.rs` | The clap `#[command(...)]` annotations driving `--help` output. About 50 lines of metadata. |
| `crates/ark-cli/src/main.rs` (l. 27-31) | Top-level `about` string: "A simple CLI agent harness and development workflow for orchestrating AI-driven programming tasks". The most-read sentence in Ark. |
| `crates/ark-cli/src/main.rs` (l. 38-66) | Subcommand list. Doc comments above each variant become the `--help` lines. |
| `crates/ark-cli/src/agent_cli.rs` | The hidden `ark agent` namespace. Doesn't appear in `ark --help`. |
| `docs/book/book.toml` | mdBook configuration: title "Ark", navy default theme, edit-url-template pointing at GitHub. |
| `docs/book/src/SUMMARY.md` | Table of contents — drives the sidebar. |
| `docs/book/src/getting-started/` | quick-start.md, installation.md, first-task.md — the on-ramp. |
| `docs/book/src/workflow/` | lifecycle.md, tiers.md, specs.md, worktrees.md — the conceptual core. |
| `docs/book/src/reference/` | One page per CLI verb (`ark-init.md`, `ark-load.md`, etc.) plus `cli-overview.md` and `config-toml.md`. |
| `docs/book/src/contributing/` | adding-a-platform.md, adding-a-slash-command.md, release-process.md, workspace-layout.md. |
| `docs/book/src/platforms/` | claude-code.md, codex.md, opencode.md — per-integration deep dives. |
| `templates/claude/commands/ark/quick.md` etc. | The slash-command bodies themselves. Read by Claude Code as the command's implementation. |
| `.ark/workflow.md` | 394 lines. The single most important doc inside an installed project. |
| `README.md` | 105 lines. Marketing front-door. |
| `AGENTS.md` | Two audiences (agents working in the Ark repo + users of the published CLI by negation). 149 lines. |

### Code patterns

**`ark --help` output is shaped by clap's defaults.** Each subcommand's doc comment becomes its summary line. From `crates/ark-cli/src/main.rs:39-66`:

```rust
/// Scaffold `.ark/` and Claude Code integration from the embedded templates.
Init(InitArgs),
/// Bring Ark into a project: restore from `.ark.db` if present, else scaffold.
Load(LoadArgs),
/// Freeze Ark state into `.ark.db` and remove the live files.
Unload(TargetArgs),
/// Remove Ark from the project, including any `.ark.db` snapshot.
Remove(TargetArgs),
/// Refresh embedded templates to the current CLI version.
Upgrade(UpgradeArgs),
/// Print a structured snapshot of git + .ark/ workflow state.
Context(ContextArgs),
```

Each line is a sentence-fragment description, ~10 words. Adequate but information-thin: no mention that `init` requires a TTY for the platform prompt, no hint that `unload` should be paired with `load`, no link to mdBook.

**`ark agent` is hidden** (`crates/ark-cli/src/main.rs:62-65`):

```rust
/// Internal commands invoked by the Ark workflow and slash commands.
/// Not covered by semver — prefer the slash commands over calling these directly.
#[command(hide = true)]
Agent(AgentArgs),
```

`ark --help` doesn't list `agent` at all. `ark agent --help` works but the entire subtree is intentionally undocumented in the top-level help. The principle: users should discover workflow operations through slash commands, not through `ark agent task new` directly.

**The mdBook structure** (from `docs/book/src/SUMMARY.md` referenced as the canonical order):

```
Introduction
Getting Started
  - Installation
  - Quick Start
  - Your First Task
Workflow
  - Lifecycle
  - Tiers
  - Specs
  - Worktrees
Platforms
  - Claude Code
  - Codex
  - OpenCode
Reference
  - CLI Overview
  - ark init / load / unload / remove / upgrade / context / agent
  - config.toml
Contributing
  - Adding a Platform
  - Adding a Slash Command
  - Workspace Layout
  - Release Process
```

Five sections, roughly canonical for a CLI tool's docs site. The Getting Started → Workflow → Reference path is the "diátaxis" pattern (tutorial → how-to → reference). Contributing-section presence is uncommon for projects in alpha; it signals the project's stance on external contributors.

### External references

#### Aider's docs as a model

Aider's documentation is at [aider.chat/docs](https://aider.chat/docs). The structure:

- **Top-level Usage page** with installation, basic in-chat commands.
- **Detailed Commands reference** listing `/add`, `/drop`, `/clear`, `/undo`, `/tokens`, `/help`, `/architect`, `/code`, `/ask`, etc.
- **FAQ** for common questions.
- **Troubleshooting** with structured paths for token limits, model issues, support.
- **LLM Leaderboards** — performance comparisons across models.
- **Tutorial videos** page linking community videos.

The documentation is broad. The in-chat experience also surfaces docs: `/help <question>` lets users ask aider questions about aider itself, with a built-in retrieval step. "Users can use the `/help` command to see in-chat commands, and they can run `/help <question>` to ask for help about using aider, customizing settings, troubleshooting, and using LLMs" ([aider *Using /help* docs](https://aider.chat/docs/troubleshooting/support.html)).

The combination of (a) static docs site, (b) in-chat command reference, (c) AI-mediated help is unusual. Most CLI tools stop at (a) and (b).

#### Claude Code's slash command discovery

Claude Code emits a slash-command list on `/` autocomplete in the TUI. Users discover commands by typing `/` and seeing the dropdown. New `/init`, `/cost`, `/clear`, `/compact`, etc. each have one-line descriptions.

The discoverability model: Claude Code surfaces slash commands at the input prompt level, not behind a `--help` flag. Ark's slash commands (`/ark:quick`, `/ark:design`, `/ark:research`, `/ark:commit`) inherit this discoverability for free — they appear in the autocomplete dropdown because they live in `.claude/commands/ark/`.

The Codex equivalent is "skills" (e.g. `.codex/skills/ark-quick/SKILL.md`); the OpenCode equivalent is `.opencode/commands/`. Both surface in their respective tool's UI.

#### CLI `--help` quality patterns

From the Rust ecosystem (clap docs, [Kevin Knapp's CLI design notes](https://kbknapp.dev/rust-cli/)):

- Top-level summary should fit on one screen.
- Each subcommand needs a one-line description.
- Long descriptions go in `--help` (long form) vs `-h` (short).
- `did you mean` suggestions when a typo is detected ("Suggestions: Suggests corrections when the user enters a typo. For example, if you defined a --myoption argument, and the user mistakenly typed --moyption (notice y and o transposed), they would receive a Did you mean '--myoption'?" — [Kevin Knapp's blog](https://kbknapp.dev/rust-cli/)).

Ark uses clap, gets the typo suggestions for free, has one-line descriptions on each subcommand. The remaining gap: `ark --help` doesn't link to the docs site. Most modern Rust CLIs (cargo, rustup, ripgrep) include a `For more information, try '--help'` footer; ark could add `For full docs: https://anekoique.github.io/ark/`.

#### mdBook as a Rust documentation pattern

mdBook is the canonical Rust-ecosystem docs generator. Used by The Rust Book, the Cargo Book, the Rustonomicon, dozens of crates. Ark follows convention: `docs/book/` with `book.toml`, `src/`, navy theme, edit-url linking back to GitHub.

mdBook's strengths: chapter-by-chapter linear reading, sidebar TOC, search, code-block syntax highlighting, easy GitHub Pages deployment. Weaknesses: no API reference auto-generation (compare to `rustdoc` for the library crates), no versioning out of the box (you have to maintain `mdbook serve` per release branch).

Ark's `docs/book/` is structurally sound. The content density is moderate — short pages, mostly conceptual. The `getting-started/quick-start.md` and `workflow/tiers.md` pages do the heaviest lifting; reference pages mostly mirror `--help` content.

#### The "two-day onboarding wall"

The phrase appears in developer-experience literature ([Fullscale, *Fast Developer Onboarding*](https://fullscale.io/blog/fast-developer-onboarding-framework/)): new developers hit a productivity wall around day 2 when they've exhausted the documented happy path and need to figure out the undocumented parts.

For a tool like Ark, the day-2 questions are:

- "How do I edit a PLAN after `task plan` has been invoked?"
- "What happens if I run `task commit` with no staged work?"
- "Can I revert a `task commit`?"
- "How do I delete a task I started by mistake?"
- "Why didn't the slash command find my project SPEC?"
- "What's the difference between `/ark:design` and `/ark:design --deep`?"

Some of these have explicit answers in `.ark/workflow.md` (e.g. the `NothingStaged` recovery is named in the error message). Others require code reading or trial-and-error.

The remediation pattern in best-in-class tools:

- **FAQ pages** that lead with the actual user question, not a topic taxonomy ("How do I…?" not "Tasks").
- **Error messages that link to docs** ("see https://anekoique.github.io/ark/troubleshooting/nothing-staged").
- **Interactive tutorials** (`rustlings`, `nimble`'s tutorial walk-through, `cargo`'s book chapter 1).
- **In-tool help** (aider's `/help <question>`).

Ark hits some of these. The error messages name recovery actions but don't link to docs. The workflow doc serves as a partial FAQ but is structured as a reference, not as a Q&A.

#### The "first command" question

A new user types `ark` and gets... what? In Ark today:

```
$ ark
error: 'ark' requires a subcommand but one was not provided
[subcommands: init, load, unload, remove, upgrade, context, archive, cleanup, help]
Usage: ark <COMMAND>

For more information, try '--help'.
```

(Approximate, given clap's defaults.) The output is competent but unfriendly to a brand-new user. Compare to `cargo`'s bare output:

```
Rust's package manager

Usage: cargo [+toolchain] [OPTIONS] [COMMAND]
...
```

`cargo` opens with "Rust's package manager." `ark` opens with "error." The first impression differs.

This is fixable with `Cli::default_subcommand_or_help` patterns or by making the bare `ark` invocation print the about-string + usage hint. clap supports both shapes.

#### Slash command discoverability inside agent platforms

| Platform | Discovery mechanism |
| --- | --- |
| Claude Code | `/` triggers autocomplete; commands from `.claude/commands/` appear. |
| Codex | Skills appear in the skill selector; SKILL.md files under `.codex/skills/`. |
| OpenCode | `/` triggers commands from `.opencode/commands/`. |
| Aider | `/help` command lists in-chat commands. |
| Cline | Sidebar UI; no global slash menu. |
| Continue.dev | `@`-mentions and slash commands; YAML config drives availability. |

Ark's design *defers* slash-command discoverability to the host platform. This is the right call — Ark would gain little by reinventing autocomplete. But it does mean Ark has no fallback if the user can't recall the command name. There's no `ark commands` to list `/ark:quick`, `/ark:design`, etc.; the canonical list lives in `templates/{claude,codex,opencode}/commands/`.

#### Tutorials and videos

The Ark mdBook has no tutorial videos. The README links to no tutorials. Aider has a "[Tutorial videos](https://aider.chat/docs/usage/tutorials.html)" page that links community-produced YouTube content. Claude Code has Anthropic-produced video content. Cursor has a YouTube channel.

Video is high-cost to produce but pays back disproportionately for tools whose value is interaction-shaped. The Ark workflow is hard to convey statically — the PLAN ⇄ REVIEW loop, the focus-binding semantics, the worktree dance — these benefit from screen recording in a way that static prose doesn't capture.

This is the largest off-the-shelf upgrade Ark's docs could see, but also the most expensive.

#### Where docs sites typically lose users

Based on common dev-tool docs feedback patterns:

- **No quickstart on the front page.** Ark gets this right — README has a Quick Start section.
- **Reference docs that explain what but not why.** Ark's `docs/book/src/reference/` mostly mirrors `--help`; the *why* lives in `workflow/`. The cross-link could be more explicit.
- **No "where to go next" guidance.** After reading Quick Start, what's the next page? Ark's SUMMARY.md has implicit ordering but no explicit "now read X" prompts.
- **Missing search.** mdBook has built-in search. Ark's `book.toml` enables it (`[output.html.search] enable = true`). Done.
- **No versioning.** Old links break when sections renumber. Ark's `edit-url-template` points to a specific branch (`main`); historical versions are not preserved as separate sites.

### Internal versus external doc surface

Inside an installed project, the user's encounter with Ark is mediated by:

1. **`CLAUDE.md` / `AGENTS.md` managed block** — first thing the agent reads.
2. **`.ark/workflow.md`** — the full procedural doc.
3. **`.ark/specs/INDEX.md`** + per-spec `SPEC.md` files — convention rules.
4. **Slash commands in `.claude/commands/ark/`** — the executable surface.
5. **`ark --help` output** — when the user types `ark <something>`.

Outside an installed project:

1. **README.md** at the GitHub root — discovery, install instructions.
2. **mdBook at the published URL** — full docs site.
3. **GitHub Issues / Discussions** — community.

The split is logical but means a user has to traverse both surfaces to learn the tool fully. Anyone who installs without ever reading the GitHub README — common when adopting through a colleague's recommendation — never sees the marketing front page.

### Where Ark's docs stand

**Strong:**
- mdBook structure is canonical.
- README Quick Start is concise (4-line install + scaffold + slash command).
- Workflow doc inside `.ark/` puts the rules where the user is working.
- Per-CLI-verb reference pages exist.
- `book.toml` enables search.
- Contributing section signals openness.

**Mediocre:**
- `ark --help` output is just the clap default; no docs link, no "what's new in this version."
- No FAQ page.
- No troubleshooting page indexed by error name.
- Reference pages mostly recap `--help`.
- No tutorial videos.
- No interactive tutorial (rustlings-style).

**Missing:**
- No "first task end-to-end walkthrough" with screenshots / output snippets, although `first-task.md` exists, its content depth is unknown without reading.
- No migration guides from other harnesses.
- No "common gotchas" page.
- No `ark commands` to list slash commands from the CLI (you have to look in templates dirs or run the platform).

### Caveats / Not found

- The actual content of each mdBook page beyond filenames was not surveyed in this research; just the structure.
- No traffic data on which mdBook pages get hits vs which are unread.
- Aider's `/help <question>` is described in their docs but the implementation depth (RAG vs in-context) wasn't traced.
- Whether Ark uses anchored URLs in error messages was checked — no, error messages don't include URLs (`crates/ark-core/src/error.rs`).
- Per-platform slash-command-list output (the Claude Code dropdown, the Codex skill selector) is platform-controlled; Ark has no control over the UX there.

## Directions for Ark

1. **Add a docs-site URL footer to every CLI subcommand `--help` output.** clap supports `after_help`:
   ```rust
   #[command(after_help = "For full docs: https://anekoique.github.io/ark/")]
   ```
   Single attribute, costs ~80 bytes of help output per subcommand, biggest discoverability win for users who learn by reading `--help`.

2. **Embed URLs in error messages.** `error.rs` has 60+ `Error` variants. Each `#[error(...)]` could include a URL stub: `"VERIFY.md at {path:?} has {items} pending… see https://anekoique.github.io/ark/troubleshooting/verify-incomplete"`. The URL doesn't need to exist for low-frequency errors — a one-paragraph "What is VerifyIncomplete?" page is cheap to write. For the common errors (`NothingStaged`, `NoFocus`, `IllegalPhaseTransition`), this closes the day-2 wall.

3. **Add a `troubleshooting/` chapter to mdBook indexed by error name.** Match the error type names verbatim: `troubleshooting/nothing-staged.md`, `troubleshooting/no-focus.md`, `troubleshooting/verify-incomplete.md`, etc. Even one paragraph per error is enough — the page exists for SEO and inbound search, not for narrative reading. Pairs with direction #2.

4. **Make `ark` (no args) print a friendly summary, not an error.** Replace the clap default with a custom about-block that includes the top three commands, an mdBook link, and a one-line "fresh project? `ark init`" hint. clap supports `arg_required_else_help(false)` for this. Improves first-impression UX at the cost of allowing typos like `ark statu` to fall through more silently.

5. **Author a single 5-minute screencast linked from the README.** Demonstrate one full task end-to-end (probably a `/ark:quick` and a `/ark:design`, showing the PRD → execute → verify → commit dance). Video is the only medium that captures the interaction-shape of the workflow. Highest-cost direction; highest-pay-off for awareness and adoption.
