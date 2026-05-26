# Your First Task

Walking through a complete `/ark:quick` end-to-end. Quick tier is the smallest ceremony — one artifact (`PRD.md`), no plan, no review. We'll fix a real issue: a typo in the project README.

This guide assumes you've run `ark init` and the project is open in Claude Code. The flow is identical in Codex (`ark-quick` skill) and OpenCode (`/ark:quick`).

## Step 1: Trigger the slash command

```
/ark:quick fix typo in readme
```

Behind the scenes, the slash command:

1. Calls `ark context --scope phase --for design --format json` to read git state and project conventions.
2. Asks the agent to scaffold the task: `ark agent task new --slug fix-readme-typo --title "fix typo in readme" --tier quick`.
3. Drops a `PRD.md` template into `.ark/tasks/fix-readme-typo/`.

Slug is derived from the title — lowercase, hyphenated, ASCII, ≤40 chars.

## Step 2: Fill the PRD

The agent opens `.ark/tasks/fix-readme-typo/PRD.md` and fills four sections:

```markdown
[**What**]
Fix the typo "agentic" → "agent" in README.md, line 12.

[**Why**]
Reader-reported. Spelling.

[**Outcome**]
- README.md line 12 reads "agent" not "agentic".
- No other text changes.

[**Related Specs**]
None — typo fix.
```

Quick-tier PRDs are usually 4–6 lines per section. The Outcome doubles as the verification checklist; there's no separate VERIFY phase.

## Step 3: Execute

The agent advances the phase:

```bash
ark agent task execute
```

This transitions `task.toml.phase` from `design` → `execute`. Now the agent edits the README. For a typo fix, that's literally one `Edit` call. The agent runs whatever checks the project enforces (`cargo build`, `pytest`, etc.), confirms they pass, and reports back.

## Step 4: Commit

You decide when to close out — Ark deliberately stops at the end of execute and waits.

```
/ark:commit
```

This calls `ark agent task commit`, which lands the staged work in a single git commit (with rollback on any failure) and flips the task to `phase = committed`. The directory stays at `.ark/tasks/fix-readme-typo/` until you bulk-archive it later via `ark archive`.

## What you have at the end

- One commit on the branch (the typo fix itself, plus the `task.toml` flip).
- A `phase = committed` task ready for `ark archive` to move it into `.ark/tasks/archive/<YYYY-MM>/`.
- An archived `PRD.md` you can grep for later.

## What about the other tiers?

The flow is the same shape but with more (or fewer) artifacts:

- **Standard** adds a `PLAN.md` between design and execute, plus a `VERIFY.md` between execute and archive.
- **Deep** does what standard does, plus a `00_REVIEW.md` paired with `00_PLAN.md`. If the review verdict isn't *Approved*, the agent copies both to `01_*` and iterates. On commit, the final PLAN's `## Spec` section gets promoted to `.ark/specs/features/<path>/SPEC.md`.
- **Research** (`/ark:research`) goes the *other* direction — no PLAN, no VERIFY. It gathers a reference corpus under `research/` when you can't yet write a PRD. See [Tiers → Research](../workflow/tiers.md).

For details, see [Lifecycle](../workflow/lifecycle.md).
