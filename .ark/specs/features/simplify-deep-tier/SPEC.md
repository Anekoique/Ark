
[**Goals**]

- G-1: Deep tier runs one PLAN then one REVIEW with no loop back to PLAN.
- G-2: Deep tier seeds plain `PLAN.md` / `REVIEW.md`, parallel to `VERIFY.md`.
- G-3: REVIEW findings are folded into `PLAN.md` in place before EXECUTE.
- G-4: `TaskToml` drops `iteration` / `max_iterations`; legacy files still load.
- G-5: PLAN / REVIEW templates carry no loop vocabulary.

[**Non-goals**]

- NG-1: Standard, quick, and research tiers' lifecycles are unchanged.
- NG-2: No reopen-for-second-review mechanism replaces the removed loop.

[**Architecture**]

```
crates/ark-core/src/commands/agent/
├── state.rs            # deep table: Design→Plan→Review→Execute→Verify→Committed→Archived
│                       #   TaskToml drops iteration + max_iterations fields
├── task/
│   ├── phase.rs        # artifact_for: deep PLAN→PLAN.md, REVIEW→REVIEW.md (no NN_)
│   ├── new.rs          # construct TaskToml without iteration/max_iterations
│   ├── new_tests.rs    # drop iteration/max_iterations assertions; delete deep_tier_seeds_max_iterations
│   ├── promote.rs      # remove BOTH the max_iterations block AND the PLAN.md→00_PLAN.md rename
│   └── discard.rs      # recognize plain PLAN.md / REVIEW.md as seeded artifacts
├── spec/extract.rs     # find_final_plan prefers PLAN.md (NN_ fallback kept); CHANGELOG cites resolved filename
└── commands/context/   # TaskSummary loses iteration; ArtifactKind keeps its filename-derived iteration
    ├── model.rs        #   drop TaskSummary.iteration field
    ├── gather.rs       #   stop reading toml.iteration; add flat REVIEW.md classify arm (PLAN.md already had one)
    ├── render.rs       #   drop the `iteration:` line + `iter={}` in active-task line
    ├── projection.rs   #   test fixtures drop iteration
    └── mod.rs          #   test fixtures drop iteration
crates/ark-core/src/state/checkout/{io,reconcile}.rs  # test fixtures drop iteration/max_iterations
templates/ark/templates/
├── PLAN.md             # strip Iteration / Depends-on / Log / Response Matrix
└── REVIEW.md           # strip Iteration / Target-Plan; single Verdict + Findings
templates/claude/commands/ark/design.md   # linear Phase 3 REVIEW (no loop)
templates/ark/workflow.md (applied: .ark/workflow.md) # lifecycle diagram + REVIEW section
```

> Source-first: edit `templates/ark/` and `templates/claude/`, then `cargo build` re-embeds them. The applied `.ark/` copies regenerate.

[**Data Structure**]

```rust
// TaskToml — the persisted per-task record. Carries no iteration counter.
struct TaskToml {
    id: String,
    title: String,
    tier: Tier,
    phase: Phase,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
    committed_at: Option<DateTime<Utc>>,
    branch: Option<String>,
    worktree_path: Option<PathBuf>,
    base_branch: Option<String>,
    start_head: Option<String>,
    journal_path: Option<String>,
}

// TaskSummary (ark context projection) — mirrors the live task; no iteration field.
struct TaskSummary {
    slug: String,
    title: String,
    tier: Tier,
    phase: Phase,
    path: PathBuf,
    updated_at: DateTime<Utc>,
}

// ArtifactKind (ark context) — RETAINED as-is. Its iteration is derived from the
// artifact *filename*, not from TaskToml, so legacy NN_PLAN.md / NN_REVIEW.md
// archives still render correctly. A flattened PLAN.md classifies as iteration 0.
enum ArtifactKind {
    Prd,
    Plan { iteration: u32 },
    Review { iteration: u32 },
    Verify,
    TaskToml,
}
```

[**API Surface**]

```rust
// state.rs — the complete legal deep-tier transitions (no Review→Plan back-edge):
//   Design → Plan, Plan → Review, Review → Execute,
//   Execute → Verify, Verify → Committed, Committed → Archived.
fn can_transition(tier: Tier, from: Phase, to: Phase) -> bool;

// phase.rs — artifact_for has no iteration parameter; deep and standard agree
// on flat filenames:
fn artifact_for(phase: Phase, tier: Tier) -> Option<(&'static str, String)>;
//   Plan   → ("PLAN",   "PLAN.md")     // all tiers that reach Plan
//   Review → ("REVIEW", "REVIEW.md")   // deep only
//   Verify → ("VERIFY", "VERIFY.md")
```

[**Constraints**]

- C-1: @source-scan: Phase::Review, Phase::Plan @ crates/ark-core/src/commands/agent/state.rs
The deep transition table has no `Review → Plan` arm.
- C-2: @test-binding: deep_design_to_plan_to_review
Deep tier seeds `PLAN.md` and `REVIEW.md` with no `NN_` prefix.
- C-3: @test-binding: task_toml_loads_without_optional_fields
A `task.toml` carrying `iteration` / `max_iterations` keys still deserializes.
- C-4: @source-scan: iteration @ crates/ark-core/src/commands/agent/state.rs
`TaskToml` declares neither `iteration` nor `max_iterations`.
- C-5: @judgment
PLAN and REVIEW templates contain no Iteration, Log, Response Matrix, or `NN_` text.

---
