---
description: Start a quick-tier task. For trivial, reversible changes. Produces PRD.md only.
argument-hint: "<title>"
---

# `/ark:quick $ARGUMENTS`

Create a quick-tier task for a trivial, reversible change. No clarifying questions, no PLAN, no VERIFY.

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

### 2. Derive slug and create task directory

Turn the title into a slug: lowercase, hyphen-separated, ASCII, ≤40 chars.

```bash
SLUG="<derive-from-title>"
mkdir -p ".ark/tasks/$SLUG"
echo "$SLUG" > .ark/tasks/.current
```

If `.ark/tasks/$SLUG/` already exists, stop — pick a more specific name.

### 3. Scan feature specs index

```bash
cat .ark/specs/features/INDEX.md
```

Identify any feature SPEC this change touches. You'll record them in PRD's `[**Related Specs**]`.

### 4. Write PRD

Copy `.ark/templates/PRD.md` into the task dir and fill it in:

```bash
cp .ark/templates/PRD.md .ark/tasks/$SLUG/PRD.md
```

Fill four sections:
- **What** — one-line description
- **Why** — the reason
- **Outcome** — observable success criteria (doubles as verification checklist for quick tier)
- **Related Specs** — any `specs/features/<name>/SPEC.md` this change touches (or leave blank)

### 5. Write `task.toml`

```toml
id = "<slug>"
title = "<title>"
tier = "quick"
phase = "execute"
status = "in_progress"
created_at = "<ISO-8601 UTC>"
updated_at = "<ISO-8601 UTC>"
```

Write it to `.ark/tasks/$SLUG/task.toml`.

### 6. Implement the change

Follow the PRD's Outcome. Stay within scope — if work grows beyond trivial, stop and suggest promoting to standard.

### 7. Verify against PRD's Outcome

Run whatever check the Outcome describes (test, build, manual). Record the result by updating PRD's Outcome section with what you verified.

### 8. Commit

The user commits. Do not run `git commit` — show the diff and let the user decide.

### 9. Archive

Once the user confirms the commit succeeded:

```bash
ARCHIVE_DIR=".ark/tasks/archive/$(date -u +%Y-%m)"
mkdir -p "$ARCHIVE_DIR"
mv ".ark/tasks/$SLUG" "$ARCHIVE_DIR/"
rm -f .ark/tasks/.current
```

Update `task.toml` before moving to set `phase = "archived"` and add `archived_at = "<ISO-8601 UTC>"`.

## If the task grows mid-flight

Stop. Tell the user: "This change is larger than quick-tier scope. Recommend promoting to standard (`/ark:design`) — I'll preserve the PRD as historical context." Wait for user decision.
