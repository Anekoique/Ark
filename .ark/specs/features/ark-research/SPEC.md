
> This section is the durable design record. On deep-tier commit, it is copied verbatim into `specs/features/ark-research/SPEC.md`. The Spec is self-contained per iteration — no "see 00_PLAN" references appear here.

[**Goals**]

- G-1: `Tier::Research` is a first-class variant alongside Quick/Standard/Deep.
- G-2: `Research → Committed → Archived` is the entire legal lifecycle for tier=research.
- G-3: `task new --tier research` scaffolds PRD.md + task.toml; --worktree is opt-in.
- G-4: `task commit` on research tier skips VERIFY and SPEC extract; stages task.toml + research/**.
- G-5: `/ark:research` ships on Claude / Codex / OpenCode; Claude+OpenCode byte-identical bodies, Codex applies the documented substitution map.

[**Non-goals**]

- NG-1: No new artifact template (PRD.md is reused; no BRIEF / FINDINGS / SCOPE files).
- NG-2: No SPEC promotion: research tier never invokes `parse_spec_path` / `spec_extract` / `spec_register`.
- NG-3: No `[**Research**]` PRD block for cross-task citation; PRDs may mention research slugs informally in prose.
- NG-4: No `ark context --for research` phase projection; the existing `design` projection serves research tasks.
- NG-5: No `task_promote` cross-over with research tier; both directions are rejected (see C-18). Cross-over is by `task new`.

[**Architecture**]

```
crates/ark-core/src/
├── commands/agent/
│   ├── state.rs                       (*) Tier += Research; Phase += Research;
│   │                                       can_transition += (Research, Research, Committed)
│   │                                       and (Research, Committed, Archived)
│   └── task/
│       ├── new.rs                     (*) build_task_toml: Tier::Research → Phase::Research
│       │                                   (other tiers unchanged: Phase::Design)
│       ├── phase.rs                   (*) early-return at the head of artifact_for:
│       │                                   `if matches!(tier, Tier::Research) {
│       │                                      return None; }`
│       │                                   the existing `match phase` body is untouched
│       ├── promote.rs                 (*) early-return in task_promote: every (from, to)
│       │                                   involving Tier::Research returns
│       │                                   Error::WrongTier { expected: from, actual: opts.to };
│       │                                   phase_exists_in_tier untouched
│       └── commit.rs                  (*) check_phase_for_commit gains (Research, Research);
│                                           task_commit short-circuits the VERIFY gate and the
│                                           deep-tier SPEC branch for Tier::Research;
│                                           private helper research_corpus_files() walks
│                                           task_dir.join("research") recursively;
│                                           ark_files_for_first_commit (private) gains a
│                                           tier: Tier parameter and appends the corpus

crates/ark-cli/src/
└── agent_cli.rs                       (*) parse_tier (hand-rolled, line 324) gains
                                            "research" => Ok(Tier::Research); error message
                                            tier list updated.

templates/
├── claude/commands/ark/research.md    (+) new slash command; mirrors quick.md shape
├── codex/skills/ark-research/SKILL.md (+) new skill; Codex substitution map applied
├── opencode/commands/ark/research.md  (+) new command; byte-identical to claude variant
└── ark/workflow.md                    (*) ## Tiers bullet list grows a Research bullet;
                                            lifecycle arrow gains a second line for research;
                                            new ## Research subsection (≤30 lines)
```

Module coupling: untouched. `state.rs`, `new.rs`, `phase.rs`, `promote.rs`, `commit.rs` each get a localized edit; no new modules, no new public types, no new `Error` variants beyond reusing the existing `Error::WrongTier { expected: Tier, actual: Tier }`.

Private helper signatures (not part of the public surface):

```rust
// crates/ark-core/src/commands/agent/task/commit.rs
fn research_corpus_files(task_dir: &Path) -> Vec<PathBuf>;
//   Returns project-root-relative paths under <task_dir>/research/, recursively.
//   Empty Vec when the directory is absent. Uses walk_files_excluding (or sibling
//   walker) already present in io/fs.

fn ark_files_for_first_commit(
    task_dir: &Path,
    tier: Tier,                                          // (+) new param replacing `deep: bool`
    spec_path: &Path,
    intermediate_indexes: &[PathBuf],
    workspace_files: &[PathBuf],
) -> Vec<PathBuf>;
//   When tier == Tier::Research: append research_corpus_files(task_dir).
//   When tier == Tier::Deep:    append spec_path + intermediate_indexes (unchanged).
//   Always:                     prepend task.toml; append workspace_files.
//   Caller in task_commit threads `prev_toml.tier`.
```

Call graph for `task new --tier research` (no worktree variant; --worktree branch is identical to other tiers):

```
task_new(opts)
  ├── validate_slug, validate_title
  ├── scaffold task dir + copy_template("PRD", PRD.md)
  ├── build_task_toml: select initial phase per tier:
  │     Tier::Research → Phase::Research
  │     _              → Phase::Design
  └── state_mutate: register active + bind focus (unchanged)
```

Call graph for `task commit` on research tier:

```
task_commit(opts)
  ├── load TaskToml; check_phase_for_commit(Research, Research) → Ok
  ├── require_staged_work (unchanged; user staged PRD + research/)
  ├── pending = VerifyPendingCounts::default()    (skip parse_verify_md)
  ├── deep = false; feature_segments unused; spec_extract / spec_register skipped
  ├── workspace_record (unchanged)
  ├── save task.toml { phase = Committed, committed_at = now }
  ├── ark_files = ark_files_for_first_commit(task_dir, Tier::Research, ...)
  │     ← appends every file under <task_dir>/research/ recursively
  ├── stage_files + git commit (unchanged)
  ├── guard.commit + clear_focus (unchanged)
  └── return TaskCommitSummary { tier: Research, deep_spec_promoted: false, ... }
```

Failure path: any error before `guard.commit()` triggers `RollbackGuard::Drop`, which already handles soft-reset + ark_files unstage + task.toml restore. Research tier adds no new snapshot surface (no SPEC, no INDEX).

Call graph for `task promote` involving research tier:

```
task_promote(opts)
  ├── validate_slug
  ├── load TaskToml
  ├── if from == Tier::Research || opts.to == Tier::Research:
  │     return Err(Error::WrongTier { expected: from, actual: opts.to })
  └── (existing phase_exists_in_tier check + tier swap, untouched)
```

[**Data Structure**]

```rust
// crates/ark-core/src/commands/agent/state.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Quick,
    Standard,
    Deep,
    Research,           // (+) new variant; serde tag "research"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
    Committed,
    Archived,
    Research,           // (+) new variant; serde tag "research"
}

// can_transition table additions:
//   (Research, Research, Committed) => true
//   (Research, Committed, Archived) => true
// Every other (Research, X, Y) tuple => false (default arm).
```

No `task.toml` schema additions: research tier reuses the existing fields (`id`, `title`, `tier`, `phase`, `iteration`, `created_at`, `updated_at`, `committed_at`, etc.). `iteration` is unused by research-tier code paths but persists as `0`.

No new `Error` variants. `task new --tier research` succeeds via the existing parser; misuse routes through existing `IllegalPhaseTransition` (e.g. `ark agent task plan` on a research task) or the existing `Error::WrongTier { expected, actual }` (`task promote` involving research tier).

[**API Surface**]

Public surface re-exported from `crates/ark-core/src/lib.rs`:

```rust
pub use commands::agent::{Tier, Phase, Status, TaskToml};   // unchanged signatures; enums grow
```

CLI surface: `ark agent task new --tier research` accepted by the existing hand-rolled `parse_tier` at `crates/ark-cli/src/agent_cli.rs:324`. The change is one match arm:

```rust
fn parse_tier(s: &str) -> Result<Tier, String> {
    match s {
        "quick"    => Ok(Tier::Quick),
        "standard" => Ok(Tier::Standard),
        "deep"     => Ok(Tier::Deep),
        "research" => Ok(Tier::Research),    // (+) one new arm
        other      => Err(format!(
            "expected one of quick|standard|deep|research, got `{other}`"
        )),
    }
}
```

Slash-command body shape: canonical bodies live at `templates/claude/commands/ark/research.md` (Claude), `templates/opencode/commands/ark/research.md` (OpenCode — byte-identical to Claude modulo frontmatter), and `templates/codex/skills/ark-research/SKILL.md` (Codex — closed-form substitution map per C-11 applied to the Claude body).

The body opens with frontmatter (Claude/OpenCode `description:` plus Claude's `argument-hint`; Codex's `name: ark-research description: ...`), then a single-line synopsis ("Create a research-tier task: the deliverable is a curated reference corpus under `.ark/tasks/<slug>/research/`. No PLAN, no VERIFY, no SPEC promotion. Follow-up implementation is optional."), then five steps under `### Step N`: pull `ark context --scope phase --for design`, create the task with `ark agent task new --tier research`, fill the PRD (What/Why/Outcome remap, SPEC Path ignored, Related Specs optional), iteratively dispatch the `ark-researcher` subagent (one per topic, each writes `research/<topic>.md`), stage `.ark/tasks/<slug>/` and invoke `/ark:commit`. Closes with a "if the corpus turns into implementation" subsection pointing at a fresh `/ark:quick` or `/ark:design` (no promote), and a `See Also` pointing at `workflow.md §Research` and `/ark:commit`.

Codex variant applies the closed-form substitution map per C-11: every `/ark:<name>` substring (covering `/ark:research`, `/ark:quick`, `/ark:design`, `/ark:commit`) rewrites to `ark-<name>`; `$ARGUMENTS` rewrites to `<topic>`; the H1 line rewrites accordingly. Frontmatter shape per `templates/codex/skills/ark-quick/SKILL.md`. The subagent name `ark-researcher` is NOT a slash-command form and is left unchanged on both sides.

[**Constraints**]

- C-1: @test-binding: tier_research_round_trips_as_lowercase
`Tier::Research` serializes/deserializes as `"research"` via `#[serde(rename_all = "lowercase")]`; pre-existing tasks with `tier = "quick"|"standard"|"deep"` load unchanged.
- C-2: @test-binding: phase_research_round_trips_as_lowercase
`Phase::Research` serializes/deserializes as `"research"` identically; pre-existing tasks with the other six phases load unchanged.
- C-3: @test-binding: can_transition_research
`can_transition` returns `true` for exactly two new tuples involving Research: `(Research, Research, Committed)` and `(Research, Committed, Archived)`. Every other `(Research, _, _)` returns `false`.
- C-4: @test-binding: task_new_with_tier_research_starts_in_phase_research
`task new --tier research` writes `phase = Research` (not `phase = Design`); the `build_task_toml` switch selects per-tier initial phase.
- C-5: @test-binding: task_new_research_with_worktree_succeeds
`task new --tier research --worktree` succeeds and produces the same worktree topology as deep tier; the flag is opt-in across all tiers per `worktree` SPEC G-4.
- C-6: @test-binding: research_tier_skips_verify_gate
`task_commit` on `Tier::Research` skips `parse_verify_md` and treats `pending_verify = VerifyPendingCounts::default()`; skips `parse_spec_path`, `spec_extract`, `spec_register`; `deep_spec_promoted = false`.
- C-7: @test-binding: research_tier_commit_stages_corpus_and_clears_focus
`task_commit` on `Tier::Research` stages `task.toml` plus every file under `.ark/tasks/<slug>/research/` recursively (if the directory exists); absent `research/` directory is not an error.
- C-8: @test-binding: check_phase_for_commit_accepts_only_legal_inputs
`check_phase_for_commit` accepts `(Tier::Research, Phase::Research)`; every other phase under `Tier::Research` returns `Error::IllegalPhaseTransition`.
- C-9: @test-binding: artifact_for_research_tier_returns_none
`ark agent task plan|review|execute|verify` on a research task fails with `IllegalPhaseTransition { tier: Research, from: Research, to: X }`; no `task.toml` mutation occurs and no `*_PLAN.md` / `*_REVIEW.md` / `VERIFY.md` artifact is written.
- C-10: @judgment
PRD template is reused unchanged; the slash command body documents the research-tier semantic remap (Outcome optional, SPEC Path ignored).
- C-11: @test-binding: research_slash_command_claude_and_opencode_bodies_match
`/ark:research` slash-command bodies follow shipping per-platform conventions: Claude (`templates/claude/commands/ark/research.md`) and OpenCode (`templates/opencode/commands/ark/research.md`) are byte-identical modulo their respective frontmatter; Codex (`templates/codex/skills/ark-research/SKILL.md`) applies the following closed-form substitution map: every `/ark:<name>` substring in the body is rewritten to `ark-<name>` (covering at minimum `/ark:research`, `/ark:quick`, `/ark:design`, `/ark:commit`); `$ARGUMENTS` is rewritten to `<topic>`; the H1 line `# \`/ark:research $ARGUMENTS\`` is rewritten to `# \`ark-research <topic>\``. `<topic>` is chosen over the existing `/ark:quick`-convention `<task description>` because the argument on research tier is literally a topic, not a description of work to perform.
- C-12: @test-binding: researcher_prompt_carries_paths_summaries_contract
The slash command names `ark-researcher` as the dispatch target and mirrors `/ark:quick`'s `[USER]` staging step verbatim ("User runs `git add .ark/tasks/<slug>/`. Then invoke `/ark:commit -m \"<message>\"`.").
- C-13: @judgment
`workflow.md`'s `## Tiers` bullet list (currently three bullets: Quick / Standard / Deep) grows a fourth bullet at the end: `- **Research** — knowledge-gathering; corpus IS the deliverable; follow-up implementation optional. Artifacts: PRD.md, research/`.
- C-14: @judgment
`workflow.md`'s one-line lifecycle arrow (currently `DESIGN → PLAN → [REVIEW ⇄ PLAN] → EXECUTE → VERIFY → COMMIT → (later) ARCHIVE`) gains an immediately-following line: `RESEARCH → COMMIT → (later) ARCHIVE   research tier`. The existing arrow is not rewritten.
- C-15: @judgment
`workflow.md` ships a "Research" subsection ≤30 lines covering: (a) when to use research tier vs. embedded `ark-researcher` dispatch inside a tiered task; (b) the field-by-field PRD remap (What/Why/Scope filled; Outcome optional; SPEC Path ignored; Related Specs optional); (c) follow-up implementation is optional and uses a fresh `task new`, not `task promote`.
- C-16: @tool: clippy
No `Error` enum additions; no `task.toml` schema additions; no `Layout` getter additions.
- C-17: @test-binding: research_tier_commit_does_not_promote_spec
`Tier::Research` is never reached at any production site that branches on `Tier::Deep`. Reachability is gated by four independent mechanisms: (a) `check_phase_for_commit` rejects every `(Research, X≠Research)` tuple in `commit.rs` before the VERIFY block runs; (b) `check_transition` rejects every illegal Research-phase transition in `phase.rs::transition`, and the head-of-function early-return in `artifact_for` ensures no plan/review/verify artifact is seeded even if `transition()` is ever reordered; (c) the `Tier::Research`-source-or-target early-return in `task_promote` (C-18) fires before `phase_exists_in_tier`; (d) `build_task_toml` in `new.rs` selects `Phase::Research` directly at `task new` time, so no Deep-only initial-phase or `max_iterations`-seed code runs for research tier.
- C-18: @test-binding: task_promote_rejects_research_source
`task_promote` rejects every invocation involving `Tier::Research` (as source OR target) with `Error::WrongTier { expected: <source_tier>, actual: <target_tier> }`. Cross-over between research and tiered implementation is by a fresh `task new`, not promotion.

---
