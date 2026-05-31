# Research: Brownfield Adoption

- Query: Adopting an agent harness in an existing repository with existing code, existing conventions, existing CI; the "extract-spec" problem; onboarding into a 100k-line repo; migration between harnesses; the reality that users don't read docs first.
- Scope: mixed
- Date: 2026-05-20

## Findings

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `crates/ark-core/src/commands/init.rs` | Default-Skip write mode (`crates/ark-cli/src/main.rs:565-569`) ensures existing files survive `ark init`. |
| `crates/ark-core/src/io/fs.rs` (managed-block functions) | `merge_managed_blocks`, `read_managed_block`, `update_managed_block`, `remove_managed_block`. The brownfield-safe insertion primitives. |
| `crates/ark-core/src/commands/agent/spec/` | Spec-related agent operations (`spec_register`). Promotes deep-tier PLANs into `specs/features/`. Note: there is no `spec_import` for back-filling specs from existing prose. |
| `.ark/specs/project/INDEX.md` | The user-authored convention index. Brownfield adopters populate this with rules already implicit in their codebase. |
| `templates/ark/specs/project/INDEX.md` | The seeded INDEX shipped by `ark init` — empty managed block users write into. |
| `crates/ark-core/src/commands/agent/spec/register.rs` (referenced in subagent flow) | Inserts rows into `specs/features/INDEX.md` programmatically. |
| `AGENTS.md` (l. 5-10) | Repo-level introduction; brownfield onboarders find this via Claude Code reading it. |
| `templates/ark/specs/project/INDEX.md` (seeded scaffold) | Empty `<!-- ARK:START -->` block ready for user-authored rows. |
| `.ark/workflow.md` (l. 312-322) | The "Specs" section spelling out user-authored (`project/`) vs auto-promoted (`features/`) ownership. |

### Code patterns

**Default-Skip protects brownfield files** (`crates/ark-cli/src/main.rs:565-569`):

```rust
let mode = if a.force {
    WriteMode::Force
} else {
    WriteMode::Skip
};
```

A first-time `ark init` in a repo that already has `CLAUDE.md` will run in Skip mode. The on-disk content survives; only the managed block inside it gets merged. This is the load-bearing brownfield invariant.

**The `merge_managed_blocks` contract** lets an existing CLAUDE.md gain 5 lines of Ark content without disturbing the user's existing prose. From `crates/ark-core/src/commands/init.rs:235`:

```rust
let contents = merge_managed_blocks(&dest, entry.contents)?;
```

If the user has a 200-line CLAUDE.md with no `<!-- ARK -->` block, `merge_managed_blocks` returns the template's bytes (which contain the block). After writing, the user's file is 205 lines, with the block inserted. If the user has a 200-line CLAUDE.md *already containing* an `<!-- ARK -->` block (e.g. from a prior install), the block body refreshes from the template; the rest of the file remains untouched. This is what makes Ark safely re-runnable on a partially-Ark repo.

**The `specs/project/` ownership model** (`.ark/workflow.md:316`):

> **Project specs** — `specs/project/<name>/SPEC.md`. User-authored conventions. Apply to every task. **Read every entry in `specs/project/INDEX.md` before any task.** Agents never edit project SPECs without explicit instruction.

The brownfield migration story for conventions: user reads their CONTRIBUTING.md / STYLE.md / linting rules, distills them into `.ark/specs/project/<name>/SPEC.md`, references each in `INDEX.md`. The agent reads these at the start of every task. Convention adoption is *transcription*, not extraction.

**No automated extraction exists.** Searching `crates/ark-core/src/commands/agent/spec/` reveals `register`, `extract` (deep-tier PLAN to feature SPEC), and related modules — but no command that points at a user's existing `docs/CONTRIBUTING.md` and emits a SPEC. The closest analog is `ark agent spec import` mentioned in the task brief for this research, but the CLI surface does not currently include it. This is a structural brownfield gap.

### External references

#### OpenSpec — brownfield-first by design

"Spec Kit excels at Greenfield (0→1) and acts as a 'Senior Architect in a Box.' Spec Kit's rigid workflow forces the AI to generate database schemas, API contracts, and component hierarchies before writing code. In contrast, OpenSpec's core value proposition lies in its 'brownfield-first' strategy, designed to handle existing codebase evolution (1→n)" ([Avasdream, *OpenSpec vs Spec Kit*](https://avasdream.com/blog/openspec-vs-spec-kit-ai-development)).

OpenSpec uses delta markers (`ADDED`, `MODIFIED`, `REMOVED`) to express change relative to existing functionality. The mechanism: a proposal directory contains *diff statements* against the current spec, not whole-file rewrites. Apply-phase merges the diff into the canonical spec. Archive moves the applied proposal.

This is structurally different from Ark's tier model. Ark expresses change as *task-scoped iteration* (PLAN revisions in deep tier); OpenSpec expresses it as *delta against spec*. Both legitimate. Ark's deep-tier PLAN ⇄ REVIEW loop covers similar ground for new features; the gap is specifically when adopting Ark into a repo with extensive *existing* spec-shaped prose that doesn't yet live in `specs/features/`.

#### Spec-Kit — greenfield-first

Spec Kit "provides a structured process for coding agent workflows" ([GitHub Blog, *Spec-driven development with AI*](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)), with slash commands `/spec`, `/plan`, `/tasks`. The workflow is "start with a spec, then plan, then tasks, then code" — naturally fitting new-feature work better than retrofitting an existing repo.

Spec Kit is one of the harnesses Ark could be migrated *from* or *to*. The structural overlap (slash commands as the user surface, Markdown templates, an AGENTS.md or CLAUDE.md tap-in) is substantial. The migration would mostly be: rewrite `docs/specifications/*.md` from Spec Kit's shape into Ark's `specs/features/` shape.

#### The OpenAI Cookbook brownfield migration pattern

The OpenAI Cookbook hosts a [sandbox-agent recipe for migrating a legacy codebase](https://developers.openai.com/cookbook/examples/agents_sdk/sandboxed-code-migration/sandboxed_code_migration_agent). The pattern: spin up sandbox containers, agent reads existing code, agent proposes changes, agent applies them. Each agent step is contained in a sandbox so failed migrations don't corrupt the repo.

This is *brownfield migration via agent*, not *brownfield adoption of an agent harness*. The two problems share the constraint that you can't assume the repo's existing conventions are documented; both require some form of convention discovery. The Cookbook recipe leans on agent intelligence; Ark leans on user transcription into `specs/project/`.

#### Martin Fowler on harness engineering

"The agent harness acts like a cybernetic governor, combining feed-forward and feedback to regulate the codebase towards its desired state" ([Martin Fowler, *Harness engineering for coding agent users*](https://martinfowler.com/articles/harness-engineering.html)). The framing matters for brownfield: the harness's job is to encode the *desired state* in a form the agent can read. In a greenfield repo, desired state is whatever the team decides. In a brownfield repo, desired state must be inferred from existing code, and that inference is expensive.

The argument: a harness that doesn't surface convention is asking the agent to re-derive it on every task. Adopting Ark in a brownfield repo without first writing project SPECs is doing exactly this.

#### "Most users won't read docs first" — the discoverability axiom

The standard finding in developer-experience research: most users skim the README, ignore the docs site, and learn the tool from running it. This is reflected in CLI design: good `--help` output, error messages that name the recovery action, `did you mean` suggestions.

Ark's tier discoverability ranks moderately:

- `/ark:quick` and `/ark:design` are listed in the README Quick Start.
- `/ark:research` (added recently per the `ark-research` SPEC) is not yet in the README.
- The tier semantics (quick/standard/deep/research) live in `.ark/workflow.md`, which users only discover by reading `CLAUDE.md`'s managed block.

A brownfield adopter who runs `ark init`, opens Claude Code, and sees `<!-- ARK -->` block-text pointing at workflow.md has a ~30-second discovery path. A user who skips that step has effectively zero discovery — they have to type `/ark:` and hope Claude Code autocompletes.

#### The Ona "AI migrations" framing

"Org-wide migrations are the next strategic AI frontier" ([Ona, *Why org-wide migrations are the next strategic AI frontier*](https://ona.com/stories/migrations-are-the-next-ai-frontier)). The thesis: large codebases need automated migration tooling because the cost of human migration scales with codebase size. Adopting an agent harness into a 100k-line repo *is itself* a migration — migrating from "ad hoc Claude Code sessions" to "structured tier workflow."

The implication for Ark: adoption ergonomics on a large existing repo are first-class. A first-time adopter has a 30-page CONTRIBUTING.md, a 5-page STYLE.md, and three dialects of code review. They need either (a) a way to compress this into `specs/project/`, or (b) a way for the agent to read those existing files directly without an Ark spec wrapping.

Today Ark supports (a) by transcription; (b) requires the user to add the file paths to the agent's context somehow (e.g. `@docs/CONTRIBUTING.md` in CLAUDE.md). Neither is automated.

#### The "first principle" of brownfield adoption

From the [General Partnership *Practical Guide to Brownfield AI Development*](https://thegeneralpartnership.substack.com/p/a-practical-guide-to-brownfield-ai): the brownfield problem isn't "make the agent smart enough"; it's "make the existing conventions legible to the agent." Tools that succeed here either (a) generate `AGENTS.md`-style summaries from existing code (Claude Code's `/init` does this), or (b) treat the existing code as authoritative and constrain the agent to small, well-tested changes.

Ark does neither directly. Ark assumes the user is the source of convention truth (project SPECs are user-authored). The agent reads project SPECs at every task start, but no mechanism converts existing docs into SPECs automatically.

### The brownfield-adopter journey

A realistic story for a 100k-line existing repo:

```
day 0    user runs `ark init`, picks Claude Code only
         scaffold lands in 30 files; CLAUDE.md gets a 5-line block

day 0    user opens Claude Code, asks the agent to read CLAUDE.md
         agent follows the link to .ark/workflow.md; reads it

day 0    user tries /ark:quick "fix the typo on line 47"
         agent runs ark context; sees no project specs registered;
         starts the task anyway because PRD is in template

day 1    user notices the agent ignored their team's "no `unwrap()`
         in production code" convention because no SPEC documents it

day 2    user writes .ark/specs/project/error-handling/SPEC.md
         capturing the rule; adds row to INDEX

day 3    user runs /ark:design "add rate limiting middleware"
         agent reads the new SPEC; respects the rule throughout
```

The path works. Day-2 friction is real: the user has to transcribe their team's conventions from CONTRIBUTING.md / STYLE.md / wiki / Slack pins into SPEC files. There's no shortcut.

### Convention extraction — what exists, what doesn't

**Exists:**

- `ark agent spec register` — programmatically inserts rows into `specs/features/INDEX.md`. Used by the deep-tier commit pipeline.
- Deep-tier PLAN extraction (`task commit` reads `## Spec` from the PLAN, writes `specs/features/<path>/SPEC.md`).
- `merge_managed_blocks` — preserves user-authored content alongside Ark-owned content.
- `ark agent task new --slug <s> --title "<t>"` — creates a new task on top of an existing repo without disturbing the rest of the tree.

**Does not exist:**

- `ark agent spec import` — back-fill a SPEC from existing prose (the brief mentions it, but the CLI doesn't ship it).
- `ark agent convention sync` — point at `CONTRIBUTING.md` / `STYLE.md` and emit SPEC scaffolds.
- `ark migrate --from <other-harness>` — convert Spec-Kit / OpenSpec / Aider-style project files into Ark's shape.
- `ark doctor` — scan for adoption gotchas (missing project SPECs, orphan files in `.ark/tasks/`, stale managed blocks).

### Migration between harnesses

A user moving from Aider to Ark: zero artifacts to migrate. Aider writes nothing to the project beyond git commits.

A user moving from Spec-Kit to Ark: rewrite `docs/specifications/` into `specs/features/`. Spec-Kit's slash commands (`/spec`, `/plan`, `/tasks`) overlap meaningfully with Ark's (`/ark:design`, the PLAN phase, etc.); workflow translation is doable manually.

A user moving from OpenSpec to Ark: rewrite OpenSpec's proposal/apply/archive directories into Ark's `tasks/` + `specs/features/` shape. The delta-marker model doesn't translate cleanly — Ark expresses change as task iterations, not deltas.

A user moving from Cline to Ark: drop the VS Code extension's per-workspace settings, run `ark init`, port any project-specific rules from Cline's "instructions" field into `specs/project/`. Most of the value transfer is conceptual (the team's conventions) rather than file-based.

No automated migration command exists for any of these paths. Ark inherits the broader CLI ecosystem's lack of cross-harness migration tools.

### The "100k-line repo" adoption playbook (synthesized)

From the references above, a working playbook would look like:

1. **Read first.** `ark init` writes the scaffold; user reads `.ark/workflow.md` end-to-end (about 15 minutes). This is the single mandatory step that nothing automates.

2. **Inventory existing conventions.** User opens `docs/CONTRIBUTING.md`, `docs/STYLE.md`, `.github/CODEOWNERS`, the team wiki. Lists the rules the team enforces by review.

3. **Transcribe to project SPECs.** One SPEC per coherent rule cluster (error handling, naming, testing). Cite file locations in the existing repo if helpful. Add rows to `.ark/specs/project/INDEX.md`.

4. **Write a placeholder feature SPEC for active major systems.** Ark expects `specs/features/` to be auto-populated, but a brownfield user can pre-populate it by writing manual deep-tier PLANs and committing them (the SPEC promotion machinery handles the extraction). Or hand-write SPEC.md files and register them via the INDEX.md.

5. **Pick a low-stakes first task.** `/ark:quick "fix typo"` to exercise the workflow. Then `/ark:design "add rate limiting"` for a real feature, observing whether the agent respects the freshly-written project SPECs.

6. **Iterate the SPECs.** When the agent does something the team would reject in code review, add a SPEC for that rule. Convention discovery is iterative, not upfront.

The playbook is reasonable but undocumented. A brownfield-adoption page in `docs/book/src/getting-started/` would close the discoverability gap.

### Caveats / Not found

- No measured data on how long brownfield adoption actually takes in practice. The "day 0 / day 1 / day 2" timeline above is speculative based on the workflow shape.
- The exact CLI surface of `ark agent spec import` was referenced in the task brief but doesn't appear in `crates/ark-core/src/commands/agent/spec/`; if it exists at all, it's not in the current source tree.
- The OpenSpec brownfield model details (delta markers) were sourced from third-party summaries; the OpenSpec source tree itself was not surveyed.
- Cross-harness migration scripts may exist as community tooling (gists, blog posts) but were not surfaced by the searches conducted.

## Directions for Ark

1. **Add a `docs/book/src/getting-started/brownfield-adoption.md` page.** Document the 5-step playbook above. Cite real repo structures users might find (CONTRIBUTING.md, .editorconfig, CODEOWNERS) and how each maps to Ark concepts. The largest discoverability win for brownfield adopters; cheapest to ship.

2. **Implement `ark agent spec import <path>`.** Read a Markdown file (CONTRIBUTING.md, STYLE.md, etc.); prompt the user for a SPEC slug and scope; emit `.ark/specs/project/<slug>/SPEC.md` with the source file's content as the body and a row in `INDEX.md`. The transformation is mechanical — the value is in shortening "open editor, copy-paste, write INDEX row" into one command. Mentioned in the task brief; the missing CLI surface.

3. **Add `ark doctor` to detect adoption gotchas.** Checks: `.ark/specs/project/INDEX.md` has 0 rows? (Likely missing conventions.) `.claude/commands/ark/` contains non-Ark slash commands not in the manifest? (User-authored; report cleanly.) `CLAUDE.md` exists but contains no `<!-- ARK -->` block? (Partial install; recoverable.) `task.toml` exists with `phase = Committed` but `archived_at` empty? (Pending archive.) Each check should print a one-line problem + suggested command.

4. **Surface "no project SPECs registered" warning in `ark context`.** The session context already enumerates project SPECs (`specs.project` in the JSON). When the list is empty, emit a one-line warning: `warning: no project SPECs registered; agents will not enforce any team conventions on tasks. See ark agent spec import`. Brownfield adopters who skip step 3 of the playbook get an in-workflow nudge.

5. **Provide a migration target for OpenSpec-shaped repos.** `ark migrate openspec` would walk a `specs/` tree shaped like OpenSpec (proposal/apply/archive directories), pull the proposal contents into draft `specs/features/<path>/SPEC.md`, and seed an `INDEX.md` row. Even a partial / interactive port would shrink the friction of moving teams from OpenSpec to Ark. Not a high-frequency need, but cheap signal for the value of "Ark plays well with prior art."
