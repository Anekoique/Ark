---
description: Start a standard or deep-tier task. Produces PRD → PLAN → (REVIEW loop if --deep) → EXECUTE → VERIFY.
argument-hint: "[--deep] <title>"
---

# `/ark:design $ARGUMENTS`

Create a standard-tier task (default) or deep-tier task (if `--deep` is in arguments).

- **Standard** — feature work with testable scope. Single PLAN, no REVIEW loop, single VERIFY gate.
- **Deep** — architectural or cross-cutting work. Iterated PLAN ↔ REVIEW loop, VERIFY gate, SPEC extracted on archive.

Parse `$ARGUMENTS`:
- If it contains `--deep`, tier = `deep`, title = remainder.
- Otherwise, tier = `standard`, title = `$ARGUMENTS`.

## Preconditions

- `.ark/` is initialized.
- For **standard**: change is feature-scoped, testable, doesn't break APIs or architecture. If it does, use `--deep`.
- For **deep**: change is architectural, cross-cutting, or introduces a new subsystem.

## Phase 1 — DESIGN

### 1.1 Read workflow & project specs

```bash
cat .ark/workflow.md
cat .ark/specs/project/INDEX.md
```

Read every SPEC referenced in the project INDEX.

### 1.2 Scan feature specs

```bash
cat .ark/specs/features/INDEX.md
```

Identify any feature SPEC relevant to the task.

### 1.3 Brainstorm with the user

**Standard tier** — ask up to **3** clarifying questions focused on what's ambiguous in the title. Examples:
- What's the observable outcome?
- Any constraints you'd call out up front?
- Any existing patterns I should follow?

**Deep tier** — conduct a thorough brainstorm covering:
- Problem framing and non-goals
- Boundaries and constraints (performance, security, compatibility)
- Alternatives considered and why rejected
- Risks and assumptions
- Interaction with existing feature SPECs

Do not proceed until the user confirms direction.

### 1.4 Create task directory

Derive slug from title: lowercase, hyphen-separated, ASCII, ≤40 chars.

```bash
SLUG="<derive-from-title>"
mkdir -p ".ark/tasks/$SLUG"
echo "$SLUG" > .ark/tasks/.current
```

Refuse if `.ark/tasks/$SLUG/` already exists.

### 1.5 Write PRD

```bash
cp .ark/templates/PRD.md .ark/tasks/$SLUG/PRD.md
```

Fill:
- **What** — one-line description
- **Why** — the reason
- **Outcome** — observable success criteria
- **Related Specs** — feature specs this task touches (list each path with one line on how it interacts)

### 1.6 Write `task.toml`

```toml
id = "<slug>"
title = "<title>"
tier = "standard"   # or "deep"
phase = "plan"
status = "in_progress"
iteration = 0
max_iterations = 3   # deep only — agent judges based on complexity; omit for standard
created_at = "<ISO-8601 UTC>"
updated_at = "<ISO-8601 UTC>"
```

Write to `.ark/tasks/$SLUG/task.toml`.

## Phase 2 — PLAN

### 2.1 Copy PLAN template

```bash
cp .ark/templates/PLAN.md .ark/tasks/$SLUG/00_PLAN.md
```

### 2.2 Fill the PLAN

Using the PRD and related specs as input, fill `00_PLAN.md`:

- Frontmatter: Status = `Draft`, Iteration = `00`, Depends on = PRD + related specs
- `## Summary` — what this PLAN proposes
- `## Log` — *None in 00_PLAN*
- `## Spec` — Goals (G-N), Non-goals (NG-N), Architecture, Data Structure, API Surface, Constraints (C-N)
- `## Runtime` — Main Flow, Failure Flow, State Transitions
- `## Implementation` — phases (Phase 1, Phase 2, Phase 3 as needed)
- `## Trade-offs` — options with adv./disadv. (T-N)
- `## Validation` — Unit/Integration/Failure/Edge tests (V-*-N), Acceptance Mapping linking each G/C to a V

**Gate:** every Goal (G-N) must be mapped to at least one Validation (V-*-N) in the Acceptance Mapping table.

### 2.3 Advance

**Standard tier:** set `task.toml.phase = "execute"`. Skip to Phase 4.

**Deep tier:** set `task.toml.phase = "review"` and proceed to Phase 3.

## Phase 3 — REVIEW (deep tier only — plan review loop)

### 3.1 Copy REVIEW template

```bash
cp .ark/templates/REVIEW.md .ark/tasks/$SLUG/00_REVIEW.md
```

### 3.2 Act as reviewer

Ideally, this is a fresh agent or a reviewer model. For this command, ask the user: *"Should I self-review, or will you run the reviewer?"*

If self-review: switch framing — *you are now the reviewer*. Read 00_PLAN.md critically against the PRD and project specs. Fill 00_REVIEW.md with:

- Verdict (Approved / Approved with Revisions / Rejected)
- Blocking / Non-blocking counts
- Findings (R-NNN) with Severity, Section, Problem, Why it matters, Recommendation
- Trade-off Advice (TR-N)

### 3.3 Loop if revisions needed

If verdict is *Rejected* or *Approved with Revisions*:

1. Increment iteration: `NN = 01`, `02`, ...
2. Copy PLAN template to `NN_PLAN.md`
3. Fill the `Response Matrix` in `## Log` — every prior CRITICAL/HIGH finding must appear with Accepted/Rejected/Deferred + reasoning
4. Revise the relevant sections
5. Copy REVIEW template to `NN_REVIEW.md` and review again
6. Repeat until verdict is *Approved* (zero open CRITICAL)

**Max iterations** is recorded in `task.toml.max_iterations` (agent-judged — typically 3–5 for deep). If exhausted without approval, halt and ask the user how to proceed.

### 3.4 Advance

When the latest REVIEW is *Approved* with zero open CRITICAL, set `task.toml.phase = "execute"`.

## Phase 4 — EXECUTE

### 4.1 Implement the plan

Work through the latest PLAN's Implementation phases. Follow project specs and related feature SPECs.

If implementation reveals gaps in the design, **update the latest PLAN's `## Spec` section** to reflect reality. Do not silently diverge.

### 4.2 Run checks

Run whatever checks the project enforces (tests, lints, builds). Implementation is complete when checks pass and code is committed (by the user).

### 4.3 Advance

Once code is committed: set `task.toml.phase = "verify"`.

## Phase 5 — VERIFY

### 5.1 Copy VERIFY template

```bash
cp .ark/templates/VERIFY.md .ark/tasks/$SLUG/VERIFY.md
```

### 5.2 Act as verifier

Ideally a fresh agent or reviewer model. If self-verifying, apply the **higher quality bar**: this is not just "does it work" — it covers plan fidelity, correctness, code quality, organization, abstraction, and SPEC drift.

Fill VERIFY.md:
- Verdict (Approved / Approved with Follow-ups / Rejected)
- Findings (V-NNN) with Severity, Scope, Location, Problem, Why it matters, Expected
- Follow-ups (FU-NNN) if any

**VERIFY does not loop.** Single-pass gate.

### 5.3 Decide

- **Approved / Approved with Follow-ups** → proceed to Phase 6 (archive). If follow-ups exist, report them to the user; they can create new tasks for them.
- **Rejected** → halt. Summarize findings to the user and ask how to proceed (create fix tasks, promote tier, accept with acknowledgement, discard).

## Phase 6 — ARCHIVE

### 6.1 Update task.toml

Set `phase = "archived"`, add `archived_at = "<ISO-8601 UTC>"`.

### 6.2 Deep-tier only: extract SPEC

Take the `## Spec` section from the final PLAN (highest NN). Write it to:

```bash
mkdir -p .ark/specs/features/$SLUG
cp .ark/templates/SPEC.md .ark/specs/features/$SLUG/SPEC.md
# Fill SPEC.md from the final PLAN's ## Spec section
```

If the task modified an existing feature SPEC, append a `[**CHANGELOG**]` entry instead of overwriting.

Then update the managed block in `specs/features/INDEX.md`:

```
<!-- ARK:FEATURES:START -->
| <feature> | <one-line scope> | <YYYY-MM-DD> from task `<slug>` |
<!-- ARK:FEATURES:END -->
```

Append the new row (or update the existing row's Promoted date if this task revised an existing feature).

### 6.3 Move task directory

```bash
ARCHIVE_DIR=".ark/tasks/archive/$(date -u +%Y-%m)"
mkdir -p "$ARCHIVE_DIR"
mv ".ark/tasks/$SLUG" "$ARCHIVE_DIR/"
rm -f .ark/tasks/.current
```

### 6.4 Report to user

Summarize: tier, final verdict, any follow-ups, promoted SPEC path (deep), archive location.
