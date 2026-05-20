# `ark-research` REVIEW `00`

> Status: Open
> Feature: `ark-research`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: 0
- Non-blocking: 7

## Summary

The PLAN's design is fundamentally sound: a fourth `Tier::Research` + `Phase::Research`, two new transition-table rows, and a tier-conditional branch in `task_commit` is the minimum-surface change that delivers the PRD's Outcome, and it does not contradict any existing feature SPEC (every cited SPEC — `ark-agent-namespace`, `subagent-support`, `detachable-feature-spec`, `worktree`, `task-concurrency-control`, `workspace`, `ark-context` — is preserved or unchanged). The Acceptance Mapping covers every Goal and every Constraint G-1..G-5 / C-1..C-17. No CRITICAL findings.

The PLAN does carry two HIGH issues worth addressing in iteration 01: (1) **C-11's "byte-identical bodies across Claude / Codex / OpenCode" claim contradicts shipping reality** — the existing Codex skills (e.g. `ark-quick/SKILL.md`) deliberately rename `/ark:quick → ark-quick`, `/ark:commit → ark-commit`, and replace `$ARGUMENTS` with `<task description>`. The PLAN inherits this misstatement from the PRD's Outcome bullet. (2) **`task_promote` interaction with `Tier::Research` is unspecified** — `phase_exists_in_tier` in `promote.rs` is exhaustive on `(Tier, Phase)`, and adding `Tier::Research` + `Phase::Research` without updating that table silently breaks both directions of promotion involving Research. The remaining five non-blocking findings are MEDIUM/LOW polish.

---

## Findings

### R-001 `byte-identical-bodies claim contradicts existing Codex skill convention`

- **Severity:** HIGH
- **Section:** `## Spec` C-11; `## API Surface` slash-command body; mirrored from PRD `[**Outcome**]` bullet 6.
- **Problem:** C-11 says "`/ark:research` slash-command body is byte-identical across Claude / Codex / OpenCode modulo per-platform frontmatter, mirroring `subagent-support` C-22." This is false on two counts. First, `subagent-support` C-22 governs **agent prompts** (`ark-researcher`, `ark-reviewer`, `ark-verifier` under `.../agents/`), not slash commands; the PLAN conflates the two file families. Second, the existing slash commands ship divergent bodies on purpose: comparing `templates/claude/commands/ark/quick.md` against `templates/codex/skills/ark-quick/SKILL.md` shows Codex renames `/ark:quick → ark-quick`, `/ark:design → ark-design`, `/ark:commit → ark-commit`, and replaces `# `/ark:quick $ARGUMENTS`` with `# `ark-quick <task description>``. The H1 line and every cross-reference differ by design — Codex skills are dispatched by skill name without a leading slash. The PLAN's V-IT-3 ("bodies byte-identical after stripping frontmatter") would fail for every existing platform-pair on the established convention.
- **Why it matters:** A parity test written to V-IT-3's letter will not exist on the current tree (no such test passes today for `quick.md` vs `ark-quick/SKILL.md`); writing one would either be vacuously true (only comparing two of three platforms) or assert a constraint Ark has actively violated for every prior slash command. The Executor will get stuck either dropping V-IT-3 or rewriting every shipped skill body. Neither is the right outcome.
- **Recommendation:** Restate the parity contract to match shipping reality. Two options: (a) "Claude and OpenCode bodies are byte-identical modulo frontmatter; Codex's SKILL.md substitutes `/ark:research` with `ark-research`, `/ark:commit` with `ark-commit`, and replaces `$ARGUMENTS` with `<topic>` (or `<task description>`), per existing pattern", with V-IT-3 widened to test the actual diff shape. Or (b) compare bodies after a documented substitution map applied to the Codex body. Update the PRD `[**Outcome**]` bullet in the same iteration so the PLAN's C-11 maps to a real Outcome.

### R-002 `task_promote silently breaks for Tier::Research`

- **Severity:** HIGH
- **Section:** `## Architecture` (module table omits `promote.rs`); `## Constraints` (C-3/C-9 do not name promote).
- **Problem:** `crates/ark-core/src/commands/agent/task/promote.rs::phase_exists_in_tier` is an exhaustive matches! over `(Tier, Phase)`: `(Tier::Quick, Design | Execute | Archived) | (Tier::Standard, Design | Plan | Execute | Verify | Archived) | (Tier::Deep, _)`. Once `Tier::Research` and `Phase::Research` are added, two failure modes appear silently: (a) `task promote --to research` on any non-Research task fails because `(Tier::Research, <existing phase>)` is never in the table; (b) `task promote --to quick|standard|deep` on a research task fails because `(Tier::Quick|Standard|Deep, Phase::Research)` is never in the table. Both produce `IllegalPhaseTransition { to: <current phase> }` — a confusing error that does not name the real reason. The PLAN does not list `promote.rs` in the Architecture file tree and does not state which of (a)(b) is intended behavior.
- **Why it matters:** Whatever the chosen policy (forbid both promotions, allow only Research→other, allow nothing), it is a design decision that needs to be recorded as a Constraint with a Validation entry. The current PLAN leaves it implicit, which means the Executor will pick one at random and the Verifier will have no contract to grade against. The PRD's Outcome bullets are also silent here — implicitly the user gets whatever the implementer chose.
- **Recommendation:** Add a Constraint (e.g. C-18) stating the explicit policy. The minimum sensible policy: "`task promote` rejects any source-or-target involving `Tier::Research` with `Error::WrongTier` or a new variant; research↔implementation cross-over is by `task new`, not promotion." Add `promote.rs` to the Architecture file tree even if "no code change" — make the omission deliberate, not accidental. Mention it in NG (or add `NG-5`) if you want to keep promotion entirely out of scope. Add V-UT-N covering the rejection.

### R-003 `workflow.md tier listing is a bullet list, not a table — C-13 wording misleads`

- **Severity:** MEDIUM
- **Section:** `## Constraints` C-13, C-14; `## Implementation` Phase 5 step 7.
- **Problem:** C-13 says "`workflow.md`'s tier table grows a Research row at the end with cell text matching the existing terse table style." There is no tier *table* in `templates/ark/workflow.md`; the file uses a three-bullet list under `## Tiers` (`- **Quick** — …`, `- **Standard** — …`, `- **Deep** — …`). Phase 5 step 7's instruction repeats the same misnomer ("Tier table: add Research row at the bottom"). Similarly the "lifecycle diagram" on line 75 is a one-line ASCII arrow, not a structured diagram — adding a track to it has a specific shape the PLAN does not call out.
- **Why it matters:** A Verifier scoring C-13 by looking for a markdown table will mark it as not-done; an Executor following Phase 5 verbatim may convert the bullets to a table just to satisfy the wording. Either way the file ends up reshaped beyond the PRD's promise of "tier table grows row 4."
- **Recommendation:** Restate C-13 as "the `## Tiers` bullet list grows a fourth bullet `- **Research** — …`" with the exact cell text inline. Restate C-14 to describe the existing ASCII arrow's shape and the precise way `Research → Committed → Archived` slots in (likely a second line beneath the first arrow). Match the Implementation step's wording.

### R-004 `slash-command body shape diverges from /ark:quick template idiom`

- **Severity:** MEDIUM
- **Section:** `## API Surface` (the markdown block inside [**API Surface**]).
- **Problem:** The proposed `/ark:research` body inlines bare commands directly under each `### Step N:` heading (e.g. `ark context --scope phase --for design --format json` on the next line, no fence). Every existing slash-command template in `templates/claude/commands/ark/*.md` and `templates/codex/skills/ark-*/SKILL.md` puts commands inside triple-backtick fenced ```bash blocks, includes per-step explanatory prose, and ships a closing `## See Also` section. The PLAN's draft also lacks the per-step "what this does" sentence the existing templates use ("Returns the snapshot of git, active tasks, and project + feature specs.") and the `## If the task grows mid-flight` / `## See Also` sections that ship across all three other commands.
- **Why it matters:** Without those subsections the research command body reads as an outline, not a runnable contract — and an AI dispatcher consuming `argument-hint`/`description` frontmatter may surface the leading prose as the description. C-11's parity test will also be brittle: if the V-IT-3 parity test eventually settles, the diff against the Codex variant will be confined to the substitution map; an outline-shaped body diverges from that pattern in more places than necessary.
- **Recommendation:** Mirror `templates/claude/commands/ark/quick.md` more closely — fenced ```bash code blocks under each step, one explanatory sentence per step, plus a "## If the corpus turns into implementation" subsection (the research analogue of `/ark:quick`'s "## If the task grows mid-flight") explaining the user's path forward: `ark agent task new --tier <quick|standard|deep>` + PRD citing the research slug, not in-place tier promotion. Add a `## See Also` block pointing at `workflow.md` §Research and `/ark:commit`.

### R-005 `phase.rs claim "no code change" is fragile under future refactors`

- **Severity:** MEDIUM
- **Section:** `## Architecture` (call graph annotation); `## Constraints` C-9.
- **Problem:** The PLAN argues `phase.rs` requires no edit because `check_transition` rejects every (`Research`, `Research`, `Plan|Review|Execute|Verify`) tuple before the `artifact_for` switch is consulted. That is true today. But there is no positive assertion that `artifact_for(Phase::Plan|Phase::Review|Phase::Verify, Tier::Research, _)` would be benign if ever reached, and there is no test that pins the contract. If a future refactor reorders `transition()` so the artifact seed runs before `check_transition`, a stray `00_PLAN.md` or `VERIFY.md` would be written into a research task dir before the error surfaces — exactly the corruption the rollback pattern elsewhere is designed to prevent.
- **Why it matters:** "Phase.rs needs no code change" is a strong claim; the PLAN's V-IT-1 / V-IT-2 only test the user-visible error, not the absence of side effects. The deep-tier `task_commit` rollback story (RollbackGuard) sets a precedent that mid-flight failures must not leave artifacts on disk.
- **Recommendation:** Add a sentence to C-9 or a sibling Constraint: "Research-tier illegal-transition errors must surface before any artifact write; V-IT-1 asserts no `*_PLAN.md` / `*_REVIEW.md` / `VERIFY.md` exists in the task dir after the error." Strengthen V-IT-1 accordingly. Optionally, in `artifact_for`, add a defensive arm `(_, Tier::Research, _) => None` so the contract holds even if the order ever changes. (One line; matches the "validate at boundaries" discipline of `rust/STYLE.md` S-37.)

### R-006 `ark_files_for_first_commit signature change is API-visible but not justified in Spec`

- **Severity:** LOW
- **Section:** `## API Surface` `ark_files_for_first_commit` declaration; T-5 trade-off.
- **Problem:** T-5 picks "thread `tier` into `ark_files_for_first_commit`" over branching at the caller, but the function is private (line 385 of `commit.rs`, no `pub`), so the comment "single call site selects the right policy" applies to both options. The chosen option is fine; the trade-off framing implies an API-shape decision (it isn't). Worse, the PLAN's prose-API table renders the function as if it has a stable signature — readers may treat it as part of the public surface to be preserved.
- **Why it matters:** Minor signal/noise issue in the Spec. The PLAN's API Surface section reads as a contract; mixing private helpers with public re-exports muddles that.
- **Recommendation:** Either move `ark_files_for_first_commit`'s signature out of `[**API Surface**]` into `[**Architecture**]` (where private call-graph entries already live), or annotate it as private with one line of prose. T-5's framing can stay as a trade-off of *where the conditional lives* rather than *what the API looks like*.

### R-007 `CLI parser claim "clap value-enum gains research" misdescribes existing parser`

- **Severity:** LOW
- **Section:** `## API Surface` "CLI surface" paragraph; `## Implementation` Phase 2 step 3.
- **Problem:** The PLAN says `--tier` is parsed by a "clap value-enum" that "gains `research`". The actual parser in `crates/ark-cli/src/agent_cli.rs::parse_tier` is a hand-rolled `match s { "quick" => …, … }` function passed via `value_parser = parse_tier`, not a clap `ValueEnum` derive. The change is still trivial — one match arm — but the PLAN's description sets the Executor up to look for a derive macro that does not exist.
- **Why it matters:** Cosmetic, but it costs the Executor a minute to discover. It also weakens the PLAN's "I read the code" signal.
- **Recommendation:** Replace the sentence with "Add a `"research" => Ok(Tier::Research)` arm to `parse_tier` and update the error message's tier list to `quick | standard | deep | research`."

---

## Trade-off Advice

### TR-1 `PRD-on-research semantic remap vs. dedicated BRIEF.md template`

- **Related Plan Item:** T-2.
- **Topic:** Compatibility vs Clean Design.
- **Reviewer Position:** Prefer A (PRD reuse) — confirmed.
- **Advice:** Adopt T-2's choice as-stated, but tighten the in-PRD semantics carved out for research tier.
- **Rationale:** Reusing `PRD.md` with documented "Outcome optional, SPEC Path ignored, Related Specs optional" semantics avoids a new template and the load/save round-trip risk that came with `ark-context`'s phase-projection schema. The cost is that the PRD template (`templates/ark/templates/PRD.md`) carries fields that are no-ops on one of four tiers; readers of a research-tier PRD see a placeholder `[**Outcome**]` and `[**SPEC Path**]` they can ignore. Acceptable.
- **Required Action:** Keep with clarification. Add one line under `[**Outcome**]` in the workflow.md "Research" subsection explicitly listing the remap (Outcome → "Why this corpus is the right next step", SPEC Path → ignored, Related Specs → optional). C-15 already commits to this; just be concrete about the field list.

### TR-2 `--worktree opt-in vs. forbidden on research tier`

- **Related Plan Item:** T-3.
- **Topic:** Flexibility vs Safety.
- **Reviewer Position:** Prefer A (opt-in).
- **Advice:** Confirm T-3 as-chosen.
- **Rationale:** Research tasks rarely conflict with in-flight code work (corpus is markdown under `.ark/tasks/<slug>/research/`, distinct from `src/`), so the typical user benefits from staying on the parent checkout. But power users running parallel research streams under different branches benefit from `--worktree` and the per-checkout focus binding. `worktree` SPEC G-4 already requires opt-in across all tiers; aligning research tier with the same rule is the lowest-surprise choice. The PLAN's C-5 captures this correctly.
- **Required Action:** Adopt. No further wording needed beyond C-5 + workflow.md §Research.

### TR-3 `research/ directory pre-creation`

- **Related Plan Item:** T-4.
- **Topic:** Compatibility vs Clean Design.
- **Reviewer Position:** Prefer A (lazy creation).
- **Advice:** Adopt T-4 as-chosen.
- **Rationale:** `subagent-support` SPEC G-4 already says `research/` is created on first `ark-researcher` dispatch; the new `task new --tier research` flow must not pre-empt that. The PLAN's C-7 explicitly handles the absent-`research/` case as a no-op in `ark_files_for_first_commit`, which is the right behavior. V-F-3 covers the empty-corpus close.
- **Required Action:** Adopt.
