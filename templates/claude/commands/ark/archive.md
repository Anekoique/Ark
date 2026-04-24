---
description: Close out the current (or a named) Ark task. Moves it to archive; on deep tier, extracts and registers the feature SPEC.
argument-hint: "[<slug>]"
---

# `/ark:archive $ARGUMENTS`

Archive a completed Ark task. Explicit, user-invoked — `/ark:design` and `/ark:quick` deliberately stop at VERIFY without archiving.

## Preconditions

- The task has completed its tier's final pre-archive phase:
  - Quick: `phase = "execute"`
  - Standard/Deep: `phase = "verify"` with a VERIFY verdict of *Approved* or *Approved with Follow-ups*.
- The user has confirmed they want to archive now (not implicit; they typed `/ark:archive`).

If VERIFY was *Rejected*, refuse and tell the user to address findings first.

## Steps

### 1. Resolve the slug

Parse `$ARGUMENTS`:
- If a slug is given, use it.
- Otherwise, use `.ark/tasks/.current` (the CLI defaults to it automatically).

### 2. Pre-archive sanity check

```bash
cat .ark/tasks/<slug>/task.toml
```

Confirm `phase` matches one of the preconditions above. If not, halt and report the mismatch to the user.

For standard/deep tiers, skim `VERIFY.md`'s Verdict line. If it's *Rejected*, halt.

### 3. Run the archive

```bash
ark agent task archive            # uses .ark/tasks/.current
# or
ark agent task archive --slug <slug>
```

This single command:
- Transitions `task.toml.phase` to `Archived` and sets `archived_at` to now (UTC).
- **Deep tier only:** extracts the final PLAN's `## Spec` section to `.ark/specs/features/<slug>/SPEC.md` (appends a CHANGELOG entry if the SPEC already existed), then upserts the corresponding row in `.ark/specs/features/INDEX.md`'s `ARK:FEATURES` managed block.
- Moves `.ark/tasks/<slug>/` → `.ark/tasks/archive/YYYY-MM/<slug>/`.
- Clears `.ark/tasks/.current` if it pointed at this slug.

### 4. Report to user

Summarize in one message:
- Tier and slug.
- Archive path.
- Deep-tier only: the promoted SPEC path and the INDEX row.
- Any follow-ups from VERIFY worth re-surfacing.

## Failure modes

- `TaskNotFound` → no task dir at `.ark/tasks/<slug>/`. Likely already archived or wrong slug.
- `IllegalPhaseTransition` → task isn't ready (wrong phase for its tier). Tell the user what phase it's in and what's expected.
- `SpecSectionMissing` (deep tier) → the final `NN_PLAN.md` has no `## Spec` section. The task isn't really done — tell the user to finish PLAN first.
- `ManagedBlockCorrupt` → `specs/features/INDEX.md` has a `ARK:FEATURES:START` without matching `END`. Repair the file, then retry.
