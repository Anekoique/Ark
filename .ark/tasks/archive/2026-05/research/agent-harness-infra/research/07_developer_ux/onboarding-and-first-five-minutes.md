# Research: Onboarding and the First Five Minutes

- Query: What happens in the first five minutes when a new developer runs `ark init` (or comparable agent-harness installers); how it compares to mature CLI onboarding patterns; what's right and what's wrong in Ark today.
- Scope: mixed (internal source + external prior art)
- Date: 2026-05-20

## Findings

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `crates/ark-cli/src/main.rs` | Entry point. Defines `Cli`, `Command::Init`, `InitArgs`, and the `resolve_platforms_pure` / `interactive_select_platforms` pair that drives the first-run platform prompt. |
| `crates/ark-cli/src/main.rs` (l. 273-285) | `interactive_select_platforms`: prints "Select integrations to install:" and prompts `install <id> integration? [Y/n]` for each registered platform. Default-yes. |
| `crates/ark-cli/src/main.rs` (l. 469-501) | `resolve_and_persist_identity`: developer-name prompt with five-level precedence (`--developer`, `--no-developer`, existing `.ark/.developer`, TTY prompt, non-TTY skip-silent). |
| `crates/ark-core/src/commands/init.rs` | The scaffold itself. `init()` walks `ARK_TEMPLATES` + every selected platform's tree, writes via `write_file`, records into `.ark/.installed.json`, applies managed blocks, installs the SessionStart hook. |
| `crates/ark-core/src/commands/init.rs` (l. 95-115) | `InitSummary::fmt` — first user-visible output after init: `N file(s): X created · Y unchanged · Z skipped · W overwritten`. |
| `templates/ark/workflow.md` | The doc the user is implicitly told to read after init (`CLAUDE.md` managed block points there). 394 lines. |
| `docs/book/src/getting-started/quick-start.md` | Hosted onboarding narrative. Lists what gets scaffolded; identifies user-owned vs Ark-owned paths. |
| `README.md` (l. 43-77) | Three-line Quick Start: `ark init`, then `/ark:quick` / `/ark:design`. |

### Code patterns

**The platform prompt loop in `crates/ark-cli/src/main.rs:273-285`:**

```rust
fn interactive_select_platforms() -> anyhow::Result<Vec<&'static Platform>> {
    eprintln!("Select integrations to install:");
    let mut chosen = Vec::with_capacity(PLATFORMS.len());
    for platform in PLATFORMS {
        eprint!("  install {} integration? [Y/n] ", platform.id);
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).ok();
        if !matches!(line.trim().to_ascii_lowercase().as_str(), "n" | "no") {
            chosen.push(*platform);
        }
    }
    Ok(chosen)
}
```

Three prompts. Default-yes (the bare-Enter case falls to the `else` branch). No `[?]` help affordance. No "you can change this later" hint. Output goes to stderr (good for redirection).

**The non-TTY refusal in `crates/ark-cli/src/main.rs:262-266`:**

```rust
anyhow::bail!(
    "init requires at least one of --claude, --codex, or --opencode when stdin is not a TTY \
     (use --no-claude / --no-codex / --no-opencode to opt out per platform)"
);
```

Explicit non-default. Bails rather than picking "all platforms" silently. This is the right call for CI / containerized installs — the user gets a discoverable error naming every flag they can pass.

**The pre-existing-manifest defaulting branch (l. 255-258):**

```rust
if let Some(set) = installed
    && !set.is_empty()
{
    return Ok(set.to_vec());
}
```

Re-running `ark init` with no flags keeps the same platform selection. Quiet idempotence — a common UX win that most installers get wrong by re-prompting.

**The post-init summary line (`init.rs:95-114`):**

```rust
write!(
    f,
    "{} file(s): {} created · {} unchanged · {} skipped · {} overwritten",
    ...
)?;
if self.skipped > 0 {
    write!(
        f,
        "\nnote: {} existing file(s) preserved; pass --force to overwrite",
        self.skipped
    )?;
}
```

Counters + conditional recovery hint. The recovery hint is *only* emitted when relevant — exactly the right discipline. No emoji, no decorative noise.

**The developer identity precedence (`main.rs:469-498`)** is the most opinionated piece of first-run UX:

```rust
fn resolve_and_persist_identity(
    project_root: &Path,
    explicit: Option<&str>,
    no_developer: bool,
) -> anyhow::Result<()> {
    if no_developer {
        return Ok(());
    }
    let identity = if let Some(name) = explicit { ... }
    else if Layout::new(project_root).developer_file().exists() { ... }
    else if std::io::stdin().is_terminal() {
        // prompt up to 3 times, write `.ark/.developer`
    } else {
        return Ok(()); // non-TTY: skip silently
    };
    bootstrap_workspace(project_root, &identity)
}
```

Five branches, each with a clear precedence. The aborted-prompt case (`Err(_) => return Ok(())`) refuses to fail init — also a careful choice; the user can re-run later.

### External references

#### git: minimal, fast, opinionated about what NOT to do

`git init` creates a `.git/` directory with subdirectories for objects, refs/heads, refs/tags, and template files. That's the entire scaffold. It does not create source files, does not author a `.gitignore`, does not commit anything. The aside-from-`.git` design — "an existing project remains unaltered (unlike SVN)" — is the canonical example of unobtrusive scaffolding in CLI history ([Atlassian, *Git init*](https://www.atlassian.com/git/tutorials/setting-up-a-repository/git-init)).

The user-facing surface after `git init` is one line: `Initialized empty Git repository in /path/.git/`. No "now run X" follow-up. Discoverability happens through `git help`, not through the init output.

#### `npm init`: many questions, defaults available

`npm init` asks a series of questions to create `package.json`: name, license, scripts, description, author, keywords, version, main file. You can either answer or press Enter for the default ([npm docs](https://docs.npmjs.com/cli/v11/commands/npm-init/)). `npm init -y` skips all questions ([GeeksforGeeks](https://www.geeksforgeeks.org/node-js/npm-init/)). For repeat users, `~/.npm-init.js` lets you customize the questions asked and fields created — explicit power-user accommodation.

The eight-question model has been criticized for friction; the `-y` flag was added precisely because the interactive flow was getting in the way. Ark's three-platform prompt is well below the npm-init friction floor and stays interactive only on a TTY without explicit flags.

#### `cargo new`: zero questions, three files

`cargo new hello_rust` creates `.git`, `.gitignore`, `Cargo.toml`, `src/main.rs`. Four artifacts, no prompts. The Rust core team "ships you `cargo new` and trusts the community to build framework-shaped scaffolders on top" ([Andrew Nesbitt, *Package Manager Design Tradeoffs*](https://nesbitt.io/2025/12/05/package-manager-tradeoffs.html)). The deliberate non-extension is the design choice — no `cargo init --with-axum-and-tokio`.

#### `next create` / `create-react-app`: kitchen sink, many questions

`npx create-next-app` asks ~8 questions (TypeScript? ESLint? Tailwind? `src/` directory? App router? Import alias?). The CRA equivalent dumps a fully working multi-file scaffold with build scripts, test scripts, a default index page, and `node_modules/`. CRA was officially deprecated in February 2024, with the React team recommending Vite or Next.js for new projects ([Build5Nines, *Create React App Is Now Deprecated*](https://build5nines.com/create-react-app-is-now-deprecated-time-to-migrate-to-vite-or-next-js/)). The reasons span beyond UX, but the deprecation does mark the kitchen-sink model as having lost its century.

#### Claude Code `/init`: AI-generated onboarding doc

Claude Code's `/init` slash command is conceptually adjacent to but mechanically different from `ark init`. It "scans your project and generates a CLAUDE.md file — a persistent briefing that tells Claude your conventions, stack, and rules" ([Kaushik Gopal, *Build your own /init command like Claude Code*](https://kau.sh/blog/build-ai-init-command/)). The output is a single Markdown file derived from reading the codebase; no platform integration is installed because Claude Code is itself the platform.

The user-visible mechanic: "Every time you open the project again, Claude reads this file first so it does not start from zero" ([How Do I Use AI, *What Does /init Do in Claude Code?*](https://www.howdoiuseai.com/blog/2026-04-16-what-does-init-do-in-claude-code-claudemd-setup)). Refresh via `/init --refresh`. Commit to source control so the team gets the same Claude experience.

Note the asymmetry: Claude Code's `/init` is *generative* (writes content derived from your code); Ark's `ark init` is *extractive* (writes content from embedded templates). Both end up authoring a CLAUDE.md, but for different reasons.

#### Aider: zero scaffolding, runtime configuration

Aider's first-run path is the opposite extreme: no scaffold, no `.aider/` directory, configuration through environment variables and CLI flags. Running `aider factorial.py` "displays version information, available models, and git repo status, along with a prompt telling you to use `/help` to see in-chat commands" ([apidog, *What is Aider AI*](https://apidog.com/blog/aider-ai/)).

The first five minutes look like: install via pip, set `OPENAI_API_KEY`/`ANTHROPIC_API_KEY`, run `aider <file>`, type a request. No persistent on-disk footprint beyond git commits (aider auto-commits its changes). Discovery happens through `/help` and `/help <question>` in-chat.

This is `cargo new`-class minimalism applied to an agent harness. The cost: zero project-level customization without environment variables; convention discoverability is conversation-level only.

#### Cline: GUI-driven, API-key gate

Cline (VS Code extension) "First launch prompts you to configure a provider. To use Cline effectively, you will need an API key from a supported provider" ([DeployHQ, *Cline for VS Code*](https://www.deployhq.com/guides/cline)). The on-disk footprint is the VS Code extension manifest plus per-workspace settings, not a top-level `.cline/` directory.

The five-minute path: install extension, click icon, paste API key, describe a task. No scaffolding, no project conventions captured outside what you type into the prompt box.

#### Continue.dev: generates `config.yaml` on first run

"The first time you use Continue, it generates a `config.yaml` with sensible defaults, from which you can customize everything from model selection to context providers, slash commands, and more" ([Continue Docs, *Configuration*](https://docs.continue.dev/customize/deep-dives/configuration)). Local user-level configuration lives in `~/.continue/config.yaml`. Editing the YAML triggers automatic reload — no restart.

This is `npm init`-shaped — one file, opinionated defaults, customize after the fact. Less aggressive than Ark's full directory scaffold; more substantial than Aider's environment-variable approach.

#### OpenHands: no install on the agent side, repo-side microagents

OpenHands runs as a Docker-hosted service; the brownfield-adoption story is "drop `.openhands/microagents/repo.md` into your repo and the agent picks it up" ([OpenHands repo microagent README](https://github.com/OpenHands/OpenHands/blob/main/AGENTS.md)). There is no `openhands init` to run inside the project itself. The harness side is server-side; the per-project side is a single optional Markdown file.

#### Spec-Kit and OpenSpec: spec-first as scaffolding

Spec Kit is "an open-sourced toolkit for spec-driven development that provides a structured process for coding agent workflows with tools including GitHub Copilot, Claude Code, and Gemini CLI" ([GitHub Blog, *Spec-driven development with AI*](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)). Its scaffold authors `/spec`, `/plan`, `/tasks` slash commands and templates.

OpenSpec's three-phase state machine (proposal/apply/archive) is enforced by structure, not prose — the scaffold writes the directories and the agent's role doc tells it which directory to land in. The user's first-run pattern: install OpenSpec CLI, run init in a repo, read an AGENTS.md "README for Robots" that explains where new specs go.

Ark's tier-based workflow is structurally similar (quick/standard/deep, the `task new`-`task commit` lifecycle), and Ark inherits the AGENTS.md pattern via the `<!-- ARK -->` managed block.

### The "first five minutes" archetype

Across the surveyed tools, three distinct shapes emerge:

1. **Zero-friction runtime tools** (Aider, Cline, basic Claude Code session) — install once, run a command, talk to the model. No project artifacts. Configurable via env vars / GUI settings. Discoverability is conversational (`/help`).

2. **Single-artifact tools** (Claude Code `/init`, OpenHands microagents, Continue.dev's `config.yaml`) — generate or scaffold one file as the contract between repo and agent. Easy to delete, easy to commit, easy to share.

3. **Structural-scaffold tools** (Ark, Spec-Kit, OpenSpec) — install a directory tree, a workflow doc, slash-command templates, and a managed-block tap into the user's agent-facing config (`CLAUDE.md` / `AGENTS.md`). Heavier first-run footprint, but the workflow itself is the product.

Ark sits clearly in (3) and is reasonably efficient about it: ~30 files total, three interactive prompts on a TTY, two top-level managed blocks (`CLAUDE.md`, `AGENTS.md`), one `.ark/.installed.json` manifest tracking what was written.

### What Ark's first-run does well

- **TTY-aware prompting.** `resolve_platforms_pure` distinguishes TTY from non-TTY and refuses silent defaults in CI (`crates/ark-cli/src/main.rs:262-266`). Most installers either prompt-everywhere or silent-default-everywhere; the TTY-gated split is the considered choice.
- **Idempotent re-init.** `second_init_is_idempotent` test (l. 449-456 of `init.rs`) asserts `created == 0 && overwritten == 0 && unchanged > 0` on the second call. Rerunning `ark init` to add a new platform or refresh templates is safe.
- **Manifest-driven cleanup.** Every file written goes into `.ark/.installed.json` so `ark remove` knows exactly what to delete and leaves user files alone (`init.rs:243-247`).
- **Managed blocks merged on write.** `merge_managed_blocks` is called inside `extract` (`init.rs:235`) so re-running init never clobbers `spec register`-written rows.
- **Conditional recovery hint.** The "pass --force to overwrite" hint only appears when skipped > 0 (`init.rs:106-112`). No noise when not needed.
- **Aborted identity prompt is non-fatal** (`main.rs:494-495`). The user can ctrl-C the identity prompt and init still completes — they can set the developer name later.

### What Ark's first-run gets wrong (or under-delivers)

- **No "what's next" sentence after init.** Compare `git init`'s `Initialized empty Git repository in /path/.git/` — at least telling the user where the work landed. Ark prints `N file(s): X created · Y unchanged…` and then exits. No "now open Claude Code and run `/ark:quick <title>`". The README has it; the CLI does not.
- **Platform prompt's `[Y/n]` doesn't explain stakes.** A new user has no way to know that `n` to Codex means they're locking themselves out of `.codex/` integration. There's no `[?]` to expand info. The single-line prompt format is space-efficient but information-poor.
- **No "we detected an existing platform integration" branch.** If a project already has `.claude/commands/` from somewhere else, `ark init` doesn't warn before overlaying — it just merges managed blocks and trusts the manifest. A first-run user has no signal about what's being merged.
- **Developer identity prompt is undocumented in the README.** The README's Quick Start (`README.md:43-77`) lists `ark init` and goes straight to `/ark:quick`. The developer-name dialog is described nowhere in user-facing docs except mdBook's deeper pages.
- **Non-TTY without platform flags errors without naming the docs link.** `init requires at least one of --claude, --codex, or --opencode` lists the flags but not where to read more. Compare `cargo new`'s error messages which usually link to a docs URL.
- **`InitSummary::fmt` is one terse line.** "5 file(s): 5 created · 0 unchanged · 0 skipped · 0 overwritten" tells a first-time user nothing about what those files do. After `cargo new`, you at least know `src/main.rs` is where to start; after `ark init`, you have to read mdBook to find out where to look.
- **No `--dry-run` for init.** `cargo new --dry-run` doesn't exist either, but `terraform plan` does. For an installer that touches user-owned files (`CLAUDE.md`, `AGENTS.md`, `.claude/settings.json`), a preview mode would help cautious adopters.

### Five-minute experience timeline (best case)

```
0:00  user reads README, decides to try ark
0:30  npm install -g @anekoique/ark   # or cargo install --git ...
1:00  ark --version
1:10  cd <project> && ark init
1:15  TTY prompt: install claude-code integration? [Y/n] _
1:30  TTY prompt: install codex integration? [Y/n] _
1:40  TTY prompt: install opencode integration? [Y/n] _
1:50  TTY prompt: developer name? _
2:10  "30 file(s): 30 created · 0 unchanged · 0 skipped · 0 overwritten"
2:30  user opens Claude Code in the project
2:40  user reads CLAUDE.md, sees <!-- ARK --> block pointing at .ark/workflow.md
3:30  user opens .ark/workflow.md, reads "Quick Start" section
4:30  user runs /ark:quick "fix typo"
5:00  the agent reads ark context, the workflow doc, prompts for the PRD
```

The five minutes spend most of their budget on (a) reading workflow.md, (b) understanding what just got written, (c) deciding what the first task is. Ark's interactive prompts cost <30 seconds total — these are not the bottleneck.

### Five-minute experience timeline (worst case)

```
0:00  user runs `ark init` from CI shell, gets the non-TTY error
0:10  user reads error, sees three flags, picks one
0:30  user runs `ark init --claude`, gets a 30-file scaffold
0:40  user has no idea what changed, opens .ark/
1:00  user finds workflow.md, doesn't recognize the tier vocabulary
2:00  user reads project SPEC INDEX, finds it empty
3:00  user gives up and reads docs/book/
5:00  user finally runs /ark:quick
```

The worst case is dominated by missing inline guidance: the user has to discover `workflow.md`, then discover `tiers`, then discover the slash command list. None of this is hidden, but none of it is signposted from the `ark init` output line.

### Comparison table — the "first five minutes" cost across tools

| Tool | Install | First-run prompts | Persistent footprint | "Next command" signposted? |
| --- | --- | --- | --- | --- |
| `git init` | system pkg | 0 | `.git/` | no |
| `npm init -y` | nodejs | 0 | `package.json` | no |
| `cargo new` | rustup | 0 | `src/`, `Cargo.toml`, `.git/`, `.gitignore` | no |
| `npx create-next-app` | nodejs | ~8 | full app | yes (final output) |
| `aider <file>` | pip | 0 | none | yes (`/help`) |
| Cline first launch | VS Code Marketplace | 1 (API key) | extension only | yes (chat panel) |
| Continue.dev first launch | VS Code Marketplace | 0 | `~/.continue/config.yaml` | yes |
| Claude Code `/init` | binary | 0 | `CLAUDE.md` | implicit |
| OpenSpec init | npm/cargo | 0 | spec scaffold | yes |
| `ark init` | npm/cargo | 1-4 (TTY) | `.ark/` + `.claude/` + `.codex/` + `.opencode/` + managed blocks | no |

Ark's footprint is the heaviest in the survey because it owns three platform integration directories at once. That's a structural decision tied to Ark's identity as a cross-platform harness; the trade-off is more files written than any peer but a single workflow that works across Claude Code, Codex, and OpenCode.

### Caveats / Not found

- No first-run telemetry exists for Ark — the "first five minutes" analysis is reasoning from source + docs, not from user studies.
- The exact wording of competitor install summaries (Aider, Cline) was not captured in primary sources; the comparison rests on documentation rather than running each tool.
- No data on `ark init --force` vs interactive selection ratios in the wild.
- The "discoverability gap between init output and workflow.md" is observed by code inspection, not verified with new-user testing.

## Directions for Ark

1. **Add a one-line "next" footer to `InitSummary::fmt`.** Pattern:
   ```
   30 file(s): 30 created · 0 unchanged · …
   Ark scaffolded into <root>. Open your agent (Claude Code / Codex / OpenCode) and run `/ark:quick "<title>"` to start your first task.
   ```
   This costs ~80 bytes of output and closes the discoverability gap most aggressively. Single biggest five-minute win identified.

2. **Make the platform prompt explain stakes.** Replace `install <id> integration? [Y/n]` with two-line form:
   ```
   install claude-code integration? installs .claude/commands/ark/ and a SessionStart hook [Y/n] _
   ```
   Or add a `?` answer that prints a one-paragraph blurb. The information-poor `[Y/n]` is the only first-run prompt that doesn't give the user the context to answer.

3. **Surface platform integration overlay before scaffolding.** Before writing `.claude/commands/ark/`, detect existing `.claude/commands/` directories (any commands directory, not just the manifest) and print:
   ```
   note: .claude/commands/ already exists with 4 user file(s); Ark will add .claude/commands/ark/ alongside (no overlap).
   ```
   First-run adopters in brownfield repos most fear silent overwrites; surfacing the pre-existing state defuses this.

4. **Document the developer-identity dialog in the README Quick Start.** Two-sentence addition. Today the README treats `ark init` as a single line; users hit the identity prompt and have no doc to consult about what happens if they ctrl-C.

5. **Add `ark init --print-tree` (or a dry-run mode).** Print the list of files that would be written, the managed blocks that would be inserted, the hook entry that would be added — without writing anything. Pairs naturally with the existing `--force` / `--no-<platform>` flags for cautious users in shared repos. The implementation is cheap: `init.rs::extract` is already walking the template trees; gate `write_file` behind a `WriteMode::DryRun`.

## Caveats / Not found

- The Ark CLI does not currently emit any tutorial-style next-step output; this section assumes adding one is desirable. Could conflict with the "boring CLI adapter" principle of `ark-cli` in `AGENTS.md` (l. 7) — placement might belong in `InitSummary::fmt` (already in `ark-core`).
- The README Quick Start has been deliberately terse (37 lines for both Quick Start and Lifecycle). Adding identity-dialog docs there might violate that voice; the alternative is the mdBook installation page.
- Spec-Kit and OpenSpec first-run UX details (exact prompts, summary lines) were not verified against running installs — research relied on documentation rather than empirical observation.
