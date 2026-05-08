# `doc-tighten` PLAN

> Status: Draft
> Feature: `doc-tighten`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - PRD: `PRD.md`
> - Related specs: `ark-context`, `ark-agent-namespace`, `ark-workflow-refactor`, project `LAYOUT.md`

---

## Summary

Rewrite the agent-facing surface (Claude commands, Codex skills, OpenCode commands), `workflow.md`, and the artifact templates (PRD/PLAN/REVIEW/VERIFY/SPEC) into a tabular, Trellis-style contract. Single source of truth: `workflow.md` carries cross-cut rationale; commands point to it. Templates use schema-marker placeholders that constrain agent output rather than narrating intent.

## Log `None in 00_PLAN`

---

## Spec

[**Goals**]

- G-1: Each command/skill file is in tabular `Step N:` format with `[AI]` / `[USER]` markers and an explicit gate per step. Per-step prose is one line max.
- G-2: Cross-cut rationale (what `ark context --for X` returns, what each `ark agent task <verb>` does) lives in `workflow.md` once. Commands reference, not duplicate.
- G-3: Templates use schema markers (`<one-line description>`) instead of pep-talk hints (`{Clear definition of the "What" and "Why".}`). Agents are constrained, not prompted.
- G-4: Three platform mirrors (Claude `.md`, Codex `SKILL.md`, OpenCode `.md`) stay synchronised — same trim applied uniformly, only frontmatter differs.
- G-5: No semantic loss — every CLI command, gate, failure mode, and rule that exists today still exists post-rewrite.
- G-6: Command/skill files shrink ~30% (e.g. `design.md` 227→~160; `commit.md` 143→~100). `workflow.md` stays roughly the same length but absorbs cross-cut content cleanly. PLAN.md template drops from 185→~110.

[**Non-goals**]

- NG-1: No CLI surface changes — the verbs, flags, and exit codes stay identical.
- NG-2: No SPEC churn under `.ark/specs/features/` or `.ark/specs/project/` — this is a docs/templates pass only.
- NG-3: No `.ark/.installed.json` or `config.toml` rewrites.
- NG-4: No new commands/skills, and no command merges (e.g. don't combine `quick` + `design`).

[**Architecture**]

```
templates/ark/workflow.md          ← single source of truth (rationale + tier table + phase contracts)
                ▲
                │ links to
                │
templates/<platform>/commands/ark/*.md     ← tabular Step N: format, [AI]/[USER] markers
templates/codex/skills/ark-*/SKILL.md      ← same content, different frontmatter

templates/ark/templates/{PRD,PLAN,REVIEW,VERIFY,SPEC}.md  ← schema-marker placeholders
```

[**Data Structure**]

Doc structure (per command/skill):

```
---
description: <one-line; existing>
argument-hint: "<existing — Claude only>"
---

# `/ark:<verb> $ARGUMENTS`

<one-paragraph: what this does, who runs it>

## Preconditions
- <bullet>
- <bullet>

## Steps

### Step 1: <verb-phrase> `[AI]` | `[USER]`
```bash
<command>
```
<one line: what advances; what's the gate>

### Step 2: ...

## Failure Modes

| Code | Cause | Recovery |
|------|-------|----------|
| ... | ... | ... |

## See Also
- `workflow.md` §<section>
```

Template marker style change:

```
old:  `{Clear definition of the "What" and "Why".}`
      - G-1: ...
      - G-2: ...

new:  <one bullet per goal — what observable behaviour ships>
      - G-1:
      - G-2:
```

[**API Surface**]

No CLI changes. Doc-only changes mean no public-API surface modifications. Files modified:

```
templates/ark/workflow.md
templates/ark/templates/PRD.md
templates/ark/templates/PLAN.md
templates/ark/templates/REVIEW.md
templates/ark/templates/VERIFY.md
templates/ark/templates/SPEC.md
templates/claude/commands/ark/{design,quick,commit,resume,discard,record}.md
templates/codex/skills/ark-{design,quick,commit,resume,discard,record}/SKILL.md
templates/opencode/commands/ark/{design,quick,commit,resume,discard,record}.md
templates/ark/specs/INDEX.md          (light)
templates/ark/specs/features/INDEX.md  (light)
templates/ark/specs/project/INDEX.md   (light)
```

[**Constraints**]

- C-1: Commands keep their existing frontmatter (`description`, `argument-hint`). Don't change argument-hint values.
- C-2: Every CLI invocation that appears in the current command/skill files must still appear post-rewrite (verbatim). The trim is in the surrounding prose, not in the commands themselves.
- C-3: Failure modes documented in the current files must survive in a `## Failure Modes` table; nothing is dropped.
- C-4: The `--worktree` instructions, `cd .ark/worktrees/<branch>/` reminders, and the deep-tier MUST-use-worktree rule must remain explicit (these are easy to lose in a trim).
- C-5: PRD/PLAN/REVIEW/VERIFY templates are *seeded by the CLI* (`ark agent task new` copies PRD; `task plan` seeds PLAN; etc.) and `VERIFY.md` is auto-populated with `{{PROJECT_SPEC_COMPLIANCE}}` etc. Marker placeholders the CLI substitutes (e.g. `{{PLAN_FIDELITY}}`) MUST be preserved verbatim.
- C-6: Three-way platform parity: every change to a Claude command must be applied identically (modulo frontmatter) to the matching Codex skill and OpenCode command. Verify via diff post-edit.
- C-7: `templates/ark/templates/SPEC.md` is the body promoted into `specs/features/<slug>/SPEC.md` on deep-tier commit; the `[**CHANGELOG**]` section and the `Goals / Architecture / Data Structure / API Surface / Constraints` block must remain (CLI relies on this shape).

## Runtime

[**Main Flow**]

1. Phase 1 — workflow.md restructure: introduce a "Phase contracts" table that lists, per phase, the `ark context --for <phase>` projection contents + the `ark agent task <verb>` action. Move scattered rationale into this table. Trim §6 "Mechanics" prose; keep CLI invariants.
2. Phase 2 — template tightening: rewrite PRD, PLAN, REVIEW, VERIFY, SPEC with schema markers. Preserve CLI placeholders (`{{...}}`). Verify shape (heading order, `[**...**]` block count) is unchanged.
3. Phase 3 — command/skill rewrite (Claude first): apply the tabular Step N: format with `[AI]` / `[USER]` markers, gates, and `## Failure Modes` tables. Reference workflow.md sections instead of restating.
4. Phase 4 — platform mirroring: replicate Claude trims into Codex skills and OpenCode commands. Diff each pair to confirm only frontmatter + `argument-hint` lines differ.
5. Phase 5 — INDEX touches: trim `templates/ark/specs/INDEX.md` and the two child INDEX scaffolds for consistency with the new style.

[**Failure Flow**]

1. If a CLI invocation is dropped during trim → diff against pre-rewrite reveals it; restore.
2. If a marker placeholder (e.g. `{{PROJECT_SPEC_COMPLIANCE}}`) is munged → CLI seeding will produce a malformed VERIFY.md at next `task verify`. Catch via grep before commit.
3. If platforms drift → side-by-side diff in Phase 4 fails; re-sync.

[**State Transitions**]

- Files: `pre-trim → trimmed (in worktree)` per file. No CLI state changes.
- Task phase: design → plan → execute → verify → committed (standard tier path).

## Implementation

[**Phase 1 — workflow.md**]

- Add §3.5 (or equivalent) "Phase contracts" table:

  | Phase | Context projection | Mutating CLI | Gate |
  |-------|--------------------|--------------|------|
  | design | git, project specs, features index, recent archive | `task new --slug <s> --title "<t>" --tier <t> [--worktree]` | PRD complete |
  | plan | + current PRD + related feature specs | `task plan` | every G-N mapped to V-*-N |
  | review | + latest PLAN | `task review` | verdict Approved, zero open CRITICAL |
  | execute | + git dirty + latest PLAN | `task execute` | implementation complete; checks pass |
  | verify | + VERIFY.md path | `task verify` | every checklist item non-PENDING |
  | commit | body-free; paths to VERIFY + latest PLAN | `task commit -m "<msg>"` | one atomic git commit |

- Trim Lifecycle ascii art? Keep it; it's load-bearing for orientation.
- Compress §6 "Mechanics" prose into a tighter table where possible.
- Preserve §3 tier table verbatim (it's already tight).

[**Phase 2 — Templates**]

PRD.md (23→~18 lines):
- Replace `{One-line description of the change or feature.}` → `<one-line description>`
- Same pattern for Why / Outcome / Related Specs.

PLAN.md (185→~110 lines):
- Schema markers throughout. The Log section's `[**Added/Changed/Removed/Unresolved/Response Matrix**]` template stays (this is filled iteration 1+; iter 0 says `None in 00_PLAN`).
- Trade-offs / Validation marker examples shrink to single-line schemas.

REVIEW.md (85→~60 lines):
- Verdict + Findings shape preserved. Marker prose shrinks.

VERIFY.md (87→~60 lines):
- The `{{PROJECT_SPEC_COMPLIANCE}}` etc. markers are NOT touched (CLI substitutes them).
- The Findings example block trims its hint prose.

SPEC.md (51→~35 lines):
- Goals / NG / Architecture / Data Structure / API Surface / Constraints — identical shape. Hint prose shrinks.

[**Phase 3 — Claude commands**]

Six files. Apply the doc-structure schema (§Spec → Data Structure):
- `design.md` 227→~160: drop the embedded tier comparison (it's in workflow.md §3); compress phase intros; keep all CLI calls and the `--worktree` rule.
- `quick.md` 95→~70.
- `commit.md` 143→~100: preserve all 5 failure modes; collapse the journal-style paragraph into the existing constraint block.
- `resume.md` 40→~30.
- `discard.md` 54→~40.
- `record.md` 80→~50.

[**Phase 4 — Mirror to Codex + OpenCode**]

For each Claude file, copy content into the matching Codex `SKILL.md` (frontmatter is `name:` + `description:`) and OpenCode `*.md` (frontmatter is `description:` only — no `argument-hint`). Replace `/ark:<verb>` with the appropriate slash form for OpenCode (same), and with `ark-<verb>` skill name for Codex. Confirm post-trim line counts match within 5% across mirrors.

[**Phase 5 — INDEX scaffolds**]

- `templates/ark/specs/INDEX.md`: already tight; light touch only.
- `templates/ark/specs/project/INDEX.md`: keep the Layout-A guidance; tighten "How to Use" prose.
- `templates/ark/specs/features/INDEX.md`: keep markers (`<!-- ARK:FEATURES:START -->`); shrink "How to Use" prose.

## Trade-offs

- T-1: **Reference vs replicate rationale.** Pulling rationale into workflow.md saves bytes but means each command/skill needs `## See Also` + a one-line orientation. We accept the indirection because workflow.md is loaded by every fresh agent during the design step (`cat .ark/workflow.md` is in every command).
- T-2: **Schema markers vs hint prose in templates.** Schema markers (`<one-line description>`) constrain agent output but lose mentorship. Mitigated by workflow.md's phase contracts table — the agent sees the constraint *and* the rationale, just in different files.
- T-3: **Three-platform parity vs single canonical doc.** Could collapse Codex skills into transcluding Claude commands at install time, but that's an `ark init` change (out of scope per NG-1). Stick with copy-on-install.

## Validation

[**Unit / structural checks**]

- V-UT-1: After rewrite, `grep -c '^### Step ' templates/claude/commands/ark/design.md` ≥ 12 (sanity: phases × steps preserved).
- V-UT-2: `grep -E 'ark agent task (new|plan|review|execute|verify|commit|discard|resume)' templates/claude/commands/ark/design.md` returns the same set as pre-rewrite (no CLI dropped).
- V-UT-3: For each marker placeholder (`{{PROJECT_SPEC_COMPLIANCE}}`, `{{RELATED_FEATURE_COMPLIANCE}}`, `{{PRD_CONSTRAINTS}}`, `{{PLAN_FIDELITY}}`), `grep -F '{{<marker>}}' templates/ark/templates/VERIFY.md` returns a hit.
- V-UT-4: Wordcount delta — `wc -l` for the six Claude commands sums to ≥ 25% reduction.

[**Integration checks**]

- V-IT-1: Run `ark agent task new --slug doc-tighten-test --tier quick --worktree` from a fresh shell after rewrite and verify the seeded `PRD.md` is the new tighter PRD (CLI reads from `templates/`, so this confirms install-path correctness).
- V-IT-2: Run `ark agent task verify` (in a separate dummy task) and verify the seeded `VERIFY.md` still substitutes the auto-populated sections — confirms CLI placeholders were preserved.
- V-IT-3: Diff Claude-vs-OpenCode for each command — only frontmatter and `argument-hint` lines should differ.

[**Failure / Robustness**]

- V-F-1: Spot-check that `## Failure Modes` table in each command lists every failure that the prior prose flagged (e.g. `commit.md` lists all 5: NothingStaged, VerifyIncomplete, CommitMessageRequired, GitCommitFailed, IllegalPhaseTransition).
- V-F-2: Worktree mention check — `grep '\.ark/worktrees/<branch>/' templates/claude/commands/ark/design.md` returns ≥ 2 hits (deep tier MUST-rule + the cd reminder).

[**Edge cases**]

- V-E-1: Templates with CLI placeholders (VERIFY.md `{{...}}` and SPEC.md `[**CHANGELOG**]`) — confirm they survive verbatim.
- V-E-2: PRD has the special `[**Related Specs**]` block that the CLI parses to filter feature specs in the plan-phase context call; ensure the bracket block markers are preserved exactly.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (tabular Step N: format) | V-UT-1 |
| G-2 (rationale lives in workflow.md once) | V-UT-2, V-UT-4 |
| G-3 (schema markers in templates) | V-UT-3, V-IT-1 |
| G-4 (three-platform parity) | V-IT-3 |
| G-5 (no semantic loss) | V-UT-2, V-F-1, V-F-2 |
| G-6 (~30% size reduction) | V-UT-4 |
| C-1 (frontmatter preserved) | V-IT-3 |
| C-2 (every CLI call preserved) | V-UT-2 |
| C-3 (failure modes preserved) | V-F-1 |
| C-4 (worktree instructions explicit) | V-F-2 |
| C-5 (CLI placeholders verbatim) | V-UT-3, V-IT-2 |
| C-6 (three-platform diff is frontmatter-only) | V-IT-3 |
| C-7 (SPEC.md shape preserved) | V-UT-3 (extended to SPEC) |
