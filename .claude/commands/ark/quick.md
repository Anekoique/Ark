---
description: Start a quick-tier task. For trivial, reversible changes. Produces PRD.md only.
argument-hint: "<title>"
---

# `/ark:quick $ARGUMENTS`

Create a quick-tier task for a trivial, reversible change. No clarifying questions, no PLAN, and no separate VERIFY.md artifact.

Structural operations (task dir creation, phase transitions, archive moves) are handled by `ark agent` — do not hand-edit `task.toml` or move directories with `mv`.

## Preconditions

- `.ark/` is initialized.
- The change is reversible in one commit and introduces no new abstractions.
  If not, stop and suggest `/ark:design` (standard) or `/ark:design --deep` instead.

## Steps

### 1. Read workflow & project specs

```bash
cat .ark/workflow.md
cat .ark/specs/project/INDEX.md
```

Project specs listed in `specs/project/INDEX.md` apply to every task — read each `SPEC.md` referenced there before touching code.

### 2. Scan feature specs index

```bash
cat .ark/specs/features/INDEX.md
```

Identify any feature SPEC this change touches. You'll record them in PRD's `[**Related Specs**]`.

### 3. Create the task

Turn the title into a slug: lowercase, hyphen-separated, ASCII, ≤40 chars.

```bash
ark agent task new --slug <slug> --title "<title>" --tier quick
```

This scaffolds `.ark/tasks/<slug>/` with `PRD.md` + `task.toml`, and points `.ark/tasks/.current` at the new slug. Refuses if the slug already exists.

### 4. Fill the PRD

Edit `.ark/tasks/<slug>/PRD.md`:
- **What** — one-line description
- **Why** — the reason
- **Outcome** — observable success criteria (doubles as verification checklist for quick tier)
- **Related Specs** — any `specs/features/<name>/SPEC.md` this change touches (or leave blank)

### 5. Advance to execute

```bash
ark agent task execute
```

### 6. Implement the change

Follow the PRD's Outcome. Stay within scope — if work grows beyond trivial, stop and suggest promoting to standard.

### 7. Verify against PRD's Outcome

Run whatever check the Outcome describes (test, build, manual). Record the result by updating PRD's Outcome section with what you verified.

### 8. Commit

The user commits. Do not run `git commit` — show the diff and let the user decide.

### 9. Archive

Once the user confirms the commit succeeded, tell the user: "Run `/ark:archive` to close out the task." Do NOT archive automatically. See `/ark:archive`.

## If the task grows mid-flight

Stop. Tell the user: "This change is larger than quick-tier scope. Recommend promoting to standard (`/ark:design`) — I'll preserve the PRD as historical context." Wait for user decision.

To promote mid-flight:

```bash
ark agent task promote --to standard
```

Then continue from Phase 2 of `/ark:design` (write PLAN, etc.). Existing artifacts are preserved — the agent decides what to reshape.
