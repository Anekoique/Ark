# Research: Scaffolding Philosophy

- Query: Where Ark sits on the minimal-vs-opinionated scaffolding axis; the **managed-block** pattern as a hybrid; conflict resolution during upgrade; brownfield vs greenfield bias.
- Scope: mixed
- Date: 2026-05-20

## Findings

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `crates/ark-core/src/templates.rs` | Embeds template trees via `include_dir!`; supplies the walker used by `init` and `upgrade`. |
| `crates/ark-core/src/io/fs.rs` | Home of `merge_managed_blocks`, `read_managed_block`, `update_managed_block`, `remove_managed_block`. The managed-block primitives. |
| `crates/ark-core/src/commands/init.rs` (l. 200-251) | `extract_filtered` — the scaffolding loop. Every template file goes through `merge_managed_blocks` before `write_file`. |
| `crates/ark-core/src/state/manifest.rs` | `.ark/.installed.json` — records every scaffolded path + sha256 hash + every managed block (`file`, `marker`). The provenance database. |
| `templates/ark/` | The Ark-owned tree (workflow.md, templates/, specs/, config.toml). |
| `templates/claude/`, `templates/codex/`, `templates/opencode/` | Per-platform trees (commands/, skills/, agents/, plugins/). |
| `AGENTS.md` (l. 143-149) | The managed block itself, with `<!-- ARK:START -->` / `<!-- ARK:END -->` delimiters around three lines pointing at `.ark/workflow.md` and `.ark/specs/INDEX.md`. |

### Code patterns

**Managed block opening in shipped templates.** Every template that participates in the managed-block flow ships with the delimiters already in place. From a hypothetical `templates/claude/CLAUDE.md`:

```markdown
<!-- ARK:START -->
Ark is installed in this project. Use `/ark:quick` or `/ark:design` to start tasks.

See `.ark/workflow.md` for the full workflow.

@.ark/specs/INDEX.md
<!-- ARK:END -->
```

**The merge step in `init.rs:233-236`:**

```rust
let contents = merge_managed_blocks(&dest, entry.contents)?;
let outcome = write_file(&dest, &contents, mode)?;
```

The on-disk block body is spliced *into* the template before writing. This means a user who edits within the block, then re-runs `ark init`, keeps their edits — only the surrounding text refreshes. The single most consequential design choice in Ark's scaffolding model.

**Hash tracking in `init.rs:245-247`:**

```rust
if outcome != WriteOutcome::Skipped {
    manifest.record_file_with_hash(relative, &contents);
}
```

A file Ark writes gets its hash recorded. A file Ark *would* have written but didn't (because the user already has different content and we're in `Skip` mode) does NOT get its hash recorded — recording would falsely claim ownership of user content. The comment at l. 243-245 calls this out explicitly.

**`init`'s manifest is additive across platforms** (`init.rs:144-161`):

```rust
for platform in &opts.platforms {
    drop_manifest_entries_under(&mut manifest, platform.dest_dir);
    ...
    extract_filtered(platform.templates, ...);
    platform.apply_managed_state(&layout, &mut manifest)?;
}
```

The `drop_manifest_entries_under` call wipes stale entries for the targeted platform only. Running `ark init --codex` on a Claude-installed project adds Codex without forgetting Claude — tested in `second_init_with_subset_keeps_other_platform_in_manifest` (l. 362-398).

### External references

#### `cargo new` — the minimal-scaffold archetype

`cargo new hello_rust` produces four files: `.git/`, `.gitignore`, `Cargo.toml`, `src/main.rs`. No additional dependencies, no build configuration beyond a `[package]` section, no test scaffold, no CI templates. "Rust hands you a Cargo.toml and wishes you well" ([Andrew Nesbitt, *Package Manager Design Tradeoffs*](https://nesbitt.io/2025/12/05/package-manager-tradeoffs.html)).

The implicit philosophy: every line in the scaffold is something the user will read on day one. Anything beyond that creates friction without value. Framework-shaped scaffolders (Loco, go-blueprint) opt-in *on top of* the minimal kit, never below.

This is the right model when the tool has nothing to assert about workflow — `cargo` knows nothing about how you'll structure your project, so it scaffolds nothing structural.

#### `create-react-app` (deprecated 2024) — the kitchen-sink archetype

CRA scaffolded ~20 files (depending on version), pre-installed React, react-dom, react-scripts, and the entire Webpack build pipeline as transitive deps; created build / test / start scripts; authored a default index page; populated `node_modules/`. Designed to give a first-time React user a running app within seconds of `create-react-app my-app && cd my-app && npm start`.

The React team formally deprecated CRA for new projects on February 14, 2024, advising users to migrate to Vite or Next.js ([Build5Nines, *Create React App Is Now Deprecated*](https://build5nines.com/create-react-app-is-now-deprecated-time-to-migrate-to-vite-or-next-js/)). The cited reasons are technical (Webpack vs esbuild speed), but the broader pattern is that kitchen-sink scaffolds calcify around their original choices and can't migrate users to better defaults later.

Lesson: heavy scaffolds work when the templates are evergreen. They become tech debt when the underlying ecosystem shifts faster than the scaffold can keep up.

#### `create-next-app` — middleweight, configurable

`npx create-next-app` asks ~8 questions (TypeScript? Tailwind? App router?) and scaffolds accordingly. Lighter than CRA in absolute file count, but still authors a working app. The question-driven model lets the scaffold ship variants without committing to one default.

The cost of the variant approach is decision fatigue at install time and a much larger test matrix for the maintainer.

#### Rails generators — generators-everywhere

Rails ships generators for every common noun: `rails generate model`, `rails generate controller`, `rails generate migration`, `rails generate scaffold`. The CLI is itself a scaffolding sub-language. `rails new <app>` is the initial kit; the generators continue scaffolding throughout the project's life.

The cost: locked-in framework conventions. Strong opinions about file naming, directory layout, generated test files. Users who don't want Rails conventions abandon Rails entirely (Sinatra exists; Roda exists).

#### Aider / Cline / OpenHands — scaffold-free agent harnesses

Aider writes nothing to the project on first run beyond what the agent commits via git. Cline lives entirely in the VS Code extension layer. OpenHands runs server-side; the project-side artifact is one optional `.openhands/microagents/repo.md` file ([OpenHands AGENTS.md docs](https://github.com/OpenHands/OpenHands/blob/main/AGENTS.md)).

The "agent harness with no on-disk scaffold" model leans entirely on conversational context — the agent reads your code, infers conventions, asks if it gets confused. Ark explicitly rejects this approach: the workflow IS the scaffold.

#### Spec-Kit — workflow-shaped scaffolding

Spec Kit's `init` writes slash commands (`/spec`, `/plan`, `/tasks`), template files for spec/plan/task docs, and an AGENTS.md-style "README for Robots" ([GitHub Blog, *Spec-driven development with AI*](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)). Conceptually identical to Ark's approach — install slash commands + workflow doc + templates — but doesn't use a managed-block pattern (it owns its own files outright).

#### OpenSpec — three-phase state machine on disk

OpenSpec writes a `specs/` tree with a three-phase state machine (proposal → apply → archive) encoded as directory structure. The AGENTS.md tells the agent which directory to land in for each phase ([Avasdream, *OpenSpec vs Spec Kit*](https://avasdream.com/blog/openspec-vs-spec-kit-ai-development)). Heavier scaffold than Ark per-task, lighter on the platform-integration side (one AGENTS.md vs three CLAUDE.md/AGENTS.md targets).

The brownfield-first design ("delta markers ADDED/MODIFIED/REMOVED that track what changes relative to existing functionality") is a direct contrast with Ark: OpenSpec teaches the agent to express change *as a diff*; Ark teaches it to express change *as a tier-graded task*.

#### Chezmoi — the dotfile-manager precedent

Chezmoi manages dotfiles by maintaining a source tree (`~/.local/share/chezmoi/`) and applying it to the home directory via `chezmoi apply`. The source tree contains templates, secrets references, scripts. The destination is the user's actual `~/.zshrc`, `~/.config/nvim/`, etc.

Conflict resolution: "Run `chezmoi diff` to see what changes would be made, and `chezmoi apply` to make the changes" ([chezmoi *Daily operations*](https://www.chezmoi.io/user-guide/daily-operations/)). On a detected conflict, "chezmoi will detect that ~/.zshrc has changed since chezmoi last wrote it and prompt you what to do. You can resolve differences with a merge tool by running `chezmoi merge ~/.zshrc`" ([chezmoi *Merge* docs](https://www.chezmoi.io/user-guide/tools/merge/)).

The relevant pattern: a *source state* (templates), a *target state* (computed from source + user data), and a *destination state* (what's on disk). Conflicts emerge when destination state diverges from the previously-applied target state. Ark uses the same trichotomy — embedded templates (source), `merge_managed_blocks`-transformed bytes (target), on-disk files + manifest hashes (destination state record).

Chezmoi also supports `modify_` scripts that can read the current file on stdin and write modified contents on stdout — a more programmable analog of Ark's managed blocks.

#### GNU Stow — symlink-based, fully reversible

Stow takes the opposite approach: instead of writing files into the destination, it creates symlinks from `~/.zshrc` to `~/dotfiles/zsh/.zshrc`. The "files" never actually leave the source tree.

"To uninstall, you simply need to remove the symbolic links that Stow created. The original files remain untouched in their dedicated Stow directory, ready to be re-linked if needed, or safely deleted later" ([Linux Vault, *How to Use GNU Stow*](https://www.thelinuxvault.net/blog/how-to-use-gnu-stow-to-manage-programs-installed-from-source-and-dotfiles/)). `stow -D` removes all symlinks cleanly.

Symlinks are not viable for Ark — agent platforms (Claude Code, Codex, OpenCode) read literal files at known paths; Ark needs the templates to *be* at those paths, not point to them. But the philosophical model — reversibility as a primary UX property — is identical to Ark's `unload` / `remove` design (covered in `install-upgrade-uninstall-lifecycle.md`).

### The opinionated–minimal axis, mapped

```
minimal ←─────────────────────────────────────────────→ opinionated
git init   cargo new   npm init -y   continue.dev   ark    spec-kit   create-next-app   rails new
                                                                                     create-react-app
```

Ark ends up to the right of midpoint — heavier than `npm init`, lighter than `create-next-app`. The total scaffold count (~30 files) is substantial, but unlike CRA the files are all human-readable doc / template content, not generated code. The closest precedent is spec-kit and openspec — workflow-shaped scaffolds for AI-coding tools.

### Why Ark scaffolds heavy

Three forces push Ark away from a minimal scaffold:

1. **Multi-platform integration.** A single `ark init` writes into `.claude/`, `.codex/`, and `.opencode/` simultaneously. The cross-platform consistency story requires *Ark* to author the per-platform files because no individual platform tool would.

2. **Workflow is the product.** Ark's tier model (`quick`/`standard`/`deep`/`research`), the phase machine, the PRD/PLAN/REVIEW/VERIFY/SPEC structure — these are the deliverables. Aider could ship as a binary because the workflow is conversational. Ark ships as a binary *and* a directory tree because the directory IS the workflow.

3. **Promotable specifications.** `specs/project/` is user-authored; `specs/features/` gets populated by `task commit` on deep-tier tasks. The mechanism requires the directory tree to exist with INDEX.md files in place. Spec promotion (`crates/ark-core/src/commands/agent/spec/`) wouldn't work against a not-yet-scaffolded tree.

### The managed-block pattern — Ark's hybrid

The managed block delimiter pair (`<!-- ARK:START -->` / `<!-- ARK:END -->`) is the heart of Ark's scaffolding philosophy. It allows Ark to be opinionated *inside* user-owned files. Three properties make it work:

1. **Idempotent.** `merge_managed_blocks` is called on every `init` and `upgrade`. The block body refreshes; surrounding text is preserved.
2. **Recoverable.** `read_managed_block` and `remove_managed_block` are public surface; `ark unload` and `ark remove` can extract the block back into the snapshot or delete it cleanly.
3. **Hash-distinct.** The block body is tracked separately from the rest of the file. Ark never claims ownership of `CLAUDE.md` as a file, only of the block within it.

The closest comparable patterns in other tools:

- **Chezmoi `modify_` scripts** — programmable; the user writes a shell script that takes stdin and produces stdout. More powerful than fixed-delimiter blocks; far more error-prone to author.
- **`.gitignore` entries from `npm`/`yarn`** — when you `npm init`, npm appends standard ignore entries to `.gitignore`. No marker pair; updates require manual editing.
- **`# BEGIN ANSIBLE MANAGED BLOCK`** — Ansible's `blockinfile` module uses textual markers identical in spirit to Ark's `<!-- ARK -->`. Same idempotency, same "owned region in user-owned file" guarantee. The Ansible model is the closest precedent for the Ark approach.
- **Editor-modeline blocks** (`/* vim: set ts=2: */`) — fixed, user-authored, no tool refreshes them.

Ark's managed block carries less content than chezmoi's modify_ scripts (it's static text, not programmable) but more content than git's hooks (which are entire executable files). The sweet spot lands at "three lines of pointer text" — enough to direct the agent into the workflow, not so much that the user feels their file is being colonized.

### Brownfield vs greenfield bias

Ark's design biases toward **greenfield** in two ways:

1. **`ark init` is the entry point.** No `ark adopt` or `ark sync` for picking up an existing project's conventions. The PRD-and-tier workflow assumes you're starting from scratch on the task level.
2. **Specifications are tier-promoted from new tasks.** `specs/features/` is populated by `task commit` on deep-tier tasks — meaning you have to first write a deep-tier task to get a feature SPEC. There's no `ark agent spec extract` for back-filling specs from existing code.

Ark biases toward **brownfield** in three ways, mostly via the managed-block pattern:

1. **Insertion into existing CLAUDE.md / AGENTS.md.** The user can have a 200-line CLAUDE.md and Ark slots in 5 lines via the managed block. No conflict, no overwrite.
2. **`.ark/specs/project/` is user-authored, never extracted.** A brownfield adopter can write project SPECs that document existing conventions; Ark just reads them.
3. **`ark init` is safe on existing repos.** The `Skip` write mode (default for `init` per `crates/ark-cli/src/main.rs:565-569`: `if a.force { WriteMode::Force } else { WriteMode::Skip }`) means existing files survive; only fresh files get written. The `init_force_preserves_existing_managed_block_rows` test (l. 404-428) covers the `--force` edge.

The mix is reasonable for a tool whose product is "a workflow you adopt." The remaining gap is the lack of an `ark agent spec import` for extracting feature specs from existing prose — covered in detail in `brownfield-adoption.md`.

### Conflict resolution during scaffolding

`ark init` has two conflict modes:

| Mode | When | Behavior |
| --- | --- | --- |
| `WriteMode::Skip` (default) | First `init` | Existing on-disk content survives; managed blocks merge in. `summary.skipped += 1`. |
| `WriteMode::Force` | `--force` | Existing content is overwritten unconditionally. Managed-block bodies still merge (so the user's `spec register`-written rows survive). |

The default-Skip behavior is the right call: a first-time user running `ark init` in a directory that happens to contain `.claude/commands/ark/quick.md` from somewhere else gets their file preserved. The `--force` flag is an opt-in escape hatch.

What's missing: an `--interactive` mode that prompts per-file. `cargo install --force` is unconditional; `chezmoi apply` is interactive with merge tool integration. Ark's `init` is unconditional in both directions (always skip or always force). This is defensible — `init` is meant to be the simple path — but more granular control would help adopters in repos with many partially-overlapping files. (See `install-upgrade-uninstall-lifecycle.md` for the more sophisticated conflict resolution available in `ark upgrade`.)

### Conflict resolution during upgrade

`ark upgrade` is where Ark's most thoughtful conflict design lives, with four conflict policies tied to four CLI flags:

| Flag | `ConflictPolicy` | Effect on user-modified file |
| --- | --- | --- |
| (none, TTY) | `Interactive` | Prompts per-file: `[o]verwrite / [s]kip / [c]reate .new?` |
| `--force` | `Force` | Always overwrite |
| `--skip-modified` | `Skip` | Always preserve |
| `--create-new` | `CreateNew` | Write `<path>.new` next to the user's file |

From `crates/ark-cli/src/main.rs:317-330`:

```rust
fn policy(&self) -> ConflictPolicy {
    if self.force {
        ConflictPolicy::Force
    } else if self.skip_modified {
        ConflictPolicy::Skip
    } else if self.create_new {
        ConflictPolicy::CreateNew
    } else {
        ConflictPolicy::Interactive
    }
}
```

The `.new` sidecar mode (`CreateNew`) is the standout: instead of forcing a binary choice between "lose the user's edit" and "lose the new template," Ark writes the new template to a sibling file with `.new` appended. The user can `diff workflow.md workflow.md.new`, manually merge, then delete the sidecar. This is the dotfile-manager pattern (chezmoi's merge tool) reinterpreted for non-interactive contexts.

### Caveats / Not found

- No empirical study of how often users hit conflicts at `init` time vs `upgrade` time was located. The design implies init conflicts are rare (new project) and upgrade conflicts are common (older projects), but field data would help calibrate.
- The exact behavior of OpenSpec's brownfield iteration ("delta markers ADDED/MODIFIED/REMOVED") was sourced from third-party blog summaries rather than the OpenSpec source code itself.
- Whether the Ansible `blockinfile` model was a direct influence on Ark's managed-block design is unknown; the syntactic similarity (`<!-- ARK:START -->` vs `# BEGIN ANSIBLE MANAGED BLOCK`) could be convergent.
- `git init`'s default scaffold contents (`.git/objects`, `.git/refs/heads`, etc.) were not surveyed in detail; the cited Atlassian doc was the source of the "minimal scaffold" framing.

## Directions for Ark

1. **Document the managed-block contract as a stable interface.** Today the markers (`<!-- ARK:START -->` / `<!-- ARK:END -->`) are described in passing in `templates/ark/workflow.md` and the source comments. They deserve a SPEC under `.ark/specs/project/` — third-party tools that want to coexist with Ark (e.g. another workflow generator that also wants to drop into CLAUDE.md) need to know whether they can claim a similar marker pair and what happens when two tools want the same file. Promotable convention; clear semver story.

2. **Add `ark init --dry-run` with a per-file conflict preview.** Walk every template, show what would be created / skipped / merged. Pairs naturally with the existing `WriteMode::Skip` / `Force` flags. Surfaces the platform-overlay risk for brownfield repos without writing anything.

3. **Offer an `--interactive` mode at init time** (mirroring the upgrade conflict pipeline). When Ark's about to overwrite an existing file under `Skip` mode, prompt `[s]kip / [o]verwrite / [c]reate .new?`. Reuses `ConflictChoice` and the `Prompter` trait; one code path, two callers. Brownfield onboarders gain the granular control the upgrade pipeline already has.

4. **Add `ark agent spec import` for back-filling feature specs.** Brownfield adopters with existing convention prose in their `docs/` or `CONTRIBUTING.md` cannot easily migrate it into `.ark/specs/project/` or `.ark/specs/features/`. A read-only command that points at a Markdown file and emits a SPEC-shaped scaffold (with the `<!-- ARK -->` block frame and the right INDEX.md row) would close the convention-import gap. The mechanism already exists half-way: `crates/ark-core/src/commands/agent/spec/` knows how to write a SPEC and update INDEX rows; the missing piece is "extract from arbitrary Markdown" parsing.

5. **Make the platform-integration footprint configurable to a single platform.** Today `ark init` defaults to all platforms on the first TTY run (the interactive prompt offers all three with default-yes). A meaningful number of users will only ever use Claude Code, only ever Codex, etc. A simpler default — say, default to the platform with the highest install count among existing dotfiles, or prompt with `[Y/n]` only for the *detected* platform — would lighten the scaffold for single-tool users while keeping the multi-platform path. The detection lives one fs scan away: if `~/.claude/` exists, default to Claude; if `~/.codex/` exists, default to Codex; otherwise prompt.
