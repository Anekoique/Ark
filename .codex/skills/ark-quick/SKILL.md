---
name: ark-quick
description: Start a quick-tier Ark task. For trivial, reversible changes. Produces PRD.md only. Use when the user asks for a small fix, typo, or one-line change that's reversible in a single commit.
---

# `ark-quick`

Create a quick-tier task for a trivial, reversible change. No clarifying questions, no PLAN, and no separate VERIFY.md artifact.

Structural operations (task dir creation, phase transitions, archive moves) are handled by `ark agent` — do not hand-edit `task.toml` or move directories with `mv`.

## Preconditions

- `.ark/` is initialized.
- The change is reversible in one commit and introduces no new abstractions.
  If not, stop and suggest `ark-design` (standard) or `ark-design --deep` instead.

## Steps

### 1. Pull project context

```bash
ark context --scope phase --for design --format json
```

The output is the authoritative snapshot of `.ark/`, git, and project specs for the design phase. Read it before reading the workflow doc — it tells you what specs to consult, what tasks are active, and where you're starting from.

`.ark/workflow.md` is also worth a quick scan if you haven't read it recently:

```bash
cat .ark/workflow.md
```

### 2. Create the task

Turn the title into a slug: lowercase, hyphen-separated, ASCII, ≤40 chars.

```bash
ark agent task new --slug <slug> --title "<title>" --tier quick
```

This scaffolds `.ark/tasks/<slug>/` with `PRD.md` + `task.toml`, and points `.ark/tasks/.current` at the new slug. Refuses if the slug already exists.

### 3. Fill the PRD

Edit `.ark/tasks/<slug>/PRD.md`:
- **What** — one-line description
- **Why** — the reason
- **Outcome** — observable success criteria (doubles as verification checklist for quick tier)
- **Related Specs** — any `specs/features/<name>/SPEC.md` this change touches (or leave blank)

### 4. Advance to execute

```bash
ark agent task execute
```

### 5. Implement the change

Follow the PRD's Outcome. Stay within scope — if work grows beyond trivial, stop and suggest promoting to standard.

### 6. Verify against PRD's Outcome

Run whatever check the Outcome describes (test, build, manual). Record the result by updating PRD's Outcome section with what you verified.

### 7. Commit

The user commits. Do not run `git commit` — show the diff and let the user decide.

### 8. Archive

Once the user confirms the commit succeeded, tell the user: "Use `ark-archive` to close out the task." Do NOT archive automatically. See `ark-archive`.

## If the task grows mid-flight

Stop. Tell the user: "This change is larger than quick-tier scope. Recommend promoting to standard (`ark-design`) — I'll preserve the PRD as historical context." Wait for user decision.

To promote mid-flight:

```bash
ark agent task promote --to standard
```

Then continue from Phase 2 of `ark-design` (write PLAN, etc.). Existing artifacts are preserved — the agent decides what to reshape.
